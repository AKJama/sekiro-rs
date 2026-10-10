//! Boss arenas: where the game fights a boss, as data.
//!
//! An [`ArenaSpec`] names a map, the MSB enemy part the game places the boss as, and a radius.
//! [`ArenaLayout::resolve`] reads the map's `layout.json` (written by `sekiro-extract map`) and
//! returns the boss's MSB transform and where Wolf starts; [`crop`] keeps the collision within
//! the radius. Everything is in Bevy space (FromSoftware x mirrored, see `docs/FORMATS.md`).
//!
//! Adding a boss is one new row in [`ARENAS`].

use std::path::Path;
use std::sync::Arc;

use glam::{Vec2, Vec3};
use sekiro_formats::hknp::CollisionMesh;
use serde_json::Value;

use crate::body::Ground;
use crate::collision::CollisionWorld;
use crate::duel::Duel;
use crate::enemy::EnemySpec;
use crate::events::EventEntry;

/// One boss arena.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArenaSpec {
    /// Command-line name (`--arena owl`).
    pub name: &'static str,
    /// Map id, exported to `cache/maps/<map>`.
    pub map: &'static str,
    /// The boss's MSB enemy part: his position and facing.
    pub boss_part: &'static str,
    /// The enemy (see [`EnemySpec::by_name`]).
    pub enemy: &'static str,
    /// Horizontal radius in metres around the boss: map pieces, objects and collision that
    /// reach inside it are loaded.
    pub radius: f32,
    /// Where Wolf starts: an MSB player start part, or `None` for [`FALLBACK_DISTANCE`]
    /// metres in front of the boss.
    pub player_start: Option<&'static str>,
    /// Map piece models not shown in this fight: event-state alternatives that the map's
    /// event script keeps hidden during it, by model name prefix.
    pub hidden_models: &'static [&'static str],
}

/// The map events that run a boss fight (see [`crate::events`]): which events of the map's
/// script to run, from where, and the boss's entity id in them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BossEvents {
    /// The map whose `event/<map>.emevd` holds the events.
    pub map: &'static str,
    /// The boss's MSB entity id, as the events name him.
    pub boss_entity: i32,
    /// Event flags that are on when the fight starts (the story state the fight needs).
    pub flags: &'static [i32],
    pub events: &'static [EventEntry],
}

/// The Great Shinobi Owl on the castle top (`docs/OWL.md`, "Phase 2"). From
/// `event/m11_01_00_00.emevd`:
///
/// - 11115860 sets the fight up after the intro cutscene; we join it after its `Label6`:
///   immortality on, the overhead HP bar off, the boss bar (NpcName 905060) after 2 s;
/// - 11115870 is phase 2: when his first deathblow's get-up (`a000_012000`) raises event
///   message 20, it hides the boss bar, sends HKS command 506000 (his heat-up `W_Event20005`)
///   and gives him SpEffect 200051 ("behaviour pattern change 1"); after 3 s it waits for
///   message 60 (his smoke escape) or for the heat-up SpEffect 3506080 to end, sets flag
///   11110860 and shows the boss bar again. With 11110860 already set (a retry) it only gives
///   200051;
/// - 11115872 lifts the immortality when his finishing deathblow clip adds 3506050;
/// - 11115850 handles his defeat on message 50 (in the finishing clip) or his death;
/// - 11115873 shows the smoke map effect on message 40.
///
/// Flags 8301 and 9303 are the story state in which the event script starts this fight
/// (11115860 needs both); they open no other branch of the events we run.
pub const OWL_EVENTS: BossEvents = BossEvents {
    map: "m11_01_00_00",
    boss_entity: 1_110_850,
    flags: &[8301, 9303],
    events: &[
        EventEntry {
            id: 11_115_860,
            from_label: Some(6),
        },
        EventEntry {
            id: 11_115_870,
            from_label: None,
        },
        EventEntry {
            id: 11_115_872,
            from_label: None,
        },
        EventEntry {
            id: 11_115_850,
            from_label: None,
        },
        EventEntry {
            id: 11_115_873,
            from_label: None,
        },
    ],
};

/// The fight events of an enemy, by character id (also used off the arena, on the flat floor).
pub fn boss_events(chr: &str) -> Option<&'static BossEvents> {
    match chr {
        "c5060" => Some(&OWL_EVENTS),
        _ => None,
    }
}

/// Every arena. The player start choices are documented in `docs/OWL.md` ("The castle roof").
pub const ARENAS: &[ArenaSpec] = &[ArenaSpec {
    name: "owl",
    map: "m11_01_00_00",
    boss_part: "c5060_0000",
    enemy: "owl",
    radius: 60.0,
    // On the same roof, 18.9 m from the Owl; the other start on the roof (`c0000_0010`) is 1 m
    // from him.
    player_start: Some("c0000_0003"),
    // The burnt floor of the castle top (entity 1114551, group 1117551) overlaps the tatami
    // floor m015070 (1114550, group 1117550). Event 11115943 activates the burnt one only once
    // flag 11115921 is set (after cutscene 11010080, which starts the later fight with c5430 on
    // this roof) and the tatami again with 11110100; the Owl fight shows the tatami.
    hidden_models: &["m015072"],
}];

/// Distance in front of the boss where Wolf starts when the arena names no player start.
pub const FALLBACK_DISTANCE: f32 = 6.0;

impl ArenaSpec {
    pub fn by_name(name: &str) -> Result<&'static Self, String> {
        let name = name.to_ascii_lowercase();
        ARENAS.iter().find(|a| a.name == name).ok_or_else(|| {
            let known: Vec<&str> = ARENAS.iter().map(|a| a.name).collect();
            format!("unknown arena {name}: known are {}", known.join(", "))
        })
    }

    pub fn enemy_spec(&self) -> EnemySpec {
        EnemySpec::by_name(self.enemy).expect("arena enemies are known enemy names")
    }
}

/// A position on the map and a Bevy-space yaw in radians (yaw 0 faces -Z).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spot {
    pub position: Vec3,
    pub yaw: f32,
}

/// Where an arena puts its two characters.
#[derive(Debug, Clone, PartialEq)]
pub struct ArenaLayout {
    /// The boss at his MSB part's transform.
    pub boss: Spot,
    /// Wolf, facing the boss.
    pub wolf: Spot,
    /// Where Wolf's spot came from, for logs.
    pub wolf_source: String,
}

impl ArenaLayout {
    /// Reads the boss's part and Wolf's start from `layout.json`.
    pub fn resolve(spec: &ArenaSpec, layout: &Value) -> Result<Self, String> {
        let row = |key: &str, name: &str| -> Result<Spot, String> {
            let r = layout[key]
                .as_array()
                .into_iter()
                .flatten()
                .find(|r| r["name"].as_str() == Some(name))
                .ok_or_else(|| format!("{}: no part {name} in {key}", spec.map))?;
            let t = &r["translation"];
            let f = |i: usize| t[i].as_f64().map(|v| v as f32);
            let position = match (f(0), f(1), f(2)) {
                (Some(x), Some(y), Some(z)) => Vec3::new(x, y, z),
                _ => return Err(format!("{name}: no translation")),
            };
            let yaw = r["yaw_degrees"].as_f64().unwrap_or(0.0) as f32;
            Ok(Spot {
                position,
                yaw: yaw.to_radians(),
            })
        };
        let boss = row("enemies", spec.boss_part)?;
        let (position, wolf_source) = match spec.player_start {
            Some(start) => (
                row("players", start)?.position,
                format!("player start {start}"),
            ),
            None => {
                let ahead = forward(boss.yaw) * FALLBACK_DISTANCE;
                (
                    boss.position + ahead,
                    format!("{FALLBACK_DISTANCE} m in front of {}", spec.boss_part),
                )
            }
        };
        // Facing the boss (the MSB yaw of a player start is the way a warp leaves you facing,
        // not necessarily toward the fight).
        let wolf = Spot {
            position,
            yaw: yaw_toward(position, boss.position),
        };
        Ok(Self {
            boss,
            wolf,
            wolf_source,
        })
    }

    /// [`ArenaLayout::resolve`] from `<dir>/layout.json`.
    pub fn load(spec: &ArenaSpec, dir: &Path) -> Result<Self, String> {
        let path = dir.join("layout.json");
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let json: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        Self::resolve(spec, &json)
    }

    /// The arena's centre: the boss's start.
    pub fn center(&self) -> Vec3 {
        self.boss.position
    }
}

/// How far below its start a character may fall before it is put back (fall death is not
/// modelled yet; the castle roof's eaves are about 6 m above the next roof down).
pub const FALL_RESPAWN_DEPTH: f32 = 30.0;

/// An arena ready to fight on: where both stand and the collision they stand on.
pub struct Arena {
    pub spec: &'static ArenaSpec,
    pub layout: ArenaLayout,
    pub world: Arc<CollisionWorld>,
}

impl Arena {
    /// Reads `cache/maps/<map>`: the layout, and the collision within the arena's radius (or
    /// all of it with `full_map`).
    pub fn load(cache: &Path, spec: &'static ArenaSpec, full_map: bool) -> Result<Self, String> {
        let dir = cache.join("maps").join(spec.map);
        let layout = ArenaLayout::load(spec, &dir)?;
        let path = dir.join("collision.bin");
        let world = if full_map {
            CollisionWorld::load(&path)?
        } else {
            CollisionWorld::load_within(&path, layout.center(), spec.radius)?
        };
        Ok(Self {
            spec,
            layout,
            world: Arc::new(world),
        })
    }

    /// The floor at or just below a spot (its own height when there is none).
    pub fn floor_at(&self, p: Vec3) -> Vec3 {
        let y = Ground::floor(&*self.world, p.x, p.z, p.y + 1.0).unwrap_or(p.y);
        p.with_y(y)
    }

    /// Puts Wolf and the boss on their spots, on the arena's collision, with the fall respawn.
    pub fn place(&self, duel: &mut Duel) {
        for (c, spot) in [
            (&mut duel.player, self.layout.wolf),
            (&mut duel.enemy, self.layout.boss),
        ] {
            c.body.position = self.floor_at(spot.position).to_array();
            c.body.yaw = spot.yaw;
            c.body.fall_top = c.body.position[1];
            c.ground = Box::new(self.world.clone());
        }
        duel.fall_respawn_depth = Some(FALL_RESPAWN_DEPTH);
    }
}

/// The horizontal direction a yaw faces (yaw 0 faces -Z, positive yaw turns left).
pub fn forward(yaw: f32) -> Vec3 {
    Vec3::new(-yaw.sin(), 0.0, -yaw.cos())
}

/// The yaw at `from` that faces `to`.
pub fn yaw_toward(from: Vec3, to: Vec3) -> f32 {
    let d = to - from;
    (-d.x).atan2(-d.z)
}

/// Whether a horizontal box reaches within `radius` of `center` (on the horizontal plane).
pub fn box_within(min: Vec2, max: Vec2, center: Vec2, radius: f32) -> bool {
    let nearest = center.clamp(min, max);
    nearest.distance_squared(center) <= radius * radius
}

/// The triangles of `mesh` that reach within `radius` of `center` on the horizontal plane
/// (bounding-box test), with only the vertices they use.
pub fn crop(mesh: &CollisionMesh, center: Vec3, radius: f32) -> CollisionMesh {
    let c = Vec2::new(center.x, center.z);
    let mut remap = vec![u32::MAX; mesh.vertices.len()];
    let mut out = CollisionMesh::default();
    for (t, tri) in mesh.indices.as_chunks::<3>().0.iter().enumerate() {
        let p = tri.map(|i| {
            let v = mesh.vertices[i as usize];
            Vec2::new(v[0], v[2])
        });
        let min = p[0].min(p[1]).min(p[2]);
        let max = p[0].max(p[1]).max(p[2]);
        if !box_within(min, max, c, radius) {
            continue;
        }
        for &i in tri {
            let r = &mut remap[i as usize];
            if *r == u32::MAX {
                *r = out.vertices.len() as u32;
                out.vertices.push(mesh.vertices[i as usize]);
            }
            out.indices.push(*r);
        }
        out.materials
            .push(mesh.materials.get(t).copied().unwrap_or(u32::MAX));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn yaw_toward_matches_forward() {
        let from = Vec3::new(1.0, 0.0, 2.0);
        for to in [
            Vec3::new(5.0, 3.0, -1.0),
            Vec3::new(-4.0, 0.0, 2.0),
            Vec3::new(1.0, 0.0, 9.0),
        ] {
            let f = forward(yaw_toward(from, to));
            let want = (to - from).with_y(0.0).normalize();
            assert!(f.distance(want) < 1e-5, "{f} vs {want}");
        }
        // Yaw 0 faces -Z; a quarter turn left faces -X.
        assert!(forward(0.0).distance(Vec3::NEG_Z) < 1e-6);
        assert!(forward(std::f32::consts::FRAC_PI_2).distance(Vec3::NEG_X) < 1e-6);
    }

    #[test]
    fn resolves_boss_and_start() {
        let layout = json!({
            "enemies": [{ "name": "c5060_0000", "translation": [104.5, 54.0, 244.0], "yaw_degrees": -46.0 }],
            "players": [{ "name": "c0000_0003", "translation": [120.0, 54.0, 234.0], "yaw_degrees": -46.8 }],
        });
        let spec = ArenaSpec::by_name("OWL").unwrap();
        let a = ArenaLayout::resolve(spec, &layout).unwrap();
        assert_eq!(a.boss.position, Vec3::new(104.5, 54.0, 244.0));
        assert!((a.boss.yaw - (-46f32).to_radians()).abs() < 1e-6);
        let to_boss = (a.boss.position - a.wolf.position).normalize();
        assert!(forward(a.wolf.yaw).dot(to_boss) > 0.999);

        let no_start = ArenaSpec {
            player_start: None,
            ..*spec
        };
        let a = ArenaLayout::resolve(&no_start, &layout).unwrap();
        let d = a.wolf.position.distance(a.boss.position);
        assert!((d - FALLBACK_DISTANCE).abs() < 1e-4, "{d}");
        assert!(ArenaSpec::by_name("nobody").is_err());
        assert_eq!(spec.enemy_spec(), EnemySpec::owl());
    }

    #[test]
    fn crop_keeps_nearby_triangles() {
        let mesh = CollisionMesh {
            vertices: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0],
                [100.0, 0.0, 0.0],
                [101.0, 0.0, 0.0],
                [100.0, 0.0, 1.0],
                // A long triangle that crosses the circle without a vertex inside it.
                [-50.0, 0.0, 5.0],
                [50.0, 0.0, 5.0],
                [50.0, 0.0, 6.0],
            ],
            indices: vec![0, 1, 2, 3, 4, 5, 6, 7, 8],
            materials: vec![1, 2, 3],
        };
        let out = crop(&mesh, Vec3::ZERO, 10.0);
        assert_eq!(out.materials, vec![1, 3]);
        assert_eq!(out.vertices.len(), 6);
        assert_eq!(out.indices, vec![0, 1, 2, 3, 4, 5]);
    }
}

//! Deathblows: the paired "throw" that kills a posture-broken or unaware enemy.
//!
//! See `docs/DEATHBLOW.md` for the data trail. In short:
//! - ThrowParam rows pair an attacker animation with a defender animation. A deathblow is two
//!   rows: a *start* row (the attacker lunges, no defender animation) and a *body* row (both
//!   play, the defender snapped to a dummy poly on the attacker).
//! - The start row's range and angles decide whether the deathblow can begin. The body row is
//!   checked when the start animation's grab event fires.
//! - The defender plays `defAnimId`, or `defAnimId + 1` when the blow kills (the behaviour
//!   graph's `ThrowDefDeath<id>` states). The damage lands on the attacker's
//!   `ThrowAttackBehavior` TAE event.
//!
//! Space is the simulation's world space (Bevy: right-handed, +Y up). A [`Placement`] with yaw 0
//! faces -Z and positive yaw turns counter-clockwise seen from above, as in [`crate::body`].

use std::path::Path;
use std::sync::Arc;

use sekiro_formats::param::{RawParam, Row, Table};
use sekiro_formats::paramdef::ParamDef;
use sekiro_formats::{Result, flver};

use crate::input::angle_diff;

/// One ThrowParam row, with the fields the deathblow needs.
#[derive(Debug, Clone, PartialEq)]
pub struct ThrowRow {
    pub id: i32,
    pub atk_chr: i32,
    pub def_chr: i32,
    /// Maximum horizontal distance between the two characters, metres.
    pub dist: f32,
    /// Allowed angle, degrees, between the defender's back direction and the direction from
    /// the defender to the attacker: 90..180 means "in front", 0..90 "behind".
    pub diff_ang_min: f32,
    pub diff_ang_max: f32,
    /// Allowed height of the defender above / below the attacker.
    pub upper_y: f32,
    pub lower_y: f32,
    /// Maximum angle, degrees, between the attacker's facing and the direction to the defender.
    pub diff_ang_my_to_def: f32,
    pub atk_anim: i32,
    pub def_anim: i32,
    pub atk_anim_offset: i32,
    pub def_anim_offset: i32,
    /// Attacker dummy poly the defender is snapped to (0: none).
    pub atk_sorb_dummy: i16,
    pub def_sorb_dummy: i16,
    pub throw_kind: i64,
    /// Turn the attacker to face the defender when the row starts.
    pub turn_attacker: bool,
    /// Seconds over which the defender slides onto the sorb dummy.
    pub interpolation_time: f32,
}

impl ThrowRow {
    pub fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row.id(),
            atk_chr: row.i32("AtkChrId")?,
            def_chr: row.i32("DefChrId")?,
            dist: row.f32("Dist")?,
            diff_ang_min: row.f32("DiffAngMin")?,
            diff_ang_max: row.f32("DiffAngMax")?,
            upper_y: row.f32("upperYRange")?,
            lower_y: row.f32("lowerYRange")?,
            diff_ang_my_to_def: row.f32("diffAngMyToDef")?,
            atk_anim: row.i32("atkAnimId")?,
            def_anim: row.i32("defAnimId")?,
            atk_anim_offset: row.int("atkAnimOffset")? as i32,
            def_anim_offset: row.int("defAnimOffset")? as i32,
            atk_sorb_dummy: row.int("atkSorbDmyId")? as i16,
            def_sorb_dummy: row.int("defSorbDmyId")? as i16,
            throw_kind: row.int("throwKind")?,
            turn_attacker: row.int("isTurnAtker")? != 0,
            interpolation_time: row.f32("adsrobModelPosInterpolationTime")?,
        })
    }

    /// The attacker's clip, e.g. `a200_510000`.
    pub fn attacker_anim(&self) -> String {
        format!("a{:03}_{:06}", self.atk_anim_offset, self.atk_anim)
    }

    /// The defender's clip when it survives, e.g. `a000_012000`.
    pub fn defender_anim(&self) -> Option<String> {
        (self.def_anim > 0).then(|| format!("a{:03}_{:06}", self.def_anim_offset, self.def_anim))
    }

    /// The defender's clip when the blow kills: the graph's `ThrowDefDeath<defAnimId + 1>`.
    pub fn defender_death_anim(&self) -> Option<String> {
        (self.def_anim > 0)
            .then(|| format!("a{:03}_{:06}", self.def_anim_offset, self.def_anim + 1))
    }

    /// Whether the two characters satisfy this row's distance, height and angle limits.
    pub fn in_range(&self, attacker: &Placement, defender: &Placement) -> bool {
        let dx = attacker.position[0] - defender.position[0];
        let dz = attacker.position[2] - defender.position[2];
        let dist = (dx * dx + dz * dz).sqrt();
        if dist > self.dist {
            return false;
        }
        let dy = defender.position[1] - attacker.position[1];
        if dy > self.upper_y || dy < -self.lower_y {
            return false;
        }
        if dist < 1e-4 {
            return true;
        }
        let to_attacker = [dx / dist, dz / dist];
        let [fx, fz] = defender.forward();
        let back = [-fx, -fz];
        let def_angle = angle_between(back, to_attacker);
        if def_angle < self.diff_ang_min - 1e-3 || def_angle > self.diff_ang_max + 1e-3 {
            return false;
        }
        let my_angle = angle_between(attacker.forward(), [-to_attacker[0], -to_attacker[1]]);
        my_angle <= self.diff_ang_my_to_def + 1e-3
    }
}

/// Angle in degrees between two unit 2D vectors.
fn angle_between(a: [f32; 2], b: [f32; 2]) -> f32 {
    (a[0] * b[0] + a[1] * b[1])
        .clamp(-1.0, 1.0)
        .acos()
        .to_degrees()
}

/// ThrowParam, loaded from the extracted gameparam folder.
pub struct ThrowTable {
    pub rows: Vec<ThrowRow>,
}

impl ThrowTable {
    pub fn load(param_dir: &Path, defs_dir: &Path) -> Result<Self> {
        let raw = RawParam::parse(std::fs::read(param_dir.join("ThrowParam.param"))?)?;
        let def = ParamDef::load(&defs_dir.join("ThrowParam.xml"))?;
        let table = Table::new("ThrowParam", &raw, Arc::new(def), None)?;
        let rows = table
            .rows()
            .map(|r| ThrowRow::from_row(&r))
            .collect::<Result<_>>()?;
        Ok(Self { rows })
    }

    pub fn load_from_cache(cache: &Path) -> Result<Self> {
        Self::load(
            &cache.join("raw/param/gameparam/gameparam.parambnd.d"),
            &cache.join("refs/paramdex/Defs"),
        )
    }

    /// The player-vs-`def_chr` row with the given suffix (`id % 1000`) and variation digit
    /// (`id / 1000 % 10`; 0 is the standard enemy, others are "Varier" and tutorial copies).
    pub fn player_row(&self, def_chr: i32, variation: i32, suffix: i32) -> Option<&ThrowRow> {
        self.rows.iter().find(|r| {
            r.atk_chr == 0
                && r.def_chr == def_chr
                && r.id.rem_euclid(1000) == suffix
                && (r.id / 1000).rem_euclid(10) == variation
        })
    }
}

/// Why the enemy can be deathblown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Situation {
    /// Posture broken (the TrunkCollapse stagger): front or back deathblow.
    PostureBroken,
    /// Not in combat with the player: the stealth deathblow from behind.
    Unaware,
}

/// ThrowParam row suffixes (`id % 1000`) of the standard player-vs-enemy deathblows, as
/// `(start, body)` pairs in the order they are tried.
pub fn candidate_pairs(situation: Situation) -> &'static [(i32, i32)] {
    match situation {
        // Near front, far front, then from behind a broken enemy.
        Situation::PostureBroken => &[(5, 6), (0, 1), (110, 111)],
        // Stealth deathblow from behind.
        Situation::Unaware => &[(20, 21)],
    }
}

/// Picks the deathblow rows whose start row the current positions satisfy.
pub fn select<'a>(
    table: &'a ThrowTable,
    def_chr: i32,
    variation: i32,
    situation: Situation,
    attacker: &Placement,
    defender: &Placement,
) -> Option<(&'a ThrowRow, &'a ThrowRow)> {
    candidate_pairs(situation).iter().find_map(|&(s, b)| {
        let start = table.player_row(def_chr, variation, s)?;
        let body = table.player_row(def_chr, variation, b)?;
        start.in_range(attacker, defender).then_some((start, body))
    })
}

/// Where a character stands and which way it faces.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Placement {
    pub position: [f32; 3],
    pub yaw: f32,
}

impl Placement {
    pub fn forward(&self) -> [f32; 2] {
        [-self.yaw.sin(), -self.yaw.cos()]
    }

    /// A point given in this character's local space (-Z forward) in world space.
    pub fn transform(&self, local: [f32; 3]) -> [f32; 3] {
        let (s, c) = self.yaw.sin_cos();
        [
            self.position[0] + local[0] * c + local[2] * s,
            self.position[1] + local[1],
            self.position[2] - local[0] * s + local[2] * c,
        ]
    }

    /// Yaw that faces `target` from here.
    pub fn yaw_towards(&self, target: [f32; 3]) -> f32 {
        let dx = target[0] - self.position[0];
        let dz = target[2] - self.position[2];
        (-dx).atan2(-dz)
    }

    fn lerp(&self, to: &Placement, t: f32) -> Placement {
        let p = |i: usize| self.position[i] + (to.position[i] - self.position[i]) * t;
        Placement {
            position: [p(0), p(1), p(2)],
            yaw: self.yaw + angle_diff(to.yaw, self.yaw) * t,
        }
    }
}

/// A dummy poly as a frame in its character's local space (world convention, mirrored from
/// FromSoftware space).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DummyFrame {
    pub position: [f32; 3],
    /// Yaw of a character standing on the dummy and facing along its forward vector.
    pub yaw: f32,
}

impl DummyFrame {
    /// Reads dummy `reference_id` from a FLVER. Throw dummies hang off root-level nodes with no
    /// attach bone, so they are fixed in the character's frame; the parent node's translation is
    /// applied and its rotation (identity in c0000) ignored.
    pub fn from_flver(model: &flver::Flver, reference_id: i16) -> Option<Self> {
        let d = model
            .dummies
            .iter()
            .find(|d| d.reference_id == reference_id)?;
        let offset = usize::try_from(d.parent_bone)
            .ok()
            .and_then(|p| model.nodes.get(p))
            .map_or([0.0; 3], |n| n.translation);
        let p = [
            d.position[0] + offset[0],
            d.position[1] + offset[1],
            d.position[2] + offset[2],
        ];
        // Mirror X into world space.
        let (fx, fz) = (-d.forward[0], d.forward[2]);
        Some(Self {
            position: [-p[0], p[1], p[2]],
            yaw: (-fx).atan2(-fz),
        })
    }

    /// Where a character snapped to this dummy of `owner` stands.
    pub fn placement(&self, owner: &Placement) -> Placement {
        Placement {
            position: owner.transform(self.position),
            yaw: owner.yaw + self.yaw,
        }
    }
}

/// Times of the throw events in the attacker's clips, from `cache/anim/<chr>/tae.json`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThrowTimes {
    /// The grab check in the start clip (its first throw `CommonBehavior` event).
    pub grab: (f32, f32),
    /// When the damage lands in the body clip (`ThrowAttackBehavior`).
    pub damage: f32,
    /// Time in the defender's survive clip where a lethal blow branches to the death clip
    /// (its one-frame `ChrActionFlag` 69 event; the death clip's first frame matches it).
    pub death_branch: Option<f32>,
}

impl ThrowTimes {
    pub fn from_tae_json(tae: &serde_json::Value, start: &str, body: &str) -> Option<Self> {
        let events = |name: &str| -> Option<Vec<(String, f32, f32)>> {
            let anim = tae["files"]
                .as_array()?
                .iter()
                .flat_map(|f| f["animations"].as_array().into_iter().flatten())
                .find(|a| a["name"] == name)?;
            Some(
                anim["events"]
                    .as_array()?
                    .iter()
                    .filter_map(|e| {
                        Some((
                            e["decoded"]["name"].as_str()?.to_string(),
                            e["start"].as_f64()? as f32,
                            e["end"].as_f64()? as f32,
                        ))
                    })
                    .collect(),
            )
        };
        let grab = events(start)?
            .into_iter()
            .filter(|(n, _, _)| n == "CommonBehavior")
            .map(|(_, s, e)| (s, e))
            .reduce(|a, b| if b.0 < a.0 { b } else { a })?;
        let damage = events(body)?
            .into_iter()
            .filter(|(n, _, _)| n == "ThrowAttackBehavior")
            .map(|(_, s, _)| s)
            .reduce(f32::min)?;
        Some(Self {
            grab,
            damage,
            death_branch: None,
        })
    }

    /// Loads the attacker's throw events and, given the defender's survive clip, its death
    /// branch time.
    pub fn load(
        cache: &Path,
        attacker_chr: &str,
        start: &str,
        body: &str,
        defender: Option<(&str, &str)>,
    ) -> Option<Self> {
        let read = |chr: &str| -> Option<serde_json::Value> {
            let text = std::fs::read_to_string(cache.join(format!("anim/{chr}/tae.json"))).ok()?;
            serde_json::from_str(&text).ok()
        };
        let mut times = Self::from_tae_json(&read(attacker_chr)?, start, body)?;
        if let Some((chr, clip)) = defender {
            times.death_branch = read(chr).and_then(|t| death_branch(&t, clip));
        }
        Some(times)
    }
}

/// The first `ChrActionFlag` 69 event in a defender throw clip.
pub fn death_branch(tae: &serde_json::Value, clip: &str) -> Option<f32> {
    let anim = tae["files"]
        .as_array()?
        .iter()
        .flat_map(|f| f["animations"].as_array().into_iter().flatten())
        .find(|a| a["name"] == clip)?;
    anim["events"]
        .as_array()?
        .iter()
        .filter(|e| e["decoded"]["name"] == "ChrActionFlag")
        .filter(|e| {
            e["decoded"]["fields"].as_array().is_some_and(|f| {
                f.iter()
                    .any(|x| x["name"] == "FlagType" && x["value"] == 69)
            })
        })
        .filter_map(|e| e["start"].as_f64().map(|s| s as f32))
        .reduce(f32::min)
}

/// How many deathblows an enemy takes: NpcParam `ninsatuNum`, where 0 (ordinary enemies) means
/// one. Inferred from the data; bosses carry 2 or more.
pub fn deathblow_marks(ninsatu_num: i32) -> i32 {
    ninsatu_num.max(1)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// The attacker's lunge; the defender keeps playing its stagger.
    Start,
    /// The paired animation, defender snapped to the attacker's sorb dummy.
    Body,
    /// Over. The defender keeps playing its clip, now under its own root motion.
    Done,
    /// The grab window passed without the body row's conditions holding.
    Missed,
}

/// What changed this step.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DeathblowStep {
    /// Start this clip on the attacker.
    pub attacker_clip: Option<String>,
    /// Start this clip on the defender.
    pub defender_clip: Option<String>,
    /// Where the defender must stand this frame (Body phase only).
    pub defender_placement: Option<Placement>,
    /// The blow landed this step.
    pub hit: bool,
    /// The defender died this step.
    pub killed: bool,
}

/// One deathblow in progress.
#[derive(Debug, Clone)]
pub struct Deathblow {
    pub start: ThrowRow,
    pub body: ThrowRow,
    pub phase: Phase,
    /// Time in the current phase's attacker clip.
    pub time: f32,
    pub times: ThrowTimes,
    pub sorb: DummyFrame,
    /// Duration of the attacker's body clip.
    pub body_duration: f32,
    /// The blow kills (no deathblow marks left after it).
    pub lethal: bool,
    defender_from: Option<Placement>,
    hit: bool,
    branched: bool,
}

impl Deathblow {
    /// Begins a deathblow. Returns it with the first step, which starts the attacker's lunge
    /// and, for turning rows, faces it toward the defender.
    pub fn begin(
        rows: (&ThrowRow, &ThrowRow),
        times: ThrowTimes,
        sorb: DummyFrame,
        body_duration: f32,
        marks_left: i32,
    ) -> (Self, DeathblowStep) {
        let db = Self {
            start: rows.0.clone(),
            body: rows.1.clone(),
            phase: Phase::Start,
            time: 0.0,
            times,
            sorb,
            body_duration,
            lethal: marks_left <= 1,
            defender_from: None,
            hit: false,
            branched: false,
        };
        let step = DeathblowStep {
            attacker_clip: Some(db.start.attacker_anim()),
            ..DeathblowStep::default()
        };
        (db, step)
    }

    /// The yaw the attacker should take when the start row turns it toward the defender.
    pub fn start_facing(&self, attacker: &Placement, defender: &Placement) -> Option<f32> {
        self.start
            .turn_attacker
            .then(|| attacker.yaw_towards(defender.position))
    }

    /// The defender clip the body starts with: the survive clip, which a lethal blow leaves for
    /// the death clip at [`ThrowTimes::death_branch`] (straight away when that is unknown).
    pub fn defender_clip(&self) -> Option<String> {
        if self.lethal && self.times.death_branch.is_none() {
            self.body.defender_death_anim()
        } else {
            self.body.defender_anim()
        }
    }

    /// Advances by `dt` with the characters where they are now (the attacker already moved by
    /// its own root motion this frame).
    pub fn step(&mut self, dt: f32, attacker: &Placement, defender: &Placement) -> DeathblowStep {
        let mut out = DeathblowStep::default();
        match self.phase {
            Phase::Start => {
                self.time += dt;
                if self.time >= self.times.grab.0 {
                    if self.body.in_range(attacker, defender) {
                        self.phase = Phase::Body;
                        self.time = 0.0;
                        self.defender_from = Some(*defender);
                        out.attacker_clip = Some(self.body.attacker_anim());
                        out.defender_clip = self.defender_clip();
                        out.defender_placement = Some(*defender);
                    } else if self.time > self.times.grab.1 {
                        self.phase = Phase::Missed;
                    }
                }
            }
            Phase::Body => {
                self.time += dt;
                let target = self.sorb.placement(attacker);
                let from = self.defender_from.unwrap_or(target);
                let w = if self.body.interpolation_time > 0.0 {
                    (self.time / self.body.interpolation_time).min(1.0)
                } else {
                    1.0
                };
                out.defender_placement = Some(from.lerp(&target, w));
                if !self.hit && self.time >= self.times.damage {
                    self.hit = true;
                    out.hit = true;
                    out.killed = self.lethal;
                }
                if self.lethal
                    && !self.branched
                    && self.times.death_branch.is_some_and(|b| self.time >= b)
                {
                    self.branched = true;
                    out.defender_clip = self.body.defender_death_anim();
                }
                if self.time >= self.body_duration {
                    self.phase = Phase::Done;
                }
            }
            Phase::Done | Phase::Missed => {}
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row() -> ThrowRow {
        ThrowRow {
            id: 11010000,
            atk_chr: 0,
            def_chr: 1010,
            dist: 3.0,
            diff_ang_min: 90.0,
            diff_ang_max: 180.0,
            upper_y: 1.8,
            lower_y: 1.8,
            diff_ang_my_to_def: 35.0,
            atk_anim: 500000,
            def_anim: 0,
            atk_anim_offset: 200,
            def_anim_offset: 0,
            atk_sorb_dummy: 0,
            def_sorb_dummy: 0,
            throw_kind: 40000,
            turn_attacker: true,
            interpolation_time: 0.5,
        }
    }

    #[test]
    fn front_angle_and_range() {
        let r = row();
        let attacker = Placement::default(); // at origin facing -Z
        let facing_back = Placement {
            position: [0.0, 0.0, -2.0],
            yaw: std::f32::consts::PI, // facing +Z, toward the attacker
        };
        assert!(r.in_range(&attacker, &facing_back));
        let turned_away = Placement {
            position: [0.0, 0.0, -2.0],
            yaw: 0.0, // facing away: attacker is behind it
        };
        assert!(!r.in_range(&attacker, &turned_away));
        let far = Placement {
            position: [0.0, 0.0, -3.5],
            ..facing_back
        };
        assert!(!r.in_range(&attacker, &far));
        let attacker_looking_away = Placement {
            yaw: std::f32::consts::FRAC_PI_2,
            ..attacker
        };
        assert!(!r.in_range(&attacker_looking_away, &facing_back));
    }

    #[test]
    fn timeline_lunge_lock_hit_branch() {
        let start = row();
        let body = ThrowRow {
            id: 11010001,
            dist: 2.0,
            diff_ang_min: 0.0,
            diff_ang_max: 180.0,
            diff_ang_my_to_def: 180.0,
            atk_anim: 510000,
            def_anim: 12000,
            atk_sorb_dummy: 267,
            turn_attacker: false,
            ..row()
        };
        let times = ThrowTimes {
            grab: (0.27, 0.33),
            damage: 0.9,
            death_branch: Some(1.67),
        };
        let sorb = DummyFrame {
            position: [0.0, 0.0, -1.2],
            yaw: std::f32::consts::PI,
        };
        let (mut db, first) = Deathblow::begin((&start, &body), times, sorb, 2.67, 1);
        assert_eq!(first.attacker_clip.as_deref(), Some("a200_500000"));
        let mut attacker = Placement::default();
        let defender = Placement {
            position: [0.0, 0.0, -1.8],
            yaw: std::f32::consts::PI,
        };
        let dt = 1.0 / 30.0;
        let mut log = Vec::new();
        for _ in 0..120 {
            attacker.position[2] -= 0.5 * dt; // the attacker's root motion
            let s = db.step(dt, &attacker, &defender);
            log.push((db.phase, s, attacker.position[2]));
        }
        let body_start = log
            .iter()
            .position(|(_, s, _)| s.attacker_clip.is_some())
            .unwrap();
        assert_eq!(
            log[body_start].1.attacker_clip.as_deref(),
            Some("a200_510000")
        );
        assert_eq!(
            log[body_start].1.defender_clip.as_deref(),
            Some("a000_012000")
        );
        assert_eq!(log.iter().filter(|(_, s, _)| s.killed).count(), 1);
        let branch = log
            .iter()
            .find_map(|(_, s, _)| s.defender_clip.clone().filter(|c| c.ends_with("012001")));
        assert!(branch.is_some());
        // After the slide-in the defender stands on the dummy, 1.2 m ahead, facing the attacker.
        let (_, last, z) = log
            .iter()
            .rev()
            .find(|(p, _, _)| *p == Phase::Body)
            .unwrap();
        let p = last.defender_placement.unwrap();
        assert!((p.position[2] - (z - 1.2)).abs() < 0.05, "{p:?}");
        assert!(log.iter().any(|(p, _, _)| *p == Phase::Done));
    }
    #[test]
    fn sorb_dummy_in_front_facing_back() {
        // c0000 dummy 267: 1.2 m in front of the player (source -Z), forward +Z.
        let d = DummyFrame {
            position: [0.0, 0.0, -1.2],
            yaw: (-0.0f32).atan2(-1.0),
        };
        let attacker = Placement {
            position: [1.0, 0.0, 0.0],
            yaw: std::f32::consts::FRAC_PI_2, // facing -X
        };
        let p = d.placement(&attacker);
        assert!((p.position[0] - (1.0 - 1.2)).abs() < 1e-5, "{p:?}");
        assert!(p.position[2].abs() < 1e-5, "{p:?}");
        // Faces back along +X, toward the attacker.
        let f = p.forward();
        assert!((f[0] - 1.0).abs() < 1e-5 && f[1].abs() < 1e-5, "{f:?}");
    }
}

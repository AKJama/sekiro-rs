//! Sim step cost at the m11_00 spots where the live game's fixed step slowed down (#145, #156),
//! counted in collision work so the tests do not depend on the machine.
//! Skips without the exported maps.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use sekiro_formats::hknp::CollisionMesh;
use sekiro_sim::body::{Ground, WaterSurface};
use sekiro_sim::collision::{CollisionWorld, TriggerContact};

fn cache() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../cache")
}

/// The play world's collision as `crates/game/src/map_world.rs` `collision_world` builds it
/// (both Ashina blocks, helpers stripped, placed objects), without the story's switches.
fn play_world(cache: &Path) -> Option<CollisionWorld> {
    let blocks = ["m11_00_00_00", "m11_01_00_00"];
    if !blocks
        .iter()
        .all(|b| cache.join("maps").join(b).join("collision.bin").is_file())
    {
        return None;
    }
    let mut mesh = CollisionMesh::default();
    let mut helpers = sekiro_sim::hit_filter::HelperCollision::default();
    for b in blocks {
        let dir = cache.join("maps").join(b);
        let mut part =
            CollisionMesh::from_bytes(&std::fs::read(dir.join("collision.bin")).unwrap()).unwrap();
        let layout: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join("layout.json")).unwrap()).unwrap();
        helpers.extend(sekiro_sim::hit_filter::strip_helpers(&mut part, &layout));
        mesh.extend(&part);
    }
    let mut world = CollisionWorld::from_mesh(&mesh);
    world.add_helpers(&helpers);
    for b in blocks {
        world
            .add_block_objects(&cache.join("maps").join(b), b, |_, who| who)
            .unwrap();
    }
    Some(world)
}

/// A Ground that counts the floor and wall queries it forwards.
struct Counted {
    inner: Box<dyn Ground>,
    queries: Arc<AtomicU64>,
}

impl Counted {
    fn count(&self) {
        self.queries.fetch_add(1, Ordering::Relaxed);
    }
}

impl Ground for Counted {
    fn water_surface(&self, x: f32, z: f32, top: f32) -> Option<WaterSurface> {
        self.inner.water_surface(x, z, top)
    }
    fn lowest_floor(&self, x: f32, z: f32) -> Option<f32> {
        self.count();
        self.inner.lowest_floor(x, z)
    }
    fn height(&self, x: f32, z: f32) -> Option<f32> {
        self.count();
        self.inner.height(x, z)
    }
    fn trigger_contact(&self, from: [f32; 3], to: [f32; 3]) -> Option<TriggerContact> {
        self.inner.trigger_contact(from, to)
    }
    fn near_trigger(&self, part: &str, feet: [f32; 3]) -> bool {
        self.inner.near_trigger(part, feet)
    }
    fn fall_return_below(&self, x: f32, y: f32, z: f32) -> bool {
        self.inner.fall_return_below(x, y, z)
    }
    fn floor(&self, x: f32, z: f32, top: f32) -> Option<f32> {
        self.count();
        self.inner.floor(x, z, top)
    }
    fn slide(&self, feet: [f32; 3], delta: [f32; 2]) -> [f32; 2] {
        self.count();
        self.inner.slide(feet, delta)
    }
    fn slide_assisted(&self, feet: [f32; 3], delta: [f32; 2], walls: u8) -> [f32; 2] {
        self.count();
        self.inner.slide_assisted(feet, delta, walls)
    }
    fn max_step(&self) -> f32 {
        self.inner.max_step()
    }
    fn material(&self, x: f32, z: f32, top: f32) -> Option<u32> {
        self.count();
        self.inner.material(x, z, top)
    }
    fn floor_with_material(&self, x: f32, z: f32, top: f32) -> Option<(f32, Option<u32>)> {
        self.count();
        self.inner.floor_with_material(x, z, top)
    }
}

/// At the worst wall spot of #145 a slide's 4 m cells hold about 18k triangles, nearly all on
/// other storeys; only those in the body's height band may reach the per-sub-step push-out.
#[test]
fn wall_slide_tests_only_the_body_band() {
    let Some(world) = play_world(&cache()) else {
        return;
    };
    let [x, y, z] = [-257.0, -45.0, 8.0];
    let feet = [x, world.floor(x, z, y + 1.0).unwrap_or(y), z];
    let tested = (0..8)
        .map(|k| {
            let a = k as f32 * std::f32::consts::FRAC_PI_4;
            world.slide_triangle_count(feet, [a.sin() * 0.13, a.cos() * 0.13])
        })
        .max()
        .unwrap();
    // 7k-18k before the band filter.
    assert!(tested < 2000, "{tested} triangles per push-out pass");
}

/// The deathblow on the soldier where the live game stalled (#156, Wolf at about
/// (-126.3, -47.2, -193.9) facing 270 degrees): no step of the throw may spend more than a
/// bounded number of ground queries searching for pair clearance (the start's search tried
/// 640 placements of the whole paired path).
#[test]
fn deathblow_pair_search_is_bounded() {
    use sekiro_sim::{Buttons, InputFrame, arena::yaw_toward, duel::Duel};
    let cache = cache();
    let Some(world) = play_world(&cache) else {
        return;
    };
    let world = Arc::new(world);
    let mut duel = Duel::load(&cache, 1.6).unwrap();
    let queries = Arc::new(AtomicU64::new(0));
    duel.player.ground = Box::new(Counted {
        inner: Box::new(world.clone()),
        queries: queries.clone(),
    });
    duel.enemy.ground = Box::new(Counted {
        inner: Box::new(sekiro_sim::collision::EnemyGround(world.clone())),
        queries: queries.clone(),
    });
    let heading = 270.0f32.to_radians();
    let wolf = glam::Vec3::new(-126.28, -47.23, -193.88);
    let foe = wolf + glam::Vec3::new(heading.sin(), 0.0, heading.cos()) * 1.6;
    for (c, p, target) in [(&mut duel.player, wolf, foe), (&mut duel.enemy, foe, wolf)] {
        let y = world.floor(p.x, p.z, p.y + 1.0).unwrap_or(p.y);
        c.body.position = [p.x, y, p.z];
        c.body.yaw = yaw_toward(p, target);
        c.body.fall_top = y;
    }
    duel.enemy_active = false;
    duel.god = true;
    let (mut started, mut broke, mut thrown, mut worst) = (false, false, 0, 0);
    for step in 0..900 {
        if !broke {
            duel.rules.fighters[1].posture.remaining = 1;
        }
        queries.store(0, Ordering::Relaxed);
        let r = duel.step(
            &InputFrame {
                buttons: Buttons {
                    attack: !started && step % if broke { 10 } else { 40 } == 0,
                    ..Default::default()
                },
                ..Default::default()
            },
            1.0 / 60.0,
        );
        broke |= r
            .hits
            .iter()
            .any(|(h, r)| h.attacker == 0 && r.posture_broke);
        started |= r.deathblow.started.is_some();
        if started {
            worst = worst.max(queries.load(Ordering::Relaxed));
            // The start and the first steps of paired travel; the rest repeats them.
            thrown += 1;
            if thrown > 30 {
                break;
            }
        }
    }
    assert!(started, "the deathblow never started");
    // 93,722 before the bounded search, about 10,500 after (most of them the wall projection).
    assert!(worst < 20_000, "{worst} ground queries in one throw step");
}

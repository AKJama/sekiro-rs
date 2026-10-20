//! Sim step cost at the m11_00 spots where the live game's fixed step slowed down (#145, #156,
//! #161), counted in collision work so the tests do not depend on the machine.
//! Skips without the exported maps.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use sekiro_formats::hknp::CollisionMesh;
use sekiro_sim::body::{Ground, WaterSurface};
use sekiro_sim::collision::{CollisionWorld, TriggerContact};

use crate::arena_c_support::cache;

/// [`build_play_world`], built once for both tests.
fn play_world() -> Option<Arc<CollisionWorld>> {
    static WORLD: OnceLock<Option<Arc<CollisionWorld>>> = OnceLock::new();
    WORLD
        .get_or_init(|| build_play_world(&cache()).map(Arc::new))
        .clone()
}

/// The play world's collision as `crates/game/src/map_world.rs` `collision_world` builds it
/// (both Ashina blocks, helpers stripped, placed objects), without the story's switches.
fn build_play_world(cache: &Path) -> Option<CollisionWorld> {
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
    let Some(world) = play_world() else {
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
    // The world builds while the duel loads; the duel's result waits for the skip check.
    let world = std::thread::spawn(play_world);
    let duel = Duel::load(&cache(), 1.6);
    let Some(world) = world.join().unwrap() else {
        return;
    };
    let mut duel = duel.unwrap();
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
        c.snap_yaw(yaw_toward(p, target));
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

/// Where Wolf stood idle against a wall and the step took about 12 ms (#161): the hook search
/// casts a sight ray to each grapple point in view, and every one is blocked here. A ray may
/// only test the triangles of the cells it crosses that reach its height over each cell.
#[test]
fn blocked_hook_rays_test_only_their_band() {
    let Some(world) = play_world() else {
        return;
    };
    let points: Vec<sekiro_sim::wire::Point> = serde_json::from_slice(
        &std::fs::read(cache().join("maps/m11_00_00_00/grapple.json")).unwrap(),
    )
    .unwrap();
    let [x, y, z] = [-214.998, -38.577, 13.956];
    let chest = glam::Vec3::new(x, world.floor(x, z, y + 1.0).unwrap_or(y) + 1.2, z);
    let (mut rays, mut tested) = (0, 0);
    for q in points.iter().filter_map(|p| p.position) {
        let ray = glam::Vec3::from(q) - chest;
        if ray.length() < 25.0 {
            rays += 1;
            tested += world.ray_triangle_count(chest, ray.normalize(), ray.length() - 0.1);
        }
    }
    assert!(rays >= 5, "only {rays} hooks near the spot");
    // 168,198 for these 9 rays before (after gathering and sorting 284,900 cell entries),
    // 26,750 after.
    assert!(
        tested < 60_000,
        "{tested} triangles tested by {rays} hook rays"
    );
}

/// The wall jump's air kick into the ceiling at about (-256.25, -42.2, 11.43) took up to 30 ms
/// a step (#161): `docking::wall_contact` probes eight 0.8 m slides from the chest, and each
/// tested every triangle of its 4 m cells in the body's height band.
#[test]
fn wall_contact_slides_test_only_nearby_triangles() {
    let Some(world) = play_world() else {
        return;
    };
    let (feet, lift) = (
        [-256.25f32, -42.2, 11.43],
        sekiro_sim::docking::WALL_PROBE_LIFT,
    );
    let chest = [feet[0], feet[1] + lift, feet[2]];
    let tested = (0..8)
        .map(|k| {
            let a = k as f32 * std::f32::consts::FRAC_PI_4;
            let reach = sekiro_sim::docking::WALL_REACH;
            world.slide_triangle_count(chest, [a.sin() * reach, a.cos() * reach])
        })
        .max()
        .unwrap();
    // 2,815 before the horizontal cull, 525 after.
    assert!(tested < 1000, "{tested} triangles per push-out pass");
}

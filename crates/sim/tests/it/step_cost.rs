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
pub(crate) fn play_world() -> Option<Arc<CollisionWorld>> {
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

/// The most triangles any of eight slides of `reach` around `feet` tests on each push-out pass.
fn worst_slide(world: &CollisionWorld, feet: [f32; 3], reach: f32) -> usize {
    (0..8)
        .map(|k| {
            let a = k as f32 * std::f32::consts::FRAC_PI_4;
            world.slide_triangle_count(feet, [a.sin() * reach, a.cos() * reach])
        })
        .max()
        .unwrap()
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
    fn ceiling(&self, feet: [f32; 3], rise: f32) -> Option<f32> {
        self.count();
        self.inner.ceiling(feet, rise)
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
    let tested = worst_slide(&world, feet, 0.13);
    // 7k-18k before the band filter.
    assert!(tested < 2000, "{tested} triangles per push-out pass");
}

/// A rising head's ceiling sweep tests only the flat faces in its height band whose boxes reach
/// its rim: at the worst wall spot of #145, and just under the cap beam pressed against the gate
/// fence (#170).
#[test]
fn ceiling_sweeps_test_only_nearby_faces() {
    use sekiro_sim::body::Ground;
    let Some(world) = play_world() else {
        return;
    };
    let [x, z] = [-257.0, 8.0];
    let wall = [x, world.floor(x, z, -44.0).unwrap(), z];
    let [x, z] = [-141.79, -149.49];
    let floor = [x, world.floor(x, z, -50.0).unwrap(), z];
    let beam = world
        .ceiling(floor, 5.0)
        .expect("the cap beam over the fence");
    for feet in [wall, [x, beam - 0.1, z]] {
        let tested = world.ceiling_triangle_count(feet, 0.2);
        // 30 and 32.
        assert!(tested < 100, "{tested} triangles at {feet:?}");
    }
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
    use sekiro_sim::wire::{SIGHT_LIFT, Wire, sight_ray};
    let Some(world) = play_world() else {
        return;
    };
    let wire = Wire::load(&cache(), &cache().join("maps/m11_00_00_00")).unwrap();
    let [x, y, z] = [-214.998, -38.577, 13.956];
    let feet = glam::Vec3::new(x, world.floor(x, z, y + 1.0).unwrap_or(y), z);
    let chest = feet + glam::Vec3::Y * SIGHT_LIFT;
    let (mut rays, mut tested) = (0, 0);
    for point in wire.points.iter().filter_map(|p| p.position) {
        let (dir, length) = sight_ray(chest, point.into());
        if length < 25.0 {
            rays += 1;
            tested += world.ray_triangle_count(chest, dir, length);
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
    use sekiro_sim::docking::{WALL_PROBE_LIFT, WALL_REACH};
    let Some(world) = play_world() else {
        return;
    };
    let [x, y, z] = [-256.25, -42.2, 11.43];
    let tested = worst_slide(&world, [x, y + WALL_PROBE_LIFT, z], WALL_REACH);
    // 2,815 before the horizontal cull, 1,417 after.
    assert!(tested < 2000, "{tested} triangles per push-out pass");
}

/// The horizontal cull of a slide's triangles (#161) changes no slide: at both #161 spots,
/// moves of every length a step makes, up to 2 m along each axis into the slanted walls there,
/// end where they end without it.
#[test]
fn slide_cull_keeps_every_contact() {
    use rayon::prelude::*;
    let Some(world) = play_world() else {
        return;
    };
    let world: &CollisionWorld = &world;
    let mut starts = Vec::new();
    for ([x, y, z], lifts) in [
        ([-215.0f32, -38.6, 14.0], &[0.0][..]),
        (
            [-256.0, -42.2, 11.4],
            &[0.0, sekiro_sim::docking::WALL_PROBE_LIFT][..],
        ),
    ] {
        for i in -2..=2 {
            for j in -2..=2 {
                let (x, z) = (x + i as f32 * 0.25, z + j as f32 * 0.25);
                let floor = world.floor(x, z, y + 1.0).unwrap_or(y);
                // Only where a body can stand: one that starts inside a wall is pushed out of
                // it, as far as the wall is thick and not along the move.
                starts.extend(
                    lifts
                        .iter()
                        .map(|lift| [x, floor + lift, z])
                        .filter(|&feet| world.slide_uncull(feet, [0.0, 0.0]) == [0.0, 0.0]),
                );
            }
        }
    }
    let moves: Vec<[f32; 2]> = (0..16)
        .flat_map(|k| {
            let a = k as f32 * std::f32::consts::PI / 8.0;
            [0.13, 0.8, 1.4, 2.0].map(|len| [a.sin() * len, a.cos() * len])
        })
        .collect();
    // And bodies pressed against the walls there (slid 2 m eight ways), where a slide starts in
    // contact.
    let pressed: Vec<[f32; 3]> = starts
        .iter()
        .flat_map(|&feet| {
            moves.iter().skip(3).step_by(8).map(move |&d| {
                let [dx, dz] = world.slide(feet, d);
                [feet[0] + dx, feet[1], feet[2] + dz]
            })
        })
        .collect();
    starts.extend(pressed);
    let worst = starts
        .par_iter()
        .flat_map_iter(|&feet| {
            moves.iter().map(move |&d| {
                let [a, b] = [world.slide(feet, d), world.slide_uncull(feet, d)];
                (a[0] - b[0]).abs().max((a[1] - b[1]).abs())
            })
        })
        .reduce(|| 0.0, f32::max);
    assert!(worst <= 1e-5, "the cull moved a slide by {worst} m");
}

//! Placed objects' collision (`cache/maps/<id>/object_collision.bin`, `docs/OBJECT-COLLISION.md`)
//! over the Ashina Outskirts and Ashina Castle blocks: how many objects and triangles each map
//! carries, by `ObjectParam.collisionType`, and the rideable pine over the Outskirts start ravine,
//! which the player must be able to walk along after the grapple. Skips when the maps have not
//! been exported.

use std::collections::BTreeMap;
use std::path::PathBuf;

use sekiro_formats::hknp::ObjectCollision;
use sekiro_sim::body::{Body, BodyInput};
use sekiro_sim::collision::CollisionWorld;
use sekiro_sim::hit_filter;

fn load(id: &str) -> Option<(PathBuf, ObjectCollision)> {
    let dir = sekiro_sim::cache::root().join("maps").join(id);
    if !dir.join("object_collision.bin").is_file() || !dir.join("collision.bin").is_file() {
        eprintln!(
            "skipping: {} has no exported object collision",
            dir.display()
        );
        return None;
    }
    let objects =
        ObjectCollision::from_bytes(&std::fs::read(dir.join("object_collision.bin")).unwrap())
            .unwrap();
    Some((dir, objects))
}

/// The block's own map collision plus every placed object's ([`CollisionWorld::load_map`]). The
/// play world (`map_world::collision_world` in the game crate, tested there on the same pine)
/// adds the connected blocks too and leaves out the objects the story switches off.
fn world(dir: &std::path::Path) -> CollisionWorld {
    CollisionWorld::load_map(dir).unwrap()
}

/// [`CollisionWorld::load_map`] without the placed objects: the block's character collision, its
/// helpers and its fall-prevention walls.
fn bare(dir: &std::path::Path) -> CollisionWorld {
    let layout: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("layout.json")).unwrap()).unwrap();
    let block = sekiro_sim::lo_collision::block_collision(dir, &layout, |_| false).unwrap();
    let mut world = CollisionWorld::from_mesh(&block.mesh);
    world.add_helpers(&block.helpers);
    world.add_map_fall_assist(dir, |_| true).unwrap();
    world
}

/// collisionType -> (placed objects, triangles).
fn census(objects: &ObjectCollision) -> BTreeMap<u8, (usize, usize)> {
    let mut out = BTreeMap::new();
    for part in &objects.parts {
        let entry = out.entry(part.collision_type).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += part.count as usize;
    }
    out
}

#[test]
fn object_collision_per_map() {
    // (map, placed objects with static collision, their triangles, those the player stands on).
    for (id, parts, triangles, player) in [
        ("m11_00_00_00", 1119, 116_996, 901),
        ("m11_01_00_00", 1386, 124_004, 1159),
    ] {
        let Some((_, objects)) = load(id) else {
            continue;
        };
        let by_type = census(&objects);
        println!("{id}: by collisionType {by_type:?}");
        assert_eq!(objects.parts.len(), parts, "{id}");
        assert_eq!(objects.mesh.triangle_count(), triangles, "{id}");
        let standable = objects
            .parts
            .iter()
            .filter(|p| hit_filter::object_mask(p.collision_type) & hit_filter::PLAYER != 0)
            .count();
        assert_eq!(standable, player, "{id}");
    }
}

/// The rideable snow-covered pine over the Outskirts start ravine (`o002500_0005`), from its
/// tip over the ravine (the grapple landing, -141.58, -44.26, -123.86) along the curving trunk
/// to the upper platform it leads to, in waypoints on the trunk's top.
const TRUNK: [[f32; 2]; 8] = [
    [-140.9, -124.1],
    [-139.0, -124.0],
    [-137.8, -123.4],
    [-136.8, -122.5],
    [-136.0, -121.3],
    [-136.0, -120.0],
    [-135.9, -118.8],
    [-135.5, -117.0],
];
const TREE: &str = "o002500_0005";
/// [`TREE`] as [`CollisionWorld::floor_object`] names it.
const TREE_PART: &str = "m11_00_00_00/o002500_0005";

/// Walks a body through `waypoints` at run speed (forward root motion), returning every tick's
/// feet and whether it was grounded.
fn walk(world: &CollisionWorld, start: [f32; 3], waypoints: &[[f32; 2]]) -> Vec<([f32; 3], bool)> {
    let mut b = Body {
        position: start,
        grounded: true,
        ..Body::default()
    };
    let mut out = Vec::new();
    for w in waypoints {
        for _ in 0..600 {
            let (dx, dz) = (w[0] - b.position[0], w[1] - b.position[2]);
            if dx.hypot(dz) < 0.4 || b.position[1] < start[1] - 3.0 {
                break;
            }
            b.yaw = (-dx).atan2(-dz);
            b.step(
                &BodyInput {
                    root_motion: [0.0, 0.0, -0.08, 0.0],
                    ..BodyInput::default()
                },
                1.0 / 60.0,
                world,
            );
            out.push((b.position, b.grounded));
        }
    }
    out
}

#[test]
fn the_grapple_pine_has_a_floor_from_tip_to_platform() {
    let Some((dir, objects)) = load("m11_00_00_00") else {
        return;
    };
    let tree = objects.parts.iter().find(|p| p.name == TREE).expect(TREE);
    assert_eq!(
        tree.collision_type, 0,
        "the pine is an ordinary object (group 24)"
    );
    let bare = bare(&dir);
    let world = world(&dir);
    let tip = TRUNK[0];
    let start_y = world
        .floor(tip[0], tip[1], -40.0)
        .expect("a floor at the trunk's tip");
    assert!((start_y + 44.1).abs() < 0.3, "tip floor at {start_y}");
    assert_eq!(world.floor_object(tip[0], tip[1], -40.0), Some(TREE_PART));
    // Without the object collision nothing holds Wolf over the ravine but the event walls.
    assert!(bare.floor(tip[0], tip[1], -40.0).is_none_or(|y| y < -70.0));

    let path = walk(&world, [tip[0], start_y, tip[1]], &TRUNK[1..]);
    let end = path.last().unwrap().0;
    let mut on_tree = 0.0f32;
    for pair in path.windows(2) {
        let ((a, _), (b, grounded)) = (pair[0], pair[1]);
        assert!(grounded, "fell off the trunk at {b:?}");
        if world.floor_object(b[0], b[2], b[1] + 0.5) == Some(TREE_PART) {
            on_tree += (b[0] - a[0]).hypot(b[2] - a[2]);
        }
    }
    println!("walked the trunk {on_tree:.1} m on {TREE}, ended at {end:?}");
    assert!(on_tree > 6.0, "only {on_tree:.1} m on the trunk");
    let last = TRUNK[TRUNK.len() - 1];
    assert!(
        (end[0] - last[0]).hypot(end[2] - last[1]) < 0.6,
        "did not reach the platform: {end:?}"
    );
    assert!(
        bare.floor(end[0], end[2], end[1] + 0.5).is_some(),
        "the platform is map collision"
    );

    // Off the tip's end over the ravine he falls.
    let off = walk(
        &world,
        [-139.5, start_y + 0.05, -124.05],
        &[[-144.0, -124.3]],
    );
    let fell = off
        .iter()
        .position(|(_, g)| !g)
        .expect("walked off the tip without falling");
    let edge = off[fell].0;
    println!("off the tip: airborne at {edge:?}");
    assert!(edge[0] < -141.0, "fell before the tip's end at {edge:?}");
}

/// Genichiro's arena (#18): on his roof, 7.1 m from Wolf's start, stands a torch stand, a placed
/// object (`o116100_4044`, collisionType 1: the player and enemies collide with it). Walking at it
/// he is held at its face and slides around it; his body never overlaps it. The arena built its
/// collision without the objects, so he walked straight through.
#[test]
fn wolf_cannot_walk_through_an_arena_object() {
    use sekiro_sim::collision::RADIUS;
    let cache = sekiro_sim::cache::root();
    if !cache
        .join("maps/m11_01_00_00/object_collision.bin")
        .is_file()
    {
        eprintln!("skipping: m11_01_00_00 has no exported object collision");
        return;
    }
    let spec = sekiro_sim::arena::ArenaSpec::by_name("genichiro").unwrap();
    let arena = sekiro_sim::arena::Arena::load(&cache, spec, false).unwrap();
    let start = arena.floor_at(arena.layout.wolf.position);
    // Toward the stand, 206 degrees from +Z about +Y; knee high it is 7.15 m away.
    let a = 206f32.to_radians();
    let dir = glam::Vec3::new(a.sin(), 0.0, a.cos());
    let hit = arena.world.raycast(start + glam::Vec3::Y * 0.4, dir, 12.0);
    assert!(
        hit.is_some_and(|d| (d - 7.15).abs() < 0.02),
        "the stand 7.15 m ahead: {hit:?}"
    );
    let face = start + dir * 7.15;
    let goal = start + dir * 10.0;
    let path = walk(&arena.world, start.to_array(), &[[goal.x, goal.z]]);
    let nearest = path
        .iter()
        .map(|(p, _)| (glam::Vec3::from(*p) - face).with_y(0.0).length())
        .fold(f32::MAX, f32::min);
    println!("nearest to the stand's face {nearest:.2} m (body radius {RADIUS})");
    assert!(
        nearest > 0.9 * RADIUS,
        "Wolf's centre came {nearest:.2} m from the stand's face: he walked into it"
    );
}

/// The grapple wire's sight line is blocked by placed objects (#18): on Ashina Castle, from the
/// ground at (182.93, 141.98) looking at the pine `o002540_0001`, its hook 27 is marked through
/// the map collision alone but `o113400_3044` stands 2.8 m in front of Wolf, in the way. The
/// wire's world was the map's without objects, so it fired through them.
#[test]
fn the_wire_sight_line_is_blocked_by_an_object() {
    use sekiro_sim::wire::{SIGHT_LIFT, Wire, sight_ray};
    use std::sync::Arc;
    let Some((dir, _)) = load("m11_01_00_00") else {
        return;
    };
    if !dir.join("grapple.json").is_file() {
        return;
    }
    let cache = sekiro_sim::cache::root();
    let feet = glam::Vec3::new(182.92863, -20.94238, 141.97514);
    let body = Body {
        position: feet.to_array(),
        yaw: 115f32.to_radians(),
        ..Body::default()
    };
    // On the pine at (164.80, -18.70, 150.43).
    const HOOK: usize = 27;
    let search = |world: CollisionWorld| {
        let mut wire = Wire::load(&cache, &dir, Arc::new(world)).unwrap();
        wire.search(&body, body.yaw, 0.25, false);
        (wire, HOOK)
    };
    let (open, hook) = search(bare(&dir));
    assert_eq!(open.points[hook].source_part, "o002540_0001");
    assert_eq!(
        open.candidate.as_ref().map(|t| t.index),
        Some(hook),
        "through the map collision alone the hook is marked"
    );
    let (wire, _) = search(world(&dir));
    let point = glam::Vec3::from(wire.points[hook].position.unwrap());
    let chest = feet + glam::Vec3::Y * SIGHT_LIFT;
    let (ray, length) = sight_ray(chest, point);
    let world = wire.collision.as_ref().unwrap();
    let blocked = world.raycast(chest, ray, length);
    assert!(
        blocked.is_some_and(|d| (d - 2.83).abs() < 0.05),
        "the object 2.8 m ahead blocks the sight line: {blocked:?}"
    );
    assert_ne!(
        wire.candidate.as_ref().map(|t| t.index),
        Some(hook),
        "the hook behind the object is not marked"
    );
}

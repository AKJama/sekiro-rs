//! Placed objects' collision (`cache/maps/<id>/object_collision.bin`, `docs/OBJECT-COLLISION.md`)
//! over the Ashina Outskirts and Ashina Castle blocks: how many objects and triangles each map
//! carries, by `ObjectParam.collisionType`, and the rideable pine over the Outskirts start ravine,
//! which the player must be able to walk along after the grapple, and whose fall-prevention
//! walls keep him on it. Skips when the maps have not been exported.

use std::collections::BTreeMap;
use std::path::PathBuf;

use sekiro_formats::hknp::ObjectCollision;
use sekiro_sim::body::{Body, BodyInput};
use sekiro_sim::collision::CollisionWorld;
use sekiro_sim::duel::Duel;
use sekiro_sim::hit_filter::{self, FALL_ASSIST_MOVE};
use sekiro_sim::{Buttons, InputFrame};

fn load(id: &str) -> Option<(PathBuf, ObjectCollision)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../cache/maps")
        .join(id);
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

/// The block's own map collision plus every placed object's. The play world
/// (`map_world::collision_world` in the game crate, tested there on the same pine) adds the
/// connected blocks too and leaves out the objects the story switches off.
fn world(dir: &std::path::Path) -> CollisionWorld {
    let mut world = CollisionWorld::load_map(dir).unwrap();
    world
        .add_block_objects(dir, "m11_00_00_00", |_| true.into())
        .unwrap();
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

/// The fall-prevention variant of the pine placed with it (`o002501`, 落下防止): walls along
/// both sides of the trunk's top and across its tip, in group 92.
const WALLS: &str = "o002501_0005";

/// Run speed, metres per 60 Hz tick.
const RUN: f32 = 0.08;
/// Walk speed, metres per 60 Hz tick.
const WALK: f32 = 0.025;

/// Walks a body through `waypoints` at run speed (forward root motion), returning every tick's
/// feet and whether it was grounded.
fn walk(world: &CollisionWorld, start: [f32; 3], waypoints: &[[f32; 2]]) -> Vec<([f32; 3], bool)> {
    walk_at(world, start, waypoints, RUN, 0)
}

/// [`walk`] at `speed` metres per tick, asking for the fall-prevention walls in `assist` as the
/// player script's `act(3016)` does on the ground.
fn walk_at(
    world: &CollisionWorld,
    start: [f32; 3],
    waypoints: &[[f32; 2]],
    speed: f32,
    assist: u8,
) -> Vec<([f32; 3], bool)> {
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
                    root_motion: [0.0, 0.0, -speed, 0.0],
                    fall_assist: assist,
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
    let bare = CollisionWorld::load_map(&dir).unwrap();
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

/// Walking square to the trunk at walk and run speed, the fall-prevention walls
/// (`o002501_0005`, collisionType 6, group 92) hold Wolf on its top: toward its left side
/// (seen from the tip) all along it, both ways near the tip, and at the tip's end. Without
/// them he walks off and falls. Toward the right side past the tip the walls stand where the
/// trunk is steeper than the 50 degree slope limit under a point foot, so this simulation
/// still drops him there (`docs/OBJECT-COLLISION.md`, Unresolved); the exe's capsule rests on
/// that flank.
#[test]
fn the_fall_prevention_walls_hold_wolf_on_the_trunk() {
    let Some((dir, objects)) = load("m11_00_00_00") else {
        return;
    };
    let walls = objects.parts.iter().find(|p| p.name == WALLS).expect(WALLS);
    assert_eq!(walls.collision_type, 6);
    assert_eq!(hit_filter::object_group(6), 92);
    assert_eq!(hit_filter::object_mask(6), FALL_ASSIST_MOVE);
    let world = world(&dir);
    // Halfway along each stretch of the trunk over the ravine, walk square to it.
    for (i, pair) in TRUNK[..6].windows(2).enumerate() {
        let (a, b) = (glam::Vec2::from(pair[0]), glam::Vec2::from(pair[1]));
        let mid = (a + b) * 0.5;
        let left = (b - a).normalize().perp();
        let y = world
            .floor(mid.x, mid.y, -40.0)
            .expect("a floor on the trunk");
        let start = [mid.x, y, mid.y];
        let sides: &[glam::Vec2] = if i == 0 { &[left, -left] } else { &[left] };
        for &side in sides {
            let target = (mid + side * 3.0).to_array();
            for speed in [WALK, RUN] {
                let path = walk_at(&world, start, &[target], speed, FALL_ASSIST_MOVE);
                let (end, _) = *path.last().unwrap();
                let lowest = path.iter().map(|p| p.0[1]).fold(f32::MAX, f32::min);
                assert!(
                    lowest > y - 0.5,
                    "walked off toward {target:?} at {speed}: down to {lowest}"
                );
                assert_eq!(
                    world.floor_object(end[0], end[2], end[1] + 0.5),
                    Some(TREE_PART),
                    "{end:?}"
                );
                let free = walk_at(&world, start, &[target], speed, 0);
                let (end, _) = *free.last().unwrap();
                assert!(
                    free.iter().any(|&(_, grounded)| !grounded)
                        && world.floor_object(end[0], end[2], end[1] + 0.5) != Some(TREE_PART),
                    "without the walls he should walk off from {start:?} toward {target:?}"
                );
            }
        }
    }

    // Off the tip's end over the ravine, the end wall holds him too.
    let y = world.floor(-139.5, -124.05, -40.0).unwrap();
    let off = walk_at(
        &world,
        [-139.5, y, -124.05],
        &[[-144.0, -124.3]],
        RUN,
        FALL_ASSIST_MOVE,
    );
    let (end, _) = *off.last().unwrap();
    assert!(off.iter().all(|&(_, g)| g), "walked off the tip");
    assert!(end[0] > -141.5, "past the tip's end at {end:?}");
    assert_eq!(
        world.floor_object(end[0], end[2], end[1] + 0.5),
        Some(TREE_PART)
    );

    // The walls are one-sided: from beside the trunk, where the grapple can set him down, he
    // walks in onto it. That also checks their facing: they face the trunk in Bevy space, since
    // the export's mirror reverses the winding (in the game's space they face out).
    let beside = glam::Vec3::new(-137.09, -43.55, -121.83);
    let path = walk_at(
        &world,
        beside.to_array(),
        &[[-136.3, -121.0], TRUNK[7]],
        RUN,
        FALL_ASSIST_MOVE,
    );
    let (end, _) = *path.last().unwrap();
    assert!(
        (end[0] - TRUNK[7][0]).hypot(end[2] - TRUNK[7][1]) < 0.6,
        "did not walk in from beside the trunk: {end:?}"
    );
}

/// With the player script driving Wolf, his ground states ask for the assist every frame
/// (`act(3016, 0)`), so pushing the stick toward the trunk's side keeps him on it; a jump
/// (whose states do not ask) carries him over the wall and off.
#[test]
fn wolf_stays_on_the_trunk_until_he_jumps_off() {
    let Some((dir, _)) = load("m11_00_00_00") else {
        return;
    };
    let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../cache");
    if !cache.join("raw/chr/c1010.behbnd.d/c9997.hkx").exists() {
        return;
    }
    let mut duel = Duel::load(&cache, 4.0).unwrap();
    duel.enemy_active = false;
    let world = world(&dir);
    let y = world.floor(-136.0, -120.0, -40.0).unwrap();
    let start = [-136.0, y, -120.0];
    duel.player.ground = Box::new(world);
    duel.player.body.position = start;
    // The camera looks along -X, so the stick pushes Wolf west, square to the trunk.
    let west = |f: usize, jump: bool| InputFrame {
        move_stick: if f >= 20 { [0.0, 1.0] } else { [0.0; 2] },
        camera_yaw: std::f32::consts::FRAC_PI_2,
        buttons: Buttons {
            jump,
            ..Buttons::default()
        },
    };
    let mut asked = false;
    for f in 0..140 {
        duel.step(&west(f, false), 1.0 / 60.0);
        asked |= duel.player.behavior.env.fall_assist.get() & FALL_ASSIST_MOVE != 0;
        assert!(
            duel.player.body.grounded,
            "walked off at {:?}",
            duel.player.body.position
        );
    }
    let held = duel.player.body.position;
    println!("held at {held:?}");
    assert!(
        asked,
        "the player script never asked for the fall-prevention assist"
    );
    assert!(
        held[0] < start[0] - 0.8,
        "did not walk to the side: {held:?}"
    );

    let mut free = false;
    for f in 0..90 {
        duel.step(&west(20 + f, f == 0), 1.0 / 60.0);
        free |= !duel.player.body.grounded && duel.player.behavior.env.fall_assist.get() == 0;
    }
    let end = duel.player.body.position;
    println!("jumped to {end:?}");
    assert!(free, "the jump kept asking for the assist");
    assert!(
        end[0] < held[0] - 2.0,
        "the jump did not carry him over the wall: {end:?}"
    );
    assert!(
        end[1] < held[1] - 3.0,
        "he did not leave the trunk: {end:?}"
    );
}

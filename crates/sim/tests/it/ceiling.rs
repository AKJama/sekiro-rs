//! Jumps under the low roof of the shack by m11_00's start 0 (#160): the rise must stop at the
//! ceiling, or the head goes into the roof and the fall lands on the roof's underside (the
//! mesh's winding is not consistent, so any flat face is a floor), trapping Wolf and the camera
//! in it. Skips without the exported maps.

use sekiro_formats::hknp::CollisionMesh;
use sekiro_sim::body::{Body, BodyInput};
use sekiro_sim::collision::{CollisionWorld, HEAD_TOP, MAX_SLOPE_DEGREES, RADIUS};
use sekiro_sim::tae::VelocityChange;
use sekiro_sim::{Buttons, Character, InputFrame};

use crate::movement_helpers::{DT, cache, settle};
use crate::step_cost::play_world;

/// The heights of the faces over `(x, z)` flat enough to stand on, which the collision also
/// takes for ceilings from below.
fn flat_faces(world: &CollisionWorld, x: f32, z: f32) -> impl Iterator<Item = f32> {
    let min_ny = MAX_SLOPE_DEGREES.to_radians().cos();
    world
        .surfaces_at(x, z)
        .into_iter()
        .filter(move |&(_, ny)| ny.abs() >= min_ny)
        .map(|(h, _)| h)
}

/// The floor at `(x, z)` near the shack's floor level and the lowest ceiling over it.
fn floor_and_ceiling(world: &CollisionWorld, x: f32, z: f32) -> (f32, f32) {
    let floor = world.floor(x, z, -45.5).expect("a floor in the shack");
    let ceiling = flat_faces(world, x, z)
        .filter(|&h| h > floor + 0.1)
        .reduce(f32::min)
        .expect("a ceiling over the shack floor");
    (floor, ceiling)
}

/// Wolf's own standing jump where the shack's ceiling is 3.13 m over the floor, under the 3.93 m
/// a free jump's head reaches: his head stops under it and he lands back on the floor (before,
/// he rose into the roof and landed in it, 2.38 m up).
#[test]
fn standing_jump_under_the_shack_ceiling_lands_on_the_floor() {
    let (Some(world), Some(cache)) = (play_world(), cache()) else {
        return;
    };
    let [x, z] = [-123.3, -193.33];
    let (floor, ceiling) = floor_and_ceiling(&world, x, z);
    assert!(
        ceiling - floor < 3.9,
        "the ceiling is {:.2} m up",
        ceiling - floor
    );
    let mut wolf = Character::load_from_cache(&cache).unwrap();
    wolf.ground = Box::new(world.clone());
    wolf.body.position = [x, floor, z];
    settle(&mut wolf);
    let (mut peak, mut airborne) = (f32::MIN, false);
    for i in 0..150 {
        let input = InputFrame {
            buttons: Buttons {
                jump: (0..3).contains(&i),
                ..Buttons::default()
            },
            ..InputFrame::default()
        };
        let report = wolf.tick(&input, DT);
        assert!(
            report.behavior.script_errors.is_empty(),
            "{:?}",
            report.behavior.script_errors
        );
        airborne |= !report.grounded;
        peak = peak.max(report.position[1]);
    }
    assert!(airborne, "never jumped");
    assert!(
        peak + HEAD_TOP <= ceiling + 0.01,
        "head rose to {:.2}, the ceiling is at {ceiling:.2}",
        peak + HEAD_TOP
    );
    let y = wolf.body.position[1];
    assert!(
        wolf.body.grounded && (y - floor).abs() < 0.3,
        "landed at {y:.2}, the floor is at {floor:.2}"
    );
}

/// Running jumps across the shack, in eight directions at the jump's two forward speeds: none
/// ends with a ceiling inside the body (before, 4 of these 16 landed in the roof, 2 m up).
#[test]
fn running_jumps_in_the_shack_stay_under_its_roof() {
    let Some(world) = play_world() else {
        return;
    };
    let [x, z] = [-128.3, -189.08];
    let (floor, _) = floor_and_ceiling(&world, x, z);
    let mut trapped = Vec::new();
    for k in 0..8 {
        for horizontal in [3.0, 6.4] {
            let yaw = k as f32 * std::f32::consts::FRAC_PI_4;
            let mut body = Body {
                position: [x, floor, z],
                yaw,
                fall_top: floor,
                ..Default::default()
            };
            let jump = VelocityChange {
                fall_type: 0,
                horizontal_scale: 0.0,
                vertical_scale: 0.0,
                horizontal,
                vertical: 9.64,
                horizontal_angle: 0.0,
            };
            body.step(
                &BodyInput {
                    velocity_change: Some(jump),
                    ..Default::default()
                },
                DT,
                &*world,
            );
            for _ in 0..120 {
                if body.grounded {
                    break;
                }
                body.step(&BodyInput::default(), DT, &*world);
            }
            let [ex, ey, ez] = body.position;
            if flat_faces(&world, ex, ez).any(|h| h > ey + 0.1 && h < ey + HEAD_TOP - 0.05) {
                trapped.push((yaw.to_degrees().round(), horizontal, body.position));
            }
        }
    }
    assert!(trapped.is_empty(), "landed inside the roof: {trapped:?}");
}

/// A floor at y 0 for z > 0 and a wall at z 0 whose top, `top` m up, runs back over z < 0.
fn wall_world(top: f32) -> CollisionWorld {
    let mut m = CollisionMesh::default();
    for q in [
        [
            [-3.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            [3.0, 0.0, 3.0],
            [-3.0, 0.0, 3.0],
        ],
        [
            [-3.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            [3.0, top, 0.0],
            [-3.0, top, 0.0],
        ],
        [
            [-3.0, top, -3.0],
            [3.0, top, -3.0],
            [3.0, top, 0.0],
            [-3.0, top, 0.0],
        ],
    ] {
        let base = m.vertices.len() as u32;
        m.vertices.extend_from_slice(&q);
        m.indices.extend([0, 1, 2, 0, 2, 3].map(|i| base + i));
        m.materials.extend([0, 0]);
    }
    CollisionWorld::from_mesh(&m)
}

/// A running jump into a wall whose top is within its reach: pressed against the wall as its
/// head sphere's centre passes the top, whose edge is then at the sphere's rim, the body rises
/// as high as by a wall too tall to reach the top of, so ledge grabs and wall jumps still carry
/// it up past the top.
#[test]
fn a_jump_flush_against_a_wall_rises_past_its_top() {
    let top = 2.5;
    let head = HEAD_TOP - RADIUS;
    // The peak height and where the body was as its head's centre passed `top`.
    let rise = |world: &CollisionWorld| {
        let mut body = Body {
            position: [0.0, 0.0, 0.6],
            ..Default::default()
        };
        let jump = VelocityChange {
            fall_type: 0,
            horizontal_scale: 0.0,
            vertical_scale: 0.0,
            horizontal: 3.0,
            vertical: 9.64,
            horizontal_angle: 0.0,
        };
        let mut input = BodyInput {
            velocity_change: Some(jump),
            ..Default::default()
        };
        let (mut peak, mut passing) = (f32::MIN, None);
        for _ in 0..120 {
            body.step(&input, DT, world);
            input = BodyInput::default();
            peak = peak.max(body.position[1]);
            if body.position[1] + head > top {
                passing.get_or_insert(body.position);
            }
        }
        (peak, passing)
    };
    let (peak, passing) = rise(&wall_world(top));
    assert!(
        passing.is_some_and(|p| (p[2] - RADIUS).abs() < 0.01),
        "not pressed against the wall as the head passed its top: {passing:?}"
    );
    let (free, _) = rise(&wall_world(20.0));
    assert!(
        (peak - free).abs() < 1e-3,
        "peaked at {peak}, {free} by a taller wall"
    );
}

//! Jumps under the low roof of the shack by m11_00's start 0 (#160): the rise must stop at the
//! ceiling, or the head goes into the roof and the fall lands on the roof's underside (the
//! mesh's winding is not consistent, so any flat face is a floor), trapping Wolf and the camera
//! in it. Skips without the exported maps.

use sekiro_sim::body::{Body, BodyInput};
use sekiro_sim::collision::{CollisionWorld, HEAD_TOP, MAX_SLOPE_DEGREES};
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

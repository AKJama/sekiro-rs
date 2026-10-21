//! Running jumps at the fence beside the wooden gate near m11_00's start 0 (#170). A cap beam
//! juts from the fence's top over the gate side, and its edge passes over the rim of the head
//! of a body pressed against the fence: the rise must stop under it, or the slide shoves the
//! head out of the beam, through the fence. Skips without the exported maps.

use glam::{Vec2, Vec3};
use sekiro_sim::{Buttons, Character, InputFrame};

use crate::movement_helpers::{DT, cache, settle};
use crate::step_cost::play_world;

/// Two points on the fence's gate-side face.
const FENCE: [Vec2; 2] = [Vec2::new(-140.31, -149.62), Vec2::new(-142.98, -150.39)];

/// How far `p` is on the gate side of the fence's face (negative past it).
fn gate_side(p: [f32; 3]) -> f32 {
    let along = (FENCE[1] - FENCE[0]).normalize();
    let normal = Vec2::new(-along.y, along.x);
    let normal = if normal.y > 0.0 { normal } else { -normal };
    (Vec2::new(p[0], p[2]) - FENCE[0]).dot(normal)
}

/// Wolf sprints at the fence from the gate side and jumps, square on and at 25 degrees (the
/// stick and camera turned together): his feet never cross the fence's face (before, the
/// first of each pair went through and ended about 15 m past it).
#[test]
fn running_jumps_at_the_gate_fence_stay_on_the_gate_side() {
    let (Some(world), Some(cache)) = (play_world(), cache()) else {
        return;
    };
    let mut through = Vec::new();
    for (x, yaw) in [
        (-140.5, 0.0f32),
        (-140.85, 0.0),
        (-143.88, -25.0),
        (-144.23, -25.0),
    ] {
        let mut wolf = Character::load_from_cache(&cache).unwrap();
        wolf.ground = Box::new(world.clone());
        let z = -141.5;
        let feet = Vec3::new(x, world.floor(x, z, -50.0).unwrap(), z);
        wolf.body.position = feet.to_array();
        settle(&mut wolf);
        wolf.snap_yaw(yaw.to_radians());
        let mut worst = f32::MAX;
        for i in 0..200 {
            let input = InputFrame {
                move_stick: [0.0, 1.0],
                camera_yaw: yaw.to_radians(),
                buttons: Buttons {
                    dodge: i > 5,
                    jump: (40..43).contains(&i),
                    ..Buttons::default()
                },
            };
            let report = wolf.tick(&input, DT);
            assert!(
                report.behavior.script_errors.is_empty(),
                "{:?}",
                report.behavior.script_errors
            );
            worst = worst.min(gate_side(report.position));
        }
        if worst < 0.0 {
            through.push((x, yaw, worst, wolf.body.position));
        }
    }
    assert!(through.is_empty(), "went through the fence: {through:?}");
}

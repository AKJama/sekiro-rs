//! Movement regressions adapted from FwLo's MIT RustSekiro src/sim_tests
//! (part03, part07, part09 and part10), limited to the requested smoothing ports.
//! These drive our real HKS graph and clip/param data, not his procedural states.

use glam::Vec3;
use sekiro_sim::duel::Duel;
use sekiro_sim::{Buttons, Character, InputFrame};
use std::path::PathBuf;

const DT: f32 = 1.0 / 60.0;

fn cache() -> Option<PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../cache");
    path.join("raw/chr/c0000.behbnd.d/c0000.hkx")
        .is_file()
        .then_some(path)
}

fn wolf() -> Option<Character> {
    let mut wolf = Character::load_from_cache(&cache()?).unwrap();
    assert!(wolf.settle(DT, 400) < 400);
    Some(wolf)
}

fn movement(x: f32, y: f32) -> InputFrame {
    InputFrame {
        move_stick: [x, y],
        ..Default::default()
    }
}

fn advance(wolf: &mut Character, input: &InputFrame, frames: usize) {
    for _ in 0..frames {
        let report = wolf.tick(input, DT);
        assert!(
            report.behavior.script_errors.is_empty(),
            "{:?}",
            report.behavior.script_errors
        );
    }
}

// part03: releasing_the_stick_at_a_run_stops_in_about_a_third_of_a_metre.
#[test]
fn releasing_the_stick_at_a_run_stops_in_about_a_third_of_a_metre() {
    let Some(mut wolf) = wolf() else { return };
    advance(&mut wolf, &movement(0.0, 1.0), 60);
    let start = Vec3::from(wolf.body.position);
    let mut speed = 0.0;
    for i in 0..40 {
        let before = Vec3::from(wolf.body.position);
        wolf.tick(&InputFrame::default(), DT);
        if i == 5 {
            speed = before.distance(Vec3::from(wolf.body.position)) / DT;
        }
    }
    let slid = start.distance(Vec3::from(wolf.body.position));
    assert!(
        speed < 1.0 && slid < 0.4,
        "speed {speed:.3}, slid {slid:.3}"
    );
}

// part09 + part10: blend momentum, including the outgoing loop wrapping.
#[test]
fn an_attack_out_of_a_run_keeps_momentum_even_at_the_loop_wrap() {
    let Some(mut wolf) = wolf() else { return };
    for wrap in [false, true] {
        advance(&mut wolf, &movement(0.0, 1.0), 90);
        if wrap {
            let mut at_wrap = false;
            for _ in 0..600 {
                let clip = wolf.behavior.runtime.main_clip().unwrap();
                if clip.looping && clip.duration.is_some_and(|d| d - clip.time < 3.0 * DT) {
                    at_wrap = true;
                    break;
                }
                wolf.tick(&movement(0.0, 1.0), DT);
            }
            assert!(at_wrap, "never reached the run loop's wrap");
        }
        let before = Vec3::from(wolf.body.position);
        let mut input = movement(0.0, 1.0);
        input.buttons.attack = true;
        advance(&mut wolf, &input, 6);
        let distance = before.distance(Vec3::from(wolf.body.position));
        assert!(
            wolf.state().starts_with("GroundAttackCombo1"),
            "{}",
            wolf.state()
        );
        assert!(
            (0.26..0.65).contains(&distance),
            "first 0.1 s moved {distance:.3} m"
        );
        advance(&mut wolf, &InputFrame::default(), 100);
    }
}

// part03: sprint_keeps_its_speed_into_the_loop.
#[test]
fn sprint_keeps_its_speed_into_the_loop() {
    let Some(mut wolf) = wolf() else { return };
    let mut start_min = f32::INFINITY;
    let mut loop_min = f32::INFINITY;
    for i in 0..300 {
        let mut input = movement(0.0, 1.0);
        input.buttons.dodge = i >= 90;
        let before = Vec3::from(wolf.body.position);
        wolf.tick(&input, DT);
        let speed = before.distance(Vec3::from(wolf.body.position)) / DT;
        match wolf.state().as_str() {
            "SprintStartFromStep" => start_min = start_min.min(speed),
            "SprintLoop" => loop_min = loop_min.min(speed),
            _ => {}
        }
    }
    assert!(
        start_min.is_finite() && start_min > 5.0,
        "start {start_min}"
    );
    assert!(loop_min.is_finite() && loop_min > 6.0, "loop {loop_min}");
}

#[test]
fn releasing_sprint_with_forward_held_keeps_the_clip_clock_moving() {
    use std::fmt::Write;
    let Some(mut wolf) = wolf() else { return };
    let mut input = movement(0.0, 1.0);
    advance(&mut wolf, &input, 90);
    input.buttons.dodge = true;
    advance(&mut wolf, &input, 120);
    assert_eq!(wolf.state(), "SprintLoop");
    input.buttons.dodge = false;
    let period = wolf.clips.clip("a000_000500").unwrap().duration;
    let budget =
        wolf.tae.blend_in("a000_000450").unwrap() + wolf.tae.blend_in("a000_000460").unwrap() + DT;
    let mut trace = String::from("tick,state,clip,time,root_blend_weight,speed_m_s,blend_in\n");
    let mut previous: Option<f32> = None;
    let mut first_run = None;
    for tick in 0..90 {
        let before = Vec3::from(wolf.body.position);
        let r = wolf.tick(&input, DT);
        let clip = r.animation.as_deref().unwrap();
        let speed = before.distance(Vec3::from(r.position)) / DT;
        writeln!(
            trace,
            "{tick},{},{clip},{:.6},{:.6},{speed:.6},{:.6}",
            wolf.state(),
            r.anim_time,
            r.root_blend_weight,
            r.blend_in
        )
        .unwrap();
        assert!(r.behavior.script_errors.is_empty(), "{trace}");
        assert_eq!(wolf.clips.alias(clip), "a000_000500", "{trace}");
        if let Some(time) = previous {
            let expected = (time + r.advanced) % period;
            assert!(
                (r.anim_time - expected).abs() < 1e-4,
                "displayed run phase reset or stalled, expected {expected}\n{trace}"
            );
        }
        assert!(
            (0.0..12.0).contains(&speed),
            "offset applied as travel\n{trace}"
        );
        previous = Some(r.anim_time);
        if clip == "a000_000500" {
            first_run.get_or_insert((tick + 1) as f32 * DT);
        }
    }
    if let Some(path) = std::env::var_os("SEKIRO_SPRINT_RELEASE_TRACE") {
        std::fs::write(path, &trace).unwrap();
        eprintln!("{trace}");
    }
    assert!(
        first_run.is_some_and(|time| time <= budget),
        "run did not start within authored handover blend time {budget}\n{trace}"
    );
}
// part03: mashed_air_kicks_kick_once_per_jump, including ref 431 additive requests.
#[test]
fn mashed_air_kicks_kick_once_per_jump() {
    let Some(mut wolf) = wolf() else { return };
    let mut kicks = 0;
    let mut previous = String::new();
    let mut airborne = false;
    for i in 0..120 {
        let input = InputFrame {
            buttons: Buttons {
                jump: i % 2 == 0,
                ..Default::default()
            },
            ..Default::default()
        };
        wolf.tick(&input, DT);
        let state = wolf.state();
        if state == "AirKick" && previous != state {
            kicks += 1;
        }
        previous = state;
        airborne |= !wolf.body.grounded;
        if airborne && wolf.body.grounded {
            break;
        }
    }
    assert_eq!(kicks, 1);
}

// part07: bodies block instead of pushing an idle character, on ground and in air.
#[test]
fn approaching_a_body_stops_the_mover_and_its_air_launch() {
    let Some(cache) = cache() else { return };
    for airborne in [false, true] {
        let mut duel = Duel::load(&cache, 3.0).unwrap();
        duel.enemy_active = false;
        duel.enemy.body.yaw = std::f32::consts::PI;
        if airborne {
            duel.player.body.position = [0.0, 0.4, -2.0];
            duel.player.body.grounded = false;
            duel.player.body.velocity = [0.0, 1.0, -6.4];
        }
        let enemy_start = Vec3::from(duel.enemy.body.position);
        for _ in 0..if airborne { 6 } else { 90 } {
            duel.step(&movement(0.0, 1.0), DT);
        }
        assert!(
            enemy_start.distance(Vec3::from(duel.enemy.body.position)) < 0.02,
            "idle enemy shoved"
        );
        if airborne {
            assert!(
                duel.player.body.velocity[2].abs() < 1.0,
                "{:?}",
                duel.player.body.velocity
            );
        }
    }
}

// part03: a_locked_sprint_deflect_turns_to_the_enemy_instead_of_snapping.
#[test]
fn locked_sprint_deflect_turns_at_the_authored_rate() {
    for sx in [1.0, -0.7] {
        let Some(mut wolf) = wolf() else { return };
        wolf.lock_target = Some([0.0, 0.0, -14.0]);
        let mut errors = Vec::new();
        for i in 0..110 {
            let mut input = movement(sx, 0.5);
            input.buttons.dodge = i < 70;
            input.buttons.guard = (60..110).contains(&i);
            wolf.tick(&input, DT);
            if i == 60 {
                assert_eq!(wolf.state(), "SprintToDeflectGuard");
            }
            let error =
                sekiro_sim::input::angle_diff(wolf.body.yaw, wolf.yaw_to([0.0, 0.0, -14.0]));
            errors.push(error.to_degrees().abs());
        }
        assert!(
            errors[60] > 70.0 && errors[70] > 40.0 && errors[90] < 5.0,
            "{errors:?}"
        );
    }
}

#[test]
fn locked_steps_and_jumps_use_the_current_facing() {
    for jump in [false, true] {
        let Some(mut wolf) = wolf() else { return };
        wolf.lock_target = Some([100.0, 0.0, 0.0]);
        let input = InputFrame {
            move_stick: [0.0, 1.0],
            buttons: Buttons {
                jump,
                dodge: !jump,
                ..Default::default()
            },
            ..Default::default()
        };
        wolf.tick(&input, DT);
        assert!(
            wolf.body.yaw.abs() < 15.0f32.to_radians(),
            "snapped to target: {}",
            wolf.body.yaw
        );
        assert!(
            wolf.body.position[0].abs() < 0.01,
            "moved toward target: {:?}",
            wolf.body.position
        );
    }
}

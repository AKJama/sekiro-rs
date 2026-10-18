use glam::Vec3;
use sekiro_sim::{
    Buttons, InputFrame,
    body::{Body, FlatGround},
    collision::CollisionWorld,
    duel::Duel,
    wire::{Point, Search, Variation, Wire},
};
use std::path::PathBuf;

fn cache() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../cache")
}
fn synthetic(distance: f32) -> Wire {
    let mut wire = Wire::default();
    wire.points.push(Point {
        position: Some([0.0, 0.0, -distance]),
        variation: 1,
        model: "synthetic".into(),
        source_part: "point".into(),
    });
    wire.search = Some(Search {
        action_distance_near: 3.0,
        action_distance_far: 15.0,
        find_distance_far: 25.0,
    });
    wire.variations.insert(
        1,
        Variation {
            id: 1,
            target_marker_type: 0,
            extend_wire_action_distance: 0.0,
            recognition_angle_up: 90.0,
            recognition_angle_down: 90.0,
            recognition_angle_left: 90.0,
            recognition_angle_right: 90.0,
            chr_physics_homing_id_offset: 0,
        },
    );
    wire
}
#[test]
fn ranges_angles_and_ranking() {
    let body = Body::default();
    for (distance, selected) in [
        (2.9, false),
        (3.0, true),
        (15.0, true),
        (15.1, true),
        (25.0, true),
        (26.0, false),
    ] {
        let mut wire = synthetic(distance);
        wire.search(&body, 0.0);
        assert_eq!(wire.candidate.is_some(), selected);
    }
    let mut wire = synthetic(10.0);
    wire.search(&body, std::f32::consts::PI);
    assert!(wire.candidate.is_none());
    let mut off_centre = wire.points[0].clone();
    off_centre.position = Some([1.0, 0.0, -5.0]);
    wire.points.insert(0, off_centre);
    wire.search(&body, 0.0);
    assert_eq!(wire.candidate.unwrap().index, 1);
    // A distant, more centred preview must not steal a usable hook (including wire chains).
    let mut wire = synthetic(10.0);
    wire.points[0].position = Some([0.0, 6.0, -10.0]);
    let mut distant = wire.points[0].clone();
    distant.position = Some([0.0, 8.0, -20.0]);
    wire.points.push(distant);
    wire.search(&body, 0.0);
    assert_eq!(wire.candidate.unwrap().index, 0);
}
#[test]
fn marker_distance_boundaries_and_variation_extension() {
    use sekiro_sim::wire::MarkerState::{Distant, Hidden, Ready};
    let search = synthetic(10.0).search.unwrap();
    for (distance, state) in [
        (2.99, Hidden),
        (3.0, Ready),
        (15.0, Ready),
        (20.0, Distant { fill: 0.5 }),
        (25.0, Distant { fill: 0.0 }),
        (25.01, Hidden),
    ] {
        assert_eq!(search.marker_state(distance, 0.0), state);
    }
    assert_eq!(search.marker_state(20.0, 5.0), Ready);
    assert_eq!(search.marker_state(22.5, 5.0), Distant { fill: 0.5 });
    assert_eq!(search.marker_state(25.0, 10.0), Ready);
}
#[test]
fn extracted_counts_transform_and_collision() {
    for (map, objects, edges) in [("m11_01_00_00", 172, 224), ("m11_00_00_00", 237, 364)] {
        let dir = cache().join("maps").join(map);
        if !dir.join("grapple.json").exists() {
            continue;
        }
        let wire = Wire::load(&cache(), &dir).unwrap();
        let names: std::collections::HashSet<_> = wire
            .points
            .iter()
            .filter(|p| p.model.starts_with('o'))
            .map(|p| &p.source_part)
            .collect();
        assert_eq!(names.len(), objects);
        assert_eq!(wire.points.len(), edges);
        if map == "m11_01_00_00" {
            let point = Vec3::from(
                wire.points
                    .iter()
                    .find(|p| p.source_part == "o003610_0014")
                    .unwrap()
                    .position
                    .unwrap(),
            );
            assert!(point.distance(Vec3::new(97.1, 52.8, 251.0)) < 0.1);
            let floor = wire
                .collision
                .as_ref()
                .unwrap()
                .floor(point.x, point.z, point.y + 0.1)
                .unwrap();
            eprintln!("roof point={point:?} floor={floor} gap={}", point.y - floor);
            assert!((point.y - floor).abs() < 2.0);
            let chest = Vec3::new(103.5, 53.995594 + 1.2, 243.6);
            let ray = point - chest;
            eprintln!(
                "roof LOS distance={} hit={:?}",
                ray.length(),
                wire.collision
                    .as_ref()
                    .unwrap()
                    .raycast(chest, ray.normalize(), ray.length())
            );
        }
    }
}
#[test]
fn real_hks_synthetic_ground_shot_and_out_of_range() {
    if !cache().join("raw/chr/c1010.behbnd.d/c9997.hkx").exists() {
        return;
    }
    for distance in [10.0, 20.0, 30.0] {
        let mut duel = Duel::load(&cache(), 4.0).unwrap();
        duel.enemy_active = false;
        duel.player.ground = Box::new(FlatGround::default());
        duel.player.body = Body::default();
        duel.player.wire = synthetic(distance);
        for _ in 0..30 {
            duel.step(&InputFrame::default(), 1.0 / 60.0);
        }
        let press = InputFrame {
            buttons: Buttons {
                grapple: true,
                ..Buttons::default()
            },
            ..InputFrame::default()
        };
        let r = duel.step(&press, 1.0 / 60.0);
        let fired = r
            .player
            .behavior
            .events
            .iter()
            .any(|e| e == "W_GroundWireShoot");
        assert_eq!(fired, distance == 10.0, "{:?}", r.player);
        let mut moved = false;
        for tick in 0..120 {
            let r = duel.step(&InputFrame::default(), 1.0 / 60.0);
            moved |= r
                .player
                .animation
                .as_deref()
                .is_some_and(|a| a.starts_with("a000_20201"));
            if tick % 15 == 0 {
                eprintln!(
                    "tick={tick} anim={:?} pos={:?} events={:?}",
                    r.player.animation, r.player.position, r.player.behavior.events
                );
            }
        }
        if fired {
            assert!(moved);
            assert!(
                Vec3::from(duel.player.body.position).distance(Vec3::new(0.0, 0.0, -10.0)) < 0.5
            );
            assert!(duel.player.body.grounded);
        }
    }
}
#[test]
fn line_of_sight_blocks_wall() {
    use sekiro_formats::hknp::CollisionMesh;
    let mut wire = synthetic(10.0);
    wire.collision = Some(CollisionWorld::from_mesh(&CollisionMesh {
        vertices: vec![[-50.0, -50.0, -5.0], [50.0, -50.0, -5.0], [0.0, 50.0, -5.0]],
        indices: vec![0, 1, 2],
        ..CollisionMesh::default()
    }));
    wire.search(&Body::default(), 0.0);
    assert!(wire.candidate.is_none());
}

#[test]
fn obstruction_during_travel_releases_wire() {
    use sekiro_formats::hknp::CollisionMesh;
    let mut wire = synthetic(10.0);
    let mut body = Body::default();
    wire.search(&body, 0.0);
    assert!(wire.step(
        &mut body,
        Some("a000_202000"),
        None,
        1.0 / 60.0,
        &FlatGround::default()
    ));
    wire.collision = Some(CollisionWorld::from_mesh(&CollisionMesh {
        vertices: vec![[-50.0, -50.0, -5.0], [50.0, -50.0, -5.0], [0.0, 50.0, -5.0]],
        indices: vec![0, 1, 2],
        ..CollisionMesh::default()
    }));
    for _ in 0..60 {
        wire.step(
            &mut body,
            Some("a000_202012"),
            Some((0.0, 0.5)),
            1.0 / 60.0,
            &FlatGround::default(),
        );
    }
    assert!(body.position[2] > -5.0);
    assert!(wire.active.is_none());
    assert!(!body.grounded);
}

#[test]
fn nonrideable_offset_and_release() {
    let mut wire = synthetic(10.0);
    let variation = wire.variations.get_mut(&1).unwrap();
    variation.target_marker_type = 1;
    variation.chr_physics_homing_id_offset = 20;
    wire.offsets.insert(300000020, Vec3::new(0.0, -2.5, 7.5));
    let mut body = Body::default();
    wire.search(&body, 0.0);
    assert_eq!(
        wire.candidate.as_ref().unwrap().arrival,
        Vec3::new(0.0, -2.5, -17.5)
    );
    wire.step(
        &mut body,
        Some("a000_202000"),
        None,
        1.0 / 60.0,
        &FlatGround::default(),
    );
    for _ in 0..31 {
        wire.step(
            &mut body,
            Some("a000_202012"),
            Some((0.0, 0.5)),
            1.0 / 60.0,
            &FlatGround::default(),
        );
    }
    body.velocity[1] = -3.0;
    assert!(body.position[2] < -10.0, "swing-through must pass the hook");
    assert!(wire.perch.is_none());
    assert!(!wire.step(
        &mut body,
        Some("a000_201030"),
        None,
        1.0 / 60.0,
        &FlatGround::default()
    ));
    assert_eq!(body.velocity[1], -3.0);
    assert!(!body.grounded);
}

#[test]
fn castle_shachi_clear_approach_and_blocked_original_spawn() {
    let dir = cache().join("maps/m11_01_00_00");
    if !dir.join("grapple.json").exists() {
        return;
    }
    let mut wire = Wire::load(&cache(), &dir).unwrap();
    let body = Body {
        position: [103.5, 53.995594, 243.6],
        yaw: 139f32.to_radians(),
        ..Body::default()
    };
    wire.search(&body, body.yaw);
    assert!(
        wire.candidate.is_none(),
        "original balcony spawn must not grapple through collision"
    );
    for (position, yaw, positive) in [
        ([104.03038, 53.234768, 254.9751], 60.0f32, true),
        ([103.5, 53.995594, 243.6], -41.0, false),
    ] {
        let mut duel = Duel::load(&cache(), 4.0).unwrap();
        duel.enemy_active = false;
        duel.player.ground = Box::new(CollisionWorld::load(&dir.join("collision.bin")).unwrap());
        duel.player.wire = Wire::load(&cache(), &dir).unwrap();
        duel.player.wire.view_pitch = 0.25;
        duel.player.body = Body {
            position,
            yaw: yaw.to_radians(),
            ..Body::default()
        };
        let script = sekiro_sim::input::parse_action_script(include_str!(
            "../../../game/scripts/grapple_shachi.txt"
        ))
        .unwrap();
        let mut shot = false;
        let mut move_start = false;
        let mut arrival_tick = None;
        let anchor = Vec3::new(97.10218, 52.843166, 250.9751);
        for (tick, mut frame) in script.into_iter().enumerate() {
            frame.camera_yaw = yaw.to_radians();
            let report = duel.step(&frame, 1.0 / 60.0);
            shot |= report
                .player
                .behavior
                .events
                .iter()
                .any(|e| e == "W_GroundWireShoot");
            move_start |= report.player.animation.as_deref() == Some("a000_202012");
            if shot
                && arrival_tick.is_none()
                && Vec3::from(report.player.position).distance(anchor) < 0.01
            {
                arrival_tick = Some(tick);
            }
        }
        if positive {
            assert!(shot && move_start);
            assert_eq!(arrival_tick, Some(140));
            assert!(Vec3::from(duel.player.body.position).distance(anchor) < 0.5);
            assert!(duel.player.body.grounded);
        } else {
            assert!(!shot);
        }
    }
}

#[test]
fn solved_arc_arrives_ten_metres_away_five_up_at_point_eight_three() {
    let mut wire = synthetic(10.0);
    wire.points[0].position = Some([0.0, 5.0, -10.0]);
    let mut body = Body::default();
    wire.search(&body, 0.0);
    wire.step(
        &mut body,
        Some("a000_202000"),
        None,
        1.0 / 60.0,
        &FlatGround::default(),
    );
    let mut apex = 0.0_f32;
    for tick in 0..50 {
        wire.step(
            &mut body,
            Some("a000_202011"),
            Some((0.0, 0.83)),
            1.0 / 60.0,
            &FlatGround::default(),
        );
        apex = apex.max(body.position[1]);
        if tick == 24 {
            assert!(body.position[1] > 2.5);
        }
    }
    let distance = Vec3::from(body.position).distance(Vec3::new(0.0, 5.0, -10.0));
    eprintln!("arc flight=0.83 s distance={distance} apex={apex}");
    assert!(distance < 0.2);
    assert!(apex > 5.0);
}

fn synthetic_duel() -> Option<Duel> {
    if !cache().join("raw/chr/c1010.behbnd.d/c9997.hkx").exists() {
        return None;
    }
    let mut duel = Duel::load(&cache(), 4.0).unwrap();
    duel.use_simple_ai();
    duel.enemy_active = false;
    duel.player.ground = Box::new(FlatGround::default());
    duel.player.body = Body::default();
    duel.player.wire = synthetic(10.0);
    for _ in 0..30 {
        duel.step(&InputFrame::default(), 1.0 / 60.0);
    }
    Some(duel)
}

#[test]
fn jump_fires_air_wire_and_holds_during_905() {
    let Some(mut duel) = synthetic_duel() else {
        return;
    };
    let jump = InputFrame {
        buttons: Buttons {
            jump: true,
            ..Buttons::default()
        },
        ..InputFrame::default()
    };
    duel.step(&jump, 1.0 / 60.0);
    let mut fired = false;
    let mut held: Option<f32> = None;
    for tick in 0..180 {
        let input = InputFrame {
            buttons: Buttons {
                grapple: tick == 24,
                ..Buttons::default()
            },
            ..InputFrame::default()
        };
        let r = duel.step(&input, 1.0 / 60.0);
        if tick % 10 == 0 {
            eprintln!(
                "air tick={tick} anim={:?} pos={:?} events={:?}",
                r.player.animation, r.player.position, r.player.behavior.events
            );
        }
        fired |= r
            .player
            .behavior
            .events
            .iter()
            .any(|e| e == "W_AirWireShoot");
        if r.player.animation.as_deref() == Some("a000_202100") && r.player.anim_time < 0.49 {
            if let Some(y) = held {
                assert!((r.player.position[1] - y).abs() < 0.001);
            }
            held = Some(r.player.position[1]);
        }
    }
    assert!(fired && held.is_some());
    assert!(Vec3::from(duel.player.body.position).distance(Vec3::new(0.0, 0.0, -10.0)) < 0.5);
}

#[test]
fn wire_fall_chains_to_a_second_synthetic_hook() {
    let Some(mut duel) = synthetic_duel() else {
        return;
    };
    // Shoot again during the first arc, before the ridden point becomes a perch.
    duel.player.wire.points[0].position = Some([0.0, 6.0, -10.0]);
    let mut second = duel.player.wire.points[0].clone();
    second.position = Some([0.0, 8.0, -20.0]);
    second.source_part = "second".into();
    duel.player.wire.points.push(second);
    let mut chain = false;
    let mut shot = false;
    for tick in 0..240 {
        let in_fall = duel.player.wire.active.is_some() && duel.player.body.position[2] < -8.0;
        let press = tick == 0 || (in_fall && !shot);
        if in_fall && !shot {
            shot = true;
        }
        let r = duel.step(
            &InputFrame {
                buttons: Buttons {
                    grapple: press,
                    ..Buttons::default()
                },
                ..InputFrame::default()
            },
            1.0 / 60.0,
        );
        chain |= r
            .player
            .animation
            .as_deref()
            .is_some_and(|a| matches!(a, "a000_202600" | "a000_202700"));
        if tick % 15 == 0 {
            eprintln!(
                "chain tick={tick} anim={:?} pos={:?} events={:?}",
                r.player.animation, r.player.position, r.player.behavior.events
            );
        }
    }
    assert!(chain);
    assert!(duel.player.body.position[2] < -18.0);
}

#[test]
fn castle_roof_chain_route() {
    let dir = cache().join("maps/m11_01_00_00");
    let Some(mut duel) = synthetic_duel() else {
        return;
    };
    duel.player.ground = Box::new(CollisionWorld::load(&dir.join("collision.bin")).unwrap());
    duel.player.wire = Wire::load(&cache(), &dir).unwrap();
    duel.player.wire.view_pitch = 0.25;
    duel.player.body = Body {
        position: [104.03038, 53.234768, 254.9751],
        yaw: 60f32.to_radians(),
        ..Body::default()
    };
    let script = sekiro_sim::input::parse_action_script(include_str!(
        "../../../game/scripts/grapple2_chain.txt"
    ))
    .unwrap();
    let mut chain = false;
    for (tick, mut frame) in script.into_iter().enumerate() {
        frame.camera_yaw = 60f32.to_radians();
        let r = duel.step(&frame, 1.0 / 60.0);
        chain |= r
            .player
            .animation
            .as_deref()
            .is_some_and(|a| matches!(a, "a000_202600" | "a000_202700"));
        if tick % 15 == 0 || tick == 120 {
            eprintln!(
                "roof chain tick={tick} anim={:?} pos={:?} candidate={:?} active={:?} events={:?}",
                r.player.animation,
                r.player.position,
                duel.player
                    .wire
                    .candidate
                    .as_ref()
                    .map(|t| &duel.player.wire.points[t.index].source_part),
                duel.player
                    .wire
                    .active
                    .as_ref()
                    .map(|t| &duel.player.wire.points[t.index].source_part),
                r.player.behavior.events
            );
        }
    }
    assert!(chain);
    assert!(duel.player.body.grounded);
    assert!(
        Vec3::from(duel.player.body.position).distance(Vec3::new(93.31853, 53.092392, 254.30566))
            < 0.6
    );
}

#[test]
fn third_hook_uses_wire_move_wire_two_shoot() {
    let Some(mut duel) = synthetic_duel() else {
        return;
    };
    for (distance, height) in [(10.0, 6.0), (20.0, 8.0), (30.0, 10.0)] {
        let mut point = duel.player.wire.points[0].clone();
        point.position = Some([0.0, height, -distance]);
        if distance == 10.0 {
            duel.player.wire.points[0] = point;
        } else {
            duel.player.wire.points.push(point);
        }
    }
    let mut second = false;
    let mut third = false;
    let mut wire_two = false;
    for tick in 0..300 {
        let z = duel.player.body.position[2];
        let press = if tick == 0 {
            true
        } else if z < -8.0 && !second {
            second = true;
            true
        } else if z < -16.0 && !third {
            third = true;
            true
        } else {
            false
        };
        let r = duel.step(
            &InputFrame {
                buttons: Buttons {
                    grapple: press,
                    ..Buttons::default()
                },
                ..InputFrame::default()
            },
            1.0 / 60.0,
        );
        wire_two |= r.player.animation.as_deref() == Some("a000_202700");
        if press {
            eprintln!(
                "third tick={tick} anim={:?} events={:?}",
                r.player.animation, r.player.behavior.events
            );
        }
    }
    assert!(wire_two);
    assert!(duel.player.body.position[2] < -28.0);
}

#[test]
fn wire_attack_and_guard_cancel_flight_through_hks() {
    for guard in [false, true] {
        let Some(mut duel) = synthetic_duel() else {
            return;
        };
        duel.player.behavior.env.unlocked.insert(14);
        duel.player.behavior.env.unlocked.insert(15);
        duel.player.wire.points[0].position = Some([0.0, 6.0, -10.0]);
        let expected = if guard {
            "W_WireMoveDeflectGuardStart"
        } else {
            "W_WireFallAttackCombo1"
        };
        let mut cancelled = false;
        for tick in 0..160 {
            let r = duel.step(
                &InputFrame {
                    buttons: Buttons {
                        grapple: tick == 0,
                        attack: !guard && tick == 90,
                        guard: guard && (90..130).contains(&tick),
                        ..Buttons::default()
                    },
                    ..InputFrame::default()
                },
                1.0 / 60.0,
            );
            cancelled |= r.player.behavior.events.iter().any(|e| e == expected);
            if guard && (tick % 10 == 0 || !r.player.behavior.events.is_empty()) {
                eprintln!(
                    "guard tick={tick} anim={:?} flags={:?} events={:?}",
                    r.player.animation, r.player.action_flags, r.player.behavior.events
                );
            }
        }
        assert!(cancelled, "{expected} did not fire");
        assert!(duel.player.wire.active.is_none());
    }
}

#[test]
fn fast_wire_fall_uses_land_ready_before_land_wire_move() {
    let Some(mut duel) = synthetic_duel() else {
        return;
    };
    duel.player.wire.points[0].position = Some([0.0, 10.0, -10.0]);
    duel.player
        .wire
        .variations
        .get_mut(&1)
        .unwrap()
        .target_marker_type = 1;
    let mut ready = false;
    let mut landed = false;
    for tick in 0..360 {
        let r = duel.step(
            &InputFrame {
                buttons: Buttons {
                    grapple: tick == 0,
                    ..Buttons::default()
                },
                ..InputFrame::default()
            },
            1.0 / 60.0,
        );
        ready |= r.player.animation.as_deref() == Some("a000_202045");
        landed |= r.player.animation.as_deref() == Some("a000_202035");
        if r.player.clip_started {
            eprintln!(
                "landing tick={tick} anim={:?} grounded={} vy={} events={:?}",
                r.player.animation,
                r.player.grounded,
                duel.player.behavior.env.vertical_velocity,
                r.player.behavior.events
            );
        }
    }
    assert!(ready && landed);
}

#[test]
fn perch_support_ends_outside_point_six_metres() {
    use sekiro_sim::{body::Ground, wire::PerchGround};
    let terrain = FlatGround::default();
    let ground = PerchGround {
        terrain: &terrain,
        perch: Some(Vec3::new(0.0, 5.0, 0.0)),
    };
    assert_eq!(ground.floor(0.59, 0.0, 6.0), Some(5.0));
    assert_eq!(ground.floor(0.61, 0.0, 6.0), Some(0.0));
    assert_eq!(ground.floor(0.0, 0.0, 4.0), Some(0.0));
}

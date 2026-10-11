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
        (15.1, false),
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
    for distance in [10.0, 30.0] {
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
        vertices: vec![[-5.0, -5.0, -5.0], [5.0, -5.0, -5.0], [0.0, 5.0, -5.0]],
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
        vertices: vec![[-5.0, -5.0, -5.0], [5.0, -5.0, -5.0], [0.0, 5.0, -5.0]],
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
            "../../game/scripts/grapple_shachi.txt"
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
            assert_eq!(arrival_tick, Some(90));
            assert!(Vec3::from(duel.player.body.position).distance(anchor) < 0.5);
            assert!(duel.player.body.grounded);
        } else {
            assert!(!shot);
        }
    }
}

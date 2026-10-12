use glam::Vec3;
use sekiro_sim::body::{Body, Ground};
use sekiro_sim::docking::{Docking, MapEdge};
use sekiro_sim::duel::Duel;
use sekiro_sim::{Buttons, InputFrame};
use std::path::PathBuf;

struct Step;
impl Ground for Step {
    fn height(&self, _: f32, z: f32) -> Option<f32> {
        Some(if z >= 0.0 { 0.0 } else { -4.0 })
    }
}
fn synthetic() -> Docking {
    let mut docking = Docking::default();
    docking.edges = vec![MapEdge {
        kind: 2,
        variation: 0,
        start: [-2.0, 0.0, 0.0],
        end: [2.0, 0.0, 0.0],
        source_part: "synthetic".into(),
        chain: 0,
    }];
    docking
}
fn cache() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../cache")
}

#[test]
fn closest_reach_facing_and_ends() {
    let mut docking = synthetic();
    assert_eq!(
        docking.edges[0].closest(Vec3::new(4.0, 9.0, 2.0)),
        (Vec3::new(2.0, 0.0, 0.0), 1.0)
    );
    let mut body = Body {
        position: [0.0, 0.0, 0.4],
        ..Body::default()
    };
    docking.search(&body, &Step);
    let target = docking.candidate.as_ref().unwrap();
    assert_eq!(target.outward, -Vec3::Z);
    assert_eq!(target.floor, Vec3::new(0.0, 0.0, 0.25));
    body.yaw = std::f32::consts::PI;
    docking.search(&body, &Step);
    assert!(docking.candidate.is_none());
    body.yaw = 0.0;
    body.position[2] = 1.1;
    docking.search(&body, &Step);
    assert!(docking.candidate.is_none());
    body.position = [1.99, 0.0, 0.4];
    docking.search(&body, &Step);
    assert_eq!(docking.candidate.unwrap().end, 2);
}

#[test]
fn researched_edge_has_upper_floor_and_outward_drop() {
    let dir = cache().join("maps/m11_01_00_00");
    if !dir.join("edges.json").exists() || !dir.join("collision.bin").exists() {
        return;
    }
    let docking = Docking::load(&dir.join("edges.json")).unwrap();
    let world = sekiro_sim::collision::CollisionWorld::load(&dir.join("collision.bin")).unwrap();
    let mut found = false;
    for (i, edge) in docking
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.source_part == "h019900" && e.chain == 7 && e.kind == 2)
    {
        let point = (Vec3::from(edge.start) + Vec3::from(edge.end)) * 0.5;
        let target = docking.target(i, point, &world);
        eprintln!(
            "research edge {:?} -> {:?}; target={target:?}",
            edge.start, edge.end
        );
        if let Some(t) = target {
            assert!((t.floor.y - point.y).abs() <= 0.3);
            found = true;
        }
    }
    assert!(found, "research edge does not sit on a collision ledge");
}

#[test]
fn hks_ground_hang_climb_and_drop() {
    if !cache().join("raw/chr/c1010.behbnd.d/c9997.hkx").exists() {
        return;
    }
    let mut duel = Duel::load(&cache(), 4.0).unwrap();
    duel.enemy_active = false;
    duel.player.ground = Box::new(Step);
    duel.player.docking = synthetic();
    duel.player.body.position = [0.0, 0.0, 0.4];
    let dt = 1.0 / 60.0;
    for _ in 0..30 {
        duel.step(&InputFrame::default(), dt);
    }
    let press = InputFrame {
        buttons: Buttons {
            action: true,
            ..Buttons::default()
        },
        ..InputFrame::default()
    };
    let r = duel.step(&press, dt);
    assert!(
        r.player
            .behavior
            .events
            .iter()
            .any(|e| e.starts_with("W_StandToHang")),
        "{:?}",
        r.player
    );
    let mut idle = false;
    for _ in 0..180 {
        let r = duel.step(&InputFrame::default(), dt);
        idle |= r.player.animation.as_deref() == Some("a000_020000");
    }
    assert!(idle, "did not reach HangIdle");
    assert!((duel.player.body.position[1] + 1.5).abs() < 0.01);
    let r = duel.step(&press, dt);
    assert_eq!(r.player.animation.as_deref(), Some("a000_217500"));
    for _ in 0..180 {
        duel.step(&InputFrame::default(), dt);
    }
    assert!(duel.player.body.grounded);
    assert!(Vec3::from(duel.player.body.position).distance(Vec3::new(0.0, 0.0, 0.25)) < 0.01);
    duel.player.body.position = [0.0, 0.0, 0.4];
    duel.player.body.yaw = 0.0;
    duel.step(&press, dt);
    for _ in 0..180 {
        duel.step(&InputFrame::default(), dt);
    }
    let drop = InputFrame {
        buttons: Buttons {
            dodge: true,
            ..Buttons::default()
        },
        ..InputFrame::default()
    };
    let r = duel.step(&drop, dt);
    assert!(
        r.player
            .behavior
            .events
            .iter()
            .any(|e| e == "W_HangToFreeFall")
    );
    assert!(duel.player.docking.active.is_none());
    assert!(!duel.player.body.grounded);
}

#[test]
fn castle_start_two_route() {
    let dir = cache().join("maps/m11_01_00_00");
    if !dir.join("edges.json").exists() {
        return;
    }
    let mut duel = Duel::load(&cache(), 4.0).unwrap();
    duel.enemy_active = false;
    let world = sekiro_sim::collision::CollisionWorld::load(&dir.join("collision.bin")).unwrap();
    let y = world.floor(83.307, 243.649, 31.0).unwrap();
    duel.player.ground = Box::new(world);
    duel.player.docking = Docking::load(&dir.join("edges.json")).unwrap();
    duel.player.body.position = [83.307, y, 243.649];
    duel.player.body.yaw = 133.216f32.to_radians();
    let script =
        std::fs::read_to_string(cache().join("../crates/game/scripts/m11_01_ledge_hang.txt"))
            .unwrap();
    let frames = sekiro_sim::input::parse_action_script(&script).unwrap();
    let mut entered = false;
    let mut idle = false;
    let mut climbed = false;
    let mut edge_point = None;
    for (step, mut frame) in frames.into_iter().enumerate() {
        frame.camera_yaw += 133.216f32.to_radians();
        let r = duel.step(&frame, 1.0 / 60.0);
        if step == 150 {
            let bones = duel.player.bone_source_matrices();
            let world = duel.player.source_to_world();
            for (i, bone) in duel
                .player
                .skeleton
                .bones
                .iter()
                .enumerate()
                .filter(|(_, b)| b.name.ends_with("Hand"))
            {
                let hand = world.transform_point3(bones[i].transform_point3(Vec3::ZERO));
                let floor = duel
                    .player
                    .ground
                    .floor(hand.x, hand.z, hand.y + 0.3)
                    .unwrap();
                assert!(
                    (hand.y - floor).abs() < 0.1,
                    "{} floats above the ledge: {hand:?} floor={floor}",
                    bone.name
                );
                eprintln!("hang hand {} at {:?}", bone.name, hand);
            }
        }
        entered |= r
            .player
            .behavior
            .events
            .iter()
            .any(|e| e.starts_with("W_StandToHang"));
        idle |= r.player.animation.as_deref() == Some("a000_020000");
        climbed |= r.player.animation.as_deref() == Some("a000_217500");
        if let Some(t) = &duel.player.docking.active {
            edge_point = Some(t.point);
        }
        if step % 10 == 0
            || !r.player.behavior.events.is_empty()
            || duel.player.docking.candidate.is_some()
        {
            eprintln!(
                "route step {step} pos={:?} env3044={} candidate={:?} anim={:?} events={:?}",
                duel.player.body.position,
                duel.player.behavior.env.hang_candidate,
                duel.player.docking.candidate,
                r.player.animation,
                r.player.behavior.events
            );
        }
    }
    assert!(entered && idle && climbed);
    assert!(duel.player.body.grounded);
    assert!(Vec3::from(duel.player.body.position).distance(edge_point.unwrap()) < 0.3);
}

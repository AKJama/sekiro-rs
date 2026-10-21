use glam::Vec3;
use sekiro_sim::body::{Body, Ground, root_motion_world};
use sekiro_sim::docking::{Docking, MapEdge};
use sekiro_sim::duel::Duel;
use sekiro_sim::{Buttons, InputFrame};
use std::path::PathBuf;

struct Wall;
impl Ground for Wall {
    fn height(&self, _: f32, _: f32) -> Option<f32> {
        Some(0.0)
    }
    fn slide(&self, feet: [f32; 3], delta: [f32; 2]) -> [f32; 2] {
        [delta[0], (feet[2] + delta[1]).max(0.3) - feet[2]]
    }
}
fn synthetic(variation: u8) -> Docking {
    Docking::new(vec![MapEdge {
        kind: 3,
        variation,
        start: [-2.0, 0.4, 0.0],
        end: [2.0, 0.4, 0.0],
        source_part: "wall".into(),
        chain: 1,
    }])
}
#[test]
fn cover_search_facing_variations_and_slide_clamps() {
    for variation in [0, 1, 2, 3, 10, 11, 12, 13] {
        let mut docking = synthetic(variation);
        let mut body = Body {
            position: [0.0, 0.0, 0.7],
            ..Body::default()
        };
        docking.search(&body, &Wall);
        assert!(docking.candidate.is_some());
        assert!(docking.cover_step(&mut body, Some("a000_218103"), None, 0.0, &Wall));
        assert!(body.grounded);
        assert!((body.position[2] - 0.7).abs() < 1e-6);
        let anchored = body.position;
        for yaw in [0.0, std::f32::consts::PI] {
            docking.cover_step(&mut body, Some("a000_021200"), Some(yaw), 0.1, &Wall);
            assert_eq!(
                body.position, anchored,
                "wall-normal input slid along the wall"
            );
        }
        // Ends are named facing the wall: with Wolf's back to it (facing +Z), -X is left.
        for (yaw, x, end) in [
            (std::f32::consts::FRAC_PI_2, -2.0, 1),
            (-std::f32::consts::FRAC_PI_2, 2.0, 2),
        ] {
            for _ in 0..100 {
                docking.cover_step(&mut body, Some("a000_021200"), Some(yaw), 0.1, &Wall);
            }
            assert!((body.position[0] - x).abs() < 1e-6);
            assert_eq!(docking.active.as_ref().unwrap().end, end);
        }
        assert!(!docking.cover_step(&mut body, Some("a000_218490"), None, 0.0, &Wall));
        assert!(docking.active.is_none() && body.grounded);
        body.yaw = std::f32::consts::PI;
        docking.search(&body, &Wall);
        assert!(docking.candidate.is_none());
    }
}
#[test]
fn hks_cover_enter_idle_slide_peek_and_exit() {
    use rayon::prelude::*;
    // Each case builds its own duel, so they run side by side.
    let cases = [
        (0, false),
        (1, false),
        (2, false),
        (3, false),
        (10, false),
        (11, false),
        (12, false),
        (13, false),
        (1, true),
    ];
    cases
        .par_iter()
        .for_each(|&(variation, away)| hks_cover(variation, away));
}

const DT: f32 = 1.0 / 60.0;

fn press() -> InputFrame {
    InputFrame {
        buttons: Buttons {
            action: true,
            ..Buttons::default()
        },
        ..InputFrame::default()
    }
}

/// Wolf docked into cover on the synthetic wall of this variation, idle, with his back to it
/// (facing +Z) and the camera behind him, so stick left ([`shimmy`]) slides him to +X.
fn docked(variation: u8) -> Option<Duel> {
    let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../cache");
    if !cache.join("raw/chr/c1010.behbnd.d/c9997.hkx").exists() {
        return None;
    }
    let mut duel = Duel::load(&cache, 4.0).unwrap();
    duel.enemy_active = false;
    duel.player.ground = Box::new(Wall);
    duel.player.docking = synthetic(variation);
    duel.player.body.position = [0.0, 0.0, 0.7];
    for _ in 0..30 {
        duel.step(&InputFrame::default(), DT);
    }
    let r = duel.step(&press(), DT);
    assert!(
        r.player
            .behavior
            .events
            .iter()
            .any(|e| e.starts_with("W_StandToCover2")),
        "{:?}",
        r.player
    );
    let mut idle = false;
    for _ in 0..180 {
        let r = duel.step(&InputFrame::default(), DT);
        idle |= r.player.animation.as_deref() == Some("a000_021000");
    }
    assert!(idle, "variation {variation}: no cover idle");
    assert!(duel.player.docking.active.is_some(), "not in cover");
    Some(duel)
}

/// The stick along the wall with the camera behind Wolf: -1 is left, 1 is right.
fn shimmy(stick_x: f32) -> InputFrame {
    InputFrame {
        move_stick: [stick_x, 0.0],
        camera_yaw: std::f32::consts::PI,
        ..InputFrame::default()
    }
}

fn hks_cover(variation: u8, away: bool) {
    let Some(mut duel) = docked(variation) else {
        return;
    };
    assert_eq!(
        duel.player
            .behavior
            .runtime
            .variable("Selector_Environment"),
        Some(if variation >= 10 { 1.0 } else { 0.0 })
    );
    let origin = Vec3::from(duel.player.body.position);
    let slide = shimmy(-1.0);
    let mut peek = false;
    for tick in 0..600 {
        let r = duel.step(&slide, DT);
        peek |= r
            .player
            .behavior
            .events
            .iter()
            .any(|e| e == "W_CoverActionStart");
        if tick % 60 == 0 {
            eprintln!(
                "cover tick={tick} pos={:?} anim={:?} events={:?} end={} blocked={:?}",
                duel.player.body.position,
                r.player.animation,
                r.player.behavior.events,
                duel.player.behavior.env.docking_end,
                duel.player.behavior.env.docking_blocked
            );
        }
    }
    assert!(Vec3::from(duel.player.body.position).distance(origin) > 1.0);
    // The stick slides Wolf to +X, the right end facing the wall: free or right-only look.
    assert_eq!(
        peek,
        matches!(variation, 1 | 3 | 11 | 13),
        "variation {variation}"
    );
    assert!(
        duel.player.docking.active.is_some(),
        "peek released docking"
    );
    let end = if away {
        InputFrame {
            move_stick: [0.0, 1.0],
            camera_yaw: std::f32::consts::PI,
            ..InputFrame::default()
        }
    } else {
        press()
    };
    let r = duel.step(&end, DT);
    assert!(
        r.player
            .behavior
            .events
            .iter()
            .any(|e| e.starts_with("W_CoverEnd")),
        "{:?}",
        r.player
    );
    for _ in 0..180 {
        duel.step(&InputFrame::default(), DT);
    }
    assert!(duel.player.docking.active.is_none() && duel.player.body.grounded);
    let before = Vec3::from(duel.player.body.position);
    for _ in 0..60 {
        duel.step(
            &InputFrame {
                move_stick: [0.0, 1.0],
                camera_yaw: std::f32::consts::PI,
                ..InputFrame::default()
            },
            DT,
        );
    }
    assert!(Vec3::from(duel.player.body.position).distance(before) > 0.5);
}

#[test]
fn outskirts_wall_route() {
    let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../cache");
    let dir = cache.join("maps/m11_00_00_00");
    if !dir.join("edges.json").exists() {
        return;
    }
    let mut duel = Duel::load(&cache, 4.0).unwrap();
    duel.enemy_active = false;
    duel.player.ground =
        Box::new(sekiro_sim::collision::CollisionWorld::load(&dir.join("collision.bin")).unwrap());
    duel.player.docking = Docking::load(&dir.join("edges.json")).unwrap();
    duel.player.body.position = [172.70456, -68.85, 63.21088];
    let yaw = 14.819877f32.to_radians();
    duel.player.body.yaw = yaw;
    let script =
        std::fs::read_to_string(cache.join("../crates/game/scripts/m11_cover.txt")).unwrap();
    let mut entered = false;
    let mut idle = false;
    let mut peek = false;
    let mut exited = false;
    let mut first = None;
    let mut last = None;
    for (tick, mut frame) in sekiro_sim::input::parse_action_script(&script)
        .unwrap()
        .into_iter()
        .enumerate()
    {
        frame.camera_yaw += yaw;
        let r = duel.step(&frame, 1.0 / 60.0);
        entered |= r
            .player
            .behavior
            .events
            .iter()
            .any(|e| e.starts_with("W_StandToCover2"));
        idle |= r.player.animation.as_deref() == Some("a000_021000");
        peek |= r.player.animation.as_deref() == Some("a000_205020");
        exited |= r
            .player
            .behavior
            .events
            .iter()
            .any(|e| e == "W_CoverEndFromCoverLook");
        if let Some(t) = &duel.player.docking.active {
            assert_eq!(duel.player.docking.edges[t.edge].source_part, "h018700");
            assert_eq!(duel.player.docking.edges[t.edge].chain, 1);
            first.get_or_insert(Vec3::from(duel.player.body.position));
            last = Some(Vec3::from(duel.player.body.position));
            let clearance = duel
                .player
                .ground
                .slide(duel.player.body.position, [0.0; 2]);
            assert!(clearance[0].hypot(clearance[1]) <= 0.05);
        }
        if tick % 30 == 0 || !r.player.behavior.events.is_empty() {
            eprintln!(
                "wall route tick={tick} pos={:?} animation={:?} events={:?}",
                duel.player.body.position, r.player.animation, r.player.behavior.events
            );
        }
        if matches!(tick, 150 | 270) {
            let bones = duel.player.bone_source_matrices();
            let world = duel.player.source_to_world();
            for (i, bone) in duel
                .player
                .skeleton
                .bones
                .iter()
                .enumerate()
                .filter(|(_, b)| b.name.ends_with("Head") || b.name.ends_with("Foot"))
            {
                let p = world.transform_point3(bones[i].transform_point3(Vec3::ZERO));
                eprintln!("cover pose tick={tick} {}={p:?}", bone.name);
            }
            if let Some(clip) = duel.player.clips.clip("a000_205010") {
                eprintln!(
                    "cover peek root={:?}",
                    clip.root_motion.as_ref().map(|r| r.total())
                );
            }
        }
    }
    assert!(
        entered && idle && peek && exited,
        "entered={entered} idle={idle} peek={peek} exited={exited}"
    );
    assert!(first.unwrap().distance(last.unwrap()) > 0.5);
    assert!(duel.player.docking.active.is_none() && duel.player.body.grounded);
}

// Issue #172: docked on the free-look wall, Wolf runs to each end in turn. On the way the
// shuffle clip the script picks must carry him the way he slides (its sides are named facing
// the wall, so with his back to it his own left is its right), and at the end he must peek out
// past the open end, not back along the wall.
#[test]
fn cover_shuffle_and_peek_follow_the_slide() {
    let Some(mut duel) = docked(1) else {
        return;
    };
    let head = duel
        .player
        .skeleton
        .bones
        .iter()
        .position(|b| b.name.ends_with("Head"))
        .unwrap();
    // Stick left runs to the +X end (the right one, facing the wall), stick right to -X.
    for (stick_x, out, clip) in [(-1.0, 1.0, "a000_205021"), (1.0, -1.0, "a000_205020")] {
        let mut shuffled = 0;
        let mut peek_ticks = 0;
        let mut lean = f32::NEG_INFINITY;
        for _ in 0..600 {
            let before = duel.player.body.position[0];
            let r = duel.step(&shimmy(stick_x), DT);
            let moved = duel.player.body.position[0] - before;
            let anim = r.player.animation.unwrap_or_default();
            if anim == clip {
                // The largest lean over the first ticks of the loop, so a blend frame cannot
                // decide it.
                let bones = duel.player.bone_source_matrices();
                let world = duel.player.source_to_world();
                let p = world.transform_point3(bones[head].transform_point3(Vec3::ZERO));
                lean = lean.max((p.x - duel.player.body.position[0]) * out);
                peek_ticks += 1;
                if peek_ticks == 20 {
                    break;
                }
            } else if (anim.starts_with("a000_0214") || anim.starts_with("a000_0215"))
                && moved != 0.0
            {
                let total = duel
                    .player
                    .clips
                    .clip(&anim)
                    .and_then(|c| c.root_motion.as_ref().map(|m| m.total()))
                    .expect("cover move clip without root motion");
                let [clip_x, _] = root_motion_world(total, duel.player.body.yaw.sin_cos());
                assert!(
                    clip_x * moved > 0.0,
                    "stick {stick_x}: {anim} carries the root {clip_x} along X, Wolf slid {moved}"
                );
                assert!(moved * stick_x < 0.0, "stick {stick_x} slid {moved}");
                shuffled += 1;
            }
        }
        assert!(
            shuffled > 20,
            "stick {stick_x}: only {shuffled} shuffle ticks"
        );
        assert!(peek_ticks > 0, "stick {stick_x}: no {clip} peek");
        assert!(
            lean > 0.1,
            "stick {stick_x}: head leans {lean} toward the open end"
        );
        // Stick back the other way ends the peek.
        for _ in 0..60 {
            duel.step(&shimmy(-stick_x), DT);
        }
    }
}
#[test]
fn cover_rejects_missing_wall_bad_height_and_unsupported_variation() {
    let mut docking = synthetic(1);
    let mut body = Body {
        position: [0.0, 0.0, 0.7],
        ..Body::default()
    };
    docking.search(&body, &sekiro_sim::body::FlatGround::default());
    assert!(docking.candidate.is_none());
    body.position[1] = 2.0;
    docking.search(&body, &Wall);
    assert!(docking.candidate.is_none());
    body.position = [0.0, 0.0, 1.1];
    docking.search(&body, &Wall);
    assert!(docking.candidate.is_none());
    body.position = [0.0, 0.0, 0.7];
    for variation in [4, 9, 14, 255] {
        docking.edges[0].variation = variation;
        docking.search(&body, &Wall);
        assert!(docking.candidate.is_none());
    }
}

#[test]
fn cover_obstacle_queries_distinguish_each_end() {
    struct Corner;
    impl Ground for Corner {
        fn height(&self, _: f32, _: f32) -> Option<f32> {
            Some(0.0)
        }
        fn slide(&self, feet: [f32; 3], delta: [f32; 2]) -> [f32; 2] {
            [
                (feet[0] + delta[0]).min(2.05) - feet[0],
                (feet[2] + delta[1]).max(0.3) - feet[2],
            ]
        }
    }
    let mut docking = synthetic(1);
    let body = Body {
        position: [2.0, 0.0, 0.7],
        ..Body::default()
    };
    docking.search(&body, &Corner);
    let target = docking.candidate.as_ref().unwrap();
    // The +X end is the right one, facing the wall.
    assert!(!docking.cover_blocked(target, &Corner, true));
    assert!(docking.cover_blocked(target, &Corner, false));
}

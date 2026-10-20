//! The fall-prevention walls at a map edge (#119): the Outskirts' `h900400` part (hit filter
//! 28, group 92) stands along cliff tops, and the player script asks for it in its walk and run
//! states (`act(3016, 0)`) but not in `SprintLoop` or a jump, so walking at the edge holds Wolf
//! and sprinting carries him over. Skips when the map has not been exported.

use sekiro_sim::collision::CollisionWorld;
use sekiro_sim::hit_filter::FALL_ASSIST_MOVE;
use sekiro_sim::{Buttons, Character, InputFrame};

use crate::movement_helpers::{DT, cache, settle, stick};

/// A cliff top 35 m east of the Outskirts' start 2: a point on the wall and the way out over
/// the edge, where the ground falls away 7.5 m and then further down the slope.
const EDGE: [f32; 2] = [-126.8, 28.06];
const OUT: [f32; 2] = [0.43, 0.90];

/// Wolf on the map 4 m inside the edge, facing out, and the camera yaw that pushes him out.
fn wolf_at_the_edge() -> Option<(Character, f32)> {
    let cache = cache()?;
    let dir = cache.join("maps/m11_00_00_00");
    if !dir.join("collision.bin").is_file() {
        return None;
    }
    let world = CollisionWorld::load_map(&dir).unwrap();
    let (x, z) = (EDGE[0] - OUT[0] * 4.0, EDGE[1] - OUT[1] * 4.0);
    let y = world.floor(x, z, -24.0).expect("a floor inside the edge");
    let mut wolf = Character::load_from_cache(&cache).unwrap();
    wolf.ground = Box::new(world);
    wolf.body.position = [x, y, z];
    settle(&mut wolf);
    // The stick's forward is the camera's: yaw 0 looks along -Z.
    Some((wolf, (-OUT[0]).atan2(-OUT[1])))
}

/// Pushes Wolf out over the edge for `ticks`, sprinting when `sprint`, and returns how far past
/// the wall's line he got (metres along [`OUT`]), how far he fell, the tick he first crossed it
/// and whether the script asked for the walls while grounded.
fn push_out(wolf: &mut Character, yaw: f32, sprint: bool, ticks: usize) -> (f32, f32, usize, bool) {
    let start_y = wolf.body.position[1];
    let (mut past, mut crossed, mut asked) = (f32::MIN, ticks, false);
    for tick in 0..ticks {
        let input = InputFrame {
            buttons: Buttons {
                dodge: sprint,
                ..Buttons::default()
            },
            ..stick(0.0, 1.0, yaw)
        };
        let report = wolf.tick(&input, DT);
        assert!(
            report.behavior.script_errors.is_empty(),
            "{:?}",
            report.behavior.script_errors
        );
        let p = wolf.body.position;
        past = past.max((p[0] - EDGE[0]) * OUT[0] + (p[2] - EDGE[1]) * OUT[1]);
        if past > 0.0 {
            crossed = crossed.min(tick);
        }
        asked |= wolf.body.grounded && wolf.behavior.env.fall_assist.get() & FALL_ASSIST_MOVE != 0;
    }
    (past, start_y - wolf.body.position[1], crossed, asked)
}

#[test]
fn walking_at_a_cliff_edge_holds_wolf_and_sprinting_goes_over() {
    let Some((mut wolf, yaw)) = wolf_at_the_edge() else {
        return;
    };
    // Run at the edge for 3 s: held at the wall, still on the cliff top.
    let (past, fell, _, asked) = push_out(&mut wolf, yaw, false, 180);
    assert!(asked, "the walk and run states never asked for the walls");
    assert!(past < 0.0, "ran {past:.2} m past the wall");
    assert!(fell.abs() < 0.5 && wolf.body.grounded, "fell {fell:.2} m");

    // Sprint into it: held through the sprint's start, then the loop asks for nothing and he
    // goes over.
    let (past, fell, crossed, _) = push_out(&mut wolf, yaw, true, 240);
    let over = crossed as f32 * DT;
    // The sprint's start still asks for the walls; the loop (from about 0.6 s) does not.
    assert!((0.3..1.0).contains(&over), "went over after {over:.2} s");
    assert!(past > 1.0, "sprint held at the wall ({past:.2} m past it)");
    assert!(
        fell > 3.0,
        "sprint did not go over the edge (fell {fell:.2} m)"
    );
}

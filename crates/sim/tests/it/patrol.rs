use std::sync::Arc;

use sekiro_sim::InputFrame;
use sekiro_sim::collision::CollisionWorld;
use sekiro_sim::duel::{Duel, population::Population};

/// An Outskirts ronin (c1010_0003) walks its two-point MSB route and stands for the route's
/// arrival wait at each point, as NonBattleAct scripts it (#116): the goal parameter `false`
/// reads back as 0 (walk), and a fresh NonBattleAct reads its pooled slot's last result
/// (GetLastResult Success) to queue the arrival wait. It used to run back and forth.
#[test]
fn outskirts_patrol_walks_and_waits_at_its_points() {
    let cache = sekiro_sim::cache::root();
    let map = "m11_00_00_00";
    let dir = cache.join(format!("maps/{map}"));
    if !dir.join("collision.bin").exists() || !cache.join("anim/c1010").is_dir() {
        return;
    }
    let mut wolf = Duel::load(&cache, 5.0).unwrap();
    wolf.god = true;
    // The issue's spawn, out of the ronin's sight.
    wolf.player.body.position = [-83.0, -44.6, -37.5];
    wolf.player.ground = Box::new(CollisionWorld::load_map(&dir).unwrap());
    let world = Arc::new(CollisionWorld::load_map(&dir).unwrap());
    let state = sekiro_sim::world_events::evaluate(
        &cache,
        map,
        sekiro_sim::world_events::FirstVisit::for_map(map).unwrap(),
    )
    .unwrap();
    let population =
        Population::load_map(&cache, map, world, wolf.player.body.position, Some(&state)).unwrap();
    wolf.set_population(population);
    let ronin = |wolf: &Duel| {
        let foe = wolf
            .population
            .as_ref()
            .unwrap()
            .foes
            .iter()
            .find(|f| f.part == "c1010_0003")
            .unwrap();
        let patrol = foe.duel.ai.as_ref().unwrap().state.patrol.clone().unwrap();
        (foe.duel.enemy.body.position, patrol)
    };
    let (_, patrol) = ronin(&wolf);
    let wait = patrol.arrival(0).expect("point 0 has an arrival entry").1 as f32;
    assert!(wait >= 5.0, "MSB arrival wait {wait}");
    let dt = 1.0 / 60.0;
    let mut track = Vec::new();
    let mut arrived = None;
    for step in 0..60 * 40 {
        wolf.step(&InputFrame::default(), dt);
        let t = step as f32 * dt;
        let (at, patrol) = ronin(&wolf);
        track.push((t, at));
        if arrived.is_none() && patrol.previous == 0 {
            arrived = Some((t, at));
        }
        if arrived.is_some_and(|(t0, _)| t > t0 + wait) {
            break;
        }
    }
    let (t0, at0) = arrived.expect("the ronin reaches its first point");
    let flat = |a: [f32; 3], b: [f32; 3]| (a[0] - b[0]).hypot(a[2] - b[2]);
    // Walking pace: at most 2 m in any second (running is about 5 m/s).
    let fastest = track
        .windows(61)
        .map(|w| flat(w[0].1, w[60].1))
        .fold(0.0f32, f32::max);
    // The arrival wait: still at the point most of the wait later (turning in place allowed).
    let stood = track
        .iter()
        .filter(|(t, _)| *t >= t0 && *t <= t0 + wait * 0.8)
        .map(|(_, at)| flat(*at, at0))
        .fold(0.0f32, f32::max);
    eprintln!("ronin: arrived {t0:.1} s, fastest {fastest:.2} m/s, drift {stood:.2} m in the wait");
    assert!(fastest < 2.0, "the patrol runs ({fastest:.2} m/s)");
    assert!(
        stood < 0.5,
        "the patrol leaves its point during the wait ({stood:.2} m)"
    );
}

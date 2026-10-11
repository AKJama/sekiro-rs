//! The boss brain driving the Great Shinobi Owl (c5060): plans run through his own battle goal,
//! the tally counts what happened, a slow controller falls back to his Lua AI without freezing,
//! and his reflexes keep working while a plan runs. Skips when `cache/` lacks him.

use std::path::PathBuf;

use sekiro_sim::brain::observation::observe;
use sekiro_sim::brain::{LuaBrain, Plan, ScriptedBrain, Side};
use sekiro_sim::duel::Duel;
use sekiro_sim::enemy::EnemySpec;
use sekiro_sim::lua_ai::AiEvent;
use sekiro_sim::{Buttons, InputFrame};

const DT: f32 = 1.0 / 60.0;

fn load(distance: f32) -> Option<Duel> {
    let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../cache");
    if !cache.join("anim/c5060/tae.json").exists()
        || !cache.join("raw/script/m11_01_00_00.luabnd.d").is_dir()
    {
        eprintln!("skipping: the Owl is not extracted");
        return None;
    }
    Some(Duel::load_with(&cache, distance, EnemySpec::owl()).unwrap())
}

fn scripted(opener: &[i32], delay: f32) -> Box<ScriptedBrain> {
    Box::new(ScriptedBrain::new(
        Plan::new(opener),
        Plan::new(opener),
        u32::MAX,
        delay,
    ))
}

fn frame(guard: bool, attack: bool, lock_on: bool) -> InputFrame {
    InputFrame {
        buttons: Buttons {
            guard,
            attack,
            lock_on,
            ..Buttons::default()
        },
        ..InputFrame::default()
    }
}

/// Keeps Wolf alive and unbroken, so the Owl fights for the whole run.
fn keep_wolf_fresh(duel: &mut Duel) {
    let wolf = &mut duel.rules.fighters[0];
    wolf.vitality.hp = wolf.vitality.max_hp;
    wolf.posture.remaining = wolf.posture.max;
}

/// The Owl's attack and step requests (`AiEvent::Action`) since the last call.
fn actions(duel: &mut Duel) -> Vec<i32> {
    duel.ai
        .as_mut()
        .unwrap()
        .drain_log()
        .into_iter()
        .filter_map(|e| match e {
            AiEvent::Action(a) => Some(a),
            AiEvent::Error(e) => panic!("AI error: {e}"),
            _ => None,
        })
        .collect()
}

#[test]
fn lua_brain_leaves_his_choices_unchanged() {
    let Some(mut plain) = load(6.0) else { return };
    let mut with_brain = load(6.0).unwrap();
    with_brain.set_brain(Box::new(LuaBrain), false).unwrap();
    let (mut a, mut b) = (Vec::new(), Vec::new());
    for _ in 0..1800 {
        keep_wolf_fresh(&mut plain);
        keep_wolf_fresh(&mut with_brain);
        plain.step(&InputFrame::default(), DT);
        with_brain.step(&InputFrame::default(), DT);
        a.extend(actions(&mut plain));
        b.extend(actions(&mut with_brain));
    }
    assert!(a.len() >= 8, "{a:?}");
    assert_eq!(a, b);
    let stats = with_brain.boss_brain.as_ref().unwrap().stats;
    assert_eq!((stats.requests, stats.planned), (0, 0));
    assert!(stats.fallback > 0, "{stats:?}");
}

#[test]
fn owl_performs_the_planned_actions_in_order() {
    let Some(mut duel) = load(4.0) else { return };
    // An order his own acts never use. Not the kick (3005): when it lands it leaves SpEffect
    // 3506000 on Wolf, and his AI's interrupt answers with its own follow-up (3007).
    let plan = [3003, 3002, 3001];
    duel.set_brain(scripted(&plan, 0.1), false).unwrap();
    let mut runs: Vec<Vec<i32>> = Vec::new();
    let mut seen = 0;
    let mut largest_obs = 0;
    for _ in 0..3600 {
        keep_wolf_fresh(&mut duel);
        duel.step(&InputFrame::default(), DT);
        let driver = duel.boss_brain.as_ref().unwrap();
        if driver.history.len() > seen {
            seen = driver.history.len();
            runs.push(Vec::new());
            let obs = observe(&duel, driver.time, [None, Some(1.0)], "3000+3001+3002");
            largest_obs = largest_obs.max(serde_json::to_string(&obs).unwrap().len());
        }
        let acted = actions(&mut duel);
        if let Some(run) = runs.last_mut() {
            run.extend(acted);
        }
    }
    let stats = duel.boss_brain.as_ref().unwrap().stats;
    eprintln!("{stats:?}; runs {runs:?}; largest observation {largest_obs} bytes");
    let complete = runs.iter().filter(|r| r.as_slice() == plan).count();
    assert!(
        complete >= 5,
        "{complete} complete runs of {plan:?}: {runs:?}"
    );
    // Every planned slot starts with the plan's first action (a passive Wolf never makes him
    // react), and nothing but the plan's actions run inside it.
    for r in &runs {
        assert!(r.iter().all(|a| plan.contains(a)), "{r:?}");
    }
    assert!(stats.planned as usize >= complete);
    assert!(largest_obs < 1024, "{largest_obs}");
}

#[test]
fn tally_counts_outcomes_in_a_scripted_duel() {
    let Some(mut duel) = load(3.0) else { return };
    duel.set_brain(scripted(&[3000, 3001], 0.1), false).unwrap();
    // [owl hit, owl blocked or deflected] from the duel's own reports, while Owl plans run.
    let mut owl = [0u32; 2];
    let mut wolf_hits = 0u32;
    for i in 0..3600 {
        keep_wolf_fresh(&mut duel);
        // First 30 s passive, then hold guard and slash now and then.
        let guarding = i >= 1800 && i % 240 < 200;
        let attack = i >= 1800 && i % 240 == 220;
        let r = duel.step(&frame(guarding, attack, i == 1802), DT);
        for (h, res) in &r.hits {
            if h.attacker == 1 {
                owl[usize::from(res.guarded || res.deflected)] += 1;
            } else if !res.guarded && !res.deflected {
                wolf_hits += 1;
            }
        }
        actions(&mut duel);
    }
    for _ in 0..120 {
        duel.step(&InputFrame::default(), DT);
    }
    let driver = duel.boss_brain.as_ref().unwrap();
    let s = driver.tally.summary();
    eprintln!(
        "reports: owl hits {owl:?}, wolf hits {wolf_hits}; tally {}",
        serde_json::to_string(&s).unwrap()
    );
    let total = |side: Side, f: fn(&sekiro_sim::brain::Counts) -> u32| {
        let m = if side == Side::Boss { &s.boss } else { &s.wolf };
        m.values().map(f).sum::<u32>()
    };
    let n: u32 = s
        .boss
        .0
        .iter()
        .filter(|(key, _)| {
            key.parse::<i32>()
                .is_ok_and(|id| (3000..3100).contains(&id))
        })
        .map(|(_, c)| c.n)
        .sum();
    for (key, c) in &s.boss.0 {
        if key
            .parse::<i32>()
            .is_ok_and(|id| (5200..5300).contains(&id))
        {
            assert_eq!(c.whiffed, 0, "back step {key} cannot whiff");
        }
    }
    let hit = total(Side::Boss, |c| c.hit);
    let guarded = total(Side::Boss, |c| c.blocked + c.deflected);
    let whiffed = total(Side::Boss, |c| c.whiffed);
    assert!(owl[0] > 0 && owl[1] > 0, "{owl:?}");
    // An exchange counts a hit or a guard once however many of its hits landed.
    assert!(hit > 0 && hit <= owl[0], "{hit} vs {owl:?}");
    assert!(guarded > 0 && guarded <= owl[1], "{guarded} vs {owl:?}");
    assert!(
        whiffed + hit + guarded >= n,
        "every attack exchange has an outcome"
    );
    assert!(s.boss["3000"].n > 0 && s.boss["3001"].n > 0);
    // Wolf's slashes are in his tally, and the ones that landed punished the Owl.
    assert!(total(Side::Wolf, |c| c.n) > 0);
    if wolf_hits > 0 {
        assert!(total(Side::Wolf, |c| c.hit) > 0);
        assert!(total(Side::Boss, |c| c.punished) > 0);
    }
}

/// Runs a minute against a passive Wolf; returns the number of steps with an Owl action
/// request and the longest stretch of steps without one.
fn action_rhythm(duel: &mut Duel) -> (usize, usize) {
    let mut last_action = 0;
    let mut longest_gap = 0;
    let mut count = 0;
    for i in 0..3600 {
        keep_wolf_fresh(duel);
        duel.step(&InputFrame::default(), DT);
        if !actions(duel).is_empty() {
            longest_gap = longest_gap.max(i - last_action);
            last_action = i;
            count += 1;
        }
    }
    (count, longest_gap.max(3600 - last_action))
}

#[test]
fn slow_brain_falls_back_to_his_lua_ai_without_freezing() {
    let Some(mut duel) = load(6.0) else { return };
    // Far slower than any plan: by the next act the answer is still out, so his Lua AI picks.
    duel.set_brain(scripted(&[3005], 4.0), false).unwrap();
    let (count, gap) = action_rhythm(&mut duel);
    let stats = duel.boss_brain.as_ref().unwrap().stats;
    let mut lua = load(6.0).unwrap();
    let (lua_count, lua_gap) = action_rhythm(&mut lua);
    eprintln!(
        "{stats:?}; {count} actions, longest gap {gap} steps (Lua alone: {lua_count}, {lua_gap})"
    );
    assert!(stats.fallback >= 5, "{stats:?}");
    assert!(stats.requests >= 3, "{stats:?}");
    assert!(count >= 15, "{count}");
    // No longer idle than his own AI gets (it waits and strafes too), plus a second.
    assert!(
        gap <= lua_gap.max(240) + 60,
        "the Owl stood still for {gap} steps"
    );
}

#[test]
fn reflex_deflects_happen_while_a_plan_runs() {
    let Some(mut duel) = load(2.5) else { return };
    duel.set_brain(scripted(&[3000, 3001, 3002], 0.1), false)
        .unwrap();
    let mut reflexes_in_plan = 0;
    let mut guarded_in_plan = 0;
    for i in 0..5400 {
        keep_wolf_fresh(&mut duel);
        // Lock on and slash every half second.
        let r = duel.step(&frame(false, i % 30 == 0, i == 2), DT);
        let running = !duel.boss_brain.as_ref().unwrap().running.is_empty();
        let acted = actions(&mut duel);
        if running {
            reflexes_in_plan += acted.iter().filter(|a| (3100..=3103).contains(*a)).count();
            guarded_in_plan += r
                .hits
                .iter()
                .filter(|(h, res)| h.attacker == 0 && (res.guarded || res.deflected))
                .count();
        }
    }
    let stats = duel.boss_brain.as_ref().unwrap().stats;
    eprintln!(
        "{stats:?}; guard/deflect requests {reflexes_in_plan}, guarded hits {guarded_in_plan}"
    );
    assert!(stats.planned >= 3, "{stats:?}");
    assert!(reflexes_in_plan > 0, "no guard or deflect while a plan ran");
}

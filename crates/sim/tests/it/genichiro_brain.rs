//! Offline castle Genichiro planning, real clips, arrows and isolated memory.
use std::path::PathBuf;

use sekiro_sim::brain::{BossActions, BrainKind, GENICHIRO, LuaBrain, Plan, ScriptedBrain};
use sekiro_sim::{InputFrame, duel::Duel, lua_ai::AiEvent, registry::Registry};

const DT: f32 = 1.0 / 60.0;

#[test]
fn counter_memory_suppresses_mikiri_bait_and_is_in_the_fitted_coach_input() {
    let Some(mut duel) = load(4.0) else { return };
    duel.set_brain(BrainKind::Scripted.build("c7100"), false)
        .unwrap();
    let driver = duel.boss_brain.as_mut().unwrap();
    driver.memory.boss.insert(
        "3076".into(),
        sekiro_sim::brain::tally::Counts {
            n: 25,
            hit: 1,
            countered: 24,
            counter_kinds: [24, 0, 0],
            ..Default::default()
        },
    );
    duel.step(&InputFrame::default(), DT);
    let hook = duel.ai.as_ref().unwrap().state.act_hook.as_ref().unwrap();
    let factor = hook
        .counter_weights
        .iter()
        .find(|(id, _)| *id == 14)
        .unwrap()
        .1;
    assert!(factor > 0.0 && factor < 0.1, "exploration floor: {factor}");
    assert!(
        hook.counter_weights
            .iter()
            .find(|(id, _)| *id == 1)
            .unwrap()
            .1
            > 0.0
    );
    let mut input = duel
        .boss_brain
        .as_ref()
        .unwrap()
        .coach_input(&Default::default());
    assert!(
        input.fit(sekiro_sim::brain::coach::INPUT_BUDGET) <= sekiro_sim::brain::coach::INPUT_BUDGET
    );
    assert_eq!(input.counters["3076"].countered, 24);
    assert_eq!(input.boss_phase, "c7100");
}

struct CheckpointRecorder {
    inner: ScriptedBrain,
    seen: std::rc::Rc<std::cell::RefCell<Vec<Option<&'static str>>>>,
}

impl sekiro_sim::brain::BossBrain for CheckpointRecorder {
    fn name(&self) -> &'static str {
        "scripted"
    }
    fn default_mode(&self) -> sekiro_sim::brain::RequestMode {
        sekiro_sim::brain::RequestMode::Checkpoints
    }
    fn submit(&mut self, request: &sekiro_sim::brain::BrainRequest) {
        self.seen.borrow_mut().push(request.why);
        self.inner.submit(request);
    }
    fn poll(&mut self, now: f32) -> Option<sekiro_sim::brain::BrainReply> {
        self.inner.poll(now)
    }
}

#[test]
fn genichiro_posture_pressure_checkpoints_rearm() {
    let Some(mut duel) = load(4.0) else { return };
    let seen = std::rc::Rc::default();
    let plan = GENICHIRO.parse("hold2>strafe_left").unwrap();
    duel.set_brain(
        Box::new(CheckpointRecorder {
            inner: ScriptedBrain::new(plan.clone(), plan, u32::MAX, 0.2),
            seen: std::rc::Rc::clone(&seen),
        }),
        false,
    )
    .unwrap();
    duel.boss_brain.as_mut().unwrap().anti_stall = false;
    let attack = duel
        .player
        .tae
        .anim("a050_300000")
        .unwrap()
        .events
        .iter()
        .find_map(|e| match e.kind {
            sekiro_sim::tae::TaeKind::Attack { judge, .. } => duel.fighters[0].attacks.get(&judge),
            _ => None,
        })
        .unwrap();
    let per_hit = duel
        .rules
        .wolf_posture_per_hit(attack, [&duel.player, &duel.enemy])
        .unwrap();
    assert!(per_hit > 0.0);
    for hits_left in [100.0, 5.0, 2.5, 100.0, 5.0, 2.5] {
        for _ in 0..120 {
            let boss = &mut duel.rules.fighters[1];
            boss.posture.remaining = ((hits_left * per_hit) as i32).min(boss.posture.max);
            duel.step(&InputFrame::default(), DT);
            actions(&mut duel);
        }
    }
    assert_eq!(
        *seen.borrow(),
        [
            Some("start"),
            Some("pressure"),
            Some("pressure_critical"),
            Some("pressure"),
            Some("pressure_critical")
        ]
    );
}

#[test]
fn genichiro_plan_sets_obey_the_six_second_attack_rule() {
    use sekiro_sim::brain::coach::{GamePlan, PlanSet, WeightedChoice};
    use sekiro_sim::brain::limits::Pacing;
    let plan = GamePlan {
        version: 1,
        sets: vec![PlanSet {
            name: "press".into(),
            plans: ["hold2", "3000+3001+3002", "back_off9>3009+3044"]
                .map(|plan| WeightedChoice {
                    plan: plan.into(),
                    weight: 1.0,
                })
                .to_vec(),
            ..PlanSet::default()
        }],
        start_set: "press".into(),
        ..GamePlan::default()
    }
    .validate(&GENICHIRO)
    .plan;
    assert_eq!(plan.sets[0].plans.len(), 2);
    let pacing = Pacing::new(0.0);
    assert!(pacing.eligible(&GENICHIRO.parse("3000+3001+3002").unwrap(), 6.0, &GENICHIRO));
    assert!(!pacing.eligible(
        &GENICHIRO.parse("back_off9>3009+3044").unwrap(),
        6.0,
        &GENICHIRO
    ));
}

fn load(distance: f32) -> Option<Duel> {
    let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../cache");
    if !cache.join("anim/c7100/skeleton.bin").exists() {
        eprintln!("skipping: Genichiro is not extracted");
        return None;
    }
    let spec = Registry::load(&cache)
        .unwrap()
        .resolve("c7100:71001000")
        .unwrap();
    let mut duel = Duel::load_with(&cache, distance, spec).unwrap();
    duel.god = true;
    Some(duel)
}

fn actions(duel: &mut Duel) -> Vec<i32> {
    duel.ai
        .as_mut()
        .unwrap()
        .drain_log()
        .into_iter()
        .filter_map(|e| match e {
            AiEvent::Action(a) => Some(a),
            AiEvent::Error(e) => panic!("{e}"),
            _ => None,
        })
        .collect()
}

#[test]
fn scripted_genichiro_attacks_idle_god_wolf_for_sixty_seconds() {
    let Some(mut duel) = load(6.0) else { return };
    duel.set_brain(BrainKind::Scripted.build("c7100"), false)
        .unwrap();
    let mut chosen_attacks = 0;
    for _ in 0..3600 {
        let report = duel.step(&InputFrame::default(), DT);
        assert!(report.enemy.behavior.script_errors.is_empty());
        if report.enemy.clip_started {
            let driver = duel.boss_brain.as_ref().unwrap();
            if let Some(animation) = &report.enemy.animation
                && let Some(id) = animation
                    .strip_prefix("a000_")
                    .and_then(|s| s.parse::<i32>().ok())
                && (3000..3100).contains(&id)
                && driver
                    .history
                    .iter()
                    .any(|p| p.steps.contains(&sekiro_sim::brain::plan::Step::Act(id)))
            {
                chosen_attacks += 1;
            }
        }
        actions(&mut duel);
    }
    let driver = duel.boss_brain.as_ref().unwrap();
    eprintln!(
        "Genichiro: {chosen_attacks} chosen attack starts; {:?}",
        driver.stats
    );
    assert!(chosen_attacks >= 5);
    assert!(driver.stats.planned >= 5);
    assert_eq!(driver.stats.invalid, 0);
    assert!(driver.history.iter().all(|p| p.value() == "3000+3001+3002"));
}

#[test]
fn genichiro_move_sheet_builds_real_combo_timelines() {
    let Some(mut duel) = load(4.0) else { return };
    duel.set_brain(BrainKind::Scripted.build("c7100"), false)
        .unwrap();
    let sheet = &duel.boss_brain.as_ref().unwrap().move_sheet;
    for action in GENICHIRO.actions {
        assert!(
            sheet.contains_key(&action.id.to_string()),
            "missing {}",
            action.id
        );
    }
    let combo = &sheet["3000+3001+3002"];
    assert_eq!(combo.hits.len(), 3);
    assert_eq!(
        combo.hits.iter().map(|h| h[3] as i32).collect::<Vec<_>>(),
        [100, 101, 102]
    );
    assert!((combo.hits[1][0] - (0.9 + 0.767)).abs() < 0.002);
    assert!(combo.hits.iter().all(|h| h[1] > 0.0 && h[2] > 0.0));
    assert!(combo.reach.unwrap() > 0.0);
    assert_eq!(sheet["3040+3041"].hits.len(), 7);
    assert!(!sheet["3007"].cancels.is_empty());
}

#[test]
fn brain_bow_plan_fires_real_arrow_bullets() {
    let Some(mut duel) = load(8.0) else { return };
    let plan = Plan::new(&[3009, 3044]);
    duel.set_brain(
        Box::new(ScriptedBrain::new(plan.clone(), plan, u32::MAX, 0.0)),
        false,
    )
    .unwrap();
    let mut arrows = std::collections::BTreeSet::new();
    for _ in 0..1800 {
        duel.step(&InputFrame::default(), DT);
        for bullet in &duel.bullets.live {
            if bullet.look() == sekiro_sim::bullets::BulletLook::Missile {
                arrows.insert(bullet.def.id);
            }
        }
        actions(&mut duel);
    }
    eprintln!("brain arrows: {arrows:?}");
    assert!(arrows.contains(&71000500));
    assert!(arrows.contains(&71000530));
    assert!(duel.boss_brain.as_ref().unwrap().stats.planned > 0);
}

#[test]
fn lua_brain_preserves_genichiros_choices() {
    let Some(mut plain) = load(6.0) else { return };
    let mut hooked = load(6.0).unwrap();
    hooked.set_brain(Box::new(LuaBrain), false).unwrap();
    let (mut a, mut b) = (Vec::new(), Vec::new());
    for _ in 0..1800 {
        plain.step(&InputFrame::default(), DT);
        hooked.step(&InputFrame::default(), DT);
        a.extend(actions(&mut plain));
        b.extend(actions(&mut hooked));
    }
    assert!(a.len() >= 5);
    assert_eq!(a, b);
    assert_eq!(hooked.boss_brain.as_ref().unwrap().stats.planned, 0);
}

#[test]
fn choices_and_memory_are_character_specific() {
    assert_eq!(BossActions::for_chr("c7100").unwrap().chr, "c7100");
    assert!(BossActions::for_chr("c7110").unwrap().get(3039).is_some());
    for choice in GENICHIRO.choices() {
        assert!(GENICHIRO.parse(&choice.value).is_ok(), "{}", choice.value);
    }
    for id in [3020, 3021, 3039, 3042, 3043, 3033, 3047, 3060] {
        assert!(GENICHIRO.get(id).is_none());
    }
    let dir = std::env::temp_dir().join(format!(
        "sekiro-genichiro-brain-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let path =
        sekiro_sim::brain::memory::resolve_for(Some(dir.to_str().unwrap()), "c7100", "coach", true)
            .unwrap();
    let layers = sekiro_sim::brain::memory::LayerPaths::new(&path, "testwolf");
    assert_eq!(layers.boss, dir.join("c7100.boss.json"));
    assert_eq!(layers.player, dir.join("c7100.player-testwolf.json"));
    for chr in ["c1010", "c5060"] {
        let other = sekiro_sim::brain::memory::LayerPaths::new(
            &dir.join(format!("{chr}.json")),
            "testwolf",
        );
        assert_ne!(layers.boss, other.boss);
        assert_ne!(layers.player, other.player);
    }
    let Some(mut duel) = load(4.0) else { return };
    duel.set_brain(BrainKind::Scripted.build("c7100"), false)
        .unwrap();
    duel.boss_brain
        .as_mut()
        .unwrap()
        .use_player_memory(path, "testwolf");
    for _ in 0..960 {
        duel.step(&InputFrame::default(), DT);
    }
    drop(duel);
    assert!(layers.boss.exists());
    assert!(layers.player.exists());
    std::fs::remove_dir_all(&dir).unwrap();
}
#[test]
fn tomoe_handover_preserves_the_brain_and_reset_continues_numbering() {
    let Some(mut duel) = load(1.6) else { return };
    duel.set_brain(BrainKind::Scripted.build("c7100"), false)
        .unwrap();
    for _ in 0..150 {
        duel.step(&InputFrame::default(), DT);
    }
    take_mark(&mut duel);
    assert_eq!(duel.rules.fighters[1].vitality.deathblows_left, 1);
    assert!(duel.boss_brain.is_some());
    take_mark(&mut duel);
    for _ in 0..600 {
        duel.step(&InputFrame::default(), DT);
        if duel.spec.chr == "c7110" {
            break;
        }
    }
    assert_eq!(
        (duel.spec.chr.as_str(), duel.spec.npc_param),
        ("c7110", 71100000)
    );
    assert!(duel.boss_brain.is_some());
    assert_eq!(duel.boss_brain.as_ref().unwrap().overlay().fight, 1);
    assert_eq!(duel.boss_brain.as_ref().unwrap().stats.fights, 0);
    duel.step(&InputFrame::default(), DT);
    assert!(duel.ai.as_ref().unwrap().state.act_hook.is_some());
    assert!(
        duel.boss_brain
            .as_ref()
            .unwrap()
            .move_sheet
            .contains_key("3039")
    );
    // The headless duel starts at the origin, whereas the map hand-over uses
    // Tomoe's authored castle placement. Put idle Wolf in that fight's range.
    let p = duel.enemy.body.position;
    duel.player.body.position = [p[0], p[1], p[2] + 6.0];
    duel.enemy.body.yaw = std::f32::consts::PI;
    let mut attacks = 0;
    for _ in 0..1800 {
        duel.step(&InputFrame::default(), DT);
        attacks += actions(&mut duel)
            .iter()
            .filter(|id| (3000..3100).contains(*id))
            .count();
    }
    assert!(attacks >= 3, "Tomoe attacks: {attacks}");
    assert!(
        duel.boss_brain
            .as_ref()
            .unwrap()
            .history
            .iter()
            .any(|p| p.actions().contains(&3039))
    );
    duel.reset().unwrap();
    assert_eq!(duel.spec.chr, "c7100");
    assert_eq!(duel.boss_brain.as_ref().unwrap().overlay().fight, 2);
    let planned = duel.boss_brain.as_ref().unwrap().stats.planned;
    for _ in 0..600 {
        duel.step(&InputFrame::default(), DT);
    }
    assert!(duel.boss_brain.as_ref().unwrap().stats.planned > planned);
}

fn take_mark(duel: &mut Duel) {
    duel.enemy_active = false;
    for _ in 0..360 {
        duel.step(&InputFrame::default(), 1.0 / 60.0);
    }
    let p = duel.player.body.position;
    duel.player.body.yaw = 0.0;
    duel.enemy.body.position = [p[0], p[1], p[2] - 1.6];
    duel.enemy.body.yaw = std::f32::consts::PI;
    let mut broke = false;
    let mut started = false;
    for i in 0..1500 {
        if !broke {
            duel.rules.fighters[1].posture.remaining = 1;
        }
        let frame = InputFrame {
            buttons: sekiro_sim::Buttons {
                attack: if broke {
                    !started && i % 10 == 0
                } else {
                    i % 40 == 0
                },
                lock_on: (2..4).contains(&i),
                ..sekiro_sim::Buttons::default()
            },
            ..InputFrame::default()
        };
        let report = duel.step(&frame, 1.0 / 60.0);
        assert!(
            report.enemy.behavior.script_errors.is_empty(),
            "{:?}",
            report.enemy.behavior.script_errors
        );
        broke |= report
            .hits
            .iter()
            .any(|(h, r)| h.attacker == 0 && r.posture_broke);
        started |= report.deathblow.started.is_some();
        if started && duel.deathblow.is_none() {
            break;
        }
    }
    assert!(started, "deathblow not started");
}

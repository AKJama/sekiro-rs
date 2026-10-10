use super::actions::{OWL, PlanError, SOLDIER};
use super::habits::{HabitTracker, WolfFrame};
use super::memory::{FightEnd, Loaded, Memory};
use super::*;
use crate::lua_ai::NumberOnActivate;

fn start(t: &mut Tally, time: f32, side: Side, key: &str) {
    t.record(
        time,
        TallyEvent::Start {
            side,
            key: key.into(),
        },
    );
}

fn hit(t: &mut Tally, time: f32, attacker: Side, guarded: bool, deflected: bool) {
    t.record(
        time,
        TallyEvent::Hit {
            attacker,
            guarded,
            deflected,
        },
    );
}

#[test]
fn tally_counts_each_outcome() {
    let mut t = Tally::default();
    // 3005 hits, then 3005 is deflected and punished, then 3031 whiffs, then a shuriken (3009)
    // is blocked after its clip ended.
    start(&mut t, 0.0, Side::Boss, "3005");
    hit(&mut t, 0.4, Side::Boss, false, false);
    t.record(1.0, TallyEvent::End { side: Side::Boss });
    start(&mut t, 2.0, Side::Boss, "3005");
    hit(&mut t, 2.4, Side::Boss, true, true);
    t.record(3.0, TallyEvent::End { side: Side::Boss });
    // Wolf's slash lands 0.5 s after the deflected kick ended: punished, and a hit for Wolf.
    start(&mut t, 3.2, Side::Wolf, "GroundAttackCombo1Release");
    hit(&mut t, 3.5, Side::Wolf, false, false);
    t.record(3.8, TallyEvent::End { side: Side::Wolf });
    start(&mut t, 5.0, Side::Boss, "3031");
    start(&mut t, 6.0, Side::Boss, "3009");
    t.record(6.5, TallyEvent::End { side: Side::Boss });
    hit(&mut t, 6.9, Side::Boss, true, false);
    let done = t.advance(10.0);
    assert_eq!(done.len(), 5);
    let kick = t.counts(Side::Boss, "3005");
    assert_eq!(
        kick,
        Counts {
            n: 2,
            hit: 1,
            blocked: 0,
            deflected: 1,
            whiffed: 0,
            punished: 1
        }
    );
    let throw = t.counts(Side::Boss, "3031");
    assert_eq!((throw.n, throw.whiffed, throw.punished), (1, 1, 0));
    let shuriken = t.counts(Side::Boss, "3009");
    assert_eq!((shuriken.n, shuriken.blocked, shuriken.whiffed), (1, 1, 0));
    let slash = t.counts(Side::Wolf, "GroundAttackCombo1Release");
    assert_eq!((slash.n, slash.hit, slash.punished), (1, 1, 0));
}

#[test]
fn punish_window_and_ranked_summary() {
    let mut t = Tally::default();
    start(&mut t, 0.0, Side::Boss, "3000");
    t.record(1.0, TallyEvent::End { side: Side::Boss });
    // 1.5 s after the end: outside the 1 s window, not a punish.
    hit(&mut t, 2.5, Side::Wolf, false, false);
    for (i, key) in ["3001", "3002", "3002"].iter().enumerate() {
        let s = 3.0 + i as f32 * 2.0;
        start(&mut t, s, Side::Boss, key);
        t.record(s + 1.0, TallyEvent::End { side: Side::Boss });
    }
    t.advance(20.0);
    assert_eq!(t.counts(Side::Boss, "3000").punished, 0);
    let s = t.summary();
    assert_eq!(s.boss.len(), 3);
    // Most used first, then by key.
    assert_eq!(
        serde_json::to_string(&s.boss).unwrap(),
        r#"{"3002":[2,0,0,0,2,0],"3000":[1,0,0,0,1,0],"3001":[1,0,0,0,1,0]}"#
    );
}

#[test]
fn exchanges_wait_out_the_punish_window() {
    let mut t = Tally::default();
    start(&mut t, 0.0, Side::Boss, "3005");
    t.record(1.0, TallyEvent::End { side: Side::Boss });
    assert!(t.advance(1.5).is_empty());
    hit(&mut t, 1.8, Side::Wolf, false, false);
    let done = t.advance(2.0);
    assert_eq!(done.len(), 1);
    assert!(done[0].outcome.punished);
}

#[test]
fn plans_become_acts() {
    let act = OWL.act_for(&[3000, 3001, 3002], 0.7).unwrap();
    assert_eq!(act.approach, Some(f64::from(4.8f32 - 0.7)));
    let goals: Vec<_> = act.goals.iter().map(|g| (g.goal, g.params[0])).collect();
    assert_eq!(
        goals,
        [
            ("GOAL_COMMON_ComboAttackTunableSpin", 3000.0),
            ("GOAL_COMMON_ComboRepeat", 3001.0),
            ("GOAL_COMMON_ComboFinal", 3002.0),
        ]
    );
    assert_eq!(act.goals[1].number, Some(NumberOnActivate::Add(5, 7.0)));
    let single = OWL.act_for(&[3031], 0.7).unwrap();
    assert_eq!(single.goals[0].goal, "GOAL_COMMON_AttackTunableSpin");
    let step = OWL.act_for(&[5201, 3005], 0.7).unwrap();
    assert_eq!(step.approach, None);
    assert_eq!(step.goals[0].goal, "GOAL_COMMON_SpinStep");
    assert_eq!(step.goals[0].number, Some(NumberOnActivate::Set(5, 0.0)));
    assert_eq!(OWL.act_for(&[], 0.7), Err(PlanError::Empty));
    assert_eq!(
        OWL.act_for(&[3000, 3001, 3002, 3063], 0.7),
        Err(PlanError::TooLong(4))
    );
    assert_eq!(OWL.act_for(&[3037], 0.7), Err(PlanError::Unknown(3037)));
}

fn request(id: u64, t: f32, tally: TallySummary) -> BrainRequest {
    BrainRequest {
        id,
        obs: Observation {
            t,
            ..Observation::default()
        },
        tally,
        memory: None,
    }
}

#[test]
fn scripted_brain_switches_and_waits() {
    let mut b = ScriptedBrain {
        delay: 0.5,
        ..ScriptedBrain::default()
    };
    b.submit(&request(1, 2.0, TallySummary::default()));
    assert_eq!(b.poll(2.4), None);
    let r = b.poll(2.5).unwrap();
    assert_eq!((r.id, r.plan), (1, Some(Plan::new(&[3005]))));
    assert_eq!(b.poll(3.0), None, "one answer per request");
    let mut tally = TallySummary::default();
    tally.boss.insert(
        "3005",
        Counts {
            n: 3,
            deflected: 2,
            ..Counts::default()
        },
    );
    b.submit(&request(2, 4.0, tally));
    assert_eq!(b.poll(4.5).unwrap().plan, Some(Plan::new(&[3031])));
    assert!(!LuaBrain.plans());
    assert_eq!(BrainKind::by_name("Scripted"), Ok(BrainKind::Scripted));
    assert!(BrainKind::by_name("gpt").is_err());
}

/// The largest observation: long state names, every number at its widest.
fn widest_observation() -> Observation {
    let side = |act| FighterView {
        hp: 0.33,
        posture: 0.67,
        lives: 2,
        state: "GroundAttackCombo5ReleaseFromStep".into(),
        act,
        attacking: true,
        guarding: true,
        airborne: true,
        staggered: true,
        deathblow_open: true,
    };
    Observation {
        t: 1234.56,
        boss: side(Some(3063)),
        wolf: side(None),
        dist: 12.34,
        dy: -1.25,
        wolf_bearing: -179.0,
        boss_bearing: 135.0,
        locked_on: true,
        since_boss_hit: Some(12.34),
        since_wolf_hit: Some(0.5),
        plan: vec![3000, 3001, 3002],
    }
}

#[test]
fn observation_json_stays_small() {
    let json = serde_json::to_string(&widest_observation()).unwrap();
    assert!(json.len() < 1024, "{} bytes: {json}", json.len());
    assert!(json.contains("\"dist\":12.34"), "{json}");
}

#[test]
fn soldier_plans_become_his_acts() {
    assert_eq!(BossActions::for_chr("c1010").map(|a| a.chr), Some("c1010"));
    assert_eq!(BossActions::for_chr("c5060").map(|a| a.chr), Some("c5060"));
    assert!(BossActions::for_chr("c9999").is_none());
    // Act01: approach to 2.5 m less his radius, then the combo; no AI number.
    let act = SOLDIER.act_for(&[3000, 3001], 0.5).unwrap();
    assert_eq!(act.approach, Some(2.0));
    let goals: Vec<_> = act.goals.iter().map(|g| (g.goal, g.params[0])).collect();
    assert_eq!(
        goals,
        [
            ("GOAL_COMMON_ComboAttackTunableSpin", 3000.0),
            ("GOAL_COMMON_ComboFinal", 3001.0),
        ]
    );
    assert!(act.goals.iter().all(|g| g.number.is_none()));
    // Act09's rush sets AI number 5 to 1; the stone throw (Act07) never approaches.
    let rush = SOLDIER.act_for(&[3011], 0.5).unwrap();
    assert_eq!(rush.goals[0].number, Some(NumberOnActivate::Set(5, 1.0)));
    assert_eq!(SOLDIER.act_for(&[3009], 0.5).unwrap().approach, None);
    let step = SOLDIER.act_for(&[5211], 0.5).unwrap();
    assert_eq!(step.goals[0].goal, "GOAL_COMMON_SpinStep");
    assert_eq!(step.goals[0].number, None);
    // Ids his script names but his TAE has no clip for, and the Owl's, are refused.
    for id in [3004, 3006, 5202, 3031] {
        assert_eq!(SOLDIER.act_for(&[id], 0.5), Err(PlanError::Unknown(id)));
    }
    // Every combo is a valid plan of its own table.
    for table in [&SOLDIER, &OWL] {
        for c in table.combos {
            assert!(table.act_for(c.ids, 0.5).is_ok(), "{}", c.value());
        }
    }
    assert_eq!(SOLDIER.combos[1].value(), "3005+3007");
}

#[test]
fn scripted_brain_plays_each_bosses_own_plans() {
    let mut b = ScriptedBrain::for_actions(&SOLDIER);
    b.submit(&request(1, 0.0, TallySummary::default()));
    assert_eq!(b.poll(1.0).unwrap().plan, Some(Plan::new(&[3000, 3001])));
    let mut tally = TallySummary::default();
    tally.boss.insert(
        "3000",
        Counts {
            n: 2,
            deflected: 2,
            ..Counts::default()
        },
    );
    b.submit(&request(2, 1.0, tally));
    assert_eq!(b.poll(2.0).unwrap().plan, Some(Plan::new(&[3005, 3007])));
    // The CLI kinds pick the table by chr.
    let mut built = BrainKind::Scripted.build("c1010");
    built.submit(&request(3, 0.0, TallySummary::default()));
    assert_eq!(
        built.poll(1.0).unwrap().plan,
        Some(Plan::new(&[3000, 3001]))
    );
    let mut owl = BrainKind::Scripted.build("c5060");
    owl.submit(&request(4, 0.0, TallySummary::default()));
    assert_eq!(owl.poll(1.0).unwrap().plan, Some(Plan::new(&[3005])));
}

#[test]
fn wolf_keys_are_short() {
    assert_eq!(
        driver::wolf_key("GroundAttackCombo1Release"),
        "AttackCombo1"
    );
    assert_eq!(
        driver::wolf_key("GroundAttackCombo5ReleaseFromStep"),
        "AttackCombo5FromStep"
    );
    assert_eq!(driver::wolf_key("SprintAttack"), "SprintAttack");
}

const DT: f32 = 1.0 / 60.0;

/// Feeds a tracker a scripted Wolf: `frames` of (state, dist, guarding), with `events` at
/// given frame indices.
fn run_wolf(
    tracker: &mut HabitTracker,
    t0: f32,
    frames: &[(&str, f32, bool)],
    events: &[(usize, TallyEvent)],
) -> f32 {
    let mut t = t0;
    for (i, &(state, dist, guarding)) in frames.iter().enumerate() {
        t += DT;
        let now: Vec<TallyEvent> = events
            .iter()
            .filter(|(k, _)| *k == i)
            .map(|(_, e)| e.clone())
            .collect();
        let frame = WolfFrame {
            t,
            dt: DT,
            dist,
            state,
            airborne: false,
            guarding,
            hp: 1.0,
        };
        tracker.step(&frame, &now);
    }
    t
}

fn boss_start(key: &str) -> TallyEvent {
    TallyEvent::Start {
        side: Side::Boss,
        key: key.into(),
    }
}

fn wolf_attack() -> TallyEvent {
    TallyEvent::Start {
        side: Side::Wolf,
        key: "AttackCombo1".into(),
    }
}

#[test]
fn habits_from_a_scripted_wolf() {
    let mut tr = HabitTracker::new(SOLDIER.back_steps);
    let mut t = 0.0;
    // Ten rounds: the soldier steps back (5201, 0.6 s), Wolf follows and attacks from 3 m
    // 0.4 s after the step ends; then the soldier slashes (3000), Wolf deflects it and holds
    // his guard.
    for _ in 0..10 {
        let mut frames = Vec::new();
        for i in 0..60 {
            frames.push(("StandMove", 2.0 + i as f32 / 60.0, false));
        }
        for _ in 0..24 {
            frames.push(("StandMove", 3.0, false));
        }
        for _ in 0..40 {
            frames.push(("GroundAttackCombo1", 3.0, false));
        }
        for _ in 0..90 {
            frames.push(("DeflectGuardIdle", 2.0, true));
        }
        let events = [
            (0, boss_start("5201")),
            (36, TallyEvent::End { side: Side::Boss }),
            (60, wolf_attack()),
            (100, TallyEvent::End { side: Side::Wolf }),
            (124, boss_start("3000")),
            (
                140,
                TallyEvent::Hit {
                    attacker: Side::Boss,
                    guarded: true,
                    deflected: true,
                },
            ),
            (170, TallyEvent::End { side: Side::Boss }),
        ];
        t = run_wolf(&mut tr, t, &frames, &events);
    }
    // A dodge and a jump answer two of the soldier's kicks.
    let frames: Vec<_> = (0..120)
        .map(|i| match i {
            10..30 => ("GroundStep", 3.0, false),
            70..90 => ("GroundJumpStart", 3.0, false),
            _ => ("StandIdle", 3.0, false),
        })
        .collect();
    let events = [
        (0, boss_start("3005")),
        (50, TallyEvent::End { side: Side::Boss }),
        (60, boss_start("3005")),
        (110, TallyEvent::End { side: Side::Boss }),
    ];
    t = run_wolf(&mut tr, t, &frames, &events);
    tr.flush(t);
    let h = &tr.habits;
    let back = &h.after["back_step"];
    assert_eq!((back.n, back.to["attack"]), (10, 10), "{back:?}");
    assert!((back.attack_dist.mean().unwrap() - 3.0).abs() < 0.01);
    let deflect = &h.after["deflect"];
    assert_eq!((deflect.n, deflect.to["guard"]), (10, 10), "{deflect:?}");
    assert_eq!(h.answers["3000"].to["deflect"], 10);
    assert_eq!(h.answers["5201"].to["none"], 10);
    assert_eq!(h.answers["3005"].to["dodge"], 1);
    assert_eq!(h.answers["3005"].to["jump"], 1);
    assert_eq!(h.jumps, 1);
    // Wolf attacks 0.4 s after each back step ends.
    let p = h.punish["5201"];
    assert_eq!(p.n, 10);
    assert!((p.mean().unwrap() - 0.4).abs() < 0.02, "{p:?}");
    let s = h.summary();
    let json = serde_json::to_string(&s).unwrap();
    eprintln!("{json}");
    assert_eq!(s.after["back_step"], "attack 100% from 3.0m n10");
    assert_eq!(s.after["deflect"], "guard 100% n10");
    assert_eq!(s.vs["3000"], "deflect 100% n10");
    assert_eq!(s.attack_m, Some(3.0));
    assert_eq!(s.punish_s["5201"], 0.4);
    assert_eq!(s.vs["3005"], "dodge 50%, jump 50% n2");
}

#[test]
fn heals_and_low_hp_spells() {
    let mut tr = HabitTracker::new(&[5201]);
    let mut t = 0.0;
    for (state, hp) in [
        ("StandIdle", 0.9),
        ("StandIdle", 0.4),
        ("ItemDrink", 0.4),
        ("StandIdle", 0.8),
        ("StandIdle", 0.3),
        ("StandIdle", 0.7),
    ] {
        for _ in 0..30 {
            t += DT;
            tr.step(
                &WolfFrame {
                    t,
                    dt: DT,
                    dist: 4.0,
                    state,
                    airborne: false,
                    guarding: false,
                    hp,
                },
                &[],
            );
        }
    }
    let heals = tr.habits.heals;
    assert_eq!((heals.hp.n, heals.at_low, heals.low_spells), (1, 1, 2));
    assert_eq!(
        tr.habits.summary().heal.unwrap(),
        "1 heals, mean hp 0.40; healed in 1 of 2 low-hp spells"
    );
}

fn temp_file(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sekiro-brain-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

/// A fight's tally and habits: the soldier's 3000 deflected once, Wolf's slash landing.
fn one_fight() -> (Tally, HabitTracker) {
    let mut t = Tally::default();
    let mut h = HabitTracker::new(SOLDIER.back_steps);
    let events = [
        (0, boss_start("3000")),
        (
            20,
            TallyEvent::Hit {
                attacker: Side::Boss,
                guarded: true,
                deflected: true,
            },
        ),
        (40, TallyEvent::End { side: Side::Boss }),
        (50, wolf_attack()),
        (
            60,
            TallyEvent::Hit {
                attacker: Side::Wolf,
                guarded: false,
                deflected: false,
            },
        ),
    ];
    let frames = vec![("StandIdle", 2.0, false); 120];
    for (i, e) in &events {
        t.record(*i as f32 * DT, e.clone());
    }
    let end = run_wolf(&mut h, 0.0, &frames, &events);
    t.flush(end);
    h.flush(end);
    (t, h)
}

#[test]
fn memory_round_trips_and_merges_fights() {
    let path = temp_file("c1010.json");
    let (tally, habits) = one_fight();
    let mut m = Memory::new("c1010");
    m.add_fight(&tally, &habits.habits, FightEnd::BossWon);
    m.save(&path).unwrap();
    let (mut back, how) = Memory::load(&path, "c1010");
    assert_eq!(how, Loaded::Read);
    assert_eq!(back, m);
    back.add_fight(&tally, &habits.habits, FightEnd::WolfWon);
    back.add_fight(&tally, &habits.habits, FightEnd::Unfinished);
    back.save(&path).unwrap();
    let (again, _) = Memory::load(&path, "c1010");
    assert_eq!((again.fights, again.boss_wins, again.wolf_wins), (3, 1, 1));
    assert_eq!(again.boss["3000"].n, 3);
    assert_eq!(again.boss["3000"].deflected, 3);
    assert_eq!(again.wolf["AttackCombo1"].hit, 3);
    assert_eq!(again.habits.answers["3000"].to["deflect"], 3);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("\"version\": 1"), "{text}");
    assert!(!path.with_extension("json.tmp").exists());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn corrupt_or_foreign_memory_starts_fresh_and_is_kept_aside() {
    for (text, want) in [
        ("{ not json", "bad JSON"),
        (r#"{"version":99,"chr":"c1010"}"#, "format version 99"),
        (r#"{"version":1,"chr":"c5060"}"#, "memory of c5060"),
    ] {
        let path = temp_file("c1010.json");
        std::fs::write(&path, text).unwrap();
        let (m, how) = Memory::load(&path, "c1010");
        assert_eq!(m, Memory::new("c1010"));
        match how {
            Loaded::Corrupt(why) => assert!(why.contains(want), "{why}"),
            other => panic!("{other:?}"),
        }
        let mut aside = path.as_os_str().to_owned();
        aside.push(".corrupt");
        assert_eq!(std::fs::read_to_string(&aside).unwrap(), text);
        assert!(!path.exists());
        let _ = std::fs::remove_file(aside);
    }
    let missing = temp_file("none.json");
    assert_eq!(Memory::load(&missing, "c1010").1, Loaded::Missing);
}

#[test]
fn memory_paths_from_the_flag() {
    use std::path::PathBuf;
    let def = PathBuf::from("logs/memory/c1010.json");
    assert_eq!(memory::resolve(None, "c1010", true), Some(def.clone()));
    assert_eq!(memory::resolve(None, "c1010", false), None);
    assert_eq!(memory::resolve(Some("off"), "c1010", true), None);
    assert_eq!(memory::resolve(Some("on"), "c1010", false), Some(def));
    assert_eq!(
        memory::resolve(Some("m/me.json"), "c1010", true),
        Some(PathBuf::from("m/me.json"))
    );
    assert_eq!(
        memory::resolve(Some("m"), "c1010", true),
        Some(PathBuf::from("m").join("c1010.json"))
    );
}

/// A memory with every map full: many fights of every soldier action, many Wolf attacks and
/// every habit.
fn full_memory() -> Memory {
    let mut m = Memory::new("c1010");
    m.fights = 123;
    m.boss_wins = 45;
    m.wolf_wins = 67;
    let counts = |n| Counts {
        n,
        hit: n / 3,
        blocked: n / 4,
        deflected: n / 5,
        whiffed: n / 6,
        punished: n / 7,
    };
    for (i, a) in SOLDIER.actions.iter().enumerate() {
        m.boss.insert(a.id.to_string(), counts(100 + i as u32));
        let r = m.habits.answers.entry(a.id.to_string()).or_default();
        r.n = 100;
        r.to.insert("deflect".into(), 61);
        r.to.insert("dodge".into(), 39);
        m.habits
            .punish
            .insert(a.id.to_string(), habits::Mean { sum: 41.7, n: 100 });
    }
    for w in [
        "AttackCombo1",
        "AttackCombo2",
        "AttackCombo3",
        "AttackCombo5FromStep",
        "SprintAttack",
        "JumpAttack",
    ] {
        m.wolf.insert(w.into(), counts(150));
    }
    for trig in ["deflect", "guard", "guarded", "back_step"] {
        let r = m.habits.after.entry(trig.into()).or_default();
        r.n = 100;
        r.to.insert("attack".into(), 77);
        r.to.insert("wait".into(), 23);
        r.attack_dist = habits::Mean { sum: 312.0, n: 77 };
    }
    m.habits.seconds = 3600.0;
    m.habits.range_seconds = [1200.0, 1800.0, 600.0];
    m.habits.attack_dist = habits::Mean {
        sum: 2900.0,
        n: 1000,
    };
    m.habits.jumps = 90;
    m.habits.heals.hp = habits::Mean { sum: 12.3, n: 40 };
    m.habits.heals.at_low = 30;
    m.habits.heals.low_spells = 41;
    m
}

#[test]
fn requests_fit_the_budget() {
    let mut tally = Tally::default();
    for (i, a) in SOLDIER.actions.iter().enumerate() {
        for k in 0..=i {
            tally.record(k as f32, boss_start(&a.id.to_string()));
            tally.record(k as f32 + 0.5, TallyEvent::End { side: Side::Boss });
        }
    }
    for w in ["AttackCombo1", "AttackCombo2", "SprintAttack", "JumpAttack"] {
        tally.record(
            0.0,
            TallyEvent::Start {
                side: Side::Wolf,
                key: w.into(),
            },
        );
    }
    tally.flush(100.0);
    let mut req = BrainRequest {
        id: 123456,
        obs: widest_observation(),
        tally: tally.summary(),
        memory: Some(full_memory().summary()),
    };
    let untrimmed = req.json_len();
    let len = req.fit(REQUEST_BUDGET);
    let json = serde_json::to_string(&req).unwrap();
    eprintln!("untrimmed {untrimmed} bytes, fitted {len}: {json}");
    assert!(untrimmed > REQUEST_BUDGET, "the test should need trimming");
    assert!(len <= REQUEST_BUDGET, "{len}");
    assert_eq!(len, json.len());
    // The most telling entries survive: this fight's busiest boss action, the habits'
    // responses and single values, the fight count.
    assert!(json.contains("\"5211\":[15,"), "{json}");
    let m = req.memory.as_ref().unwrap();
    assert!(!m.habits.after.is_empty(), "{json}");
    assert!(m.habits.attack_m.is_some() && m.habits.range_pct.is_some());
    assert_eq!(m.fights, 123);
    // A typical request (one earlier fight, this fight's few actions) needs no trimming.
    let (t1, h1) = one_fight();
    let mut small = Memory::new("c1010");
    small.add_fight(&t1, &h1.habits, FightEnd::BossWon);
    let mut typical = BrainRequest {
        id: 12,
        obs: widest_observation(),
        tally: t1.summary(),
        memory: Some(small.summary()),
    };
    let before = typical.json_len();
    eprintln!(
        "typical {before} bytes: {}",
        serde_json::to_string(&typical).unwrap()
    );
    assert_eq!(typical.fit(REQUEST_BUDGET), before);
    assert!(before < 900, "{before}");
}

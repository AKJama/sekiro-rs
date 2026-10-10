use super::actions::{OWL, PlanError};
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
    start(&mut t, 0.0, Side::Owl, "3005");
    hit(&mut t, 0.4, Side::Owl, false, false);
    t.record(1.0, TallyEvent::End { side: Side::Owl });
    start(&mut t, 2.0, Side::Owl, "3005");
    hit(&mut t, 2.4, Side::Owl, true, true);
    t.record(3.0, TallyEvent::End { side: Side::Owl });
    // Wolf's slash lands 0.5 s after the deflected kick ended: punished, and a hit for Wolf.
    start(&mut t, 3.2, Side::Wolf, "GroundAttackCombo1Release");
    hit(&mut t, 3.5, Side::Wolf, false, false);
    t.record(3.8, TallyEvent::End { side: Side::Wolf });
    start(&mut t, 5.0, Side::Owl, "3031");
    start(&mut t, 6.0, Side::Owl, "3009");
    t.record(6.5, TallyEvent::End { side: Side::Owl });
    hit(&mut t, 6.9, Side::Owl, true, false);
    let done = t.advance(10.0);
    assert_eq!(done.len(), 5);
    let kick = t.counts(Side::Owl, "3005");
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
    let throw = t.counts(Side::Owl, "3031");
    assert_eq!((throw.n, throw.whiffed, throw.punished), (1, 1, 0));
    let shuriken = t.counts(Side::Owl, "3009");
    assert_eq!((shuriken.n, shuriken.blocked, shuriken.whiffed), (1, 1, 0));
    let slash = t.counts(Side::Wolf, "GroundAttackCombo1Release");
    assert_eq!((slash.n, slash.hit, slash.punished), (1, 1, 0));
}

#[test]
fn punish_window_and_recent_window() {
    let mut t = Tally::default();
    t.recent_len = 2;
    start(&mut t, 0.0, Side::Owl, "3000");
    t.record(1.0, TallyEvent::End { side: Side::Owl });
    // 1.5 s after the end: outside the 1 s window, not a punish.
    hit(&mut t, 2.5, Side::Wolf, false, false);
    for (i, key) in ["3001", "3002", "3002"].iter().enumerate() {
        let s = 3.0 + i as f32 * 2.0;
        start(&mut t, s, Side::Owl, key);
        t.record(s + 1.0, TallyEvent::End { side: Side::Owl });
    }
    t.advance(20.0);
    assert_eq!(t.counts(Side::Owl, "3000").punished, 0);
    let s = t.summary();
    assert_eq!(s.owl.len(), 3);
    // Only the last two exchanges are recent.
    assert_eq!(s.owl_recent.len(), 1);
    assert_eq!(s.owl_recent["3002"].n, 2);
    assert_eq!(
        serde_json::to_string(&s.owl["3002"]).unwrap(),
        "[2,0,0,0,2,0]"
    );
}

#[test]
fn exchanges_wait_out_the_punish_window() {
    let mut t = Tally::default();
    start(&mut t, 0.0, Side::Owl, "3005");
    t.record(1.0, TallyEvent::End { side: Side::Owl });
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
    tally.owl.insert(
        "3005".into(),
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

#[test]
fn observation_json_stays_small() {
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
    let obs = Observation {
        t: 1234.56,
        owl: side(Some(3063)),
        wolf: side(None),
        dist: 12.34,
        dy: -1.25,
        wolf_bearing: -179.0,
        owl_bearing: 135.0,
        locked_on: true,
        since_owl_hit: Some(12.34),
        since_wolf_hit: Some(0.5),
        plan: vec![3000, 3001, 3002],
    };
    let json = serde_json::to_string(&obs).unwrap();
    assert!(json.len() < 1024, "{} bytes: {json}", json.len());
    assert!(json.contains("\"dist\":12.34"), "{json}");
}

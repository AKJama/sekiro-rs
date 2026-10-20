use sekiro_sim::InputFrame;
use sekiro_sim::deathblow::Placement;
use sekiro_sim::duel::idol::IdolPhase;
use sekiro_sim::duel::{Duel, RESPAWN_DELAY};

const DT: f32 = 1.0 / 60.0;

/// Steps until `done`, at most `seconds`; the phases passed on the way.
fn run(duel: &mut Duel, seconds: f32, done: impl Fn(&Duel) -> bool) -> Vec<Option<IdolPhase>> {
    let mut seen = vec![duel.idol_phase()];
    for _ in 0..(seconds / DT) as usize {
        duel.step(&InputFrame::default(), DT);
        if seen.last() != Some(&duel.idol_phase()) {
            seen.push(duel.idol_phase());
        }
        if done(duel) {
            return seen;
        }
    }
    panic!("not done in {seconds} s: {seen:?}");
}

#[test]
fn wolf_kneels_rests_stands_and_comes_back_at_the_idol() {
    let cache = sekiro_sim::cache::root();
    if !cache.join("anim/c0000/a000_710310.bin").exists() {
        eprintln!("skipping: cache not extracted");
        return;
    }
    let mut duel = Duel::load(&cache, 30.0).unwrap();
    duel.enemy_present = false;
    duel.enemy_active = false;
    run(&mut duel, 1.0, |d| d.player.body.grounded);
    let start = duel.player.body.position;
    let kneel = Placement {
        position: [start[0] + 2.0, start[1], start[2]],
        yaw: 1.0,
    };
    let checkpoint = Placement {
        position: [start[0] + 2.0, start[1], start[2] + 0.5],
        yaw: -2.0,
    };
    duel.rules.fighters[0].vitality.hp = 10;
    assert!(duel.kneel_at_idol(kneel, checkpoint));
    assert_eq!(duel.player.body.position, kneel.position);
    let max = duel.rules.fighters[0].vitality.max_hp;
    assert_eq!(duel.rules.fighters[0].vitality.hp, max);
    // A second press while kneeling does nothing.
    assert!(!duel.kneel_at_idol(kneel, checkpoint));

    let praying = |d: &Duel| d.idol_phase() == Some(IdolPhase::Praying);
    let seen = run(&mut duel, 3.0, praying);
    assert_eq!(
        seen,
        [None, Some(IdolPhase::Kneeling), Some(IdolPhase::Praying)]
    );
    duel.rules.fighters[0].vitality.hp = 10;
    duel.rest_at_idol();
    assert_eq!(duel.rules.fighters[0].vitality.hp, max);
    // Only once: the second press waits for the first.
    duel.rest_at_idol();
    let seen = run(&mut duel, 5.0, praying);
    assert_eq!(
        seen,
        [
            Some(IdolPhase::Praying),
            Some(IdolPhase::Resting),
            Some(IdolPhase::Praying)
        ]
    );
    duel.leave_idol();
    let seen = run(&mut duel, 3.0, |d| d.idol_phase().is_none());
    assert_eq!(
        seen,
        [Some(IdolPhase::Praying), Some(IdolPhase::Leaving), None]
    );

    // A death brings Wolf back at the idol, not at the start.
    duel.rules.fighters[0].vitality.hp = 0;
    run(&mut duel, RESPAWN_DELAY + 1.0, |d| {
        d.respawn_in.is_none() && !d.rules.fighters[0].dead()
    });
    assert_eq!(duel.player.body.position, checkpoint.position);
}

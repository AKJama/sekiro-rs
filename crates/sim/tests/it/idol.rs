use sekiro_sim::InputFrame;
use sekiro_sim::deathblow::Placement;
use sekiro_sim::duel::idol::{IdolPhase, IdolSpot, LIGHTING_SECS, LitKey};
use sekiro_sim::duel::{Duel, RESPAWN_DELAY};

const DT: f32 = 1.0 / 60.0;

/// Steps with `input` until `done`, at most `seconds`; the phases passed on the way.
fn run_with(
    duel: &mut Duel,
    input: &InputFrame,
    seconds: f32,
    done: impl Fn(&Duel) -> bool,
) -> Vec<Option<IdolPhase>> {
    let mut seen = vec![duel.idol_phase()];
    for _ in 0..(seconds / DT) as usize {
        duel.step(input, DT);
        if seen.last() != Some(&duel.idol_phase()) {
            seen.push(duel.idol_phase());
        }
        if done(duel) {
            return seen;
        }
    }
    panic!("not done in {seconds} s: {seen:?}");
}

fn run(duel: &mut Duel, seconds: f32, done: impl Fn(&Duel) -> bool) -> Vec<Option<IdolPhase>> {
    run_with(duel, &InputFrame::default(), seconds, done)
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
    let spot = IdolSpot {
        kneel: Placement {
            position: [start[0] + 1.0, start[1], start[2]],
            yaw: 1.0,
        },
        checkpoint: Placement {
            position: [start[0] + 1.0, start[1], start[2] + 0.5],
            yaw: -2.0,
        },
        lit: LitKey::Flag(11100000),
    };
    duel.set_idols(vec![spot]);
    assert_eq!(duel.idol_in_reach(), Some(0));

    // The action press kneels him at the spot with full Vitality; holding it does nothing more.
    duel.rules.fighters[0].vitality.hp = 10;
    let mut press = InputFrame::default();
    press.buttons.action = true;
    duel.step(&press, DT);
    assert_eq!(duel.idol(), Some(0));
    assert_eq!(duel.idol_in_reach(), None);
    let [x, _, z] = duel.player.body.position;
    assert!((x - spot.kneel.position[0]).hypot(z - spot.kneel.position[2]) < 0.05);
    let max = duel.rules.fighters[0].vitality.max_hp;
    assert_eq!(duel.rules.fighters[0].vitality.hp, max);
    // The first kneel lights the idol: Wolf stays held while its "found" banner plays, and
    // the menu's Rest and Esc do nothing until it is over.
    assert_eq!(duel.idol_phase(), Some(IdolPhase::Lighting));
    assert_eq!(duel.idol_lighting(), Some(0.0));
    duel.rest_at_idol();
    duel.leave_idol();

    let praying = |d: &Duel| d.idol_phase() == Some(IdolPhase::Praying);
    // His own input is held while he is at the idol: the stick does not walk him away.
    let mut walk = press;
    walk.move_stick = [0.0, 1.0];
    // The banner's whole clip, to the frame.
    for _ in 0..(LIGHTING_SECS / DT) as usize - 2 {
        duel.step(&walk, DT);
        assert_eq!(duel.idol_phase(), Some(IdolPhase::Lighting));
    }
    let seen = run_with(&mut duel, &walk, 4.0 * DT, praying);
    assert_eq!(seen, [Some(IdolPhase::Lighting), Some(IdolPhase::Praying)]);
    assert_eq!(duel.idol_lighting(), None);
    let [x, _, z] = duel.player.body.position;
    assert!((x - spot.kneel.position[0]).hypot(z - spot.kneel.position[2]) < 0.05);
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
    duel.step(&InputFrame::default(), DT);
    assert_eq!(duel.idol(), None);

    // A death brings Wolf back at the idol, not at the start.
    duel.rules.fighters[0].vitality.hp = 0;
    run(&mut duel, RESPAWN_DELAY + 1.0, |d| {
        d.respawn_in.is_none() && !d.rules.fighters[0].dead()
    });
    assert_eq!(duel.player.body.position, spot.checkpoint.position);

    // It stays lit through the respawn: kneeling there again goes straight to the prayer.
    run(&mut duel, 2.0, |d| d.idol_in_reach() == Some(0));
    duel.step(&press, DT);
    assert_eq!(duel.idol(), Some(0));
    let seen = run(&mut duel, 3.0, praying);
    assert_eq!(seen, [Some(IdolPhase::Kneeling), Some(IdolPhase::Praying)]);
}

#[test]
fn the_nearer_item_takes_the_press_and_stays_taken_through_a_rest() {
    let cache = sekiro_sim::cache::root();
    if !cache.join("anim/c0000/a000_710310.bin").exists()
        || !cache.join("raw/map/mapstudio/m11_00_00_00.msb").exists()
    {
        eprintln!("skipping: cache not extracted");
        return;
    }
    let data = std::sync::Arc::new(sekiro_sim::items::ItemData::load(&cache).unwrap());
    let mut pellets = sekiro_sim::items::map_pickups(&cache, "m11_00_00_00", &data)
        .unwrap()
        .into_iter()
        .find(|p| p.name == "Item_宝_02")
        .unwrap();
    assert_eq!((pellets.lot.id, pellets.lot.flag), (1100020, 51100020));
    let mut duel = Duel::load(&cache, 30.0).unwrap();
    duel.enemy_present = false;
    duel.enemy_active = false;
    duel.kit.goods.retain(|(id, _)| *id != 3020);
    run(&mut duel, 1.0, |d| d.player.body.grounded);
    let start = duel.player.body.position;
    // The Pellets half a metre off, an idol's kneeling spot a metre off.
    pellets.position = [start[0] + 0.5, start[1], start[2]];
    duel.set_pickups(vec![pellets], data);
    duel.set_idols(vec![IdolSpot {
        kneel: Placement {
            position: [start[0] + 1.0, start[1], start[2]],
            yaw: 1.0,
        },
        checkpoint: Placement {
            position: [start[0] + 1.0, start[1], start[2] + 0.5],
            yaw: -2.0,
        },
        lit: LitKey::Flag(11100000),
    }]);
    assert_eq!(duel.pickup_in_reach(), Some(0));
    assert_eq!(duel.idol_in_reach(), None);

    let mut press = InputFrame::default();
    press.buttons.action = true;
    duel.step(&press, DT);
    assert_eq!(duel.kit.count(3020), 2);
    assert!(duel.kit.taken_flags.contains(&51100020));
    assert_eq!(duel.idol(), None);
    // Taken, it leaves the press to the idol; a Rest does not bring it back.
    duel.step(&InputFrame::default(), DT);
    duel.step(&press, DT);
    assert_eq!(duel.idol(), Some(0));
    let praying = |d: &Duel| d.idol_phase() == Some(IdolPhase::Praying);
    run(&mut duel, 5.0, praying);
    duel.rest_at_idol();
    run(&mut duel, 0.5, |d| {
        d.idol_phase() == Some(IdolPhase::Resting)
    });
    run(&mut duel, 5.0, praying);
    assert!(!duel.pickup_lies(0));
    assert_eq!(duel.kit.count(3020), 2);
}

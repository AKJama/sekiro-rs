//! Wolf against an Ashina soldier: hits, blocks and a perfect deflect, all driven by both
//! characters' own scripts and graphs. Skips when `cache/` has not been extracted.

use sekiro_sim::duel::{Duel, DuelReport};
use sekiro_sim::{Buttons, InputFrame};
use std::path::PathBuf;

const DT: f32 = 1.0 / 60.0;

fn load() -> Option<Duel> {
    let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../cache");
    if !cache.join("raw/chr/c1010.behbnd.d/c9997.hkx").exists() {
        eprintln!("skipping: cache not extracted");
        return None;
    }
    Some(Duel::load(&cache, 4.0).unwrap())
}

fn frame(guard: bool, attack: bool) -> InputFrame {
    InputFrame {
        buttons: Buttons {
            guard,
            attack,
            ..Buttons::default()
        },
        ..InputFrame::default()
    }
}

fn run(
    duel: &mut Duel,
    steps: usize,
    mut input: impl FnMut(usize) -> InputFrame,
) -> Vec<DuelReport> {
    (0..steps)
        .map(|i| {
            let r = duel.step(&input(i), DT);
            assert!(
                r.player.behavior.script_errors.is_empty(),
                "{:?}",
                r.player.behavior.script_errors
            );
            assert!(
                r.enemy.behavior.script_errors.is_empty(),
                "{:?}",
                r.enemy.behavior.script_errors
            );
            r
        })
        .collect()
}

fn states(reports: &[DuelReport], enemy: bool) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for r in reports {
        let s = if enemy { &r.enemy } else { &r.player };
        let name = s.behavior.state_path.last().cloned().unwrap_or_default();
        if out.last() != Some(&name) {
            out.push(name);
        }
    }
    out
}

#[test]
fn soldier_attacks_and_guard_blocks_or_deflects() {
    let Some(mut duel) = load() else { return };
    // Held guard from the start: the soldier's first attack is blocked.
    let reports = run(&mut duel, 600, |i| frame(i >= 60, false));
    let hits: Vec<_> = reports.iter().flat_map(|r| r.hits.clone()).collect();
    let (_, blocked) = hits
        .iter()
        .find(|(h, _)| h.attacker == 1)
        .expect("the soldier never hit");
    assert!(blocked.guarded && !blocked.deflected, "{blocked:?}");
    assert!(
        states(&reports, false)
            .iter()
            .any(|s| s.starts_with("StandDeflectEasy"))
    );
    assert!(
        states(&reports, true)
            .iter()
            .any(|s| s.starts_with("AttackNoBound"))
    );
}

#[test]
fn timed_guard_press_deflects() {
    let Some(mut duel) = load() else { return };
    // Find when the soldier's first hit would land, then replay pressing guard just before it.
    let probe = run(&mut duel, 700, |_| frame(false, false));
    let hit_at = probe
        .iter()
        .position(|r| r.hits.iter().any(|(h, _)| h.attacker == 1))
        .expect("the soldier never hit");
    let mut duel = load().unwrap();
    let press = hit_at.saturating_sub(8);
    let reports = run(&mut duel, hit_at + 60, |i| frame(i >= press, false));
    let deflect = reports
        .iter()
        .flat_map(|r| r.hits.clone())
        .find(|(h, _)| h.attacker == 1)
        .expect("no hit after the press");
    assert!(deflect.1.deflected, "{:?}", deflect.1);
    assert!(
        states(&reports, false)
            .iter()
            .any(|s| s.starts_with("StandDeflectHard"))
    );
    assert!(
        states(&reports, true)
            .iter()
            .any(|s| s.starts_with("AttackBoundEnemy"))
    );
}

#[test]
fn wolf_slash_staggers_the_soldier() {
    let Some(mut duel) = load() else { return };
    duel.enemy_active = false;
    // Walk up to the idle soldier, then attack twice.
    let reports = run(&mut duel, 400, |i| match i {
        0..=90 => InputFrame {
            move_stick: [0.0, 1.0],
            ..InputFrame::default()
        },
        120 | 160 => frame(false, true),
        _ => InputFrame::default(),
    });
    let wolf_hits = reports
        .iter()
        .flat_map(|r| r.hits.clone())
        .filter(|(h, _)| h.attacker == 0)
        .count();
    assert!(wolf_hits >= 1, "the slash never connected");
    assert!(
        states(&reports, true)
            .iter()
            .any(|s| s.starts_with("Damage")),
        "{:?}",
        states(&reports, true)
    );
}

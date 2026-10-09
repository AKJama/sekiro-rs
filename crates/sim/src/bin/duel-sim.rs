//! `duel-sim [--script FILE] [--steps N] [--passive]`: Wolf (input script, default: guard on and
//! off) against an Ashina soldier with the stand-in brain. Prints state changes and every hit.

use sekiro_sim::duel::Duel;
use sekiro_sim::{InputFrame, demo};
use std::path::Path;

const DEFAULT: &str = "\
# stand still, then hold guard for a while, then attack the soldier
120 0 0 -
1 0 0 guard
300 0 0 guard
60 0 0 -
1 0 0 attack
40 0 0 -
1 0 0 attack
40 0 0 -
1 0 0 attack
200 0 0 -
";

fn main() {
    let mut script = None;
    let mut steps = None;
    let mut passive = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--script" => script = Some(std::fs::read_to_string(args.next().unwrap()).unwrap()),
            "--steps" => steps = args.next().and_then(|s| s.parse().ok()),
            "--passive" => passive = true,
            other => panic!("unknown argument {other}"),
        }
    }
    let frames = demo::parse(script.as_deref().unwrap_or(DEFAULT)).unwrap();
    let mut duel = Duel::load(Path::new("cache"), 4.0).expect("load");
    duel.enemy_active = !passive;
    println!(
        "player rig {} dummies, {} attacks; enemy rig {} dummies, {} attacks",
        duel.fighters[0].rig.len(),
        duel.fighters[0].attacks.len(),
        duel.fighters[1].rig.len(),
        duel.fighters[1].attacks.len()
    );
    let n = steps.unwrap_or(frames.len());
    let (mut lp, mut le) = (String::new(), String::new());
    for i in 0..n {
        let f = frames.get(i).copied().unwrap_or_default();
        let f = InputFrame { ..f };
        let r = duel.step(&f, 1.0 / 60.0);
        let kp = format!(
            "{} {}",
            r.player.behavior.state_path.join("/"),
            r.player.animation.as_deref().unwrap_or("-")
        );
        let ke = format!(
            "{} {}",
            r.enemy.behavior.state_path.join("/"),
            r.enemy.animation.as_deref().unwrap_or("-")
        );
        if kp != lp {
            println!("{i:5} WOLF    {kp}");
            lp = kp;
        }
        if ke != le {
            println!("{i:5} SOLDIER {ke}");
            le = ke;
        }
        for (h, res) in &r.hits {
            println!(
                "{i:5} HIT {} -> {} judge {} atk {} guarded={} deflected={} defender type {} level {} hp {:?}",
                ["wolf", "soldier"][h.attacker],
                ["wolf", "soldier"][h.defender],
                h.judge,
                h.attack.atk_id,
                res.guarded,
                res.deflected,
                res.defender.damage_type,
                res.defender.level,
                duel.hp
            );
        }
        for e in r
            .player
            .behavior
            .script_errors
            .iter()
            .chain(&r.enemy.behavior.script_errors)
        {
            println!("{i:5} SCRIPT ERROR {e}");
        }
    }
}

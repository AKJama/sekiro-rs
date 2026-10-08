//! `wolf-sim [--script FILE] [--cache DIR] [--all]`: plays an input script (default: the built-in
//! demo) through the full character simulation at 60 steps per second and prints every change of
//! state or clip, with position and active behaviour references.

use sekiro_sim::{PlayerCharacter, demo};
use std::path::PathBuf;

fn main() {
    let mut cache = PathBuf::from("cache");
    let mut script = None;
    let mut all = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--cache" => cache = PathBuf::from(args.next().expect("--cache DIR")),
            "--script" => script = Some(args.next().expect("--script FILE")),
            "--all" => all = true,
            other => panic!("unknown argument {other}"),
        }
    }
    let text = match script {
        Some(path) => std::fs::read_to_string(path).expect("read script"),
        None => demo::DEMO.to_owned(),
    };
    let frames = demo::parse(&text).expect("parse script");
    let t0 = std::time::Instant::now();
    let mut wolf = PlayerCharacter::load_from_cache(&cache).expect("load");
    let dt = 1.0 / 60.0;
    let settled = wolf.settle(dt, 400);
    println!(
        "loaded in {:?}, settled to idle after {settled} steps",
        t0.elapsed()
    );
    let mut last = String::new();
    for (i, f) in frames.iter().enumerate() {
        let r = wolf.tick(f, dt);
        let key = format!(
            "{} {}",
            r.behavior.state_path.join("/"),
            r.animation.as_deref().unwrap_or("-")
        );
        if all || key != last || !r.behavior.script_errors.is_empty() {
            println!(
                "{i:5} stick=({:+.0},{:+.0}) {:<44} {:<12} t={:4.2} pos=({:+6.2},{:+5.2},{:+6.2}) yaw={:+4.0} {}refs={:?}{}",
                f.move_stick[0],
                f.move_stick[1],
                r.behavior.state_path.join("/"),
                r.animation.as_deref().unwrap_or("-"),
                r.anim_time,
                r.position[0],
                r.position[1],
                r.position[2],
                r.yaw.to_degrees(),
                if r.grounded { "" } else { "AIR " },
                r.behavior_refs,
                if r.behavior.script_errors.is_empty() {
                    String::new()
                } else {
                    format!(" ERRORS {:?}", r.behavior.script_errors)
                }
            );
        }
        last = key;
    }
}

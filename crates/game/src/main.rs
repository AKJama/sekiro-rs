//! `sekiro-rs`: the Bevy app.
//!
//! Modes:
//! - `sekiro-rs --viewer <file.glb> [--screenshot <out.png>] [--yaw <deg>] [--pitch <deg>]
//!   [--distance <m>] [--height <m>] [--hide m12,hair] [--only m68,m69]`: inspect an exported
//!   model in bind pose; `--hide`/`--only` filter primitives by material name, `--combat` shows
//!   enemies with weapons drawn.
//! - `sekiro-rs --map <id> [--screenshot <out.png>] [--at <s>] [--camera x,y,z,yaw,pitch]
//!   [--start <n>] [--collision] [--no-pieces] [--hide m9,m8] [--groups display|draw|off]
//!   [--uncapped]`: fly through an area exported by `sekiro-extract map`.

mod arena;
mod character;
mod deathblow_demo;
mod draw_mask;
mod map;
mod play;
mod viewer;

use std::path::PathBuf;

fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == name).map(|w| w[1].clone())
}

fn arg_list(args: &[String], name: &str) -> Vec<String> {
    arg_value(args, name)
        .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default()
}

fn arg_f32(args: &[String], name: &str, default: f32) -> anyhow::Result<f32> {
    match arg_value(args, name) {
        Some(v) => v.parse().map_err(|e| anyhow::anyhow!("{name} {v}: {e}")),
        None => Ok(default),
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(glb) = arg_value(&args, "--viewer") {
        let options = viewer::Options {
            glb: PathBuf::from(glb),
            screenshot: arg_value(&args, "--screenshot").map(PathBuf::from),
            yaw_degrees: arg_f32(&args, "--yaw", 180.0)?,
            pitch_degrees: arg_f32(&args, "--pitch", 8.0)?,
            distance: arg_f32(&args, "--distance", 3.4)?,
            height: arg_f32(&args, "--height", 0.95)?,
            hide: arg_list(&args, "--hide"),
            only: arg_list(&args, "--only"),
            combat: args.iter().any(|a| a == "--combat"),
        };
        return viewer::run(options);
    }
    if let Some(id) = arg_value(&args, "--map") {
        return map::run(map::Options {
            cache: PathBuf::from(arg_value(&args, "--cache").unwrap_or_else(|| "cache".into())),
            id,
            screenshot: arg_value(&args, "--screenshot").map(PathBuf::from),
            at: arg_f32(&args, "--at", 2.0)?,
            camera: map::parse_camera(arg_value(&args, "--camera").as_deref())?,
            collision: args.iter().any(|a| a == "--collision"),
            no_pieces: args.iter().any(|a| a == "--no-pieces"),
            start: arg_f32(&args, "--start", 0.0)? as usize,
            hide: arg_list(&args, "--hide"),
            uncapped: args.iter().any(|a| a == "--uncapped"),
            groups: map::GroupMode::parse(arg_value(&args, "--groups").as_deref())?,
        });
    }
    if args.iter().any(|a| a == "--play") {
        let screenshots = arg_list(&args, "--shots")
            .into_iter()
            .map(|s| {
                let (step, path) = s
                    .split_once(':')
                    .ok_or_else(|| anyhow::anyhow!("--shots wants step:path, got {s}"))?;
                Ok((step.parse()?, PathBuf::from(path)))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        return play::run(play::Options {
            cache: PathBuf::from(arg_value(&args, "--cache").unwrap_or_else(|| "cache".into())),
            script: arg_value(&args, "--script"),
            screenshots,
            exit_after_script: args.iter().any(|a| a == "--exit"),
        });
    }
    if args.iter().any(|a| a == "--arena") {
        return arena::run(arena::Options {
            cache: PathBuf::from(arg_value(&args, "--cache").unwrap_or_else(|| "cache".into())),
            screenshot: arg_value(&args, "--screenshot").map(PathBuf::from),
            at: arg_f32(&args, "--at", 3.0)?,
            deathblow: args.iter().any(|a| a == "--deathblow"),
        });
    }
    eprintln!("usage: sekiro-rs --play [--script FILE|builtin] [--shots step:path,...] [--exit]");
    eprintln!("usage: sekiro-rs --arena [--deathblow] [--screenshot <out.png> --at <seconds>]");
    eprintln!("       sekiro-rs --viewer <file.glb> [--screenshot <out.png>] [--yaw <deg>]");
    eprintln!("                 [--pitch <deg>] [--distance <m>] [--height <m>]");
    std::process::exit(2);
}

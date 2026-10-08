//! `sekiro-rs`: the Bevy app.
//!
//! Modes:
//! - `sekiro-rs --viewer <file.glb> [--screenshot <out.png>] [--yaw <deg>] [--pitch <deg>]
//!   [--distance <m>] [--height <m>] [--hide m12,hair] [--only m68,m69]`: inspect an exported
//!   model in bind pose; `--hide`/`--only` filter primitives by material name.

mod arena;
mod character;
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
        };
        return viewer::run(options);
    }
    if args.iter().any(|a| a == "--arena") {
        return arena::run(arena::Options {
            cache: PathBuf::from(arg_value(&args, "--cache").unwrap_or_else(|| "cache".into())),
            screenshot: arg_value(&args, "--screenshot").map(PathBuf::from),
            at: arg_f32(&args, "--at", 3.0)?,
        });
    }
    eprintln!("usage: sekiro-rs --arena [--screenshot <out.png> --at <seconds>]");
    eprintln!("       sekiro-rs --viewer <file.glb> [--screenshot <out.png>] [--yaw <deg>]");
    eprintln!("                 [--pitch <deg>] [--distance <m>] [--height <m>]");
    std::process::exit(2);
}

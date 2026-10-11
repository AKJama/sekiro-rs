//! `sekiro-rs`: the Bevy app.
//!
//! Modes:
//! - `sekiro-rs --viewer <file.glb> [--screenshot <out.png>] [--yaw <deg>] [--pitch <deg>]
//!   [--distance <m>] [--height <m>] [--hide m12,hair] [--only m68,m69]`: inspect an exported
//!   model in bind pose; `--hide`/`--only` filter primitives by material name, `--combat` shows
//!   enemies with weapons drawn.
//! - `sekiro-rs --arena owl [--full-map] [--script FILE ...]`: Wolf against a boss where the
//!   game places him (`sekiro_sim::arena`); a play mode, see `play.rs`.
//! - `sekiro-rs --map <id> [--screenshot <out.png>] [--at <s>] [--camera x,y,z,yaw,pitch]
//!   [--start <n>] [--collision] [--no-pieces] [--objects] [--hide m9,m8]
//!   [--groups draw|display|off] [--uncapped]`: fly through an area exported by
//!   `sekiro-extract map`.
//!
//! Map lighting: `--hour H` (default 12), `--no-env-probe` for hemisphere fallback,
//! `--probe-sky` to show the environment probe capture as the sky (debug only).
//! `--no-grading`, `--no-bloom`, `--no-auto-exposure` disable individual map post effects.

mod arena;
mod audio;
mod brain_overlay;
mod brain_stage;
mod bullet_fx;
mod character;
mod combat_hud;
mod deathblow_demo;
mod draw_mask;
mod input_replay;
mod lighting;
mod map;
mod perf_log;
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
    for flag in [
        "--attack-power",
        "--progression",
        "--brain-stage",
        "--window-size",
        "--hour",
    ] {
        if args.iter().any(|a| a == flag)
            && arg_value(&args, flag).is_none_or(|v| v.starts_with("--"))
        {
            anyhow::bail!("{flag} requires a value");
        }
    }
    let attack_power = sekiro_sim::progression::AttackPower::from_options(
        arg_value(&args, "--attack-power").as_deref(),
        arg_value(&args, "--progression").as_deref(),
    )
    .map_err(anyhow::Error::msg)?;
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
    if let Some(id) = arg_value(&args, "--map")
        && !args.iter().any(|a| a == "--play")
    {
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
            objects: args.iter().any(|a| a == "--objects"),
            hour: arg_f32(&args, "--hour", 12.0)?,
            gparam: arg_value(&args, "--gparam"),
        });
    }
    // `--arena <name>` (a boss arena, see `sekiro_sim::arena`) is a play mode; a bare `--arena`
    // is the presentation demo below.
    let boss_arena = match arg_value(&args, "--arena").filter(|v| !v.starts_with("--")) {
        Some(name) => Some(
            sekiro_sim::arena::ArenaSpec::by_name(&name)
                .map_err(|e| anyhow::anyhow!("--arena: {e}"))?,
        ),
        None => None,
    };
    if args.iter().any(|a| a == "--play") || boss_arena.is_some() {
        let screenshots = arg_list(&args, "--shots")
            .into_iter()
            .map(|s| {
                let (step, path) = s
                    .split_once(':')
                    .ok_or_else(|| anyhow::anyhow!("--shots wants step:path, got {s}"))?;
                Ok((step.parse()?, PathBuf::from(path)))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let opponent = match (arg_value(&args, "--enemy"), boss_arena) {
            (Some(name), _) => sekiro_sim::enemy::EnemySpec::by_name(&name)
                .map_err(|e| anyhow::anyhow!("--enemy: {e}"))?,
            (None, Some(arena)) => arena.enemy_spec(),
            (None, None) => sekiro_sim::enemy::EnemySpec::soldier(),
        };
        let player = sekiro_sim::brain::memory::player_name(
            &arg_value(&args, "--player").unwrap_or_else(|| "default".into()),
        )
        .map_err(anyhow::Error::msg)?;
        let stage = arg_value(&args, "--brain-stage")
            .map(|name| {
                brain_stage::load(
                    &name,
                    &opponent.chr,
                    &player,
                    std::path::Path::new("logs/datasets"),
                )
            })
            .transpose()?;
        let brain_name = stage
            .as_ref()
            .map(|s| s.brain.to_string())
            .unwrap_or_else(|| arg_value(&args, "--brain").unwrap_or_else(|| "lua".into()));
        let coach_backend = arg_value(&args, "--coach").unwrap_or_else(|| "codex".into());
        let brain_label = if brain_name.eq_ignore_ascii_case("coach") {
            let backend = sekiro_brain_net::CoachBackend::by_name(&coach_backend)
                .ok_or_else(|| anyhow::anyhow!("unknown --coach {coach_backend}"))?;
            format!(
                "coach · {}",
                arg_value(&args, "--coach-model").unwrap_or_else(|| backend.default_model().into())
            )
        } else {
            brain_name.clone()
        };
        let window_size = arg_value(&args, "--window-size")
            .map(|s| -> anyhow::Result<(u32, u32)> {
                let (w, h) = s
                    .split_once('x')
                    .ok_or_else(|| anyhow::anyhow!("--window-size wants WIDTHxHEIGHT"))?;
                let size = (w.parse()?, h.parse()?);
                anyhow::ensure!(
                    size.0 >= 800 && size.1 >= 600,
                    "--window-size minimum is 800x600"
                );
                Ok(size)
            })
            .transpose()?
            .unwrap_or((1600, 900));
        return play::run(play::Options {
            brain_overlay: args.iter().any(|a| a == "--brain-overlay"),
            brain_label,
            window_size,
            attack_power,
            cache: PathBuf::from(arg_value(&args, "--cache").unwrap_or_else(|| "cache".into())),
            script: arg_value(&args, "--script"),
            input: arg_value(&args, "--input"),
            screenshots,
            exit_after_script: args.iter().any(|a| a == "--exit"),
            enemy: !args.iter().any(|a| a == "--no-enemy" || a == "--passive"),
            arena: boss_arena,
            full_map: args.iter().any(|a| a == "--full-map"),
            hide: arg_list(&args, "--hide"),
            objects: args.iter().any(|a| a == "--objects"),
            lock_on: args.iter().any(|a| a == "--lock-on"),
            simple_ai: args.iter().any(|a| a == "--simple-ai"),
            god: args.iter().any(|a| a == "--god"),
            immortal: args.iter().any(|a| a == "--immortal"),
            force_act: arg_value(&args, "--force-act")
                .map(|v| v.parse())
                .transpose()
                .map_err(|e| anyhow::anyhow!("--force-act wants an action id: {e}"))?,
            break_posture: args.iter().any(|a| a == "--break-posture"),
            brain: sekiro_brain_net::from_cli_with(
                &brain_name,
                arg_value(&args, "--brain-model"),
                &opponent.chr,
                &sekiro_brain_net::CoachCli {
                    budget_usd: arg_value(&args, "--brain-budget")
                        .map(|v| v.parse::<f64>())
                        .transpose()
                        .map_err(|e| anyhow::anyhow!("--brain-budget wants USD: {e}"))?,
                    token_budget: arg_value(&args, "--brain-token-budget")
                        .map(|v| v.parse::<u64>())
                        .transpose()
                        .map_err(|e| {
                            anyhow::anyhow!("--brain-token-budget wants a token count: {e}")
                        })?,
                    backend: arg_value(&args, "--coach"),
                    model: arg_value(&args, "--coach-model"),
                    effort: arg_value(&args, "--coach-effort"),
                },
            )
            .map_err(|e| anyhow::anyhow!("--brain: {e}"))?,
            coach_interval: arg_f32(&args, "--coach-interval", 30.0)?.max(0.0),
            player,
            brain_log: args.iter().any(|a| a == "--brain-log"),
            brain_memory: if let Some(stage) = stage {
                stage.memory
            } else {
                sekiro_sim::brain::memory::resolve_for(
                    arg_value(&args, "--brain-memory").as_deref(),
                    &opponent.chr,
                    &brain_name.to_ascii_lowercase(),
                    sekiro_brain_net::is_hosted(&brain_name),
                )
            },
            opponent,
            map: arg_value(&args, "--map"),
            start: arg_f32(&args, "--start", 0.0)? as usize,
            spawn: map::parse_camera(arg_value(&args, "--spawn").as_deref())?
                .map(|[x, y, z, yaw, _]| [x, y, z, yaw]),
            enemy_distance: arg_f32(&args, "--enemy-distance", 5.0)?,
            hour: arg_f32(&args, "--hour", 12.0)?,
            gparam: arg_value(&args, "--gparam"),
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
    eprintln!(
        "usage: sekiro-rs --play [--attack-power 1..99] [--progression castle-roof] [--script FILE|builtin] [--shots step:path,...] [--exit]"
    );
    eprintln!(
        "                 [--map <id> [--start <n> | --spawn x,y,z,yaw,0]] [--no-enemy] [--lock-on]"
    );
    eprintln!(
        "                 [--simple-ai] [--passive] [--god] [--immortal] [--force-act <id>] [--enemy c1010|c5060] [--brain lua|scripted|decisions|responses|coach] [--brain-model <name>] [--coach claude|codex|api] [--coach-model <name>] [--coach-effort <level>] [--coach-interval <secs>] [--brain-log] [--brain-memory off|on|<path>] [--player <name>]"
    );
    eprintln!("       sekiro-rs --arena owl [--full-map] [--play options] (a boss on his own map)");
    eprintln!(
        "                 [--brain-overlay] [--brain-stage lua|early|alive|outsmarted] [--window-size WIDTHxHEIGHT] (F8 toggles overlay)"
    );
    eprintln!("usage: sekiro-rs --arena [--deathblow] [--screenshot <out.png> --at <seconds>]");
    eprintln!("       sekiro-rs --map <id> [--start <n>] [--camera x,y,z,yaw,pitch] [--collision]");
    eprintln!("                 [--screenshot <out.png> --at <seconds>] (see src/map.rs)");
    eprintln!("       sekiro-rs --viewer <file.glb> [--screenshot <out.png>] [--yaw <deg>]");
    eprintln!("                 [--pitch <deg>] [--distance <m>] [--height <m>]");
    std::process::exit(2);
}

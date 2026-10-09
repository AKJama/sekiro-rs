# sekiro-rs

A Rust and [Bevy](https://bevyengine.org) reimplementation of Sekiro: Shadows Die Twice that runs on your own installed copy of the game.

This is a fan project for study.
It is not affiliated with or endorsed by FromSoftware or Activision.
**No game files are in this repository, only code.**
To build anything you need your own copy of Sekiro (Steam, v1.6.0.0), from which the tools here generate the data on your machine.

## How it works

Sekiro is data-driven, so instead of hand-writing its behaviour this project rebuilds the engine and feeds it the game's own data:

| Behaviour | Where the game keeps it | What sekiro-rs does |
|---|---|---|
| Player state machine (inputs, guard, deflect, attacks, jumps, dodges) | Havok Script bytecode (`c0000.hks`) | Runs it in a small Havok Script VM written in Rust |
| States and animation selection | Havok behaviour graphs | Behaviour graph runtime |
| Timings (hit frames, deflect windows, cancel windows) | TAE events | TAE runtime |
| Numbers (damage, posture, guard rates, recovery) | PARAM tables | Param reader driven by Paramdex definitions |
| Animation | Havok spline-compressed HKX | Decoded to sampled clips with root motion |
| Characters | FLVER, TPF, MTD | Exported to skinned, textured GLB |
| Levels | MSB, map pieces, Havok collision | Work in progress |
| Engine-side combat rules | the executable | Recovered by static analysis and rewritten as original Rust (`docs/COMBAT-RULES.md`) |

## Status

- Wolf and Ashina soldiers render, textured and animated with the game's clips.
- Wolf is playable: run, sprint, jump, step, crouch, attack combos, guard and deflect, all chosen by the game's own player script.
- Combat rules (hit, block, deflect, posture damage, breaks, recovery) are implemented and unit tested against the real params; wiring them to live hits is in progress.
- In progress: enemies with their own scripts, hit detection, deathblows, a real map with collision.

## Build and run

Requires Windows, Rust (stable, MSVC) and Sekiro v1.6.0.0.

```powershell
# 1. Public community reference files (archive keys, file names, param definitions)
./tools/fetch-refs.ps1
# 2. Unpack your install into cache/ (read only on the game folder; about 32 GB)
cargo run --release -p sekiro-extract -- unpack
# 3. Generate models, animations and params
cargo run --release -p sekiro-extract -- models
cargo run --release -p sekiro-extract -- anims c0000 c1010
cargo run --release -p sekiro-extract -- params
# 4. Play
cargo run --release -p sekiro-game -- --play
```

Controls: WASD move, mouse camera, left mouse attack, right mouse guard and deflect, Space jump, Shift step (hold to sprint), C crouch, Esc releases the mouse.
A gamepad works too.

## Layout

- `crates/formats`: readers for the game's formats.
- `crates/extract`: `sekiro-extract`, turns your install into `cache/`.
- `crates/hks`: Havok Script VM.
- `crates/sim`: Bevy-free simulation (behaviour graph, TAE, input, body, combat).
- `crates/game`: the `sekiro-rs` Bevy app.
- `docs/`: formats, params, the player script interface and the combat rules, in our own words.

## Disclaimer

sekiro-rs is an unofficial, non-commercial fan project made for education and research into game engine design and interoperability.
It is not affiliated with, endorsed by, or sponsored by FromSoftware, Inc. or Activision Publishing, Inc.

Sekiro: Shadows Die Twice and all of its content, including characters, models, textures, animations, audio, data and trademarks, belong to FromSoftware, Inc. and its publishers.
None of that content is included in or distributed with this repository.
This repository contains only original source code and documentation written for this project.

- You need your own legitimately purchased copy of the game; the tools read it locally and never modify the installation or save files.
- Everything derived from the game stays in the gitignored `cache/` and `re/` folders on your machine. Do not commit, upload or share those folders or anything generated into them.
- This project is single-player only. It has no online features and does not interact with online services or anti-cheat systems.
- No warranty is given; use it at your own risk.

If you are a rights holder and have a concern about this project, please open an issue and it will be addressed promptly.

See [CREDITS.md](CREDITS.md) for the community research this builds on.

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.

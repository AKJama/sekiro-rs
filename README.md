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
| Levels | MSB, map pieces, Havok collision, ledge and grapple edges | All ten maps exported and walkable |
| Enemy AI | Lua 5.0 battle scripts, EMEVD event scripts | Own Lua VM and goal runtime; boss phases from the map's events |
| Rendering | GPARAM draw params, compiled shaders | Fog, exposure, tone mapping, bloom and grading reimplemented from the game's own shaders (`docs/SHADERS.md`) |
| Engine-side combat rules | the executable | Recovered by static analysis and rewritten as original Rust (`docs/COMBAT-RULES.md`) |

## Status

Work in progress.
What runs today:

- **Combat:** deflect, block, posture both ways, perilous attacks and Mikiri, deathblows with the game's own throw pairs and deathblow camera, death and respawn.
- **Bosses and enemies:** the Great Shinobi Owl on the Ashina Castle roof (both phases, his tools), Genichiro Ashina on the castle keep roof (both deathblows, the Way of Tomoe hand-over, his lightning and Lightning Reversal), Ashina soldiers; every character placed in the game loads by id.
- **Boss brain:** an AI that learns how you fight and adapts the boss (`docs/BRAIN.md`), for the Owl, Genichiro and the soldier.
- **Movement:** run, sprint, jump with air control, step, crouch, ledge hang and climb, grappling hook (arcs, mid-air and chained), wall hug, swimming and diving, fall death.
- **World:** all ten maps load and are walkable, with light probes, point lights, layered moss and snow materials, and the game's fog, tone mapping, bloom and colour grading.
- **Not yet:** the game's particle effects (simple stand-ins are used), sky domes, menus beyond the start and pause menus, items, prosthetic tools and combat arts, stealth, and most enemies' special mechanics.

The detailed state is in [STATUS.md](STATUS.md) and [docs/PROGRESS.md](docs/PROGRESS.md).

## Build and run

Requires Windows, Rust (stable, MSVC, with the Visual Studio C++ build tools) and your own Steam copy of Sekiro v1.6.0.0.
Point the tools at your install with `--game <folder>` or the `SEKIRO_DIR` environment variable if it is not in the default Steam location.
Extraction only reads the game folder and writes everything to the gitignored `cache/` folder (about 50 GB in total, so leave room).

```powershell
# 1. Public community reference files (archive keys, file names, param definitions)
./tools/fetch-refs.ps1
# 2. Unpack your install into cache/raw (about 32 GB)
cargo run --release -p sekiro-extract -- unpack
# 3. Params first, then the maps (the character list comes from the maps)
cargo run --release -p sekiro-extract -- params
foreach ($m in 'm10_00_00_00','m11_00_00_00','m11_01_00_00','m11_02_00_00','m13_00_00_00','m15_00_00_00','m17_00_00_00','m20_00_00_00','m25_00_00_00','m89_00_00_99') { cargo run --release -p sekiro-extract -- map $m }
# 4. Characters and animations (defaults to Wolf and every placed character)
cargo run --release -p sekiro-extract -- models
cargo run --release -p sekiro-extract -- anims
# 5. Play
cargo run --release -p sekiro-game
```

Running the game with no arguments opens the start menu: pick the fight, the place, the AI, the time of day and god mode, then Start.
Esc in a fight opens the pause menu (resume, back to the start menu, quit).

The same choices work as command-line flags, which skip the menu:

```powershell
$game = 'target/release/sekiro-rs.exe'
& $game --arena owl --progression castle-roof          # Great Shinobi Owl on the castle roof, his own AI
& $game --arena genichiro --progression castle-roof    # Genichiro on the castle keep roof
& $game --arena genichiro --genichiro-phase tomoe       # straight to the Way of Tomoe lightning phase
& $game --play --enemy soldier                          # an Ashina soldier on a flat floor
& $game --play --map m11_00_00_00 --start 3 --no-enemy  # walk around a map
```

Useful flags: `--god` (you cannot die), `--immortal` (you cannot die but posture still breaks), `--hour 0-23` (time of day), `--enemy cXXXX` (any character by id).

Controls: WASD move, mouse camera, left mouse attack, right mouse guard and deflect, Space jump, Shift step (hold to sprint), C crouch, Q lock-on, E grab a ledge or hug a wall, F grappling hook, Esc pause menu.
A gamepad works too.

## The boss brain

The boss brain replaces the boss's choice of moves with a model-driven controller that adapts to how you fight, while the boss keeps his own moves, timings and rules.
A coach model reviews your fights (after each fight and about every 30 seconds mid-fight) and writes a game plan; in the fight a fast local system picks moves from that plan.
It sees only what a player could see, and it learns across sessions (memory under the gitignored `logs/memory/`).

```powershell
& $game --arena owl --progression castle-roof --brain coach --coach codex --brain-overlay
& $game --arena genichiro --progression castle-roof --brain coach --coach codex --brain-overlay
```

- The coach runs through the [Codex CLI](https://github.com/openai/codex) on your own ChatGPT plan (install it and sign in first); it is started isolated, without your other Codex settings or project files.
- Optional: with an `OPENAI_API_KEY` (environment variable or a gitignored `.env`) a small, fast model also switches between the coach's prepared plans at key moments in the fight. This uses paid API calls; without a key the coach still works.
- `--brain-overlay` (F8 toggles it) shows the current plan and the coach's notes; the overlay footer counts tokens.
- `--player <name>` keeps a separate profile per player; `--brain-memory off` leaves memory untouched.
- `--brain-token-budget N` stops model calls after N tokens and falls back to the boss's own AI.

Details, including how the plans, checkpoints and memory work: [docs/BRAIN.md](docs/BRAIN.md).

## Layout

- `crates/formats`: readers for the game's formats.
- `crates/extract`: `sekiro-extract`, turns your install into `cache/`.
- `crates/hks`: Havok Script VM.
- `crates/sim`: Bevy-free simulation (behaviour graph, TAE, input, body, combat).
- `crates/game`: the `sekiro-rs` Bevy app.
- `docs/`: formats, params, the player script interface and the combat rules, in our own words.

## Tested setup

Windows 11 64-bit, Steam Sekiro app 814380, build 5794815 (game version 1.6.0.0), Rust 1.99 (MSVC), Bevy 0.19.1.
Other game versions are untested.

## Research boundary

Format and engine research happens in local, gitignored folders (`cache/` and `re/`) and is never committed.
That includes extracted game data, analysis databases and any decompiler output.
The Rust code in this repository is original and written from that research and from public community documentation; no game code is loaded, linked or copied into it.
Engine-side rules are documented in our own words in `docs/`, with confidence labels.

This is an AI-assisted project, developed with Anthropic Claude Code and OpenAI Codex, and reviewed and playtested by the author.

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

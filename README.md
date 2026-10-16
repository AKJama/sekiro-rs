# sekiro-rs

A Rust and [Bevy](https://bevyengine.org) reimplementation of Sekiro: Shadows Die Twice that runs on your own installed copy of the game.

This is a fan project for study.
It is not affiliated with or endorsed by FromSoftware or Activision.
**No game files are in this repository, only code.**
To build anything you need your own copy of Sekiro (Steam, v1.6.0.0), from which the tools here generate the data on your machine.

## Getting started

1. Install Rust (stable, MSVC) and the Visual Studio C++ build tools on Windows.
   For help, you can also install an AI agent such as Claude Code or Codex.
2. Find your Steam Sekiro v1.6.0.0 installation folder.
   If it is not in the default Steam location, use `--game <folder>` or the `SEKIRO_DIR` environment variable.
3. Leave about 50 GB free for `cache/` and follow the [setup commands](docs/GETTING-STARTED.md#build-and-run) in PowerShell from this repo's folder.
   You can ask your agent: "Follow docs/GETTING-STARTED.md to set up my installed copy of Sekiro, then run the game."
4. After setup, run the game and choose a fight in the start menu:
   ```powershell
   cargo run --release -p sekiro-game
   ```

## More

- [Setup, controls, architecture and AI workflow](docs/GETTING-STARTED.md)
- [Boss AI and coach](docs/BRAIN.md), [combat](docs/COMBAT-RULES.md) and [movement](docs/MOBILITY.md)
- [Maps](docs/MAP-COMPLETENESS.md), [lighting](docs/LIGHTING.md) and [shaders](docs/SHADERS.md)
- [Formats](docs/FORMATS.md), [progress](docs/PROGRESS.md), [status](STATUS.md) and [contributing](CONTRIBUTING.md)

## Working together

See the shared [collaboration board](https://github.com/AKJama/sekiro-collab) and its `WORK.md` for who is working on what.

## Credits

Special thanks to FwLo, creator of [RustSekiro](https://github.com/FFwLo/RustSekiro), for map shaders and materials, effects, cloth, combat and camera rules, and traced engine values.
Everyone else whose work helped is in [CREDITS.md](CREDITS.md).

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

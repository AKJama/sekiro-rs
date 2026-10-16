# sekiro-rs

A Rust/Bevy reimplementation of Sekiro: Shadows Die Twice (Steam, v1.6.0.0, build 5794815) that runs on the player's own installed game data.

## How this project works

- The game is data-driven: player logic is HKS script, timings are TAE events, numbers are params, AI is Lua, levels are MSB.
  Re-implement the engine and feed it the real data; do not hand-tune what the data already says.
- Exe-side logic (damage, posture, env() functions) comes from static analysis of `sekiro.exe` in Ghidra.
  Write original Rust from what you learn; never paste decompiler output into source.
- Build first, then compare against the real game.
  Keep known approximations in `docs/FIDELITY.md`; they never block progress.
- Every milestone must be runnable. Show it working (screenshot or log), then move on.
- Shared plan with [RustSekiro](https://github.com/FFwLo/RustSekiro): read `WORK.md` in [AKJama/sekiro-collab](https://github.com/AKJama/sekiro-collab) and claim a task there before starting it.

## Layout

- `crates/formats`: readers for game formats (archives, DCX, BND4, FLVER, TPF, HKX, TAE, PARAM, MSB, HKS).
- `crates/extract`: `sekiro-extract` CLI, turns the install into `cache/`.
- `crates/sim`: Bevy-free gameplay simulation, fixed tick, deterministic, unit tested.
- `crates/game`: the `sekiro-rs` Bevy app.
- `cache/`: everything derived from the game. Gitignored, regenerate with `sekiro-extract`.
- `re/`: Ghidra project and decompile dumps. Gitignored. Notes in own words go in `docs/re/`.

## Hard rules

- Never write to the game install, its saves, or the running game's memory.
- Back up saves before any session that runs the game.
- Game files, extracted data and research artefacts stay in the local gitignored `cache/` and `re/` folders; only original source and docs are tracked (whitelist `.gitignore`).
- No online play, anti-cheat or DRM work.
- Every change goes through an issue, a branch and a pull request; follow [`.claude/skills/pr-flow/SKILL.md`](.claude/skills/pr-flow/SKILL.md) whichever agent you are.
- Never publish anything outside the repo, its issues and PRs, and the collab board without asking.

## Conventions

- Windows, PowerShell. Python via `uv` only.
- Cargo: `cargo`. Keep `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` clean.
- No em dashes. One sentence per line in long Markdown.
- Update `STATUS.md` at the end of each working session; the to-do list lives in GitHub issues.

## Tests

- The normal run is `cargo test --release --workspace`.
  Integration tests live in one binary per crate (`crates/<crate>/tests/it/main.rs`, one module per file), so a new test file is a `mod` line there, not a new binary.
- The normal run holds tests that guard distinct behaviour and finish in a few seconds, including one end-to-end fight per boss (Owl, Genichiro).
  Anything slower, such as full-roster sweeps or long parameter sweeps, is marked `#[ignore = "slow: <reason>"]`; run that tier with `cargo test --release --workspace -- --include-ignored`.
- Prefer one focused test per behaviour, table-driven when only the inputs differ; don't add tests that restate the code (constants, defaults, getters, serde round-trips of a type already covered).
- Tests that build many independent `Duel`s can spread the cases over cores with `rayon` (a dev-dependency of `sekiro-sim`).

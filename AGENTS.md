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
- No game files, extracted data, decompiled code or Ghidra databases in git (whitelist `.gitignore`).
- No online play, anti-cheat or DRM work.
- Local commits are fine; never add a remote, push or publish without asking.

## Conventions

- Windows, PowerShell. Python via `uv` only.
- Cargo: `cargo`. Keep `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` clean.
- No em dashes. One sentence per line in long Markdown.
- Update `STATUS.md` at the end of each working session.

# Design

## Goal

A Rust/Bevy reimplementation of Sekiro that loads the player's own installed game data and plays like the original.
The first target is the combat core: Wolf against an Ashina soldier with guard, deflect, posture, posture break and deathblow.
World, movement, AI and sound follow.

## Approach

Sekiro is data-driven, so the engine is rebuilt and the original data is fed through it.

| Behaviour | Where the game keeps it | How we use it |
|---|---|---|
| Player state machine (inputs, guard, deflect, attacks, dodges) | `action/script/c0000*.hks`, Havok Script bytecode | Run it in our own HKS VM, with `env()`/`act()` implemented in Rust |
| Timings (hit frames, deflect windows, invulnerability, cancels) | TAE events in each `anibnd` | TAE runtime fires events while clips play |
| Numbers (damage, posture, guard rates, recovery) | PARAM tables in `gameparam.parambnd` | Typed tables from Paramdex definitions |
| Animation | Havok HKX, spline compressed | Decoded to sampled clips with root motion |
| Characters | FLVER meshes, TPF textures, MTD materials | Exported to GLB, skinned in Bevy |
| Hitboxes | Dummy polys on the weapon model plus AtkParam radii | Sphere/capsule sweeps between dummy positions |
| Enemy AI | Lua in `script/aicommon` and per-enemy logic | Later: run via the same scripting layer |
| Levels | MSB plus map piece FLVERs and Havok collision | Later |
| Engine-side formulas | `sekiro.exe` | Static analysis in Ghidra, reimplemented in original Rust |

## Crates

- `sekiro-formats`: readers only, no Bevy.
- `sekiro-extract`: CLI that turns the install into `cache/`.
- `sekiro-sim`: deterministic fixed-tick simulation (actors, animation playback, TAE, HKS host, combat). No Bevy, unit tested.
- `sekiro-game`: Bevy app (`sekiro-rs`) for rendering, input, camera, audio and debug UI.

## Milestones

1. Real models: Wolf and Ashina soldier textured and animated in Bevy.
2. Combat slice: flat arena, soldier attacks on a timer, Wolf guards and deflects; HP and posture bars; sparks; posture break and deathblow.
3. World: one map area, collision, locomotion, lock-on.
4. AI and sound.
5. Validation: compare posture/HP/animation traces against the real game, read-only.

## Validation

Build from the game's data first, then compare against the real game.
Gaps and approximations live in `docs/FIDELITY.md` and never block a milestone.

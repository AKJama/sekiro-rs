# Audio

Sounds play through the game's own sound engine: the FMOD Ex 4.4 DLLs of the player's install run the game's own event projects.
Our decoded-sample path is kept only as a fallback for when the DLLs cannot be loaded.

## Design

- `crates/sim/src/sound.rs` names the sounds of each simulation step as event keys (`c000004010`, `z199999980`): TAE `PlaySound_*` events and hit, block and deflect sounds.
- `crates/fmod` (`sekiro-fmod`) loads `fmodex64.dll`, then `fmod_event64.dll`, read-only from the install (`--game <dir>`, else `SEKIRO_DIR`, else the default Steam folder), and drives the Event API through `libloading`, the same way `oo2core` is loaded for DCX.
- `crates/game/src/audio.rs` starts the event system, loads the fight's projects from `cache/raw/sound`, places every cue as an event at its character (1 m above the feet) and puts the listener on the camera every frame.
  FMOD then applies each event's own variations, volume and pitch randomisation, layering, voice limits and 3D distance rolloff, as the game does.
- If the DLLs are missing, the decoded samples in `cache/sound` play as Bevy positional one-shots (the old path).
  `SEKIRO_NO_FMOD=1` forces that fallback.
- With FMOD running, a key no project has stays silent: the game is silent there too.

FMOD runtime facts used (from FwLo's RustSekiro, `docs/kb/sound.md`, MIT; our code is our own):

- Init with `FMOD_INIT_3D_RIGHTHANDED` (0x2), which matches Bevy's axes; 0x4 is `FMOD_INIT_SOFTWARE_DISABLE` and makes every event fail.
- The master category plays at 0.7 (the exe's Master slot times 0.7); the SE, music and voice categories stay at 1.0 at default options.
- An event's path is `project/group/event` and the event is named after its sound key: `main/main/c000004010`, `c5060/c5060/c506001001`.
  `sekiro-fmod` walks every group of a loaded project, so the game only needs the key.

## Projects

| Project | Events | Contents |
|---|---|---|
| `main` | 1875 | Wolf, shared footsteps, armour, hit, block and deflect sounds; plain FSB |
| `c5060` | 57 | The Owl; plain FSB |
| `c1010` | | The Ashina soldier; plain FSB |
| `m11` | 274 | Ashina objects and ambience; plain FSB |
| `sm11`, `xm11`, `vm11`, `rm11` | 86, 4, 3955, 65 | Area SFX, music, voices, cutscenes; encrypted FSB, FMOD cannot open them (error 25) |
| `smain` | 50 | Common SFX; encrypted, and its language banks (`smain_<lang>.fsb`) exist only loose in the install (error 23) |

The game loads `main`, the map area's projects (`m11_01_00_00` gives `m11`, `sm11`, `xm11`, `vm11`, `rm11`) and `smain` at start, and the enemy's `cXXXX` when the fight starts.
The samples of `main`, the enemy and `m11` are loaded up front (about 0.1 s), so no first play waits for the disk.
Nothing is decrypted: events in the encrypted banks fail once, are logged, and stay silent.

## Event keys

A key is a type letter plus a nine-digit id.
The letter comes from the TAE sound type: 0 a environment, 1 c character, 2 f menu, 3 o object, 4 p cutscene, 5 s SFX, 6 m BGM, 7 v voice, 8 x floor material, 9 b armour material, 10 g ghost (DSAnimStudio Sekiro template), and 12 z for hit-effect sounds (exe, via FwLo).

- Floor `x`: `c` + (id rounded down to 1000 + floor material), so footstep `x000001000` on tatami (HitMtrlParam 28) is `c000001028`.
  The floor material is the hit material of the collision triangle under the character's feet (`Ground::material`, from the `hknp` `materialNameData`); the flat floor uses 1, cobblestone.
  The Owl's castle top is tatami (28) where Wolf starts and indoor wood (32) where the Owl stands.
- Armour `b`: `c` + (id + material), trying the wearer's two materials in order and playing the first that exists: Wolf's body armour (EquipParamProtector 131000) `defenseMaterial1/2` 113, 139; an NPC's NpcParam `materialSe1/2` (Owl and soldier 114, 108).
- Hits (`HitEffectSeParam`, `z`): one sound per defender material row (above), column `<atkMaterial_forSe group>_<atkType: Slash, Blow, Thrust>_<atkPow_forSe: S, L, LL>`.
  Wolf's slash on the Owl gives `z100000103` and `z000000108`; the Owl's slash on Wolf gives `z000000013`.
- Blocks: `HitEffectSeParam` at the attack's own `defSeMaterial1/2` rows (`z200000101`, `z113000102`).
- Deflects: `HitEffectSeJustGuardParam` at those rows, `HitEffectSeParam` when it names nothing (`z199999980`).
- Gated TAE sounds (non-zero `StateInfo`) play only while an SpEffect with that state is active (`crates/sim/src/tae.rs`), so enchantment and upgraded-prosthetic sounds stay off.

The earlier pass read the hit tables with `atkAttribute` and `atkSize`, used the `s` letter and Wolf's weapon as the guard row; those were wrong and are replaced by the rules above.

## Coverage

`cargo run -p sekiro-fmod --example sound_probe --release` loads the Owl fight's projects with the silent FMOD output, keys every TAE sound of both characters, and runs a 20 s headless fight on the castle top (Wolf locks on, walks in, attacks, holds guard and taps it to deflect; the Owl runs his own AI).
Numbers from 2026-10-10:

| TAE sounds | Events | Distinct keys | Play |
|---|---|---|---|
| Wolf `c` | 6128 | 298 | 293 (4 have no event, 1 is in encrypted `smain`) |
| Wolf `x` footsteps | 5605 | 9 | 9 |
| Wolf `b` armour | 4014 | 7 | 3 (`c000000213`, `c000001213`, `c000002213`, `c000003113` exist with neither material) |
| Wolf `s` | 43 | 17 | 17 |
| Owl `c` | 1484 | 55 | 55 |
| Owl `s` | 5 | 3 | 3 |

The Owl's TAE has no voice (`v`), floor or armour events; his swings, steps, cloth and grunts are all `c5060` events.
960 of Wolf's TAE sounds are gated and skipped by default.

In the 20 s fight 202 cues fired and 169 started: Wolf's swings (`c000004010`), steps on tatami and wood, armour rustle, guard and deflect sounds, the deflect clang (`z199999980`), hits both ways (`z000000013`, `z100000103` + `z000000108`), and the Owl's swings, steps and moves (`c506001001` to `c506008003`).
Of the rest, 31 are `c000000156` (fired as Wolf starts a guard; no project has it) and one was declined by an event's own playback limit (FMOD error 83), which the game also skips.
A 60 s run adds block clangs (`z200000101`, `z113000102`): 798 cues, 666 started.

## Loudness

`sound_probe` also measures each event's audibility (FMOD's mixed volume of the event's channels) with the listener 0.5 to 30 m away.

| Event | 0.5 m | 2 m | 5 m | 8 m | 12 m | 20 m | 30 m | Play distance |
|---|---|---|---|---|---|---|---|---|
| `c000004010` Wolf swing | 0.12 | 0.12 | 0.12 | 0.11 | 0.08 | 0.03 | 0 | 29 m |
| `c000001028` step on tatami | 0.21 | 0.19 | 0.19 | 0.16 | 0.13 | 0.05 | 0 | 29 m |
| `z199999980` deflect clang | 0.34 | 0.34 | 0.33 | 0.33 | 0.31 | 0.25 | 0.17 | 60 m |
| `z000000013` hit on Wolf | 0.70 | 0.69 | 0.67 | 0.63 | 0.55 | 0.35 | 0.11 | 40 m |
| `c506005000` Owl | 0.57 | 0.60 | 0.51 | 0.51 | 0.44 | 0.31 | 0.08 | 40 m |

Sounds within a few metres play at a steady level whatever the camera does, and fade smoothly with distance.
The old Bevy path attenuated by the camera-to-character distance with no minimum distance, so a camera pushed in by a wall or corner made everything much louder: the reported "louder near corners, quiet otherwise".

## Tools

- `cargo run -p sekiro-fmod --example fmod_list --release -- [project...] [--events] [--has key...]`: event counts, event paths, and whether keys start.
- `cargo run -p sekiro-fmod --example sound_probe --release -- [--seconds N] [--quiet] [--preload]`: the coverage and loudness report above.
- `cargo run -p sekiro-sim --example duel_sounds --release`: the cues of the scripted soldier duel against the decoded banks (fallback path).

## Fallback: decoded samples

Used only when FMOD cannot start.
`crates/formats/src/fsb.rs` reads FSB5 headers, sample tables and Vorbis packets; `crates/formats/src/fev.rs` reads the FEV string table and sound definitions.
FSB5 Vorbis samples keep only a CRC-32 of the setup header; `uv run tools/sound/vorbis_setups.py cache` rebuilds matching headers (7 of 11 setups), and `sekiro-extract sound [banks] [--all]` decodes to `cache/sound/<bank>/*.wav` plus `events.json` (default banks: main, c1010, m11).
This path has no event volumes, rolloff or voice limits, and does not cover `c5060`.

## Missing

- Sounds play 1 m above the character's feet, not at the TAE event's dummy poly.
- `c000000156` and the four armour keys above exist in no project; whether the game derives them differently is not known.
- Encrypted banks (area SFX, music, voices, `smain`) stay silent until the user decides about them (see `STATUS.md`).
- No map reverb (`FMOD_EventSystem_SetReverbProperties`), music or option sliders; event velocities are zero (no Doppler).

## Volume

`--volume 0.0..1.0` sets the player's master volume (default 0.5, since full volume was reported too loud).
In play, minus and equals (or `[` and `]`, or the keypad minus and plus) lower and raise it in 10% steps, and the level shows top right for 1.5 s.
It scales FMOD's master category (on top of the game's own 0.7) or, on the fallback path, Bevy's global volume (`crates/game/src/audio.rs`, `adjust_volume`).

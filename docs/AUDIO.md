# Audio

Sounds come from the game's own banks in `cache/raw/sound`, decoded by our own readers.
This is a first pass; open items are at the end.

## Files

| Files | Contents |
|---|---|
| `*.fsb` (112 of 174) | FSB5 sample banks, all Vorbis (codec 15), mostly mono 48 kHz |
| `sm*`, `vm*`, `xm*`, `rm*`, `smain` `.fsb` (62) | Encrypted banks: common SFX, voices, floor-material footsteps, music; not read |
| `*.fev` | RIFF `FEV ` event projects: a string table (`STRR`) with event names and an FMOD Designer block (`LGCY`) with sound definitions |
| `*.itl` | Not read |

`crates/formats/src/fsb.rs` reads FSB5 headers, sample tables, names and the Vorbis packet streams.
`crates/formats/src/fev.rs` reads the string table and the sound definitions' waveform lists (name, weight, bank index, sample index, length).

## Vorbis headers

FSB5 Vorbis samples keep only a CRC-32 of the setup header.
Stock libvorbis gives the same setup header for the same channels, rate and VBR quality, so `uv run tools/sound/vorbis_setups.py cache` searches a quality grid and writes matching headers to `cache/sound/vorbis/`.
7 of the 11 setups in the plain banks match (48, 44.1 and 32 kHz); the low-rate ones (8 to 24 kHz, about 2% of samples) do not yet.
`sekiro-extract sound [banks] [--all]` then decodes with lewton to `cache/sound/<bank>/*.wav` plus `events.json` (default banks: main, c1010, m11).

## Naming

A sound event is a type letter plus a nine-digit id, the letter from the TAE sound type: 0 a environment, 1 c character, 2 f menu, 3 o object, 4 p cutscene, 5 s SFX, 6 m BGM, 7 v voice, 8 x floor material, 9 b armour material, 10 g ghost (DSAnimStudio Sekiro template).
Sample names add variation letters (`c101001001`, `c101001001b`, ...).
Events map to sound definitions by that stem; the event records' own links to definitions are not decoded yet.

## What plays

- TAE `PlaySound_*` events (128 to 132) on the playing clips (`crates/sim/src/tae.rs`, `TaeFrame::sounds`).
- Hit, block and deflect sounds from `HitEffectSeParam` and `HitEffectSeJustGuardParam` (`crates/sim/src/sound.rs`): row = defender material (Kusabimaru `defSeMaterial1` 101, soldier body `materialSe1` 114), column = attacker `atkMaterial_forSe`, kind and size. A deflect of the soldier's slash is `s999999980`.
- `crates/game/src/audio.rs` plays cues as positional one-shots with weighted variations.

In the built-in scripted duel 268 cues fire and 114 are in the decoded banks: the soldier's attack and movement sounds and some of Wolf's.

## Missing

- Encrypted banks: floor footsteps (`x`), most hit sounds (`s000000118`), armour (`b`) and many of Wolf's sounds.
- Low-rate Vorbis setups.
- Event-to-definition links, volumes and pitch randomisation in `LGCY`.
- Floor material lookup for `x` sounds.

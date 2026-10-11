# Credits

sekiro-rs reimplements its own readers and runtime, but stands on years of community format research.
No code from these projects is vendored; their documentation and source were read as references, except for the code adapted from FwLo's RustSekiro listed below.

## Formats and data

- [soulsmods/SoulsFormatsNEXT](https://github.com/soulsmods/SoulsFormatsNEXT) and [JKAnderson/SoulsFormats](https://github.com/JKAnderson/SoulsFormats): BND4, BXF4, DCX, FLVER, TPF, MTD, MSB, TAE, PARAM and EMEVD layouts.
- [soulsmods/Paramdex](https://github.com/soulsmods/Paramdex): param definitions and row names (fetched at setup, not vendored).
- [vawser/Smithbox](https://github.com/vawser/Smithbox): param field descriptions and enums.
- [Meowmaritus/DSAnimStudio](https://github.com/Meowmaritus/DSAnimStudio): Sekiro TAE event templates (fetched at setup).
- [Nordgaren/UXM-Selective-Unpack](https://github.com/Nordgaren/UXM-Selective-Unpack): archive keys and the file name dictionary (fetched at setup).
- [PredatorCZ/HavokLib](https://github.com/PredatorCZ/HavokLib), [Grimrukh/soulstruct-havok](https://github.com/Grimrukh/soulstruct-havok), [Meowmaritus/MVDX2](https://github.com/Meowmaritus/MVDX2): Havok tagfile, spline animation and collision references.
- [soulsmods/fstools-rs](https://github.com/soulsmods/fstools-rs): Rust format reading reference.

## Scripts

- [AinTunez/DarkScript3](https://github.com/AinTunez/DarkScript3) (`sekiro-common.emedf.json`, after the soulsmods EMEDF tables): names and argument layouts of the event script instructions and their enums, read as facts for `crates/formats/src/emedf.rs`.
- [katalash/DSLuaDecompiler](https://github.com/katalash/DSLuaDecompiler), [Surasia/hksc-disassembler](https://github.com/Surasia/hksc-disassembler): Havok Script references, used locally for study.
- [iitsigor/SekiroHKS](https://github.com/iitsigor/SekiroHKS): names for engine functions used by the player script.

## Engine research

- [borgCode/SekiroTool](https://github.com/borgCode/SekiroTool), [Xu060113/SekiroCraft-Passthrough](https://github.com/Xu060113/SekiroCraft-Passthrough), [thisguymartin/sekiro-deflect-observer](https://github.com/thisguymartin/sekiro-deflect-observer), [ElaDiDu/Sekiro-Practice-CT](https://github.com/ElaDiDu/Sekiro-Practice-CT): memory layout and posture research used to cross-check our own findings.
- [FFwLo/RustSekiro](https://github.com/FFwLo/RustSekiro) (MIT): its knowledge base (`docs/kb`) supplied traced engine values we use: gravity, fall-control table values, turn speed, body radius, the input accept/execute flags, camera and lock-on rules, the locked-on walk twist (WalkTwist cones, its rate and the graph's Master and spine twist), lunge, auto-aim, knockback timing and invulnerability events (whose action-module bit also identifies the Owl's warp flag, ChrActionFlag 94, as invulnerability), the AtkParam hit stop fields, and which NPC TAE flags accept a queued AI request, and the sound runtime: playing events through the install's own FMOD Ex DLLs, the init flag, the master volume of 0.7, event naming, and the floor, armour and hit-sound material rules, the deathblow absorb (the defender placed once at the throw clip's frame 0, then both clips' own root motion), and the enemy grab pattern (a `throwFlag` 1 contact starts the throw, the `ThrowAttackBehavior` events carry its damage, Wolf's thrown clips live in `chr/c0000_cXXXX.anibnd`, step i-frame type 7 is the grab type, no grab on an airborne Wolf), and bullet flight past `dist` (`accelOutRange` and `gravityOutRange`) with the `lockShootLimitAng` aim. Each use is credited where it appears.

## Code adapted from RustSekiro

From [FFwLo/RustSekiro](https://github.com/FFwLo/RustSekiro), under the MIT licence below:

- The deathblow camera (TAE 151 / 153 / 155 rules) in `crates/sim/src/camera.rs` and `crates/game/src/play.rs`, from his `src/camera.rs`.
- The perilous kanji HUD stand-in in `crates/game/src/combat_hud.rs`, from his `src/hud.rs`.
- The sweep i-frame and kick-off (env 2004) rules in `crates/sim/src/hits.rs` and `crates/sim/src/duel.rs`, from his `src/combat.rs` and `src/player.rs`.
- The Mikiri contact/flag rule, parry posture field and counter/break ThrowParam selection in `crates/sim/src/perilous.rs` and `crates/sim/src/duel.rs`, from his `src/combat.rs` and knowledge base, checked against our soldier and Owl data.

```
MIT License

Copyright (c) 2026 FFwLo

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Inspiration

- [Funny-Bones/ELDEN-RING-Combat-Rewrite](https://github.com/Funny-Bones/ELDEN-RING-Combat-Rewrite): a data-driven Elden Ring combat sandbox in Bevy.
- [trevaintdead/ai-game-modding-guides](https://github.com/trevaintdead/ai-game-modding-guides): the workflow and rules this project follows.

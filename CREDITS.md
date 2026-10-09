# Credits

sekiro-rs reimplements its own readers and runtime, but stands on years of community format research.
No code from these projects is vendored; their documentation and source were read as references.

## Formats and data

- [soulsmods/SoulsFormatsNEXT](https://github.com/soulsmods/SoulsFormatsNEXT) and [JKAnderson/SoulsFormats](https://github.com/JKAnderson/SoulsFormats): BND4, BXF4, DCX, FLVER, TPF, MTD, MSB, TAE and PARAM layouts.
- [soulsmods/Paramdex](https://github.com/soulsmods/Paramdex): param definitions and row names (fetched at setup, not vendored).
- [vawser/Smithbox](https://github.com/vawser/Smithbox): param field descriptions and enums.
- [Meowmaritus/DSAnimStudio](https://github.com/Meowmaritus/DSAnimStudio): Sekiro TAE event templates (fetched at setup).
- [Nordgaren/UXM-Selective-Unpack](https://github.com/Nordgaren/UXM-Selective-Unpack): archive keys and the file name dictionary (fetched at setup).
- [PredatorCZ/HavokLib](https://github.com/PredatorCZ/HavokLib), [Grimrukh/soulstruct-havok](https://github.com/Grimrukh/soulstruct-havok), [Meowmaritus/MVDX2](https://github.com/Meowmaritus/MVDX2): Havok tagfile, spline animation and collision references.
- [soulsmods/fstools-rs](https://github.com/soulsmods/fstools-rs): Rust format reading reference.

## Scripts

- [katalash/DSLuaDecompiler](https://github.com/katalash/DSLuaDecompiler), [Surasia/hksc-disassembler](https://github.com/Surasia/hksc-disassembler): Havok Script references, used locally for study.
- [iitsigor/SekiroHKS](https://github.com/iitsigor/SekiroHKS): names for engine functions used by the player script.

## Engine research

- [borgCode/SekiroTool](https://github.com/borgCode/SekiroTool), [Xu060113/SekiroCraft-Passthrough](https://github.com/Xu060113/SekiroCraft-Passthrough), [thisguymartin/sekiro-deflect-observer](https://github.com/thisguymartin/sekiro-deflect-observer), [ElaDiDu/Sekiro-Practice-CT](https://github.com/ElaDiDu/Sekiro-Practice-CT): memory layout and posture research used to cross-check our own findings.

## Inspiration

- [Funny-Bones/ELDEN-RING-Combat-Rewrite](https://github.com/Funny-Bones/ELDEN-RING-Combat-Rewrite): a data-driven Elden Ring combat sandbox in Bevy.
- [trevaintdead/ai-game-modding-guides](https://github.com/trevaintdead/ai-game-modding-guides): the workflow and rules this project follows.

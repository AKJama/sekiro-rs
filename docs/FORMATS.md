# Formats

## Animation: HKX skeletons, clips and TAE events

Code: `crates/formats/src/hkx/` (tagfile reader, spline decoder), `crates/formats/src/anim.rs` (types, sampling, conversion), `crates/formats/src/tae.rs`, `crates/extract/src/anims.rs`.

### Coordinates

Skeletons, clips and root motion are stored in FromSoftware source space: left-handed, +Y up.
Characters face source -Z: walk and run root motion travel toward -Z, and the idle pose's toes point toward -Z.
The `forward = (0, 0, 1)` field in Havok's reference frame object is a basis declaration, not the facing direction.
Conversion to Bevy uses the same X mirror as the model export: `x -> -x`, `M -> S M S` with `S = diag(-1, 1, 1)`.
`sekiro_formats::anim::to_bevy` applies it once at load time: translations negate x, quaternions `[x, y, z, w]` become `[x, -y, -z, w]`, scales are unchanged, root-motion displacement negates x and root-motion yaw negates.
Applying it to every local transform gives exactly the mirrored model-space matrices (unit tested), so it composes with the mirrored GLB bind pose.
Units are metres.

### Cache layout

`sekiro-extract anims [chr...]` (default `c0000 c1010`) reads `cache/raw/chr/<chr>.anibnd.d` and every `<chr>_*.anibnd.d`, and writes `cache/anim/<chr>/`:

| File | Content |
|---|---|
| `skeleton.bin` | `anim::Skeleton` in postcard |
| `<aXXX_YYYYYY>.bin` | `anim::AnimClip` in postcard |
| `clips.json` | One summary row per clip, decode failures, decoder counters |
| `tae.json` | Every TAE animation, its resolved HKX, events with raw params (hex) and template-decoded fields |

The binary format is [postcard](https://docs.rs/postcard) 1.x of the serde types; read it with `Skeleton::from_bytes` and `AnimClip::from_bytes`.
TAE parameter names and types come from DSAnimStudio's `TAE.Template.SDT.xml`, fetched into `cache/refs/` by `tools/fetch-refs.ps1` (pinned commit).

### Data model

The Havok skeleton (`skeleton.hkx` in the main anibnd) is the bone order clips are bound to; it is not the FLVER node order.
Match FLVER or GLB joints by name.
All Havok bone names exist in the FLVER, and reference-pose positions agree within 0.05 mm for every bone except the IK foot targets (`*_Foot_Target*`) and the soldier's `[omit]` helpers, which are parentless nodes in the FLVER.
Drive those few from Havok model space if they are needed at all.

A clip keeps only the bones it animates (`tracks`), sampled at every authored frame (30 fps in Sekiro).
A channel with a single entry is constant for the whole clip.
`AnimClip::sample(time, &skeleton)` returns local transforms for every bone: tracked bones are interpolated (lerp, shortest-path slerp), untracked bones take the reference pose.
Additive clips (`blend == Additive`, about 1% of clips) store deltas; untracked bones are identity and `Trs::add` applies a delta to a base pose.
`Skeleton::model_space(&pose)` composes local transforms into model-space matrices.

Root motion is separate from the bone tracks, as `AnimClip::root_motion`: evenly spaced samples over the clip, xyz displacement from the start in the character's starting frame, w the accumulated yaw in radians about +Y (quarter and half turns appear as about 1.571 and 3.142).
The `Master` bone does not carry the travel, so apply root motion to the character's transform.
Cameras are bones too: `z_dummy_camera` legitimately moves several metres.

### TAE

Sekiro TAE files are 64-bit, version `0x1000D`.
The player's TAEs are split per category (`a00.tae`, `a50.tae`, `a200.tae`) and store IDs within the category; the full ID is `category * 1_000_000 + id` and the HKX is `a{category:03}_{id:06}`.
The soldier's single `c1010.tae` stores full IDs.
An animation either plays its own HKX, imports another animation's HKX with its own events (`Standard` with `imports_hkx_from`), or imports another animation entirely (`ImportOther`).
`tae.json` resolves chains to the HKX that actually plays.
Each event carries the type of the event group it belongs to as `group`.
Some events end right after their last meaningful field, so trailing asserted padding can be absent; that is not an error.

### Decoder notes

Havok 2016 tagfiles (`TAG0`, SDK `20160200`) either embed their `TYPE` section or reference a `TCM0` compendium in the same binder by an 8-byte ID (`TCRF`).
Values are walked by type and member name, so no Havok class layout is compiled in.
All 1783 clips of the two characters decode.
The spline decoder implements 8 and 16-bit vectors, the 32, 40, 48-bit and uncompressed rotation codecs, and multiple blocks (the longest Wolf clip has 1271 frames across five blocks and is continuous at the boundaries).
`hkaInterleavedUncompressedAnimation` is read too but has not been met in this data.
Float tracks are skipped.
Rotation masks with both static and spline bits set (thousands of tracks) decode as spline, as every public reader does.
Decoded samples match the earlier sekiro-deflection Python decoder within 1e-6 on six clips, including the walk and run loops that project could not export.
That earlier failure was its own range check rejecting `|translation| > 5`, which the camera dummy bone exceeds in locomotion (8.2 m in the walk loop, 11.4 m in the run loop); the clips themselves were fine.

### Regenerate

```powershell
./tools/fetch-refs.ps1
cargo run --release -p sekiro-extract -- anims c0000 c1010
cargo test -p sekiro-formats --test it -- anim_data::
```

## Models: FLVER, TPF, MTD and the GLB export

Code: `crates/formats/src/flver.rs`, `crates/formats/src/tpf.rs`, `crates/formats/src/mtd.rs`, `crates/extract/src/models/`.
Layouts were reimplemented from SoulsFormatsNEXT (`FLVER2`, `TPF`, `MTD`); no code was copied.

### Coordinate convention (shared with animation)

FromSoftware data is left-handed, +Y up, in metres.
Characters face source -Z (the right hand is at source -X in the T-pose).
Everything is mirrored across X once, at export: positions, normals and dummy vectors become `(-x, y, z)`, and every matrix becomes `S M S` with `S = diag(-1, 1, 1)`.
For a local transform that means translation x negated and quaternion `[x, y, z, w] -> [x, -y, -z, w]`, which is what `sekiro_formats::anim::to_bevy` applies to clips.
The result is right-handed +Y up, and characters still face -Z, which is Bevy's `Transform::forward()`.
Triangle index order is kept: FromSoftware front faces are clockwise in their left-handed space, and the mirror makes them counter-clockwise, which is glTF's front face.
This was checked on single-sided meshes (the Wolf's head renders inside-out if the indices are swapped).

### FLVER2 (version 0x2001A)

- Header 0x80 bytes, then fixed-size tables in order: dummies (0x40), materials (0x20), nodes (0x80), meshes (0x30), face sets (0x20), vertex buffers (0x20), buffer layouts (0x10), textures (0x20).
  Names are UTF-16 at absolute offsets; index and vertex data offsets are relative to the header's data offset.
- Nodes hold bones, mesh owners and dummy owners in one table.
  Local transform is `T * Ry * Rz * Rx * S` in column-vector form (Euler radians), see `Node::local_matrix`.
- Dummies: reference id, position, forward, upward, parent node (the space the position is in, usually the root-level `Model_Dmy`) and attach node (the bone it follows, e.g. `R_Weapon`).
  The GLB export copies the skeleton's dummies, mirrored, into the scene `extras.dummies`.
- Meshes: every Sekiro character mesh has an empty local bone palette, and vertex bone indices index the node table directly.
  Verified on c1010: indices reach 109, the last bone-flagged node is 109, and skinning with that reading reproduces the authored shapes.
- Map pieces and objects: unweighted (non-dynamic) vertices are stored in the frame of the node their normal W names (through the mesh bone palette when there is one), and the game draws them through that node's bind-pose matrix (`Mesh::model_space_vertices`).
  Map objects rely on it: an Outskirts bamboo clump (`o112400`-`o112440`) keeps each leaf cluster at its own node's origin, scaled and turned into the crown by the node.
  Evidence: in 2,122 of the 2,170 object FLVERs with such vertices only the transformed vertices reproduce the header bounds (none match better untransformed); the rest are unchanged either way.
  Map pieces only name identity nodes, so they are unaffected.
  No placed piece or object in m11_00_00_00 or m11_01_00_00 has a bone palette or a Float3 normal (which has no W).
  Character meshes are unverified: the map exporter applies this rule, the character exporter does not yet.
- Face sets: keep flags `0` (LOD0); `0x01000000`/`0x02000000` are LOD1/2 and `0x80000000` marks motion-blur copies.
  All observed sets are 16-bit triangle lists; strips with 0xFFFF restarts are unrolled anyway.
  The face set's cull flag becomes glTF `doubleSided`.
- Vertex layouts: each mesh has one or more buffers, each with its own layout; members are decoded per SoulsFormats `Vertex.Read`.
  Sekiro uses Float3 positions, UByte4 normals and tangents (`(b - 127) / 127`), UByte4 bone indices, UByte4Norm weights, UByte4Norm colours, and Short2/Short4 UVs divided by 2048 (Short4 is two UV channels).
  Cloth meshes carry a second position/normal/tangent stream (layout member index 1); the first is the rendered one, extras are kept in `Vertices::extra_*`.
- EdgeGeom (PS3) compression and big-endian files are rejected.

### Materials and textures

FLVER materials name an MTD and list texture slots, but in Sekiro every slot path is empty.
The MTD (`mtd/allmaterialbnd.mtdbnd`) gives each slot a default path such as `N:\...\chr\c1010\tex\c1010_yoroi_merge_a.tif`; the file stem is the TPF texture name.
Textures are looked up in the model's own TPFs (`chr/<id>.texbnd`, the partsbnd), then by path: `\chr\cXXXX\` loads that character's texbnd (c1010 borrows c1019's head textures), `\parts\` loads `parts/common_body.tpf` (the shared `FC_A_0000_*` detail maps) and the named part.
Materials stack many layers; the exporter takes the albedo slot ending in `AlbedoMap_0`, else the first albedo that is not an overlay (`FC_A_0000_*`, damage, expression or skin-detail maps), and the normal map named like that albedo (`x_a` -> `x_n`).
`g_AlphaRef > 0` becomes alpha MASK, `g_BlendMode` 2 or 4+ becomes BLEND.

TPF (PC) is a header plus per-texture records (offset, size, format byte, type, mip count, name offset, optional float block); payloads are complete DDS files.
Sekiro uses BC1, BC4, BC7 (sRGB and linear) and some BC5/BC6H; `tpf::Dds` reads the legacy and DX10 headers and finds mip 0.
Normal maps (`_n`, usually BC7) store X in red and Y in green with blue empty, DirectX style; the exporter rebuilds Z and flips Y for glTF.
Tangents are not exported; Bevy generates MikkTSpace tangents for materials with normal maps.

### Variant meshes

Characters ship every look in one FLVER.
A material named `#NN#...` belongs to display-mask group NN.
For enemies, NpcParam `modelDispMask0..31` of the row `<model number> * 10000` (10100000 for c1010) switches groups on; unmasked meshes always show.
Enemy GLBs contain every variant mesh; the draw mask is game state, not baked in.
Scene extras carry `drawMask` (NpcParam, the out-of-combat look) and `combatDrawMask`, which applies the `ChangeChrDrawMask` TAE event of the weapon-draw animation `a000_001040` (values per group: 0 off, 1 on, 255 unchanged).
Enemy weapons live inside the chrbnd FLVER, skinned to weapon bones that the Havok skeleton animates: for c1010, group 0 is the katana in hand (bone `Kodachi`, under `R_Hand`), group 2 a wakizashi in hand (`Wakizashi`), and groups 1 and 3 the hip pieces on the `Sheath01`/`Sheath02` bones (1 shows the sheathed katana with its hilt, 3 stays on in combat; exact split not verified in game).
Drawing sets groups 0 and 3 on and 1 and 2 off; sheathing (`a000_201000`) turns 1 back on and 0 off.
`sekiro-game` applies a mask with the `DrawMask` component (`crates/game/src/draw_mask.rs`), matching primitives by the `#NN#` FLVER material name; `DrawMask::apply_event` takes TAE event values.
For the player, all groups show unless an equipped protector sets `invisibleFlagNN`.
The Wolf is the c0000 skeleton plus the parts named by CharaInitParam row 10010 (Ashina Castle start: protectors 100000, 131000, 102000, 103000, weapon 5100).
Protector models map to `BD_M_`/`AM_M_`/`LG_M_` by the body/arm/leg flags; head protector model 200 has no `HD_M_0200`, and is the face part `FC_M_0200` (head, hair, eyes, beard).
The weapon's equipModelId gives `WP_A_0300` and its scabbard `WP_A_0300_1`, both hung on c0000 dummy polys whose attach bone they then follow.
Which dummies is the weapon's WepAbsorpPosParam row (EquipParamWeapon `absorpParamId`, 5000 for the katana): `right_0` = 20 for the blade (attach bone `R_Weapon`), `right_1` = 147 for the scabbard (attach bone `Sheath`, left hip), and `rightHang_0` = 147 for the blade while sheathed.
Each part's model space is its dummy's frame: +Z along the dummy's forward, +Y along its upward made orthogonal to forward (`flver::Dummy::frame`); facts from FwLo's RustSekiro (`docs/kb/visuals.md`, `model.rs::dummy_frame`).
Dummy 22 has dummy 20's position and forward but the opposite upward, and dummy 1 sits at the same spot with its forward along the blade; only 20 with the plain frame matches the real game, where the idle blade points forward and slightly out and curves up (#103).
Parts are skinned to the c0000 skeleton by bone name, each with its own inverse bind matrices; part and skeleton bind poses agree within 0.005 (matrix elements).

### Regenerate and view

```powershell
cargo run --release -p sekiro-extract -- models            # c1010 and c0000
cargo run --release -p sekiro-extract -- models c1010 --verbose
cargo run --release -p sekiro-extract -- model-info cache/raw/chr/c1010.chrbnd.d/c1010.flver
cargo run --release -p sekiro-game -- --viewer cache/models/c0000.glb
cargo run --release -p sekiro-game -- --viewer cache/models/c1010.glb --screenshot cache/screens/c1010_front.png
```

The viewer orbits with the left mouse button, raises the target with the right button and zooms with the wheel.
`--hide`/`--only` take comma-separated material tokens: `m12` matches FLVER mesh 12 exactly, anything else is a substring of the material name `m<mesh> <material> | <owner node> | <mtd>`.

## Behaviour graphs: hkbBehaviorGraph in `chr/*.behbnd`

`cXXXX.behbnd` holds the character's Havok behaviour graph as one self-contained `TAG0` tagfile (the type section is inline; no compendium is needed).
The player's is `c0000.hkx` (3 MB, 5,151 generator nodes).
NPCs ship a tiny wrapper (`c1010.hkx`) whose only generator is an `hkbBehaviorReferenceGenerator` naming `Behaviors\c9997`, the shared NPC graph that sits next to it in the same binder.
The reader is `crates/formats/src/hkb.rs`, built on the generic tagfile reader; `cargo run -p sekiro-sim --bin hkb-outline -- <file.hkx>` prints the tree.

### Data model

- `hkbBehaviorGraph.data` holds the string tables: 1,908 event names, 311 variable names and 2,324 animation names for the player.
  Variables have a declared type (bool, int, real, quad, ...) and an initial value in `hkbVariableValueSet`; reals are stored as f32 bit patterns in the word array, vectors index the quad array.
- Every generator becomes a `Node` in one arena with its name, `userData` and variable bindings (`memberPath` such as `selectedGeneratorIndex` or `blendingControlData/weight`, plus a variable index).
- `hkbStateMachine`: states (`stateId`, name, generator, own transitions) and wildcard transitions.
  A transition is an event index, a target state id, optional nested target, priority, flags and a transition effect.
  Every wildcard in the player graph uses flags `0xE00` (global wildcard, local wildcard, self transition allowed).
- `CustomManualSelectorGenerator` (CMSG, 2,041 in the player graph): FromSoftware's per-state animation node.
  It carries the six-digit `animId`, an `offsetType` and one or more candidate clips named `aOOO_IIIIII`; the engine plays the candidate whose `aOOO` prefix matches the offset it derives for that type.
  Observed types: 0 and 11 fixed (mostly `a000`), 13 right-hand weapon motion category (`a050` with the katana), 14 left-hand prosthetic (`a070` to `a079`), 17 throw-specific (`a2xx`).
  `animeEndEventType` says what happens when the clip ends: 0 takes the state's own transition (stand-to-guard into guard idle), 2 sends the CMSG end event (event 0, `Idle_wild`, back to idle), 1 and 3 do nothing (hold or loop).
  These meanings are inferred from the data and checked only against our own simulation, not against the exe.
  `enableScript` marks states with HKS hooks.
- `hkbClipGenerator`: animation name, playback mode (0 single, 1 loop), speed, start time and crop amounts.
- `hkbManualSelectorGenerator` picks one child by `selectedGeneratorIndex`, almost always bound to a variable.
- `hkbLayerGenerator` layers each wrap a generator; the top-level additive layers have their weight bound to a `*Blend` variable (`AddDeflectGuardBlend`, ...), and two have on/off events.
- `hkbBlenderGenerator`, `hkbScriptGenerator` (named script callbacks such as `ModifiersLayer_onGenerate()`), `hkbModifierGenerator` and `CustomDockingGenerator` (kept generically) complete the tree.
- Transition effects are `CustomTransitionEffect` / `hkbBlendingTransitionEffect` (all with duration 0 in the player graph; blending is TAE-driven) and `hkbManualSelectorTransitionEffect`, which picks among effects by a bound variable.

### Blending

TAE event 16 (`Blend`) supplies the incoming clip's fade interval when it starts at time zero.
The template exposes no curve parameter for this event.
The effect's curve 0 is smoothstep, `w = t * t * (3 - 2 * t)`, confirmed in the executable at RVA `1408650` (constants 3 and 1 at `3289230` and `328919c`).
The runtime interpolates translation and scale and uses shortest-path quaternion slerp for rotations with that weight, using the character clock so hit stop also freezes the fade.
An interrupted fade now freezes the displayed local pose, including the unfinished fade, rather than reverting to the outgoing clip's raw pose.
Both Wolf and NPCs use this renderer path.

The installed c0000 graph's blending effects have duration 0 and curve 0, including the effects selected by `hkbManualSelectorTransitionEffect`.
The c9997 graphs in Owl c5060, soldier c1010 and Genichiro c7100 bind the attack wildcards to `CustomTransitionEffect` named `DefaultTransition`, with duration 0, curve 0 and flags 0.
Their other effects (`ThrowTransition`, `MovementResetTransition`, `AddBlendSyncTransition`) also have duration 0 and curve 0.
The attack CMSG `generatorChangedTransitionEffect` references are null.
These graph fields therefore supply no positive fallback duration for an attack that lacks event 16.
Native activation at RVA `11ee170` saves the authored duration and initially raises durations below 0.05 s to 0.05 s.
The duration calculation at `11ee2c0` then resolves the target clip and calls through `11e9180`, `1017f80`, `b5c170` and `b5c0f0` into the animation's duration lookup at `9840f0`.
The TAE metadata builder at `984230` initializes the clip duration to 0 and replaces it with the nonzero end time of event 16, or stores per-key overrides from event 17.
The lookup returns that default when no per-key override exists; all four audited characters have no event 17 and all their event 16 entries start at zero.
Thus a loaded target clip with no Blend event supplies 0, replacing the initial 0.05 s value; there is no positive clip fallback on this traced path.
If the callback cannot supply a duration it returns -1 and the effect restores the authored graph duration, also 0 in these graphs.
The temporary 0.05 s value is not a justified general-purpose fade for missing event 16.
FwLo's MIT visuals notes supplied the zero-duration/TAE interpretation; the values below were independently read from this install with our readers.

Incoming attack blend lengths, in seconds (`none` means no start event, not an authored positive duration):

| Incoming animation id | Owl c5060 | Soldier c1010 | Genichiro c7100 |
|---|---:|---:|---:|
| 3000 | 0.2 | 0.2 | 0.2 |
| 3001 | none | 0.2 | 0.333333 |
| 3002 | none | 0.2 | 0.133333 |
| 3003 | 0.2 | 0.2 | 0.3 |
| 3004 | 0.2 | none | 0.2 |
| 3005 | 0.233333 | 0.2 | 0.2 |
| 3006 | none | none | 0.266667 |
| 3007 | none | 0.2 | 0.333333 |
| 3008 | 0.233333 | 0.2 | 0.2 |
| 3009 | 0.233333 | 0.2 | 0.2 |
| 3010 | 0.2 | 0.2 | none |
| 3011 | 0.233333 | 0.2 | 0.333333 |
| 3012 | 0.166667 | 0.3 | 0.166667 |
| 3013 | 0.233333 | 0.2 | 0.166667 |

Complete Owl attack-entry inventory, grouped by the incoming duration (each id has prefix `a000_`):

| Seconds | Owl animation ids |
|---|---|
| 0, no event | 3001, 3002, 3006, 3007, 3014, 3016, 3017, 3030, 3032, 3035, 3036, 3046, 3090 |
| 0.1 | 3020, 3021, 3022, 3023, 3027, 3037, 3039, 3044, 3092, 3093 |
| 0.133333 | 3029 |
| 0.166667 | 3012, 3018, 3019, 3025, 3026, 3028, 3038, 3040, 3047, 3049, 3065, 3069, 3091 |
| 0.2 | 3000, 3003, 3004, 3010, 3015, 3024, 3031, 3033, 3034, 3041, 3042, 3043, 3045, 3048, 3060, 3061, 3064, 3066, 3068 |
| 0.233333 | 3005, 3008, 3009, 3011, 3013, 3063 |

Owl's bound reactions 8600 and 8700 each specify 0.033333 s; attack-to-attack and recoil-to-attack entry use the incoming attack's event above.
TAE events remain on the requested animation's timeline, including aliases that borrow another HKX; pose weights never gate hit windows or cancel flags.
Root displacement and yaw continue to come exclusively from the incoming simulated clip.
This preserves combat and movement timing; the native engine's root blending is still unverified (FwLo's hand-off also records it as untraced).

`crates/sim/tests/blending_data.rs` dumps the transition effects, attack wildcards, CMSG references and event lengths without writing extracted files.
The local full dump is `target/alt/blending/data-audit.txt`.
Read-only native research ran against a copy of the existing Ghidra project; outputs and the copy stay in `target/alt/blending/`.
The scripted roof replay `crates/game/scripts/owl_blending.txt` deflects at step 61 and replaces recoil 8600 with attack 3003 at step 98.
Before/after images at steps 97, 99, 101, 103, 105 and 107 are in `target/alt/blending/`.
The inspected contact sheet is `target/alt/blending/frames.png`; the after sequence eases the arms into the new attack without an abrupt position change.
The before sequence already blends, so this replay does not establish that the reported late deflect into a different combo continuation has been fixed.
Screenshot scheduling captured the final after image one fixed tick later than its before counterpart (HUD clip time 0.17 versus 0.15 s).
All 251 state/position entries in the two simulation traces match exactly, as do the deflect at tick 61 and the next contact at tick 159.

### How the player graph is organised

The root machine has one state, `Master`, whose script and modifier generators lead to `Master LayerGenerator`.
Its layers are the additive machines (`AddActionInput_SM`, `AddDeflectGuard_SM`, `AddDamage_SM`, ...) and `Master Blend`, which holds `Master_SM` (172 states) for the full body.
`Master_SM` states group behaviours into nested machines (`Idle_SM`, `DeflectGuard_SM` with 44 states, `StandMoveableAction_SM`, `GroundAttack_SM`, ...).
Script events (`W_StandToDeflectGuard`) are global wildcards of the nested machine that owns the target state, so firing one also switches every enclosing machine to the state containing it.
HKS state hooks are named after the state, not the CMSG: `StandToDeflectGuard_onActivate`, `_onUpdate`, `_onDeactivate`.

### Runtime

`crates/sim/src/behavior.rs` (`BehaviorRuntime`) runs a graph: current state per machine, event matching (active machines first, then global wildcards of inactive ones), the active clip set with times, layer enable rules, clip-end events and state hooks.
Simulation transitions switch immediately; the game renderer crossfades the full-body pose using the incoming TAE Blend duration.
`crates/sim/src/player.rs` (`PlayerBehavior`) wires it to the HKS VM; `cargo run -p sekiro-sim --bin player-sim -- hold` (or `repeat`) prints state and animation per frame.

## Maps: MSB layouts, map pieces and hit collision

Code: `crates/formats/src/msb.rs`, `crates/formats/src/bxf4.rs`, `crates/formats/src/hknp.rs`, `crates/extract/src/maps/`, `crates/game/src/map.rs`.
Layouts were reimplemented from SoulsFormatsNEXT (`MSBS`, `BXF4`) and DSMapStudio's collision loader (`HavokCollisionResource`, HKX2 `hknpCompressedMeshShapeData`); no code was copied.

### Area

All ten mapstudio MSBs in the install archive now extract and load; the complete inventory, start audit and screenshots are in [MAP-BASELINE.md](MAP-BASELINE.md).
`m11_00_00_00` uses Ashina Outskirts (place name 1100); `m11_01_00_00` uses Ashina Castle (1110), and `m11_02_00_00` uses Ashina Reservoir (1120).
It places 43 Ashina soldiers (`c1010`) among 144 enemies, 4498 map pieces of 535 models, 1509 objects, 166 hit collision parts and 10 player start points.

### MSB (`map/mapstudio/<id>.msb`)

- Header `MSB `, then a chain of params, each: version, entry count + 1, name offset, entry offsets, next-param offset (0 ends the chain).
  The loader reads `MODEL_PARAM_ST` and `PARTS_PARAM_ST` and skips the rest (events, regions, routes, layers).
- Models: name, type (0 map piece, 1 object, 2 enemy, 4 player, 5 collision), SIB path, instance count.
- Parts: name, type, model index, position, rotation in degrees, scale, then offsets to optional blocks.
  The transform composes like FLVER nodes: `T * Ry * Rz * Rx * S` in column-vector form (`Part::matrix`).
- Part mask block (map pieces, objects, enemies, collisions): 48 words, which DSMapStudio splits as display groups[8], draw groups[8], collision mask[32].
- Enemy type data: think param, NPC param, talk id, platoon, chara init, collision part index, backup event anim, event flag.
- Collision type data: hit filter, sound space, map name id (the place-name FMG id), play region.

### BXF4 split binders (`.tpfbhd` + `.tpfbdt`, `.hkxbhd` + `.hkxbdt`)

The `BHF4` header is laid out exactly like a BND4 header (same format byte and per-file records); payload offsets point into the separate `BDF4` file.
Every payload in Sekiro's map binders is a KRAK (Oodle) DCX, so the map export needs the game folder for `oo2core_6_win64.dll` (read only).
Map textures live in `map/mXX/mXX_000N.tpfbhd`, one TPF per texture; `other/maptex.tpf` and each object's own TPF fill the gaps.

### Map pieces and textures

- `map/<id>/<id>_<model digits>.mapbnd.d/<id>_<model digits>.flver`, with a `_S` sibling that is not used.
  Map piece FLVERs are static (one node, no skinning); LOD0 face sets only.
- Map materials leave FLVER texture paths empty and stack many layers (`M_Multiple`, `MultiBlend3`, ...).
  The base colour is the first albedo slot with a path in MTD order (for `M_Multiple` slot 8, the main surface; moss and snow overlays come later), or the slot named `<material>_a`.
- Real UVs are the Short2/Short4 channels (UV 0 and 1); the UByte4Norm "UVs" carry blend data and are not exported.
- Textures are written as the DDS files the TPFs contain (BC1 and BC7 sRGB in m11), next to the GLBs in `models/tex/`, so the GPU gets the original block compression and mip chain.
- Materials whose MTD is a shadow caster, dummy, invisible blocker or fog card (`GroundFog`, `LandScape_CloudFog`) are not exported.

### Hit collision (`map/<id>/h<area>.hkxbhd`)

- One Havok 2016 tagfile per collision model, sharing a type compendium stored in the same binder.
  Each holds `hknpPhysicsSceneData` with one `hknpPhysicsSystemData` whose body uses `fsnpCustomParamCompressedMeshShape`.
- Compressed mesh: sections with a run of primitives (`primitives.data`: start `>> 8`, count `& 0xFF`) and packed vertices.
  `sharedVertices.data & 0xFF` is the section's packed vertex count and `>> 8` its first entry in `sharedVerticesIndex`.
  A primitive index below the packed count is a packed vertex (x 11, y 11, z 10 bits, scaled by the section's `codecParms` scale and offset); otherwise it is a shared vertex (x 21, y 21, z 22 bits over the mesh tree domain).
  Indices `(a, b, c, d)` with `c != d` are a quad, split as `(a, b, c)` and `(a, c, d)`; `DE AD DE AD` marks an empty primitive slot (78 in m11, skipped).
  Over every `h` and `l` binder of the nine exported maps no authored triangle's shape key points at one, and every shape's `numConvexShapes` is 0, so there is no convex shape to decode (#189).
- Materials: `triangleIndexToShapeKey` maps each authored triangle to its shape key (left-aligned in `numShapeKeyBits`), and `pParam.triangleDataArray[i].primitiveDataIndex` selects a `PrimitiveData` whose `materialNameData` is the hit material id.
  The shape key of a decoded triangle is `(section << 8) | (primitive << 1) | second_triangle_of_quad`; with that every one of the 2.3 million m11 triangles finds its material (34 distinct ids).
- The HKX bodies carry identity transforms and the vertices are in the collision model's own space: the MSB collision part transform places them like any other part (all m11 collision parts share `(90, 3, -40)`, rotated 20 degrees about Y).
  Check: with the transform every player start is within 0.7 m of a collision vertex, without it up to 51 m away.
- Each block has two more hit binders beside `h`, with the same layout: `l<area>.hkxbhd` (low resolution) and `f<area>.hkxbhd` (the fall-prevention assist walls, `docs/OBJECT-COLLISION.md`).
- The exe gives an ordinary part's `h` body Havok group 38 and its `l` body group 39 or 40; characters and the camera collide only with 39 and 40, so they walk on `l` (`docs/HIT-FILTERS.md`).
  The `h` mesh is what projectile rays, foot IK, ragdolls and debris meet.
- `sekiro-extract map <id> --lo-collision` (and every full `map` export) writes the `l` binder, placed per MSB collision part like `collision.bin`, to `collision_lo.bin` (#189).
  It is written whole or not at all: a model that fails to decode fails the `l` export with the models named, and the last `collision_lo.bin` (or, without one, `collision.bin`) stays in use.
  All nine exported maps decode with no failures.
  `l` has no unset materials: `h` has 116,158 triangles with material `u32::MAX` across the nine exported maps, `l` none.
  In both binders a material id carries the special setting: it is `special * 100 + row`, so `l`'s 721, 722 and 735 are HitMtrlParam rows 21, 22 and 35 (water) with special setting 7, which `h` never uses in these maps.
  The simulation takes the row with `% 100` (`body::hit_mtrl_row`) for fall death, footsteps and water.

| Map | `l` triangles | Collision parts with an `l` model | Empty `DE AD DE AD` slots in `l` |
|---|---|---|---|
| m10_00_00_00 | 273,514 | 143 of 144 | 2 |
| m11_00_00_00 | 279,243 | 164 of 166 | 7 |
| m11_01_00_00 | 233,985 | 220 of 221 | 69 |
| m11_02_00_00 | 65,619 | 71 of 72 | 2 |
| m13_00_00_00 | 89,302 | 46 of 46 | 3 |
| m15_00_00_00 | 171,407 | 131 of 131 | 1 |
| m17_00_00_00 | 222,364 | 139 of 140 | 3 |
| m20_00_00_00 | 351,872 | 144 of 144 | 120 |
| m25_00_00_00 | 479,127 | 240 of 240 | 15 |

- Parts with an `h` model and no `l` model: m11_00's `h900700` (hit filter 8, 75 `h` triangles) and m11_01's `h012400` (hit filter 8, 2 `h` triangles).
  m11_00's `h009500` has no model in either binder, nor do the other parts the counts above leave out.
  A part without an `l` model gets an empty range in `collision_lo.bin`, so characters and the camera do not meet it, as in the exe.

### Draw groups

In the game the hit collision under the player selects which pieces draw, swapping distant stand-ins (`m9*` far models, low-detail blocks) for detailed geometry.
The map viewer finds the collision part under the camera and shows pieces whose draw groups intersect that collision's draw groups (`--groups draw`, the default).
Using the collision's display groups instead (`--groups display`) looked the same from the starts compared, but 49 of 166 collision parts, including the ones under player starts 5 and 7, have no display groups, while the collision under every player start tested (0 to 5 and 7) has draw groups.
Which block the engine really uses is not confirmed.
Eleven map pieces have no draw groups at all and are left out of the layout.
This suppression applies only to maps containing collision parts.
`m89_00_00_99` has no collision parts and has zero-group sky-dome pieces, so those pieces must be retained.
Its 26 visible pieces are co-located sky variants, not walkable terrain; choosing their event state and sky shader remains unimplemented.

### Findings across all maps

The English place-name FMG uses version 2, 16-byte groups (`offset index`, first ID, last ID, padding), and 64-bit string offsets.
MSB collision map-name IDs identify Hirata Estate (1000), Ashina Outskirts (1100), Ashina Castle (1110), Ashina Reservoir (1120), Abandoned Dungeon (1300), Mibu Village (1500), Ashina Depths (1510), Sunken Valley (1700), Sunken Valley Passage (1710), Senpou Temple, Mt. Kongo (2000), and Fountainhead Palace (2500).
Boundary collision parts can name adjoining areas, so these IDs do not imply a one-name-per-MSB correspondence.

An object binder need not have render geometry.
`o115923.objbnd` contains only HKX physics and no FLVER; the extractor identifies this membership pattern and counts it as nothing visible rather than a failed model.
An unrelated binder with neither HKX nor FLVER still reports a missing model.

Ordinary map GPARAM lookup uses `mAA_BB_0000.gparam`.
If that file is absent, cinematic map `mAA_BB_CC_DD` tries `sAA_BB_CCDD.gparam`; m89 therefore uses `s89_00_0099.gparam`.
Explicit GPARAM overrides take precedence.

### Cache layout

`sekiro-extract map <id>` writes `cache/maps/<id>/`:

| File | Content |
|---|---|
| `models/<model>.glb` | One static GLB per placed map piece and object model, mirrored on X like every export |
| `models/tex/<name>.dds` | Textures the GLBs reference by relative URI |
| `layout.json` | Bevy-space instances: `pieces`, `objects`, `enemies` (chr model, NPC and think params, dummy flag), `players`, `collisions` (groups, triangle range in `collision.bin`), `models` (GLB, triangle count, bounds) |
| `collision.bin` | `sekiro_formats::hknp::CollisionMesh` in postcard: vertices, indices, one hit material id per triangle, in Bevy space (the `h` binder) |
| `collision_lo.bin` | `sekiro_formats::hknp::PartCollision` in postcard: the `l` binder's mesh like `collision.bin`, with each collision part's triangle range |
| `fall_assist.bin` | `PartCollision` of the `f` binder's fall-prevention assist walls |

Transforms in `layout.json` are translation, quaternion `[x, y, z, w]` and scale, already mirrored (`M -> S M S`).
Enemy and player rows also carry `yaw_degrees`, the Bevy-space yaw (FromSoftware's Y rotation negated).

### Regenerate and view

```powershell
cargo run --release -p sekiro-extract -- map m11_00_00_00 --summary
cargo run --release -p sekiro-extract -- map m11_00_00_00
cargo run --release -p sekiro-extract -- map m11_00_00_00 --lo-collision
cargo run --release -p sekiro-game -- --map m11_00_00_00 --start 3
cargo run --release -p sekiro-game -- --map m11_00_00_00 --start 3 --screenshot cache/screens/map_m11_gate.png --at 2
cargo run --release -p sekiro-game -- --map m11_00_00_00 --start 3 --collision --no-pieces
```

The viewer flies with the right mouse button held (look), WASD, Space/E and Q/Ctrl for up and down, Shift for speed and the wheel for base speed.
C toggles the collision wireframe, G cycles draw-group modes, M toggles enemy (red, dummy orange) and player start (blue) markers, P prints the camera pose as a `--camera x,y,z,yaw,pitch` argument.
`--objects` also spawns MSB objects, `--hide m9,o11` skips models by name prefix, `--uncapped` turns vsync off.

### Known approximations

- Normal maps: Sekiro's are BC7 with X and Y in red and green (DirectX Y down); the export decodes them, flips Y and re-encodes BC5 with a box-filtered mip chain, which Bevy reads as two-channel and rebuilds Z from.
  The FLVER tangents are not exported, so Bevy generates MikkTSpace tangents at load (the map takes about 14 s to load instead of 2 s).
  The normal map is the one named like the albedo, else the one named after the material, else the first; detail normals of other layers are ignored.
- Single albedo layer: moss, snow and blend-mask layers, vertex-colour blending, detail maps and the multi-layer shaders are ignored.
- Objects are off by default: many are event-state props (siege barricades, a 400 m dome around the castle gate) that the game enables from its event scripts.
- Sun, ambient light and distance fog come from the map's GPARAM; local light sets and full shader/exposure fidelity remain approximate.
- Bevy's IO threads overflow the default 2 MiB stack while loading all map GLBs and DDS files at once (still at 4 MiB), so `--map` raises `RUST_MIN_STACK` to 16 MiB before Bevy starts; the root cause inside Bevy's loaders was not tracked down.

### Walking on the map

Code: `crates/sim/src/collision.rs` (`CollisionWorld`), the `Ground` trait in `crates/sim/src/body.rs`, `crates/game/src/play.rs`.

- Characters and the camera collide with `collision_lo.bin` (`crates/sim/src/lo_collision.rs`), with the layout's part ranges re-ranged into it for the story's part removal and the helpers.
  A block exported before it existed falls back to `collision.bin`, with one log line per block that names the `--lo-collision` re-export when the cache has the raw `l` binder to export it from.
  `h`'s swim hits carry no water material, so such a block has no swim planes.
  The draw groups follow the part the player stands on, so the game's map world (`MapWorld::load`) reads the parts from `l` too, each part's range from `collision_lo.bin`, and from `collision.bin` only for a block without one.
  The `h` mesh stays for drawing: the collision wireframe and the render sweeps.
  `CollisionWorld::load` reads a collision file as it is, every triangle solid for everyone; a map as characters meet it is `CollisionWorld::load_map`.
- The collision is bucketed in a 4 m grid on the horizontal plane (2.3 million triangles index in 0.2 s).
- Floors: the highest surface within 50 degrees of level under the feet, no higher than a step (0.5 m) above them, sampled at the centre and four points 8 cm around it so seams between triangles do not drop the body.
  Tests use `|normal.y|` and a vertical line, so the inconsistent winding of the collision (and the X mirror) does not matter.
  A grounded body follows the floor up or down by a step plus what a 50 degree slope rises over the tick's travel; an airborne body lands on the highest floor between its previous height plus a step and its new height.
- Walls: three spheres of radius 0.35 m stacked above the step zone are pushed out of every nearby triangle horizontally, in sub-steps of half a radius, so the body stops at walls, slides along them, and treats steep slopes as walls.
- The camera is pulled in front of the first triangle between Wolf and the eye.
- `crates/sim/tests/map_collision.rs` walks the castle gate route (off the start terrace, through the moat gate, up the stairs, and back down) at run and sprint speed and asserts every grounded tick stands on the highest floor within a step; it fails without the step-tolerant landing.

```powershell
cargo run --release -p sekiro-game -- --play --map m11_00_00_00 --start 3
cargo run --release -p sekiro-game -- --play --map m11_00_00_00 --start 3 --no-enemy --script crates/game/scripts/m11_gate_route.txt --shots 860:cache/screens/play_m11_stairs.png --exit
cargo run --release -p sekiro-game -- --play --map m11_00_00_00 --spawn -142.6,-44.0,125.0,180,0 --no-enemy --script crates/game/scripts/m11_ledge_jump.txt --shots 125:cache/screens/play_m11_ledge_top.png --exit
```

`--spawn x,y,z,yaw,0` puts Wolf on the floor at or below `y + 1` instead of at a player start; `PLAY_LOG_ALL=1` logs every simulation step of a script instead of every 30th.

## Event scripts: EMEVD

`event/<map>.emevd` (and `common.emevd`, `common_func.emevd`) arrive already decompressed in `cache/raw/event/`.
`crates/formats/src/emevd.rs` reads them (layout from SoulsFormats `EMEVD`, the 64-bit DS3 / Sekiro form, version 0xCD): a header of 64-bit counts and offsets, 0x30-byte events (id, instruction range, parameter range, rest behaviour), 0x20-byte instructions (bank, id, argument length and offset into one argument blob, layer offset), 0x20-byte parameters (instruction index, target byte, source byte, byte count) and the linked-file names.
`InitializeEvent` (2000[0]: slot, event id, arguments) and `InitializeCommonEvent` (2000[6]: event id, arguments, for `common_func.emevd`) start events; `Event::resolve` copies a start's arguments into the instructions as the parameters say.
`crates/formats/src/emedf.rs` names the instructions the boss events use and decodes their arguments (packed with natural alignment, padded to 4 bytes), after DarkScript3's Sekiro EMEDF.

```powershell
cargo run -p sekiro-formats --example emevd_dump -- cache/raw/event/m11_01_00_00.emevd --find 1110850
cargo run -p sekiro-formats --example emevd_dump -- cache/raw/event/m11_01_00_00.emevd 11115870
```

The interpreter that runs them in a fight is `crates/sim/src/events.rs` (`docs/OWL.md`, "Phase 2").

## FXR effects

`crates/formats/src/fxr.rs` reads the little-endian version 5 files in the installed `sfxbnd_*.ffxbnd` BND4 bundles.
The 144-byte header contains eleven offset/count pairs, two flags, three further offset/count pairs and trailing padding.
The primary record strides are 16 bytes for states/transitions, 96 for conditions, 48 for nodes, 32 for emitter configurations, 64 for actions, 40 for properties, 32 for modifiers, 24 for the preserved ninth table, 16 for sequence descriptors and 4 for raw values.
Record links are 64-bit even though the header table offsets are 32-bit.
Node links select contiguous ranges of configurations, actions and children.
Actions retain the low-16-bit ID, full tag, variant, two fixed-field groups, two property groups and sequence references.
Properties preserve the evaluator, raw payload bits and modifier range.
Constants and curve payload arrays are available separately.
`Action::property`, `field`, `float_field` and `transform` expose the implemented Sekiro positions from EvenTorset's Unlicense schemas.
These cover appearance properties for 603/604/609, emission 300/301, attributes 128/129/131, movement 55/60, shape dimensions and static transforms 35/36.
`Fxr::initial_configuration` resolves action 199's state-zero configuration mapping.
`Property::compile_unmodified` compiles constants or control-free key arrays into allocation-free scalar/RGBA samples with an explicit caller-selected linear or stepped interpolation policy.
It preserves the argument ID, holds endpoints and rejects non-finite or unordered keys, modifiers, looping and cubic evaluators.
This utility does not establish native function-ID or clock mappings.
`Action::appearance` names 603/604/606-609, and `Blend::from_raw` preserves additive, source, alpha, multiply and subtract operations from the existing FwLo notes.
The named action accessors assign supported Sekiro field positions; appearance classification alone does not establish renderer support.
`Property::compile_native_unmodified` adds native stepped/linear/Bezier/Hermite families and looping with finite-key validation.
Hermite controls are angles, using the curve family described by FwLo's MIT engine research.
Random modifiers remain explicit errors; unsupported properties do not silently become linear curves.
The original bytes preserve remaining condition and extension-table fields.
Unknown actions are never rejected solely for their IDs.

`cargo run --offline --release -p sekiro-formats -j 6 --example fxr_dump -- cache/raw/sfx/sfxbnd_commoneffects.ffxbnd.d/f000220505.fxr` prints a read-only inspection.
Set `CARGO_TARGET_DIR=target/alt` and `CARGO_BUILD_JOBS=6` before running it.
`crates/game/scripts/fxr_audit.ps1` checks every installed binder and unpacked FXR, without proprietary fixtures in source.
The current cache has 2,913 FXR entries across eight bundles, including 1,856 common effects and 450 m11 effects.
A first 603-only player in `crates/game/src/fxr.rs` draws HDR, depth-tested texture/blend mesh batches with alpha remapping, finite lifetimes and bounded emissions.
The structural coverage is not a rendering-fidelity claim.

TAE effect requests 96/99/100/118-121 now decode into `TaeEffect` in the simulation, preserving body/weapon dummy IDs, slots, flags, repeat metadata and StateInfo gating.
The tick's `TaeFrame.effects` distinguishes start edges from still-active requests.
The player consumes eligible body-dummy cues and Bullet `sfxId_Bullet` requests through a single play-plugin registration.
Dummy position is posed, but orientation currently uses character yaw; restricted missing dummies, weapon/blade requests, extra conditions and repeating variants remain unsupported.
`--no-fxr` disables playback entirely, leaving the previous stand-ins.
Stand-ins are also retained while the player has incomplete action support; a playable layer does not establish that its whole effect is supported.

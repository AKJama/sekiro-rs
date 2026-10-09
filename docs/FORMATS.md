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
cargo test -p sekiro-formats --test anim_data
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
The katana sits on dummy 1 (attach bone `R_Weapon`) with its blade (model -Y) along the dummy's forward vector; attached straight to the `R_Weapon` bone it would point backwards in every pose.
The scabbard sits on dummy 147 (attach bone `Sheath`, left hip) with its model +Y along the dummy's upward vector.
The two conventions were chosen by checking the idle stance (blade forward and down) and the front deathblow (blade through the enemy); how the engine itself orients weapons on dummies is not confirmed.
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

### How the player graph is organised

The root machine has one state, `Master`, whose script and modifier generators lead to `Master LayerGenerator`.
Its layers are the additive machines (`AddActionInput_SM`, `AddDeflectGuard_SM`, `AddDamage_SM`, ...) and `Master Blend`, which holds `Master_SM` (172 states) for the full body.
`Master_SM` states group behaviours into nested machines (`Idle_SM`, `DeflectGuard_SM` with 44 states, `StandMoveableAction_SM`, `GroundAttack_SM`, ...).
Script events (`W_StandToDeflectGuard`) are global wildcards of the nested machine that owns the target state, so firing one also switches every enclosing machine to the state containing it.
HKS state hooks are named after the state, not the CMSG: `StandToDeflectGuard_onActivate`, `_onUpdate`, `_onDeactivate`.

### Runtime

`crates/sim/src/behavior.rs` (`BehaviorRuntime`) runs a graph: current state per machine, event matching (active machines first, then global wildcards of inactive ones), the active clip set with times, layer enable rules, clip-end events and state hooks.
Blends are not modelled; transitions switch instantly.
`crates/sim/src/player.rs` (`PlayerBehavior`) wires it to the HKS VM; `cargo run -p sekiro-sim --bin player-sim -- hold` (or `repeat`) prints state and animation per frame.

## Maps: MSB layouts, map pieces and hit collision

Code: `crates/formats/src/msb.rs`, `crates/formats/src/bxf4.rs`, `crates/formats/src/hknp.rs`, `crates/extract/src/maps/`, `crates/game/src/map.rs`.
Layouts were reimplemented from SoulsFormatsNEXT (`MSBS`, `BXF4`) and DSMapStudio's collision loader (`HavokCollisionResource`, HKX2 `hknpCompressedMeshShapeData`); no code was copied.

### Area

`m11_00_00_00` holds both Ashina Outskirts and Ashina Castle (place names 1100 and 1110 in the English `item.msgbnd` place-name FMG).
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
  Indices `(a, b, c, d)` with `c != d` are a quad, split as `(a, b, c)` and `(a, c, d)`; `DE AD DE AD` marks a convex primitive (78 in m11, skipped).
- Materials: `triangleIndexToShapeKey` maps each authored triangle to its shape key (left-aligned in `numShapeKeyBits`), and `pParam.triangleDataArray[i].primitiveDataIndex` selects a `PrimitiveData` whose `materialNameData` is the hit material id.
  The shape key of a decoded triangle is `(section << 8) | (primitive << 1) | second_triangle_of_quad`; with that every one of the 2.3 million m11 triangles finds its material (34 distinct ids).
- The HKX bodies carry identity transforms and the vertices are in the collision model's own space: the MSB collision part transform places them like any other part (all m11 collision parts share `(90, 3, -40)`, rotated 20 degrees about Y).
  Check: with the transform every player start is within 0.7 m of a collision vertex, without it up to 51 m away.
- The low-resolution `l*` and `f*` binders are not decoded.

### Draw groups

In the game the hit collision under the player selects which pieces draw, swapping distant stand-ins (`m9*` far models, low-detail blocks) for detailed geometry.
The map viewer finds the collision part under the camera and shows pieces whose draw groups intersect that collision's draw groups (`--groups draw`, the default).
Using the collision's display groups instead (`--groups display`) looked the same from the starts compared, but 49 of 166 collision parts, including the ones under player starts 5 and 7, have no display groups, while the collision under every player start tested (0 to 5 and 7) has draw groups.
Which block the engine really uses is not confirmed.
Eleven map pieces have no draw groups at all and are left out of the layout.

### Cache layout

`sekiro-extract map <id>` writes `cache/maps/<id>/`:

| File | Content |
|---|---|
| `models/<model>.glb` | One static GLB per placed map piece and object model, mirrored on X like every export |
| `models/tex/<name>.dds` | Textures the GLBs reference by relative URI |
| `layout.json` | Bevy-space instances: `pieces`, `objects`, `enemies` (chr model, NPC and think params, dummy flag), `players`, `collisions` (groups, triangle range in `collision.bin`), `models` (GLB, triangle count, bounds) |
| `collision.bin` | `sekiro_formats::hknp::CollisionMesh` in postcard: vertices, indices, one hit material id per triangle, in Bevy space |

Transforms in `layout.json` are translation, quaternion `[x, y, z, w]` and scale, already mirrored (`M -> S M S`).
Enemy and player rows also carry `yaw_degrees`, the Bevy-space yaw (FromSoftware's Y rotation negated).

### Regenerate and view

```powershell
cargo run --release -p sekiro-extract -- map m11_00_00_00 --summary
cargo run --release -p sekiro-extract -- map m11_00_00_00
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
- Lighting is a single sun, ambient light and distance fog, not the map's GPARAM light sets.
- Bevy's IO threads overflow the default 2 MiB stack while loading all map GLBs and DDS files at once (still at 4 MiB), so `--map` raises `RUST_MIN_STACK` to 16 MiB before Bevy starts; the root cause inside Bevy's loaders was not tracked down.

### Walking on the map

Code: `crates/sim/src/collision.rs` (`CollisionWorld`), the `Ground` trait in `crates/sim/src/body.rs`, `crates/game/src/play.rs`.

- `collision.bin` is bucketed in a 4 m grid on the horizontal plane (2.3 million triangles index in 0.2 s).
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

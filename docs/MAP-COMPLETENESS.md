# Map completeness: Ashina Outskirts and Castle

## Grass

Grass is attached to a MapPiece model through the sibling `.grass` member of its map binder, rather than placed as MSB Objects.
The version-1 file has a 40-byte header, 20-byte BVH nodes, 36-byte vertices containing local position and six painted density floats, 24-byte faces containing a normal and three indices, and 24-byte BVH bounds.
These are [SoulsFormats format facts](https://github.com/soulsmods/SoulsFormatsNEXT/blob/master/SoulsFormats/Formats/GRASS.cs).
The six consecutive MSBS grass-config integers bind those six channels to GrassTypeParam rows, in byte order; the reader's inherited `grass_type_param_id` name identifies only the third integer.
Zero IDs disable individual channels even where their density surface is painted.
Types 135 and 136 in the first two slots select the large and small dry-grass models `o007600` and `o007610`, explaining why counting only positive third-slot IDs missed most grass-bearing parts.
Surface positions and normals are transformed to world space before sampling, so scaled parts change density counts and the hex grid stays anchored in world XZ.
Finished placements are converted back under the mirrored parent, cancelling its scale on the clump model; MapOffset is not added again.

Static analysis of the installed executable identifies distribution type 0 as Halton with bases 2 and 3, and type 1 as a hexagonal XZ grid.
RVA `f93320` initializes the Halton bases, `f93970` counts triangle area times the mean of its three channel densities times the type's baseDensity, and `f93e50` warps barycentric weights by each corner density plus one third before normalization.
The Halton path carries fractional counts between faces; spacing belongs to the hex-grid path, not the Halton density divisor.
RVA `f945d0` uses the authored spacing, projected triangle containment and interpolated height for the hex grid; dithering value 3 bypasses its coverage threshold.
The type row also supplies the FLVER model, scale ranges in percent, normal influence, inclination, orientation, shadow and wind settings.
Models and their alpha-tested materials/textures use the existing FLVER/MTD export path, retaining scene lighting and snow metadata.
Research dumps remain local under `re/exports/grass/`; no decompiler output is included in source.

`sekiro-extract map` writes an independent `grass/grass.json` with source-part bindings, type provenance and parent-local placements, plus `grass/models/` and its DDS textures, for the two Ashina maps only.
The canonical layout, model inventory and visibility identities remain independent of this additional geometry.
`map_grass.rs` shares mesh and material handles for automatic GPU instancing and parents clumps to the existing terrain roots, inheriting draw, story and residency decisions without adding visibility-manifest parts.
Its temporary hidden asset-loading scenes participate in the existing readiness walk and are removed before diagnostic enumeration.
`SEKIRO_NO_GRASS=1` provides a paired control without changing fixtures or cache data.

Remaining fidelity limits are the native BVH leaf seed and cross-channel fractional-count ordering, exact transform jitter and hex dither for values other than 3, grass-specific distance representations, and the wind waveform and units.
Placements are deterministic, but their seed/rounding grouping is an approximation, and clumps currently retain their authored rest pose rather than using an invented wind animation.
The authored wind and all type-row fields remain in the manifest for a later verified shader implementation.

### Grass verification (15 October 2026)

Only Outskirts and Castle were fully re-exported, using the job's release extractor under `target/alt-grass` with six Cargo jobs and six Rayon workers.
A grass-only refresh with that extractor then incorporated world-space sampling without changing either canonical layout.
Both complete `layout.json` documents compare equal to their pre-export snapshots, including model inventories, transforms, masks and every part.
The grass provenance audit resolves every emitted channel back to its MSB configuration and checks all declared model image URIs exist.

| Map | Clumps | Source parts | Channel bindings | Models | DDS textures |
| --- | ---: | ---: | ---: | ---: | ---: |
| Outskirts | 7,740 | 113 | 241 | 11 | 24 |
| Castle | 6,813 | 45 | 88 | 7 | 18 |

Both required six-view render gates pass with the existing fixtures and tolerances, taking 40.7 seconds for Outskirts and 47.6 seconds for Castle.
All twelve selected collisions and sorted visible-part sets are exactly unchanged; grass adds child geometry without adding canonical part identities.
The following totals report every geometry and colour category without changing colour fixtures.

| Map | Void before / after | Depth gap before / after | Untextured before / after | Grey before / after | Pop-in before / after |
| --- | ---: | ---: | ---: | ---: | ---: |
| Outskirts | 0 / 0 | 493 / 493 | 0 / 0 | 321 / 324 | 24,353 / 24,369 |
| Castle | 1,933 / 1,933 | 4,886 / 4,885 | 903 / 903 | 398 / 409 | 26,629 / 26,537 |

The only geometry-metric change is Castle view 0, whose depth-gap count decreases from 149 to 148.
A six-view `SEKIRO_NO_GRASS=1` control with the same new executable restores 4,886 total depth-gap pixels and also passes the gate in 59.9 seconds while tests run concurrently.
Its collision PNG is byte-identical to the grass-enabled capture.
At pixel `(88, 39)`, collision depth remains 5.485 m while the diagnostic visual depth changes from 10.048 m to 7.358 m, so the added grass clears the existing 7.485 m gap threshold.
Every other view and geometry category remains numerically identical.
The per-view colour counts and visible-part comparisons are recorded in `target/alt-grass/gate-comparison.json`, and the independent pixel calculation is in `target/alt-grass/castle-grass-pixel.json`.

The fixed cameras reuse the reviewed matcher poses for Outskirts spot 013 / shot 044 and Castle spot 060 / shot 213, at the recorded player focus, 43-degree FOV and 2560 x 1440 resolution.
Visual inspection confirms dry grass and weeds on the previously bare foreground slopes and beside the aligned walls and paths.
The Castle camera can intersect a generated foreground blade because exact distribution phase and native near-camera grass handling remain unresolved; the source screenshot does not prove exact tuft positions.
The four requested original renderer captures are `target/alt-grass/progress/grass-outskirts-before.png`, `grass-outskirts-after.png`, `grass-castle-before.png` and `grass-castle-after.png`.
Reference PNGs were only read, and no external implementation source was copied.
Release tests pass 121 game unit tests with two existing ignores, six game integration tests, 25 extractor unit tests and two extractor integration tests, including the normal map data gate.
The game unit suite took 212.1 seconds in the concurrent shared tree, with existing installed lighting/story checks accounting for the long-running cases; the game integration suite took 5.7 seconds.
Workspace all-target release Clippy with warnings denied, workspace formatting and scoped whitespace checks pass.

## Layout schema (job B)

`layout.json` declares `layout_version: 2`; absence means the legacy unversioned format.
The additions are optional for old consumers, and the game already accesses the JSON dynamically, so no game change is needed.
The authoritative placement table is `parts`, in original MSB order, including MapPiece, Object, Enemy, Player, Collision, DummyObject, DummyEnemy, ConnectCollision and unknown types.
`part_index` is the zero-based full-table index; `kind`, type-local `id`, `name`, `model_index`, nullable `model`, `sib_path`, `entity_id` and eight `entity_group_ids` preserve identity.
Unknown kinds retain their numeric type as `Other(<number>)`.
`msb_models` preserves the original model table independently of successful GLB assets in `models`, and `part_counts` counts every canonical row by kind.

Each row retains Bevy `translation`, quaternion `rotation` in xyzw order, `scale` and `yaw_degrees`.
`source_transform` preserves the original left-handed position, Euler rotation in degrees and scale.
`lod_param_id` is the authored byte, including 255, with no default-row rewrite.
`is_shadow_only`, `entity_data`, nullable `gparam`, `scene_gparam`, `grass_config`, `enemy`, `collision` and `object` retain every setting supplied by the reader, including unknown fields.
`display_data` is null when absent; otherwise it contains `display_groups[8]`, `draw_groups[8]`, `collision_mask[32]`, `condition1` and `condition2`.
The separate nullable `display_group` contains signed `condition`, signed `disp_groups[8]`, `unk24` and `unk26`.
`masks` and unsigned `disp_groups` retain the reader's compatibility bit-pattern arrays.
No mask preprocessing, event-state evaluation, LOD selection or invented BackreadGroups alias is applied.

Object `object_references` preserves all three full-table indices and their nullable resolved names.
Enemy `enemy_collision` does the same for its collision-part reference.
The original signed indices remain in `object` and `enemy`; -1 is never converted into a valid index.
Collision `triangles` is null when no geometry was exported, or `[first_triangle, triangle_count]` in `collision.bin`.

`connections` contains every complete ConnectCollision row, also present in `parts`.
Its `connection` preserves signed `collision_index`, four raw `map_id` bytes, nullable `source_collision` with full-table index/name/model, `target_blocks` and `resolution`.
The collision index resolves within the collision-only list, not the full parts table.
It identifies the attached source collision, not a collision in the target block; MSBS encodes no target-collision index.
`resolution` is `resolved`, `unbound` for -1, or `invalid_collision_index`.
`map_inventory` records the sorted cached MSB inventory used to resolve targets.
`target_blocks` lists inventory matches under the executable matcher documented below: signed-negative area/third/fourth bytes bypass equality, and a signed-negative block selects block zero.
An all-255 no-op yields no targets.
An empty target list retains the authored connection and does not prove the target never exists.
These matches do not claim residency, active source occupancy or event enablement.

`render_status.asset` records `exported`, `physics_only`, `no_exported_primitives`, `missing_model_asset`, `model_export_failed`, `model_not_exported`, `no_model`, `connection_only`, `player_start`, `helper_dummy`, `character_pipeline` or `unsupported_part_kind`.
`render_status.reason` normally matches asset status; an exported MapPiece excluded from the compatibility view instead records `legacy_zero_draw_groups`.
`render_status.legacy_piece` records membership in the existing viewer's piece array.
An exported asset is eligible for drawing, not evidence that groups, story events or scene options enable it.
`model_status` records each attempted map/Object asset's outcome, and missing or physics-only assets retain their placed rows.
`no_exported_primitives` means the current geometry builder emitted nothing, including material/helper/LOD filtering; job G's material audit supplies finer diagnostics.
`map_offsets` preserves event identity, source byte offset, indices and the original position/Y rotation for ancillary-resource provenance.
It is not applied again to placed coordinates.

`pieces`, `objects`, `enemies`, `players` and `collisions` remain compatibility projections with their original field values and selection, plus optional missing metadata fields from the canonical rows.
Zero-group and missing-model pieces survive in `parts` while the old `pieces` projection retains the old draw behavior.
New consumers must use `parts` and its nested masks/settings rather than infer completeness or retail visibility from the compatibility arrays.
The old collision `display_groups` projection remains unchanged; the authoritative separate MSBS DispGroups is `display_group.disp_groups`.
The extractor integration module `crates/extract/tests/it/map_layout.rs` checks canonical identity, masks, conditions, LOD IDs, signed reference resolution and exported per-kind counts against job A's audit.

### Job B export verification (14 October 2026)

Both maps were re-exported with the release extractor built under `target/alt-mb`, with four Rayon workers.
The canonical export counts match the raw MSB audit in `target/alt-ma/msb-audit.txt` exactly:

| Kind | Outskirts raw/exported | Castle raw/exported |
|---|---:|---:|
| MapPiece | 4498 / 4498 | 3636 / 3636 |
| Object | 1509 / 1509 | 1674 / 1674 |
| Enemy | 144 / 144 | 284 / 284 |
| Player | 10 / 10 | 11 / 11 |
| Collision | 166 / 166 | 221 / 221 |
| DummyObject | 19 / 19 | 10 / 10 |
| DummyEnemy | 44 / 44 | 30 / 30 |
| ConnectCollision | 99 / 99 | 298 / 298 |
| Total | 6489 / 6489 | 6164 / 6164 |

Outskirts retains 98 resolved source collisions and the explicit unbound -1 row; all 298 Castle connections resolve.
The legacy piece arrays retain 4483 and 3623 rows, preserving viewer selection while all omitted rows are auditable in `parts`.
Exporter logs are `target/alt-mb/export-m11_00.txt` and `target/alt-mb/export-m11_01.txt`.
`cargo build --release --offline -p sekiro-extract -j 4`, `cargo test --release --offline -p sekiro-extract -j 4`, extractor all-target Clippy with warnings denied and `cargo fmt --all --check` pass.
All 20 tests pass, including the two integration tests; installed scalar comparisons require identical authored f32 bits after JSON parsing.
No game rendering or geometry-builder changes were made, and job G's material-audit module and call remain intact.

Research date: 14 October 2026.
Scope: `m11_00_00_00` is Ashina Outskirts, including the castle gate approach; `m11_01_00_00` is Ashina Castle.
This report records research and the implementation split, with completed jobs documented explicitly.
The [collaboration board](https://github.com/AKJama/sekiro-collab/blob/main/WORK.md) already lists this research task as claimed.

## Findings and evidence limits

The largest confirmed omissions are whole Object categories and connected map blocks, followed by incomplete visibility/state handling and deliberately omitted fog surfaces.
The cache already contains distant scenery and genuine FLVER detail levels, so inventing a terrain skirt or finding an imaginary Castle LOD map is not the first fix.
An additional concrete exporter issue is that eight Outskirts models contain 93 meshes available only at LOD1 or LOD2, which our LOD0-only export drops.
Those models occur in 154 MapPiece instances, although an absent lower-detail representation does not by itself prove that their LOD0 appearance has a hole.

Evidence is labelled as a **format fact**, **reference implementation**, **local measurement**, or **unresolved retail behavior** below.
Smithbox and DSMapStudio are map editors, not implementations of Sekiro's streaming scheduler or story simulation.
RustSekiro is an arena exporter with useful verified rendering corrections and explicit performance approximations.
None of these projects proves every retail rule, and copying their editor visibility or arena distance defaults would not make our maps exact.
In particular, the precise MSBS visibility-condition algebra, streaming lead/unload times and default LOD binding for ID 255 remain unresolved.

Source snapshots inspected: Smithbox `cbd477a8fd6d436b3e011c8548e1de8fd8876918`, DSMapStudio `97bc264cbc3e82a783eb017f57edf651f8ba000b`, and RustSekiro `2842cfb723f99a109fa1f0da644de03f8d7c6812`.
Repository trees and individual files were read through GitHub/web tools without cloning any repository.

## Rules supported by the reference projects

### 1. A model, a placed part and an enabled part are different things

**Format fact:** the MSB Model table names assets; its Parts table supplies instances, transforms, entity IDs, entity-group IDs and type-specific data.
MapPiece parts use map FLVER assets, Object parts use object assets, Collision parts supply collision, and ConnectCollision parts describe map-loading relationships.
An Object is an ordinary dynamic or interactive world element, not a synonym for an optional siege prop.
DummyObject is a distinct category described as normally absent, unused or cutscene-only in [SoulsFormats' MSBS PartsParam](https://github.com/vawser/Smithbox/blob/cbd477a8fd6d436b3e011c8548e1de8fd8876918/src/Andre/SoulsFormats/SoulsFormats/Formats/MSB/MSBS/PartsParam.cs).

**Reference implementation:** Smithbox loads all MSB parts and associates each with its correct model class, while RustSekiro's exporter includes both MapPiece and Object geometry.
Neither supports globally treating all Object parts as absent in the initial story state.
See [Smithbox MapContainer.LoadMSB](https://github.com/vawser/Smithbox/blob/cbd477a8fd6d436b3e011c8548e1de8fd8876918/src/Smithbox.Program/Editors/Map%20Editor/Data/MapContainer.cs) and [RustSekiro's map exporter](https://github.com/FFwLo/RustSekiro/blob/2842cfb723f99a109fa1f0da644de03f8d7c6812/tools/sekiro-extract/src/map.rs).

**Local gap:** `crates/game/src/map.rs:68`, `:800` and `:803` gate the entire Object category behind `scene.objects`.
`crates/game/src/play.rs:703` propagates the same option.
The exporter retains Object instance rows at `crates/extract/src/maps/mod.rs:421`, but a missing model entry is silently skipped by the viewer at `crates/game/src/map.rs:825`.
Ordinary props must be instantiated and then filtered by authored groups and event state.
Do not instantiate all DummyObjects as a substitute.

### 2. ConnectCollision references a collision list entry, not an arbitrary part index

**Format fact:** ConnectCollision stores a collision index and four map-ID bytes, with a separate display-group/condition structure.
The index resolves within `Parts.Collisions`, not the concatenated list of all part types.
Its documented purpose is to request another map while the player occupies the attached collision.
See [SoulsFormats ConnectCollision](https://github.com/soulsmods/DSMapStudio/blob/97bc264cbc3e82a783eb017f57edf651f8ba000b/src/Andre/SoulsFormats/SoulsFormats/Formats/MSB/MSBS/PartsParam.cs#L1853) and [Smithbox's Sekiro ConnectCollision metadata](https://github.com/vawser/Smithbox/blob/cbd477a8fd6d436b3e011c8548e1de8fd8876918/src/Smithbox.Data/Assets/MSB/SDT/Meta/Part/ConnectCollision.xml).

**Reference implementation:** DSMapStudio resolves trailing `0xFF` bytes as wildcard map fields when building cross-map relationships.
Sekiro can wildcard the block as well as the last two fields.
An all-wildcard target is a no-op, and wildcard matching excludes skybox suffix `_99` in that editor.
This describes candidate discovery, not proof that the retail game loads every candidate simultaneously.
See [SpecialMapConnections.GetRelatedMaps](https://github.com/soulsmods/DSMapStudio/blob/97bc264cbc3e82a783eb017f57edf651f8ba000b/src/StudioCore/MsbEditor/SpecialMapConnections.cs#L180).

**Local gap:** the reader already retains `(collision index, map bytes)` at `crates/formats/src/msb.rs:122` and `:382`, but extraction never serializes ConnectCollision rows in the layout assembled at `crates/extract/src/maps/mod.rs:414`.
It discards their conditions at `crates/formats/src/msb.rs:323`.
The runtime reads one layout and one collision mesh at `crates/game/src/map.rs:349` and has no connected-block asset lifecycle.
Loading a neighbor must preserve its own collision/group context and apply target-map masks rather than exposing every piece in that map.
Castle's 103 source collisions with multiple ConnectCollision entries prove that one collision must be able to request more than one neighbor.

**Unresolved retail behavior:** the sources do not specify prefetch distance, backread stages, unload hysteresis, how connection `Condition` interacts with masks, or the active-variant choice behind wildcard fields.
A bounded neighborhood preload is a reasonable interim implementation, but must be labelled an approximation and validated at both sides of each seam.
Do not recursively load the entire connectivity graph and call that retail streaming.

### 3. MapOffset is already baked into part coordinates

**Format fact:** MSBS MapOffset records a position and Y rotation, and SoulsFormats explicitly describes the shift as already accounted for in MSB coordinates.
Smithbox parents placed parts directly to the map root, while it uses its MapOffset node for ancillary resources such as BTL lights and navigation.
See [MSBS Event.MapOffset](https://github.com/vawser/Smithbox/blob/cbd477a8fd6d436b3e011c8548e1de8fd8876918/src/Andre/SoulsFormats/SoulsFormats/Formats/MSB/MSBS/EventParam.cs#L716) and [MapContainer](https://github.com/vawser/Smithbox/blob/cbd477a8fd6d436b3e011c8548e1de8fd8876918/src/Smithbox.Program/Editors/Map%20Editor/Data/MapContainer.cs#L113).

**Local measurement:** Outskirts has one MapOffset with position `(90, 3, -40)` and Y rotation `20`; Castle has none.
Do not apply that Outskirts shift again to its already placed GLBs when adding Castle.
Our `transform_json` at `crates/extract/src/maps/mod.rs:44` mirrors the instance matrix, which is consistent with retaining the authored coordinates.
The current MSB reader ignores MapOffset events at `crates/formats/src/msb.rs:213`, so ancillary-resource provenance cannot be represented through this reader.
A seam comparison should test existing coordinates before introducing any new root transform.

### 4. MSBS has three separate mask arrays and another DispGroups structure

**Format fact:** `DisplayDataStruct` in current SoulsFormats, formerly `UnkStruct1`, contains `DisplayGroups[8]`, `DrawGroups[8]`, `CollisionMask[32]`, then two condition bytes.
The 32-word collision mask is not another eight-word draw mask.
`DisplayGroupStruct`, formerly `UnkStruct2`, separately stores a condition integer, `DispGroups[8]` and two unknown shorts.
Collision and ConnectCollision carry that second structure.
SoulsFormats leaves several semantics unknown, and [Smithbox's Sekiro metadata](https://github.com/vawser/Smithbox/blob/cbd477a8fd6d436b3e011c8548e1de8fd8876918/src/Smithbox.Data/Assets/MSB/SDT/Meta/Part/Part.xml#L148) does not resolve those condition meanings.
The exact layout is in [MSBS PartsParam](https://github.com/vawser/Smithbox/blob/cbd477a8fd6d436b3e011c8548e1de8fd8876918/src/Andre/SoulsFormats/SoulsFormats/Formats/MSB/MSBS/PartsParam.cs#L636).

**Reference implementation:** Smithbox and DSMapStudio compare part draw masks with an active display mask by wordwise bit intersection.
Their DrawGroup implementation also considers null masks, explicit AlwaysVisible and all-zero part masks visible.
See [Smithbox DrawGroup.IsInDisplayGroup](https://github.com/vawser/Smithbox/blob/cbd477a8fd6d436b3e011c8548e1de8fd8876918/src/Smithbox.Program/Renderer/Utils/DrawGroup.cs#L19) and [DSMapStudio DrawGroup](https://github.com/soulsmods/DSMapStudio/blob/97bc264cbc3e82a783eb017f57edf651f8ba000b/src/StudioCore/Scene/DrawGroup.cs#L19).
Their editor display-group selection is not a reconstruction of Sekiro's per-frame player/camera visibility logic.

**Local gaps:** `crates/formats/src/msb.rs:112` misleadingly describes the entire 48-word structure as a collision mask.
`crates/formats/src/msb.rs:317` reads its arrays but omits both condition bytes, and `:323` drops the second structure's condition.
`crates/extract/src/maps/mod.rs:395` exports the first array as collision `display_groups`, not the separate `p.disp_groups` array that the reader retains.
`crates/game/src/map.rs:76` describes the chosen draw/draw or display/draw intersection as the game rule without source verification.
`crates/game/src/map.rs:1131` chooses just one nearest vertical collision and `:1162` retains previous groups when the chosen mask is zero or no surface is found.
Airborne, grappling, stacked-floor, seam and helper-collision behavior need a defined policy backed by evidence rather than this accidental retention.

**Reference disagreement:** `crates/extract/src/maps/mod.rs:134` and `:355` permanently drop zero-draw-group pieces on maps with collision, contrary to both editors' all-zero-visible policy.
Runtime `crates/game/src/map.rs:1201` also hides all-zero masks once an active nonzero mask exists.
Retain these parts and their conditions in the cache so the future runtime can decide correctly.
Do not infer either that all such parts never render or that every such part always renders in the retail game.

**BackreadGroups caution:** neither inspected MSBS PartsParam exposes a field literally named `BackreadGroups`.
Other games' similarly named arrays are not enough evidence to rename Sekiro's 32-word CollisionMask or its DispGroups.
ObjectBase has references to other parts/collisions that SoulsFormats tentatively associates with loading and grappling, which our reader omits by not handling Object type data at `crates/formats/src/msb.rs:338`.
Preserve those references alongside masks, and keep residency/backread separate from render visibility.
See [MSBS ObjectBase](https://github.com/vawser/Smithbox/blob/cbd477a8fd6d436b3e011c8548e1de8fd8876918/src/Andre/SoulsFormats/SoulsFormats/Formats/MSB/MSBS/PartsParam.cs#L1044).

### 5. Distant models, mesh LOD and sky assets are separate systems

**Reference implementation:** RustSekiro's `MAP_FAR` adds existing distant-view `m6*` models and sky/cloud `m9*` pieces beyond its arena radius, using low mesh detail.
Its `MAP_LOD` defaults of 30/55 meters are exporter performance choices, not values to copy as Sekiro's universal thresholds.
It does not implement a general connected-map streaming system.
See [RustSekiro exporter](https://github.com/FFwLo/RustSekiro/blob/2842cfb723f99a109fa1f0da644de03f8d7c6812/tools/sekiro-extract/src/map.rs#L896) and [its map research notes](https://github.com/FFwLo/RustSekiro/blob/2842cfb723f99a109fa1f0da644de03f8d7c6812/docs/kb/map.md#L237).

**Format fact:** FLVER face flags `0`, `0x01000000` and `0x02000000` distinguish ordinary, LOD1 and LOD2 index sets.
`0x80000000` is the motion-blur flag, not a fourth ordinary visible copy.
LOD sets may share vertex buffers or exist on separate meshes, so selecting only an alternate index buffer on an already exported LOD0 mesh is insufficient.
See [SoulsFormats FaceSet](https://github.com/soulsmods/DSMapStudio/blob/97bc264cbc3e82a783eb017f57edf651f8ba000b/src/Andre/SoulsFormats/SoulsFormats/Formats/FLVER/FLVER2/FaceSet.cs#L18) and [Smithbox FLVER resource loading](https://github.com/vawser/Smithbox/blob/cbd477a8fd6d436b3e011c8548e1de8fd8876918/src/Smithbox.Program/Renderer/Resource%20Types/FlverResource.cs#L1557).

**Data-defined LOD:** Sekiro's LodParam uses `LOD_BANK_ST`; its fields specify level boundaries, hysteresis distances, draw/shadow/motion-blur limits, texture LOD and crossfade settings.
The Paramdef says a negative 0/1 boundary prevents switching to LOD1/2, a negative 1/2 boundary prevents LOD2, and zero crossfade time uses distance rather than time.
See [Sekiro LodBank Paramdef](https://github.com/vawser/Smithbox/blob/cbd477a8fd6d436b3e011c8548e1de8fd8876918/src/Smithbox.Data/Assets/PARAM/SDT/Defs/LodBank.xml).
The installed row 0 uses 30/60 meters with play distances 2/4 and a 0.2-second crossfade.
Row 60 uses 80/2000 meters, disables texture LOD and disables crossfade; row 61 has both boundaries -1, texture LOD off and crossfade off.
Installed Windows Low/Middle/High LodPlatform rows 4/5/6 all have distance scales 1.0.
Most placed parts use ID 255, which is not a row in the 64-row LodParam table, so its fallback binding must still be established.

**Local gaps:** the reader keeps `lod_param_id` at `crates/formats/src/msb.rs:116` and `:330`, but extraction omits it from instance rows at `crates/extract/src/maps/mod.rs:366` and `:375`.
`crates/formats/src/flver.rs:124` requires flags exactly zero and `crates/extract/src/maps/pieces.rs:259` skips any mesh without that set.
No runtime mesh LOD exists in `crates/game/src/map.rs:855`, which always spawns GLB scene 0.
Arena cropping uses a fixed visual radius at `crates/game/src/map.rs:830`, and `FarScenery` at `:848` bypasses groups based on being outside the arena collision radius rather than an authored distant-model classification.
That bypass can expose incompatible scenery; the regular full-map view does not have that specific bypass because `within` is None.
The normal `--map` viewer also has no distance-LOD switch, so its pop-in must not automatically be attributed to mesh LOD transitions.

### 6. Events change group membership state, geometry and animation

**Instruction facts:** `2005[3]` activates/deactivates Objects and `2012[1]` activates/deactivates map parts.
`2003[6]` changes map collision and `2003[7]` changes map visibility; `2004[29]` is character backread state, a separate operation.
Object destruction/restoration and animation commands also affect what the player sees.
The instruction table includes ceremony operations, but no `2003[57/59]` or `2005[17]` calls were found in the two base map scripts or their common/common_func scripts in this audit.
See [DarkScript3's Sekiro instruction table](https://github.com/AinTunez/DarkScript3/blob/master/DarkScript3/Resources/sekiro-common.emedf.json) and [readable EMEDF](https://soulsmods.github.io/emedf/sekiro-emedf.html).

An event's target can be an entity-group ID affecting many parts, not just a single part's EntityID.
Initialization calls must substitute event parameters, including calls into common_func, before interpreting branch conditions.
Extracting a list of all instructions or taking the last textual enable is not equivalent to executing the script.
The first visit depends on story flags, NPC quests, collected items, doors and the player's current location.
The table below gives confirmed branches and initial conditions rather than claiming one universal final state for all interactive objects.

**Local gaps:** layout rows omit entity-group IDs at `crates/extract/src/maps/mod.rs:375`, and ordinary piece rows also omit entity IDs at `:366`.
Arena mode reparses the raw MSB to recover IDs/groups at `crates/game/src/arena_b.rs:34`, but ordinary exploration has no equivalent general world event host.
The existing scene interpreter already handles Object/map-part enable commands at `crates/sim/src/arenas/scene.rs:80`.
Its results must be connected to exploration and neighboring blocks, rather than implemented as another set of hardcoded map hides.
`crates/game/src/arena_b.rs:61` and `crates/game/src/map.rs:1186` separately write Visibility; event enabling can override group hiding, so the eventual state must combine residency, event enable, draw masks, LOD and frustum decisions in one place.
The existing `hidden_models` workaround at `crates/sim/src/arena.rs:225` and `:238` hides `m015072` for two arenas but does not provide first-visit map-state execution.

### 7. Texture loss, helper geometry and horizon effects need separate diagnosis

**Reference implementation:** RustSekiro's notes establish that a missing floor or a visible far shell can arise from wrong winding, not necessarily groups or streaming.
They also distinguish fog-wall proxy Objects from their FXR appearance and avoid treating their proxy meshes as ordinary visible surfaces.
Its exporter skips some ObjAct doors because it does not execute their opening animations, which is an acknowledged arena limitation and should not become our general map rule.
See [RustSekiro's winding and LOD investigation](https://github.com/FFwLo/RustSekiro/blob/2842cfb723f99a109fa1f0da644de03f8d7c6812/docs/kb/map.md#L310) and [object handling](https://github.com/FFwLo/RustSekiro/blob/2842cfb723f99a109fa1f0da644de03f8d7c6812/tools/sekiro-extract/src/map.rs#L959).

Our map exporter mirrors X and retains the map's triangle order at `crates/extract/src/maps/pieces.rs:271` and `:448`, consistent with that correction.
It retains face-set culling at `:402`, so there is no demonstrated universal winding bug to fix from this research.
It does flatten every map/Object model into static mesh geometry without applying mesh-owner node transforms at `:259` and `:456`.
RustSekiro enables reference-pose handling for Object meshes in its exporter, so our objects require an ownership/animation audit before declaring their placement exact.

Our `skip_material` at `crates/extract/src/maps/pieces.rs:226` deliberately omits ground/cloud fog as well as invisible/dummy/shadow materials.
White space below cliffs can therefore remain even after terrain and neighbor geometry is present.
This is an effects/material gap, not permission to draw every fog proxy with a standard opaque shader.
Grey blocks likewise require identifying their model, MTD and event/group state before deciding whether they are an intended distant shell, a proxy, or a texture fallback.

Texture indexing at `crates/extract/src/maps/textures.rs:33` already scans all area texture binders plus shared maptex, so simply loading a neighbor does not automatically recover the reported missing textures.
The resolver takes the first texture on a failed name match at `:117`, and first-source wins on duplicate names at `:68` and `:85`.
Missing requested textures are counted transiently at `crates/extract/src/maps/mod.rs:266`, but the GLB omits their references at `:286`.
The reported historical 1-14 missing textures per map cannot be reconstructed exactly from that lossy layout/GLB output alone without re-running texture demand resolution.
This audit did not regenerate either cache export and does not claim fresh missing-texture totals.

## Local data checks

Counts below come from direct reads of the installed-cache MSBS tables and existing layout.json files, not from a screenshot or from assumptions about part names.
ConnectCollision indices were resolved against the Collision-only list.
Entity groups were read at entity-data offset `0x1C`, consistent with the existing Rust reader; masks and LOD bytes were checked independently.

| Measurement | Outskirts m11_00 | Castle m11_01 |
|---|---:|---:|
| MapPiece parts | 4,498 | 3,636 |
| Unique placed MapPiece models | 535 | 569 |
| Object parts | 1,509 | 1,674 |
| DummyObject parts | 19 | 10 |
| Enemy / DummyEnemy parts | 144 / 44 | 284 / 30 |
| Player parts | 10 | 11 |
| Collision parts | 166 | 221 |
| ConnectCollision parts | 99 | 298 |
| Exported layout pieces | 4,483 | 3,623 |
| Exported layout objects | 1,509 | 1,674 |
| Exported models of all render categories | 712 | 767 |
| Ordinary Object instances without a model GLB entry | 56 | 61 |
| Zero-draw-mask MapPiece parts | 11 | 12 |
| Collisions with zero DisplayGroups / DrawGroups | 49 / 14 | 19 / 19 |
| Collisions with zero separate DispGroups | 163 | 221 |
| ConnectCollisions with nonzero separate DispGroups | 72 | 0 |
| Source collisions with multiple connections | 2 | 103 |
| Placed m6* distant pieces / m9* pieces | 22 / 52 | 20 / 19 |
| Pieces with LodParamID 255 / 60 / 61 | 4,436 / 2 / 60 | 3,625 / 0 / 11 |
| Objects with LodParamID 255 | 1,509 | 1,674 |
| MapOffset events | 1 | 0 |

Object rows without GLBs are not automatically broken scenery.
They include known helper/physics/proxy categories such as `o000101` and `o0025*`, but the other omitted models require individual classification.
Outskirts has seven instances of absent model `o110000`; Castle has nineteen of `o111690` and three of `o111751`.
Our code already recognizes valid physics-only binders at `crates/extract/src/maps/mod.rs:291`, so those must not be counted as missing visible surfaces without checking the binder and material.

### Connectivity and block inventory

| Source | Raw target bytes expressed as a pattern | Connection rows | Available matching base map |
|---|---|---:|---|
| m11_00 | `m11_01_XX_XX` | 97 | m11_01_00_00 |
| m11_00 | `m20_00_XX_XX` | 2 | m20_00_00_00 |
| m11_01 | `m11_00_XX_XX` | 177 | m11_00_00_00 |
| m11_01 | `m11_02_XX_XX` | 86 | m11_02_00_00 |
| m11_01 | `m13_XX_XX_XX` | 8 | m13_00_00_00 |
| m11_01 | `m13_00_XX_XX` | 2 | m13_00_00_00 |
| m11_01 | `m17_00_XX_XX` | 25 | m17_00_00_00 |

The ten Dungeon references comprise two different wildcard forms, not ten different neighboring blocks.
The Outskirts connection named `h004490_1000` has CollisionIndex `-1`, meaning no attached collision.
The initial audit incorrectly used Python's negative indexing to report the last collision, `h900900`, as its target; Job A's raw-table verification corrects that example.
Resolve non-sentinel indices within the collision-only list, never by the connect part's name.
The `m11_00` and `m11_01` split is a real bidirectional block boundary, not an alternate story version or an LOD pair.

The cached mapstudio directory contains base maps `m10_00`, `m11_00`, `m11_01`, `m11_02`, `m13_00`, `m15_00`, `m17_00`, `m20_00`, `m25_00` and shared sky `m89_00_00_99`.
No separate `m11_00` or `m11_01` distant/LOD suffix block exists in `cache/raw/map` or that directory.
The cached worldmsblist lists those three m11 base maps separately and the m89 sky map separately.
It lists additional maps not extracted here, including `m17_99_00_00`, but that file's name alone does not establish its rendering purpose or make it an Ashina LOD dependency.
Do not confuse cinematic/rematch `.emevd` names with map blocks actually present in the cache.
The `map/m11` directory contains area texture resources, while the full m11 IDs contain map geometry resources.

### Actual FLVER detail inventory

Only unsuffixed FLVER files referenced by placed MapPiece models were counted in this table.
This avoids double-counting companion `_S.flver` files and unplaced assets.

| Measurement | m11_00 | m11_01 |
|---|---:|---:|
| Referenced map FLVERs successfully read | 535 | 569 |
| Meshes | 3,029 | 3,390 |
| Flag-0 face sets | 2,936 | 3,390 |
| LOD1 face sets | 2,610 | 3,122 |
| LOD2 face sets | 2,541 | 3,036 |
| Meshes with no flag-0 face set but with LOD sets | 93 | 0 |

The eight Outskirts models with such meshes are `m004100`, `m004110`, `m004140`, `m004200`, `m004210`, `m004300`, `m004410` and `m004420`.
Their missing representations affect 154 placed instances of those models.
These are whole LOD-specific meshes, so exporting only flag-0 meshes loses data that cannot be restored by runtime distance selection.
Conversely, drawing all 93 meshes alongside LOD0 could overlap representations and is not the proposed fix.

### Pieces permanently absent from layout

Outskirts' eleven zero-draw-mask parts are `m003615_0000`, `m004800_8000`, `m004800_8006`, `m004800_8008`, `m004800_8009`, `m902000_9000`, `m902000_9019`, `m902000_9049`, `m911030_9000`, `m911039_9000` and `m911065_9000`.
Four additional raw parts absent from layout are `m900100_9000`, `m900101_9000`, `m911009_9000` and `m925009_9000`.
The latter four lack exported model entries; they were not removed by the zero-group filter.

Castle's twelve zero-draw-mask parts are `m003811_2000`, `m004300_8008`, `m022000_2000`, `m022010_2000`, `m400091_7000`, `m803001_9000`, `m911000_9000`, `m911041_9000`, `m911061_9000`, `m911062_9000`, `m911065_9000` and `m911139_9000`.
The additional absent part is `m920009_9000`, whose model has no exported entry.
At least six of Castle's deleted parts belong to `m9*` models, so the deletion affects horizon/sky-related content as well as local pieces.
These numbers prove data loss, not that every deleted part belongs in every story state.

## Object state on the first visit

For the following checks, first visit means before later Castle invasion/endgame states, with `8301`, `8302` and `8306` off, and with relevant one-time object/quest flags unset unless stated.
Outskirts before Gyoubu and Castle before Genichiro are different progress checkpoints, so their boss-idol flags are described separately.
Location-driven encounters, NPC quests and player interactions can legitimately change the following defaults.

Direct map instructions plus parameter-resolved InitializeEvent/InitializeCommonEvent calls address visibility for 477 of Outskirts' 1,509 ordinary Objects and 243 of Castle's 1,674 ordinary Objects.
The remaining 1,032 / 1,431 ordinary Objects are not addressed by the audited visibility commands, which is strong evidence against omitting their entire category.
Of all ordinary Objects, 961 / 1,397 have neither a positive entity ID nor a positive entity-group ID.
Those counts are a static reference analysis, not a count of final on-screen objects or a complete event replay.
They exclude unrelated instructions such as destruction, animation and treasure state, and do not claim that every referenced branch runs at startup.

### Story-controlled scenery groups

| Map / target | Members in the MSB | Confirmed first-visit branch | Evidence in the installed scripts |
|---|---|---|---|
| m11_00 group 1106600 | 182 ordinary Objects | Enabled with 8302 off | Event 50 calls common 20004112 with the disable-when-8302 condition |
| m11_00 group 1106610 | 215 Objects, 3 DummyObjects, 2 DummyEnemies | Disabled with 8302 off | Event 50 calls 20004112 with the disable-when-not-8302 condition |
| m11_00 group 1106640 | 13 parts | Disabled after initialization settles | Event 0 passes both OFF and ON tests for 8310 to 20004112 |
| m11_00 groups 1107600 / 1107610 | 115 / 36 parts | Enabled / disabled | Parallel common 20004110 map-part calls keyed to 8302 |
| m11_00 group 1106200 | 28 Objects and 1 DummyObject | Disabled without Wolf SpEffect 4100 | Common 20005560 toggles on that SpEffect, not on draw groups |
| m11_01 group 1116200 | 18 Objects and 7 DummyObjects | Enabled | Common 20004112 disables when one of 8306/8301/8302 is ON |
| m11_01 group 1116201 | 64 Objects | Enabled | Common 20004112 disables when 8306 or 8302 is ON |
| m11_01 group 1116203 | 56 Objects | Disabled | Common 20004112 matches 8301 OFF |
| m11_01 group 1116204 | 66 Objects | Disabled | Common 20004112 matches 8302 OFF |
| m11_01 groups 1117201 / 1117203 / 1117204 | 36 / 7 / 33 parts | Enabled / disabled / disabled | Corresponding 20004110 map-part conditions |
| m11_01 group 1117210 | 1 part | Disabled after initialization settles | Both OFF and ON cases for 8306 are passed to 20004110 |
| m11_01 group 1117550 | Normal floor m015070_4000 | Enabled | Event 11115943, first-visit path |
| m11_01 group 1117551 | Burned floor m015072_4000 | Disabled | Event 11115943, until later cutscene flag 11115921 with 8301 ON |
| m11_01 groups 1116650 / 1116651 | 17 / 9 Objects | Disabled | Events 11110650 / 11110651 wait for 8301 / 8302 |

Calls also mention groups 1116202 and 1117200, but the audited base MSB has no members for those groups.
Job C corrects the original table's inverted common-helper interpretation: instruction 9 skips the enable command when the OR group passes.
The actual resolved bytes for 1000[1] are `[1, 1, 255, 0]`, meaning skip one line on PASS of group -1.
An event command without matching members is not proof of missing geometry.
The scene must support real group targets; for example, 1117550/1117551 are not individual EntityIDs.

### Interactive and transient objects

| Target(s) | State and trigger confirmed from the installed EMEVD |
|---|---|
| Outskirts 1101959, model o005000 | Event 50 disables it unconditionally |
| Outskirts 1101955, boss idol | Common 20005500 disables it while Gyoubu flag 9301 is off, then enables it after that flag; the later Demon event has another override |
| Outskirts 1101310, model o113415 | Event 11105310 restores the object for the initial encounter; its late-state/defeat branch disables it |
| Outskirts fog-wall targets 1101290/1101291/1101800/1101801/1101802 | Common 20005820 starts disabled and enables only under the passed live-boss/encounter flags; their visible fog is SFX rather than ordinary opaque proxy geometry |
| Castle 1111952, boss idol | Common 20005500 waits for flag 9403; later local events can override it |
| Castle 1111440, model o001402 | Event 11115205 disables it initially and enables it only after the authored character condition; it is also controlled by 11115200 |
| Castle 1111420/1111421/1111425/1111430/1111435/1111436/1111437 | Events 11115420-11115424 initialize these proxies disabled, then toggle them based on generator/skit-area conditions |
| Castle 1111311 and 1111958 | Events 11115310 and the initial 11115360 branch disable them |
| Castle 1111713/1111714/1111715, model o001400 | Event 11115620 leaves them present while corresponding 11110553/54/55 interaction flags are unset; they are disabled after interaction, while the associated door animation changes |
| Castle 1111610, model o111400 | Event 11115615 restores animation 1 and briefly toggles it off/on after waiting for object backread; it is enabled once that initial restore settles |
| Castle group 1116500 | 11 Objects and 6 DummyObjects; boss phase events control it, so it is not ordinary always-on scenery |
| Castle 1111511/1111731 and groups 1117740/1117741 | Event 11115760 disables all initially, then selects a branch using quest flags 1280-1283 and later-story flag 8301 |
| Castle 1111721/1111723 and group 1117700 | Event 11115763 disables initially, then enables when flag 1661, 8301 or 8302 becomes true |
| Castle 1111730 | Event 11115773 initially disables it, then enables when flag 1380 is OFF; common initialization raises 1380 from the all-off input fixture, so this settled fixture leaves it disabled |

Ordinary scenery should start enabled subject to loading/render gates, with the actual constructor/preconstructor/common events applying the exceptions above.
Do not flatten the temporary off/on restoration commands into permanent exclusions.
Job C supplies a per-part startup snapshot with documented progress, quest, interaction, region, inventory and SpEffect inputs below.
Live character, generator and skit-area observations still need the corresponding world systems; unimplemented observations remain explicit blockers in the snapshot diagnostics.
The existing lighting profile's empty morning flag list is a lighting preset, not that complete fixture.

## Gaps ranked by visible impact

Ranking is an evidence-based triage estimate, not a measured percentage of bad pixels.

| Rank | Gap and code location | Visible symptoms explained | Confidence / scope |
|---|---|---|---|
| 1 | Whole Object category absent, map.rs:68 and :803 | Missing walls, fences, trees, doors and other props throughout both maps | Confirmed structural omission; 1,509 / 1,674 instances, with story exceptions required |
| 2 | Connections lost, extract/maps/mod.rs:414; one layout only, map.rs:349 | Empty seams, absent neighboring terrain, disconnected or apparently floating distant structures | Confirmed 99 / 298 connection rows; exact visual contribution needs a seam walk |
| 3 | Authored group/condition state reduced to one collision and one mask, msb.rs:317; map.rs:1131/:1201 | Pop-in, coarse shells at the wrong location, missing detailed pieces, incompatible variants | Confirmed approximation; retail condition algebra unresolved |
| 4 | No exploration event state, extract/maps/mod.rs:366/:375; arena_b.rs:61 | Missing initial props, later siege scenery, doubled/burned floors, wrong doors and boss proxies | Confirmed many group switches; enabling every Object would expose wrong story states |
| 5 | Fog surfaces discarded, extract/maps/pieces.rs:226/:233 | White void below cliffs and missing cloud/ground-fog coverage | Confirmed asset omission; coordinate with existing sky/fog job |
| 6 | Zero-group pieces deleted, extract/maps/mod.rs:134/:355 | Specific local/horizon omissions persisting even with groups off | Confirmed 11 / 12 raw parts; per-part visibility still needs verification |
| 7 | LOD0-only export, formats/flver.rs:124; extract/maps/pieces.rs:259 | Missing alternative representations and excessive detail cost; possible distance-dependent silhouette errors | Confirmed 93 lower-detail-only Outskirts meshes; Castle has none in its placed map FLVERs |
| 8 | Unclassified missing model/material/texture fallbacks, extract/maps/mod.rs:291/:308; textures.rs:113 | Grey blocks or wrong surfaces | Confirmed 56 / 61 Object instances without GLBs; some are intentional physics-only helpers |
| 9 | Static Object export ignores model node pose, extract/maps/pieces.rs:259/:456 | Misplaced or closed interactive objects once Object rendering is enabled | Implementation gap; contribution not measured |

Increasing view distance alone cannot recover unexported parts, missing objects or authored fog materials.
Drawing all distant models all the time can also create overlaps rather than solve missing terrain.

## Separate fix jobs and file ownership

These are proposed jobs, not changes made by this research session.
Shared bootstrap wiring should be serialized through the world-loader owner so concurrent jobs do not compete for map.rs.
Do not edit the existing sky/fog work or automated map-sweep work as part of these jobs without coordinating their owners.

| Job | Owned implementation files | Concrete deliverable and acceptance check |
|---|---|---|
| A. Preserve complete MSBS visibility/loading metadata | crates/formats/src/msb.rs; crates/formats/tests/it/msb_completeness.rs; its tests/it/main.rs mod line | Named mask structures, all conditions, Object part/collision references, precise connection index semantics and MapOffset provenance; verify both raw counts and references |
| B. Make map layout lossless and auditable | crates/extract/src/maps/mod.rs; new crates/extract/src/maps/layout.rs; extractor integration test module | Keep every placed part row with entity/group/LOD data, connections and an explicit render-status reason; stop deleting zero masks; reproduce raw counts without altering assets |
| C. Execute story state for exploration | new crates/sim/src/world_events.rs; crates/sim/src/arenas/scene.rs; new crates/game/src/map_story.rs; focused sim integration module | Run constructors/common calls with substitutions and a defined first-visit fixture; emit state by entity/group; prove normal/burned floor and invasion group selections |
| D. Load the connected map neighborhood | crates/game/src/map.rs bootstrap; new crates/game/src/map_world.rs; crates/game/src/play.rs integration | Wildcard candidate resolver, multi-neighbor requests, separate per-block collision/group state and asset readiness; walk Outskirts-Castle, Castle-Reservoir and Castle-Dungeon without missing geometry or a double MapOffset |
| E. Use one visibility decision per part | new crates/game/src/map_visibility.rs; crates/game/src/arena_b.rs visibility integration | Combine authored event state, residency and visibility without competing writes; zero-group diagnostics, airborne/helper/seam policies; expose the chosen collision and source masks in diagnostics |
| F. Export and select real detail representations | crates/formats/src/flver.rs; crates/extract/src/maps/pieces.rs; new crates/game/src/map_lod.rs; focused exporter test module | Export separate LOD-only meshes, preserve face flags and part IDs, derive thresholds/hysteresis/crossfade from params; validate all eight affected Outskirts models and establish ID-255 binding before claiming retail parity |
| G. Resolve material and model provenance | crates/extract/src/maps/textures.rs; new crates/extract/src/maps/material_audit.rs; crates/game/src/map_material.rs diagnostics | Record every missing texture and provenance, eliminate silent wrong-first-texture fallbacks, classify omitted render models and grey proxies; hand fog cards to the already claimed fog job |
| H. Pose and animate world Objects faithfully | new crates/extract/src/maps/objects.rs; new crates/game/src/map_objects.rs; focused Object-pose integration module | Preserve rigid mesh-owner transforms and authored initial object animations/destruction; prove castle door and gate geometry against their bind/reference pose and EMEVD state |

**Job D done - connected loader/bootstrap (14 October 2026), with owning-block material routing integrated on 15 October 2026.**
The routing follow-up adds per-block material/mesh audits and GPU-ready capture checks; its controls and seven reviewed fixture cells are documented in [the sweep review](MAP-COMPLETENESS-SWEEP.md#owning-block-texture-routing-review-2026-10-15).
Outskirts/Castle exploration uses a bounded six-block warm cache, retail signed wildcard matching, independent collision and normal/additional masks, multi-target unions and requested/ready/retiring/dormant states, without another MapOffset transform.
Canonical MapPiece/Object roots combine C's first-visit snapshots, E's single visibility writer and F's per-instance detail selector; ordinary Objects are enabled by default and declared helpers/physics-only assets are skipped.
Other entry maps and arena sessions retain their previous bootstrap, and Explore's broad-phase collision contains the same bounded neighbourhood.
Installed-data tests prove all five directed seam demands, and a synthetic grounded crossing checks union at the seam and removal of stale target masks.
The connected workload requires a 64 MiB IO stack, and fixed-camera captures now wait for rendered settling frames and a stable empty pipeline queue after parts become drawable.

The two-view Castle control uses the same executable/cache/poses, spacing 80 m, one yaw, 128 by 72 pixels, 12 settling frames and `--objects --no-layers --no-shadow --no-giiv`; only the before side adds `--no-connected-blocks`.
Depth-gap candidates decrease from 8,360 to 809 and void candidates from 4,608 to zero; grey candidates increase from 30 to 353 and normal/all differences from 263 to 3,260.
This is an incomplete two-of-43-view geometry/state comparison of the integrated C/D/E/F work, not evidence of complete visual parity or elimination of pop-in.
Reports are preserved in `target/alt-md/sweep-before-castle/` and `target/alt-md/sweep-after-castle/`, and the normal Castle room capture is `target/alt-md/castle-start2-normal.png`.
The earlier four-view Castle control fails the sweep's CPU/GPU collision-depth validation, so its totals are excluded from this comparison.

The matched Outskirts two-of-79-view control samples positions below the visible landscape and worsens: depth-gap candidates rise from 14,659 to 18,018, void from zero to 9,216 and normal/all differences from zero to 12,926, while grey candidates fall from 185 to zero.
Neither camera reachability nor full-area coverage is established, and these results leave visual acceptance open rather than establishing that Outskirts is complete.
Both comparisons assert identical pose/settings fields in `target/alt-md/sweep-comparison.json`, with the individual Outskirts reports under `target/alt-md/sweep-before-outskirts/` and `target/alt-md/sweep-after-outskirts/`.
They use the same cached exports and renderer controls; enabling the connected loader also enables the integrated story, mask and detail decisions, so this comparison does not isolate streaming alone.

Final game release tests pass 100 unit tests and six integration tests, with two existing ignores; the installed-neighbour test includes all five native directed requests and the synthetic crossing.
Game all-target release Clippy with warnings denied and workspace formatting pass using `target/alt-md`.
Logs are `verified-tests.txt`, `verified-clippy.txt`, `verified-fmt.txt` and `native-neighbour-tests.txt` under that build directory.
Cargo build and final checks use offline mode and four jobs.

Occupancy uses the nearest support under player/camera positions, with a 0.5 m tie band for overlapping seam surfaces and an above-surface fallback.
That policy, two update stages of retirement with retained warm assets, unknown external/non-core request gates and unit extra LOD scales remain explicit approximations.
F's weights are retained while the renderer submits the dominant representation; a retail dual-submission coverage shader, live world predicates/animations and the material-owner routing call remain open.

### Fog surface inventory (14 October 2026)

**Fog surfaces done - export and traced surface rendering.**
Both maps were re-exported with `target/alt-fs/release/sekiro-extract.exe`, retaining the existing detail-level work.
Outskirts now exports 716 model GLBs and 4,487 legacy pieces; Castle exports 768 GLBs and 3,624 pieces.
The canonical 6,489/6,164 parts and 99/298 connections are unchanged.
The existing layout fixture was updated for the four restored Outskirts assets and one restored Castle asset.

The two maps use exactly two landscape fog MTDs: `M[m11_00]_GroundFog.mtd` and `M[m11_00]_LandScape_CloudFog1.mtd`.
Their shader families are `M[LandScape]_GroundFog` and `M[LandScape]_CloudFog`, respectively.
Neither map contains the separate `M[Sky]_Cloud` or library CloudSea material.
The complete per-material inventory, every MTD parameter and texture slot, and placed part transforms/masks are in [fog-inventory.json](../target/alt-fs/fog-inventory.json).
The inventory reads the native MTDs and existing material audit rather than inferring a family from the chosen diffuse texture.

| Map | Model and placed part | Material names | MTD family | Bevy part translation |
|---|---|---|---|---|
| Outskirts | m900100 / m900100_9000 | 02 - Default | GroundFog | (-90, 3, -40) |
| Outskirts | m900101 / m900101_9000 | 02 - Default | GroundFog | (-90, 3, -40) |
| Outskirts | m911008 / m911008_9000 | 雲_2, Material #94, Material #95 | CloudFog | (-90, 3, -40) |
| Outskirts | m911009 / m911009_9000 | 雲_2, Material #94, Material #95 | CloudFog | (-90, 3, -40) |
| Outskirts | m925009 / m925009_9000 | Material #10, #9, #3, #1, 雲_2, Material #5, #7, #12, #14 | CloudFog | (-4101.7295, -127, 2172.7058) |
| Castle | m920009 / m920009_9000 | 雲_2, Material #94, Material #95 | CloudFog | (0, 0, 0) |

GroundFog binds mask slot 0 `m99_lowcloud_3m`, flow slot 2 `m99_galaxy_a`, albedo slot 3 `m99_ground_00_a` and height slot 4 `m99_ground_00_1m`.
The installed static switch is 1, which selects masked colour 3 instead of the slot-3 albedo sample.
CloudFog binds mask slot 1 `m99_lowcloud_3m` and mask slot 7 `m99_cloudfog_3m`.
All six texture-slot references resolve to the shared map texture bank, and all authored slots use UV channel 0.
Both families use blend mode 2 with zero alpha reference.

The local shader inventory and `GroundFog_0_Fwd`, `GroundFog_1_Fwd`, `CloudFog_Fwd` pixel disassemblies establish their scrolling samples, channel selection, mask dots, diffuse/grazing lighting and linear-view-depth fade.
Matching vertex disassembly forwards TEXCOORD1.x as coverage and COLOR1.x as occlusion, rather than treating RGB vertex data as a surface tint.
CloudFog's soft-intersection multiplier is float 0, installed as 0.1; GroundFog uses its angle-dependent density squared.
The exporter retains those streams, UVs, normals/tangents, every resolved sampler and all MTD constants in `sekiro_fog_surface` extras.
The new material uses the existing map irradiance atlas, the default environment probe outside volumes and each GLB's own asset-source path, including connected blocks.
RustSekiro's current map exporter still filters every fog MTD, so no surface implementation was copied.

Native Fwd shadow permutations, instance colour-slot overrides and forward volumetric-fog composition remain fidelity work.
The material shares the renderer's existing irradiance selection, environment-probe approximation of native instance SH and hemisphere fallback.
The paused volumetric fog files and the other jobs' visibility, LOD and residency policies are unchanged.

### Fog surface verification

`CARGO_TARGET_DIR=target/alt-fs cargo build --release --offline -p sekiro-game -p sekiro-extract -j 4` passes.
The same target directory and four build jobs were used for `cargo test --release --offline -p sekiro-extract -p sekiro-game`, all-target release Clippy with warnings denied and `cargo fmt --all --check`.
The final run passes 131 tests with two existing ignores, including all six native fog models, canonical layout preservation, exported tangents and the fog shader's single-sample/MSAA soft-depth compositions.
An initial shader vector-conversion error and an overly broad mixed-material test assertion were corrected before this final run.

Matched Outskirts captures are [before](../target/alt-fs/before-outskirts-surface-diagnostic.png) and [after](../target/alt-fs/after-outskirts-surface-diagnostic.png), at `-145,-26,60,240,-40` near its early cliff path.
These use `--no-connected-blocks --no-env-probe --no-fog`; the baseline additionally uses `--no-fog-surfaces`.
The regular global-fog controls are [before](../target/alt-fs/before-outskirts-cliffs.png) and [after](../target/alt-fs/after-outskirts-cliffs.png); the current global pass obscures most surface differences at this pose.
The surface diagnostic isolates the recovered card coverage without changing the volumetric fog implementation.
Primary Castle roof-edge controls are [before](../target/alt-fs/before-castle-edge-final.png) and [after](../target/alt-fs/after-castle-edge-final.png), at `145,58,265,225,-30` with the same primary-only diagnostic flags.
The Castle-only CloudFog asset supplies distant cloud bands; the broad low sea belongs to the neighboring Outskirts GroundFog meshes.
The connected Castle comparison is [before](../target/alt-fs/before-castle-cloud-sea.png) and [after](../target/alt-fs/after-castle-cloud-sea.png), using the same roof-edge pose with `--no-env-probe --no-layers --no-fog` and only the baseline's `--no-fog-surfaces` toggle.
Both Outskirts GroundFog materials load through their own connected-block asset source, and their sea covers the empty background.
The connected diagnostic also exposes a dark foreground card with probe/global fog disabled; coverage and normal-w/identity-transform audits pass, and full visual parity is not claimed.
The final [probe-lit Castle control](../target/alt-fs/after-castle-probe-lighting.png) exercises the default environment fallback and global fog with `--no-connected-blocks --no-giiv`, without shader or asset errors.
Native forward fog, instance colour/SH overrides and the shared renderer's untraced per-mesh render-group semantics still need parity validation.

The one-view, 100 m spacing, primary-only sweep completes for both maps at 256x144 with environment probes and ordinary material layers disabled.
Outskirts reports `void=0` and `depth_gap=14,412` both with and without surfaces; Castle's surface-enabled smoke reports `void=0` and `depth_gap=36,711`.
Reports are [Outskirts before](../target/alt-fs/sweep-outskirts-before.json), [Outskirts after](../target/alt-fs/sweep-outskirts-after.json) and [Castle after](../target/alt-fs/sweep-castle-after.json).
These are single sampled views, and `map_material::sweep_base` does not recognize the new surface material, so the diagnostic depth pass omits its triangles.
The numbers cannot establish fog coverage improvement or whole-area completeness.

### Job D bootstrap handoff

`map::world::MapWorld` owns a bounded warm neighbourhood, individual block collision meshes, readiness and independent normal/additional request masks.
`BlockPart` on each logical render root preserves block ID and canonical part index; the root also carries Job E's `VisibilityPart`.
Job C first-visit snapshots feed `AuthoredVisibility`, and Job E alone writes logical-root visibility.
Job F's selector runs per MapPiece instance, with authored scene availability read from the GLB metadata; `DetailWeight` retains submission weights while the renderer selects the dominant scene pending a weighted coverage shader.
The diagnostic `--no-connected-blocks` keeps the legacy single-map loader for before/after comparisons.
The sweep bootstrap must use the cache as its asset root and `maps/<id>/` as its scene prefix for these two maps, as the viewer and Explore bootstraps now do.
A map-folder asset root produces sibling paths that Bevy's nested GLTF texture loader rejects, even when the GLB itself uses an explicit path override.
The material renderer must call `world::material_texture_path` for layer URIs instead of using one loader-wide map prefix.
The small routing patch is prepared in `target/alt-md/material-routing.patch` for the material owner or an approved integration edit.

**Job F done - export and selection interface (14 October 2026).**
All eight affected Outskirts models retain their original LOD0 scenes and now export all 93 separate LOD1/2 meshes used by 154 MapPiece instances, with face flags, mesh/node provenance and face-specific culling preserved.
Both maps were re-exported with the release extractor under `target/alt-mf`; Outskirts retains 6,489 parts and 99 connections, Castle retains 6,164 parts and 298 connections, and every job B per-kind count and the pre-export identity/transform/mask/LOD digest matches.
The cached-param selector implements signed MapPiece ID defaults, oriented-bounds distance, strict hysteresis and time/distance crossfade weights; Job D currently displays the dominant weighted representation, so visible dual-submission crossfade and its untraced coverage shader remain open.

### Job F export verification

The original eight GLBs had one scene and 59 LOD0 primitives altogether; the new GLBs keep those 59 and add 93 primitives in distinct LOD1/2 scenes.
The focused exporter regression checks every nonempty retained source face set, its index count, flags, mesh/node identity and correct scene membership, including motion-blur exclusion from ordinary scenes.
Lower-detail vertex buffers compact all populated streams to referenced vertices while preserving source values and triangle order, avoiding unused-vertex tangent comparisons and redundant GPU data.
The sampled MikkTSpace comparison improves from 69.5% to 95.1% handedness agreement and from 61.9% to 93.3% direction agreement after that compaction.
Flag-zero faces retain the legacy material prefix, equivalent levels reuse those materials, and distinct culling variants append separate materials; primitive `source_material` preserves the native FLVER material index independently.
The independent post-CLI audit reads the regenerated GLBs and verifies the following counts:

| Model | LOD0 before/after | LOD1 after | LOD2 after | Recovered LOD-only meshes |
|---|---:|---:|---:|---:|
| m004100 | 8 / 8 | 6 | 4 | 10 |
| m004110 | 6 / 6 | 5 | 4 | 9 |
| m004140 | 6 / 6 | 5 | 4 | 9 |
| m004200 | 6 / 6 | 5 | 4 | 9 |
| m004210 | 6 / 6 | 7 | 5 | 12 |
| m004300 | 6 / 6 | 5 | 6 | 11 |
| m004410 | 11 / 11 | 8 | 7 | 15 |
| m004420 | 10 / 10 | 11 | 7 | 18 |

Exported model counts remain 712 and 767, with zero export failures and job G's unresolved texture names unchanged at three and zero respectively.
Audit results and exporter logs are `target/alt-mf/after-export-audit.json`, `export-audit.log`, `export-m11_00.log` and `export-m11_01.log`.
The eight-view sweep baseline stopped on collision-depth validation at pixel 128,72, where the CPU was clear to 100 m but the GPU reported about 48.34 m.
Its one valid Outskirts row is retained in `target/alt-mf/before-sweep-outskirts-partial.json`; subsequent one-view baseline runs stalled at asset loading and were stopped, so they supply no complete-area totals.
Fixed-camera baseline captures and their commands/logs remain under `target/alt-mf`, and scene-0 comparisons are compatibility evidence rather than proof that the broader visibility/streaming holes are solved.
Inspected after-captures are `after-outskirts-legacy.png` at the same explicit Outskirts camera and `after-castle-start.png` at the same Castle start camera, paired with `before-outskirts-start.png` and `before-castle-start.png`.
The Outskirts after-capture includes the concurrent connected loader changes and logged foreign-block layer-texture lookup errors; Castle's after-capture uses `--no-connected-blocks`.
These frames reflect the combined C/D/E/F work and do not isolate Job F's contribution.
The primary-only Outskirts one-view after sweep completed at the baseline's first valid pose, with the following limited comparison:

| Metric | Baseline valid partial view | After one-view smoke |
|---|---:|---:|
| Void | 18,432 | 0 |
| Depth gap | 20,062 | 20,062 |
| Untextured | 0 | 0 |
| Grey | 0 | 7 |
| Pop-in | 0 | 0 |
| Score | 38,494 | 20,069 |

The after report is `target/alt-mf/after-sweep-outskirts.json`, with `complete_sample: false`; unchanged depth-gap counts and the small sample prevent an area-completeness claim.
The final `cargo test --release --offline -p sekiro-formats -p sekiro-extract -p sekiro-game -j 4` run passes 213 tests and one doctest, with four existing ignores, including the exporter, tangent, roof-snow and canonical-layout regressions.
All-target Clippy for those three crates with warnings denied and `cargo fmt --all --check` pass; final logs are `tests-verified.log`, `clippy-verified.log` and `fmt.log` under `target/alt-mf`.

### Job F integration interface

Job D's loader scopes `map::lod` runtime selection to Outskirts and Castle; other maps continue using scene 0.
`LodTable::load(cache, platform_row)` reads cached LodParam and LodPlatform, with Windows quality rows 4, 5 and 6, and `map_piece(lod_param_id)` implements the signed-byte MapPiece default and errors on missing explicit rows.
Keep one `LodState` per canonical `part_index`, compute `bounds_distance(camera_position, instance_transform, model_min, model_max)`, then call `update(rule, scales, distance, frame_delta, available_levels)` to obtain three complementary submission weights.
Job D owns attaching these representations to the shared loader and Job E owns combining their submission with residency, story and group gates.
The function does not write entity Visibility or replace a part's identity.

Each exported GLB has separate scenes 0, 1, 2 and 3 for ordinary LOD0, LOD1, LOD2 and LOD_EX geometry.
Scene 0 remains the default for legacy consumers, and empty authored levels remain explicitly empty scenes.
Primitive extras preserve `sekiro_lod`, `face_flags`, `source_mesh`, mesh-local `source_face_set`, `source_node`, `source_material` and `cull_backfaces`; motion-blur face sets are retained in an auxiliary mesh without ordinary-scene submission.
`sekiro_lod_version: 1` in document extras identifies the new assets; canonical layout part indices, names and authored LOD IDs remain unchanged.
When an entire requested level is absent, the interface uses the nearest available authored model level, preferring higher detail at a tie; this availability fallback is an explicit robustness policy rather than a traced retail rule.
Do not add individual LOD0 meshes into an otherwise complete authored LOD1/2 representation.

The extra per-type scale/adjustment families and world distance multiplier remain explicit `LodScales` inputs because their configuration provenance is unresolved.
Use the loaded platform scales and unit type/world scales with zero adjustments until those inputs are established, and record that effective-threshold limitation.
Objects with ID 255 need their model-side default and must not use the MapPiece binding indiscriminately.
The selector implements weighted time/distance crossfade decisions, but the retail dither/coverage shader remains untraced and the final dual-scene rendering is Job D's integration work.
Connected-block layer texture URI routing remains a separate material-owner integration issue exposed by the capture logs, rather than an omitted FLVER representation.
Fog cards remain filtered by the `m.contains("fog")` branch in `pieces::skip_material`; Job F does not change that later job's surface handling.

**Job E done - visibility implementation (14 October 2026).**
The confirmed six-mask preprocessing, MapPiece-only zero fallback and per-target connection unions are implemented in `map_visibility.rs`, with one root writer combining event, residency, readiness and draw decisions.
`arena_b.rs` publishes managed-root event snapshots instead of competing writes, while other-map loading remains on its original path.
All eight Job E regressions pass; full-game units, all-target Clippy and formatting pass, with the shared tangent integration failure and visual-capture blockers recorded below.
The four-view sweep baseline scores are 62,172 for Outskirts and 307,586 for Castle; integrated after totals remain unavailable because capture readiness is blocked, so no area-completeness or retail-occupancy claim is made.

### Job E integration interface

Job E's first integrated Outskirts capture attempt is blocked by sibling-map asset paths being rejected as unapproved despite the loader's override calls.
The evidence is `target/alt-me/after-outskirts.txt`, including rejected `../m20_00_00_00/models/...` GLBs and scene labels; capture readiness is never reached.
Job D owns the loader/asset-root correction, and this attempt is not an after measurement.
The complete game release unit suite passes 100 tests with two ignored, including all eight Job E regressions.
The separate game integration suite passes five tests but `map_tangents::exported_tangents_match_mikktspace` fails on the current shared Outskirts exports: 3,624,354 vertices, 0.695 handedness agreement and 0.619 direction agreement.
The tangent/export correction belongs with the exporter owners; Job E does not weaken or suppress this check.
Fixed-camera viewer attempts at the same retained baseline poses also remain below capture readiness and report missing DDS paths, including neighboring-map texture names under the primary map's texture prefix.
Logs are `target/alt-me/fixed-outskirts.txt` and `fixed-castle.txt`; neither attempt produced an after image.
Castle's `--visibility-part m902000_9002` diagnostic confirms collision `h018700` and connection `h018700_1000` contributing the authored target mask to m17_00_00_00, while readiness and residency independently keep the part hidden.
Baseline reports and inspected captures are preserved under `target/alt-me/before/<map>/objects/smoke-4/`; these are incomplete smoke samples from the pre-integration sweep executable, not a complete survey or an isolated E effect.

Job E verification commands used the isolated target and four Cargo jobs:

```powershell
$env:CARGO_TARGET_DIR = 'target/alt-me'
cargo test --release --offline -p sekiro-game -j 4
cargo test --release --offline -p sekiro-game -j 4 map::visibility::tests
cargo test --release --offline -p sekiro-game --bin sekiro-rs -j 4 managed_roots_combine_groups_events_and_residency_across_restarts
cargo clippy --release --offline -p sekiro-game --all-targets -j 4 -- -D warnings
cargo fmt --all --check
```

The module is `crate::map::visibility` (`map_visibility.rs`), declared with one mod addition in map.rs.
Job D installs `MapVisibilityPlugin` once and puts `VisibilityPart::from_row(map, canonical_row)?` on every rendered MapPiece/Object root for the two scope maps.
Other maps return `None` before schema parsing and keep their existing path.
The legacy `apply_groups` query must exclude `VisibilityPart`, and managed far scenery must use this same writer rather than bypass the authored masks.
`arena_b` already excludes managed roots from its direct writer and replaces `ArenaVisibility.maps[map]` with a fresh event snapshot in `VisibilityInputs`.
Job C's optional `MapStory` resource is consumed directly by `(map, entity_id, part_name)`, default absent.
External/dynamic event hosts can also supply `AuthoredVisibility.maps[map] = EventState { by_name, by_entity }`, default empty; entity-group IDs share the entity lookup table.
Named overrides precede direct entity IDs, then the first authored positive entity-group ID; an arena snapshot takes precedence when it contains an applicable override.
Every event restart must replace its snapshot, including an empty snapshot.

Job D decodes each scope layout once with `VisibilityMap::from_layout` and calls `requests(participating_collision_names, connection_enabled, target_enabled)` for every contributing source block.
The callbacks apply the source-enable/connection-processing and target-world gates; unresolved retail gates must be documented by D rather than assumed to be exe facts.
Merge multiple source blocks with `BlockRequest::merge`, retaining each collision/connection contribution for diagnostics.
The returned `normal` masks use the separate Collision/ConnectCollision `display_group` accessor, including the integer Condition's single bit, and never copy source masks into a connected target.
Set `VisibilityBlocks.0[map] = BlockVisibility { request, draw, resident, ready, groups_off }` before `VisibilityWrite` in PostUpdate.
`draw` is the accumulated per-block draw request; `request.normal` and `request.additional_backread` retain the separate residency requests.
Asset readiness and actual retained block residency are explicit D inputs; a backread intersection alone does not enable drawing.
With no participating collision, the masks are empty; any retained navigation/airborne context must be explicitly supplied as participating names, avoiding accidental last-mask retention.
The upstream occupancy/navigation decision remains D's policy; the executable audit does not justify replacing it with a new nearest-surface or spherical heuristic in E.

`--visibility-debug` logs changed part decisions with original/effective masks and all chosen collision/connection sources.
`--visibility-part <exact-name>` logs the complete JSON diagnostic for that part when request or event inputs change, including unchanged decisions.
Capture consumers can call `diagnostic(part, block, decision)` directly.
`VisibilityMap` exposes zero-primary counts, zero-effective counts before the MapPiece construction fallback, and the number of all-bit MapPiece fallbacks.
The fallback applies only to MapPiece and remains subject to event, nonempty demand, residency and asset readiness; Object and Collision zero masks receive no invented universal fallback.

| Zero draw-mask audit | Outskirts primary / effective before fallback | Castle primary / effective before fallback |
|---|---:|---:|
| MapPiece | 111 / 1 | 15 / 0 |
| Object | 447 / 51 | 313 / 30 |
| Collision | 49 / 10 | 19 / 11 |
| Enemy | 144 / 144 | 284 / 284 |

These counts include all canonical parts rather than only exported GLBs.
The MapPiece construction path expands one Outskirts part and no Castle parts to all bits; the two condition bytes perform mask preprocessing, not story-flag evaluation.
Zero Collision defaults outside the confirmed path and Enemy rendering remain their category pipelines' responsibility.

**Job C done (14 October 2026):** constructors and substituted common calls now produce typed entity/group overrides and per-part first-visit lists for Outskirts and Castle.
The installed-data regression proves the normal Castle floor, hidden burned floor, invasion-only groups and completed Object restoration; the old common-helper truth table and floor part names are corrected above.
The full sim release suite passes 518 tests with 20 ignored, and sim/game all-target release Clippy passes with warnings denied using `target/alt-mc` and four jobs.
Live world predicates and unrelated rendering checks remain explicit limits below.

### Job C first-visit fixture and handoff

`world_events::FirstVisit::for_map` defines exploration startup after the prologue.
Both maps start with `11120100` ON; Castle also starts with Gyoubu defeat flag `9301` ON.
Every other flag starts OFF, including `8301`, `8302`, `8306`, `11115921`, Genichiro/idol flag `9403`, pickup, interaction, quest and event-slot completion flags.
This is a reproducible exploration fixture, not an imported save or a claim that every optional quest has the same state in a player's first visit.
Flag `11120100` is essential: common preconstructor 50 otherwise raises `8306` before the map's story helpers execute.
All event layers are eligible, loaded Objects answer backread-ready, all characters start alive, and Wolf has no SpEffects and occupies no encounter regions.
Relevant quest/pickup inventory is empty, no action button or Object interaction occurs, and no TAE event message has fired.
Characters are backread-ready unless the scripts remove them; the player is an offline host in its own world.
Common initialization may populate initially unset quest flags, including `1380`; the emitted flag set is the resulting state rather than a repetition of the input fixture.
Character combat state, generators, skit areas and interaction predicates without an implemented observation remain unknown and block their dependent path.
Their event, slot, instruction and opcode are retained in `WorldState.unsupported`; unrelated omitted actions are listed separately.
This snapshot does not execute combat, award items, play cutscenes or simulate a save.

The interpreter runs common/map preconstructors 50, then constructors 0 and their local/common calls, with byte substitutions from each actual invocation.
It advances startup for 720 fixed ticks after preconstructor initialization so brief Object restore sequences can finish.
The interpreter uses Sekiro's [DarkScript3 instruction table](https://github.com/AinTunez/DarkScript3/blob/master/DarkScript3/Resources/sekiro-common.emedf.json) for comparison, condition-group, flag and control-flow layouts.
Notably, 1000[3] is an unconditional skip, 1000[7]/[8] test compiled groups, and 1000[1]/[2] test uncompiled groups.
The existing combat interpreter and its arena-specific entry points are not used or changed.

Job D loads `map::story::load(cache, map_id, layout_v2)` into `MapStory.maps` for the base map and each supported resident neighbor.
The function returns `None` for every other map before reading event files, preserving their current loading behavior.
Job E reads `StoryMap::enabled(entity_id, part_name)` as the story input to its single visibility decision.
`StoryMap.parts` and `WorldState::parts` expose a per-map placement list keyed by entity ID and part name, with the canonical part index and last applicable command's event/slot/target/order.
Visibility commands remain typed as Object, MapPiece, Collision or Enemy and address both individual entities and all eight entity-group slots.
The most recently executed matching group/entity command wins within that part kind; parts without a command retain the ordinary enabled default.
Collision commands also populate `SceneState.collisions_enabled`, separately from visual MapPiece commands.
No story system writes Bevy Visibility independently of job E.
`--no-map-story` is a diagnostic control that skips this snapshot for paired sweep measurements while retaining the same geometry and loader.
DummyObjects and other helper kinds receive no invented ordinary-part defaults.

The focused integration replay checks pre-invasion groups in both installed maps, normal versus burned Castle floor selection, Castle Object restoration and a parameterized multi-slot synthetic script with overlapping entity/group writes and independent collision state.
Its per-part snapshots and opcode diagnostics are written under `target/alt-mc/story/`.
Scene animations retain Object 2005[7] restore poses for the Object-pose consumer, although applying those poses remains job H.

| Settled story gate | Outskirts enabled / disabled | Castle enabled / disabled |
|---|---:|---:|
| Ordinary Objects | 1235 / 274 | 1530 / 144 |
| MapPieces | 4462 / 36 | 3591 / 45 |
| Collisions | 160 / 6 | 204 / 17 |

These counts describe story enablement before job E's masks, residency, readiness and renderer/helper gates.
Executed commands reach 456 / 232 ordinary Objects, 151 / 81 MapPieces and 10 / 19 Collisions in the two maps.
The remaining supported placements retain their ordinary startup defaults.

#### Job C validation and paired sweep (14 October 2026)

All commands use `CARGO_TARGET_DIR=target/alt-mc`, offline Cargo and four build jobs.
`cargo test --release --offline -p sekiro-sim -j 4` passes 200 unit and 318 integration tests, with 20 existing ignored tests.
The focused `world_events` filter passes all three tests in 5.02 seconds during the final render session.
`cargo clippy --release --offline -p sekiro-sim -p sekiro-game --all-targets -j 4 -- -D warnings` and `cargo fmt --all --check` pass.
The final workspace formatting recheck observed in-progress fog additions in `crates/extract/src/maps/pieces.rs`; job C left that owned file unchanged and checked sim/game formatting separately.
`cargo test --release --offline -p sekiro-game --test it -j 4` passes all six integration tests.
The full game run passes 99 unit tests with two ignored, but the existing `genichiro_settled_snow_selection` assertion at `lighting/maps/m11_01.rs:100` fails after the detail-level export changed material indexing.
The expected wood-floor snow variant 11 is present in exported material indices 18-20; the test still indexes by the original FLVER material position.
That test and its exporter are outside job C's ownership and were left to their owners.

The paired sweeps use the same executable, cache, camera coordinates, spacing 80 m, one yaw, optional edge pitch -25, two-view limit, 128 by 72 resolution and 12 settling frames.
Both sides use `--objects --no-layers --no-shadow --no-giiv`; only the before side adds `--no-map-story`.
These are geometry/state diagnostic controls, not native-material or full-area comparisons.
The native-material attempt hit connected-neighbor layer texture requests resolved against the primary map's texture folder, so layers were disabled in the matched controls.
The two retained Outskirts positions are below the visible landscape, and represent only two of 79 sampled views at this coarse spacing; Castle represents two of 43.
All four reports and worst-view image sets completed, and the native Castle processes returned exit code zero.

| Pixel candidates, before -> after story state | Outskirts | Castle |
|---|---:|---:|
| Void | 9216 -> 9216 | 0 -> 0 |
| Depth gap | 18018 -> 18018 | 2739 -> 6852 |
| Untextured | 0 -> 0 | 0 -> 0 |
| Grey | 0 -> 0 | 376 -> 446 |
| Normal/all-visible difference | 11649 -> 15040 | 3271 -> 2259 |
| Summed score | 38883 -> 42274 | 6386 -> 9557 |

The scores do not establish an improvement in map completeness.
The sweep's collision reference includes every cached triangle except material IDs 22/36/38, without filtering job C's disabled collision groups, and its all-visible comparison changes authored visibility.
The measured differences therefore need reachable-camera validation and story-aware collision references before being classified as defects.
The paired reports are under `target/alt-mc/control-before/target/alt-hs/sweep/` and `target/alt-mc/control-after/target/alt-hs/sweep/`, with an identity-checked summary at `target/alt-mc/story/sweep-comparison.json`.
The normal and burned floor states and invasion gates are proved by the installed-script regression rather than inferred from these coarse captures.

**Job A done (14 October 2026):** all ten cached MSBS files retain 55,352 placed parts, with raw/parsed counts verified independently for every type, 582 ConnectCollision rows and seven MapOffset events.
Outskirts retains all 99 connection rows and Castle all 298; 580 connections resolve within the collision-only list, while Outskirts `h004490_1000` and Dungeon `h009700_1004` explicitly store `-1` and remain unbound.
All 1,973 non-sentinel Object/DummyObject references and 1,723 Enemy/DummyEnemy collision references resolve within the full parts table, and 160 patrol references resolve within the PatrolInfo event list; masks, every condition and MapOffset provenance match their raw records.
The reader preserves SoulsFormats DisplayGroups/DrawGroups/CollisionMask and the separate DispGroups structure without inventing a BackreadGroups alias; retail loading/condition semantics and game integration remain with the later jobs.
Per-map type counts and every connection's resolved source collision, target-map bytes and condition are recorded in `target/alt-ma/msb-audit.txt` by the installed-data integration test.

**Job G done - extractor (14 October 2026).**
Exact-name resolution now searches area archives and cached map, object, character and shared texture banks, with material/slot/source provenance in each map's `material-audit.json`.
The unsafe first-texture substitution is removed, and albedo/normal uses of one authored image receive separate DDS payloads and GLB references.

| Re-export measurement | Outskirts before -> after | Castle before -> after |
|---|---:|---:|
| Fresh missing export texture names | 7 -> 3 | 0 -> 0 |
| Exported model GLBs | 712 -> 712 | 767 -> 767 |
| Object instances without render GLBs | 56 -> 56 | 61 -> 61 |

Outskirts' 56 omissions are authored meshless FLVER helpers; Castle has 60 such instances and one physics-only binder, with zero genuine visible-model export failures in either set.
The three unresolved Outskirts names are `m11_camp_curtain_00_n`, `m96_snow_01_a` and `3dsmax`, the last an authored directory path used as a mask slot.
All ten cached maps have inventories, with summed per-map missing texture demands reduced from 63 to 50; only these two maps were re-exported, using `target/alt-mg`.
The extractor's 18 release unit tests and two integration tests, all-target Clippy with warnings denied and formatting checks pass.

A precedes B, and B supplies the schema contract for C/D/E/F.
F and H should agree on a geometry-builder interface before H requires changes to pieces.rs, which remains F's file.
D alone owns bootstrap edits in map.rs and play.rs, incorporating the new C/E/F/H modules after their interfaces are reviewed.
The game and sim root-module declarations should be small integration handoffs to that owner rather than simultaneous edits by every job.
Existing hole-sweep captures should be used to compare fixed camera/grounded walks before and after each job, with one runnable result per job.

## Open retail questions and verification record

The remaining executable research should be narrow: identify MSBS Condition1/Condition2 and connection Condition evaluation, ID-255 LOD resolution, streaming request/retirement policy and target-block visibility-mask transfer.
Only after those are understood can the single-collision heuristic and unconditional FarScenery bypass be replaced with an exact game rule.
AutoDrawGroupCollision and other event types should not be promoted to explanations merely by name: neither audited MSB contains event type 24, and their event types 19/24 are absent.

Since the reference projects did not settle those rules, existing static-analysis artifacts were searched without running the retail game or changing re/.
`re/exports/strings.tsv` contains WorldBackRead, WorldMapMan_BackreadRequestUpdate, a worldmsblist path, BackReadGroup debug text and LodBank references.
Those are useful Ghidra entry points, but the existing targeted exports do not establish the relevant algorithms and no callable Ghidra service was available in this session.
They are not evidence that CollisionMask means BackreadGroups, or that a particular streaming radius is exact.
No retail screenshot comparison, live-memory access or new decompilation was performed.

Independent audits used `uv run --offline --no-project python -` for binary-table/header checks and `uv run --offline --no-project python -m target.alt-mr.map_audit` for the reusable original audit helper.
The uv cache was redirected to `target/alt-mr/uv-cache`, and the helper source lives under the ignored build directory.
The helper reports semantic counts and references only; it does not regenerate maps or write extracted game artifacts.
Existing `emevd_dump` was run through Cargo for the Castle floor event, first-visit Object events and common helpers 20004110/12/20, 20005500/53/60 and 20005820/25.
All Cargo commands used `CARGO_TARGET_DIR=target/alt-mr`, offline mode and `-j 4` where applicable.

The exact fresh 1-14 missing-texture counts, a complete settled first-visit event snapshot and the unresolved executable rules above remain open.
No source crate was changed, no cache/re content was changed, and no commit or publication was made.

Validation commands and results:

- `cargo test --offline --release -j 4 -p sekiro-formats msb`: two focused installed-data/MSB tests passed, zero failures.
- `cargo clippy --offline -j 4 -p sekiro-formats --all-targets -- -D warnings`: passed.
- `cargo fmt --all --check`: final run passed.

An earlier formatting check observed in-progress formatting differences in the concurrently edited map_sweep.rs; that file was left to its owner and the final check passed.

## Rules from the exe

The D1 follow-up additionally traces the missing `EnableAnalyticalFog` default to false in constructor `0x1402a0740` and its registration at `0x14029c180`.
This is independent of draw-group preprocessing and LOD selection.
See the D1 diagnosis in `VISUAL-PARITY.md` and the forward fog packet in `SHADER-INPUTS.md`.

The horizon material is the native Mountain family on Outskirts `m925000`, with related landscape facets on `m902000`, `m902010` and Castle `m911061`.
The `m6*` names alone do not identify that horizon.
The targeted export option refreshes the selected placed models and required textures, preserving layout, collision, grass, environment and lighting sidecars:

```powershell
sekiro-extract map m11_00_00_00 --models m902000 m902010 m925000
sekiro-extract map m11_01_00_00 --models m911061
```

Research date: 14 October 2026.
This section supersedes the earlier unresolved statements about mask preprocessing, connection Condition and map-piece LOD ID 255.
It describes static observations from the existing Ghidra program, not a retail screenshot comparison or a live-memory experiment.
Addresses below are virtual addresses at image base `0x140000000`; function roles are inferred unless a diagnostic string establishes the name.

Ghidra ran successfully in the sandbox after a launcher workaround.
The supplied batch launcher initially failed because its Java-home chooser tried to write outside the workspace.
Launching the same headless application directly with the installed JDK and workspace-local settings/cache/temp directories avoided that failure.
The existing project stores the program under a different name than the executable, so processing by the executable name initially reported that no program matched.
All successful inspections processed that stored program with `-noanalysis -readOnly`, with original scripts under `target/alt-gh/scripts`.
No script changes program bytes, symbols, functions or analysis, and no analysis or program save was requested.
Ghidra itself maintains transient project housekeeping files even in this mode; a subsequent open reported removing a stale presave file after the bounded instruction scan was interrupted.
Raw function exports, instruction-search results, runtime settings and audit helpers remain under `target/alt-gh/`.
No decompiled code is reproduced here.

### 1. Condition1/Condition2 select mask preprocessing, not story flags

**High confidence:** the MSB loader at `0x1410957c0` handles these bytes while fixing up the loaded part records.
The installed Outskirts and Castle PartsParam versions are both 35, so the modern six-array branch applies; the loader also contains conversions for older layouts.
For MapPiece, Object and Collision types, preprocessing occurs when `Condition1 == 0` and `Condition2 != 0`.
For the other three zero/nonzero combinations, this particular preprocessing leaves the masks unchanged.
Neither byte is looked up as an event flag in this branch.

The editor's 32-word `CollisionMask` actually contains four eight-word slices used alongside the first two arrays.
Keep the editor names for file compatibility, but distinguish these executable roles:

| Byte offset within DisplayDataStruct | Editor field | Role in the inspected executable path |
|---|---|---|
| `0x00` | `DisplayGroups[0..7]` | Primary draw mask |
| `0x20` | `DrawGroups[0..7]` | Alternative draw mask |
| `0x40` | `CollisionMask[0..7]` | Selector between the two draw masks |
| `0x60` | `CollisionMask[8..15]` | Primary backread mask |
| `0x80` | `CollisionMask[16..23]` | Alternative backread mask |
| `0xa0` | `CollisionMask[24..31]` | Selector between the two backread masks |
| `0xc0` / `0xc1` | `Condition1` / `Condition2` | Preprocessing gates described above |

For each bit, a set draw-selector bit chooses the primary draw bit, and a clear selector bit chooses the alternative draw bit.
The backread selector performs the same operation on the two backread masks.
Equivalently, using original mathematical notation, the resulting masks are `(primary & selector) | (alternative & ~selector)`, independently for draw and backread.
The loader writes the results into the primary masks.
For Collision parts it then unions the resulting primary draw mask into the primary backread mask, including when the preprocessing gate was false.
This is materially different from intersecting the first two editor-named arrays with each other.

**High confidence:** accessors `0x141092570` and `0x141092400` read the resulting primary draw and backread masks respectively.
The executable's own MSB debug exporter at `0x140854470` calls the first accessor for its `draw_group` output; `0x140c1be40` supplies that output label.
Thus the first array's editor name does not establish its retail role.
At `0x140853d40`, a MapPiece with an empty resulting primary draw mask is expanded to all 256 bits, and its backread mask is set to that expanded mask.
That confirms an all-zero-visible fallback on this map-piece construction path, rather than the permanent zero-mask deletion currently described above.
Do not extrapolate that fallback to every part category: collision construction has additional special-case defaults at `0x1409b5310`.

**High confidence:** map-piece backread selection at `0x14085dcc0` tests the part's resulting draw mask against the block's normal request mask first.
If that fails, it tests the part's resulting backread mask against the block's additional backread mask.
These intersections request different residency states; actual drawing still has its own group, event/ceremony, asset-readiness and rendering gates.
The generic 256-bit intersection helper is `0x14085d8c0`.
This establishes the mask algebra without claiming that satisfying it alone makes a part visible in every event state.

**High confidence:** the separate Collision/ConnectCollision integer `Condition` is a group-bit index in the inspected display-mask accessor, not an event condition to evaluate as true or false.
Accessor `0x141092480` reads that integer and unions its single bit with the structure's eight explicit `DispGroups` words.
Helper `0x1410925f0` creates the bit in a 256-bit mask.
A negative authored integer contributes no bit; explicit `DispGroups` still contribute.
The `-1` argument used by the ordinary callers means use the authored integer, rather than overriding it with another index.
Values outside the intended 0-255 range should not be assigned new semantics from this helper's unchecked shift behavior.

### 2. ID 255, distance thresholds, hysteresis and crossfade

**High confidence for MapPiece:** `0x1410a7f80` reads the authored LOD byte as signed.
Byte 255 therefore becomes -1 and selects the caller's default, with a negative default replaced by 0.
MapPiece setup at `0x140858ee0` supplies -1, so ordinary map pieces with ID 255 bind LodParam row 0.
This is a default-ID rule, not a failed search for a nonexistent row 255.
The renderer creates the LOD instance at `0x140f18d40`, initializes it at `0x140f3a830`, and resolves its row through `0x14108e2a0`.
That resolver searches the loaded LodParam rows and returns no row when an explicit requested ID is absent; it does not itself retry row 0.

**High confidence for the default mechanism, category-dependent result:** Object setup at `0x14087e580` obtains its default from `0x1408707e0` before calling the same authored-byte accessor.
The object default comes from a signed byte at offset `0x0b` of its model-side record when present.
Consequently, ID 255 means use the category/model default, and should not be globally rewritten to row 0 for every Object or character.
The exact provenance of that object model-side byte remains open in this audit.

**High confidence:** `0x14108e010` accepts the LodParam table, and `0x14108e0a0` selects LodPlatform row 4, 5 or 6 from the graphics quality setting.
LodParam is `LOD_BANK_ST`; its relevant fields are `lv01_BorderDist`, `lv01_PlayDist`, `lv12_BorderDist`, `lv12_PlayDist`, `lodCrossFadeEnable` and `lodCrossFadeTime`.
The boundary scales come from LodPlatform's `lv01_BorderDist_Scale` and `lv12_BorderDist_Scale`.
The boundary selector also applies per-type scale tables and additive adjustment globals, so the parameter's bare distances are not the entire effective threshold in every configuration.
Those extra tables are refreshed by `0x140f3aa60`; their complete configuration-table provenance was not established here.

**High confidence:** LOD update `0x140f3ac20` uses the distance from the view position to the instance's oriented bounds, followed by an additional world distance scale.
Helper `0x140161830` transforms the point into the bounds' local axes, clamps it to the half-extents and returns squared distance to the nearest point; `0x142384a98` takes the square root.
Thus this is not simply distance from Wolf to the part's MSB origin.
The bounds-update path and camera inputs should be reproduced before applying the thresholds to our exported instances.

Let B01 be the scaled and adjusted first boundary, and B12 the larger of B01 and the scaled and adjusted second boundary.
Let P01 and P12 be the authored play distances.
Selector `0x140f3b240` behaves as follows for the ordinary mesh path:

| Current level | Remain/select LOD0 | Remain/select LOD1 | Select LOD2 |
|---|---|---|---|
| 0 | Distance below B01 + P01 | Otherwise distance below B12 | Otherwise |
| 1 | Distance below B01 - P01 | Otherwise distance below B12 + P12 | Otherwise |
| 2 | Distance below B01 | Otherwise distance below B12 - P12 | Otherwise |

The table preserves the selector's direct-jump behavior: a jump from level 0 can go straight to level 2, and a jump from level 2 can go straight to level 0.
At equality, the selector advances to the next branch because its retention comparisons are strict less-than.
A negative authored first boundary forces LOD0; a negative second boundary prevents LOD2.
With the installed row 0 and unit scales/zero adjustments, outward 0-to-1 and 1-to-2 transitions occur at 32 m and 64 m, while inward 2-to-1 and 1-to-0 transitions require less than 56 m and 28 m.
The nominal boundary centers remain 30 m and 60 m.
These are bounds distances, and the crossfade delays completion of a selected transition.

**High confidence:** with crossfade enabled and positive time, `0x140f3ac20` retains the old/current level and a target level while accumulating frame delta divided by `lodCrossFadeTime`.
Completion commits the target level; disabled crossfade commits immediately.
With crossfade enabled and nonpositive time, the update instead blends by distance across B01 ± P01 or B12 ± P12, with a linear blend fraction across each band's width.
This distance mode does not use the history-based selector for its ordinary level pair.
It also uses the second per-type multiplier/adjustment family, rather than the first family used by `0x140f3b240`, so the effective centers must be calculated for the selected mode.
Renderer submission at `0x140f19a40` sends both level representations during a transition with complementary weights.
The installed default row enables a 0.2-second transition; rows 60 and 61 disable it.
The exact FLVER face-set-to-render-submission mapping and pixel coverage/dither shader were not traced here, so these addresses establish the level decision and weighted submissions, not the final crossfade pixels.

### 3. Streaming uses navigation demand and separate group masks

**High confidence:** `WorldMapMan_BackreadRequestUpdate` is a profiler label in the table rooted at `0x143b04870`, registered by `0x14096c7c0`.
It is not itself a callable function address.
The functional request path followed here is `0x140affb20`, which updates WorldBackRead, combines per-block masks and distributes them to the block, collision, map-piece, object and character managers.
WorldBackRead's constructor is `0x14081d460`, its navigation-demand update is `0x14081e460`, and its request/retirement state machine is `0x14081eae0`.

The constructor maintains two sets labelled `Default` and `Hit` in its debug menu.
After initialization, Default has start/end distances 0/1 m, while Hit has 80/90 m.
Both sets initialize a 0.2-second promotion interval and a 3-second retirement interval.
These are executable constructor defaults, not a universal spherical neighbor-prefetch radius or measured disk-unload time.
The values are exposed by `0x14081e910` and may be overridden; the navigation query uses the greater end distance of the two sets.

**High confidence for the state transitions:** `0x14081f020` creates a pending request only for a valid navigation record within the set's start distance, and refreshes existing records within its end distance.
New requests have their promotion timer seeded by proximity, so 0.2 seconds is not an unconditional delay for every newly requested record.
`0x14081eae0` promotes eligible pending records, keeps active records while demand remains inside the end distance, and moves stale/out-of-range records to a retiring state.
A retiring record can return to active when qualifying demand returns; otherwise it is removed when its retirement timer reaches the configured interval.
Each state change resets its timer, and pending promotion also considers other records in the set.
Active and retiring records continue contributing groups, which is the retention/hysteresis mechanism.

**High confidence:** `0x14081e460` obtains navigation records through the world navigation manager, rather than deciding solely from nearest vertical collision or Euclidean block-center distance.
It retains current/previous navigation context through `0x14081f1f0` and queries navigation around the tracked position through `0x14103afc0`.
The latter supplies the range and position to a staged query; `0x14103b4c0` advances that query before results are incorporated.
This makes residency a navigation-driven policy with asynchronous readiness and retained demand, rather than recursively loading every graph neighbor.
The precise gate traversal, computed navigation-distance metric, query work budget and airborne/no-navigation fallback remain open.

**High confidence:** `0x140a652d0`, `0x140ae1b70` and `0x1406c6120` reference `map:/WorldMsbList.worldloadlistlist`.
The last routine builds an indexed list of area/block pairs from that resource and searches it for a requested pair, with capacity for 32 entries.
The list therefore supplies world map/block inventory and indexing; finding its string is not evidence of a fixed streaming radius or timeout.
ConnectCollision relationships and the runtime request masks operate on that indexed world context.

**High confidence:** the broad `CollisionMask` editor field is not one 1024-bit backread group set.
Its primary backread slice is the eight words at DisplayDataStruct `0x60`, after the preprocessing in question 1.
WorldBackRead accumulates per-block 256-bit group masks through `0x14080e8a0` and `0x140b029b0`.
The `BackReadGroup` debug display at `0x1409fad10` reads a separate eight-word runtime set on the character record.
That diagnostic alone does not identify an MSBS field.
Actual residency is driven by the accumulated demand masks and part-level effective draw/backread masks; treating the whole CollisionMask as an interchangeable BackreadGroups array would lose the selectors and alternative masks.

Block resource requests are staged by `0x140825bf0`, `0x1408254f0` and `0x140826520`.
The block becomes requested when its combined normal/additional request masks are nonempty or another explicit request gate applies.
The block state machine waits for resource readiness and performs setup/teardown stages, so removing a WorldBackRead record is not synonymous with immediately freeing all of the neighboring block's assets.
**Medium confidence:** this identifies the block demand and staged retirement policy, but every external request override, resource reference count and final release barrier has not been traced.
No exact wall-clock unload latency or complete resident-neighborhood trace is claimed.

### 4. ConnectCollision contributes a target-block mask by union

**High confidence:** collision setup `0x1409b5310` enumerates ConnectCollision parts of type 11 and attaches each row to the source Collision whose collision-list index matches the row's first type-data integer.
This confirms the collision-list index interpretation and permits multiple connection rows on the same source collision.
Runtime accumulation `0x140b029b0` starts with the participating source collision's display mask and follows that collision's attached connection list when its connection-processing gate is enabled.
For each target world block matching the row's map ID, it unions the mask returned by `0x141092480` into that target block's 256-bit slot.
The contribution is the row's explicit `DispGroups` plus the single group selected by its nonnegative integer Condition.
It does not copy the source collision's mask over the target block or enable every target-map part.
Contributions from multiple participating collisions/connections are accumulated rather than choosing one connection to win.
The resulting per-block masks are installed by `0x140affb20`; the visibility-mask distribution path at `0x140810320` uses the same connection-mask accessor and union operation.

**High confidence for this matcher:** `0x1409b5f60` interprets map-ID bytes as signed.
A negative area, third or fourth byte bypasses that field's equality test, but a negative block byte is resolved to block 0 in this particular matcher.
That differs from the editor's broad block-wildcard discovery rule cited earlier.
Do not use editor wildcard expansion as proof of the retail target set.
Connection processing also has runtime source-enable and target-world gates, including a special target-map predicate at `0x140b02930`, so an authored connection row is not an unconditional load instruction.
The full meaning of the additional non-core-map gate was not resolved here.

**Medium confidence for the complete occupancy story:** the attached-source-collision and per-target-mask path is established, but selecting all participating player/camera/support collisions is a separate upstream operation.
This audit does not settle stacked-floor, airborne or grappling selection merely by identifying ConnectCollision.
Implement the confirmed target-mask union while preserving those upstream selection questions as explicit fidelity work.
Section 5 settles the airborne and grappling cases: the player's last hit collision stays in force.

### 5. The player's hit collision selects the map

**High confidence:** in play the per-block display masks follow one collision part, the player character's current hit collision, and the camera takes no part.
The per-frame map step `0x140a75920` reads the player (`WorldChrMan+0x88`), passes his position to the map manager (`0x14080ec20`) and then calls `0x140810310` with the character's hit collision at `ChrIns+0x1ad8`.
That wrapper reaches `0x140b028a0`, which stores the collision in the mask object's single sticky slot `+0x20` only when the pointer is non-null.
The same step then calls `0x14080ea80`, the wrapper of the per-block mask install `0x140affb20`.
The install unions, through `0x140b029b0`, the sticky slot's collision and up to eight transient collisions from slots `+0x28` to `+0x60`, then clears the transient slots.
The transient slots are filled by `0x140affaa0` (via `0x14080e880`) only from `0x140ecef80`, which walks a list of map ID and part index pairs held by its caller rather than any camera or ray state.
The one-collision path `0x140810320` starts from empty masks, forces the reserved block `0x59000063` to all groups, and unions a single collision's mask and connections; its only caller `0x140810bb0` is reached from `0x140a6c960`, `0x140a6cac0`, `0x140a6cf00`, `0x140a6d0f0` and `0x140a6d490`, which also call the install wrapper and were not traced further.
No camera object, camera position or camera ray appears in any of these functions.

**High confidence:** the character's hit collision is set by `ChrIns::SetHitCollision` `0x1409f5820`, which also copies that collision's draw-parameter block (`ChrIns+0x1a68` onwards) and map block index, so lighting and fog follow the same part.
Its per-frame caller `0x140a01d70` asks the character physics module for the ground contact's collision (`0x140bbf160`, an ID looked up through `0x14080cff0`) and sets it only when one exists.
`0x1409f7760` does the same from `0x140bbf860` and also skips a null result.
So while the character has no ground contact (in the air, grappling, perched on the wire), the hit collision stays the last one, and in any case `0x140b028a0` never installs a null.
By inference from the collision matrix (`docs/HIT-FILTERS.md`), parts the player capsule does not collide with cannot become his ground contact, so only player-collidable hit filters (`sekiro_sim::hit_filter::player_collides`) drive the masks.

**High confidence for spawn:** when the character has no hit collision yet, `0x140b024e0` (called from `0x140810180` on map entry and from `0x140811e10`) casts from the player's position plus a fixed offset, takes the first hit whose part type byte is 8 (map collision) and installs it with `0x1409f5820`, falling back to `0x140a24d70`.

**Medium confidence:** when no transient collision is queued, `0x140affb20` starts from a per-block array copied from the map manager (`+0x78` and `+0x4a0` of the object at `+0x18`) rather than from zero; whether that array is the previous frame's result or a fixed base was not resolved.
When transient collisions are queued, every block except the reserved one is cleared before the union.
`WorldChrMan+0x98` gates the player-driven update in `0x140a75920`; its meaning (possibly a free or debug camera) was not resolved.

The implementation follows the high-confidence branches.
In play, `map_world::MapWorld::stand` selects the player-collidable part under Wolf's feet, from 0.5 m above them to `STAND_REACH` (1 m) below, and keeps the last part when none is found.
Before he has stood on anything, the nearest player-collidable part below him stands in for the exe's spawn cast.
That one part drives demand, participating parts and the lighting selection, and the orbit camera no longer contributes.
The map viewer, render gates and captures keep the camera-driven probe with its 0.5 m tie band, because there the focus is a free camera rather than a character.
The height band stands in for the physics ground contact; the exact contact (the low-resolution binder, capsule contact points, slopes and steps) is approximated.

The evidence comes from six read-only decompile exports made with the same `MapProbe.java` script against `re/ghidra`; they are local working files and are not checked in.

**Reproduction:** grappling from the ledge at (-132.3, -42.2, -123.9) to the branch and turning the camera a full circle (`--live-orbit 400,8`), the old camera-and-focus probe selected `h900000` with empty display groups, and 62 of 76 live samples had no drawn map mesh within 20 m of Wolf.
With the standing part the branch keeps `h023300` at every camera yaw, the Outskirts block keeps its display groups, and all 79 samples had at least 533 drawn meshes near Wolf.
The lighting region now also stays on the ledge's part on the branch instead of switching to the helper wall's.
`map_world::tests::standing_part_survives_the_branch_while_the_camera_orbits` checks this on the cached data at both ends of the branch, and the live play test repeats the grapple and the turn in the game.

### Verification and remaining work

The read-only function exports establish the loader algebra, map-piece ID-255 fallback, bounds-distance LOD selection, weighted level submissions, two WorldBackRead state sets and connection-mask union.
An original local audit confirmed PartsParam version 35, the installed condition-byte combinations and LodParam/LodPlatform values without regenerating cache data.
Its command was `uv run --offline --no-project python -m target.alt-gh.map_rules_audit`, with the uv cache redirected under `target/alt-gh`.
The Ghidra runner and raw exports are `target/alt-gh/Run-Probe.ps1`, `target/alt-gh/scripts/MapProbe.java` and `target/alt-gh/probe1.txt` through `probe13.txt`.
The exploratory instruction scan was deliberately stopped after identifying the loader references; its partial result is not used to claim that no other readers exist.
One export attempt failed on a PowerShell address coercion and was followed by quoted-address exports; conclusions use the successful function exports.

`cargo fmt --all --check` passed with `CARGO_TARGET_DIR=target/alt-gh`.
No Rust crate was touched, so crate Clippy and test runs were not applicable to this documentation-only research.
The collaboration-board API read failed through the configured proxy; this report's existing claim record was retained, with no remote mutation.
No game session, save access, live-memory read, commit or publication occurred.

Still open: the object model default's source, extra LOD scale-table provenance, exact FLVER render-group binding and crossfade pixel shader, navigation-distance/gate traversal and fallback rules, every block-request override and final resource-release barrier, and the exact physics ground contact at seams and stacked floors (section 5 approximates it with a height band).
These limits do not invalidate the specific confirmed branches above, but they prevent claiming a complete retail streaming or visibility implementation.

## Gates and Outskirts regression

### Serial export and the historical regression

The two Ashina maps were exported once each, serially, with the current extractor and four worker threads.
Both exports had zero model failures: Outskirts exported 716 GLBs and Castle exported 768.
The canonical placement, transform, model, mask, condition and LOD inventories were identical before and after export.
Outskirts has 6,489 complete MSB parts versus 4,487 legacy layout rows, and Castle has 6,164 versus 3,624.
The other eight maps received metadata-only layout refreshes, preserving their existing geometry, textures and collision.

The old two Outskirts eyes were [-280,-118.832954,120] and [-280,-114.090096,40], both looking along yaw 0 at pitch 0.
The fresh cache reproduces the historical depth and void counts exactly with the old unfiltered collision reference and the same isolated rendering controls.

| Selector | Depth gap | Void | Untextured | Grey | Pop-in |
| --- | ---: | ---: | ---: | ---: | ---: |
| Legacy, connected blocks disabled | 14,659 | 0 | 0 | 185 | 0 |
| Current connected visibility | 18,018 | 9,216 | 0 | 0 | 15,045 |

Both eyes select h900001, whose display and draw demand masks are entirely zero and whose hit filter is 13.
The second eye is roughly 50 m below the real h004100 floor at -64.5096.
The legacy selector interpreted empty demand as no active filter and submitted all scenery, whereas the confirmed executable algebra requests no scenery.
The apparent Jobs D/E regression therefore comes from inappropriate survey poses on a helper plane, rather than loss of valid player-facing geometry or a required cache race.
The larger pop-in count has the same cause: the normal pass has no scenery while the all-visible diagnostic deliberately forces it back on.
The narrow fix excludes zero-demand and first-visit-disabled collisions from ordinary survey sampling and collision comparison.
The explicit --sweep-raw-collision control retains historical reproducibility; gameplay physics and the confirmed visibility rule are unchanged.
The reproduced pop-in total differs from Job D's earlier 12,926, so only the depth and void totals are claimed as exact historical reproduction.

### Data gate and numeric fixtures

The normal map_data_gate test checks all ten extracted maps in 4.83 seconds without a renderer or vertex-buffer reads, including the connected-world oracle.
It checks 55,352 complete placements, all eight part-type counts, model ownership, drawn-model meshes, declared standard/layer/fog texture URIs, authored missing-texture ceilings and 582 connection rows.
Two authored collision index -1 connections are explicitly unbound and are checked as such.
Unavailable maps skip only when their exported layout is absent; a broken extracted map fails.
Ten spatially dispersed standing collisions in each Ashina area compare the complete visible-part set against a small independent implementation of the exe mask rule using raw MSB fields.
The comparison includes raw attached-connection indices, the connection's own display mask, signed target matching and requested neighbours, with separate checked-in primary and connected visible counts.
The oracle assumes ready resident assets and enabled event gates so it isolates the confirmed mask algebra from story and asynchronous loading.
Each map's checked-in JSON contains only numbers: eight part counts, three missing-resource ceilings and, for Ashina, collision-table indices with expected visible counts.
Missing exported models and declared image URIs are zero for all ten maps.
Authored missing-name ceilings are 24, 5, 3, 1, 1, 25, 1, 7, 8 and 0 in the gate's map order.
These include raw material slots omitted from exported samplers and must not be confused with the exporter's smaller selected-sampler warning counts.

### Capture readiness and speed

The old readiness observer counted nested GLTF node-world notifications as additional placed scenes, allowing its counter to exceed the expected scene total before nested meshes were actually ready.
Readiness now counts each placed or LOD root once and checks the nested WorldInstance hierarchy before publishing a loaded map.
Material and fog metadata are parsed once per material rather than once per placed instance.
Fixed captures instantiate static node/mesh/material subassets directly from the same loaded GLTF assets, preserving hierarchy, transforms, extras and material identities.
The capture loader conservatively includes the union of every requested frustum plus the renderer's shadow-caster distance, with no far-plane exclusion.
Interactive loading remains on the ordinary scene path, and --sweep-all-assets provides an ordinary-loader diagnostic control.
An initial fast-capture prototype exposed a scheduling bug: newly instantiated fog meshes could miss PreUpdate fog conversion and be consumed as ordinary layered materials in Update.
Those opaque cloud cards produced false untextured counts and artificially hid void pixels.
Capture instantiation now explicitly precedes the FogConversion system set, preserving the authored fog path before ordinary material wrapping.
The diagnostic also consults uploaded GPU images instead of treating a retired CPU image asset and missing load-state record as proof of a missing texture.
The checked-in six-view gates include the first Outskirts idol, cliff/downward views, the Castle and distant landscape, and both sides of the Ashina seam.
Each area uses one windowless process and loads its neighbourhood once.
Every view and every metric has its own ceiling, with tolerance max(16 pixels, 1% of its baseline), plus a 120-second total runtime limit.
Reports retain selected collisions and visible-part names for investigation; the gate never updates its baseline automatically.

Before the exposure correction, the ordinary-loader control at the two Castle roof views measured 1,937 void, 4,668 depth-gap and zero untextured pixels with experimental routing enabled.
The corrected direct loader matched all three geometry/texture totals exactly, taking 24.1 seconds versus 161.8 seconds for the ordinary scene path.
Grey was 222 versus 220 and pop-in was 8,873 versus 8,852, so this is diagnostic geometry parity rather than an assertion of identical native RGB.

The original sweep disabled authored auto-exposure to hold brightness constant, producing washed-out outdoor reference captures.
The corrected sweep keeps authored exposure bounds and grading, meters each normal view with the recovered native reset argument four, then holds that GPU metering history through its screenshot and all-visible comparison.
The reserved post-control channel is used only by captures; normal interactive exposure behavior is unchanged.
This makes the comparison independent of asynchronous loading time and avoids comparing two independently adapted images.

### Material-routing decision and final six-view numbers

This section records the historical 14 October decision.
Production owning-block routing supersedes it on 15 October after equal-binding/GPU-readiness controls isolate the larger colour difference to correctly textured, already different normal/all-visible neighbour scenery.
See [the routing review](MAP-COMPLETENESS-SWEEP.md#owning-block-texture-routing-review-2026-10-15) for the independent block audit, unchanged geometry and per-view fixture decisions.

The prepared Job D patch was tested after the fog scheduling fix with the same serially exported cache, camera lists, native settings and shader clock.
The first Outskirts camera is at the first idol facing toward the temple, and the remaining five retain cliff, distant Castle and seam coverage.

| Area / routing | Seconds | Void | Depth gap | Untextured | Grey | Pop-in |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Outskirts, production prefix | 33.4 | 0 | 493 | 0 | 402 | 23,807 |
| Outskirts, owning-GLB routing | 39.6 | 0 | 493 | 0 | 400 | 24,344 |
| Castle, production prefix | 34.9 | 1,933 | 4,886 | 903 | 568 | 26,580 |
| Castle, owning-GLB routing | 46.8 | 1,933 | 4,886 | 0 | 618 | 31,688 |

The layer renderer's primary-map texture prefix predates the connected-block viewer, but Job D made that assumption incorrect for borrowed models.
The failed requests include m13 cliff/ground samplers and m17 mountain/rock samplers under a primary m11 texture directory, despite their files existing under the owning exported block.
Owning-GLB routing fixes those requests and removes Castle's 903 untextured pixels without changing the selected collisions, visible-part sets, void or depth-gap totals.
It also changes the colour comparison with forced all-visible neighbour scenery, raising pop-in by 537 pixels in Outskirts and 5,108 in Castle.
This exceeds the per-view gate's tolerance, so the patch is not enabled in production under the requested no-worse-numbers rule.
The opt-in --test-material-routing diagnostic retains the exact experiment for review, while canonical gates reject that flag.
The remaining routing defect is explicitly recorded in FIDELITY.md rather than hidden by a larger pop-in tolerance or a revised visibility mask.
The final default and experimental controls are retained under target/alt-gt/native-off/ and native-on/.
The pre-fix fast-capture untextured and void counts are invalid baselines because opaque fog cards affected both categories; the checked-in fixtures use the corrected fog path.
Remaining positive counts at valid views are regression ceilings and investigation candidates, not proof that legitimate open vistas, authored grey stone or alternate LOD scenery are erroneous.

### Tangent expectation

The earlier handedness failure compared unused index-buffer vertices as though MikkTSpace had computed a tangent for them.
Those unused LOD vertices have no face-derived MikkTSpace result, so that comparison cannot establish an export error.
The test now compares only vertices referenced by triangle indices while preserving the existing handedness and direction thresholds.
On regenerated data, 2,470,531 referenced vertices give 97.1% handedness and 96.0% direction agreement; 69,701 unused vertices are excluded.
The old all-buffer diagnostic is still printed, and no tangent sign, coordinate conversion or winding was changed to satisfy the test.

### Reference coverage and limits

The external reference index supplies 442 positioned captures out of 444 screenshots, including menu/help frames with positions.
The gate adds all 150 distinct position-derived camera poses, sharing one windowless map load, and derived side-by-side images are saved under target/alt-gt/reference/.
Original screenshots remain outside the repository and are read directly for composition.
The practice-tool overlay gives player position, not camera rotation, so these use player feet plus 1.65 m eye height and the nearest authored player-start yaw.
The comparison images label that limitation; their camera direction, story state, exposure and menu contents are not retail camera-matched evidence.
The optional 150-view reference tool is not yet reliable and is excluded from required gates.
It has a separate 900-second budget; the required six-view gates retain the requested two-minute area budget.
The final reference baseline, with native exposure and grading retained, took 418.7 seconds.
Its totals were [183573,158854,305085,195450,1977906] in void/depth-gap/untextured/grey/pop-in order across the 150 distinct poses.
The large untextured total includes Castle-position captures viewed through the Outskirts primary prefix and reflects the routing defect left off in production, not a claim of absent exported DDS files.
No exact camera, story or photometric parity is inferred from these aggregate reference counts.
The 400.7-second repeat failed four tolerances: view 57 pop-in 12136 to 12620, view 61 pop-in 11514 to 12349, view 62 pop-in 6469 to 7340, and view 66 grey 779 to 797.
Void, depth-gap and untextured totals remained identical; repeat totals were [183573,158854,305085,195544,1978983].
Shading variability remains unresolved, and further reference-repeat investigation is deferred.
The required gates consist only of the normal CPU data gate and the two canonical six-view render gates.

### Validation

The final canonical six-view gates passed in 43.5 seconds for Outskirts and 41.2 seconds for Castle.
Their totals were respectively [0,493,0,398,23814] and [1933,4886,903,562,26585] in void/depth-gap/untextured/grey/pop-in order.
The final normal cargo test invocation passed the ten-map data gate in 6.56 seconds with no ignored-test opt-in.
The complete game unit suite passed 107 tests with two existing slow tests ignored; the shader/material/tangent integration binary passed all six tests.
The extractor passed 23 unit and two integration tests.
The focused uploaded-image lifecycle test and the expanded connected data oracle passed after their final changes.
Cargo formatting and Clippy with all targets and warnings denied passed for both touched crates.
All build/check commands used CARGO_TARGET_DIR=target/alt-gt and -j 4, with offline dependencies.
Commands are recorded in MAP-COMPLETENESS-SWEEP.md; detailed control logs and reports remain under target/alt-gt.
No real-game process, install, saves or running-game memory was accessed or modified, and no commit or publication was made.

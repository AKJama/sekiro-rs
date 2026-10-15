# Authored Ashina map effects

The map loader installs MSB region effects, ModelSfxParam attachments and parameter-resolved event-created object effects through the existing FXR player.
The first-visit fixture is the same one documented in MAP-COMPLETENESS.md.
There is no additional MapOffset translation for these native map placements.

## Inventory and event evidence

Run `cargo run --release --offline -p sekiro-sim --example map_fx_audit -j 6 -- cache` to reproduce the complete inventory without a renderer or retail-game session.
Each catalogue row retains the placement name, entity, FXR, mirrored world position, orientation, enabled state, activation part, dummy, ModelSfxParam row and last executed source command.
The report also retains every parameter-resolved FX command in instantiated event bodies, including branches that did not execute, separately from executed writes.
Native FXR trees and raw region records belong in the local audit output, not tracked source.

Outskirts has 157 SFX/WindSFX regions and Castle has 95.
These counts exclude ModelSfxParam and event-created object attachments.
Including parameter attachments, explicit object FX and registered-idol TAE produces 312 placements in Outskirts and 287 in Castle.
Of these, 240 and 223 are story-enabled before residency, distance and activation-parent gates.
The model-parameter contributions are 147 and 181 placements; explicit object FX contribute one and four, and registered-idol TAE contributes seven in each map.
An entity of -1 has no script-addressable entity ID and is still a real placement.
StartDisabled supplies the initial gate; an executed spawn or delete overrides it.
Ordinary object enablement and loaded-map visibility also gate each attachment.

ModelSfxParam rows use the object model number times 1000 plus the MSB model_sfx_param_relative_id.
A negative relative ID disables the automatic parameter path.
Each positive sfxId slot supplies its own dmypolyId, resolved through the object's FLVER dummy and parent-bone chain before the placed transform and X reflection.
For example, o116000 uses row 116000000, FXR 839530 and dummy 200.
RegisterBonfire commands retain their entity, unlock flag, kindling state and event provenance.
Their settled lit/unlit presentation reads the object's own TAE idle animation 11/1 and its time-zero FX events, including the authored FXR and dummy, rather than an invented glow on the idol material.
The animation-selection bridge is an explicit approximation until the native bonfire controller is traced; the FXR IDs and dummy locations themselves come from TAE.
The inventory encodes this provenance with command 200903, distinguishing registration-driven TAE from explicit 2006 object FX commands.
An activation part can be a meshless helper, so effects use its authored residency demand and story enablement instead of requiring that a mesh was rendered.

Native Outskirts event 11105107 deletes 1108010/1108020, waits two seconds and selects ordinary snow or invasion embers using flags 8302/9100, region 1102107 and map membership.
Castle event 11115170 deletes and restores snow entity 1114350, suppressing it for invasion, regions 1112150/1112151 and player SpEffect 110040.
The retained region graph resolves the authored composite indoor volumes.
The rooftop ordinary-fight and Tomoe transitions are described in LIGHTING.md's Genichiro snow audit.

## Runtime and fidelity limits

The following counts are story-enabled/declared placements before camera range and parent-residency gates.
The complete per-placement list, positions, camera-follow classification and event controllers are retained in the local `map-placed-effects.csv` audit beside the reproducible census.

| FXR | Outskirts | Castle |
| ---: | ---: | ---: |
| 60 | 0/1 | 0/0 |
| 1010 | 5/6 | 7/7 |
| 1012 | 1/1 | 0/0 |
| 650642 | 0/0 | 0/1 |
| 800081 | 6/6 | 0/0 |
| 800082 | 4/4 | 6/6 |
| 800140 | 3/3 | 0/0 |
| 800150 | 0/13 | 0/3 |
| 800220 | 0/0 | 43/43 |
| 800240 | 0/0 | 4/4 |
| 800250 | 2/3 | 0/0 |
| 800270 | 1/1 | 4/4 |
| 810101 | 0/11 | 0/4 |
| 810115 | 0/20 | 0/7 |
| 811000 | 6/8 | 0/0 |
| 811020 | 0/0 | 9/9 |
| 811021 | 4/4 | 24/24 |
| 811201 | 0/0 | 0/1 |
| 811210 | 1/1 | 0/0 |
| 811240 | 0/0 | 0/1 |
| 811241 | 0/0 | 0/1 |
| 811242 | 0/0 | 0/1 |
| 811243 | 0/0 | 0/1 |
| 811260 | 0/0 | 13/17 |
| 811280 | 0/0 | 0/1 |
| 811300 | 0/6 | 0/5 |
| 811305 | 0/1 | 0/1 |
| 811310 | 0/1 | 0/12 |
| 811311 | 0/0 | 0/5 |
| 811320 | 0/9 | 0/5 |
| 811321 | 0/0 | 0/3 |
| 811500 | 0/0 | 0/1 |
| 839500 | 2/2 | 1/1 |
| 839510 | 8/8 | 0/0 |
| 839520 | 46/46 | 27/27 |
| 839530 | 50/54 | 37/37 |
| 839531 | 61/62 | 32/39 |
| 839535 | 0/0 | 7/7 |
| 839537 | 0/0 | 3/3 |
| 839538 | 0/0 | 2/2 |
| 839540 | 10/10 | 0/0 |
| 839541 | 3/3 | 3/3 |
| 839542 | 1/1 | 0/0 |
| 839543 | 1/1 | 0/0 |
| 839545 | 1/1 | 0/0 |
| 839547 | 24/24 | 1/1 |
| 839548 | 0/1 | 0/0 |

SEKIRO_NO_MAP_FX disables this module at runtime while retaining the same executable and renderer for before/after controls.
Effects farther than 120 metres sleep and submit no particles or lights; sleeping trees can be evicted when the shared 192-effect budget fills.
The player retains its global particle budget and per-node pool bounds.
Effect meshes remain transparent, omit prepass/shadow rendering and stay off the geometry/collision diagnostic layers.
Effect lights reuse a bounded pool of sixteen entities and the existing scene-to-lumens bridge.

Action 10000 has a bounded CPU implementation using the authored texture, colour, size variation, velocity variation, emission volume, follow factor and duration fields.
The integer emission/duration fields are interpreted on the FXR 30 Hz clock; the native GPU scheduler's exact count semantics remain unverified.
The 2048-particle per-node bound can reduce dense weather emission.
This fallback does not yet reproduce GPU collision, all emitter shapes, random turns, angular motion or every acceleration/growth field.
Camera-follow detection comes from native node movement 46 in the FXR tree, including effect references.

Action 607 uses authored normal-map UV scrolling, size, orientation, intensity and radius with Bevy's transmission image.
Its screen displacement uses the recovered 0.01 scale and linear normal-map samples; its flat billboard still approximates native volumetric distortion shapes.
Action 609 uses authored diffuse colour, multiplier and radius; native effect-light units and specular separation remain open.
The existing soft-depth path only copies matching four-sample depth, so single-sample map captures retain hard intersection edges.
Native model appearances 605 and wind-force action 10300 remain outside this job's sprite/weather implementation; 10300 does not directly draw pixels.
The module is a first-visit presentation snapshot; ongoing quest and boss-phase event execution remains the responsibility of the gameplay event host.
The existing global material, exposure and emissive-fidelity issues are not closed by placing these effects.

The spot-053 reference contains Gyoubu's unlocked boss idol, while the documented Outskirts fixture precedes Gyoubu and disables that idol through common event 20005500 and flag 9301.
That comparison must retain its state discrepancy rather than force an inactive idol or its FX into the first-visit scene.
The enabled temple idol at spot-002 supplies the matched glow check.
Paused-clock captures prime new effects to two seconds, freeze their simulation through normal/all-visible comparison and re-prime camera-relative trees after a camera teleport.
SEKIRO_MAP_FX_REALTIME selects wall-clock stepping for stationary performance measurement on that capture path.

## Map FX implementation and verification

Owned source changes are crates/formats/src/msb.rs; crates/sim/src/map_effects.rs and world_events.rs; the sim module/test registration lines and map_fx_audit example; crates/game/src/map_fx.rs, fxr.rs, fxr_tree.rs and fxr_distort.wgsl; the main module line and single map.rs install call.
The focused tests are the weather movement/follow/expiry case in fxr_tree.rs and the installed map-effect parameter/provenance case in crates/sim/tests/it/map_effects.rs.
Documentation changes are this file, the colour review in MAP-COMPLETENESS-SWEEP.md, FIDELITY.md, CREDITS.md and STATUS.md.
Only two Outskirts grey fixture cells belong to this FX job; the concurrent texture-routing changes and other source files retain their owner's changes.
No game data was regenerated, no reference image was copied, and no commit or publication was made.

The normal release workspace run reports 811 passed and 24 existing ignores, including 126 game unit tests and six game integration tests.
The installed map-effect test and weather movement/follow/expiry regression pass.
The game unit run takes 220.85 seconds and the sim integration run 655.00 seconds under concurrent builds/rendering; these are recorded durations rather than a claim of a fast clean-machine run.
The final dedicated all-map data gate exits zero.
Workspace formatting and release all-target Clippy with denied warnings pass with target/alt-fx and six Cargo jobs.

The final default six-view gates pass in 75.5 seconds for Outskirts and 88.1 seconds for Castle.
Their geometry totals are 0/493/0 and 1933/4885/0 for void/depth-gap/untextured, respectively.
Every per-view geometry measurement, selected collision and sorted visible-part set equals the same-executable runtime-off control.
The colour fixture review, depth quantization exception and repeat results are in MAP-COMPLETENESS-SWEEP.md and target/alt-fx/gates/final-comparison.json.
The frozen capture executable has SHA-256 7aa59510e01da0f3f75dc778f00799db9bc8f7e301d7ae877e1394238f53c476.

The complete 599-row local inventory is cache/map-fx-audit/2026-10-15-alt-fx/map-placed-effects.csv.
Its census JSON, native FXR trees, raw-region census, ModelSfxParam dump and EMEVD/TAE dump evidence remain in that gitignored cache audit directory.
The source example reproduces the inventory; test logs, gate reports and generated progress PNGs remain under target/alt-fx.

## Stationary FX performance

The same frozen executable renders one 1600 by 900 night camera per area, with eight captures separated by 1000 settling frames.
SEKIRO_MAP_FX_REALTIME=1 advances FX with real time while the map shader clock stays fixed.
The before control sets SEKIRO_NO_MAP_FX=1; the after control runs authored effects normally.
Samples exclude the first capture, one second after each preceding readback, the last 0.2 seconds before the next readback and every timestamp before the final data-gate/build completion plus one second.
The table reports medians of one-second smoothed diagnostic snapshots, not per-frame percentiles or an interactive-viewer FPS guarantee.

| Night camera | Frame ms off / on | Opaque GPU ms off / on | Transparent GPU ms off / on | Retained snapshots off / on |
| --- | ---: | ---: | ---: | ---: |
| Outskirts battlefield fire | 10.724 / 16.385 | 3.679 / 3.709 | 0.026 / 0.096 | 50 / 108 |
| Castle courtyard | 3.551 / 7.854 | 1.540 / 1.533 | 0.003 / 0.008 | 16 / 48 |

The CPU fallback and shared FXR tree/property/batch work add substantial frame cost even though measured opaque GPU work stays nearly unchanged.
The live effect/particle/light pools and range sleeping bound this work; a native GPU particle implementation remains a performance and fidelity follow-up.
These camera-specific results do not establish a global frame-time limit or a GPU speedup.
Complete capture metadata, timing windows, sample counts/ranges and logs are under target/alt-fx/performance/ and comparison.json.

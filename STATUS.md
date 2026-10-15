# Status

## Reference story and snow remeasurement (2026-10-15)

Selected the named `genichiro-defeated` reference state from the installed defeat flags, common clock events and the retail overcast sky/travel-menu evidence.
Gyoubu-only remains 06:00, while adding Genichiro flag 9303 raises 9800 and selects 12:00, environment 2, sub-ID 0 and sky m900010 in both Ashina areas.
Known local defeat writes and the completed-cutscene guard 9403 also seed the offline story snapshot; unknown quest and interaction flags remain unset.
Reference matching, exact-camera replay, report validation and canonical render gates now use this state, with guards against silently reusing night captures.
Replayed all 51 previously accepted pairs into the external reference collection's `pairs-genichiro-defeated/`, preserving all camera/player/FOV fields.
Renewed triage accepts 50 pairs at 47 spots and excludes the obstructed spot-046 camera; current leading counts are L1 47, M1 46 and F1 36 spots.
All 430 original pair/pose file fingerprints remain unchanged.

The six requested views plus snow-heavy spots 008, 013, 089 and 090 were measured with the existing scene masks and independently reviewed snow-region polygons.
Snow-region mean linear luminance remains 2.41x to 8.22x retail, with mixed saturation errors and persistent excess brightness across non-snow surfaces.
No tone, exposure, fog or material implementation was changed by this state job.
The measured snapshot is f8e67514, including a refreshed capture after the parallel point-light material edit; later concurrent FX/fog changes are excluded from those measurements.
The separately recorded final gate build 875ffe43 includes the later FX-light colour-packing edit present when built.
Both canonical gates pass in 57.474 and 68.480 seconds after reviewed colour-only baselines, with identical geometry categories, collisions and visible-part sets relative to the progression controls.
Geometry fixture cells, cameras, tolerances and the 120-second budget remain unchanged.

Release workspace tests pass 817 tests with 25 existing ignores, the refreshed material build passes 137 game tests with three existing ignores, and the later FX build passes 11 focused tests.
The final installed-data gate, workspace all-target release Clippy with warnings denied, formatting and six Python checks pass.
An end-to-end CLI check confirms the default matcher rejects an existing night collection before starting a renderer.
All Cargo work uses target/alt-story.
Eight full-resolution real/night/new-state panels, ten snow-mask overlays, all measurements, camera checks, binary/source manifests, renewed triage and gate proofs are in the external progress collection's 2026-10-15/story directory.
Details, remaining ranked L1/M1 candidates and per-view colour notes are in docs/VISUAL-PARITY.md, docs/LIGHTING.md and docs/MAP-COMPLETENESS-SWEEP.md.
No staging, commit, worktree, publication, retail-game session, save edit or game-install write was performed.

## Post-chain colour audit (2026-10-15)

Implemented the native pre-gamma hue/saturation/value/contrast matrix, accepted the authored ToneMap-Contrast name and restored the full authored spatial bloom pyramid.
Exact-night colour correction defaults remain neutral, LUT 20 sampling and output gamma agree with the ordinary native path, and no warmer tint or exposure offset was invented.
Saturation moves closer to retail in five of six matched views; luminance moves closer in 009, 025 and 091 and farther away in 002, 003 and 092.
Blue snow and the remaining brightness mismatch stay open; docs/COLOUR.md records the full numbers, native evidence and unresolved LUT blending, display/quality settings, temporal bloom and exposure limits.
All six sorted visibility records agree, and all 17,217 map/draw-param asset fingerprints are unchanged.
The audited release workspace run passes 816 tests with 25 existing ignores, including the installed-data gate, and release workspace all-target Clippy with warnings denied and formatting pass.
The final Outskirts and Castle gates pass in 54.114 and 58.818 seconds, with unchanged geometry triples totalling 0/493/0 and 1933/4885/0 respectively.
Only reviewed colour fixture cells change, with every view documented in docs/MAP-COMPLETENESS-SWEEP.md and the 120-second limit and tolerances unchanged.
Requested panels, masks, manifests and measurements are in the external progress collection's 2026-10-15/colour directory.
The protected sky/fog files and a late concurrent FXR diagnostic edit are outside this task; that late FXR edit is excluded from this build's validation.
No staging, commit, publication, worktree, retail-game session or game-install write was performed by this task.

## Painted backdrop and fog follow-up (2026-10-15)

Restored the separate m89 FarMountain companion selected with each dome, including night m900130 and its layered panorama, native colour/haze shader, alpha threshold and depth-write flag.
Corrected omitted fog sampling controls to executable defaults of near offset 1, 32 slices and sample offset 0, plus SAT sky/fog-card gamma 2.2 and independent alpha.
Traced spot-011's blue arc to m925009_9000 CloudFog material 7 and corrected the fog cards' substitution of diffuse RGB/intensity for the native independent SpecColor0 input.
The final arc remains conspicuous and the night painted range insufficiently readable, so D1/F1 visual parity remains open; no arbitrary density, colour, height or exposure adjustment was introduced.
Both canonical render gates pass after reviewed colour-only fixture changes, in 45.910 and 59.719 seconds, with identical geometry metrics, selected collisions and visible-part sets across all twelve views.
All 24 before/final diagnostic depth and collision images are pixel-identical.
The independent final repeat differs at one Castle view-4 depth pixel by one channel code, with unchanged geometry categories.
Release workspace tests pass 815 tests with 25 existing ignores; the final touched-crate rerun passes 163 tests with three existing ignores, including the installed-data and sky-companion regressions.
Workspace all-target release Clippy with warnings denied, formatting and diff checks pass using target/alt-backdrop.
Six real/before/after comparisons for spots 011, 022, 091, 009, 045 and 089 are in the external progress collection's 2026-10-15/backdrop directory, with results.json and gate/pixel proofs.
Native research stays in re/research/backdrop-2026-10-15; details and remaining limitations are in docs/BACKDROP.md.
No shared lighting or post source was edited, and the before/final executables embed identical current tone and exposure shaders.
No staging, commit, worktree, publication or retail-game session was performed.

## Exposure and tone curve (2026-10-15)

Corrected mode-7 sensitometric mapping from natural exponentials to the native shader's base-two powers, with an actual GPU regression that fails on the previous production function and passes after the correction.
The RustSekiro inspection at `0c9e5915b3d98d8a1e00319a64513100ff322e5e` found the same curve error, retained by-eye exposure offsets and documented bloom approximations; no implementation was copied.
Authored exposure, metering/adaptation settings, light colours and snow inputs remain unchanged.
The final controlled spot-002 mean linear luminance decreases from 19.70435 to 13.63111 percent white, versus retail 7.02082, a 30.82% decrease with a remaining 1.94x retail ratio.
Luminance is closer in four of six views and farther away in two; saturation and B/R worsen in all six, so L1/M1 brightness and snow-colour parity remain open.
The full luminance/saturation/hue table, native evidence, unknowns and image provenance are in docs/EXPOSURE.md.
All six complete visibility records agree, and SHA-256/size/timestamp fingerprints remain equal across 17,214 map/draw-param files.
An external capture-script stale-marker mistake was corrected with unique batch IDs, and all reported comparisons were repeated; provisional images remain separately archived.
Only colour fixture fields were re-baselined, with every per-view change explained in docs/MAP-COMPLETENESS-SWEEP.md.
All twelve before/final geometry triples agree: Outskirts totals void/depth-gap/untextured 0/493/0 and Castle 1933/4885/0.
The final gates pass in 49.948 and 58.686 seconds with unchanged tolerances and the 120-second limit.
Release workspace tests pass 814 tests with 25 existing ignores, including the installed-data gate, 127 game unit tests and seven game integration tests.
Release workspace all-target Clippy with warnings denied and workspace formatting pass.
All Cargo work uses target/alt-exposure; captures and validation artifacts are in the external progress collection's 2026-10-15/exposure directory.
No staging, commit, remote update, retail game session or game-install write was performed by this task.

## Extra LightSet sun-shadow audit (2026-10-15)

Native deferred shader disassembly applies the sun-shadow colour factor only to slot 0 diffuse/specular; the other three slots receive separate SSAO visibility weights and have no independent shadow sample in this path.
The installed LightSet data has no extra-slot shadow selector, so no Rust/WGSL, packing, fixture or tolerance change was made.
The evidence, all nine retail/before/after metrics for four matched night cameras, and all twelve per-view colour comparisons are in docs/LIGHTING.md.
Fresh captures reuse the preceding current-light images as before; the temple's masked RGB difference is zero, and the remaining three pairs differ by at most 0.08673 display bytes on average, with identical sorted visibility decisions and camera/collision settings.
These repeat differences are not a visual improvement; excessive brightness remains open.
Both six-view gates pass in 45.296/64.342 seconds, with totals `(void,depth_gap,untextured,grey,pop_in)` of `(0,493,0,2356,28846)` and `(1933,4885,0,1702,31552)`.
Formatting, release workspace all-target Clippy with warnings denied, the installed-data gate, 126 game unit tests and six integration tests pass, with two existing unit-test ignores.
All Cargo work uses target/alt-sf and six jobs; rendering is serial and windowless.
Requested before/after temple PNGs, the real/before/after panel and complete local audit records are saved in the external progress collection's dated shadow-fix folder.
Existing status/visual-parity edits are preserved; no commit or publishing operation was made, and cache/reference data remained read-only.

## Verified map-light capture labels (2026-10-15)

All eight earlier light pairs retain the expected mapping: before is legacy (`SEKIRO_MAP_LIGHTS_LEGACY=1`), and after is current (the variable absent).
The stronger-shadow Outskirts example is legacy; both states retain the preceding GI and fog corrections.
Reproducing the original fourth-pose sequence reduces Outskirts before-to-legacy RGB MAD to 0.001913 and after-to-current MAD to 0.000217, versus about 39.253 for the opposite states.
The original frozen capture executable, recorded per-image FOVs, map arguments, night story, resolutions and benchmark warm-up are reused, with one windowless renderer at a time after waiting for the snow audit's process.
The media progress date folder contains `rerender-2026-10-15/` with 16 state-named PNGs, process-environment/commit/binary-hash/time JSON records, eight labelled contact sheets and all per-file pixel comparisons.
The 16 old PNGs are preserved with matching hashes in `superseded-2026-10-15/`, and `lights-README.txt` records the verified mapping.
The six-job release build passes in `target/alt-lightsfix`; no engine source, game install or saves were modified, and no commit was made.

## Snow-tone data audit - visual parity M1 (2026-10-15)

M1 remains open: this session audits the authored snow albedo/facets, night GPARAMs, grading LUTs and environment selection without changing the renderer or inventing replacement colour values.
The snow texture and night lights author a blue contribution; matched grading-disabled and layer-disabled controls retain the cold cast, while exact excess brightness/contrast relative to retail remains unresolved.
The numbers, limitations and all twelve per-view gate comparisons are recorded under M1 in docs/VISUAL-PARITY.md.
Both unchanged canonical six-view gates pass in 72.902 seconds for Outskirts and 80.769 seconds for Castle, with geometry/visibility equal to the preceding FX final controls and no fixture or tolerance edits.
Totals `(void,depth_gap,untextured,grey,pop_in)` are `(0,493,0,2357,28850)` and `(1933,4885,0,1702,31552)` respectively.
The release game suite passes 126 unit tests and six integration tests with two existing ignores, including the data gate and native settled-snow facet regression; formatting and diff checks pass.
Independent matching-camera baseline and audit-repeat PNGs are pixel-identical under target/alt-snow/progress/snow-before.png and snow-after.png, with verified copies in the external progress root's 2026-10-15 directory.
These names do not imply a visual correction; the unsupported generic snow-family, material/lighting composition and post-calibration questions remain explicit.
All Cargo commands use target/alt-snow and six jobs, game rendering is serial and windowless, reference images and cache stay read-only, and no commit or publication occurs.

## Authored map FX - visual parity E1/E2 (2026-10-15)

Outskirts and Castle now place their native MSB SFX, ModelSfxParam dummy attachments, event-created object FX and registered-idol TAE through the existing FXR player in the shared map loader.
The local 599-row inventory lists 312 placements / 240 story-enabled in Outskirts and 287 / 223 in Castle, with positions, camera-relative classification and event provenance.
Native weather action 10000 draws falling flakes, 607 draws normal-map heat distortion and 609 supplies bounded effect lights; effects beyond 120 metres sleep, and meshless activation helpers retain their authored residency gate.
The inventory and native event/TAE/FXR evidence are under cache/map-fx-audit/2026-10-15-alt-fx/, with implementation and limits in docs/MAP-FX.md.
The six requested runtime-off/on PNGs are under target/alt-fx/progress/, with an additional spot-053 review preserving its pre-Gyoubu idol-state mismatch.
Both final six-view gates pass in 75.5/88.1 seconds, preserving every off/on geometry metric, selected collision and sorted visible-part set at totals 0/493/0 and 1933/4885/0.
Only Outskirts views 0 and 3 gain reviewed grey-fixture updates; every colour shift and Castle view 4's single encoded depth-pixel step are documented in docs/MAP-COMPLETENESS-SWEEP.md.
The release workspace run reports 811 passed and 24 existing ignores, including 126 game unit tests and six integration tests; the final dedicated data gate exits zero, and release all-target Clippy with denied warnings plus formatting pass under target/alt-fx with six jobs.
Stationary 1600 by 900 night frame-snapshot medians rise from 10.724 to 16.385 ms at the battlefield fire view and 3.551 to 7.854 ms in the Castle courtyard, with nearly unchanged opaque GPU time.
The bounded CPU particle fallback remains a measurable cost; native GPU scheduling, exact bonfire idle selection, soft-depth and particle-lighting fidelity, unsupported model/wind/spot-light appearances and live quest/arena-phase FX control remain open.
No commit, publication, retail-game run, save access, game-install write or reference-image modification occurred, and map.rs receives only one install line from this job.

## Owning-block neighbour textures (2026-10-15)

Additional map-layer samplers now resolve from their owning GLB and block's existing models/tex directory, preserving the original material cache keys and exporter layout.
The frozen preceding build reproduces 459/429 distinct missing DDS requests for Outskirts/Castle primary; the corrected controls and both final default gates have zero failed texture requests across all six resident blocks.
The capture audit counts neighbour materials and mesh instances even when normally hidden or occluded, rejects failed declared requests, waits for GPU image uploads before every phase and requires equal normal/all-visible material identities and readiness.
Shared-material readiness checks are deduplicated per poll to retain the unchanged 120-second gate budget.
Final default six-view gates pass in 67.5/67.3 seconds with Outskirts void/depth-gap/untextured 0/493/0 and Castle 1933/4885/0; all geometry counts, selected collisions and visible-part sets equal the before controls.
One Castle diagnostic-depth pixel has a one-code repeat rounding difference without changing any geometry metric, and every collision-depth pixel is identical.
GPU-ready and triple-settling controls prove that the larger normal/all colour differences follow corrected rendering of existing alternate neighbour scenery, with no material re-keying or unequal texture readiness.
Only seven reviewed texture/colour fixture cells change, with every view's decision and the unchanged tolerances documented in MAP-COMPLETENESS-SWEEP.md.
The release game suite passes 126 unit and six integration tests with two existing ignores, including the normal data gate; the final focused hidden-neighbour/shared-material audit checks also pass.
Release workspace all-target Clippy with denied warnings and formatting pass under target/alt-tr with six jobs.
Read-only retail-pair review and matched night PNGs are retained as target/alt-tr/progress/routing-outskirts-before.png, routing-outskirts-after.png, routing-castle-before.png and routing-castle-after.png.
Lighting/material tone, excessive blue/bright snow and incomplete distant Castle appearance remain fidelity work, and the existing spatial normal/all-visible differences are not claimed as repaired streaming or retail parity.
Concurrent FX work is preserved; this job makes no map.rs edit and only adds a missing mutable binding needed to compile the concurrent FX mesh update.
The final gate executable is frozen as target/alt-tr/routing-final.exe with its hash in the sweep review; routing-isolation controls disable the concurrent map FX while final default gates include the compiled FX integration.
The shared-plan read/claim could not be completed because the configured GitHub proxy was unavailable.
No commit, remote mutation, retail-game run, save/install write, cache/re change, reference-image write or protected-log write occurred.

2026-10-15: Authored BTL type, ancillary MapOffset, squared diffuse colour, spot rotation/cone and shadow flags now drive clustered point/spot lights for ready map blocks.
Outskirts has 16 base-bank points and Castle 32 points plus two spots; the six-block warm catalogue retains 246 records without activating dormant neighbours.
All four LightSet diffuse/specular slots now contribute with recovered constructor directions, camera-relative transforms and sharpness, missing banks/rows log errors, and render-probe installation is idempotent.
Both six-view gates pass in 40.7/48.8 seconds with every geometry metric, selected collision and visible-part manifest equal to runtime-off controls; manually reviewed colour-only fixtures and all decisions are in docs/MAP-COMPLETENESS-SWEEP.md.
The data gate, 806 workspace tests with 24 existing ignores, final focused packet/shader checks, release all-target Clippy with denied warnings and formatting pass under target/alt-lights with six jobs.
Windowless 1600 x 900 night frame snapshot medians change from 3.801 to 4.845 ms at the Outskirts start and 3.419 to 3.408 ms in the Castle courtyard; opaque GPU costs rise from 1.995 to 3.121 and 0.971 to 1.513 ms.
Matched runtime-off/on night PNGs are in target/alt-lights/progress/; L1 remains open for bright/blue snow and material/tone fidelity, while E1 still needs flames, smoke, idol effects and fixture-emissive verification.
Stock-Bevy attenuation, spot shape, independent BTL specular response, flicker and automatic nonzero-bank selection remain explicit fidelity limits in docs/LIGHTING.md and docs/FIDELITY.md.
Other agents' work is retained, main.rs was not edited by this job, and no commit, publication, real-game run or reference-image modification occurred.

## Authored Ashina grass - visual parity G1 (2026-10-15)

The new grass exporter reads native surfaces, all six MSB channel bindings, GrassTypeParam models and alpha-tested textures, with Halton and world-space hex placement rules recovered through static executable analysis.
Outskirts exports 7,740 clumps on 113 source parts with 11 models and 24 textures; Castle exports 6,813 clumps on 45 source parts with seven models and 18 textures.
The new map_grass module shares mesh/material assets for GPU instancing, inherits terrain-root visibility and keeps authored no-shadow flags after the shared caster budget.
Only these two maps were re-exported with the job's own extractor under target/alt-grass and six jobs; both complete canonical layouts compare equal to their saved baselines.
The two six-view gates pass in 40.7/47.6 seconds, retaining every visible-part set and selected collision, with Outskirts geometry totals unchanged at 0/493/0 and Castle at 1933/4885/903 for void/depth-gap/untextured.
Castle's sole geometry change removes one depth-gap pixel through added grass; a grass-off control restores that pixel with byte-identical collision depth, and no fixtures were edited.
Matched spot-013 and spot-060 before/after PNGs are under target/alt-grass/progress/, with grass rules, all colour totals and the pixel calculation documented in MAP-COMPLETENESS.md.
Release game tests pass 121 unit and six integration tests with two existing ignores, extractor tests pass 25 unit and two integration tests, and the normal data gate, workspace all-target release Clippy with denied warnings and workspace formatting pass.
The game unit run took 212.1 seconds in the concurrent tree, including existing long installed lighting/story checks.
Native seed/BVH grouping, transform jitter, grass LOD, wind waveform/units and near-camera grass handling remain documented approximations; the Castle capture exposes a foreground blade intersecting the camera.
No commit, publication, retail-game run, save access, install write, reference-image write, protected-log write or edit to another agent's assigned source files occurred.

2026-10-15: Ordered spatial GI candidates reduce Hirata/Sunken Valley/Fountainhead opaque GPU samples from 5.85/3.08/15.35 to 3.48/1.93/11.02 ms with matched fallback coverage; the data gate, both six-view gates and measured-before comparisons, 120 game unit tests, six integration tests, workspace Clippy and formatting pass under target/alt-gi with six jobs; D: PNG export is access-denied, and L1 plus the opt-in direct-light compositor remain open; proof is in target/alt-gi/captures/2026-10-15-validated/.

2026-10-15: Native fog enable, integer bank/light routing, finite sky geometry, analytical padding and HDR radiance conversion are corrected locally.
The fog-on/off control disproves a normal/all-visible fog mismatch, and the sweep now rejects unequal fog uniforms between the two colour captures.
The manually reviewed grey/pop-in fixtures preserve all geometry counts, visible-part sets and gate tolerances; both six-view gates pass in 37.4/42.4 seconds under target/alt-fog/final/render-gate/.
The data gate, 112 game unit tests, six integration tests, release all-target Clippy with warnings denied and workspace formatting pass with target/alt-fog and six Cargo jobs.
Matched runtime fog-off/on night PNGs cover the temple idol, Castle courtyard and Outskirts cliff in the external progress collection; remaining retail lighting, material, sky and snowfall differences are documented in docs/LIGHTING.md and the per-metric fixture review in docs/MAP-COMPLETENESS-SWEEP.md.
No commit, game-install/save access or reference-image modification occurred.

## Map gates and Outskirts regression (2026-10-14)

Added a normal CPU data gate for all ten maps, numeric fixtures, twenty independent raw-MSB visibility cases including connected neighbours, and strict windowless six-view render gates for both Ashina areas.
The final normal cargo test invocation passes the connected data gate in 6.56 seconds; final render gates pass in 43.5 seconds for Outskirts and 41.2 seconds for Castle.
Both Ashina maps were exported once each, serially, with zero model failures and unchanged canonical placement/mask/LOD inventories; eight older layouts received metadata-only refreshes.
The historical Outskirts depth/void regression reproduces exactly on h900001's zero-demand helper plane, including a camera roughly 50 m below visible terrain.
The old selector drew all scenery for empty demand; the confirmed exe selector correctly draws none, so ordinary surveys now reject those poses while a raw-collision control preserves reproduction.
Fixed nested-scene readiness accounting, cached repeated material/fog metadata parsing, added fast static GLTF capture instancing with conservative coverage, and ordered instantiation before authored fog conversion.
Capture exposure now meters with the native reset argument and holds the same GPU history through normal/all-visible screenshots, preserving authored bounds and grading.
The tangent test excludes index-unreferenced vertices while retaining its original thresholds; regenerated referenced data gives 97.1% handedness and 96.0% direction agreement.
The material-routing experiment removes Castle's 903 untextured pixels but raises pop-in beyond tolerance, so production routing stays off and --test-material-routing retains the reviewed control.
The game unit suite passed 107 tests with two existing slow tests ignored, shader/material/tangent integration passed six, and the extractor passed 23 unit plus two integration tests; formatting and all-target Clippy with denied warnings pass.
Reference coverage includes 442 positioned screenshots mapped to 150 distinct camera poses, with derived comparisons under target/alt-gt/reference/ and explicit camera/story limitations.
The optional 150-pose reference tool is not yet reliable and is excluded from required gates; its repeat failed three pop-in tolerances and one grey tolerance, with shading variability left open.
The final reference baseline took 418.7 seconds; the four repeat failures and deferred investigation are documented in MAP-COMPLETENESS.md.
No real-game, save or install access, live-memory work, commit or publication occurred.

## Reference index follow-up (2026-10-14)

Added a post-processing command and Windows PowerShell OCR helper under tools/reference_index for per-shot HUD evidence, consecutive same-position spot grouping and an independent Windows.Media.Ocr overlay read.
The spot rule uses a 30-second maximum gap and 2 mm coordinate tolerance; read-only analysis of the current index yields 149 spots, including 96 multi-shot spots covering 389 captures.
The HUD pixel heuristic labels 16 screenshots visible and retains its red vitality and orange posture/boss bar evidence counts per shot.
Windows OCR disagreements are appended to review.json without changing either reading.
The current turn grants repository writes only, so the external index.json, spots.json and review.json have not yet been updated with these new fields, and the Windows OCR pass remains to be run where the reference output folder is writable.
Python compilation, PowerShell syntax parsing, workspace formatting and scoped whitespace checks pass; map code and cache data were not touched.

## Real-game reference screenshot index (2026-10-14)

Added the standalone Pillow/NumPy reader and spatial/contact-sheet tool in tools/reference_index, with runtime calibration and review inputs kept outside tracked source.
The external reference index covers 444 original screenshots: 439 ordinary views and five menu/help views, with 442 positions assigned to the exported Outskirts and Castle blocks.
The overlay agrees with source placed coordinates across both blocks; conversion to Bevy mirrors X without adding MapOffset again.
Twenty-four held-out visual checks pass for both coordinates and IGT, and nine rejected automatic readings have explicit manual transcriptions, including one coordinate correction.
The external report records idol-distance evidence, alternative-offset comparisons and both travel menus' eleven visible unlocked destinations, including Castle Tower Lookout.
Area contact sheets, capture-order overviews and a collision-outline map retain the global shot numbers; the two menus without positions remain unpositioned.
All 442 positions have matching tolerant collision support, and 422 have strict downward floor results; the remaining 20 slightly penetrate the nearby exported floor and retain null strict heights.
Python compilation, output invariants, held-out checks, workspace formatting and scoped whitespace checks pass; no Rust crate was edited or rebuilt.
No commits, pushes, real-game runs, source screenshot changes, game/save writes, map-code edits or cache/map writes were made.


## Authored map fog surfaces (2026-10-14)

Outskirts uses 17 fog material entries across five models and Castle uses three entries on one model, all from the native GroundFog and CloudFog shader families.
The exporter retains their geometry, face culling, UVs, coverage/occlusion streams, all mask textures and MTD constants, preserving the preceding LOD work.
The independent map_fog_surface material ports scrolling masks, GroundFog's static branch/parallax, lit diffuse/grazing terms and native soft-depth factors, with shared GI and default environment-probe fallback.
Both maps were re-exported with target/alt-fs and four Rayon workers, yielding 716/768 GLBs and 4487/3624 legacy pieces with canonical parts and connections unchanged.
Touched-crate release tests pass 131 tests with two existing ignores; all-target release Clippy with warnings denied and formatting pass using four build jobs.
The shader regression includes environment-probe and single-sample/MSAA depth compositions, and inspected GPU captures and the complete parameter/part inventory are under target/alt-fs.
Matched Outskirts and connected Castle surface diagnostics show the recovered bands and cloud sea; the current global pass obscures much of the Outskirts difference.
Both one-view primary-map sweeps report void 0, but their depth diagnostic omits the custom fog material and cannot measure this fix or establish area completeness.
The connected diagnostic without probe/global fog also exposes a dark foreground card; native forward fog, instance colour/SH overrides, shadow permutations and shared per-mesh render-group semantics remain fidelity work.
Coverage and identity-transform audits pass, and a final probe-lit Castle control exercises the added environment fallback without shader or asset errors.
Only the fog job's implementation, its two existing verification fixtures, the requested registration/mod lines and related documentation were edited; paused fog, visibility, LOD and world files were not edited.
No commits, remote mutations, real-game sessions, save writes, install writes or protected log-directory writes were made.

## Connected Ashina blocks - Job D (2026-10-14)

Outskirts/Castle Explore now warms their bounded six-block neighbourhood, matches signed retail wildcard fields, unions multiple target masks and retains independent collision/request state per block without adding MapOffset again.
Canonical Objects load by default with declared helper/physics-only reasons skipped, and C's snapshots, E's root visibility writer and F's per-instance detail selector are integrated.
Other entry maps and arena bootstraps stay on their previous path; Explore physics includes the bounded neighbouring geometry.
Installed-data tests prove all five directed requests, with a grounded synthetic crossing checking seam unions and stale-mask removal.
The connected workload needs a 64 MiB IO stack, and map captures now wait for rendered settling and a stable empty pipeline queue; the inspected Castle room capture has walls, floor and neighbouring exterior geometry.
All 106 game release tests pass with two existing ignores, all-target release Clippy with warnings denied passes, and workspace formatting passes under target/alt-md with offline final checks and four jobs.
Matched two-view diagnostic controls improve Castle depth-gap/void candidates from 8360/4608 to 809/0, while below-landscape Outskirts candidates worsen from 14659/0 to 18018/9216; reports and pose-identity checks are under target/alt-md.
These incomplete samples leave visual acceptance open, and normal/all differences rise in both maps.
The prepared neighbouring-material routing call is outside D's owned files and awaits its owner or the requested integration approval; current native-layer comparisons are therefore not certified.
Dominant LOD submission, support/tie-band occupancy and retained warm retirement are documented approximations, with live predicates/poses, weighted coverage and exact retail release gates still open.
No commits, protected log writes, cache/re changes, retail game sessions, save access or install writes were made.


## First-visit story state - Job C (2026-10-14)

Outskirts and Castle now run parameter-resolved map/common constructors into typed entity/group visibility and per-part lists for the shared world loader and single visibility writer.
The documented fixture keeps prologue flag 11120100 on, gives Castle Gyoubu flag 9301, and leaves invasion/endgame flags off; common initialization supplies default quest flags.
The installed-script replay proves the normal Castle floor, hidden burned floor, invasion gates and completed Object restore; the earlier common-helper truth table and floor names were corrected from the actual instructions.
Settled ordinary Object gates are 1235 enabled / 274 disabled in Outskirts and 1530 / 144 in Castle, with unsupported world observations and omitted non-presentation actions retained as diagnostics.
Other maps receive no story override, and the existing arena interpreter remains unchanged.
All 518 sim release tests pass with 20 ignored, all six game integration tests pass, sim/game all-target release Clippy with warnings denied passes, and touched-crate formatting passes under target/alt-mc with four jobs.
The final workspace formatting recheck caught in-progress extractor fog additions outside job C's files, which were left to their owner.
The full game unit run has 99 passes, two ignores and the existing snow assertion failure caused by changed exported material indices; the expected variant is present in the new GLB.
Paired two-view diagnostic sweeps completed for both maps, with layers/shadows/GIIV disabled to bypass the connected texture-owner routing issue; their increased and decreased candidate counts do not prove area completeness.
Reports, images, first-visit snapshots and the paired summary are under target/alt-mc; docs/MAP-COMPLETENESS.md records the interfaces, exact controls, counts and remaining live-world/rendering limits.
No commits, cache/re modifications, protected log writes, game sessions, save access or install writes were made.

## Real map detail representations - Job F (2026-10-14)

All eight affected Outskirts models now retain 59 original LOD0 primitives and 93 separate LOD1/2 meshes for 154 MapPiece instances, with original face flags, mesh/node/material provenance and face-specific culling.
Lower-level vertex compaction preserves all streams and triangle values, restoring the sampled MikkTSpace check to 95.1% handedness and 93.3% direction agreement; legacy material ordering/reuse also keeps the native roof-snow check passing.
The new `map_lod.rs` interface loads LodParam/LodPlatform, binds signed-negative MapPiece IDs including 255 to row 0, measures camera-to-oriented-bounds distance and returns strict hysteresis and time/distance crossfade weights.
Both maps were re-exported using the release extractor in target/alt-mf, with four Rayon workers and unchanged job B per-kind counts, canonical identities/transforms/masks/LOD IDs, 6,489/6,164 parts, 99/298 connections and 712/767 model GLBs.
The final three-crate release run passes 213 tests and one doctest with four existing ignores; all-target Clippy with warnings denied and workspace formatting pass, using four build jobs.
The focused exporter module verifies every retained face set in all eight models, stream remapping, level isolation, material-prefix reuse and culling variants; canonical cache checks, tangent and roof-snow regressions all pass.
Matched-camera before/after captures and audits are under target/alt-mf, reflecting the concurrent C/D/E/F integration rather than an isolated Job F visual effect.
The first valid Outskirts sweep view changes from void 18,432 and score 38,494 to void 0 and score 20,069, with depth gap unchanged at 20,062; this one-view smoke is not an area-completeness result.
The earlier larger sweep failed collision-depth validation and subsequent baseline smokes stalled; the final primary-only after smoke completed, and both after-captures were inspected.
Still open: the renderer's dominant-weight submission needs actual dual-representation crossfade, its coverage shader and extra type/world threshold scales remain untraced, Object ID-255 defaults need model provenance, and connected-block layer texture routing remains with the material/loader owners.
Fog surfaces remain filtered at pieces::skip_material's fog branch for the later surface-handling job; other map caches were not re-exported.
The preceding Job E tangent failure note describes the earlier cache and is resolved by this exporter session.
No commits, remote changes, real-game sessions, save writes, install writes or writes to protected log directories were made.

## One map-root visibility decision - Job E (2026-10-14)

`map_visibility.rs` implements the executable's per-bit draw/backread selection, condition-byte preprocessing, Collision backread union, MapPiece-only zero fallback and additive connected-block target masks.
The central writer combines optional Job C story state, arena event snapshots, residency, readiness and draw masks; arena staging stops directly writing managed roots.
Outskirts has one effective-zero MapPiece, `m902000_9002`, expanded to all bits; Castle has none, while zero Object and Collision masks receive no invented universal fallback.
All eight E regressions and all 100 game release unit tests pass, with two existing ignores; five game integration tests pass and the shared exported-tangent test fails at 0.695 handedness agreement and 0.619 direction agreement.
Game all-target release Clippy with warnings denied and workspace formatting pass using target/alt-me and four build jobs.
Four-view baseline scores are 62,172 for Outskirts and 307,586 for Castle; connected sweep and fixed-camera after attempts are blocked by asset paths/readiness, with evidence under target/alt-me and no area-completeness claim.
The live Castle part diagnostic exposes h018700 and its h018700_1000 contribution to the target block, source masks and independent readiness/residency gates.
Job D owns final occupancy/navigation and loader behavior; tangent/export and per-block texture routing remain outside E's files, and the integration interface and blockers are documented in docs/MAP-COMPLETENESS.md.
No commits, remote changes, original-game sessions, save writes or install writes were made.

## Lossless map layout export - Job B (2026-10-14)

Layout version 2 keeps every placed part, original model table, mask, condition, entity/group ID, LOD byte and reader setting, with explicit asset/render reasons and resolved Object/Enemy references.
Connections retain raw signed collision indices, attached source collision and inventory-matched target blocks, including the authored unbound -1 row.
The existing viewer projections and field values remain compatible; new world consumers use the complete parts/connections tables documented in docs/MAP-COMPLETENESS.md.
Re-exported Outskirts has 6,489 parts and 99 connections; Castle has 6,164 parts and 298 connections, with every per-kind count matching target/alt-ma/msb-audit.txt.
All 20 extractor release tests, extractor all-target Clippy with warnings denied and workspace formatting pass using target/alt-mb and four jobs.
Game rendering, streaming, event visibility and LOD selection remain with jobs C/D/E/F; job G's material-audit wiring is preserved.
No commits, game sessions, save writes or install writes were made.

## Runtime cache discovery (2026-10-14)

The simulation default and game CLI now share runtime cache discovery: `--cache`, `SEKIRO_CACHE`, the nearest executable-directory ancestor containing `cache`, then the working directory's `cache`.
The only non-test cache/log/config `CARGO_MANIFEST_DIR` use was the enemy default; existing test paths were preserved, and menu changes remained with their owner.
The release executable passes Ashina Castle capture loads from an external temporary folder with `--cache` and from the repository without it, plus executable-ancestor and environment-override launches.
A relocated Owl session reaches 30 idle ticks and exits successfully with memory disabled; logs, rendered captures and the verification script are under `target/alt-cp`.
Release tests for sim and game pass 605 tests with 22 ignored, all-target release Clippy passes with warnings denied, and workspace formatting passes.
All Cargo builds and checks use `target/alt-cp` and four jobs; no commits were made.

## Map material/model provenance - Job G extractor (2026-10-14)

Exact texture names now resolve across cached area, map, object, character and shared banks, with per-material slot/source/error records and all ten map inventories under cache/maps.
Outskirts missing texture names fall from seven to three, Castle remains at zero, and shared albedo/normal images use separate DDS payloads and GLB references.
All 56 Outskirts and 61 Castle omitted Object instances are authored helpers: 56 and 60 meshless FLVER instances respectively, plus one Castle physics-only binder, with no visible-model export failures in those sets.
Both maps were re-exported from target/alt-mg; 18 release unit tests and two integration tests, all-target Clippy and formatting pass with four build jobs.
The three unresolved Outskirts texture paths and runtime helper/proxy handling remain open; game code, pieces.rs, protected data and saves were not changed by this job, and no commit was made.

## MSBS metadata completeness - Job A (2026-10-14)

The formats reader now preserves named visibility masks and conditions, Object references, complete placed-part flags/settings and MapOffset provenance while retaining the existing caller fields.
Independent raw-table checks cover all ten cached maps: 55,352 parts, 582 connections (580 resolved and two explicit -1 references), 1,973 Object references, 1,723 Enemy collision references, 160 PatrolInfo references and seven MapOffsets.
Castle retains all 298 connections; the old h004490_1000-to-h900900 audit example was a negative-indexing error and is corrected in docs/MAP-COMPLETENESS.md.
The release formats suite passes 84 tests with two existing slow tests ignored; formats/all-target Clippy with warnings denied, workspace compilation and formatting pass using target/alt-ma with four build jobs.
The remaining jobs own game integration and retail loading/condition semantics; cache/re data and game files were untouched, and no commit was made.

## Genichiro rooftop snow audit (2026-10-14)

Native Castle m11_01 data supports settled snow around pavilion edges and ordinary camera-relative snowfall FXR 839500, removed during the Tomoe storm handover.
Genichiro correctly selects morning environment 1 and base GPARAM; native snow facets stay enabled on edge surfaces while tatami excludes them, so no snow-selection special case was added.
Added a native-versus-exported material and event-selection regression in m11_01 and evidence in docs/LIGHTING.md, with inspected ordinary and no-layers/no-FXR controls under target/alt-gs/captures.
All 90 game release tests pass with two existing ignores; game all-target release Clippy with warnings denied and final workspace formatting pass using target/alt-gs and four jobs.
Native weather action 10000 remains unsupported, and exact snow brightness/coverage/density awaits matched original-game reference frames.
No commits, collaboration-board edits, game/save sessions or writes to cache, re or protected log directories were made.

2026-10-14: Hirata's two memory selections, collision rows and ArenaSpec flags are audited, with captures and uncapped frame times under target/alt-m10/captures/; m10, core selection and collision tests plus game all-target Clippy and formatting pass, the broad run's separate Senpou sky assertion is recorded, and shared fog/local-light/material/performance gaps still prevent claiming exact visual parity or steady pacing.

## Coach-selected break punishes (2026-10-13)

Wolf's ordinary break window comes from a050_190100's 1.3-second TAE execution cancel.
The coach receives complete near/mid/far punish facts and returns ordered preferences; otherwise neutral Thompson sampling uses recorded punish outcomes, with fastest-reachable selection only when nothing is viable.
Memory v6 preserves valid-fight punish uses, landed hits, window damage and kill conversions; legacy profiles migrate without invented evidence.
Real Genichiro/Owl fixtures, all sim brain and coach tests, brain-net tests, workspace/all-target Clippy with warnings denied and formatting pass using target/alt-po and six jobs.

## Paired placement at walls and ledges (2026-10-13)

Reproduced castle-top deathblows crossing collision and falling onto lower roofs; paired clearance now checks the real relocation sweep, stable supporting floors and actual deathblow, grab and throw-recovery movement.
Unsafe cramped-space displacement keeps the last safe feet, while ordinary movement and knockback can still leave the arena.
All six placement regressions pass, including four installed-map replays in the slow tier, along with the targeted deathblow, combat, arena, Owl grab, perilous counter and Genichiro filters.
Workspace/all-target Clippy with warnings denied and the formatting check pass using `target/alt-db` and six build jobs.
Per-tick before/after position traces and the verified 4.17-second castle entrance capture are under `target/alt-db/captures/` (`castle-door.mp4`, also available as GIF).

## Brain Task A: reachable attacks and break conversion (2026-10-13)

Task A is complete: cached data-derived move/combo sheets gate brain-owned attacks, choose the fastest reachable break punish, expose posture-timed combos, penalise idle/no-threat guarding and report first-hit hit/miss distances.
The controlled Genichiro A/B reduces true whiffs from 6/18 (33.3%) to 3/14 (21.4%); the Owl converts 1/1 breaks on both sides, with delay reduced from 0.917 s to 0.450 s.
Old bow whiff rates cannot be corrected exactly from saved aggregates; new projectiles retain their firing move, and utility animations cannot whiff.
All seven required release filters, all 42 brain-net tests, workspace/all-target Clippy with warnings denied and formatting pass using target/alt-ta and six jobs offline.
GPU and shared simulation changes are preserved, with two behavior-preserving lighting lint adjustments; no commits, real coach calls or writes to protected data/log directories.
See `docs/BRAIN.md` for sheet examples, prediction limits and the player's manual checks; Task A is marked done in `docs/BRAIN-ROADMAP.md`.

## Decisions coach context and idle guards (2026-10-12)

Decisions checkpoint input now includes up to 768 serialized bytes of coach notes, active-set intent, principles, newest lessons and move confidence, with a documented trimming order.
The offline size fixture measures 1968 bytes for the full body with four maximum-length set descriptions.
Idle guards now produce failure evidence for their plan/set and both samplers; they never reset the attack clock, and the six-second deadline requests an allowed attack instead of trusting Lua to attack.
The real-data scripted Genichiro regression fails with the old harmless-guard scoring and passes with the fix.
All seven required release filters and all 42 brain-net tests pass, as do workspace/all-target Clippy with warnings denied and the formatting check.
All builds used target/alt-dc and six jobs; verification used offline fakes, with no commits, git-state changes or writes to the protected data/log directories.

Last updated 13 October 2026.
Read this first in a new session, then `AGENTS.md` and `docs/DESIGN.md`.
How much of the game is done overall: `docs/PROGRESS.md`.

## Mibu event regression fix (2026-10-12)

Reproduced `mibu_bridge::arena_d_mibu_bridge` on installed data: event 11505810 stalled at instruction 11, despite Wolf being about 9.5 m from the boss inside its authored 20 m radius.
Since b082d8a, EMEVD 3[3] became a blocking condition, but only the screen-monkeys host could evaluate it; Mibu therefore waited on false rather than reaching boss bar 905000.
The staged flag 11505801 was unchanged, and the Demon arrival flag removal remained scoped to the Demon.
Mapped duel characters now answer the radius condition, with a shared decoder reading the state at byte 1 rather than the entity count at byte 16; screen monkeys use the same decoder.
SetNetworkUpdateRate 2004[34] is a documented offline no-op, with inside/outside wait regression coverage and a stronger Mibu event-completion assertion.
The focused installed-data replay shows `bar Some(905000); skipped []`.
One full `CARGO_TARGET_DIR=target/alt cargo test --release --workspace -j 12` run passes 696 tests with zero failures and 18 existing ignored tests, including 305 simulation integration passes in 145.24 seconds.
Workspace/all-target Clippy with 12 jobs and warnings denied passes, and `cargo fmt --all --check` passes.
Cargo ran offline; no commits, git-state changes, network access, or model/API calls were made.

## Contextual move fighter and coach review (2026-10-12)

Added a free contextual Thompson move fighter at ordinary and break-response plan slots, with a two-attack default, confidence/lock/exploration controls, actual contact-time conversion rewards, damage-rate rewards and faster-win credit.
Player memory version 5 retains contextual posteriors with 0.98 fight decay, shared transfer, bounded lessons and measured hypotheses; both loaders migrate old profiles.
The coach receives a lifetime arsenal table and ten-fight digest within 8192 bytes, and the same coach enters cross-fight REVIEW MODE every five fights or on an eight-fight plateau.
The 64-seed offline replay keeps combined Mikiri/deflect use below 5% from fight four (2.34%, 2.08%, 2.31%), raises the quick punisher to 94.47%, retains probes and switches family at 7.8% of boundaries.
The previous trained persistent sampler ends at 0.81% bad moves and 49.59% quick punisher; the old streak baseline ends at 36.95% and 31.36% respectively.
Brain library validation passes 93 tests, brain-net passes 42, and game build plus library/binary Clippy pass.
Requested integration filters and workspace/all-target Clippy are blocked by the explicit old GamePlan literal at crates/sim/tests/it/coach.rs:242, outside the permitted paths; its minimal default-field update awaits permission.
All Cargo work used target/alt-fb, three jobs and offline mode; logs, cache, research files and git state were not written.
See docs/BRAIN.md for controls, replay numbers, prompt audit, budget policy and the A/B flag `--brain-moves off`.

## Brain attack learning (2026-10-12)

Genichiro's initial forty-fight evidence shows 29 Mikiri counters in 60 thrust exchanges and 115 whiffs in 119 uses of callback animation 3076, while the checkpoint bandit already holds about 162 discounted updates.
Added recent phase-specific attack evidence shared by ordinary and break-response sampling, Lua counter callbacks and local checkpoint quality, with a three-use safety minimum, stronger counter/punishment failures, actual posture progress, bounded player transfer, decay and exploration.
Removed automatic outcome bans and made mature recent rates available to coach judgement, with structured preferences and matching checkpoint hints executing its notes.
Player memory version 4 migrates through both direct and layered loaders, preserves old raw counts and source backups, and adds per-fight action reporting.
The offline always-Mikiri experiment reduces thrust use to 4.38% with persistent sampling and 1.25% with local checkpoints by fight two, while retaining subsequent probes.
The filtered release run passes 150 tests with one existing slow test ignored; the final migration, retention and brain unit rerun passes all 80 tests, and final workspace/all-target Clippy passes with warnings denied.
All builds and checks used target/alt-fk and three Cargo jobs, with offline fakes or headless duels; existing logs, cache/re, game saves and git state were left untouched.
The local collaboration board was read without changing it under this session's no-git/no-network instruction.
Exact findings, benchmark limits, test evidence and manual follow-up are in `docs/BRAIN-LEARNING.md`, with the design updated in `docs/BRAIN.md`.

## Group-s Demon of Hatred fight pass (2026-10-12)

The Demon of Hatred (c7020:70200000, entity 1100900) is fightable at `--arena demon` in all three phases on the installed data and the map's own Lua AI.
Wolf's arrival at region 1102910 now starts the fight (animation frozen until then, three fog walls, bar 907021), and Wolf's death re-freezes it.
The first and second marks play the kneel, refill HP from the clip's SpEffect 300610, run the Lua's heat-up roar 20001 (immune to HP and posture damage), swap its phase SpEffect and raise 3702005 at one bar.
The final mark plays ThrowParam 17020180/17020181, the map's Event20200 and Wolf's Event710207, and defeat releases the walls.
New shared mechanisms are SpEffect categories (Demon only), endless TAE effects (Demon only), defender SpEffect HP cut rates, a throw-pair re-grounding step, and a Demon-only body radius and `GetDist` offset.
Eleven fast tests pass and three slow ones (every attack lands, every projectile row flies, the three-phase soak) pass when run with `--ignored`; Owl, Genichiro, brain, attack-power, posture, perilous and the other boss regression filters pass, and `arena_d_mibu_bridge` fails on its own boss-bar event (a different group's work).
Missing: TAE 705 FacingAngleCorrection (act 3026 never lands), the first-visit introduction, camera sub-IDs, FXR effects and sound, and the exact original `GetDist` and category rules; see `docs/BOSSES-s.md`.

## Group-f Hirata Father combat (2026-10-12)

Story Father and Inner Father now host their native c5060 Lua/HKS and a separate c5070 bird, including event-driven appearance, vanish, return and finishing execution.
Fixed the shallow-floor waterside reference that cancelled Wolf's sword actions, resumed the bird's animation on enable, retained each variant's real Lua under the scripted CLI option and corrected both HUD names.
The controlled two-mark replay plays the real get-up, restores 2552 HP, observes native phase-two contacts and reaches defeat flags 9317 and 9346 in 18.99 seconds.
Both variants pass native arena-combat and Mikiri checks, all six Owl/Genichiro, tactics, attack-power, posture-recovery and swimming regression filters pass, and the final release game build succeeds.
Both arena launch forms exit zero for both encounters, with reviewed first-phase captures under target/alt-f.
Neither encounter is fully DONE: full attack/fire-counter coverage, native dummy-220 hit masks, collision-perfect finishing placement, continuous unassisted victory and graphical phase-two verification remain open.
Workspace Clippy fails twice on the concurrent fight.rs hp_damage argument-count lint, and workspace formatting still reports shared differences; those files and ordering are left untouched.
Exact commands, per-phase evidence, fixture limitations, screenshots and changed files are in `docs/BOSSES-f.md`.

## Group-c Sword Saint combat continuation (2026-10-12)

The continuous silvergrass replay runs final Tomoe's own Lua and real deathblow into all three story Isshin phases, both native get-ups and the separate Mortal Blade execution, reaching map defeat flag 9312.
The second placed Isshin now has a distinct `silvergrass-inner-isshin` arena, think/event identity, native Inner Isshin bar name and a three-mark replay reaching the original rematch flag 9347.
Fixed c5400's lost a100 throw/get-up group and reconstructed the queued kneeling request and HKS death ownership, preserving authored messages 20/30 and a six-second final-prompt delay without synthetic HP or victory changes.
Four installed-data tests pass, covering the full story/rematch fights, both Mikiri/sweep-evasion sets, pistol damage and airborne lightning reversal; the story fight passed alone in 8.41 seconds.
All required regression filters plus Isshin Ashina pass together: 102 passed and one existing slow test ignored.
Workspace/all-target Clippy and the release game build pass after other agents repair their transient shared visibility errors; inspected arena captures show the real models, named bars and Lua contacts.
Complete move/branch coverage, sweep kick-off, full graphical finishing verification and original-game comparison remain open, so no phase is claimed fully DONE.
Exact commands, per-phase evidence, changed files, screenshots and limits are in `docs/BOSSES-c.md`; cache/re, original saves, existing logs and git state remain untouched.

## Group-d sanctuary encounter continuation (2026-10-12)

The Old Dragons opening now runs all twenty-five fighters' real Lua/HKS against a shared Wolf, applies the authored 4% controller damage, hosts revivals and black conversion, and reaches the native Divine Dragon hand-over from sword inputs at the normal arena start.
The complete first-phase replay records 113 outgoing contacts, two incoming contacts and 23 revivals, with no enemy stats, placements, AI or event-flag edits.
Added event bitfields, event-request interrupts, MSB group routing, attached tree props and posed-eye selection of the Dragon's native 15200090 finishing pair.
The isolated finishing fixture plays both native clips and reaches defeat flag 9310; all five phase-two trees now perform their own rise and hold through Lua/event commands.
The encounter is not DONE: the opening tree-area execution, exact native revival scheduling and full poison coverage remain open, and phase two is blocked by the authored tree wire-target activation, with no grapple, lightning charge or reversal in the sixty-second replay.
Four Dragon tests and 100 Owl/Genichiro, tactics, attack-power and posture-recovery regressions pass, with slow encounter fixtures ignored by default.
All four strengthened Dragon fixtures pass in 120.29 seconds, and the release game build passes.
Workspace Clippy passed earlier, but its final retries encounter a concurrent `fight.rs:921` too-many-arguments lint, left untouched under the shared-tree rule.
Both sanctuary CLI forms exit successfully with reviewed animated combat captures, the named Old Dragons bar and real incoming/outgoing contacts after fixing the source-verified meshless helper and hidden-controller HUD.
The captures still expose washed-out arena presentation and cloth restarts, and do not certify complete visual fidelity.
The completed Monk code was left untouched, and `docs/BOSSES-d.md` contains the concrete limits, changed files and commands.

## Group-b ape continuation (2026-10-12)

Guardian Ape's separate headless continuation and terror now run from the installed throw, TAE, bullet and status rows, with guard/deflect, dung-shot payload and sweep-counter checks.
The burrow's brown ape now has independent Lua/HKS combat, live map-assigned TARGET_EVENT state, its own animated transform and vitality/posture/deathblow panel.
The installed-data replay spends the main ape's first mark, lets Lua reach the real summon region, records both apes' contacts, kills the brown ape with its paired throw, then reaches the main ape's finishing message, flag 9307 and both fog removals.
Fixed shallow cave water incorrectly trapping Wolf in swim transitions; the depth threshold uses existing step height and remains an explicit native-fidelity approximation.
Neither ape is declared DONE because full attack/counter verification, selectable prosthetics and optional Mortal Blade severance remain incomplete.
Father and Inner Father remain untouched in the requested order; see `docs/BOSSES-b.md` for precise implemented phases, controlled fixtures and blockers.
All seven ape tests pass including the slow two-ape replay, and the requested Owl/Genichiro and supporting regressions, workspace Clippy and release game build pass.
Both arena launch forms run, but graphical captures expose heavy staged haze and cave-camera obstruction and do not establish complete visual phase fidelity.
New files pass direct rustfmt checks; workspace fmt --check reports concurrent shared formatting differences, left untouched.

## Group-c Shura combat checkpoint (2026-10-12)

Emma's six release tests verify installed Lua attacks on the roof, guard/deflect responses, both Mikiri thrusts, sweep evasion, the grab and her real deathblow-to-Isshin map hand-over.
Isshin Ashina uses his data-selected c5400 donor; three tests verify both marks, get-up HP restoration, final map defeat, Mikiri, sweep evasion, the grab and damaging event-driven floor fire at 33 authored points.
The finishing command now waits for the throw's TAE defeat-message window, and boss AI receives MSB region membership and Wolf's actual HP for its single-target last-target query.
Two fire-source references remain unresolved in the original MSB, native target-switch history and request hand-off remain inferred, and no graphical or original-game comparison is claimed.
All requested Owl, Genichiro, brain-tactics, attack-power and posture-recovery regression filters pass; the final workspace Clippy and game-build attempts hit another agent's ape HUD privacy errors, which were left untouched.
The five new Rust files pass direct rustfmt checks, with shared module ordering preserved.
Sword Saint's four-phase combat bring-up and rematch row remain unstarted; exact commands, evidence, changed files and limits are in `docs/BOSSES-c.md`, and no phase is overstated as fully DONE.

## Group-d True Monk continuation (2026-10-12)

True Monk runs natural Lua combat through all three marks, including three independent c5005 phantom AIs, authored get-up stat changes and the a100 centipede phase.
The final pair now preserves its authored message 40 before accepting the map continuation, enabling centipede entity 2500852 and reaching defeat flag 9309.
The support renderer applies the event's one-shot dummy-260 warp and explicit centipede playback.
Three installed-data tests pass, including the slow three-phase replay and thrust/sweep counters in both animation sets, and both arena CLI forms produce reviewed first-phase captures.
Native entrance/fog presentation, smooth phantom shading, complete move/status coverage and full finishing visual verification remain open, so True Monk is not claimed DONE.
Dragon probes pass for a placed Old Dragon's real Lua contact and the Divine Dragon retry's Lua attacks, bullets and contacts, but the encounter remains blocked by aggregate/group combat, resurrection, event-value operations, tree/lightning interactions and the special finishing throw.
All five group-d tests pass in the final combined run, and the eleven-test Owl failure retry passes after the shared tree update.
The release game build and an earlier Clippy check pass, but later final Clippy and build retries are blocked by concurrent `ape_hud.rs` private-interface errors; exact commands and results are in `docs/BOSSES-d.md`.
No cache/re, existing logs, git state, network or external model/API calls were used.

## Group-a native release and descent continuation (2026-10-12)

Butterfly's immediate-action Lua wrapper now reaches HKS env 125, and all eight illusions play their release clip and emit the installed damaging Bullet children.
The native second-entity descent now completes its region/SpEffect gate, command 10, a100 entrance/descent clips and flag 11005802, replacing the staged instruction-18 bypass.
The map event's wait survives hand-over, and cleared entrance effects restore correctly on retry.
Gyoubu's two mounted bars pass the Lua/contact and paired-execution regression without per-frame position holding, while retaining a reduced-posture fixture.
Butterfly pursuit homing, summon warps/recreation, Snap Seed and complete attack/counter fidelity remain unresolved, including two late-wave illusion floor falls, so no boss is claimed DONE and this continuation did not work on Demon.
See `docs/BOSSES-a.md` for exact evidence, limits and final validation results.
The final three Butterfly tests and Gyoubu's unheld-pair test pass, along with Owl, Genichiro, tactics, attack-power and posture-recovery regression filters, with one existing slow tactics sweep ignored.
Workspace Clippy and the game build remain blocked by concurrent ape HUD privacy errors; the posture retry passes after concurrent Old Dragons borrow errors were resolved by their owner.

## Group-e playable monkey continuation (2026-10-12)

The hidden Folding Screen Monkey has a normal-arena-start sword replay with all four native Lua instances enabled and no placement, HP or posture edits.
Fixed the following event's inverted radius condition and the auxiliary Wolf origin fallback, and bell input now plays the installed 710700 gesture.
The four-target victory fixture retains each monkey's Lua instead of disabling its AI and reaches the real map defeat event.
The complete puzzle is not DONE: native sensory acquisition/history, roof-link navigation, paired room-door activation and decoded object animation remain missing.
The m89 re-export still omits m900085 in this checkout; no cache files were modified.
The inspected graphical replay kills the hidden target at step 128 and shows shared HP 74.
Monkey tests and combat regression filters pass; the latest workspace lint and game build are blocked by concurrent ape HUD private-type errors after earlier clean checks.
See `docs/BOSSES-e.md` for per-target coverage, exact blockers and continuation verification.

## Group-a combat continuation (2026-10-12)

Butterfly's eight placed illusions now run separate Lua/HKS combat with shared Wolf contacts, shared AI event flags and the authored release pulse reaching the cast.
Full projectile conversion and native descent remain blocked by event-relative point/warp and immediate-action hosting; the tested descent stalled, so the existing entry fallback was retained.
Gyoubu's two mounted bars now have a focused real-Lua/contact and paired-throw regression, including first-mark HP refill and map defeat, using an explicit positioning/reduced-posture fixture.
The final mark selects installed ThrowParam 15080170/15080171, and the staged active fight skips a stale opening AI command.
His second bar exposed an inverted TAE lunge-bound panic, now handled without changing the game data, with bound precedence documented as approximate.
Neither boss is claimed DONE, and Demon remains untouched while the native blockers remain unresolved.
See `docs/BOSSES-a.md` for per-phase gaps, changed files and continuation verification.

## Group-b boss fight pass, Guardian Ape (2026-10-12)

Fixed the waterfall ape's false death so its real decapitation, twelve-second map wait and TAE HP refill reach a live headless phase.
Headless deathblows now use the a235 player pair and retain the ape's a100 animation group, and the installed-data arena replay reaches Lua hits in both phases and defeat flag 9304.
The scripted CLI option retains c5100's extracted Lua AI with an explicit diagnostic when no external action table exists.
The revival's maximum-stat effect is read from params, and both new tests and all requested regression filters pass in `target/alt-b` with three jobs.
Workspace/all-target Clippy and the release game build pass, and a one-dependency support-actor scheduling fix lets the graphical replay exit successfully.
The inspected first-phase capture shows the real model, animation, named bar and Lua hits, with washed-out fog/exposure and fur-cloth reset warnings.
Guardian Ape is not DONE: the separate final paired continuation, terror and complete attack/counter verification, and full two-phase graphical verification remain outstanding.
Headless Ape and both Fathers remain outstanding; see `docs/BOSSES-b.md` for per-phase scope, evidence and gaps.

## Group-a boss combat progress (2026-10-12)

Lady Butterfly now uses her second entity's real finishing ThrowParam pair 15090680/15090681 instead of the first phase's ordinary pair.
TAE finishing message 10 and player EzState execution requests are hosted, and her Lua GetDistXZ query uses horizontal target distance.
Two focused release tests pass in 5.09 seconds on Hirata collision, covering natural Lua attacks, both paired deathblows, the entity hand-over, correct mark counts, both finishing clips and the map defeat flag.
The phase-two throw retains its live a100 animation group, and both arena CLI forms visibly load Butterfly and exit successfully with scripted guard input and memory disabled.
The illusion actors still lack simultaneous combat, shared AI state and projectile conversion, and the native phase-two descent remains bypassed, so Butterfly is not DONE.
Gyoubu and Demon were left at their arena-staging baseline while Butterfly remains incomplete.
The previously missing c5090, c1510, c5080 and c7020 models are now present.
All requested combat regression filters, workspace/all-target Clippy with warnings denied and the final release game build pass in target/alt-a with three build jobs.
The existing slow sweep remains ignored, and shared formatting requests were left untouched under the additive-edit rule.
Verification, inferred throw-family selection and remaining per-phase gaps are recorded in `docs/BOSSES-a.md`.

## Group-d boss combat, pass 2 (2026-10-12)

The Mibu ghost Monk now runs its real Lua battle goal without the chained-goal-setter crash, and its Mikiri uses the authored defender dummy instead of requiring a dummy on Wolf.
Four arena combat tests cover Lua attacks, guard and deflect, thrust and sweep counters, and single-mark removal through map victory and fog removal.
The scoped release regression passes 115 tests with one existing slow case ignored, including Owl, Genichiro, brain tactics, attack power, posture recovery and perilous counters.
Unlisted scripted plan tables retain the character's real Lua AI with a diagnostic, allowing offline scripted-mode arena entry without installing Owl plans on unrelated bosses.
The ghost is not claimed fully DONE: visual replays show opaque ghost shading and an incorrect finishing defender clip, alongside snap-seed event bitfields, AI target history, entrance hosting and throw-clearance gaps.
True Monk is stopped at missing `cache/models/c5005.glb`; NpcParam 50050000 selects c5000 animations, and the cache owner should run `sekiro-extract models c5005:50050000`.
The Dragon encounter was not started, preserving the requested boss order.
Workspace/all-target Clippy and the release game build pass in `target/alt-d` with three jobs, and both Mibu entry forms produce reviewed graphical captures.
The new test file passes rustfmt; workspace formatting only requests prohibited sorting of shared module tables.
See `docs/BOSSES-d.md` for exact commands, phase-by-phase limits and verification results.

## Group-e Folding Screen Monkeys audit (2026-10-12)

The c1260 zero-animation extract is a skeleton-only binder, not a decoder failure; all four placed monkey rows explicitly use c1250 animations and c1259 textures.
Added a focused four-placement donor-loading integration check and `docs/BOSSES-e.md`, with the verified row identities and an honest encounter coverage table.
The three character GLBs reported missing by pass 1 are now present, but the authored m900085 illusion sky export is still missing; its exact owner-run extraction command is documented.
The encounter is not DONE: simultaneous monkey combat, independent Lua/event routing, sensory/navigation state, puzzle interactions, shared health and natural victory remain unimplemented.
No combat or phase logic was changed, and no cache/re, existing logs, git state, network or model/API calls were used.
The donor and Senpou staging tests pass in 1.14 and 3.51 seconds; all requested Owl/Genichiro, tactics, attack-power and posture-recovery regression filters pass, with one existing slow sweep ignored.
Workspace/all-target Clippy and the release game build pass in `target/alt-e` with three jobs; workspace formatting reports concurrent/shared-file differences, while the new Rust file passes direct rustfmt checking.

## Group-c arena staging (2026-10-12)

Added `silvergrass`, `silvergrass-genichiro`, `silvergrass-isshin`, `watering-hole` and `apes-burrow` from the installed MSBs and EMEVDs.
Headless staging covers story starts, real hit collision, event fog and bars, the actual Tomoe-to-story-Isshin hand-over, storm state, Guardian Ape's delayed phase/victory events and the burrow companion/severance events.
The prologue selects its authored midnight environment and sky; the silvergrass finishing boundary switches independently of entrance fog.
Support actors have event visibility, draw masks, animations and centipede dummy warps, without simultaneous companion combat.
Graphical completion remains blocked by missing `cache/models/c5400.glb`, `c5100.glb` and `c5001.glb`, now reported before opening a window.
Four focused release tests pass in 8.32 seconds; workspace/all-target Clippy with warnings denied and the release game build pass in `target/alt-c` with four jobs.
The new Rust files pass direct rustfmt checks; workspace formatting only requests prohibited reordering of the shared test modules, which remain additive.
Exact event IDs, commands, changed files and remaining presentation approximations are in `docs/ARENAS-c.md`, with a summary in `docs/FIDELITY.md`.
No game data, research data, existing logs or git state were changed, and no graphical run or screenshot is claimed for missing-model fights.

## Group-a arena staging (2026-10-12)

Added `hirata-hall`, `hirata-father`, `hirata-inner-father` and `senpou-screens` from the installed layouts and EMEVDs, with story starts, map collision, real fog flags, supporting-cast hooks and story lighting.
Inner Father preserves think 50630000 and reads the m10_00_50_61 gauntlet script; Hirata uses the fixed past-night environment variant 0.
Senpou uses warp region 2002200, all four screen-event slots and the script's gravity-disabled hidden health controller, with no invented fog gate.
All three focused headless tests pass: two Hirata tests in 9.05 seconds and the Senpou test in 5.03 seconds.
The final workspace/all-target Clippy and release game build pass offline with four jobs in `target/alt-a`.
Workspace formatting reports only the appended test-module ordering in `tests/it/main.rs`, which was preserved under the shared-tree instruction; the new group-a Rust files pass direct rustfmt checking.
CLI startup reports missing models c5090/c1510 for Butterfly, c5070 for both Fathers, and c1000/c1250/c1260 for Senpou; no window, brain request or cache write is needed to report these errors.
The authored illusion sky m900085 is also absent from the exported sky library, and auxiliary combat and native set-piece interaction hosts remain incomplete, so these arenas are not reported fully DONE.
See `docs/ARENAS-a.md` for the reviewed events, exact missing paths, changed files, commands and verification limits.
No commits, Git operations, network calls, screenshots or protected data/log writes were used.

## Group-d arena staging (2026-10-12)

Added `mibu-bridge`, `fountainhead-bridge` and `fountainhead-sanctuary` rows from the installed map layouts and event scripts, with authored starts, collision, fog and story lighting.
Supporting phantom and Dragon cast presentation uses separate event state, and the sanctuary uses the original cinematic warp regions and cloud-sea sky.
All three focused headless staging tests passed in 6.98 seconds, including the True Monk phantom enables and the script's retry transition from Old Dragons to Divine Dragon with preserved sky and cast-removal state.
Visible staging remains blocked by missing `cache/models/c5000.glb`, `c5200.glb`, `c5300.glb`, `c5310.glb` and `c5320.glb`; cache was kept read-only.
Simultaneous cast combat, aggregate Old Dragons defeat and attached tree or finishing props remain incomplete, so these arenas are not reported fully DONE.
See `docs/ARENAS-d.md` for the exact reviewed events, missing data, changed files and verification commands.
Workspace/all-target Clippy with warnings denied and the release game build pass with four jobs in `target/alt-d`.
The new group-d Rust files pass direct rustfmt checks; the workspace formatting check only requests sorting the shared test-module table, which was left appended as instructed.

## Group-b arena staging (2026-10-12)

Added `gyoubu`, `demon` and `shura` arena rows from the installed MSBs and EMEVDs, including story starts, collision, fog and phase event rows.
Shura uses distinct Emma and Isshin fog lifecycles and changes the tatami to the burnt roof and evening to night through the map script.
The map renderer now reads original MSB entity/group identities and placed SFX to apply the shared event scene state.
All three focused headless staging tests and the existing Genichiro hand-over regression pass, including the exact Shura story warp and delayed Emma-to-Isshin transition.
Scripted boss defeat no longer schedules the harness's three-second automatic retry before a delayed map transition completes; player death still retries normally.
Workspace/all-target Clippy and the release game build pass in `target/alt-b` with four jobs; only the prohibited module-order rewrite remains in the workspace format check.
Exact commands, results and changed files are recorded in `docs/ARENAS-b.md`.
Graphical boss placement remains blocked by missing `cache/models/c5080.glb`, `c7020.glb`, `c7400.glb` and `c5430.glb`; cache was kept read-only.
Boss attack fidelity, the first-visit Demon encounter and Isshin's attack-driven fire helper remain separate work.

## Local checkpoint chooser (2026-10-12)

Coach brains now default to a free local Beta Thompson sampler over the coach's offered sets and existing combat checkpoints; a signed-in Codex or Claude CLI needs no API key.
Optional `checkpoint_prefs` seed four pseudo-counts, and rewards use four-second damage/break/counter windows with passivity and void-fight safeguards.
Version-3 player profiles and shared boss contribution snapshots retain phase-specific discounted evidence without duplicate saves; pooled transfer is capped, and `--checkpoint decisions|off` preserves the alternatives.
The overlay shows the selected set/context/posterior mean, session reports list choices and rewards, and coach reviews receive a fitted per-context/set summary.
See `docs/BRAIN.md` and the updated README and `.env.example` for setup, flags, reward formula and memory layout.
Verified in `target/alt` with four build jobs: full release tests for sim and brain-net, the release game build, workspace formatting, and scoped brain Clippy with warnings denied.
Workspace Clippy remains blocked by concurrent changes in `crates/game/src/cloth.rs` and `crates/extract/src/models/textures.rs`; these files were left untouched.
The headless soldier duel selected a set and retained a reward with zero controller requests or memory writes; all provider/CLI checks used offline fakes and temporary files.

## Havok cloth (2026-10-12)

Ported FwLo's cloth solver (MIT) onto a new reader: `crates/formats/src/hkx/cloth.rs` flattens every `*_c.hkx` cloth (reference skin, display deform, driven bones, constraint sets, capsules), `sekiro-extract models` writes `cache/models/<id>.cloth.bin`, and `crates/game/src/cloth.rs` simulates it (`--no-cloth` turns it off).
Cloth now moves on Wolf (scarf, coat flaps, hair), castle Genichiro (cape, sash rope, skirt plates, helmet fur), Tomoe Genichiro, the Owl (coat, haori, ponytail, breech) and the soldier (haori, sash, straw cape, hair, spear cloth); every deformer reproduces the FLVER rest pose to 0.0000.
Two findings beyond his notes: position-only (`...DeformPOperator`) displays use a unit-length triangle normal axis (found per display from the rest pose), and bone-space PN blocks carry packed normals he left zero (zero normals showed as dark stippling until read).
Cost: 0.85 ms of CPU a frame for Wolf plus a boss (11 and 5 cloths), 0.04 ms to apply; frames stay at the 16.6 ms vsync.
Gaps: a mesh driven by two cloths keeps only the first; cloths on hidden variants and the second sim cloth of two-sim containers are skipped; no cloth-against-cloth or body-mesh collision, so layers can interpenetrate.
Frames are in `target/alt/cloth/`.

## FXR effects, hit sparks, blood and stains (2026-10-12)

Ported FwLo's node-tree FXR player (MIT) onto our own reader: `crates/formats/src/fxr_json.rs` (with the generated `fxr_schema.rs` and `tools/gen_fxr_schema.py`) turns an installed effect into the named tree, `crates/game/src/fxr_tree.rs` runs nodes, emitters and particles, and `crates/game/src/fxr.rs` batches sprites, flipbooks, 604 multi-texture layers, lines and tracers with soft particles.
Real effects now: deflect and guard sparks and hit sparks and blood from the HitEffectSfx tables (`crates/sim/src/hit_fx.rs`), the deathblow blood spray (220505), the firecracker bang and flames, Owl warp puffs, Genichiro's lightning bolts, plus DecalParam floor stains from hits and TAE 138 (`crates/game/src/decals.rs`, CPU BC1/BC4 decode in `formats::tpf`).
Stand-ins stay where an effect needs a model (605: arrows, shuriken, the smoke ball) and under `--no-fxr`.
Frames are in `target/alt/fx/`; heavy firecracker bursts cost about 2-4 ms of CPU in draw collection and 2 ms in stepping (JSON-tree lookups, first-use parse spikes of 10 ms).
Gaps: 605 models, 607/608 distortion and radial blur, 609 lights, GPU particles (10000), hit-point placement for deflects (held on the body), stain blend is untraced.

## Character materials (2026-10-12)

The extractor now bakes each character material's gloss and metallic data into the glTF metallic-roughness texture: roughness is 1 - the shininess in the blue channel of `_n` (floor 0.089), metallic comes from the `_m` mask, so armour and the katana catch the map's specular GIIV light and cloth stays matte.
Hair and fur cards (`Fur_NTC*` shaders) now use the shared strand texture with its alpha cutout instead of the fur-coat colour map, and `_e` (edge) materials without an alpha reference are cutouts.
That was the cause of the Owl's odd face: his beard, brows and hair rendered as grey opaque flaps; the soldier and Genichiro hair use the same fix.
c0000, c5060, c7100, c7110 and c1010 were re-extracted; no game code changed, since actors already share the map shader.
Gaps: DetailBlend layers, SSS skin, per-material colour multipliers, alpha-to-coverage and the game's own shininess-to-specular curve are not modelled.
Checks: see `docs/VISUALS-PLAN.md`; pairs in `D:/sekiro-rs-media/progress/2026-10-12/`.

## Continuous Genichiro brain and counter learning (2026-10-11)

The retained driver controls castle Genichiro and Way of Tomoe as one fight, then reinstalls the proper table and advances numbering on reset; Owl's existing phase path remains continuous.
Tomoe has a Lua-derived action table, lightning plans and a phase move sheet, with c7100-owned persistence and phase-specific evidence, plans, failures and bans.
Only byte-identical decoded clips with matching TAE timelines pool evidence; the inspected installed attack clips did not qualify.
Actual Mikiri pairs, successful sweep kicks and HKS lightning-return windows tally typed counters, visible in memory, coaching, reports and the overlay.
Lua retains counter timing, candidates and mandatory choices; the brain reweights or vetoes offered callbacks, using NoAction only when Lua offers it.
All 449 sim/brain-net release tests, formatting, workspace/all-target Clippy and the release game build pass offline with six jobs in `target/alt`.
No commits or real model/coach calls; protected research, game data and existing log/memory files were not changed.

## Enemy damage-motion armour and aerial falling (2026-10-11)

Category-1001 SpEffect tables now replace incoming damage levels using the native enum translation and priority, preserving HP/posture while protected clips continue.
Audited windows include Genichiro 3026/3030/3025, Owl aerial and committed swings, and soldier commitments; full IDs and timings are in `docs/COMBAT-RULES.md`.
Authored foot elevation now selects aerial damage aliases and transfers height into falling physics when interrupted, fixing the grounded reaction snap.
Genichiro 3015 protection remains unresolved because its audited data supplies no verified armour source; positive durability/toughness meters remain unimplemented.
Six installed-data regressions pass, and the inspected `target/alt/armour/final-{windup,hit,strike,landing,recovery}.png` captures show 3030 continuing after 80 HP and 30 posture damage.
All combined sim/brain-net release tests, formatting, workspace/all-target Clippy and the release game build pass offline with six jobs in `target/alt`.
No commits, network, or protected-path writes.

## FXR 603 playback milestone, incomplete (2026-10-11)

Original Rust/WGSL now play supported 603 layers with native unmodified curves, authored emission/lifetime/motion/color/scale, TPF/DDS textures, alpha/additive batches and bounded particle pools.
TAE body-dummy and Bullet sfxId requests are connected; all existing stand-ins remain, and `--no-fxr` disables the player entirely.
The installed scan accepts 50 layers across 18 effects; `target/alt/fxr/billboard-final-{152,154,156}.png` shows an orange textured flare and the disabled comparison removes it.
Expiry and additive-blending bugs found in E2E runs are fixed; final preview logs contain no shader/slab errors.
At 90 live preview particles, FXR CPU work averages about 0.09-0.10 ms with a 0.314 ms maximum; 15-17 ms VSync frames do not establish GPU cost or the heavy-effect budget.
Requested combat captures still lack real deflect sparks, blood, soft smoke and authored lightning; early firecracker/smoke/arrow captures are rejected and shuriken shots missed the trail.
GPU instancing, modifiers/spread, 604 composition, 606 ribbons, 609 lights, soft particles, hit/blood routing and exact dummy orientation remain open.
The supplied EvenTorset `src/fxr.ts` is zero bytes; credits and the remaining gaps are documented in `CREDITS.md` and `docs/VISUALS-PLAN.md`.
Formatting, workspace/all-target Clippy, all 120 formats/game release tests, the full sim release suite and the release build pass offline with six jobs in `target/alt` after concurrent repairs.
No commits, network, dependencies or protected-path writes; `play.rs` only gained the plugin registration.

## Deathblow wall clearance and Tomoe fade (2026-10-11)

ThrowParam dummy frames now drive one shared, floor-aware capsule reservation through the full paired body clip; both characters bypass independent wall pushes while held.
Genichiro absorbs onto Wolf dummy 266; Owl absorbs Wolf onto his dummy 230 instead.
The bounded wall search is inferred, with a diagnostic fallback for exhausted space and no full-mesh clearance claim.
Genichiro's EMEVD cinematic now fades out over 0.5 s, swaps and applies the MSB warp under black, holds 0.75 s and fades in over 0.5 s with simulation and input paused.
Owl's phase-2 event has no cutscene and keeps its playable HKS heat-up.
Inspected `target/alt/polish/deathblow-wall-{windup,strike,recovery}.png` show connected contact and clear recovery; `tomoe-final-{before,fade-out,black,fade-in,after}.png` show the swap hidden under complete black and authored placement afterward.
The combined sim/brain-net release suite passes all 439 tests, including synthetic contact, real-data transition, Owl wall deathblow and grab regressions.
Formatting, workspace/all-target Clippy and the release game build pass offline with six jobs in `target/alt`.
No commits, network, worktrees or protected-path writes.

## FXR sampling groundwork, player blocked (2026-10-11)

Added named appearance IDs 603/604/606-609, documented blend operations and allocation-free property sampling with explicit caller-selected linear/stepped policies.
Native function-ID/clock mapping is not asserted; unsupported modifiers, loops and cubic evaluators fail explicitly.
The supplied offline notes lack action field/property layouts, and their referenced action-schema checkout is absent; requested a local schema location without accessing the network.
The particle player, wiring, `--no-fxr`, real-effect captures and frame-cost measurements remain undone; all effects still use stand-ins.
Formatting, workspace/all-target Clippy, 115 formats/game release tests and the release game build pass offline with six jobs in `target/alt`.
The simulation retry passes all 399 tests, including the concurrently repaired wall-clearance deathblows; reports are under `target/alt/fxr/player-*`.
No commits, dependencies, protected-path writes or combat-job edits.

## Sky exposure and silhouette fog follow-up (2026-10-11)

Sky now uses the same scene-radiance bridge and camera exposure as diffuse sun/ambient instead of independent gains with missing exposure.
MSAA fog uses each original colour sample's matching depth before resolving, removing the noon silhouette and roof-railing fringes.
All 32 v9 horizon captures across Outskirts/Castle at 6/12/18/0 h were inspected; glow fades continuously, but the reported discontinuous vertical seam could not be established.
The matched v9 ambient roof pair remains dark but readable; no duplicate GIIV exponent/pi/exposure or actor-scale bug was found, and no intensity was tuned by eye.
Night-world parity remains unresolved, including the variant-04 GILM fallback and horizontal background band.
Requested release tests pass (55 tests), workspace/all-target Clippy and formatting pass, and the release game build passes offline with six jobs in `target/alt`.
No commits, network, worktrees or protected-path writes.

## FXR reader foundation (2026-10-11)

Added a bounds-checked Rust version 5 FXR table reader and read-only inspector, preserving action fields, properties, modifiers, sequence descriptors and original bits.
Installed-data coverage parses 2,913 FXRs across all eight binders and checks their unpacked counts, including 1,856 common effects and 450 m11 effects; no separate character effect binders are present in this cache.
TAE 96/99/100/118-121 now preserve visual effect requests and tick start edges with StateInfo gating.
VISUALS-PLAN steps 10-11 remain incomplete: action/asset semantics and the particle renderer, bullet/hit routing, stand-in replacement, `--no-fxr`, screenshots and frame-cost measurements are still outstanding.
No combat effect has been replaced and no rendering-fidelity claim is made.
Installed-data coverage, four reader tests and two TAE effect tests pass, along with formatting, workspace/all-target Clippy and the release game build, offline with six jobs in `target/alt`.
The combined release suite fails `deathblow_near_a_wall_does_not_clip` and `deathblow_with_wolf_backed_to_a_wall_does_not_clip`; the collision/throw code is reserved for the concurrent combat job.
No commits, network, new dependencies or protected-path edits.

## GIIV bounce and spatial probe blending (2026-10-11)

Re-extracted all four requested maps with linear BC7 X/Y/Z/W coefficients, preserving independent resolutions and signed/exponent packing in RGBA8 volume exports.
The shared 3D atlas evaluates native directional diffuse per fragment in matching MSB Env_Box bounds, with cubic fades and GILM box-corrected specular toward Env_Point captures.
Wolf and enemies use the same shader, with gameplay tint synchronization; `--no-giiv` restores region-blended Lambert diffuse.
All six requested before/after pairs under `target/alt/visuals/v8-giiv-*` were inspected: subtle warm roof bounce, clearer Hirata banks and spatially varied interior shade, without new visible edge banding/blotches.
Dawn, probe-disabled and Fountainhead loading controls pass without shader errors, including the previously blocked mixed-resolution Outskirts volumes.
Two repeated roof comparisons measure about 2.5-2.8 ms additional opaque GPU work; total frame medians remain around 16.6-16.7 ms.
Formatting, workspace/all-target Clippy, requested release tests (117 passed, two existing ignored) and the release game build pass offline with six jobs in `target/alt`.
Exact IV/PCC uploads, authored fades/scales, overlap state and time interpolation remain documented approximations in `SHADERS.md` and `VISUALS-PLAN.md`.
No commits, network, worktrees or protected-path writes.

## Authored sky domes (2026-10-11)

Added camera-centred, unlit m89 meshes with original material masks, reverse-Z infinity, no shadows/depth writes and a native-fog horizon blend over ray Y 0..0.22.
Local common event 100 and MSB entity groups establish nearest 0/2/6/12/18/22 skies for m11_00/m11_01 and fixed compound m900060/m900070 skies for m10/m25.
The supplemental sky-only export reuses existing GLBs and writes only under `cache/maps/`; `--no-sky-dome` restores the old clear colour.
Sky shares HDR exposure/tone mapping; unresolved SAT colour packing uses gamma two, Ver2 gain 8 and original Fountainhead/Cloud gain 1.
Captured and inspected the requested views plus upward/lake/clear controls under `target/alt/visuals/v8-sky-*`; the initial Outskirts 6 h GIIV dimension mismatch was isolated with a labelled sky-only diagnostic and subsequently fixed by the GIIV session above.
Three final-build paired Owl runs measure median added frame cost 0.159 ms, range 0.092..0.573 ms; raw samples are in `v8-sky-bench.json`.
Formatting, workspace/all-target Clippy, requested release tests and the alternate release game build pass with six jobs.
Selection, capture judgements, calibration and remaining cloud/event/LightShaft gaps are documented in `MAP-BASELINE.md` and `VISUALS-PLAN.md`.
No commits, network or protected-path edits.

## Animation blend continuity (2026-10-11)

Wolf and NPC fades now retain the displayed local pose when interrupted, including any unfinished crossfade, with character-clock hit-stop freezing.
The renderer also uses the incoming clip's TAE blend when a previously active clip becomes the displayed main clip without restarting.
Graph and TAE values for Owl, soldier and Genichiro are documented in `docs/FORMATS.md`, with a reproducible data audit and two pose regression tests.
All installed transition effects audited have duration 0 and curve 0; Owl 3000/3003/3004 blend for 0.2 s, 3005 for 0.233333 s, while 3001/3002/3006/3007 lack a start Blend event.
Native research confirms curve 0 is smoothstep, now used in playback, and the loaded-clip callback returns 0 for a missing Blend event.
The effect's temporary 0.05 s initialization is overwritten by that callback; no invented positive fallback was added.
The reported late combo-continuation snap is not claimed fixed without a matching replay.
The scripted roof replay deflects at tick 61 and replaces recoil 8600 with attack 3003 at tick 98; before images already blend smoothly.
Before/after frames in `target/alt/blending/frames.png` were inspected; the after sequence eases continuously, and all 251 simulation entries and both contacts match.
Formatting, workspace/all-target Clippy, the combined release test suite (430 tests) and the release game build pass with `CARGO_TARGET_DIR=target/alt` and four build jobs.
No commits, network or protected-path edits.

## CPU frame regression (2026-10-11)

Restored bindless material slabs and multi-draw batching for all map surfaces, retaining layered textures, hemisphere lighting and every visual feature.
The latest hemisphere change had extended a non-bindless material wrapper to the whole map, making render preparation/submission the bottleneck.
Uncapped frame means at seconds 6-11 after loading: m11 viewer 23.751 -> 16.654 ms, play/start 3 37.162 -> 16.930 ms, Owl 36.631 -> 16.821 ms.
Opt-in `--profile-systems` exposes map-system CPU timers and Bevy/render diagnostics; periodic `--visual-stats` already works in play.
Before/after screenshots in `target/alt/perf` were inspected with equivalent scene shading; the viewer world-crop difference averages 0.327/255 per channel.
Remaining viewer time is largely swapchain waiting, with opaque shading and the 18,424 static mesh entities as further costs.
Details and reproduction commands are in `docs/VISUALS-PLAN.md` section 8.
Formatting, workspace/all-target Clippy, 423 release game/sim tests and the release game build pass offline with six jobs in `target/alt`.
No commits, network, worktrees or protected-path changes.

## Exe-traced renderer CPU uploads (2026-10-11)

Fog now uses verified GameScale lengths/inverse densities, raw signed height, geometric slices, byte/gamma colours, extinction and directional slant from `docs/SHADER-INPUTS.md`.
LightSet colours are squared times W; all map surfaces retain directional hemisphere coefficients with one Bevy bridge `SCENE_TO_BEVY` (3135.61 = pi / default camera exposure since 2026-10-15, formerly an inferred 4500).
Separate sun/ambient/local tuning and the post 0.5 multiplier are removed; existing EV adaptation, bloom and sensitometric mode 7 were checked against the trace.
All seven v7 before/final scenes were inspected against v5/v6: clearer foreground fog, retained snow detail, cool noon shade and warmer dawn/dusk; an extra probe-disabled capture verifies the hemisphere fallback.
Formatting, workspace/all-target Clippy, requested release game/formats/extract tests and release game build pass offline with six jobs in `target/alt`.
Artifacts and save backup are under `target/alt/visuals`; no commits, network or protected-path writes.
BTL/bank routing, actor directional ambient, native GIIV/froxel/local/transparent fog, sky dome and complete bloom/table scheduling remain gaps in `docs/SHADERS.md` and `docs/VISUALS-PLAN.md`.

## Native ordinary post chain (2026-10-11)

Original GPU passes replace Bevy AutoExposure, Bloom and ACES; the 3D camera uses Tonemapping None and HUD cameras stay outside the chain.
The newly delivered SHADER-INPUTS trace supplies weighted biased-log adaptation, natural-exponential soft EV limits, authored/default native tone operators and bloom normalization; sensitometric mode 7 and gamma 2.2 are the default.
The existing Bevy lighting/camera units use one explicit 0.5 HDR-to-native radiance calibration shared by metering, bright extraction and tone input; the old BASE_EV/METER_EV offsets and alpha*0.08 mapping are removed.
Native luminance-subtractive extraction, seven-tap blur and three-level composition precede tone transfer and the confirmed trilinear grading LUT.
All seven requested v6 before/final scene pairs and stage controls were inspected, with retained snow detail, subtle halos and zero pure-white pixels in the final world crops.
Native post GPU time is 0.06-0.08 ms; noon viewer frame snapshots are 22.222 ms before and 25.381 ms after, with shared-machine noise preventing an isolated overhead claim.
Formatting, workspace/all-target Clippy, the requested release game/formats/extract suite (104 passed, two existing ignored) and the release game build pass offline with six jobs in target/alt.
Artifacts and save backup are in target/alt/visuals; this session made no commits, network calls or protected-path writes.
GIIV, full bloom/temporal scheduling, exact tap/reduction/table layouts, HDR output, matrix/vignette and LUT blending remain gaps; newly resolved fog/light CPU uploads remain a follow-up.

## Shader fog and arena scenery (2026-10-11)

Bevy DistanceFog is replaced by an original HDR depth pass implementing recovered height density, directional colours, independent source gates, midpoint integration and analytical far continuation, with MSAA and `--no-fog` preserved.
Named provisional CPU conversions use height scale 0.01 and `world_y+offset`; castle FarDepth is 400 and authored GameScale 2.5 is retained pending verified packing.
Owl and Genichiro retain 60 m collision and objects while rendering map pieces to 1500 m, including all 3622 castle pieces versus 1553 cropped; `--no-far-scenery` restores the render control.
Matched Owl runs add about 2.1 GiB peak working set and 1.0-1.4 s loading; fog costs about 0.34 ms GPU, while paced frame medians near 16.7 ms do not isolate distant-geometry cost.
All requested v5 screenshot combinations were inspected, with smooth distance haze, warmer dusk and no arena crop edge; additional Fountainhead lake captures expose the existing flat water limitation.
Native CPU transforms, local fog/shadows, fog cards and transparent-depth handling remain documented gaps in `docs/SHADERS.md` and `docs/VISUALS-PLAN.md`.
Workspace formatting, all-target Clippy with `-D warnings`, the requested release game/sim/formats/extract tests and release game build pass offline with six jobs in `target/alt`.
Artifacts and save backup are under `target/alt/visuals/`; no commits, network calls or protected-path writes were made.

## Mobility step 6: swimming and cover sword (2026-10-11)

Water material planes now drive HKS swimming, diving, root-motion up/down movement, surface buoyancy, water landing and shore exit, with solid underwater floors retained.
The m13 Abandoned Dungeon replay reaches SwimMove, SwimToDive, DiveMoveDown, DiveJump / DiveToAir, water landing and ground movement after a shore jump; synthetic tests also reach DiveToSwim and shallow-water exit.
Cover docking now uses the hang fix's scabbard attachment; the matched `target/alt/mobility/cover-sword-{before,after}.png` captures show a clear right hand.
All five `target/alt/mobility/swim-*.png` captures were inspected: the dive position lowers and the shore exit stands on terrain, but pale water leaves visible immersion unverified and the resurface camera is too close.
Native water rendering/body offsets, volume bounds, underwater optics and camera remain documented gaps in docs/MOBILITY.md; concurrently owned map/material files were not edited.
Formatting, workspace/all-target Clippy with `-D warnings`, the full release sim/formats suite and the release game build pass, including all six swimming tests and existing mobility/cover/hang/grapple tests.
Validation logs are under `target/alt/mobility/swim-*`.
The complete sim/formats suite reports 445 passed and two existing ignored.
All Cargo work uses offline mode, target/alt and six jobs; saves were backed up under `target/alt/mobility/save-backup-20261011-swim` before game runs.
No commits, network calls or protected-path writes were made.

## Castle Genichiro boss brain (2026-10-11)

Castle c7100 row 71001000 now uses the shared coach/scripted/Decisions/Responses controller lookup, with 24 Lua/TAE/AtkParam-derived actions, seven named combos, all seventeen existing tactics and fifteen mixed chains.
The new offline suite records 52 brain-chosen attack starts in 60 seconds across 18 plans, arrow bullets 71000500 and 71000530 through 71000533, real combo move sheets, unchanged Lua choices, posture checkpoints, six-second pacing and isolated c7100 boss/player memory paths.
The first deathblow keeps the driver; the second hands castle c7110 row 71100000 back to its separate Lua goal, with no c7100 plan or memory carried across.
Grabs, automatic downed-Wolf/counter reactions, predictive arrow 3013 and unavailable lightning remain excluded from the offered vocabulary; reset after the hand-over needs a new run to reinstall the brain.
The forty-second scripted god-mode replay keeps memory off, and all four `target/alt/genichiro/brain-{005,010,020,040}.png` captures were inspected.
The overlay clearly shows Genichiro's active combo and move label; dark armor and excessive close framing after knockback at the roof edge remain gaps outside this brain change.
Formatting, workspace/all-target Clippy with `-D warnings`, the full release sim/brain-net suite (417 passed) and the release game build pass offline with six jobs in `target/alt`.
Check logs and screenshots are under `target/alt/genichiro/`; no commits, real model/coach calls, network access or protected-path writes were made, and saves were backed up before game runs.
See `docs/BRAIN.md` and `docs/GENICHIRO.md` for the complete table, derivation and exclusions.

## Visuals steps 6 and 8 (2026-10-11)

Original layered map materials support `M[Multiple]` and `M[MultipleGround]`, with overlay/snow/layer C from the first byte of FLVER packed blend streams 0/4/2, repeat samplers and `--no-layers` / `--show-blend` controls.
Foliage cutouts are two-sided, Wind/Gradient sampler mismatches have an MTD fallback, and previously discarded decal vertex alpha is retained with neutral RGB.
Hirata's suffixed envmap contains 148 GILM cubes; full extraction now succeeds and writes edges, grapple points and lights.
All ten maps were regenerated sequentially, and all four requested `target/alt/visuals/v4-*-{before,after}.png` pairs were inspected: snow tops, moss patches and open foliage silhouettes improve without obvious tiling seams.
Sunken Valley retains an isolated opaque SnowGround triangular patch; native coverage, secondary masks, UV transforms and gloss remain gaps documented in `docs/VISUALS-PLAN.md`.
Two uncapped timing series disagree under concurrent jobs: Outskirts layer-on wall cost ranges from -4.81 to +3.93 ms; matched short opaque GPU samples are 2.25 ms off / 2.20 ms on, so no reliable isolated cost is claimed.
Formatting, workspace/all-target Clippy with `-D warnings`, the requested release game/formats/extract suite (99 passed, two existing ignored, including `map_tangents`) and the release game build pass offline with six jobs in `target/alt`.
FwLo receives one facts-only credit clause; this work made no commits or network calls, and saves were backed up before game runs.

## Mobility step 4: wall hug (2026-10-11)

Kind-3 cover supports all eight variations, HKS entry/idle, authored-root-distance sliding with endpoint clamps, restricted look-out states and action/away-stick exits, using the existing E / Y action button.
The verified m11_00 h018700 chain-1 replay moves about 1.26 m, reaches the left peek loop and restores Idle; all four final `target/alt/mobility/cover-{idle,slide,peek,exit}.png` captures were inspected.
Wolf stays outside the visible wall, but foot/render alignment and the native camera-dependent peek offset remain gaps; the requested variation-0 chain-6 route enters against collision without a matching visible full wall.
Search, placement and away-stick thresholds, the 0.7 m cover offset frame interpretation, immediate entry anchoring and unsupported doors/corners/IK are documented in `docs/MOBILITY.md`.
Formatting, workspace/all-target Clippy with `-D warnings`, the release game build and the full release sim/formats suite pass offline with six jobs in `target/alt`: 431 passed, two existing ignored, including five cover and four unchanged ledge-hang tests.
Validation logs are `target/alt/mobility/cover-{fmt,clippy,build}-latest.log` and `cover-suite-pass.log`; the final scripted log is `cover-final.log`.
No commits, network access or protected-path writes were made; saves were backed up under `target/alt/mobility/save-backup-20261011-cover` before game runs.

## Start and pause menus (2026-10-11)

No arguments or `--menu` opens a Segoe UI fight picker with soldier, Owl, Genichiro, arbitrary character ids, map/start, AI/stages, overlay, AP, hour, player mode and memory name.
Menu and CLI use one options builder; Esc pauses, and Start / Back relaunch after the app drops so brain session saving follows the existing Quit path.
Preferences save best-effort outside the repo, with choices also carried through relaunch; disk persistence was verified using a writable redirected LOCALAPPDATA.
Reviewed 1600x900 / 1280x720 menus, the loaded soldier fight, pause, returned menu and restored preferences under `target/alt/menu/`; text fits, inactive controls are grey and the pause panel remains readable over combat.
Formatting, workspace/all-target Clippy with `-D warnings`, the release build, 369 sim tests and the final 26 game tests pass offline with six jobs in `target/alt`.
Four menu/CLI equivalence tests cover the requested cases and Genichiro's registry selection; three small simulation/extractor lint fixes keep shared workspace checks clean.
The existing `--no-enemy` behavior still leaves a passive rendered opponent, and physical gamepad input / live coach calls were not exercised.
See `docs/INTERFACE.md` for the lifecycle rationale, capture driver and verification; no commits, network calls or protected-path writes were made.

## Visuals steps 4 and 5 (2026-10-11)

Sun shadows now use two cascades over 60 m and exclude map mesh bounds beyond 45 m, with 0.02 depth bias and 0.6-texel normal bias; characters retain shadows.
Outskirts/Castle eligible map casters drop from 18027/15933 to 1260/2958; matched scripted medians remain approximately 16.64/16.69 ms at 60 Hz.
Short uncapped viewer samples improve from 59.8 to 62.6 fps and 60.2 to 112.7 fps; opaque GPU timings do not isolate shadow cost.
The original version-2 BTL reader validates all 735 records in 18 banks; light sidecars were exported for m11_00, m11_01, m25_00 and many-light m10_00, preserving banks and Bevy-space positions.
Distance-culled point lights use provisional 900 candela per authored unit, a nearest-64 cap and no local shadows; native attenuation, colour space and bank activation remain gaps.
Every generated `target/alt/visuals/v3-*.png` was inspected; Wolf's outdoor shadow persists, and calibrated tower/corridor lights preserve detail without the rejected cyan wash.
Workspace formatting, Clippy with `-D warnings`, the requested release suite (89 passed including `map_tangents`, two existing ignored) and the release game build pass offline with six jobs in `target/alt`.
Mappings, counts, measurements and capture judgements are in `docs/VISUALS-PLAN.md`; optional step 6 layered materials remains pending.
No commits, network access or protected-path writes were made; saves were backed up before game runs.

## Mobility step 3(b): solved grapple (2026-10-11)

Grapple flight now solves the authored 0.833333 / 0.666667 s arrival under 19.6 m/s2 gravity, with horizontal homing, collision stops, air shots, both chain shots, HKS cancels, wire landing and a 0.6 m ornament perch.
WP_A_9500 exports with 80 bones and 16 clips; play uses the permitted dummy-144 cable line after the skinned preview failed visual review.
TAE 720 now sheaths the katana during hang and early climb; matched before/after captures show clear hands and no blade on the tiles.
The three `--simple-ai` castle replays land on the roof at 0.279 m from the anchor or perch on the second ornament at 0.045 m; reviewed captures and logs are under `target/alt/mobility/grapple2-*`.
All required checks pass with `CARGO_TARGET_DIR=target/alt`, six jobs and offline Cargo: formatting, workspace/all-target Clippy with `-D warnings`, the release game build and the release sim/formats/extract suite (433 passed, two existing ignored), including all 15 wire tests.
Native skinned cable deformation, exact EaseIn curve evaluation and the prior target-search gaps remain documented in `docs/MOBILITY.md`.
No commits or protected-path writes were made.

## Enemy registry and castle Genichiro (2026-10-11)

Placed enemies now resolve NpcParam, think, Lua bundle, animation donors and names from extracted data, with generic `--enemy cXXXX[:row]` selection and roster extraction defaults.
The smoke loads 102/102 character types across 1754 placements; this is loading coverage, not a claim of complete combat fidelity.
Castle Genichiro uses c7100 row 71001000, 1598 HP, 660 posture and two marks, with 17 Lua attacks in the idle-Wolf 60-second test and seven arrow contacts from Bullet 71000500.
The first mark restores HP and continues Lua; the second drives EMEVD 11115820's sequential hand-over to castle c7110 row 71100000, 2556 HP, 900 posture and one mark.
The c7110 move set, final deathblow victory event and reset to c7100 are tested, and the renderer reloads the model and animation library at hand-over.
Generic a000-a400 animation-set selection is tested separately on Isshin because Genichiro changes character rather than clip group.
Lightning and Lightning Reversal, cinematic playback/region warp, simultaneous actors and automatic all-boss event discovery remain gaps.
See `docs/GENICHIRO.md` and `docs/ENEMIES-PLAN.md`; reviewed screenshots and run/check logs are under `target/alt/genichiro` and `target/alt/genichiro-*.log`.
Fmt, workspace clippy, the release game build and the final full sim/brain-net release suite pass (404 tests), including unchanged brain, Owl and soldier regressions.
No commits or network were used.

## Visuals steps 2 and 3 (2026-10-11)

Map cameras now use HDR/ACES fitted, conservative GPARAM bloom, nearest-hour display-space LUTs and compute auto exposure; the HUD is unaffected and probe captures remain lighting only.
`--no-grading`, `--no-bloom` and `--no-auto-exposure` provide controls; unsupported compute falls back to fixed Tone Map exposure.
The requested three maps were re-extracted, adding four/four/two lossless LUT strips and fixing the stored-bitangent-to-glTF-tangent conversion.
The original map_tangents thresholds now pass with 0.933 direction and 0.953 handedness agreement; matched before/after normal-map screenshots were reviewed.
All requested grading/bloom views, Castle night controls, the temple-to-outdoor walk and the enlarged sky crop were inspected under `target/alt/visuals/v2-*.png`.
Grading is plausibly teal/amber at sunset and cooler at noon, with no visible bands or sampled white clipping; sky domes, local lights and exact Yebis curves remain unresolved.
Noon medians are 16.675 ms Outskirts and 16.641 ms Castle, within 0.040 ms of HDR/ACES-only controls at the 60 Hz cap, which cannot establish uncapped GPU cost.
Workspace formatting, Clippy with `-D warnings`, the requested 84-test release suite (two existing ignored tests) and the alternate release game build pass with four jobs and offline Cargo.
Mappings, capture names and limitations are in `docs/VISUALS-PLAN.md`; no commits or protected-path writes were made.

## Mobility step 3(a): ground grapple (2026-10-11)

The edge exporter writes authored grapple points; castle has 172 objects / 224 points and outskirts has 237 objects / 360 points plus four unplaced collision-owner records.
Wire search reads real params, camera angles and collision LOS; HKS drives the ground shoot, move and landing, with a projected target ring.
The requested balcony spawn is obstructed and correctly does not fire; yaw -41 also stays Idle.
The clear roof route at (104.03038, 53.234768, 254.9751), yaw 60, fires at step 30, travels in clip 202012 for 0.5 s, reaches the Shachi exactly at step 90 and settles grounded 0.279 m from it in LandWireMove.
Reviewed captures are `target/alt/mobility/grapple-before.png`, `grapple-mid.png` and `grapple-after.png`; detailed findings and placeholders are in `docs/MOBILITY.md`, step 3.
Travel is a straight fit to TAE 901, with a 20 m/s fallback; native homing/root warp, screen-space filtering, wire-set changes, hook model, air/chained shots and full capsule sweeps remain unimplemented.
Formatting, workspace/all-target Clippy with `-D warnings`, the requested release sim/formats/extract suite (418 passed, two existing ignored) and the release game build pass with `target/alt` and four jobs.
The final suite log is `target/alt/mobility/grapple-suite-v2.log`; both final scripted game runs exited successfully.
No commits, network calls or protected-path writes were made; saves were backed up under `target/alt/mobility/save-backup-20261011-grapple` before game runs.

## Visuals steps 0 and 1 (2026-10-11)

Read-only GPARAM/MSB dumps and hour comparisons are in `target/alt/visuals/`.
MSB probe regions, HDR BC6H extraction, Lambert cubes, spatial selection, hour variants and camera environment lighting are implemented and all three requested maps re-extracted.
The intensity is 9000/pi cd/m2; `--no-env-probe` preserves the old ambient fallback.
Six matched screenshot pairs show warmer evening and cooler interior shade; the probes contain nearby scenery, so they light the world but are not shown as the sky (`--probe-sky` shows them for debugging).
See `docs/VISUALS-PLAN.md` step 1 for approximations and the capture names.
Formatting, workspace Clippy, 64 release formats/extract tests (two ignored), 14 game unit tests and the alternate release build pass.
The pre-existing map_tangents regression still fails with direction agreement 0.004.
Matched frame medians stay at 16.6-16.8 ms with a 60 Hz cap; measured probe deltas range from -0.034 to +0.076 ms.
No commit or protected-path write was made.

## Mobility step 2: ledge hang (2026-10-11)

Kind-2 variation-0 map ledges now support ground grab, HangIdle, root-track-fitted climb up and drop through the existing HKS transitions.
The new edge reader and extractor write placed `edges.json` records for m11_00 and m11_01; collision confirms the researched h019900 chain-7 roof edge.
E / gamepad Y is the action button; play scripts accept `action`.
The original start-2 route stayed outside reach; the verified upper-side route at (83.307, 30.6, 243.649), yaw 133.216, finishes at (80.9384, 29.5759, 245.8745), 0.255 m from the edge.
Reviewed captures are `target/alt/mobility/hang-side.png` and `hang-climbed-side.png`; `docs/MOBILITY.md` records the route, placeholders and remaining docking gaps.
The drop replay fires HangToFreeFall and lands on the lower roof; `target/alt/mobility/hang-drop.png` shows the released fall.
Formatting, workspace Clippy, 401 release sim/formats tests (two existing ignored tests) and the alternate release game build pass; game unit tests pass and the pre-existing map_tangents failure remains.
All commands use `target/alt` and four jobs; no commits or protected-path writes were made.

## Mobility step 1 (2026-10-11)

Air control uses the four fall types, world-space stick, per-type gravity, decay, braking and the along-speed cap (`docs/MOBILITY.md`, step 1).
HKS fall height and vertical speed now use cm and cm/s, so the stiff landings play.
Landing from at least `HitMtrlParam.fallDeathDist` (metres assumed) injects DEATH_FALLING and uses the six-second respawn path; falling off the Owl's roof onto the ground 62 m below is such a death.
A fall into the void (a placeholder 40 m below the lowest collision) returns Wolf to where he last stood on collision, as Sekiro's fall return does; its HP cost is in the exe and not traced, so none is taken.
Reviewed replays under `target/alt/mobility/` show a 0.78 m sideways landing offset and a 1284.9 cm short stiff landing at m11 start 4 (the route has an intermediate landing; a 16.8 m sim regression verifies 1680 cm and clip 200020).
Formatting, workspace Clippy, all sim tests and the alternate release build pass; the game's pre-existing `map_tangents` failure remains.

## Boss brain token observability and budgets (2026-10-11)

Tokens are the primary metric in fight summaries, session reports and the recording overlay.
Subscription Codex/Claude usage is labelled as a plan proxy, with no computed or displayed dollar estimate.
API-key requests retain billed USD estimates from the editable `config/brain-prices.toml`, with standard OpenAI model/pricing and Decisions source URLs checked on 11 October.
The price table is whitelisted for tracking; unknown API models show unknown USD.
The task-specified Haiku seed retains an explicit cache-pricing TODO and is never applied to subscription Claude.

Codex JSON completed-turn events supply tokens while `-o` supplies the final plan.
Reported CLI usage now survives an invalid plan, so failures cannot bypass the token budget.
Telemetry keeps legacy fields alongside GenAI semantic names and `sekiro.*` extras, including billing and the plan-proxy flag.
Fight summaries print both fight and session totals; the report Cost section groups kind/model and separates plan tokens from API tokens and billed USD.
F8 displays the session totals and persistent budget notice in a smaller overlay footer.

`--brain-token-budget N` counts all reported input plus output tokens; `--brain-budget USD` counts only billed API usage.
Both default to no limit and zero stops immediately.
Budgeted admission is serialized across the decision and coach workers, and queued work is rechecked before starting a call.
After a limit is reached, the current fight retains its plan and the next uses Lua AI.
Unknown billed prices stop a USD-budgeted session; a final completed call can overshoot a limit because usage is known after completion.

Workspace Clippy with `-D warnings`, `cargo fmt --all` and the final formatting check are clean.
The requested release sim/brain-net library, binary and integration suite passed all 368 tests across 37 targets; its log is `target/alt/brain-cost-release-tests.log`.
Offline regressions cover realistic fake Codex events and final-message separation, cached/reasoning subsets, unknown prices, queued-call stopping, subscription token/USD separation, failed-plan token accounting, current-plan retention and next-fight Lua fallback.
All Cargo commands used `target/alt` and six jobs (fmt uses `CARGO_BUILD_JOBS=6`).
No model/API calls, credential reads, real model CLI execution beyond `codex exec --help`, protected memory/dataset writes, cache/re deletion or links, worktrees or commits were made.
The overlay footer was checked through compilation and driver/format tests; a newly rendered window was not inspected in this session.
The pre-existing game `map_tangents` test was outside the requested suite.

## Where things stand

| Area | State | Docs |
|---|---|---|
| Archives, formats | Whole install unpacks to `cache/raw` in under a minute | `docs/FORMATS.md` |
| Models | Wolf (c0000 + parts) and Ashina soldier (c1010) export as textured, skinned GLB; enemy weapons via draw masks | `docs/FORMATS.md` |
| Animation | All c0000/c1010 clips decode; TAE parsed and decoded; clip aliases resolved | `docs/FORMATS.md` |
| Params | All 186 tables load from Paramdex defs; typed combat tables | `docs/PARAMS.md` |
| Player behaviour | Real `c0000.hks` runs in our HKS VM against the real behaviour graph; TAE cancel windows, blending, lock-on | `docs/HKS-INTERFACE.md`, `docs/HKS-COMBAT.md` |
| Movement | Run, sprint, jump (running jump fixed), step, crouch; root motion; walks the real m11 map with floor/wall collision | `docs/FORMATS.md` (Walking on the map) |
| Combat | Engine rules for hit/block/deflect, posture both ways, breaks, recovery; HP/posture HUD | `docs/COMBAT-RULES.md` |
| Deathblows | Real ThrowParam pairs; posture break -> deathblow -> death; Wolf death and respawn; the deathblow camera from the clips' TAE 151 / 153 / 155 | `docs/DEATHBLOW.md` |
| Enemy | Soldier runs its own HKS script, behaviour graph and real Lua AI (Lua 5.0 VM + goal runtime); real combo cancels | `docs/AI.md` |
| Great Shinobi Owl | c5060 (not c5070, his bird) with his own HKS, graph, TAE, params and Lua AI on the flat floor and on the Ashina Castle top where the game places him (`--arena owl`); shuriken bullets; two deathblows with the get-up and finishing pairs; his grabs (3040, 3041) throw Wolf from the data (ThrowParam 25060000 / 25060100); phase 2 (heat-up, SpEffect 200051, boss bar, defeat) runs from the map's own event script (`event/m11_01_00_00.emevd`) | `docs/OWL.md` |
| Owl's tools | Bullets with child bullets, flight from the Bullet params, area and lingering hits, shared hit records; poison build-up and damage over time, recovery seal, blinding, lock-on break; warp clips fade him out, unhittable, when forced (his castle AI never warps); simple stand-in looks and a HUD status line | `docs/OWL.md` (Tools) |
| Boss brain | The Owl's and the Ashina soldier's act choices can come from an outside controller (`--brain lua\|scripted\|decisions\|responses`, `--brain-log`): plans of his own action ids run through his battle goal, reflexes stay with his Lua AI, mirrored observation, this fight's outcome tally, and shared boss knowledge in `logs/memory/<chr>.boss.json` and Wolf's habits/history in `<chr>.player-<name>.json` (`--brain-memory`, `--player`) | `docs/BRAIN.md` |
| Audio | The game's own FMOD Ex runtime from the install plays the game's event projects (`main`, the enemy's `cXXXX`, the map area's): TAE sounds with floor-material footsteps and armour rustle, hit, block and deflect clangs from the hit-effect params, listener on the camera with the events' own 3D rolloff; decoded samples only as a fallback | `docs/AUDIO.md` |
| Map | All ten archive maps extracted and loadable; 69 starts across nine gameplay maps walk; m89 is sky-only | `docs/MAP-BASELINE.md`, `docs/FORMATS.md` |

## Combo-aware boss session (2026-10-11)

Boss brain installation now computes a move sheet from the loaded TAE/clip/AtkParam data, including attack groups inside offered tactic plans.
Coach and mid-review input contain hit times/counts, guard/deflect posture forecasts from the shared combat resolver, clip duration, recovery, conditional cancel windows and nominal capsule/root-motion reach.
The generated Owl example and window-query mapping are in `docs/BRAIN.md`, "Combo-aware move sheet and break log".
Complete fitted Owl coach inputs measured 3916 to 4067 bytes, within the 4096-byte budget, with the current combo and break evidence retained.

Every Wolf posture break records the boss action/combo and hit index/count, state, remaining clip-clock wait, actual response start/delay and outcome.
Per-action/hit conversion totals reach the coach and mid-fight review; session reports include the totals and individual break records.
`break_conversion_delay` is mean seconds to the first landed conversion per converted break (null without one), in metrics, trend, coach result and scorecard.

The normal middle-hit Owl response already used the real TAE window in the baseline.
A repeated-current-attack response reproduced a 14.8-second late start because the runtime mistook the old clip for a new response.
It now tracks the clip clock and keeps combo cancellation enabled until the new attack starts.
Both the ordinary and repeated punish start 0.300 seconds after the middle hit, through the 0.667-second TAE window before the 2.733-second natural end.
The fixture with cancel flags removed only from an in-memory timeline starts at the first free frame, 2.400 seconds after the break.
Long data-defined recoveries also retain the queued response past the ordinary queue deadline.
Coach guidance asks for last-hit or fast-single breaks, a `finisher` set for `wolf_posture_75`, and review of failed conversions.

The final requested release sim/brain-net suite passed 360 tests across 37 targets, including the concurrent cost-accounting integration.
Final targeted Owl verification passed all 12 cases after adding the attack group inside a tactic plan, including AP1/AP6 guard/deflect forecast equality and frame-boundary assertions.
Unit regressions cover allowed/conditional/expired windows, all break outcomes, mean conversion delay and legacy defaults.
Workspace Clippy with `-D warnings` and formatting are clean.
Logs are `target/alt/combo-final-tests.log`, `combo-owl-tests.log`, `combo-unit.log` and `combo-clippy.log`.
All Cargo commands used `target/alt` and six jobs (fmt uses `CARGO_BUILD_JOBS=6`).
No model/API calls, real model CLIs, credential reads, protected memory/dataset writes, cache/re deletion or links, worktrees or commits were made.
Attack timings, hit stop and Wolf's input buffer were unchanged.

## Recording brain overlay and stages (2026-10-11)

`--brain-overlay` starts the top-right recording panel; it is off by default and F8 toggles it.
It shows the controller/model, fight/player, active set and description, current plan step and move label, checkpoint choice and age, coach thinking/notes/principles, recent bans and the post-fight scorecard.
Coach notes last eight seconds, bans five seconds and scorecards six seconds.
The wrapped, fixed-width panel uses a dark translucent background and Segoe UI, hides the debug readout while visible, and reserves space from the floating boss bars.
The driver view is independent of decision logging.

`--brain-stage lua|early|alive|outsmarted` selects the recording baseline or an isolated coach snapshot.
Lua has no memory; early uses `owl-2026-10-11-golden`, alive uses `owl-2026-10-11-alive`, and outsmarted uses `owl-2026-10-11-outsmarted`, each from `logs/datasets/<dataset>/c5060.json`.
Coach stages copy and migrate into a unique retained temporary directory, print its path, and direct all memory writes there, overriding normal brain and memory flags.
Missing/invalid datasets produce a clear error.
Named players receive the snapshot's habits/history without counting its boss evidence twice.
`--window-size WIDTHxHEIGHT` sets the play window size (default 1600x900, minimum 800x600).
PowerShell commands for each stage are in `docs/BRAIN.md`, "Recording overlay and stage presets".

Final workspace Clippy with `-D warnings`, formatting and diff whitespace checks passed.
The requested release sim/brain-net library, binary and integration suite passed all 353 tests, and all 11 game binary tests passed.
Offline tests cover transient card timing, newest coach notes, actual move labels, shared ban reasons, missing datasets, source preservation and migration of all three real stage snapshots into temporary copies.
The pre-existing game `map_tangents` test was not part of the requested suite and was left alone.
All Cargo commands used `target/alt` and six jobs; fmt uses `CARGO_BUILD_JOBS=6` because it has no `-j` option.
The final release executable is `target/alt/release/sekiro-rs.exe`; the user's release and preview executables were not replaced.

Scripted Owl fights with memory off were checked at 1920x1080 and 1280x720; every run completed under a 90-second cap.
Reviewed final screenshots: `target/alt/overlay-review/brain-1080-final-150.png`, `brain-1080-final-300.png`, `brain-720-final-150.png` and `brain-720-final-300.png` in that directory.
The raw-input F8 replay (`crates/game/scripts/brain_overlay_toggle.txt`) verified default off, on and off again, reviewed in `target/alt/overlay-review/toggle-off.png`, `toggle-on.png` and `toggle-off-again.png`.
Initial flat-floor screenshots `target/alt/overlay-review/brain-1080-150.png` and `brain-1080-300.png` were also reviewed before the font and debug-readout polish.
The requested suite log is `target/alt/overlay-review/final-sim-net-tests.log`.
No real model/API calls, real model CLIs, credential reads, protected memory/dataset writes, cache/re deletions or links, worktrees or commits were made.
Live hosted coach calls were deliberately not exercised; coach events were tested offline.

## Map baseline commands

Use `target/alt/release/sekiro-rs.exe --map <id>` for the viewer and add `--play` to walk.
The alternate build leaves the user's default release executable alone.

| `--map` ID | Area | Suggested `--start` |
| --- | --- | ---: |
| m10_00_00_00 | Hirata Estate | 0 |
| m11_00_00_00 | Ashina Outskirts | 3 |
| m11_01_00_00 | Ashina Castle | 2 |
| m11_02_00_00 | Ashina Reservoir | 0 |
| m13_00_00_00 | Abandoned Dungeon | 0 |
| m15_00_00_00 | Mibu Village / Ashina Depths | 3 |
| m17_00_00_00 | Sunken Valley / Sunken Valley Passage | 0 |
| m20_00_00_00 | Senpou Temple, Mt. Kongo | 0 |
| m25_00_00_00 | Fountainhead Palace | 1 |
| m89_00_00_99 | Sky-dome library, no floor | Viewer only |

Map extraction has zero failed models after handling the castle's HKX-only object binder.
The sky library now retains its collisionless zero-group pieces and resolves its cinematic GPARAM.
Initial viewer cameras now clip against nearby collision; the castle and Mibu obstructed starts were verified after rebuilding.
Final alternate release build, workspace Clippy, workspace formatting, formats/extractor release tests and both map regressions passed; details and screenshots are in `docs/MAP-BASELINE.md`.

## How to run

```powershell
./tools/fetch-refs.ps1
cargo run --release -p sekiro-extract -- unpack
cargo run --release -p sekiro-extract -- models
cargo run --release -p sekiro-extract -- anims c0000 c1010 c5060
cargo run --release -p sekiro-extract -- params
cargo run --release -p sekiro-extract -- map m11_00_00_00
cargo run --release -p sekiro-extract -- map m11_01_00_00
cargo run --release -p sekiro-game -- --play --map m11_00_00_00 --start 3
cargo run --release -p sekiro-game -- --arena owl
```

Useful flags: `--attack-power N` selects Wolf's Memory attack power (1 to 99, default 1); `--progression castle-roof` selects AP6, overridden by explicit `--attack-power`; both also work in `duel-sim`; `--play` alone uses a flat floor; `--enemy owl` (or `c5060`) fights the Great Shinobi Owl instead of the soldier; `--arena owl` fights him on the castle top where the game places him (only the map within 60 m; `--full-map` loads all of it; `docs/OWL.md`, "The castle top"); in play, `--hide <model prefixes>` skips map piece models and `--objects` adds MSB objects; `--lock-on`, `--no-enemy`, `--simple-ai`, `--script FILE --shots step:path --exit` for scripted runs with screenshots; `--input FILE` replays raw keys and mouse through the live input path and logs camera and movement per frame and step (`crates/game/src/input_replay.rs`); `--arena --deathblow` for the deathblow demo; `--viewer <glb>` and `--map <id>` for inspection; `--game <dir>` (or `SEKIRO_DIR`) points at the install whose FMOD DLLs play the sounds; `--volume 0.0..1.0` sets the master volume (default 0.5), and in play minus and equals (or `[` and `]`) change it in 10% steps with a short on-screen readout.
Testing aids: `--break-posture` holds the enemy's posture at 1 point outside deathblows, so Wolf's next landed hit breaks it (quick deathblow tests, also for the Owl's second life); `--spawn x,y,z,yaw,pitch` now also works in an arena, putting Wolf there and the enemy `--enemy-distance` metres (default 5) in front of him, for example next to the castle-top railing (`--arena owl --passive --god --break-posture --spawn 101.23,53.95,247.33,135,0 --enemy-distance 1.6`); `--god` makes Wolf take no HP or posture damage (hits, reactions, hit stop and knockback unchanged), so long replays are not cut short by his death; `--passive` (same as `--no-enemy`) keeps the enemy standing there with his AI off; `--force-act <id>` makes the enemy request that act whenever he is free (with `--passive`, only that act; `--force-act 3040` is the Owl's grab, script `crates/game/scripts/owl_grab.txt`); the Owl's tools on demand: `--arena owl --passive --god --force-act 3010 --spawn 104.5,54.0,244.06,135,0 --enemy-distance 4 --script crates/game/scripts/owl_tools.txt --shots ... --exit` (3010 firecrackers, 3017 explosive slash, 3030 poison, 3031 recovery seal, 3069 smoke; always give `--shots` paths under the repo, a shot that cannot be saved keeps the run from exiting); `PLAY_LOG_COMBO=1` logs one `combo` line per step (attack button, buffered presses, Wolf's state, clip and clip time, both hit stops, the step's hits, the enemy's state and clip, distance).

## In progress when the session paused

- Grappling hook and ledge hang (player/enemy behaviour).
- Fall death and respawn, FLVER tangents to cut the 14 s map load, map lighting from the game's draw params, foliage alpha.
- Audio (2026-10-10, FMOD path built, to be confirmed by ear): sounds at the TAE events' dummy polys, map reverb, music. Coverage and loudness numbers: `docs/AUDIO.md`.

Uncommitted files in the working tree belong to these tasks; check `git status` and build before committing.

### Leads for grapple and ledges (found, not built)

- Ledges: `map/<id>/<id>.edgebhd/.edgebdt` hold one `.edge` file per collision part (16 files, 255 edges on m11). Header `u32 version=4, u32 count, u32 collision number, u32 0`, then 64-byte records: two vec4 endpoints, a zero vec4, `u32 chain id`, `u32 flags` (low byte 2 or 3, second byte 0..3 or 10, meaning unknown), 8 bytes padding. Endpoints are in the collision part's local space; apply its layout transform. `.wallbhd/.wallbdt` is likely wall-hug data, undecoded.
- Grapple: map pieces have no dummies; object `o000100` (placed 67 times on m11) carries dummy 90 and is very likely the grapple point. Search rules: WirePointSearchParam row 0 (act 3 to 15 m, find to 25 m, filter and soft-lock boxes); WireSetParam picks search rows per area; WireVariationParam (angles, marker type, homing offset) is probably set per placed object in MSB fields the parser still skips.
- Mikiri and perilous warning are implemented; see the counter session below and `docs/COMBAT-RULES.md`.
- Study tools: `crates/sim/examples/bxf_list.rs`, `dummy_census.rs`.

## Decision needed from the user

- 62 of 174 sound banks (`sm*`, `smain`, `vm*`, `xm*`, `rm*`: area SFX, voices, music, cutscenes) are encrypted FSB that the game's FMOD cannot open without the game's key. Footsteps, hits, clangs, Wolf's and the Owl's sounds are in the plain banks and play; the encrypted ones stay silent. Reading them means decrypting with a key from the game; ask before touching them (see `docs/AUDIO.md`).

## Mikiri Counter session (2026-10-11)

Soldier act 3013 and Owl act 3033 now counter when Wolf steps forward into their thrust during his real TAE ChrActionFlag 125 window, 0.000 to 0.400 s.
The incoming blow and on-hit effects are cancelled, and the contacted AtkParam's `staminaDamageAttackHitParry` deals 90 posture to the enemy.
ThrowParam suffix 190 plays Wolf's per-enemy `511800` counter and enemy `ThrowDef13800`; suffix 120 plays `511100` / `ThrowDef13100` when the damage breaks posture (the soldier by default).
The pair uses its actual absorb dummy and 0.5 s interpolation, and releases through Wolf's existing animation cancels or end, with env 273 / 206 supplied to the enemy throw hook.
No input buffering, movement, attack timing, hit stop or combo logic changed, and no fixed delay was added.
FwLo's RustSekiro supplied the rule and pair selection, checked against our extracted data and credited once in its existing CREDITS section; the requested pull failed because this environment could not connect to GitHub.
Details, data versus inferred rules, and limitations (latent skill multipliers and automatic posture-break deathblow follow-up) are in `docs/COMBAT-RULES.md`, "Mikiri Counter".
The real-data reproduction first failed with Wolf hit at tick 46; the fixed 2 m replays counter for both enemies and verify their reaction states, Wolf's animation, no incoming damage, and release.
The ten perilous tests also cover early/late, backward/side, unlocked stepping away, sweeps and ordinary slashes, and confirm the 0.400 s data window and counter dummy rows.
The full release sim suite passed (268 tests); workspace Clippy with `-D warnings`, formatting and diff whitespace checks passed.
Owl's smoke blinding hit remains a normal hit and its existing regression passes.
Logs: `cache/mikiri-tests-final.log`, `cache/mikiri-focused.log`; scripted demo: `crates/game/scripts/soldier_mikiri.txt`.
The game release executable was built in `target/alt`.
The soldier demo completed in about eight seconds under a 55 s cap, saving `cache/screens/mikiri_soldier_55.png` and `mikiri_soldier_70.png`.
Both screenshots were inspected; the blade stomp is visible, and `cache/mikiri-game.err.log` confirms `ThrowAtk511100` / `ThrowDef13100` from tick 47 with no hit stop.
All Cargo builds and tests used `target/alt` and six jobs; cargo fmt uses `CARGO_BUILD_JOBS=6` because it has no `-j` option.
No commits, links, junctions, protected brain edits or game-install writes were made.

```powershell
.\target\alt\release\sekiro-rs.exe --play --passive --god --force-act 3013 --lock-on --enemy-distance 2 --script crates/game/scripts/soldier_mikiri.txt --shots 55:cache/screens/mikiri_soldier_55.png,70:cache/screens/mikiri_soldier_70.png --exit
```

## Known issues

- Done 2026-10-11, to be tried in play: the boss **coach** (`--brain coach`, `docs/BRAIN.md` "The coach"). Between fights a reasoning model (`--coach claude|codex|api`, default `api` = Responses, `gpt-6-luna`, low effort, 45 s; `claude` = slim `claude -p --model claude-haiku-5-5`, no API key, `--coach-model`, `--coach-effort`) reads the fight summary (about 4 KB: result, trend, per-plan and per-set outcomes, Wolf's habits, the plan played, its own notes) and writes a versioned game plan (opener, 2 to 4 named plan sets, start set, avoid, notes), validated against the boss's actions. Fight 1 with no memory starts with his own Lua AI and can be reviewed mid-fight; the plan arriving during the respawn plays from fight 2 (opener, then the start set), a late one a fight later, a failure keeps the old plan, and the newest plan is saved in `logs/memory/<chr>.json`. In the fight only the Decisions API picks a set at the checkpoints, with a tiny request (about 100 tokens of numbers plus the set names). The console prints the request and a 3 to 5 line summary of each plan. Against reward hacking: winning faster is the first goal; new metrics `attacks/min`, `damage/min` and `passive_share` (a fight above 35% passive failed), and hard limits in the driver (an attack at least every 6 s, one retreat per 8 s, plans need an attack). Also fixed: an attack after `retreat12` or a strafe swung at empty air; it now turns to face Wolf first (30 degree rule). Tests `crates/sim/tests/coach.rs`, `crates/sim/src/brain/tests.rs`, `crates/brain-net/tests/offline.rs` (mock servers and a fake CLI only). Not verified: any real API, `claude` or `codex` call (flag names are from `--help` output), that the strict schema with `$defs` is accepted, that 6000 output tokens suffice for `medium` reasoning, how the plans look in play.

- Done 2026-10-10, to be confirmed in play: three features after FwLo's RustSekiro video (his code adapted under MIT, `CREDITS.md`). (1) The deathblow camera: Wolf's deathblow clips drive it through TAE 155 (CameraSetParam slot 41, CameraParam 4000), 153 (distance, 1.5 m at the soldier's blow) and 151 (focus on dummy 142 of his animated camera bone, with look limits; the Owl's deathblow), and it hands back on the params' own chase rates (`docs/DEATHBLOW.md`, "Camera"). (2) The perilous kanji: the warning dummy bullet (judge 980 to 989) is made at Wolf within 5 m (soldier) or 9 m (Owl), its SpEffect 211000 shows VFX 48100 "危" at Wolf's dummy 220, and the HUD draws a red 危 over Wolf with a Windows Japanese font (the game's FXR is not decoded; no sound found). (3) The jump counter: Wolf's jumps carry sweep i-frames (TAE 954 type 1), his air kick (PCBehavior 901, anti-ground) is now a real attack, the sweeps' SpEffect 400200 multiplies its posture damage by 8 (the soldier's posture breaks), and env 2004 lets his script kick off the enemy (`docs/COMBAT-RULES.md`, "Perilous attacks"). Tests `crates/sim/tests/perilous.rs`, `deathblow_camera.rs`; scripts `crates/game/scripts/soldier_deathblow.txt`, `soldier_jump_counter.txt`; screenshots `cache/screens/dbcam_soldier_*.png`, `dbcam_owl2_*.png`, `kanji_soldier_*.png`.

- Done 2026-10-11, to be tried in play (user's first real soldier fight: the model chose the leaping slash 3003 twelve times although Wolf deflected it 5 of 5): the boss brain now probes instead of repeating. (1) Plans are 1 to 3 steps, each an attack or a movement *tactic* run through the engine's own native goals (hold, guard, walk_in, run_in, strafe, back_off, retreat, keep_range, relative to Wolf, each with a lifetime cap); the soldier and the Owl offer 73 and 75 choices (his actions, combos, 15 tactics, about 40 curated chains), text form such as `walk_in4>hold2>3013`. (2) Request modes `slot` (as before), `plan_end` and `checkpoints` (hosted default: only at fight start, half HP, a lost deathblow mark, Wolf punishing twice or deflecting three times in a row; a fight takes 1 to 5 requests); a reply is a plan set (the Decisions probabilities) the boss samples locally at every act, with a penalty for options repelled repeatedly this fight and a bonus for untried ones (temperature, seed by environment variables). (3) Per-fight metrics for Wolf (perfect-deflect and guard rate, hits taken, HP and posture lost, punishes, kill time) are kept in the memory file (version 2, last 10 fights), sent as a trend, and printed as one summary line per fight; `--brain lua --brain-memory on` saves the baseline to `logs/memory/<chr>.lua.json`. (4) Nothing is requested or logged once a fighter is dead; the hosted worker warms the route (DNS and TCP, no request) and its timeout is 3 s. `docs/BRAIN.md`, tests `crates/sim/tests/brain_tactics.rs`, `crates/sim/src/brain/tests.rs`, `crates/brain-net`. Not verified: any live API call (probability field names come from the docs), how the tactics look in play (distances and caps are tested, the animation blends are not).
- Done 2026-10-10, to be tried in play: the Ashina soldier (c1010) can be driven by the boss brain, as a testbed for whether a hosted model plays better than the game's AI. His action table (`brain::actions::SOLDIER`, from `101000_battle.lua`, his BehaviorParam and AtkParam rows) has 13 attacks and 2 back steps with his acts' approach distances, his rushes' AI number, combos `3000+3001` and `3005+3007`, and reflex acts 23, 25 and 27; his script also names 3004, 3006, 5202 and 5203, which his TAE has no clips for, so they are refused. The driver, observation (`boss`, `boss_bearing`, `since_boss_hit`), tally and hosted requests are generic now (question `boss_action`, instructions naming the enemy). Memory: each fight's tally and Wolf's habits (attack distance, range, his response after deflects, guards and back steps, his answer to and punish speed for each boss action, heals, jumps) go into `logs/memory/<chr>.json` (version 1) at each death, respawn, every 15 s and on exit; requests carry this fight and all fights within 1200 bytes. Try: `.\target\release\sekiro-rs.exe --play --enemy soldier --brain scripted --brain-memory on --brain-log`. `docs/BRAIN.md`, tests `crates/sim/tests/soldier_brain.rs`, `crates/sim/src/brain/tests.rs`, `crates/brain-net`.
- Fixed and checked in play 2026-10-10: the soldier's missing sword was a stale combat draw mask exported before `anims`, leaving group 0 hidden.
  Model extraction now reads weapon-draw TAE 233 directly from the unpacked character TAE, so `models` can run before `anims`.
  Regenerated `cache/models/c1010.glb`; both sword meshes follow `Kodachi` under `R_Hand` with unit scale and now have inherited visibility.
  Regression: `models::tests::soldier_combat_mask_does_not_require_extracted_animations` in `sekiro-extract`.
  Checked screenshots: `cache/screens/sword_duel_before.png`, `sword_duel_after.png`, `sword_roof_verified.png`, `sword_roof_attack.png`, and `sword_owl_{012,040,110,250}.png` (Owl's sword and warp hiding still work).
  Final alternate release captures also checked: `cache/screens/sword_final_duel.png` and `sword_final_owl_{012,040,110,250}.png`.
  Temporary entity diagnostics, removed from source, remain in `cache/screens/sword_mask_before.err` and `sword_roof_verified.err`.
  Release build, extractor regression, full sim release suite and workspace Clippy passed in `target/alt` with six build jobs.
  Touched Rust files pass formatting; the workspace formatting check still reports other agents' active brain, camera and perilous changes.

- Done 2026-10-10, to be confirmed in play: the Owl's phase 2. An EMEVD reader (`crates/formats/src/emevd.rs`, names from DarkScript3's Sekiro EMEDF) and a small event interpreter (`crates/sim/src/events.rs`) run his fight's own events: the first deathblow's get-up raises event message 20 (TAE 231), event 11115870 hides the boss bar, sends HKS command 506000 (heat-up `a000_020005`, then the kneeling loop `a000_021003`) and gives SpEffect 200051; his AI escapes with smoke (20006 when Wolf comes within 2 m, 20007 when Wolf attacks, 20008 after five loops), which resets his HP and brings the bar back; with 200051 his kick follow-up is 3030 poison instead of 3007; the finishing clip's message 50 handles the defeat. New engine answers: env 105 (event command), 206 (in a throw), 123 (AI endure cancel), observe areas, and melee AtkParam SpEffects on the defender. Open: while he begs at 1 HP our duel allows a deathblow (unknown in the game); the heat-up's judge 910 blows Wolf far away, so 20006 rarely fires in practice; boss music and map effects are not run (`docs/OWL.md`, "Phase 2").
- Done 2026-10-10: the Owl's warp and vanish. Finding: in the data the castle Owl (c5060, battle goal 506001, map events m11_01_00_00) never vanishes or teleports; the firecrackers' SpEffect 3506021 "attacking warp" is only acted on by the Hirata event script (Owl (Father) swapping with his bird), and his castle AI never asks for his fade-out clips. Those clips (3037, 3039, 3044, 3092, 3093) now work when forced: TAE 193 fades him out and in, ChrActionFlag 94 makes him unhittable while gone (same exe bit as TAE 951), TAE 760 carries him toward Wolf, and the renderer hides him with a grey puff where he goes and comes back. Try `--arena owl --passive --god --force-act 3044 --enemy-distance 8`. `docs/OWL.md` "Warp and vanish", tests `crates/sim/tests/owl_warp.rs`, screenshots `cache/screens/warp_3044_*.png`.
- Done 2026-10-10, to be confirmed in play: the Owl's grabs (3040 collapse throw, 3041 see-through counter) played his lunge but never caught Wolf. Enemy throws on Wolf now run from the data: a `throwFlag` 1 contact (his left-foot sphere) starts ThrowParam 25060000 / 25060100, Wolf is absorbed onto the Owl's dummy 233 / 234 and plays his thrown clip from `c0000_c5060.anibnd` (`a221_600110` / `600120`), the damage lands at the Owl's `ThrowAttackBehavior` events (192 HP; 32 then 208), Wolf is invulnerable in the clip (TAE 952) and gets up in it, or dies into `a221_600111`. Guard and deflect do not stop it; stepping out of reach, step i-frames (type 7) and jumping do. No grab has a warning bullet in the data, so no red kanji is shown. `docs/OWL.md` "Grabs", `docs/DEATHBLOW.md` "Enemy throws on Wolf", tests `crates/sim/tests/owl_grab.rs`, screenshots `cache/screens/grab_{open,rail,guard}_*.png`.
- Fixed 2026-10-10, to be confirmed in play: combos on the Owl froze and jittered. Every direct hit froze both characters for 0.1 s, which the game does not do against the Owl (his NpcParam hit stop types are 0), and the drawn animation slid backwards during the freeze; the fight also hitched about 2 s in, on its first step and on first-shown clips. See "Frame pacing" below.
- Fixed 2026-10-10, to be confirmed in play: during the Owl's deathblow the stick turned Wolf and the Owl's body followed; near walls the pair clipped in; the game froze for about half a second as a deathblow started. Causes: Wolf's throw states pick their clip by animation offset type 17, which was never set to ThrowParam `atkAnimOffset` (221 for the Owl), so Wolf played the soldier's 2.0 s `a200_510000` and was free after 2 s while the Owl (6.6 s clip) was still held on him; the defender was held on the dummy of Wolf's live pose every step, teleported and through walls; a knockback left from the breaking hit resumed when the pair ended; the push-apart ignored collision; and every deathblow start parsed c0000's 23 MB `tae.json` (0.45 s). Now the offset types are set from the row, the absorb places one character once at the body clip's frame 0 and both play their own root motion (after FwLo's RustSekiro), all of it moves through collision (a blocked absorb shifts the other character instead, inferred), and throw times come from the decoded TAE (deathblow-start frames now 16 to 18 ms, were a 250 ms clamped frame). Wolf can walk out from 4.0 s, where his clip's TAE opens the move cancel. `docs/DEATHBLOW.md`, tests `crates/sim/tests/deathblow_lock.rs`, screenshots `cache/screens/deathblow_{steer,owlwall,wolfwall,owlrail,wolfrail}_*.png`.
- Fixed 2026-10-10, to be confirmed by ear: nearly no sounds in the Owl fight and volume that jumped near walls and corners. The Owl's bank was never decoded and Bevy's spatial audio scaled every sound by the camera's distance with no minimum. Sounds now play through the game's own FMOD (`crates/fmod`, `crates/game/src/audio.rs`): all 55 of the Owl's TAE sound keys play, and the events' own rolloff is flat out to about 5 m. The hit-sound lookup also used the wrong param fields and letter; fixed from FwLo's exe trace. `docs/AUDIO.md`.

- Fixed 2026-10-10, to be confirmed in play: locked on, diagonal walks and runs (W+A, S+D, ...) went straight forward, back or sideways. The script picks one of four strafe clips by 60 degree cones (`MoveDirection`), and the engine applied that clip's straight root motion; the game's exe also sets `TwistMasterAngle` to the stick's angle minus the clip's direction, and the graph turns the Master bone by it and turns Spine..Spine2 back (WalkTwist, traced by FwLo's RustSekiro, `docs/kb/movement.md`). Now `Character::walk_twist` eases to that angle at 0.1 rad per 1/30 s, locked walk and run root motion is turned by its target (so travel follows the stick even while the twist catches up after a cone change; our choice, not traced), and the renderer turns the legs by it with the chest kept on the target (`apply_walk_twist` in `crates/game/src/character.rs`). The forward diagonals that "turned away" were the same bug: running straight at a target 6 m away, Wolf ran through it and the lock-on facing swung round. Tests: the `locked_diagonal_*` cases in `crates/sim/tests/camera_turn_movement.rs` (target moved to 20 m, since holding 45 degrees off the target bearing spirals in and the test has no enemy body) and `walk_twist_turns_the_hips_and_keeps_the_shoulders` in the game crate; app run: `--arena owl --passive --god --input FILE` with W+A then S+D logs travel 0 degrees off the stick and `walk_twist` near -48 and -42 (`cache/screens/diag_*.png`, camera behind Wolf).
- Mouse look was far too sensitive (reported 2026-10-10); `MOUSE_SENSITIVITY` in play.rs is now 0.0012 radians per mouse count (was 0.0035), not yet confirmed by the user.
- Fixed 2026-10-10, to be confirmed in play: movement "inverted / tank-like" after turning the camera. Free move starts picked the locked-on strafe clips (back, left, right) from the stick's angle to Wolf's facing while steering turned him to the stick, so he side-stepped or ran backwards; now a free move start turns the body to the stick and keeps the turn as the lower-body twist (`TwistLowerRootAngle`, drawn as a smooth turn), so the script picks the forward start or its 180 degree turn-and-go. That snap-and-twist model is inferred from the script and graph, not traced in the exe. Locked-on strafes now follow the stick (engine `MoveDirection` with the WalkTwist cones, and bound graph selectors re-read when a variable changes). Tests: `crates/sim/tests/camera_turn_movement.rs`.
- Fixed 2026-10-10, to be confirmed in play: mashing attack on the Owl "repeated the first slash and stun-locked him". Wolf's part is the game's: when the Owl guards (`a000_003100`), Wolf's blocked slash recoils (`EasyDeflected`) and the script's follow-up is the quick `EasyDeflectedAtk` (`a050_305000`). The bug was the Owl's: env 258 (his NpcParam `guardLevel`, 4) read 0, so his script's guard table gave no guard reaction and every blocked slash played the hit flinch `a000_008010`, with no guard window and no consecutive-guard count, and he never got out to counter. He now plays `GuardDamageSmall` (`a000_008501`) and counters. Tests: `crates/sim/tests/combo_on_hit.rs`; app check: `PLAY_LOG_COMBO=1 sekiro-rs --arena owl --god --input FILE` with W held and left click mashed.
- Fixed 2026-10-10: after Esc freed the mouse, the camera ignored it until Esc again (clicking did nothing); a click now takes the mouse back (and does not attack). Mouse motion during map loading no longer turns the start view. Wolf and the enemy are drawn between simulation steps, so frames with no or two steps no longer stall or jump.
- Fixed 2026-10-10, to be confirmed in play: locked on, holding a direction and dodge stepped that way, then sprinted forwards or backwards. Causes: the engine ignored the script's `act(2018)` (release the lock-on angle lock, asked on every sprinting frame), so steering kept turning Wolf to the target and the script's locked-sprint brake (|MoveAngle| >= 135) fired over and over; root motion samples are displacements in the clip's start frame but were applied as if in the current facing, so a sprint start that turns 90 or 180 degrees (a000_00115x) carried Wolf sideways; `act(3025)` turned the wrong way and ran before `act(2019)`, so diagonal step tilts went off the stick; the locked action angles (Kick/Jump/Attack/SubAttackAngle) were measured from the old facing although the script reads them right after facing the target. `TurnAngle` is now measured from the root-motion travel direction (the facing when still), which lets the back sprint start (a000_001152) play instead of braking into the sprint stop slide toward the enemy; that model is inferred from the script and clips, not traced in the exe. Tests: `crates/sim/tests/camera_turn_movement.rs` (locked step, diagonal tilt, locked sprint in four directions, release).
- The lock-on camera loses Wolf while he sprints locked on: he leaves the frame, or the camera ends up inside him near the castle-top railing (`cache/screens/locksprint_*.png`, 2026-10-10). Camera framing and collision, not movement.
- Wolf's max HP/posture come from CalcCorrectGraph at progression level 1; the real progression input is not yet traced.
- Player HP damage formula is inferred (weapon attack power x correction), not traced.
- Single-clip rendering: layered upper/lower-body states show one clip (FwLo's RustSekiro plays them with the graph's bone weights; next to port).
- Auto-aim has no line-of-sight check; knockback column choice per damage level is matched by name, not traced.
- Shading is plain PBR; map lighting is hand-picked; 3 map textures from other areas are missing.
- Deathblow needs lock-on to meet the angle limit.

## Next steps

- Deathblow presentation (user, 2026-10-10): the deathblow camera is done; next camera shake (TAE RumbleCam 144-147 -> rumblebnd camera_NNN.hkx), then particle effects (FFX/FXR: blood on deathblows and hits, sparks on clashes, Owl's smoke and firecrackers). See FwLo's docs/kb/camera.md for the camera rules.
1. Owl on the castle top (`docs/OWL.md`, "Next steps"): the fight runs there (`--arena owl`) with his tools, grabs and phase 2 (from the map's event script, which also confirms the intact tatami floor `m015070`); next the boss music switch (encrypted banks) and a capture to check the heat-up against.
2. Finish the three in-progress tasks above.
3. Validation against the real game: record posture/HP/animation traces in a real fight (read-only) and compare with `duel-sim` output.
4. More enemies from the m11 layout (43 soldiers placed in the MSB).
5. Combat arts, prosthetics and items only after the core loop is solid.

## Publishing

The repo is public at github.com/AKJama/sekiro-rs.
Local git hooks (`.git/hooks/pre-commit`, `commit-msg`) reject personal paths, emails, co-author lines and anything under `cache/` or `re/`; they are not part of the repo, so re-create them on a new clone.
Write commands in docs as plain `cargo`, never a full user path.

## Frame pacing (2026-10-10)

`PLAY_LOG_PERF=1` writes one CSV row per rendered frame to `cache/perf/` (`crates/game/src/perf_log.rs`: wall time, steps, drawn and simulated clip times, hit stops, hits, sounds, and where a slow frame went: main schedule, fixed steps, clip loads, render-world time per render set, pipeline counts).
`tools/perf/owl-replays.ps1 -Exe <exe> -Tag <tag>` replays the same raw input against the Owl (passive, real AI, and a no-contact control with the Owl 1000 m away, each run twice: cold, then warm); `uv run tools/perf/summary.py -v --fail-over 50 cache/perf/<tag>-*warm.csv` prints median, p95, p99, max and every frame over 25 ms with what happened in it, and exits 1 if a frame without a hit takes over 50 ms.
That warm-run check is manual (it needs the game window on a desktop); run it after renderer, loading or audio changes.

Clean runs on an idle machine (RTX 5070 Ti, Vulkan, vsync 60 Hz; gameplay frames only):

| Run | Before (c1782f8): median / p95 / p99 / max ms, frames over 25 ms | After |
|---|---|---|
| passive cold | 16.67 / 17.77 / 18.96 / 57.19, 4 | 16.68 / 17.39 / 17.97 / 20.06, 0 |
| passive warm | 16.66 / 17.72 / 18.80 / 52.54, 3 | 16.67 / 17.43 / 18.32 / 21.16, 0 |
| AI cold | 16.67 / 17.56 / 18.60 / 57.05, 2 | 16.68 / 17.25 / 17.79 / 18.15, 0 |
| AI warm | 16.68 / 17.47 / 18.12 / 52.16, 4 | 16.69 / 17.34 / 18.66 / 30.09, 1 |
| control cold | 16.68 / 17.23 / 17.89 / 52.37, 3 | 16.67 / 17.15 / 18.12 / 30.11, 1 |
| control warm | 16.69 / 17.19 / 18.18 / 55.63, 4 | 16.69 / 17.13 / 17.65 / 18.63, 0 |

The earlier 42 to 52 ms medians in `cache/perf/REPORT.md` were another agent compiling on the same machine.
What the slow frames were, and what changed:

- The contact freezes were hit stop, which the game does not apply in this fight: the defender's NpcParam `hitStopType` scales the attacker's stop and is 0 on the Owl and the soldier, and `hitStopType_Defencer` is 0 on every NPC. Hit stop now follows the exe (a 0.06 s hold, then a quadratic ease over the AtkParam time, scaled per NpcParam type; `docs/COMBAT-RULES.md`, "Hit stop"), so Wolf's hits on the Owl stop nobody.
- Drawn clip time ran backwards during hit stop (by up to 11 ms) because the frames between steps trailed the last step by a fixed step length even when the character's clock had stopped. They now trail by the character's own advance in that step (`StepReport::advanced`), and crossfades advance on the character's clock, so a stopped character holds exactly still and resumes without a jump (test `drawn_clip_time_holds_through_hit_stop`). The backward frames outside clip restarts went from 25 and 10 (passive and AI warm) to 0.
- Every run had a 35 ms and a 52 to 57 ms frame about 1.9 s after the map finished loading, also with no input and with sound off: the render world waits that long for the next swapchain image, with no CPU work, pipeline compile, clip or sound load in it; with map pieces hidden or on the flat floor it does not happen, and with the start held 3 s it still came 1.9 s after loading (cause in the driver or presentation, not found). The fight now starts 2.5 s after the map has loaded (`MAP_SETTLE_SECS` in `play.rs`), so the stall falls in the loading time.
- The fight's first step took 95 ms: the hit-sound tables were read on first use. They and the enemy's FMOD project (c5060.fev, which loaded a few frames into the fight) now load at start.
- Clips shown for the first time were read and decoded on that frame (26 to 55 ms on cold runs, for the Owl's damage and attack clips). Once the fight runs, a background thread decodes all of both characters' clips (0.4 s for Wolf's 1433, no measurable memory change).
- Left: one 30 ms frame in some runs with 16 ms in the main schedule and nothing else in it; the replay writes a stderr line per frame and step into a PowerShell redirect, which likely blocks now and then (not seen without the replay logs, not proven).

## Collision-hole audit (2026-10-11)

The headless tool is `crates/sim/examples/collision_holes.rs`; results and top-ten lists are in `docs/COLLISION-HOLES.md`.
All nine walking maps have JSON under `cache/holes/`, with screenshots and review notes under `cache/screens/holes/` and `cache/holes/<map>.review.json`.
The final audit found 208,241 raw render/collision discrepancies, reproduced 119 with Body, retained 12 after visual review and rejected four visual false positives.
Unreviewed and inconclusive cases remain unconfirmed; raw counts are not gameplay defect counts.
There are 360 real-movement checks and 18,432 seeded capsule probes, each with 64 moves and subdivisions no longer than 0.1 m.
Floor sampling uses 0.5 m cells, slopes up to 50 degrees, a 0.3 m collision tolerance and nearby collision plus line-of-sight filtering.
Body floor confirmation requires more than five ticks over 5 m below the specific render witness while within 1 m horizontally, preventing unrelated ledge falls from confirming a shallow gap.
Large unlabelled background meshes required a conservative collision-neighbourhood voxel mask, and render walls use a BVH to keep memory around 2 GB per worker rather than duplicating triangles across a grid.
The final full scans plus latest cached validation passes total 1,283.46 seconds (21.39 minutes).
The distributed pipeline ran from 23:17:48 to 23:34:20 UTC on 10 October (16.53 minutes to the last full scan), using three gpt-6-luna workers with two Rayon threads each; screenshot review, tuning pilots and build checks are additional time.
Mibu review cameras use `--groups display --hide m9,m8` and stable Body eye positions to avoid occluding far/helper pieces; the headless audit still loads all map pieces.
Confirmed here means a Body reproduction plus a visually retained surface; reachability is a nearby-surface/start-distance proxy, and enclosure parity does not prove watertight solid geometry.

Rerun from the repo root:

```powershell
$env:CARGO_TARGET_DIR='target/alt'
$env:CARGO_BUILD_JOBS='6'
$env:RAYON_NUM_THREADS='6'
& "$env:USERPROFILE\.cargo\bin\cargo.exe" build --release -p sekiro-sim --example collision_holes -j 6
.\target\alt\release\examples\collision_holes.exe cache/maps --spacing 0.5 --probes 2048
.\target\alt\release\examples\collision_holes.exe --report
```

The six release tool tests pass, including the missing floor patch, solid floor and 100 m thin-wall sweep.
Final `cargo clippy --workspace --all-targets -j 6 -- -D warnings` and `cargo fmt --all -- --check` pass with the alternate target configuration above; the tool file was formatted separately to avoid altering protected concurrent edits.
A broader workspace test attempt during concurrent brain edits stopped on a temporary `BrainRequest.brief` compile error; the subsequent workspace Clippy check passes, and no full-workspace test success is claimed.
The known pre-existing `map_tangents` failure was not changed.
Tracked task changes are the new example, dev-only Rayon dependency in `crates/sim/Cargo.toml` and Cargo.lock, `docs/COLLISION-HOLES.md`, and this status section.
No collision engine code, protected brain files, game-install files, links, junctions or commits were changed by this audit.

The concurrent brain agent's commit 9809964 included this shared status section; this audit issued no commit, and the collision tool, dependency changes and report remain uncommitted.

## Playtest fixes (2026-10-11)

No commits were made.
Coach selection now defaults to the Responses API with gpt-6-luna, configurable low reasoning effort and a 45 s timeout; strict schema, store:false and usage accounting are retained.
Claude receives our replacement system prompt separately from its compact stdin summary, an empty strict MCP config, no settings sources, safe mode to skip CLAUDE.md/customizations without changing subscription authentication, low effort, no tools or persistence, and model claude-haiku-5-5.
Codex ignores user config (including configured MCP servers), uses low reasoning effort and retains its ephemeral read-only invocation.
All flags were checked only against installed CLI help; Claude's dynamic-section flag is ignored with a replacement system prompt and was omitted.
Exact argument vectors and the empty MCP file are checked through the fake CLI; no real API requests, real authenticated CLI runs or credential reads were performed.
Live coach latency, context size and provider acceptance remain unverified.

Checkpoint requests now prefer posture_half at 50% posture damage and posture_critical at 85%, re-arming below 25%, with half_hp retained as a once-per-fight fallback.
A critical crossing supersedes an unsent half-posture or HP warning; a jump over both thresholds requests critical directly.
Coach hints, instructions and docs include these events; regression tests cover two cycles, exact boundaries and fallback.

Lock-on already dropped its target on HP death, but MoveDirection retained the last strafe while a stick stayed held, steering a sideways clip along the free body's facing.
The common release now resets that selector and walk twist, publishes the HKS lock flag and is used on deathblow start as well as death, manual unlock and smoke lock breaks.
Lethal finishing pairs reject relocking, and HP deaths release the camera target in the contact step.
The failing real-script replay moved [0.545, 1.543] m in X/Z over 0.5 s after death; the fix moves [2.165, -0.308] m for a camera-relative right stick.
Tests cover the HKS lock query, dead-target refusal, held-stick free movement, locking after reset and relock attempts during/after a lethal deathblow.

The normal-Lua soldier pressure replay at 1.6 m with attack presses every eight frames for four seconds already passes before a gameplay change: one landed hit (tick 26), one block (64), one deflect (98), no posture break.
His Lua requests EndureAttack guard 3100 and deflect 3101, the guard reactions expose guardLevel 4 and SpEffect 200250, and his next counter request is 3008.
There are no unanswered AI calls in the replay; baseline evidence is logs/soldier-pressure-before.log, and soldier_guards_mashed_slashes retains the regression.
No forced guard, input, attack timing, hit stop or combo/cancel change was added.
The reported soldier stun lock remains unreproduced in this setup; an exact failing replay is needed to establish any remaining root cause.
Details are in docs/BRAIN.md, docs/AI.md and docs/COMBAT-RULES.md.

Validation: cargo fmt --all and its check, workspace/all-targets Clippy with -D warnings, and the requested release sekiro-sim plus sekiro-brain-net lib/bin/integration suites passed (304 tests).
All Cargo invocations used target/alt and six build jobs; fmt uses CARGO_BUILD_JOBS because it has no -j option.
Logs: logs/playtest-fixes-tests.log and logs/playtest-fixes-clippy.log.
No rendered game session was run; real scripts, graphs, Lua AI, TAE and params were exercised through the deterministic duel tests.
No cache/re links, junctions, worktrees, deletions or game-install writes were made.

## Combat fundamentals and actionable checkpoints (2026-10-11)

Added `docs/COMBAT-FUNDAMENTALS.md`: sourced base mechanics in original words, a rule-by-rule code/test audit and six fight-impact-ranked fidelity gaps.
The largest gaps are general/thrust dodge i-frames, unverified complete poise/guard-counter parity, and player progression scaling; combat mismatches were documented without changing simulation rules.
Checkpoint posture events are now `pressure` at about 6 estimated Wolf hits left and `pressure_critical` at about 3, with no pressure warning below 2 hits.
The estimate uses Wolf's first combo TAE judge and the same AtkParam, weapon, reinforcement, enemy defence and active-effect posture formulas as the duel.
Each warning fires once per posture cycle and re-arms when remaining posture recovers above 70%; HP-half is retained as a fallback.
Requests are suppressed during enemy deathblow openings, posture breaks, damage staggers, throw/get-up reactions, death and respawn.
Queued pressure waits through a brief stagger and is revalidated before sending; deathblow openings discard obsolete pressure.
`mark_lost` only fires with another mark remaining: none for the soldier, one after Owl's first deathblow and get-up.
Coach instructions now include a 447-character combat paragraph; the schema, hint names and `docs/BRAIN.md` use the new events.
The real-data offline regression first demonstrated the old percentage timing and later caught a get-up request before Owl could act; the final soldier/Owl deathblow regression passes with suppression through the throw state.
Validation passed: `cargo fmt --all`, `cargo fmt --all --check`, workspace/all-targets Clippy with `-D warnings`, and the requested release sim plus brain-net lib/bin/integration suites (305 tests).
All Cargo commands used `CARGO_TARGET_DIR=target/alt` and six jobs; fmt uses `CARGO_BUILD_JOBS=6` because it has no `-j` flag.
Final release test log: `target/alt/combat-fundamentals-final-tests.log`.
No commits, real model/API calls, real Claude/Codex CLI runs, credential reads, game sessions, game-install writes, worktrees, or cache/re links, junctions or deletions were made.

## Plan spacing, deflect and timed back steps (2026-10-11)

Plan attack approaches now evaluate the real Lua `Approach_Act_Flex` range check when the attack step activates, after earlier tactics, instead of while the whole plan is queued.
The soldier's action-table distances already match `101000_battle.lua`; his hit radius was subtracted once and NPC TAE 760 lunges already work.
The actual range reproduction is `retreat12>3000` with stationary Wolf initially 1.6 m away: before, attack start 4.57 m, first active frame 3.19 m, observed capsule/root-travel reach 4.29 m, no contact; after, start 2.02 m, first active 0.80 m, reach 3.92 m, contact.
The six reported plans contact stationary Wolf at 2.5, 4 and 6 m even on the original runtime, so their historical whiff rates were not reproduced as pure stationary range failures.
Owl's firecrackers now use `506001_battle.lua` Act07's 3.5 m approach and reset pressure number 5 to zero instead of the previous approximation.

A second bug inflated the evidence: movement, guard and back-step exchanges were counted as whiffed attacks because they contacted nothing.
They still contribute usage and punishment but cannot whiff in the tally, per-plan/per-set summaries or outcome logs.
The 15 s offline `walk_in4>3000+3001` fight runs four plans and lands eight attacks: its count changes from [12,8,0,0,4,0] to [12,8,0,0,0,0].
Existing memory files were not read or rewritten; historical totals retain their old false whiffs.

Both bosses offer `deflect` and `dodge_back` and five new attack chains each; all chains are listed in `docs/BRAIN.md`.
`deflect` faces Wolf for up to 1.5 s and routes the real parry-timing interrupt through the boss's own battle callback into a local queue, preserving the following attack after its guard/deflect response.
The soldier's rank, facing, weapon reach, interval and consecutive-guard roll still choose guard 3100 versus deflect 3101; the duel's guard judge, repel defence, active effects and attack data still decide the contact.
Owl uses his own parry callback.
`dodge_back` watches Wolf's visible attack wind-up within 2.5 m and requests soldier 5211 or Owl 5201, then continues the attack chain.
The soldier's pressure replay guards then requests 3000, finishes readiness after 36 ticks, and its timed short step gains 0.86 m and finishes after 38 ticks before requesting 3002.
Both idle waits finish after 90 ticks at 60 Hz.
Both caps feed anti-stall lead time; `dodge_back` also uses the eight-second retreat allowance.

The coach summary now includes per-plan mean [attack-start distance, observed melee reach] in metres, rounded to two decimals.
Reach samples actual animated dummy-poly capsules, AtkParam radii, root travel and Wolf's body radius; projectile-only and interrupted-before-active attacks have no melee sample.
The radial envelope and averages are diagnostics, with direction, timing and target movement still relevant.
Ranges trim with plan outcomes and the worst-case summary stays inside 4096 bytes.
Updated `docs/BRAIN.md` and `docs/AI.md` document the mechanics, limitations, chains and before/after evidence.

Validation passed: cargo fmt --all and its check, workspace/all-targets Clippy with -D warnings, and the requested release sim plus brain-net lib/bin/integration suite (310 tests).
The final coach wording was also rechecked through the complete offline brain-net suite.
The range regression exercises all thirteen soldier attacks and nine mixed plans at 1.6, 2.5, 4 and 6 m (88 cases), plus dedicated retreat-whiff, reactive soldier/Owl pressure, timeout, telemetry/budget and non-attack-whiff tests.
The Owl tally test now checks contact/whiff outcomes only for attacks and separately verifies that back steps cannot whiff.
Logs: `target/alt/plan-range-before.log`, `plan-retreat-before.log`, `plan-combat-tests-final.log`, `plan-protocol-tests.log`, and `plan-combat-clippy.log`.
All Cargo commands used target/alt and six jobs; fmt uses CARGO_BUILD_JOBS=6 because it has no -j option.
This task made no commits, real model/API calls, real Claude/Codex CLI runs, credential reads, live game runs, game-install writes, worktrees, or cache/re links, junctions or deletions.
Input buffering, attack timing, hit stop and Wolf's movement were not changed by this task.
Concurrent --immortal edits in the game and Duel were left intact and are separate from this work.

## Five playtest fixes (11 October 2026)

Raw duel input now determines whether a fight is valid: at least three relevant presses in the first ten seconds and any relevant held input during at least 40% of its duration.
Void fights print `fight void: no player input` and do not enter history, tally, habits, wins, plan failure evidence or post-fight coach calls; current-fight autosaves also exclude inactive evidence.
The window requests focus and cursor lock on the first input frame and re-grabs on the first click inside it.
Windows can still refuse foreground activation.
The no-input integration test uses a temporary memory file and fake coach; active-fight save tests remain green.

The normal-AI, non-god posture reproduction sampled a soldier block at tick 64 (Wolf attack 5000011) and deflect at tick 98 (5000050).
Both attack rows specify zero attacker posture return, and Wolf's remaining posture stayed 120 to 120.
The executable research confirms the attack-row return fields and the deflect multiplier path, so no unsupported nonzero return damage was introduced.
Nonzero NPC return tests still cover 45, 22, 11 and 5 posture damage.
Matched original-game contact footage remains unverified; the audit and limitation are recorded in docs/COMBAT-RULES.md.

Held-stick movement retained the old heading because free_move_start_turn skipped an already moving Wolf after lock release.
Release now clears the remembered heading and queues the camera-relative turn when movement is permitted, including after deathblow recovery.
The reproduction's backward-stick sideways drift was 0.600 m in half a second; all four stick directions at three camera yaws now have less than 0.000002 m sideways drift.
The raw-key replay is crates/game/scripts/held_move_kill.txt.
The final live run held W through a soldier deathblow and recovery, logged lock false and zero direction error, and completed in under ten seconds (exact PID 13736, 60-second cap).
Its first-frame log reports focused true and cursor Locked.
The screenshot target/alt/screens/held_move_kill_final.png shows the soldier at zero HP during the deathblow.
Saves were backed up to the local temporary sekiro-playtest-saves-20261011-015359 directory before game sessions.

Per-plan uses, deflected_all and fatal counters persist in coach memory.
Three uses with a summed failure rate of at least 0.6 exclude the plan from sampling and openers, including across subsequent fights.
Fatal attribution covers lost marks, death within three seconds of the final attack exchange and a punished posture break, including death during the final attack.
Only a validated coach retry with a reason re-allows a plan; historical counts remain intact and new failures can ban it again.
Void fights roll back provisional bans and never persist their failures.
Coach summaries include banned choices and packed failure counters within 4096 bytes, tested with the entire known vocabulary banned.
Codex defaults are gpt-6.1-sol and medium effort, with both CLI overrides tested through a fake executable.

Coach and checkpoint instructions reward HP plus posture damage per minute, fewer perfect deflects and survival while passive_share is below 0.35.
Engaged_time appears in metrics, scorecards, trends and coach input; it is duration below that passivity threshold and zero otherwise.
Retreat allowance is once per five seconds and permits retreat chains followed by an attack, while preserving the six-second attack rule and passivity tracking.
The checkpoint body still fits its 1200-byte limit.

Validation passed: cargo fmt --all and --check, workspace/all-targets Clippy with -D warnings, the requested release sim and brain-net lib/bin/integration suite (321 tests), and the release game build.
All Cargo work used target/alt and six jobs; fmt used CARGO_BUILD_JOBS=6 because it has no -j option.
Logs are target/alt/playtest-clippy-final.log, playtest-tests-final.log, playtest-build-final.log and playtest-game-final.err.log.
Updated docs/BRAIN.md and docs/COMBAT-RULES.md document behavior, evidence and the remaining original-game posture parity question.
No commits, real model/API calls, real Claude/Codex CLI runs, credential reads, protected memory-log edits, game-install writes, worktrees or cache/re links, junctions or deletions were made.
Attack timing, hit stop and the input buffer were unchanged.

## Owl-driven brain session (2026-10-11)

Boss-only pressure checkpoints and aggregate Lua plan counts hid Wolf's break opportunities and the individual moves' performance.
The read-only golden Owl dataset confirms five boss wins, six 3040 grabs that all whiffed, four punished grabs and 127/64/49 Lua acts in its last three fights.
Wolf posture checkpoints now fire at 25%, 50% and 75% damage once per cycle, re-arm below 15%, and report breaks without requesting during deathblows, boss breaks, death or respawn.
Metrics, scorecards, trends and coach input now include Wolf breaks, hits within 2.5 seconds after a break, the fraction of breaks punished, peak Wolf posture and boss-win time to kill Wolf.
Multiple hits after one break do not inflate the punish rate above 100%, and the breaking blow is excluded.

The coach can author one to three weighted on_wolf_break responses, validated against the normal action vocabulary and avoid list.
The fighter replaces Owl's Act40 grab follow-up locally and retains the response through delayed Lua replanning until its first attack starts or the punish window expires.
A single response attack uses the existing ComboFinal cancellation path and preserves the current animation's TAE windows.
The real-data scripted slash-to-break regression starts response 3000 after 18 ticks (0.300 seconds); the same setup without a response plan requests Lua grab 3040 after two ticks.
Response outcomes persist separately and have independent on_wolf_break:<plan> failure evidence, with the existing three-use/60% ban and reasoned retry rule.

Combined Lua weights are capped at 25% per set; the coach is instructed to mix at least three different attacks, with warnings for narrower legacy sets.
Coach input now reports individual action outcomes, including Lua choices and zero-count untried actions, plus measured attack distance versus reach.
Up to six principles of 90 characters each persist in the game plan, return on every coach call and print in plan summaries.
The coach input limit is 8192 bytes to retain the new evidence and lessons; the in-fight chooser still uses its tiny existing input.
F9 and normal game exit print a session action/range table and fight trend and append to logs/brain-session-<timestamp>.md.
Report tests use temporary files, and the game enables reports for the real Lua AI as well as controlled brains.

Brain validation passed: owned-file formatting, shared workspace formatting checks, all-target Clippy with -D warnings and the complete requested release sim/brain-net lib/bin/integration suite (333 tests).
The final shared checks include the parallel posture agent's updates; all Cargo work used target/alt and six jobs.
Logs: target/alt/brain-break-final.log, brain-unit-final.log, brain-tests-final.log, brain-clippy-final.log, brain-clippy-core.log, brain-clippy-changed-tests.log and brain-fmt-final.log.
Live model plan quality, rendered F9 behavior and parity against the original game's fights remain unverified.
The brain work did not edit combat/* or posture_recovery.rs, commit, call a real model/API/CLI, open credentials, alter protected memory/dataset logs, create worktrees or cache/re links, or delete cache/re contents.
Attack timing, hit stop and the input buffer were unchanged by the brain changes.

## Guarded-pressure posture recovery (2026-10-11)

Report: the Owl's posture barely rises while he guards.
Finding: the engine's one "no recovery" flag is HKS `act(110)` (`SetStaminaRecoveryDisabled`, bit 21 of the action state's +0x88; its only writer is the HKS command handler `b32410`, reached from `b4e520`).
No shipped script calls it, so it is not the cause; the sim now honours it anyway (`CombatRules::recover_with_state`, `PlayerEnv::recovery_disabled`, `act(110)` in `player.rs`), for NPCs and Wolf alike.
There is no post-damage delay timer in the recovery update (`a04850`); pauses come from TAE 960 types and TAE 225.
The Owl's guard `a000_003100` has neither, so at full HP he recovers 60 per second while guarding (0.3 s of 0% in the guard-hit reaction `a000_008501`), and a guarded Wolf slash costs him only 14 to 15 of 660.
Sim numbers (Wolf slashing a forced guard every 0.6 s for 10 s): Owl 660 -> 646 -> 646 -> 654 -> 624 -> 634 -> 646, never below 624 and back to about 650 within 0.5 s of each hit; the soldier (guard recovers at 25% of 20) 75 -> 45 -> 15 -> 0, broken in three hits.
No recovery rule was changed, because the data and code say this is what the game does at full HP; whether the live Owl builds faster is unverified and the next suspects are guarded posture damage and progression scaling (`docs/COMBAT-FUNDAMENTALS.md`, mismatch 4).
Tests: `crates/sim/tests/posture_recovery.rs`, `act_110_disables_posture_recovery` in `player.rs`.

## Mid-fight coach reviews (2026-10-11)

Coach mode now reviews after 45 seconds of fight time since the last send, or early after a surviving boss mark loss, two Wolf posture breaks, or all plans in the active set being banned.
The first 20 seconds, inactive-player fights, posture breaks, deathblows and respawn block sends; one coach call is outstanding at most, and a pending post-fight review supersedes a mid-fight trigger.
`--coach-interval <secs>` defaults to 45; 0 disables every mid-fight trigger.
Mid requests use `phase: "mid"`, partial fight metrics and outcomes first, elapsed time, both fighters' HP/posture, boss marks and the current plan, within 4096 compact JSON bytes.
Validated replies wait for the next non-reflex decision slot, preserve existing attack goals and skip their opener, while being saved through the normal memory path.
Console and JSONL telemetry identify the phase and reason; fight summaries count mid reviews and their completed latency.
The objective is winning faster, breaking Wolf posture and hitting after, dealing HP plus posture damage per minute, then taking little HP plus posture damage and limiting Wolf perfect deflects.
Passive share above 0.35 fails regardless; engaged time is information and only a small losing-fight tiebreaker.
`boss_damage_taken_per_min` is persisted in metrics and included in scorecards, trends and coach results.
Offline regressions cover the 120-second 45/90 cadence and post call, mark loss at 30 seconds resetting the next timer to 75, serialized slow calls with post-review priority, deferred plan application and opener skipping, the 20-second floor, zero disabling, request budgeting and mock HTTP mid-phase telemetry.
Validation passed: cargo fmt --all and --check, workspace all-target Clippy with -D warnings, and the full requested release sim/brain-net lib/bin/integration suite (340 tests).
All Cargo work used target/alt and six jobs; logs are target/mid-final.log and target/mid-clippy-final-pass.log.
The worst-case input regression fitted 4078 bytes under the 4096-byte limit.
The all-banned check excludes Lua fallback choices so they cannot hide a failed coached set.
No commits, real model/API/CLI calls, credential reads, protected log edits, combat edits, worktrees, cache/re links or cache/re deletions were made by this task.

## Wolf's attack power (2026-10-11)

Both the game and `duel-sim` accept `--attack-power 1..99` (default 1) and `--progression castle-roof` (AP6, required-route Memories); explicit AP overrides the preset.
Damage reads the real `SpEffectParam` Player Growth_Core row `160000 + AP`: element `*AttackRate` fields for HP and `staminaAttackRate` for posture before truncation.
All 99 rows are checked, including the AP52 exception (6.775, followed by 6.77 at AP53); no hand-written progression table drives gameplay.
CalcCorrectGraph 500/501/504 still describes Wolf's separate Prayer Necklace growth, fixed at 1, rather than being fed Memory attack power.
The local RustSekiro clone was checked and leaves its AP formula unresolved; the extracted rows and community Attack Power/Memory references are documented in `docs/COMBAT-RULES.md`.
Combat fundamentals mismatch 3 is resolved for attack power; full Prayer Necklace progression and live-game parity remain outside this setting.
Real-script opening slash HP/posture at AP1/5/10: soldier 80/30, 160/60, 260/97 before the remaining-HP cap (195 actual HP damage at AP10 on a fresh soldier); guarding Owl 0/15, 0/30, 0/48.
At AP6, the existing ten-second pressure cadence (one attack press every 0.6 s) lands 11 guarded contacts and lowers Owl's remaining posture to 512/660, versus 624/660 at AP1, without changing his 60/s guard recovery.
The posture builds but does not break within this run.
Validation: workspace/all-targets Clippy with -D warnings, cargo fmt --all and its check, and the full requested release sim/brain-net lib/bin/integration suite passed (344 tests).
The alternate release game executable was built; actual CLI checks rejected invalid/missing values, and duel-sim confirmed explicit AP5 overrides castle-roof with a one-tick passive run.
Logs: target/alt/ap-baseline.log, ap-focused.log, ap-focused-final.log, ap-suite.log, ap-clippy-final.log, ap-game-build.log and ap-cli.log.
All Cargo commands used CARGO_TARGET_DIR=target/alt and six jobs; fmt uses CARGO_BUILD_JOBS=6 because it has no -j flag.
No commits, real model/API calls, authenticated CLI runs, credential reads, protected-log edits, game sessions, game-install writes, worktrees, or cache/re links, junctions or deletions were made.

```powershell
$env:CARGO_TARGET_DIR = 'target/alt'
$env:CARGO_BUILD_JOBS = '6'
& "$env:USERPROFILE\.cargo\bin\cargo.exe" run --release -j 6 -p sekiro-game -- --arena owl --progression castle-roof
```

## Owl's phase-2 deathblow protection (2026-10-11)

Reproduced with a real first deathblow in the sim: attack during `a000_020005` immediately started the finishing pair 15060180 / 15060181.
Deathblow selection now rejects the active TAE judgment marker 3506080, which covers the heat-up, kneeling loop and first 0.233 s of the escape.
The red mark and input both use this check; ordinary slashes stay available.
The transition also carries ChrActionFlag 24 throughout: native `b51b70` sets action +0x88 bit 16, `bdbb40` reads it, and `b6e6a0` stores it in damage record +0x1CA.
Implemented its traced damage-level-zero and no-knockback behavior (`b68d00`, `b6bfe0`) and the HKS `env(349)` answer, so a landed slash cannot replace the protected animation with posture collapse.
Owl's unchanged castle Lua detects Wolf's attack through `INTERUPT_ParryTiming`, checks 3506080 and its parry range, and requests escape 20007.
The escape restores HP and posture from its TAE, returns the phase-2 boss bar and keeps one mark; posture break and HP depletion both still open the finishing deathblow afterward.
The user's real-game confirmation replaces the previous uncertainty about a deathblow while begging in `docs/OWL.md`.
That document separates the exe-traced damage-motion rule from the marker-based transition gate; the complete native throw-defence/ThrowKind condition dispatcher remains unreconstructed and is recorded in `docs/FIDELITY.md`.
The real-data regressions test no red mark or second deathblow in both transition clips, landed ordinary slashes preserving the transition, Lua escape 20007, and the finishing blow after HP depletion alongside the existing posture-break victory.
Focused `owl_phase2`, `owl` and `deathblow_lock` tests passed.
Final validation: `cargo fmt --all`, workspace/all-targets Clippy with `-D warnings`, and the full requested release sim lib/bin/integration suite passed (313 tests, zero failures).
Full suite log: `cache/tests/owl-transition-final.log`; local native research: `re/exports/decomp/owl_transition/`; parameter evidence: `cache/tests/owl-transition-params.txt`.
All Cargo commands used `CARGO_TARGET_DIR=target/alt` and six jobs; fmt uses `CARGO_BUILD_JOBS=6` because it has no `-j` option.
No commits, real model/API calls, protected-log edits, game sessions, game-install writes, worktrees, or cache/re links, junctions or deletions were made.

## Shared boss knowledge and player profiles (2026-10-11)

Boss knowledge is now shared across player profiles, with counted action/plan/break-response evidence, reasoned bans, six universal principles and the best known sanitized game plan with its results.
`--player <name>` selects isolated habits, history, notes, player principles, local failure bans and the current tuned plan; the default is `default`.
A fresh player starts immediately with the shared plan at 20 recorded fights, while thinner knowledge uses the Lua baseline.
Mid-fight review defaults to 30 seconds, with the interval flag retained.
Coach requests separate boss knowledge and player profiles under the same 4096-byte budget; responses add `player_principles`, `boss_principles` and `boss_bans`.
Old `principles` replies remain readable as player principles.

First load copies legacy memory to a byte-identical `.v2-backup.json` and creates `.boss.json` plus `.player-default.json`, retaining the original.
Unclassified legacy principles remain local until the coach explicitly promotes universal lessons.
Temporary-copy migration preserved the outsmarted Owl dataset's 21 fights, 20 boss wins, one Wolf win and 656 action/plan exchanges, and the alive dataset's seven fights, six boss wins, one Wolf win and 281 action/plan exchanges.
Both migrations retained all break-response counts and backup bytes, and repeated loads did not inflate totals.
New offline regressions cover rich/thin knowledge in a real extracted-data duel, shared coach lessons, isolated habits and bans, repeated/stale saves and full-layer Unicode budget pressure.
Details and file examples are in `docs/BRAIN.md`.
No commits, model calls, real coach CLIs, game runs, links or junctions were made, and protected memory/dataset files were untouched.
All Cargo commands used `target/alt` and six build jobs; formatting uses `CARGO_BUILD_JOBS=6` because cargo fmt has no `-j` option.Formatting, workspace Clippy with `-D warnings`, and the requested release sim/brain-net lib/bin/integration suite passed (351 tests across 37 targets).
The final full-layer budget regression includes duplicated boss lessons/bans inside the played plan and confirms they are removed before fitting.
Validation logs: `target/alt/shared-memory-tests-final.log`; no `map_tangents` exception was needed for these checks.

## Genichiro castle lightning (2026-10-11)

Tomoe row 71100000 now runs lightning acts 3039/3042/3043 through its Lua and explicit brain table, using Bullet 71100521 and AtkParams 71100521/71100361.
Grounded and guarded contacts discharge into Electrified; airborne Wolf catches charge 9492 and the installed HKS selects weak Lightning Reversal on attack, returning AtkParam_Pc 5000281 with effect 9400.
The castle replay records 2032 HP and 225 posture damage to Genichiro; a blocked grounded bolt records 272 HP and 120 posture damage and Wolf's shipped shock reaction.
Simple blue sparks/tint, jagged bolt lines and a white return flash are visible in inspected `target/alt/genichiro/lightning-*.png` captures, although the castle scene remains washed out and the incoming beam is thin.
Five real-data regressions cover bow spawn, grounded/guarded shock, airborne capture, return, missed return on landing and both lightning slashes.
`docs/GENICHIRO.md` records all ids and placeholders: native electric resistance scaling/threshold, charged input acceptance, TAE 922 timing and the absent weapon dummy replaced with 10 m return reach.
Castle c7100 has the clips but lacks matching Lua acts and attack rows, so its lightning choices remain excluded.
Formatting, workspace/all-target Clippy, the full release sim/brain-net suite including doctests, and the release game build passed with `target/alt` and six jobs.
Validation logs are `target/alt/lightning-suite-final.log`, `lightning-clippy-final2.log` and `lightning-game-final.log`.
No commits, network/model calls or protected data writes were made; scripted app runs used simple AI and brain memory off, with saves backed up locally first.

## Projectile guard ownership (2026-10-11)

Reproduced Owl act 3014 and Genichiro act 3009 through real TAE launches, bullet flight and Wolf's installed HKS guard scripts: deflect sent owner recoil 1008 and block sent 1019 despite zero attacker posture damage.
Projectile contacts now resolve through a separate entry point: the shooter supplies damage scaling but receives no recoil, hit stop, posture feedback or attacker break result.
Melee resolution and Wolf's guard damage, reaction, status and lightning paths retain their existing arithmetic.
The source is the actual Bullet contact, since all 249 referenced existing NPC attack rows have isArrowAtk 0.
Owl AtkParam 50600150 has deflect/block posture bases 6/12 and all attacker feedback fields zero; Genichiro 71000500 has 9/18, also zero feedback.
Both have guardAtkRate 30, katana guard/just-guard permissions enabled and zero guarded attacker reaction selectors.
The 6 m regression records Wolf posture 5/11 for Owl and 8/17 for Genichiro after existing cuts and truncation, with zero guarded HP loss and unchanged shooter clips.
Coverage also uses Owl fan row 50600131's nonzero 30 feedback, an already empty shooter meter, the character-contact recoil control, and guarded lightning still electrifying Wolf.
The short rule is in docs/COMBAT-RULES.md; docs/PROJECTILE-GUARD.md contains the complete Owl/Genichiro parameter audit and local citations.
Native bullet-to-owner notification dispatch remains inferred, explicitly distinguished from the extracted parameter facts and Paramdex descriptions.
Formatting and workspace/all-target Clippy with -D warnings passed.
The final full release sekiro-sim/sekiro-brain-net suite, including doctests, passed: 433 tests across 50 targets.
The required release game build was attempted but remains blocked by Windows access denied while the other job's target/alt/release/sekiro-rs.exe is running; permission to close that playtest was requested, and it was left running.
Validation logs: target/alt/projectile-baseline.log, projectile-focused.log, projectile-suite-final.log, projectile-clippy-final.log and projectile-game-build.log.
A superseded overlapping rebuild encountered a Windows lock on a running Owl test executable; the final sequential rerun passed.
All Cargo commands used CARGO_TARGET_DIR=target/alt and four jobs, with CARGO_BUILD_JOBS=4 for formatting.
No commits, network calls, live game sessions, protected data writes, worktrees, links or deletions were made by this task.


## Arena integration regression (2026-10-12)

Reproduced the Owl slash failure through the real-data Duel integration test and isolated the new AiReplan handler by temporarily reverting its application.
Owl's post-intro startup event replanned after his first AI tick, discarding the initial combat goal and changing deterministic Lua choices, tactic plans, and tool positioning.
FightEvents now tracks its first event pass and suppresses that redundant startup replan for enemies already initialized in battle, resetting this state on reload and preserving later event replans.
The full release sekiro-sim/sekiro-brain-net suite passes: 463 passed, 0 failed, 3 ignored, including all existing regressions and the new arena tests.
Workspace/all-target Clippy with -D warnings and cargo fmt --all --check pass.
All build and test commands used CARGO_TARGET_DIR=target/alt and 12 jobs.
No tests were weakened or deleted, and no commits or git state changes were made.

## Combat counter follow-ups (2026-10-12)

The initial scripted Genichiro and Owl Mikiri-to-deathblow fixtures failed because the counter defender clip was treated as a deathblow get-up lock until posture had recovered.
Deathblows now respect the queued counter startup and Wolf's TAE flag 154 at 1.200 s, replace the counter pair cleanly, and clear the counter lock when Wolf cancels through another action.
Seven new installed-data regressions pass for Genichiro, Owl and the soldier, including the first valid deathblow press, a move-cancel follow-up, and actual sweep-stomp meter loss.
The reported reduced sweep-counter damage remains unreproduced: AtkParam_Pc 901's 10 posture and SpEffect 400200's 8x anti-ground rate already give 80 at AP1 and 180 with castle-roof AP6 effect 160006.
Additional Genichiro 3007 / 3048 range and timing probes produced misses rather than a landed low-damage counter, so no unsupported damage multiplier was added.
The release counter_followups, owl, genichiro, brain_tactics, attack_power, posture_recovery, deathblow and perilous filters all passed, with their slow tests left ignored.
Workspace/all-target Clippy with -D warnings and formatting of the new test file passed; the parameter and animation findings are appended to docs/COMBAT-RULES.md.
All builds and tests used target/alt-fc and three jobs, with no commits, git state changes, network/model calls, live game sessions or protected-path writes.

## Castle-top camera obstruction (2026-10-12)

Reproduced the Genichiro/Owl obstruction as the hanging bamboo blind `m006672_4001`, which crosses the rendered sight line without a camera collision hit.
Added near-camera dither from the native MTD opt-out and shader distance transform, with the provisional fade interval, noise and interpolation differences documented in [FIDELITY.md](docs/FIDELITY.md#castle-top-camera-obstruction-2026-10-12).
Inspected Genichiro and Owl before/after captures are retained under `target/alt-fm/`, with an original reproduction script in `crates/game/scripts/camera_ceiling_edge.txt`.
All 23 focused release tests passed: 11 map tests including the installed-data obstruction guard, seven camera unit tests and five collision unit tests.
Scoped game/format all-target Clippy and rustfmt checks of the two new Rust files passed.
The simulation camera integration build and final workspace/all-target Clippy remain blocked by untouched `crates/sim/tests/it/coach.rs:242`, whose `GamePlan` initializer lacks `explore`, `lessons` and `move_controls`.
All Cargo checks used `target/alt-fm` and three jobs, with no git operations, network/model calls, live game sessions or writes to the game, cache, re or protected output directories.

## Castle rooftop renderer and restart cleanup (2026-10-13)

Specialized blind camera fade removes the shared discard from ordinary map shaders, lowering measured opaque GPU time from 13.153 to 10.455 ms at 1080p on the RTX 5070 Ti.
Mean frame time is 16.781 ms before and 16.681 ms after, matching the 16.681 ms baseline-equivalent feature control; short idle runs did not reproduce the reported long-session 35 fps state.
Fixed deferred decal-budget overflow, respawn stain/transient-FX cleanup, stale placed-effect holds and unused character-material wrappers.
The twenty-reset rendered probe keeps mesh/material assets, lights, cameras, rigs and main draw batches flat, with 6,730 materials instead of the former growth of about 56 per phase cycle.
Phase-one Genichiro restores the arena's morning environment after Tomoe; headless story-time and visual storm/reset checks pass.
Measurements and captures are under target/alt-pf, with details in docs/VISUALS-PLAN.md.
The initial Lua measurement inadvertently wrote logs/brain-session-1791847964710.md; it was left untouched, and subsequent captures used simple AI to bypass reports.
All 59 game unit tests and five integration tests pass with target/alt-pf and four jobs; workspace/all-target Clippy with warnings denied and the final workspace formatting check pass.
No brain files were edited, and no commits or git-state mutations were made.

## Wolf victory recording (2026-10-13)

Reproduced a final Genichiro/Tomoe deathblow leaving Wolf wins at zero; map-backed encounters never schedule the boss-death respawn that the brain required to settle a result.
Encounter defeat now settles once, including cinematic event frames and before reset; confirmed victories survive the low-input void gate and retain kill time in memory, reports and coach trends.
Regressions cover Genichiro handover, final victory and persisted summaries, Owl and both Father variants, single-phase c1010, and player death remaining a boss win.
Validation with target/alt-ww and six jobs: 98 brain unit tests, 53 brain integration tests and 19 local coach tests pass, with one existing slow test ignored; workspace/all-target Clippy with warnings denied and workspace formatting checks pass.
Existing player memory and logs were left untouched; logs contain no raw combat victory timestamps, so an exact recount requires additional fight telemetry or a recording and should first produce a reviewed copy.

## Shared map story lighting (2026-10-13)

The map loader now selects installed event-100 lighting, map-specific story presets, GPARAM sub-IDs and sky groups through shared logic, with one original metadata module per map and `--story` selection.
All four collision GPARAM IDs are retained independently, lighting regions switch with player/camera movement, and reservoir file switching uses the same mechanism as the other maps.
Seven installed-data selector tests pass, including boss states, clear/storm restoration and sky-override priority; earlier focused map/lighting/sky/story/arena checks passed, including the Tomoe restart regression.
Accepted captures cover all nine default map starts and both castle-top times under `target/alt-lf/captures/`, with frame medians from 16.504 to 20.522 ms and detailed evidence in `docs/LIGHTING.md`.
Exact visual equivalence remains limited by missing event sky exports, unresolved nonzero bank/family provenance and the existing rendering calibration; no tone curve, exposure or probe-blend implementation was added.
Final workspace/all-target Clippy with warnings denied and workspace formatting both pass after the other agent resolved their temporary compilation/lint/formatting blockers.
All builds and tests use target/alt-lf and six jobs, with no commits, git-state changes, network/API calls or writes to protected data directories.
2026-10-13: Fixed Task A move-sheet selection applying reach gates and generated alternatives to fixed/scripted brains; added fixed-plan and generic/installed Tomoe bullet-range regressions, and explicitly opted adaptive fixtures into move selection without changing assertions; sekiro-sim release tests pass (199 unit + 315 integration, 20 ignored, 0 failed), workspace/all-target Clippy with warnings denied and formatting checks pass using target/alt-lt and eight build jobs.
2026-10-13: Audited m11_02 Reservoir story inputs and preserved boss arena states; added map-local installed-data and native-container tests, nine before/after capture pairs and uncapped start/heavy profiles under target/alt-m11_02; found 20 native GIIV volumes per environment missing from the derived cache, global-probe fallback and unresolved fog-bank/family routing; exact lighting, a walking boundary transition and steady frame time remain unverified; Reservoir and six shared selector tests plus collision regression pass, broad selection retains m13/m25 failures, final Clippy is blocked by shared map_material.rs:356 clone_on_copy, and formatting passes; no shared renderer, cache or git-state changes.
2026-10-13: Audited Abandoned Dungeon m13_00 across all five native clock states, added an installed-data regression and before/after captures in target/alt-m13/captures, verified collision IDs and the vertical zero-light wall, documented shared indirect-light/profiler limits and latest uncapped median/p99 24.827/40.306 ms at start 0 and 29.493/40.454 ms at start 1; m13 and moving-focus tests plus 11 lighting-selection tests pass (two ignored), final game/all-target Clippy and formatting pass with target/alt-m13 and four Cargo jobs, while exact native visual equivalence and steady frame time remain unverified.

## Layered map materials (2026-10-13)

2026-10-13: Ported RustSekiro 265e142's M[Multiple]/M[MultipleGround] families and exporter streams/constants/raw normals, preserving bindless GI and matching depth/motion camera fade; isolated m10/m17/m11_01 exports, paired captures and timings are in target/alt-ml/captures, with material/formats/extract checks passing, concurrent m13/m25 failures resolved, new protected m11_01/m20 lighting failures reported, and workspace Clippy/fmt clean.
2026-10-13: Audited m15_00 native clock/sky/sub-ID/collision/ArenaSpec selection, added its installed-data regression and before/after default/village/Monk captures under target/alt-m15, and documented shared fog-enable aliasing, missing 800 native GIIV export planes, inactive hemisphere and unrepresented specular-light controls; selection/region tests, final game all-target Clippy and formatting pass, while exact rendering and steady uncapped pacing remain blocked by shared data/rendering and host contention, with the after boundary walk unverified due repeated respawns.
2026-10-13: Audited Fountainhead m25_00 against native events, GPARAMs, collision banks and sky; corrected dragon-defeated to include 9310 and removed the True Monk reset while preserving ArenaSpec starts; both live hall crossings and before/after captures are retained under target/alt-m25/captures with uncapped median/p99 timings and shared water, exposure, local-light and GI-cost findings; the map regression, seven selector tests, six map tests, game all-target Clippy with warnings denied and workspace formatting pass using target/alt-m25 and four Cargo jobs, while exact rendered parity and steady frame time remain shared-renderer follow-ups.
2026-10-14: Audited m11_00 story GPARAM/sky selection, collision-bank doorway round trip and Gyoubu/Demon ArenaSpec defaults; added the map-local installed-data regression and 14 settled state captures under target/alt-m11_00/captures, documented shared four-light/specular, sky-group, startup-readiness and presentation/stall limits with measured frame/GPU timings; m11/core-selector/collision tests and final game/all-target Clippy plus workspace formatting pass, with the broader filter exposing an unrelated m25 dragon-flag assertion; exact visual matching and steady uncapped frame time remain shared/environment follow-ups.
2026-10-14: Audited Sunken Valley m17_00 against installed event, GPARAM, native probe, collision and sky data; retained faithful presets and both afternoon ArenaSpecs, added an installed-data regression, and verified 110/100/110 walks in all five states; paired captures and uncapped-flag median/p99 measurements are under target/alt-m17/captures, with shared layered-normal/AO/specular gaps, absent dawn probes, unknown missing-row fallback and residual presentation pacing documented; the map regression, seven shared selector tests, collision regression, game all-target Clippy with warnings denied and formatting pass using target/alt-m17 and four jobs, while exact rendered parity and steady uncapped pacing remain unverified.
2026-10-14: Audited Senpou m20_00 story lighting and fixed the cleared illusion entry sky reference; two native m20 tests, six shared selector tests and the moving-collision regression pass, with game/all-target Clippy and workspace formatting clean using target/alt-m20 and four jobs; accepted before/after pairs, GPARAM/fog/probe measurements and uncapped start median/p99 42.912/62.535 -> 17.966/25.475 ms and heavy 63.371/155.242 -> 16.750/23.467 ms are under target/alt-m20/captures; missing illusion sky/environment-5 GI, empty sub-ID-10 family provenance, shared light/fog/probe omissions, night spikes and an incomplete physical boundary walk prevent claiming exact fidelity or steady frame time across all states; only m20 source and its documentation were changed, with no shared renderer, cache or git-state edits.
2026-10-14: Audited m11_01's nine lighting states, corrected native Tomoe/Emma/Owl encounter GPARAM presets, verified a grounded 100/110/100 boundary round trip, recorded before/after captures and presentation-limited perf measurements, documented shared renderer blockers, and passed targeted release tests, Clippy and formatting using target/alt-m11_01 with four jobs.

2026-10-14: Completed research-only map completeness audit in docs/MAP-COMPLETENESS.md from Smithbox, DSMapStudio, SoulsFormats and RustSekiro plus installed m11_00/m11_01 data.
Confirmed omitted Object categories, 99/298 ConnectCollision rows, lost visibility metadata and 93 Outskirts meshes available only at lower detail levels; recorded first-visit event/group branches, ranked gaps and eight separate fix jobs.
Two focused release MSB tests, formats all-target Clippy with warnings denied and final workspace formatting pass using target/alt-mr and four Cargo jobs.
Exact retail visibility-condition algebra, streaming policy, ID-255 LOD binding, complete first-visit replay and fresh missing-texture totals remain open; no source, cache/re, commit or publication changes.

## Explore-first menu (2026-10-14)

Explore map is the first/default mode, opens with Map selected, enables map objects and leaves first-visit story lighting to the per-map data; the Hour row is removed and direct --hour/--story testing flags remain available.
Number shortcuts 1-9/0 plus Enter select any extracted map; starts wrap through authored points and reset on map changes, while full map IDs and saved short-ID migration fix the old menu's invalid launch paths.
The existing fight choices and settings remain available after Explore; old saved Hour fields are discarded, and unset-hour equivalence uses to_bits for NaN.
With target/alt-mn and four jobs, the release game build, 84 game unit tests, six integration tests, all eight menu tests, game all-target release Clippy with warnings denied and workspace formatting pass.
Accepted menu/fight screenshots, Castle Enter and Outskirts Left/Enter launches, and raw WASD movement in all four directions are verified under target/alt-mn/captures; the early Outskirts sky-only shot and pre-walk Castle shot are not accepted as settled walking evidence.
The job edits menu.rs, docs/INTERFACE.md and this status entry only, with no commits, original-game runs, protected data writes or changes to map loading, lighting or rendering.
The initial no-enemy capture still rendered a passive soldier and health bar; the authorized follow-up below resolves that issue.
The extracted m89 sky library has no collision floor, so normal walking there remains an engine/data boundary.

## Executable map-rule research (2026-10-14)

Added the address-cited "Rules from the exe" section to docs/MAP-COMPLETENESS.md from read-only Ghidra inspection of the existing analyzed program.
Confirmed condition-byte mask preprocessing, the four separate slices inside the editor's CollisionMask, connection Condition as a group index, map-piece LOD ID 255 selecting row 0, bounds-distance hysteresis/crossfade and per-target connection-mask unions.
Traced navigation-driven Default/Hit request sets with retained retirement demand and staged block resources; exact navigation traversal, final release barriers, object model defaults and final LOD pixel behavior remain explicitly open.
The batch launcher initially failed on sandboxed Java settings, but direct headless startup with workspace-local settings succeeded using -noanalysis -readOnly; no program edits or re-analysis were requested, although Ghidra maintained transient project housekeeping files.
All scripts, raw exports and local audit results are under target/alt-gh; the offline version/condition/LOD audit and cargo fmt --all --check pass, with no Rust crate changes requiring Clippy or tests.
No game session, save/live-memory access, commit or publication occurred; the collaboration-board read was blocked by the configured proxy and the existing claim record was retained.

## Enemy-free Explore follow-up (2026-10-14)

Explore and CLI --no-enemy now omit the opponent model, encounter-specific spawns, name, health bar, lock-on and auto-aim targets; --passive retains its visible testing opponent and disabled AI.
The game-local adapter for the mandatory opponent slot prevents contacts and enemy-death resets while preserving Wolf's normal death/restart, without editing shared simulation or map loading code.
Debug details are hidden by default in Explore, F3 toggles player details only, and F3 is listed in both the menu and gameplay controls.
Fresh castle-start.png and outskirts-walk.png under target/alt-mn/captures are visually inspected with no enemy or enemy UI, zero locked-on frames despite --lock-on and raw Q requests, and successful Outskirts movement in all four directions.
Captures use the existing --no-connected-blocks switch and a process-local 64 MiB thread stack to isolate concurrent streaming and its IO-stack overflow; no map, lighting, shader or general rendering implementation is modified.
The complete early follow-up release test run passes 87 unit tests and six integration tests; the final repeat on concurrent source/data updates passes all 100 unit tests and five integration tests, with two slow tests ignored and map_tangents::exported_tangents_match_mikktspace failing on regenerated map data at handedness agreement 0.695.
Final game all-target release Clippy with warnings denied and workspace formatting pass, using target/alt-mn and four jobs; the tangent export/test remains with its rendering/data owner.
This follow-up changes play.rs, main.rs, menu.rs, input_replay.rs, docs/INTERFACE.md and this status entry, with no commits or writes to game data, saves, brain memory or session reports.

## Automated visual map sweep (2026-10-14)

Added the windowless --sweep game mode in map_sweep.rs and map_sweep.wgsl, with CLI/material/visibility integration and shader validation coverage.
It samples stacked walkable collision floors, eye-height yaws and cliff pitches, then measures below-horizon void, collision depth gaps, unavailable textures, grey candidates and normal-versus-all-visible differences.
Independent CPU collision rays guard each GPU capture against stale poses and invalid depth encoding.
Per-view JSON, category totals, worst-20 terminal summaries and only the worst screenshot sets are written under target/alt-hs/sweep.
Connected Castle baselines score 8,054,641 without objects and 8,195,856 with objects across 216 matching views; connected Outskirts scores 23,295,999 and 22,712,108 across 364 matching views.
The 80 m coarse sweeps take 17.4-25.8 minutes each, with complete sample flags, matching pose lists and exactly 80 retained PNGs per control.
Run commands, thresholds, baseline counts, executable fingerprint and coverage limits are documented in docs/MAP-COMPLETENESS-SWEEP.md.
Using target/alt-hs and four jobs, the final release game tests pass 100 unit tests and six integration tests with two existing slow tests ignored; game all-target Clippy with warnings denied and workspace formatting pass.
The freshly rebuilt executable's isolated Castle objects smoke capture passes in 21.7 seconds and its normal image is visually inspected.
Finer reachable-route coverage and interpretation of intentional sky, falls, neutral materials and alternate scenery remain open; the underlying renderer/data causes remain with their fix owners.
No commits, publication, original-game runs or writes to game data, saves, cache/re, brain memory or datasets were made by this job.

## Visual parity work list completed (2026-10-15)

Finished docs/VISUAL-PARITY.md with 51 accepted pairs at 48 spots and 63 rejected pairs from the 114 available comparisons, plus a ranked list of twelve causes with owning tracks, counts, asset candidates and evidence filenames.
Fog and sky remain one item, the seven uncertain valley cameras contribute no accepted defect count, and the two zero-demand helper collisions remain tool exceptions.
The final external progress review ledger supersedes the incomplete reference triage ledger, and eight byte-identical copies of accepted example pairs are retained under the progress root's 2026-10-15 directory.
Corrected unsupported draft claims about thousands of active grass emitters, absent layered-material handling, fog-surface identity/placement and story-state evidence from the already-closed gate view.
The contact-sheet tool now rebuilds pages after pose insertion/reordering and accepts a separate writable sheet output; the summary tool accepts an external annotation ledger, distinguishes rejected from unreviewed pairs and documents six build jobs without overwriting the hand-reviewed work list.
An initial contact-sheet command refreshed derived review sheets in the reference root; source screenshots were unchanged, and subsequent generated artifacts were directed to the progress root.
The final build, all-target game Clippy with warnings denied and workspace formatting pass with Cargo builds using target/alt-pt and six jobs.
The five Python matcher tests, source compilation, matcher CLI and both review/report commands pass using uv with the existing cached-wheel wrapper.
One formatting check caught a concurrent map_sweep.rs log-statement edit; the owner's correction passed the final recheck without this job editing that protected file.
No renderer/map fixes, game window, re-match, real-game/save/install access, commits or publication were performed for this completion.
2026-10-15: Corrected all 13 m89 dome-group overwrites, synchronized exported cloud layers and separated event sky/environment selection from clock-only changes in sky.rs and arena_b.rs; eight sky regressions, the environment/restart regression, before/after data gates, 118 game unit tests plus six integration tests, workspace Clippy and fmt pass using target/alt-sky and six jobs; matched spot-011/091 captures and gate numbers are under target/alt-sky/progress, with no per-view metric regression beyond tolerance and identical collision/visible-part manifests, but both canonical render gates remain blocked by shared grey/pop-in fixture failures already present with the correction off, and F1/D1 plus missing m900085 remain open.
2026-10-15: Shared sweep texture/pipeline readiness with ordinary, play, viewer and arena captures; fixed --uncapped being ignored outside maps and logged concrete supported presentation modes; historical/current ordinary and play proofs plus before/after six-view gates live under target/alt-capture/progress; 120 game unit and six integration tests pass with two ignored, workspace Clippy and fmt are clean, and both gates preserve per-view geometry exactly, with Castle's three pre-existing grey failures and sustained uncapped throughput still open (Vulkan Immediate/Mailbox medians 16.682/16.693 ms).
2026-10-15: Restored D1 snowy horizon terrain behind Outskirts village and Castle keep by using the native false default for omitted analytical-fog continuation and retaining Mountain snow/rock, mask, raw-normal/detail and baked-occlusion data; refreshed four placed model exports without layout or ancillary changes; all twelve geometry diagnostics and visibility selections stay identical, with four reviewed colour fixture fields documented in MAP-COMPLETENESS-SWEEP.md; frozen render gates pass in 68.552/85.507 seconds, the final shared-tree workspace run passes 814 tests with 25 ignored, and release all-target Clippy plus fmt pass using target/alt-mountains; four real/before/after panels and manifests are under the external 2026-10-15/mountains progress directory; the full-roster story audit moves to the slow tier, and no exposure/tone code, staging, commits or publication belong to this job.

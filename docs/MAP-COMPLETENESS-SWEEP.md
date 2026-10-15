# Map completeness sweep

The sweep measures rendered views above walkable collision using the shared map loader, materials, sky, lighting and draw-group selector.
It opens no window and never starts the real game or a gameplay/brain session.
All derived reports and screenshots stay under `target/alt-hs/sweep/<map>/pieces/` or `objects/`.
The gate commands below explicitly use target/alt-gt; the older broad-sweep default directory remains target/alt-hs.

## Run

The normal CPU data gate checks all ten extracted maps and skips only maps without a layout.
It reads MSB placement metadata, GLB JSON headers and file existence, without loading vertex buffers or starting Bevy's renderer.
The per-map numeric fixtures are `crates/game/tests/data/<map>.json`: type counts in MapPiece/Object/Enemy/Player/Collision/DummyObject/DummyEnemy/ConnectCollision order, missing exported models/declared image URIs/authored texture names, then ten `[MSB collision part index, primary visible count, connected visible count]` rows for each Ashina map.
The independent raw-MSB oracle also checks attached connection masks, signed target matching and the complete visible-part set across requested exported neighbours.
The visibility comparison disables event overrides and assumes ready resident assets so it tests the executable mask rule independently of story and loading policy.

```powershell
$env:CARGO_TARGET_DIR = 'target/alt-gt'
cargo test --release --offline -p sekiro-game -j 4 map_data_gate -- --nocapture
cargo build --release --offline -p sekiro-game -j 4
& target/alt-gt/release/sekiro-rs.exe --render-gate --map m11_00_00_00 --sweep-out target/alt-gt/render-gate
& target/alt-gt/release/sekiro-rs.exe --render-gate --map m11_01_00_00 --sweep-out target/alt-gt/render-gate
```

Run both render commands serially before reviewing a commit.
Each command starts one windowless process, loads the area and its bounded neighbourhood once, and renders every fixed view in that process.
The numeric `<map>.render.json` fixtures hold width, height and `[x,y,z,yaw,pitch,void,depth_gap,untextured,grey,pop_in]` rows, in Bevy world coordinates.
The gate uses ordinary Objects, the first-visit story fixture and native material settings, and rejects diagnostic overrides.
`report.json` retains all measurements, selected collisions and visible-part manifests; `gate.json` records pass/fail details.
Every category is checked separately at every view, with a tolerance of 16 pixels or 1% of that view's baseline, whichever is greater.
The gate also rejects a run exceeding 120 seconds, including CPU setup and map loading.
Baseline edits are manual review actions; running the gate never rewrites a checked-in fixture.
The fast fixed-view path instances static GLTF subassets directly, retaining native materials, node transforms, extras and LOD policy.
It loads the union of the requested frusta and nearby shadow casters conservatively, without a far-plane exclusion.
Use --sweep-all-assets with an ordinary --sweep control to compare against the unbounded ordinary scene loader.
Use --test-material-routing with an ordinary --sweep control to opt into the prepared connected-block layer-routing experiment; production routing and canonical gates leave it off.
The loader now waits for nested scene readiness and the texture diagnostic recognizes images already uploaded to the GPU after their CPU assets retire.

The required gates are the normal CPU data gate and the two six-view render commands above.
The optional reference tool is not yet reliable and is excluded from required gates.
Its supplementary fixture adds 150 distinct poses from the external screenshot index, at 256 x 144 pixels, with a separate 900-second budget.
The repeat failed four tolerances: pop-in at views 57, 61 and 62, and grey at view 66.
Shading variability remains open; further reference-repeat investigation is deferred.
Its numeric rows use player feet plus 1.65 m and the nearest authored player-start yaw because the position overlay does not supply camera rotation.
This is position-based reference coverage, not a claim of retail camera or story parity.
The 442 derived side-by-side comparisons are in target/alt-gt/reference/; source photographs are read directly outside the repository.

```powershell
& target/alt-gt/release/sekiro-rs.exe --reference-gate --map m11_00_00_00 --sweep-out target/alt-gt/reference-gate
```

Refresh a previously extracted legacy layout with `sekiro-extract map <id> --layout-only` before running the data gate.
That operation updates complete placement metadata using the raw MSB and existing material audit, preserving geometry, textures and collision.
An extracted but outdated or broken map fails the gate instead of silently skipping.

For investigation, `--sweep-views <json>` accepts a numeric array of fixed `[x,y,z,yaw,pitch]` rows and `--sweep-out <directory>` separates control reports.
`--sweep-raw-collision` retains the historical unfiltered collision reference, including below-landscape zero-demand planes, solely for reproducing old survey results.
Ordinary surveys exclude zero-demand and first-visit-disabled collision rows from both camera sampling and the collision reference.
This is a survey eligibility rule; it does not reinterpret retail physics filters or change gameplay collision.

```powershell
$env:CARGO_TARGET_DIR = 'target/alt-hs'
cargo build --release -p sekiro-game --bin sekiro-rs -j 4 --offline
$game = 'target/alt-hs/release/sekiro-rs.exe'
& $game --sweep --map m11_01_00_00 --spacing 80 --views 4
& $game --sweep --map m11_01_00_00 --spacing 80 --views 4 --objects
& $game --sweep --map m11_00_00_00 --spacing 80 --views 4
& $game --sweep --map m11_00_00_00 --spacing 80 --views 4 --objects
```

`--sample-only` prints the number of views without starting the renderer.
`--limit-views N` restricts the run to the first N views for a smoke test and writes to a separate `smoke-N/` subfolder; such runs are incomplete area samples.
`--spacing` controls the horizontal collision grid in metres, and `--views` is the number of evenly spaced yaw directions per point.
Defaults are 80 m and four yaws, with 1.65 m player eye height and a 60 degree vertical field of view.
`--edge-pitches -25,-50` adds those pitches in every yaw direction near detected cliff edges.
`--sweep-width 256 --sweep-height 144` controls image resolution.
`--settle-frames 4` controls the minimum wait after changing a view or render pass.
`--worst 20` controls the number of retained screenshot sets, with zero disabling screenshots.
Use the same map data, story selection, sampling options and image resolution when comparing a fix.
The existing `--cache` and story/lighting/rendering diagnostic flags are accepted by the shared map loader.
Runs overwrite `report.json`, and `partial.json` checkpoints preserve measurements if a run is interrupted.

## Sampling and measurements

Upward or downward wound collision triangles with slope at most 50 degrees populate horizontal grid cell centres.
The sampler keeps separate quarter-metre height layers, checks the simulation's supporting floor, and rejects points without 1.65 m overhead clearance.
It does not collapse overlapping roof, street and interior levels into one height.
A point is near an edge if any of four floor probes 2 m away finds no support or a drop exceeding 3 m.
The original collision-hole audit's walkable slope, supporting-floor queries and collision material exclusions are reused.
This is a visual survey rather than the older render-versus-collision traversal audit in [COLLISION-HOLES.md](COLLISION-HOLES.md).

Each view produces four offscreen GPU renders with identical camera geometry.
The normal view uses the shared map draw groups and normal frustum culling.
The all-visible comparison disables authored/group visibility and frustum culling for the already loaded map geometry, and asks the shared LOD selector for the highest available representation.
The visual diagnostic pass renders geometry at its real instance transform, preserving base texture alpha tests and material face culling.
A second diagnostic pass renders collision, excluding hit-material IDs 22, 36 and 38 as the simulation raycaster does.
Native, diagnostic geometry and collision have separate fixed render targets; only the requested pass renders.
Authored fog surfaces also supply diagnostic depth using their native scrolling masks and vertex coverage, with a 0.05 alpha cutoff.
Their soft intersection fade needs the native scene depth and remains omitted on the isolated diagnostic target.
Diagnostic passes exclude the sky and bypass fog, grading, native post processing and anti-aliasing.
Depth is Euclidean eye distance encoded logarithmically into two 8-bit channels, with zero reserved for no surface and the blue channel marking absent base textures or unavailable declared material textures.
Shader output compensates for the target's sRGB conversion so the depth bytes survive GPU readback.
Native normal/all renders share one authored exposure result and identical shader time to prevent adaptation or animated clouds/wet surfaces from being counted as visibility changes.
Each normal view meters with the recovered native reset rate, preserving the authored exposure bounds and grading, then holds that GPU history for its screenshot and all-visible comparison.
The sweep does not disable auto-exposure or change interactive exposure behavior.
The sweep waits for declared material textures and pending GPU pipelines and fails on permanent shader errors or stalled loading/captures.
The shared checks live in `crates/game/src/capture_ready.rs` and are also used by ordinary screenshots and play, viewer and arena captures.
Four sparse CPU collision rays per view independently validate the GPU depth encoding and camera pose; a mismatch exceeding 0.5 m plus 5% stops the run before publishing a baseline.

The per-view counts are:

| Field | Pixel criterion |
| --- | --- |
| `void` | A world-space ray points below the horizontal horizon and finds no rendered geometry within 1,000 m. |
| `depth_gap` | Collision within 100 m is more than max(2 m, 10% of collision distance) closer than rendered geometry, or rendered geometry is absent. |
| `untextured` | The frontmost diagnostic material has no base texture or an unavailable declared base/normal/roughness/occlusion/emissive/layer texture. |
| `grey` | A rendered surface pixel in the normal view has RGB spread at most 8 and each channel between 70 and 210. |
| `pop_in` | Normal/all RGB differ by more than 12 in any 8-bit channel. |
| `score` | Sum of all five counts, including overlapping categories. |

`report.json` contains all eye positions, yaws, pitches, edge flags, counts, settings, elapsed time, run timestamp, full-sample completion flag, category totals and the worst 20 rows.
The same worst 20 rows and totals are printed to the terminal.
Retained views have `NNNNNN-normal.png`, `-all.png`, `-depth.png` and `-collision.png` companions.
A higher score ranks a view for investigation; it is not proof of a defect.

## Interpretation and coverage limits

Clear colour and authored sky below the horizon are detected as absent local geometry rather than by assuming that sky has one fixed RGB colour.
Legitimate open vistas, water, intentional falls, distant scenery, neutral stone and deliberately untextured materials can therefore be flagged.
Grey and untextured counts are stand-in candidates, not assertions that every grey surface is wrong.
The depth pass preserves authored base glTF cutouts and face culling, but treats blended/additive/multiply materials as opaque wherever base alpha exceeds 0.05 so arithmetic blending cannot corrupt the encoded depth bytes.
Layered displacement, camera fade and translucent compositing can therefore differ from the native visual material.
Collision rays and rendered surfaces are sampled per pixel rather than with full player capsule traversal.
A coarse grid misses narrow corridors, small floors and cliff edges between sample points; reducing spacing is required before claiming stronger area coverage.
Sampling is deterministic but does not establish that every floor is reachable from a player start.
The sweep shares the loader's current authored/story visibility, but does not run gameplay EMEVD timelines or enemy/player animation.
`--objects` includes eligible exported MSB objects, including story-gated props in the all-visible control.
The pieces control filters authored Object parts even when the connected-block viewer normally includes ordinary objects by default.
The all-visible control includes alternate draw-group scenery and can reveal intentional stand-in swaps as well as bad pop-in.
The connected Ashina renderer's parameter-driven distance LOD selector is exercised in the normal view, with its highest available level forced by the all-visible control.
The sweep advances one deterministic 0.6 s pose-settling step before holding shader time; the installed LodParam maximum crossfade time is 0.2 s.
The shared renderer currently submits the dominant LOD rather than the retail dual-submission coverage shader.
A zero score only covers the sampled poses, image resolution and current exported assets.

## Fog fixture review (2026-10-15)

The corrected native fog changes colour-sensitive measurements without changing submitted geometry.
The suspected unfogged all-visible colour render was disproved: phases 0 and 1 reuse the same native camera, and fog is removed only from the separate diagnostic cameras.
The sweep now compares the complete optional GameFog uniform at both colour captures and fails immediately if it differs, including an enabled/disabled mismatch.
The one-view Outskirts control uses the canonical view-0 pose and 128 x 72 resolution, ordinary Objects and the default story.
With `--no-fog` in both colour renders it gives grey 5 and pop-in 8707; with identical authored fog in both it gives grey 17 and pop-in 8753.
Both controls give void 0, depth gap 75 and untextured 0.
These controls are under `target/alt-fog/final/control-off/` and `control-on/`, with complete capture-time uniforms in `target/alt-fog/control-off.log` and `control-on.log`.
Disabling fog is an isolation control, not an exact reconstruction of the previous implementation's incorrect bank direction, sky endpoint and integration.

The following table lists every manually reviewed fixture change from the committed baseline.
Each grey change follows from the fog's corrected scattering colour and attenuation moving pixels across the existing neutral-colour criterion.
Each pop-in change follows from applying the corrected depth-dependent fog to the existing normal/all-visible surfaces before the unchanged 12-code-value comparison; different visible surfaces retain different depths and incident lighting even with identical fog parameters.
Fog can therefore move an existing colour difference across that threshold in either direction.
The normal view's authored exposure metering also sees the corrected scattering, and its result remains held through the comparison.
The fixed cameras, all geometry metrics, visibility fixtures, tolerance of 16 pixels or 1%, and 120-second budget are unchanged.

| Map | View | Grey before -> fixture | Pop-in before -> fixture |
| --- | ---: | ---: | ---: |
| Outskirts | 0 | 6 -> 17 | 8663 -> 8753 |
| Outskirts | 1 | 111 -> 97 | 198 -> 191 |
| Outskirts | 2 | 9 -> 15 | 2415 (unchanged) |
| Outskirts | 3 | 249 -> 155 | 4752 -> 5131 |
| Outskirts | 4 | 0 (unchanged) | 3607 -> 3586 |
| Outskirts | 5 | 27 -> 37 | 4172 -> 4277 |
| Castle | 0 | 13 -> 31 | 5088 -> 5097 |
| Castle | 1 | 9 -> 7 | 7507 -> 7451 |
| Castle | 2 | 264 -> 200 | 4597 -> 4975 |
| Castle | 3 | 154 -> 34 | 3983 -> 3785 |
| Castle | 4 | 106 -> 107 | 2518 -> 2514 |
| Castle | 5 | 22 -> 20 | 2887 -> 2842 |

The final independently rerun gates pass all six views in 37.4 seconds and 42.4 seconds respectively, under `target/alt-fog/final/render-gate/`.
Outskirts repeats every reviewed fixture count exactly.
Castle's repeat grey/pop-in pairs are 31/5098, 7/7452, 201/4975, 34/3785, 104/2520 and 24/2851, all within the unchanged tolerances.
Sorting the visible-part identities before comparison proves all twelve manifests and selected collisions equal the preceding fog-correction run; entity iteration order is not a visibility change.
Outskirts totals remain void/depth-gap/untextured 0/493/0, and Castle retains 1933/4886/903.
The gate still measures existing all-visible differences, including alternate scenery and LOD; passing these fixtures is not a claim of zero pop-in or retail pixel parity.

## Historical coarse baseline

These earlier concurrent-working-tree measurements are not the checked-in fixed-view gate baselines.
Current ordinary surveys exclude zero-demand helper planes, so their pose count and collision oracle differ from this historical run.
The --sweep-raw-collision diagnostic retains those planes for investigation, but other intervening renderer changes still prevent treating the old executable/cache snapshot as a current reproducible baseline.

Recorded on 2026-10-14 using the current connected Ashina renderer and extracted cache, with no story/hour override.
The baseline executable was built from the shared working tree during concurrent renderer and extraction work, so these are session baselines rather than a reproducible committed-source/cache snapshot.
The capture executable SHA-256 was `adc73fd522ad3754295b6c34d536e4ce61ec34ee3dd315d0b06b4c57dcfd038b`.
All four controls use 80 m spacing, four yaws, edge pitches -25 and -50 degrees, 256 x 144 pixels, four settling frames, fixed exposure and the same deterministic shader clock.
These are full samples at that coarse spacing, not exhaustive checks of narrow corridors or individual wall faces.
Castle has 216 views (7,962,624 view pixels), and Outskirts has 364 views (13,418,496 view pixels).
Category totals count repeated visibility across poses, and the score includes category overlaps.

| Map | Objects | Views | Minutes | Void | Depth gap | Untextured | Grey | Pop-in | Score |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| m11_01_00_00 | off | 216 | 17.43 | 1,435,559 | 2,450,981 | 167,430 | 112,899 | 3,887,772 | 8,054,641 |
| m11_01_00_00 | on | 216 | 21.22 | 1,443,309 | 2,496,056 | 152,011 | 107,618 | 3,996,862 | 8,195,856 |
| m11_00_00_00 | off | 364 | 25.84 | 5,887,518 | 8,320,862 | 30,573 | 179,677 | 8,877,369 | 23,295,999 |
| m11_00_00_00 | on | 364 | 24.53 | 5,886,557 | 8,013,010 | 88,668 | 142,584 | 8,581,289 | 22,712,108 |

The coarse baseline used `--spacing 80 --views 4 --settle-frames 4`, adding `--objects` for the objects control.
The JSON reports retain the worst 20 poses and every per-view count, and the four matching screenshots for the worst 20 points are in each control folder.
Both controls have identical pose lists for each map, all four reports mark their coarse sample complete, and each control has exactly 80 retained PNGs.
Castle without objects ranks point 87 first: eye [120, -13.263, 280], yaw 270 degrees, pitch -50 degrees, with 36,864 void, 35,327 depth-gap and 36,752 pop-in pixels (score 108,943).
Its normal image is entirely clear sky, while the collision and all-visible images contain nearby surfaces.
This is direct evidence of missing submitted geometry at that pose, although a coarse collision sample can still lie on an inaccessible or unintended surface.
Outskirts without objects ranks point 100 first: eye [-200, -74.044, -40], yaw 180 degrees, pitch -50 degrees, with 36,864 pixels each of void, depth-gap and pop-in (score 110,592).
Its all-visible screenshot exposes nearby terrain, undersides and pink missing-texture fallback surfaces, illustrating why the comparison is useful for triage but cannot by itself identify a single renderer cause.

## Validation

The focused sampler and metric tests cover stacked floors, interior grid points, cliff pitches, horizon classification, collision gaps, texture flags, grey pixels and visibility differences.
The existing shader integration test compiles the diagnostic extension with the normal PBR permutations and a collision mesh without UVs, tangents or vertex colours.
End-to-end captures use real extracted maps and validate collision depth independently at every reported view.
The final `cargo test --release -p sekiro-game -j 4 --offline` run passed 100 unit tests and six integration tests, with two existing slow tests ignored.
`cargo clippy -p sekiro-game --all-targets -j 4 --offline -- -D warnings` and `cargo fmt --all --check` passed.
All builds and checks used `CARGO_TARGET_DIR=target/alt-hs`.
The freshly rebuilt executable also passed an isolated Castle objects smoke capture in 21.7 s with CPU depth validation and a visually inspected normal image.

## Authored-light colour fixture review, 2026-10-15

The matched three-way controls are `target/alt-lights/final-compare/{before,btl-only,after}/gates/`.
They use one frozen executable: runtime-off restores this job's old lighting, BTL-only retains corrected local placement/upload with the four-slot compositor disabled, and after enables both corrections.
Every void, depth-gap and untextured count is identical across all three controls at all twelve views.
Every selected collision and sorted visible-part manifest is also identical.
Outskirts retains geometry totals 0/493/0; Castle retains 1933/4885/903 in these matched controls.
Castle view 0 already measures depth gap 148 in the runtime-off control against the older fixture's 149; its unchanged geometry fixture is retained within the existing tolerance.

Only the two colour columns are updated.
Camera coordinates, resolution, geometry fixture columns, first-visit state, 16-pixel-or-1-percent tolerances and the 120-second budget are unchanged.
The following table accounts for every changed colour fixture value; the middle columns isolate the much smaller BTL-only response from the recovered LightSet compositor.
Grey is the existing neutral-colour threshold, and pop-in is the existing normal/all-visible colour threshold, so neither is a geometry measurement.

| Map | View | Grey old fixture | Grey off / BTL / new fixture | Pop-in old fixture | Pop-in off / BTL / new fixture |
| --- | ---: | ---: | --- | ---: | --- |
| Outskirts | 0 | 17 | 16 / 16 / 215 | 8753 | 8751 / 8751 / 8711 |
| Outskirts | 1 | 97 | 97 / 102 / 183 | 191 | 191 / 189 / 294 |
| Outskirts | 2 | 15 | 18 / 18 / 383 | 2415 | 2420 / 2420 / 2484 |
| Outskirts | 3 | 155 | 155 / 155 / 655 | 5131 | 5131 / 5131 / 5503 |
| Outskirts | 4 | 0 | 0 / 0 / 648 | 3586 | 3590 / 3586 / 6831 |
| Outskirts | 5 | 37 | 37 / 37 / 147 | 4277 | 4277 / 4277 / 4817 |
| Castle | 0 | 31 | 31 / 33 / 196 | 5097 | 5062 / 5056 / 5083 |
| Castle | 1 | 7 | 7 / 8 / 288 | 7451 | 7451 / 7447 / 7488 |
| Castle | 2 | 200 | 198 / 198 / 517 | 4975 | 4976 / 4976 / 5023 |
| Castle | 3 | 34 | 35 / 35 / 49 | 3785 | 3764 / 3764 / 3743 |
| Castle | 4 | 107 | 119 / 116 / 482 | 2514 | 2523 / 2527 / 2400 |
| Castle | 5 | 20 | 18 / 21 / 92 | 2842 | 2768 / 2769 / 2871 |

Outskirts view 0 has no measured BTL-only change; its grey increase and reduced colour difference come from the four-slot diffuse/specular response and its exposure metering.
View 1 has a small BTL-only shift of +5 grey and -2 pop-in pixels, followed by the larger native-LightSet changes shown above.
View 2 has no BTL-only shift, isolating its increased neutral pixels and normal/all-visible contrast to LightSet shading.
View 3 likewise isolates both increases to the recovered direct response on the already different normal/all-visible surfaces.
View 4 keeps zero BTL-only grey pixels and a four-pixel pop-in shift, while the full compositor moves many wall pixels into the neutral band and increases contrast against its existing all-visible geometry.
View 5 has no BTL-only shift; both increases are from the direct compositor and held-per-pair exposure.

Castle view 0 has BTL-only changes of +2 grey and -6 pop-in pixels; the larger neutral-colour increase is from LightSet, while the complete change reduces pop-in from the old fixture.
View 1 has BTL-only changes of +1 grey and -4 pop-in pixels, with its full increase following the direct response.
View 2 has no BTL-only shift, isolating both full changes to independent diffuse/specular lighting.
View 3 has no BTL-only shift, and the direct compositor adds 14 neutral pixels while reducing the colour-difference count by 21.
View 4 has small BTL-only shifts of -3 grey and +4 pop-in pixels, then a larger LightSet neutral-colour increase and reduced normal/all-visible contrast.
View 5 has small BTL-only shifts of +3 grey and +1 pop-in pixels, then the full direct response raises both criteria.
Small differences between an old fixture and its fresh runtime-off control remain within the original tolerances and are not attributed to a particular lamp.
The larger four-slot changes are accepted because they implement recovered input, direction and accumulation rules; no authored colour, intensity or gate threshold was adjusted to achieve these counts.

The runtime-off, BTL-only and full controls take respectively 40.6/41.0/42.2 seconds for Outskirts and 48.1/55.2/70.4 seconds for Castle.
The full controls deliberately fail the preceding colour fixtures before this manual review, while passing every geometry and time check.
These controls establish the cause of the fixture changes, not retail pixel parity or a claim that the existing all-visible differences are repaired.

The final independently rerun production-default gates pass all six views in 40.7 seconds for Outskirts and 48.8 seconds for Castle.
Their reports are under `target/alt-lights/final/gates/`, and `accepted-comparison.json` proves all twelve final geometry measurements, selected collisions and sorted visible-part manifests equal the runtime-off controls.
Final totals are Outskirts geometry 0/493/0, grey 2231, pop-in 28639; Castle geometry 1933/4885/903, grey 1619, pop-in 26630.
The reviewed fixture tolerances remain unchanged, including the small independent colour-repeat differences.

Night PNGs are under `target/alt-lights/progress/` with paired `lights-<view>-before.png` and `lights-<view>-after.png` names.
The requested views are `shrine`, `interior`, `fire` and `outskirts-start`; additional pairs cover `castle-idol` and the first Castle gate camera labelled `castle-start`.
The before control uses `SEKIRO_MAP_LIGHTS_LEGACY=1` in the same executable as after, preserving the preceding GI, fog, sky and material work.
The pair-derived eye, FOV and indexed player position are reused, with equal capture metadata and selected collisions verified for all six pairs.
The final capture executable is `target/alt-lights/frozen-final/sekiro-rs.exe`, SHA-256 `15c26c2dd0de14ad70dd0ae885253ab95e5b8166cb4b36c335f1b792a7b0f628`.
The optional camera-follow fill uses the same recovered conversion as the two local slots; that fill-follow flag is off in the observed Ashina packets.

The shrine and hall show substantially more interior/wood detail with all four LightSet slots active.
Night snow remains excessively bright and blue against the retail pairs, with hard lighting and material/normal differences still visible, so L1 is not closed.
The battlefield fire at spot-045 still has no flame or smoke, and the Castle idol at spot-123 still has no blue flame, mist or effect glow.
Those missing effect presentations belong to E1 FX integration; these images do not certify that every remaining dark fixture is FX-only or that its emissive response is correct.
The hut at spot-016 was reviewed from its existing external pair and the native-radius census, rather than counted as an additional fresh capture.
No external reference image was copied or modified.

Stationary performance uses windowless 1600 x 900 native-colour rendering at the Outskirts start and first Castle gate camera, with the same night state and a frozen shader clock.
Each process warms the first capture, then measures seven further identical poses with 1000 settling frames each, staying within the existing CLI bound.
Statistics exclude the first second after every preceding readback and the final 0.2 seconds before the next image, avoiding capture/PNG work in the measured intervals.
The table reports the median of one-second smoothed frame-time and GPU-opaque diagnostic snapshots, not per-frame percentiles or an unbounded interactive-viewer FPS guarantee.
The scene loader retains the same conservative fixed-frustum/shadow-caster scope in both modes, and no CPU compiler was running in the retained steady intervals.
Commands, complete diagnostics, sample counts and ranges are in `target/alt-lights/performance-long/` and `frame-comparison.json`.

| Night camera | Frame ms before / after | Opaque GPU ms before / after | Retained snapshots before / after |
| --- | ---: | ---: | ---: |
| Outskirts start | 3.801 / 4.845 | 1.995 / 3.121 | 17 / 31 |
| Castle courtyard | 3.419 / 3.408 | 0.971 / 1.513 | 14 / 16 |

The extra direct response raises opaque GPU cost in both views; Castle wall-frame timing remains near its CPU/scheduling floor rather than demonstrating a speedup.
The six-view end-to-end budget remains 120 seconds and both final runs pass it.

`cargo test --release --offline --workspace -j 6` passes 806 tests with 24 existing ignores across the workspace binaries and doctests.
That run includes 122 game unit tests, six game integration tests and the normal all-map data gate.
The final sharpness-field separation check and all layered shader permutations also pass after the last packet/shader corrections.
Release workspace all-target Clippy with denied warnings, workspace formatting and scoped whitespace checks pass under `CARGO_TARGET_DIR=target/alt-lights` with six jobs.
The implementation changes are the BTL reader/export metadata, new `map_lights.rs`, LightSet packing/composition, idempotent performance-probe installation and the small `map.rs` integration; `main.rs` was not edited by this job.
The existing grass, GI, fog, sky, presentation and material work is retained, and no commit or publication was made.

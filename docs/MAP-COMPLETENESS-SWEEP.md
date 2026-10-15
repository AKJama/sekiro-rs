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
Layer textures resolve from their owning GLB in production, preserving the block's existing `models/tex/` layout.
Use `--test-primary-material-prefix` with an ordinary `--sweep` control to reproduce the former wrong-folder requests; canonical gates reject that diagnostic flag.
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
The sweep waits for declared material textures to reach the GPU before every capture phase, then waits for three distinct completed renders with no pending pipelines.
It also checks that normal/all-visible material handles, declared texture handles and readiness are identical within each pair.
Permanent shader errors, stalled loading/captures and failed declared texture requests in any resident block fail the render gate.
`texture-audit.json` records material GLB provenance and texture requests by owning block, including normally hidden and occluded pieces.
Each view's `material_blocks` records material, mesh-instance, untextured-material and untextured-mesh-instance counts plus distinct failed requests per block, independently of the frontmost-pixel metric.
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

## Owning-block texture routing review, 2026-10-15

The historical routing decision above is superseded by production owning-GLB routing and an independent connected-block texture audit.
The defect was in additional layer samplers: standard GLTF textures already resolved from the owning model, but `sekiro_layers` requests used the primary map's prefix.
No textures were copied, and no exporter layout or cached asset was changed.
The same original-material cache key is retained, so changing visibility does not rebuild or re-key a material.

The frozen preceding lights executable reproduces 459 distinct failed DDS paths with Outskirts primary and 429 with Castle primary.
All requests target the primary block's `models/tex/` despite matching declarations and files under neighbouring blocks.
The following inventory intersects those runtime failures with owning-model GLB declarations and existing exported files.
Shared sampler names can have multiple owners, so these columns deliberately do not sum to the distinct failed-path totals.
The complete model, texture-name and requested-folder inventories are in `target/alt-tr/texture-errors.json`.

| Owning models | Outskirts primary failures | Castle primary failures |
| --- | ---: | ---: |
| Outskirts | 0 | 86 |
| Castle | 59 | 0 |
| Reservoir | 85 | 64 |
| Dungeon | 95 | 96 |
| Valley | 104 | 104 |
| Senpou | 144 | 91 |

The pixel diagnostic already traversed neighbour roots and checked their layer handles.
Its zero Outskirts count was a frontmost-normal-view coverage limit, not proof that every resident material was valid.
Affected neighbour surfaces can be hidden by normal visibility or occluded, while contributing to the deliberately forced all-visible comparison.
The new `material_blocks` counts and `texture-audit.json` include those materials regardless of visible pixel coverage, and any failed declared request is a hard gate failure without a numeric tolerance.
Intentional materials with no base sampler remain separately identifiable through `untextured_materials`; they are not treated as failed file requests.

The old routing flag was replayed in the frozen preceding executable, separately from the current readiness changes.
All twelve void/depth-gap measurements, selected collisions and visible-part sets remain equal to the wrong-prefix controls.
Outskirts view 5 gains 480 newly flagged normal/all-visible colour pixels and loses 52, of which 477 new candidates have byte-identical normal pixels.
Castle view 5 gains 4,124 and loses 37, of which 3,341 new candidates have byte-identical normal pixels.
Those pixels therefore change because the all-visible neighbour rendering changes, not because a newly appearing surface changed the normal image.
The complete per-view byte comparisons are in `target/alt-tr/old-patch-comparison.json`.

The corrected controls wait for GPU texture uploads before every phase and verify identical resident material/texture identities and readiness between normal/all-visible captures.
They reproduce the old-patch Outskirts view 5 count of 5,245 and Castle views 2/3/5 counts of 5,879/3,782/6,958 exactly.
This rules out unequal texture-loading times and material re-keying as the cause of those increases.
Correct neighbour texture bindings change the rendering of already different normal/all-visible surfaces; the metric is a spatial colour comparison, not a measurement of temporal streaming transitions.
The existing intentional alternate scenery and highest-detail all-visible control remain unchanged.
An independent control triples settling from four to twelve frames in every phase, retaining GPU readiness and binding checks, and takes 83.3/104.8 seconds.
Its key Outskirts view 5 pop-in count is 5243, while Castle views 2/3/5 give 5879/3782/6959, within two pixels of the corrected four-frame controls.
Every void/depth-gap/untextured count remains unchanged, and `target/alt-tr/settled/` plus `settled-comparison.json` retain this control.
The original large colour increases therefore persist with extra settling and are not accepted as a texture-loading race.

Only the following seven fixture cells are revised after this review.
No camera, resolution, void/depth-gap fixture, geometry tolerance, colour tolerance or runtime budget is changed.

| Map | View | Changed fixture cells | Reason |
| --- | ---: | --- | --- |
| Outskirts | 0 | None | No material-related metric shift beyond the existing tolerance. |
| Outskirts | 1 | None | Both native images and all pixel metrics match the wrong-prefix control. |
| Outskirts | 2 | None | Correct layers slightly change colour on existing surfaces, within tolerance. |
| Outskirts | 3 | None | Correct layers change existing surface colour and 16 additional normal/all candidates, within tolerance. |
| Outskirts | 4 | None | The small colour-repeat differences remain within tolerance. |
| Outskirts | 5 | Pop-in 4817 -> 5245 | Correctly textured forced-visible neighbours add 480 and remove 52 threshold candidates; 477 additions leave the normal pixel unchanged. |
| Castle | 0 | None | Existing geometry is unchanged and the small colour response remains within tolerance. |
| Castle | 1 | None | Correct layers slightly change existing colours, within tolerance. |
| Castle | 2 | Untextured 898 -> 0, grey 517 -> 588, pop-in 5023 -> 5879 | Neighbour layer requests succeed; corrected roof-vista shading removes every missing-texture pixel and changes neutral/normal-all colour thresholds on existing geometry. |
| Castle | 3 | Untextured 5 -> 0, pop-in 3743 -> 3782 | The remaining five missing-texture pixels are repaired; 42 new and three resolved colour candidates give the +39 net change, with 35 additions leaving the normal pixel unchanged. |
| Castle | 4 | None | Correct layers slightly change existing colours, within tolerance. |
| Castle | 5 | Pop-in 2871 -> 6958 | Correctly textured forced-visible neighbours add 4124 and remove 37 candidates; 3341 additions leave the normal pixel unchanged. |

The corrected controls retain all twelve void/depth-gap counts, selected collisions and sorted visible-part manifests.
Collision PNGs are byte-identical in decoded RGB at every view, and diagnostic depth RG is byte-identical at all twelve views.
The independent retained controls are in `target/alt-tr/before/gates/`, `target/alt-tr/old-patch/` and `target/alt-tr/after/gates/`, with measurements in `corrected-comparison.json`.

The final optimized production-default gates pass all six views in 67.5 seconds for Outskirts and 67.3 seconds for Castle under `target/alt-tr/final/gates/`.
Their totals are Outskirts void/depth-gap/untextured 0/493/0, grey 2229 and pop-in 29009; Castle 1933/4885/0, grey 1693 and pop-in 31544.
All six blocks report zero failed declared texture requests, and every view confirms equal resident normal/all-visible material bindings and readiness.
The per-phase readiness poll deduplicates shared material handles, retaining the complete mesh-instance audit without repeating image checks for every instance.
The earlier unoptimized GPU-ready control took 99.2/132.6 seconds, with Castle failing the unchanged 120-second budget; it is retained as an explicit performance failure, not accepted as a gate pass.

Final selected collisions and sorted visible-part sets equal the frozen before controls at all twelve views, and every void/depth-gap count is unchanged.
Decoded collision depth is identical everywhere; one Castle view 4 diagnostic-depth pixel at [88,42] moves from encoded 11217 to 11218, with no changed geometry count or texture flag.
That one-code rounding difference also occurs in the old-patch repeat; the earlier corrected control gives fully identical diagnostic depth.
The comparisons are in `target/alt-tr/final-comparison.json` and `depth_delta.py`.
No geometry fixture cell is updated for that repeat variation.

The frozen final executable is `target/alt-tr/routing-final.exe`, SHA-256 `447a0aaacfb36362b79739bec4a4193b0ce83e51baeed88d9edff1f964c551cf`.
The production-default final gates include the concurrent map-FX integration available in that build; the routing-isolation controls and progress PNGs explicitly use `SEKIRO_NO_MAP_FX=1` to match the preceding executable's absence of those effects.
No fixture is changed for the small additional FX-on colour shifts.

Read-only reference review uses external `pairs/spot-006-shot-026.png` for the Castle vista from Outskirts and `pairs/spot-145-shot-428.png` for the Castle return view over the Outskirts scenery.
The indexed camera/player positions and 43-degree FOV are reused for matched 1600 by 900 night captures, with 32 settling frames.
`target/alt-tr/progress/` contains `routing-outskirts-before.png`, `routing-outskirts-after.png`, `routing-castle-before.png` and `routing-castle-after.png`.
The pose, FOV, selected collision and sorted visible-part set are equal in each pair, as checked by `photo-comparison.json`.
Only 37/396 pixels respectively change by more than 12 RGB code values in these normal views, consistent with the missing requests mainly affecting hidden, occluded or distant neighbour surfaces.
The reference review still shows excessive blue/bright snow and dark or incomplete distant Castle shading, so this routing fix does not close retail material, lighting, sky or visibility fidelity work.
No external reference image was copied into the repository or modified.

The release game suite passes 126 unit tests and six integration tests, with two existing ignored tests.
The normal data gate passes, and the final focused audit tests also verify two hidden neighbour mesh instances sharing one material are both counted.
Release workspace all-target Clippy with denied warnings passes with six jobs under `CARGO_TARGET_DIR=target/alt-tr`.

## Native map FX colour review, 2026-10-15

The MSB, ModelSfxParam, event-object and registered-idol TAE paths are documented in MAP-FX.md.
The frozen executable for this comparison is target/alt-fx/frozen/sekiro-rs.exe, SHA-256 7aa59510e01da0f3f75dc778f00799db9bc8f7e301d7ae877e1394238f53c476.
SEKIRO_NO_MAP_FX=1 supplies the before control in that same executable.
All twelve off/on void, depth-gap and untextured measurements are identical, with the same selected collisions and sorted visible-part sets.
Geometry totals remain Outskirts 0/493/0 and Castle 1933/4885/0, including the concurrent owning-block texture-routing fix.
Every collision PNG and eleven depth PNGs are byte-identical.
Castle view 4 differs at depth pixel (88,42), from encoded RGB (43,209,0) to (43,210,0), a single quantization step; no measured geometry category changes.
The pixel record and complete comparison are under target/alt-fx/gates/.

Only Outskirts view 0 grey changes from 215 to 274 and view 3 grey from 655 to 725 in the checked-in fixtures.
No camera, geometry fixture, other colour fixture, tolerance, resolution, story flag or time budget changes.
The following table retains every view's prior fixture, fresh runtime-off control and FX-on colour result.
Grey and pop-in are colour thresholds, not geometry counts.

| Map | View | Grey old / off / on | Pop-in old / off / on |
| --- | ---: | --- | --- |
| Outskirts | 0 | 215 / 215 / 274 | 8711 / 8711 / 8650 |
| Outskirts | 1 | 183 / 183 / 189 | 294 / 294 / 295 |
| Outskirts | 2 | 383 / 378 / 378 | 2484 / 2484 / 2486 |
| Outskirts | 3 | 655 / 657 / 725 | 5503 / 5519 / 5422 |
| Outskirts | 4 | 648 / 648 / 641 | 6831 / 6832 / 6756 |
| Outskirts | 5 | 147 / 147 / 150 | 5245 / 5245 / 5241 |
| Castle | 0 | 196 / 197 / 202 | 5083 / 5093 / 5098 |
| Castle | 1 | 288 / 287 / 290 | 7488 / 7488 / 7487 |
| Castle | 2 | 588 / 587 / 586 | 5879 / 5878 / 5878 |
| Castle | 3 | 49 / 50 / 51 | 3782 / 3782 / 3782 |
| Castle | 4 | 482 / 486 / 480 | 2400 / 2447 / 2362 |
| Castle | 5 | 92 / 89 / 90 | 6958 / 6960 / 6943 |

Outskirts view 0 gains 59 neutral pixels and loses 61 colour-difference pixels with the native temple idol, snow and effect lighting active.
View 1 changes by six neutral pixels and one colour-difference pixel, within the existing colour tolerances.
View 2 has no off/on grey change and a two-pixel pop-in increase; its five-pixel grey difference from the older fixture is already in the runtime-off control.
View 3 gains 68 neutral pixels and loses 97 colour-difference pixels with authored local effects, their light and snowfall; only its grey fixture needs review.
View 4's brazier/fire presentation reduces grey by seven and pop-in by 76 pixels; the pre-Gyoubu boss idol stays disabled.
View 5 changes by three neutral pixels and four fewer colour-difference pixels, without exceeding its original tolerances.

Castle view 0 changes by five pixels in each colour category with the map effects active, within the existing tolerances.
View 1 gains three grey pixels and loses one pop-in pixel.
View 2 loses one grey pixel with unchanged pop-in; the small difference is not attributed to an individual emitter.
View 3 gains one grey pixel with unchanged pop-in, while retaining its existing void and depth-gap counts.
View 4 loses six grey and 85 pop-in pixels with FX; its runtime-off control already differs from the older pop-in fixture by 47 pixels and deliberately fails that colour threshold.
That prior/control difference is not evidence of an FX regression and no Castle fixture is changed for it.
View 5 gains one grey pixel and loses 17 pop-in pixels, remaining within the existing fixtures.
These small shifts are threshold observations rather than proof of individual particle contributions or retail pixel parity.

The off controls take 85.1 seconds for Outskirts and 91.1 seconds for Castle, and the initial on controls take 70.2 and 80.7 seconds under concurrent workspace testing.
The initial Outskirts on gate fails only the two reviewed grey cells; Castle on passes every gate.
The 120-second budget and the existing 16-pixel-or-one-percent category tolerances are retained.

Matched 1600 by 900 night PNGs use the stored external pair camera, player and 43-degree FOV for spot-002 shots 004/005, spot-045 shot-160 and the extra spot-053 shot-186 review.
All four pairs retain equal poses, FOVs, selected collisions and visible-part sets, as checked by target/alt-fx/progress/photo-comparison.json.
The requested images are fx-snow-before/after.png, fx-fire-before/after.png and fx-idol-before/after.png under target/alt-fx/progress/.
The extra fx-idol-053-review pair retains the reference's story mismatch: the native first-visit fixture disables Gyoubu's idol, so the temple idol supplies the glow check.
Reference images were read in place, without copying them into the repository.
The captures show flakes, the blue idol flame and authored fire/smoke, while fire density/shape, particle lighting, the scene's blue cast and the documented native GPU/bonfire-controller approximations remain fidelity work.

The final independently rerun default gates pass in 75.5 seconds for Outskirts and 88.1 seconds for Castle after the two grey-cell updates.
Every final per-view geometry metric, selected collision and sorted visible-part set still equals the runtime-off control, as recorded in target/alt-fx/gates/final-comparison.json.
Final colour totals are grey 2353 and pop-in 28842 for Outskirts, and grey 1700 and pop-in 31551 for Castle.
Small repeat changes from the initial on captures stay within the unchanged tolerances and do not prompt further fixture edits.
The normal release workspace run reports 811 passed and 24 existing ignores, including 126 game unit tests and six integration tests, and the final focused data gate exits zero.
Release workspace all-target Clippy with denied warnings and workspace formatting pass with six jobs under target/alt-fx.

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

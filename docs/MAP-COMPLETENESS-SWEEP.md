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
The canonical Ashina gate uses ordinary Objects, the named post-Genichiro reference progression and native material settings, and rejects diagnostic overrides.
Each render fixture declares `story: genichiro-defeated`, which must agree with the canonical selection policy.
Ordinary exploration and unqualified broad surveys keep their first-visit default.
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
The earlier first-visit repeat failed four tolerances: pop-in at views 57, 61 and 62, and grey at view 66.
The optional reference gate now also selects the named reference story, but its old supplementary colour cells have not been re-baselined for that state and are not claimed as passing.
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

## Live play check

The sweep and render gates never run the real play path, so they missed a live regression (2026-10-16): Explore in Ashina Outskirts took about 125 s to become playable, ran at 10 to 20 fps, and froze the closed window for about 27 s.
The live check runs the play binary exactly as the Explore menu launches it, in a window, and measures what a player sees.
It is an ignored (slow-tier) test because it needs a GPU, a desktop session and the extracted cache, and it opens one game window at a time.

```powershell
$env:CARGO_TARGET_DIR = 'target/alt-live'
cargo test --release --offline -p sekiro-game live_play -- --include-ignored --nocapture
```

The test runs four sessions of `--play --map m11_00_00_00 --start 0 ... --no-enemy` with `--live-check <summary.json>`.
The first walks a short input script out of the start hall; the second uses `--live-tour starts` to move Wolf to every authored player start; the third tours Castle starts 300 to 600 m away.
The fourth spawns on the ledge east of the ravine north of start 0, grapples to the branch over it and turns the camera a full circle there with `--live-orbit 400,8`, and asserts that the Outskirts block keeps its display groups and that drawn map meshes stay near Wolf in every sample.
It asserts the time from launch until the loading screen lifts (at most 90 s), the longest frame while loading after the first (at most 2 s, below Windows' 5 s "Not Responding" limit), the median and 95th-percentile frame times from the first playable frame (at most 20 ms and 40 ms), that Wolf walked, that every grounded sample and every tour stop (every Outskirts player start, then six Ashina Castle starts reached from the Outskirts load) has a drawn map triangle on the collision floor under Wolf, and that at least 90% of the collision floor within 20 m lies inside drawn map meshes.

The same flags work by hand on any play command:

```powershell
& target/alt-live/release/sekiro-rs.exe --play --enemy soldier --map m11_00_00_00 --start 0 --brain lua --progression castle-roof --player default --objects --simple-ai --no-enemy --live-check live.json --live-shots shots --script walk.txt
```

`--live-check <file>` writes the summary and exits after the script (plus two seconds), after `--live-tour` (`x,y,z;x,y,z` or `starts`), or after `--live-seconds` (default 20) of unscripted play.
`--live-shots <dir>` saves `play-start.png`, the first frame without the loading screen, one `tour-NN.png` per tour stop and, with `--live-orbit <step>,<seconds>`, one `orbit-DDD.png` every 45 degrees of the camera turn.
The summary also lists every connected block's request state change after play starts (`block_changes`), with the collision part selecting the map; a state marked `(no groups)` draws none of its parts.
Every measurement is also logged as a `live check:` line, and the loading stages are logged as `startup:`, `map setup:`, `loading:` and `play ready:` lines.
Building with `--features profile-spans` times every Bevy system through its tracing span and logs `span profile` (mean ms per frame) and `span stall` (longest single run) lines every five seconds.

Play starts only when loading has finished (`crates/game/src/play_gate.rs`): Wolf's model is bound, every map GLB and texture has loaded and spawned, no pipeline is compiling, every image has been uploaded, every spawned mesh entity's mesh has been prepared by the render world, and the drawn meshes within 60 m have their textures, and then 60 consecutive frames stay within 1/30 s.
A start with no collision floor under Wolf (a `--spawn` over a hole) cannot become playable, so the game exits with an error naming the position before loading instead of holding the loading screen (#231).
Play starts on the next frame: a further 2.5 s settle made no difference to the first frames of play (about 80 ms worst frame either way, `longest_start_frame_ms` in the summary).
The check skips the frames that pay for its own 4 Hz ground samples when it collects play frame times.
Until then an opaque loading screen with a progress bar covers the view and the simulation does not step.

Reference numbers (Ryzen 7 5700X3D, RTX 5070 Ti, warm file cache, 2026-10-16):

| Measure | HEAD e7bebb6 | After |
| --- | ---: | ---: |
| Launch to playable | about 125 s (map loaded 111 s after it was added) | 43 to 47 s (map loaded about 17 to 24 s after it was added) |
| Frame time once playable at start 0 | 52 ms median (19 fps) | 16.5 to 16.6 ms median, 20 to 21 ms p95 (vsync 60 Hz) |
| Longest frame while loading (after the first) | 8.7 s | about 0.4 s |
| Map scene instances, entities | 31,968, 244,000 | 24,624, 198,000 |
| Exit after closing the window | about 27 s | about 1.5 s |

Bevy 0.19's material specialization skips a mesh entity whose mesh is not yet prepared and never retries it, so with the per-frame upload budget an entity extracted before its mesh stays undrawn.
`LateMeshes` in `crates/game/src/capture_ready.rs` tracks meshes that were not prepared when their entity appeared and marks those entities changed once the mesh is prepared, for the whole session.
An earlier one-off re-extraction of every mesh entity before play caused most of the 2 to 7 s loading frames; the incremental tracking removed them.
The first window frame, which runs the startup systems, takes about 7 s.
Launch to first window frame is about 26 s of synchronous setup (character and animation libraries 9.7 s, simulation 3.3 s, the 13.3 million triangle connected collision world 3.3 s, layouts and story 6 s).
Fixed: the void after a fall or grapple in the ravine north of start 0 reproduced with `--live-tour drop:-137.6,-44.3,-124.5`.
Wolf landed on the helper collision `h900000` (hit filter 15) at y = -74.2, whose display and draw demand masks are zero, so the confirmed visibility rule drew no scenery and the screen showed only sky and fog; the map viewer at the same camera shows the same void with draw groups on and the ravine with `--groups off`.
Since 2026-10-16 Wolf falls through `h900000`: filter 15 is an event wall the player never stands on, and its contact starts the fall return (`docs/HIT-FILTERS.md`).
The special hit filters are now traced in the executable; only filters 8, 22 and 27 (and unlisted values) hold the player.
Fixed: after grappling to that branch the whole Outskirts block disappeared at most camera yaws, because the orbit camera's position over the ravine also chose the map's collision part.
In play the map now follows the part Wolf last stood on, as the executable follows the character's hit collision (`docs/MAP-COMPLETENESS.md`, section 5).

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

## D1 mountain and finite-fog review, 2026-10-15

The horizon was submitted but obscured by an incorrect missing-field default for analytical fog continuation.
The native constructor and editor registration both default that switch to false; the base Ashina files omit it.
The corrected renderer retains the authored finite fog volume, densities, colours and explicit switches.
The Mountain material additionally retains its native snow/rock albedos, raw normal and detail normal, three mask projections and baked vertex occlusion.
Targeted extraction refreshes Outskirts m902000/m902010/m925000 and Castle m911061 without rewriting either layout or ancillary sidecars.
Both layout hashes remain identical, and all selected samplers resolve with zero missing textures.
Exposure, tone mapping, sky geometry/order, camera planes, draw groups and LOD rules are unchanged.

The no-fog control already exposes the mountains with the original camera and sky, ruling out clipping or sky occlusion for the inspected views.
The fog-only control restores their contrast but retains the former green single-albedo appearance.
The complete correction exposes snowy terrain behind the village and keep.
The matched spot-009, spot-011, spot-022 and spot-091 cameras, selected collisions and complete sorted visibility records agree before/after.
Spot-022 remains chiefly a foreground gate/fire view with limited far coverage; it is not an independent proof that every retail mountain pixel is matched.
The sky, blue/bright snow and shared material-lighting differences remain separate fidelity limits, without a replacement exposure or tint.

Both same-session baseline gates pass with the original fixtures.
Every one of the twelve reviewed views retains its exact void, depth-gap and untextured counts, selected collision and sorted visible-part identities after the correction.
Totals in that order remain `(0,493,0)` for Outskirts and `(1933,4885,0)` for Castle.
The following are the first complete after measurements used for fixture review, compared with the same-session before executable and original cache.
Grey and pop-in are colour classifications, so restoring finite-volume transmission can change their thresholds on distant geometry and the normal/all-visible comparison without changing a submitted part.

| Area / view | Grey before -> after | Pop-in before -> after | Fixture decision and per-view review |
| --- | ---: | ---: | --- |
| Outskirts 0 | 276 -> 222 | 8651 -> 8560 | Retain both fixtures; both counts decrease with finite-volume colour. |
| Outskirts 1 | 188 -> 189 | 294 -> 295 | Retain both fixtures; one-pixel colour crossings remain within tolerance. |
| Outskirts 2 | 380 -> 379 | 2489 -> 2489 | Retain both fixtures; grey decreases by one and pop-in is identical. |
| Outskirts 3 | 717 -> 752 | 5413 -> 5412 | Grey fixture 725 -> 752; inspected normal/all-visible pairs retain silhouettes and coverage while distant fog/background contrast changes colour classification. |
| Outskirts 4 | 641 -> 641 | 6756 -> 6757 | Retain both fixtures; grey is identical and pop-in changes by one. |
| Outskirts 5 | 149 -> 149 | 5241 -> 5240 | Retain both fixtures; grey is identical and pop-in decreases by one. |
| Castle 0 | 203 -> 199 | 5088 -> 5099 | Retain both fixtures; grey decreases and the colour-comparison change remains within tolerance. |
| Castle 1 | 288 -> 262 | 7488 -> 7518 | Retain both fixtures; grey decreases and pop-in remains within the original 1% allowance. |
| Castle 2 | 588 -> 625 | 5879 -> 5840 | Grey fixture 588 -> 625; the formerly washed-out far landscape becomes readable behind the roofs, with identical geometry diagnostics. |
| Castle 3 | 51 -> 246 | 3782 -> 4278 | Grey fixture 49 -> 246 and pop-in 3782 -> 4278; inspected under-keep pairs expose distant landscape/sky contrast in the normal/all-visible comparison, while the existing void/depth-gap mask stays identical. |
| Castle 4 | 480 -> 480 | 2418 -> 2362 | Retain both fixtures; grey is identical and pop-in decreases. |
| Castle 5 | 91 -> 103 | 6944 -> 6988 | Retain both fixtures; both colour-category changes remain within the original tolerance. |

Only those four grey/pop-in fixture fields change.
No camera, geometry fixture, tolerance or 120-second budget changes.
Both final six-view gates pass in 68.552 seconds for Outskirts and 85.507 seconds for Castle.
These runs use the frozen mountain executable and identical post settings; the separately owned tone-curve correction that appeared later in the shared tree is excluded from this before/after attribution.
Both saved executable images contain the complete unchanged original tone shader; its hash and byte-presence checks are retained in `protected-tone-proof.json`.
The data gate passes after the selected exports, and release all-target workspace Clippy with warnings denied and workspace formatting pass using `target/alt-mountains` and six Cargo jobs.
The measured multi-minute full-roster boss-story audit is moved to the prescribed ignored slow tier, with its focused Genichiro/Owl story check retained in the normal suite.
The initial new fog regression incorrectly assumed terminal fixed-density transmission always exceeded 50%; that unsupported assertion was removed, retaining the missing-switch and explicit-switch regressions.
The final `cargo test --release --workspace` run passes 814 tests with 25 ignored and zero failures, including the installed-data gate, mountain export regression, missing-switch regression and the shared tree's separately corrected native tone regression.
Final workspace release/all-target Clippy with `-D warnings` and `cargo fmt --all --check` both pass.

The source changes for this job are `crates/game/src/fog.rs`, `crates/game/src/map_material.wgsl`, `crates/formats/src/map_layers.rs`, `crates/extract/src/main.rs`, `crates/extract/src/maps/mod.rs`, `crates/extract/src/maps/pieces.rs` and `crates/extract/src/maps/pieces_lod_tests.rs`.
The slow-tier split is confined to tests in `crates/game/src/lighting_selection.rs`.
The four reviewed fixture fields are in `crates/game/tests/data/m11_00_00_00.render.json` and `m11_01_00_00.render.json`.
Documentation records are this page, `MAP-COMPLETENESS.md`, `VISUAL-PARITY.md`, `FIDELITY.md`, the mountain/fog additions in `SHADERS.md` and `SHADER-INPUTS.md`, and `STATUS.md`.
Concurrent tone/exposure source, tests and documentation belong to their separate owner and are not mountain changes.

Runtime reports, diagnostic controls, native-input research and executable hashes are retained under `target/alt-mountains/progress/`.
The external `2026-10-15/mountains/` progress directory contains four `spot-*-real-before-after.png` panels, their individual images and camera/visibility manifests.

## Exposure tone fixture review (2026-10-15)

The sensitometric mode-7 curve now uses the native base-two exponentials rather than the earlier natural-exponential transcription.
Only grey and pop-in fixture fields are re-baselined; camera poses, geometry fields, 128x72 resolution, 16-pixel-or-1% tolerances and the 120-second limit are retained.
This review starts from the concurrent mountain job's completed fixtures rather than reverting its colour updates.
The before/after controls retain identical void, depth-gap and untextured counts at all twelve views: totals are 0/493/0 for Outskirts and 1933/4885/0 for Castle.
The complete six retail-camera visibility manifests also agree, and all 17,214 fingerprinted map/draw-param assets remain unchanged across the repeated captures.
No geometry metric is relaxed or made worse.

Grey requires neutral RGB spread at most 8 and all channels in 70..210.
The corrected curve moves pixels into or out of that range, including previously brighter neutral surfaces, without creating or removing geometry.
Pop-in measures an existing normal/all-visible difference above 12 display values; changing the nonlinear output curve changes how many existing differences cross that threshold.
The per-view explanation reports the net criterion change, without assigning it to an unverified individual light or material.
Original fixture values below include ordinary prior capture variance; the external before reports retain the fresh control counts independently.

| Map / view | Grey fixture -> new | Pop-in fixture -> new | Per-view interpretation |
| --- | ---: | ---: | --- |
| Outskirts 0 | 274 -> 233 | 8711 -> 8360 | 41 fewer neutral-range pixels; 351 fewer above-threshold differences. |
| Outskirts 1 | 183 -> 48 | 294 -> 219 | 135 fewer neutral-range pixels; 75 fewer above-threshold differences. |
| Outskirts 2 | 383 -> 333 | 2484 -> 2581 | 50 fewer neutral-range pixels; 97 more differences exceed 12 after the curve's slope changes. |
| Outskirts 3 | 752 -> 1418 | 5503 -> 4894 | 666 more pixels meet the neutral-range criterion after darkening; 609 fewer above-threshold differences. |
| Outskirts 4 | 648 -> 505 | 6831 -> 5321 | 143 fewer neutral-range pixels; 1510 fewer above-threshold differences. |
| Outskirts 5 | 147 -> 259 | 5245 -> 4804 | 112 more pixels meet the neutral-range criterion after darkening; 441 fewer above-threshold differences. |
| Castle 0 | 196 -> 152 | 5083 -> 4932 | 44 fewer neutral-range pixels; 151 fewer above-threshold differences. |
| Castle 1 | 288 -> 35 | 7488 -> 7359 | 253 fewer neutral-range pixels; 129 fewer above-threshold differences. |
| Castle 2 | 625 -> 528 | 5879 -> 5746 | 97 fewer neutral-range pixels; 133 fewer above-threshold differences. |
| Castle 3 | 246 -> 159 | 4278 -> 4172 | 87 fewer neutral-range pixels; 106 fewer above-threshold differences. |
| Castle 4 | 482 -> 228 | 2400 -> 2167 | 254 fewer neutral-range pixels; 233 fewer above-threshold differences. |
| Castle 5 | 92 -> 67 | 6958 -> 6779 | 25 fewer neutral-range pixels; 179 fewer above-threshold differences. |

The fresh pre-baseline after runs take 78.825 seconds and 93.754 seconds, retaining every geometry count from the before controls.
The Outskirts rejection contains only the expected colour-fixture differences; Castle's existing tolerances already admit the new counts.
An earlier run during compilation also exceeded the unchanged time limit and is not used as the final gate result.
The reviewed numbers, full before/after reports and verification JSON are under `target/alt-exposure/gates-before/`, `gates-after/` and the external `2026-10-15/exposure/` collection.
See [EXPOSURE.md](EXPOSURE.md) for the native rule, RustSekiro inspection and six-view luminance/saturation/hue results, including the unresolved snow-colour gap.

Both independent final repeats in `target/alt-exposure/gates-final/` pass: Outskirts 49.948 seconds and Castle 58.686 seconds.
Their geometry triples exactly equal the fresh before controls at every view, and the saved pre-change fixture comparison confirms that only the two colour fields changed.
Final grey/pop-in totals are 2796/26172 and 1169/31146 respectively, with ordinary capture variation inside the unchanged per-view tolerances.
Release workspace tests, the installed-data gate, native post composition and GPU numeric regression, workspace all-target release Clippy with warnings denied, and formatting pass.

## Painted backdrop and fog-card follow-up, 2026-10-15

This follows the terrain-mountain and exposure work above, preserving both sets of changes.
The missing painted range belongs to each m89 sky group's separate FarMountain companion, not the dome's cloud textures or the `m6*` naming family.
The exporter and runtime now retain that companion, its native colour/haze shader, alpha threshold and depth-write control.
The executable also establishes omitted fog sampling defaults of near offset 1, 32 slices and sample offset 0, plus SAT RGB conversion with exponent 2.2 and independent alpha.
Fog-card highlights now use the independent native sun specular record rather than diffuse RGB/intensity.
The full data trace, blue-arc isolation and remaining retail haze discrepancy are in [BACKDROP.md](BACKDROP.md).
No density multiplier, panorama height adjustment, replacement tint or exposure/post change is part of this task.

The frozen before executable is the exposure job's final control, retaining the preceding mountain fixes and native base-two tone curve.
Both captures use unchanged canonical first-visit gate settings and the same extracted map geometry.
Every selected collision and sorted visible-part manifest agrees across all twelve views.
All twelve void/depth-gap/untextured triples agree exactly, and every diagnostic depth and collision image is pixel-identical.
Geometry totals remain Outskirts `0/493/0` and Castle `1933/4885/0` in void/depth-gap/untextured order.

The following are fresh before/final measurements, not substitutions for the preceding job's controls.
Small count changes can include ordinary capture variance; the larger vista changes are reviewed as the combined horizon/fog/material correction, without claiming a separate numeric attribution to each input.
Only Outskirts view 3 grey and Castle views 2 and 3 grey/pop-in fixture fields are updated.
All geometry fields, cameras, tolerances and the 120-second gate limit remain unchanged.

| Area/view | Grey before -> final | Pop-in before -> final | Review and fixture decision |
| --- | ---: | ---: | --- |
| Outskirts 0 | 238 -> 233 | 8360 -> 8360 | Temple/idol clearing retains coverage; small neutral classification variance, no fixture edit. |
| Outskirts 1 | 48 -> 48 | 220 -> 220 | Dark hut retains the same surfaces and colour counts, no fixture edit. |
| Outskirts 2 | 338 -> 332 | 2583 -> 2579 | Snow path and rock edge remain aligned; small colour changes stay within tolerance, no fixture edit. |
| Outskirts 3 | 1416 -> 1434 | 4896 -> 4887 | Open cliff view retains silhouettes while fog/card/background colour shifts neutral classification; grey fixture 1418 -> 1434. |
| Outskirts 4 | 505 -> 505 | 5321 -> 5324 | Door/wall view retains its illumination boundaries; three comparison pixels vary, no fixture edit. |
| Outskirts 5 | 260 -> 258 | 4804 -> 4798 | Sheltered passage retains detail and depth; small colour changes stay within tolerance, no fixture edit. |
| Castle 0 | 152 -> 151 | 4932 -> 4927 | Doorway and outside snow stay aligned; small threshold changes, no fixture edit. |
| Castle 1 | 35 -> 36 | 7359 -> 7359 | Rock/roof passage retains its normal/all-visible difference; no fixture edit. |
| Castle 2 | 528 -> 558 | 5746 -> 5628 | Panoramic outlook has the reviewed atmospheric/background colour change with identical diagnostic geometry; grey fixture 528 -> 558 and pop-in 5746 -> 5628. |
| Castle 3 | 159 -> 155 | 4172 -> 4100 | Under-keep outlook retains the roof framing and void mask; altered far fog/background colour lowers the comparison count; fixtures 159 -> 155 and 4172 -> 4100. |
| Castle 4 | 228 -> 230 | 2167 -> 2168 | Snow courtyard/tree view retains coverage; small colour variance, no fixture edit. |
| Castle 5 | 67 -> 68 | 6778 -> 6779 | Gate passage retains framing and light boundary; no fixture edit. |

The first frozen final runs measure 89.5 seconds for Outskirts and 99.6 seconds for Castle inside the gate.
Outskirts passes before any fixture edit; Castle rejects only view-2 grey, 528 -> 558.
The manually reviewed colour baselines above address that rejection without weakening a geometry check.
Review contact sheets and all 24 diagnostic image equality results are retained under `target/alt-backdrop/`.
The final image set uses spots 011/037, 022/074, 091/290, 009/034, 045/160 and 089/282.
Two unsuitable extra matches, 048/167 and 093/296, are archived as rejected diagnostics rather than counted as backdrop evidence.

Both independent final gate repeats pass after the reviewed colour fixture edits: Outskirts in 45.910 seconds and Castle in 59.719 seconds.
Their geometry triples remain identical to every preceding control view.
The independent repeat retains all diagnostic pixels except one Castle view-4 depth pixel changing by one channel code; its measured geometry categories remain identical.
Release workspace tests pass 815 tests with 25 existing ignores, including the installed-data gate.
The final touched-crate rerun passes 163 game/extractor tests with three existing ignores, including the sky-companion regression and the installed-data gate again.
Workspace all-target release Clippy with warnings denied, formatting and `git diff --check` pass.
The frozen before and final executables both embed the exact current tone and exposure shader source, confirming that their correction is retained and unchanged by this task.
All Cargo work uses `CARGO_TARGET_DIR=target/alt-backdrop`.
No staging, commit, worktree, remote publication or retail-game session was performed.

## Post-chain colour fixture review, 2026-10-15

The [post-chain audit](COLOUR.md) restores the authored spatial bloom pyramid and wires native pre-gamma colour correction.

All twelve fixed views were reviewed in normal and all-visible phases against their frozen before controls.

Every void/depth-gap/untextured triple is identical between those controls; only colour threshold counts are re-baselined.

Camera fields, resolution, geometry baselines, tolerances and the 120-second limit remain unchanged.

The contact sheets and raw per-view records are retained in the external colour collection.

| Map / view | Grey fixture -> reviewed | Pop-in fixture -> reviewed | Review note |
| --- | ---: | ---: | --- |
| m11_00_00_00 / 0 | 233 -> 116 | 8360 -> 9154 | Snow courtyard: wider glow around the idol and snow, with the same bamboo, steps and stone silhouettes. |
| m11_00_00_00 / 1 | 48 -> 50 | 219 -> 215 | Dark timber passage: small threshold changes, with the same doorway and snow coverage. |
| m11_00_00_00 / 2 | 333 -> 152 | 2581 -> 2377 | Overhead snow path: broader snow highlights and roof-edge glow, with the same path and roof boundaries. |
| m11_00_00_00 / 3 | 1434 -> 1861 | 4894 -> 5521 | Snowy ledge: brighter foreground bloom and softer background contrast, with the same structure and cliff silhouette. |
| m11_00_00_00 / 4 | 505 -> 589 | 5321 -> 6398 | Timber gate close-up: lifted warm timber and beam shading, with the same gate geometry in both phases. |
| m11_00_00_00 / 5 | 259 -> 230 | 4804 -> 4798 | Roof and branches: lifted dark surfaces and distant glow, with the same roof, branch and wall edges. |
| m11_01_00_00 / 0 | 152 -> 169 | 4932 -> 4976 | Timber corridor: warmer and brighter wood around the doorway, with the same corridor silhouette. |
| m11_01_00_00 / 1 | 35 -> 249 | 7359 -> 8641 | Cart and stone wall: wider highlight spill across snow, wheel and masonry, with the same object coverage. |
| m11_01_00_00 / 2 | 558 -> 656 | 5628 -> 5627 | Rooftop panorama: broader snow highlights and reduced local contrast, with the same skyline and roofs. |
| m11_01_00_00 / 3 | 155 -> 297 | 4100 -> 4951 | Angled roof: brighter snow and shadow fill, with the same roof edges and open depth regions. |
| m11_01_00_00 / 4 | 228 -> 668 | 2167 -> 2638 | Tree courtyard: warmer snow and lifted blue shadows, with the same tree, paving and surrounding walls. |
| m11_01_00_00 / 5 | 67 -> 79 | 6779 -> 7412 | Stone walkway: wider light spill along masonry and snow, with the same walkway and skyline. |

The final shared-tree Castle view 4 was reviewed again against the isolated post snapshot in both phases.
Its pop-in colour count is 2638 rather than the isolated 2607, with identical tree, snow, paving and wall coverage and the same geometry triple (0,5,0).
Only that additional colour cell is refreshed; the 16-pixel/1-percent tolerance and every geometry field remain unchanged.

## Reference story fixture review, 2026-10-15

The canonical gate now uses `genichiro-defeated`, the best-supported reference candidate documented in [LIGHTING.md](LIGHTING.md#reference-progression-audit-2026-10-15).
This changes its former morning first-visit default to event-selected afternoon, rather than changing night into a hand-picked hour.
The reference matcher previously forced night; those are separate former defaults.
Both fixtures declare the named state and the gate rejects a fixture whose story disagrees with its canonical policy.
The two normal controls use the same frozen executable, fixed views, native materials and exposure policy, differing only in their story preset.
Neither uses an hour override.

Every measured void/depth-gap/untextured triple is unchanged between the morning and progression controls.
Outskirts totals remain 0/493/0 and Castle totals remain 1933/4885/0.
Castle view 0 measures 148 depth-gap pixels in both controls, versus its retained fixture baseline of 149; this one-pixel improvement is not a geometry re-baseline.
Every camera field, geometry fixture cell, resolution, 16-pixel/1-percent tolerance and 120-second budget is retained.
Selected collisions agree at every view, every old visible-part set is a subset of the new set, and visible counts never decrease.
Known progression enables idol props `o005000_0005` in Outskirts and `o005000_0006` in Castle, plus three group-1106700 objects at Outskirts views 0 and 1.
These additions are story content, not replacements for missing geometry.
Collision diagnostic images are pixel-identical in all twelve views.
Depth images differ at 12 pixels in Outskirts view 0, 39 in Outskirts view 4 and one pixel in Castle view 4, without worsening any geometry category.
The source IDs and full added/removed sets are retained in the external story collection's `gate-comparison.json`.

Normal and all-visible images were reviewed separately at every view before accepting the colour cells below.
Grey and pop-in are image-colour threshold counts, so their change under a different LightSet, probe environment and sky does not establish lost geometry.
The preceding colour audit's per-view notes remain historical evidence above.

| Map / view | Grey fixture -> afternoon | Pop-in fixture -> afternoon | Review note |
| --- | ---: | ---: | --- |
| Outskirts / 0 | 116 -> 10 | 9154 -> 2728 | Temple courtyard changes from warm direct light to cool overcast light; bamboo, steps, stones and walls retain their silhouettes, with additional enabled story props. |
| Outskirts / 1 | 50 -> 342 | 215 -> 239 | Timber passage receives flatter grey-blue fill; the doorway and snow-floor boundary remain aligned, with three additional group-1106700 props. |
| Outskirts / 2 | 152 -> 6 | 2377 -> 2562 | Overhead path changes from peach highlights to blue-white snow; path, roof and rock boundaries remain aligned. |
| Outskirts / 3 | 1861 -> 3 | 5521 -> 6551 | Snow ledge becomes cool and the backdrop haze changes; the ledge, structure and cliff retain their depth coverage. |
| Outskirts / 4 | 589 -> 2921 | 6398 -> 7867 | Timber gate loses the warm directional wash and gains neutral wood fill; leaves, posts and floor retain coverage, with the progression idol now enabled. |
| Outskirts / 5 | 230 -> 0 | 4798 -> 5392 | Roof and branches acquire cool highlights; their silhouettes and wall coverage remain aligned. |
| Castle / 0 | 169 -> 66 | 4976 -> 5077 | Corridor wood and outside snow change from warm to cool; doorway, pillars and the exterior depth boundary remain aligned, with two additional enabled idol props across the loaded neighbourhood. |
| Castle / 1 | 249 -> 43 | 8641 -> 5770 | Cart and stone wall lose orange illumination; wheel, wall and snow coverage remain aligned with identical visible-part sets. |
| Castle / 2 | 656 -> 1 | 5627 -> 5487 | Rooftop panorama changes to the pale overcast sky and cool snow; skyline and roofs retain coverage, with the Castle progression idol enabled. |
| Castle / 3 | 297 -> 0 | 4951 -> 6149 | Angled roof receives cool daylight and a different distant sky; roof edges and existing open-depth regions remain aligned. |
| Castle / 4 | 668 -> 7 | 2638 -> 0 | Tree courtyard becomes blue-white under afternoon light; tree, paving and walls retain the same visible-part set, with the same geometry counts. |
| Castle / 5 | 79 -> 5 | 7412 -> 2099 | Stone walkway loses the warm wash and gains the pale daylight backdrop; walkway, masonry and skyline retain coverage. |

Review images are `outskirts-gate-review.png` and `castle-gate-review.png` under the external progress collection's `2026-10-15/story/` directory.
These fixtures guard the selected story's coverage and repeatability; their acceptance does not certify retail snow, brightness or colour parity.

## Fire brightness colour fixture review, 2026-10-15

The soft-particle depth fade now works in the single-sample capture path and the effect colour no longer carries an invented sqrt 3 (see MAP-FX.md, fire brightness audit).
Geometry is unchanged: Outskirts 0/493/0 and Castle 1933/4885/0 for void/depth-gap/untextured, and every per-view geometry field equals the same-executable `SEKIRO_NO_MAP_FX` control.

| Map / view | Grey fixture -> reviewed | Review note |
| --- | ---: | --- |
| m11_00_00_00 / 4 | 589 -> 2916 | Timber gate close-up: the warm orange wash from the 15 m fire glow disc is gone, so the gate and floor read neutral grey-blue like the effects-off control, with the same gate geometry. |

Other regressions seen in the same run (Outskirts views 1-5 pop-in and view 1 grey, Castle views 0 and 3 pop-in) are identical with `SEKIRO_NO_MAP_FX=1`, so they come from the concurrent colour-chain work and were not edited here.


### Concurrent point-light colour refresh and final gates

A parallel renderer task changed point/spot response and FX-light RGB packing after the isolated story comparison.
The state job preserves those implementation edits and their earlier colour notes.
The refreshed 51 daytime pairs and 11 matched night controls use comparison executable `f8e67514fb15979464ce3de41c7a116ff97d5de39815706c633377fbbf9bd18c`.
Subsequent FX/fog edits remain outside that measurement snapshot.
The final gate executable is `875ffe4329a1a6edf070a4cbb961cb593df14c8eb24497d7bd181d9740a4072f`, including the FX-light colour-packing change present when it was built.
All twelve refreshed gate views retain exactly the progression control's geometry triples, selected collisions and visible-part sets.
The first current-build gates reject only four colour cells: Outskirts views 0 and 1 grey, Outskirts view 1 pop-in, and Castle view 0 grey.
All normal/all-visible pairs were reviewed again, and the current colour cells below are retained to guard the reviewed response; geometry cells, camera fields, tolerances and budget remain unchanged.

| Map / view | Grey story baseline -> latest | Pop-in story baseline -> latest | Review note |
| --- | ---: | ---: | --- |
| m11_00_00_00 / 0 | 10 -> 76 | 2728 -> 2642 | Temple courtyard local glow changes; bamboo, stones and steps retain coverage. |
| m11_00_00_00 / 1 | 342 -> 2274 | 239 -> 292 | Timber passage receives a broader warm local-light contribution; doorway and snow-floor edges remain aligned. |
| m11_00_00_00 / 2 | 6 -> 8 | 2562 -> 2557 | Only small colour-threshold changes; path and roof edges remain aligned. |
| m11_00_00_00 / 3 | 3 -> 6 | 6551 -> 6553 | Only small colour-threshold changes; ledge and cliff coverage remain aligned. |
| m11_00_00_00 / 4 | 2921 -> 2 | 7867 -> 6992 | Timber gate receives a strong warm local-light wash; gate leaves, posts, floor and enabled idol retain coverage. |
| m11_00_00_00 / 5 | 0 -> 0 | 5392 -> 5392 | No colour-cell change; branch, roof and wall silhouettes retain coverage. |
| m11_01_00_00 / 0 | 66 -> 469 | 5077 -> 5084 | Corridor gains a warm edge/pillar contribution; doorway, exterior snow and pillars retain coverage. |
| m11_01_00_00 / 1 | 43 -> 43 | 5770 -> 5758 | Small colour-threshold change; cart, wall and snow retain coverage. |
| m11_01_00_00 / 2 | 1 -> 0 | 5487 -> 5485 | Small colour-threshold change; skyline and roof coverage remain aligned. |
| m11_01_00_00 / 3 | 0 -> 0 | 6149 -> 6139 | Small colour-threshold change; angled roof and existing open-depth regions remain aligned. |
| m11_01_00_00 / 4 | 7 -> 7 | 0 -> 0 | No colour-cell change; tree, paving and walls retain coverage. |
| m11_01_00_00 / 5 | 5 -> 7 | 2099 -> 2107 | Small colour-threshold changes; stone walkway and masonry retain coverage. |

The independent final repeats pass both complete canonical gates: Outskirts in 57.474 seconds and Castle in 68.480 seconds.
Their totals remain void/depth-gap/untextured 0/493/0 and 1933/4885/0.
The review images are `current-outskirts-gate-review.png` and `current-castle-gate-review.png` in the external story collection, with `concurrent-colour-review.json` and `final-gates.json`.
The frozen pre-point-response controls also pass independently in 61.749 and 68.449 seconds, with the same geometry categories.
The workspace release suite passes 817 tests with 25 existing ignores.
The refreshed material build passes all 137 non-ignored game tests with three existing ignores, and the later FX build passes 11 focused FX/shader tests.
Workspace all-target release Clippy with warnings denied and workspace formatting pass under `CARGO_TARGET_DIR=target/alt-story`.
The six Python checks pass, including rejection of a historical-state report; the same guard prevents the matcher from silently resuming night poses as daytime captures.

## Brightness bridge colour fixture review, 2026-10-15

Two reviews, both colour fields only; geometry fields (position, yaw, pitch, void, depth gap, untextured), resolution, story and tolerances are unchanged.
No per-pixel attribution was made: grey counts cross a fixed neutral-RGB criterion and pop-in counts cross a fixed 12-display-value threshold, so a global radiance change moves them without any asset change.

### Drift before the change (HEAD e89596c)

Both gates were run on a fresh build of HEAD (`target/alt-bright`).
Castle passed.
Outskirts failed three colour cells that moved after the fixtures were baselined.
The recent effect commits `1a1036f`, `38a6039` and `1213844` touch lights, effect textures and effect colour, but their individual contributions were not isolated.
No geometry cell moved.

| Outskirts view | Cell | Fixture | HEAD measured |
| --- | --- | ---: | ---: |
| 0 | pop-in | 2642 | 2761 |
| 4 | grey | 2 | 533 |
| 4 | pop-in | 6992 | 7649 |

### After the `SCENE_TO_BEVY` correction

The bridge changes from 4500 to 3135.61 (0.697), so all light, hemisphere, probe, sky and fog radiance falls by 30 percent.
Every colour cell was re-measured; geometry totals stay at depth gap 493 / 4885 and void 0 / 1933, identical to the HEAD controls.

| Map / view | Grey fixture -> new | Pop-in fixture -> new | Reason |
| --- | ---: | ---: | --- |
| Outskirts 0 | 76 -> 69 | 2642 -> 2777 | Pop-in includes the HEAD drift (2761), a further 16 from the bridge |
| Outskirts 1 | 2274 -> 467 | 292 -> 183 | Grey falls by 1807 and pop-in by 109 as the 0.697 radiance change moves pixels across both criteria |
| Outskirts 2 | 8 -> 8 | 2557 -> 2669 | Pop-in rises by 112 from the radiance change; grey unchanged |
| Outskirts 3 | 6 -> 3 | 6553 -> 6498 | Both differences are within the old tolerance |
| Outskirts 4 | 2 -> 351 | 6992 -> 6759 | Grey includes the HEAD drift (533 at 4500), 351 after the bridge; pop-in falls by 233 |
| Outskirts 5 | 0 -> 0 | 5392 -> 5231 | Pop-in falls by 161 |
| Castle 0 | 469 -> 187 | 5084 -> 4982 | Grey falls by 282 and pop-in by 102 |
| Castle 1 | 43 -> 45 | 5758 -> 4930 | Pop-in falls by 828 |
| Castle 2 | 0 -> 0 | 5485 -> 5772 | Pop-in rises by 287 |
| Castle 3 | 0 -> 0 | 6139 -> 5411 | Pop-in falls by 728 |
| Castle 4 | 7 -> 9 | 0 -> 0 | Grey differs by 2 |
| Castle 5 | 7 -> 22 | 2107 -> 2106 | Grey rises by 15 |

Final totals (void, depth gap, untextured, grey, pop-in) are Outskirts 0 / 493 / 0 / 898 / 24118 in 43.3 seconds and Castle 1933 / 4885 / 0 / 266 / 23199 in 41.9 seconds, both under the 120-second limit and both passing every per-view tolerance against the refreshed fixtures.
The cells were taken from the first post-change runs; the second runs used for the pass agree within 3 pixels in every total.
Raw reports are under `target/alt-bright/gate-head/` and `target/alt-bright/gate-after/`.

## Layer stack colour fixture review, 2026-10-15

The default render now resolves LightSet and post fields through the executable's layer stack (`default`, `mapbase`, block file, collision overlays), see [LIGHTING.md](LIGHTING.md#layer-stack-and-record-defaults-2026-10-15).
Colour fields (grey, pop-in) of both render fixtures are re-baselined; geometry fields (position, yaw, pitch, void, depth gap, untextured), resolution, story and tolerances are unchanged.
No per-pixel attribution was made: the change moves tone-map mode (`mapbase` supplies 8, the old fallback was 7), tone and colour-correction fields, row-100 hemisphere, fill direction and sharpness, and the overlay fields of the collision under each camera.

The control is the same executable with `SEKIRO_ABLATE=nolayers`, so it includes the concurrent snow-coverage work in the shared tree.
The control's differences from the previous fixture (Outskirts 1 grey, Outskirts 2 pop-in and Castle 1 pop-in, the cells that task reports as rejected) come from that work, not from this change.
Raw reports are under `target/alt-bright/gate-nolayers/` (control), `gate-layers/` (first layered run) and `gate-final/` (pass against the new fixtures).

| Map / view | Grey fixture -> control -> new | Pop-in fixture -> control -> new | Note |
| --- | ---: | ---: | --- |
| Outskirts 0 | 69 -> 65 -> 68 | 2777 -> 2768 -> 2491 | Pop-in falls by 277 with the layered tone curve |
| Outskirts 1 | 467 -> 498 -> 1623 | 183 -> 179 -> 245 | Grey rises by 1125: the large snow-lit plane crosses the neutral-RGB criterion under the mode-8 curve |
| Outskirts 2 | 8 -> 8 -> 14 | 2669 -> 2736 -> 2255 | Pop-in falls by 481 |
| Outskirts 3 | 3 -> 3 -> 49 | 6498 -> 6425 -> 6240 | Grey rises by 46 |
| Outskirts 4 | 351 -> 350 -> 754 | 6759 -> 6756 -> 7435 | Grey rises by 404 and pop-in by 679 |
| Outskirts 5 | 0 -> 0 -> 0 | 5231 -> 5222 -> 5377 | Pop-in rises by 155 |
| Castle 0 | 187 -> 194 -> 198 | 4982 -> 4982 -> 5040 | Within the old tolerance |
| Castle 1 | 45 -> 40 -> 56 | 4930 -> 4987 -> 4892 | Within the old tolerance |
| Castle 2 | 0 -> 0 -> 3 | 5772 -> 5741 -> 5426 | Pop-in falls by 315 |
| Castle 3 | 0 -> 0 -> 0 | 5411 -> 5466 -> 5787 | Pop-in rises by 321 |
| Castle 4 | 9 -> 10 -> 13 | 0 -> 9 -> 0 | Within the old tolerance |
| Castle 5 | 22 -> 21 -> 55 | 2106 -> 2088 -> 1878 | Grey rises by 33, pop-in falls by 210 |

Final totals (void, depth gap, untextured, grey, pop-in) are Outskirts 0 / 493 / 0 / 2508 / 24043 in 55.5 seconds and Castle 1933 / 4885 / 0 / 325 / 23017 in 56.8 seconds, both under the 120-second limit and both passing every per-view tolerance.
The geometry columns of the layered runs equal the control runs and the fixtures in every view.
The first layered Outskirts run gave the same totals as the final one (deterministic); the Castle pop-in total moved 23023 -> 23017.

## Node-bound object fixture review, 2026-10-16

Unweighted map-object vertices are now exported through the node their normal W names, see [FORMATS.md](FORMATS.md#flver2-version-0x2001a).
The cache was re-exported for the affected object models only (bamboo, trees, shrubs and similar), so the leaf clusters that sat at each model's root now stand in the crown.
Colour fields (grey, pop-in) that moved past the tolerance in either direction are re-baselined; geometry fields, cameras, resolution, story, tolerances and the 120-second budget are unchanged.
The control is the unchanged executable on the old cache (`target/alt-void-head/gate/`); the new runs are under `target/alt-trees/gate/` and `target/alt-trees/gate-final/`.
Every view selects the same collision and the same visible parts in the control and the new run, so the pixel differences come from the re-exported models.
Image differences sit on the bamboo canopy (Outskirts 0), dry shrubs (Castle 0, 1 and 5) and the all-visible comparison images (Outskirts 3 and 5, Castle 3).
Outskirts 4 shows a faint, even change across the lit wall, consistent with the moved models changing the shadow casters; it was not attributed further.

| Map / view | Grey fixture -> control -> new | Pop-in fixture -> control -> new | Note |
| --- | ---: | ---: | --- |
| Outskirts 0 | 68 -> 68 -> 93 | 2491 -> 2491 -> 2648 | Bamboo canopy now in the crown |
| Outskirts 3 | 49 -> 49 -> 49 | 6240 -> 6240 -> 3984 | All-visible image only; the normal image is unchanged |
| Outskirts 4 | 754 -> 758 -> 816 | 7435 -> 7433 -> 7489 | Grey fixture only; pop-in stays within tolerance |
| Outskirts 5 | 0 -> 0 -> 0 | 5377 -> 5377 -> 4501 | All-visible image only |
| Castle 0 | 198 -> 205 -> 185 | 5040 -> 5029 -> 5103 | Pop-in fixture only; shrubs move |
| Castle 1 | 56 -> 56 -> 42 | 4892 -> 4892 -> 4840 | Pop-in fixture only |
| Castle 3 | 0 -> 0 -> 0 | 5787 -> 5787 -> 5468 | All-visible image only |
| Castle 5 | 55 -> 55 -> 32 | 1878 -> 1882 -> 1898 | Shrub at the lower right |

Final totals (void, depth gap, untextured, grey, pop-in) are Outskirts 0 / 497 / 0 / 2595 / 21120 in 53.3 seconds and Castle 1933 / 4888 / 0 / 275 / 22736 in 47.7 seconds, both passing every per-view tolerance.
The first new runs agree with the final runs within 9 pixels in every cell.
Depth gap moves within tolerance at Outskirts 0 (75 -> 79) and Castle 1 (15 -> 17), and those geometry cells are retained.

## Vertex tint gate and native specular (#7)

Gating the Multiple vertex tint on mask 6 lifts rock, stone, wood and bark that the ungated tint had multiplied to black, and specular now takes the same 1/pi as diffuse with the native F0 0.03.
Lighter neutral rock crosses the fixed grey criterion and the brighter albedo moves pop-in counts, so only those colour cells are re-baselined.

| Map / view | Grey | Pop-in |
|---|---|---|
| Outskirts 0 | 93 -> 138 | |
| Outskirts 2 | 14 -> 251 | |
| Outskirts 4 | | 7435 -> 7909 |
| Outskirts 5 | | 4501 -> 4597 |
| Castle 0 | 198 -> 409 | |
| Castle 1 | 56 -> 342 | |
| Castle 2 | | 5426 -> 5508 |
| Castle 3 | | 5468 -> 6181 |
| Castle 4 | 13 -> 34 | |
| Castle 5 | 32 -> 87 | |

Void, depth-gap and untextured counts are unchanged within tolerance, and untextured stays 0 in every view.
The user confirmed in live play that the black glossy patches are gone.

## Native sun shadow range (#9)

The sun now uses the native `ShadowParam` cascades to 1000 m, and map shadow casters reach 300 m instead of 45 m, so distant pieces gain their shadows in both captures.
The normal-distance and all-visible images then differ in more shadowed pixels, so only the pop-in cells that moved past tolerance are re-baselined.

| Map / view | Pop-in |
|---|---|
| Outskirts 3 | 3984 -> 4062 |
| Outskirts 4 | 7909 -> 8188 |
| Outskirts 5 | 4597 -> 4704 |
| Castle 2 | 5508 -> 5817 |
| Castle 5 | 1898 -> 2639 |

Void, depth-gap, untextured and grey cells pass unchanged; capture time grows to 60.7 and 68.8 seconds, inside the 120-second budget.

## Per-part light sets (#30)

Map pieces and objects now use their own MSB light set row 0, which has no local light and a dimmer sun than the collision rows, so surfaces render darker and flatter-coloured.
The Outskirts gate passes unchanged; two Castle cells moved past tolerance and are re-baselined.

| Map / view | Cell |
|---|---|
| Castle 2 | pop-in 5817 -> 5938 |
| Castle 5 | grey 87 -> 265 |

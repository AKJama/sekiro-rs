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

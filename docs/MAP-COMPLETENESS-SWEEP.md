# Map completeness sweep

The sweep measures rendered views above walkable collision using the shared map loader, materials, sky, lighting and draw-group selector.
It opens no window and never starts the real game or a gameplay/brain session.
All derived reports and screenshots stay under `target/alt-hs/sweep/<map>/pieces/` or `objects/`.

## Run

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
Diagnostic passes exclude the sky and bypass fog, grading, native post processing and anti-aliasing.
Depth is Euclidean eye distance encoded logarithmically into two 8-bit channels, with zero reserved for no surface and the blue channel marking absent base textures or unavailable declared material textures.
Shader output compensates for the target's sRGB conversion so the depth bytes survive GPU readback.
Native normal/all renders use fixed exposure and identical shader time to prevent adaptation or animated clouds/wet surfaces from being counted as visibility changes.
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

## Baseline

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

Reproduce the coarse baseline with `--spacing 80 --views 4 --settle-frames 4`, adding `--objects` for the objects control.
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

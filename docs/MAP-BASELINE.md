# Map baseline, 10 October 2026

All ten MSB entries in the install's archive were enumerated and extracted.
All ten load in the viewer and in play mode.
Nine are gameplay maps; m89 is a library of sky domes with no collision terrain.
This is a baseline inspection, not a claim that materials, event state or lighting match the original game exactly.
No commits were made, and the install and default release executable were left untouched.

## Inventory and measurements

Area names come from the English place-name FMG and MSB collision map-name IDs, rather than guesses from map-number prefixes.
Counts below are retained piece instances, object instances, exported GLBs, DDS files, collision parts and decoded collision triangles.
Objects are extracted but remain off by default because their visibility is event-dependent.
Extraction times are observed wall times, including warm-cache reruns for the already extracted maps.
Load times are the app's asset-load report, excluding process startup and screenshot delay.
Memory is peak process working set, not GPU memory or system-wide consumption.

| Map | FMG area | Extract s | Pieces / objects / GLBs / DDS | Collision parts / triangles | Viewer load s | Peak viewer memory | Verdict |
| --- | --- | ---: | --- | --- | --- | --- | --- |
| m10_00_00_00 | Hirata Estate | 19.61 | 5388 / 1543 / 614 / 567 | 144 / 2162322 | 3.6-5.3 | 4099-4642 MiB | Usable with issues: flat water, sparse foliage and single material layers |
| m11_00_00_00 | Ashina Outskirts | 3.4 | 4483 / 1509 / 712 / 732 | 166 / 2326355 | 2.9-3.1 | 4215 MiB | Usable with issues: material layering and foliage |
| m11_01_00_00 | Ashina Castle | 2.5 | 3623 / 1674 / 767 / 824 | 221 / 1656384 | 2.7-4.2 | 4027 MiB | Usable with issues: flat interior materials |
| m11_02_00_00 | Ashina Reservoir | 3.4 | 1674 / 801 / 383 / 528 | 72 / 817510 | 1.6-1.8 | 2931 MiB | Usable with issues: material layering and foliage |
| m13_00_00_00 | Abandoned Dungeon | 2.70 | 1809 / 277 / 189 / 198 | 46 / 899299 | 0.8-1.3 | 883-1005 MiB | Usable with issues: flat pale water |
| m15_00_00_00 | Mibu Village / Ashina Depths | 3.87 | 5197 / 1237 / 389 / 544 | 131 / 1949620 | 1.7-3.0 | 2264-2827 MiB | Usable with issues: pale water and foliage |
| m17_00_00_00 | Sunken Valley / Sunken Valley Passage | 15.54 | 8141 / 1103 / 631 / 529 | 140 / 4748810 | 3.9 | Approximately 4.4-4.6 GB | Usable with issues: conspicuous snow/foliage material layers |
| m20_00_00_00 | Senpou Temple, Mt. Kongo | 5.77 | 5729 / 1622 / 662 / 562 | 144 / 2881605 | 5.2 | Approximately 6.8-7.1 GB | Usable with issues: foliage and material layers |
| m25_00_00_00 | Fountainhead Palace | 5.73 | 4670 / 771 / 754 / 564 | 240 / 2606457 | 5.4 | Approximately 6.0-6.3 GB | Usable with issues: large pale lake and narrow arrival start |
| m89_00_00_99 | Sky-dome library; no FMG area | 6.35 | 26 / 1 / 27 / 3 | 0 / 0 | 0.4 | 561-581 MB | Broken for walking; loadable for sky inspection, no authored floor |

Play-mode observed asset loads were 2.9-6.1 s for m10, 3.6 s for m11_00, 2.9 s for m11_01, 1.9 s for m11_02, 0.9-1.8 s for m13 and 1.8-3.3 s for m15.
Additional play asset loads were m17 3.4 s, m20 5.4 s and m25 4.8 s; peak working sets were 5422.5, 7492.0 and 6457.4 MiB respectively.
Play working sets were 5233-5356 MiB for m10, 4872 MiB for m11_00, 4779 MiB for m11_01, 3561 MiB for m11_02, about 1550 MiB for m13 and 3229-3859 MiB for m15.
m89 play reached its first capture in 9.25 s with a 1246 MB working set.
All runs were bounded below 90 seconds.
The larger maps require several GB of RAM; streaming is not implemented.

Missing texture references reported by extraction: m10 11, m11_00 6, m11_01 0, m11_02 5, m13 0, m15 14, m17 2, m20 1, m25 3 and m89 1.
The m89 reference is `systex_dummy_a`.
No large pink or black missing-texture surfaces were found in the reviewed gameplay views.
These counts do not establish that every optional material layer is correct.

## Starts and collision

`crates/sim/examples/map_baseline.rs` audits the exported collision through the same `CollisionWorld` and body movement used by play.
It searches for a floor below authored start Y + 1 m, settles the body and walks 1.5 m in four headings at each start.
All 69 gameplay starts have floors, and no sampled grounded walk tick was below its supporting collision floor.
Start counts are m10 9, m11_00 10, m11_01 11, m11_02 3, m13 3, m15 5, m17 10, m20 8 and m25 10.
The final rebuilt audit is `cache/screens/maps/collision_baseline_final.jsonl`.
All 276 walks across gameplay starts report zero below-floor ticks and stable standing after settling.
The initial audit is retained as a diagnostic, but its initial-height comparison flagged slopes and was corrected to compare consecutive settled ticks.

m11_01 start 2 is authored about 1.60 m above its supporting floor and play correctly snaps it down.
Other sampled starts are within approximately 0.53 m of their supporting floors.
Some walk headings leave real ledges: m11_00 start 4 drops about 16.8 m, m20 start 0 about 6.0 m and m25 start 0 about 2.9 m before landing.
Longer m15 and m17 scripts also descend to lower paths rather than fall through visible ground.
m25 start 0 is the narrow arrival branch; use `--start 1` for an easier walking baseline.
Walking away from that branch into the ravine continues falling because no ground supports that route.
Grappling, swimming and fall respawn remain outside this audit.
m89's sole origin start has no floor and falls to Y -107 m after 180 simulation steps.
No synthetic floor was added to make the sky library appear playable.

## Reviewed screenshots

All paths below are under `cache/screens/maps/`.
Every listed image was visually reviewed by the lead.
The viewer captures provide three different starts or camera spots per map; play captures check floor placement and short movement.

| Map | Viewer images | Play images |
| --- | --- | --- |
| m10_00_00_00 | `m10_00_00_00_viewer_0.png`, `_viewer_1.png`, `_viewer_2.png` | `_play_0.png`, `_play_1.png`, `_play_2.png` |
| m11_00_00_00 | `m11_00_00_00_viewer_0.png`, `_viewer_1.png`, `_viewer_2.png` | `_play_3_60.png`, `_play_3_240.png` |
| m11_01_00_00 | `m11_01_00_00_viewer_0.png`, `_viewer_1.png`, `_viewer_2.png` | `_play_2_60.png`, `_play_2_240.png` |
| m11_02_00_00 | `m11_02_00_00_viewer_0.png`, `_viewer_idx_1.png`, `_viewer_idx_2.png` | `_play_0_60.png`, `_play_0_240.png` |
| m13_00_00_00 | `m13_00_00_00_viewer_0.png`, `_viewer_1.png`, `_viewer_2.png` | `_play_0.png`, `_play_1.png`, `_play_2.png` |
| m15_00_00_00 | `m15_00_00_00_viewer_0.png`, `_viewer_1.png`, `_viewer_2.png`, `_viewer_3.png` | `_play_0.png` through `_play_4.png` |
| m17_00_00_00 | `m17_00_00_00_viewer_0.png`, `_viewer_1.png`, `_viewer_2.png` | `_play_30.png`, `_play_120.png`, `_play_240.png` |
| m20_00_00_00 | `m20_00_00_00_viewer_0.png`, `_viewer_1.png`, `_viewer_2.png` | `_play_30.png`, `_play_120.png`, `_play_240.png` |
| m25_00_00_00 | `m25_00_00_00_viewer_0.png`, `_viewer_1.png`, `_viewer_2.png` | `_play_30.png`, `_play_120.png`, `_play_240.png`; `_idle_30.png`, `_idle_120.png`, `_idle_240.png`; `_short_30.png`, `_short_60.png`, `_short_90.png` |
| m89_00_00_99 | `m89_00_00_99_fixed_0_alt.png`, `_fixed_1.png`, `_fixed_2.png`; diagnostic `_fixed_0.png`, `_debug_0.png` through `_debug_8.png` | `_play_origin_1.png`, `_play_origin_60.png`, `_play_origin_180.png` |

Additional reviewed camera-fix captures are `m11_01_00_00_camera_fixed.png` and `m15_00_00_00_camera_fixed.png`.
Additional reviewed m11 captures are `<id>_memory_viewer.png`, `<id>_memory_play_60.png` and `<id>_memory_play_240.png` for each of m11_00, m11_01 and m11_02.
Working sets in these six follow-up runs were sampled every 250 ms.

Suffix-only entries retain the full map ID prefix from the first entry in their row.
The old m11_02 `_viewer_4.png` and `_viewer_6.png` used out-of-range start indices and are not evidence for actual player starts.
No obvious simultaneous early/late building version or major geometry hole was observed in these sampled gameplay views.
Coverage is limited to the captured locations, not an exhaustive traversal of every map surface.

## Changes and verification

- `crates/extract/src/maps/mod.rs`: retain zero-group geometry in collisionless layouts, with a regression test; gameplay zero-group suppression remains intact.
- `crates/extract/src/maps/mod.rs`: recognize HKX-only object binders as physics-only rather than failed render exports, with a regression test.
  The actual `o115923.objbnd` contains one HKX and no FLVER; m11_01 regeneration now reports zero failed GLBs and 24 models with nothing visible.
- `crates/game/src/map.rs`: resolve cinematic GPARAM names using all map suffix fields, so m89 finds `s89_00_0099.gparam` instead of failing on nonexistent `m89_00_0000.gparam`.
  A regression test covers the naming convention.
- `crates/game/src/map.rs`: initial viewer camera collision clipping, reproduced by m11_01 viewer 0 and m15 viewer 1.
  The release rebuild, regression and both after-images passed; the starts now show their surrounding routes.
- `crates/sim/examples/map_baseline.rs`: reproducible player-start and short-walk audit.

The final alternate release game build passed.
`cargo test --release -p sekiro-formats -p sekiro-extract -j 6` passed, including both new extractor regressions.
Both game map regressions passed.
`cargo clippy --workspace --all-targets -j 6 -- -D warnings` and `cargo fmt --all -- --check` passed.
Every Cargo command used `CARGO_TARGET_DIR=target/alt`, with six jobs (formatting uses `CARGO_BUILD_JOBS=6`).
Touched Rust files were formatted directly to avoid reformatting another contributor's work.
The protected brain refactor temporarily blocked compilation, but the contributor completed it before final verification.
The known game `map_tangents` test was not changed or used as a blocker.

## Remaining visual issues, ranked

1. The sky library is visually incorrect as a combined scene: co-located dome variants render as flat fogged hemispheres, and it has no playable ground (`m89_00_00_99_fixed_0_alt.png`, `_play_origin_180.png`).
2. Large water surfaces are pale flat polygons, particularly Fountainhead and Mibu; water shaders are outside this baseline (`m25_00_00_00_viewer_0.png`, `_viewer_2.png`, `m15_00_00_00_viewer_2.png`, `m13_00_00_00_play_1.png`).
3. Snow, moss, foliage and interior materials lose their original multi-layer shading; Sunken Valley's white foliage polygons are especially conspicuous (`m17_00_00_00_viewer_0.png`, `m11_01_00_00_viewer_2.png`).
4. Sun, ambient light and fog use the map GPARAM, but local light sets, shader exposure, particles, animated foliage and event-controlled object state remain approximate.

Measured cache growth from the session's initial snapshot was 6,896,486,701 bytes (6.42 GiB), including extracted maps, screenshots and diagnostics.
The ending measured cache size was 43,670,034,031 bytes.
Concurrent work may also have written cache artifacts, so this is measured repository cache growth rather than perfect per-task attribution.
Per-map output sizes are recorded in `cache/screens/maps/final_inventory.json`.
The eight newly extracted map directories total 6,723,090,910 bytes (6.26 GiB), excluding the two pre-existing m11 directories and the screenshot/diagnostic directory.

# Lighting selection

This page describes selection from the installed Sekiro data, independently of the renderer's tone curve, auto exposure and probe blending.
The shared implementation is `crates/game/src/lighting_selection.rs`.
Each map's original flags and event references live in one module under `crates/game/src/lighting/maps/`.
No game asset or decoded event body is copied into those modules.

## Selection

The loader evaluates installed `common.emevd` event 100 through the shared event interpreter with the current map and story flags.
Event 820's existing `StoryTime` rules first raise flags 9800 and 9801 from defeated bosses 9303 through 9306.
Flag-range conditions distinguish all on, all off, any on and any off, and map-membership conditions are evaluated against the loaded area and block.
Event 100 selects the environment variant and the enabled m89 sky group, including the Hirata memory, Fountainhead and the Ashina fire branch.
The selected sky group's model comes from the installed m89 MSB rather than an invented map-to-sky table.
The most recent enabled map-part command wins over the common sky, with a bounded recency list so repeated events do not grow scene state indefinitely.
`SetLighting` supplies a story-hour override, and `SetAreaEnvmap` supplies an independent environment override.
The existing authored variant/hour mapping is 0/0, 1/6, 2/12, 3/18, 4/22 and 5/2.
The native fixed Hirata and Fountainhead GPARAM rows have only hour 0, so their single authored values apply regardless of the story clock.

Ordinary draw-param stems use the map area/block and base family 00.
`SetAreaGparamSubID` changes the final two decimal digits rather than selecting a file by nearest hour.
The resolver accepts a present family-00 file, a unique installed candidate, or an explicit per-map event-family mapping.
Missing or ambiguous candidates fail rather than silently choosing a nearby file.
The reservoir storm has the sole sub-ID-30 file `m11_02_0930`, and its previously supported clear companion `m11_02_0920` is retained for map-event sub-ID 20.
The common invasion sub-ID 20 uses `m11_02_0020`, independently of that event-family mapping.
The 09 clear/storm pairing is an inference from the authored catalog and the previous arena implementation; EMEVD exposes the sub-ID but does not establish the engine's family-selection rule.
The unused numbered families and default/dying/eclipse files are not evidence that their parameters should be mixed into a map's selected file.

All map and boss event overrides now pass through the same loaded GPARAM library.
The reservoir-specific file-loading code has been removed from `arena_c_scene.rs`.
Event restart restores the map's initial file, and file changes invalidate the existing sun, hemisphere, fog and post-effect parameter caches.
Transition seconds remain recorded in event state; interpolation belongs to rendering and currently snaps to the selected file.
Castle phase-one restart clears the retained Tomoe lighting and sub-ID override before the scene is synchronized.
The existing cached probe handles are cleared on environment changes so a restart cannot retain the old camera cubemap handle.

## Story states and first visits

The default is the earliest reachable lighting state, not a guessed average time of day.
The reservoir is the opening prologue, where the installed event conditions use 8306 and the retained past-night flag 9810 without time-advancing boss flags.
Hirata is a memory and event 100 selects variant 0 for the map independently of world progression.
Fountainhead is entered after world progression, but event 100 selects its fixed variant 4 independently of the world-clock flags.
The other maps use the morning branch with no time-advancing boss defeated as the earliest available state.
Optional map visit order is not uniquely determined by the event files, so this earliest-state policy is explicit rather than a claim that every player first visits those maps in that state.
Boss arenas instead seed their existing `ArenaSpec.beaten` and `BossEvents.flags`, and preserve `ArenaSpec.story_time` for the actual starting phase.
An explicit `--story NAME` selects the corresponding map profile, and an explicit `--hour H` remains a clock diagnostic override.
Unknown story names produce an error listing that map's available names.

The clock names `morning`, `afternoon`, `evening`, `night` and `dawn` correspond respectively to no time flags, 9800, 9800-9801, 9800-9802 and 9800-9803.
`invasion` also sets 8302, selecting event 100's Ashina fire branch and sub-ID 20.
`tomoe` references the castle handover event 11115820, including the area sub-ID and `SetLighting(3)` instructions.
`shura-fire` references event 11115910's sub-ID-40 instruction.
`illusion` uses 8400 and 9303, references event 12005210's sky and sub-ID instructions, and runs region event 12005826.
`illusion-cleared` additionally sets 62000705 and references event 12005855's clear sub-ID instruction.
The reservoir defeat and storm names reference events 11125860 and 11125872 respectively, rather than treating the defeat-time clear request as the initial Sword Saint fight state.
`dragon-defeated` uses 9309 and the reset in event 12505950.

| Map | Default hour / environment | Selectable story states | GPARAM files / sub-IDs used | Collision LightSet IDs | Remaining selection/rendering limits |
| --- | --- | --- | --- | --- | --- |
| m10_00, Hirata | 0 / 0 | memory, father-memory (8304) | m10_00_0000 / 0 | 0, 100, 110, 120 | Memory cinematic lighting transitions are not a complete world-event simulation |
| m11_00, Outskirts | 6 / 1 | five clock names, invasion, tomoe | m11_00_0000, 0020, 0030 / 0, 20, 30 | 0, 100, 110, 130 | Fire sky export and GPARAM family provenance need verification |
| m11_01, Castle | 6 / 1 | five clock names, invasion, tomoe, shura-fire | m11_01_0000, 0010, 0020, 0740; runtime also 0750 / 0, 10, 20, 40, 50 | 0, 100, 110, 120 | Some event sky models are not exported |
| m11_02, Reservoir | 0 / 0 | prologue, five clock names, invasion, sword-saint-defeated, sword-saint-storm | m11_02_0000, 0020, 0920, 0930 / 0, 20, 30 | 0, 100, 110, 120 | 09 family pairing is inferred; transitions snap |
| m13_00, Dungeon | 6 / 1 | five clock names | m13_00_0000 / 0 | 0, 100, 110, 130 | First visit order is player-dependent |
| m15_00, Depths / Mibu | 6 / 1 | five clock names | m15_00_0000 / 0 | 0, 100, 110, 120 | Event 11505290's palanquin transition is interaction-gated, not a fixed map default |
| m17_00, Valley | 6 / 1 | five clock names | m17_00_0000 / 0 | 0, 100, 110, 120, 130 | Export has only environment variants 1-4 |
| m20_00, Senpou | 6 / 1 | five clock names, illusion, illusion-cleared | m20_00_0000, 0010 / 0, 10 | 0, 100, 110, 120, 130 | Illusion region switching uses native region 2002880; export has only variants 1-4 |
| m25_00, Fountainhead | 22 / 4 | palace, dragon-defeated | m25_00_0000 / 0 | 0, 100, 110, 120, 130 | Dragon-specific rendering remains with its existing arena staging |
| m89_00_00_99, sky library | 6 / 1 | five clock names | s89_00_0099 / 0 | No terrain | Cinematic/library map, no default player start |

## Regions and environment maps

The MSB collision under the player or free camera supplies `[LightSetID, FogParamID, LightScatteringID, EnvMapID]`.
Older layout exports are supplemented from the read-only installed MSB for the three previously omitted fields.
The focus is checked whenever it moves, even if draw-group culling is off, and the nearest surface below takes precedence over a ceiling above.
A negative field leaves the previous selection in force; zero is a real selected ID and resets an interior selection.
`RegionSelection` retains all four independently, `MapLighting` consumes the light ID, and the fog and camera environment selectors retain their own bank IDs.
The installed nine gameplay maps all use zero for their collision fog, scattering and environment IDs, so nonzero mappings cannot be verified from this corpus.
An unverified positive environment bank is reported without guessing that its number names a GILM probe.
There is no recovered renderer for legacy FogParam or LightScattering banks, and their IDs are not silently aliased to LightSetID.
Volumetric fog retains its existing renderer and row convention; no new scattering formula has been invented.
The existing spatial probe selection and blending remain unchanged for the native zero environment bank.
Missing complete probe pairs for an authored environment are reported rather than replaced by nearest-time probes.
The existing GI coefficient/reflection atlas is constructed at map load; changing its hourly layers dynamically is still a rendering follow-up, independent of the selector and camera cubemap handle reset.
These limits mean selection is reviewable against the event data, but exact visual equivalence for every story state is not yet established.

## Verification

Use `CARGO_TARGET_DIR=target/alt-lf` and six Cargo jobs for every command in this work.
`cargo run --offline -p sekiro-formats --example lighting_audit -j 6` produces a read-only census of relevant installed event instructions, GPARAM keys and collision tuples.
Its local output is `target/alt-lf/lighting-audit.txt`.
Load a first-visit scene with `sekiro-rs --play --map <map> --simple-ai --passive` without an hour override.
Load a later scene with `--story <name>` and inspect the `map story lighting` log for hour, environment, sub-ID, file, flags and sky.
Use `--arena genichiro` and `--arena owl` for morning/evening castle-top controls, and restart Genichiro after Tomoe to verify the storm reset.
Cross an interior/outdoor collision boundary and check the independently selected region IDs.
For Senpou, `--story illusion` enables the original region event, which alternates sub-IDs 0 and 10 across region 2002880.
Redirect `PLAY_PERF_DIR` into `target/alt-lf/captures/` before enabling `PLAY_LOG_PERF=1` so verification never writes into cache or logs.
Captures and raw performance CSVs stay under that target directory and are not tracked.

## Capture results

All nine gameplay maps were captured at their default start 0, and castle top was captured at both Genichiro's morning and Owl's evening story state.
The run command and raw stdout/stderr are retained with each PNG under `target/alt-lf/captures/`.
`capture-one.ps1`, `idle.txt`, `idle-warm.txt` and `summarize.ps1` make those local runs reproducible.
The first Hirata and castle shots were startup diagnostics and are retained with `-startup` suffixes; the castle diagnostics showed only sky before map pipelines were ready.
The accepted Hirata and castle shots use tick 720 of a 900-tick idle script, while the other accepted map shots use tick 180 of a 240-tick script.
Median wall times use `perf_log.rs` rows with no pending pipelines, at ticks 480-840 for the warmed runs and 120-240 for the other maps.
The CSV paths and sample counts are in `frame-times.csv`, and the multi-map visual review is in `review.png`.
These are short VSync-capped observations on the shared GPU, with concurrent CPU validation, rather than isolated GPU-cost or exact-game visual comparisons.

| Capture | Median frame ms | Ready-pipeline samples | Visual review |
| --- | --- | --- | --- |
| m10-default.png | 16.668 | 360 | Memory sky is selected, but grey/overbright terrain and broken foliage/material layers still look wrong |
| m11_00-default.png | 16.504 | 48 | Dim temple interior with morning light outside; no obvious selection mismatch |
| m11_01-default.png | 16.582 | 120 | Morning exterior light reaches the start; pale bright snow and layered materials need rendering review |
| m11_02-default.png | 16.600 | 94 | Dark prologue passage; no obvious selection mismatch |
| m13-default.png | 16.608 | 120 | Dark cave interior; no obvious selection mismatch |
| m15-default.png | 16.732 | 54 | Dim rocky passage; no obvious selection mismatch |
| m17-default.png | 17.491 | 78 | Morning valley; conspicuous pale terrain/material coverage remains |
| m20-default.png | 16.491 | 120 | Warm entrance interior; no obvious selection mismatch |
| m25-default.png | 20.522 | 81 | Authored palace sky is selected; bright blue/green night scene still needs tone/probe calibration |
| castle-top-morning.png | 16.582 | 359 | Rooftop and morning shadow are visible; flat pale fog obscures distant scenery |
| castle-top-evening.png | 16.728 | 360 | Evening shadow and sky differ from morning; flat brown fog obscures distant scenery |

Hirata, the castle horizon and Fountainhead need lighting/rendering follow-up despite matching the selected event variants.
The additional `senpou-illusion.png` capture confirms sub-ID 10 at 12 h and native sky model m900085; that model is absent from the read-only sky export, so the current renderer logs the missing model and retains m900010.
The specific cause of their brightness/fog mismatch is not established by these captures, and tone-curve, exposure and probe-blend calibration remains the other collaborator's work.
Valley and castle snow/foliage problems are visible material issues rather than evidence of a wrong story flag.
The remaining hard provenance gaps are the 09 reservoir family pairing, unavailable event sky exports, nonzero FogParam/LightScattering/EnvMap bank mappings, and an exact first-visit order for optional areas.
Those gaps are explicit rather than covered by invented tuning.

## Checks

The selector tests cover every default, all declared presets, every existing boss arena, invasion sub-IDs, reservoir clear/storm restoration and independent region fields.
The collision-system regression crosses two authored-config fixtures with culling disabled and verifies the light, fog, scattering and environment selections.
The castle restart regression verifies both hour reset and removal of the retained storm sub-ID.
The event regression checks all four flag-range states, and existing map, lighting, sky, story and arena filters pass.
The slow ignored arena tier was not run.
The slower installed-data preset census is explicitly marked ignored and was run with the selector's `--include-ignored` filter.
All seven selector tests, including the native Senpou sky-override regression, pass.
Earlier validation passed 12 map, 14 lighting, four sky, one arena-effect, one castle-restart, two story/range and 20 arena tests, with one slow arena case ignored.
The GPARAM format filter also passes its two parser tests.
The final map, lighting, sky, arena-effect, castle-restart, story, flag-range and arena filters all pass against the shared working tree.
Workspace/all-target Clippy with warnings denied and `cargo fmt --all --check` both pass after the other agent resolved their temporary blockers.
The local final outcomes are retained in `checks-final-summary.txt`, `clippy-final.txt` and `fmt-final.txt`.

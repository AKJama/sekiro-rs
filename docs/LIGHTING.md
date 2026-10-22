# Lighting selection

This page describes selection from the installed Sekiro data, independently of the renderer's tone curve, auto exposure and probe blending.
The shared implementation is `crates/game/src/lighting_selection.rs`.
Each map's original flags and event references live in one module under `crates/game/src/lighting/maps/`.
No game asset or decoded event body is copied into those modules.

## Light unit bridge, 2026-10-15

The lux-per-native-unit bridge is now traced rather than fitted: the deferred accumulators have no `1/pi`, so `SCENE_TO_BEVY = pi / exposure = 3135.61` and hemisphere, probe, sky and fog radiance use `1 / exposure` per native unit.
The previous 4500 was 1.435 times too large.
The directional slots remain about 4 to 7 times brighter than retail; ablations and the candidate list are in [EXPOSURE.md](EXPOSURE.md#brightness-audit-2026-10-15).
The `reference progression` and light-set selection described below are unchanged.

## Sensitometric exposure audit, 2026-10-15

The subsequent [post-chain colour audit](COLOUR.md) wires the missing pre-gamma colour matrix, accepts the authored filmic-contrast field name and replaces the three-level bloom approximation with the native authored pyramid.
The exact-night colour controls are neutral and LUT 20 is already sampled in the recovered display domain; no warmer tint or exposure offset is substituted.

The shared mode-7 tone curve used natural exponentials where the native shader's `exp` instructions require base-two powers.
The corrected `tonemap.wgsl` uses `exp2` with the original coefficients and authored contrast, without changing exposure, light colours, snow textures or the inferred scene-unit bridge.
The controlled spot-002 mean linear luminance falls from 19.70435 to 13.63111 percent white, compared with retail 7.02082.
The blue cast and excess saturation remain, and colour ratios worsen as the corrected shoulder compresses highlights less aggressively; this is a native-formula correction rather than complete L1/M1 parity.
The pinned RustSekiro inspection found the same exponential-base error and retained by-eye exposure offsets, so no exposure implementation was copied.
See [EXPOSURE.md](EXPOSURE.md) for native evidence, all six luminance/saturation/hue comparisons, remaining unknowns, source isolation and validation.

## Extra LightSet slots and sun shadows, 2026-10-15

The native deferred directional accumulator leaves slots 1, 2 and 3 unattenuated by the sun's shadow map.
Only slot 0 receives the sun-shadow colour factor, for both its diffuse and specular contributions.
The separate `SpecColor` values belong to those same four slots, rather than a fifth independent shadow-casting light.
Consequently the existing `map_material.wgsl` slot-0-only sun attenuation is retained, with no packing or BTL change.
Applying the sun factor to every slot would contradict the inspected native shader, even if it darkened the image toward a particular reference.

The evidence is the installed renderer bundle's `GXLightAcc_DDirLightShadowed1.ppo`, compared with `GXLightAcc_DDirLight1.ppo` and `GXLightAcc_DDirLightShadowed1_Cloud.ppo` in the ignored shader inventory and disassembly described in [SHADERS.md](SHADERS.md#evidence-and-coverage).
The shadowed shader samples red and alpha from `g_ShadowTexture` at t32.
At disassembly lines 195-201, red supplies the four-component `1 + FC_SsaoWeight * (red - 1)` factor, while alpha supplies the RGB factor `1 - FC_ShadowColor.rgb + alpha * FC_ShadowColor.rgb`.
The preceding `GXRend_CompositeShadowMask.ppo` explicitly copies `g_SSAOTexture.r` into output red and initializes alpha to one, independently of its G-buffer-derived green/blue channels.
Thus the shared texture name does not mean that its red SSAO channel is the sun shadow.

In the shadowed accumulator's ordinary diffuse branch, lines 242-248 apply the RGB sun factor to `Col_0`, but apply only the second SSAO weight to `Col_1`.
Lines 250-251 apply the sun factor to `Spc_0`; line 288 applies only the second SSAO weight to `Spc_1`.
The enabled local branches at lines 325-367 and 369-411 use the third/fourth SSAO weights with `Col_2/3` and `Spc_2/3`, without the sun factor or additional shadow samples.
The positive material-channel diffuse branch and the cloud-shadow sibling retain that same slot distinction.
The unshadowed sibling samples only red and omits the sun factor altogether.
These operations establish no independent shadow map for the three extra directional slots in the inspected accumulation path.

A read-only census of installed `m11_00_0000.gparam` and `m11_01_0000.gparam` LightSet groups confirms four angle/diffuse/specular slots, hemisphere colours and two local-enable switches, with no per-extra-slot shadow selector.
For example, at hour 22 the Outskirts row 100 enables both local slots and supplies local-1 diffuse `(0.676345,0.790052,0.926500,1)` and specular `(0.431373,0.549020,0.764706,0.5)`.
The Castle row 100 supplies the same local-1 RGB with diffuse power 0.5 and enables both local slots.
These are authored fill/specular contributions, not evidence of omitted sun-shadow flags.
The CPU blend described in [SHADER-INPUTS.md](SHADER-INPUTS.md#lights-and-ambient-function-310630-the-lightset-blend) separately uploads all four SSAO weights; that visibility response remains distinct from sun-shadow attenuation and is not newly implemented by this audit.
The existing Bevy shadow response also does not reproduce native `FC_ShadowColor` tinting, so this finding establishes slot routing rather than full lighting parity.

The earlier legacy/current comparison isolates excessive output to the native direct compositor as a whole, but does not establish that unshadowed extra slots are a defect.
The remaining brightness regression requires a separate material/BRDF, SSAO, unit-bridge or composition investigation; no brightness multiplier or shadow heuristic is introduced here.

### Shadow audit measurements

Before images are the preceding current-light captures; after images are freshly rendered with the same saved camera/player/FOV, night story, 2560 x 1440 matcher renderer and unchanged game-source hashes.
The same scene/ground masks, exact inverse-sRGB transfer and Rec.709 weights are reused.
Y statistics are percent linear white, RGB means are display bytes, ground contrast is ground Y P95/P05, and `>90%` is the percentage above 0.90 display brightness.
These are unchanged-renderer controls, not an improvement claim.

| Shot / setup | Mean Y | P05 Y | P95 Y | Ground contrast | Mean R | Mean G | Mean B | B/R | >90% |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 002 / real | 7.02082 | 1.46816 | 26.12795 | 13.49085 | 54.48552 | 65.08182 | 71.46529 | 1.31164 | 0.01377 |
| 002 / before | 19.70544 | 1.55635 | 65.73266 | 47.92656 | 82.21704 | 102.94774 | 115.33511 | 1.40281 | 2.07724 |
| 002 / after | 19.70544 | 1.55635 | 65.73266 | 47.92656 | 82.21704 | 102.94774 | 115.33511 | 1.40281 | 2.07724 |
| 003 / real | 7.50628 | 1.17112 | 22.61886 | 6.63697 | 57.72892 | 66.47912 | 75.43655 | 1.30674 | 0.00024 |
| 003 / before | 22.72757 | 0.83534 | 65.16383 | 2.39233 | 85.27277 | 104.40159 | 118.30477 | 1.38737 | 0.33798 |
| 003 / after | 22.72755 | 0.83534 | 65.16383 | 2.39233 | 85.27271 | 104.40153 | 118.30470 | 1.38737 | 0.33798 |
| 025 / real | 10.60234 | 2.32630 | 42.36289 | 5.93134 | 73.56626 | 79.78949 | 84.84180 | 1.15327 | 0.00005 |
| 025 / before | 12.95241 | 1.19072 | 70.19357 | 7.88895 | 65.99703 | 82.22550 | 92.95315 | 1.40844 | 3.74569 |
| 025 / after | 12.95186 | 1.19072 | 70.19357 | 7.94986 | 65.99586 | 82.22424 | 92.95193 | 1.40845 | 3.74564 |
| 092 / real | 4.34681 | 1.26413 | 12.77151 | 4.16246 | 44.53931 | 53.69585 | 57.30477 | 1.28661 | 0.00000 |
| 092 / before | 6.98900 | 1.05396 | 33.03845 | 10.84215 | 48.16652 | 63.47323 | 74.32054 | 1.54299 | 0.16966 |
| 092 / after | 6.98299 | 1.05396 | 33.03135 | 10.93505 | 48.13876 | 63.44132 | 74.29480 | 1.54335 | 0.16966 |

The shots are spot-002-shot-004, spot-003-shot-012, spot-025-shot-085 and Castle spot-092-shot-293.
Masked RGB mean absolute differences are respectively 0, 0.00006118, 0.03753040 and 0.08672587 display bytes.
The sorted complete visibility records, collision choices and camera settings agree in all four pairs.
The requested temple PNGs and real/before/after panel are retained in the external progress collection, with the complete measurements, mask provenance, source-hash comparison and render requests.
Small repeat differences do not establish a fidelity gain; the excessive output remains unresolved.

### Shadow audit render gates

Both canonical six-view gates pass with `target/alt-sf`, six Cargo jobs, serial windowless processes and unchanged fixtures/tolerances.
Outskirts takes 45.296 seconds with totals `(void, depth gap, untextured, grey, pop-in) = (0,493,0,2356,28846)`.
Castle takes 64.342 seconds with totals `(1933,4885,0,1702,31552)`.
The following table records every colour-category difference from the committed fixture, rather than changing that fixture to match this run.
Grey changes reflect crossing the neutral-RGB criterion; pop-in changes reflect crossing the normal/all-visible threshold of 12 display values with the existing renderer.
They are observations from unmodified rendering, not effects of a sun-shadow fix, and are not assigned to individual emitters.

| Map / view | Grey fixture -> measured | Pop-in fixture -> measured |
| --- | ---: | ---: |
| Outskirts 0 | 274 -> 274 | 8711 -> 8650 |
| Outskirts 1 | 183 -> 189 | 294 -> 295 |
| Outskirts 2 | 383 -> 378 | 2484 -> 2484 |
| Outskirts 3 | 725 -> 724 | 5503 -> 5420 |
| Outskirts 4 | 648 -> 641 | 6831 -> 6756 |
| Outskirts 5 | 147 -> 150 | 5245 -> 5241 |
| Castle 0 | 196 -> 205 | 5083 -> 5097 |
| Castle 1 | 288 -> 288 | 7488 -> 7488 |
| Castle 2 | 588 -> 588 | 5879 -> 5879 |
| Castle 3 | 49 -> 51 | 3782 -> 3782 |
| Castle 4 | 482 -> 480 | 2400 -> 2362 |
| Castle 5 | 92 -> 90 | 6958 -> 6944 |

Void and untextured counts equal their fixtures in every view.
Depth gaps also agree except Castle view 0, which measures 148 rather than 149, within the original 16-pixel minimum tolerance.
Raw reports and logs are under `target/alt-sf/render-gate/`.
Workspace formatting and release all-target Clippy with warnings denied pass.
The release game suite passes 126 unit tests, including the installed-data gate, and six integration tests, with two existing unit-test ignores.

## Selection

The loader evaluates installed `common.emevd` event 100 through the shared event interpreter with the current map and story flags.
Event 820's existing `StoryTime` rules first raise flags 9800 and 9801 from defeated bosses 9303 through 9306.
Flag-range conditions distinguish all on, all off, any on and any off, and map-membership conditions are evaluated against the loaded area and block.
Event 100 selects the environment variant and the enabled m89 sky group, including the Hirata memory, Fountainhead and the Ashina fire branch.
The selected sky group's model comes from the installed m89 MSB rather than an invented map-to-sky table.
The most recent enabled map-part command wins over the common sky, with a bounded recency list so repeated events do not grow scene state indefinitely.
The renderer retains the first m89 MSB part as each group's dome, matching the shared selector, and keeps each exported companion cloud primitive with that dome.
Clock-only changes retain the authored sky and environment; only their independent native commands replace them, while explicit `--hour` remains a diagnostic override.
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

### Reference progression audit, 2026-10-15

The external reference collection is overcast daytime, not night.
Two explicit Ashina presets encode the user's candidate boss progression without an hour override.

| Candidate preset | Defeat flags | Event-820 clock result | Both maps: hour / environment / sub-ID | Sky | GPARAM |
| --- | --- | --- | --- | --- | --- |
| `gyoubu-defeated` | 9301 | No time-advancing boss; 9800 remains off | 6 / 1 / 0 | m900000 | Area's ordinary 0000 file |
| `genichiro-defeated` | 9301, 9303 | One boss in 9303-9306 raises 9800 | 12 / 2 / 0 | m900010 | Area's ordinary 0000 file |

Installed Outskirts event 11105800 instruction 14 sets Gyoubu flag 9301, while Castle event 11115800 instruction 16 sets Genichiro flag 9303.
Common event 820 counts flags 9303-9306, requiring one for afternoon and three for evening; Gyoubu's 9301 is outside that range.
Common event 100 resolves the resulting environment and m89 sky group.
The installed-data regression `reference_progression_uses_installed_boss_and_clock_events` verifies both defeat writes and both maps' selected hour, environment, file, sub-ID and sky.
These candidates therefore have different lighting, even though neither selects invasion, fire or Tomoe weather.

The reference's reviewed travel menu at shot 25 offers Castle Tower Lookout, and its loading screen shows Attack Power 3.
Together with the overcast afternoon skyline this supports `genichiro-defeated`; the user's approximate playtime alone does not prove or disprove a boss flag.
The indexed clocks read 01:54:03.34 at spot 002, 01:58:40.95 at the Castle travel menu, and 02:24:17.94 at spot 092.
Those are playtime readings, not the world lighting hour.
This is the best-supported candidate rather than a recovered save snapshot.
Unknown quest, idol, pickup and interaction flags remain unset, so gate pose and NPC state cannot independently certify every flag.
Named progression presets seed offline map-story evaluation with those boss flags and their known persistent defeat writes.
Gyoubu adds 11100800, 6801 and 6078 from event 11105800; Genichiro additionally adds 11110800, 6803 and the completed-cutscene guard 9403 from event 11115800.
The regression checks every one of those positive writes against the installed event instructions.
Retaining 9403 prevents a post-defeat load from replaying the completion sequence merely because the sparse fixture omitted its completion guard.
Generic clock presets retain their existing first-visit object policy.
Reference matching, exact-camera replay and canonical Ashina gates now use the post-Genichiro preset.
Ordinary exploration and boss-arena defaults keep their earlier earliest-visit policy.

The default is the earliest reachable lighting state, not a guessed average time of day.
The reservoir is the opening prologue, where the installed event conditions use 8306 and the retained past-night flag 9810 without time-advancing boss flags.
Hirata is a memory and event 100 selects variant 0 for the map independently of world progression.
Fountainhead is entered after world progression, but event 100 selects its fixed variant 4 independently of the world-clock flags.
The other maps use the morning branch with no time-advancing boss defeated as the earliest available state.
Optional map visit order is not uniquely determined by the event files, so this earliest-state policy is explicit rather than a claim that every player first visits those maps in that state.
Boss arenas instead seed their existing `ArenaSpec.beaten` and `BossEvents.flags`, and preserve `ArenaSpec.story_time` for the actual starting phase.
An explicit `--story NAME` selects the corresponding map profile, and an explicit `--hour H` remains a clock diagnostic override.

### Matching real-game opening screenshots (#47)

The plain Outskirts start, with no `--story` or `--hour`, is the real game's opening: no Gyoubu flag 9301 and no story-time flag 9800-9803, so common event 100 selects morning variant 1 (the 6 h key), sub-ID 0 and sky `m900000`.
The render and reference gates use `genichiro-defeated` (12 h), so they are not like-for-like with opening screenshots.
The engine's clock hour for variant 1 is not in the data; the outdoor row also authors an unused, more orange 4 h key, but a playtest against the reference shot (2026-10-17) matched the sky best at 6 h.
Unknown story names produce an error listing that map's available names.

The clock names `morning`, `afternoon`, `evening`, `night` and `dawn` correspond respectively to no time flags, 9800, 9800-9801, 9800-9802 and 9800-9803.
`invasion` also sets 8302, selecting event 100's Ashina fire branch and sub-ID 20.
`tomoe` references the castle handover event 11115820, including the area sub-ID and `SetLighting(3)` instructions.
`shura-fire` references event 11115910's sub-ID-40 instruction.
`illusion` uses 8400 and 9303, references event 12005210's sky and sub-ID instructions, and runs region event 12005826.
`illusion-cleared` additionally sets 62000705 and references event 12005855's clear sub-ID instruction.
The reservoir defeat and storm names reference events 11125860 and 11125872 respectively, rather than treating the defeat-time clear request as the initial Sword Saint fight state.
`dragon-defeated` retains True Monk prerequisite 9309 and sets Divine Dragon defeat 9310 from event 12505880; event 12505950 is True Monk's defeat and its reset does not belong to this preset.

| Map | Default hour / environment | Selectable story states | GPARAM files / sub-IDs used | Collision LightSet IDs | Remaining selection/rendering limits |
| --- | --- | --- | --- | --- | --- |
| m10_00, Hirata | 0 / 0, first memory; fixed hour-0 rows | memory, father-memory (8304) | m10_00_0000 / 0 in both states; sky m900060 | 0, 100, 110, 120 | Native selection matches; shared fog-enable lookup and omitted local directional fields remain; see Hirata audit below |
| m11_00, Outskirts | 6 / 1 | five clock names, invasion, tomoe | m11_00_0000, 0020, 0030 / 0, 20, 30 | 0, 100, 110, 130 | Event/catalog choices and fire/storm dome exports verified; shared four-light/specular and runtime sky-group gaps remain; requested uncapped runs still show a 60 Hz limit |
| m11_01, Castle | 6 / 1 | five clock names, invasion, tomoe, shura-fire, owl-fight | m11_01_0000, 0010, 0020, 0740, 0750 / 0, 10, 20, 40, 50 | 0, 100, 110, 120 | Native encounter flags restore skipped intro GPARAMs; all selected sky exports are present; shared phase/probe/sky routing and presentation limits remain; see Castle audit |
| m11_02, Reservoir | 0 / 0 | prologue, five clock names, invasion, sword-saint-defeated, sword-saint-storm | m11_02_0000, 0020, 0920, 0930 / 0, 20, 30 | 0, 100, 110, 120 | Event selection audited; 09 clear pairing remains inferred; global probe fallback and fog-bank routing need shared fixes; see Reservoir audit |
| m13_00, Dungeon | 6 / 1 | morning, afternoon, evening, night, dawn | m13_00_0000 / 0 in all five states | 0, 100, 110, 130 | Selection agrees with installed data; dark cave rendering and shared-machine frame-time limits are documented in the m13 audit below |
| m15_00, Depths / Mibu | 6 / 1 | five clock names; dawn returns to 6 / 1 | m15_00_0000 / 0; fixed sky m900015 | 0, 100, 110, 120 | Event 11505290's afternoon transition is interaction-gated; 50 sized probe regions have no exported GIIV planes; exact visual fidelity and isolated steady frame time remain unverified |
| m17_00, Valley | 6 / 1 | morning, afternoon, evening, night, dawn | m17_00_0000 / 0 in all five states | 0, 100, 110, 120, 130 | Native selection and 110/100 crossing audited; native/exported probes have only variants 1-4; shared material, specular, missing-row and pacing limits remain; see Valley audit |
| m20_00, Senpou | 6 / 1 | five clock names, illusion, illusion-cleared | m20_00_0000, 0010 / 0, 10 | 0, 100, 110, 120, 130 | Cleared illusion entry sky corrected; native region round trip verified; 85 GI volumes per variant 1-4; empty 0010 family, missing m900085/variant-5 GI and shared light omissions block exact fidelity; see Senpou audit |
| m25_00, Fountainhead | 22 / 4, fixed authored hour-0 rows | palace, dragon-defeated (9309, 9310) | m25_00_0000 / 0 | 0, 100, 110, 120, 130 | Native selection checked; live 100-to-130 crossing passes in both states; water, exposure defaults and frame-time limits remain shared |
| m89_00_00_99, sky library | 6 / 1 | five clock names | s89_00_0099 / 0 | No terrain | Cinematic/library map, no default player start |

Fountainhead follow-up, 2026-10-13:
Both selectable profiles use common event 100's fixed environment 4, sky group 8907070/model m900070 and sub-ID 0, irrespective of world-clock progression.
The 22-hour selector label is correct: the installed base file has only hour-0 entries, so it does not interpolate ordinary night values into this map.
The corrected Dragon preset adds flag 9310 and removes the unrelated True Monk reset; its selected file, sky and values remain unchanged.
The native audit preserves the bridge arena's 22-hour and sanctuary arena's 12-hour starts; the existing Divine Dragon transition still owns its SetLighting(1) phase.
Native collision fields are checked independently, with fog/scattering/environment banks all zero; the live start-5 hall walks change LightSet 100 to 130 in both profiles.
LightSet 110 is also selected in the stationary indoor control, while the attempted doorway walks were obstructed and are not counted as crossing proof.
Collision 130 has no matching GPARAM row and currently falls back to row 0 through `Param::at`; no invented 130-to-220 remapping was added, and the engine's missing-row fallback remains a provenance limit.

At the default start, native sun RGB/W `(0.607843, 0.705882, 0.921569, 1)` becomes linear RGB `(0.369473, 0.498269, 0.849289)` at 4500 lux, and fill W 0.4 becomes 1800 lux.
The authored hemisphere mean is `(0.165783, 0.230277, 0.321626)`; the shared probe path replaces global ambient when GILM 0/variant 4 is ready and loads 49 GIIV volumes into a 106 MiB atlas.
Fog GameScale 1.2, fixed density 0.0025, height offset 45 and far depth 700 upload as density 0.00208333, height offset -45 and far depth 840; sky colour becomes `(0.003697, 0.016988, 0.049433)`.
Both states upload exposure gain 2.143547, middle gray -4.5, adaptation bounds -0.5/0.5 and LUT 20, but auto exposure is false because the native file lacks the enable field queried by `lighting.rs:167`.
Recovering the native enable/default and calibrating tone, exposure and probe composition belongs to RustSekiro's collaborator; a warmer map tint would contradict the authored light colours.
Other confirmed shared gaps are omitted local-light colour/enable fields in `lighting.rs:881`, and textureless Water0 exports using the generic opaque grey/roughness fallback in `crates/extract/src/maps/pieces.rs:377,397` instead of native water rendering.
The water evidence includes m500010 and m810000, with base colour `(0.45, 0.43, 0.40, 1)`, roughness 0.85 and no native material extras.

All new Fountainhead proof is under `target/alt-m25/captures/`, including before/after PNGs for both profiles, start-3 lake views, hall crossing images, raw CSVs, commands, binary/source hashes and `report.md`.
These runs use `--uncapped`, which selects AutoNoVsync, at 1600 by 900; ready samples use ticks 480-840 after asset loading and exclude ticks 710-750 around screenshot readback.
Other game processes and compilers were active, and shared `map.rs` changed between the before/after source snapshots, so these are observations rather than an isolated performance comparison of the metadata patch.

| Fountainhead view | Before median / p99 ms | After median / p99 ms | Ready samples before / after |
| --- | --- | --- | --- |
| palace, default start 0 | 22.398 / 73.963 | 46.066 / 64.473 | 194 / 114 |
| dragon-defeated, start 0 | 23.816 / 81.810 | 39.894 / 56.782 | 180 / 134 |
| palace, lake-side start 3 | 16.610 / 25.409 | 17.401 / 49.924 | 318 / 271 |

Default-start GPU diagnostics before the patch show approximately 14.788 ms opaque, 2.964 ms prepass and 1.969 ms fog; after samples show 31.394, 4.634 and 4.524 ms under different contention.
The start-3 opaque cost stays near 6 ms in both runs.
Disabling GIIV reduces the default opaque diagnostic to 12.953 ms, but worsens wall time under contention, so it establishes neither an overall speedup nor a justified reason to remove authored probes.
~~The shared per-fragment scan in `map_material.wgsl:84` tests all 49 volumes; spatial candidate filtering that preserves native blending is a concrete optimization for the shared probe implementation.~~
Fixed by the [shared spatial GI lookup](#spatial-gi-and-fallback-2026-10-15), with the original record order and blend arithmetic retained.
No safe map-specific visibility exclusion was established, no native geometry or lighting was hidden to improve timings, and steady frame time or exact rendered parity is not claimed.
The map regression, all seven shared selection tests including the installed preset census, and six collision/map tests pass; game all-target Clippy with warnings denied and workspace formatting both pass, with results retained under `target/alt-m25/`.

### Valley audit - 2026-10-14

The current layered-material export was audited against common event 100, m17 events, native GPARAM rows, MSB collision tuples and the installed m89 sky library.
Morning, afternoon, evening, night and dawn select respectively hour/environment 6/1, 12/2, 18/3, 22/4 and 2/5, with skies m900000, m900010, m900020, m900030 and m900050.
All five retain base file m17_00_0000/sub-ID 0; no m17 event lighting override was found that should replace those presets.
Watering Hole and Ape's Burrow retain their ArenaSpec afternoon starts, rather than inheriting the map's morning default.
The map-specific installed-data regression logs each native parameter beside actual LightSet/GameFog conversions and audits the raw environment containers.

Collision fog/scattering/environment banks are independently zero, and the default start selects h000100/LightSet 100.
The Gun Fort short replay proves LightSet 110 to outdoor 100 and back to 110 without a respawn in every selectable story state.
Rows 120 and 130 are absent from the base LightSet group and currently fall back to row 0 through `gparam.rs:69-73`.
Family 0200/0220 contains row 120, but the engine's family/missing-row rule has not been established; no speculative file switch or row remapping was added.
Both raw m17 environment containers and exported probes contain only variants 1-4, so dawn's correctly selected environment 5 loads no GI volumes and uses the hemisphere fallback.
Recovering the native absent-variant fallback belongs to shared probe selection, rather than changing the authored dawn clock.

Morning sun RGBA approximately `(1, 0.784314, 0.705882, 3)` becomes linear RGB `(1, 0.615148, 0.498269)` at 13500 lux, and fill W 0.3 becomes 1350 lux.
The hemisphere mean is `(0.046136, 0.066436, 0.103806)` before ready probe 0 replaces global ambient.
Each environment 1-4 installs 71 complete coefficient volumes in a 47 MiB atlas.
Morning fog uploads density 0.001, distribution 0.03, height-density scale 0.05, offset 550 and near/far 0.1/400; afternoon GameScale 2 converts fixed density 0.0025 to 0.00125 and far depth 400 to 800.
Native auto exposure is enabled in all five states, with exposure stops 0.3/0.4/1.2/1/0.2 and middle gray -3.6/-5.4/-6.6/-4.9/-4.9.
Those values agree with logged renderer uploads; absolute radiance, tone, metering and probe composition remain the other collaborator's work.

The old claim that m000064 lacks a decoded SnowGround layer is stale: its current export has version 2/kind 2/variant 6 and authored snow/detail raw normals.
The conspicuous triangular snow pattern disappears with `--no-layers` but persists with `--no-giiv` and `--no-shadow`.
Its 999 normals are normalized and continuous across coincident vertices, and maximum absolute normal-dot-tangent is 6.1e-8.
This isolates a shared layered-material normal-path defect around `map_material.wgsl:375-411,438`, but does not yet identify the precise native arithmetic or UV correction.
Shared follow-ups also include unused native normal-map AO, independent directional SpecColor0/1 routing, and ~~per-fragment scanning of all 71 GI records in `map_material.wgsl:84`~~.
Spatial candidates are fixed; the four diffuse/specular input records are retained, but the [experimental native direct compositor](#spatial-gi-and-fallback-2026-10-15) remains opt-in because it fails Castle's grey gate.
The proposed shared fixes and exact evidence are in `target/alt-m17/captures/report.md`; no map tint or geometry exclusion was used to conceal them.

Before/after story-state PNGs, raw uncapped perf CSVs, native audits, controls, walks and source/binary hashes are retained under `target/alt-m17/captures/`.
These are current-export comparisons with an unchanged production profile, not a claimed map-specific rendering speedup.
The default viewer baseline reports GPU opaque/fog/prepass costs 2.79/1.74/0.40 ms, while the wider start-3 baseline reports approximately 2.70/1.35/0.31 ms.
The heavier open-valley start-1 view reports approximately 5.04/1.46/0.61 ms; it supplies the heavy-open before/after perf pair, with start 3 retained as an indoor control.
Active compilers and host scheduling affect the much larger wall times, so exact rendered parity and steady pacing remain unverified.

| Valley view/state | Before median / p99 ms | After median / p99 ms | Ready samples before / after |
| --- | --- | --- | --- |
| morning, start 0 | 63.144 / 121.793 | 96.700 / 277.270 | 85 / 50 |
| afternoon, start 0 | 58.247 / 149.565 | 31.828 / 164.261 | 74 / 122 |
| evening, start 0 | 59.681 / 206.493 | 16.974 / 96.047 | 71 / 213 |
| night, start 0 | 107.482 / 264.849 | 71.794 / 209.677 | 52 / 63 |
| dawn, start 0 | 56.401 / 317.134 | 16.760 / 57.941 | 89 / 276 |
| morning, heavy open start 1 | 82.958 / 250.793 | 16.704 / 19.904 | 61 / 320 |
| morning, indoor start 3 | 57.155 / 121.683 | 16.605 / 26.242 | 86 / 320 |

Runs select AutoNoVsync and continuous updates, but the later 16.7 ms plateau still prevents treating their wall medians as demonstrated uncapped throughput.
Heavy start-1 after diagnostics show opaque/fog/prepass 4.60/1.54/0.47 ms; the default viewer after diagnostics show 3.70/1.14/0.35 ms.
A per-process DX12 control fails to warm the default view's pipelines within the 60-second run limit, and neither DX12 run has ready samples in the prescribed tick window, so those controls are excluded from the performance table.

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

## Spatial GI and fallback, 2026-10-15

The GI upload now appends a conservative world-space grid to the existing storage buffer once per map.
Each cell retains the original large-to-small record order, and the shader still runs the original AABB, oriented-box fade, coefficient sampling and reflection blend on those candidates.
Whole-map boxes participate in every cell and in the outside-grid list; a cell of padding prevents CPU/GPU boundary rounding from removing a contributing box.
The boundary regression compares ordered receiver coverage against a full scan, including rotated AABBs, negative coordinates, inclusive faces, empty maps and outside-grid receivers.
Outskirts averages 3.89 candidates instead of 55 records, and Castle averages 5.45 instead of 107, using 70/67 KiB additional storage respectively.

Camera probe readiness no longer zeros the authored hemisphere globally.
The shader removes the same diffuse and specular hemisphere response under spatial GI coverage that the fallback adds, preventing an ambient-specular remainder inside covered regions.
Camera fallback now uses the existing smallest-containing-box and nearest-point selection instead of unconditionally choosing the largest box.
Outskirts logs confirm local probes 10 and 200 replacing the old camera-wide probe-0 choice.

LightSet and the material upload retain four separate diffuse/specular RGB records, with RGB squared and multiplied by W, native enable flags and camera-follow control.
Production retains the existing direct lighting while native material routing and BRDF details remain unrecovered.
The opt-in `SEKIRO_GI_NATIVE_LIGHTS=1` compositor uses the current material BRDF and sun shadow with separate diffuse/specular inputs; its code is excluded from production shaders by a material pipeline define.
The raw Outskirts GPARAM contains local-angle entries only for bank 120, while banks 100/110 enable those lights and author their colours without supplying angles or a bank-zero angle fallback.
Those colours remain retained; this masking was superseded later the same day, when the constructor default directions were recovered (see [Light-set defaults and camera-local slots](#light-set-defaults-and-camera-local-slots-2026-10-15)).
Applying guessed local angles failed the six-view gates; disabling that compositor isolated the regression, and masking missing directions restored the Outskirts gate.
Castle still rejects the experimental compositor at grey views 0/1/2, measuring 117/25/291 against fixture values 31/7/200, so it is not enabled by default.
The default GI and fallback path passes the independent Castle ablation with grey total 396 and unchanged geometry counts.
Native default directions, sharpness, per-light SSAO, ambient-reflection composition and the existing HDR/exposure/tone bridge remain fidelity work; visual-parity item L1 stays open.

`SEKIRO_GI_LEGACY=1` restores the pre-change lookup, hemisphere suppression, camera fallback and direct-light behavior for matched comparisons.
`SEKIRO_GI_FULL_SCAN=1` disables only spatial filtering while preserving the corrected lighting path.
The component controls `SEKIRO_GI_HEMISPHERE=0` and `SEKIRO_GI_LOCAL_FALLBACK=0` support isolated diagnosis without changing source or gate fixtures.
Proof, commands, executable hashes, rejected experiments and the native field census are local to `target/alt-gi/captures/`.

The final proof is in `target/alt-gi/captures/2026-10-15-validated/`, using one executable and runtime switches for before/after.
Default-start Vulkan GPU timestamp samples at 1600 x 900 are:

| Map | Opaque before / after ms | Early prepass before / after ms | Candidates mean / maximum |
| --- | --- | --- | --- |
| Hirata | 5.85 / 3.48 | 1.41 / 1.45 | 5.35 / 39, from 142 |
| Sunken Valley | 3.08 / 1.93 | 0.36 / 0.36 | 2.73 / 18, from 71 |
| Fountainhead | 15.35 / 11.02 | 2.88 / 2.83 | 2.92 / 15, from 49 |

These are short GPU pass observations, not an overall FPS or steady pacing claim.
The filter-only comparison keeps the corrected fallback path active on both sides and switches only `SEKIRO_GI_FULL_SCAN`.
The separate frozen-time Sunken Valley pair is pixel-identical; Hirata and Fountainhead retain small frame differences with mean absolute channel errors 0.0681/255 and 0.00488/255, so exact whole-frame equality is not claimed for those maps.
The ordered-coverage regression and unchanged shader blend arithmetic guard the receiver lighting itself.

Both canonical six-view gates and the independent comparison against the measured before runs pass every per-view tolerance without editing fixtures.
Outskirts totals `(void, depth gap, untextured, grey, pop-in)` change from `(0,493,0,321,24353)` to `(0,493,0,322,24363)`.
Castle totals change from `(1933,4886,903,399,26652)` to `(1933,4886,903,397,26632)`.
Before/after gate durations are 45.71/51.21 seconds for Outskirts and 50.59/54.87 seconds for Castle, below the 120-second budget.
The data gate, 120 game unit tests, six integration tests, the focused probe tests and workspace all-target Clippy pass with `target/alt-gi` and six Cargo jobs.
The owned Rust files and the final workspace formatting check pass.

The night proof retains matched temple/shrine poses from spot-002/spot-003, the checked Castle courtyard camera `(343,-37.68,100,90,0)`, and the accepted great-hall interior pose from spot-044.
The first automatic Castle pose was rejected during image QA and replaced; it is not accepted lighting evidence.
All eight requested-name PNGs are ready under `target/alt-gi/captures/2026-10-15-validated/media/export/`.
Copying them to `D:/sekiro-rs-media/progress/2026-10-15/` was denied by filesystem permissions, so that export remains blocked.
Comparison against the read-only retail pairs confirms L1's hard blue-white illumination, cold cast and dark shrine/hall remain; this optimisation does not close L1 or justify an exposure/tint adjustment.

## Verification

### Shared sky selection correction (2026-10-15)

The read-only m89 measurement found 13 exported dome groups overwritten by later unexported cloud models.
`sky.rs` now retains the first MSB part, as the shared selector does, and stores each exported companion cloud mesh/material separately so both layers change together.
The runtime `SEKIRO_LEGACY_SKY_SELECTION=1` control restores the old mapping, cloud and clock behavior for comparison without changing git state.
`arena_b.rs` no longer derives an environment from a clock-only update, and `sky.rs` retains the authored initial sky until a native sky command replaces it.
Explicit `--hour` remains an intentional diagnostic override.
Missing Senpou m900085 is reported as incomplete sky fidelity and retains the last available dome; no export was changed.

All eight sky regressions and the complete game suite pass: 118 unit tests and six integration tests, with two existing ignored tests.
The CPU data gate passes before and after, and workspace Clippy with denied warnings and formatting pass using `target/alt-sky` and six jobs.
Both canonical six-view render gates currently fail their checked-in grey/pop-in fixtures even with this correction disabled.
No measured category worsens beyond the unchanged 16-pixel/1% per-view tolerance when enabling the sky correction, and all twelve selected collisions and sorted visible-part manifests match.
Geometry totals remain 0/493/0 for Outskirts and 1933/4886/903 for Castle in void/depth-gap/untextured order.
Outskirts grey/pop-in totals are 1047/27440 before and 1025/27443 after; Castle totals are 1163/26379 before and 1166/26389 after.
Those shared colour-baseline failures still block claiming that the required render gates pass; their fixtures were not edited.
Reports, fixed executable/source hashes, camera requests, gate comparisons and matched spot-011/spot-091 captures are under `target/alt-sky/progress/`.
The matched night-sky regions change by at most one display code value: 15/102000 sampled Outskirts pixels and 41/175000 Castle pixels differ.
These static initial-state captures already select m900030 before the correction; the changed group and clock behavior is proven by the eight sky regressions and the independent environment-event regression.
The pale retail sky and readable distant snowy range in the external reference pairs remain F1/D1 work: the protected initial night sky already agrees before/after, so this correction does not establish static sky, fog, material or distant-scenery parity.
Genuine environment changes still require coefficient-atlas synchronization in the GI/material owner's code.

### Shared native fog correction (2026-10-15)

The subsequent [painted-backdrop follow-up](BACKDROP.md) restores each sky group's separate FarMountain companion and its native haze-colour shader.
It also corrects omitted fog sampling defaults from the executable constructor, SAT sky/fog-card RGB conversion, and the fog cards' independent sun specular input.
Exposure, tone mapping and the shared LightSet implementation are outside that follow-up's changes.

The exact global `Enable` lookup no longer matches `EnableAnalyticalFog`, restoring Hirata and Depths without changing authored density.
Fog light references stay as integers and resolve the native map bank plus independent LightSet row, with bank 1 / light 0 and the decoded dummy light as native fallbacks.
The recovered map-bank encoding is `((area + 100) * 100 + block) * 10000` from executable functions `f47270` and `f472a0`.
The current area's bank uses its event-selected GPARAM snapshot; references to another area read that area's base bank, with cross-area event sub-file selection still a fidelity limit.
The bank resolver's time blend rotates resolved directions as in `169080`, `30d730` and `30d3e0`, rather than interpolating identifiers or substituting the collision-selected scene sun.
Global fog retains the verified linear direct/opposite colour mix, byte quantization and gamma 2.2 packing; the local-light `ScatteringG` path remains separate.
Scattering uses the same scene radiance bridge and camera exposure as sky and diffuse lighting.
Native SkyDome forward shaders sample finite mesh depth, so the post pass now intersects the live dome's authored polygon rings instead of assigning every clear-depth pixel 20 km or applying a horizon ramp.
An independent triangle intersection regression checks the supported exported SkyDome meshes, including Hirata and the night variants.
The native solver integrates view-depth slice thickness and writes far-plane scattering colour with alpha zero in its analytical padding layer; continuation now uses that source and the full added optical depth.
Background uploads run after sky swaps without reintroducing fog on the sweep's diagnostic cameras.
Clear pixels outside a supported finite dome use the terminal authored volume; separate transparent-surface depth, local fog sources and shadowed fog remain renderer fidelity work.
Captured inputs, source/binary hashes, before/after images, the review sheet and gate measurements are retained under `target/alt-fog/captures/`.
The follow-up disproves the suspected normal/all-visible fog mismatch: both colour passes reuse one native camera, while only the independent depth and collision cameras strip fog.
An Outskirts view-0 control with fog disabled in both colour renders measures grey/pop-in 5/8707, versus 17/8753 with identical fog uniforms in both renders, retaining void/depth-gap/untextured 0/75/0.
The sweep now rejects a colour pair if any fog uniform or its enabled state changes between screenshots, and logs the complete state for each capture.
Both required six-view gates pass with the manually reviewed fog-sensitive fixtures, in 37.4 seconds for Outskirts and 42.4 seconds for Castle.
Every selected collision, visible-part set and void/depth-gap/untextured count remains unchanged; the complete grey/pop-in fixture review is recorded in MAP-COMPLETENESS-SWEEP.md.
No threshold, camera, visibility fixture, authored density or scattering colour was relaxed to pass the gates.
Matched night captures use the runtime `--no-fog` switch for the before control and the authored fog for after, at the temple idol, Castle courtyard and Outskirts cliff.
The retail temple and Castle reference pairs remain softer and less blue, with weaker distant contrast than our rendering.
The corrected aerial perspective should be assessed separately from the remaining probe/hemisphere, direct-light specular, material-normal, sky-background and absent falling-snow differences; these images do not establish exact retail visual parity.
The cliff control visibly replaces near-black distant rock contrast with grey-blue haze while retaining foreground detail, which moves in the direction of the retail references.
The temple remains excessively blue and crisp at short range, where simply increasing distance fog would obscure geometry instead of correcting its lighting or material response.
The reviewed external pairs include `spot-002-shot-004.png` at the temple, `spot-061-shot-218.png` over a walled courtyard, `spot-070-shot-242.png` among roofs and `spot-133-shot-380.png` above Castle.
The high Castle reference contains pale distant mountain silhouettes that our background still lacks; the approximate sky background and incomplete distant composition remain separate from the restored fog and are not repaired by increasing density.
The relative contribution of the remaining renderer differences to retail's softer look has not been isolated.
The final 1600 x 900 night controls retain these exact `[x,y,z,yaw,pitch]` cameras in both states:

| Capture stem | Camera |
| --- | --- |
| fog-outskirts | `[-138.8885,-49.4547,-154.1603,353.307,-0.535]` |
| fog-castle | `[343,-37.68,100,90,0]` |
| fog-cliff | `[-197.4507,-46.5,150.72162,225,-15]` |

Capture manifests and diagnostic companions are under `target/alt-fog/final/captures/`; before uses explicit `--no-fog`, and capture-time logs verify no GameFog component on either colour pass.
The normal game suite passes 112 unit tests and six integration tests, including the installed-data gate; release all-target Clippy with warnings denied and `cargo fmt --all --check` pass.
All follow-up builds and checks use `CARGO_TARGET_DIR=target/alt-fog` and six Cargo jobs.

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

## Shared capture readiness and presentation, 2026-10-15

`capture_ready.rs` now supplies the sweep, render gates, ordinary map screenshots, play shots, model viewer and arena captures with one pipeline monitor and the sweep's declared-texture checks.
The monitor retains GPU image residency after CPU assets retire, rejects permanent shader failures and starts with an unknown/pending render state.
Window captures require three distinct completed renders without pending pipelines or textures, in addition to their existing scene/rig readiness and map residency settling.
Play keeps due shots queued while waiting, including when the input script ends before compilation finishes.
The interim ordinary map path already waited twelve residency frames and three pipeline polls; it now uses the shared texture-aware checks instead of a profiling probe.

The historical ordinary `--map m13_00_00_00 --screenshot ... --at 0` reproduction is blank apart from UI, while the new capture defers at 18 ready / 97 pending pipelines and saves at 115 ready / zero pending with textures ready.
`target/alt-capture/progress/capture-before.png` uses the executable preserved during the 2026-10-14 session, and `capture-after.png` uses the new implementation.
The current-tree control, `current-tree-before.png`, was already complete with the interim guard; these images establish startup readiness, not colour fidelity between renderer revisions.
The final play smoke test retains its tick-1 shot at 8 ready / 51 pending pipelines and saves `play-ready.png` at 59 ready / zero pending with textures ready.

`presentation.rs` applies `--uncapped` to every window entry point, including the floor scene that previously ignored it, and resolves supported concrete modes before Bevy configures the surface.
It logs backend, requested/configured mode and supported modes, with an explicit warning when an uncapped request must fall back to FIFO.
`--present-mode immediate|mailbox|fifo` provides a diagnostic override, and frame-time logs include only pipeline-ready render intervals.
Vulkan reports supported `[Fifo, FifoRelaxed, Mailbox, Immediate]` and configures Immediate for `--uncapped` on this host.
The warm floor control remains capped: median 16.633 ms before, 16.682 ms with Immediate, and 16.693 ms with explicit Mailbox.
A zero-delay polling control also stayed at 16.684 ms and was removed; a DX12 control was slower under concurrent validation and is not a throughput claim.
Representative Immediate rows have about 2.3 ms main CPU time and a 12.5 ms render/presentation interval.
Sustained throughput above 60 Hz remains unproven and requires further host/backend presentation investigation; individual sub-16.6 ms frames do not establish an uncapped result.

Both six-view gates preserve every per-view void, depth-gap and untextured count exactly against the same-session before executable.
Outskirts passes with totals 0 / 493 / 0, while Castle retains 1933 / 4886 / 903 and its pre-existing grey fixture failures at views 0, 1 and 2.
Castle's before grey values are 115 / 25 / 291 and after values are 117 / 25 / 290, against fixtures 31 / 7 / 200; no fixture was changed.
These colour failures preceded this job in the shared tree containing in-progress lighting work.
Commands, raw logs, CSV summaries, gate reports and image provenance are under `target/alt-capture/progress/`.

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
| m10-default.png | 16.668 | 360 | Earlier VSync-capped review; superseded by the post-material-port Hirata audit below |
| m11_00-default.png | 16.504 | 48 | Dim temple interior with morning light outside; no obvious selection mismatch |
| m11_01-default.png | 16.582 | 120 | Morning exterior light reaches the start; pale bright snow and layered materials need rendering review |
| m11_02-default.png | 16.600 | 94 | Dark prologue passage; no obvious selection mismatch |
| m13-default.png | 16.608 | 120 | Earlier VSync-capped review; superseded by the uncapped m13 audit below |
| m15-default.png | 16.732 | 54 | Dim rocky passage; no obvious selection mismatch |
| m17-default.png | 17.491 | 78 | Morning valley; conspicuous pale terrain/material coverage remains |
| m20-default.png | 16.491 | 120 | Warm entrance interior; no obvious selection mismatch |
| m25-default.png | 20.522 | 81 | Historical VSync-capped review; fixed palace selection confirmed and uncapped follow-up recorded in the Fountainhead notes above |
| castle-top-morning.png | 16.582 | 359 | Rooftop and morning shadow are visible; flat pale fog obscures distant scenery |
| castle-top-evening.png | 16.728 | 360 | Evening shadow and sky differ from morning; flat brown fog obscures distant scenery |

Hirata, the castle horizon and Fountainhead need lighting/rendering follow-up despite matching the selected event variants.
The additional `senpou-illusion.png` capture confirms sub-ID 10 at 12 h and native sky model m900085; that model is absent from the read-only sky export, so the current renderer logs the missing model and retains m900010.
The specific cause of their brightness/fog mismatch is not established by these captures, and tone-curve, exposure and probe-blend calibration remains the other collaborator's work.
Valley and castle snow/foliage problems are visible material issues rather than evidence of a wrong story flag.
The remaining hard provenance gaps are the 09 reservoir family pairing, unavailable event sky exports, nonzero FogParam/LightScattering/EnvMap bank mappings, and an exact first-visit order for optional areas.
Those gaps are explicit rather than covered by invented tuning.

### m15_00 review, 2026-10-13

Common event 100's Depths branch uses sky group 8907015 (model m900015) in every selected clock state.
Morning, afternoon, evening and night select environments 1, 2, 3 and 4 at 6, 12, 18 and 22 h respectively.
World dawn takes the branch's morning fallback, environment 1 at 6 h, rather than the generic environment 5 at 2 h.
All use sub-ID 0 and m15_00_0000; the other numbered families are not selected merely because they exist.
Event 11505290 requests SetLighting(2) after action-button interaction or completion flag 51500050, and does not change the sub-ID or sky.
The ghost Monk's mibu-bridge ArenaSpec has no mandatory earlier time boss and keeps its morning starting state.
The m15-only installed-data regression guards this selection, the four native collision tuples and the m15 ArenaSpec hours, and logs raw GPARAM values alongside LightSet and GameFog conversions.

The default passage, watermill approach and ghost Monk approach resolve to light set 100, with independent fog/scattering/environment IDs zero.
A scripted native village walk near (246.6, -429.5, 1116.4) crosses light sets 100 and 110 and returns to 100, while fog/scattering/environment remain zero; entry and exit screenshots and the selected-row log are retained as boundary captures.
At morning the renderer logs sun and fill at 900 lux, linear light RGB (0.221453, 0.301423, 0.393695), hemisphere mean (0.061111, 0.076336, 0.093253), and environment intensity 1432.3944 cd/m2.
~~The converted hemisphere is not the final active ambient: after camera probe 95 loads, lighting.rs:786-789 sets GlobalAmbientLight.brightness to zero and map_material.rs:638 disables the hemisphere term.~~
The [shared GI correction](#spatial-gi-and-fallback-2026-10-15) retains the authored hemisphere as fallback and replaces it per receiver under native GI coverage.
With zero installed GIIV records, the scene then uses that camera cubemap without the missing native per-receiver coefficient lighting; load the original coefficients before calibrating brightness or probe blending.
Native post settings log exposure 1.3, uploaded multiplier 2.4622889, middle gray -6, adaptation limits -0.3/0.01 and auto exposure disabled.
The morning ID-100 raw sun diffuse colour is (0.470588, 0.549020, 0.627451, 0.2), while its separate SpecColor0 has the same RGB and intensity 0.3.
~~LightSet::from_gparam at lighting.rs:884-885 reads only DiffColor0/1, and the Bevy direct-light path consequently does not preserve the separate authored specular-light control.~~
The [authored map-light correction](#authored-map-lights-2026-10-15) now carries all four slots and their independent specular colours through the default map compositor; material-side fidelity limits remain explicit.
The selected fog upload is None despite authored fixed density 0.007, height density scale 0.007, distribution 0.01, height offset -400 and far depth 650 at ID 100.
The exact shared cause is fog.rs:180-181 asking for "Enable", combined with crates/formats/src/gparam.rs:115 accepting any parameter name containing that substring.
This selects VolumetricFog EnableAnalyticalFog = 0 and incorrectly disables the entire fog pass instead of only analytical continuation.
~~A shared fix should use an exact canonical global-enable name when one exists, otherwise leave volumetric fog enabled, and route EnableAnalyticalFog only to the continuation control.~~
Fixed on 2026-10-15: Depths uploads authored density 0.007 and height density 0.0035 with analytical continuation still disabled; see the shared native fog correction below.
The native m15 regression should then expect a nonempty fog upload with fixed density 0.007 and height density 0.0035 at game scale 1; no map-specific fog gain is justified.
The pale water still has a separate shared material-rendering gap, but absent fog is now a measured rendering error rather than an authored disable.
The map has 50 sized native probe regions and 50 complete GILM pairs in each selected environment, but no exported GIIV planes; the renderer consequently logs zero complete coefficient volumes.
The read-only raw TPF census confirms 200 GIIV planes and 50 GILM cubes in each of the four native environment containers, so this is an incomplete local export rather than an authored absence of GIIV.
The exact data-loading gap is lighting.rs:533-552, which requires four env/giiv<probe>_<variant>_<axis>.rgba8 planes before installing a volume.
The incomplete-volume guard at lighting.rs:580-581 discards all 50 records, leaving map_material.wgsl's gi_lighting loop without native spatial irradiance or per-receiver reflection-region records.
The existing exporter writes those planes in crates/extract/src/maps/env.rs:302-310; regenerate the m15 environment export in a separately authorized cache-writing session, then compare the same views before changing map brightness.
Alternatively, a shared loader can decode the selected raw environment TPF's BC7 coefficient planes into memory using the existing export_volume conversion in crates/extract/src/maps/env.rs:18-40, without writing cache files.
The missing planes establish absent native spatial irradiance, but do not alone prove the entire brightness difference against the original game.
Probe blending, tone curve and exposure calibration remain the collaborator's work; no map-specific compensation was invented.
The pale water and incomplete layered-material rendering remain shared rendering follow-ups described in RUSTSEKIRO-VISUALS.md, without a demonstrated map-specific replacement to borrow.

Review artifacts and uncapped perf logs live under target/alt-m15/captures/.
The before executable is a copied, hash-recorded prior release from target/alt-lf; after captures use the isolated target/alt-m15 release, so concurrent shared renderer changes may affect comparison.
An intermediate isolated build showed only sky and HUD despite roughly 12,800 mesh entities and 88 prepared village draw batches, with an opaque GPU pass of approximately 0.002 ms.
A newer relinked shared-source snapshot restored terrain and actors without an m15 production change.
Those broken captures, logs and executable are preserved under captures/broken-after and excluded from the accepted frame-time CSV; accepted after captures use the hash-recorded restored snapshot.
The host had roughly 30 concurrent rustc processes on an eight-core CPU during the before runs, and recorded wall times cannot establish isolated steady frame pacing.
One before village run saved its screenshot and reached step 600 before Bevy's system_information_diagnostics_plugin.rs:217 panicked on a closed diagnostics channel during shutdown.
A shared dependency fix should tolerate a failed send during application exit; m15 does not own that worker.

All accepted runs request AutoNoVsync through --uncapped and continuous Winit updates, with pipeline-ready rows at simulation ticks 360-600.
Before and after values below are median / p99 wall milliseconds; raw sample counts and CSV paths are in captures/frame-times.csv.
They compare different shared-source snapshots and different host contention, so the improvement is not an m15 lighting or culling optimization.

| m15 state / view | Before median / p99 ms | After median / p99 ms |
| --- | --- | --- |
| morning / default | 234.397 / 2388.324 | 16.229 / 1026.608 |
| afternoon / default | 238.064 / 3526.051 | 16.473 / 656.379 |
| evening / default | 167.133 / 1598.135 | 16.404 / 688.066 |
| night / default | 258.982 / 1612.666 | 16.777 / 1073.596 |
| dawn / default | 199.213 / 1401.975 | 16.313 / 786.834 |
| morning / village | 128.434 / 1863.534 | 16.645 / 613.017 |
| afternoon / village | 120.255 / 1851.422 | 16.569 / 508.588 |
| evening / village | 108.024 / 1781.755 | 16.597 / 570.302 |
| night / village | 154.831 / 1518.190 | 17.314 / 760.845 |
| dawn / village | 323.623 / 3836.203 | 16.552 / 571.434 |
| mibu-bridge ArenaSpec | 180.666 / 2152.951 | 16.672 / 518.072 |

The after village GPU samples put the opaque pass around 1.8-1.9 ms, early prepass around 1.0 ms and native post around 0.08 ms.
Cloth simulation samples are usually 1.2-1.7 ms, with one default-view sample at 4.124 ms; raw CPU-system and GPU-pass samples are in profile-costs.csv.
Despite the uncapped request, after medians still cluster near 16.6 ms; a representative row has main CPU 4.911 ms, fixed CPU 1.226 ms and render-schedule CPU 16.176 ms, including an 11.4 ms render-stage interval.
Effective surface present mode is not logged, and the 16.6 ms plateau plus large p99 spikes prevent a claim of steady uncapped frame time.
A shared capture fix should ~~log the effective present mode/backend and~~ offer an offscreen timestamp control to exclude presentation waits; add_render_probe is also registered by both --profile-systems and PLAY_LOG_PERF, producing duplicate render-set stamps in these CSVs.
The mode/backend logging item is fixed by `presentation.rs`; offscreen timing isolation and duplicate profiler installation remain separate follow-ups.
No map-specific culling cost was demonstrated by these captures.
The final after boundary run repeatedly respawns instead of completing the before walk, so live after-state region movement remains unverified; its command, spawn, step log and screenshots are retained rather than presented as a passing walk.
The native m15 data test passes, the collision-system regression passes, and the final combined lighting filter passes 11 tests with two slow tests ignored.
The separate shared selector filter passes six tests with one slow test ignored; final sekiro-game all-target Clippy with warnings denied and workspace formatting checks pass.
Transient m25 assertion and shared compilation errors resolved in later shared edits; no other map or shared code was edited in this review.

## Abandoned Dungeon audit (2026-10-13)

Only `lighting/maps/m13.rs` and this map's documentation are changed by this audit.
The profile remains unchanged because no incorrect story flag, sub-ID or sky selection has been established.
The installed-data regression logs the native LightSet, fog and exposure rows alongside `LightSet::from_gparam`, including each collision ID and whether it actually has an authored row.
There is no demonstrated full-map Dungeon fix to borrow in `RUSTSEKIRO-VISUALS.md:106-114`, so no old exposure offsets or guessed brightness multiplier are introduced.

| Story | Flags | Hour / environment | GPARAM / sub-ID | Installed sky model | Selection result |
| --- | --- | --- | --- | --- | --- |
| morning | none | 6 / 1 | m13_00_0000 / 0 | m900000 | Matches common event 100 |
| afternoon | 9800 | 12 / 2 | m13_00_0000 / 0 | m900010 | Matches common event 100 |
| evening | 9800, 9801 | 18 / 3 | m13_00_0000 / 0 | m900020 | Matches common event 100 |
| night | 9800, 9801, 9802 | 22 / 4 | m13_00_0000 / 0 | m900030 | Matches common event 100 |
| dawn | 9800, 9801, 9802, 9803 | 2 / 5 | m13_00_0000 / 0 | m900050 | Matches common event 100 |

The six environment variants have complete probe-0 pairs, with distinct specular DDS hashes rather than a repeated nearest-hour substitute.
Measured RGBA16F diffuse probe-0 mean luminance is 0.001671/0.001599/0.001682/0.001646/0.001679 for morning/afternoon/evening/night/dawn, or approximately 2.29-2.41 after the renderer's 1432.3944 multiplier.
This is a cubemap texel average, not a measurement of the final per-receiver GI contribution.
The default start is `[369.498, -43.974, 285.327]`, on LightSet 110; start 1 provides the wider pool/cage view.
The exported 46 collision parts comprise 40 with LightSet 110, one with 0, one with 100 and four with 130; installed fog, scattering and environment IDs are zero independently.
The only LightSet 0 part, `h025500`, is a small elevator-area vertical wall at approximately `[319.9, -42.6, 393.6]`, comprising two triangles with no horizontal surface, rather than a broad outdoor terrain region.
The LightSet 100/130 parts are at Y -265 through -373, hundreds of metres below the normal starts.
An ordinary Dungeon walk must not be described as crossing a large exterior morning-light region or a registered boss arena: there is currently no m13 `ArenaSpec` in the arena catalog.
The shared arena-selection regression remains the check that existing arenas retain their `ArenaSpec.story_time`.
The scripted elevator-area walk moved from `[319.9, -42.1, 393.55]` to approximately `[322.83, -43, 400.05]` and retained LightSet 110 after the initial row-zero setup.
The grounded player correctly did not select the vertical LightSet 0 wall through the floor-directed ray; the shared moving-focus regression separately exercises independent ID switching, including zero resets with draw-group culling off.
Two nearby free-camera captures also retain LightSet 110, and the direct native MSB audit confirms that all 46 exported collision LightSet IDs agree with the installed collision parts.
Its command, input script, two screenshots and runtime log are retained as `region-walk*`, `walk.txt`, `region-before.png` and `region-after.png` under the capture directory.

At the morning start, authored row 110 has sun RGBW `[0,0,0,0]` and fill RGBW `[0.705882,0.705882,0.705882,0.05]`.
The renderer logs zero sun and 225 lux fill, consistent with gamma-2 colour conversion and the existing 4500-unit scene scale.
Inherited hemisphere mean is `[0.046136,0.066436,0.103806]`, delta `[0.030757,0.044291,0.069204]`, and brightness 1432.3944 before probe application.
The native tone inputs and runtime upload agree: exposure 2.4 gives gain 5.278032, middle grey -6.8, adaptation limits -0.5/0.01, auto exposure enabled and mode 7.
Fog upload agrees with the authored global row: density `[0.04,0.2,0.5,350]`, depths `[0.01,200,0,0]`, dark sky RGB `[0.001963,0.002586,0.002586]`, zero directional/opposite RGB and extinction `[1,1,1]`.
The dark fog colour and zero cave sun are authored inputs, so brightening those values would change the game data.
The no-probe diagnostic increases the same scene-region mean display-code luminance from 0.123626 to 0.228839, while disabling fog reduces it to 0.115417 and reveals the background stairs.
These measurements use PNG RGB code values in the fixed rectangle X 200-1399, Y 150-699, avoiding the main HUD bands; they are not linear HDR luminance or native-game reference measurements.
This attributes much of the rendered darkness to the indirect/probe path rather than to the selected hour or fog extinction alone, without establishing which indirect composite is faithful.
The 36 exported BTL lights contribute no local-light candidates at either start: the nearest is 483.207 m from start 0 and 597.399 m from start 1, outside the shared 60 m plus authored-range activation rule.
The logs load the bank but never activate a light at those starts.
`extract/src/maps/lights.rs:26` exports reflected BTL positions, and `game/src/map.rs:1302,1340,1372` loads, culls and spawns them directly.
The shared lighting/export investigation should validate the BTL coordinate frame and authored placement against MSB/native rendering, and apply a recovered placement transform if one is required; this evidence alone does not justify an invented map offset or a claim that a light must cover these starts.

Shared follow-ups, deliberately left unedited:

- ~~`lighting.rs:786-788`, `apply_probes`, sets global ambient brightness to zero when a camera probe is ready; `map_material.rs:466-485`, `update_hemisphere`, then sets the hemisphere enable bit to zero, and `map_material.wgsl:555-567` skips the authored hemisphere contribution.~~
  Fixed by the [per-receiver hemisphere fallback](#spatial-gi-and-fallback-2026-10-15), without an unconditional ambient boost.
  Runtime evidence is the nonzero LightSet 110 hemisphere followed by `environment probe Some(0)` at the same start.
  This proves the renderer suppresses those inputs, but does not prove that adding them over native GI would match the original compositing rule.
  Have the probe-blending collaborator validate the native GI/hemisphere composition and replace the global camera-ready gate with the recovered per-receiver rule; do not simply add a global ambient boost.
- `formats/src/gparam.rs:69-72` silently inherits row zero for collision LightSet 130, because the selected file has LightSet rows 0, 100, 110 and 120, but no 130.
  Preserve the native collision ID and report the fallback explicitly in the shared resolver; trace the native missing-bank behavior before introducing any mapping to 120.
  This does not explain the default-start darkness, since that start selects the authored 110 row.
- `play.rs:666` invokes `perf_log::register!`, whose `perf_log.rs:327-329` installs a render probe; `map.rs:549-557` installs another probe for `--profile-systems`.
  The captures therefore show 15 render-set timing intervals for a seven-stage stamp sequence.
  Make probe installation idempotent and reuse one resource before relying on these CPU render-set labels; wall-frame CSV timing and separate GPU timestamps remain usable.
- `map.rs:554` enables Bevy `SystemInformationDiagnosticsPlugin` during profiling.
  Several completed captures panic on shutdown at Bevy 0.19.1 `system_information_diagnostics_plugin.rs:217` with `SendError` after the screenshot is saved.
  Avoid installing this asynchronous system-information sampler for short render profiling, or adopt its upstream shutdown fix when available.

Exact original-game image equivalence remains unverified because no matched original-game captures were available for these five views.
Tone curve, auto exposure and probe-blending calibration remain with the other collaborator's project.
No rendering material, transparency, depth, motion-vector or MSAA behavior is changed here.

Before/after PNGs, commands, native numeric dumps and performance CSVs are under `target/alt-m13/captures/`.
The five default-start pairs are named `before-<story>-start0.png` and `after-<story>-start0.png`; the wider pool-view pair is `before-morning-start1.png` and `after-morning-start1.png`.
Both passes use the same immutable review executable, whose SHA-256 is recorded in `baseline-binary.json`, because this audit changes tests and documentation rather than runtime rendering.
These comparisons therefore demonstrate the remaining problem, not a claimed rendering improvement from a map-specific patch.
The reusable capture command includes `--uncapped`, which sets `AutoNoVsync` in the map loader, and renders 900 simulation ticks with its screenshot requested at tick 840.
`PLAY_PERF_DIR` points inside the capture directory for every run.
The first short attempts with pending pipelines are not used in the following table.
The statistics use the latest CSV for each correctly named run, simulation ticks 360-840 and `x_pipelines_waiting == 0`; p99 is the empirical nearest-rank quantile, with small samples disclosed.

| View | Before median / p99 ms | After median / p99 ms | Accepted frame rows before / after |
| --- | --- | --- | --- |
| morning, start 0 | 209.921 / 479.318 | 156.408 / 305.637 | 18 / 48 |
| afternoon, start 0 | 260.090 / 525.061 | 209.397 / 333.012 | 21 / 20 |
| evening, start 0 | 253.491 / 614.788 | 174.417 / 395.433 | 21 / 33 |
| night, start 0 | 174.198 / 330.290 | 152.103 / 537.190 | 31 / 47 |
| dawn, start 0 | 179.318 / 412.803 | 157.163 / 380.046 | 24 / 47 |
| morning, start 1, wider pool/cage view | 107.144 / 209.299 | 104.724 / 232.782 | 48 / 59 |

These wall times are not steady, and their difference must not be attributed to the unchanged map profile.
Concurrent compilation counted 46 Cargo/rustc processes early in the session and 35 later, with other map runs also using the shared GPU.
Warm GPU timestamp medians for the final three diagnostic samples were approximately 1.58/1.63 ms for the start-0 opaque pass before/after and 0.692/0.689 ms for fog.
At the wider pool view they were approximately 1.58/1.33 ms opaque and 0.891/0.885 ms fog.
Other story captures showed contention-dependent opaque costs up to 6.63 ms and fog up to 1.47 ms.
The last CPU profile batch's largest named system was cloth simulation, about 1.41 ms at the default start and 6.86/4.09 ms before/after at the pool; `pick_groups` was at most about 0.19 ms in those batches.
Cloth, simulation scheduling and the duplicate shared profiler are shared costs; no map-specific culling defect was established by these runs.
An isolated uncapped capture after compilation and the shared profiler fix is still required before claiming a steady frame time or diagnosing a map rendering regression.
Earlier checks caught shared `map_material.rs:356` cloning a `Copy` attribute, a formatting difference in shared `map.rs:1700`, and an unrelated m25 native-flag assertion.
Their owners resolved those issues while this audit was running; this map agent did not edit those files.
Final game-package/all-target Clippy with warnings denied and `cargo fmt --all --check` both pass, using target `alt-m13` and four Cargo jobs.
Only `lighting/maps/m13.rs` was formatted by this agent.
The installed m13 regression passes, the moving-focus/all-four-IDs regression passes, and the complete lighting-selection filter passes 11 tests with two slow tests ignored.
The m13 event audit uses the full installed filename `m13_00_00_00.emevd` and finds no lighting, environment, sub-ID or map-part overrides to add to this map's five clock presets.

The first new-build smoke capture is rejected: it shows only sky/UI while 42 render pipelines remain pending, so it is not a lighting or performance comparison.
After the ongoing shared changes, `current-validated-morning-start0.png` and `current-validated-morning-start1.png` render complete geometry with 147 ready pipelines and zero pending pipelines.
Their additional uncapped measurements use the same tick range and pipeline-ready filter as the comparison table:

| Latest validated build view | Median / p99 ms | Accepted rows | GPU opaque / fog ms |
| --- | --- | --- | --- |
| morning, start 0 | 24.827 / 40.306 | 314 | 0.606 / 0.973 |
| morning, start 1 | 29.493 / 40.454 | 279 | 0.660 / 1.480 |

These later results include other agents' shared changes and different machine contention, so their improvement is not credited to an unchanged m13 runtime profile.
Median main/fixed CPU-frame buckets are 23.944/12.226 ms at start 0 and 28.662/14.968 ms at start 1, while the largest last-batch named system is cloth simulation at about 1.3 ms.
The gap requires shared simulation/camera/scheduling profiling; the duplicated render-stage intervals must be repaired before assigning that unmeasured CPU time to a render stage.
The native indirect-light composition and matched original-game comparison remain unresolved, and the measured median/p99 gap still does not establish a steady frame time.

## Reservoir audit (m11_02, 2026-10-13)

The map-local installed-data regression checks all nine presets, the four native collision configurations, and the Reservoir boss arenas without changing their `ArenaSpec` hours or flags.
No map-specific brightness multiplier, sky substitution, material override or static culling crop was added.
The existing selection metadata agrees with the event instructions and catalog at the level established below; exact rendered equivalence remains blocked by shared renderer issues and incomplete family provenance.

| Reservoir state | Hour / environment | GPARAM / sub-ID | Event-selected sky | Selection conclusion |
| --- | --- | --- | --- | --- |
| prologue | 0 / 0 | m11_02_0000 / 0 | m900040 | Matches common event 100 with 8306 and retained 9810 |
| morning | 6 / 1 | m11_02_0000 / 0 | m900000 | Matches common event 100 |
| afternoon | 12 / 2 | m11_02_0000 / 0 | m900010 | Matches common event 100 |
| evening | 18 / 3 | m11_02_0000 / 0 | m900020 | Matches common event 100 |
| night | 22 / 4 | m11_02_0000 / 0 | m900030 | Matches common event 100 |
| dawn | 2 / 5 | m11_02_0000 / 0 | m900050 | Matches common event 100 |
| invasion | 22 / 4 | m11_02_0020 / 20 | m900095 | Matches common fire branch with 8302 |
| sword-saint-defeated | 22 / 4 | m11_02_0920 / 20 | m900095 | Event 11125860 instruction 19 requests 20 over five seconds; family 09 companion remains inferred |
| sword-saint-storm | 22 / 4 | m11_02_0930 / 30 | m900095 | Event 11125872 instruction 12 requests 30 over five seconds; 0930 is the sole installed Reservoir sub-ID-30 file |

The catalog also contains 0900 and other numbered families, and the area-sub-ID instruction does not encode a family.
The paired 0920 choice is therefore retained as an explicit inference rather than claimed as recovered engine behavior.
The native collision tuples are `[0,0,0,0]`, `[100,0,0,0]`, `[110,0,0,0]`, and `[120,0,0,0]`, with respectively 10, 45, 16 and one collision parts in the layout.
These zero collision bank IDs do not establish the meaning of the distinct nonzero GPARAM `VolumetricFog DirLight LightBankId` references.
That reference is 111020000 in the selected Reservoir files, including both 09 boss files.
~~The engine `(LightBankId, LightId)` resolver remains unrecovered as recorded in `SHADER-INPUTS.md`; neither the decimal encoding nor a substitute file lookup is asserted here.~~
The 2026-10-15 static recovery decodes native map banks and their independent LightSet row IDs, including the native fallback and direction blending.

At default start 0, row 110 has authored sun RGBA `(0.60062,0.675698,0.750775,2)` and fill RGBA `(0.74,0.86381,1,0.25)` with zero up/down hemisphere values.
`lighting.rs:858-915,958-993` converts these to linear sun RGB `(0.360744,0.456568,0.563663)` at 9000 lux and fill RGB `(0.5476,0.746168,1)` at 1125 lux, and the runtime log confirms those inputs.
Prologue fog has GameScale 12, fixed density 0, height distribution 0.3, height offset -100, and height density scale 0.075; its upload is `(0,0.025,0.003125,100)` with near/far depths 0.12/2400 metres.
Exposure 0 becomes multiplier 1, auto exposure is enabled, middle gray is -3, and adaptation limits are -0.4/8, matching the raw selected values and logged `GamePost` upload.
The final comparison in `captures/input-comparison.txt` finds an exact match between the converted row-110 `LightSet` and the runtime light input for all nine states.
Those uploads establish input agreement, not the correctness of tone-curve or auto-exposure calibration, which belongs to the other collaborator.

The shared probe fallback at `lighting.rs:746-763` chooses the largest available box before calling the position-dependent selector.
The start is inside boxes 0, 1, 10 and 11, with volumes 125000000000, 11050000, 189182 and 30618 cubic metres respectively, but every captured story state selects probe 0.
The exported Reservoir has zero GIIV coefficient files, so no spatial coefficient record corrects this global fallback.
The read-only native-container census finds 100 textures, including 80 GIIV axis textures (20 complete volumes), in each of the six installed Reservoir environment TPFs.
This establishes a missing derived-data export rather than an absence of authored spatial lighting in the game.
The current exporter already discovers these containers and writes the coefficient files at `crates/extract/src/maps/env.rs:278-311`; the required repair is to regenerate the Reservoir environment export and verify all 20 volumes for each environment.
That repair cannot be performed in this task because writing under `cache/` is explicitly forbidden.
Prologue diffuse-cube mean luminance is 0.04732070 for probe 0 and 0.03480409 for probe 10; global-probe selection is not evidence that increasing ambient intensity would restore fidelity.
~~Proposed shared fix: when complete spatial GI records are unavailable, use containing local boxes and the existing position-dependent selection rather than unconditionally using the largest box; preserve the current spatial GI path where complete records exist.~~
Implemented by the [shared fallback selection](#spatial-gi-and-fallback-2026-10-15), while retaining spatial GI blending.
Probe blending and its calibration remain with the collaborator handling that system.

~~At `fog.rs:284-291,216-217`, fog direction is the current selected file/row sun, and `LightBankId` is ignored.~~
For example, base night row 0 has direction `(0.433013,0.500000,0.750000)`, whereas both selected 09 boss files yield `(0.972957,0.211886,-0.091971)` despite retaining the same authored fog bank reference.
~~Proposed shared fix: recover and implement the bank/light resolver with integer identifiers, then supply its resolved direction to the existing fog formula instead of substituting the active scene sun.~~
Implemented in the shared native fog correction, with integer references and independently resolved bank/light directions.
The bright pale ground in the heavy storm view also reaches the shared approximate layer shader at `map_material.wgsl:242-247,284-293`, which uses one height channel and a hard-coded normal-facing snow weight rather than recovered material-family inputs.
The capture demonstrates a material fidelity gap, but does not isolate its contribution from exposure; the proposed follow-up is the native per-family mask/UV/weight path described in `RUSTSEKIRO-VISUALS.md`, not a Reservoir tint adjustment.
That audit has no demonstrated full-map Reservoir darkness fix to copy, so no new RustSekiro code or credit entry was added.

Captures, commands, raw parameter/test logs, profiler output and CSVs are under `target/alt-m11_02/captures/` and its parent directory.
The before executable was copied from the previously built baseline into this isolated target; the after executable was rebuilt from the shared working tree, so unrelated concurrent renderer changes prevent attributing visual or timing differences to this map's test-only source change.
The initial sky-only morning image was rejected and replaced with a longer-warmed capture.
All captures use `--uncapped`, which selects `AutoNoVsync` and continuous Winit updates at `map.rs:563-570`.
Timing samples exclude pending pipelines and screenshot readback, using the 150-tick interval ending 30 ticks before the recorded screenshot command.
Small sample counts and 100% host CPU usage make these contention observations, not evidence of steady frame time.
With `PLAY_LOG_PERF=1` and `--profile-systems` together, `map.rs:556-558` and `perf_log.rs:326-329` install render probes twice, so the duplicated stage stamps must not be treated as an unambiguous per-stage GPU breakdown.
Proposed shared instrumentation fix: install one render probe and reuse it for both consumers.
Per-pass GPU diagnostics remain separately identified by their named `elapsed_gpu` paths.
The profiled night run also logs a shutdown-only worker panic at Bevy 0.19.1 `system_information_diagnostics_plugin.rs:217:56`, where a background send unwraps `SendError` after the app receiver is gone.
The screenshot and measured interval precede this panic; a shared follow-up should remove the unnecessary system-information plugin from short capture runs or make the worker tolerate receiver shutdown.

| Reservoir uncapped view | Before median / p99 ms | After median / p99 ms | Ready-pipeline samples before / after |
| --- | --- | --- | --- |
| Default prologue start 0 | 299.567 / 397.812 | 175.889 / 305.877 | 10 / 14 |
| Heavy storm exterior, spawn 396,-44.3,183,90,0 | 173.143 / 321.931 | 316.523 / 694.636 | 15 / 12 |

The default after-run named GPU measurements are approximately 1.283 ms opaque, 0.847 ms fog, and 0.106 ms bin unpacking; the heavy after-run opaque measurement is approximately 3.733 ms and its authored storm fog is disabled.
Cloth CPU simulation varies from 1.873 ms to 13.156 ms mean in consecutive heavy-view reporting intervals, with a 44.616 ms maximum under contention.
No map-specific culling change is justified by those measurements, and no steady-frame-time claim is made.
The source change adds verification only; the apparent before/after timing differences are not an optimization result.
All nine before/after state PNG pairs, both heavy-view PNGs, and their contact sheets are present in the capture directory.
The outdoor walking replays produced movement and entry/out/return captures but stayed in light row 100 after initial selection, so a real indoor/outdoor walking transition is not established by these replays.
The additional well replay moved from `(304.79,-53.85,305.30)` to `(289.90,-49.82,275.80)` with jumps and remained in row 110; its final capture shows the route stopped against a wall.
The installed configurations and shared collision-system regression verify independent selection and reset behavior, but they do not substitute for that remaining E2E proof.
Final validation passes the two Reservoir tests including the ignored native-container census, six shared selector tests with one slow census ignored, and the independent collision-selection regression.
The earlier broad selector filter also ran other agents' map tests and recorded failures in m13, m15 and m25; those files were left untouched.
The final broad selector filter records nine passes, two failures and three ignored tests: m13 calls the selector with short map ID `m13_00`, producing a missing `raw/event/m13_00.emevd` error at `lighting/maps/m13.rs:44`, and m25's native Dragon flag assertion fails at `lighting/maps/m25.rs:37`.
Those are reported to their owners through this audit rather than changed in the Reservoir task.
An earlier game/all-target Clippy check passed, but the final rerun after concurrent shared edits fails at `map_material.rs:356:42` with `clone_on_copy` on `layout.0.contains(a.clone())`; the proposed shared fix is `layout.0.contains(a)` because `MeshVertexAttribute` is `Copy`.
The shared file was left untouched as required.
`cargo fmt --all --check` passes, and all commands use `CARGO_TARGET_DIR=target/alt-m11_02` with four jobs for builds/tests/Clippy.

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

## m11_00 audit (2026-10-14)

The installed-data regression in `lighting/maps/m11_00.rs` verifies all seven selectable states, four collision tuples and both boss defaults.
The existing profile needed no production flag, file or brightness change.
Family 00 is present for every requested sub-ID and contains the collision banks; selection follows the shared family-00 resolver, without a new independent recovery of the executable's family-selection mechanism.
The complete local event/GPARAM/MSB evidence is under `target/alt-m11_00/captures/`, including `raw-gparam.json`, `native-msb.json`, `event-instructions.json` and `selection-tests.txt`.

| State | Hour / environment | File / sub-ID | Native dome | Selection result |
| --- | --- | --- | --- | --- |
| morning | 6 / 1 | m11_00_0000 / 0 | m900000 | Matches installed event 100 |
| afternoon | 12 / 2 | m11_00_0000 / 0 | m900010 | Matches installed event 100 |
| evening | 18 / 3 | m11_00_0000 / 0 | m900020 | Matches installed event 100 |
| night | 22 / 4 | m11_00_0000 / 0 | m900030 | Matches installed event 100 |
| dawn | 2 / 5 | m11_00_0000 / 0 | m900050 | Matches installed event 100 |
| invasion | 22 / 4 | m11_00_0020 / 20 | m900095 | Matches event 100's 8302 branch; fire dome is exported |
| tomoe | 18 / 1 | m11_00_0030 / 30 | m900080 | Matches castle event 11115820 instructions 24, 26 and 28; SetLighting changes the clock without changing the environment |

Gyoubu retains ArenaSpec's morning, environment 1 and sub-ID 0; Demon retains night, environment 4, flag 8302 and sub-ID 20.
Default start 0 stands on h022400 / LightSet 110, and gate start 8 stands on h017700 / LightSet 100.
The successful `doorway-center` gameplay replay starts at player start 1, walks through the entrance and returns outside, logging 100 -> 110 -> 100.
Native collision fog, scattering and environment IDs are all zero; the independent-field collision regression also passes with culling disabled.

The morning bank-110 upload agrees with the GPARAM: sun RGB (1, .745098, .627451), W 2 becomes linear (1, .555171, .393695) at 9000 lux; fill W .5 becomes 2250 lux.
The hemisphere mean is (.046136, .066436, .103806), with the renderer's common 1432.3944-unit ambient bridge.
Fog uploads density .00080000004 and sky (.157281, .246800, .409826); post uploads exposure 2^1.4 = 2.639016, middle gray -6.8 and adaptation limits -.5 / 2.
Raw and converted values for every bank and story state are retained in the regression output and capture logs.
The settled temple remains dim; matching selected inputs does not establish visual equivalence to the original game.

Shared renderer handoff:

- ~~`lighting.rs:880` consumes two DiffColor lights~~; the [shared correction](#spatial-gi-and-fallback-2026-10-15) retains all four diffuse/specular records and enables, and the constructor default local directions were recovered later that day.
  Production direct-light composition remains unchanged because that compositor fails the Castle gate.
  `SHADER-INPUTS.md:229-242` also records native sharpness and per-light SSAO weights, which remain fidelity work.
  Morning bank 110 enables LocalLight1 with RGB (.294118, .392157, .490196), W .7, but that term is not uploaded.
  Sun specular W is 1.5 versus diffuse W 2; the single Bevy light colour/intensity collapses that distinction.
  Implement the recovered four-light record and independent diffuse/specular terms in the shared renderer, retaining explicit limits for unrecovered angle defaults and camera-follow behavior rather than compensating with map brightness.
- ~~`sky.rs:349` overwrites a group's dome with its later cloud model; group 8907080 maps first to m900080 and later to m900180, and group 8907100 maps first to m900095 and later to m900200.~~
  ~~The cloud names are absent from the exported dome library, so `sky.rs:465` can discard runtime event selection and fall back to the ordinary clock sky.~~
  ~~Resolve each group to its exported dome as the shared selector does, preserving its cloud association separately.~~
  Corrected 2026-10-15 for all 13 exported dome groups, with cloud mesh/material swaps tied to the selected export.
- ~~`play.rs:1100` waits for asset readiness and scene settling, and `play.rs:1738` captures by simulation step without waiting for pipeline readiness.~~
  The discarded five-second image showed only clear colour/UI; its log still had pending pipelines, and the first performance run ended with 83 pipelines pending.
  ~~Add pipeline-ready capture/performance gating in the shared harness; do not report cold blank frames as lighting or steady rendering results.~~
  Fixed by the shared texture/pipeline monitor and ready-frame timing logs described above.
- `map.rs:567` requests AutoNoVsync, which Bevy can fall back to FIFO when unsupported.
  Default, gate and the 800x600 control with shadows/probes/GI/fog/bloom disabled all remained near 16.7 ms.
  ~~Log the actual configured swapchain mode and supported modes, and provide an explicit supported Immediate mode before initial surface configuration~~; investigate driver/presentation limiting before attributing the remaining wait to map geometry.
  Mode selection and logging are fixed; the measured 60 Hz plateau remains open.

Disabling auto exposure changes the uploaded enable bit from 1 to 0 and lowers the sampled display luma from 57.73 to 55.75 rather than curing the dark room.
Disabling probes lowers it to 52.17, and disabling GIIV lowers it to 56.64.
These measurements use 9065 fixed image samples in the lower scene area, excluding the top HUD; they are display-space diagnostics, not recovered scene radiance.
Tone curve, auto exposure, probe blending and the inferred scene-unit bridge remain with the collaborator's renderer work.
No exposure multiplier or unconditional hemisphere restoration was applied.
RustSekiro's documented gate shadow/LOD improvements are useful shared rendering/extraction leads, but its notes contain no verified full-map Outskirts fix to copy into this profile.

### m10_00 Hirata audit, 2026-10-14

Both memory profiles select common event 100's environment 0, sky group 8907060/model m900060 and sub-ID 0/file m10_00_0000.
The default omits father-memory flag 8304 and is the first Hirata memory, independently of the world clock.
All entries in the selected native file have hour 0 and IDs 0, 100, 110 or 120; the other numbered families are not selected merely because they exist.
Map event 11005610's interaction/cinematic clock changes and Butterfly event 11005640 do not supply another environment or sub-ID.
The m10-only regression checks both profiles, the native collision tuples and the Hirata arenas' retained ArenaSpec hours, prerequisite flags and BossEvents flags.
The veranda walk near (55.6, -765.15, 455.3) changes LightSet 110 to outdoor 100; native fog/scattering/environment banks remain independently zero.
Both after-profile exits pass, with a brief authored row-0 selection during the edge drop before returning to 100, retained in the logs.
The attempted start-6 hall walks hit walls and are retained as diagnostics, and the original walking veranda return reaches lower ground rather than proving re-entry.
The final reentry.txt controls walk off, settle and jump back onto the veranda in both profiles, restoring LightSet 110 at (53.89, -765.20, 451.60), with landed endpoint PNGs and the full 110-to-100-to-110 log sequence retained.
The transient authored row-0 samples during drop/jump are documented; this verifies both lighting endpoints without claiming the ledge is a walking doorway.

At the default start, raw sun and fill RGB/W (0.588235, 0.686275, 0.823529, 0.4) become linear RGB (0.346020, 0.470973, 0.678200) at 1800 lux each.
Indoor row 110 retains the sun but changes fill to linear RGB (0.153787, 0.186083, 0.221453) at 900 lux; row 120 uses 1350 lux and row 0 disables fill.
The native hemisphere mean converts to (0.059054, 0.092272, 0.144175), ~~before the shared probe path disables global ambient and the hemisphere term~~ and is now retained as [per-receiver fallback](#spatial-gi-and-fallback-2026-10-15).
The re-export supplies 142 complete GIIV volumes for variant 0 in a 100 MiB atlas, and the memory sky is present.
Exposure 1.2 uploads as gain 2.297397, middle gray -6, adaptation limits -0.2/2 and LUT 20; the absent queried enable field makes auto exposure false.
Recovering the native exposure enable/default, tone curve and probe composition remains the RustSekiro collaborator's work.
The pale cyan wet start and very dark hall remain visible, but a per-map warming or exposure multiplier would not follow the measured game data.

~~The confirmed shared fog error is fog.rs:180-181 querying "Enable" through formats/src/gparam.rs:115's substring matcher, which resolves to EnableAnalyticalFog=0 and disables the entire pass.~~
Native fixed density 0.0032, height distribution 0.13, height offset -840, height density scale 0.015 and far depth 650 are present despite the logged fog upload being None.
~~Use an exact global-enable field when present, otherwise retain volumetric fog, and use EnableAnalyticalFog solely for continuation; the existing conversion should produce density (0.0032, 0.13, 0.0075, 840) and far depth 650.~~
Fixed on 2026-10-15: Hirata now uploads those authored values and retains its disabled analytical continuation.
A second shared omission is lighting.rs:881-922 retaining only sun/fill while native LightSet 100 enables LocalLight1 diffuse/specular RGB (0.274510, 0.392157, 0.490196), W 1, and LocalLight2 specular W 0.5, both with sharpness 0.15.
Carry the authored local directional fields, enable flags and separate diffuse/specular behavior through shared rendering instead of adding an invented map fill; these differ from the existing BTL point lights.

The current GLBs have 614 models, 2473 materials, 746 material extras and no missing declared image paths, but 23 textureless materials include m209000's M[m10_00]_River_Foam1.
The shared exporter fallback at extract/src/maps/pieces.rs:377,396-397 makes that effect grey (0.45, 0.43, 0.40) and roughness 0.85; recover its native transparent effect family instead of correcting it through lighting.
Some textureless records are dummy materials, and omitted original references do not appear as missing declared images, so this does not establish that every pale surface lacks an albedo.
The RustSekiro material port and its existing credit are retained; its visual notes supply no independently validated m10 tint correction to copy.

~~The shared GI shader scans all 142 volume records per fragment at map_material.wgsl:84; spatial candidate filtering must preserve native coverage, ordering and blending.~~
Fixed by the [shared spatial candidates](#spatial-gi-and-fallback-2026-10-15), retaining the existing receiver coverage and blend order.
The courtyard viewer submits all 5388 pieces when h015610 has zero draw groups, whereas the default viewer submits 4161; map.rs:1161 leaves an initial zero group without an active visibility mask.
Recover native zero-draw/display-group behavior before changing that shared culling fallback; no geometry was hidden without evidence.
All captures, selected/raw parameter measurements, commands, hashes, raw CSVs, material census and the exact shared-fix proposals are under target/alt-m10/captures/REPORT.md.
Uncapped runs request AutoNoVsync and continuous Winit updates, and separate GPU profiles avoid the double-probe combination of PLAY_LOG_PERF and --profile-systems.
The baseline default GPU samples are opaque 5.65 ms and prepass 1.89 ms; the broader courtyard samples are opaque 6.67 ms and prepass 0.71 ms.
At least 14-16 rustc processes were active during baseline captures, so these shared-machine wall intervals cannot demonstrate steady frame time or an optimization speedup.
After captures observed eight rustc processes, and the lower wall medians have no accompanying m10 runtime optimization.
The after default GPU samples are opaque 5.59 ms and prepass 1.89 ms; the broader courtyard samples are opaque 7.33 ms and prepass 0.65 ms.
Ready-pipeline wall summaries use simulation ticks 360-600 and exclude 470-510 around screenshot readback; all raw rows and sample counts are in the local frame-times.csv.

| Hirata view | Before median / p99 ms | After median / p99 ms | Before / after samples |
| --- | --- | --- | --- |
| memory, default start 0 | 102.901 / 177.724 | 19.012 / 89.452 | 33 / 124 |
| father-memory, default start 0 | 59.641 / 154.970 | 16.473 / 217.449 | 48 / 91 |
| memory, broader courtyard start 4 | 52.066 / 117.574 | 24.624 / 82.970 | 52 / 116 |

The m10 native-data regression and shared collision-region regression pass.
The separately focused shared-core selection filter passes six tests with the full installed-preset census ignored as slow; game all-target Clippy with warnings denied and workspace formatting pass using target/alt-m10 and four build jobs, and only m10.rs was formatted.
The broad lighting_selection filter reports 15 passes, one failure and three ignored tests; its sole failure is the other map owner's m20.rs:174 illusion-cleared sky assertion, actual m900010 versus expected m900085, which was left untouched.
A matched original-game control was unavailable; exact rendered parity is not claimed.

### m11_00 proof and frame time

Fourteen settled images, `before-<state>.png` and `after-<state>.png`, cover every state in the table above.
The repeats use the same executable SHA256 recorded in `binary-sha256.txt`; the map change is a regression/audit addition, so these are verification repeats rather than a claimed visual fix.
`arena-gyoubu.png`, `arena-demon.png` and `doorway-center.png` provide arena and real boundary-walk controls.
The initial blank five-second image is preserved as `startup-incomplete-morning.png` and is excluded from visual conclusions.
All runs used target/alt-m11_00, Vulkan on an RTX 5070 Ti, 1600x900 and the optimized dev profile; the reduced controls used 800x600.
Performance-only runs do not take screenshots or enable per-frame camera logging, and avoid the duplicate render probe installed when perf_log and --profile-systems are combined.
The summary selects simulation steps 1500-1770 with zero pending pipelines, excluding exit sleep; raw CSVs and `perf-summary.json` retain every run.

| View | Before median / p99 ms (frames) | Final repeat median / p99 ms (frames) |
| --- | --- | --- |
| Default temple start | 16.704 / 18.030 (271) | 23.728 / 64.821 (165) |
| Heavy gate start 8 | 16.685 / 18.070 (271) | 28.684 / 72.059 (146) |

The first after pair was also unstable: temple 74.179 / 272.193 ms (49 frames), gate 44.229 / 141.753 ms (81 frames).
The reduced Vulkan control remained at 16.617 / 19.081 ms (270 frames), while the separate DX12 low-draw control was much slower at 179.417 / 255.006 ms (25 frames).
Requested AutoNoVsync therefore did not produce an independently verified uncapped Vulkan scene cost, and the same unchanged binary did not maintain a steady frame time throughout the shared-machine session.
The actual swapchain mode/driver limiter and the origin of intermittent Prepare stalls remain unresolved shared/environment blockers; no claim of a map-specific performance regression or improvement is made.
Before repeats, CPU render/presentation occupied about 11.8-11.9 ms and Prepare 1.7-1.9 ms; in final repeats Prepare rose to about 13.4/17.1 ms while the render interval was about 7.0/6.6 ms.
These are intervals between CPU render-schedule stamps and can include synchronization waits, not isolated GPU costs.
The independent gate profile reports GPU opaque rendering 2.732 ms, fog 1.322 ms, early prepass .281 ms, bin unpacking .112 ms, native post .062 ms and clustering .058 ms near its end.
The GPU timings do not explain the much larger CPU stalls by themselves, and do not justify changing this map's authored culling or lighting data.

### m11_00 validation

The installed m11_00 state/region/arena regression passes, as do all seven shared selector tests and the focused collision-system test.
The broader lighting_selection filter reported 13 passes and one unrelated failure in `lighting/maps/m25.rs:37`, whose dragon-defeat flag assertion belongs to the Fountainhead agent.
Final `cargo clippy --offline -p sekiro-game --all-targets -j 4 -- -D warnings` and `cargo fmt --all --check` pass; the owned map file also passes its individual rustfmt check.
Every Cargo command used CARGO_TARGET_DIR=target/alt-m11_00 and four jobs, with offline dependency resolution.
No other map file, shared renderer, game install, cache/re/logs directory or git state was edited.

## Senpou audit (2026-10-14)

Only `crates/game/src/lighting/maps/m20.rs` and Senpou documentation were changed.
The existing local `sekiro-collab/WORK.md` already claims this map audit.
`docs/RUSTSEKIRO-VISUALS.md` does not establish a validated fix for Senpou's colour cast, so no unverified colour correction or source port was applied.
All assets and installed events were read-only.

### Native selection and limits

| State | Flags seeded by profile | Hour / environment | GPARAM / sub-ID | Requested sky | Result |
| --- | --- | --- | --- | --- | --- |
| morning, default | none | 6 / 1 | m20_00_0000 / 0 | m900000 | Event selection agrees; entrance remains dark and brown |
| afternoon | 9800 | 12 / 2 | m20_00_0000 / 0 | m900010 | Event selection agrees; exact renderer equivalence unverified |
| evening | 9800, 9801 | 18 / 3 | m20_00_0000 / 0 | m900020 | Event selection agrees; warmed capture shows geometry normally |
| night | 9800, 9801, 9802 | 22 / 4 | m20_00_0000 / 0 | m900030 | Event selection agrees; frame-time spikes remain |
| dawn | 9800-9803 | 2 / 5 | m20_00_0000 / 0 | m900050 | Event selection agrees; no complete environment-5 probes or GI coefficients are exported |
| illusion | 8400, 9303 | 12 / 2 | m20_00_0010 / 10, region switches to 0000 / 0 | m900085 | Event sub-ID/sky agree; selected file has no authored entries and sky export is missing |
| illusion-cleared | 8400, 9303, 62000705 | 12 / 2 | m20_00_0000 / 0 | m900085 | Fixed missing entry sky command; sky export still missing |

The cleared preset represents entry/re-entry with 8400 and 62000705, not the separate boss-return sequence that disables the illusion sky in event 12005800:42.
Event 12005210:8-10 skips sub-ID 10 when 62000705 is set, but still enables sky group 8907085 at instruction 25.
The previous cleared preset referenced only event 12005855:12, so its common sky incorrectly remained m900010.
The map-local fix adds the original sky activation reference before the clear sub-ID reference.
The installed-data regression failed on that exact sky mismatch before the change and passes afterward.

Native collision configurations are `[0,0,0,0]`, `[100,0,0,0]`, `[110,0,0,0]`, `[120,0,0,0]` and `[130,0,0,0]`.
The entrance selects light 110; the cliff heavy view selects h005300 and light 100.
Light 130 has no authored LightSet row in 0000 and follows the existing id-zero fallback contract rather than an invented map-specific row.
The native-region regression reads region 2002880's geometry directly from the installed MSB, moves inside/outside/inside, and verifies event 12005826 selects sub-IDs 0/10/0 through the same library used by rendering.
Setting 62000705 prevents a subsequent outside switch back to 10.
The moving-focus collision regression also passes with culling off and checks all four independent IDs.
A physical scripted entrance walk moved about 9 m, stopped before the stair/exit boundary and retained light 110; it does not establish a complete indoor/outdoor walking transition.
The currently registered Senpou arena, `senpou-screens`, retains its ArenaSpec noon hour, environment 2, sub-ID 10, sky m900085 and beaten/boss flags.

The re-export now supplies 85 complete coefficient volumes for each of variants 1-4, and runtime logs confirm that all 85 are consumed.
The missing m900085 sky was reported without extracting anything.
No matched original-game capture was available, so matching event selection must not be read as proof of exact visual fidelity.

### Measured renderer inputs and shared fixes

The complete native parameter audit and converted `LightSet`/`GameFog` values are in `target/alt-m20/captures/audit-before.txt` and `audit-after.txt`.
Each state logs all collision light IDs, light colours/weights, hemisphere values, fog, tone/exposure and grading rows.
Runtime logs also record the selected file, actual light set, converted post upload, GI count and active camera probe.

At the morning entrance, authored sun RGBA is `(1,0.72549,0.54902,1.2)` and fill RGBA is `(0.823529,0.705882,0.647059,0.05)`.
The renderer uses squared linear RGB `(1,0.526336,0.301423)` at 5400 lux and `(0.678200,0.498269,0.418685)` at 225 lux, matching its documented 4500-unit bridge.
Hemisphere up/down are `(0.509804,0.647059,0.803922)` with weights 1/0.1, producing mean `(0.142945,0.230277,0.355460)` and delta `(0.116955,0.188408,0.290831)`.
Fog uploads fixed density 0.00105, height coefficient 0.03, height density 0.002, affine height offset 100, near/far 0.01/800 and sky/direct starts 5/0.5.
Its packed sky/direct/opposite RGB are `(0.190463,0.311181,0.436813)`, `(0.915750,0.358654,0.127530)` and `(0.045427,0.071731,0.098723)`.
Native post exposure 0.7 uploads gain 1.624505, middle gray -4.3, adaptation limits -0.5/0.5, auto exposure enabled, LUT 0 and tone mode 7.
Those CPU conversions are reproducible; native scene-to-Bevy calibration and final tone/exposure equivalence remain separate rendering work.

The matched viewer comparisons use identical entrance camera poses and wait for all render pipelines.
Sampling 9680 display-RGB pixels outside the HUD/capsule gives native mean RGB `(38.856,30.841,27.151)`, R/B 1.431; without fog `(21.943,20.969,20.139)`, R/B 1.090.
Without grading R/B is 1.481, without BTL lights 1.431 and without GIIV coefficients 1.474.
Thus fog contributes substantially to the visible warm cast, grading does not introduce that cast, and the entrance BTL lights are not its measured source.
Authored fog itself is warm in the sunward direction, so these comparisons do not justify suppressing or recolouring it in m20.
The images and numeric samples are `diagnostic-*.png` and `image-means.csv` in the capture directory.

Shared-code findings and proposed fixes, deliberately left unedited:

- `lighting_selection.rs:129-133` accepts a present family-00 file without validating any authored rows, and `lighting_selection.rs:412-414` replaces the complete active GPARAM with it.
  Native m20_00_0010 has zero entries, so illusion renders invented default sun/fill at 6750/900 lux, default fog `(0.0015,0.1,0.125,150)` and default post values.
  The populated 0900/0910 pair is a concrete candidate, but EMEVD provides the sub-ID rather than family provenance.
  Proposed fix: detect empty full replacements, recover the native family/partial-file inheritance rule, then apply that rule in the shared resolver/library; do not silently select a guessed nearest family or claim generic defaults are native lighting.
- `lighting.rs:881-927`, `925-951` and `957-995` read/spawn/apply only two directional lights.
  Morning light 110 sets `Light1 FollowedCamera=1`, `Enable LocalLight1=1` and `Enable LocalLight2=1`, but none of those controls are consumed.
  LocalLight1 contributes authored blue RGBA `(0.45098,0.511763,0.607843,0.3)`; LocalLight2 has diffuse weight 0 but specular weight 0.1.
  The sun's separate specular weight is 0.2 versus diffuse 1.2, while the Bevy light uses one illuminance for both responses.
  Proposed fix: carry camera-follow/enables and all four native light slots through LightSet, and preserve independent diffuse/specular colours and weights in the shared material lighting path instead of compensating with m20 tints or exposure.
- ~~`lighting.rs:744-764` chooses the largest enclosing volume for the camera fallback, which selects probe 0 despite the entrance lying inside local volume 100.~~
  Fixed by the [existing smallest-containing-box selection](#spatial-gi-and-fallback-2026-10-15).
  Mean morning diffuse cubemap RGB is `(0.366156,0.262202,0.229525)` for probe 0 versus nearly neutral `(0.005517,0.005210,0.005320)` for probe 100.
  Spatial receiver GI is separate; these means demonstrate a substantially warmer/brighter fallback, not the exact radiance of every surface.
  Proposed fix: review local-versus-global fallback selection and receiver coverage with the collaborator responsible for probe blending, retaining global fallback only where the native selection requires it.
- ~~`fog.rs:172-232` and `fog.wgsl:44-53` do not consume authored `Global ScatteringG=0.25` or `DirLight LightBankId=120000000`; the current source uses a linear directional colour mix and the selected sun direction.~~
The recovered global shader correctly uses the linear direct/opposite colour mix; `ScatteringG` belongs to local point/spot fog, which remains unimplemented.
Bank 120000000 now resolves its native light independently, and the verified warm direct colour remains `(0.91575015, 0.35865393, 0.12752977)` before the common camera radiance conversion.
  Tone curve, auto exposure and probe blending remain the collaborator's project.
- `sky.rs:397-405` retains an ordinary clock sky when an event-selected model is missing.
  Both illusion states now correctly request m900085 but visibly retain m900010.
  ~~Proposed fix: expose the missing authored sky as an explicit incomplete-fidelity condition.~~
  Corrected 2026-10-15: missing requests warn once per changed request with `incomplete sky fidelity` and retain the last available dome and its matching clouds.
  Exact rendering still requires a separately supplied complete read-only export; this job did not extract or modify game data.
- `perf_log.rs:223-255` stores intervals between render-set stamps, including intervening work and possible surface acquisition waits.
  The heavy profile's 11.9 ms gap before Queue is not a GPU shader duration or an unambiguous Queue-system cost.
  Proposed fix: split swapchain acquisition/presentation waits from CPU scheduling and add finer system timing before attributing the remaining wall time to map rendering.

### Captures, frame time and validation

Accepted before/after pairs are `before-ready-<state>.png` and `after-ready-<state>.png` for all seven names above, plus the heavy view, under `target/alt-m20/captures/`.
The corresponding folders contain raw stderr/stdout and perf_log CSVs.
All application runs request AutoNoVsync through `--uncapped` at 1600x900.
The matched play runs idle for 900 fixed ticks and capture at tick 720; summaries use ticks 600-840, exclude screenshot ticks 715-735 and require zero pending pipelines.
Early `before-*.png` trials captured during compilation and are not accepted visual/performance proof.
In particular, the blue evening frame and blank heavy frame disappeared after proper warm-up; they were not established fog defects.

| State / view | Before median / p99 ms | After median / p99 ms | Before / after ready samples |
| --- | --- | --- | --- |
| morning, default | 42.912 / 62.535 | 17.966 / 25.475 | 85 / 199 |
| afternoon | 46.984 / 137.012 | 18.823 / 28.206 | 66 / 183 |
| evening | 29.006 / 41.574 | 30.318 / 60.948 | 129 / 117 |
| night | 34.623 / 59.069 | 55.601 / 134.813 | 99 / 57 |
| dawn | 36.813 / 61.216 | 18.425 / 25.702 | 93 / 191 |
| illusion | 50.558 / 335.072 | 16.465 / 20.879 | 44 / 220 |
| illusion-cleared | 99.551 / 200.765 | 16.297 / 20.502 | 35 / 222 |
| heavy, start 2 cliff walkway | 63.371 / 155.242 | 16.750 / 23.467 | 53 / 220 |

Separate viewer profiles disable PLAY_LOG_PERF to avoid installing the render probe twice.
Start/heavy GPU costs are main opaque 2.09/2.65 ms, fog 0.67/1.29 ms and early prepass 0.32/0.25 ms; native post is about 0.06 ms.
The heavy render schedule reports mean 16.354 ms, maximum 17.058 ms and 113 ready pipelines with none waiting, with an 11.9 ms interval before Queue.
The heavy scene uses native draw-group culling: 3503/5729 pieces shown and 13751 instantiated mesh entities.
Measured shadow-budget CPU work is about 0.035 ms mean and 1.036 ms maximum in the warmed heavy profile.
No data-supported map-specific object removal or performance fix was identified.
Concurrent machine work and shared-source changes are uncontrolled; the sole production change is a missing sky request, so timing differences are not evidence of a performance optimisation.
The approximately 16.7 ms wall times must not be equated with GPU cost or proof that presentation pacing is absent.
Night spikes and the native-data/export gaps prevent claiming steady frame time or exact game lighting across all states.

Two m20 installed-data/region tests, six shared lighting-selection tests and the moving-collision regression pass, with the declared slow full-preset test left ignored.
Final `cargo clippy --offline -p sekiro-game --all-targets -j 4 -- -D warnings` and `cargo fmt --all --check` both pass.
Builds and tests use CARGO_TARGET_DIR=target/alt-m20 and four jobs, with no commits, git-state changes, network/model calls or writes to protected data directories.

## Castle audit (2026-10-14)

This audit changes only `lighting/maps/m11_01.rs`, the Castle row and these notes, and one dated `STATUS.md` line.
All Cargo builds and tests use `CARGO_TARGET_DIR=target/alt-m11_01`, four jobs and offline resolution.
Original event references and encounter flags select the installed files without changing their light, material, fog or exposure values.

| Story | Seed flags | Hour / environment | GPARAM / sub-ID | Native sky | Selection result |
| --- | --- | --- | --- | --- | --- |
| morning | none | 6 / 1 | m11_01_0000 / 0 | m900000 | Matches common event 100 |
| afternoon | 9800 | 12 / 2 | m11_01_0000 / 0 | m900010 | Matches common event 100 |
| evening | 9800, 9801 | 18 / 3 | m11_01_0000 / 0 | m900020 | Matches the ordinary evening branch |
| night | 9800-9802 | 22 / 4 | m11_01_0000 / 0 | m900030 | Matches common event 100 |
| dawn | 9800-9803 | 2 / 5 | m11_01_0000 / 0 | m900050 | Matches common event 100 |
| invasion | 9800-9802, 8302 | 22 / 4 | m11_01_0020 / 20 | m900095 | Matches the Ashina fire branch |
| tomoe | 11115802, 11115801 | 18 / 1 | m11_01_0010 / 10 | m900080 | Matches 11115820:24,25,28 |
| shura-fire | 9800, 9801 | 18 / 3 | m11_01_0740 / 40 | m900020 | Matches 11115910:29 |
| owl-fight | 9303-9306, 8301, 11115722 | 18 / 3 | m11_01_0750 / 50 | m900020 | Matches 11115860:54 |

The 07-family files are the sole installed candidates for native sub-IDs 40 and 50; the engine's general family-selection rule is still not recovered.
The new `owl-fight` name distinguishes the encounter override from an ordinary evening visit.
Tomoe's native phase-two flag 11115802 now selects its handover instructions for an ArenaSpec that actually starts with c7110.
Emma's 1270/1276 flags select sub-ID 40, and Owl's 8301/9303/11115722 flags select sub-ID 50.
The Shura rule precedes the Owl rule because their story and entry flags overlap.
Their staged event entry labels skip the earlier native intro instructions, which is why restoring the requested GPARAM at selection time is necessary.
Genichiro retains 6 h, environment 1, m11_01_0000 and m900000; Owl retains 18 h, environment 3 and m900020 while using its authored encounter GPARAM.
The regression checks all nine presets, their native skies, every exported collision LightSet against the read-only native MSB, and Genichiro/Owl/Shura/Tomoe ArenaSpec selections.

The native collision tuples are `[0,0,0,0]`, `[100,0,0,0]`, `[110,0,0,0]` and `[120,0,0,0]`.
A grounded scripted round trip beginning near `[93.670,36.700,228.025]` enters bank 110 near `[98.055,36.500,225.594]` and returns to bank 100 near `[93.506,36.700,228.116]`, with zero fall distance in the logged stationary checkpoints.
Independent fog, scattering and environment banks remain their native zero IDs.
The accepted evidence is `door-roundtrip*` and `roundtrip-outside.png`, `roundtrip-inside.png`, `roundtrip-return.png` under the capture directory.
Earlier centroid/scaffold attempts fell or stayed in one bank and are retained as diagnostics, not accepted crossing proof.
The interior diagnostic camera is occluded by nearby geometry; its lighting-bank and grounded-position logs establish the crossing without claiming a clean native-game camera match.

### Authored inputs and renderer uploads

Raw rows and `LightSet::from_gparam` conversions are logged by `castle_installed_story_lighting`; actual post and fog uploads are recorded in every capture log.
`lighting-values.json` records the native diffuse/specular and hemisphere colours alongside actual direct-light intensities, fog vectors and exposure inputs.
The following values are for the default start's bank 100, after the selected file is applied.

| Story | Sun / fill lux | Fixed fog density per metre | Height density / distribution | Height plane Y / far depth metres | Exposure EV / multiplier | Auto exposure |
| --- | --- | --- | --- | --- | --- | --- |
| morning | 13500 / 1350 | 0.0008 | 0.2 / 0.08 | -150 / 350 | 0.5 / 1.414214 | on |
| afternoon | 4500 / 2250 | 0.0006 | 0.05 / 0.04 | -150 / 1000 | 0.5 / 1.414214 | on |
| evening | 9000 / 1350 | 0.0015 | 0.125 / 0.08 | -110 / 300 | 0.8 / 1.741101 | on |
| night | 18000 / 2250 | 0.003 | 0.125 / 0.08 | -110 / 200 | 0.3 / 1.231144 | on |
| dawn | 13500 / 2250 | 0.008823529 | 0.07352941 / 0.29411763 | -85 / 340 | 0.2 / 1.148698 | on |
| invasion | 18000 / 2250 | 0.005 | 0.125 / 0.08 | -110 / 200 | 0.1 / 1.071773 | on |
| tomoe | 4500 / 0 | 0 | 0.05 / 0.095 | +20 / 400 | 0.25 / 1.189207 | off |
| shura-fire | 9900 / 6750 | 0.0015 | 0.35 / 0.07 | -85 / 300 | 0 / 1 | on |
| owl-fight | 13500 / 6750 | 0.0015 | 0.35 / 0.07 | -85 / 300 | 0 / 1 | on |

At morning bank 100, raw sun RGBW `[1,0.745098,0.627451,3]` becomes linear RGB `[1,0.555171,0.393695]` and 13500 lux, while fill RGBW `[0.784314,0.882353,1,0.3]` becomes `[0.615148,0.778547,1]` and 1350 lux.
Native hemisphere up/down RGB `[0.392157,0.470588,0.588235]` with multipliers 0.5/0.1 yields mean `[0.046136,0.066436,0.103806]` and nominal brightness 1432.3944 before probe application.
The captured native post upload uses middle gray -5.2, adaptation limits -0.7/2, mode 7 and multiplier 1.4142135 at this start.
Morning fog's raw GameScale 1.25, fixed density 0.001, distribution 0.1, height scale 0.5, offset -150 and far depth 280 convert to the uploaded values in the table.
Owl's native 0750 differs from ordinary evening 0000: sun diffuse multiplier 3 versus 2, fill 1.5 versus 0.3, exposure 0 versus 0.8, sun yaw 2.617994 versus -3.141593, and height offset -85 versus -110.
At the roof's Y 54.04, its corrected height layer is approximately 139 m below the roof, consistent with the diagnostic observation in `RUSTSEKIRO-VISUALS.md`.
No validated Castle renderer replacement was demonstrated there, so no MIT source was copied and no credit-file change was needed.

### Shared follow-ups, left unedited

- `map.rs:421-434` reconstructs the named arena's default enemy and then treats any finite scene hour as an override; `lighting_selection.rs:183,283` consequently seeds and times that default enemy instead of the requested starting phase.
The reproduced `--arena genichiro --genichiro-phase tomoe` capture loads ordinary evening `m11_01_0000`, sub-ID 0, environment 3 and an ordinary sky, while the native Tomoe preset and a Tomoe ArenaSpec select `0010`, sub-ID 10, environment 1 and m900080.
The shared fix must pass the actual starting EnemySpec/BossEvents into selection and apply the nearest-hour environment/sky diagnostic only for an explicit `--hour`, allowing this map's phase flag rule to run.

- ~~`arena_b.rs:115` calls `lighting_selection.rs:392-398` to derive a probe environment from every changed lighting hour, even without SetAreaEnvmap.~~
Native 11115820 changes lighting to 3 and sub-ID to 10 without changing the environment, so a real morning-to-Tomoe handover can incorrectly select camera probes 3 while retaining the loaded GIIV atlas 1.
~~Retain `selection.initial.environment` unless the native scene requests an environment change and restore it on restart.~~
Corrected 2026-10-15: clock-only updates preserve the probe environment and handles; explicit environment events invalidate handles, and restart restores the initial environment.
Synchronizing coefficient atlases with genuine environment changes remains the GI owner's work; this sky correction does not edit material or probe-shading code.
The independent environment-routing correction is distinct from the collaborator's probe-blending calibration work.

- ~~`sky.rs:488-495` discards the event-selected initial sky whenever the lighting hour differs from the initial hour.~~
Native Shura instructions 11115930:50 and 11115943:14 request SetLighting(4) without replacing the sky group, so the renderer can fall back to a generic night dome despite the retained native evening group.
~~Preserve the authoritative native sky selection across clock-only changes until an explicit map-part/environment request replaces it.~~
Corrected 2026-10-15: the latest enabled native sky group or initial authored sky survives clock-only changes, independently of environment selection.

- ~~`lighting.rs:884-885` reads only DiffColor0/1, and the standard directional-light path uses those intensities for both diffuse and specular response.~~
  Superseded by the [authored map-light correction](#authored-map-lights-2026-10-15), retaining the independent diffuse/specular colours and all four slots.
Morning bank 100 has separate native sun specular multiplier 1.5 versus diffuse 3, and fill specular 0.5 versus diffuse 0.3 with the same respective RGB values.
Recover and carry the separate native specular colours and intensities through shared direct-light shading; scaling this map's materials or exposure cannot preserve both controls.
This is a measured omitted control, not a quantified explanation of every pale material without an original-game reference.

- ~~`fog.wgsl:22,74,100-117` assigns depth-zero sky pixels an uncalibrated 20 km distance and applies analytical continuation plus a fixed 0-to-0.22 sky-height blend.~~
For a representative morning roof-height horizontal ray, the actual density gives cap optical depth 0.280, continued optical depth 4.119 and sky transmission approximately 0.01626.
The before rooftop horizon patch at X 920-1229, Y 205-274 has display-code luminance mean/std 0.628664/0.025787 with fog and 0.799008/0.051250 without it; a full-map no-fog control gives 0.795444/0.051436 and still lacks distant silhouettes.
These are PNG code-value measurements, not linear HDR or native-game luminance.
~~Recover the native sky/background fog routing and analytical continuation before changing this map's authored density or adding a per-map horizon ramp.~~
The 2026-10-15 correction uses the live authored sky geometry, native volume slice thickness and the analytical source padding layer, with unchanged authored densities.
The remaining featureless background also uses the approximate lower-hemisphere gradient in `sky.wgsl:34,51-54`; the controls do not prove fog alone is responsible for absent silhouettes.

- ~~`lighting.rs:786-789` zeros global ambient once the camera probe is ready, and `map_material.rs:476,485,638` disables the authored hemisphere contribution accordingly.~~
  Fixed by the [shared per-receiver fallback](#spatial-gi-and-fallback-2026-10-15).
Castle loads 107 complete native coefficient volumes in the observed environments, so the missing-GI export diagnosis from other maps does not apply here.
Evaluate the authored hemisphere, native volume contribution and fallback coverage with the probe owner rather than introducing a Castle brightness gain.
Tone curve, auto exposure and probe blending remain with the other collaborator's project.

### Proof and timing

All local artifacts are under `target/alt-m11_01/captures/`, including before/after PNGs for the nine story selections, morning Genichiro/evening Owl/Shura arena controls, command manifests, raw perf CSVs, parameter logs and executable hashes.
The new owl-fight default-start before image reuses the pre-fix evening capture because the old selector used that same hour/environment/sky and ordinary GPARAM; the separately captured `before-owl.png` records the actual pre-fix arena and its encounter flags.
The final after executable is a frozen dedicated-target build; earlier after snapshots and unsuccessful walk attempts remain as diagnostics.
Concurrent shared-source changes can affect before/after comparisons, so no rendering or timing improvement is attributed solely to the map metadata change.

All measured commands request `--uncapped`, which sets AutoNoVsync and continuous Winit updates, and redirect perf output through PLAY_PERF_DIR.
Frame statistics use zero-pending-pipeline rows at simulation steps 360 through 479, excluding the screenshot at step 480, with the median and nearest-rank p99 reported below.
The per-state samples and all raw rows are in `frame-times.csv`; the accepted final Vulkan state captures have 119-120 ready samples each.
Initial before captures had concurrent compilation and camera logging enabled, so the repeat start/full-map controls with camera logging disabled are the fairer comparisons.

| View | Before median / p99 ms | After median / p99 ms | Ready samples before / after |
| --- | --- | --- | --- |
| Default start, repeated profile | 16.615 / 24.413 | 16.704 / 27.726 | 120 / 120 |
| Heavy full-map castle-top view | 16.698 / 19.386 | 16.686 / 19.011 | 120 / 120 |
| Genichiro morning control | 40.797 / 74.887 | 16.661 / 19.983 | 44 / 120 |
| Owl evening encounter control | 32.626 / 45.309 | 16.632 / 20.071 | 61 / 120 |
| Shura encounter control, repeated before | 16.676 / 20.251 | 16.678 / 21.558 | 120 / 120 |

Final GPU pass medians are opaque 2.772 ms and fog 1.477 ms at the start, and opaque 4.930 ms and fog 1.377 ms in the heavy view.
The corresponding logged CPU system medians are cloth simulation 1.237 ms at the start, and moving-focus collision selection 1.133 ms plus cloth 0.788 ms in the heavy view; these system summaries cover logger windows rather than precisely the CSV sample interval.
~~The shared per-fragment receiver lookup in `map_material.wgsl:84-86` scans 107 volume bounds before coefficient/reflection sampling; any acceleration must preserve native ordering and weights and belongs with shared probe rendering.~~
Fixed by the [shared ordered spatial lookup](#spatial-gi-and-fallback-2026-10-15).
No demonstrated Castle-specific object or mesh was removed.
Despite AutoNoVsync, the quiet Vulkan cadence remains near 16.6 ms while GPU passes cost much less and the render schedule includes presentation/waiting time.
~~The shared renderer should log the actual configured surface present mode~~; presentation timestamps and driver/compositor limits still need investigation instead of interpreting wall_ms as exclusive GPU cost.
`presentation.rs` now logs the concrete supported mode, but the shared host still shows the 60 Hz plateau in Immediate and Mailbox controls.
A short DX12 backend diagnostic was also attempted; its requested warm interval still had pending pipelines, so those logs are retained separately and excluded from accepted perf statistics.
Earlier after runs observed another sekiro-rs process and 11 concurrent rustc processes, and their large frame-time spikes remain in the retained intermediate evidence.
These shared-machine observations do not establish isolated steady frame pacing.

The final map regression and all six active shared lighting-selection tests pass; the existing exhaustive all-map sweep remains ignored.
`cargo clippy -p sekiro-game --all-targets -j 4 -- -D warnings` and `cargo fmt --all --check` pass in the dedicated target.
There were no commits, git-state operations, network/model calls, or deliberate writes to protected game-data/log directories.
Exact native visual equivalence remains blocked by the shared phase/probe/sky/specular/fog issues above and the absence of matched original-game reference frames for this session.

## Genichiro rooftop snow audit (2026-10-14)

The native data does contain settled snow and ordinary falling snow during Genichiro Ashina's rooftop fight.
The rooftop is `m11_01_00_00`; `m11_00_00_00` is the adjacent Outskirts, whose sub-ID also changes during the Tomoe handover.
With progression flags 9800-9803 and invasion flag 8302 unset, common event 100 selects its morning branch at instructions 92-97: sky group 8907000 (`m900000`) and environment 1.
Our Genichiro ArenaSpec has no beaten time-advancing bosses or Tomoe flag 11115802, and the captured runtime selects 06:00, environment 1, sub-ID 0 and `m11_01_0000`.
Its ordinary pavilion mesh remains active; event 11115943 swaps it for the burnt floor only in the later Shura state, after 8301 and 11115921.
The roof capture uses collision LightSet 100 with independent fog/scattering/environment row IDs 0, not a storm GPARAM.
The base GPARAM's morning volumetric fog has GameScale 1.25, fixed density 0.001, far depth 280 and height offset -150; none is a snow-material enable switch.

Native event 11115800 spawns map SFX entity 1114350 at instruction 126 for the ordinary fight, and at instruction 45 when restoring ordinary lighting after the encounter.
The MSB maps this origin-placed, camera-relative effect to FXR 839500 in the common-effects archive.
Map event 11115170 also controls it: invasion flag 8302 ends ordinary snowfall, and regions 1112150/1112151 or SpEffect 110040 suppress it indoors.
Those two composite regions reference nine boxes whose highest top is 53.71 m, below the approximately 54.04 m fighting floor, so their geometric exclusion does not cover the rooftop.
Composite child indices and box dimensions were decoded using the [SoulsFormatsNEXT shape layout](https://github.com/soulsmods/SoulsFormatsNEXT/blob/master/SoulsFormats/Formats/MSB/Shape.cs).
Its weather action 10000 uses texture 17004, pale blue-white particles, nominal size 0.025 with variation, downward speed `[0,-1,0]` with variation and a 15-by-15-by-5 emission volume.
This establishes small falling flakes, not a dense snow cover across the fighting floor; the raw emission count 40 and duration 180 are not a verified particles-per-second rate.
Event 11115820:24-29 replaces the sky with 8907080 (`m900080`), sets castle sub-ID 10 and Outskirts sub-ID 30, selects lighting variant 3 and deletes 1114350 without changing the environment.
Its instructions 38-40 spawn entities 1114832/1114835/1114836, mapped by the MSB to FXRs 811240/811241/811242, for the storm.
The nearby 811201 and camera-relative 811305 FXRs use an orange ember flipbook texture 3000; their placement alone is not evidence for snow.

| Native surface | Active material facet | Snow evidence |
|---|---|---|
| Pavilion `m015070`, material 0 | MultipleGround 6 | Dedicated SnowGround geometry, 10,750 vertices, base albedo `m11_snow_20_a` plus snow/detail normals; remains snow even with overlay blending disabled |
| Pavilion `m015070`, materials 6 and 9 | Multiple 11 | Floor1 enables snow albedo slot 12 and normal slot 10, with upward snow direction and authored masks |
| Pavilion tatami, material 3 | Multiple 1 | Snow slots 12/10 excluded by the native static switch; exported snow binding absent |
| Adjacent roof tiles | MultipleGround 18 | Snow albedo/normal slots 11/7 enabled with upward direction |
| Perimeter boards `m016010` / `m016015` | MultipleGround 6 | Snow texture and mask with upward direction |
| Broken wood `m016300` / `m016301` | Multiple 21 | Snow albedo slot 12 enabled, snow normal slot 10 excluded |

The inspected reproduction shows discrete white deposits beside railings and pavilion edges while most tatami remains uncovered.
The same deposits remain in the matched `--no-layers --no-fxr` control, confirming that dedicated base snow geometry is sufficient to explain them.
Our build supplies these through the native base snow geometry and `map_material.rs` bindings consumed by `map_material.wgsl::game_layers`, with the exported static facets rather than a weather-wide snow toggle.
There is no separate synthetic snowfall system; additionally `fxr.rs::collect_node` skips native weather action 10000, and ordinary Genichiro staging runs 11115810/11115820 rather than the weather-spawning 11115800.
Consequently the visible settled snow is supported by the data, while native falling snow remains an unimplemented effect rather than a reason to remove the deposits.
No story selection, GPARAM or snow-disable special case was changed.
The new `genichiro_settled_snow_selection` regression checks morning selection, native snowfall spawn/removal and native-versus-exported snow facets, including snow-free tatami.
Read-only event/material/FXR audits, decoded effect textures and rendered controls are retained under `target/alt-gs/captures/`; the reproduction binary was the existing `target/alt-m11_01/release/sekiro-rs.exe`.
Exact snow brightness, blend coverage and snowfall density still need a matched original-game frame; this audit establishes presence and selection, not pixel-level equivalence.
Validation uses `CARGO_TARGET_DIR=target/alt-gs`: `cargo test --release --offline -p sekiro-game -j 4` passes 84 unit tests and six integration tests, with two existing ignores.
`cargo clippy --release --offline -p sekiro-game --all-targets -j 4 -- -D warnings`, `cargo fmt -p sekiro-game -- --check` and the final `cargo fmt --all --check` pass.

## Authored map lights, 2026-10-15

This follow-up supersedes the earlier two-light production path and the statement that every BTL entry is a point light.
BTL record offset `0x18` is the 32-bit type: point 0 or spot 1 in the installed files.
Offset `0x1c`, previously mistaken for the type, is a separate flag.
The runtime decodes the preserved raw sidecar records, so existing version-1 exports remain usable without re-exporting the maps or disturbing the grass exporter.

Outskirts bank 0000 contains 16 points; bank 0002 contains six points.
Castle bank 0000 contains 32 points and two spots; bank 0002 contains 25 points and one spot; bank 0004 repeats 32 points and two spots.
All entries in these Ashina banks author the shadow flag off.
Bank 0000 remains the default; `--light-bank` explicitly selects another bank, and a missing sidecar, selected bank or LightSet row is reported as an error rather than disappearing silently.
Automatic story selection of nonzero BTL banks remains untraced.

Ancillary BTL coordinates now receive the MSB MapOffset before the common X reflection.
Outskirts authors translation `(90, 3, -40)` and Y rotation 20 degrees; Castle has no MapOffset.
Placed MSB geometry receives no additional transform.
This resource relationship is independently documented by [Smithbox's MapOffset and BTL parenting](https://github.com/vawser/Smithbox/blob/cbd477a8fd6d436b3e011c8548e1de8fd8876918/src/Smithbox.Program/Editors/Map%20Editor/Data/MapContainer.cs#L91).
The [BTL field layout](https://github.com/soulsmods/SoulsFormatsNEXT/blob/master/SoulsFormats/Formats/BTL.cs) supplies type, rotation, range, cone, separate powers and shadow provenance.

Connected Ashina views load all six warm blocks' base banks, totalling 246 lights before zero-intensity/range rejection.
Bevy's clustered GPU range/frustum culling replaces the arbitrary 64-light, 60-metre CPU cutoff.
Authored point ranges and shadow flags are preserved, and native diffuse RGB is squared before its authored power.
The current Bevy bridge interprets the spot cone in degrees as a full angle, converts it to a half angle and reflects its rotation, including the native forward-axis conversion.
Exact native spot width/aspect, cone-edge sharpness, source radius, independent local specular powers, flicker and attenuation are not yet reproduced by Bevy's stock point/spot response.
These are explicit approximations, not values adjusted against screenshots.

All four LightSet diffuse/specular records now contribute by default, including camera-relative local slots, constructor defaults, authored enables and sharpness.
The recovered accumulation uses ordinary Lambert diffuse for the first two slots, the native wrapped diffuse for the extra slots and the documented tuned specular expression.
Material gloss/Fresnel routing still uses the existing exported material representation.
`SEKIRO_MAP_LIGHTS_LEGACY=1` restores this job's old two-light compositor, unplaced point-only BTL upload and CPU cap for matched-camera controls while retaining the preceding GI and fog corrections.
`SEKIRO_GI_NATIVE_LIGHTS=0` disables only the new LightSet compositor, leaving corrected BTL placement active for ablation.

The render-stage probe now has one idempotent installation shared by map profiling and play performance logging.
This fixes the duplicate performance probe; it does not remove a GI volume or an environment-map probe.
Warm catalogues retain all 246 records, while only blocks in the shared ready state activate their lights; dormant/requested/retiring neighbours cannot illuminate the current block.

The read-only installed-bank census also decodes Hirata 147 points/30 spots, Reservoir 3/3, Dungeon 36/0, Depths 98/14, Valley 30/7, Senpou 82/35 and Fountainhead 32/6 in their base banks.
Hirata authors three local shadow flags and Valley seven; all other base-bank local shadow flags are off.
No unsupported light type occurs in the installed nine-map bank census.

The requested external reference positions are outside every primary base-bank BTL radius: shrine spot-003, hut spot-016, hall entrance spot-044, battlefield spot-045 and Castle idol spot-123.
The nearest distance/range pairs are respectively 61.53/4, 240.69/10, 9.95/6, 25.26/5 and 49.72/8 metres.
These are distances from the indexed player positions, not claims that no visible surface in those views receives a local light.
In particular, the hall's farther floor and fixtures can be inside a light even while the entrance camera/player is outside it.
Missing flames, smoke and effect-generated lights cannot be repaired by increasing a BTL range or inventing fixture lights.
The frozen matched-camera evidence and remaining fixture/emissive observations are recorded with the final render-gate review in MAP-COMPLETENESS-SWEEP.md.

Final validation and all per-view colour-fixture decisions are in [the sweep review](MAP-COMPLETENESS-SWEEP.md#authored-light-colour-fixture-review-2026-10-15).
Both final six-view gates pass in 40.7/48.8 seconds with unchanged geometry; the data gate, 806 workspace tests, final focused packet/shader checks, all-target Clippy and formatting pass.
The matched night controls expose interior detail but leave excessive blue/bright snow, absent flames/smoke/idol effects, uncertain fixture emissive fidelity and the documented stock-Bevy local-light response approximations.
No L1 or E1 spot count is declared resolved from this limited camera sample.

## Native point and spot light falloff, 2026-10-15

Map point and spot lights (BTL and FXR 609) no longer use Bevy's inverse-square response in `map_material.wgsl`.
The native deferred shaders use `saturate(1 - d^2/R^2) * max(t^2 + (1 - t^2) N.L, 0) / (1 + 0.7 d^2/R^2)` with `t = 1.4/(1.4 + d/R)`, and the spot adds `max(1 - (tan/tan_outer)^2, 0)`.
`DiffColor.w` is `1/range` (builder `3892e0` offset 0x4c), so the light range already carried by Bevy is the only extra input.
The full derivation, the specular term, the untraced inputs and the traced unit factor `NATIVE_LIGHT_BUFFER_SCALE` (1.0) with the separate `NATIVE_POINT_LIGHT_UNEXPLAINED_RESIDUAL` are in `docs/MAP-FX.md` ("Native point-light falloff and flame colour").
FXR 609 diffuse colour is squared like the BTL colour, because both go through `3892e0`.
`SEKIRO_MAP_POINT_LIGHTS_BEVY=1` restores the previous curve for matched-camera controls and `SEKIRO_MAP_POINT_LIGHT_SCALE` overrides the unit constant.

## Second brightness audit, 2026-10-15

See [EXPOSURE.md](EXPOSURE.md#brightness-audit-second-pass-2026-10-15).
Recovered here: the deferred AO multiplier is `SSAO.x * G1.z` with the identity remap (`FC_Mult = (0,1,0,0)`, `FC_Add = (1,0,1,1)`), baked occlusion is vertex `COLOR1.x` and is 1 on 99.7 percent of Outskirts vertices.
`SEKIRO_LIGHT_SET=<id>` is a diagnostic that forces one LightSet row; unset keeps the collision's authored row.

## LightSet selection audit, 2026-10-15

See [EXPOSURE.md](EXPOSURE.md#lightset-row-100-selection-audit-2026-10-15).
Collision `SceneGparam` event bytes select overlay family files per layer and are parsed but unused; `mapbase.gparam` is an unread base layer.
Neither explains the row-100 sun excess, and `SEKIRO_LIGHT_SET=0` remains a diagnostic only.

## Layer stack and record defaults, 2026-10-15

The executable evaluates a map block as a stack of GPARAM files, not as one file.
This section replaces the "unread base layer" and "parsed but unused" statements above; the loader now implements it.
The recovered functions are `ed5cd0` (per-block layer manager), `f46300`, `f46340`, `f46480`, `f46fc0`, `f47580`, `189d20`, `167840` (LightSet manager), `17af10` (priority slots), `17b9a0` (evaluator) and the key resolvers `168e10` and `1410a7f20`.

Priority table (`0x142b54500`, indexed by the file kind of `143b146f0`), lowest first:

| Priority | File kind |
| ---: | --- |
| 1 | `default` |
| 2 | `M_template`, `ML_template`, `S_template`, `empty` |
| 3 | `MDLVW_*` |
| 4 | `MapBase` (`mapbase.gparam`) |
| 5 to 9 | the five layers of the block, layer 0 to 4 |
| 10 | `MapOverride_NNNN` |
| 11 and up | remo, dying and `eclipse` banks |

- A block registers every file under its bank, `((area + 100) * 100 + block) * 10000`, in a slot indexed by priority (up to 21 slots).
  `MapBase` sits under the block layers, so it supplies only what the block files do not author.
- The evaluator (`17b9a0`) walks the slots in ascending priority for every bound field of a record.
  A slot whose file carries the record's row for that field writes the value; a higher slot overwrites a lower one.
  The unit is the field and the row: the whole time-keyed list of one value id comes from one file.
- Rows (`(bank, row)` records) are created by the registration of each file for the value ids of that file's first parameter of the group.
  The union over the stack is the set of rows; `mapbase` registers 0, 100, 110, 120 and 130 for every block, so row 120 exists in Outskirts although the block file never registers it.
- A new record starts from the template record and a field that no slot writes keeps that default (`1403125b0` copies the template, `140227f30` stages the copy).
  The template is the `(0, 1)` record, which falls back to `(1, 0)` and then to the built-in default blob.
  A missing field therefore never inherits row 0 of the same file.
  The default blob is taken to be `default.gparam` (inferred, not traced to its loader).
- A lookup of a `(bank, row)` that was never registered returns the `(1, 0)` record (`168e10`), here the same default values.
- The collision's key is `(MSB block bank, first collision gparam int)`; the second int is the fog key.
- Layer 0 of a block is the file `m<area>_<block>_<d><sub>` named by the story sub-ID.
  Layers 1 to 4 take the four signed bytes of the collision `SceneGparam` struct as the hundreds digit `d` of the same family: `number = (d - base / 100) * 100 + base`, so `m11_00_0000` with byte 5 selects `m11_00_0500` and `m11_02_0930` with byte 5 selects `m11_02_0530`.
  A byte outside 0 to 9 (-1 in all installed data) leaves the layer empty; it does not keep the previous overlay.
  The fourth `SceneGparam` float is the fade time between old and new overlays; the loader switches at once (open).
- `mapoverride_0000` to `_0003` use GPARAM game id 3 (value ids without times).
  They parse now.
  Their LightSet group is empty and they would sit at priority 10, but the block field that selects `MapOverride_NNNN` was not found (no event instruction sets it), so they are not applied.

Implemented in `sekiro_formats::gparam::layer` (priorities in `gparam::priority`) and `lighting_selection::LayerFiles`.
`LightSet` rows get record defaults and the `(1, 0)` fallback (`Gparam::light_row`); every other group is layered per parameter and id but keeps the former fallback to row 0, because the row used by the other group managers was not recovered.
The stack is `default`, `mapbase`, the story file, then the overlays of the collision under the focus (`set_overlays`, from the layout's `scene_gparam`).
`SEKIRO_ABLATE=nolayers` restores the single-file lookup for before/after captures.
The read-only `gparam_inspect` example (`cargo run --release -p sekiro-formats --example gparam_inspect -- <file> [group] [param]`) lists the value ids each parameter carries, or the authored values with a parameter filter.

Effect on the installed Outskirts rows:

- Row 100 (133 of 166 collisions): `Hemi Color Up/Down`, `Angle1`, `Sharpness0/1/2` and `SsaoWeight0..3` are not authored by `m11_00_0000`.
  They now come from `mapbase` row 100 (sharpness 1 and 0.15 instead of the former default 50, SSAO weights 1, and the `mapbase` fill direction instead of the row-0 one).
  The sun and fill colours and powers are the block's own row-100 values and are unchanged.
- Overlays `0100`, `0300`, `0400` and `0600` to `0900` author LightSet fields for row 0 only (`0200` has none), so for row-100 collisions they change the tone-map and post groups (which keep the row-0 lookup) but not the row-100 light.
  `0500` also authors rows 100, 110 and 120; it is selected by four Outskirts collisions in layer 2, including the ones at spot-002 and spot-003.
  Installed layer-2 selections: file 8 for 29 collisions, file 7 for 14, file 1 for 10, file 5 for 4 and file 3 for 4; layer 1 files 9, 8, 1 and 6 are used by a few.
- `mapbase` also supplies post fields that the Ashina block files omit, most visibly `ToneMapMode` 8 (the old fallback was 7), tone contrast, Reinhard contrast and white balance, `ColorH/S/V`, auto-exposure adaptation speeds, `LightMask Influence` and the SSAO block.
  221 parameters with values exist only in `mapbase` for `m11_00_0000`.
- The measured default-render effect is in [EXPOSURE.md](EXPOSURE.md#layer-stack-result-2026-10-15): the noon row-100 sun is still 4.4 times row 0 and scenes stay 2 to 5 times retail.

## Sun shadow range and camera rotation, 2026-10-17

Playtest: at the Outskirts first gate, rotating only the camera changed which parts of a distant cliff were lit, and `SEKIRO_ABLATE=nolocal` did not remove it.
The cause was the sun shadow range, not the shadow map fit.
Bevy fits each cascade to a bounding sphere and snaps it to texels, so the cascade content is stable under rotation.
But `fetch_directional_shadow` returns fully lit for any fragment whose view depth is past the last cascade, and our last cascade ended at 60 m.
That cut is a plane facing the camera, so turning the camera swept it across the cliff and switched its shadowed parts on and off.
Native `default.gparam` `ShadowParam` authors cascade cut points 10, 30 and 70 m with `Far Fade Start` 1000 m, and the m11_00 files do not override them.
The sun now uses four cascades with bounds from those fields (10/30/70/1000 m by default), so receivers keep the shadow lookup to the native far fade.
Map shadow casters are no longer budgeted by distance from the player: with 45 m, and later 300 m, ground and cliff shadows popped in on approach (#66).
Bevy already culls casters against each cascade, so the default leaves that to it; a playtest on 2026-10-17 held 60 fps with casters to 1000 m.
`--shadow-casters <metres>` brings back a budget around the player, and offscreen captures load casters within that range, or the shadow distance by default.
`--shadow cascades,distance` still overrides the count and range, using the native cut points below that range.
The cut points and far fade are re-read from the light file in force whenever `apply_lighting` applies a light-file or hour change, and the sun's `CascadeShadowConfig` is rewritten only when the bounds differ; a `--shadow` range is kept across those changes (#65).
The native far fade (10 m) and `Cascade Fade Range Scale` are not reproduced; Bevy keeps its own cascade overlap.


## Per-part light sets, 2026-10-17

Outdoors looked flat and evenly bright, like a room lit by LED panels (#30).
Native lights each instance with its own light set, not one scene-wide set.
The deferred light shader picks a light record per pixel from `GBuffer3.w`, which each instance writes from its own `envData` index into the scene's list of light sets (`1f3b20` builds one 288-byte record per entry).
Every draw carries its own (bank, id) pair (`19a690`), and `168e10` resolves id 0 as an ordinary row; only (-1, -1) is remapped.
The code that copies a part's MSB light set into its draw was not found, so that last link is inferred, not traced.

Every cached map authors light set 0 on its map pieces and objects, -1 on enemies, and 100, 110, 120 or 130 on collisions.
In the Outskirts at hour 6, row 0 authors its LocalLight colours with W 0 and half the row-100 sun.
We now draw map pieces and objects with row 0, and characters and the few parts authoring another set with the collision-selected row.
The collision row still drives post, fog, sky and the shadowed sun direction, which matches row 0 at hour 6.
Parts authoring a set above 0 probably use their own row natively; we give them the collision row for now.

`SEKIRO_ABLATE=scenelight` restores the old single scene set, and `SEKIRO_LIGHT_SET=<id>` now forces that row on every surface.

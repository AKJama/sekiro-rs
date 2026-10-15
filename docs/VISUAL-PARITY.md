# Visual parity work list: Ashina Outskirts and Castle

This is the reviewed work list of visible differences found between the real game and our renderer in Ashina Outskirts (m11_00_00_00) and Ashina Castle (m11_01_00_00), grouped by cause and ranked by reach and visibility.
The original work list is retained as historical evidence below the current story-state review.
The reference is the user's own 2560x1440 screenshot collection (game 1.06.0, overcast daytime, falling snow, Outskirts through Ashina Castle Gate and Castle through the Upper Tower), indexed as described in `tools/reference_index/README.md`.
Screenshots, matched camera poses, side-by-side pair images and per-pair visibility dumps live outside the repository under the external reference root; nothing from them is copied into source.

## Reference state and snow remeasurement, 2026-10-15

The reference is overcast daytime.
Use `genichiro-defeated`, selected by boss progression and the installed common event, for reference matching, exact-camera pair regeneration and the canonical Ashina gates.
The two candidate saves do not render identically.

| Candidate | Known global defeat flags | Clock selected by the data | Both maps: environment / sub-ID / sky |
| --- | --- | --- | --- |
| Gyoubu defeated, Genichiro alive | 9301 | Morning, 06:00 | 1 / 0 / m900000 |
| Both defeated | 9301, 9303 | Afternoon, 12:00 | 2 / 0 / m900010 |

Common event 820 counts defeated bosses 9303-9306, raising 9800 after one and 9801 after three; Gyoubu's 9301 is outside that range.
Common event 100 then selects the environment and sky, while both candidates retain their area's ordinary base GPARAM.
The named map-story snapshots also retain the known local defeat writes and Genichiro's completed-cutscene guard 9403.
The installed-data regression verifies these writes and both maps' resolved hour, environment, sub-ID, file and sky; see [LIGHTING.md](LIGHTING.md#reference-progression-audit-2026-10-15).

The Castle Tower Lookout travel destination, loading-screen Attack Power 3 and pale overcast skyline favour the post-Genichiro candidate.
The eight-view `candidate-stories.jpg` comparison shows the Gyoubu-only candidate's orange/pink morning sky and warm directional wash, versus the afternoon candidate's closer pale sky and cool fill.
Indexed playtime is 01:54:03.34 at spot 002 and 02:24:17.94 at spot 092; playtime is not the world-lighting hour.
This selects the best-supported candidate without claiming a recovered save snapshot.
Unknown quest, idol and interaction flags remain unset, and the gate at spot 053 still differs from the reference's open leaf.

### Current capture and review

All 51 previously accepted pairs were replayed into `pairs-genichiro-defeated/`, preserving each saved camera, player position and FOV without a new search or hour override.
The original `pairs/` and original `poses.json` are retained untouched.
The renewed ledger accepts 50 pairs at 47 distinct spots and rejects spot 046/shot 164 because foreground obstruction prevents enough shared camp landmarks from being identified.
That rejection contributes no current defect count, and its cause is not inferred from the image.
The other broad camera matches retain perspective offsets, so they support regional measurements and triage rather than subpixel geometry claims.
Replay records the original camera-fit score as a saved score, with its source executable identified separately; no new camera fit is claimed.
Wolf, NPCs and HUD differences remain excluded.

The measured comparison executable SHA-256 is `f8e67514fb15979464ce3de41c7a116ff97d5de39815706c633377fbbf9bd18c`.
A late parallel material edit added native point/spot response after the first controlled capture set, so the accepted daytime pairs, night controls and measurements were refreshed for the current build.
The original controlled story comparison and its measurements remain separately archived in `controlled-before-native-points/`.
The two-candidate sky comparison uses that internally consistent controlled build; the event-selected hours and sky groups are unchanged by the parallel lighting work.
Subsequent concurrent FX and fog edits are outside this comparison snapshot; the separately identified final gate build includes the later FX-light colour packing present when it was built.
This job changes state selection and tooling, and makes no tone, exposure, fog or material implementation edits.

The canonical gate also adopts this state because it is the state supported by the reference evidence.
Every controlled geometry triple and selected collision is retained, every old visible-part set is a subset of the new set, and no visible count decreases.
Only named-state metadata and reviewed colour cells change in the render fixtures.
Per-view evidence, additional enabled story props and the final gate results are recorded in [MAP-COMPLETENESS-SWEEP.md](MAP-COMPLETENESS-SWEEP.md#reference-story-fixture-review-2026-10-15).

### Current ranking

These counts come from the renewed daytime ledger and supersede the historical night ranking below.
Priority remains distinct accepted spots times visual prominence, with overlapping symptoms counted in their relevant categories rather than treated as independent pixel measurements.

| Rank | Current cause | Spots / pairs | Prominence | Priority |
| ---: | --- | ---: | ---: | ---: |
| 1 | L1: Residual lighting and exposure | 47 / 50 | 5 | 235 |
| 2 | M1: Snow, ground, rock and plaster response | 46 / 49 | 4 | 184 |
| 3 | F1: Mid-distance haze and backdrop response | 36 / 37 | 4 | 144 |
| 4 | E1: Fire, smoke, lantern and idol response | 24 / 26 | 4 | 96 |
| 5 | M2: Foliage and tree snow response | 22 / 23 | 3 | 66 |
| 6 | W1: Water surfaces | 2 / 2 | 5 | 10 |
| 7 | O1: Object cloth and prop pose | 2 / 2 | 4 | 8 |
| 8 | S1: Reference story and interaction state | 1 / 1 | 5 | 5 |
| 9 | O2: Sky gap between gate leaves | 1 / 1 | 3 | 3 |

The old broad missing-grass count is withdrawn because low clumps are visibly present in the current pairs.
Their density, placement and treatment still require local evidence before any new geometry count is assigned.
The painted range is visible, so residual horizon contrast and profile differences remain under F1 without an unsupported missing-mass count.
Ordinary weather particles are also visible; unmatched single-frame particle phases do not establish a missing-snowfall count or certify native density, motion or indoor occlusion.
Fire/smoke shape and local emission balance remain E1 residuals in the measured build.

### Measurements

The six requested spots are 002/004, 003/012, 009/034, 025/085, 091/290 and 092/293.
The four additional snow-heavy spots are 008/033, 013/044, 089/282 and 090/288.
All measurements use original 2560x1440 PNG pixels, exact inverse-sRGB Rec.709 luminance, mean display-HSV saturation and saturation-weighted circular hue, following [EXPOSURE.md](EXPOSURE.md#measurement-method).
The six existing scene masks are reused, and the four additional views use the matcher's fixed overlay/player exclusion mask.
Snow uses separate manually reviewed polygons, selected from surface identity before reading results and without brightness or colour thresholding.
The same polygon is applied to real and ours, excluding characters, HUD and obvious emitters.
These are visible snow-covered regions including tile gaps, dirt, sparse vegetation, shading and local haze, rather than isolated albedo samples.
The smallest snow region is the distant alley patch at spot 092, with 5,655 pixels; all ten masks and real/ours overlays are retained for review.
Y and saturation are percentages; hue is degrees with red at zero.
Each pair in the table is real / current ours.

| Spot | Scene Y real / ours % | Scene S real / ours % | Scene hue real / ours degrees | Snow Y real / ours % | Snow ratio |
| --- | ---: | ---: | ---: | ---: | ---: |
| spot-002 | 7.021 / 30.562 | 23.96 / 19.75 | 202.91 / 205.50 | 20.247 / 87.213 | 4.31x |
| spot-003 | 7.506 / 42.151 | 23.90 / 19.87 | 209.90 / 209.84 | 19.218 / 90.562 | 4.71x |
| spot-009 | 10.473 / 23.276 | 22.07 / 26.23 | 206.72 / 212.70 | 33.299 / 91.219 | 2.74x |
| spot-025 | 10.602 / 26.279 | 17.63 / 21.37 | 206.31 / 211.29 | 37.417 / 97.900 | 2.62x |
| spot-091 | 19.733 / 49.871 | 15.43 / 15.81 | 197.00 / 204.66 | 11.730 / 56.371 | 4.81x |
| spot-092 | 4.347 / 20.841 | 22.50 / 26.07 | 196.32 / 207.85 | 21.059 / 71.504 | 3.40x |
| spot-008 | 7.543 / 20.798 | 24.41 / 25.08 | 209.53 / 214.03 | 39.012 / 94.160 | 2.41x |
| spot-013 | 12.464 / 39.283 | 17.32 / 17.66 | 204.76 / 210.55 | 12.755 / 58.736 | 4.60x |
| spot-089 | 22.309 / 53.451 | 19.76 / 14.87 | 198.14 / 203.44 | 24.886 / 91.110 | 3.66x |
| spot-090 | 12.433 / 68.074 | 22.38 / 12.24 | 201.54 / 203.91 | 10.996 / 90.351 | 8.22x |

Every measured scene remains brighter than retail, by 2.22x to 5.62x mean linear luminance.
Every snow region is brighter, by 2.41x to 8.22x; 6 of the ten current snow-region means exceed 90 percent linear white.
The very bright roof at spot 090 therefore remains a measured problem after selecting the correct story.
Saturation errors have both signs, and spot 003's whole-scene hue is already close to retail, so a global warmer tint or desaturation is not established by these statistics.
Whole-scene hue does not identify a pure snow material colour.
The complete JSON retains real, same-build night and current-state scene and snow Y/S/H, pixel counts and luminance P95.

### Brightness audit and unit bridge, 2026-10-15

The cause search is in [EXPOSURE.md](EXPOSURE.md#brightness-audit-2026-10-15).
It proves one cause and fixes it: the lux-per-unit bridge was fitted (4500) and the traced native arithmetic gives `pi / exposure = 3135.61`, which lowers every light, hemisphere, probe, sky and fog term by 0.697.
It also localises the rest of the excess to the directional light slots, which are about 4 to 7 times too bright against retail even though the sky, fog and ambient units now agree with retail.
Each pair below is retail / before / after, using the same masks and formulas as the Measurements table above; saturation is mean display-HSV percent and hue is degrees.

| Spot | Scene Y % | Scene saturation % | Scene hue | Snow Y % |
| --- | ---: | ---: | ---: | ---: |
| spot-002 | 7.0 / 29.4 / 23.9 | 24.0 / 20.1 / 21.1 | 203 / 206 / 207 | 20.2 / 87.5 / 78.3 |
| spot-003 | 7.5 / 40.9 / 34.4 | 23.9 / 19.9 / 21.5 | 210 / 211 / 210 | 19.2 / 90.8 / 82.4 |
| spot-009 | 10.5 / 23.1 / 17.7 | 22.1 / 26.3 / 26.6 | 207 / 213 / 213 | 33.3 / 91.2 / 85.2 |
| spot-025 | 10.6 / 24.0 / 18.6 | 17.6 / 22.4 / 22.8 | 206 / 212 / 213 | 37.4 / 92.5 / 88.1 |
| spot-091 | 19.7 / 45.8 / 36.6 | 15.4 / 16.6 / 19.2 | 197 / 207 / 207 | 11.7 / 56.7 / 44.0 |
| spot-092 | 4.3 / 20.9 / 14.3 | 22.5 / 26.1 / 26.2 | 196 / 208 / 208 | 21.1 / 71.5 / 57.8 |
| spot-008 | 7.5 / 20.9 / 16.5 | 24.4 / 25.0 / 25.0 | 210 / 214 / 214 | 39.0 / 94.2 / 88.2 |
| spot-013 | 12.5 / 37.3 / 28.6 | 17.3 / 18.6 / 20.2 | 205 / 210 / 210 | 12.8 / 58.8 / 46.1 |
| spot-089 | 22.3 / 51.6 / 40.7 | 19.8 / 14.8 / 17.9 | 198 / 205 / 206 | 24.9 / 91.1 / 84.4 |
| spot-090 | 12.4 / 68.2 / 58.3 | 22.4 / 12.1 / 15.4 | 202 / 204 / 205 | 11.0 / 90.4 / 82.3 |

The scene excess over retail falls from 2.2-5.5 times to 1.7-4.7 times, and the snow excess from 2.4-8.2 times to 2.3-7.5 times.
Real photographs of spot-002 and spot-091 show an overcast scene with soft or absent sun shadows and thin snow over grey gravel, so part of the snow-region excess is also coverage (M1 item 3), not light.
Panels and measurements are in the external `2026-10-15/brightness/` collection.

### L1 Current residual lighting and exposure

Current reach is 47 accepted spots.
Snow, plaster, stone and timber remain over-bright in the daytime pairs, so the residual is broader than a snow-only texture or tint issue.
The confirmed night-reference mismatch is removed from the current cause list.
Ranked remaining candidates are:

1. The directional light slots (sun, fill, local 1 and 2).
   The scene-radiance unit is now traced (`SCENE_TO_BEVY = pi / exposure`) and checked against the sky; the ablations show retail carries only about 0.1 to 0.25 of the native `Col * albedo * N.L` on open lit surfaces, with the sky, fog and ambient units agreeing.
   Candidates are the deferred AO multiplying the whole irradiance (we have no SSAO and no baked occlusion outside mountains), the untraced `FC_ShadowColor` default and the untraced `FC_SsaoWeight` source; see [EXPOSURE.md](EXPOSURE.md#remaining-unexplained-factor).
2. Native ambient/probe, SSAO and shadow-colour response.
   These can explain different local contrast and fill; the prior native audit still establishes sun-shadow attenuation only for directional slot 0, rather than justifying shadows on every extra slot.
3. Metering reduction layout, reset/adaptation history and the retail brightness setting.
   Existing formula audits remain evidence, while a fixed screenshot comparison cannot recover every real-game history or configuration input.
4. Distance-specific atmosphere and instance inputs under F1.
   These affect distant views but cannot alone explain the near snow patches at spots 002 and 003.

These are investigation priorities, not proven replacement parameters.
The varying scene and snow ratios do not supply a supported exposure offset, radiance multiplier or new tone curve.

### M1 Current snow and surface response

Current reach is 46 accepted spots.
The separate snow measurements confirm the user's bright-snow observation on ground, exposed rock, roof tiles and the distant alley patch.
Ranked remaining candidates are:

1. L1's directional-slot excess and highlight compression erase fine dirty breakup when large snow regions approach display white.
   After the traced bridge the snow regions are still 2.3 to 7.5 times retail, and 7 of the ten means remain above 78 percent, so coverage and saturation are still unreliable evidence for changing an authored snow mask or albedo.
   The retail spot-002 and spot-091 photographs show thin snow over grey gravel and tiles, while ours renders thick white cover on the same surfaces, so layer coverage (item 3) is now ranked as a joint cause of the snow-region numbers.
2. Native view-dependent BRDF, roughness/gloss and detail-normal response, including the generic handling of separate `M[Snow]`/`Snow_mb*` families.
   The large roof/ground ratio variation warrants this isolation after the shared radiance path is understood.
3. Layer coverage, mask/UV selection and snow-normal facets, plus local occlusion around tile gutters, stones and vegetation.
   The data audits already establish authored textures, neutral tints and several correct facet switches; the images do not prove that every affected material has the same defect.
4. Albedo/tint or grading compensation.
   The known slightly blue ordinary snow albedo and neutral authored colour controls remain data, while these regional measurements do not establish a replacement tint, saturation value or fog multiplier.

### External evidence

The reference root contains `pairs-genichiro-defeated/poses.json`, direct full-resolution pair PNGs, individual `-ours.png` files, visibility records and the renewed `triage.json`.
The external progress collection's `2026-10-15/story/` directory contains `measurements.json`, `measurements.md`, `snow-regions.json`, scene/snow masks, ten `-snow-review.jpg` overlays, the renewed `parity-review.json` and generated `parity-summary.md`.
Its eight full-resolution `-real-night-genichiro-defeated.png` panels cover spots 002, 003, 008, 009, 025, 089, 091 and 092.
`candidate-stories.jpg` retains the controlled real/Gyoubu-only/both-defeated sky comparison.
`binary.json`, `source-at-capture.json`, replay metadata and per-part manifests retain capture provenance.
Build/check logs remain under `target/alt-story/`, and raw installed-event audits remain ignored under `re/research/story-2026-10-15/audit/`.

## Historical night-capture summary

- 114 matched pairs at 106 spots were reviewed: 51 accepted pairs at 48 distinct spots, and 63 rejected pairs.
- Accepted coverage is 29 Outskirts pairs at 28 spots and 22 Castle pairs at 20 spots; 43 of the 149 indexed spots have no rendered pair.
- These are observed lower bounds, not a claim that unpaired or rejected views have no defects; Upper Tower interiors in particular remain poorly matched.
- Widespread causes come first: night lighting/exposure, fog/sky response, snow/ground response, missing placed effects and falling snow.
- Lighting affects all 48 accepted spots; the other causes are counted only where their relevant surfaces, distances or effects are visible.
- Missing low vegetation and an absent or unreadable distant mountain range follow.
- Water geometry is present but looks opaque and pale; seven valley views are washed out but rejected for camera uncertainty, so they add no accepted defect count.
- Only a handful of items are single-spot problems: a story-state idol and gate, a camp tent canvas, a hanging banner, and a sky gap between gate leaves.
- Two matcher cameras landed on zero-demand helper collision (`h900000`, `h900500`) and rendered nothing; that is a tool and visibility-selection problem to fix before trusting those spots.

The "what is there" track owns geometry, visibility, distant models, neighbouring blocks, fog surfaces and export.
The "how it looks" track owns lighting, sky, volumetric fog, materials, exposure and effects.

## How the evidence was gathered

The matcher (`tools/reference_index/match_poses.py`) searches our windowless renderer for the camera that best matches each screenshot, starting from the practice-tool player position in the index.
It writes one `pairs/<spot>-shot-<n>.png` per matched shot (real | ours | edge difference, 2560x1440 per panel), a `.search.json` with the ranked candidates and a `.visibility.json` with the per-part visibility decision that produced the render.
The match score is a structural edge cost and is not proof of a correct camera, so this review inspected all existing pair sheets and the additional Castle pairs, accepting only when the same landmarks (walls, roofs, gate frames, rocks, trees, stairs) line up in both panels.
Pairs whose camera faces a different structure, the ground or a wall are listed under rejected pairs and are not counted.
Wolf, NPCs, enemies, corpses and the HUD are ignored throughout.
Counts below are distinct accepted spots with observed evidence, not pixels, visibility-dump totals or an extrapolation across the area.
Acceptance requires identifiable shared landmarks; it supports broad visual triage, not subpixel camera calibration or a claim that a different foreground prop is missing.
Several usable pairs retain small perspective offsets, so projected bounds identify candidates rather than pixel ownership.
The final per-pair decisions and per-cause spot sets are preserved in the external progress root as `2026-10-15/parity-review.json`.
That ledger supersedes the incomplete inherited `pairs/triage.json`; the reference collection is the source, not the destination for the final review.

The matcher's first 101 pairs were rendered with the renderer build recorded in `poses.json` (`renderer_sha256` starting 994e411c); the Castle pairs added for coverage on 15 October used the build starting 361b6dec, which includes the in-progress fog work, so their fog and sky reading can differ from the earlier pairs.

## Historical night-capture work list

Priority is accepted spot count multiplied by visual prominence (5 fills or transforms a major part of the frame, 4 conspicuous, 3 local, 2 individually small but widespread).
This places area-wide causes before isolated defects; rows are overlapping evidence, so their counts must not be added.

| Rank | Item | Kind | Accepted spots | Prominence | Priority | Owning fix track |
|---:|---|---|---:|---:|---:|---|
| 1 | [L1 Night lighting and exposure](#l1-night-lighting-and-exposure) | lighting and exposure | 48 | 5 | 240 | how it looks |
| 2 | [F1 Fog, haze and sky](#f1-fog-haze-and-sky) | fog, sky and fog surfaces | 38 | 5 | 190 | how it looks |
| 3 | [M1 Snow, ground, rock and plaster response](#m1-snow-ground-rock-and-plaster-response) | materials and textures | 47 | 4 | 188 | how it looks |
| 4 | [E1 Fire, lantern, ember and idol effects](#e1-fire-lantern-ember-and-idol-effects) | effects and local light | 32 | 5 | 160 | how it looks |
| 5 | [E2 Falling snow](#e2-falling-snow) | weather effects | 47 | 2 | 94 | how it looks |
| 6 | [G1 Grass, weeds and ground clutter](#g1-grass-weeds-and-ground-clutter) | missing foliage geometry | 23 | 4 | 92 | what is there |
| 7 | [D1 Far mountain range absent or unreadable](#d1-far-mountain-range-absent-or-unreadable) | distant scenery | 5 | 5 | 25 | what is there |
| 8 | [M2 Foliage and tree snow](#m2-foliage-and-tree-snow) | materials and textures | 5 | 3 | 15 | how it looks |
| 9 | [O1 Object cloth and prop pose](#o1-object-cloth-and-prop-pose) | geometry and Object pose | 3 | 4 | 12 | what is there |
| 10 | [W1 Water surfaces](#w1-water-surfaces) | water material | 2 | 5 | 10 | how it looks |
| 11 | [S1 Reference story and interaction state](#s1-reference-story-and-interaction-state) | visibility and story | 1 | 5 | 5 | what is there |
| 12 | [O2 Sky gap between gate leaves](#o2-sky-gap-between-gate-leaves) | Object geometry or pose | 1 | 3 | 3 | what is there |

### L1 Night lighting and exposure

Kind: lighting and exposure.
Track: how it looks.
Seen at 48 accepted spots.

What is wrong:

- Hard, bright directional illumination produces crisp shadows, so snow and white plaster blow out to blue-white while shadowed rock, wood and interiors crush to near black.
- The real game is soft overcast daytime: snow is light grey with texture, shadows are wide and open, and rock, wood and stone keep their warm brown and grey albedo.
- Everything in ours sits in a cold blue cast; retail is neutral to slightly warm, with warm pools around fires and lamps (see E1).
- Sheltered places are nearly unreadable in ours: the small shrine at spot-003 is black, the hut at spot-016, the great hall at spot-044 and the gatehouses at spot-053, spot-056 and spot-066 lose all interior detail.
- Moonlight-like bright patches with sharp shadow edges appear on ground, roofs and walls (spot-025, spot-092, spot-094 tree shadow on the moat); retail has none at night.

Likely cause: the night light bank selection, sun or moon intensity and colour, ambient or probe level, exposure metering and grading are not the retail values; isolate direct, indirect and exposure one at a time against `docs/LIGHTING.md`.

Examples: `pairs/spot-002-shot-004.png`, `pairs/spot-003-shot-012.png`, `pairs/spot-025-shot-085.png`, `pairs/spot-092-shot-293.png`.

The 2026-10-15 [exposure follow-up](EXPOSURE.md) corrects a proven mode-7 exponential-base error and measures six controlled retail/before/after views.
Luminance moves closer in four views and farther away in two, while saturation and B/R worsen in all six; L1 and M1 remain open.
The subsequent [post-chain colour audit](COLOUR.md) implements the missing pre-gamma colour matrix and full authored bloom pyramid, while preserving the exact-night neutral correction defaults and verified LUT sampling.
Its new six-view measurements retain the residual brightness and blue-snow limitations rather than fitting a tint or exposure compensation.

### F1 Fog, haze and sky

Kind: fog, sky and fog surfaces.
Track: how it looks.
Seen at 38 accepted spots.

What is wrong:

- No aerial perspective: distant cliffs, the castle and far rooftops stay crisp and dark where retail fades them through pale grey-blue haze (spot-006, spot-009, spot-011, spot-023, spot-030, spot-048, spot-093).
- Gorges and valleys have no mist bands or cloud sea (spot-008, spot-018, spot-020, spot-050).
- The sky is a flat dark teal field; retail is a pale, bright overcast with visible cloud texture, and it is the brightest thing in most outdoor frames (spot-009, spot-011, spot-022, spot-067, spot-091).
- Because the sky is dark, silhouettes against it (trees, roofs, far towers) read inverted: dark on dark instead of dark on pale.

Likely cause: the capture builds have incorrect atmosphere/sky integration; the concurrent fog job has identified enable lookup, light-bank and sky-continuation faults, so these frozen captures are not a verdict on its latest changes.
Keep volumetric fog, sky and authored fog surfaces together as this one item, with the how-it-looks owner coordinating any surface/export work with what-is-there.
In `cache/maps/m11_00_00_00/layout.json`, `m900100_9000` and `m900101_9000` are placed near `(-90, 3, -40)`, not the origin; their inspected GLBs identify `M[m11_00]_GroundFog`, so they must not simply be labelled CloudFog from their names.
`m900030` and the `m89` sky library are sky candidates; use the retained material audit to distinguish sky/cloud and landscape fog pieces.

Uncounted valley symptom, formerly A1: spots 031-037 are heavily washed out, with 2289 visible parts reported for spot-031.
Camera headings and recognizable landmarks fail review there, so all seven pairs are rejected and contribute zero accepted spots to F1.
Examples for that diagnostic only are `pairs/spot-032-shot-116.png` and `pairs/spot-034-shot-123.png`.
Visibility counts alone do not establish that their intended cliff/shrine geometry reached the camera; isolate surface fog versus volumetric fog after a valid camera is obtained.

Examples: `pairs/spot-011-shot-037.png`, `pairs/spot-009-shot-034.png`, `pairs/spot-093-shot-296.png`, `pairs/spot-006-shot-026.png`.

### M1 Snow, ground, rock and plaster response

Kind: materials and textures.
Track: how it looks.
Seen at 47 accepted spots.

What is wrong:

- Ground snow reads as near-uniform bright blue-white and loses much of its fine dirty breakup; retail snow is grey-white with footprints, dirt, leaf litter and brown earth showing through (spot-002, spot-003, spot-013, spot-025, spot-058, spot-123).
- Stepping stones and paths render black without snow dusting (spot-002).
- Rock faces show sparkly white speculars and glossy facets (spot-008, spot-018, spot-020, spot-050).
- Puddles and wet ground are mirror-glossy (spot-058, spot-066).
- Mossy stone in the moat and sluice is saturated yellow-green; retail is grey stone with a faint moss tint (spot-073).
- White plaster walls blow out (spot-013, spot-060, spot-079, spot-092, spot-094); this overlaps L1 but the plaster's dirt and streak layer is also much weaker.

Likely cause: layer selection, masks, UVs, detail normals, roughness, snow/wet terms and illumination need isolation against the native material extras.
Layered material handling already exists, so the images do not establish that all surfaces use a single albedo or default roughness.
Spot-025's visible candidates include `m005100_2023` and `m005120_2137` with `M[m11_00]_Ground1`; spot-022 includes `m004951_2001` with Ground1/Steps1 and StoneWall families.
The layout, GLB extras and `cache/maps/<map>/material-audit.json` identify `Ground_Snow1`, `M[Snow]`, `Snow_mb*`, Rock and StoneWall families for follow-up.
Separate L1's exposure/lighting contribution before changing authored material coverage.

Examples: `pairs/spot-003-shot-012.png`, `pairs/spot-013-shot-044.png`, `pairs/spot-050-shot-174.png`, `pairs/spot-073-shot-251.png`.

#### M1 data audit - 2026-10-15

The subsequent [exposure correction](EXPOSURE.md) changes the shared tone curve from a mistranscribed natural exponential to the native base-two powers, without changing the authored snow or lighting inputs audited below.
It does not resolve the requested browner snow colour, and its measured colour limitations remain explicit.

Decision: retain the authored snow albedo, material tints, night lights and post settings.
The data establishes a slightly blue snow texture and strongly blue incident light, but does not supply a replacement contrast, scene-radiance scale or global saturation value that would establish retail parity.
No renderer adjustment is justified by these inputs alone, and M1 remains open.
This is an audit and diagnostic control, not a completed snow-tone fix.

The read-only inputs are both maps' base `cache/raw/param/drawparam/m11_0*_0000.gparam` files, `cache/maps/<map>/grading`, `env.json`, the selected variant-4 environment resources, material-audit records and exported GLB material extras.
The shared board already assigns native tone/exposure/night-snow calibration to RustSekiro; this local task does not replace that claim.
The local task claim, raw parameter census, material/facet census, texture statistics and LUT probes are retained under `target/alt-snow/audit/`.
No extracted asset is changed or copied into tracked source.

| Input | Outskirts | Castle | Interpretation |
| --- | --- | --- | --- |
| Night story hour / environment / sub-ID | 22 / 4 / 0 | 22 / 4 / 0 | Ordinary night base family, distinct from the morning first-visit gates |
| Grading enable / LUT | 1 / 20 | 1 / 20 | Both strips are present; existing selection and recovered strip-axis routing agree |
| Exposure stops / gain | -0.2 / 0.870551 | 0.3 / 1.231144 | Retain the authored gain rather than decreasing snow alone |
| Auto exposure / Middle Gray / bounds | 1 / -4.9 / -0.5 to 0.5 | 1 / -4.9 / -0.5 to 0.5 | Existing native post settings already consume these fields |
| Bloom threshold / alpha / saturation control | 0 / 0.5 / 0 | 0.05 / 0.3 / 0 | Sturate Ctrl is the bloom bright-pass interpolation, not a whole-image saturation control |
| LightSet row-0 primary diffuse RGBA | (0.450980,0.588235,0.764706,2) | (0.450980,0.588235,0.764706,1.5) | Squared RGB times W is blue: approximately (0.406767,0.692042,1.16955) and (0.305075,0.519031,0.877163) |
| Hemisphere up RGBA | (0.4284,0.473573,0.51,0.5) | Same | Also authored cool; changing its hue would contradict the data |
| Hemisphere down RGBA | (0.4128,0.4464,0.48,0.5) | Same | GI coverage replaces this fallback in the existing shader |

The matched spot-002 camera selects collision `h022700`, whose native LightSet is 100, with independent fog/scattering/environment bank IDs all zero.
Its night primary diffuse is `(0.431373,0.549020,0.764706,4)`, becoming approximately `(0.744330,1.205691,2.339101)` before the common scene bridge.
Its fill is the same RGB at W 0.25, and local slot 1 is `(0.676345,0.790052,0.9265,1)`.
Thus row 0 is a catalogue comparison, not a substitution for the actual capture bank.

Night LUT 20 maps neutral display inputs 51, 102, 153, 204 and 255 to `(42,47,49)`, `(98,102,102)`, `(156,158,158)`, `(209,210,208)` and `(255,255,255)` respectively in both maps.
It slightly cools shadows and is nearly neutral in the highlights; it does not explain a large blue-white highlight cast on its own.
The current shader's red-fastest `(r,16*b+g)` strip layout, trilinear interpolation and ordering after ordinary output power agree with the native shader recovery in `SHADERS.md`.
No authored global saturation or contrast field is present in these base files' tone/bloom/grading groups.
Tone mode 7 and FilmicContrast 0.5 therefore remain traced constructor defaults, rather than new values selected from the screenshots.
The unresolved colour-matrix/vignette producers, native LUT blending and former inferred `SCENE_TO_BEVY=4500` bridge (now traced) do not provide a supported replacement number.

The shared `m11_snow_01_a` top mip has mean byte RGB `(228.81,233.11,243.13)` and per-channel 10th/90th percentiles `(222,227,239)` / `(233,237,247)`.
The dedicated `m11_snow_20_a` has mean `(225.94,226.11,225.94)` and percentiles `(219,220,219)` / `(231,231,231)`.
These are full-texture statistics before material masks, lighting and grading, not measured retail snow colours or desired output values.
The BC1 sRGB payload is decoded read-only in memory for this census; the runtime continues to sample the existing albedos as sRGB and the raw normal/mask textures as linear data.
Thus the texture itself supports some blue in ordinary snow and fine brightness variation, but not removing warm colour from unrelated wood and ground.

Concrete layered facets include Outskirts `m005100` Ground1, MultipleGround variant 16, with `m11_snow_01_a`, its raw snow normal and a white snow tint `(1,1,1)`.
Its authored snow direction is `(-0.2,0.85,0.2)` and height-mask selector `(0,0.5,0.7)`; those inputs already reach the layer shader.
Its Rock1 facet is Multiple variant 20, with the same snow albedo, tint `(1,1,1)` and direction `(0,1,-0.25)`, without an enabled snow-normal slot.
Castle pavilion `m015070` includes the previously verified dedicated SnowGround MultipleGround-6 facet using `m11_snow_20_a`; roof deposits use MultipleGround-18, while snow-free tatami uses Multiple-1.
The retained facet switches are evidence against globally reducing snow coverage or adding an invented grey multiplier.
Separate `M[Snow]`/`Snow_mb*` families and native gloss/normal response still have generic-material limitations; their names alone do not establish replacement arithmetic.

The broad blue cast and extreme light/shadow contrast remain shared L1/material-response candidates.
Authored cool albedo and illumination contribute to colour, while the exact excess relative to retail is not isolated by static parameter agreement.
Keep unknown radiance/BRDF, normal/detail, matrix and post-composition terms explicit instead of compensating through a map tint, arbitrary desaturation, stronger fog or geometry changes.

Matched night diagnosis uses the external spot-002-shot-004 camera `[-138.888512,-49.454685,-154.160263,353.307100,-0.534677]`, player `[-138.41,-50.993,-158.238]`, 43-degree FOV and 1600 by 900 output.
All four controls retain equal poses, FOVs, selected collisions and sorted visible-part identities.
Disabling grading changes full-frame RGB by mean absolute 3.515/255, with maximum channel difference 10/255.
In the fixed mixed foreground rectangle `(80,630)-(1440,855)`, blue/red changes only from 1.40380 to 1.39034, so the cold cast remains without grading.
Disabling layers removes authored snow deposits and detail, with foreground blue/red still 1.33971; this is a destructive diagnostic of the material contribution, not a parity correction or an isolated test of snow albedo.
These controls retain native tone/exposure, whose meter can respond to layer changes.
The rectangles contain multiple surfaces and are not pure-snow measurements, retail target colours or tuning objectives.
The independent baseline and final audit-repeat captures are pixel-identical because no rendering setting or implementation changed.
They are saved as `target/alt-snow/progress/snow-before.png` and `snow-after.png`; the latter means after the audit, not after a claimed visual fix.
Both are also exported under the requested date in the external progress root, with matching SHA-256 `a69c8df0e0f081a41cf13934ee841d45846061a5992c705d7de23c7429c38bfb`.
Reference pairs were inspected in place without copying their images into the repository.

Both canonical six-view gates use the unchanged first-visit story, 128 by 72 fixtures, per-category tolerances of 16 pixels or 1% and 120-second budget.
They pass in 72.902 seconds for Outskirts and 80.769 seconds for Castle, using `target/alt-snow/release/sekiro-rs.exe`, SHA-256 `c7c7c0efbbef606738592d1e06c8c8e8c400854f9a1f82935ee3c7de6fddb93f`.
Outskirts totals `(void,depth_gap,untextured,grey,pop_in)` are `(0,493,0,2357,28850)`; Castle totals are `(1933,4885,0,1702,31552)`.
Every per-view geometry count, selected collision and sorted visible-part set equals the preceding FX job's final report.
Castle view 0 retains that preceding run's depth gap 148 versus fixture 149, a pre-existing one-pixel difference within the unchanged tolerance; the other eleven views equal their geometry fixtures.
All resident blocks retain zero failed texture requests.
No fixture, camera, source, material setting or tolerance changes in this audit.
The table compares the checked-in colour fixture with this fresh capture at each view.
These small threshold differences are observations against earlier captures, not effects of a snow correction; their individual particle/readback causes are not isolated.

| Map | View | Grey fixture / measured | Pop-in fixture / measured | Per-view interpretation |
| --- | ---: | --- | --- | --- |
| Outskirts | 0 | 274 / 274 | 8711 / 8650 | Grey delta +0; pop-in delta -61; within existing tolerances. |
| Outskirts | 1 | 183 / 189 | 294 / 295 | Grey delta +6; pop-in delta +1; within existing tolerances. |
| Outskirts | 2 | 383 / 378 | 2484 / 2486 | Grey delta -5; pop-in delta +2; within existing tolerances. |
| Outskirts | 3 | 725 / 725 | 5503 / 5422 | Grey delta +0; pop-in delta -81; within existing tolerances. |
| Outskirts | 4 | 648 / 641 | 6831 / 6756 | Grey delta -7; pop-in delta -75; within existing tolerances. |
| Outskirts | 5 | 147 / 150 | 5245 / 5241 | Grey delta +3; pop-in delta -4; within existing tolerances. |
| Castle | 0 | 196 / 205 | 5083 / 5097 | Grey delta +9; pop-in delta +14; within existing tolerances. |
| Castle | 1 | 288 / 288 | 7488 / 7488 | Equal colour counts. |
| Castle | 2 | 588 / 588 | 5879 / 5879 | Equal colour counts. |
| Castle | 3 | 49 / 51 | 3782 / 3782 | Grey delta +2; pop-in delta +0; within existing tolerances. |
| Castle | 4 | 482 / 480 | 2400 / 2362 | Grey delta -2; pop-in delta -38; within existing tolerances. |
| Castle | 5 | 92 / 90 | 6958 / 6944 | Grey delta -2; pop-in delta -14; within existing tolerances. |

The game release suite passes 126 unit tests and six integration tests with two existing ignores, including `map_data_gate` and `genichiro_settled_snow_selection`.
Workspace formatting and `git diff --check` pass.
The requested Cargo target directory and six-job limit are retained; captures and gates run serially as windowless processes after checking for other game processes.
The full game test run takes 257.38 seconds under concurrent host compilation, and is not a steady performance claim.
No renderer code changed, so this session does not claim a new Clippy result or a completed visual-parity fix.

#### M1 snow coverage arithmetic audit - 2026-10-15

The supported layer corrections below do not solve the thick-white appearance in the six matched views.
Their snow-region luminance changes range from -0.01 to +1.20 percentage points, while retail remains much darker.
M1 stays open; these measurements do not justify reducing an authored snow weight, mask selector, threshold or tint.

The evidence is the installed MTD/FLVER data, exported GLB extras and the native `M[Multiple]` / `M[MultipleGround]` Gbuf vertex and pixel shader disassembly in the ignored shader research collection.
Representative pixel facets checked are Multiple 11/20/27 and MultipleGround 6/16/18/27, including wet, overlay, extra ground layer and snow-normal switches.
The existing 34-facet sampler tables retain their authored selections.
The shared local WORK.md rendering claim covers the afternoon snow investigation; no shared-board commit or publication was made.

| Coverage input | Multiple | MultipleGround | Audit result |
| --- | --- | --- | --- |
| Snow vertex weight | Packed stream 4, G | Packed stream 4, G | Existing export is correct; this is TEXCOORD5.w after the native vertex shader |
| Overlay / extra layer | Stream 0 R / stream 1 B | Stream 0 R / stream 1 G | Existing export is correct |
| Detail / wet vertex weights | Stream 5 B / stream 3 R | Same | Existing export is correct |
| Snow direction | float3_7 | float3_5 | Mirrored X for Bevy, without normalizing the authored vector |
| Height selector | float3_5 | float3_3 | Dot product of linear mask-9 RGB, not an alpha or fixed single-channel lookup |
| Snow mask scale | float_32 | float_22 | Applied before the native 0.01 addition |
| Common height / sharpness | float_4 / float_5 | Same | Final saturation follows the complete height operation |
| Detail-normal amplitude | float_1 | float_23 | RG detail, weighted by the detail vertex channel, enters the facing normal |
| Wet-normal amplitude | float_0 | float_11 | Ground incorrectly packed float_12; corrected to native byte offset 448 |
| Snow normal | Slot 10, replacement | Slot 7, additive XY times float_31 | Static facets retain their distinct normal behavior |
| Texture tiling | Separate scale for every slot | Same | Exported non-unit scales were ignored by the layer shader; now uploaded and sampled |

The recovered coverage is `h = (dot(mask9.rgb, selector) * snow_mask_scale + 0.01) * height_scale`, `w = dot(snow_direction, bumped_world_normal) * snow_vertex_weight`, and `snow = saturate((w - h * (1 - w)) * (1 + sharpness))`.
Wet-enabled Ground facets multiply `w` by `1 - wet_vertex_weight * float_7` before the height operation, without an intermediate saturation; our shader omitted that exclusion.
Multiple has no corresponding wet exclusion from its snow coverage.
The height and sharpness packing, RGB channel selectors and snow vertex channel were already correct.
No separate invented snow threshold or contrast term is added.

The snow-facing normal incorrectly normalized only its tangent-space Z component before transformation.
It now transforms the complete base/overlay/ground-layer/detail normal with individually normalized basis vectors and normalizes the resulting world vector, matching the inspected shader operations.
Back faces flip the geometric normal before the facing calculation.
Ground decodes detail RG before multiplying its vertex weight; Multiple retains the native 0.502016 neutral instead of the former rounded 0.502.
Ground's flow normal follows snow and replaces the composed normal; Multiple adds flow before its replacement snow normal.
Ground wet albedo now follows snow and uses the recovered nested multiplication/interpolation rather than the previous approximate lerp toward wet colour.

Mask 9 uses instance-scaled UV0 with its own FLVER texture scale; Multiple mask 6 uses UV1 and its separate scale for extra-layer/wet/tint heights.
Detail uses unscaled-by-instance UV0 with its authored texture scale, and wet flow uses UV1 plus authored time scrolling before texture tiling.
The tiling omission affects real inputs, including masks scaled 2 by 2.2 and normal/detail textures scaled up to 16 in the audited Ashina snow materials.
The exporter now retains base-albedo scale as `base_uv`, the runtime uploads the twelve texture scales, and the material audit records the layer inputs and source channels.
Existing cached map metadata was refreshed from raw FLVER/MTD data in 933 GLBs, with each binary mesh/vertex payload unchanged byte for byte.

The inspected snow arithmetic has no active story/weather GPARAM snow-amount uniform.
Its inputs are the material constants, vertex channels, textures, instance UV scale and bumped normal; the active per-frame time only scrolls wet flow.
The dynamic wet parameter declarations in these shaders are unused, so they do not establish a weather-wide snow reduction.
This does not certify every possible CPU-side material override, separate M[Snow]/Snow_mb family or native tangent quantization path.
Native detail alpha contributes to Multiple's G-buffer occlusion rather than an additional snow-coverage mask; that occlusion response is outside this coverage correction.

All six captures use `genichiro-defeated`, the saved player/camera/FOV and 2560 by 1440 output, with one windowless renderer at a time.
Checked lighting, story, tone and fog source hashes are equal between the frozen before and after builds.
The production shader is verified verbatim in each executable; their digests and source comparison are retained externally.
Existing reviewed snow polygons and exact inverse-sRGB Rec.709 luminance are reused, without brightness-based pixel selection.
Small regional deltas can include native particle phase variation and do not measure pure albedo or isolated coverage.

| Spot | Retail snow Y % | Before Y % | After Y % | Change in percentage points |
| --- | ---: | ---: | ---: | ---: |
| 002/004 | 20.247 | 78.325 | 78.397 | +0.071 |
| 003/012 | 19.218 | 82.448 | 82.436 | -0.011 |
| 008/033 | 39.012 | 88.215 | 88.250 | +0.035 |
| 025/085 | 37.417 | 88.096 | 88.135 | +0.039 |
| 090/288 | 10.996 | 82.296 | 83.499 | +1.204 |
| 091/290 | 11.730 | 44.002 | 45.018 | +1.016 |

Real | before | after full-resolution panels, previews, individual renders, measurements and executable provenance are in the external progress collection's `2026-10-15/snow-cover/` directory.
The six panel stems are `spot-002-shot-004`, `spot-003-shot-012`, `spot-008-shot-033`, `spot-025-shot-085`, `spot-090-shot-288` and `spot-091-shot-290`, each ending in `-real-before-after.png`.
No lighting-selection source, render fixture or MAP-COMPLETENESS-SWEEP.md is edited by this task.

Both frozen before gates pass, in 82.424 seconds for Outskirts and 101.971 seconds for Castle.
After gates finish in 68.284 and 91.877 seconds, retaining the same geometry triples, selected collisions and sorted visible-part sets in all twelve views.
All twelve collision images and all twelve depth images are pixel-identical between before and after.
Void/depth-gap/untextured totals remain Outskirts 0/493/0 and Castle 1933/4885/0.
The full after gates reject three colour cells: Outskirts view 1 grey, Outskirts view 2 pop-in and Castle view 1 pop-in.
These failures are reported without editing the fixtures or their tolerances.
Normal and all-visible phases were reviewed separately in the external `outskirts-gate-review.png` and `castle-gate-review.png` sheets.
The colour deltas below are after minus frozen before; fixture deltas retain the small pre-existing capture variance as well.

| Map / view | Grey before -> after | Grey delta / fixture delta | Pop-in before -> after | Pop-in delta / fixture delta |
| --- | ---: | ---: | ---: | ---: |
| Outskirts 0 | 69 -> 65 | -4 / -4 | 2777 -> 2768 | -9 / -9 |
| Outskirts 1 | 467 -> 498 | +31 / +31 | 183 -> 179 | -4 / -4 |
| Outskirts 2 | 6 -> 7 | +1 / -1 | 2668 -> 2735 | +67 / +66 |
| Outskirts 3 | 2 -> 2 | 0 / -1 | 6494 -> 6422 | -72 / -76 |
| Outskirts 4 | 351 -> 350 | -1 / -1 | 6758 -> 6758 | 0 / -1 |
| Outskirts 5 | 0 -> 0 | 0 / 0 | 5231 -> 5222 | -9 / -9 |
| Castle 0 | 193 -> 194 | +1 / +7 | 4980 -> 4982 | +2 / 0 |
| Castle 1 | 44 -> 40 | -4 / -5 | 4932 -> 4985 | +53 / +55 |
| Castle 2 | 0 -> 0 | 0 / 0 | 5774 -> 5748 | -26 / -24 |
| Castle 3 | 0 -> 0 | 0 / 0 | 5409 -> 5466 | +57 / +55 |
| Castle 4 | 9 -> 10 | +1 / +1 | 0 -> 0 | 0 / 0 |
| Castle 5 | 22 -> 21 | -1 / -1 | 2105 -> 2089 | -16 / -17 |

Raw reports are retained under `target/alt-snow/snow-gate-before/` and `snow-gate-after/`, with external `gate-deltas.json` preserving each geometry and colour comparison.
The release workspace suite passes 819 tests with 25 existing ignores, including the installed-data gate, native height/wet/basis GPU samples and full layered shader composition validation.
Source changes are confined to map_material.wgsl, material/layer handling in map_material.rs and map_bindings.wgsl, formats/map_layers.rs, extractor material_audit.rs and layer metadata in pieces.rs, the existing map_shader.rs/map_layers.rs test modules, this M1 section and STATUS.md.
Workspace all-target Clippy with warnings denied, cargo fmt --all --check and git diff --check pass under CARGO_TARGET_DIR=target/alt-snow.
The format check initially found two concurrently created temporary formats examples; rustfmt was applied to zz_pick_tmp.rs and zz_sg_tmp.rs, without semantic changes or staging them.

### E1 Fire, lantern, ember and idol effects

Kind: effects and local light.
Track: how it looks.
Seen at 32 accepted spots.

What is wrong:

- Every brazier, torch and lantern is an unlit model: no flame, no sparks, no warm spill on the surrounding ground and walls (spot-013, spot-021, spot-025, spot-043, spot-053, spot-056, spot-058, spot-066, spot-078, spot-079).
- Campfires, burning buildings, smoke columns and drifting embers are absent on the battlefield before the Castle Gate and around the burned hall (spot-019, spot-022, spot-045, spot-047, spot-048, spot-049, spot-052, spot-061, spot-088, spot-090).
- Sculptor's Idols have no blue flame, ground glow or mist (spot-002, spot-006, spot-071, spot-123).
- Lit windows and interior lamps are dark (spot-003 shrine, spot-016 hut, spot-057 and spot-066 gatehouses, spot-044 hall).
- Emissive materials (`Emissive_wood1`, `EmissiveCloth_Simple`, `StoneLight_multi`, `Emissive_Simple1`) exist in the audit but do not glow in the captures.

Likely cause: the map capture path instantiates no map SFX (FXR) emitters and no point lights for them, and fixture emissive/local-light response needs verification; the idol at spot-002 (`o115040_0000`) is loaded, resident and visible in its `.visibility.json`, so this is effect integration, not missing fixture geometry.

Provenance: spot-053 has visible `o116000_6108` and spot-056 has `o116000_1001`/`o116000_1006`, all using `M[m11_00]_Emissive_Simple1`; these are loaded fixtures whose flame presentation is missing.
A material name proves an asset family, not that every emissive term is ignored.

Examples: `pairs/spot-045-shot-160.png`, `pairs/spot-022-shot-074.png`, `pairs/spot-056-shot-194.png`, `pairs/spot-002-shot-005.png`.

### E2 Falling snow

Kind: weather effects.
Track: how it looks.
Seen at 47 accepted spots.

What is wrong: the reference has snow falling in every outdoor frame and none of ours does; it is small per pixel but it is everywhere, and it carries part of the soft, hazy feel.

Likely cause: the weather particle effect is a map or gameplay SFX that the capture path does not start.

Examples: `pairs/spot-002-shot-004.png`, `pairs/spot-018-shot-059.png`, `pairs/spot-049-shot-169.png`.

### G1 Grass, weeds and ground clutter

Kind: missing foliage geometry.
Track: what is there.
Seen at 23 accepted spots.

What is wrong:

- Dense clumps of dry grass, weeds and low ferns beside paths, walls, steps and idols are missing (spot-002, spot-003, spot-013, spot-014, spot-019, spot-021, spot-022, spot-025, spot-043, spot-045, spot-046, spot-049, spot-053, spot-056, spot-058, spot-060, spot-066, spot-067, spot-078, spot-079, spot-092, spot-123, spot-145).
- Some foliage does render: the rock-ledge tufts at spot-008 and spot-050, the ferns at spot-016 and the bamboo at spot-002 are placed models with Grass materials (`M[Grass_o007660]_tr` and friends), so cutout rendering itself works.
- Their repeated absence beside otherwise aligned ground suggests a shared foliage-generation or representation problem.

Likely cause: investigate GrassConfig-driven instancing alongside placed foliage cutouts, export and LOD; the missing clumps cannot all be assigned to one source from screenshots alone.
The canonical layouts contain 57 Outskirts and 30 Castle parts with positive grass type IDs; most other MapPiece records use ID zero, so the inherited claim of thousands of active grass emitters was unsupported.
Concrete candidates are `m020000_0000`/`m020010_0000` (type 127), `m021000_0000`/`m030000_0000` (type 123), and Castle `m100000_1000`/`m101000_1000` (type 127).
The type IDs resolve through `cache/params/GrassTypeParam.json`; a runtime source search found no GrassConfig consumer.
That makes instancing a leading hypothesis, while the surviving Grass-material tufts prove that some placed vegetation already renders.

Examples: `pairs/spot-013-shot-044.png`, `pairs/spot-003-shot-012.png`, `pairs/spot-056-shot-194.png`, `pairs/spot-060-shot-213.png`.

### D1 Far mountain range absent or unreadable

Kind: distant scenery.
Track: what is there.
Seen at 5 accepted spots.

What is wrong: the snow-covered mountain range that fills the horizon behind the village and behind the castle keep is not in our frame; the sky meets the nearest cliffs directly (spot-009, spot-011, spot-022, spot-045, spot-091).
At spot-089 a dark mountain silhouette is present but unlit, so some far geometry is drawn.

Likely cause: check far clipping, sky depth/order, distant lighting and neighbouring draw-mask union.
Spot-011's `.visibility.json` marks 20 of Outskirts' 22 `m6*` parts visible, yet the retail mountain range is not recognizable in the render.
At spot-091 all 22 Outskirts `m6*` parts are resident but hidden, while 12 of Castle's 20 are visible.
These are per-camera observations, not a universal draw count for either area.

Provenance: the layouts retain Outskirts `m600000_6000`, `m600090_6000`, `m600091_4000`, `m601000_6000` and Castle `m600000_6000`, `m601000_6000`, `m601100_6000`, `m601101_6000`.
Visibility means submitted scene roots, not proof of in-frustum or unobscured mountain pixels; geometry absence, sky occlusion and dark shading are still competing explanations.
The what-is-there owner should establish submission/coverage before handing any residual brightness mismatch to how-it-looks.

Examples: `pairs/spot-011-shot-037.png`, `pairs/spot-009-shot-034.png`, `pairs/spot-091-shot-290.png`, `pairs/spot-022-shot-074.png`.

#### D1 data diagnosis - 2026-10-15

The subsequent [painted-backdrop follow-up](BACKDROP.md) identifies a separate missing m89 FarMountain companion, including night `m900130`, which carries the layered panorama.
The terrain restored below is therefore only one component of D1.
That follow-up also traces the blue arc to `m925009_9000` CloudFog and corrects its use of diffuse light in place of the independently authored specular light.

The `m6*` prefix is not a reliable identification of the snowy horizon.
The inspected `m600000` and `m601000` exports contain local stone, buildings and scenery stand-ins, while Outskirts `m925000_9000` supplies the broad horizon through the native `M[LandScape]_Mountain` family.
Its authored MSB transform and kilometre-scale FLVER bounds are retained.
The supplied spot-011 visibility dump already marks that mountain part ready, resident and visible.

The base Ashina GPARAM files omit `EnableAnalyticalFog`.
Executable constructor `0x1402a0740` initializes the blended fog record's byte `+0x20` to zero, and registration `0x14029c180` associates that field with `EnableAnalyticalFog`.
The forward packet producer `0x14039f000` uses a large continuation threshold when that byte is false.
Our former missing-field fallback enabled continuation and integrated kilometres of extra fixed-density fog over the horizon.
The correction preserves the authored finite volume and any explicit continuation switch, including explicit true values in other GPARAM families.

The Mountain Gbuf and Fwd shaders also use five texture slots, two albedos, three mask samples, normal-map blue for snow coverage and native `COLOR1.x` for baked occlusion.
The former generic export retained the rock albedo and base normal but discarded the snow albedo, detail normal, mask and baked occlusion.
The exporter and renderer now retain those inputs, with no replacement texture or scene-exposure change.
Matched before/after controls expose snowy horizon terrain behind the village and keep; the reviewed gate measurements and remaining sky/lighting limitations are recorded in `MAP-COMPLETENESS-SWEEP.md`.

### M2 Foliage and tree snow

Kind: materials and textures.
Track: how it looks.
Seen at 5 accepted spots.

What is wrong:

- Tree branches and trunks are caked in bright white snow (spot-060, spot-123); retail branches are dark with light dusting.
- Leaves are glossy saturated green with strong highlights; retail foliage is darker, softer and partly translucent (spot-060, spot-123, spot-002 bamboo).
- Red maple and bare winter trees are present and placed correctly (spot-013, spot-016, spot-019, spot-047), so this is shading, not placement.

Likely cause: `TreeBranch*_tr`, `Translucency*_tr` and `Translucency_Pivot*` materials need their native translucency and highlight response verified, and snow coverage, normals and direct-light response are candidates for the excessive highlights.

Provenance: spot-002 includes visible `o112420_8013` with Translucency_Pivot_Parent/Parent_Child materials and `m001310_8003` with Tree Branch1_tr.
Spot-070 is rejected for camera uncertainty and no longer contributes evidence.

Examples: `pairs/spot-060-shot-213.png`, `pairs/spot-123-shot-362.png`, `pairs/spot-002-shot-004.png`.

### O1 Object cloth and prop pose

Kind: geometry and Object pose.
Track: what is there.
Seen at 3 accepted spots.

What is wrong:

- spot-014: the camp tent on the right is a bare pole frame with a strip of canvas; retail shows a full canvas roof and walls over the same frame (the audit also lists `m11_camp_curtain_00_n` among the three missing Outskirts textures).
- spot-067: the tall banner pole beside the tree carries no cloth; retail has a long hanging banner.
- spot-049: the white hanging banner on the left post is present but hangs as a dark narrow strip.

Likely cause: investigate cloth/reference-pose export, animation, two-sided/cutout materials and story variants before changing MSB placement.
Visible candidates include spot-014 `o113200_1803`/`o113210_1801` (Flag1_cloth), `m006600_1801`/`m006610_1809` (Flag1_tr), spot-049 `m006000_6003`/`o113211_6005`, and spot-067 `o113210_1001` (Flag1_cloth).
These bounds and material families do not identify the tent canvas conclusively; the missing `m11_camp_curtain_00_n` texture is a normal-map clue, not proof that the canvas mesh was dropped.
Spot-058's cart/barrel difference is left as a question because perspective, breakage and state were not isolated.

Examples: `pairs/spot-014-shot-048.png`, `pairs/spot-067-shot-230.png`, `pairs/spot-049-shot-169.png`.

### W1 Water surfaces

Kind: water material.
Track: how it looks.
Seen at 2 accepted spots.

What is wrong: the moat is a flat pale blue-white plane; at spot-073 the stone walls and sluice continue visibly below it, and at spot-094 it reads as a snow field with leaf debris on top.
Retail water is dark green, reflective, with lily pads and leaf litter floating on it.

Likely cause: native water transmission, reflection, roughness and depth handling are missing or misrouted for `M[m11_01]_Tessellation_Water`; the pairs alone do not prove which shader term fails; the Outskirts `BackGround_Ice*` materials are a related audit candidate, not counted water evidence here.

Provenance: spot-073 has visible Castle `m210000_2000` (`m210000`) and `m801000_8000` (`m801000`) with Tessellation_Water materials.
The sluice frame is recognizable, so this is present surface geometry with the wrong appearance.
Spot-072 is rejected because its plain wall does not securely identify the same corner.

Examples: `pairs/spot-094-shot-301.png`, `pairs/spot-073-shot-251.png`.

### S1 Reference story and interaction state

Kind: visibility and story.
Track: what is there.
Seen at 1 accepted spot.

What is wrong:

- spot-053: the Castle Gate idol `o005000_0005` (entity 1101955) is absent in ours; its `.visibility.json` shows it resident but `event_enabled: false`, because the first-visit fixture has not reached that idol, while the reference save has it lit.
- spot-053: the gate leaves behind the idol are closed in ours and open in the reference.

Likely cause: the renderer runs the documented first-visit fixture and the reference was taken later (its travel menu lists eleven unlocked idols including Castle Tower Lookout); these are not missing models and should be ignored for parity unless the fixture is changed to match the reference save.

Provenance: qualify the idol by map, since `o005000_0005` also exists in Castle and other neighbours.
The Outskirts instance (entity 1101955) is ready/resident/group-visible but event-disabled; Castle's same-name instance has entity 1111951.
Gate candidates include visible `o111330_6000` (Outskirts, entity 1101610) and `o111300_1000` (Castle); the live animation/controller remains unresolved.
Spot-056 is closed in both references and is not counted as a story mismatch.

Example: `pairs/spot-053-shot-186.png` (the only accepted view showing this state mismatch; a second example would duplicate it).

### O2 Sky gap between gate leaves

Kind: Object geometry or pose.
Track: what is there.
Seen at 1 accepted spot.

What is wrong: a bright vertical sliver shows between the two gate leaves at the Castle Gate; retail has a dark seam.

Likely cause: the two leaf Objects are not at their closed reference pose, or the gate's backing piece is hidden by the first-visit state; check with the Object pose audit from `docs/MAP-COMPLETENESS.md`.

Provenance: the gate candidates are the same `o111330_6000`/`o111300_1000` pair as S1; both roots are visible at spot-056.
Their visibility does not prove the leaf/backing submesh pose is correct, and the exact leaf mesh still needs isolation.

Example: `pairs/spot-056-shot-194.png` (sole accepted seam view; no independent second example).

## Accepted pairs

### m11_00_00_00

- spot-002: `pairs/spot-002-shot-004.png`, `pairs/spot-002-shot-005.png`.
- spot-003: `pairs/spot-003-shot-012.png`.
- spot-006: `pairs/spot-006-shot-026.png`.
- spot-008: `pairs/spot-008-shot-033.png`.
- spot-009: `pairs/spot-009-shot-034.png`.
- spot-011: `pairs/spot-011-shot-037.png`.
- spot-013: `pairs/spot-013-shot-044.png`.
- spot-014: `pairs/spot-014-shot-048.png`.
- spot-016: `pairs/spot-016-shot-053.png`.
- spot-018: `pairs/spot-018-shot-059.png`.
- spot-019: `pairs/spot-019-shot-064.png`.
- spot-020: `pairs/spot-020-shot-068.png`.
- spot-021: `pairs/spot-021-shot-071.png`.
- spot-022: `pairs/spot-022-shot-074.png`.
- spot-023: `pairs/spot-023-shot-077.png`.
- spot-025: `pairs/spot-025-shot-085.png`.
- spot-030: `pairs/spot-030-shot-104.png`.
- spot-043: `pairs/spot-043-shot-153.png`.
- spot-044: `pairs/spot-044-shot-157.png`.
- spot-045: `pairs/spot-045-shot-160.png`.
- spot-046: `pairs/spot-046-shot-164.png`.
- spot-047: `pairs/spot-047-shot-166.png`.
- spot-048: `pairs/spot-048-shot-168.png`.
- spot-049: `pairs/spot-049-shot-169.png`.
- spot-050: `pairs/spot-050-shot-174.png`.
- spot-052: `pairs/spot-052-shot-180.png`.
- spot-053: `pairs/spot-053-shot-186.png`.
- spot-061: `pairs/spot-061-shot-218.png`.

### m11_01_00_00

- spot-056: `pairs/spot-056-shot-194.png`.
- spot-057: `pairs/spot-057-shot-198.png`.
- spot-058: `pairs/spot-058-shot-205.png`.
- spot-060: `pairs/spot-060-shot-213.png`.
- spot-066: `pairs/spot-066-shot-229.png`.
- spot-067: `pairs/spot-067-shot-230.png`.
- spot-071: `pairs/spot-071-shot-243.png`, `pairs/spot-071-shot-244.png`.
- spot-073: `pairs/spot-073-shot-251.png`.
- spot-075: `pairs/spot-075-shot-254.png`.
- spot-078: `pairs/spot-078-shot-259.png`.
- spot-079: `pairs/spot-079-shot-264.png`.
- spot-088: `pairs/spot-088-shot-277.png`.
- spot-089: `pairs/spot-089-shot-282.png`.
- spot-090: `pairs/spot-090-shot-288.png`.
- spot-091: `pairs/spot-091-shot-290.png`.
- spot-092: `pairs/spot-092-shot-293.png`.
- spot-093: `pairs/spot-093-shot-296.png`.
- spot-094: `pairs/spot-094-shot-301.png`.
- spot-123: `pairs/spot-123-shot-362.png`, `pairs/spot-123-shot-363.png`.
- spot-145: `pairs/spot-145-shot-428.png`.
## Reproduce and tool validation

All Cargo commands use `CARGO_TARGET_DIR=target/alt-pt` and six jobs.
See `tools/reference_index/README.md` for the headless matcher, dependencies, review tooling and camera rules.
No re-match or game window was needed for this review; the interrupted Castle search was not restarted.
Build, game all-target release Clippy with denied warnings, workspace formatting and all five Python matcher tests pass.
The offline Python run used the repository's existing `target/alt-pt/run-python.ps1` uv wrapper and cached Pillow/NumPy/SciPy wheels because the default uv cache is not writable in this sandbox.
Do not treat this environment workaround as an installed or portable dependency setup.

```powershell
$env:CARGO_TARGET_DIR = 'target/alt-pt'
cargo build --release -p sekiro-game -j 6
cargo clippy --release -p sekiro-game --all-targets -j 6 -- -D warnings
cargo fmt --all --check
uv run --no-project --with pillow --with numpy --with scipy python -m unittest tools.reference_index.test_match_poses
```

## Clearest examples

These eight accepted pairs are also preserved in the external progress root under `2026-10-15/`.

- `parity-lighting-spot-002.png` - L1, neutral retail snow versus cold, hard lighting.
- `parity-fog-spot-009.png` - F1, Castle silhouette and missing depth haze.
- `parity-snow-spot-003.png` - M1, ground detail and shrine exposure.
- `parity-fire-spot-045.png` - E1, battlefield fire and smoke absent.
- `parity-grass-spot-013.png` - G1, bare path edges beside matching gatehouse geometry.
- `parity-horizon-spot-011.png` - D1, missing or unreadable mountain horizon.
- `parity-sky-spot-091.png` - F1/L1, overcast sky and roof brightness.
- `parity-water-spot-094.png` - W1, pale moat surface.

## Rejected pairs

All 63 pairs below are excluded from every ranked count.
Their scores and visibility totals cannot substitute for shared landmark alignment.

| Pair | Reason |
|---|---|
| `pairs/spot-004-shot-016.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-005-shot-020.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-006-shot-027.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-007-shot-030.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-010-shot-035.png` | Different heading or pitch; shared landmarks do not line up. |
| `pairs/spot-011-shot-036.png` | Different heading or pitch; shared landmarks do not line up. |
| `pairs/spot-012-shot-040.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-015-shot-050.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-017-shot-058.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-024-shot-079.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-024-shot-080.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-026-shot-088.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-027-shot-092.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-028-shot-095.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-029-shot-096.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-031-shot-111.png` | Valley wash and wrong/uncertain camera; no secure common landmark (F1 diagnostic only). |
| `pairs/spot-032-shot-116.png` | Valley wash and wrong/uncertain camera; no secure common landmark (F1 diagnostic only). |
| `pairs/spot-033-shot-122.png` | Valley wash and wrong/uncertain camera; no secure common landmark (F1 diagnostic only). |
| `pairs/spot-034-shot-123.png` | Valley wash and wrong/uncertain camera; no secure common landmark (F1 diagnostic only). |
| `pairs/spot-035-shot-124.png` | Valley wash and wrong/uncertain camera; no secure common landmark (F1 diagnostic only). |
| `pairs/spot-036-shot-125.png` | Valley wash and wrong/uncertain camera; no secure common landmark (F1 diagnostic only). |
| `pairs/spot-037-shot-126.png` | Valley wash and wrong/uncertain camera; no secure common landmark (F1 diagnostic only). |
| `pairs/spot-038-shot-131.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-039-shot-136.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-040-shot-138.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-041-shot-141.png` | Zero-demand helper collision; zero visible parts (tool exception V1). |
| `pairs/spot-042-shot-147.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-048-shot-167.png` | Different heading or pitch; shared landmarks do not line up. |
| `pairs/spot-051-shot-175.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-054-shot-187.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-054-shot-188.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-055-shot-190.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-059-shot-209.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-062-shot-224.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-063-shot-225.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-064-shot-226.png` | Different heading or pitch; shared landmarks do not line up. |
| `pairs/spot-065-shot-227.png` | Different heading or pitch; shared landmarks do not line up. |
| `pairs/spot-068-shot-233.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-069-shot-234.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-070-shot-242.png` | Foliage/plain wall does not securely identify the same view; shading alone is not camera evidence. |
| `pairs/spot-072-shot-247.png` | Foliage/plain wall does not securely identify the same view; shading alone is not camera evidence. |
| `pairs/spot-074-shot-252.png` | Foliage/plain wall does not securely identify the same view; shading alone is not camera evidence. |
| `pairs/spot-076-shot-255.png` | Zero-demand helper collision; zero visible parts (tool exception V1). |
| `pairs/spot-077-shot-258.png` | Different heading or pitch; shared landmarks do not line up. |
| `pairs/spot-080-shot-268.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-081-shot-270.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-082-shot-271.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-083-shot-272.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-084-shot-273.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-085-shot-274.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-086-shot-275.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-087-shot-276.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-097-shot-311.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-099-shot-319.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-102-shot-327.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-110-shot-339.png` | Corridor recognized, but camera appears across the floor/beam structure; insufficient alignment for an interior defect claim. |
| `pairs/spot-113-shot-345.png` | Different heading or pitch; shared landmarks do not line up. |
| `pairs/spot-116-shot-352.png` | Reference is obscured by an effect while ours faces floorboards; no structural match. |
| `pairs/spot-120-shot-358.png` | Different heading or pitch; shared landmarks do not line up. |
| `pairs/spot-129-shot-372.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-133-shot-380.png` | Different heading or pitch; shared landmarks do not line up. |
| `pairs/spot-139-shot-407.png` | Near floor, wall, roof or another surface replaces the intended view. |
| `pairs/spot-149-shot-440.png` | Different heading or pitch; shared landmarks do not line up. |

### V1 Tool exceptions, not rendering defects

Kind: matcher collision/visibility selection.
Owner: matcher tool, outside the two rendering fix tracks.
Accepted spots: 0; rejected spots: 2.
Spot-041 selects `h900000` and spot-076 selects `h900500`; both dumps contain zero visible parts.
The selector receives zero demand on those helper planes, so the empty captures prove no missing scenery.
Inspect ordinary-collision support before rematching, without changing the renderer's correct zero-demand semantics.
Examples: `pairs/spot-041-shot-141.png`, `pairs/spot-076-shot-255.png`.

## Open questions

- F1: after a valid valley camera is found, which authored surface or volumetric term causes the wash at spots 031-037?
- F1/L1: how much of these frozen-build differences remains after the concurrent fog/sky/light-bank changes, at identical cameras and exposure history?
- D1: are the visible `m6*` roots clipped, occluded by sky, dark, or using an unsuitable representation, and which neighbouring block supplies the retail horizon?
- G1: which absent clumps come from positive GrassTypeParam IDs, ID-zero semantics, placed foliage, cutouts or LOD?
- O1: is the tent a cloth/animation or visibility problem, and does the missing normal texture affect its appearance rather than its geometry?
- O1: is the spot-058 cart/barrel discrepancy perspective, destruction state or an Object mismatch?
- S1/O2: which controller and leaf submeshes own the open gate state and closed seam, and which reference flags should a comparison fixture replay?
- Coverage: Upper Tower interior matches remain rejected, so interior completeness and lighting cannot be certified there; improve specific guided cameras before adding defect counts.
- Rejected spot-051 and spot-149 textures look smeared, but the cameras do not match; no texture defect or count is assigned from them.

### Brightness second pass, 2026-10-15

Direct light is not explained by AO, shadow colour or a GI scale (see [EXPOSURE.md](EXPOSURE.md#brightness-audit-second-pass-2026-10-15)).
The excess follows the authored LightSet row 100 sun and fill; the diagnostic `SEKIRO_LIGHT_SET=0` brings scene Y to 1.06-1.54 times retail at seven spots.
Retail / current HEAD (row 100) / forced row 0, scene Y % and snow Y %:

| Spot | Scene Y | Snow Y |
| --- | ---: | ---: |
| spot-002 | 7.0 / 23.9 / 10.6 | 20.2 / 78.3 / 29.8 |
| spot-003 | 7.5 / 34.4 / 11.6 | 19.2 / 82.4 / 29.3 |
| spot-009 | 10.5 / 17.7 / 11.1 | 33.3 / 85.2 / 56.2 |
| spot-025 | 10.6 / 18.6 / 11.7 | 37.4 / 88.1 / 69.2 |
| spot-091 | 19.7 / 36.6 / 26.9 | 11.7 / 44.0 / 18.6 |
| spot-092 | 4.3 / 14.3 / 4.8 | 21.1 / 57.8 / 28.3 |
| spot-008 | 7.5 / 16.5 / 8.9 | 39.0 / 88.2 / 57.2 |
| spot-013 | 12.5 / 28.6 / 13.7 | 12.8 / 46.1 / 18.3 |
| spot-089 | 22.3 / 40.7 / 29.1 | 24.9 / 84.4 / 49.7 |
| spot-090 | 12.4 / 58.3 / 29.0 | 11.0 / 82.3 / 43.6 |

Saturation and hue are in the JSON in the external collection (`measurements.json`).

### LightSet row selection audit, 2026-10-15

The engine key is `(block bank, collision LightSet id)` with a `(1, 0)` fallback, plus per-collision overlay layers from `SceneGparam` bytes.
No rule was found that dims row 100; the brightness excess (L1 item 1) stays open and nothing was changed in rendering.
Details and open candidates are in [EXPOSURE.md](EXPOSURE.md#lightset-row-100-selection-audit-2026-10-15).


### LightSet layer stack, 2026-10-15

The engine layer stack is recovered and implemented in the loader: `default`, `mapbase`, the block file and the four collision overlays, with record defaults for fields a row does not author (details in [LIGHTING.md](LIGHTING.md#layer-stack-and-record-defaults-2026-10-15)).
It does not dim the row-100 sun: the block's own noon row 100 (0.615, 0.779, 1.0 times W 1) outranks `mapbase`, and it stays 4.1 to 4.4 times row 0, the same ratio as retail's measured 0.1 to 0.25.
The default render changes only through data (`mapbase` post fields, tone mode 8, row-100 hemisphere, fill direction and sharpness, overlay post fields); scene luminance stays 2 to 5 times retail, so item 1 remains open.
The ten-spot comparison and the ruled-out fields (`LightMask Influence`, `SsaoWeight`, `Sharpness`) are in [EXPOSURE.md](EXPOSURE.md#layer-stack-result-2026-10-15).

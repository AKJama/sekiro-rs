# RustSekiro visual work and reuse audit

Research date: 2026-10-13.
RustSekiro snapshot: [265e142bf3b36a5190d8ae4885acad7a4890cd16](https://github.com/FFwLo/RustSekiro/commit/265e142bf3b36a5190d8ae4885acad7a4890cd16), authored 2026-10-12 and the latest main commit returned by GitHub during this audit.
Local comparison: the working files in sekiro-rs, including pre-existing uncommitted changes, rather than just published main.
Local line references describe the files observed during this session and may move during concurrent work.
This is a source and documentation audit, not a new screenshot comparison or performance measurement.
Only this report was created; no source, STATUS.md, shared-board entry, asset, save or research dump was changed or created.
No game was run, no repository was cloned, and the prohibited tool named in the request was neither opened nor used.

## What to borrow first

1. **Borrow the October 12 map-material findings first.**
   FwLo's `M[Multiple]` and `M[MultipleGround]` implementation uses shader variants, the actual mask-channel selectors, separate masks, UV routing, tint, detail normals, wetness and material-specific snow direction.
   Our `map_material.wgsl:242-299` still uses approximate smoothstep coverage, one mask's red channel, generic up-facing snow and a layer-C interpretation.
   This is the strongest concrete lead for pale Hirata and Sunken Valley surfaces, although his two decoded families do not establish a fix for our separate `Asset_SnowGround_Multi` defect.
   Difficulty: medium to large, because the exporter must preserve additional vertex streams, constants and textures while our bindless material and GI paths must survive.
   Sources: [map shader findings][map-notes], [shader implementation][map-shader], [exporter][export-map], [October 12 commit][C12].
2. **Borrow temporal probe interpolation, while retaining our spatial GI.**
   His `src/map.rs:1013-1103` blends two RGBA16F diffuse/specular cubemaps around the hour, including midnight wraparound.
   Ours has per-receiver spatial volumes and probe blending but selects an authored environment variant discretely; `docs/LIGHTING.md:81-83` explicitly records the stale map-load GI atlas on hour changes.
   Extend the idea to both our cubemap arrays and GIIV coefficients, with cached resources and history resets, rather than replacing them with his one-probe arena lighting.
   Difficulty: medium for cubemaps, large for synchronized GIIV and story transitions.
   This helps transitions and inconsistent ambient colour; it is not proof that default-hour dark interiors or casts will disappear.
   Sources: [probe implementation][map-source], [initial probe export][C10], [current implementation][C12].
3. **Borrow his missing FX appearance handlers and character fabric shaders next.**
   We already adapted his FXR tree, sprite/tracer batching, soft particles, multi-texture effects, blood, decals and cloth.
   The outstanding FX additions are model particles 605, distortion 607, radial blur 608 and effect lights 609, plus his line appearance 601 where needed.
   His October 12 character DetailBlend and coat Fresnel work is also a real addition beyond our basic character gloss/metal path.
   Difficulty: medium for models and lights, large for distortion integration and fabric export/shading.
   Sources: [FX runtime][fxr], [distortion shader][distort], [character shader][character-shader], [visual findings][visual-notes], [FX introduction][C10], [model-particle update][C11], [fabric update][C12].
4. **Borrow performance experiments and actual LOD export, not blanket arena limits.**
   We already use his two-cascade, 60 m shadow budget and 45 m caster concept.
   His additional useful ideas are correctly selected FLVER LOD face sets, material batching, and excluding pieces far below an arena from its shadow batch.
   Difficulty: medium to large for LOD/batching, small to medium for a carefully bounded arena-only shadow policy.
   Sources: [performance and LOD notes][map-notes], [LOD/caster export][export-map], [October 10 batching][C10b], [October 11 fixes][C11], [October 12 shadow work][C12].

**Do not replace our tone, exposure, fog, sky or GI pipeline wholesale.**
Much of his latest work in those areas is explicitly adapted from ours, and his notes still report brightness, saturation and colour mismatches against the original game.
His source is useful as a second implementation and diagnostic reference, but it is not an independently validated solution to every map's current appearance.

## Evidence and provenance

The public README was read, but it mixes newer arena/auto-exposure claims with older statements that the native lighting model is not reproduced.
The dated knowledge base and executable source are more precise than that summary.
The remote source and commit metadata were read through the GitHub connector's read-only GitHub/raw/API URL fetch after ordinary web and shell network requests failed.
No remote source was saved locally.

The [shared FEATURES board][features] still labels RustSekiro's native shader rendering, GI and sky as partial, its BTL point lights as absent, and its map coverage as gate/boss arenas.
The [shared WORK board][work] records October 12 fog density, metering/adaptation, height fog, sky domes, bloom and fog walls as done **after sekiro-rs, credited**.
It separately claims tone curve/auto exposure/night snow/probe-hour blending and layered-material refinement for FwLo, so a claim must not be read as completed visual equivalence.
The board's sekiro-rs green rendering cells are also stronger than our current local capture evidence justifies.
Our `docs/LIGHTING.md:99-128` records pale terrain, flat Castle fog and Fountainhead colour problems even after event selection was corrected.

FwLo's [THIRD_PARTY.md][third-party] explicitly credits our light colour packing, fog colour packing, soft adaptation limits, common scene scale, metering/adaptation facts, bottom-centred region boxes, story time flags, fog density, fog walls, height-fog implementation, sky implementation and bloom weights.
His newer map-material shader recovery, character fabric work, FX shaders and hour interpolation are not listed there as sekiro-rs ports.
That establishes documented provenance, not proof that every uncredited line was independently invented.
His original map reader/material chooser also credits sekiro-rs, so the material history is mixed: an initial shared/ported foundation followed by his more detailed shader work.
Conversely, our `CREDITS.md:26-42`, `fxr.rs:1-8` and `decals.rs:1-16` already acknowledge substantial FwLo work.

### Relevant commit history from October 8 onward

| Date | Commit | Rendering relevance |
| --- | --- | --- |
| October 9 | [555c83f][C9a] | Initial public source snapshot containing character models and cloth; empty commit message, so file additions establish availability rather than precise invention date |
| October 9 | [61d2c7c][C9b] | Cloth/model updates and first THIRD_PARTY file; empty commit message |
| October 10 | [60c08dd][C10g] | Gore, DecalParam stains and clash bloom |
| October 10 | [ec9e85d][C10] | Map runtime/exporter, cubemap exporter, LUT pass, FXR player, distortion and multi-texture shaders added |
| October 10 | [a100ad8][C10notes] | Documentation of grading and auto exposure |
| October 10 | [b9a25ac][C10b] | Layered map shader, soft-particle shader, map performance work and parallel cloth; release message reports 22 to 48 fps with 20 enemies |
| October 11 | [42e2bbb][C11a] | Map/model/exporter updates; not the final shader-based material model |
| October 11 | [9df085f][C11] | Cloud-sea stand-in, hit stains, model FX additions, cloth fixes and map LOD/winding/missing-texture work |
| October 12 | [265e142][C12] | Height fog, sky and fog-wall files added; grading, probe blend, layered shader, character detail/fabric and shadow work included; message explicitly names sekiro-rs fog/height fog/sky/fog walls/bloom |

The API history returned no October 8 commit, although the knowledge base contains October 8 development entries.
Large commits bundle work by several sessions; use their changed files and dated notes rather than attributing every bundled change to the commit subject alone.

## Per-area comparison

"Closer" below means better supported rendering structure or narrower known gaps, unless an actual original-game comparison is cited.
It does not mean this audit visually verified a winner.
Difficulty describes a future borrow, not work performed here.

| Area | FwLo's implemented work and commit evidence | Our current implementation, local line ranges | Closer to the original and provenance | Reuse and difficulty |
| --- | --- | --- | --- | --- |
| Tone mapping / tone curve / LUT | `src/grading.rs:1-138`, `grading_lut.wgsl:1-58`: mode-7 Sensitometric curve, output power, display-domain LUT, Bevy tone map disabled when the custom path is active; curve hardcodes default contrast through map setup; [C10][C10], [C12][C12] | `lighting.rs:108-233,240-283`, `tonemap.wgsl:20-65`: native post settings, modes 0/3/7/8, authored contrasts, exposure then tone then grading | Ours has broader native controls; his October 12 curve and colour rules explicitly follow our traced findings; neither analytically evaluated curve guarantees native table quantization | Use his display-domain/LUT diagnostics and measured comparisons; no new curve to borrow; small research reuse |
| Auto exposure | `src/map.rs:982-991,1105-1191`: soft limits sampled into Bevy compensation spline, 64x64 centre mask, full histogram range, authored speeds mapped approximately to Bevy; legacy meter/base offsets retained; [C10][C10], [C12][C12] | `lighting.rs:147-188,191-233`; custom GPU metering/adaptation via `tonemap.rs`, with freeze and authored light/dark rates | Ours is structurally closer; his centre weight, rates and soft limits credit sekiro-rs; his speed mapping is first-order in dt and compensation adds by-eye offsets | Borrow diagnostic methodology, not old +2 EV night cap or meter/base tuning; small; calibrating either renderer requires matched views |
| Light probes and probe blending | `src/map.rs:1013-1103,1275 onward`, `tools/sekiro-extract/src/cubemap.rs`: one exported arena probe, diffuse/specular maps and CPU RGBA16F interpolation between hours; [C10][C10], [C12][C12] | `lighting.rs:467-646,647-814`; all available probe arrays, per-receiver spatial blend in `map_material.wgsl:89-177`; discrete environment hour selection in `lighting_selection.rs:392-397` | Ours is closer spatially; his is more complete temporally; initial probe conversion/hour mapping is his useful work, region-box and scale corrections credit ours | Borrow hour brackets/interpolation while extending to our complete arrays and GIIV; medium to large; never substitute a whole-map arena probe |
| GI / bounce light | GILM-derived irradiance cube with baked-sun clamp in exporter; GIIV volumes documented but not used; [C10][C10], [map notes][map-notes] | `lighting.rs:512-643`, `map_material.wgsl:130-177,306-361`: native coefficient volumes, reflection arrays, directional hemisphere fallback, spatial receiver evaluation shared with actors | Ours is closer in represented data; both retain approximations; no independent native GIIV implementation to import from him | Keep ours; borrow probe-time synchronization concept only; large if applied to all coefficients |
| Sky | `src/sky.rs`, `sky.wgsl`: camera-centred m89 domes, hour mapping, Hirata/Fountainhead variants and cloud layer; [C12][C12] | `sky.rs:106-191,323-384,465-513`; selector supplies event-selected native model/group, missing variants logged; calibrated gamma-2 colour packing and approximate cloud shading | Same shading ancestry; ours has newer story/event selection and horizon fog handling; his sky is explicitly adapted from our sky/export code | No new dome renderer to borrow; compare cloud/fog integration diagnostically; small |
| Fog and height fog | `height_fog.rs:46-89`, `height_fog.wgsl`: native-model HDR depth pass, GameScale packing, signed height, geometric slices and analytical continuation; 128 slices assumed; [C12][C12] | `fog.rs:172-231,256-301,411-483`; authored/default slice controls, absorption, height attenuation, source gates, signed slant, analytical continuation and calibrated dome horizon blend | Ours is more complete; his implementation explicitly derives from our fog files; both lack recovered fog-light bank routing, local fog sources, shadowed fog and separate transparent depth | No missing global/height-fog feature to import; his faint rooftop haze is a diagnostic clue, not a verified Castle fix; small research reuse |
| Fog walls | `src/fog_wall.rs`, `tools/fog_walls.py`: common-event wall requests, persistent FXR until defeat, exported wall list; boundary collision not loaded according to notes; [C12][C12] | Existing boss-event fog-wall path; shared board records Castle and other arena staging before his port | Our rule implementation is the credited source; his visible FX does not establish a complete collision boundary | No new rule to borrow; missing FX appearance support may improve wall drawing indirectly; medium |
| Bloom | `src/map.rs:1194-1227`: authored enable, threshold, level count and normalized geometric level weights mapped to Bevy additive bloom; [C12][C12] | `lighting.rs:169-188,201-210`; `tonemap.wgsl:40-79`: exposure-aware bright pass, saturation control, seven-tap blur and fixed three-level composite normalized by authored level weights | Mixed; ours is closer in threshold domain/blur and his chain represents variable levels and Smooth weights more directly; his weights credit our findings and his notes admit pre-AE threshold, Gaussian and feedback gaps | Study variable-level scheduling without moving our threshold before AE; medium; neither side is a full native bloom replacement |
| Point lights | FXR 609 lights, brightest eight rebuilt each frame, untraced intensity scale; no BTL map-light system found; `src/fxr.rs:1313-1317,1603 onward`; [C10][C10], [C12][C12] | BTL load and selection in `map.rs:1229-1297`, update/cap in `1301-1363`; up to 64, focus culled, no shadows; scene-to-lumens conversion in `lighting.rs:286-289`; no 609 in FXR appearance whitelist | Ours is closer for map lighting; his is ahead for effect-emitted lights; FX work appears original to him, BTL units/bank routing remain unverified in ours | Borrow bounded effect-light handler, integrate with existing light budget and traced units when recovered; medium |
| Shadows | Two cascades / 60 m, 45 m caster distance, baked caster byte per batch, below-arena exclusion and material batching; `src/map.rs:1547-1560,1635-1650`, exporter; [C10b][C10b], [C12][C12] | `lighting.rs:292-316,925-955`, `map.rs:1365-1429`: already two / 60 / 45, authored first split, moving-focus world-bounds budget and character shadows retained | Ours is safer for walkable maps; his is optimized for fixed arenas; current common budget was already informed by his original performance findings | Borrow below-arena experiment only where sun direction and bounds prove discarded casters cannot shade visible receivers; small to medium; no whole-map static exclusion |
| Materials and shaders | `src/map_material.wgsl:205-331`, `src/map.rs` custom attributes, exporter `game_params` and variant slot tables; native Multiple/MultipleGround masks, UVs, tint, detail/wet/snow; `src/sekiro_material.wgsl` DetailBlend and coat Fresnel; [C12][C12] | `map_material.rs:114-135,414-565`, `map_material.wgsl:234-318`: bindless extension, one height mask and approximate layer coverage, generic snow orientation; basic character material gloss/metal, GI and tint proxy | His decoded layer/fabric math is closer for the named families; our spatial lighting/bindless integration is more complete; initial material chooser/readers borrowed from ours, new shader recovery not credited as ours | Highest-value borrow; medium to large exporter plus runtime change; retain bindless batching, GI, alpha coverage and prepass/motion consistency |
| Decals | `src/gore.rs`: DecalParam hit/TAE stains, projection/range/size rules and textures; flat floor quads and empirical mask-alpha lift; [C10g][C10g], [C11][C11] | `decals.rs:1-16,95-137,170-285`: already adapted from his work, capped flat floor stains, same empirical alpha approach | Broad parity for implemented floor stains; neither is recovered native deferred projection onto walls/terrain | No major fresh borrow established; inspect later fixes when demonstrated; small |
| FX | `src/fxr.rs`: sprites, lines, tracers, multi-texture, model 605, distortion 607, radial blur 608, lights 609; shader-based colour/softness and trails; [C10][C10], [C10b][C10b], [C11][C11], [C12][C12] | `fxr.rs:1-12,668-704,891-1019,1030-1118,1192 onward`; existing port accepts 600/602/603/604/606/10012 and batches/reuses meshes; missing appearances remain | His supported appearance coverage is closer; both still approximate lit-particle lighting/specular, normals and some blend/shape rules; most of our working subset already derives from him | Borrow 601/605/607/608/609 where needed; medium to large; no verified 10000 GPU-particle solution found on either side |
| Water | `src/map.rs:1976-1998`: translucent tinted fallback with ripple normal, explicitly marks real water shader as gap; [C11][C11]; early notes saying water skipped are historical | No dedicated water renderer in requested modules; `VISUALS-PLAN.md:848-854` still plans reflections, scrolling normals and shoreline; earlier captures document pale flat water | Neither implements shader-accurate water; his basic fallback gives a modest rendered improvement without proving fidelity | Optional small fallback, not a first-priority native solution; medium for a proper shared water renderer |
| Grass / foliage | Static map plants, cutouts and corrected LOD/winding/two-sided face-set export; no native instanced grass/wind renderer verified; [C11][C11] | Static exported foliage and cutout fixes; `VISUALS-PLAN.md:797-821,963-966`; GrassType/GrassLodRange-driven placement and sway remain planned | Neither has verified native grass; compare foliage correctness separately from grass instances | Borrow exporter diagnostics/LOD and material findings; no finished grass system to borrow; medium to large |
| LOD and culling | Radius/draw-group-selected arena extraction, merged material batches, actual face-set LOD with corrected flags and strip handling, optional far scenery at lowest LOD; [C10b][C10b], [C11][C11] | Focus-driven draw groups and full-map pieces, shadow bounds budget; `VISUALS-PLAN.md:757-761` says existing GLBs contain LOD0 only | His actual LOD is more complete; ours supports walkable-map region changes; arena crop/static LOD is not a substitute for runtime streaming/culling | Borrow LOD decoding/export and batching strategy; large integration for dynamic focus, GI materials and alpha modes |
| Performance / temporal rendering | Self-reported shadow, batching and cloth ablations; 20 m batch subdivision made performance worse; no working DLSS/FSR/FG found in audited code | `LIGHTING.md:101-122` VSync-capped multi-map timings; `UPSCALING.md:19-21,79-93` custom-pass temporal gaps; bindless layered path and bounded/reused FX buffers | No comparable measured winner; different scenes, hardware/load and coverage; original experiments useful, cloth optimizations already ported per CREDITS | Borrow LOD, ablation method and arena caster idea; do not import per-frame rebuilding or assume FPS gains transfer; medium to large |

## What could fix the current map problems

### Dark Outskirts, Reservoir, Abandoned Dungeon and Depths

There is no checked-in, demonstrated RustSekiro fix for these four full-map views.
His latest exposure path still includes renderer-specific meter/base offsets, and his notes call for refitting them after changing tone curves.
Do not adopt the obsolete +2 EV night cap recorded in the October 10 notes as if it were a recovered game rule.
Our event-selected defaults, independent collision IDs, fixed-memory/palace environments and explicit missing-bank behavior are newer and should be preserved (`LIGHTING.md:8-84`).
His named hour/probe/LUT/AE debug controls and luminance/range/warmth/saturation comparisons are useful diagnostics.
Compare the same camera, story state and authored hour with probe, GIIV, AE, LUT and fog individually disabled before concluding that exposure is the cause.
Ours already has native exposure and BTL lights, so check their selected inputs and coverage rather than adding another global brightness multiplier.

### Pale or grey Hirata and Sunken Valley

His material-family recovery is the first concrete missing implementation to try.
In particular, mask 6 is not interchangeable with mask 9, tint data is not simply a third albedo weight, and snow direction comes from the material instead of a universal normal-Y threshold.
His detail normal, gloss and wetness evaluation also recovers texture response that a brightness adjustment cannot recreate.
Validate the affected material's actual shader family first.
Our local notes isolate Valley's remaining triangular patch to `Asset_SnowGround_Multi`, including m000064 (`VISUALS-PLAN.md:808-812`), which is not proven to share his Multiple/MultipleGround math.
His missing-texture fallback and repeat-sampler diagnosis are useful checks, but our docs already record corresponding extraction/alpha fixes; they are not automatically new fixes for this case.

### Orange Senpou and blue/green Fountainhead

FwLo's source does not demonstrate a validated solution for these full-map casts.
Our selector now chooses the authored fixed Fountainhead environment and Hirata memory, so retain those event rules instead of borrowing his earlier per-fight/hour guesses.
His comparisons show his own Owl frame becoming blue and Genichiro remaining desaturated after the native tone curve port.
The published notes give Owl warmth +26 for the original versus -11 for his frame, and saturation 100 versus 44, so his pipeline is not a reliable colour reference without matched capture conditions ([map notes][map-notes], lines 471-508).
Temporal probe blending helps intermediate hours but cannot explain a bad exact-key default on its own.
Separate sky colour, GI/probe response, LUT input domain and material albedo before changing their scales together.

### Flat distant Castle fog

His height fog is our model ported into an arena coordinate system, not a newly recovered solution.
His notes describe faint 18 h rooftop haze, with the height layer roughly 140 m below the roof, while our accepted captures still show flat pale/brown distant fog (`LIGHTING.md:121-126`).
That discrepancy motivates comparing raw world height, origin correction, selected rows, near/far continuation, sky/background treatment and radiance/exposure domains at the same camera/hour.
Our pass already handles controls that his exports omit and has a calibrated sky-horizon blend; replacing it with his pass would lose behavior without identifying the cause.
His far-scenery-at-lowest-LOD export could improve the distant silhouette beneath the fog, but does not itself correct fog extinction or colour.

### Frame time

FwLo reports the gate improving from 57 fps to 83-86 fps after shadow/caster changes and the Castle arena from 48 to 66 fps after below-floor caster exclusions ([map notes][map-notes], lines 221-236).
Those correspond approximately to 17.5 to 11.6-12.0 ms and 20.8 to 15.2 ms, but are his reported whole-frame rates, not isolated GPU measurements reproduced here.
The same notes report variation of about eight fps and a failed cell-splitting experiment that fell to 39 fps.
Our current two-cascade/60 m/45 m policy already incorporates his main finding.
The remaining transfers are actual LODs, material batching, carefully proven caster exclusion and controlled shadow/cloth/material ablations.
Our local medians of approximately 16.5-17.5 ms for most scenes and 20.5 ms for Fountainhead were short VSync-capped observations on a shared GPU, so they cannot be ranked against his uncapped arena results (`LIGHTING.md:101-122`).
Our layer timing experiments also reverse sign under concurrent compilation, explicitly preventing attribution to material cost (`VISUALS-PLAN.md:824-831`).
Do not treat either set as evidence that one renderer is globally faster.
Keep the single-sample TAA/depth/motion work identified in `UPSCALING.md` compatible with any new material vertex attributes or distortion pass.
No finished RustSekiro upscaler or frame-generation implementation was found to borrow.

## Integration shape and attribution

Both projects target Bevy 0.19.1, which makes Bevy resource/pipeline techniques transferable ([his Cargo.toml][cargo]).
FwLo's renderer is a single application around `src/map.rs`, custom SHMP binary batches and arena-space coordinates, with models in a separate custom binary format.
Our renderer loads GLB pieces and extras, keeps full map coordinates, evaluates spatial GIIV/reflection volumes, preserves StandardMaterial bindless batching and separates simulation events from rendering.
An imported SHMP/model loader would bypass much of our architecture and require duplicated extraction/cache contracts.
Prefer adapting the recovered material rules into our existing exporter and extension, with sufficient data preserved for each shader family.
For map layers, that means the static switch, all needed normalized blend streams, UV0/UV1, texture tiling, mask-channel selectors, HDR material constants and snow direction.
Do not apply his arena yaw/mirror conversion again to data already converted by our exporter.
His character detail path likewise requires new textures/constants and cannot be obtained by copying only the fragment function.
His own remaining material caveats include colour gamma flags, G-buffer gloss/occlusion interpretation, shader defaults, texture-tiling provenance and incomplete ripple/metal handling ([map notes][map-notes], [visual notes][visual-notes]).

For FX, adapt handlers to our existing `FxrLib`/tree and simulation bridge, preserving current effect/particle caps and batch resource reuse.
His point lights are despawned/recreated every frame and his distortion samples Bevy's transmission texture; neither behavior should be adopted without checking our render schedule, viewport, depth and exposure conventions.
His model, distortion and radial-blur code still documents shape, blend and lighting approximations.
Our temporal readiness audit also requires matching alpha/camera-fade coverage in colour/depth/motion passes, previous CPU cloth positions and a single-sample depth path.

RustSekiro's [MIT licence][license] names `Copyright (c) 2026 FFwLo` and requires preservation of its copyright and permission notice for copies or substantial portions of the software.
Our existing `CREDITS.md:33-48` already carries an adapted-code section and the FwLo MIT notice.
For an eventual borrow, extend that section with the source paths, immutable commit, receiving paths and what was adapted, retaining the complete existing MIT notice.
A suitable proposed attribution sentence is:

> Map material layer evaluation and shader-family metadata adapted from FFwLo/RustSekiro, src/map_material.wgsl and tools/sekiro-extract/src/map.rs, commit 265e142bf3b36a5190d8ae4885acad7a4890cd16, under the MIT licence, Copyright (c) 2026 FFwLo.

Add separate equivalent entries for hour interpolation, character fabric shaders or additional FX handlers if they are actually adapted.
For findings used to write independent code, describe them as research references rather than claiming copied code.
Preserve existing AKJama attribution if using portions of his files that themselves adapt sekiro-rs.
The repository licence covers original project code, not Sekiro assets or generated data, as his README explicitly states.
No game textures, shader binaries, extracted meshes or raw disassembly are needed in a tracked implementation or this report.

## Sources

- [RustSekiro README at audited snapshot][readme]
- [Rendering/map knowledge base][map-notes]
- [Character/FX/cloth knowledge base][visual-notes]
- [Handoff and open issues][handoff]
- [Progress log][progress]
- [Licence][license] and [attribution notes][third-party]
- [Shared FEATURES][features] and [shared WORK][work]
- Local requested files were read by numbered ranges: `lighting.rs`, `lighting_selection.rs`, `sky.rs`, `fog.rs`, `map_material.rs`, `map_material.wgsl`, `decals.rs`, `fxr.rs`, `LIGHTING.md`, `VISUALS-PLAN.md` and `UPSCALING.md`.
- Supporting local references: `map.rs:1229-1429`, `tonemap.rs:169-214`, `tonemap.wgsl:1-79` and `CREDITS.md:26-48`.

[C9a]: https://github.com/FFwLo/RustSekiro/commit/555c83fcdbe24098aaf1029e3af6f9e453a4bd4f
[C9b]: https://github.com/FFwLo/RustSekiro/commit/61d2c7c8a58e44ea904b34aa62667550d30cded3
[C10g]: https://github.com/FFwLo/RustSekiro/commit/60c08dd33ea495fcd6027ae581ff5ffaca166406
[C10]: https://github.com/FFwLo/RustSekiro/commit/ec9e85dbcca69ddd298a38f0178dcee94cde7ce2
[C10notes]: https://github.com/FFwLo/RustSekiro/commit/a100ad838977ebd0c3929b02df5ee394098edcc6
[C10b]: https://github.com/FFwLo/RustSekiro/commit/b9a25ac03102429ba7b17072972233307b03b434
[C11a]: https://github.com/FFwLo/RustSekiro/commit/42e2bbb943febfe2fa9cb82ed5d1363ad6daa14b
[C11]: https://github.com/FFwLo/RustSekiro/commit/9df085fc465f3f224b77d24dafda71c4911d1a53
[C12]: https://github.com/FFwLo/RustSekiro/commit/265e142bf3b36a5190d8ae4885acad7a4890cd16
[readme]: https://github.com/FFwLo/RustSekiro/blob/265e142bf3b36a5190d8ae4885acad7a4890cd16/README.md
[map-notes]: https://github.com/FFwLo/RustSekiro/blob/265e142bf3b36a5190d8ae4885acad7a4890cd16/docs/kb/map.md
[visual-notes]: https://github.com/FFwLo/RustSekiro/blob/265e142bf3b36a5190d8ae4885acad7a4890cd16/docs/kb/visuals.md
[handoff]: https://github.com/FFwLo/RustSekiro/blob/265e142bf3b36a5190d8ae4885acad7a4890cd16/docs/HANDOFF.md
[progress]: https://github.com/FFwLo/RustSekiro/blob/265e142bf3b36a5190d8ae4885acad7a4890cd16/docs/PROGRESS.md
[third-party]: https://github.com/FFwLo/RustSekiro/blob/265e142bf3b36a5190d8ae4885acad7a4890cd16/THIRD_PARTY.md
[license]: https://github.com/FFwLo/RustSekiro/blob/265e142bf3b36a5190d8ae4885acad7a4890cd16/LICENSE.md
[cargo]: https://github.com/FFwLo/RustSekiro/blob/265e142bf3b36a5190d8ae4885acad7a4890cd16/Cargo.toml
[features]: https://github.com/AKJama/sekiro-collab/blob/main/FEATURES.md
[work]: https://github.com/AKJama/sekiro-collab/blob/main/WORK.md
[map-source]: https://github.com/FFwLo/RustSekiro/blob/265e142bf3b36a5190d8ae4885acad7a4890cd16/src/map.rs#L1013-L1103
[map-shader]: https://github.com/FFwLo/RustSekiro/blob/265e142bf3b36a5190d8ae4885acad7a4890cd16/src/map_material.wgsl#L205-L331
[export-map]: https://github.com/FFwLo/RustSekiro/blob/265e142bf3b36a5190d8ae4885acad7a4890cd16/tools/sekiro-extract/src/map.rs
[fxr]: https://github.com/FFwLo/RustSekiro/blob/265e142bf3b36a5190d8ae4885acad7a4890cd16/src/fxr.rs
[distort]: https://github.com/FFwLo/RustSekiro/blob/265e142bf3b36a5190d8ae4885acad7a4890cd16/src/fx_distort.wgsl
[character-shader]: https://github.com/FFwLo/RustSekiro/blob/265e142bf3b36a5190d8ae4885acad7a4890cd16/src/sekiro_material.wgsl

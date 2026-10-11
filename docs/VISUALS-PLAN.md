# Visuals plan: making Sekiro look like Sekiro

Research and plan only, written 2026-10-11.
No code was changed for this document.
Claims are marked **[V]** (verified, with source) or **[I]** (inferred, needs a check).

Sources:

- Our code and docs: `crates/game/src/lighting.rs`, `map.rs`, `play.rs`, `crates/formats/src/gparam.rs`, `mtd.rs`, `crates/extract/src/maps/pieces.rs`, `docs/MAP-BASELINE.md`, `docs/FIDELITY.md`, `STATUS.md`.
- FwLo's RustSekiro (MIT, read through `gh api` only): `docs/kb/visuals.md`, `docs/kb/map.md`, `docs/HANDOFF.md` (sections 4.8 and 9), `src/fxr.rs`, `src/grading.rs`, `src/map.rs`, `tools/fxr-dump`, `tools/fxr_extract.py`.
- A read-only look at the string table of `cache/raw/param/drawparam/m11_01_0000.gparam` (UTF-16 names only) and a directory listing of `cache/raw`.
- `tools/sekiro-dearxan` was not read and must not be.

We write our own code.
Borrowed facts get a light credit in `CREDITS.md` where used (one FwLo entry already exists).

## 1. Where we are today

### 1.1 What renders now **[V]**

- Map pieces are exported as GLBs with one base albedo, one normal map and an alpha mode per material (`crates/extract/src/maps/pieces.rs`, `choose_material`).
  The base colour is the first MTD albedo slot, so moss, snow, blend masks and detail layers are dropped (`docs/FIDELITY.md`).
- Materials whose MTD is a shadow caster, dummy, invisible blocker or fog card are not exported (`docs/FORMATS.md`, Models).
- Lighting (`crates/game/src/lighting.rs`): one shadow-casting sun from `LightSet` `Angle0`/`DiffColor0`, one non-shadowing fill from `Angle1`/`DiffColor1`, flat `GlobalAmbientLight` from the average of the two `Hemi` colours, `ClearColor` and `DistanceFog` from `VolumetricFog` `SkyColor`, `FixedDensity` and `DirectColor`.
  Intensities use hand-tuned constants (`LUX_PER_UNIT` 9000, `AMBIENT_PER_UNIT` 1800).
  The light set id comes from the MSB collision part under the focus (`map.rs`), and `--hour` (default 16) picks the time of day.
- Shadows: 4 cascades, 250 m, on the sun only.
  Nothing marks far pieces as non-casters.
- Camera: plain `Camera3d` with Bevy defaults, 60 degree field of view and a 20 km far plane in the viewer (`map.rs`, `camera_environment`).
  No HDR, bloom, tonemapper choice, auto exposure, LUT, sky box or environment map light were found by grep.
- No local point lights, no particles or FXR (`bullet_fx.rs` draws simple shapes), no cloth, no water shader, no foliage wind.
- Bevy is pinned to `=0.19.1` with `default-features = false` and features `3d png dds ui default_font bevy_audio wav` (`crates/game/Cargo.toml`).
  FwLo also uses Bevy 0.19.1, so his Bevy-side techniques transfer directly **[V]** (his `Cargo.toml`).

### 1.2 Data we already hold **[V]**

`cache/raw` already contains everything the plan needs, so no new unpacking tool is required:

- `param/drawparam/*.gparam` (253 files).
- `map/<id>/<id>_NNNN.btl` point light files (m11_01 has `_0000`, `_0002`, `_0004`).
- `map/<id>/<id>_envmap_00..05.tpf` GI probes and `map/m11/m11_cgrading.tpf` colour-grading LUTs.
- `sfx/sfxbnd_commoneffects.ffxbnd` plus per-map `sfxbnd_m10/m11/m13/m15...`.
- `shader/` bundles: `grass`, `gxflvershader`, `gxffxshader`, `gxdecal`, `guishader`.
- `param/` tables `GrassTypeParam`, `GrassLodRangeParam`, `DecalParam`, `ModelSfxParam`, `HitEffectSfx*Param`, `ObjectMaterialSfxParam`, `FootSfxParam` (in `cache/params`).

### 1.3 GPARAM groups present in a real map file **[V]**

String table of `m11_01_0000.gparam`, group (editor page) names:

LightSet, Fog, SSAO, Setting Info, Tone Map [Yebis], Bloom [Yebis], LightShaft [Yebis], Anti-Alias, MotionBlur [Yebis], FeedBackBlur [Yebis], ColorGrading [Yebis], ShadowParam, WaterInteraction Simulation, AOFields, AOFields Model, SeaWave, Glow Filter, ColorParaffin Filter, Lens Simulation, Cloud Shadow, Wet, SSR, VolumetricFog.

Param names seen include:

- LightSet: `Directional Light Angle0/1`, `DiffColor0/1`, `SpecColor0/1`, `LocalLight Angle1/2`, `LocalLight DiffColor1/2`, `LocalLight SpecColor1/2`, `Enable LocalLight1/2`, `Hemi Color Up/Down`.
- Tone Map: `ToneMap-Exposure`, `Enable AutoExposure`, `AutoExposure Middle Gray`, `Adaption Max/Min`, `FreezeAdaptionFade`.
- Bloom: `GlareNumLevels`, `Glare Luminance`, `Glare Threshold`, `GlareBloom Alpha`, `GaussianRadiusScale`, `Saturate Ctrl`, `Smooth`, `FeedbackWeight`.
- LightShaft: `LightBankId`.
- ColorGrading: `Enable`, `LutSourceId`.
- VolumetricFog: `Enable`, `GameScale`, `NearDepth`, `FarDepth`, `FixedDensity`, `LocalLightScale`, `LocalLightDistScale`, `HeightFogDistribution`, `HeightFogOffset`, `HeightFogDensity Scale`, `SkyColor`, `StartDist`, `DirLight LightBankId`, `DirectColor`, `OppositeColor`.

We only read about six of these (LightSet colours and angles, VolumetricFog SkyColor/FixedDensity/DirectColor).
The first plan step is to dump every group and param for one map to a scratch file so each later step starts from real numbers (cache read only, no tracked data).

## 2. What FwLo did, in one table

All statements below are from his notes unless marked.

| Feature | His state | His own caveats |
| --- | --- | --- |
| Sun, fill, sky, fog from LightSet/VolumetricFog | Done, same formulas as ours (his `LUX_PER_UNIT` 9000 is credited to our repo) | Intensities by eye |
| GI probe cube maps (envmap tpf, BC6H) as `EnvironmentMapLight` and `Skybox` | Done for the arena, hour variants blended | Probe sun differs from gparam sun by about 30 degrees at 18 h |
| Colour-grading LUT as a post pass after tonemapping | Done | Nearest key, no blend between LUT ids over time |
| Auto exposure from Tone Map group via Bevy `AutoExposure` | Done, but with measured offsets and a +2 EV cap at night | `Adaption Max` 8 washed out night, so Yebis is probably not a plain EV clamp |
| Shadows | 2 cascades, 60 m, far batches non-casters | Cascades were the main frame cost |
| Layered map materials (overlay, snow, layer C) | Done in a custom `ExtendedMaterial`, vertex-colour blend bytes | UV set per layer is a guess, byte 1/3/5 unused, only the base mask is bound |
| FXR particles | Large player, about 85 KB `fxr.rs`, billboards, tracers, lights, distortion, soft particles | No normal maps, no real lighting of lit particles, no sounds, light intensity scale is a guess |
| Cloth | Havok cloth from `*_c.hkx`, characters only | Bend-stiffness and transition sets not solved |
| Water | Not done: water materials are skipped at export | Not covered in his notes |
| Point lights (BTL), light banks, SSAO, SSR, bloom from GPARAM, LightShaft | Not done by him from GPARAM; his `vfx.rs` bloom is a hand-made threshold 1.2 | Not covered |
| Foliage, wind, grass | Not done | WindForce in FXR is ignored, plants only |

## 3. Per feature

### 3.1 Sun, ambient, GPARAM draw params and light banks

- **Data [V]:** `LightSet` rows keyed by light set id (0 outdoors, 100/110... interiors) and time of day, selected by the MSB collision part's gparam config `[LightSetID, FogParamID, LightScatteringID, EnvMapID]` (FwLo `map.md`, MSB section).
  Our `Part.light_set` already carries the first field.
- **Not used yet [V]:** the other three gparam config ids (`FogParamID`, `LightScatteringID`, `EnvMapID`), `LocalLight` directional pairs and `Enable LocalLight1/2`, `SpecColor`, and `LightBankId` fields.
- **Light banks [I]:** `LightBankId` appears in LightShaft and VolumetricFog DirLight.
  Neither FwLo nor our code resolves it.
  Likely an index into a per-map set of bank definitions (possibly BTL contents or a param table).
  Needs a dump of the id values across all 253 gparam files and a search of `cache/params` for a matching table before any claim.
- **FwLo:** done for sun, fill, hemisphere, sky and fog, and he notes the intensity scale is by eye.
  He turned the sun by the arena yaw, which we do not need.
- **Us:** sun and fill, flat ambient, constants by eye.
- **Gap:** no sky, no probe lighting (ambient is flat so interiors and shade look dead), no local directional lights, no specular colour, no time-of-day blend between envmap variants.
- **Plan:** see 5 steps 1 and 2.

### 3.2 Environment probes and sky

- **Data [V]:** `map/<id>/<id>_envmap_00..05.tpf` hold `gilm<probe>_<variant>` 128x128 BC6H cube maps and `giiv...` BC7 3D irradiance volumes (FwLo `map.md`).
  Variants 00..05 are times of day: 0 h, 6 h, 12 h, 18 h, 22 h, 2 h, matched by sky colour and sun direction.
  The MSB regions `Env_Box<probe>` (type 17) or nearest `Env_Point<probe>` (type 2) choose the probe for a position.
  Our `crates/formats/src/msb.rs` skips regions today.
- **FwLo:** one probe for the arena, decoded with `bcdec_rs`, written as RGBA16F cube DDS with mips plus a 16x16 Lambert irradiance cube, used as `EnvironmentMapLight` and `Skybox`, ambient off.
- **Us:** none.
  The sky is `ClearColor`.
- **Gap and win:** this is the largest single look change available.
  Real sky behind the world, real shade colour, real reflections on wet stone and metal.
- **Risks:** one probe per map is wrong for a walkable map, because interiors need the interior probe.
  Switch probe by the focus' region box, blend on change, and fall back to the light set hemisphere ambient.
  m89 sky domes are separate meshes, see 3.10.

### 3.3 Shadows

- **Data [V]:** `ShadowParam` group exists in every GPARAM.
  Its fields are not yet dumped.
  Hit collision "draw groups" decide piece visibility (`map.rs`, already used).
  Shadow-only materials (`shadow caster` MTDs) are currently dropped at export, so they cannot cast.
- **FwLo [V]:** shadow cascades were the dominant cost, not triangles.
  Radius 80 m with 6.86 M triangles ran at 57 fps and without shadows at 111.
  Fix: far batches tagged `NotShadowCaster` beyond 45 m, 2 cascades over 60 m, giving 83 to 86 fps.
  Splitting into 20 m cells for culling made it worse (per-entity cost).
- **Us:** 4 cascades over 250 m on whole maps (2.3 to 4.7 M collision triangles, 4000+ pieces).
  Perf risk is high but not measured by us.
- **Gap:** no `ShadowParam` use, no per-caster rules, no contact or SSAO.
- **Plan:** step 4.

### 3.4 Tone mapping, bloom, colour grading LUT and auto exposure

- **Data [V]:**
  - `ColorGrading[Yebis]` `LutSourceId` per time of day (m11_01: 0 at 0/6/18 h, 20 at 2/22 h, 30 at 12 h).
    `map/m11/m11_cgrading.tpf` holds textures `m11_01_cgrading00<n>` as 16x256 RGBA8 strips (16 slices of 16x16, x red, row = blue*16 + green).
  - `Tone Map[Yebis]` `ToneMap-Exposure`, `Enable AutoExposure`, `Middle Gray`, `Adaption Min/Max`.
  - `Bloom[Yebis]` fields listed in 1.3.
- **FwLo [V]:** LUT as a custom post pass after tonemapping on the display colour, nearest LUT id (not blended).
  Auto exposure as Bevy `AutoExposure` with a compensation curve: target EV(L) = base + Exposure + clamp(MiddleGray + METER - L, Min, Max).
  His constants `METER_EV` 2.6 and `BASE_EV` -0.8 are specific to his lighting scale, so ours must be re-measured after our lighting changes.
  Night `Adaption Max` 8 was capped to +2 EV because it blew out.
  Bevy's compensation spline needs points snapped to 1/64 EV.
  His bloom is a hand-made threshold, not GPARAM-driven.
- **Us:** none of these.
  Default tonemapper, no HDR requirement set.
- **Gap:** flat contrast and colour.
  The LUT alone is a cheap, large grade win.
- **Plan:** steps 2 and 3.

### 3.5 Fog

- **Data [V]:** `VolumetricFog` global (FixedDensity, HeightFogDistribution, HeightFogOffset, HeightFogDensity Scale, NearDepth, FarDepth, StartDist), sky light colour, directional scatter (DirectColor, OppositeColor) and local-light scale.
  Fog card materials (`GroundFog`, `LandScape_CloudFog`) are currently dropped at export.
- **FwLo:** exponential `DistanceFog` with a directional glow, same as ours, `FixedDensity` clamped to at least 0.0003.
- **Us:** the same.
- **Gap:** no height fog, no opposite-sun colour, no `StartDist`, no fog cards, no `GameScale`.
  Hirata, Sunken Valley and Fountainhead rely on height fog and cloud cards for their mood.
- **Plan:** step 7.
  Exponential distance fog stays the base, height fog is a small custom term, fog cards render as soft additive or alpha quads.
  Check **[I]** how `GameScale` and `StartDist` scale distances before using them.

### 3.6 Local point lights (BTL)

- **Data [I]:** `m11_01_00_00_0000/0002/0004.btl` (7 to 9 KB each).
  SoulsFormatsNEXT documents a BTL reader (CREDITS already lists SoulsFormatsNEXT) with light type, position, colour, intensity, radius and shadow flags, but the exact Sekiro field layout must be checked against the library and our own bytes.
  FwLo did not read BTL files (no mention in his notes), so there is no prior art to borrow here.
- **Us:** none.
  Torches, lanterns and windows are lit only by sun and ambient.
- **Gap:** interiors (castle, dungeon, temple) have no lit pools of light.
- **Risks:** Bevy forward lighting handles tens of point lights per view with clustering, hundreds is costly.
  Cull by distance from the focus and cap shadowed lights at zero (BTL shadow flag ignored at first).
- **Plan:** step 5.

### 3.7 Materials and shaders: multi-layer, foliage, water

#### Multi-layer map materials

- **Data [V]:** MTD shaders `M[Multiple]` and `M[MultipleGround]`.
  Slots 8/3/9 are base albedo, normal, mask, 5/4/6 overlay, 7/0/17 layer C, 12/10 (or 11/7) snow.
  Vertices carry two UV sets and six UByte4 blend members: byte 0 overlay weight, byte 4 snow, byte 2 layer C, bytes 1/3/5 unused.
  The `_3m` mask textures hold height-like data in R that decide where layers win (FwLo `map.md`, Layers).
- **FwLo:** done with an `ExtendedMaterial` and `map_material.wgsl`, about 72 to 80 fps versus 86 for single layer on his gate.
  The snow also uses surface up-facing (N.y 0.2 to 0.7).
  Compiled shaders were not decompiled, so the blend function is read from data, not the real shader.
  He noted snow blows out white at night.
- **Us:** single layer GLB (`pieces.rs`), which is the root of the MAP-BASELINE ranked issue 3 (snow, moss, interiors).
- **Gap:** GLB cannot carry six blend channels and the extra textures as standard glTF material.
  Options: custom vertex attributes in the GLB plus extras and a Bevy `ExtendedMaterial` on load, or a small own binary mesh format next to the GLB.
  Decision for step 6: keep GLB for plain materials and add a second export only for layered materials, so non-layered assets keep working unchanged.

#### Foliage

- **Data [V]:** `GrassTypeParam`, `GrassLodRangeParam` and `shader/grass.shaderbnd` exist in the cache.
  Trees and bushes in MTD alpha modes (`g_AlphaRef`, `g_BlendMode`).
- **FwLo:** not done, WindForce ignored.
- **Us:** alpha mode MASK or BLEND exists in `pieces.rs`.
  Sunken Valley white foliage polygons and sparse foliage are the named baseline issues.
- **Gap:** white polygons point to a wrong texture pick, missing alpha channel or missing double-sided flag.
  That is an extraction bug to diagnose, not a shader project.
  Grass instances come from the MSB or the GrassType params **[I]**, how the placement works needs investigation.
- **Plan:** step 8 (fix) then step 12 (wind and grass).

#### Water

- **Data [I]:** `SeaWave` and `WaterInteraction Simulation` GPARAM groups, water MTD shaders (names not yet listed by us) and `SSR` group.
  Water materials are skipped at export (FwLo) and we get flat pale polygons.
- **FwLo:** skipped, nothing to borrow.
- **Us:** pale flat polygons (MAP-BASELINE ranked issue 2: Fountainhead, Mibu, m13 dungeon).
- **Plan:** step 9.
  First list the water MTDs and their textures with `sekiro-extract` (our `mtd.rs` can read them), then render a tinted, reflective, scrolling-normal surface with the probe cube as reflection.
  Do not attempt real waves or interaction ripples first.

### 3.8 Particles and effects (FXR)

- **Data [V]:** `sfx/sfxbnd_commoneffects.ffxbnd` holds 2349 `f<id>.fxr` files plus `s<id>.tpf` textures and `s<id>.flver` models.
  Per-map `sfxbnd_mXX` hold map ambient effects.
  Effects are referenced by TAE events 96/118/120 `FFXID`, `Bullet sfxId_*`, `SpEffectVfxParam`, `HitEffectSfxParam` and others (FwLo `fxr_extract.py`).
- **FwLo's dependency:** `@cccode/fxr` 32.x on npm by EvenTorset (github.com/EvenTorset/fxr), **Unlicense (public domain)** **[V]** (`gh api repos/EvenTorset/fxr`, SPDX `unlicense`).
  It is a JavaScript library that reads and writes FXR for DS3, Sekiro, Elden Ring, AC6 and Nightreign.
  His `tools/fxr-dump/dump.mjs` converts each FXR to JSON and `tools/fxr_extract.py` drives it, then the Rust player consumes the JSON.
  His `fxr.rs` header says its action semantics follow the library's action definitions (`src/actions/*.yml`).
- **Could we write the reader in Rust? [I] Yes, and we should.**
  Reasons: public domain licence removes legal risk, a Node toolchain would be a new build dependency, and the format is a table of node, action and property structures.
  Approach: read the library's FXR layout and action definitions as documentation (allowed, Unlicense), then write `crates/formats/src/fxr.rs` ourselves.
  Risk: action definitions are large.
  Mitigation: parse generically (container, node tree, action id plus raw fields, property keyframes) and decode only the action ids FwLo's list shows are needed (see below), keeping unknown actions as raw.
  Cross-check by dumping a handful of effects with Node once, outside the repo, and comparing to our reader's output (fixture-style test).
- **What FwLo built [V]:**
  - Node tree: 2000 root, 2200 basic, 2001 references, 2202 levels of detail (first child).
  - Emitters 399 once, 300 periodic, 301 by distance, shapes 400-405, spread 500-503, movement 55/60/84/105/64/65.
  - Appearances: 600 point sprite, 602 quad line, 603 billboard, 604 multi-texture billboard (nearly every tool effect), 605 model, 606 and 10012 tracers, 607 distortion, 608 radial blur, 609 point light.
  - Properties: Linear, Stepped, Hermite, Bezier keyframes with RandomDelta, RandomFraction, RandomRange modifiers, over Constant0, ParticleAge, EmissionTime and ActiveTime.
  - Blend modes: 0/4/7 add, 1 source, 2/6 normal, 3 multiply, 5 subtract.
  - Soft particles via a depth copy rather than Bevy's depth prepass, which costs about 2.4 ms on a 4 M-triangle map.
  - Lit particles are drawn at `rgbMultiplier / 8` (checked on 48 lit/unlit pairs), so lit particles were 8x too bright before.
  - Distortion and radial blur shaders ported by disassembling the game's `gxffxshader` bundle (`shader/gxffxshader.shaderbnd`, which we also have).
- **His gaps:** normal maps on sprites, real lighting, sounds, light intensity scale (`4000x` guess), tracer segment curve kind, 607 shape.
- **Us:** `bullet_fx.rs` simple shapes, STATUS lists "FFX/FXR: blood on deathblows and hits, sparks on clashes, Owl's smoke and firecrackers" as next.
- **Gap:** no particle runtime at all.
- **Perf notes:** his results are batching one dynamic mesh per (texture, blend) and `view_fade` by camera distance (without it a ground dust effect filled the screen).
  Soft particles must not use the depth prepass.
- **Plan:** step 10 (reader and a minimal player), step 11 (full feature set as needed by TAE events).

### 3.9 Cloth

- **Data [V]:** Havok cloth in `<chr>_c.hkx` next to character models (for example `c1020_c.hkx`, Wolf body `bd_m_9040`).
  Reading Havok cloth needs new `hcl` object decoding on top of our existing tagfile reader in `crates/formats/src/hkx` **[I]**.
  Our FLVER reader already keeps the cloth second vertex stream (`Vertices::extra_*`, `FORMATS.md`).
- **FwLo [V]:** Verlet in Havok world space, per-cloth gravity, damping (`pow(1 - globalDampingPerSecond, h)`), substeps and iterations from file, standard, stretch and bend links, local range, capsule collisions on bones, fixed particles lerped to reference over substeps, bend stiffness constraint from the exe.
  Display meshes are skinned on CPU in 4096-vertex chunks with a per-character parallel solve.
  Cost: 20 enemies at 22 fps (41 ms cloth) fixed to 48 fps (3.9 ms) by reading an env var once, a sane blow-up check, parallelism and a 15 m camera range cull.
  Gaps: bend-stiffness and transition sets, and a blow-up fallback to rigid after 5 restarts.
- **Us:** none, and enemy and Wolf are rigidly skinned.
- **Gap:** scarf, haori and sashes are stiff.
  This matters a lot in Wolf's silhouette but little for map atmosphere.
- **Plan:** step 13, after the map look is right.
  A good first cloth is Wolf's scarf only.

### 3.10 Sky, clouds and domes

- **Data [V]:** `m89_00_00_99` is a library of co-located sky-dome pieces (MAP-BASELINE).
  `Cloud Shadow`, `LightShaft`, `Glare` groups exist in GPARAM.
- **Us:** domes render as flat fogged hemispheres.
- **Gap:** choosing the variant and giving the dome an unlit, fog-exempt sky material.
- **Plan:** fold into step 1 (sky from the probe) and a later low-priority m89 fix.

## 4. Performance risks (all maps are heavy already)

Facts: our maps are 0.8 to 4.7 M collision triangles and 4000 to 8000 retained pieces, with peak working sets of 4 to 7 GB, and the load time is 2.7 to 5.4 s per map **[V]** (MAP-BASELINE).

| Risk | Source | Mitigation |
| --- | --- | --- |
| Shadow cascades | FwLo measured shadows halving the frame rate on 4 M triangles | 2 cascades, 60 m default, far pieces `NotShadowCaster`, no shadow on layered gallery items, configurable |
| HDR plus bloom plus auto exposure plus LUT pass | Each is a full-screen pass, auto exposure adds a histogram compute | Measure each separately with `perf_log.rs`, gate behind flags |
| Layered materials | 72 to 80 fps vs 86 on FwLo's gate | Use only on materials that need layers, bind-group texture count limits mean base mask only at first |
| Many point lights | Clustered forward cost per light | Cap, distance cull, no shadows |
| FXR particles | CPU simulation and mesh rebuild | Batch per (texture, blend), per-effect distance fade, cap live effects |
| Depth copy for soft particles | His copy had no measurable cost, Bevy's depth prepass cost 2.4 ms | Copy after opaque pass, never use the prepass |
| Cloth | 41 ms for 20 enemies before FwLo's fixes | Parallel per character, camera range cull, rigid fallback |
| Memory | Maps already use 4 to 7 GB | Probe cubes are small (128x128x6 RGBA16F with mips, well under 10 MB per variant), LUTs are 16x256 |
| Water reflections | SSR is expensive | Use the probe cube only |

Rule for every step: record the `perf_log.rs` frame time on m11_01 and m11_00 before and after, and keep a knob (flag or env var) to turn the feature off.

## 5. Ordered implementation plan

Order is by visual win per effort.
Every step has a screenshot check that uses the game's own driver: `--map <id> --play --spawn x,y,z,yaw,pitch --script FILE --shots step:path --exit`.
Starting views, in Bevy space from `layout.json` player starts **[V]** (layout lists them; `--start n` is the safer alternative and is what the baseline used):

- m11_00_00_00 (Outskirts): start 3 near the moat gate, scripted route `crates/game/scripts/m11_gate_route.txt`, and start 0 near (-125.8, -47.2, -193.1) yaw -158.
- m11_01_00_00 (Castle): start 2 near (82.9, 29.8, 254.3) yaw 88.1 (the roof route the Owl fight uses), start 1 near (202.6, -26.4, 176.7) yaw 82.8.
- Extras for specific features: m25_00_00_00 `--start 1` (lake), m17_00_00_00 (snow and foliage), m13_00_00_00 (dungeon water).

Use three fixed hours (`--hour 6`, `12`, `18`) for lighting steps, and keep the screenshots under `cache/screens/visuals/<step>/` (ignored cache, not tracked).

### Step 0. Dump GPARAM and BTL facts (no game code)

- Files: a scratch dump, possibly a small `crates/extract` subcommand or an example in `crates/formats/examples`.
  Use read-only reads of `cache/raw/param/drawparam/*.gparam`.
- Output: for m11_00, m11_01 and m25 list each group, param, id and time key, plus the distinct `LightBankId` and the gparam config ids from the MSB.
- Check: a text diff of two hours shows the expected sun, sky and LUT changes (not a screenshot).
- Why first: removes guessing about light banks, ShadowParam fields and Bloom ranges.

### Step 1. Sky and GI probe lighting (largest win)

- Files: `crates/formats/src/msb.rs` (read regions: type 17 `Env_Box`, type 2 `Env_Point`), a BC6H decode in `crates/extract` (small, own or the `bcdec_rs` crate, licence to check), `crates/extract/src/maps/` (write cube DDS and irradiance), `crates/game/src/lighting.rs` and `map.rs` (add `EnvironmentMapLight` and `Skybox` to the cameras, set ambient to zero when a probe is present).
- Behaviour: choose the probe from the focus region, choose the variant by hour (nearest at first, blended later), keep the hemisphere ambient as the fallback.
- Scale constant: tune `intensity` per FwLo's finding that the probe must sit on the same scale as `LUX_PER_UNIT` (his `ENV_PER_UNIT` = 9000/pi), then re-judge by eye.
- Check: m11_00 start 3 at hours 6, 12 and 18, and m11_01 start 2 at 12 and 18.
  Expected: a visible sky with correct colour, shaded walls tinted by sky, no flat grey.
  Also an interior spot (m11_01 start 1) to confirm the interior probe or the fallback.
- Risk: interior probes selected wrongly look worse than flat ambient.
  Mitigation: a debug flag to force fallback.

### Step 2. HDR camera, tonemapper, GPARAM bloom, colour-grading LUT

- Files: `crates/game/src/lighting.rs` (camera components), a new `crates/game/src/grading.rs` and `grading_lut.wgsl` (post pass after tonemapping, our own code using Bevy's post-process pattern), `crates/extract/src/maps/textures.rs` (export `m11_cgrading` strips as small DDS).
- Bloom: Bevy `Bloom` with intensity and threshold taken from `Bloom[Yebis]` (`Glare Threshold`, `GlareBloom Alpha`, `GaussianRadiusScale`), clamped conservatively until judged.
- LUT: nearest `LutSourceId` per hour at first, then blend two LUTs by time key (FwLo uses nearest).
- Check: m11_00 start 0 at 6, 12, 18 h with LUT on and off, and a bloom on/off pair at a torch or bright sky.
  Verify no banding by zooming on the sky gradient in a screenshot.
- Risk: the LUT must run after the tonemapper in display space, and must stay a no-op on views without the component (the HUD camera).

### Step 3. Auto exposure from the Tone Map group

- Files: `lighting.rs`, `map.rs` (a `from_gparam` helper), tests.
- Behaviour: map `Middle Gray`, `Exposure`, `Adaption Min/Max` to Bevy `AutoExposure` and a compensation curve (his formula is the method, our constants must be remeasured because our light scale and probe scale differ).
  Cap night brightening initially (his +2 EV) and revisit once the night look is compared with real footage.
- Check: m11_01 start 2 at 0 h and 12 h, and a walk from a dark interior to bright outdoors in a script with shots every 60 steps.
  Expected: brightness adapts over a second or two and neither extreme clips.
- Risk: Bevy's spline needs points snapped to 1/64 EV (his note), covered by a unit test.

### Step 4. Shadow budget and quality

- Files: `lighting.rs` (cascade config), `map.rs` (mark far batches `NotShadowCaster`), `ShadowParam` fields if useful.
- Behaviour: start from 2 cascades over 60 m and `NotShadowCaster` beyond about 45 m, then raise cautiously.
  Expose `--shadow cascades,distance` and `--no-shadow`.
- Check: m11_00 start 3 and m11_01 start 2, three shots of Wolf and the sun-lit wall at 12 h with shadow settings A/B.
  Record frame time from `perf_log.rs` for each, and accept only if the budget holds.
- Risk: shadow acne or peter-panning on thin geometry, tune bias per map.

### Step 5. Local point lights from BTL

- Files: new `crates/formats/src/btl.rs` (own reader, layout cross-checked against SoulsFormatsNEXT, with a test that decodes `m11_01_00_00_0000.btl` from the cache), `crates/extract/src/maps/mod.rs` (write lights into `layout.json` or a sidecar), `crates/game/src/map.rs` (spawn Bevy `PointLight` or `SpotLight` within a radius of the focus, despawn outside).
- Behaviour: radius cull around the focus, cap light count, no shadows.
  Colour and intensity from the BTL, with a scale constant and a `LocalLightScale` from `VolumetricFog` as a global multiplier **[I]**.
- Check: m11_01 start 1 and an interior corridor, light counts logged, shots with lights on and off.
- Risk: BTL has types (point, spot, directional) and some flags, unknown ones must be logged rather than guessed.

### Step 6. Multi-layer map materials

- Files: `crates/extract/src/maps/pieces.rs` (export layer textures and blend bytes for `M[Multiple]` and `M[MultipleGround]`), a new Bevy `ExtendedMaterial` and WGSL in `crates/game`, `map.rs` (swap material on load for layered meshes).
- Behaviour: reproduce the layer semantics from FwLo's data findings: byte 0 overlay, byte 4 snow with up-facing, byte 2 layer C, `_3m` R as height for coverage with a softness constant.
  Treat UV set per layer as a guess and keep the debug view that paints the blend weights as colour.
- Check: m17_00_00_00 (snow), m11_01 interior wall (flat interior materials), m11_00 start 3 stone walls with moss.
  Before and after pairs at the same camera.
- Risk: Bevy bind group texture limit forces masks to be skipped for layers, 7 to 10% frame cost on his gate, and the night snow overexposure he saw (check after Step 3).

### Step 7. Fog upgrade

- Files: `lighting.rs`, `map.rs`, a small WGSL addition or Bevy fog extension for height fog, exporter change to keep fog card materials instead of dropping them.
- Behaviour: `HeightFogDensity Scale`, `HeightFogOffset`, `HeightFogDistribution`, `StartDist`, `OppositeColor`.
  Start with the existing exponential model and add one term, not a volumetric solver.
- Check: m10_00_00_00 (Hirata, foggy) and m17_00_00_00 start views at 12 h.
- Risk: `GameScale` and the real distance units are unverified [I], keep a tuning knob.

### Step 8. Foliage and alpha correctness

- Files: `crates/extract/src/maps/pieces.rs`, `textures.rs`.
- Behaviour: diagnose the Sunken Valley white foliage polygons (likely a texture pick or alpha channel handling bug), make cutouts two-sided, and check that 3 missing textures (known) are the only gaps.
  Use `--hide` and `--only` material tokens from `FORMATS.md` to isolate.
- Check: m17_00_00_00 viewer 0 (white polygons), m11_00 start 0 (sparse foliage).
- Risk: alpha-to-coverage with MSAA interacts with the soft particle depth (FwLo found the same, so avoid the depth prepass).

### Step 9. Water

- Files: `crates/extract/src/maps/pieces.rs` (stop skipping water MTDs and tag the mesh), `crates/game` new water material and WGSL, `lighting.rs` for reflection from the probe.
- Behaviour: tinted, mostly opaque with fresnel reflection from the probe cube, two scrolling normal maps from the MTD, a soft shoreline via the depth copy later.
  No waves or interaction at first.
- Check: m25_00_00_00 `--start 1` (lake), m13_00_00_00 (dungeon water), m15_00_00_00.
- Risk: some "water" polygons are underground flat planes meant to be hidden by draw groups, hence do the water list first (Step 0 style dump of water MTD names).

### Step 10. FXR reader in Rust

- Files: new `crates/formats/src/fxr.rs`, `crates/extract` (an `fxr-dump` command that writes JSON or a compact binary for a chosen set of ids, from `cache/raw/sfx`), tests with fixtures generated at test time from the cache (never committed).
- Method: write the reader from the Unlicense library's definitions, validate on a few effects by one-off comparison against `@cccode/fxr` run outside the repo, and record the action ids encountered across all 2349 files (a coverage table).
- Check: a unit test that parses every FXR in `sfxbnd_commoneffects` without error and reports unknown action ids, no screenshot yet.
- Risk: effort is in the long tail of action ids, the coverage table keeps that visible.

### Step 11. FXR player, then wire to TAE events

- Files: new `crates/game/src/fxr.rs` (nodes, emitters, particles, blend modes, batched meshes), `fx_*.wgsl` as needed (multi-texture billboards, soft particles, distortion), hooks in `play.rs` and `bullet_fx.rs`, `crates/sim` TAE events 96, 118 and bullet `sfxId_*`.
- Order inside the step: billboards (603) and multi-texture (604) first because most tool effects use them, then lights (609), tracers (606), then distortion (607) and radial blur (608).
- Priority effects (from STATUS next steps): hit and clash sparks (`HitEffectSfxParam`), blood on deathblows (FFX 220505 and 220506), Owl's smoke and firecrackers.
- Check: a test hook like FwLo's "play one effect in front of the camera and shoot frames" but built on our `--script --shots` driver, plus the Owl deathblow script and the soldier deathblow script (`crates/game/scripts/soldier_deathblow.txt`).
- Risk: depth, blend order and bloom interplay, so keep a flag to disable the player and fall back to nothing.

### Step 12. Map ambient FXR, wind, grass

- Files: extraction of `sfxbnd_mXX` placements (how they are placed in MSB or event data is unverified [I]), `GrassTypeParam` and `GrassLodRangeParam` readers, a vertex-shader sway for foliage.
- Check: m11_00 and m17_00 foliage with two frames a second apart.

### Step 13. Cloth

- Files: new `crates/formats/src/hkx` cloth reader additions, new `crates/game/src/cloth.rs`, hooks in `character.rs`.
- Order: Wolf's scarf only, then robe and enemy cloth, using FwLo's solver structure as the method and Havok constraint formulas he traced from the exe.
- Check: Wolf running in `crates/game/scripts/held_move_kill.txt` or a new short run script, three shots one second apart showing the scarf trailing, plus a performance number with 20 enemies.
- Risk: the blow-up case and per-vertex work must be handled from the start (FwLo's first version cost 41 ms).

### Step 14. Sky domes (m89) and LightShaft

- Files: `map.rs` (variant choice, unlit fog-exempt dome material), a LightShaft pass if the look needs it.
- Check: m89 viewer captures listed in `MAP-BASELINE.md`.

## 6. Credits and licence notes

- FwLo's RustSekiro is MIT, so facts and approaches may be reused with credit.
  The existing `CREDITS.md` entry should get a short line when steps 1 to 3 land (GI probe variant mapping, LUT strip layout, Tone Map compensation approach), and another when layered materials and FXR land.
- `@cccode/fxr` (EvenTorset) is Unlicense: the action definitions can be read freely, and a credit is polite but not required.
  If we ever reuse its YAML definitions directly, copy them with their notice.
- Do not touch or reference `tools/sekiro-dearxan`.
- Our rule remains: no game assets or extracted data in tracked files, everything stays under ignored `cache/`.

## 7. Open questions to settle during the work

- What exactly `LightBankId` indexes **[I]**.
- The Sekiro BTL field layout (verify against bytes before trusting any library).
- How `ShadowParam` fields map to cascades and bias **[I]**.
- Whether `Adaption Max` is a plain EV clamp (FwLo suspects not **[V]** as his note, but no replacement found).
- How map ambient effects (sfxbnd_mXX) and grass instances are placed.
- Whether the shader bundles (`gxflvershader`, `grass.shaderbnd`) are worth disassembling the way FwLo did for FFX shaders (his method: D3DDisassemble on the DXBC `.ppo` and `.vpo` files).
  This is the only reliable way to settle layered-material and water formulas, and it is time boxed to one material family.

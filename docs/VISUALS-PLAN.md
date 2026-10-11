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
  Step 6 retains GLB for all materials and carries additional layered sampler metadata in material extras.

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

#### Step 0 status (2026-10-11)

Implemented `crates/formats/examples/gparam_dump.rs` as a read-only dump of every matching draw-param file for the three requested maps, including every group, param, id, time key and value, distinct `LightBankId` values and MSB config tuples.
The complete output and the 6 h versus 12 h text comparison are under `target/alt/visuals/`.
The following facts describe each map's default `_0000` GPARAM; override files are also dumped rather than assumed equivalent.

The m11 maps use a warm low morning sun, a high blue-grey noon sun, a warm low evening sun and blue moonlight at 2/22 h; the 0 h sun RGB is zero.
Sky colour changes from nearly black at 0 h through blue at 6/12 h to muted blue-grey at 18/22 h.
Both use LUT 0 at 0/6/18 h, LUT 20 at 2/22 h and LUT 30 at 12 h.
Fountainhead's default sun, sky, LUT 20 and tone settings are constant at every sampled hour.

| Hour | Outskirts exposure / middle grey / adaptation min..max | Castle exposure / middle grey / adaptation min..max | Bloom threshold / alpha, Outskirts; Castle |
| --- | --- | --- | --- |
| 0 | 0 / -3 / -0.4..8 | 0 / -3 / -0.4..8 | 0.075 / 1; 0.075 / 1 |
| 2 | 0.2 / -4.9 / -0.5..0.5 | 0.2 / -4.9 / -0.5..0.5 | 0.075 / 1; 0.075 / 1 |
| 6 | 1.4 / -6.8 / -0.5..2 | 0.5 / -5.2 / -0.7..2 | 0.015 / 0.4; 0 / 0.4 |
| 12 | 0.4 / -4.2 / -0.01..0.5 | 0.5 / -5.4 / -0.01..0.01 | 0 / 0.5; 0 / 0.5 |
| 18 | 0.2 / -4.2 / -0.5..0.5 | 0.8 / -6.6 / -0.5..1 | 0 / 0.4; 0 / 0.3 |
| 22 | -0.2 / -4.9 / -0.5..0.5 | 0.3 / -4.9 / -0.5..0.5 | 0 / 0.5; 0.05 / 0.3 |

Gaussian radius scale is 1 in these sampled default files.
Fountainhead has exposure 1.1, middle grey -4.5, adaptation -0.5..0.5 and bloom threshold/alpha 0.075/0.4 at every sampled hour.
The default light-bank values are 111000000, 111010000 and 125000000 respectively; their meaning remains unresolved.
MSB environment-map IDs in these maps are 0 or -1 and do not identify the many spatial probe regions.

### Step 1. Sky and GI probe lighting (largest win)

- Files: `crates/formats/src/msb.rs` (read regions: type 17 `Env_Box`, type 2 `Env_Point`), a BC6H decode in `crates/extract` (small, own or the `bcdec_rs` crate, licence to check), `crates/extract/src/maps/` (write cube DDS and irradiance), `crates/game/src/lighting.rs` and `map.rs` (add `EnvironmentMapLight` and `Skybox` to the cameras, set ambient to zero when a probe is present).
- Behaviour: choose the probe from the focus region, choose the variant by hour (nearest at first, blended later), keep the hemisphere ambient as the fallback.
- Scale constant: tune `intensity` per FwLo's finding that the probe must sit on the same scale as `LUX_PER_UNIT` (his `ENV_PER_UNIT` = 9000/pi), then re-judge by eye.
- Check: m11_00 start 3 at hours 6, 12 and 18, and m11_01 start 2 at 12 and 18.
  Expected: a visible sky with correct colour, shaded walls tinted by sky, no flat grey.
  Also an interior spot (m11_01 start 1) to confirm the interior probe or the fallback.
- Risk: interior probes selected wrongly look worse than flat ambient.
  Mitigation: a debug flag to force fallback.

#### Step 1 status (2026-10-11)

Implemented MSB Env_Point/Env_Box parsing, including rotated bottom-centred boxes and numeric probe IDs.
Castle cache tests confirm 107 points and 107 boxes.
The extractor writes mirrored RGBA16F 128-pixel cube DDS files with box-filtered mips, 16-pixel cosine-weighted Lambert diffuse cubes and `env.json`.
The HDR decoder adapts the locally available texture2ddecoder MIT implementation because its public BC6H API quantizes to RGBA8; the complete upstream licence is retained in the source.
Re-extraction produced 275 Outskirts, 696 Castle and 49 Fountainhead probe variants.
Outskirts has no variant 00 and Fountainhead only has variant 04, so runtime chooses the nearest actually available hour.

The camera focus selects the smallest containing box, otherwise the nearest point, with an immediate cut.
Each 3D camera uses that probe for Skybox and EnvironmentMapLight, and flat ambient becomes zero only after both images load.
`--hour` defaults to 12 and `--no-env-probe` retains the existing hemisphere fallback.
`ENV_PER_UNIT = LUX_PER_UNIT / pi = 9000 / pi = 2864.8 cd/m2`, following FwLo's scale relation without an additional eye-tuned multiplier.

All six requested views and their fallback pairs are under `target/alt/visuals/`, named `outskirts-h6`, `outskirts-h12`, `outskirts-h18`, `castle-h12`, `castle-h18` and `interior-h12`, with `-probe.png` and `-fallback.png` suffixes.
Morning/evening shade becomes warmer and noon/interior shade cooler; Castle evening is substantially darker than the flat ambient fallback.
Material layering and foliage still look approximate, as expected before steps 6 and 8.
The source GI cubes contain nearby scenery and ceilings, so using them as an infinite sky visibly duplicates architecture through Castle windows.
Sky-only colour clustering was tried and rejected because it produced artifacts; the final implementation uses the unchanged local probe radiance.
So the probes drive lighting and reflections only; the sky stays the GPARAM clear colour until the sky dome meshes (step 14), and `--probe-sky` shows the capture as a sky for debugging.
The X mirror and cube axes are unit tested, DDS loading verifies six layers, and the Outskirts morning/noon brightest direction is within 6.7/8.3 degrees of GPARAM sun direction; brightest pixels in other captures may be walls, so these are supporting evidence rather than exact sun verification.

Specular mip filtering is a box-filter approximation rather than a GGX convolution, diffuse integration uses 256 samples, and probe transitions do not blend.
The runtime frame measurements are capped at 60 Hz; `perf-summary.csv` records matched medians of 16.6-16.8 ms, with probe deltas from -0.034 to +0.076 ms, and cannot establish uncapped GPU cost.
Final formatting, workspace Clippy, release formats/extract tests (64 passed, two ignored), 14 game unit tests and the alternate release game build pass.
The permitted pre-existing map_tangents failure remains: direction agreement 0.004 and handedness agreement 0.953 across 1,288,976 vertices.
Unused `sky_*.dds` experiment outputs may remain in the ignored cache but are never loaded by the final implementation.

### Step 2. HDR camera, tonemapper, GPARAM bloom, colour-grading LUT

- Files: `crates/game/src/lighting.rs` (camera components), a new `crates/game/src/grading.rs` and `grading_lut.wgsl` (post pass after tonemapping, our own code using Bevy's post-process pattern), `crates/extract/src/maps/textures.rs` (export `m11_cgrading` strips as small DDS).
- Bloom: Bevy `Bloom` with intensity and threshold taken from `Bloom[Yebis]` (`Glare Threshold`, `GlareBloom Alpha`, `GaussianRadiusScale`), clamped conservatively until judged.
- LUT: nearest `LutSourceId` per hour at first, then blend two LUTs by time key (FwLo uses nearest).
- Check: m11_00 start 0 at 6, 12, 18 h with LUT on and off, and a bloom on/off pair at a torch or bright sky.
  Verify no banding by zooming on the sky gradient in a screenshot.
- Risk: the LUT must run after the tonemapper in display space, and must stay a no-op on views without the component (the HUD camera).

#### Step 2 historical status (2026-10-11, superseded by native post below)

Map viewer and play cameras now have `Hdr`, analytic `Tonemapping::AcesFitted` and `DebandDither::Enabled`.
ACES fitted provides a filmic shoulder and restrained highlight saturation without requiring Bevy's optional tonemapping LUT assets.
It approximates Yebis rather than reproducing its unknown curve.
The probe images remain lighting only; they are not the sky.
All effects are restricted to 3D map cameras, and the LUT render system additionally requires its own `GradingLut` component.
The pass runs in `Core3dSystems::PostProcess`, after tonemapping and before Bevy's UI pass, so HUD colours are unaffected.

Bloom uses additive composition with `intensity = clamp(GlareBloom Alpha * 0.08, 0, 0.08)`, the authored `Glare Threshold` clamped to 0..1 in the camera-exposed HDR buffer, softness 0.5 and zero low-frequency boost.
`GaussianRadiusScale` maps to isotropic `Bloom.scale`, clamped to 0.25..2.
These are conservative mappings between different bloom kernels, not a verified Yebis equivalence.
`--no-bloom` disables the effect.

The extractor exports each block's 16x256 RGBA8 strips from `<area>_cgrading.tpf` to `cache/maps/<id>/grading/lut<id>.png` without changing the display-space bytes.
Re-extraction produced four LUTs each for m11_00 and m11_01 and two for m25_00, with zero failed GLBs.
`LutSourceId` uses the nearest authored circular hour key, with light-set fallback to id zero; it is never interpolated into a nonexistent LUT index.
The embedded WGSL converts the tonemapped linear buffer to sRGB display space, samples eight strip cells for trilinear interpolation (x=red, row=blue*16+green), adds half-LSB display dither and converts back to linear for Bevy's output conversion.
The LUT is loaded as unorm data rather than an sRGB texture, avoiding a second gamma decode.
Missing or disabled LUTs leave the camera without the component and perform no pass; `--no-grading` disables grading.
FwLo's layout and compensation facts are credited in `CREDITS.md`; the pass is original code.

Every capture was inspected, including the matched normal-map before/after pair and the six grading pairs in `target/alt/visuals/v2-start{0,3}-h{6,12,18}-{on,off}.png`.
Review sheets are `v2-review-start0.png`, `v2-review-start3.png`, `v2-review-castle.png` and `v2-review-controls.png` in that directory.
Morning light is warm, noon snow and shade stay cool, and evening LUT 0 adds plausible teal/amber separation without whitening the highlights.
No fully white samples were found in the sampled world region of these views; evening temple ceilings remain dark on both sides of the grading comparison because local lights and material layers are still absent.
Castle start 2 and interior start 1 noon pairs are `v2-castle-h12-{on,off}.png` and `v2-interior-h12-{on,off}.png`.
Bloom pairs are `v2-start3-h6-on.png` / `v2-start3-h6-bloom-off.png` and the corresponding noon pair; the conservative halo is subtle and retains cliff/snow detail.
`v2-sky-zoom.png` is a sixfold nearest-neighbour crop of the Castle window sky, with fine dither and no visible bands.
The sky is still a clear colour, so this cannot establish the quality of a true sky-dome gradient before step 14.

The pre-existing tangent test was reproduced at 0.004 direction agreement and 0.953 sign agreement across 1,288,976 sampled vertices.
Sekiro's stored semantic-6 vector aligns with the UV bitangent, approximately 90 degrees from the glTF U tangent; the reader's bytes, vertex order and test space were not the problem.
The exporter now reconstructs `U = normalize(N cross mirrored_stored_vector) * mirrored_handedness`, retaining the existing mirrored sign.
The original test thresholds remain unchanged and pass after re-extraction, with direction agreement approximately 0.933 and sign agreement 0.953.
`v2-tangent-before.png` and `v2-tangent-after.png` show corrected cliff and masonry normal-map directions at the same camera before post effects were enabled.
The remaining differences from MikkTSpace are not fully traced; authored smoothing, seams and degenerate UVs are possible contributors, and all 462 sampled normal-texture references use UV0.

### Step 3. Auto exposure from the Tone Map group

- Files: `lighting.rs`, `map.rs` (a `from_gparam` helper), tests.
- Behaviour: map `Middle Gray`, `Exposure`, `Adaption Min/Max` to Bevy `AutoExposure` and a compensation curve (his formula is the method, our constants must be remeasured because our light scale and probe scale differ).
  Cap night brightening initially (his +2 EV) and revisit once the night look is compared with real footage.
- Check: m11_01 start 2 at 0 h and 12 h, and a walk from a dark interior to bright outdoors in a script with shots every 60 steps.
  Expected: brightness adapts over a second or two and neither extreme clips.
- Risk: Bevy's spline needs points snapped to 1/64 EV (his note), covered by a unit test.

#### Step 3 historical status (2026-10-11, superseded by native post below)

Bevy `AutoExposure` runs on the tested RTX 5070 Ti Vulkan device; compute support is checked before adding camera components.
An unsupported device, a disabled GPARAM switch or `--no-auto-exposure` uses fixed tonemapping compensation `BASE_EV + ToneMap-Exposure` instead.
Physical camera exposure remains Bevy's daylight default, matching the existing 9000 lux light scale.
The histogram meters the already camera-exposed HDR buffer over -12..12 log2 luminance, excluding its darkest and brightest 10 percent.
Our conservative visual calibration uses `BASE_EV = -0.5` and `METER_EV = 2.0`, rather than copying FwLo's constants.
These are provisional renderer offsets, not recovered physical Yebis units.
The target correction is `BASE_EV + Exposure + clamp(MiddleGray + METER_EV - L, AdaptionMin, AdaptionMax)`.
The compensation spline stores `L + target`, because Bevy subtracts metered L internally.
Authored min/max are clamped to -2..+2 EV, with max never below min, and every spline point is snapped to 1/64 EV on both axes.
Bright-scene adaptation runs at 3 stops/s and dark-scene adaptation at 2 stops/s, with a one-stop exponential transition.
GPARAM exposure and middle grey interpolate by hour; the night +8 maximum is deliberately capped to +2 until Yebis's interpretation is traced.

Castle start 2 at 0 h and 12 h is checked with auto and fixed exposure in `v2-castle-h{0,12}-{auto,fixed}.png`, using the noon `-on` image for the auto side.
Night auto exposure lifts readable floor and Wolf detail while retaining the bright patches; near-black samples drop from 21.129 percent fixed to 0.181 percent auto, with no sampled white clipping.
The Outskirts temple-to-courtyard walk uses `target/alt/visuals/v2-walk.txt`, with matched `v2-walk-{auto,fixed}-<step>.png` captures at 120..660 in 60-step increments and a final 720-step image without the step suffix.
The reviewed sheets `v2-review-walk-a.png`, `v2-review-walk-b.png` and `v2-review-walk-final.png` show shadow detail indoors, the doorway transition and preserved snow detail outside without an exposure flash.
The noon adaptation range is only -0.01..+0.5, so its visible change is modest compared with night.
Probe and collision light-set changes still cut immediately, as documented in step 1; exposure smooths its own metered correction.

Matched noon medians with optional effects disabled versus all effects enabled are 16.661 -> 16.675 ms for Outskirts and 16.681 -> 16.641 ms for Castle.
The historical step-1 probe medians were 16.760 and 16.601 ms respectively, but those shorter captures and parallel compilation are not an isolated HDR cost measurement.
Per-feature controls and their source CSV paths are recorded in `target/alt/visuals/v2-perf-summary.csv`.
All runs are capped at 60 Hz, so the deltas are within measurement noise and cannot establish uncapped GPU overhead.
Image sampling results are in `v2-image-metrics.csv`.

Final workspace formatting, Clippy with `-D warnings`, the requested release game/formats/extract suite (84 passed, two existing ignored tests, including map_tangents) and the release game build pass with `CARGO_TARGET_DIR=target/alt`, four jobs and offline Cargo.
Regressions cover discrete time keys, lossless LUT export, snapped/capped exposure curves and 3D-only component application with an unaffected 2D camera.
The three requested maps were re-extracted into their existing map output directories.
No commits, network requests, worktrees, protected-path writes or changes to the parallel mobility files were made.

#### Steps 2 and 3 native post status (2026-10-11)

Original GPU passes now replace Bevy ACES, Bloom and AutoExposure as a single ordered HDR chain before the HUD.
The executable tracing delivered in `SHADER-INPUTS.md` during this session supplies the post CPU transforms, replacing the initial v6 named-calibration candidates.
The camera has `Tonemapping::None`, and `--no-auto-exposure`, `--no-bloom` and `--no-grading` retain their independent controls.

Metering uses sixteen samples per reduction cell on a 64x64 grid, native biased log luminance and radial center weighting.
Adaptation uses authored MiddleGray and `1-10^(-dt*speed)`, then the native soft EV shoulders rather than hard clamps or the old +2 EV ceiling.
The CPU path's previous-frame adaptation gain is retained entirely on GPU.
Gain `2^Exposure*2^(-softlimitedEV)` applies to scene and bright extraction; shipped native pre-exposure is one.
One shared renderer-unit conversion, `BEVY_HDR_TO_NATIVE_RADIANCE=0.5`, applies to meter and scene because the existing Bevy light/daylight camera calibration does not establish native radiance units.
It leaves the recovered GPARAM transforms intact and addresses the rejected unconverted noon candidate's 1.03 percent pure-white world pixels.
Missing rates use traced LightAdapt 1 and DarkAdapt 0.5, and elapsed time is presumed seconds.

Tone transfer defaults to native mode 7, sensitometric, with traced contrast 0.5, table exponent one and ordinary output exponent 1/2.2.
Authored modes 0, 3 and 8 select the recovered filmic, Reinhard and extended-Reinhard alternatives.
`--native-tone-filmic` and `--native-tone-reinhard` expose comparison controls.
The table functions are evaluated analytically, preserving their formula while avoiding table quantization.

Bloom uses luminance-subtractive extraction, separable seven-tap Gaussian filtering and the recovered three-level 1/0.5/0.25 composite before tone transfer.
Final colour gain is `Alpha*Luminance/sum(Smooth^i)` for authored GlareNumLevels, with renderer quality scale one explicitly named as unresolved.
GaussianRadiusScale multiplies the current sigma-1.4 tap spacing, whose precise native CPU kernel remains unverified.
The full authored-level upscale schedule and motion-reprojected temporal feedback remain gaps.

The unorm LUT's red-fastest, green-row, blue-slice layout was confirmed against the recovered copy shader.
Its exact trilinear input is the ordinary output-power result; storage conversion into Bevy's linear ping-pong buffer is reversed before lookup.
Display dither runs after grading, or after tone transfer when grading is absent.
HUD colours stay outside the chain.
GIIV export, coefficient sampling, region fades and parallax correction remain pending under section 3.2/recommendation 3.
Newly resolved fog/light CPU transforms remain for their own follow-up.

The seven requested matched scene pairs are `target/alt/visuals/v6-{owl,genichiro,outskirts6,outskirts12,outskirts18,interior,hirata}-{before,final}.png`.
Every before/final view was inspected, along with noon controls for bloom, adaptation, grading, filmic/Reinhard overrides and the extra Castle midnight view.
Owl's snow rails and feather/fabric detail remain visible, morning retains warm rock against cool shade, noon shade stays blue, and evening keeps warm/cool separation with stronger midtones.
The interior doorway and Hirata floor retain readable texture rather than a bloom veil.
Bloom halos are subtle and no obvious bands appear at native capture resolution.
All seven final world crops have zero pure-white RGB pixels; noon has one near-white pixel in 1,152,000 samples, versus about 1.03 percent pure white in the rejected unconverted native candidate.
The crop excludes the top 100 and bottom 80 rows, including most HUD/debug text.
The measurements are in `v6-image-metrics.csv`, produced with a standard-library PNG reader.

| Viewer scene | Before frame snapshot, ms | Final frame snapshot, ms | Native exposure/bloom/tone GPU, ms |
|---|---:|---:|---:|
| Outskirts start 3, 6 h | 24.814 | 23.753 | 0.06 |
| Outskirts start 3, 12 h | 22.222 | 25.381 | 0.06 |
| Outskirts start 3, 18 h | 21.978 | 20.704 | 0.06 |
| Castle start 1, 12 h | 25.510 | 23.923 | 0.06 |
| Hirata start 0, 12 h | 16.639 | 16.556 | 0.08 |

Viewer snapshots are `1000/smoothedFPS`, not isolated repeatable overhead measurements, and the shared machine was also running the parallel job.
Final Owl and Genichiro diagnostics report 24.150/26.423 ms frame time and 0.062 ms native-post GPU time; the initial arena baselines did not log frame diagnostics.
The earlier Bevy morning sample reported bloom 0.07 ms and auto exposure 0.04 ms, with tone/LUT timing not included in those spans.
The new native-post span includes metering, adaptation, spatial bloom and tone transfer, with grading still separate.
These measurements support a small GPU cost, but do not establish an isolated wall-frame improvement.

Final formatting, workspace/all-target Clippy with `-D warnings`, the requested release game/formats/extract suite and the release game build pass offline in `target/alt` with six jobs.
The suite reports 104 passed and two existing ignored tests, including the new shader-composer regression covering every post shader permutation.
Validation logs are `v6-fmt-delivery.log`, `v6-clippy-delivery.log`, `v6-tests-delivery.log` and `v6-build-delivery-calibrated.log`.
The save backup, scripts, metrics and captures stay under `target/alt/visuals/`.
This session made no commits, network calls, protected-path writes or extractor outputs.

### Step 4. Shadow budget and quality

- Files: `lighting.rs` (cascade config), `map.rs` (mark far batches `NotShadowCaster`), `ShadowParam` fields if useful.
- Behaviour: start from 2 cascades over 60 m and `NotShadowCaster` beyond about 45 m, then raise cautiously.
  Expose `--shadow cascades,distance` and `--no-shadow`.
- Check: m11_00 start 3 and m11_01 start 2, three shots of Wolf and the sun-lit wall at 12 h with shadow settings A/B.
  Record frame time from `perf_log.rs` for each, and accept only if the budget holds.
- Risk: shadow acne or peter-panning on thin geometry, tune bias per map.

#### Step 4 implementation (2026-10-11)

The default sun budget is two cascades over 60 m, with map meshes outside 45 m of the focus tagged `NotShadowCaster`.
Distance is measured to the nearest point of each transformed mesh bounding box, so a large nearby wall is retained even when its instance origin is distant.
The budget refreshes twice a second and follows the focus; Wolf and enemy meshes are outside the map hierarchy and keep their shadows.
The counts include all loaded map meshes, including meshes currently hidden by draw groups, rather than promising a count of GPU draws.
No cells or additional map entities are created.

`--shadow cascades,distance`, `--shadow-casters metres`, `--shadow-bias depth,normal` and `--no-shadow` provide controls.
Normal bias is 0.6 shadow texels, reduced from Bevy's 1.8, and depth bias remains 0.02.
The first split reads GPARAM `ShadowParam` `Cascade Dist 0->1`, clamped below half the configured range, or falls back to 15 m when absent.
The dumped m25 base file has 5/25/100 m splits; the m11 base files do not supply these splits.
The two-cascade/60 m range and 45 m caster distance are an explicit renderer budget following FwLo's measured approach, not recovered Sekiro cascade settings.
GPARAM's slope-scaled depth bias of 3.0 and kernel scale of 1.5 are retained as research facts because Bevy's depth/normal biases and filtering have different units and no verified direct conversion.
`--uncapped` now also configures play-mode windows from `map.rs`; scripted play measurements remain near 60 fps, while the Castle viewer reaches 112.7 fps with the new budget.

### Step 5. Local point lights from BTL

- Files: new `crates/formats/src/btl.rs` (own reader, layout cross-checked against SoulsFormatsNEXT, with a test that decodes `m11_01_00_00_0000.btl` from the cache), `crates/extract/src/maps/mod.rs` (write lights into `layout.json` or a sidecar), `crates/game/src/map.rs` (spawn Bevy `PointLight` or `SpotLight` within a radius of the focus, despawn outside).
- Behaviour: radius cull around the focus, cap light count, no shadows.
  Colour and intensity from the BTL, with a scale constant and a `LocalLightScale` from `VolumetricFog` as a global multiplier **[I]**.
- Check: m11_01 start 1 and an interior corridor, light counts logged, shots with lights on and off.
- Risk: BTL has types (point, spot, directional) and some flags, unknown ones must be logged rather than guessed.

#### Step 5 implementation and byte census (2026-10-11)

The original bounds-checked `btl.rs` reader accepts the observed Sekiro version-2 layout and retains all 232 bytes of every record.
All 18 installed BTL files satisfy `file_size = 0x3c + string_bytes + count * 0xe8`, with no trailing bytes.
Castle `_0000` has a 1520-byte name block, records starting at 1580, 34 lights and a total length of 9468 bytes.
Its first light has RGB 255/121/12, intensity 0.3, position (-344.742, -31.3559, 258.093) and range 1 m.
Cache-backed tests cover this light, malformed offsets and truncations, and every installed map's records; they skip when the cache is absent.

| Field | Byte offset | Representation |
| --- | --- | --- |
| Version / header marker | Header +0 / +4 | u32 2 / 16 |
| Record count / name bytes | Header +8 / +12 | u32 |
| Reserved / record size | Header +16 / +20 | u32 0 / 232 |
| Names | File +0x3c | NUL-terminated UTF-16LE |
| Name offset | Record +0x10 | u32 relative to names |
| Observed light kind / RGB | Record +0x1c / +0x1d | u8 1 / three u8 |
| Intensity | Record +0x20 | f32 |
| Position / rotation | Record +0x38 / +0x44 | Three f32 each |
| Influence range | Record +0x58 | f32 metres |

| Map | Records by ascending bank suffix | Total |
| --- | --- | --- |
| m10_00_00_00 | 0000: 177, 0001: 25, 0002: 12, 0003: 19, 0004: 28 | 261 |
| m11_00_00_00 | 0000: 16, 0002: 6 | 22 |
| m11_01_00_00 | 0000: 34, 0002: 26, 0004: 34 | 94 |
| m11_02_00_00 | 0000: 6 | 6 |
| m13_00_00_00 | 0000: 36 | 36 |
| m15_00_00_00 | 0000: 112, 0001: 5 | 117 |
| m17_00_00_00 | 0000: 37 | 37 |
| m20_00_00_00 | 0000: 117 | 117 |
| m25_00_00_00 | 0000: 38, 0001: 7 | 45 |

The extractor writes versioned `cache/maps/<id>/lights.json` with separate banks, X-mirrored positions and raw record bytes.
The three requested maps and the many-light Hirata map were re-extracted using `SEKIRO_MAP_LIGHTS_ONLY=1` with our extractor.
This incremental mode writes the light sidecar and returns before unrelated probe, edge and model work; a normal map extraction also writes the sidecar.
Runtime selects the first bank, or an explicit `--light-bank number`, because the event-driven bank selection is not traced.
Banks are never combined, which avoids double-lighting shared placements.
All 735 observed kind bytes are 1 and render as `PointLight`; unsupported kinds are logged and skipped rather than assigned a guessed spotlight cone.
Shadow/cone/attenuation flags remain in the raw record and are not interpreted.

The provisional intensity mapping is `lumens = authored_intensity * 9000 * 0.1 * 4*pi`, or 900 candela per authored unit before isotropic conversion.
This shares the sun's calibration basis and preserves relative BTL powers, with a separate local calibration because native attenuation and photometric units are untraced.
The uncalibrated 9000 candela mapping flooded Castle's tower cyan and was rejected after inspecting its captures.
RGB bytes use the same linear convention as GPARAM RGB; native BTL colour space remains untraced.
`--local-light-scale multiplier` provides a calibration control, and `--no-local-lights` disables the feature.
Lights refresh twice a second, spawn within 60 m plus their authored range, and despawn outside it; the nearest 64 are retained and all local shadows are off.
`VolumetricFog.LocalLightScale` is not applied to surface lighting because its name locates it in volumetric scattering, and the proposed global surface multiplier was only an inference.

The bytes were checked locally without network access, but no local SoulsFormatsNEXT BTL source was found, so the requested independent library cross-check remains a gap.
Native bank activation, spot/directional kinds, exact attenuation, light occlusion and local shadow flags remain unresolved.

#### Steps 4 and 5 validation (2026-10-11)

Matched play controls use `--shadow 4,250 --shadow-casters 1000000 --shadow-bias 0.02,1.8 --no-local-lights`; the after run uses the default shadow budget and local lights.
Both use `--simple-ai --no-enemy --brain-memory off --uncapped`, hour 12, Outskirts start 3 or Castle start 2, and the same idle script.
Each median uses 80 pipeline-ready frames at simulation steps 130-169 and 190-229, excluding screenshot stalls and loading.
These scripted captures remain at approximately 60 Hz, so their wall times do not establish an uncapped speedup.

| Play view | Wall median before / after | Main-thread median before / after | Eligible map casters before / after |
| --- | --- | --- | --- |
| m11_00 start 3 | 16.635 / 16.637 ms | 4.239 / 3.831 ms | 18027 / 1260 |
| m11_01 start 2 | 16.705 / 16.686 ms | 4.650 / 2.971 ms | 15933 / 2958 |

The matching uncapped viewer controls and after views disable local lights, wait five seconds after loading, and report 59.8 / 62.6 fps in Outskirts and 60.2 / 112.7 fps in Castle.
Their reciprocal frame times are 16.72 / 15.97 ms and 16.61 / 8.87 ms respectively; these are short smoothed diagnostic samples, not long benchmark distributions.
The available main-opaque GPU timings are 2.07 / 2.48 ms and 0.97 / 0.92 ms respectively.
No shadow-pass timestamp appears among the six reported GPU passes, so these opaque timings do not measure the shadow savings.
Parallel jobs share this machine, which further limits comparisons of absolute cost.
Logs, CSV references and final play metrics are under `target/alt/visuals/v3-final-metrics.json` and the corresponding capture logs.

Every generated `v3-*.png` was opened and inspected, including the 120/180/240-step variants.
`v3-outskirts-final-control.png` / `v3-outskirts-final-after.png` and the corresponding Castle pair show no obvious shadow acne or new material seams; distant geometry is brighter after the deliberate shadow range reduction.
`v3-wolf-outdoor-shadow-on.png` / `v3-wolf-outdoor-shadow-off.png`, at hour 18, visibly retain Wolf's ground shadow to the right with shadows on and remove it with shadows off, without an obvious detached contact edge.
The temple `v3-wolf-shadow-*` pair is mostly roof-occluded and is not used to prove Wolf's cast shadow.
`v3-tower-night-off.png` / `v3-tower-final-on.png`, at hour 0, show a restrained blue/teal local contribution with wall, wood and cloth detail retained; 24 of bank 0000's 34 lights are active near the tower.
`v3-corridor-off.png` / `v3-corridor-final-on.png` show a subtle warm contribution with 25 active lights and no blown-out wood or Wolf highlights.
The initial `v3-tower-night-on*` captures are rejected for excessive cyan illumination; the intermediate `v3-tower-night-calibrated*` captures use the weaker 180 candela experiment, not the final 900 candela calibration.
The `v3-torch-*` view is obstructed by a foreground banner and does not establish a useful warm pool; the Castle start-1 night pair has little visible local contribution.
Existing oversized foliage cards, flat snow coverage and the absence of local occlusion remain visible limitations.

Workspace formatting and all-target Clippy with `-D warnings`, the requested release tests (89 passed including the doctest and `map_tangents`, two existing ignored), and the release game build pass offline with `CARGO_TARGET_DIR=target/alt` and six build jobs.
The verified command exit codes and logs are `v3-{fmt,clippy,tests,build}-exit.txt` and `v3-{fmt,clippy,tests,build}-verified.log`.
Saves were backed up under `target/alt/visuals/v3-save-backup` before running the game.
No commits, network access or protected-path writes were made.

### Step 6. Multi-layer map materials

Implemented in the 2026-10-11 follow-up with an original `ExtendedMaterial` and WGSL, retaining StandardMaterial lighting, shadows and fog.
The FLVER reader preserves semantic-5 UByte4Norm blend streams separately from the two texture UV sets, with all four original bytes retained.
The exporter packs stream 0 red (overlay), stream 4 red (snow) and stream 2 red (layer C) into COLOR_0 RGB and exports additional sampler metadata in material extras.
The shader neutralizes vertex colour before the StandardMaterial base sample, so blend weights cannot tint the base.
Unlayered meshes continue using ordinary GLB materials.

Verified families are `M[Multiple]` (base 8/3/9, overlay 5/4, layer C 7/0, snow 12/10) and `M[MultipleGround]` (base 8/3/9, overlay 5/4, snow 11/7).
Every numeric slot also requires the matching AlbedoMap, NormalMap or Mask semantic.
Real MTDs such as Outskirts Rock4 put `_3m` into an AlbedoMap slot and `_a` into a NormalMap slot; the runtime branch switches are untraced, so those unsuitable inputs are excluded rather than displayed as coloured patches.
Normal-only secondary layers remain supported when their albedo is absent.
Texture UV selection follows the MTD uv_number, clamped to the two actual UV sets; native per-layer UV transforms remain untraced.

Layer DDS albedos load as sRGB, normals and the base height mask as linear, with repeat addressing and anisotropy 8.
The original GLB sampler already explicitly uses repeat addressing.
Coverage uses endpoint-preserving smoothstep with a 0.25 half-width against base-mask R; snow reverses the height threshold and multiplies by smoothstep(0.2, 0.7, world-normal Y).
These coverage rules are approximations following the data interpretation in FwLo's MIT notes, not recovered native shader equations.
Only the base mask is bound, and normal layers use our existing BC5 conversion without native gloss.
`--no-layers` disables secondary texture loads and layer sampling; `--show-blend` displays the exported weights.

- Files: `crates/extract/src/maps/pieces.rs` (export layer textures and blend bytes for `M[Multiple]` and `M[MultipleGround]`), a new Bevy `ExtendedMaterial` and WGSL in `crates/game`, `map.rs` (swap material on load for layered meshes).
- Behaviour: use the first byte of packed blend streams 0 (overlay), 4 (snow with up-facing) and 2 (layer C), and `_3m` R as height for approximate coverage.
  Treat UV set per layer as a guess and keep the debug view that paints the blend weights as colour.
- Check: m17_00_00_00 (snow), m11_01 interior wall (flat interior materials), m11_00 start 3 stone walls with moss.
  Before and after pairs at the same camera.
- Risk: Bevy bind group texture limit forces masks to be skipped for layers, 7 to 10% frame cost on his gate, and the night snow overexposure he saw (check after Step 3).

### Step 7. Fog upgrade

- Files: `lighting.rs`, `map.rs`, a small WGSL addition or Bevy fog extension for height fog, exporter change to keep fog card materials instead of dropping them.
- Behaviour: `HeightFogDensity Scale`, `HeightFogOffset`, `HeightFogDistribution`, `StartDist`, `OppositeColor`.
  The shader recovery in `SHADERS.md` supersedes the earlier exponential-model proposal.
- Check: m10_00_00_00 (Hirata, foggy) and m17_00_00_00 start views at 12 h.
- Risk: `GameScale` and the real distance units are unverified [I], keep a tuning knob.

#### Step 7 status (2026-10-11)

Bevy `DistanceFog` is replaced with the recovered global density, directional blend, independent source gates, midpoint integration and analytical far continuation in `fog.rs` and `fog.wgsl`.
This is an HDR depth-based pass before bloom and tone mapping, using the main depth texture with MSAA preserved.
Sky/background and map/character scene pixels share the same integration.
`--no-fog` disables the pass, and `--visual-stats` reports frame and fog GPU timing.
The named provisional conversions are documented in `SHADERS.md`, including height scale `0.01` and affine height `world_y+offset`.
Native CPU GameScale packing, bank resolution, pre-exposure, local fog sources, shadowed fog and independent transparent depth remain gaps.
MSB FogParamID is preserved independently of LightSetID; the tested draw-param files contain no separate Fog group.

Arena collision and objects still use the existing 60 m radius.
Map pieces now render within a separate 1500 m radius, which includes all 3622 exported castle instances in these captures, versus 1553 with `--no-far-scenery`.
Far pieces bypass the local collision draw-group mask and remain subject to the existing focus-based shadow-caster budget.
They never become simulation collision, and `--full-map` retains its explicit full-collision meaning.
The cache contains only LOD0 GLBs; keeping the full pieces avoids inventing or regenerating LODs, and the measured working set fits the 32 GiB machine.

The v5 captures and measurements live under `target/alt/visuals/`, not the historical cache screenshot path above.
The matrix covers Owl and Genichiro with `--god --simple-ai`, hours 12 and 18, fog on/off and far scenery on/off, plus Outskirts start 3 and Fountainhead start 1 at noon.
Scripts and measured peak CPU working sets are retained with the captures.
All matrix images were inspected: noon haze increases with distance, dusk is warmer toward the sun, the far-scene crop edge disappears and the horizon joins the fog without visible banding.
The additional `v5-*-18-*-*-sun.png` views face the dusk sun.
Outskirts start 3 retains clear foreground silhouettes while its gate recedes into haze.
Fountainhead start 1 is inside a porch, so `v5-m25-lake-clear-12-{on,off}.png` supplements it with an unobstructed lake view.
Fog softens the distant terrain and sky seams there; the pale flat water surface remains the existing water-rendering gap.

Matched Owl noon measurements use a twenty-second idle script, `--god --simple-ai --uncapped --visual-stats`, 1600 x 900, RTX 5070 Ti and Ryzen 5700X3D with 32 GiB RAM.
Peak working set includes CPU assets and the renderer, not dedicated GPU memory.

| Fog | Scenery | Map load | Peak working set | Median frame after 12 s | Fog GPU mean |
| --- | --- | --- | --- | --- | --- |
| Off | 60 m crop | 1.8 s | 3.17 GiB | 16.688 ms | Disabled |
| Off | 1500 m | 3.2 s | 5.17 GiB | 16.669 ms | Disabled |
| On | 60 m crop | 2.3 s | 3.15 GiB | 16.674 ms | 0.344 ms |
| On | 1500 m | 3.3 s | 5.26 GiB | 16.652 ms | 0.343 ms |

Far scenery adds about 2.0-2.1 GiB and 1.0-1.4 s of warm loading; the simulation still loads the same 470455 collision triangles.
Frame medians remain near 16.7 ms despite `--uncapped`, and another job compiled during the measurements, so these wall timings do not isolate the far-geometry GPU cost.
The fog GPU timestamp does establish approximately 0.34 ms for this view.
Run logs, scripts, `v5-bench-metrics.json` and `v5-bench-summary.json` retain the measurements.
Workspace formatting, all-target Clippy with `-D warnings`, release tests for game/sim/formats/extract and the release game build pass offline with six jobs in `target/alt`.
Final check logs are `v5-{fmt,clippy,tests,build}-final.txt`.
The final release executable also exits successfully after saving `v5-final-validation.png`; the inspected Owl valley view preserves clear foreground edges and smooth distant haze, with fog GPU timestamps of 0.340-0.341 ms.

### Step 8. Foliage and alpha correctness

#### Step 8 diagnosis (2026-10-11)

Sunken Valley's sampled `M[AMSN]_md` ground/snow mesh decals use opaque BC1 albedos and vertex alpha spanning 0..1.
The old exporter omitted vertex colour entirely, making those blended cards fully opaque.
The exporter now carries that alpha with neutral RGB, because the sampled RGB bytes are constant (1,0,0) shader data rather than a visible red tint.
The cache-backed GLB regression checks preserved fractional opacity and neutral RGB.
The sampled Leaf1_Decal and branch35 materials already resolve real textures and MASK cutoff 128/255.
Their DDS transparency counts are 764806/1048576 and 1013262/1048576 pixels respectively; missing DDS alpha is not their root cause.
Cutout meshes are now always two-sided, with MTD double-sided int/bool flags also honoured.
When no albedo resolves through FLVER sampler names, primary MTD albedo/normal slots provide a fallback for Wind/Gradient naming differences, covered by a synthetic regression.
This fallback is not claimed as the cause of the sampled Sunken Valley decals.

Hirata's `_envmap_00.tpf` contains GIIV volumes while `_envmap_00_second.tpf` contains 148 GILM cubes.
Environment extraction discovers suffixed TPFs, exports the cubes and warns for containers without cubes instead of aborting the remaining map export.
Full Hirata extraction also writes 1243 placed edges, 112 grapple records and 261 lights in five banks.

The remaining triangular snow patch in the start-0 valley view is isolated to opaque `Asset_SnowGround_Multi` meshes, including m000064.
That MTD has BlendMode 0, AlphaRef (0,0) and DisableDepthWrite false; its snow weights are 255 throughout.
Its stored/reconstructed tangents agree with generated texture tangents at all 999 vertices.
Hiding m000064 removes much of that patch, while positive depth-bias experiments at 8 and 32 do not remove it.
Those unsuccessful bias changes were removed; the native SnowGround shader remains a gap rather than forcing opaque snow geometry into a foliage cutout mode.

#### Steps 6 and 8 captures and verification

All four requested before/after pairs were inspected at hour 12: `target/alt/visuals/v4-{outskirts,valley,hirata,mibu}-{before,after}.png`.
Outskirts start 3 gains snow on upward-facing wall tops, ledges and ground while vertical stone retains detail.
Valley start 0 has open branch silhouettes and snowy rock tops, but its near-right opaque SnowGround patch still shows triangular facets.
Hirata start 0 gains green moss on rocks and preserves open grass/root silhouettes; the newly discovered probes also change its lighting, so this pair does not isolate layers alone.
Mibu start 3 gains moss patches and cutout foliage, while the pale water remains the separate step-9 gap.
No obvious tiling seams appeared in these four views; they do not establish full-map or native-game visual equivalence.
The four additional `v4-perf-{outskirts,castle}-{off,on}.png` captures were inspected: leaf silhouettes stay open in both controls, and layers add snow coverage outdoors without visibly washing out Castle's wood/plaster interior.

Uncapped 600-step idle replays use `--simple-ai`, `--no-enemy`, `--brain-memory off`, and the same camera/start/hour, with `--no-layers` as control.
Medians exclude waiting pipelines, startup before step 150 and the first series' screenshot neighbourhood; sampled steps are 150..390 and 460..570.
The first off/on series recorded Outskirts 23.1422/27.07065 ms (+3.92845 ms, +17.0%) and Castle 21.216/19.6005 ms (-1.6155 ms, -7.6%).
The reversed on/off repeat recorded Outskirts off/on 26.6901/21.8766 ms (-4.8135 ms, -18.0%) and Castle 25.04765/19.75285 ms (-5.2948 ms, -21.1%).
Other jobs compiled during both series, and the sign reversal prevents attributing those wall-frame changes to layer cost.
The copied CSVs and sample counts are under `target/alt/visuals/v4-{perf,repeat}-*`, with summary JSONs alongside them.
FwLo's approximately 10% gate result remains his measurement, not a verified cost of our renderer.
Matched final-binary Outskirts viewer samples at 15 seconds report opaque GPU 2.25 ms off / 2.20 ms on and 39.4 / 44.8 fps; this single short pair also fails to establish a positive isolated layer cost.
Both `v4-gpu-{off,on}.png` captures were inspected, confirming that the same view differs in snow coverage while preserving foliage cutouts.

All ten maps were re-extracted sequentially after the final exporter changes; every extraction exited successfully.
Hirata's `edges.json`, `grapple.json` and `lights.json` were checked after full extraction.
Valley retains exactly three missing texture names: `m17_rock_92_3m`, `o007282_plant_00_a` and `o007282_plant_00_n`.
The collisionless m89 map skips edge extraction when no edge archive exists and still exports its display models.
Workspace formatting, all-target Clippy with `-D warnings`, the requested release game/formats/extract suite (99 passed, two existing ignored, including `map_tangents`) and release game build pass offline in `target/alt` with six jobs.
Logs are `v4-fmt.txt`, `v4-clippy.txt`, `v4-tests-final.txt` and `v4-build-final.txt` under `target/alt/visuals/`, with zero exit-code sidecars.
No commits, network access or protected-path writes were made by this work, and saves were backed up under `target/alt/visuals/v4-save-backup` before running the game.

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

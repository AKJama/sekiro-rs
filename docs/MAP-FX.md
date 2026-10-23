# Authored Ashina map effects

The map loader installs MSB region effects, ModelSfxParam attachments and parameter-resolved event-created object effects through the existing FXR player.
The first-visit fixture is the same one documented in MAP-COMPLETENESS.md.
There is no additional MapOffset translation for these native map placements.

## Inventory and event evidence

Run `cargo run --release --offline -p sekiro-sim --example map_fx_audit -j 6 -- cache` to reproduce the complete inventory without a renderer or retail-game session.
Each catalogue row retains the placement name, entity, FXR, mirrored world position, orientation, enabled state, activation part, dummy, ModelSfxParam row and last executed source command.
The report also retains every parameter-resolved FX command in instantiated event bodies, including branches that did not execute, separately from executed writes.
Native FXR trees and raw region records belong in the local audit output, not tracked source.

Outskirts has 157 SFX/WindSFX regions and Castle has 95.
These counts exclude ModelSfxParam and event-created object attachments.
Including parameter attachments, explicit object FX and registered-idol TAE produces 319 placements in Outskirts and 294 in Castle.
Of these, 246 and 230 are story-enabled before residency, distance and activation-parent gates.
The model-parameter contributions are 147 and 181 placements; explicit object FX contribute one and four, and registered-idol TAE contributes fourteen in each map (each idol's unlit and lit idle, #154).
An entity of -1 has no script-addressable entity ID and is still a real placement.
StartDisabled supplies the initial gate; an executed spawn or delete overrides it.
Ordinary object enablement and loaded-map visibility also gate each attachment.

ModelSfxParam rows use the object model number times 1000 plus the MSB model_sfx_param_relative_id.
A negative relative ID disables the automatic parameter path.
Each positive sfxId slot supplies its own dmypolyId, resolved through the object's FLVER dummy and parent-bone chain before the placed transform and X reflection.
For example, o116000 uses row 116000000, FXR 839530 and dummy 200.
RegisterBonfire commands retain their entity, unlock flag, kindling state and event provenance.
Their settled lit/unlit presentation reads the object's own TAE idle animation 11/1 and its time-zero FX events, including the authored FXR and dummy, rather than an invented glow on the idol material.
Both idles are inventoried for every idol, tagged lit or unlit (`IdolIdle`), and only the one matching the idol's state shows: lit when the story lights it (bonfire flag or kindling) or once Wolf has lit it in play (`Kit::found_idols`), #154.
The unlit idle (FFX 1010) is embers circling the idol 0.6 m out at 0.75 m up; the lit one (FFX 1011, 1012 at the temple's o115040) is the flame 1.1 m up, just above the hands.
Before #154 the inventory held only the first-visit idle, so an idol Wolf lit kept its circling embers instead of the flame.
An effect on a dummy takes the dummy's frame (its forward and upward), not only its position, as held weapons do (#278).
The idols' dummy 100 faces the model's -Z, where the raised hands are, so FFX 1011's flame node at +Z 0.125 m lands above the hands; placed with the part's facing alone it hung 0.25 m behind the head.
The animation-selection bridge is an explicit approximation until the native bonfire controller is traced; the FXR IDs and dummy locations themselves come from TAE.
The inventory encodes this provenance with command 200903, distinguishing registration-driven TAE from explicit 2006 object FX commands.
An activation part can be a meshless helper, so effects use its authored residency demand and story enablement instead of requiring that a mesh was rendered.

Native Outskirts event 11105107 deletes 1108010/1108020, waits two seconds and selects ordinary snow or invasion embers using flags 8302/9100, region 1102107 and map membership.
Castle event 11115170 deletes and restores snow entity 1114350, suppressing it for invasion, regions 1112150/1112151 and player SpEffect 110040.
The retained region graph resolves the authored composite indoor volumes.
These event regions are the game's only data rule for indoor snow (#79): no param or GPARAM names an indoor state, and SpEffect 110040 in the Castle event is the underwater check (SpEffectParam "underwater judgment"), not an indoor one.
Outskirts region 1102107 is one 20 x 16 x 8 m box over the temple building at MSB (94.4, -52.2, -154.1), collision h023200.
The Sculptor's hall at start 0 (collision h022400) has LightSet 110 like that building but no region, so the event keeps its snow on there.
Yet in the real game no snow falls inside the hall (checked by the user, 2026-10-22), so the engine hides weather under a roof by a rule not yet traced.

### Roof cover for falling weather (#79)

What the exe and data show (read-only Ghidra copy, `sekiro.exe` 1.6.0.0):
- The camera snow 839500 is one GPUStandardParticle (action 10000) node.
  Its fields2 slot 31, named `unkHideIndoors` by the community schema, is 1; slot 30 (`shadowDarkness`) is 0 and `particleCollision` is 0.
  The same slot is 1 on 29 of the 47 installed 10000 actions, including skill effects such as 7111240, so it is not a weather-only switch, and its runtime reader was not found within the time box.
- The GPU particle shaders have no indoor input.
  `CS_VFX_StandardParticleCollision` only bounces particles off the camera depth buffer (`g_csDepthMap`, shape 6, restitution in `m_HitParam`); the pixel shaders multiply colour (not alpha) by the sun shadow volume by `g_ps_BillboardShadowIntensity`.
- `GXVfxStandardParticleAppearance` (vtable 326abc8, init 40ea70 / 40f690, field parse 40eb10) is the CPU side; the character physics debug flag `DbgCheckRoof` (menu bc0280, read only in bcae00) belongs to a character roof sphere cast, not to effects.
- No string in the exe names weather, rain, indoor or interior effects.

What we do: after each map FX step, every particle of a falling-weather placement (camera-relative 10000) that lies below the highest play collision surface over its 0.5 m column is removed (`WeatherCover` in `crates/game/src/map_fx.rs`).
That reproduces what the game shows (no snow under the hall roof, snow in the open yard and over the roof) without claiming the engine's mechanism.
Columns are looked up lazily from `CollisionWorld::surfaces_at` and cached until the collision world changes.
A flake falling under an eave is removed rather than hidden; for straight-falling snow the two look the same.
Only the play mode has the collision; the map viewer and its render gates keep uncovered snow.
The underwater SpEffect 110040 condition is not implemented.
The rooftop ordinary-fight and Tomoe transitions are described in LIGHTING.md's Genichiro snow audit.

## Runtime and fidelity limits

The following counts are story-enabled/declared placements before camera range and parent-residency gates.
The complete per-placement list, positions, camera-follow classification and event controllers are retained in the local `map-placed-effects.csv` audit beside the reproducible census.

| FXR | Outskirts | Castle |
| ---: | ---: | ---: |
| 60 | 0/1 | 0/0 |
| 1010 | 5/6 | 7/7 |
| 1012 | 1/1 | 0/0 |
| 650642 | 0/0 | 0/1 |
| 800081 | 6/6 | 0/0 |
| 800082 | 4/4 | 6/6 |
| 800140 | 3/3 | 0/0 |
| 800150 | 0/13 | 0/3 |
| 800220 | 0/0 | 43/43 |
| 800240 | 0/0 | 4/4 |
| 800250 | 2/3 | 0/0 |
| 800270 | 1/1 | 4/4 |
| 810101 | 0/11 | 0/4 |
| 810115 | 0/20 | 0/7 |
| 811000 | 6/8 | 0/0 |
| 811020 | 0/0 | 9/9 |
| 811021 | 4/4 | 24/24 |
| 811201 | 0/0 | 0/1 |
| 811210 | 1/1 | 0/0 |
| 811240 | 0/0 | 0/1 |
| 811241 | 0/0 | 0/1 |
| 811242 | 0/0 | 0/1 |
| 811243 | 0/0 | 0/1 |
| 811260 | 0/0 | 13/17 |
| 811280 | 0/0 | 0/1 |
| 811300 | 0/6 | 0/5 |
| 811305 | 0/1 | 0/1 |
| 811310 | 0/1 | 0/12 |
| 811311 | 0/0 | 0/5 |
| 811320 | 0/9 | 0/5 |
| 811321 | 0/0 | 0/3 |
| 811500 | 0/0 | 0/1 |
| 839500 | 2/2 | 1/1 |
| 839510 | 8/8 | 0/0 |
| 839520 | 46/46 | 27/27 |
| 839530 | 50/54 | 37/37 |
| 839531 | 61/62 | 32/39 |
| 839535 | 0/0 | 7/7 |
| 839537 | 0/0 | 3/3 |
| 839538 | 0/0 | 2/2 |
| 839540 | 10/10 | 0/0 |
| 839541 | 3/3 | 3/3 |
| 839542 | 1/1 | 0/0 |
| 839543 | 1/1 | 0/0 |
| 839545 | 1/1 | 0/0 |
| 839547 | 24/24 | 1/1 |
| 839548 | 0/1 | 0/0 |

SEKIRO_NO_MAP_FX disables this module at runtime while retaining the same executable and renderer for before/after controls.
Effects farther than 120 metres sleep and submit no particles or lights; sleeping trees can be evicted when the shared 192-effect budget fills.
The player retains its global particle budget and per-node pool bounds.
Effect meshes remain transparent, omit prepass/shadow rendering and stay off the geometry/collision diagnostic layers.
Effect lights reuse a pool of entities and the existing scene-to-lumens bridge, with no count cap: Bevy's clustered culling bounds the cost, and the former nearest-16 cap made farther lights pop in on approach (#83).

Action 10000 has a bounded CPU implementation using the authored texture, colour, size variation, velocity variation, emission volume, follow factor and duration fields.
The integer emission/duration fields are interpreted on the FXR 30 Hz clock; the native GPU scheduler's exact count semantics remain unverified.
Each node keeps at most 2048 particles; when the steady population (count per emission times emissions per lifetime) is larger, every emission is thinned by the same fraction so the population stays steady (#84).
The camera snow 839500 wants 40 flakes every 1/30 s for 6 s (7200), so it keeps about 28 percent of them.
Before #84 the pool filled in 1.7 s and then emitted nothing until that batch expired, so a running Wolf left the snow behind for about two seconds in every six.
This fallback does not yet reproduce GPU collision, all emitter shapes, random turns, angular motion or every acceleration/growth field.
Camera-follow detection comes from native node movement 46 in the FXR tree, including effect references.

Action 607 uses authored normal-map UV scrolling, size, orientation, intensity and radius with Bevy's transmission image.
Its screen displacement uses the recovered 0.01 scale and linear normal-map samples; its flat billboard still approximates native volumetric distortion shapes.
Action 609 uses authored diffuse colour, multiplier and radius; native effect-light units and specular separation remain open.
The existing soft-depth path only copies matching four-sample depth, so single-sample map captures retain hard intersection edges.
Native model appearances 605 and wind-force action 10300 remain outside this job's sprite/weather implementation; 10300 does not directly draw pixels.
The module is a first-visit presentation snapshot; ongoing quest and boss-phase event execution remains the responsibility of the gameplay event host.
The existing global material, exposure and emissive-fidelity issues are not closed by placing these effects.

The spot-053 reference contains Gyoubu's unlocked boss idol, while the documented Outskirts fixture precedes Gyoubu and disables that idol through common event 20005500 and flag 9301.
That comparison must retain its state discrepancy rather than force an inactive idol or its FX into the first-visit scene.
The enabled temple idol at spot-002 supplies the matched glow check.
Paused-clock captures prime new effects to two seconds, freeze their simulation through normal/all-visible comparison and re-prime camera-relative trees after a camera teleport.
SEKIRO_MAP_FX_REALTIME selects wall-clock stepping for stationary performance measurement on that capture path.

## Map FX implementation and verification

Owned source changes are crates/formats/src/msb.rs; crates/sim/src/map_effects.rs and world_events.rs; the sim module/test registration lines and map_fx_audit example; crates/game/src/map_fx.rs, fxr.rs, fxr_tree.rs and fxr_distort.wgsl; the main module line and single map.rs install call.
The focused tests are the weather movement/follow/expiry case in fxr_tree.rs and the installed map-effect parameter/provenance case in crates/sim/tests/it/map_effects.rs.
Documentation changes are this file, the colour review in MAP-COMPLETENESS-SWEEP.md, FIDELITY.md, CREDITS.md and STATUS.md.
Only two Outskirts grey fixture cells belong to this FX job; the concurrent texture-routing changes and other source files retain their owner's changes.
No game data was regenerated, no reference image was copied, and no commit or publication was made.

The normal release workspace run reports 811 passed and 24 existing ignores, including 126 game unit tests and six game integration tests.
The installed map-effect test and weather movement/follow/expiry regression pass.
The game unit run takes 220.85 seconds and the sim integration run 655.00 seconds under concurrent builds/rendering; these are recorded durations rather than a claim of a fast clean-machine run.
The final dedicated all-map data gate exits zero.
Workspace formatting and release all-target Clippy with denied warnings pass with target/alt-fx and six Cargo jobs.

The final default six-view gates pass in 75.5 seconds for Outskirts and 88.1 seconds for Castle.
Their geometry totals are 0/493/0 and 1933/4885/0 for void/depth-gap/untextured, respectively.
Every per-view geometry measurement, selected collision and sorted visible-part set equals the same-executable runtime-off control.
The colour fixture review, depth quantization exception and repeat results are in MAP-COMPLETENESS-SWEEP.md and target/alt-fx/gates/final-comparison.json.
The frozen capture executable has SHA-256 7aa59510e01da0f3f75dc778f00799db9bc8f7e301d7ae877e1394238f53c476.

The complete 599-row local inventory is cache/map-fx-audit/2026-10-15-alt-fx/map-placed-effects.csv.
Its census JSON, native FXR trees, raw-region census, ModelSfxParam dump and EMEVD/TAE dump evidence remain in that gitignored cache audit directory.
The source example reproduces the inventory; test logs, gate reports and generated progress PNGs remain under target/alt-fx.

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

## Fire brightness audit, 2026-10-15

Spot-022 (shot 074, rendered with `--story afternoon`) showed the whole wall and ground saturated orange.
The causes were measured by ablation on the pose-server path (same camera, one term removed at a time), not guessed.

| Variant (spot-022) | Result |
| --- | --- |
| All map FX off (`SEKIRO_NO_MAP_FX`) | Grey-blue wall and ground, no orange. |
| FXR lights removed, particles kept | Still orange across the frame. |
| BTL lights off (`--no-local-lights`) | Identical to the default: no BTL light is within range of this view. |
| Only texture 1000 removed | Orange gone, the same as FX off. |

The whole wash is one layer.
FXR 800140 (placement `o116060_2000`, 6 m from the camera) has a camera-facing additive 603 layer: texture 1000 (a 64 by 64 radial alpha gradient), width 15 with uniform scale, `depthBlend` on, `premultiplyAlpha` on, colour (1, 0.5, 0.25).
The same layer exists in the other fire effects, which is why distant fires in spot-047 had a halo several times larger than the real flames.

### Cause 1: no soft-particle depth in single-sample captures

The soft fade (`alpha * saturate(min(sceneZ - quadZ, soft) / soft)`, from `GXFfxtessellateBlendSoftTexture.ppo`) reads the `DepthCopy` image.
That image is four-sample and is only filled when the camera is four-sample.
The map sweep, render gate and pose server use `Msaa::Off` with Bevy's depth prepass, so the copy was never written, `sceneZ` read as infinitely far and every soft sprite kept full alpha.
A 15 m disc 6 m from the camera therefore covered the frame.
The live viewer (four-sample) was already correct: the same pose there shows the glow confined around the fire.
Fix: `fxr.wgsl` and `fxr_multi.wgsl` read `bevy_pbr::prepass_utils::prepass_depth` when the view has `DEPTH_PREPASS`, and keep the copy for four-sample views.
The shader composition test now also validates the `DEPTH_PREPASS` variant.

### Cause 2: an invented sqrt 3 and a missing alpha square

The pixel shader splits the particle colour into length and direction, squares texture times direction and multiplies by the length.
For a texture of one that is `c_i^2 / |c|`.
`hue_squared` multiplied this by sqrt 3 "so a grey keeps its value".
The disassembly has no such factor, so every effect was 1.73 times too bright.
It is removed.
`g_ps_bPreAlphaMult` (constant buffer offset 544) multiplies the texture colour by its alpha before the square, so the colour of a soft glow falls off with alpha cubed at its edge.
The single-texture shader (`fxr.wgsl`) ignored the flag; it now receives it in the tangent w component and applies `rgb * a * a`.
The multi-texture path already honoured it.

### Lights: measured, not changed

No light was the cause.
Removing every FXR light changes spot-022 only slightly, and no BTL light is in range.
The FXR 609 lights are authored at diffuse 6.4 (brazier, two stacked nodes, radius 5) and 19.2 (800140, radius 5) with colour not squared; BTL fire lights in bank 0002 are 10 to 12 with squared colour and radius 10, so the magnitudes agree in order.
The deferred point-light shader (`GXLightAcc_DPointLight1.ppo`, disassembled with d3dcompiler) gives, for squared distance `d2`, radius `R`, colour w `b`:

```text
fall = saturate(1 - d2 / R^2)
t = 1.4 / (1.4 + d * b)
wrap = t^2 + (1 - t^2) * max(N.L, 0)
diffuse = Diff.rgb * fall * wrap / (1 + 0.7 * (d * b)^2)
```

There is no inverse-square term and the light is finite at its centre.
Bevy's stock point light is `I / d^2` with a range window, which is brighter than the native curve inside about 1 m and dimmer outside it (at 2 m with R 5 native is 0.84, Bevy 0.25).
So our lights are, if anything, under-lit at wall distance and over-lit only when a surface is within a metre of the flame.
`b` is now traced (b = 1/R) and the curve is implemented; see "Native point-light falloff and flame colour" below.

### Remaining differences at spot-022

The real frame shows a burning building with flames, layered smoke and rising embers.
Those effects (810101, 810115, 811300, 811320, 800150 near the camera) are gated off in the first-visit story state (events 11105109 and others), so they are absent, not wrong.
Matching the reference needs the burning story state (VISUAL-PARITY S1), not a renderer change.
Bloom was not edited; with the wash removed the bloom around the remaining flames is modest.

### Cost

The two changes add one branch and three multiplies per sprite fragment, and the prepass read replaces a load of an unwritten texture, so the depth-load count is unchanged.
No frame-time measurement was taken.
The gate times stayed within 56 to 61 seconds for Outskirts.

## Native point-light falloff and flame colour, 2026-10-15

### Native point and spot light curve (implemented)

`GXLightAcc_DPointLight1.ppo` and `GXLightAcc_DSpotLight1.ppo` were disassembled with d3dcompiler and re-read in full.
For a view-space light at squared distance `d2`, with `x = d2 * cb1[0].w`, the point shader gives:

```text
fall = saturate(1 - x)                           (cb1[0].w = 1/R^2)
t = 1.4 / (1.4 + d * b)                          (b = DiffColor.w)
wrap = max(t^2 + (1 - t^2) * N.L, 0)             (N.L unclamped)
diffuse = Diff.rgb * fall * wrap / (1 + 0.7 * (d*b)^2)
spec = Spec.rgb * GBuffer3.rgb^2 * fall * 2.5/(1 + (d*b)^2) * D * sat(N.L) / (max(N.V, N.L)*(1 - a/2) + a/2)
```

`D` is the GGX term `aa / (pi * (1 + nh^2 * (aa - 1))^2)`.
The roughness `a = (1 - 0.95 * sqrt(min(log2(P) / 13, 1)))^2`, with `P = 10(1+(d*b)^2) * 2^(13*gloss) / (10(1+(d*b)^2) + 2^(13*gloss))` and `gloss` the G-buffer 1 x channel.
The spot shader is identical and multiplies both terms by `max(1 - |uv|^2, 0)`, where `uv` is the projective spot-space xy divided by w (one at the cone edge), and skips fragments behind the light.
The diffuse buffer has no albedo and no 1/pi, so the compositor applies `albedo / pi * Diff` exactly as the directional path does.

Where `b` comes from: the recovered builder `3892e0` (reached from `210e00` with the effect's property block) stores the diffuse vec4 at object offset 0x40 as `(c^2 * a * power, ..., 1/range)`.
`fVar47 / fVar45` at offset 0x4c is `DAT_14328919c / range` and that constant is 1.0, so `b = 1/R` and `d*b = d/R`.
The spot shader's cbuffer matches (`PositionAndRange.w = R`, DiffColor.w = b), which confirms it.
So `x = d2 * (1/R^2)` is what Bevy already stores as `color_inverse_square_range.w`, and no extra per-light value is needed.
The same builder squares the diffuse and specular RGB, so FXR 609 colour is now squared too (`fxr.rs`); the previous note that 609 colour is not squared was wrong.
The builder's remaining inputs (object offsets 0x258, 0xf8, 0xfc and the `param_5` flag, which look like a time-of-day or story intensity modulation, and the random/flicker term at 0x108) are not traced and are left out.
`TransLightParam` (the backlight term) and the G-buffer translucency channel are treated as zero.

Implementation: `map_material.wgsl` `native_point_lights` loops the clustered point and spot lights of the fragment, subtracts Bevy's own `point_light`/`spot_light` result (same inputs as `apply_pbr_lighting`, same shadow) and adds the native one.
It runs for the native direct path only (`direct[12].x`, the existing `SEKIRO_GI_NATIVE_LIGHTS` switch) and is controlled by `direct[12].y`, set in `map_material.rs`.
`SEKIRO_MAP_POINT_LIGHTS_BEVY=1` restores Bevy's inverse-square response for A/B captures.
Light range, position and colour still come from the BTL and FXR code unchanged.

Light unit (traced 2026-10-15, residual still open).
The two builders (`3892e0` for spot lights, `388130` for point lights, both reached through the virtual `210e00` / `210010` with a flag pair that is `{scattering pass, shadows allowed}`) compute the diffuse vec4 as:

```text
rgb = c^2 * power * blink * distFade * 2^(-AntiExposureRate * EV)
w   = 1 / R
```

`c` and `power` are the diffuse colour and its alpha (property block offsets 0x70 and 0x7c).
`blink` is object offset 0x258 (spot) or 0x1dc (point), constructed as 1.0 and rewritten by the effect's flicker (FXR 609 `flickerBrightness`, `flickerIntervalMin/Max`).
`distFade` is `20d9c0`: 1 while `depth - radius` is within the view distance (object +0xbc, FXR 609 `maxViewDistance`), then a linear ramp to 0 over `min(0.07 * ViewDist, 10)` metres (`.rdata` 0x32890bc and 0x32892a0).
`depth` is a dot product with a camera plane, taken to be the view axis (inferred, not traced).
Until #83 effect lights were cut off hard at `maxViewDistance` from the camera, so the Outskirts lanterns (839531, 50 m) switched on as Wolf walked toward them; `light_view_fade` in `fxr.rs` now applies this ramp.
Offsets 0xf8 (`ScatteringScale`, FXR `volumeDensity`) and 0xfc (`ScatteringBlinkInfluence`) are read only when the first flag is set, which is the volumetric-fog scattering pass; the deferred pass never uses them.
Offset 0x108 is `AntiExposureRate` (FXR `adaptationExponent`, default 0, 0 in 800140, 800150 and the brazier effects).
The factor is `powf(2, -(rate * ctx[0x458]))`; `ctx[0x458]` is the scene EV, which `SHADER-INPUTS.md` records as 0 in shipped builds (`SC_PreExposure = 2^EV = 1`).
So the native buffer is in the same units as the directional light and the unit factor is exactly 1 (`NATIVE_LIGHT_BUFFER_SCALE`).
The FXR 800140 light also ramps its multiplier from 0 to 19.2 over 2 s, and the flicker (brightness 0.75 to 1 every 0.125 to 0.25 s) is not modelled; both only lower the mean.
Nothing traced explains the 3 to 5 times excess at the former 4500 bridge, so the earlier by-eye 0.25 stays as `NATIVE_POINT_LIGHT_UNEXPLAINED_RESIDUAL` (override `SEKIRO_MAP_POINT_LIGHT_SCALE` for the total).
The bridge itself is now traced as `pi / exposure = 3135.61` (see [EXPOSURE.md](EXPOSURE.md#brightness-audit-2026-10-15)), which is 0.697 of 4500, so the constant is rewritten as 0.25 * 4500 / 3135.61 = 0.359 to keep the same by-eye total.
The same audit found the sun, fill and local directional slots 4 to 7 times too bright against retail, so the point-light residual is probably the same unexplained direct-light factor rather than a point-light unit.

### Brazier flames: white-hot and small (fixed, size open)

Cause of white-hot: FXR sprite textures were decoded as sRGB.
The installed particle DDS files are plain UNORM (BC1 for 3000, 11622, BC7 UNORM for 11627, 11633, 31411), and the effect pixel shader squares the texture colour itself, so decoding as sRGB applied the gamma twice.
Mid-tone flame body was crushed to near black while the peak texels stayed at one, so only the hot core (rgbMultiplier 5 to 15) survived and clipped to white, which also made the flame look small.
`fxr.rs` now loads sprite textures as UNORM.
Brazier flames at spot-056 are now orange with the soft body back; the colour curve, `hue_squared` and the blend state were checked against the disassembly and are correct.
Height was fixed separately; see "Flame height" below.

### Spot-022 fire gating

The fire next to the camera is not gated.
Object `o116060_2000` carries FXR 800140 (a complete fire: flame 31421 and 31211, smoke 11633 and 21091, a 609 light) and is enabled by default; it spawns in the first-visit state and is rendered as the ground glow and small flames at a roped-off pile 6 m from the camera.
The effects previously called "burning story state" are different placements: the `燃焼` (burning) SFX entities 1104630 to 1104666 (810101, 810115, 811300 smoke) are created and deleted by event 11105109 on flag 8302: with 8302 off the event deletes them, with 8302 on it spawns them.
The group-off effects on `o115840` (811320, 800150) are toggled by event 20004112 for group 1106610.
So the gating we read is a real flag test.
If the user's reference was taken with 8302 set, the reference shows those effects and our first-visit state correctly omits them.
Whether 8302 should be on for the afternoon story state is a decision in `map_story.rs`, which the story-state agent owns; no edit was made there.
The flame sprites of 800140 were short for the same direction reason as the brazier (see "Flame height").

## Point-light falloff distance (#141), 2026-10-22

The map item glint (FFX 61) lit a warm pool about 1.5 m wide on the ground; the real game shows a small glow.
The earlier trace above that `b = 1/R` was wrong.
In both builders (`388130` point, `3892e0` spot) the diffuse w at offset 0x4c is `DAT_14328919c (1.0) / object[+0x90]`, while the radius is object `+0x19c` (stored as `1/R^2` in the position w).
The light base constructor `20d5f0` sets `+0x90` to 1.0.
For FXR 609 lights `+0x90` is taken to be the property `unk_ds3_p2_0`: across all 744 installed 609 lights it is a distance tied to the radius (R/2 or R/4 in most, for example R 3.75 with 1.875, R 2.5 with 0.625), and it is the only property that fits; the FXR writer of `+0x90` itself was not found within the time-box, so this mapping is inferred, not traced.
The glint has R 1.5 and falloff 0.25, so its `1 / (1 + 0.7 (d/0.25)^2)` term drops to 0.26 at 0.5 m and 0.08 at 1 m, where `b = 1/R` gave 0.86 and 0.76.
`fxr.rs` now reads the falloff and passes it in Bevy's `PointLight::radius`; `native_clustered_light` uses it for `b` when it is positive.
BTL lights keep radius 0 and therefore `b = 1/R`; which BTL field feeds `+0x90` is not traced.
Fire lights also fall off faster now (brazier 839530: R 5, falloff 0.625; ground fire 800140: R 5, falloff 1.25), so `NATIVE_POINT_LIGHT_UNEXPLAINED_RESIDUAL`, fitted by eye with `b = 1/R`, may need refitting at spot-022 and spot-056.
The other unused 609 fields do not explain the pool: `adaptationExponent` (0.6 here) scales by `2^(-rate * EV)` with EV 0 in shipped builds, `volumeDensity` is read only in the volumetric scattering pass, and the flicker (`flickerBrightness` 0.5 every 2 to 2.5 s) only lowers the mean; flicker is still not modelled.
Live check at Item_宝_02 (`--hour 1`): mean pixel value of the lantern base 1 m from the item fell from 108 to 53 and of the plants from 123 to 52 (ambient), and the item itself from 193 to 118.
Fire check at spot-022 (ground fire 800140, `--hour 1`, mean red): the ground at the fire went from 223 to 140, the ground between the fire and Wolf from 167 to 55, the dark side stayed at 28 to 26.
A real-game shot of the same fire (hour unknown, regions matched roughly) reads 176, 71 and 43, so the pool-to-dark ratio is now 5.4 and 2.1 against the reference's 4.1 and 1.65, where before it was 7.9 and 5.9.
The residual is left unchanged: the remaining gap is mostly our darker ambient (26 against 43), and the reference does not clearly support a refit.
The light radius also feeds Bevy's sphere-light model, but only its specular (roughness widening and a solid-angle term); diffuse uses the light centre.
Map pieces and characters (both `LayeredMaterial` with `NATIVE_DIRECT`, on by default) subtract Bevy's own point-light term with the same radius and add the native one, so it cancels there.
Only plain `StandardMaterial` meshes (blood decals, debug markers, the arena floor) see it: for the glint (radius 0.25) at 1 m with roughness 0.7 the specular peak drops to 0.63 of a point light with a wider lobe, which is not visible.

## Flame height and rising sparks, 2026-10-15

Symptom: the brazier (spot-056, FXR 839530) and the ground fire 800140 had short, low flames and no rising sparks, although the sprite sizes matched the data.
Size, emitter rate, lifetime and velocity curves were checked against the data first and agree.
The cause was the emission direction, in two places.

1. Node rotation pitch.
   Rotating a node by +X sent its Z emission axis downward (the previous mapping).
   Designers use +90 (64 rising disc and point nodes in the common effects, 40 of them with upward gravity) and +45 to stand the emitter upright or tilt a spark fountain, so the real sign is the opposite.
   With the old sign the brazier sparks and the 800140 smoke discs travelled into the floor and only gravity pulled them up.
   `make_node` now negates the X angle of `nodeTransform`.
   The Y and Z angle signs were not touched and are not independently verified.
2. Emitter direction 6.
   It was read as local forward (+Z).
   The torch effect 300163 (sparks with gravity -1 and a 5.6 degree cone, no node rotation) and the flame cores of 839530 only make sense if 6 is world up, so it is now the world up axis whatever the node tilt.
   Directions 1 to 5 are unchanged.
   Superseded by #60 below: 6 is local north.

Result at spot-056: the flame rises about 0.5 m above the coals with sparks above it, as in the reference (the coals themselves are still dark).
At the 800140 pile the flames are about 1 m tall, which is what the data gives (height 1 to 2 times scale 0.56, plus 0.375 m/s for 1.5 s).
Disc emitters (shape 401) still emitted radially for direction 0 here; superseded by #264 below (a disk's own direction is its +Z).
Open: spark sprites (width 0.05 times 0.56, texture 3000, rgb multiplier 15) are still fainter and fewer than in the reference, possibly a minimum screen size or bloom difference in the game.

## Emitter direction, 2026-10-22 (#60)

Symptom: at start 3 (beside the stone wall) and on the path toward the first gate, "ground steam" shot straight up, fast.
The effect is the blowing snow 839543 (region `SFX:吹き込む雪_ID839543_1000`, near start 3) and its twin 839542 (between starts 1 and 2).
Its streak nodes emit 16 particles every 0.2 s from a 5 x 0.25 x 1 m box at 8.3 m/s (movement 64), with direction 6 and a slight upward gravity.
With direction 6 read as world up, a headless step measured them rising at 9.3 m/s mean and up to 67 m/s.
The InitialDirection enum (reference/fxr, as FwLo's RustSekiro `docs/kb/fxr.md` and `src/fxr.rs` read it) is 0 by shape, 1 / 2 global up / down, 3 global north, 4 / 5 local up / down and 6 local north (the node's +Z).
`emit_point` now follows it: 6 is the node's +Z, and 1 to 3 are world axes turned back into the node's frame.
The snow now blows along its region's yaw at about 8 m/s, and the 6-driven fog banks (839520, 839540, 839541, 839547) drift sideways at 0.3 to 1 m/s instead of rising.
The torch 300163 and brazier 839530 nodes that motivated the world-up reading still rise through their negative gravity; their 6 nodes are slow (0.075 to 0.4 m/s).
Open: object dummy orientation (FLVER dummy forward / upward) is not applied to map object effects, only the dummy position; the brazier dummy 200 faces +Z with +Y up, so it does not change these effects.

## Disk emitter direction, 2026-10-22 (#264)

Symptom: the night ground fire on Ashina Outskirts (`--spawn -222.37,-59.0,111.03,270,0 --hour 1`) spread its smoke flat along the ground; in the game it rises as a near-vertical column.
The fire is the burning body pile o116060 (`o116060_2000`, ModelSfxParam 116060000, dummy 200) playing FXR 800140; the same effect is on two `SFX:焼死体の山` regions.
Its smoke nodes are disk emitters (shape 401, radius 0.375, direction 0) turned +90 about X, with an 11.25 degree cone, 1.27 m/s and gravity -0.25 (rising), lasting 18 to 20 s.
`emit_point` sent a disk's direction 0 out from the disk's centre, which the +90 turn lays flat, so the smoke spread sideways at 1.27 m/s.
The reference/fxr enum (EvenTorset's fxr, `src/enums/InitialDirection.yml`) gives a point, disk or rectangle emitter's own direction as LocalNorth (+Z), a sphere's as out from its centre and a cylinder's as out from its axis.
The disk now emits along its +Z, which the +90 turn stands upright.
Headless, 10 s at the placement, free particles older than 1 s: before 1.9 m up for 4.3 m out from the fire; after 6.1 m up for 0.7 m out.
The torch 300163, brazier 839530 and blowing snow 839542 do not use disk emitters and are unchanged.
The brazier's smoke drifts 1.3 m up for 0.9 m sideways: its nodes are turned +45 about X under a node emitter (2202) with direction 3, global north, so every brazier leans the same world way; not yet compared with the game.
Open: a box emitter's direction 0 (a random local axis inside, the face normal outside, per the same enum) is still read as +Z.

## Levels of detail and node emitters, 2026-10-22 (#268)

Symptom: after #264 the ground fire's smoke column read as a bright orange glow washing over Wolf; in the game the smoke is grey and only the flames and sparks are orange.
Cause: the node tree mixed up two node types.
Per EvenTorset's fxr `NodeType`, 2002 is LevelsOfDetail (one child at a time, by camera distance) and 2202 is NodeEmitter (its children are templates, `nodeSelector` 200 emits all, 201 one at random).
`NodeDef::new` kept only the first child of 2202 and every child of 2002.
So 800140 drew its near and far levels together: two 15 m additive orange glows (texture 1000, pulled 5.6 m toward the camera by its soft-sprite offset) and two smoke columns, while its near level's second template, the 604 fire sheet, never emitted.
Now a 2002 node keeps only its first, nearest level, as FwLo's RustSekiro does, and a 2202 node keeps every template.
The smoke colour data itself was read right: grey 0.5 times colour3 2.0 to 0.05, alpha blend, lit (lighting -2, drawn at the 1/8 stand-in).
The lit additive layer 31421 (blend 4, rgbMultiplier 15) still adds a faint orange at the column's foot; the real light term of lit particles stays open, as in FwLo's notes.
Effects touched: 62 common effects have a 2002 node with more than one level and 100 a 2202 node with more than one template, among them the brazier 839530 (its own 609 light, sparks and smoke now emit; before, the far level's light stood in for them) and the item glint 61 (no longer doubled by its far level).
The torch 300163, the fog wall 11 and the snow 839500 have neither and are unchanged.
Open: the LOD switch distances (`threshold0` is 1 in all 62 effects, `threshold1` 10 to 165) are not applied; the near level is drawn at every distance.

## Soft sprite pull and uniformScale, 2026-10-22 (#78)

The Genichiro door's fog wall (asset o001401, FXR 11) spread over the door frame and the walls.
FXR 11 has two 603 sprite nodes with `depthBlend` on, `unkDepthBlend1` 0.25, `unkDepthBlend2` 0, `uniformScale` on, width 0.75 to 1.5 (and 1 to 2) and particle scale 7.03 (or 5.63) by 2.81.

### What the exe does (traced read-only in the Ghidra copy)

603 is `GXFFX::GXFfxAppearanceBillboardEx` (vtable 326bdc8, created by 436bb0 when the action id is 0x25b); 604 is `GXFfxAppearanceMultiTextureBillboardEx` (326be98, created by 4408a0).
Their draws (436d70, 440a60) run on `GXFFX::GXFfxDrawContext` (vtable 3272e88).

- Sprite size: 436d70 evaluates the width property (slot 5); when `uniformScale` (fields1 +0x10) is set it reuses that value for the height instead of evaluating the height property (slot 6).
  Width and height are then multiplied by the particle's own X and Y scale, so `uniformScale` keeps the width and height properties equal, not the sprite square.
- TEXCOORD2.y: 436d70 writes `(w + h) * 0.5` (`DAT_143289124` = 0.5) at vertex offset 0x3c, the per-sprite size the soft geometry shader reads.
- `softSpriteScaleOffset`: the draw context's 46b400 resets it to (1, 0) (`MOV qword [+0x2d0], 0x3f800000`).
  Its vfunc 0xd8 (46ffb0), which the 603 draw calls, copies fields2 +0x64 (`unkDepthBlend1`) into x and fields2 +0x68 (`unkDepthBlend2`) XOR the sign bit (`DAT_14328b240` = 0x80000000) into y, at +0x2d0 / +0x2d4 and +0x590 / +0x594.
  Those are CPU images of the `DSCommonBuffer` cbuffer (image base +0x120, so `softSpriteParam` is +0x2c0); 46ee50 (vfunc 0xa8, called only with `depthBlend`) fills the depth linearization the pixel shader uses.
  The same fields2 block gives the bloom flag and colour, the four blend types and the four fade distances at the offsets the schema names, which pins the field indices.
- Shader (`GXFfxtessellateSoftTexture.gpo`): `pull = scale * T * 0.5 + offset` and `soft = scale * T * 0.5 + 2 * offset`, with the quad's depth moved by `pull` and its screen position kept.
  The pixel shader (`.ppo`) fades by `saturate(min(sceneZ - pulledZ, soft) / soft)`.

So pull = `unkDepthBlend1 * (w + h) / 4 - unkDepthBlend2`, soft = `unkDepthBlend1 * (w + h) / 4 - 2 * unkDepthBlend2`.

### What changed

`soften` in `crates/game/src/fxr.rs` used the larger side instead of the mean and added `unkDepthBlend2` instead of subtracting it; both now follow the exe.
603 / 604 sprites with `uniformScale` used the width times the X scale for the height; they now use the width property times the Y scale.
That is the main change for the fog wall: its sprites were 5.3 to 10.5 m square and are now 5.3 to 10.5 m wide by 2.1 to 4.2 m tall, and the pull drops from 0.66 to 1.3 m to 0.46 to 0.92 m.
Every soft or uniform-scale 603 / 604 effect changes the same way.
In the common effects, 603 nodes with (0.25, 0) as in FXR 11 are the second most common setting (609 nodes after 1897 with (1, 0)), and at least 95 depth-blended nodes have a non-zero `unkDepthBlend2` (for example (0.5, -0.5) in 56 nodes such as FXR 222001), which now pull the other way.
A negative soft distance (for example `unkDepthBlend2` 1.5 with a small sprite) means no fade, as in the shader.

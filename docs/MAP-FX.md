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
The animation-selection bridge is an explicit approximation until the native bonfire controller is traced; the FXR IDs and dummy locations themselves come from TAE.
The inventory encodes this provenance with command 200903, distinguishing registration-driven TAE from explicit 2006 object FX commands.
An activation part can be a meshless helper, so effects use its authored residency demand and story enablement instead of requiring that a mesh was rendered.

Native Outskirts event 11105107 deletes 1108010/1108020, waits two seconds and selects ordinary snow or invasion embers using flags 8302/9100, region 1102107 and map membership.
Castle event 11115170 deletes and restores snow entity 1114350, suppressing it for invasion, regions 1112150/1112151 and player SpEffect 110040.
The retained region graph resolves the authored composite indoor volumes.
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
Effect lights reuse a bounded pool of sixteen entities and the existing scene-to-lumens bridge.

Action 10000 has a bounded CPU implementation using the authored texture, colour, size variation, velocity variation, emission volume, follow factor and duration fields.
The integer emission/duration fields are interpreted on the FXR 30 Hz clock; the native GPU scheduler's exact count semantics remain unverified.
The 2048-particle per-node bound can reduce dense weather emission.
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
`distFade` is `20d9c0`: 1 inside the view distance, a linear ramp to 0 at `ViewDist`.
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

Result at spot-056: the flame rises about 0.5 m above the coals with sparks above it, as in the reference (the coals themselves are still dark).
At the 800140 pile the flames are about 1 m tall, which is what the data gives (height 1 to 2 times scale 0.56, plus 0.375 m/s for 1.5 s).
Disc emitters (shape 401) still emit radially for direction 0; with the corrected rotation the 800140 smoke discs now rise through gravity and speed, the radial default is unverified.
Open: spark sprites (width 0.05 times 0.56, texture 3000, rgb multiplier 15) are still fainter and fewer than in the reference, possibly a minimum screen size or bloom difference in the game.


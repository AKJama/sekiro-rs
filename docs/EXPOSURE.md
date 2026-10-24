# Exposure and tone audit, 2026-10-15

The historical comparisons below used night lighting against daytime retail screenshots.
The later [reference-state audit](VISUAL-PARITY.md#reference-state-and-snow-remeasurement-2026-10-15) supersedes their use as a current retail brightness or snow-colour ranking.
It retains the same linear-luminance, saturation and circular-hue formulas and the six existing scene masks, but selects time from the known boss-progression candidate rather than `--story night`.
Snow-region masks are separate, manually reviewed fixed polygons on settled snow, without luminance or colour threshold selection.
They measure the visible snow-covered surface including its dirt, tile gaps, sparse vegetation, shading and any local haze, rather than a pure albedo texel or a snow material buffer.

The subsequent [post-chain colour audit](COLOUR.md) checks the remaining gamma/LUT/exposure inputs, adds the missing pre-gamma matrix and full authored bloom pyramid, and records a new six-view comparison.
The measurements below remain the historical isolated exponential-base comparison.

The mode-7 sensitometric reconstruction used the wrong exponential base.
The native table writer uses base-two shader instructions; the previous WGSL and research transcription used natural exponentials with the same coefficients.
This is a shared post-processing correction, with no authored GPARAM changes or map-specific colour adjustment.

## RustSekiro inspection

Inspected [FFwLo/RustSekiro](https://github.com/FFwLo/RustSekiro/tree/0c9e5915b3d98d8a1e00319a64513100ff322e5e), pushed head `0c9e5915b3d98d8a1e00319a64513100ff322e5e`, dated 2026-10-15 00:38:48 UTC, before changing the renderer.
The GitHub CLI failed through the configured proxy; read-only GitHub connector requests supplied the pinned tree, source and MIT licence.
The shared work board already contains the separate sekiro-rs exposure claim.
No remote repository was edited or messaged, and sekiro-dearxan was not accessed.

| Area | Pushed implementation | Decision |
| --- | --- | --- |
| Tone | `src/grading_lut.wgsl` uses sensitometric mode 7, contrast 0.5, output power 1/2.2 and a display-domain LUT; its coefficients also use WGSL `exp` | Same transcription error; retain our broader authored mode/contrast handling and correct the base from native evidence |
| Exposure and meter | `src/map.rs` uses Bevy histogram exposure, a 64x64 centre mask, soft limits, and by-eye `METER_EV=2.6` / `BASE_EV=-0.8` | Do not import those offsets; our weighted biased-log GPU reduction already implements the recovered model |
| Adaptation | Authored speeds mapped to Bevy's first-order rate approximation | Retain our native `1-10^(-dt*speed)` response and one-frame CPU-gain latency model |
| Bloom | Authored enable/threshold/level count and geometric weight normalizer mapped to Bevy additive bloom | His notes acknowledge the pre-auto-exposure threshold, Gaussian, saturation and feedback gaps; retain our post-exposure extraction |
| GameScale | Fog lengths multiplied and densities divided by authored GameScale | Already implemented locally; no evidence that this is a tone or global brightness multiplier |

No RustSekiro implementation was copied for this correction.
Existing high-level RustSekiro attribution remains in [CREDITS.md](../CREDITS.md).
His knowledge-base notes still describe unresolved colour/exposure calibration, so his renderer is not a verified retail colour target.

## Native rule and scope

For scene input `x`, mode 7 computes `A=1-exp2(-1.442695*x)` and `B=1-exp2(-4.456669*x)`, then `A+FilmicContrast*(B^3-A)` before output gamma.
The evidence is the installed `ToneMap_WriteSensitometricToneMappingTablePS.ppo` disassembly, whose exponent operands directly contain these two constants.
[Microsoft's instruction definition](https://learn.microsoft.com/en-us/windows/win32/direct3dhlsl/exp--sm4---asm-) specifies that shader-model-4/5 assembly `exp` computes `2^src`.
The native table address/reconstruction still cancels analytically; table interpolation precision remains unimplemented.

At contrast 0.5, native pre-gamma samples for scene values 0, 0.01, 0.18, 1 and 4 are 0, 0.0049891566, 0.1211626855, 0.7508116073 and 0.9908357271.
The regression executes the actual production WGSL tone function on the GPU against these independent samples.
It covers shadows, middle tones and the highlight shoulder rather than merely checking that shader composition succeeds.
Before the correction it fails at scene input 0.01: the production GPU result is 0.007203084, versus native 0.0049891566.

The authored exposure gain `2^Exposure`, middle gray, centre weight, light/dark speeds, freeze behaviour and natural-exponential soft limits remain the recovered rules documented in [SHADER-INPUTS.md](SHADER-INPUTS.md#tone-map-and-exposure).
The CPU soft-limit routine `24d920` calls a natural-exponential implementation; the GPU adaptation shader separately uses a base-ten response expressed through base-two instructions.
Neither should be changed to the sensitometric curve's exponent coefficients.
Missing auto-exposure enable remains false, consistent with the tone-record constructor clearing its enable field and `2565a0` copying the authored integer to the filter enable byte.

The earlier fixed `SCENE_TO_BEVY=4500` bridge was inferred, not a recovered native unit conversion; the [brightness audit](#brightness-audit-2026-10-15) below replaces it with the traced relation.
The exact material/BRDF response, SSAO, matrix/vignette producers, full bloom scheduling/feedback and metering reduction layout remain fidelity limits.
An observed luminance ratio cannot establish replacement values for any of those terms.
The correction therefore does not claim to solve all L1 brightness, hard shadows or M1 snow colour.

## Measurement method

Six accepted saved cameras cover spot-002/shot-004, spot-003/shot-012, spot-009/shot-034, spot-025/shot-085, spot-091/shot-290 and spot-092/shot-293.
Every capture uses the matcher windowless renderer, night story, the saved player/camera/FOV, 2560x1440 output and normal authored post settings.
Processes run serially after checking for an existing renderer; no retail game session or visible window is opened.
The initial reproduction using the preceding shadow-audit executable measures spot-002 at 19.69684 percent linear white, consistent with the reported 19.70544.
The controlled comparison is recorded separately from that reproduction.

Linear luminance uses the exact inverse-sRGB transfer and Rec.709 weights `(0.2126,0.7152,0.0722)`, expressed as percent white.
The four existing scene masks from the light audit are reused; the two additional views use the matcher's fixed overlay/player exclusion mask.
Mean saturation uses display-domain HSV saturation, and hue is the saturation-weighted circular mean in degrees with red at zero.
Hue is a whole-scene statistic and cannot identify a pure snow material or substitute for landmark alignment.
The four available ground masks are also retained in the detailed measurements.

Requested real/before/after panels, individual captures, saved poses, process manifests, masks and numeric results live in the external progress collection's `2026-10-15/exposure/` directory.
No reference image, extracted game asset or decompiler output is added to tracked source.

## Measured results

Controlled before/after executable SHA-256 prefixes are `67059ea4b77cea52` and `8d3e1b15b19a1774`; complete hashes are retained in the external manifests.
Only `crates/game/src/tonemap.wgsl` differs among the 81 recorded game source files.
All six complete sorted visibility records, poses, FOVs and selected collisions agree.
SHA-256, size and modification-time records agree for all 17,214 map/draw-param asset files, totalling 17,731,526,735 bytes, across the repeated captures.
An initial rerender incorrectly accepted a stale completion marker; its images and numbers are archived in `pre-control-rerun/` and excluded here.
Fresh requests use unique completion IDs, and the repeated controls retain the same scenery.

| Spot | Setup | Linear Y % | Saturation % | Hue degrees | B/R |
| --- | --- | ---: | ---: | ---: | ---: |
| spot-002 | real | 7.02082 | 23.961 | 202.908 | 1.31164 |
| spot-002 | before | 19.70435 | 32.071 | 200.422 | 1.40281 |
| spot-002 | after | 13.63111 | 35.458 | 201.236 | 1.49803 |
| spot-003 | real | 7.50628 | 23.897 | 209.900 | 1.30674 |
| spot-003 | before | 22.72798 | 35.665 | 203.473 | 1.38738 |
| spot-003 | after | 15.76971 | 39.739 | 204.095 | 1.49961 |
| spot-009 | real | 10.47270 | 22.067 | 206.722 | 1.26647 |
| spot-009 | before | 10.16814 | 41.641 | 205.526 | 1.62831 |
| spot-009 | after | 7.24057 | 44.057 | 205.089 | 1.70245 |
| spot-025 | real | 10.60234 | 17.631 | 206.315 | 1.15327 |
| spot-025 | before | 12.84828 | 34.280 | 202.327 | 1.40763 |
| spot-025 | after | 9.15435 | 36.761 | 202.266 | 1.46804 |
| spot-091 | real | 19.73271 | 15.433 | 197.001 | 1.18173 |
| spot-091 | before | 22.34946 | 34.187 | 205.309 | 1.42545 |
| spot-091 | after | 16.47601 | 37.722 | 205.492 | 1.51060 |
| spot-092 | real | 4.34681 | 22.497 | 196.323 | 1.28661 |
| spot-092 | before | 7.07763 | 35.899 | 202.035 | 1.53836 |
| spot-092 | after | 4.70150 | 38.550 | 201.818 | 1.61180 |

spot-002 mean luminance decreases 30.82%, with a smaller absolute difference from retail; saturation changes 32.071% to 35.458% versus retail 23.961%.
spot-003 mean luminance decreases 30.62%, with a smaller absolute difference from retail; saturation changes 35.665% to 39.739% versus retail 23.897%.
spot-009 mean luminance decreases 28.79%, with a larger absolute difference from retail; saturation changes 41.641% to 44.057% versus retail 22.067%.
spot-025 mean luminance decreases 28.75%, with a smaller absolute difference from retail; saturation changes 34.280% to 36.761% versus retail 17.631%.
spot-091 mean luminance decreases 26.28%, with a larger absolute difference from retail; saturation changes 34.187% to 37.722% versus retail 15.433%.
spot-092 mean luminance decreases 33.57%, with a smaller absolute difference from retail; saturation changes 35.899% to 38.550% versus retail 22.497%.

Mean saturation and B/R move farther from retail in all six views, and the requested browner snow appearance remains unresolved.
These mixed image results isolate one proven transfer-function defect while retaining independent illumination, material, sky and post-composition gaps.
No exposure offset, saturation compensation or warmer tint is inferred from the residual errors.

## Validation and deliverables

`cargo test --release --offline --workspace -j 6` passes 814 tests with 25 existing ignores, including the installed-data gate, 127 game unit tests and seven game integration tests.
The native GPU curve regression fails on the old shader and passes on the corrected production function; all native post shader permutations also compose and validate.
`cargo clippy --release --offline --workspace --all-targets -j 6 -- -D warnings` and `cargo fmt --all --check` pass.
All Cargo commands use `CARGO_TARGET_DIR=target/alt-exposure`.
The first workspace test build encountered a Windows executable lock; the successful retry uses copied capture executables so Cargo can replace its own outputs independently.

Both independent final render-gate repeats pass against the reviewed colour fixtures: Outskirts 49.9475 seconds and Castle 58.6862 seconds, under the unchanged 120-second limit.
All twelve final void/depth-gap/untextured triples equal the fresh before controls, with totals 0/493/0 and 1933/4885/0 respectively.
Only grey and pop-in fixture cells change; pose fields, geometry baselines, resolution and tolerances remain identical to the saved pre-change fixtures.
The final colour totals are grey/pop-in 2796/26172 for Outskirts and 1169/31146 for Castle, within the original per-view tolerances of the new fixtures.
Every colour-fixture change is explained separately in [MAP-COMPLETENESS-SWEEP.md](MAP-COMPLETENESS-SWEEP.md#exposure-tone-fixture-review-2026-10-15).

The renderer change is `crates/game/src/tonemap.wgsl`, with the GPU regression in `crates/game/tests/it/native_post_shaders.rs`.
The two Ashina `.render.json` fixtures retain their geometry fields.
Documentation changes are this report, `LIGHTING.md`, `SHADER-INPUTS.md`, `SHADERS.md`, `FIDELITY.md`, `MAP-COMPLETENESS-SWEEP.md`, the exposure follow-up in `VISUAL-PARITY.md`, and `STATUS.md`.
The existing `CREDITS.md` is unchanged because this task copied no RustSekiro implementation.

The external `2026-10-15/exposure/` collection contains six full-resolution `spot-*-real-before-after.png` panels, their individual before/after images, previews, masks, poses, source/binary manifests, complete visibility records, 17,214-file asset fingerprints and measurements.
`capture-verification.json`, `verification.json` and `final-checks.json` retain the equality checks and final gate results.
Raw gate reports and Cargo logs remain under `target/alt-exposure/`.
Nothing was staged or committed, and the protected mountain/visibility/sky-ordering code was not edited by this task.

## Brightness audit, 2026-10-15

The reference comparisons use the `genichiro-defeated` afternoon state (12:00; Outskirts exposure 0.4 and MiddleGray -4.2, Castle exposure 0.5 and MiddleGray -5.4).
Both areas author auto exposure on, with Adaption Min -0.01 and Max 0.5 (Castle Max 0.01), so the soft limit bounds the adaptation offset to about -0.01 to +0.5 stops.
Exposure therefore cannot rescue a scene whose radiance is several stops off; the radiance scale itself has to be right.

### Method

Every ablation uses the matcher windowless renderer at the saved camera, player and FOV of the ten reference spots, 2560x1440, `genichiro-defeated`, one process at a time.
Real and ours are compared as linear scene values as well as display luminance.
LUT 30 is the identity for greys (the cached strip returns 0, 2, 4, ... 15 for those cell inputs), so a retail pixel is inverted through `display^2.2` and the mode-7 tone function (base-two form, contrast 0.5, checked against the documented samples) to the post-exposure scene value `x`.
The inverse is a monotone table over `x` in 0 to 10, so values above about 4 are saturated and only ratios up to that level are trusted.
Real bloom and glare are not removed, which slightly inflates the retail values.
The external `2026-10-15/brightness/` collection holds the panels, `measurements-before-after.json`, the ablation tables and the scripts (`tools/`).

### Native arithmetic that fixes the unit

- `GXLightAcc_DDirLight1` accumulates `Col * N.L` with no `1/pi` and no photometric unit, then multiplies by `SC_PreExposure`.
- `GXLightCmbAlbedo` multiplies that irradiance by the G-buffer albedo (`g_IrradianceTexture * g_GBuffer2Texture`) and `GXLightComposite` only adds irradiance, specular and emissive.
  A white Lambert surface with `Col * N.L = 1` therefore has native radiance exactly 1.
- `SC_PreExposure` is `2^EV` with EV zeroed by the scene constructor (`13cb60` writes zero to `+0x112a4`, `13dd90` clears the auto-adjust byte), so it is 1.
- The tone gain is `2^Exposure / AvgLuminance * 2^(-softclamp)` (`250060`, re-read), and the table shader's `x / (x + 0.2)` address is cancelled by the table writer, so the post chain adds no hidden factor.
- The SDR post shader has no further scale: the scene value is multiplied by `g_ToneMapSceneLumScale` and added to glare, nothing else.
- Bevy's diffuse term is `albedo / pi * lux * exposure` with `Exposure::default()` (EV100 9.7, `exposure = 1 / (2^9.7 * 1.2)`).
  Equating the two gives `SCENE_TO_BEVY = pi / exposure = 3135.61`, and `ENV_PER_UNIT = SCENE_TO_BEVY / pi = 1 / exposure`.
  The fitted 4500 was 1.435 times too large for every light, hemisphere, probe, sky and fog term.

Independent check of the unit: the sky dome and fog colours carry native radiance.
At the Castle roof (spot-091, open sky region) the retail sky has `x` = 0.504/0.622/0.691, and ours at 4500 was 0.632/0.842/0.992, a ratio of 1.25 to 1.44.
Scaling by 3135.61 / 4500 = 0.697 gives 0.440/0.587/0.691, which matches retail in blue exactly and within 13 percent in red and green.
A unit test asserts the relation, so a change of the default camera exposure fails it.

### Ablations

All values are the mean post-exposure scene value `x` over the reference scene mask / snow mask, measured before the bridge change (4500).
Retail is 0.097 / 0.262 (spot-002), 0.104 / 0.253 (spot-003) and 0.170 / 0.152 (spot-090).

| Setup | spot-002 | spot-003 | spot-090 |
| --- | --- | --- | --- |
| all terms | 0.469 / 1.675 | 0.733 / 1.879 | 1.425 / 2.050 |
| no sun slot 0 (Bevy and native) | 0.267 / 0.734 | 0.373 / 0.789 | 0.719 / 1.004 |
| no fill slot 1 | 0.380 / 1.394 | 0.551 / 1.503 | 1.173 / 1.683 |
| no local slots 2 and 3 | 0.374 / 1.346 | 0.595 / 1.549 | 1.219 / 1.788 |
| no directional specular | 0.435 / 1.625 | 0.687 / 1.831 | 1.275 / 1.855 |
| no hemisphere or GI bounce | 0.448 / 1.626 | 0.716 / 1.832 | 1.199 / 1.676 |
| no sun, fill or local slots | 0.093 / 0.096 | 0.058 / 0.083 | 0.261 / 0.377 |
| no map point lights | 0.457 / 1.677 | 0.728 / 1.880 | 1.424 / 2.050 |
| no sun shadows | 0.540 / 1.798 | 0.793 / 1.879 | 1.470 / 2.058 |

Findings from the table:

- Everything that is not a directional slot (hemisphere, GI, environment probe, sky) gives 0.06 to 0.26 and is the same order as retail.
  With the three light slots removed, scene Y is within 15 percent of retail at spots 009, 025 and 092 (1.04, 1.09 and 1.15 times), and 0.87 and 0.48 times at 002 and 003.
  At the open Castle views 089, 090 and 091 it is 1.5 to 1.7 times retail.
- The sun slot contributes 0.94, the fill 0.28 and the local slots 0.33 at the spot-002 snow, so the three slots add about 1.5 against a retail snow total of 0.26.
  The retail snow therefore carries at most about 0.1 to 0.25 of the native `Col * albedo * N.L` that the traced arithmetic predicts for an open lit surface.
- Retail roofs (spot-091) show a lit to shaded ratio of about 8 in `x`, ours about 7.5, so the sun to non-sun balance is not the discrepancy; lit and shaded roof pixels are both 6 to 7 times too bright at 4500.
- Map point lights and sun shadows change the result by 2 to 10 percent, so neither is the cause.
- Scaling the three slots by 0.1 and 0.2 (and by 0.25 together with ambient 0.25) leaves per-spot scene Y errors of 1.2 to 2.4 times, 1.3 to 3.1 times and 1.1 to 2.5 times respectively.
  No single factor fits all ten spots, so a global multiplier would be a fit rather than a cause, and none is applied.

### What the audit rules out

- Tone curve, colour matrix, LUT 30 (identity), bloom and the exposure gain formula, which are all re-read.
- Adaptation: the retail log-mean scene value after the 2^0.4 gain is 0.035 to 0.106, within 2 times of `2^MiddleGray * gain`, so the real game sits near zero offset, and our offsets (about 0.5 stops, the soft limit) cannot account for a 4 to 7 times error.
- Pre-exposure and anti-exposure terms (constructor-zeroed, see above).
- G-buffer albedo multipliers: `g_DiffuseMapColor` is exactly (1,1,1) in 3040 of the installed MTDs; the first 25 of the 77 exceptions listed are all character, rope and effect materials, and the Outskirts rock and simple wall MTDs read directly are (1,1,1).
- Albedo colour space: the snow and roof albedos checked are BC1 sRGB (DXGI 72), decoded as sRGB.

### Remaining unexplained factor

The excess is carried by the directional light slots, not by the sky, fog or ambient units.
The most specific structural gaps found in the deferred pipeline, none proven to explain the magnitude, are:

1. `GXLightDeferredAO` multiplies the whole irradiance accumulator, direct and indirect, by `g_ShadowTexture.x * g_ShadowTexture.y` (SSAO red times a remapped G-buffer baked-occlusion channel).
   We apply occlusion to ambient terms only, ordinary map pieces carry no baked COLOR1 occlusion, and there is no SSAO.
2. The sun shadow factor is `(1 - K) + K * shadow` with `K = FC_ShadowColor`.
   Its default (the GPARAM `Directional Shadow Color`, absent from every Ashina file) was not traced, and our sun shadows black out fully and stop at 60 m.
3. `FC_SsaoWeight` for the four slots comes from LightSet fields that default to 0 in `default.gparam`; whether the engine takes that default for a map file that omits them was not traced.
4. The GI scale uploads `g_envMapDiffuseColorScale` and `g_giLightingScale` are CPU values that were not found, so the absolute ambient scale could still differ from ours by a common factor.

The point-light residual (now `BTL_POINT_LIGHT_UNEXPLAINED_RESIDUAL`) is in the same range as the slot excess (0.15 to 0.35 of the traced native value).
For FXR lights it turned out to be the over-wide `b = 1/R` falloff it was fitted with, so since #323 it applies to BTL lights only (see MAP-FX.md).
It is kept as the earlier by-eye total, rewritten relative to the corrected bridge (0.25 * 4500 / 3135.61 = 0.359).

### Result

Retail, before and after for the ten spots are in [VISUAL-PARITY.md](VISUAL-PARITY.md#brightness-audit-and-unit-bridge-2026-10-15).
The change lowers every scene by about 20 to 30 percent and leaves all geometry gate counts unchanged.
Diagnostic switches for the ablations are `SEKIRO_ABLATE` (`nosun,nofill,nolocal,nospec,noamb,nogi`), `SEKIRO_SCALE_DIRECT` and `SEKIRO_SCALE_AMBIENT`; they default to the identity.

## Brightness audit, second pass (2026-10-15)

Each candidate from the first pass was traced in the executable and the shader disassembly, then tested with ablations at the matched cameras.
Panels and measurements are in the external `2026-10-15/brightness-2/` collection.

### Ambient occlusion (GXLightDeferredAO)

- `GXRend_CompositeShadowMask` writes `x = SSAO.x`, `y = FC_Mult.y * G1.z + FC_Add.y`, `z = FC_Mult.z * G1.z + FC_Add.z`, `w = 1`.
  The constants are read from the image (`2751b0` uploads two static float4): `FC_Mult = (0, 1, 0, 0)` and `FC_Add = (1, 0, 1, 1)`.
  So `y = G1.z` with no remap, and `GXLightDeferredAO` multiplies the whole irradiance and its specular by `SSAO.x * G1.z`.
- `G1.z` is the interpolated vertex `COLOR1.x` (the map G-buffer pixel shaders write `o1.z = COLOR1.x`).
  In the extracted Outskirts models `_MAP_COLOR1` is (1, 0, 0, 1) on 99.7 percent of 10.8 million vertices; only 80 of 5484 primitives (the mountains) have values below 0.5.
- The SSAO value is `AOScale * ao + AOOffset` with `ao = 1` on an unoccluded flat surface (worked through `ScreenSpaceAO2_WithNormal`).
  `default.gparam` gives scale 1 and offset 0; the `m11_00_0200` and `m11_00_0220` post banks set `Ver2 AO Scale` 0.7 and `Sample Offset` 7.
- `FC_SsaoWeight` comes from the LightSet fields, which are absent from every Ashina file, so the `default.gparam` value 0 gives `1 + 0 * (ssao - 1) = 1` for direct light.
- Verdict: ambient occlusion is at most a factor 0.7 on open surfaces and cannot explain a 4 to 7 times excess.
  It is not implemented as a pass: the eight-pair sample table and the rotation texture are CPU-generated and were not recovered, and an approximation would be an invented term.

### FC_ShadowColor

The sun factor is `(1 - K) + K * shadow` with `K = cbSceneParam.FC_ShadowColor.rgb` (byte 464).
Every GPARAM that carries `Directional Shadow Color` authors the bytes `000000ff` (s_template, remobase, m95/m96, mapoverride); `default.gparam` and the Ashina files omit it.
This changes only shadowed pixels, so it cannot change the brightness of an unshadowed lit surface.
The constructor default of the omitted field was not located.

### GI scale uploads

The Env Map group (`Env Map: ColorScale`) exists only in m95 files with values 1.0 (and 1.5, 0.7, 0.5 in a few rows); Ashina files omit it.
Per-region `Specular Color Scale` and `Diffuse Color Scale` are editor fields of the EnvMapRegion record.
The executable default of those fields was not located, so the unit scale stays.

### Shadow parameters

`default.gparam` carries `ShadowParam`: cascade distances 10 / 30 / 70, far fade start 1000 and distance 10, blur count 0.
Map files override two or three fields only, so these defaults apply to Ashina; the renderer falls back to 15 and 60 m instead.
Not changed here: the semantics of the distances (cut points or lengths) are unproven.

### Measurement that localises the excess: the LightSet row

At spot-002 the sun-only snow value is x = (0.71, 0.94, 1.27), 4.3 times the arithmetic prediction for LightSet 0.
It equals the prediction for the authored row: 133 of 166 Outskirts collisions carry LightSet 100, whose hour-12 sun is `(0.784, 0.882, 1.0)^2 * 1.0 = (0.61, 0.78, 1.0)` against `(0.139, 0.189, 0.26)` for row 0.
Our pipeline reproduces the native arithmetic for either row (flat test of the tonemap chain: a constant scene 0.2 returns x = 0.28 = 0.2 * 1.41).
Forcing row 0 (`SEKIRO_LIGHT_SET=0`, diagnostic, default unchanged) gives scene Y 1.06 to 1.54 times retail at 7 of 10 spots (before 2.2 to 5.5), and 2.33 times at spot-090.
So the excess is the authored hour-12 row-100 sun and fill, not a unit or a missing occlusion factor.
Why the retail frames do not show that sun is open: no engine element that attenuates it was found (cloud shadow groups are empty for Ashina; AO is at most 0.7).
Candidates not yet traced: how the engine layers row 100 over row 0 and over `default.gparam`, and the scene-event gparam overrides (`SceneGparam EventID`, `s11_00_*` files).

Other observations: at spot-091 a warm light on the roof survives `--no-local-lights` and all ablations and has no counterpart in the retail frame; retail snow coverage on roofs is thin (dark tile visible), which is a material issue separate from light.

## LightSet row 100 selection audit, 2026-10-15

Question: why does retail not show the authored row-100 sun at hour 12, when 133 of 166 Outskirts collisions select row 100.
No selection fix is made, because no engine rule that dims row 100 was proven.
What was recovered from the executable (read-only Ghidra decompiles, scratch outputs only):

- The key is a pair `(bank, row)`: `eb3e70` and `10a7f20` build it from the block bank at `+0x620` and the first collision gparam int.
  The block bank of a file is `((area + 100) * 100 + block) * 10000` (`f472a0`, `f47270`), the same number that the VolumetricFog `LightBankId` fields hold (111020000 is m11_02).
- Lookups (`28f4e0`, `293720`) hash bank then row, and fall back to `(1, 0)` when the row is absent.
  Rows of a group are enumerated from the value ids of the first parameter of the group (`17ad40`), so a row that the first parameter (`Angle0`) lacks is never registered; this is why Outskirts row 120 does not exist.
- Each block loads ten family files `m<area>_<block>_<d><sub>` for d = 0..9 (`ed5cd0`, number `(d - B/100) * 100 + B`), and has five layers.
  Layer 0 is the base file; layers 1 to 4 take the file index from the four signed bytes of the MSB collision `SceneGparam` struct (`1410a8110`, the debug string `SceneGparam EventID[%d][%d][%d][%d]`), with the struct's transition time as the fade.
  Our loader parses these bytes (`msb.rs`, `scene_gparam`) but nothing consumes them.
  At the ten reference spots the collision events are: spot-002 and 003 layer 2 = 5, spot-089, 090 layer 1 = 7, spot-092 layer 2 = 7, the others none.
  Overlays 0500/0700/0800 re-author the same row-100 sun (0700 lacks row 100 entirely), so they cannot explain the excess, and the no-overlay spots (009, 025, 008, 013, 091) match forced row 0 equally well.
- `mapbase.gparam` is the engine base layer for all map blocks ("MapBase"); it is not read by our loader.
  It authors `SsaoWeight0..3 = 1`, SSAO scale 1.35 offset -0.35, shadow cascades 10/50/200, blur count 2 and `LightMask Influence` 1, none of which the Ashina files override.
  Whether these change direct light by a large factor is untested; AO alone is bounded near 1 on open surfaces.
- Overlay files carry only some groups and rows, and row records are independent per `(bank, row)`; a missing field of row 100 is reset to record defaults (`228040` memcpy of defaults), not proven to inherit file row 0.

Not found: any condition (flag, weather, time, scene event) that makes the engine use row 0 or a dimmer row for collision LightSet 100.
Open candidates: the unread `mapbase.gparam` layer, the unrecovered record evaluation that merges layers, `LightMask Influence` with per-instance `ShLightMask`, and the metering/exposure gain at the pinned noon adaptation limits.

## Layer stack result, 2026-10-15

The exe layer stack (priorities, record defaults and the overlay selection) is recovered and implemented; see [LIGHTING.md](LIGHTING.md#layer-stack-and-record-defaults-2026-10-15) for the rules.
It answers the open candidates of the previous section, and it does not explain the excess.

- The effective row-100 sun at hour 12 is the block file's own value after layering: diffuse (0.784, 0.882, 1.0) squared times W 1, which is (0.615, 0.779, 1.0).
  Neither `mapbase` (priority 4) nor `default` can override it, and the overlays carry row 0 only.
  Row 0 at the same hour is (0.139, 0.189, 0.260), so row 100 is 4.4, 4.1 and 3.8 times row 0 in R, G and B.
- That ratio, 0.226 to 0.260 in the inverse, equals the 0.1 to 0.25 of native `Col * albedo * N.L` that the ablations found retail carries (see the L1 list in [VISUAL-PARITY.md](VISUAL-PARITY.md)).
  Retail's direct light behaves like the row-0 noon light on every collision, but no layer rule, default or field recovered here produces that.
- `LightMask Influence` is the fourth component of the AmbientReflection vector (record offset 0x240 in `310630`), not a direct-light factor.
  `Sharpness` only changes the specular roughness term in `native_direct`.
  `SsaoWeight0..3 = 1` gives `1 + 1 * (ssao - 1)` and the SSAO value is `1.35 * ao - 0.35`, so an unoccluded surface keeps factor 1.
  None of the three can scale the diffuse sun by 4.
- The `(bank, row)` key is confirmed: bank `((area + 100) * 100 + block) * 10000` from the MSB file name (`1410957c0`), row the first of the four collision gparam ints (`1410a7f20`), with the second int the fog key (`1410a7780`).
  No branch forces row 0.
- Remaining candidates: the A/B set blend factor in the scene context (`ctx + 0x111b8`, written outside the code read here), the time-of-day key actually used for these blocks, and whatever selects the group rows of tone map and post (not the light key).

Measured with `SEKIRO_ABLATE=nolayers` (before) against the default (after), the same executable, the ten matched cameras, 2560 by 1440:

| Spot / shot | Scene Y % real / before / after | Saturation % real / before / after | Hue degrees real / before / after | Snow Y % real / before / after |
| --- | --- | --- | --- | --- |
| spot-002 / 4 | 7.0 / 24.3 / 26.1 | 24.0 / 21.0 / 20.9 | 203 / 207 / 204 | 20.2 / 78.4 / 73.5 |
| spot-003 / 12 | 7.5 / 34.5 / 32.3 | 23.9 / 21.5 / 24.2 | 210 / 211 / 211 | 19.2 / 82.4 / 76.2 |
| spot-008 / 33 | 7.5 / 16.6 / 16.4 | 24.4 / 25.0 / 28.0 | 210 / 214 / 214 | 39.0 / 88.3 / 81.0 |
| spot-009 / 34 | 10.5 / 17.7 / 17.8 | 22.1 / 26.6 / 28.3 | 207 / 213 / 212 | 33.3 / 85.4 / 78.3 |
| spot-013 / 44 | 12.5 / 28.6 / 28.8 | 17.3 / 20.2 / 20.7 | 205 / 210 / 210 | 12.8 / 46.1 / 44.6 |
| spot-025 / 85 | 10.6 / 18.6 / 21.8 | 17.6 / 22.8 / 24.7 | 206 / 213 / 211 | 37.4 / 88.1 / 79.3 |
| spot-089 / 282 | 22.3 / 40.8 / 40.5 | 19.8 / 17.8 / 15.9 | 198 / 206 / 206 | 24.9 / 84.9 / 78.0 |
| spot-090 / 288 | 12.4 / 58.7 / 57.1 | 22.4 / 15.2 / 13.9 | 202 / 205 / 205 | 11.0 / 83.5 / 77.4 |
| spot-091 / 290 | 19.7 / 36.9 / 37.1 | 15.4 / 19.2 / 18.7 | 197 / 207 / 207 | 11.7 / 45.0 / 46.6 |
| spot-092 / 293 | 4.3 / 14.2 / 17.5 | 22.5 / 26.3 / 25.7 | 196 / 208 / 207 | 21.1 / 57.8 / 57.7 |

Scene Y changes by -2.2 to +3.3 points and snow-region Y falls by 4.9 to 8.8 points in seven views, by 1.5 at spot-013, rises 1.6 at spot-091 and is flat at spot-092.
Saturation and hue move by a few points either way.
Every scene stays 2 to 5 times retail luminance (spot-002 26.1 against 7.0, spot-090 57.1 against 12.4), so L1 item 1 remains open.
The default render changes only through proven data: the `mapbase` and overlay fields above.
Panels (real, before, after) are in the external `2026-10-15/lightset/` directory.

Other recoveries:

- `mapoverride_0000` to `_0003` parse as game id 3 (no value times) and hold no LightSet rows; their post and environment fields are not applied because the block field that selects `MapOverride_NNNN` is not found, and no event instruction writes it.
- The default record for a LightSet row is not the block's row 0: a field that no layer authors for row R takes the template value, so the earlier per-field fallback to row 0 was wrong for 17 of the 26 LightSet fields of row 100.

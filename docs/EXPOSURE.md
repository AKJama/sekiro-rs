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

The fixed `SCENE_TO_BEVY=4500` bridge remains inferred, not a recovered native unit conversion.
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

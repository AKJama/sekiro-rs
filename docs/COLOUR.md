# Post-chain colour audit, 2026-10-15

The isolated formula comparisons below remain historical night-state controls.
Their retail target is daytime overcast, so current retail colour and snow conclusions use the later [reference-state remeasurement](VISUAL-PARITY.md#reference-state-and-snow-remeasurement-2026-10-15).
The state correction does not change the post-chain implementation audited here.

The renderer omitted the native pre-gamma colour matrix and rendered an incomplete bloom pyramid.
The filmic-contrast reader also missed the actual `GS Presents - ToneMap-Contrast` GPARAM name.
These are shared post-chain corrections, with no replacement light colours, exposure offsets, snow tints or map-specific adjustments.
They do not establish retail colour parity.

## Native evidence and application order

This audit reads the installed GPARAMs, cached original LUT textures, the mapped executable and the locally disassembled ordinary SDR post shaders.
Research outputs stay in ignored `re/exports/colour/` and `re/research/colour-gparams.json`.
No decompiler listing or game asset is copied into tracked source.
The shared work board was read through the GitHub connector; the user's explicit no-publish instruction takes precedence over updating its remote claim.

The ordinary `ToneMap_TabledToneMapLUTPS` path combines exposed scene and bloom, evaluates the tone table, applies the colour matrix, clamps negative RGB, applies vignette, raises RGB to the output power, then samples the grading LUT.
The renderer retains analytical tone-table evaluation and an equivalent display-domain LUT pass before UI composition.
Vignette remains unimplemented; the audited Ashina maps do not author a nonzero power, and the default GPARAM power is zero.
HDR output is outside this ordinary SDR comparison.

### Tone correction

There is no separate `ToneCorrect` group in either audited Ashina GPARAM.
The equivalent native controls are `ColorH`, `ColorS`, `ColorV` and `GS Presents - Contrast` in `Tone Map[Yebis]`.
The default GPARAM supplies hue 0 degrees, saturation 1, value 1 and contrast 1; these fields are absent in both area files, so their native neutral defaults apply.
Their absence is not evidence for adding a warm tint or reducing saturation.

Registration `2a3160`, the `2565a0` copy and the `250130` render-packet copy connect these controls to the matrix producer `24dca0`.
The blended record's `0xe0..0xec` values become filter `0xa8..0xb4`, then packet `0x78..0x84`.
`158730` multiplies the component transforms in hue, saturation, value, contrast order.
Hue is a degree-valued rotation about normalized luminance weights `(0.29891, 0.58661, 0.11448)` after the native basis change, rather than an HSV hue offset.
For column-vector RGB, that basis is `Q = I - (1-w)*transpose(w)` and hue is `inverse(Q)*rotation(normalize(w),radians(h))*Q`.
Saturation mixes RGB toward its weighted luminance, value multiplies RGB, and contrast scales around 0.5.
The resulting matrix is uploaded with the post settings and consumed between the tone function and output gamma.
The regression exercises authored controls together, distinguishes tone-curve contrast from colour contrast, and checks that hue preserves grey.

The native filmic field is named `GS Presents - ToneMap-Contrast` in the default GPARAM, although the executable's filter/editor also calls it `FilmicContrast`.
The reader now accepts the authored name and retains the existing alternate name.
Both audited maps retain the default mode 7 and filmic contrast 0.5.

### LUT and gamma

Both areas select enabled LUT 20 at the night hour 22.
The strips are loaded as unorm bytes, without automatic sRGB decoding.
`ToneMap_Copy2Dto3DLUTCS` proves the layout `(r,16*b+g)`.
The ordinary table-plus-LUT shader proves the normalized address `(15/16)*rgb + 1/32` and linear clamped 3D sampling.
The existing eight-cell interpolation over `15*clamp(rgb,0,1)` is equivalent, including edge clamping, so it is retained.
The output exponent is applied before the LUT, and the storage sRGB inverse/forward transfers cancel around that display-domain operation.
The native filter constructor supplies gamma 2.2; no authored replacement gamma or chromatic white-balance stage was found for mode 7.
The executable also contains debug-menu labels `BrightnessGammaScale` and `BrightnessOffsetForHDR`, but this audit does not resolve their producers or establish a non-default SDR brightness value in the retail references.
Those labels alone do not establish an omitted transform or justify fitting another gamma.
`ReinhardWhiteBalance` is the scalar multiplying the input of the mode-8 table writer and is already read and applied there.
It is not a colour-temperature control for the active sensitometric mode.

`2ab260` proves that native grading can retain two LUT resources and a blend factor rather than choosing the nearest resource.
This audit does not complete the upstream time/resource-resolution policy, so intermediate-hour LUT blending remains an explicit limitation.
At the exact authored night key used in all six captures, both paths select LUT 20 and this limitation does not justify changing its sampling or colour.

### Exposure and metering

The existing weighted biased-log meter, log2 MiddleGray target, base-ten adaptation response, one-frame gain latency and natural-exponential soft EV limits agree with the inspected shader/CPU formulas.
The active night upload reads auto exposure enabled, MiddleGray -4.9, limits -0.5/+0.5, centre weight 4, light speed 1 and dark speed 0.5.
Outskirts reads exposure -0.2 stops and Castle +0.3 stops.
Freeze remains false, and the authored values reach the GPU uniform rather than a Bevy auto-exposure approximation.
No exposure adjustment is inferred from the six reference luminance errors.
Exact native reduction layout, reset timing and renderer-delta units remain limitations from [EXPOSURE.md](EXPOSURE.md).

### Bloom

`253560` extracts bright colour once, after exposure and adaptation.
`2536c0` builds the authored number of levels by bilinear downsampling the preceding blurred level, blurs horizontally and vertically, and accumulates from the smallest level upward.
`ToneMap_DownSample2x2PS` contains one bilinear sample.
`ToneMap_BloomCompositeUpScalePS` supplies the authored Smooth factor in that upward accumulation.
`2508b0` normalizes the final bloom gain by `sum(Smooth^i)` over the authored level count.
`24eb90` allocates up to eight half-size-and-smaller levels in its ordinary branch, rounded upward for odd dimensions.
Its optional quarter-resolution branch is not newly selected by this implementation; the global quality policy remains unresolved.
The blur kernel producer `2556e0` normalizes seven Gaussian samples with sigma 1.5; GaussianRadiusScale controls sample spacing.

Previously the renderer normalized for seven levels but independently extracted and blurred only three levels, combining them with fixed 1/0.5/0.25 weights.
That combination belongs to a different native composite shader and did not match the recovered full-pyramid schedule.
The renderer now builds the full pyramid and recursively combines each lower level using Smooth before applying the existing normalized gain.
At night, both areas author seven levels and Smooth 0.8, with saturation interpolation 0 and radius scale 1.
Outskirts authors threshold 0 and Alpha 0.5; Castle authors threshold 0.05 and Alpha 0.3.
The native global bloom-quality scalar is still unresolved, and temporal feedback/history scheduling is not implemented.
Neither unknown is replaced with a fitted brightness or desaturation factor.

## Measurement and controls

The six saved poses, 2560x1440 windowless night captures, exact inverse-sRGB Rec.709 luminance, display-HSV saturation and saturation-weighted circular hue follow [EXPOSURE.md](EXPOSURE.md#measurement-method).
The same scene masks are reused, and the four available ground masks are retained.
Whole-scene hue does not identify a snow-only material region.
Initial HEAD captures are preserved separately as `initial-head-reproduction` in the external collection.
The final before/after comparison uses a frozen source copy because another task is editing fog surfaces in the shared workspace.
Only this task's post source and test files differ between those two builds; protected sky/fog sources remain identical.
Game data and research artifacts remain local and untracked.

## Measured results

| Spot | Setup | Linear Y % | Saturation % | Hue degrees | B/R |
| --- | --- | ---: | ---: | ---: | ---: |
| spot-002 | real | 7.02082 | 23.961 | 202.908 | 1.31164 |
| spot-002 | before | 13.62286 | 35.429 | 201.229 | 1.49684 |
| spot-002 | after | 16.77243 | 33.749 | 202.538 | 1.46621 |
| spot-003 | real | 7.50628 | 23.897 | 209.900 | 1.30674 |
| spot-003 | before | 15.75755 | 39.708 | 204.094 | 1.49832 |
| spot-003 | after | 19.42810 | 36.578 | 205.365 | 1.44517 |
| spot-009 | real | 10.47270 | 22.067 | 206.722 | 1.26647 |
| spot-009 | before | 7.24153 | 44.064 | 205.092 | 1.70243 |
| spot-009 | after | 8.69319 | 43.086 | 207.010 | 1.68276 |
| spot-025 | real | 10.60234 | 17.631 | 206.315 | 1.15327 |
| spot-025 | before | 9.15159 | 36.736 | 202.244 | 1.46746 |
| spot-025 | after | 11.07755 | 34.969 | 204.023 | 1.44736 |
| spot-091 | real | 19.73271 | 15.433 | 197.001 | 1.18173 |
| spot-091 | before | 16.43685 | 37.691 | 205.627 | 1.50912 |
| spot-091 | after | 18.31745 | 37.147 | 206.340 | 1.49270 |
| spot-092 | real | 4.34681 | 22.497 | 196.323 | 1.28661 |
| spot-092 | before | 4.70135 | 38.550 | 201.818 | 1.61182 |
| spot-092 | after | 5.11088 | 38.675 | 202.822 | 1.61213 |

Saturation and B/R move closer to retail in five views and slightly farther away in spot-092.
Luminance moves closer in spots 009, 025 and 091, and farther away in 002, 003 and 092.
Hue moves closer in 002, 003, 009 and 025, and farther away in 091 and 092.
Restoring the omitted bloom contribution raises scene brightness; it is not a global-darkening fix.
The neutral matrix defaults and already-correct exact-night LUT sampling do not supply an authored warmer or less saturated correction for these scenes.
Snow remains too blue and L1/M1 remain open.
No replacement scene-unit bridge, material/BRDF, SSAO, gamma or bloom-quality value is inferred from these residual image errors.

## Source and data verification

The controlled before and after executable SHA-256 prefixes are `b8177daa6959984f` and `396ca67b3c36afb3`.
All six complete sorted visibility records, requested poses, FOVs and selected collisions agree.
SHA-256, size and modification-time fingerprints agree for all 17,217 map/draw-param asset files, totalling 17,733,480,816 bytes.
The source manifests differ only in `lighting.rs`, `tonemap.rs`, `tonemap.wgsl`, `exposure.wgsl` and the native-post integration test.
The external `2026-10-15/colour/` collection retains the six full-resolution real/before/after panels, individual captures, previews, scene/ground masks, poses, source/binary manifests, visibility records, measurements and verification records.
Initial HEAD reproductions and a preliminary capture before the last-level dimension correction are preserved in separate subdirectories and excluded from the reported comparison.

## Final validation and changed files

The audited release workspace run passes 816 tests with 25 existing ignores, including the installed-data gate, authored-colour regression, native shader permutations and GPU sensitometric samples.
Release workspace all-target Clippy with warnings denied and workspace formatting pass.
All Cargo commands use `CARGO_TARGET_DIR=target/alt-exposure`.
The final gates use the current shared-tree build, while the six-view measurements isolate the post changes in the frozen copy.

| Map | Final seconds | Void | Depth gap | Untextured | Grey | Pop-in |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| m11_00_00_00 | 54.114 | 0 | 493 | 0 | 3000 | 28453 |
| m11_01_00_00 | 58.818 | 1933 | 4885 | 0 | 2111 | 34243 |

All twelve final geometry triples equal the fresh before controls.
Only colour fixture cells change, with per-view review notes in [MAP-COMPLETENESS-SWEEP.md](MAP-COMPLETENESS-SWEEP.md#post-chain-colour-fixture-review-2026-10-15).
The final Outskirts gate is under `target/alt-exposure/colour-gates-verified/`, and the final Castle repeat is under `target/alt-exposure/colour-gates-verified-repeat/`.
The original 16-pixel/1-percent tolerance, all camera/geometry fields and the 120-second budget remain unchanged.
Intermediate before controls exceeded the time budget while workspace tests competed for resources; those runs are not reported as passing gates.
An intermediate shared-tree validation executable retained the frozen copy's fixture root through Cargo artifact reuse.
Clearing only the game package's release artifacts and rebuilding from the workspace root corrected that provenance issue before the final passing runs.
The verified shared-tree executable SHA-256 is `e9698a59f79b4f2029f8a2f32966570321bd6ca313679d5a88d589ea44b7f93a`.
A late concurrent diagnostic edit to `crates/game/src/fxr.rs` is outside this task and the audited build.
The five post source/test files still match the measured snapshot exactly; this task does not alter that FXR edit or the protected sky/fog sources.

Changed implementation files are `crates/game/src/lighting.rs`, `tonemap.rs`, `tonemap.wgsl` and `exposure.wgsl`, plus `crates/game/tests/it/native_post_shaders.rs`.
The two Ashina `.render.json` fixtures change only reviewed colour fields.
Documentation changes are this report, `EXPOSURE.md`, `LIGHTING.md`, `VISUAL-PARITY.md`, `SHADER-INPUTS.md`, `SHADERS.md`, `FIDELITY.md`, `MAP-COMPLETENESS-SWEEP.md` and `STATUS.md`.
Nothing was staged, committed or published, no worktree was created, and no retail game or save-writing session was started.
The complete image collection is `D:/sekiro-rs-media/progress/2026-10-15/colour/`.

- `spot-002-shot-004-real-before-after.png`

- `spot-003-shot-012-real-before-after.png`

- `spot-009-shot-034-real-before-after.png`

- `spot-025-shot-085-real-before-after.png`

- `spot-091-shot-290-real-before-after.png`

- `spot-092-shot-293-real-before-after.png`

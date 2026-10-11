# Shader inputs: GPARAM to constant buffers

How the game executable turns GPARAM draw-param values into the numbers the shaders read.
This closes most of the "CPU transforms and remaining questions" section of `docs/SHADERS.md`.
Addresses are image-relative offsets in the game executable (add 0x140000000 for the default image base).
Function names are our own labels, derived from vtable class names and string references.
No decompiled code is pasted here; every formula is our own description.

Confidence values: **verified** means read directly from the executable's arithmetic and cross-checked against shader reflection offsets.
**Inferred** means a standard library routine or semantics were identified from structure, not from symbols.

## Method

The functions were read with static analysis tools; only the resulting formulas are recorded.
The key step was the per-frame scene update at `140fa0`: for each parameter family it fetches two parameter records (A and B), blends them with one factor, and hands the result to a renderer object.
GPARAM field names lead to three kinds of functions.
The registration functions (`29c180` fog, `2a3160`/`2566b0` tone map, `2ad1d0`/`228910`/`257ff0` bloom) only bind names to struct offsets.
The copy functions (`3a15b0` fog, `2565a0` tone map, the colour/bloom block in `140fa0`) move blended values into renderer objects.
The per-frame render functions (`39ffe0`, `39f000`, `39fa70`, `39e7e0` fog; `250060`, `250130`, `2508b0` tone map and bloom) contain the actual transforms.
Library math identified by use: `2386e30` is pow, `23889f0` is ln, `239cda0` is exp (inferred).
`DAT_14328919c` is 1.0, `DAT_1432891f4` is 2.0, `DAT_1432891c0` is 1/ln2, `DAT_143289124` is 0.5 (all read from the image).

## Summary of answers

| Item | Status | One-line answer |
|---|---|---|
| Fog GameScale | verified | Authored lengths are multiplied by GameScale, authored per-length densities are divided by it (table below). |
| HeightFogOffset packing | verified | Height is camera-world Y axis dotted into view space minus the raw offset in metres; the offset is not scaled by GameScale. |
| NearDepth / FarDepth distribution | verified | Geometric slices between `GS*Near` and `GS*max(Far, Near+0.1)` with a depth offset `GS*NearOffset`. |
| Adaption Min/Max | verified | Stops (EV): a soft saturating limit on the adaptation offset, Max positive and Min negative. |
| Exposure / MiddleGray | verified | `Exposure` is a stop offset (gain `2^Exposure`); `MiddleGray` is a log2 target luminance. |
| Light colour and W | verified | Uploaded colour is `rgb*rgb*W` (squared, then times intensity W), alpha is `W*W`. |
| Ambient (Hemi up/down) | verified | Two-colour hemisphere packed as three float4 `(axis*(up-down)/2, (up+down)/2)`. |
| SC_PreExposure producer | verified | `2^EV` with EV stored in the scene context; EV is 0 unless an auto-adjust debug flag is set. |
| Bloom threshold scaling | verified | Raw `Glare Threshold` against post-exposure luminance, no extra scale. |
| Local fog light scale | partial | Fog local light colour is `LocalLightScale * light.rgb`; the light's own unit is open. |

## Fog

### GPARAM field to renderer offsets

The blended fog record (what `3a15b0` copies) has the file offset minus 0x10 for each field.
The renderer object then holds each value at `+0x1a4 + blended offset` (Enable alone is at `+0x1a1`).
Relevant fields (blended record offsets): Enable 0x00, VolumeWidth 0x04, VolumeDepth 0x08, NearDepth 0x0c, FarDepth 0x10, NearOffset 0x14, GameScale 0x18, DrawTiming 0x1c, EnableAnalyticalFog 0x20, VolumeSliceSampleOffset 0x24, CullingBySceneDepth 0x28, CancelInScatteringRateFromSSR 0x2c, FixedDensity 0x30, ScatteringG 0x34, LocalLightScale 0x38, LocalLightDistScale 0x3c, HeightFogDistribution 0x40, HeightFogOffset 0x44, HeightFogDensity Scale 0x48, AbsorptionScale 0x50, SkyColor 0x60, SkyLight HeightAtten 0x70, SkyLight StartDist 0x74, DirectColor 0xa0, OppositeColor 0xb0, DirLight HeightAtten 0xc0, DirLight StartDist 0xc4.
NearOffset exists in GPARAM but is missing from the older tables in `docs/SHADERS.md`.

### GameScale (function `39e7e0`, called from `39f000`)

`GS` is the GameScale field (castle 2.5).
Before anything is uploaded, the per-frame fog snapshot is rescaled once:

```text
Near'   = GS * Near
Far'    = GS * max(Far, Near + 0.1)        // compared with unscaled Near
Offset' = GS * NearOffset
FixedDensity'          = FixedDensity / GS
HeightFogDistribution' = HeightFogDistribution / GS
HeightFogDensityScale' = HeightFogDensityScale / GS
SkyLight StartDist'    = GS * StartDist
DirLight StartDist'    = GS * StartDist
```

The authored values are therefore in units of GS metres: Near 0.01 and Far 400 become 0.025 m and 1000 m at GS 2.5.
HeightFogOffset, AbsorptionScale, ScatteringG, colours and heights of lights are not scaled.
LocalLightDistScale is multiplied by a renderer-global quality float (`*(*(DAT_143f3b490+0x12620)+0xa4)`, value not recovered).
Confidence: verified.
Our renderer: apply exactly this table once, then treat every fog shader quantity as metres.

### Froxel depth distribution (functions `39fa70`, `39f000`)

With `n=Near'`, `f=Far'`, `o=Offset'` and `Z=VolumeDepth` (clamped 4 to 128):

```text
DepthRangeInfo     = (n, f, f-n, o)
LogDepthRangeInfo  = (log2(n+o), o, log2(f+o)-log2(n+o), 1/that)
PreComputed        = (2^(logRange/Z) - 1, (VolumeSliceSampleOffset - 0.5)/Z, Z/(Z+2), 0)
slice boundary k   = (n+o) * ((f+o)/(n+o))^(k/Z) - o
ds (shader)        = zSlice * PreComputed.x      // thickness of one slice in metres
```

`VolumeSize` is `(X, Y, Z)` and `InvVolumeSize` its reciprocal, taken from the fog object at `+0xac/+0xb0/+0xb4`.
`3a2be0` sizes them: Y is about `VolumeWidth` (authored 80 to 320, step 16, so the field is really the vertical count), X is `Y*aspect`, both capped at a quarter of the viewport and at 640, rounded up to a multiple of 8; Z is clamped to 4..128.
The XY sizing is inferred; the depth maths is verified.
Our renderer: use these exact slice boundaries; the `Z/(Z+2)` factor means the volume texture has two padding slices.

### FC_VolumeFogParam (forward shaders, function `39f000`, stored at fog object `+0x268`)

Four float4 rows, then copied by `3a14d0` into the per-frame scene packet (`1424e0`/`15cfe0`, context `+0x410`):

```text
P0 = (o, Z/(Z+2)/logRange, Z/(Z+2)*((sampleOffset-0.5)/Z - log2(n+o)/logRange), f)
P1 = (1+Absorption.r, 1+Absorption.g, 1+Absorption.b, EnableAnalyticalFog ? f : large)
P2 = (0, 1, 0, camY - HeightFogOffset)            // sign flipped, y=-1, if HeightFogDistribution < 0
P3 = (kH, sH, FixedDensity', 0)
```

`P1.w` equals the capped volume depth only when EnableAnalyticalFog is on, which is what gates the far continuation in the FLVER shader.
Confidence: verified.

### HeightFogOffset and the height vector (`39fa70`)

`g_HeightFactor.xyz` is the world-Y component of the camera's three view axes, from the camera-to-world matrix at context `+0xd0`.
`g_HeightFactor.w` is camera world Y minus the unscaled HeightFogOffset.
So `h = worldY - HeightFogOffset` in metres, then `q = kH*h`.
A negative HeightFogDistribution negates the whole vector (fog thickens the other way) while `kH` uses the absolute value.

```text
kH = max(|HeightFogDistribution'|, 1.18e-38)      // g_HeightFogParam.x
sH = 0.5 * HeightFogDensityScale'                 // g_HeightFogParam.y
g_HeightFogParam.z = SkyLight StartDist'
Global.x = FixedDensity', Global.y = ScatteringG
```

The 0.5 on `sH` pairs with the profile that approaches 2 below the reference plane.
Confidence: verified.
Our renderer: feed `h` from world Y in metres; do not scale HeightFogOffset by GameScale.

### Fog colours and extinction

SkyColor, DirectColor and OppositeColor are each clamped to 0..1, quantised to 8-bit RGB when copied (`3a15b0`, function `152340`), with W kept as a separate float.
The shader value (`39e3d0`) is `pow(byte/255, 2.2) * W` per channel, so authored RGB is gamma 2.2 and W is a linear intensity.
`ExtinctionScale = (1,1,1) + AbsorptionScale`.
`PreExposure` (cb offset 572) is the scene context value `+0x45c`, the same SC_PreExposure described below.
Confidence: verified (the exponent 2.2 is read from data; pow identity inferred).

### DirLightDirection and DirLightAngleInvCos (`39ffe0`)

Direction is the light vector fetched for (LightBankId, LightId), rotated into view space by the view matrix.
`DirLightAngleInvCos = 1 / max(+-L.y, AngleClip)`, where `L.y` is the world Y of that vector and the sign flips with a negative HeightFogDistribution.
This is the slant factor used in the vertical column attenuation.
Confidence: verified for the formula; the bank/id resolver (`140169080`) is not decoded.

### Fog local lights (`3a20e0`)

Point (type 0x65) and spot (type 0x67) lights are uploaded with `Color.rgb = LocalLightScale * light.rgb`, `Color.w` copied unchanged, and spot inverse radius squared `1/r^2`.
A per-light scattering G below zero falls back to the global ScatteringG.
Local lights render only if both LocalLightScale and LocalLightDistScale are above zero (`39f700`), and LocalLightDistScale scales the camera-centred collection radius.
Open: the unit of `light.rgb` for map lights (BTL records) and the exact radius constant.

## Tone map and exposure

### Where the values live

Blended tone-map record offsets (`2565a0` copies them to the filter object): Exposure 0x8c, ToneMapMode 0x94, FilmicContrast 0x98, ReinhardContrast 0x9c, ReinhardWhiteBalance 0xa0, Enable AutoExposure 0xa4, LightAdapt 0xa8, DarkAdapt 0xac, MiddleGray 0xb0, CenterWeight 0xb4, Adaption Max 0xb8, Adaption Min 0xbc, FreezeAdaptionFade 0xc0.
Filter object offsets: Exposure `+0x30`, mode `+0x50`, FilmicContrast `+0x54`, ReinhardContrast `+0x58`, WhiteBalance `+0x5c`, Gamma `+0x60` (default 2.2, not GPARAM), HDR paper white `+0x64` (default 250), EnableAutoExposure `+0x68`, MiddleGray `+0x6c`, LightAdapt `+0x70`, DarkAdapt `+0x74`, CenterWeight `+0x78`, Max `+0x7c`, Min `+0x80`, Freeze `+0x84`.
Constructor defaults (`24db30`): mode 7, FilmicContrast 0.5, ReinhardContrast 1.6, WhiteBalance 1.7, MiddleGray -2, LightAdapt 1, DarkAdapt 0.5, CenterWeight 4, Max 8, Min -8, `UseTable` on, `AfterGamma` on, AvgLuminance 1.

### Exposure gain (functions `250060`, `2508b0`)

```text
gain = 2^Exposure / AvgLuminance * 2^(-softclamp(adaptEV)) * 2^(-PreExposureEV)
g_ToneMapSceneLumScale    = gain        (all four components)
g_ToneMapInvSceneLumScale = 1 / gain
```

`Exposure` is in stops.
`AvgLuminance` is a debug field (default 1).
`adaptEV` is the GPU adapted value minus 20 read back one frame late (`2560e0`), i.e. `log2(sceneLuma) - MiddleGray` in smoothed form.
`PreExposureEV` is 0 unless the pre-exposure debug flag is on (next section).
The CPU-side adaptation factor is applied only while the global `DAT_143c2ba3e` is set, and the image ships with it set; with it clear the factor is 1.
Confidence: verified for the formula, inferred for which vertex-shader variant consumes the GPU gain instead.

### Adaption Min/Max (function `24d920`)

Units are stops (EV).
With `x = adaptEV` and Max, Min the GPARAM fields (Max above zero, Min below):

```text
x > 0:  y = Max * (1 - exp(-x/Max))
x < 0:  y = |Min| * (exp(x/|Min|) - 1)
```

This is a soft limit approaching `+Max` and `-|Min|`, not a hard clamp.
`FreezeAdaptionFade` set forces `y = 0`; when auto exposure is disabled the offset is reset to 0 while a reset counter is active.
Confidence: verified.
Our renderer: replace the capped hard limits with this curve.

### MiddleGray and speeds (function `2508b0`)

```text
g_AdaptParam = (MiddleGray, dt*LightAdapt, dt*DarkAdapt, 0)
```

The shader (documented in `docs/SHADERS.md`) then computes `target = log2(avgLuma) - MiddleGray` and moves toward it at rate `1 - 10^(-dt*speed)`.
Hence MiddleGray is a log2 luminance target: auto exposure aims the log-mean luminance at `2^MiddleGray` before the `2^Exposure` offset.
`dt` is a renderer-global frame delta (`*(DAT_143f3b490+0x120b4)`, unit presumed seconds, not verified); on the first frame after a reset a large constant (4.0) replaces `dt*speed`, which makes adaptation instant.
`g_AdaptCenterWeight = (CenterWeight, CenterWeight*viewH/viewW, 0, 0)`.
Confidence: verified except the unit of `dt`.

### Operator selection and curve parameters

Mode codes: Filmic 0, Sensitometric 7, ReinhardEx 8, Reinhard 3, RGBM 4, Copy 5, Gamma 6.
Modes 0, 7, 8, 3 write a lookup table (`252430`) and use the tabled pixel shader; the default mode 7 is Sensitometric (inferred as the shipped value, since the constructor default is 7).
```text
g_ReinhardParam = (ReinhardContrast, ReinhardWhiteBalance, 0, 0)
g_ToneMapParam  = (FilmicContrast, AfterGamma ? 1 : Gamma, AfterGamma ? 1/Gamma : 1, ...)
```

The Sensitometric table writer (disassembly `ToneMap_WriteSensitometricToneMappingTablePS`) is, with `x = 0.2u/(1-u)`, `A = 1 - e^(-1.442695x)`, `B = 1 - e^(-4.456669x)` and `c = FilmicContrast`:

```text
out = (A + c*(B^3 - A))^(1/g_ToneMapParam.y)
```

With the default `AfterGamma`, `y = 1` and the 1/2.2 power is applied at the end (`z`), which matches the ordinary branch in `docs/SHADERS.md`.
`HDR paper white` feeds `g_vHDRDisplayParam = (nits/10000, nits/200)`.
Confidence: verified.

## Bloom and glare (function `2508b0`, filter object offsets `+0x85..+0xa4`)

Filter fields: Enable `+0x85`, NumLevels `+0x88`, Luminance `+0x8c`, Threshold `+0x90`, Alpha `+0x94`, Saturate Ctrl `+0x98`, GaussianRadiusScale `+0x9c`, Smooth `+0xa0`, FeedbackWeight `+0xa4`.

```text
g_BrightPassThreshold = (Threshold, Threshold, Threshold, Threshold)     // raw, exposure-scaled luminance
g_BloomBoostColor     = (tint.rgb (default 1), SaturateCtrl)
g_vBloomFinalColor    = G * Alpha * Luminance / sum_{i<NumLevels} Smooth^i   // rgb
g_vBloomScaleParam    = (Smooth, FeedbackWeight, history scale, 0)
g_GlareLuminance      = (1, 0, 0, 0)       // constant
```

`G` is a global bloom scale (default 1) and `Luminance` is first multiplied by a renderer-wide quality float (`250130`); bloom is disabled when `G*Alpha <= 0` or that quality value is not positive.
There is no extra threshold scaling: the shader compares the threshold with luminance of `scene*SceneLumScale`.
`GaussianRadiusScale` multiplies the UV offsets of seven fixed blur taps in `1402556e0`; weights are a normalised Gaussian over those taps.
Confidence: verified.
Our renderer: bloom intensity is `Alpha*Luminance` normalised by the geometric level sum, not `alpha*0.08`.

## Lights and ambient (function `310630`, the LightSet blend)

LightSet file offsets: light `i` (DirLight 0, DirLight 1, LocalLight 1, LocalLight 2) starts at `0x40 + 0x60*i` with derived direction at `+0x00`, DiffColor `+0x10`, SpecColor `+0x20`, Angle `+0x30`, Sharpness `+0x40`, SsaoWeight `+0x50`.
Hemi Color Up is `+0x1e0`, Hemi Color Down `+0x1f0`.
Each colour is a float4 (r, g, b, W); components are interpolated between sets A and B in authored space first.

```text
Col_i = (r*r*W, g*g*W, b*b*W, W*W)     // cbLight Col_i, same for Spc_i
Vec_i = (lerp(direction_i), Sharpness_i)
FC_SsaoWeight = (Ssao_0, Ssao_1, Ssao_2, Ssao_3)
```

So colour space is gamma 2.0 (a square), W is a plain linear multiplier and no photometric unit is applied anywhere.
Light 1 can be rotated by the camera matrix when `Light1 FollowedCamera` is set (`+0x248`); LocalLight enable flags are `+0x249` and `+0x24a`.
Ambient: with `U = rgb*rgb*W` for Hemi Up and `D` for Hemi Down, `AmbientSH[c] = (a * (U_c - D_c)/2, (U_c + D_c)/2)` for `c = r,g,b`, where `a` is a hemisphere axis from the set (`+0x220..+0x228`, default up).
Irradiance at normal `N` is therefore `mean + dot(a,N)*(up-down)/2`.
`AmbientReflection` is another squared colour (`+0x230..+0x23c`) times its own W.
Confidence: verified for the arithmetic; the angle-to-vector construction (`30e520`, `30e6c0`) was not transcribed.
Our renderer: convert `rgb*rgb*W` to linear scene radiance, and fix a single scene-to-Bevy scale instead of separate 9000 lux and 900 candela guesses.

## SC_PreExposure producer (functions `1424e0`, `15cfe0`, `140fa0`)

The scene packet carries, at context offsets `+0x450..+0x464`: the exposure gain from `250060`, its log2, `2^EV`, `2^-EV` and `EV`.
SC_PreExposure is `2^EV` at context `+0x45c` (cbScene offset 1000 with the cb image based at context `+0x74`), and the inverse sits right after it.
`EV` is the scene context float at `+0x112a4`, written in `140fa0`.
If the byte at `+0x1129f` ("AutoAdjustPreExposure") is set, `EV = log2(exposure gain)`; a second debug byte (`+0x112a0`, "PreExposureRandomTest") makes it random; otherwise it stays 0.
Both bytes are cleared at scene creation, so shipped behaviour is `SC_PreExposure = SC_InvPreExposure = 1` and the tone map divides out `2^EV = 1`.
Confidence: verified for the code path; the shipped flag state assumes no menu sets it.
Our renderer: keep PreExposure at 1, and do not model it unless the formats need precision work.

## Still open

- Unit of map point-light and spot-light colours (BTL records) before the fog `LocalLightScale` and the lighting passes see them.
- The (LightBankId, LightId) resolver (`140169080`) and the time-blend factor between parameter sets A and B.
- Whether the shipped build runs the CPU adaptation path or the GPU-gain vertex shader (the global is set in the image, so CPU is assumed).
- Unit of the frame delta feeding `g_AdaptParam`, and the quality scalars from `DAT_143f3b490+0x12620/+0x12628` that scale LocalLightDistScale and bloom.
- Colour grading LUT id resolution (`14024d9f0`, `1402ab260`), and the colour matrix and vignette producers (`24dca0`).
- Camera-relative pieces: the exact view-space construction of the local-light collection volume constants (data at `328b000`).

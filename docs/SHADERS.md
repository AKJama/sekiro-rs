# Compiled rendering shaders

Research dated 11 October 2026, using the player's locally extracted Sekiro shaders.
No game code was changed and no game was launched.
All equations below are original mathematical descriptions of the compiled operations, rather than copied instruction listings.
The inventory, disassembly, reflection containers and scripts live in the ignored `target/alt/shaders/` directory.
This document is the only tracked deliverable for this session, so `STATUS.md` was left unchanged under the task's narrower write restriction.

## Evidence and coverage

The recursive inventory contains 22,749 files, including 22,243 stage-specific shaders: 13,306 pixel, 8,648 vertex, 100 compute, 28 geometry, 73 hull and 88 domain.
Every stage-specific shader begins with `DXBC` at offset zero and successfully disassembles through Windows `d3dcompiler_47.dll`.
The other files include packed bundles, shader bundles and material metadata; finding embedded DXBC in those containers does not make them additional unique shaders.
The disassembler produced 22,501 outputs including embedded first shaders from containers.
Repeated permutations and copies are counted as files, not as distinct algorithms.
The 22,243 stage-specific files contain 9,399 byte-distinct blobs by SHA-256.
See `target/alt/shaders/inventory.csv` and `statistics.json` for the detailed census.

The main evidence is in `gxrenderershader.shaderbnd.d`, `gxposteffect.shaderbnd.d`, `gxflvershader.shaderbnd.d` and `gxshader.shaderbnd.d`.
Reflection includes variable names, types, byte offsets, array lengths, resource names and bindings; declared but unused fields must not be mistaken for shader inputs.
The companion `.reflection.txt` files retain those definitions separately from instructions, `reflection-index.json` indexes all 22,501 outputs, and `rdef/` contains 955 reflection DXBC containers obtained through `D3DGetBlobPart` for matching families.

**Recovered** means the arithmetic or binding is visible in the inspected shader.
**Candidate mapping** means a GPARAM field matches a shader input by name and function but its CPU upload has not been traced.
**Open** means the supplied evidence does not establish the operation or mapping.
No shader family is fully recovered from raw GPARAM to displayed pixels: CPU packing, mode selection and render-state scheduling remain partly unknown.
The core fog density/march, post operators, bloom extraction/filtering, LUT addressing and representative deferred BRDF are recovered at the shader-input level.

Read alongside `docs/VISUALS-PLAN.md`, especially its step status notes, rather than treating its historical section 1 renderer inventory as current code.
The UTF-16 `target/alt/visuals/gparam-dump.txt` provides authored values and field paths.
The earlier `gparam-research.md` provides hypotheses about meanings, not formula evidence.
FwLo's locally supplied MIT visuals notes informed the disassembly approach; no implementation text was copied.

## Pipeline relationships

The shader resources establish these dependencies:

1. Material passes produce G-buffer normal, material and light-set data.
2. Deferred directional/local lighting and environment/GI passes populate lighting accumulators with scene pre-exposure.
3. `GXLightComposite.ppo` combines irradiance, specular and emissive buffers.
4. Fog density and source writes feed a logarithmic-depth integrated volume consumed by scene fog and forward FLVER shading.
5. Luminance reduction feeds adaptation; the adapted gain feeds bright-pass and tone-map vertex/pixel pairs.
6. Bloom extraction feeds blur and level composition; table-driven tone mapping adds glare before the tone-table lookup.
7. The table-plus-LUT ordinary branch applies its output exponent before the grading LUT.

These are data dependencies and within-shader order, not a recovered complete frame graph.
The standalone `ColorGrading.ppo` could be scheduled separately, and the available binaries do not establish which tone operator is active for each map or HDR mode.
Blend states are CPU pipeline state and are not stored in the pixel arithmetic.

## Fog

### Passes and volume layout

The renderer bundle contains `GXVolumetricFog_ClearDensityAndLightBuffer.cpo`, `GenerateSceneMaxDepth.cpo` with H/V variants, `WriteLocalDensityBox.cpo`, `WriteLocalLightPoint.cpo`, `WriteLocalLightSpot.cpo`, `WriteInScattering.cpo` with shadowed/pre-shadowed variants, `DirLightShadow.cpo` with reprojection variants, `SolveVolumetricFog.cpo`, `SceneComposite.ppo` with split variants, and `CancelInScattering.ppo`.
Clearing, density/source accumulation, solving and application form the volume dependency chain; the exact scheduling between local and global writes is not recovered.
Density and source are stored as packed half-float pairs in integer 3D UAVs, then unpacked for integration.
`rwLocalDensityAcc` at u1 carries density in its low half and blue source in its high half, while `rwLocalLightRGAcc` at u2 carries green in its low half and red in its high half.

For voxel indices `(i,j,k)`, with `N=VolumeSize.z`, the center reconstruction is:

```text
zCenter = exp2(LogDepthRangeInfo.x + ((k + 0.5)/N)*LogDepthRangeInfo.z)
          - DepthRangeInfo.w
nx = 2*(i+0.5)*InvVolumeSize.x - 1
ny = 1 - 2*(j+0.5)*InvVolumeSize.y
p = (zCenter*nx*ViewTangent.x, zCenter*ny*ViewTangent.y, zCenter, 1)
h = dot(p, g_HeightFactor)
```

Thus height is an affine transform of reconstructed view position, rather than the camera height alone.
The shader does not reveal how the CPU combines camera orientation, `HeightFogOffset` and `GameScale` into these constants.

### Density, angular scattering and start distances

Let `kH=g_HeightFogParam.x`, `sH=g_HeightFogParam.y`, `rhoFixed=Global.x` and `rhoLocal` be the unpacked density accumulator.
The global write constructs:

```text
q = kH*h
f(q) = exp(-q)       if q >= 0
       2 - exp(q)    if q < 0
rho = rhoLocal + rhoFixed + sH*f(q)
```

This profile is continuous with value one at its reference height, decays above it and approaches two below it.
It is not an unbounded exponential below the reference plane.
The exponential is natural `exp`: DXBC's base-two exponential is preceded here by a `1/ln(2)` conversion.
Scene maximum depth gates directional source evaluation; it does not simply erase all global density behind geometry.

For normalized view ray `v` and uploaded light direction `l`, directional colour is:

\[
\mu=v\cdot l,\qquad
C_d=\frac{1+\mu}{2}C_{\rm direct}+\frac{1-\mu}{2}C_{\rm opposite}.
\]

There is no power-16 sun glow in this global directional expression.
For positive `kH`, the vertical optical-column function used for light attenuation is:

\[
H(h)=
\begin{cases}
e^{-k_Hh}/k_H,&h>0,\\
e^{k_Hh}/k_H-2h,&h\leq0.
\end{cases}
\]

For slice boundary depths `z0,z1` and a source start distance `a`, define `clip(a)=saturate((z1-a)/(z1-z0))`.
The unshadowed write's source, before adding previously accumulated local light, is:

```text
sky = rho * AmbientColor * exp(-sH*H(h)*AmbientHeightAtten)
      * clip(g_HeightFogParam.z)
dir = rho * Cd * exp(-sH*DirLightAngleInvCos*H(h)
                    *DirLightHeightAtten*ExtinctionScale)
      * clip(DirLightStartDist)
source = localSource + PreExposure*(sky + dir)
```

Directional height attenuation is bypassed when `kH` is not positive in this write variant.
Shadowed variants additionally use directional visibility and sample/reprojection controls.
The two `StartDist` values gate light sources, not total extinction, and must not be implemented as one global no-fog distance.

Local point fog uses a different angular expression with `ScatteringG`.
For distance `d`, uploaded inverse-radius-squared `rInv2`, distance coefficient `b=Color.w`, view/light cosine `mu` and asymmetry `g`, its sampled contribution includes:

```text
radial = saturate(1-d*d*rInv2)/(1+0.7*(d*b)^2)
t = (1.4/(1.4+d*b))^2
muAdjusted = (1-t)*mu + t
phase = 0.25*(1-g*g)/(1+g*g-2*g*muAdjusted)^1.5
contribution = rhoInput*exp(-rhoInput*d)*radial*phase*Color.rgb
```

The shader averages dithered samples within the slice and applies `PreExposure` before adding the packed light source.
This is HG-like local scattering without the usual explicit `1/pi` normalization.
`rhoInput` is the density present when this pass runs, so global/local scheduling matters.
The CPU mapping of `LocalLightScale` and `LocalLightDistScale` to `Color` and radius remains open.
Local density boxes average a saturating minimum of transformed distances to their three faces, multiplied by the uploaded box density.

### Integration and apply

`SolveVolumetricFog.cpo` dispatches 8x8x1 groups, with each thread marching the depth slices for one xy column.
For slice index `n`, it reconstructs `ds=exp2(LogDepthRangeInfo.x+(n/N)*LogDepthRangeInfo.z)*PreComputed.x`.
This is the uploaded segment measure; its conversion to world metres is not proven.
The exact midpoint march is:

```text
tau = 0
L = (0,0,0)
for each slice:
    (rho, source) = unpack_half_accumulators(slice)
    deltaTau = rho*ds
    TrgbMid = min(exp(-ExtinctionScale*(tau+0.5*deltaTau)), 1)
    L += source*ds*TrgbMid
    tau += deltaTau
    volume[slice] = (L, exp(-tau))
```

The first stored slice follows the same midpoint convention, and terminal slices hold the end state plus an optional analytical continuation source.
There is no closed-form `(1-Tr)/extinction` segment integral in this solver.
`Global.x` is incorporated in the write pass and is not added again by the solver.

`SceneComposite.ppo` binds the integrated volume at t0 and scene depth at t1, maps reconstructed depth to logarithmic volume z and returns scattering RGB plus scalar transmittance in alpha.
It does not sample scene colour or perform `L+T*C` in its own pixel instructions; applying those outputs to the destination depends on CPU blend state, which remains open.
The forward FLVER shader does prove direct application, with channel transmittance `T^ExtinctionScale`, rather than treating alpha as identical RGB transmission.

### FLVER forward and far geometry

The representative file is `gxflvershader.shaderbnd.d/GXFlver_ColDifSpcBumpBlendIbl_Bloody_Cloth_Fwd.ppo`.
It binds `g_VolumeFog0` at t47 and `FC_VolumeFogParam[4]` at byte 1072 of `cbScene`.
With `P=FC_VolumeFogParam`, its lookup depth is `P[0].y*log2(min(depth,P[0].w)+P[0].x)+P[0].z`.
When `depth>P[1].w`, it analytically extends fog past the capped volume depth `D=P[0].w`.
For the shader's camera-relative vector `r`, the added path length is `length(r)*(1-D/depth)` and endpoint heights are `-r.y*D/depth+P[2].w` and `-r.y+P[2].w`.

The extension integrates `P[3].y*f(P[3].x*h)+P[3].z` along this segment.
Its primitive is `-exp(-k*h)/k` above zero and `2*h-exp(k*h)/k` below zero, with separate crossing and near-horizontal handling.
For nonhorizontal rays the height integral multiplies the absolute primitive difference by `abs(length(r)/(-r.y))`; this ratio uses the full ray length, not the extension length.
The near-horizontal branch uses the start height's density times the extension length when `abs(k*(h1-h0))<0.0001`.

The shader also samples the terminal volume layer, giving `(Lend,Tend)`, and scales added optical depth by `1-Tend`.
Writing `delta` for that scaled optical depth and `E=P[1].rgb`, the extension and final application are:

```text
Lvolume += Tvolume^E * Lend*(1-exp(-E*delta))/E
Tvolume *= exp(-delta)
Cout = SC_PreExposure * (Csurface*Tvolume^E + SC_InvPreExposure*Lvolume)
```

This is a verified far-geometry path, not evidence that all alpha, depth-only, vegetation and material permutations use identical fog.
The supplied separate `Fog` GPARAM selector is not mapped to a distinct distance/height formula by this representative path.

### Fog inputs and GPARAM mapping

Offsets below are within `CB_SHADER_COMMON_PARAM`, usually cb0.
This table retains the initial shader-only candidate mappings; `SHADER-INPUTS.md` now supplies the verified CPU transforms.

| Input and byte offset | GPARAM candidate or source | Shader-space units and status |
|---|---|---|
| `VolumeSize` uint4 0; `InvVolumeSize` float4 16 | VolumeWidth/VolumeDepth and renderer allocation | Texel counts/reciprocals; CPU packing open. |
| `DepthRangeInfo` float4 32; `LogDepthRangeInfo` float4 48; `PreComputed` float4 64 | NearDepth/FarDepth/GameScale and renderer depth convention | Packed distance/log coefficients; conversion open. |
| `ViewMatrix` matrix 80; `ViewTangent` float2 144; `DepthComputeParam` float2 152 | Camera state | Projection/view transforms, not GPARAM. |
| `g_HeightFactor` float4 160 | HeightFogOffset plus camera transform | Affine height coordinate; offset sign/scale open. |
| `Global.x` float4 176 | Global FixedDensity | Density per uploaded segment unit; strong role match. |
| `AmbientColor` float3 192 | SkyLight SkyColor | Source RGB; CPU colour conversion/intensity packing open. |
| `AmbientHeightAtten` float 204 | SkyLight HeightAtten | Optical-column multiplier; candidate. |
| `g_HeightFogParam.xy` float4 208 | HeightFogDistribution / HeightFogDensity Scale | Inverse uploaded height unit / density scale; strong role match. |
| `g_HeightFogParam.z` at 216 | SkyLight StartDist | Uploaded view-depth threshold; candidate. |
| `DirLightDirection` float3 240 | DirLight LightBankId selects a light | Direction is derived, not the ID itself; resolver open. |
| `DirLightAngleInvCos` float 252 | Derived directional geometry | Used in vertical-column attenuation; CPU formula open. |
| `DirLightDirectColor` float3 256; `DirLightHeightAtten` float 268 | DirectColor and directional height attenuation | Separate reflected fields; do not read a nonexistent float3 alpha. |
| `DirLightOppositeColor` float3 272 | OppositeColor | Opposing directional source RGB; candidate. |
| `ExtinctionScale` float3 560 | Global AbsorptionScale or derived colour extinction | Per-channel optical-depth multiplier; exact mapping open. |
| `PreExposure` float 572 | Renderer camera exposure state | Radiance multiplier; not a demonstrated raw GPARAM field. |
| `EnableAnalyticalFog` bool 584 | EnableAnalyticalFog renderer control | Enables terminal analytical branch; candidate. |
| `DirLightStartDist` float 1200 | DirLight StartDist | Uploaded view-depth threshold; candidate. |
| Local `PositionAndRadius` float4 0, `Color` float4 16, `ScatteringG` float 96 in `CB_LOCAL_LIGHT` | Local light plus LocalLightScale/LocalLightDistScale/ScatteringG | Radius coefficient and distance attenuation are transformed uploads; open. |

Shadow matrices, cascade metadata, dither, reprojection history and random vectors are renderer inputs rather than GPARAM values with demonstrated direct mappings.
The complete layouts and binding differences are in the reflection index.
For the first Outskirts noon row, the dump gives FixedDensity 0.0015, height distribution 0.1, height offset -150, height scale 0.25, GameScale 2.5, NearDepth 0.01 and FarDepth 400.
These numbers do not justify substituting raw metres for every uploaded coordinate.

### Implementation status (2026-10-11)

`crates/game/src/fog.rs` and original `fog.wgsl` replace Bevy `DistanceFog` with an HDR scene pass before bloom and tone mapping.
The pass reconstructs camera-relative rays from the main depth buffer, preserving MSAA without an extra depth prepass.
It applies the recovered bounded height profile, independent sky/directional source gates, linear DirectColor/OppositeColor angular interpolation, optional vertical-column attenuation, RGB extinction and authored geometric depth segments.
The far path uses the piecewise height primitive, crossing/near-horizontal cases, the terminal accumulated light and the documented `(1-Tend)` scaling of added optical depth.
The same pass treats clear-depth background as a 20 km ray, so geometry and horizon use the same source colours.
Application follows the demonstrated forward `Csurface*T^E+L` expression; the native scene-composite blend state remains open.
`--no-fog` removes this pass.

GPARAM values come from the active LightSet row and interpolated map/hour keys, including Enable, NearDepth, FarDepth, FixedDensity, height fields, both source StartDist values, SkyColor, DirectColor, OppositeColor and channel AbsorptionScale when present.
GameScale is applied exactly once when preparing metre-based shader inputs.
The MSB FogParamID is exported independently and selected from the collision under the focus, with read-only MSB fallback for older exports.
Separate Fog rows are read and reported, but the three tested map draw-param files have no separate Fog group to apply.
No unproven distance-fog formula is introduced for that selector.

Verified CPU uploads now follow `SHADER-INPUTS.md`: GameScale multiplies Near/Far/NearOffset and source starts, and divides fixed/height distribution/density.
Height is signed `(worldY-raw HeightFogOffset)`, `kH=abs(distribution/GS)`, `sH=0.5*densityScale/GS`, and extinction is `1+AbsorptionScale`.
Fog colours use clamped byte RGB raised to 2.2 times W, and directional slant is `1/max(sign*sun.y,AngleClip)`; the sun remains an inferred bank resolver.
Castle noon now uploads near 0.025 m, far 1000 m, fixed density 0.0006/m, height coefficient 0.04/m, height density 0.05/m and raw offset -150 m.
Geometric boundaries use the scaled NearOffset and authored VolumeDepth clamped to 4..128, independent of the surface endpoint; partial terminal slices clip at the surface.
Authored sample offset selects density evaluation within each segment and EnableAnalyticalFog gates far continuation.
Round-to-nearest byte quantization and direct ray integration in place of a padded froxel texture remain implementation assumptions.

This is global shader-derived fog rather than the complete native froxel renderer.
Local density boxes and point/spot fog sources are currently zero, and shadowed/reprojected source variants, native pre-exposure packing and local/global write ordering are not implemented.
The cached exporter still omits fog cards.
Transparent surfaces share the opaque scene depth used by this post pass, so they do not yet receive an independent forward-material fog lookup.
The direct ray pass uses authored depth slices but does not allocate the native padded 3D volume.
The shader-input recovery above remains valid independently of these implementation approximations.

## Tone mapping and exposure

### Named operators and transfer functions

All following files are in `gxposteffect.shaderbnd.d`.
`ToneMap_ReinhardToneMapPS.ppo`, `ToneMap_WriteReinhardToneMappingTablePS.ppo`, `ToneMap_WriteFilmicToneMappingTablePS.ppo`, `ToneMap_TabledToneMapPS.ppo`, `ToneMap_TabledToneMapLUTPS.ppo`, `ToneMap_ColorToLuminancePS.ppo`, `ToneMap_AdaptedLuminancePS.ppo`, `ToneMapVS.vpo` and `ToneMapAdaptVS.vpo` were inspected.
Other available variants include exponential, divide, direct filmic, sensitometric and extended Reinhard operators, reset passes and separate-begin/end passes.
Their presence alone does not identify the active Sekiro mode.
The subsequent executable trace in `SHADER-INPUTS.md` identifies native mode codes and constructor default 7 (sensitometric).

For scaled per-channel scene input `x`, the directly evaluated Reinhard branch begins with:

\[
R(x)=\left[\frac{x(1+0.01x)}{1+x}\right]^{1/p_y},
\qquad p=\texttt{g_ToneMapParam}.
\]

It then applies `g_mtxColorMultiplyer` to `(R,1)`, clamps negative RGB, multiplies by vignette, and in the ordinary output branch raises each channel to `p.z`.
The vignette factor is `smoothstep(t)^4`, where `t=1-saturate(length((2*uv-1)*V.xy)*V.z+V.w)` and `V=g_vVignettingParam`.
The HDR-flagged branch performs additional inverse-Reinhard/luminance shaping using `g_ReinhardParam.x`, HDR display parameters and an output power approximately `1/3.3`; it must not be treated as the ordinary gamma branch.
For matrix/vignette result `b`, its recovered scalar shaping is:

```text
c = g_ReinhardParam.x
Y = dot((b/max(1-b,0.01))^(1/c), (0.29891,0.58661,0.11448))
shape = Y^c/(1+Y^c)                         if Y <= 1
        0.5+19*(u^c/(1+u^c)-0.5)           otherwise, u=1+(Y-1)/19
hdrRGB = 0.497702*(shape*b/(dot(b,luma)+0.0001)
                  *g_vHDRDisplayParam.y)^0.303030
```

These constants are an observed output encoding, not proof of a particular display standard or GPARAM mode.

The tone tables encode HDR values through `u=x/(x+0.2)`.
Their writers reconstruct `x=0.2*u/(1-u)` and emit:

```text
ReinhardTable(u) = (x*(1+0.01*x)/(1+x))^(1/p.y)
v = max(x-0.004, 0)
FilmicTable(u) = (v*(6.2*v+0.5)/(v*(6.2*v+1.7)+0.06))^(2.2/p.y)
```

The extended Reinhard table writer instead uses `z=(x*g_ReinhardParam.y)^g_ReinhardParam.x` and outputs `(z/(1+z))^(1/p.y)`.

`ToneMap_TabledToneMapPS.ppo` first forms `x=scene*SceneLumScale*adaptedGain + glare*g_GlareLuminance.x`, then samples the tone table independently for RGB using `x/(x+0.2)`.
Matrix, nonnegative clamp, vignette and ordinary output exponent follow that lookup.
This proves glare is added before the tone curve in this variant.
The LUT sibling adds grading after the ordinary output exponent, as detailed below.

### Metering and adaptation

`ToneMap_ColorToLuminancePS.ppo` samples 16 offsets and computes `Y=dot(rgb,(0.299,0.587,0.114))`.
Each log sample is `log2(Y*g_vScenePreExposure.z+0.00001)+20`, with invalid sampled luminance replaced by the epsilon equivalent before adding 20.
The average therefore stores a biased log luminance, not plain luminance or unshifted EV.
Its screen weight is `w=1-(1-exp2(-r))^3`, with `r=length(2*(uv-0.5)*g_AdaptCenterWeight.xy)`.
The first reduction output is `(w*average,w,1,1)`; the final adaptation input uses a z/w weighted reduction ratio.
The intervening reduction/binding configuration is not fully reconstructed.

For reduced input `current` and previous adapted log value `old`, `ToneMap_AdaptedLuminancePS.ppo` does:

```text
target = current.z/current.w - g_AdaptParam.x
rateUp = 1 - 10^(-g_AdaptParam.y)
rateDown = 1 - 10^(-g_AdaptParam.z)
rate = rateUp if target > old else rateDown
adapted = old + (target-old)*rate
output = (adapted, exp2(20-adapted), 0, 1)
```

`ToneMapAdaptVS.vpo` reads that output's y channel and passes it to the pixel shader as the adapted gain.
`ToneMapVS.vpo` instead passes gain one.
No AdaptionMin/Max clamp is present in this adaptation pixel shader.
The uploaded rates could contain elapsed-time factors; the shader does not establish the CPU conversion from authored speed to those rates.
Likewise the negative authored Middle Gray values are compatible with logarithmic controls, but do not prove `g_AdaptParam.x` equals the raw field.

### Post constant-buffer mapping

`cbToneMap`, typically cb1, packs controls into vectors rather than retaining GPARAM field names.
The following table retains the initial shader-only candidate mappings as research provenance.
Executable-derived exposure, adaptation, mode/power and bloom uploads are now recorded in `SHADER-INPUTS.md` and implemented in the native post section below.

| Variable, type and byte offset | GPARAM candidate | Status / units |
|---|---|---|
| `g_ToneMapInvSceneLumScale` float3 0; `g_ToneMapSceneLumScale` float4 48 | ToneMap-Exposure plus scene scale/pre-exposure | Linear gain; conversion from authored exposure open. |
| `g_ReinhardParam` float4 16 | ReinhardContrast | Operator shaping; exact packing open. |
| `g_ToneMapParam` float4 32 | Gamma, FilmicContrast/ToneMapMode-derived values | y and z are power controls; identity with raw fields open. |
| `g_AdaptParam` float4 64 | Middle Gray, bright/dark adaptation speed | Log offset and response-rate arguments; upload conversion open. |
| `g_AdaptCenterWeight` float4 80 | AutoExposure Center Weight | Screen-radius weighting; candidate. |
| `g_BrightPassThreshold` float4 96 | Glare Threshold | x is threshold in scaled scene luminance; candidate. |
| `g_GlareLuminance` float4 112 | Glare Luminance | x scales glare before tone mapping; candidate. |
| `g_BloomBoostColor` float4 128 | Saturate Ctrl and bloom colour controls | w blends toward luminance, RGB multiplies; packing open. |
| `g_vBloomFinalColor` float4 144 | GlareBloom Alpha and bloom colour controls | Final linear RGB gain; packing open. |
| `g_vBloomScaleParam` float4 160 | Radius/scale-related bloom controls | Variant-specific upload; no direct identity established. |
| `g_mtxColorMultiplyer` float4x3 176 | Colour controls or renderer grading | Affine RGB transform; producer open. |
| `g_bEnableFlags` bool4 256 | Tone/HDR/renderer mode | z chooses the additional output branch; raw GPARAM bit mapping open. |
| `g_vFeedBackBlurParam` float4 272 | FeedbackWeight and time/freeze controls | Temporal producer and component mapping open. |
| `g_vVignettingParam` float4 288 | Vignetting controls | Screen-domain polynomial parameters; candidate. |
| `g_vHDRDisplayParam` float4 304 | HDR display settings | Output encoding/scale, not a demonstrated map GPARAM. |
| `g_vScenePreExposure` float4 384 | Renderer camera exposure state | z unscales metering; exact producer open. |
| `g_ToneMapSampleOffset[16]` float4 array 0 in cb2 | GaussianRadiusScale and renderer sampling | xy sample offsets, z blur weights; generated on CPU. |

AutoExposure enable, ToneMapMode, AdaptionMin/Max, FreezeAdaptionFade and GlareNumLevels can select or configure passes on CPU without appearing as a direct pixel-shader variable.
They must not be assigned invented component mappings.

The historical Bevy implementation used ACES fitted, `BASE_EV=-0.5`, `METER_EV=2`, correction `BASE_EV+Exposure+clamp(MiddleGray+METER_EV-L,min,max)`, capped adaptation bounds, and fixed brighten/darken speeds 3/2.
Those choices differ from the recovered log reduction, biased-log gain, table operators and renderer pre-exposure contract.
**Implemented:** the calibrated compensation curve is replaced by recovered reduction/adaptation and native transfer functions using the subsequently traced CPU packing.
Keep raw scene scale, metering pre-exposure and output power separate rather than folding them into one guessed EV.

## Bloom and glare

The inspected files are `ToneMap_BrightPassPS.ppo`, `ToneMap_BrightPassAndScalePS.ppo`, `ToneMap_BrightPassTemporalPS.ppo`, `ToneMap_BloomGaussBlurPS.ppo`, `ToneMap_BloomCompositePS.ppo` and the single/upscale composite siblings.
The dependency order is scaled scene extraction, filtering, level composition, then glare addition inside the table tone mapper.
The number of levels and repeated blur/downsample schedule are CPU choices.

For the ordinary bright-pass variant, define `c=sampleScene*SceneLumScale*adaptedGain`, `Y=dot(c,(0.29891,0.58661,0.11448))` and `b=g_BloomBoostColor`.
The exact extraction and colour adjustment are:

```text
bright = c*max(Y-g_BrightPassThreshold.x,0)/(Y+0.0001)
if bright contains NaN: bright = 0
gray = dot(bright,(0.29891,0.58661,0.11448))
output = b.rgb*lerp(bright,gray.xxx,b.w)
```

The Gaussian pass sums seven samples: `blur=sum(sample(uv+offset[i].xy)*offset[i].z)` for `i=0..6`.
The compiled shader consumes weights and offsets; it does not generate a Gaussian radius or normalized kernel from the GPARAM scalar itself.
The three-level composite is `(level0+0.5*level1+0.25*level2)*g_vBloomFinalColor.rgb`.
The temporal bright-pass and feedback controls are present but their complete runtime history/reset policy is not recovered.
The inspected temporal bright pass reprojects history with motion vectors, scales it by `g_vBloomScaleParam.z`, clamps it to current center/cross-neighbour RGB bounds, and interpolates current/history in log2 colour using `g_vBloomScaleParam.y` and epsilon 0.0001.
It writes the filtered scene to a second target before applying the same exposure-scaled luminance extraction to the first target.
Thus its smoothing is a motion-reprojected logarithmic filter, not merely another spatial blur.

The GPARAM candidates and offsets are in the shared post table above.
`Glare Threshold` belongs to exposure-scaled luminance, while `GlareBloom Alpha` cannot be equated to a Bevy intensity without recovering final-colour packing.
The historical Bevy implementation used threshold softness 0.5, additive bloom with `intensity=clamp(alpha*0.08,0,0.08)`, radius clamping and its own multiscale filters.
**Implemented:** luminance-subtractive extraction and the native seven-tap/three-level composition accompany the selected tone operator.
Exact CPU tap offsets, the complete authored-level schedule and temporal controls remain to be ported.

## Colour grading

The relevant files are `ColorGrading.ppo`, `ToneMap_Copy2Dto3DLUTCS.cpo`, `ToneMap_Blend2Dto3DLUTCS.cpo`, `LutCopy2DTo3DCs.cpo`, `LutCopy3DTo2DCs.cpo` and `ToneMap_TabledToneMapLUTPS.ppo`.
The copy compute shader loads strip texel `(r,16*b+g)` and writes volume texel `(r,g,b)`, proving red-fastest, green within each 16-row slice, blue selecting the slice.
The layout is 16x256 for the strip and 16x16x16 for the native lookup volume.
The blend compute shader interpolates two strips using `g_ToneMapLUTParam.x` before writing the volume.

The standalone pixel shader binds source t0 with point sampling and LUT t4 with linear clamped sampling.
It computes `uvw=(15/16)*rgb+1/32` and returns the sampled LUT RGB, clamped nonnegative, while retaining source alpha.
Thus nominal input `[0,1]` maps to texel centers, with ordinary trilinear interpolation in RGB cube coordinates.
No log-HDR shaper or gamma conversion is present in this standalone shader.
Its producer must determine whether the source is already display encoded.

The ordinary branch of `ToneMap_TabledToneMapLUTPS.ppo` establishes an internal order: glare/scene combination, tone-table lookup, matrix, vignette, output power `g_ToneMapParam.z`, then the center-adjusted LUT lookup at t2.
This is stronger evidence for grading after the output power than the earlier web interpretation.
The HDR-flagged branch additionally reverses that power after LUT lookup and applies luminance/output shaping, so the ordinary ordering is not a universal HDR claim.

`ColorGrading Enable` and `LutSourceId` are CPU resource/pass selectors, with no cbuffer in the standalone pass.
The texture ID resolver, time blending policy and relationship to `g_ToneMapLUTParam.x` remain open.
`grading.rs` now schedules a 2D-strip lookup after the native tone pass and before UI, using nearest authored LUT IDs.
Its axis layout matches the recovered strip, and its input is now the native ordinary output-power result.
**Implemented:** the proven strip layout and equivalent exact trilinear sampling run after the recovered ordinary output power.
Authored LUT blending remains open.

### Native post implementation (2026-10-11)

`lighting.rs`, `tonemap.rs`, `exposure.wgsl` and `tonemap.wgsl` replace Bevy AutoExposure, Bloom and ACES with original implementations of the recovered ordinary chain.
The 3D camera uses `Tonemapping::None`; the separate 2D HUD never receives `GamePost` or `GradingLut`.
Fog remains before the native HDR post pass, and grading runs after its output power.
The executable formulas in `SHADER-INPUTS.md`, which arrived during implementation, supersede the former post-upload hypotheses and the first v6 calibration candidates.

The meter averages `log2(Y+0.00001)+20`, with `Y=dot(scene,(0.299,0.587,0.114))`, over a deterministic 64x64 grid grouped into sixteen samples per reduction cell.
It uses the recovered radial weight `1-(1-exp2(-r))^3`, with center axes `(CenterWeight,CenterWeight*height/width)`.
The target is the weighted biased-log mean minus authored MiddleGray, with response `1-10^(-dt*LightAdapt)` when it rises and `1-10^(-dt*DarkAdapt)` when it falls.
The first valid meter initializes history immediately, approximating the native reset argument 4.
The previous adapted EV drives the recovered natural-exponential soft limit, retaining the CPU path's one-frame latency without CPU readback.
For positive EV the limit is `Max*(1-exp(-EV/Max))`; for negative EV it is `abs(Min)*(exp(EV/abs(Min))-1)`.
FreezeAdaptionFade forces this offset to zero, rather than freezing the GPU meter.
The gain is `2^Exposure * 2^(-limitedEV)`, applied once to scene and bright-pass input.
Shipped native pre-exposure and its inverse are one; no guessed BASE_EV or METER_EV remains.
The v7 light bridge folds the former post-input 0.5 conversion into `SCENE_TO_BEVY=4500`, shared by sun/fill, hemisphere, probes and the inferred BTL mapping.
Metering and exposure input now use scale one; this engine bridge is separate from the verified GPARAM transforms.
The unconverted native-tone candidate clipped about 1.03 percent of the noon world crop to pure white and was rejected.
`--no-auto-exposure` retains only `2^Exposure`.

Modes 0, 3, 7 and 8 evaluate the native filmic, Reinhard, sensitometric and extended-Reinhard table writers analytically.
The native `x/(x+0.2)` address and writer reconstruction cancel, so this avoids lookup-table quantization while preserving the transfer function.
Sensitometric uses `A=1-exp(-1.442695*x)`, `B=1-exp(-4.456669*x)` and `A+FilmicContrast*(B^3-A)`.
AfterGamma uses table power one and final output power `1/2.2`, before the LUT.
Missing authored controls use traced constructor defaults: mode 7, FilmicContrast 0.5, ReinhardContrast 1.6, ReinhardWhiteBalance 1.7, MiddleGray -2, LightAdapt 1, DarkAdapt 0.5, CenterWeight 4 and EV bounds -8/+8.
`--native-tone-reinhard` and `--native-tone-filmic` provide diagnostic overrides.
Unimplemented mode codes warn and fall back to mode 7.

Bright extraction uses `c*max(Y-Threshold,0)/(Y+0.0001)`, the recovered luminance weights and saturation interpolation.
Three spatial levels at half, quarter and eighth resolution receive separable seven-tap Gaussian filtering and the recovered `level0+0.5*level1+0.25*level2` composition.
The final gain is the recovered `Alpha*Luminance/sum(Smooth^i)`, using authored GlareNumLevels, with native glare luminance one.
The remaining renderer quality scale is explicitly named `BLOOM_QUALITY_SCALE=1`.
The seven-tap kernel has sigma 1.4 with normalized weights; its exact CPU tap spacing remains unverified, and GaussianRadiusScale multiplies that spacing.
The authored level count determines the verified normalization, but the full native level allocation/upscale schedule and temporal feedback are still unimplemented.
`--no-bloom` skips extraction and blur work.

The unorm 16x256 LUT retains the proven `(r,16*b+g)` layout and exact eight-cell trilinear lookup.
The tone pass stores its display-power result through an sRGB-to-linear conversion because Bevy's final output expects linear pixels.
Grading reverses that storage conversion before lookup and stores its result linearly again; this pair adds no semantic gamma stage.
Half-LSB display dither runs after grading, or in the tone pass when grading is disabled.
Per-view history is independent, retained across resize and released when its camera disappears.

Remaining post gaps are the exact full-screen reduction/downsample layout, native table interpolation precision, reset-counter timing, frame-delta units, quality scale, full bloom/temporal schedule, colour matrix/vignette, LUT blending and HDR display branch.
Bevy's existing daylight camera/light-unit calibration remains outside this post replacement.
The v7 follow-up ports fog and LightSet colour/hemisphere CPU transforms.
GIIV remains recommendation 3 and is not implemented by this change.

## Deferred lighting, ambient and environment

### Directional BRDF and light records

Representative files are `GXLightAcc_DDirLight1.ppo`, its `Shadowed1` sibling, `GXLightComposite.ppo`, `GXEnvMapRegion[Box][DiffSpcPcc].ppo`, `GXGiVolume_Lighting_Iv.ppo` and its HDR-decode sibling in the renderer bundle.
`GXSimple_Skybox_F.ppo` is in `gxshader.shaderbnd.d`.

The directional pass reconstructs position, normalizes the decoded normal, selects a light-set record from a G-buffer byte, and writes separate diffuse/specular accumulation targets.
Let `N,V,L,H` be normal, view, uploaded light direction and normalized half vector.
Let `q` be the sampled material channel and `w=Vec_i.w`; these are register-level inputs, not proven perceptual roughness and physical light intensity.
With `s=1+FC_SsaoWeight_i*(sampledVisibility-1)`, the ordinary first-two-light diffuse term is `s*Col_i.rgb*saturate(dot(N,L))`.
The shader does not multiply this term by `Col_i.w`; any authored colour-alpha/intensity fold happened before upload if it is represented in RGB.
Likewise specular colour uses `Spc_i.rgb`, not an extra explicit `Spc_i.w` multiplier in the inspected branch.

The recovered tuned microfacet terms are:

```text
a = (1 - 0.95*sqrt(q*(1 - 0.075/w)))^2
nL = saturate(dot(N,L)); nV = saturate(dot(N,V))
nH = saturate(dot(N,H)); lH = saturate(dot(L,H))
D = a*a/(3.14159*(1+nH*nH*(a*a-1))^2)
F0 = sampledSpecularRGB^2
F = F0 + (1-F0)*(1-lH)^5
Vis = 1/(max(nL,nV,0.0001)*(1-a/2)+a/2)
Spec = s*Spc_i.rgb*F*D*nL*Vis
```

There is no separate `4*nL*nV` denominator in this expression.
A positive second material channel enters a different diffuse branch with `(max(dot(N,L)+10*qB,0)/(20*qB+1))` and a power-32 angular addition; its original material meaning is open.
Enabled local directional slots use `min((0.5*dot(N,L)+0.5)^2,1)` for their diffuse remap and the same tuned specular construction.
Both accumulation outputs multiply by `SC_PreExposure` before storage.

| Reflected input | GPARAM candidate / source | Mapping limit |
|---|---|---|
| `cbLight.LightSetParam` at cb5, 288-byte record stride | Selected LightSet | G-buffer record selection is visible; bank upload open. |
| `Vec_0..3` float4, offsets 0/16/32/48 | Angle0/1 and LocalLight Angle1/2 | CPU angle-to-vector conversion and Vec.w open. |
| `Col_0..3` float4, offsets 64/80/96/112 | DiffColor0/1 and LocalLight DiffColor1/2 | Shader uses uploaded RGB; raw alpha/intensity and colour-space conversion open. |
| `Spc_0..3` float4, offsets 128/144/160/176 | SpecColor0/1 and LocalLight SpecColor1/2 | Independent RGB specular source; packing open. |
| `AmbientSH[3]` float4 array 192; `AmbientReflection` float4 240 | Possibly Hemi Color Up/Down or other ambient state | No demonstrated mapping; inspected deferred light accumulators do not consume these fields. |
| `bLocalLight` bool4 256 | Enable LocalLight1/2 | First two components gate additional slots; candidate. |
| `FC_SsaoWeight` float4 272 | LightSet SSAO/shadow control or renderer state | Remaps sampled visibility; exact GPARAM producer open. |
| `cbSceneParam.SC_PreExposure` float at 1000, cb8[62].z | Renderer exposure state | Verified accumulation multiplier; CPU producer open. |

Angles in the dump are consistent with radians, but exact handedness, light-vector sign and intensity units cannot be recovered from already-packed vectors.
No conversion to lux or candela is established by these shaders.

### GIIV, GILM and spatial ambient

The box environment shader binds `g_IBLTexture` cube at t25 and four irradiance 3D textures X/Y/Z/W at t27..30.
It reconstructs the receiver position, computes a box-region fade, applies box/parallax correction to the reflection direction and samples a roughness-dependent cube mip.
The cube and diffuse scales are separate `CB_ENVMAP_REGION` inputs.
`g_envMapColorScale` is a float at byte 144 and `g_envMapDiffuseColorScale` is a float at byte 324 in cb3.
For the sampled material channel `q`, the inspected environment variant chooses mip `min(6.5*(1-q),6)`.
The box fade is `smoothstep(saturate(min(faceDistances*axisFactors)*fadeScale))`, and invalid parallax intersections suppress the reflection contribution.
The first-order directional irradiance decoder for a sampled texel `t` is:

\[
\operatorname{decode}(t)=2(t_{rgb}-0.5)\,2^{20(t_a-0.5)}.
\]

This exponent is base two, unlike fog's natural exponential.
For decoded texture samples `X,Y,Z,W`, the swizzle-and-dot sequence evaluates `max(N.x*X+N.y*Y+N.z*Z+W,0)`, then multiplies by `g_envMapDiffuseColorScale`.
This establishes a constant-plus-linear-normal irradiance basis rather than a Lambert diffuse cube alone.
The exact authored coefficient normalization and asset channel naming remain open.
Region alpha and smooth spatial fade control accumulation, and this environment pass also multiplies its lighting outputs by `SC_PreExposure`.

`GXGiVolume_Lighting_Iv.ppo` has a separate cascaded-volume path with inverse matrices at byte offsets 0/64/128 of `CB_GI_LIGHTING`, texture size at 192, fade lengths at 208, scale at 224 and offset at 228.
Its cascades select and blend spatial irradiance samples with a cubic edge fade, followed by `max(coefficientEvaluation,0)*g_giLightingScale+g_giLightingOffset`.
These volume and environment controls have no established direct GPARAM fields in the supplied dump.
GILM cube assets and GIIV 3D assets match the resource functions, but the game's precise filename-to-binding resolver has not been traced.

`GXLightComposite.ppo` adds irradiance, specular and emissive accumulator RGB; it does not apply a tone curve.
`GXSimple_Skybox_F.ppo` samples a cube and multiplies RGB by `cbcol.ConstColor.rgb`, preserving sampled alpha.
Neither the simple skybox tint nor every map's separate sky-dome shader is mapped to GPARAM here.
Fog SkyColor must not automatically be equated to skybox material tint.

Current Bevy selects GILM probes by spatial regions with `ENV_PER_UNIT=SCENE_TO_BEVY/pi` and squared-RGB hemisphere mean fallback; all map materials also evaluate the directional coefficient.
It approximates diffuse irradiance with an exported Lambert cube, uses approximate specular mips, switches probes without native fades and omits GIIV coefficient sampling and native box parallax correction.
Sun and fill use squared RGB times W with SCENE_TO_BEVY=4500; hemisphere/probes use that scale divided by pi, while BTL uses the same inferred candela bridge times 4*pi for lumens.
**Recommendation:** add native GIIV decoding/basis evaluation and region fades/PCC, preserve separate authored diffuse/specular light colours, and implement the recovered BRDF without claiming physical units until the CPU upload scale is established.

## CPU transforms and remaining questions

[SHADER-INPUTS.md](SHADER-INPUTS.md) is the authoritative executable trace for GPARAM-to-shader packing, with verified arithmetic distinguished from inferred library semantics and shipped defaults.
It resolves GameScale lengths/densities, signed raw world height, depth offsets/geometric slices, byte/gamma fog colours, extinction, directional slant, squared LightSet colours and hemisphere coefficients, exposure/soft EV adaptation, bloom final colour and default sensitometric transfer.
The renderer now applies these transforms; its remaining Bevy calibration and implementation limits are documented below and in the v7 status in [VISUALS-PLAN.md](VISUALS-PLAN.md).

Still open:

- BTL native colour/intensity units, LightBankId/LightId routing and the native time-blend factor.
- Renderer quality scalars for local fog collection distance and bloom, frame-delta units, and confirmation of the shipped CPU versus GPU adaptation path.
- Scene-composite blend state, native froxel allocation/padding and local/global write scheduling, local fog, directional shadow/reprojection and independent transparent fog depth.
- GIIV/GILM binding, spatial fades/parallax correction, separate diffuse/specular material response, and directional hemisphere on actor/FX StandardMaterial meshes.
- LUT ID resolution/blending, colour matrix/vignette producers, exact tone-table/reduction layouts, full bloom/temporal scheduling and HDR display output.

## How to reproduce

Run from the repository root in PowerShell, using the existing Python 3.13 and no packages or network:

```powershell
$env:UV_CACHE_DIR = "$PWD/target/alt/shaders/uv-cache"
uv run --no-project --offline --python C:/Python313/python.exe target/alt/shaders/research.py
uv run --no-project --offline --python C:/Python313/python.exe target/alt/shaders/summarize.py
uv run --no-project --offline --python C:/Python313/python.exe target/alt/shaders/reflection.py
```

`research.py` reads shader/cache files only, finds DXBC magic, uses the container size and calls `D3DDisassemble` through ctypes.
`summarize.py` records stage counts and byte-identical shader counts.
`reflection.py` indexes disassembler reflection definitions and extracts reflection containers with `D3DGetBlobPart(D3D_BLOB_REFLECTION_DATA)` for the requested families.
All generated files are beneath `target/alt/shaders/`.
Review equations against the matching instruction and reflection files, particularly exponential bases, swizzles, unused fields, biased log luminance and pre-exposure pairs.
No Cargo build or tests are required to validate this documentation-only research; disassembly success and direct arithmetic review are the verification evidence.

## Implementation priorities

1. Replace the clamped exponential DistanceFog/glow approximation with native signed-height density, separate source start distances, channel extinction and midpoint integration, then share the volume with forward geometry.
2. Implemented for the ordinary post arithmetic and resolved CPU controls: biased-log metering/adaptation, native transfer functions, luminance-subtractive bloom and grading after output power.
   Exact reduction, table interpolation, full bloom scheduling and temporal/HDR variants remain open implementation work.
3. Add native GIIV directional irradiance and environment region blending/PCC, then preserve independent diffuse/specular light inputs and the tuned BRDF.

These recommendations separate recovered shader behavior from the still-required CPU parameter recovery, so implementation can expose the unknown transforms explicitly instead of hiding them in new calibration constants.

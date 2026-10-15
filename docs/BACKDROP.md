# Painted horizon and fog follow-up, 2026-10-15

The very distant painted range is a separate m89 sky-group model, not an omitted layer in the sky dome texture.
Ordinary night group 8907030 enables dome `m900030` and companion `m900130`.
The companion's material is `M[LandScape]_FarMtn_Night`, using shader `M[LandScape]_FarMountain` and the RGBA panorama `m99_farmountain_00_a`.
That panorama contains several overlapping snowy ridgelines, with transparent sky above them.
Its exported cylinder has 52 vertices, radius approximately 5,060 m and bounds Y -932.536 to 470.700 m.
The native MSB placement and FLVER node have identity transforms, and the FLVER texture scale is `(1,1)`.
The near and middle mountain scenery remains ordinary placed geometry, including Outskirts `m925000` and Castle `m911061`.
The earlier `m6*` census identifies local landscape and architecture stand-ins, not this painted range.

The sky loader previously chose the first model in each m89 event group and discarded the separate FarMountain model.
Its embedded-cloud support handles materials inside a dome GLB and does not recover this separate companion.
The sky exporter now derives the companion association from shared MSB entity groups and retains its native material inputs and depth-write flag.
The runtime switches its mesh and material together with the dome, including event-selected special skies.
The range uses its native alpha threshold and finite projected depth, rather than the dome's infinity depth.
The dome's opaque background remains behind it and the map depth test remains active.

The native FarMountain pixel shader is unlit and differs from a generic diffuse material.
It multiplies the albedo by colour 10 and the directional interpolation of colours 13 and 12, then blends toward a height-dependent interpolation of colours 11 and 8.
The height weight is `1 - min(uv.y^1.5,1)`, and floats 21 and 22 supply the blend amount.
Night explicitly blends strongly toward that haze colour, suppressing detail in the upper painted layers.
No replacement tint, geometry height, panorama scale or haze gain is introduced.

## Material colour packing

Executable function `0x14036d290` associates an SAT colour parameter with its independent `-Intensity` value.
Constructor `0x1403dc1a0` retains RGB, alpha and intensity separately.
Upload function `0x1403dc4b0` conditionally converts RGB with exponent 2.2, then multiplies RGB by intensity while preserving alpha.
The FarMountain and CloudFog metaparam colour descriptors enable that conversion: their descriptor bytes at offsets `+0x2e/+0x2f` are both one.
The shared sky and fog-card SAT uploads formerly used exponent 2 and multiplied fog-card alpha by intensity.
They now use the recovered exponent and independent alpha convention.
This does not change the LightSet upload, exposure, tone curve or post processing.

## Distance and height fog

The base Ashina files contain no populated legacy Fog parameters and do not author `EnableAnalyticalFog`, `NearOffset`, `VolumeDepth` or `VolumeSliceSampleOffset`.
Constructor `0x1402a0740` initializes those omitted controls to false, 1, 32 and 0 respectively.
The earlier job already corrected the analytical switch; this follow-up corrects the other three defaults.
Explicit authored values still take precedence.
The signed height profile, source colours, independent source start distances, extinction and GameScale conversion remain the recovered formulas in [SHADER-INPUTS.md](SHADER-INPUTS.md).

| Night row-0 input | Outskirts | Castle |
| --- | ---: | ---: |
| GameScale | 1 | 1 |
| Near / far depth, m | 0.01 / 200 | 0.01 / 200 |
| Fixed density, inverse m | 0.005 | 0.003 |
| Height distribution, inverse m | 0.08 | 0.08 |
| Raw height offset, m | -128 | -110 |
| Uploaded height density scale | 0.5 | 0.125 |

Fixed-density transmission alone is approximately 0.607/0.741 at 100 m and 0.368/0.549 at 200 m for Outskirts/Castle.
Height density adds attenuation according to the actual ray height, and source accumulation blends the surface toward the authored fog colours.
With analytical continuation disabled, this native path does not justify adding kilometres of extra extinction after the terminal volume.
The separate FarMountain material supplies additional painted-layer haze.
Retail haze remains a visual acceptance requirement; these recovered inputs alone do not establish complete atmosphere parity.

## Blue arc isolation

The spot-011 arc survives `--no-sky-dome` and disappears with `--no-fog-surfaces` in a fixed executable and camera.
Ray intersections at three arc pixels identify `m925009_9000`, primitive/material 7, using `M[m11_00]_LandScape_CloudFog1`.
Their distances are approximately 1,944, 1,742 and 2,388 m, in front of the painted horizon.
Its authored scrolling-mask density and vertex coverage therefore must not be removed by hiding the part or clipping its geometry.
The native CloudFog shader confirms the four mask samples, channel selectors, density bias and soft depth intersection used by the existing surface path.
It also proves that the grazing highlight uses `FC_DirLightSpc_0`, independently of diffuse `FC_DirLightCol_0`.
Our surface shader substituted the diffuse colour and intensity for both terms.
Outskirts night bank 100 has diffuse W 4 and specular W 1.5, so this substitution overstates that highlight by 2.67 times even when their RGB matches.
The base bank additionally authors different RGB and W 2 versus 0.5.
The correction uploads the existing native `SpecColor0` record to the fog-card shader without changing shared lighting.
The initial horizon-only control retains the arc and is kept separately from the final specular correction.
The final matched captures also retain a conspicuous blue arc and insufficient retail-like haze.
The painted companion is now submitted with its recovered material, but its night contrast remains too weak to reproduce the reference's readable layered range.
D1 and F1 therefore remain open visual acceptance items; the independent specular correction must not be reported as removal of the arc.

## Evidence and validation

Read-only shader disassemblies, MTD and metaparam audits, executable observations and raw FLVER/MSB checks are retained under `re/research/backdrop-2026-10-15/audit/`.
Build/check logs, Ghidra runtime logs and isolation renders are retained under `target/alt-backdrop/`.
The sky-only export updates the local m89 sidecar and required textures without rewriting map placement, collision, GI, grass or lighting sidecars.
The local shared board already claims the preceding far-mountain task; the network fetch failed and no board publication or commit was attempted.
Final gate measurements and per-view fixture explanations are recorded in [MAP-COMPLETENESS-SWEEP.md](MAP-COMPLETENESS-SWEEP.md).
Six retail/before/after panels, capture manifests and result records are retained in the external progress collection's `2026-10-15/backdrop/` directory.
The final spots are 011/037, 022/074, 091/290, 009/034, 045/160 and 089/282.
Both canonical render gates pass after the reviewed colour-only fixture changes, with unchanged geometry at all twelve views.
Release workspace tests pass 815 tests with 25 existing ignores, and the final game/extractor rerun passes 163 tests with three existing ignores.
Workspace all-target release Clippy with warnings denied, formatting and diff whitespace checks pass.
The first before/final diagnostic images are pixel-identical; the independent repeat differs at one Castle view-4 depth pixel by one channel code without changing any geometry category.

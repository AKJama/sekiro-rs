# Upscaling and frame generation readiness

Research date: 2026-10-13.
This is a read-only source audit, not a tested integration.
Other agents were editing concurrently, so local line references describe the files observed during this session.
Only this document was created; no builds, tests, SDK downloads, installs, game runs, or changes to existing files were performed.

## 1. Summary

| Technique | Feasibility in this renderer | Effort | Main dependency |
| --- | --- | --- | --- |
| FSR 2 / FSR 3 temporal upscaling | Feasible on the current architecture, but not enabled or production-ready | Medium | Temporal input coverage and a Bevy integration around an FSR implementation |
| FSR 4 upscaling | Feasible through native DX12 interop; no verified ready-made Bevy path | Large | AMD runtime integration and wgpu resource/state interop |
| DLSS Super Resolution | Feasible using Bevy's existing optional integration | Medium | Correct custom-pass inputs, Vulkan backend, SDK setup and redistribution |
| FSR frame generation | Feasible with native presentation integration; not a normal Bevy post effect | Large | Swapchain ownership, pacing, UI composition and latency integration |
| DLSS frame generation | Feasible with backend work, with a reported wgpu precedent | Large | Streamline presentation hooks, Reflex and runtime packaging |

Effort includes shipping-quality motion, transparency, resizing and exposure handling, rather than just getting an SDK dispatch to run.
The renderer has HDR scene color, usable scene depth and an engine that supports jitter and skinned motion vectors, but it does **not** currently provide complete temporal inputs.
The immediate risks are custom sky and FX shaders that skip prepasses, CPU-deformed cloth without previous vertex positions, camera-fade coverage that differs from the standard prepass, and FX depth sampling hardcoded to four samples.
Recommended first step: establish a single-sample temporal rendering mode and validate its depth/motion/jitter at native resolution, then try Bevy DLSS SR and the experimental wgpu FSR 3 port behind the same scene-input contract.
Sources for the available paths are [S1], [S2], [S3], [S4] and [S5].

## 2. Requirements and renderer audit

### Versions and built-in options

`Cargo.lock:444-447` pins Bevy **0.19.1**, `519-522` pins `bevy_anti_alias` **0.19.1**, and `6132-6135` pins wgpu **29.0.4**.
`crates/game/Cargo.toml:15-23` enables `3d`, `ui`, image and audio features with Bevy defaults disabled; it does not enable `dlss`.
No `dlss_wgpu` package is currently in the lockfile.
`Cargo.toml:5-10` declares the project's licence as **MIT OR Apache-2.0**, rather than MIT alone.

The cached 0.19.1 dependency sources were inspected in selected line ranges, rather than assuming the newest online documentation matches this project.

| Bevy option | Added in | Status in pinned 0.19.1 | Relevance |
| --- | --- | --- | --- |
| MSAA | Predates the options below; original release not verified | Available; `Msaa` defaults to `Sample4` | Current geometry AA, not upscaling |
| FXAA | 0.9, 2022-11-12 [S6] | Built-in camera effect | Cheap spatial AA fallback |
| TAA | 0.11, 2023-07-09 [S7] | Built-in `TemporalAntiAliasing`; originally experimental, still documents ghosting/transparency limitations | Requires `Msaa::Off`, jitter, depth and motion prepasses; does not perform temporal upscaling |
| CAS / RCAS | 0.11, 2023-07-09 [S7] | Built-in `ContrastAdaptiveSharpening` | Sharpening only, not FSR reconstruction |
| SMAA | 0.14, 2024-07-04 [S8] | Built-in, with lookup tables | Spatial AA fallback without motion history |
| DLSS SR and RR | 0.17, 2025-09-30 [S1] | Optional `dlss` feature, Vulkan on Windows/Linux | Actual temporal upscaling and optional ray denoising |
| FSR 2/3/4 or frame generation | No built-in implementation found in pinned sources | Not available as a Bevy camera component | Requires an additional integration |

Bevy 0.19.1's `bevy_anti_alias/Cargo.toml:96-98` selects optional `dlss_wgpu` 4.0.0.
The upstream compatibility chart associates that wrapper with wgpu 29 and DLSS SDK 310.5.3; the newest wrapper is for wgpu 30, so copying current setup instructions without matching versions is unsafe [S2].
The wrapper's documented build prerequisites include the matching DLSS SDK, Vulkan SDK and clang, plus an absolute `DLSS_SDK` path; Bevy also requires a `DlssProjectId` before plugin initialization.
Bevy's path uses the NGX DLSS SDK through `dlss_wgpu`, not Streamline.
Streamline is the separate route to pursue for DLSS frame generation.

`wgpu-ffx` is an MIT FSR 3 upscaler port targeting **wgpu 29**, with no Bevy adapter supplied by this project [S3].
It is early/experimental, its automatic `GenerateReactive` pass is unimplemented, and device-specific half-precision/wave64/Lanczos permutations are not selected.
Its manifest therefore matches our wgpu generation, but compilation, GPU backend compatibility and output quality were not verified [S3], [S9].
The archived `EmbarkStudios/fsr-rs` bindings are another research reference, not a verified maintained integration for this Bevy version [S10].
An FSR 1 WGSL shader exists, but spatial FSR 1 neither supplies temporal reconstruction nor prepares frame generation [S11].

### What the techniques consume

| Technique | Renderer inputs and placement | What we have | What we lack |
| --- | --- | --- | --- |
| FSR 2 upscaling | Low-resolution scene color, depth, dense current-to-previous motion, jitter, render/output sizes, reset, frame time and exposure/pre-exposure; before UI and normally before tonemapping; reactive and transparency/composition masks improve unsupported shading motion [S12] | HDR scene, reverse-Z depth, standard Bevy prepass infrastructure | Enabled vectors/jitter, consistent coverage, masks and render-scale control |
| FSR 3 upscaling | Similar temporal inputs, optional exposure texture/auto-exposure and optional masks; masks strongly recommended for particles and blended objects [S13] | Suitable Bevy attachments; wgpu 29 FSR port exists | Bevy adapter, material-specific masks and production validation |
| FSR 4 upscaling | Color/depth/motion/jitter/exposure and temporal state; latest guide no longer requires title-generated reactive or transparency/composition masks, though they may be provided [S14] | Much of the common input infrastructure | Native AMD DX12 runtime bridge; same deformation and coverage gaps |
| DLSS SR | Scene color, depth, motion, jitter, output size, exposure or auto-exposure, reset and camera constants; exclude UI; optional bias mask is not an FSR reactive-mask requirement [S15] | Built-in Bevy integration supplies attachments, size, jitter and conversion | SDK feature/setup, Vulkan selection, material corrections and custom-exposure validation |
| DLSS RR | Noisy ray-traced color plus SR inputs, diffuse/specular albedo, shading normals/roughness and specular hit distance or reflected motion; replaces SR when active [S16] | Bevy has an RR integration | Our audited renderer is raster PBR with authored GI/probes, not a noisy ray-tracing pipeline; no matching RR guide buffers |
| FSR FG | Rendered color plus prepared depth/motion, camera constants, jitter, frame ID/time/reset; UI-free presentation color or supported UI composition; swapchain callbacks and pacing [S17], [S18] | Scene passes and UI are logically separate | Persistent HUD-less color/UI attachments, presentation integration and latency markers |
| DLSS FG | Depth/motion, HUD-less color, presentation backbuffer, frame/camera constants; buffers alive through present; UI color/alpha composition and Reflex [S19] | Logical scene/UI separation | Streamline initialization/hooks, tagged persistent attachments, Reflex and generated-frame presentation |

Transparent effects can be handled by suitable masks or a separately composed layer when accurate vectors are impractical.
Writing opaque depth for every particle would break blending and is not the intended solution.
Alpha-tested opaque geometry should have matching discard coverage in color, depth and motion passes.
DLSS RR is not a prerequisite for SR or FG, and adding ray tracing solely to obtain RR would not advance this task.

### What our code actually does

| Local source range | Observation | Temporal consequence |
| --- | --- | --- |
| `main.rs:28-47, 98-160` | Dispatches viewer/map/play modes and effect flags | Camera/render setup lives in the mode modules, not main |
| `play.rs:783-793`, `map.rs:742-761`, `viewer.rs:145-155`, `arena.rs:138-151` | Spawn ordinary `Camera3d` cameras | No explicit TAA, DLSS, motion-vector prepass, temporal jitter or scene resolution override found in game sources |
| `lighting.rs:244-280` | Adds `Hdr`, disables built-in tonemapping/dither, removes Bevy bloom/auto-exposure and installs custom post settings | Good linear-HDR insertion point, but exposure comes from our own pipeline |
| `map_material.rs:14-16, 292-316, 537-556` | `ExtendedMaterial<StandardMaterial, MapLayers>` overrides the forward fragment shader, retaining standard vertex/prepass machinery | Ordinary opaque map/actor meshes can inherit standard motion; absence of an explicit forward motion output does not itself mean motion is unsupported |
| `map_material.wgsl:254-305` | Uses standard alpha discard, but adds screen-space camera-fade discard at 263-272 | Standard alpha test can match; custom camera fade is not repeated by an extension prepass shader, producing a coverage mismatch |
| `character.rs:398-420` | Writes animated joint transforms | Bevy's previous-skin buffers can represent this motion once enabled; runtime correctness still needs checking |
| `cloth.rs:1172-1195` | Replaces mesh positions/normals/tangents on the CPU each frame | Standard previous entity transform/skinning is insufficient for these vertex changes; preserve previous positions or explicitly handle them |
| `sky.rs:54-86`, `sky.wgsl:13-23` | Custom sky cancels camera translation, uses depth zero, disables depth writes and prepass; supports opaque and blended layers | No sky vectors; need rotational reprojection/infinite-depth treatment and cloud shading-change handling |
| `fxr.rs:901-930, 983-1019`, `fxr.wgsl:17-48`, `fxr_multi.wgsl:67-78` | Custom sprite/tracer/multi shaders, Add/Blend/Premultiplied, no prepass or depth writes; sprite corners are batched in world space | No per-particle vectors or stable previous deformation data; require masks, history and/or a separate effect layer |
| `fxr.rs:1034-1048, 1058-1118`, plugin `42-67` | Copies main depth after `MainPass` for the next frame; copy is `Depth32Float` with sample count **4**, sized from physical camera target | Usable depth exists, but this copy is stale and not an upscaler input; it silently skips size/sample mismatches and needs redesign for single-sample reduced-resolution rendering |
| `fog.rs:243-250, 294-300, 415-483`, `fog.wgsl:1-20` | Fullscreen fog after main pass, before early postprocess; samples scene depth with single/multisample variants | Natural pre-upscaler placement; verify reduced viewport coordinates, jittered reconstruction and attachment size alignment |
| `decals.rs:208-225`, `bullet_fx.rs:132-145, 205-220` | Standard alpha-blended decals and some projectile stand-ins | Blended phase does not become vector-complete merely by enabling the opaque prepass; fading/new decals need reactivity handling |
| `lightning_fx.rs:12-47` | Bolts and sparks use gizmos | Bevy queues these in `Transparent3d`, with no audited motion/mask path; they are scene effects before postprocessing |
| `tonemap.rs:50-67, 187-214, 289-319`, `tonemap.wgsl:1-12, 42-50`; `grading.rs:24-33` | Custom adaptation/bloom/tone pass in `PostProcess`, then LUT grading | These run after Bevy DLSS in `EarlyPostProcess`; expose our adapted exposure consistently instead of assuming built-in auto-exposure matches it |
| `combat_hud.rs:75-190, 347-398`, `map.rs:748-761`, `play.rs:793-810` | HUD/markers are Bevy `Node`/text UI | Already separate from scene meshes; not yet a separately retained RGBA UI texture for FG |
| `perf_log.rs:19-40, 143-165, 199-265` | `Instant` CPU timings, render-schedule stamps, pipeline and draw counts | Not GPU timestamp duration, displayed-frame counts, presentation pacing or input-to-display latency |

Inherited glTF material alpha modes are preserved, but their asset-wide inventory was not checked because this audit did not read game assets.
Consequently, the table establishes code paths and known blended/discarded effects, not a verified list of every alpha-tested surface in every map.
Pinned Bevy's `bevy_pbr/src/prepass/prepass.wgsl:137-177, 206-218` handles previous morph/skin/entity transforms and previous camera matrices.
It writes normalized motion with a convention that must be converted for each SDK.
The current DLSS node already negates/scales Bevy motion by render resolution and negates jitter, so duplicating that conversion would be wrong.
It currently sets `exposure: Automatic` and `bias: None` with TODOs in `bevy_anti_alias/src/dlss/node.rs:44-57`.
These exact-version sources are linked in [S20].

Bevy's UI pass runs after `PostProcess` and before its final spatial `upscaling` blit (`bevy_ui_render/src/lib.rs:269-272`).
That final blit is different from temporal DLSS in `EarlyPostProcess`.
Our custom tonemapping and LUT grading run before UI; sky, decals and FX run in the scene; no audited custom scene pass is intentionally drawn after tonemapping.
This is promising for SR, but FG still needs a persistent capture before UI or separate UI composition.

### APIs and hardware

| Path | Native API / wgpu route | Hardware boundary |
| --- | --- | --- |
| FSR 2/3 upscaling | Older FidelityFX SDK supports DX12/Vulkan; shader port can dispatch through wgpu, while native SDK needs handle/state interop [S12], [S21] | Latest AMD table lists RX 500-class or equivalent other-vendor GPUs and SM 6.2; practical performance depends on resolution [S4] |
| FSR 3 FG | Older SDK 1.1.4 offers DX12/Vulkan; latest AMD FSR SDK route is DX12 [S21], [S4] | Latest table lists RX 5000-class or equivalent other-vendor hardware [S4] |
| FSR 4 SR / ML FG | Current SDK 2.3 documents Windows DX12, not official Vulkan; wgpu DX12 backend alone does not expose an AMD SDK integration [S4] | SR 4.1.1: RX 7000/9000 discrete GPUs; FG 4.0.1: RX 9000; SM 6.6 and listed Agility SDK requirement [S4] |
| DLSS SR / RR | Native NVIDIA routes include DX11/DX12/Vulkan; Bevy wrapper is Vulkan-only Windows/Linux [S1], [S15], [S16] | All GeForce RTX generations; no AMD/Intel fallback inside DLSS [S22] |
| DLSS FG / multi-FG | Current Streamline guide supports DX12 and Vulkan; not DX11 FG [S19] | FG on RTX 40/50; multi-FG on RTX 50; query runtime feature support, OS, driver and scheduling requirements [S19], [S22] |

There is no browser WebGPU route to native NVIDIA/AMD SDK DLLs or their presentation interposers.
An FSR shader port has a different portability story, which must be validated per backend rather than inferred from the SDK's Vulkan support.
The current Streamline FG Windows guide specifies Windows 10 version 2004/build 19041 or later and hardware-accelerated GPU scheduling enabled; Vulkan optical-flow support has additional driver/API and optional dedicated-queue requirements [S19], [S25].

## 3. Ranked options and recommendation

1. Build the shared temporal input contract first, using native-resolution Bevy TAA as a diagnostic and retaining MSAA/SMAA/FXAA fallbacks.
   Compare fast camera rotation, skeleton animation, cloth, camera fade, clouds, sparks and transparent decals with depth/motion debug views.
   This is the recommended first step because all subsequent temporal features reuse these inputs.
2. Add optional Bevy DLSS SR on Vulkan, matching `dlss_wgpu` 4 / SDK 310.5.3 and using runtime support detection.
   The engine already manages resolution overrides, jitter, mip bias and SDK dispatch, so work can focus on our renderer's deficiencies.
3. Evaluate `wgpu-ffx` FSR 3 as the broad-hardware path using the same attachments, adding explicit reactive-mask generation and quality validation.
   Prefer this focused port over starting a complete FSR implementation, but its experimental status prevents calling it production-ready.
4. Investigate a native DX12 bridge for FSR 4 if its supported AMD hardware population justifies maintaining another backend path.
   Keep shader FSR 3 and native spatial AA as fallbacks.
5. Prototype FG only once SR inputs and real-frame latency are sound, starting with presentation ownership and HUD separation before interpolation dispatch.
   Consider FSR 3 FG for broader hardware, DLSS FG for RTX 40/50, and ML FSR FG separately for RX 9000.
6. Defer RR until there is an actual ray-tracing renderer and appropriate guide buffers.

### Is frame generation through wgpu possible?

Yes with native integration, but there is no public, portable `SurfaceTexture::present` callback for replacing SDK swapchain creation/presentation in wgpu 29.0.4 [S23].
Borrowing a raw texture/device through `as_hal` solves only part of the problem: resource transitions, queue synchronization, extended buffer lifetime and presentation ownership still need explicit integration.
The Vulkan HAL offers initialization callbacks, while the pinned DX12 HAL exposes its factory read-only.

wgpu issue #9614, opened **2026-06-01** and still shown open when checked, reports a downstream `Instance::init` patch that replaces the DXGI factory with Streamline's proxy before swapchain creation [S5].
Its author reports RTX 4090 validation with two presented frames per rendered frame.
Voxelcraft also documents DLSS FG via Streamline [S24].
These are implementation-author reports, not results reproduced here, and they do not establish unpatched Bevy support.

For Vulkan Streamline, manual integration intercepts swapchain create/destroy/image retrieval/present and requests extra extensions/features/queues before device creation [S25].
For FSR, AMD's swapchain implementation owns pacing, interpolation command submission and UI composition, including extra CPU threads [S18].
Thus a maintained HAL patch, engine integration or independently owned native presenter is plausible; merely adding a compute pass and calling normal present twice is insufficient.
No independently verified FSR FG integration on this exact Bevy/wgpu stack was found.

DLSS FG requires correctly placed Reflex sleep/markers/frame tokens [S19], [S26].
FSR should integrate Anti-Lag 2 where supported, with an ordinary low-latency pacing fallback elsewhere; AMD documents DX11/DX12 and Vulkan driver support on RX 5000 and newer [S27].
Neither is exposed here as a working Bevy/wgpu feature.
Keep simulation/input on real frames, count rendered and generated frames separately, and report latency rather than treating displayed FPS as simulation responsiveness.

## 4. Start doing now

- Give every moving opaque or alpha-tested custom material a correct motion/depth path, including previous deformation and matching discard coverage.
- Retain previous CPU-deformed cloth positions and stable particle identities/previous corners instead of expecting entity transforms to describe vertex motion.
- Handle infinite sky rotation and animated cloud/shading changes explicitly, using vectors, reprojection, masks or a documented separately composed layer.
- Produce reactive/transparency metadata for blended FX, decals and animated shading, even though current FSR 4 can operate without those masks.
- Keep UI outside temporal scene reconstruction and preserve a capture point for HUD-less color plus a separate RGBA UI layer.
- Add and retain a scene render-scale/quality setting independent of window size, with separate render/output extents and per-pass resize handling.
- Support `Msaa::Off` throughout temporal mode, especially the FXR depth copy/bindings, and use current-frame single-sample depth at scene resolution for reconstruction.
- Define jitter, motion sign/units, reverse-Z/infinite-depth flags, exposure/pre-exposure, mip bias and history resets as one shared renderer contract.
- Reset history on cuts, teleports, respawns, map changes and incompatible resolution/mode changes, with sensible first-frame previous transforms.
- Keep fog and other world-space effects aligned with scene depth/jitter before reconstruction, then tonemap/grade and compose UI at output resolution.
- Add GPU timestamp and presentation/latency measurement alongside existing CPU logs before selecting upscaling or FG defaults.
- Keep SDK binaries optional and separately licensed, and reserve a native presentation seam for future FG without committing the whole renderer to one vendor.

## 5. Licences and redistribution

FSR 2 and the older FidelityFX SDK 1.1.4 implementation, including shader FSR 3/FG code, use MIT terms; retain AMD copyright/licence notices when distributing source or binaries [S28], [S29].
The wgpu FSR 3 port is MIT, and Bevy / `dlss_wgpu` are MIT OR Apache-2.0 [S3], [S2].

Do **not** extend those conclusions to every FSR 4/Redstone runtime file.
AMD's current introduction calls the SDK open source/MIT, but its detailed licence sets a binary-only redistribution default with a list of MIT exceptions covering API/framework/source files [S30], [S31].
The default permits binary redistribution with notices and prohibits reverse engineering/decompilation/disassembly.
Treat ML runtime binaries as separately licensed, and check the actual chosen package's file exceptions before shipping.
No leaked or unofficial FSR 4 implementation is needed for the recommended approach.

Streamline's framework licence is MIT with an explicit Nsight Perf exception; DLSS-G is supplied as a precompiled plugin [S32], [S33].
The NVIDIA DLSS/NGX runtime uses the NVIDIA RTX SDK licence, not MIT [S34].
That licence permits application distribution in object-code form subject to additional functionality, protective downstream terms and notices, forbids SDK redistribution as a standalone product, and requires notification before commercial release.
It also prohibits placing the SDK itself under conflicting open-source obligations.
Original MIT/Apache integration code can remain open source while optional NVIDIA runtime binaries retain their own terms; the combined distribution is not wholly MIT.
Bevy's wrapper instructs developers to obtain the SDK separately and ship release runtime DLLs plus the guide's required notices; RR additionally needs its runtime [S2].
Ship production DLLs rather than development/watermarked builds, and preserve required notices/signature checks [S19], [S33].
The complete redistribution manifest for a selected SDK package was not verified because no SDK archives were downloaded.
The licence reviewed is NVIDIA's current published text; the exact licence bundled with the matching older DLSS 310.5.3 package must also be checked at implementation time.

## 6. Sources and verification limits

All links below were checked on **2026-10-13**.
Dates labelled "checked" indicate an undated live page or repository, not an inferred publication date.
Source disagreements are intentional: the detailed AMD licence qualifies its general MIT description, and current Streamline guides supersede the old NVIDIA landing-page statement that Vulkan support is forthcoming.
The latest Bevy release page is not used to infer features in our pinned 0.19.1 sources.
Hardware, drivers, runtime performance, artefact quality, SDK package contents and integration compilation were not tested.
The collaboration `WORK.md` public URL was unavailable during web reading; no remote task claim was published under this document-only research scope.

- [S1: Bevy 0.17 DLSS announcement](https://bevy.org/news/bevy-0-17/#deep-learning-super-sampling-dlss), published 2025-09-30.
- [S2: dlss_wgpu compatibility, setup and distribution](https://github.com/bevyengine/dlss_wgpu), checked 2026-10-13.
- [S3: wgpu-ffx status, inputs and licence](https://github.com/cwfitzgerald/wgpu-ffx), checked 2026-10-13.
- [S4: AMD FSR SDK supported hardware/APIs and version history](https://gpuopen.com/amd-fsr-sdk/), SDK 2.3 dated June 2026; checked 2026-10-13.
- [S5: wgpu DXGI proxy hook request and reported FG validation](https://github.com/gfx-rs/wgpu/issues/9614), opened 2026-06-01.
- [S6: Bevy 0.9 FXAA](https://bevy.org/news/bevy-0-9/), published 2022-11-12.
- [S7: Bevy 0.11 TAA and RCAS](https://bevy.org/news/bevy-0-11/), published 2023-07-09.
- [S8: Bevy 0.14 SMAA](https://bevy.org/news/bevy-0-14/), published 2024-07-04.
- [S9: wgpu-ffx manifest](https://github.com/cwfitzgerald/wgpu-ffx/blob/trunk/Cargo.toml), checked 2026-10-13.
- [S10: archived fsr-rs](https://github.com/EmbarkStudios/fsr-rs), checked 2026-10-13.
- [S11: FSR 1 WGSL implementation](https://github.com/firdawolf/AMD-FSR1-wgpu-shader), checked 2026-10-13.
- [S12: AMD FSR 2 integration manual](https://gpuopen.com/manuals/fidelityfx_sdk/techniques/super-resolution-temporal/), checked 2026-10-13.
- [S13: AMD FSR 3.1.5 upscaling manual](https://gpuopen.com/manuals/fsr_sdk/techniques/super-resolution-upscaler/), checked 2026-10-13.
- [S14: AMD FSR 4.1.1 upscaling manual](https://gpuopen.com/manuals/fsr_sdk/techniques/super-resolution-ml/), checked 2026-10-13.
- [S15: Streamline DLSS SR guide](https://github.com/NVIDIA-RTX/Streamline/blob/main/docs/ProgrammingGuideDLSS.md), live guide checked 2026-10-13.
- [S16: Streamline RR inputs and operation](https://github.com/NVIDIA-RTX/Streamline/blob/main/docs/ProgrammingGuideDLSS_RR.md), version 2.14.1; checked 2026-10-13.
- [S17: AMD FG API inputs](https://gpuopen.com/manuals/fsr_sdk/techniques/frame-interpolation-api/), checked 2026-10-13.
- [S18: AMD FG swapchain and UI composition](https://gpuopen.com/manuals/fsr_sdk/techniques/frame-interpolation-swap-chain/), version 3.1.7; checked 2026-10-13.
- [S19: Streamline FG guide, Vulkan requirements and shipping checklist](https://github.com/NVIDIA-RTX/Streamline/blob/main/docs/ProgrammingGuideDLSS_G.md), version 2.14.1; checked 2026-10-13.
- [S20: Bevy 0.19.1 anti-alias sources](https://github.com/bevyengine/bevy/tree/v0.19.1/crates/bevy_anti_alias/src) and [PBR prepass shader](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_pbr/src/prepass/prepass.wgsl), exact-version local registry files read 2026-10-13; tag URLs supplied for reference.
- [S21: FidelityFX SDK 1.1.4 and older DX12/Vulkan support](https://gpuopen.com/amd-fidelityfx-sdk-1/) and [FSR 3.1 integration slides](https://gpuopen.com/download/FidelityFX_Super_Resolution_3-1_Release-Overview_and_Integration.pdf), checked 2026-10-13.
- [S22: NVIDIA hardware support](https://www.nvidia.com/en-us/geforce/news/gfecnt/20251/dlss4-multi-frame-generation-ai-innovations/), January 2025 article; checked 2026-10-13.
- [S23: wgpu 29.0.4 SurfaceTexture present implementation](https://github.com/gfx-rs/wgpu/blob/v29.0.4/wgpu/src/api/surface_texture.rs), exact-version source checked 2026-10-13.
- [S24: Voxelcraft documented Streamline FG](https://github.com/jtbirdsell/voxelcraft), checked 2026-10-13; author report, not independently reproduced.
- [S25: Streamline manual native API hooks](https://github.com/NVIDIA-RTX/Streamline/blob/main/docs/ProgrammingGuideManualHooking.md), checked 2026-10-13.
- [S26: Streamline Reflex guide](https://github.com/NVIDIA-RTX/Streamline/blob/main/docs/ProgrammingGuideReflex.md), checked 2026-10-13.
- [S27: AMD Anti-Lag 2 APIs, requirements and timing](https://gpuopen.com/anti-lag-2/), latest listed version 2.0.5 dated August 2026; checked 2026-10-13.
- [S28: FSR 2 MIT licence](https://github.com/GPUOpen-Effects/FidelityFX-FSR2/blob/master/LICENSE.txt), copyright 2022-2023; checked 2026-10-13.
- [S29: FidelityFX SDK 1.1.4 MIT licence](https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK/blob/v1.1.4/LICENSE.txt), checked 2026-10-13.
- [S30: AMD SDK general open-source description](https://gpuopen.com/manuals/fsr_sdk/getting-started/), version 2.3.0; checked 2026-10-13.
- [S31: AMD detailed licence and MIT exceptions](https://gpuopen.com/manuals/fsr_sdk/license/), version 2.3.0; checked 2026-10-13.
- [S32: Streamline MIT licence / Nsight exception](https://github.com/NVIDIA-RTX/Streamline/blob/main/license.txt), copyright 2023; checked 2026-10-13.
- [S33: Streamline source versus binary plugins and production distribution](https://github.com/NVIDIA-RTX/Streamline), version 2.14.1; checked 2026-10-13.
- [S34: NVIDIA RTX SDK licence and supplement](https://github.com/NVIDIA/DLSS/blob/main/LICENSE.txt), dated 2024-03-14; checked 2026-10-13.

[S1]: https://bevy.org/news/bevy-0-17/#deep-learning-super-sampling-dlss
[S2]: https://github.com/bevyengine/dlss_wgpu
[S3]: https://github.com/cwfitzgerald/wgpu-ffx
[S4]: https://gpuopen.com/amd-fsr-sdk/
[S5]: https://github.com/gfx-rs/wgpu/issues/9614
[S6]: https://bevy.org/news/bevy-0-9/
[S7]: https://bevy.org/news/bevy-0-11/
[S8]: https://bevy.org/news/bevy-0-14/
[S9]: https://github.com/cwfitzgerald/wgpu-ffx/blob/trunk/Cargo.toml
[S10]: https://github.com/EmbarkStudios/fsr-rs
[S11]: https://github.com/firdawolf/AMD-FSR1-wgpu-shader
[S12]: https://gpuopen.com/manuals/fidelityfx_sdk/techniques/super-resolution-temporal/
[S13]: https://gpuopen.com/manuals/fsr_sdk/techniques/super-resolution-upscaler/
[S14]: https://gpuopen.com/manuals/fsr_sdk/techniques/super-resolution-ml/
[S15]: https://github.com/NVIDIA-RTX/Streamline/blob/main/docs/ProgrammingGuideDLSS.md
[S16]: https://github.com/NVIDIA-RTX/Streamline/blob/main/docs/ProgrammingGuideDLSS_RR.md
[S17]: https://gpuopen.com/manuals/fsr_sdk/techniques/frame-interpolation-api/
[S18]: https://gpuopen.com/manuals/fsr_sdk/techniques/frame-interpolation-swap-chain/
[S19]: https://github.com/NVIDIA-RTX/Streamline/blob/main/docs/ProgrammingGuideDLSS_G.md
[S20]: https://github.com/bevyengine/bevy/tree/v0.19.1/crates/bevy_anti_alias/src
[S21]: https://gpuopen.com/amd-fidelityfx-sdk-1/
[S22]: https://www.nvidia.com/en-us/geforce/news/gfecnt/20251/dlss4-multi-frame-generation-ai-innovations/
[S23]: https://github.com/gfx-rs/wgpu/blob/v29.0.4/wgpu/src/api/surface_texture.rs
[S24]: https://github.com/jtbirdsell/voxelcraft
[S25]: https://github.com/NVIDIA-RTX/Streamline/blob/main/docs/ProgrammingGuideManualHooking.md
[S26]: https://github.com/NVIDIA-RTX/Streamline/blob/main/docs/ProgrammingGuideReflex.md
[S27]: https://gpuopen.com/anti-lag-2/
[S28]: https://github.com/GPUOpen-Effects/FidelityFX-FSR2/blob/master/LICENSE.txt
[S29]: https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK/blob/v1.1.4/LICENSE.txt
[S30]: https://gpuopen.com/manuals/fsr_sdk/getting-started/
[S31]: https://gpuopen.com/manuals/fsr_sdk/license/
[S32]: https://github.com/NVIDIA-RTX/Streamline/blob/main/license.txt
[S33]: https://github.com/NVIDIA-RTX/Streamline
[S34]: https://github.com/NVIDIA/DLSS/blob/main/LICENSE.txt

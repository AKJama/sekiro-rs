# Known fidelity approximations

## Optional map enemy population

`--map-enemies` loads actual Enemy parts and exact NpcParam/NpcThinkParam rows, excluding dummy parts and reviewed boss rows.
The population uses the map visibility interpreter's startup entity/group disables before allocating actors.
Neutral NPCs and event/controller bodies are omitted, and Outskirts Ministry rows have an explicit first-visit/night-role filter where event predicates remain unsupported.
The Great Serpent remains a scripted setpiece and is omitted from the roaming combat population.
Each foe retains the existing pair combat state but no placeholder Wolf VM or player assets; the authoritative Wolf moves between kernels and advances once.
Behaviour graphs, animation clips, skeletons, TAE, FLVER throw models, ThrowParam, PARAM tables and common combat-effect lookups are shared, while actor state and status mutations remain isolated.
NpcParam `activateBorderDist` and `deactivateIntervalDist` control distance sleeping; zero activation distance currently uses an inferred 100 m default rather than a recovered engine default.
Our scheduling policy keeps near (20 m), engaged and locked foes at the full fixed rate, updates distant unaware foes at 10 Hz, and holds dormant foes outside their activation border.
Engagement wakes immediately without a backlog step; the LOD thresholds and rate are our policy, not recovered engine values.
Sight rays run at FwLo's recovered 0.2 + rand*0.2 second search interval (FUN_140623ca0), with damage and nose distance checked immediately.
Population presentation skips far offscreen pose/cloth uploads, updates distant visible poses with their simulation, and limits foe shadows to 30 m from the camera.
Cloth uses the 15 m camera range from FwLo's RustSekiro (`SHINOBI_CLOTH_RANGE`), is carried in its restart pose outside that range, and runs one parallel task per character.
CPU cloth writes its actual output bounds; other foe meshes use dynamic skinned bounds when the pose changes.
Compiled Lua script prototypes and model mesh/material handles are shared across actors; mutable VM globals and cloth buffers remain separate.
Sight/camera rays traverse only the horizontal grid cells they cross, preserving the triangle narrow phase and rejecting vertically disjoint geometry.
Map sound rows, FMOD project metadata, active character banks and the hit-effect tables are prepared behind the loading screen; sound lookups and raw parameter data are shared across NPC rows.
Character-bank event data follows NPC activation and is released when no actor of that kind remains active, before cue playback.
Acquisition uses the think row's normal sight cone, collision occlusion and sight-forget duration, plus damage and nose distance.
Native caution accumulation, hearing, bush concealment, navigation and platoon roles remain incomplete.
PatrolInfo routes use FwLo's documented loop, ping-pong and one-way stepping and Lua arrival actions, but shaped arrival regions and type-5 home-pose updates remain incomplete.
Other foes pause during a paired throw to preserve the single player's throw ownership.
Player projectiles retain per-foe contact histories rather than a shared cross-foe first-impact ordering, and population FXR presentation remains incomplete.
FwLo sources: MIT RustSekiro `src/enemy.rs` spawn_lineup/StartUnaware, `src/ai.rs` Patrol, `tools/patrols.py`, and `docs/kb/enemy.md` patrol-route notes.

## Exposure and tone

Mode 7 now uses the native base-two sensitometric exponentials; the earlier natural-exponential transcription is corrected.
The weighted log meter, base-ten adaptation response, authored exposure and CPU natural-exponential soft limits retain the recovered rules.
Tone tables are evaluated analytically, while exact table precision, reduction layout, reset timing, renderer delta units, bloom temporal feedback, vignette and HDR output remain incomplete.
The [colour audit](COLOUR.md) adds the native pre-gamma hue/saturation/value/contrast matrix and full authored bloom pyramid, and corrects the filmic-contrast field lookup.
Intermediate-hour LUT resource blending and the renderer-global bloom-quality scalar remain unresolved.
The scene-to-Bevy unit bridge remains inferred.
Matched captures still show excess brightness and blue saturation, so this correction does not close L1 or M1 and supplies no replacement tint or multiplier.
See [EXPOSURE.md](EXPOSURE.md) for the native evidence and measured limits.

## Distant mountain materials

The native Mountain family retains authored rock/snow textures, mask projections, normal strengths and baked vertex occlusion.
It shares the map renderer's existing BRDF, roughness representation, shadow filtering and GI composition; native deferred G-buffer material encoding remains a separate limitation.
The finite fog volume follows the native missing-field default for analytical continuation, while local fog sources and independent transparent-surface fog remain incomplete.
Horizon visibility does not establish exact retail sky, colour or illumination parity.

## Authored map grass

Ashina grass reads the six native surface-density channels, type rows, FLVERs and textures.
The Halton bases, area/mean-density count and density-weighted barycentric warp follow static executable analysis, while leaf seed selection, cross-channel fractional-count grouping, exact transform jitter and non-3 hex dithering remain approximate.
The renderer currently instances the authored first-detail meshes in their rest pose; native grass LOD transitions, wind waveform/units and interaction bending remain open.
Exact distribution phase and native near-camera grass handling remain unresolved, and the matched Castle camera can intersect a foreground blade.
The retained wind settings do not justify inventing an animation amplitude or period.
See the Grass section of `MAP-COMPLETENESS.md` for the data and runtime interfaces.

## Native map light composition

Spatial GI candidate filtering preserves the existing volume order, bounds, coefficient sampling and fade arithmetic.
Hemisphere fallback is removed with matching diffuse and specular coverage where spatial GI contributes, rather than disabled globally when a camera probe loads.
The four native diffuse/specular RGB records and enable flags are retained, but the separate-light compositor is opt-in under `SEKIRO_GI_NATIVE_LIGHTS=1` because it fails Castle's grey gate.
Production retains the existing direct lighting, and local-light directions absent from both the selected bank and bank zero are not invented in the experimental path.
For example, Outskirts authors its local angles only in bank 120, so banks 100/110 still need the executable's native default-direction recovery before those local terms can render.
Direct lighting still uses Bevy's material BRDF and sun shadow filtering; native light sharpness, per-light SSAO weights, separate ambient-reflection composition and exact angle construction remain unrecovered.
The existing HDR unit bridge, exposure and tone mappings are unchanged, so this does not close visual-parity item L1.

## Folding Screen Monkeys

Independent monkey combat, screen deaths, shared controller health and map victory run from installed params, animations, Lua and EMEVD.
The hidden target has a normal-start combat replay, while the four-target victory test uses controlled floor catches with native Lua enabled.
Native sensory acquisition/history, roof-link navigation, paired door activation, object animation, and complete apparition/projectile/terror presentation remain incomplete.
Wolf's bell gesture plays without native absorption, and hidden-target camouflage currently switches visibility rather than fading materials.
See [BOSSES-e.md](BOSSES-e.md) for per-target limits and the missing object-animation exporter.

## Deathblow selection

Owl's phase-2 transition rejects deathblows for the lifetime of the TAE judgment marker 3506080, matching the user's real-game confirmation.
The native damage-motion suppression bit is traced and implemented, but the complete native throw-defence and ThrowKind condition dispatcher remains unreconstructed.
See [OWL.md](OWL.md#transition-deathblow-protection) for the data, native addresses and real-data regression tests.

## Throw walls and cinematic hand-overs

The shared, floor-aware throw capsule reservation and temporary wall-push bypass are inferred engine placeholders, not traced native behavior.
The bounded search can exhaust in cramped space, in which case unsafe paired displacement is held at the last safe feet.
Swept relocation and supporting-floor checks cover paired placement and actual throw recovery; the predictor still omits airborne velocity events and lethal defender branches, and the reservation does not cover full animated meshes.
See [DEATHBLOW.md](DEATHBLOW.md#alignment) for data-derived dummy placement and wall regressions.
Genichiro's native cutscene is replaced by a timed black fade with the event's MSB warp; synchronous model loading can extend the black hold.
See [GENICHIRO.md](GENICHIRO.md#hand-over-cinematic-placeholder-2026-10-11) for timing, input suppression and captures.

## Maps

The ten-map baseline and reviewed evidence are in [MAP-BASELINE.md](MAP-BASELINE.md).
Map transforms, base textures, collision and GPARAM sun/ambient/fog load from the installed data.
Connected-block layer samplers resolve from their owning GLB and exported block texture directory.
The render gate audits failed declared requests across resident neighbours independently of frontmost pixels and verifies equal material bindings and GPU readiness through each normal/all-visible pair.
The numeric gates and paired evidence are documented in [MAP-COMPLETENESS.md](MAP-COMPLETENESS.md#gates-and-outskirts-regression).
M[Multiple] and M[MultipleGround] now render their static-switch layers, RGB mask selectors, tint, detail normals, wetness and authored snow direction.
Remaining approximations include ground ripples, variant-18 metal masks, native AO, optional HDR-colour gamma conversion and texture-tiling semantics; other material families retain their existing paths.
Water lacks its original shader, and large lakes and underground water can appear as pale flat polygons.
GroundFog and CloudFog surfaces now retain their authored meshes, masks, scrolling constants, coverage and soft-depth intersections.
Their native forward shadow permutations, instance colour-slot overrides and forward volumetric-fog composition remain incomplete; MTD colour packing uses the sky renderer's calibrated gamma-two convention.
Irradiance uses the shared map atlas selection, default environment-probe approximation of native instance SH and hemisphere fallback.
The connected Castle surface diagnostic with probe/global fog disabled exposes a dark foreground card; native forward composition, instance lighting and per-mesh render-group semantics remain to be validated.
See [MAP-COMPLETENESS.md](MAP-COMPLETENESS.md#fog-surface-inventory-14-october-2026) for the material inventory and validation evidence.
Local light sets, full exposure/scattering, particles, animated foliage and event-controlled object visibility remain incomplete.
The collision decoder skips convex primitives; the decoded mesh supports every sampled gameplay start and short walk, but does not establish coverage of all collision shapes.
The m89 sky library contains co-located dome variants without terrain; sky shaders and variant selection remain incomplete, and play mode has no authored floor there.
Grappling, swimming and fall respawn are not part of the map walking baseline.

## NPC airborne support and interruption

The simulation reconstructs NPC airborne support from the lowest authored foot target relative to the clip's starting support, with an inferred 0.15 m contact tolerance.
A full-body interruption transfers that elevation to physics with zero initial vertical velocity, then uses the shipped reaction velocity events and fall/landing graph.
This is not a reconstruction of native animation-root physics or momentum transfer.
Category-1001 SpEffect replacement is traced, but positive durability/toughness meters for other NPC rows are not implemented.
Genichiro 3015 has no verified armour source in the audited data; its 8140/8141 aliases permit an aerial interruption, so unconditional protection for that act remains unresolved.
See [COMBAT-RULES.md](COMBAT-RULES.md#super-armour-and-interruption-2026-10-11) for native addresses and timed effects.

## Reservoir and Sunken Valley staging

Group-c staging uses the map's real event rows, MSB starts and collision, including fog and the independent silvergrass finishing boundary.
The silvergrass GPARAM sub-ID changes snap to extracted presets rather than blending over five seconds, and the 09 preset family selection is inferred rather than traced.
The brown ape is an event-visible supporting actor without simultaneous combat simulation.
Centipede dummy warps and animation are wired but cannot be visually verified until `cache/models/c5001.glb` exists; `c5100.glb` and `c5400.glb` also block graphical completion in this checkout.
Character-part state instructions, paired finishing player event animations, false-victory banners, custom event camera sub-IDs and native cutscene playback remain incomplete.
See [ARENAS-c.md](ARENAS-c.md) for exact story states, event IDs, missing assets and verification limits.

## Butterfly illusions and mounted Gyoubu continuation

Butterfly's eight illusions now use independent Lua/HKS combat against one shared Wolf frame, with the installed release pulse reaching the supporting cast.
Native event-relative warp points, pursuit homing, Snap Seed and summon recreation remain incomplete.
The scoped release replay observes two illusion actors falling below the hall floor later in the encounter, so successful contact/emission assertions do not establish a playable complete illusion wave.
Immediate-action hosting now lets all eight actors play the installed release clip and emit every damaging Bullet child family; pursuit parameters are still unread by the shared Bullet engine.
The second entity's descent now runs its original animation, AI enable state, resident-effect clear, region gate and request lifecycle, without the previous instruction-18 bypass.
Its event program counter and wait survive the hand-over, but exact native cancellation/placement and graphical descent fidelity remain unverified.
Gyoubu's two mounted bars run real Lua contacts, and an initial-placement/reduced-posture fixture verifies both throw families, HP refill and map defeat without holding the pair's positions each frame.
Unconstrained mounted throw approach and clearance remain approximate.
Gyoubu a000_003043 has inverted TAE 760 bounds 6 and 4; sequential lower/upper saturation avoids a panic, with upper-bound precedence unverified against the native handler.
See [BOSSES-a.md](BOSSES-a.md) for phase scope, exact native blockers and verification limits.

## Ape encounters

Guardian Ape's revival, separate headless continuation and terror buildup run from the installed events, throws, TAE and params.
The burrow's main and brown apes have independent Lua/HKS combat with map-assigned TARGET_EVENT state and one shared Wolf frame.
Nearest-opponent selection, shallow-water depth inferred from locomotion step height, and the existing bounded throw clearance remain approximations.
Full attack/counter coverage, selectable Loaded Spear and firecracker interactions, optional Mortal Blade severance and complete native terror/equipment modifiers remain incomplete.
See [BOSSES-b.md](BOSSES-b.md) for the tested phases, controlled fixtures and unresolved contact investigations.

## Castle-top camera obstruction (2026-10-12)

The reproduced obstruction is the hanging bamboo blind `m006672_4001`, using `M[Translucency]_e`, rather than the pavilion's structural wood alone.
At Wolf `(102.35,54,239.8)`, yaw 270 degrees and a locked-on passive boss 3 m ahead, its rendered triangles cross the camera sight line while the installed high-resolution collision has no hit within the camera's 4.8 m query.
The blind passes the current arena draw-group culling, has no MSB entity or entity-group IDs, and no independent interior hide toggle was identified; inspected pavilion event toggles switch intact/burnt story variants.
The nearby floor collision has MSB hit filter 8; camera-free special-setting rows exist in the params, but their triangle binding is not retained in `collision.bin`, so this fix does not reinterpret collision material IDs.
The lock-on camera lives in `crates/game/src/play.rs` and retains its existing ray push-in, including collision materials 22/36/38 being excluded.
The blind's MTD explicitly sets `g_GX_DisableCameraFade=false`, while the inspected `Wood2_multi_NoDF` sets it true.
The native translucency depth shader uses `FC_isEnableCameraDitherFade`, distance multiplied by `FC_CameraFadeInvRange` minus `FC_CameraFadeEnd`, and an 8x8 noise threshold scaled to 0.01..1.
Native translucency interpolates vertex-camera distance, whereas this shared material path evaluates distance per fragment, as the inspected native multiple-material branch does.
The map material now reads that native opt-out and dithers eligible map fragments near the camera, with actor materials and collision unchanged.
The exact CPU producer of the fade interval and native noise texture remain unrecovered: the provisional 0.3..0.6 m interval uses CameraParam row 0's authored `camCastSphereRadiusForFade=0.3`, and the noise uses ordered 8x8 coverage.
Inspected captures show [Genichiro before](../target/alt-fm/genichiro-blind-before.png) and [after](../target/alt-fm/genichiro-blind-after.png), plus [Owl before](../target/alt-fm/owl-blind-before.png) and [after](../target/alt-fm/owl-blind-after.png); Owl's before capture disables the new path with `--no-camera-fade`.
Reproduce with the following command, substituting `owl` for the shared arena check and adding `--no-camera-fade` for the control.

```powershell
target/alt-fm/release/sekiro-rs.exe --arena genichiro --spawn 102.35,54,239.8,270,0 --enemy-distance 3 --lock-on --passive --brain scripted --brain-memory off --checkpoint off --script crates/game/scripts/camera_ceiling_edge.txt --shots 900:target/alt-fm/ceiling-check.png --exit
```

## Brain move prediction

The attack gate uses nominal animated radial envelopes and constant observed Wolf velocity through the first active window.
It does not guarantee facing, height, future evasion or target-dependent TAE 760 scaling.
Projectile summaries use nominal launch speed and lifetime; accelerating, falling, interval and delayed-child projectiles still rely on the real bullet simulation for contact.
Guarded combo posture totals predict a breaking prefix only if the contacts are guarded; deflection, evasion and posture recovery can change the result.

Posture-break punish facts use nominal radial hit envelopes and constant-velocity projectile interception, rather than full facing, obstacle and target-dependent root-motion prediction.
Hit-stun timings are selected by the installed grounded Wolf HKS and read from TAE execution cancels without situational SpEffects; recorded punish outcomes use actual combat telemetry.

### Authored local lights, 2026-10-15

BTL type decoding, ancillary MapOffset placement, connected-bank loading and all four LightSet diffuse/specular slots are implemented; see [LIGHTING.md](LIGHTING.md#authored-map-lights-2026-10-15).
The stock Bevy local-light bridge still approximates native attenuation, source radius, spot aspect/edge sharpness, independent BTL specular response and flicker, and automatic nonzero BTL bank selection is untraced.
Native material/G-buffer gloss, Fresnel and SSAO routing also remain approximate in the recovered four-slot compositor.
Map FX emitters and their flame, smoke and transient light contributions remain a separate E1 task.

## Native map effects, 15 October 2026

Ashina map effects now use MSB SFX regions, ModelSfxParam dummy attachments, event-created object FX and registered-idol TAE through the shared FXR player.
Native weather action 10000, distortion 607 and effect point lights 609 are implemented with bounded pools and sleeping distant effects.
The two six-view gates retain every runtime-off geometry metric and visible-part set; the two reviewed grey-fixture updates affect colour only.
The GPU emission scheduler/shape details, native bonfire idle selection, particle lighting, four-sample soft-depth limitation and unsupported model/wind/spot-light appearances remain explicit approximations in MAP-FX.md.
Ongoing quest and arena-phase effect control beyond the first-visit snapshot remains open.
The data-fed presentation closes the complete absence of map snow, fire/smoke and idol effects in exploration, but does not establish full E1/E2 retail pixel parity or repair emissive-material fidelity.

## Painted horizon follow-up, 2026-10-15

The sky runtime now submits the native m89 FarMountain companion, including night `m900130`, independently of the nearer placed terrain.
Its recovered colour/haze shader, finite depth and alpha threshold are retained, alongside corrected fog sampling defaults and the fog cards' independent specular colour.
These data-backed changes do not establish D1/F1 visual parity: the final night range remains too faint, distant terrain too clear and the spot-011 CloudFog arc conspicuous.
The arc is traced to `m925009_9000`, not a moon or sky-dome texture; no authored card, density or scene exposure is removed or retuned to hide it.
Native per-instance fog-card ambient/SH and atmosphere composition remain investigation candidates rather than established causes of the residual.
See [BACKDROP.md](BACKDROP.md) for the recovered inputs, limitations and six reference comparisons.

## Grapple marker, issue #100

The extractor crops the five SB_FE wire sprites from the menu layout and texture archives to PNGs in `cache/menu/wire/`; the HUD loads only those cached sprites.
The XML layout gives sprite rectangles, without movie animation instructions, so distant acquisition uses an expanding radial reveal of MENU_Wire_Distance, starting at a central dot inside MENU_Wire_02.
MENU_Wire_01 frames the ready MENU_Wire_Marker disc; non-rideable targets show the ring with MENU_Wire_Cursor instead.
The approximately 49 px outer diameter at 1440p and dot size are fitted to the supplied retail captures; the native FE movie's reveal curve, glow and transition timing remain approximations.

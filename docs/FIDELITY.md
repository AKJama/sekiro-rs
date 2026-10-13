# Known fidelity approximations

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
Rendering uses a single albedo layer rather than the original layered moss, snow, blend masks and detail materials.
Water lacks its original shader, and large lakes and underground water can appear as pale flat polygons.
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

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
Once a foe has Wolf as its target it senses with the row's perceive cone instead (issue #216), as the exe's cone test `FUN_14061bb70` does when its sight context's +0x56 flag is set.
Both cones follow the exe's geometry, ported from FwLo's RustSekiro (`src/stealth.rs`, `in_cone`): the eye is the foe's feet pushed `eye_BackOffsetDist` behind along its facing (`FUN_14061af30`), and Wolf's feet must lie from `eye_BeginDist` to `eye_dist` plus his radius from it, inside the yaw and elevation limits.
For the Outskirts soldiers (back offset 4 m, perceive begin 3 m, +-60 degrees up and down) a fighting foe keeps Wolf in sight at arm's length and even about a metre behind it, where the old head-to-chest test lost him under about 1.5 m and dropped out of battle after 15 s.
Still not modelled: Wolf's SpEffect sight cuts (crouch, hanging, wall hug), the around cone and its caution meter, light level, and the exact sight-ray points (the ray still runs from 0.8 of the foe's height to 1 m above Wolf's feet).
The perceive flag is taken as "has a target", which is FwLo's reading of the exe's targeting flag 0x100000 (set in FIND and BATTLE); it was not traced further here.
Native caution accumulation, bush concealment, navigation and platoon roles remain incomplete; hearing is described under issue #218 below.
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

## Grapple target search, issue #178

The hook search follows the exe's `WireActionTargetMng` rule (see [MOBILITY.md](MOBILITY.md#46-how-the-game-picks-the-hook-issue-178)): camera back filter (75 degrees, 135 locked on), distance bands with each variation's extension added to both the usable and the shown distance, 0.8 m front plane, view box, survivors sorted usable first then nearest the screen centre with the sight ray cast down that order until one is clear, and soft lock.
These parts are approximations:

- `WireVariationParam` recognition angles are not applied, and `wire::Variation` does not read them: the game measures them against each edge's own direction, and `grapple.json` does not carry it.
  Most rows allow 90 or 180 degrees, so this mostly matters for the kite row 41 (90 up, 50 down, 30 left, 10 right) and rows 50200 to 50202.
  Reading them back needs the edge direction in the extracted points first.
- The hook is a point, not the game's 4 cm edge segment, so closest points and ranking use that point.
- The obstacle ray is our chest ray from 1.2 m above the feet; the exe's ray (0x140efba50) was not traced in detail.
- `RaycastElevationDeg` is taken as 0, its value when the config is not set; the elevation therefore does not tilt the check ray.
- The soft lock holds only a usable (Ready) hook; a hook that drops to Distant is searched for again, our reading of the "re-search when leaving the action range" switch.
- The search's camera is the simulation's own reconstruction of the follow camera, not the exe's camera matrix (character +0x1190).
  It orbits a focus `CameraDirector::focus_height` above Wolf's feet at `CameraDirector::distance`, at the step's `InputFrame::camera_yaw` and `camera_pitch`, pulled in front of walls by the same camera ray and margins as the camera on screen (`camera::wall_pulled_distance`).
  The camera on screen differs from it in two ways the simulation leaves out so that runs replay the same at any frame rate: its focus chases Wolf at the LockCamParam chase rates and is drawn between steps (overstep interpolation), and in deathblows it chases Wolf's camera dummy (TAE 151).
  The difference grows with Wolf's speed, since the focus on screen lags him, and is largest in deathblows; standing still the two cameras agree.
  The camera's yaw and pitch are themselves the orbit's last-frame values, as for the movement stick.

## Goods Storage and restock, issue #186

Goods over their EquipParamGoods `maxNum` go to the Storage only on `isDeposit` 1 rows, up to `maxRepositoryNum`; the rest is lost, as for the Healing Gourd and most key items.
Which goods Rest tops back up from the Storage is read from `autoReplenishType` (0 or 2 in the data): 2 is taken to mean "restock from the Storage up to the carry limit".
The exe's handling of that field is not read; 2 is also set on 44 key items (the Memories 5200-5322, the Sakes 9100-9105, the carp feeds 9180-9182 and the Treasure Carp Scales 10000) and on 21 `isDeposit` 0 rows such as the gourd 3000, which never reach the Storage.
A respawn after death restocks the goods and refills the gourd as Rest does.
No data or RE note covers it (FwLo's RustSekiro kb only records that Rest refills the gourd); it follows the game's well-known behaviour that dying refills consumables like resting.
A goods with no EquipParamGoods row, or with the params missing, is carried up to 99 but never stored or restocked.
The gourd's maximum stays at the start kit's 3 uses; Gourd Seeds are not modelled.
A Rest only moves what tops the carried count up (#202): the start kit carries the Pellets' `maxNum` of 3, so a Rest with none used leaves the Storage as it is, and one used moves exactly one.
Restocking again straight after (a Travel or a death after a Rest) moves nothing more.
The in-game menu's OK (E, Enter, Space, or a second click) on a consumable in Inventory uses one at once (#151), which a later Rest then refills from the Storage; the log shows both (`game menu: used goods`, `idol: rest: ...`).
Goods with `isFullSuppleItem` 1 that Wolf holds are set to their `maxNum` on Rest, a respawn after death and a Travel (#208), with no Storage involved, by FwLo's RustSekiro `refill_rested` rule.
The six such rows are the "[Replenishment]" goods, all `isDeposit` 0 with `autoReplenishType` 2: the Healing Gourd 3000, the prevention goods 3300, 3310 and 3320, Five-color Rice 3580 and goods 3800.
The gourd's row says `maxNum` 10, but its charges follow the kit's `gourd_max` (3) instead.
The start kit carries one goods 3800 of its three, so the first Rest raises it to three.
Each of the six has an empty twin at the next id (`isSuppleItem` 1, such as 3581) that the game shows once the last is used; the kit keeps a count of 0 instead.
The exe's own refill code is not read; the rule is FwLo's reading of the params.

## Travel between Sculptor's Idols, issue #148

The Travel list is BonfireWarpParam's: one page per `menuPageNo`, titled NTC_メニューテキスト 211100 + page, whose `<?placeName@n?>` tags resolve through 地名 n.
Each page lists its rows' idols in row id order, by `msgId` name, and only pages with a found idol show.
Page 0 ("Important Places", rows 100-102) repeats idols that also sit on their area pages, so an idol can be listed twice, as in the game.
Found idols are kept in `Kit::found_idols` by `bonfireEntityId`; an idol counts as found once Wolf has lit or rested at it.
The game records them by `eventflagId`; in the data the flag and the entity are one to one, so the entity stands in for the flag.
The kneel guide at an idol not yet found reads "Commune with Sculptor's Idol" (イベントテキスト 10010182) and at a found one "Rest" (15000120), #187; which text the game picks is its own event script's choice, not traced, so "found" stands in for the game's lit flag.
The kneel (#266) is the global wildcard `W_StatueAttachStart` of c0000's `Statue_SM`, so one press kneels from standing, walking, running, crouch-walking or a quick turn; Wolf's input is held only from the step after the press, so a walk's HKS update does not read a let-go stick and send its stop over the kneel.
A press while sprinting or stepping does nothing, as c0000.hks's `Control` turns the idol's action button on (`act(3034, ACTION_BUTTON_EXEC_TYPE_BASE)`) only in the stand and crouch states, not in the sprint or the step.
Standing up (#277) plays HKS StatueAttachEnd (a000_710330), whose TAE buffers presses from 0.4 s (flag 87) and opens every cancel window (move, step, jump, attack, guard and more) at 0.8 s, before its clip ends at 1.33 s.
Wolf's input is no longer held then, so the HKS takes a move on the first frame of that window; the idol menu's key hold (keys pressed while a screen is over the game wait for their release) ends as he stands up too.
The sim gates the press on the state's name (`Stand*`, `Crouch*`) rather than on that act call, and the exe's order of the engine's kneel event against the HKS's own events of the same frame is not traced: a step pressed in the same frame wins here.
Wolf arrives at the idol's warp point: the c0000 MSB player part whose entity is the idol's minus 980 (idol 1101951, warp point 1100971), facing that part's yaw.
All 58 rows have such a part, in the map the idol's entity names (`m{area}_{block}_00_00`), 0.9 to 7.2 m from the idol.
That spot is also where Wolf now respawns after death, replacing the earlier guess of a point 1.5 m out from the kneeling spot.
The warp plays HKS StatueAttachWarp (a000_710360) and moves Wolf at 2.0 s into it, FwLo's RustSekiro value (`WARP_CUT`); the screen fades to black over that time and lifts over 0.6 s, the death screen's lift.
A warp cut short (a hit or a death during it) lifts the black from the shade it had reached, not from full black.
Both ways Wolf leaves as a rest leaves him: full Vitality, the gourd refilled and the goods restocked, so a kit carried to another map is a rested one.
Travel within the play world is a teleport plus a respawn: world reset, enemies respawn, camera snaps behind Wolf.
That respawn ends no fight: a brain's fight in progress is closed as unfinished, never recorded as Wolf's win.
The arrival animation is not modelled; Wolf stands at the warp point.
There is no runtime map switching, so Travel to an idol on another map ends the run and starts the game again on that map (`--map`, `--at-idol`, `--kit` with the kit as JSON), the path the start menu's map changes take.
The new run keeps the session flags Back to start menu keeps (the window, cache, brain, enemies and render switches) and drops one-shot automation such as `--script`, `--input`, `--shots` and `--exit`.
It is checked before this run ends, as the start menu checks its runs: its options must parse and the map's layout must hold the idol and its warp point, or the idol is greyed and refused with the reason logged.
A partial `--kit` keeps the start kit's value for every field it leaves out.
The game streams the other map in the same run, with a load screen; ours shows the window closing and a fresh one loading.
Only kit state carries over: there are no saves yet, so story flags, opened doors and killed enemies of the run are not.
An idol whose map is not extracted is listed greyed and cannot be picked.
`grayoutEventflagId` greying, the page's area picture (`menuImageId`) and Wolf's marker in his area are not modelled.
The Travel screen opens on the idol Wolf prays at, its region and its row; after a Travel the idol menu opens on Rest again.
The idol menu and the Travel screen take keys, the pad and the mouse: the pointer over a row moves the cursor there and a click takes it, as in the in-game menu.
Their key guides show the keyboard glyphs whatever the device, as the other menus do.

## Rest effect at a Sculptor's Idol, issue #153

Rest plays Wolf's rest clip (HKS StatueAttachRest, a000_710340), whose TAE has no effect of its own but sends SetEzStateRequestID 20 at 0.2 s.
The idol object's TAE (o005000) has one clip with a one-shot effect beside its lit flame (1011): clip 100 spawns FFX 1022 (common effects) on dummy 100 at its start.
FFX 1022 is a flash, rings, sound 1020 and two black sprites, a 25 m disc round the idol and a small one, that rise over 0.5 s, hold to 2.0 s and are gone at 3.0 s (the small one tails off to 6.0 s).
We play FFX 1022 at the idol's origin 0.2 s after Rest is taken.
That request 20 makes the idol play clip 100 is inferred from the data (no other idol clip plays a one-shot, and the warp's request is 40), not traced in the exe.
The screen dark is the effect's own curve (the 25 m disc's, alpha up to its 0.99) on a black overlay over the HUD and under the idol menu; the game's own screen fade for the rest, if it has one beyond the effect, is not traced.
The world resets (Vitality and the gourd refilled, the goods restocked, the foes back) the step the dark is full, 0.7 s after Rest is taken, and the dark has lifted 3.2 s after it, before the rest clip (3.77 s of flags) returns Wolf to the prayer.
A rest cut short before the dark is full (Wolf taken off the idol) resets nothing, and the dark lifts from where it got to at the curve's own rate.
No real-game capture of a rest was available to compare the timing against.

## Idol flame once lit, issue #154

An idol's idle effect is the one its object's idle clip plays: clip 1 (unlit) spawns FFX 1010 and clip 11 (lit) FFX 1011 (1012 for the temple's o115040), both on dummy 100, the idol's origin.
FFX 1010 is embers that circle the idol 0.6 m out at 0.75 m up; FFX 1011 is the flame 1.1 m up, just above the hands of the 1.05 m idol.
The map's effects used to be fixed at the first-visit story state, so an idol Wolf lit in play kept its circling embers; now the lit idle replaces the unlit one the step the idol is found (`Kit::found_idols`).
Gap: the game's lighting clip 10 also plays FFX 1000 (a 1.3 s burst) as the idol lights; it is not played yet, and the idle swaps at once.
Gap: which clip the game's bonfire controller picks is inferred from the clip numbers and their effects, not traced in the exe (see [MAP-FX.md](MAP-FX.md)).

## Edge hold, issue #203

The fall-prevention assist follows the exe's groups (see [OBJECT-COLLISION.md](OBJECT-COLLISION.md#the-fall-prevention-assist)): walking is held only by filter-28 walls and type 6 objects (group 92), and the `f` binder's bands (group 93) hold only ground actions and NPCs.
These parts are approximations or unchecked:

- Before #203 the bands held walking too, and the user's playtest of that build was taken as the real game's behaviour; whether the real game lets a walk off the roof by start 0 or the start 3 terrace was not checked in the real game.
- The cast is the simulation's sub-stepped slide with a 0.16 m hold, not the exe's shape cast (`FUN_140bc68d0`), and it does not apply the exe's 90 degree angle limit.
- `StandMoveStart` and `GroundStep` send only the walking request in the simulation, though their `g_paramHkbState` rows set both flags, so a walk's start or a step is not held by a band; the cause is not traced.

## Riding a round trunk, issue #155

The first pine past the gate at start 0 (`o002500_0005`) let Wolf walk off its bend and its tip's south side, because the body stood on the floor under its centre as if on a point foot.
Its walls (`o002501_0005`) lean out over the trunk's rounded flanks: the walls held his axis, but the feet followed the flank down and he dropped with the crown still under his capsule's rim, or dropped below the outward-leaning wall and walked out behind it.
The body now rests its rounded bottom on the highest floor its rim reaches, up to a step above the feet, 0.2 m below that floor (`Body::rim_support`, `Body::rounded_step_down`).
A rim floor above the feet counts only on a rideable object (material 8xx); a higher map floor at the rim is a ledge whose edge is a wall, and counting it held Wolf in mid-air beside the snake pit's ledge (`snake_pit_hug_exit_rests_on_the_lip`).
Other round surfaces that are not rideable still hold only at or below the feet.
This is the simulation's capsule shape, not a trace of the exe's ground probe, which is not traced.
The bounded step down also applies only on a rideable object, so map stairs and ledges step down exactly as before; measured walking and running down the castle's long stairs from start 3 and off 0.25 to 0.5 m ledges, map floors match the old code tick for tick.
On a rideable object a step off an edge keeps the feet 0.2 m below its top while the capsule's rim is over it: 3 ticks at a run, up to 10 at a slow walk.
A sweep of walks out of every Outskirts pine's top surface in 8 directions (7748 walks that leave through a wall) found 380 that walked through on main and 245 with this change; `o002500_0005` went from 25 to 0, and 12 trees still let some through (`o002510_0001` 51, `o002530_0001` 38, `o002510_0004` 33, `o002530_0000` 29, `o002500_0003` 27, `o002510_0002` 22, `o002500_0007` 16, `o002530_0008` 16, and four more with 6 or fewer).
Those remaining leaks are not investigated.

## Patrol walking and arrival waits, issue #116

Patrolling foes walk their PatrolInfo route and play its arrival waits and animations because two Lua goal methods now follow the exe.
`goal:GetLastResult` reads the result last stored in the goal's pool slot: the exe builds 32 goals once, AddGoal takes the first free slot and never resets the stored result, so a fresh NonBattleAct sees Success from the move that just ended in that slot.
The pool hands out the lowest free slot, which our allocator also does: `FUN_1405ade50` scans the 32 slots from 0 for the first whose +0xa0 is -1 (`FUN_1405cff90`), and freeing a goal (`FUN_1405cedd0`) sets it back to -1.
The slot list here grows on demand instead of being fixed at 32 entries, and a slot that has never updated reads -1, as the exe's goal constructor (`FUN_1405cd180`) sets it.
Our slot numbers count only the goals of one AI, from its top goal at slot 0; whether the exe's pool is per AI with the top goal in its first slot was not checked.
`goal:GetParam` returns a Lua boolean as a number, true as 1 and false as 0, because the exe stores goal parameters as floats; NonBattleAct's walk check is `GetParam(2) == 0`.
The exact boolean-to-float rule is assumed from that float storage and was not traced in the exe.
Native goals read a flag parameter as the exe does, set when it is above 0: ApproachTarget's walk flag (param 3, `FUN_1405c7e80`) and KeepDist's (param 4, `FUN_1405c7c40`) are passed to the move request `FUN_1405e6640` as `0.0 < param`.
MoveToSomewhere, LeaveTarget and SidewayMove's native updates were not identified; their walk flags are assumed to be read the same way.
So a numeric walk flag of 0 from an approach script (`Approach_Act_Flex`) or the brain's run tactics (RunIn, Retreat) now runs where it walked before, which makes bosses run on far approaches.
The brain's reach gate, which holds a planned attack whose target moved out of reach, now also covers the CommonAttack a planned table goal such as ComboAttackTunableSpin adds.
No AI script calls `StepNextMovePoint`, so patrol stepping happens only in the move goal on arrival at point 103, as in the exe's MoveTo terminate callbacks and FwLo's `src/ai.rs`.
A test (`no_ai_script_steps_the_patrol_itself`) checks that no shipped AI script calls it, so the route cannot step twice per point (#114).
The exe's MoveToSomewhere arrives by one rule for every target: closer than the AI's own radius plus the goal's distance, strictly (IsArrived `FUN_1405b9070`, read by the move update `FUN_1405caa00`, as FwLo's `src/ai.rs` traced it).
The exe's GetDist subtracts the same radius (FwLo: the GetDist thunk `0x1405e2c10`), so the scripts' distances and the arrival agree.
Our GetDist is centre to centre (apart from the Demon's `dist_offset`), so here the arrival adds the radius only for the patrol point (103), whose distance no script reads, and stays in GetDist's centre terms for every other target; both are strict (#114).
Adding the radius for every target alone made approaches stop a body radius short of the bands the brain and scripts then test with GetDist (`brain_tactics::walk_in_and_run_in_close_to_their_band`, `projectile_guard::owl_shuriken_deflect_keeps_throwing` failed).
Open question: moving both GetDist and the arrival to the exe's radius-based distance together, which changes every script's distance checks.
The distance is read on the ground plane; the exe's 2D or 3D switch is not mapped, as in FwLo's port.
The exe's second patrol test, standing inside a shaped point region (IsInsideMovePoint), is not modelled, so a foe on a shaped point still walks to its centre.

## Standing guards and small acts, issue #115

Map foes with no patrol route stand at their post in NonBattleAct's `Stay` because `GetSmallActAnimId` now returns -1, as the exe does when the MSB enemy has no small act.
The exe reads the enemy part's small-act index (its `init_anim_id_type`, part data +0x22) and returns -1 when it is negative (`FUN_140604450` over `FUN_14060c1b0`), and the pre and post waits return 0 (`FUN_1406044d0`).
Our unknown-method default returned 0, so NonBattleAct queued a 20 s `WaitWithAnime` of anim 0, an AttackNonCancel of the idle the foe was already playing, and repeated it.
184 of the 188 Outskirts enemy parts have no small act; the other four (indices 0, 1 and 2) point at rows of a table the exe reads by index 0x14 from a per-character resource (`FUN_141094ff0`), which is not identified, so they also read -1 here, as in FwLo's `src/ai.rs`.
`Stay(life, 0, POINT_INIT_POSE)` faces its third argument (goal parameter 1), as FwLo's `Native::Stay` reads it, not parameter 0 as `Wait` does.
POINT_INIT_POSE is the spawn point here, not the spawn facing, and an NPC in its idle state does not turn in place (its TAE turn speed is 0), so a guard moved off its facing does not turn back to it.
Hearing and the caution state are described under issue #218 below.

## Walking home after losing Wolf, issue #217

A map foe that lost Wolf away from its post walks back to it, because `GetMovePointEffectRange` now follows the exe and `BackToHome` walks.
NonBattleAct compares `GetMovePointEffectRange` with its walk-home distance (goal parameter 4, 1 when it is 0) and queues `BackToHome_With_Parry` when the range is larger; the stub returned 0, so a foe stood wherever it lost Wolf.
The binding (`FUN_1405e38f0`) reads AI +0x9084, which the route manager at AI +0x9060 fills with `FUN_1405afea0` (via `FUN_1405afe80`): the 3D distance from the foe to its move point, taken from `FUN_1405b04d0`.
That is the nearest point of the segment from the current route point to the previous one, or from the current point to the initial position (POINT_INITIAL, 100) before the first step (`FUN_1419ef320`, clamped to the segment's ends).
With no current point it is the distance to the last point of an ended route, or to the initial position for a foe with no route.
The exe's first case, a segment of an active event move area (AI +0x9160, `FUN_140630550`), is not modelled.
FwLo's `src/ai.rs` uses the flat distance from home there and marks it a gap.
`BackToHome` (goal 4) used to run as MoveToSomewhere towards its parameter 0, which `BackToHome_With_Parry` sets to TARGET_ENE_0, so with Wolf lost it did not move.
It now walks to the initial position and succeeds within 1 m, FwLo's rule from `src/ai.rs`; the exe's arrival radius and its path following are not traced.
The 1 m is measured in 3D, as `GetMovePointEffectRange` measures it: on the ground plane a foe could stop just inside 1 m while a step in the floor put it just over 1 m in 3D, and NonBattleAct then sent it home again on every tick, so it stood without a goal.

## Hearing and caution, issue #218

Map foes hear the AI sounds Wolf's clips make and turn to caution, as the shipped logic scripts expect.
Before, `GetLatestSoundTargetID` and `GetCurrTargetType` read 0 and `ClearSoundTarget` did nothing, so a guard reacted only to sight, `nose_dist` and damage.
Wolf's clips emit sounds with TAE event 237 (`CreateAISound2`, field `AISoundID`), rows of AiSoundParam: walk 1000 (0.5 m), crouched walk 1001 (0.25 m), run 1010 (2 m), crouched run 1011 (0.5 m), sprint 1020 and step 1100 (4 m), jump 1300 and landing 1310 (2 m), and his attacks and hits (2000 on, rank 1 "important").
A sound is heard when the gap between the two bodies is under its reach plus the listener's `ear_dist`, the reach being the radius times the listener's rate minus `ear_soundcut_dist`, and nothing at all when that is not positive.
That is the exe's listener (`SprjAiSoundNormalListener`, `FUN_140610470` over `FUN_140610b30` and `FUN_140610a30`), as FwLo's RustSekiro ports it (`src/enemy.rs`, `wolf_sounds`); his values and rule are used unchanged.
The listener's rate is the product of `1 - hearingSearchEnemyCut / 100` over its SpEffects (`FUN_140bfe590`), and a row with `bSpEffectEnable` scales the radius by the maker's `hearingSearchEnemyRate` (Covert B, 150010, halves it).
Gaps: the exe multiplies `ear_dist` by a listener factor at +0x20, taken as 1 here; the exe's body radii (listener vfunc +0x50, maker `FUN_1409efb80`) are our capsule radii; the sound's 0.25 s `lifeFrame` is the clip's event window.
For the Outskirts soldier c1010 (ear_dist 0.1, no cut) this means a sprint or step is heard about 4.9 m between centres, a run 2.9 m, and a crouched run 1.4 m.
A run 3.6 m off a guard's side is not heard; the exe's ranges make only a sprint or a step carry that far.
The heard sound becomes the foe's sound target for `SoundTargetForgetTime` (150 s for c1010); a held rank 1 sound is kept over a rank 0 one, and a new sound id raises `INTERUPT_ChangeSoundTarget` (46), FwLo's `Targeting::hear` (`src/stealth.rs`).
The exe compares a record serial for "new"; the id stands in for it here, so a footstep refreshed every step is one sound, as in FwLo's port.
An unaware foe with a sound target is in CAUTION (1), with none in NONE (0); a change replans unless the think row's `goalAction_ToCaution` is 0 (`FUN_1406208c0`, `FUN_14061f500`, via FwLo).
`GetCurrTargetType` answers 3 (normal enemy) with a seen target, 4 (sound) with a sound target and 0 otherwise; the indication, memory and corpse target slots are not modelled.
In caution the scripts play the soldier's alert anim 1010 once (`_COMMON_AddStateTransitionGoal`), then search with `_COMMON_AddCautionAndFindGoal`: walk to the sound and `ConfirmCautionTarget` (5100).
Anim 1010 adds SpEffect 200001 (non-combat caution) in its TAE, and that mark is what keeps the scripts from replaying it: the alert anims' AI-state SpEffects (200000, 200001, 200002, 200004, effectEndurance -1) now stay on the AI until another replaces them, FwLo's `ai_sp` in `src/enemy.rs`.
Before an anim sets one the AI has none, where the exe starts on 200000.
`ConfirmCautionTarget` follows FwLo's trace of `FUN_1405c35a0`: CommonAttack of its anim (when set), then Wait for its time; its Terminate (`FUN_1405c3770`) clears the sound target while the foe is still in caution and heard nothing new that step, so it goes back to NonBattleAct and, with the 200001 mark, plays 101000 and walks home.
Not modelled: the around cone and caution meter, indication targets, the corpse and memory slots, and sounds made by other characters (foes do not hear each other or their own attacks).


The slide's push-out (`CollisionWorld::push_out` in `crates/sim/src/collision.rs`) is the simulation's own rule, not the exe's Havok character proxy, which was not traced.
It takes the shortest horizontal push that clears every contact of both body spheres at once, so the triangles' order does not matter.
When no push of at most a body radius does that, the body is wedged (for example the fence beside the m11_00 gate and its cap beam's sloping underside): the sub-step is taken back and only its part along the touched faces is kept, so the body slides along the wedge but never into it and is never pushed through a face.
A body that was already wedged before it moved (placed by a respawn, a warp or a moving part) also gets a push out, at most a body radius a sub-step: the shortest one that clears every contact when there is one, else the one that leaves the contacts equally deep, cut short so it never passes half way to any touched triangle.
Whether the real game stops, slides or squeezes a wedged body differently was not checked.

## Head against overhangs, and landing by side of approach, issue #168

The game's Havok character proxy keeps the whole capsule out of the mesh, and its contact normals point from the surface to the capsule, so the underside of a roof is only ever a ceiling to it.
The simulation still moves the body horizontally (the slide) and vertically (the rise and the fall) as separate steps; these rules are our own, built on the slide's narrow phase, not traced from the exe.
- Rise: the head sphere is swept straight up against every flat face over it, interior, edges and corners (`first_touch`), and a touch on the sphere's cap (within 39 degrees of straight up, `CEILING_UP`) stops the rise (#160, #170).
  A touch further out, on the head's shoulder, does not stop it: when a face over the head sphere's centre touches it after the rise, the body takes the slide's own push-out at the height it rose to, so it is pushed off the edge sideways in the same tick, as the proxy slides along such a contact.
  A body wedged at that height (a wall at its legs, an edge at its head) takes the #176 wedge rule, so it is never pushed through the wall, but its head can stay up to a tick's rise into the edge.
- Wire flight: besides the rays along the body's centre line, the head sphere is swept along each tick's move against every solid face near it, and a touch on its top half (the face's closest point above the sphere's centre) ends the flight there, as a ray hit does.
  A touch on the head's side or below is still left to the rays, so a flight brushing a wall beside the centre line is not stopped by it, and neither is a face the move does not close on, such as an eave the head grazes as a flight leaves it.
- Landing: the winding of the collision mesh does not say which side of a face is solid, so the side is the one the feet came from.
  An airborne body lands on the highest floor up to a step above its feet unless a floor was already over its feet, within a step, where the tick began, and it had not been carried onto that floor over its edge on an earlier tick (`Body::carried_onto`); such a floor was come at from below and is skipped for the highest floor the feet passed through.
  Whether the game then pushes the body down out of a slab it is inside, or out to the side, was not checked; here it falls.

## Foes keep to the navmesh while they walk, issue #227

A map foe's walk and run states ask for no fall-prevention assist in `c9997.hks` (`FallPreventionAssist` is called from attacks, steps, damage, throws and other actions, never from the `Walk*` and `Run*` states), so the `f` bands at an edge do not hold a walking foe.
What keeps it off a drop in the game is its navigation mesh, which `crates/sim/src/navmesh.rs` now loads from `map/<map>/<map>.nvmhktbnd` (one `hkaiNavMesh` per file, placed by the MSB's map offset event like the map's collision parts).
A grounded foe on the mesh whose state asks for no assist has its move cut where it would leave the mesh, and the rest of the move kept along the face edge it met, so it stops at a drop's edge and slides along it.
A move that only crosses a hole in the mesh (the mesh carries on within 2 m past the move's end, along it) is let through, standing in for the path the game would take round the rocks, posts and bushes the mesh leaves out.
This is the simulation's rule, not the exe's: the AI's path search and path following over the mesh (Havok AI), the mesh's user edges between pieces, the object navmeshes (`onav`), the height tolerance (1 m here) and the hole width (2 m) were not traced.
A foe whose target is off the mesh walks in place at the edge, which matches what players see foes do in FromSoftware games, but was not checked against a recording; the Ochimusha's ranged acts when it has no path (the Lua's navmesh queries still answer open floor) are not modelled.
Foes knocked off the mesh, and attacks and steps (held by the `f` bands as before), move freely; a foe that falls off dies of the fall (#234, below).

## Foes die of a fall, issue #234

The exe runs one fall-dead routine for every character (`FUN_140b975f0`, logged as `SprjPlaylog_ChrFallDead`), and for a character that is not a player it always kills: HP to 0 through the HP setter (so a foe that cannot die, or has deathblows left, keeps 1 HP), damage type 6, and the NPC death flags (`FUN_1409ee7e0`).
It runs when the character meets a filter-15 event wall (the ravine's `h900000`), which also fades the model out over 0.5 s and then holds the body, and when a fall goes on too long: 30 s airborne or 1500 m below where it last stood (`FUN_140bc2210`, with no fade).
`crates/sim/src/npc_fall.rs` applies both to the enemy of every duel, map foes included.
In the #227 run the Outskirts soldier c1010_0000 went off the ravine edge in an attack (a000_003012 in the sim's repro): the edge has an `f` band (part `h000300`, about z -57.3 there), but the soldier had already run onto it (runs ask for no assist), so the attack's assist started past the band and did not hold it.
Not modelled: the damage type 6 is not handed to the foe's script (its `IsDead` reads HP), the exe's third trigger (physics `+0x8e8`, set after 10 s airborne with the height changing by at most 0.02 m a step) is left out, and the renderer has no model opacity, so the fade is not drawn and the foe is hidden once it ends.
The script's mid-air death (`ExecDeath` plays `W_DeathStartDefault` once env 224 passes `HEIGHT_FORCE_DEATH`, 60 m) does not play: the simulation's env 224 is the last landed fall's height, while the exe's grows during the fall.

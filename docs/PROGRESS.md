# Progress

This is a high-level view of how much of Sekiro is reimplemented.
"Done" means implemented and tested against the game data, not perfect; known gaps are in [STATUS.md](../STATUS.md) and the docs linked for each area.

## At a glance

| Area | Status | Progress |
| --- | --- | --- |
| Game data and formats | Partial | `#########-` 90% |
| Player movement | Partial | `########--` 80% |
| Combat rules | Partial | `########--` 75% |
| Deathblows and death | Partial | `#######---` 70% |
| Enemies and bosses | Partial | 102 placed types load; soldier, Owl and castle Genichiro fights verified |
| Enemy AI | Partial | `#######---` 70% |
| Maps and world | Partial | All 10 maps extracted; 9 gameplay maps walkable at baseline |
| Audio | Partial | `###-------` 30% |
| Rendering | Partial | `#######---` 70% |
| Interface and game flow | Partial | Start/pause menus and combat HUD; inventory, checkpoints and story flow pending |

## Game data and formats

| Item | Status | Notes |
| --- | --- | --- |
| Archive unpacking | Done | Encrypted headers and compressed files; whole install cached in under a minute. |
| Params | Done | All 186 tables load from Paramdex definitions. |
| Models, textures, materials | Done | FLVER/TPF/MTD to GLB for characters and map pieces. |
| Animation and TAE timelines | Done | HKX spline compressed animation. |
| Behaviour graph and HKS player script VM | Done | Havok hkb graph. |
| Map layout and collision | Done | MSB layout and hknp collision. |
| Lua 5.0 AI scripts | Done | Own interpreter. |
| Sound banks and event files | Partial | FSB5 and FEV; 112 of 174 banks decode, 62 encrypted. |
| Ledge and wall files | Partial | `.edge` parsed and placed map records extracted; `.wall` remains undecoded. |

Docs: [FORMATS.md](FORMATS.md), [PARAMS.md](PARAMS.md).

## Player movement

| Item | Status | Notes |
| --- | --- | --- |
| Walk, run, sprint, step, crouch | Done | From the real script and root motion. |
| Jump | Done | Traced gravity: 23.52 m/s2. |
| Lock-on and camera | Done | From LockCamParam/CameraParam. |
| Input buffer | Done | Uses the engine's accept/execute flags. |
| Air control and falls | Done | Per-type fall table, stick steering and braking, fall heights in cm, material fall death, void falls return to safe ground. |
| Upper/lower body animation layering | Not started | One clip shown at a time. |
| Grappling hook | Done | Solved arrival arcs, ground, air and chained shots, landing and perch, target ring; the rope is a line stand-in. |
| Ledge hang | Partial | Ground grab, idle, climb up and drop for variation 0; real-data replay and synthetic HKS tests; docking thresholds remain placeholders. |
| Wall hug | Done | Kind-3 entry, idle, root-distance sliding, endpoint peek and action/away-stick exit through HKS; docking placement and search thresholds remain approximations. |
| Swimming and diving | Done | Collision water surfaces, HKS swimming/diving/up/down, water landings, dolphin surfacing and shore exit; native water rendering/immersion, volumes and camera remain gaps. |
| Climbing | Not started | Ladder/cling data remains undecoded. |

Docs: [HKS-COMBAT.md](HKS-COMBAT.md), [FORMATS.md](FORMATS.md).

## Combat rules

| Item | Status | Notes |
| --- | --- | --- |
| Hits | Done | From weapon dummies and attack capsules. |
| Block, deflect, posture damage both ways, posture recovery | Done | Formulas from the exe. |
| Guard arc, invulnerability windows, hit stop, knockback | Done | |
| Attack lunge and auto-aim | Done | |
| HP damage | Partial | Formula inferred, not traced. |
| Perilous attacks, Mikiri counter, sweep counter | Done | |
| Super armour | Done | TAE damage-motion windows (category 1001 SpEffects); airborne interruption falls and lands. |
| Projectile guard and deflect | Done | Resolved against the bullet, no recoil for the shooter. |
| Combat arts, prosthetic tools, items (gourd) | Not started | |
| Status effects | Partial | Owl's poison, seal and blinding; Genichiro's electrified and Lightning Reversal; burn and terror not yet. |

Docs: [COMBAT-RULES.md](COMBAT-RULES.md), [HKS-COMBAT.md](HKS-COMBAT.md).

## Deathblows and death

| Item | Status | Notes |
| --- | --- | --- |
| Posture-break deathblow | Done | Real throw pairs, placed from ThrowParam dummies, kept clear of walls. |
| Multi-mark bosses and phases | Done for the Owl and Genichiro | Map event hand-overs; Way of Tomoe behind a fade. |
| Wolf death and respawn | Done | |
| Deathblow from behind, plunge, stealth | Not started | |
| Resurrection | Not started | |

Docs: [DEATHBLOW.md](DEATHBLOW.md).

## Enemies and bosses

102 placed character types load from data across 1754 placements; combat is verified for the soldier, Owl and castle Genichiro.

| Item | Status | Notes |
| --- | --- | --- |
| Ashina soldier (c1010) | Done | Own HKS script and real Lua AI. |
| Great Shinobi Owl (c5060) | Done | Lua, tools, two deathblows and map phase events, boss brain. |
| Castle Genichiro (c7100, c7110) | Done | Rooftop arena, arrows, two deathblows, Way of Tomoe with lightning and Lightning Reversal, boss brain through all three bars. |
| Placed roster | Loads | 102/102 resource smoke; donor and animation-set support; combat fidelity needs per-enemy verification. |

Docs: [AI.md](AI.md), [OWL.md](OWL.md), [GENICHIRO.md](GENICHIRO.md).

## Enemy AI

| Item | Status | Notes |
| --- | --- | --- |
| Lua 5.0 VM and goal runtime | Done | Runs the game's battle scripts. |
| Parry timing | Done | From the attack flag (TAE flag 63). |
| Stealth, perception, patrols, group behaviour | Not started | |
| Deflect counters | Done | Lua decides when; the boss brain can re-weight the offered counters. |
| Model-driven boss brain | Done for the Owl, Genichiro and the soldier | Coach, checkpoint chooser, memory across sessions, per-player profiles, overlay; see [BRAIN.md](BRAIN.md). |

Docs: [AI.md](AI.md).

## Maps and world

All ten archive maps extract and load; all 69 player starts across the nine gameplay maps have collision floors.
The tenth map is a sky-dome library without terrain.
Detailed measurements, reviewed screenshots, visual limitations and verification are in [MAP-BASELINE.md](MAP-BASELINE.md).

| Item | Status | Notes |
| --- | --- | --- |
| m10_00_00_00 (Hirata Estate) | Baseline | Walkable; water, foliage and material layers approximate. |
| m11_00_00_00 (Ashina Outskirts) | Baseline | Walkable; material layers approximate. |
| m11_01_00_00 (Ashina Castle) | Baseline | Walkable; initial viewer camera obstruction fixed and verified. |
| m11_02_00_00 (Ashina Reservoir) | Baseline | Walkable; material layers approximate. |
| m13_00_00_00 (Abandoned Dungeon) | Baseline | Walkable; pale flat water. |
| m15_00_00_00 (Mibu Village / Ashina Depths) | Baseline | Walkable; water and foliage approximate. |
| m17_00_00_00 (Sunken Valley / Sunken Valley Passage) | Baseline | Walkable; conspicuous snow/foliage layers. |
| m20_00_00_00 (Senpou Temple, Mt. Kongo) | Baseline | Walkable; foliage and material layers approximate. |
| m25_00_00_00 (Fountainhead Palace) | Baseline | Walkable; use start 1, lake shader approximate. |
| m89_00_00_99 (sky-dome library) | Inspection only | Loads after GPARAM/group fixes; no authored collision, cannot be walked. |
| Ledges, grapple points, cover edges, lights | Done | Extracted for all maps. |
| Placed enemies, objects, doors, ladders, events (EMEVD) | Started | Objects extracted, event visibility incomplete; Owl fight events run. |

Docs: [FORMATS.md (Maps)](FORMATS.md#maps-msb-layouts-map-pieces-and-hit-collision).

## Rendering

| Item | Status | Notes |
| --- | --- | --- |
| Fog | Done | The game's volumetric fog model from its shaders, with exe-traced inputs. |
| Exposure, tone mapping, bloom, colour grading | Done | The game's own post chain from its shaders. |
| Light probes and bounce light | Done | GILM probes for specular, GIIV irradiance volumes for ambient. |
| Point lights | Done | From the map's BTL files. |
| Sky | Done | Authored sky domes per map and hour. |
| Layered map materials, foliage cut-outs | Done | |
| Shadows | Done | Two cascades with a caster budget. |
| Arena far scenery | Done | Rendered to 1.5 km, collision cropped. |
| Particle effects (FXR) | Partial | Reader for all effect files; billboard playback; combat effects are stand-ins. |
| Character materials, cloth, water, wind | Not started | |

Docs: [VISUALS-PLAN.md](VISUALS-PLAN.md), [SHADERS.md](SHADERS.md), [SHADER-INPUTS.md](SHADER-INPUTS.md).

## Audio

| Item | Status | Notes |
| --- | --- | --- |
| Plain sound banks and TAE sound cues | Done | Through the game's own FMOD runtime. |
| Encrypted banks | Not started | Footsteps, voices, music. |
| 3D positioning and mixing | Partial | Listener on the camera with the events' own rolloff; no reverb or boss music yet. |

Docs: [AUDIO.md](AUDIO.md).

## Interface and game flow

| Item | Status | Notes |
| --- | --- | --- |
| HP and posture bars | Done | |
| Start and pause menus | Done | Fight/place/AI/stage/AP/hour/player choices, remembered preferences; restart through the existing CLI setup on Start and Back |
| Inventory, save files, checkpoints (idols), cutscenes, dialogue | Not started | |

Docs: `docs/INTERFACE.md`.

## How to update this file

Update the row when a feature lands.
Percentages are rough judgements except where a count is given.

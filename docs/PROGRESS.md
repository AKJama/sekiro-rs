# Progress

This is a high-level view of how much of Sekiro is reimplemented.
"Done" means implemented and tested against the game data, not perfect; known gaps are in [STATUS.md](../STATUS.md) and the docs linked for each area.

## At a glance

| Area | Status | Progress |
| --- | --- | --- |
| Game data and formats | Partial | `#########-` 90% |
| Player movement | Partial | `######----` 60% |
| Combat rules | Partial | `#######---` 65% |
| Deathblows and death | Partial | `######----` 60% |
| Enemies and bosses | Partial | `#---------` ~5% |
| Enemy AI | Partial | `#####-----` 50% |
| Maps and world | Partial | All 10 maps extracted; 9 gameplay maps walkable at baseline |
| Audio | Partial | `###-------` 30% |
| Interface and game flow | Partial | `#---------` 10% |

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
| Ledge and wall files | Partial | `.edge` ledge format decoded, not used yet. |

Docs: [FORMATS.md](FORMATS.md), [PARAMS.md](PARAMS.md).

## Player movement

| Item | Status | Notes |
| --- | --- | --- |
| Walk, run, sprint, step, crouch | Done | From the real script and root motion. |
| Jump | Done | Traced gravity: 23.52 m/s2. |
| Lock-on and camera | Done | From LockCamParam/CameraParam. |
| Input buffer | Done | Uses the engine's accept/execute flags. |
| Air control | Not started | From the fall-control table. |
| Upper/lower body animation layering | Not started | One clip shown at a time. |
| Grappling hook | Not started | Grapple points found in data. |
| Ledge hang, wall hug, swimming, climbing | Not started | |

Docs: [HKS-COMBAT.md](HKS-COMBAT.md), [FORMATS.md](FORMATS.md).

## Combat rules

| Item | Status | Notes |
| --- | --- | --- |
| Hits | Done | From weapon dummies and attack capsules. |
| Block, deflect, posture damage both ways, posture recovery | Done | Formulas from the exe. |
| Guard arc, invulnerability windows, hit stop, knockback | Done | |
| Attack lunge and auto-aim | Done | |
| HP damage | Partial | Formula inferred, not traced. |
| Perilous attacks and Mikiri counter | Not started | |
| Combat arts, prosthetic tools, items (gourd) | Not started | |
| Status effects | Not started | Burn, poison, terror. |

Docs: [COMBAT-RULES.md](COMBAT-RULES.md), [HKS-COMBAT.md](HKS-COMBAT.md).

## Deathblows and death

| Item | Status | Notes |
| --- | --- | --- |
| Posture-break deathblow | Done | Real throw pairs. |
| Wolf death and respawn | Done | |
| Multi-life bosses, deathblow from behind, plunge, stealth | Not started | |
| Resurrection | Not started | |

Docs: [DEATHBLOW.md](DEATHBLOW.md).

## Enemies and bosses

1 of about 128 character models is working (~5%).

| Item | Status | Notes |
| --- | --- | --- |
| Ashina soldier (c1010) | Done | Own HKS script and real Lua AI. |
| Great Shinobi Owl (c5070) | Partial | In progress. |
| All other enemies and bosses | Not started | The same pipeline should load them. |

Docs: [AI.md](AI.md), [OWL.md](OWL.md) (being written).

## Enemy AI

| Item | Status | Notes |
| --- | --- | --- |
| Lua 5.0 VM and goal runtime | Done | Runs the game's battle scripts. |
| Parry timing | Done | From the attack flag (TAE flag 63). |
| Stealth, perception, patrols, group behaviour | Not started | |
| Model-driven boss brain | Not started | The project's research goal. |

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
| Map lighting from draw params, foliage, fog | Partial | GPARAM sun, ambient and fog load per map; local lights and shader fidelity remain. |
| Placed enemies, objects, doors, ladders, events (EMEVD) | Started | Objects extracted, event visibility incomplete; Owl fight events run. |

Docs: [FORMATS.md (Maps)](FORMATS.md#maps-msb-layouts-map-pieces-and-hit-collision).

## Audio

| Item | Status | Notes |
| --- | --- | --- |
| Plain sound banks and TAE sound cues | Done | |
| Encrypted banks | Not started | Footsteps, voices, music. |
| 3D positioning and mixing | Not started | |

Docs: [AUDIO.md](AUDIO.md).

## Interface and game flow

| Item | Status | Notes |
| --- | --- | --- |
| HP and posture bars | Done | |
| Menus, inventory, save files, checkpoints (idols), cutscenes, dialogue | Not started | |

Docs: None yet.

## How to update this file

Update the row when a feature lands.
Percentages are rough judgements except where a count is given.

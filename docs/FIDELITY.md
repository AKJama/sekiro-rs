# Known fidelity approximations

## Deathblow selection

Owl's phase-2 transition rejects deathblows for the lifetime of the TAE judgment marker 3506080, matching the user's real-game confirmation.
The native damage-motion suppression bit is traced and implemented, but the complete native throw-defence and ThrowKind condition dispatcher remains unreconstructed.
See [OWL.md](OWL.md#transition-deathblow-protection) for the data, native addresses and real-data regression tests.

## Throw walls and cinematic hand-overs

The shared, floor-aware throw capsule reservation and temporary wall-push bypass are inferred engine placeholders, not traced native behavior.
The bounded search can exhaust in cramped space; it does not reserve full animated meshes or predict airborne velocity events and lethal defender branches.
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

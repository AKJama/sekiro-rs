# Known fidelity approximations

## Deathblow selection

Owl's phase-2 transition rejects deathblows for the lifetime of the TAE judgment marker 3506080, matching the user's real-game confirmation.
The native damage-motion suppression bit is traced and implemented, but the complete native throw-defence and ThrowKind condition dispatcher remains unreconstructed.
See [OWL.md](OWL.md#transition-deathblow-protection) for the data, native addresses and real-data regression tests.

## Maps

The ten-map baseline and reviewed evidence are in [MAP-BASELINE.md](MAP-BASELINE.md).
Map transforms, base textures, collision and GPARAM sun/ambient/fog load from the installed data.
Rendering uses a single albedo layer rather than the original layered moss, snow, blend masks and detail materials.
Water lacks its original shader, and large lakes and underground water can appear as pale flat polygons.
Local light sets, full exposure/scattering, particles, animated foliage and event-controlled object visibility remain incomplete.
The collision decoder skips convex primitives; the decoded mesh supports every sampled gameplay start and short walk, but does not establish coverage of all collision shapes.
The m89 sky library contains co-located dome variants without terrain; sky shaders and variant selection remain incomplete, and play mode has no authored floor there.
Grappling, swimming and fall respawn are not part of the map walking baseline.

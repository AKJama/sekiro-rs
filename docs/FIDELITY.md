# Known fidelity approximations

## Maps

The ten-map baseline and reviewed evidence are in [MAP-BASELINE.md](MAP-BASELINE.md).
Map transforms, base textures, collision and GPARAM sun/ambient/fog load from the installed data.
Rendering uses a single albedo layer rather than the original layered moss, snow, blend masks and detail materials.
Water lacks its original shader, and large lakes and underground water can appear as pale flat polygons.
Local light sets, full exposure/scattering, particles, animated foliage and event-controlled object visibility remain incomplete.
The collision decoder skips convex primitives; the decoded mesh supports every sampled gameplay start and short walk, but does not establish coverage of all collision shapes.
The m89 sky library contains co-located dome variants without terrain; sky shaders and variant selection remain incomplete, and play mode has no authored floor there.
Grappling, swimming and fall respawn are not part of the map walking baseline.

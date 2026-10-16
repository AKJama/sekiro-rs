# Object collision

Placed objects (MSB `Object` parts) carry their own hit collision in their binders, separate from the map's `h*` collision parts.
Until 2026-10-16 the simulation loaded only the map parts, so Wolf fell through every object he should stand on.
In the Ashina Outskirts that showed at the rideable pine over the start ravine (`o002500_0005`): the grapple landed him on the wire's virtual perch, and his first step dropped him into the ravine and the fall return.
This page records how the exe builds object collision and what the simulation now loads (`crates/extract/src/maps/object_collision.rs`, `crates/sim/src/collision.rs`, `crates/sim/src/hit_filter.rs`).
Addresses are RVAs in the 1.06 exe (`re/dump/manifest.json`), read with the headless Ghidra project.

## Where the data is

- Each object binder (`cache/raw/obj/<model>.objbnd.d`) may hold `<model>.hkx`, the intact physics, `<model>_1.hkx`, the broken pieces, and `<model>_c.hkx`, cloth.
  Across the 1395 unpacked binders there are 1000 intact, 495 broken and 32 cloth files.
- The intact file is a self-contained Havok 2016 tagfile (no compendium) holding one `hknpPhysicsSystemData`, like the map collision's files.
  Its bodies use `fsnpCustomParamCompressedMeshShape` (the map's compressed mesh with per-triangle materials) or convex shapes: `hknpConvexPolytopeShape`, `hknpBoxShape` and `hknpCapsuleShape`.
- Every body's `collisionFilterInfo` is 0 in the file: the exe assigns the group at runtime (below).
- `motionType` is 0 for static props, 1 for bodies with no mass driven by bones (door leaves, the pine's `Trunk00` to `Trunk03`), and 2 for the massive pieces of broken objects.
  Over every object file: 1019 bodies of type 0, 706 of type 1 (all massless), and 7027 of type 2 (4620 with mass, all of them in `_1.hkx` or ragdoll chains).
- The body `position` and `orientation` place each body in the model's space.
  Checked on `o002500`: decoded with that transform, the median distance from a collision vertex to the nearest FLVER vertex is 0.000 m (90th percentile 0.018 m); without the rotation, or with it inverted, it is 0.17 to 0.33 m.
- Convex polytope vertices are the hull shrunk by the shape's `convexRadius`.
  The Outskirts chest `o114740` has its box top at 1.006 m with radius 0.05 m, and its FLVER top at 1.056 m.
- Of 1509 placed objects in m11_00_00_00, 1119 have an intact physics file; in m11_01_00_00, 1388 of 1674.

## How the exe builds it

- `FUN_14087d530` sets up a placed object's physics for its break state (0 intact, up to 3): it takes the model resource's physics data at `+200 + state * 8` and creates a `FrpgPhysSysIns` (vtable `0x142a072e0`) through `FUN_140946470` and its constructor `FUN_140944be0`, so every body in the system becomes a Havok body.
- `FUN_140872b40`, the object instance's constructor, reads the model's `ObjectParam.collisionType` (`FUN_1408705d0` returns byte `+0x57` of the row, or 0 without a row) and indexes two 12-entry tables with it.
  Types 12 and above use entry 0.
  `0x1429e73b8` gives the group stored at instance `+0x134`: 24, 25, 27, 41, 47, 67, 92, 108, 47, 51, 32, 93.
  `0x1429e73e8` gives the group at `+0x130`: 28, 29, 31, 41, 47, 67, 92, 108, 47, 51, 32, 93.
- `FUN_140949fb0` then sets each body's filter (`FUN_14190d8b0`, the world's set-filter call): a body whose flags have bit 0 or 2 set (static or keyframed) gets the first group, any other (dynamic) gets the second.
  `FUN_14087d530` passes `+0x134` first and `+0x130` second for the intact state, and for a broken state passes 0x21 (デブリ, debris) for the dynamic pieces.
- The group names come from `FUN_140932c00` (group 24 オブジェ, 25 カメラに当たらないオブジェ, 27 カメラもロックもダメージも当たらないオブジェ, 28 to 31 the 動的オブジェ variants, 32 キャラとカメラが当たらないオブジェ, 108 キャラとロックが当たらないオブジェ).
- The collision matrix `FUN_140933390` (see `docs/HIT-FILTERS.md`) has the player's capsule (group 1, `FUN_1409325c0`) and enemies (group 2, `FUN_140932480`) collide with groups 24 to 27, and not with 28 to 32, 41, 47, 51, 67, 92, 93 or 108.
  The camera ray (group 48) collides with 24, 28, 41 and 108.

So the player stands on the static and keyframed bodies of objects whose `collisionType` is 0, 1 or 2, and on nothing of the others.
Type 6 is the "fall prevention" variant of a rideable pine (`o002501`, 落下防止), group 92, the fall-prevention assist hit, which the player's capsule does not collide with.

| collisionType | Group | Player and enemies | Camera | Outskirts objects (triangles) | Castle objects (triangles) |
|---|---|---|---|---|---|
| 0 | 24 | yes | yes | 351 (60404) | 616 (41023) |
| 1 | 25 | yes | no | 545 (28325) | 541 (44121) |
| 2 | 27 | yes | no | 5 (154) | 2 (60) |
| 4 | 47 | no | no | 149 (5089) | 176 (2556) |
| 6 | 92 | no | no | 29 (1382) | 24 (562) |
| 10 | 32 | no | no | 29 (4060) | 24 (4420) |
| 11 | 93 | no | no | 11 (156) | 3 (42) |

## What the simulation does

- `sekiro-extract map <id> --object-collision` (also part of the full map export) writes `cache/maps/<id>/object_collision.bin`: for every placed `Object` with an intact physics file, the static and keyframed bodies (dynamic ones are left out), placed with the part's MSB transform and mirrored into Bevy space like `collision.bin`, with the part name, entity id and `collisionType`.
  It is about 2.2 MB for m11_00_00_00 (99570 triangles) and 2.4 MB for m11_01_00_00 (92784).
- `CollisionWorld::add_objects` adds the parts the story keeps enabled: types 0 as ordinary collision, 1 and 2 as collision for the player and enemies only, 3 and 7 for the camera only, and leaves the rest out (`hit_filter::object_group`, `hit_filter::object_mask`).
- The play world (`map_world::collision_world`) adds every block's objects, skipping those the first-visit story has switched off (`DeactivateObject`, `docs/MAP-COMPLETENESS.md`): on the reference story 168 Outskirts and 64 Castle objects with collision are off.
- `CollisionWorld::floor_object` names the object under a point, and the live check records it per sample as `floor_object`.
- Standing on an object leaves the map selection on the last map part Wolf stood on (`MapWorld::stand` only scores map collision parts, and objects have no display groups).
  In the live trunk walk below the map stayed on `h023300` from the spawn to the platform, with no block request change.

## Measured

On the pine `o002500_0005` (collisionType 0), the data test (`crates/sim/tests/it/object_collision.rs`) walks a body from the tip over the ravine along the curving trunk to the platform: 9.3 m on the tree, grounded throughout; without object collision there is no floor under the tip above the event walls.
Walking off the tip, it leaves the floor at x = -141.5, the trunk's end.

In the live game, spawned on the lower ground south of the ravine at (-140, -50, -132), idle 7 s, grapple, then walk the trunk with `--script-route`:

- Before: Wolf landed on the perch at (-141.58, -44.26, -123.86) (spawned on the east ledge), and his first step west dropped him through `h900000` at y = -74.2 and the fall return.
- After: he landed at (-137.09, -43.55, -121.83), stepped onto the trunk, walked down to its tip at x = -140.2 and back up to the platform at (-135.9, -42.6, -117.7), with every sample on `o002500_0005` until the platform's map collision and no fall.

## Unresolved

- The MSB scale is applied to the collision like the drawn model; whether the exe scales object physics is not traced.
- Keyframed bodies stay in the bind pose of the hkx; the exe moves them with the object's animation (swaying trees, doors opened by their authored initial animation).
- Broken states (`_1.hkx`), destruction at runtime and `RequestObjectDestruction` are not applied; every object is intact.
- Convex polytope triangles have no hit material.
- The arena worlds (`Arena::load`) and the wire's own collision (`CollisionWorld::load_map`, used for line of sight) do not load objects.
- The body's floor is a point under the feet, so on a round trunk only the top band within the 50 degree slope limit holds Wolf (about 0.6 m wide on the pine), where the exe's 0.4 m capsule rests on the curved sides too.

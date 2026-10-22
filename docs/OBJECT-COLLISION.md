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
- A capsule (`hknpCapsuleShape`) is a polytope too, but its vertices are only a thin box around the segment from `a` to `b`, and its `convexRadius` is the capsule's radius.
  Over every object file there are 93 capsule bodies, 85 of them dynamic, and no placed object in m11_00_00_00 or m11_01_00_00 has a static or keyframed one.
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
Type 11 is group 93, its "ground actions only" twin.

## The fall-prevention assist

This is how the game stops Wolf at an edge: authored walls that only the assist sees, switched per behaviour state by the player script.
First traced for the grapple pine in draft PR #33 (#17); the map-wall part is applied for #119.

- Group names (`FUN_140932c00`): 92 落下防止アシストヒット(地上移動＆地上アクション) (fall-prevention assist hit, ground movement and ground actions), 93 the same for ground actions only, 94, 95 and 96 the sphere casts for 92, for 93, and for both.
- In the collision matrix `FUN_140933390`, group 92 meets only 94, 96 and 113, and group 93 only 95, 96, 113 and 114: no capsule, ray or other body.
- The walls are map parts with hit filter 28 (`h900400`, `h900410` in the Outskirts; every exported map has some) and objects of collisionType 6 and 11.
  In m11_00 `h900400` is 1400 triangles, bands about 1 m tall from the floor at the edge, spread over the Outskirts' cliff tops and ledges; 743 of its wall triangles have a drop of more than 2 m on their outer side.
  That is the `h` binder's copy; the `l` binder's copy of `h900400` (4850 triangles, the one the simulation loads, #189) also stands on the cliff top 60 m north of start 0, by the bent pine, as a 20 m tall wall.
- Most of the walls are in a third hit binder per map block, `map/<block>/f<tail>.hkxbhd` and `.hkxbdt` beside the `h` (hi) and `l` (lo) binders (#149).
  The exe names the set `%cREMOUNT %chiHKXBHD %cloHKXBHD %cfaHKXBHD %cWALLBHD` and waits on `m_fallAssistantHitFileCap->IsReady()`; the `.wall` files (`WALLBHD`) are another, undecoded format.
  Each file `f<tail>_<model>.hkx.dcx` (Kraken, so Oodle is needed) holds one `hknpExternMeshShape` whose `hknpDefaultExternMeshShapeGeometry` is a plain `hkGeometry` (vertices and triangles), named after the collision model of an MSB collision part and placed with that part's transform.
  m11_00 has 156 such files: 63,160 triangles over 141 placed collision parts, bands about 0.9 m tall at roof, wall and cliff edges, including the roofs and cliffs around start 0.
  The files carry no group: in m11_00 (1254 bodies) and m11_01 (1511) every body's `collisionFilterInfo`, `materialId` and `userData` are 0 and every triangle's material is -1, so the exe gives the walls their group when it loads them.
  It does so as group 93, not 92: a `lo` collision instance (flag bit 1 at `+0x12`, the one whose compendium is `l<tail>.compendium`) also loads a second file through `FUN_1409bb9a0` (`FUN_1409bbcf0`), whose name comes from `FUN_140c3c770` through `FUN_140c3c200` (fallback name `f??_??_??_??_????`, so the `f` binder's), and `FUN_1409bc320` gives that second body the filter 0x5d (93), while the instance's own body gets its part's group (`FUN_1409b58b0`), 92 for the filter-28 parts.
  In the matrix 93 meets 95, 96, 113 and 114 but not 94, so the `f` walls hold only bodies whose assist cast is in group 95 or 96: ground actions, not walking (#203, below).
- The bands cross every edge the map collision ends at, rideable objects included.
  `h023300`'s band runs along the root of the grapple pine over the start ravine (`o002500_0005`, at about (-135.7, -42.5, -118.1)), facing the platform; SpEffect 4100 (riding) has no behaviour ref, and neither HitMaterialSpecialSettingParam nor HitMtrlParam has a fall-assist field.
  Since a band holds only ground actions, Wolf walks from the platform onto the trunk and back; an attack at the root is held.
  Likewise a walk goes off the start 3 terrace's 11 m drop to the moat and off the roof by start 0, which have only bands, while the cliff north of start 0 holds it with its filter-28 wall.
  Bands also cross flat ground: in m11_00, 3062 band triangles with no filter-28 wall beside them have 2 m of floor at about the same height beyond them, and 2166 of them held a walk while the simulation took the bands as group 92 walls, among them the snowy cliff top by the bent pine (`h023300`, about (-148.9, -51.3, -131.3)), where the user was stopped walking on flat ground (#203).
- The player script asks for the assist every frame: `_UpdateFallProtection` (`c0000_transition.lua`, called from the `Update` in `c0000.lua`) sends `act(3016, 0)` in states whose `ENABLE_FALL_PROTECT_WALL_STRONG` flag is set and `act(3016, 1)` where `ENABLE_FALL_PROTECT_WALL_WEAK` is set (`g_paramHkbState` in `c0000_cmsg.lua`).
  Of 1310 states, 770 set both, 90 only the strong one (walk and run loops, idle, item and sub-weapon moves, crouch moves) and 450 neither: every jump, fall, wire, hang, cover, swim, dive and event state, and `SPRINT_LOOP`.
  It skips the request in cover, hang, wire fall, cling, swim, dive and aged-fall styles, in the wire-ready states, and while SpEffect behaviour ref 6 (`TAE_DISABLE_FALL_PROTECT`) is on; refs 11 and 12 switch off the strong and weak request.
- The act's handler (`FUN_140b32410`, case 0xbc8 = 3016) reads the first argument (`FUN_141010dc0(args, 2)`, the index every act case reads its first argument at) and sets bit `1 << arg` of the request word in the module `FUN_140b34de0` returns (`**(chr+0x1ff8)`, `+0x20`).
  `FUN_140bc6800` reads the two request bits: bit 0 alone casts with group 94, bit 1 alone with 95, both with 96; its caller `FUN_140bc58b0` skips the cast when the word is 0.
  So `act(3016, 0)` alone (walk and run loops, idle) meets only group 92, filter 28 and type 6 objects, and `act(3016, 1)` (attacks, sprint start and stop) adds group 93, the `f` bands and type 11 objects.
  In the simulation `StandMoveStart` and `GroundStep` send only `act(3016, 0)`, though their `g_paramHkbState` rows set both flags; refs 11 and 12 or the state the script sees may explain it, not checked (#203).
- `FUN_140bc68d0` casts the body along its move with that filter and cuts the move so the body stays `FallAssistMinRadius` (0.16 m, physics module `+0xaec`, constructor `FUN_140bbb010`) from a wall it heads into; a wall the body is in front of is skipped, so the walls keep a body in and let one outside walk in.
  The cast keeps only triangle hits, takes the one whose normal is closest to the move's direction (`FUN_140bbedc0`) and holds only below 90 degrees (`DAT_142dca970`); there is no check of the floor beyond the wall.
- NPCs share `FallPreventionAssist` (`c9997.lua`), which sends only `act(3016, 1)`: enemies are held by group 93 walls (the `f` bands and collisionType 11 objects) and not by the map's group 92 cliff walls.
  Their walk and run states do not call it, so a walking foe is held by the map's navmesh instead (`crates/sim/src/navmesh.rs`, #227).
- Sprinting: `SPRINT_LOOP` sends no request, while the sprint-start states before it send both, so a sprint into a filter-28 wall is held until the loop starts (0.3 to 1 s at the cliff east of start 2 and the cliff north of start 0 in `edge_hold.rs`; the user saw half a second to 1.5 s in the real game, #86, #32).

What the simulation does:

- `hit_filter::mask` gives filter 28 the `FALL_ASSIST_MOVE` bit and `object_mask` gives type 6 `FALL_ASSIST_MOVE` and type 11 `FALL_ASSIST_ACTION`; they join the world as helpers no capsule, ray or floor query sees.
- `sekiro-extract map <block> --fall-assist` (and every full `map` export) writes the `f` binder's walls, placed, to `maps/<block>/fall_assist.bin` (`hknp::PartCollision`, one triangle range per collision part).
  `CollisionWorld::load_map` and the game's `collision_world` add them as `FALL_ASSIST_ACTION` helpers (group 93), skipping parts the story switches off; arenas do not load them yet.
  A block without the binder has no walls, so the export removes an old `fall_assist.bin`; a failed write is a warning that leaves the rest of the export and the last good file whole.
  The game asks for a re-export only for a block whose binder is in `cache/raw` but whose `fall_assist.bin` is missing.
- The behaviour adapter records `act(3016, arg)` per tick (`PlayerEnv::fall_assist`, cleared before each script update) for Wolf and NPCs alike, and `BodyInput::fall_assist` hands the bits to `Body::step`, which moves through `Ground::slide_assisted`.
- `CollisionWorld::slide_for` then also keeps the feet `FALL_ASSIST_RADIUS` (0.16 m) on the inner side of those walls (`hold_inside`), in sub-steps of 0.08 m so none is stepped across.
  A slide gathers only the walls within `FALL_ASSIST_RADIUS` of the feet's height and in the slide's reach: m11_00's bands fill a quarter of the occupied cells, up to 248 a cell and mostly at other storeys, and gathering them all (up to 276 a slide) made every slide near them take the fine sub-steps and test every wall.
- The walls often stand up to 0.2 m past the floor's end, where the game's 0.4 m capsule still rests on the lip.
  A grounded body now stays on a floor its capsule rim reaches (`Body::rim_support`: eight points 0.35 m out, `RADIUS * sin 60`, the walkable slope limit), so it is held rather than dropped short of the wall.

| collisionType | Group | Player and enemies | Camera | Outskirts objects (triangles) | Castle objects (triangles) |
|---|---|---|---|---|---|
| 0 | 24 | yes | yes | 351 (70128) | 616 (61699) |
| 1 | 25 | yes | no | 545 (35393) | 541 (52625) |
| 2 | 27 | yes | no | 5 (412) | 2 (160) |
| 4 | 47 | no | no | 149 (5465) | 176 (4496) |
| 6 | 92 | no | no | 29 (1382) | 24 (562) |
| 10 | 32 | no | no | 29 (4060) | 24 (4420) |
| 11 | 93 | no | no | 11 (156) | 3 (42) |

## What the simulation does

- `sekiro-extract map <id> --object-collision` (also part of the full map export) writes `cache/maps/<id>/object_collision.bin`: for every placed `Object` with an intact physics file, the static and keyframed bodies (dynamic ones are left out), placed with the part's MSB transform and mirrored into Bevy space like `collision.bin`, with the part name, entity id and `collisionType`.
  It is about 2.7 MB for m11_00_00_00 (116996 triangles) and 3.3 MB for m11_01_00_00 (124004).
- A convex polytope's faces move out along their planes by the convex radius and are rebuilt from the moved planes, each clipped by all the others, so the shape is closed at its edges and corners (`hknp::expanded_hull`).
  An edge whose faces are more than 90 degrees apart, and a corner with such a pair, also get a bevel plane at the radius, so thin wedges do not reach far past the rounded hull.
  Over the 1431 polytopes in the intact object files (static, keyframed and dynamic), no corner is more than 1.5 radii outside the rounded hull (0.73 at box corners, the 99.9th percentile).
  A capsule is tessellated from its segment and radius instead (`hknp::capsule`).
- `CollisionWorld::add_objects` adds the parts the story keeps enabled: types 0 as ordinary collision, 1 and 2 as collision for the player and enemies only, 3 and 7 for the camera only, and leaves the rest out (`hit_filter::object_group`, `hit_filter::object_mask`).
- The play world (`map_world::collision_world`) adds every block's objects (`CollisionWorld::add_block_objects`), skipping those the first-visit story has switched off (`DeactivateObject`, `docs/MAP-COMPLETENESS.md`): on the reference story 168 Outskirts and 64 Castle objects with collision are off.
- The exception is the trees' fall-prevention walls (collisionType 6), which common event 20005560 switches on while Wolf has SpEffect 4100 and off when he loses it, whatever the story says.
  The groups come from the block's events: each `InitializeCommonEvent` of 20005560 passes its group as the only argument, and the event itself waits on SpEffect 4100 (`hit_filter::ride_wall_groups`, which checks that id).
  m11_00 starts it for group 1106200 alone (the Outskirts pines), and m11_01 for none.
  HitMaterialSpecialSettingParam 8 ("objects that can be ridden") gives that SpEffect, and a triangle's material id is its special setting times 100 plus its HitMtrlParam row, so the pines' trunks are 808.
  The play world adds these walls with `hit_filter::RIDE_ASSIST_MOVE` / `RIDE_ASSIST_ACTION` (`hit_filter::ride_mask`), so they hold riders and nobody on the ground.
  A body counts as riding on each assisted tick where the surface holding it up has special setting 8: the floor under its centre when that is within a step, otherwise the highest rim floor, the one `rim_support` stands it on (`Ground::floor_with_material` gives each floor's height and material from one query).
  The riding state then lasts 0.06 s after the last such tick (SpEffectParam 4100 effectEndurance, `hit_filter::SP_EFFECT_4100_ENDURANCE`), so a thin map floor poking through a branch or a tick over a gap in the trunk does not drop the walls (`Body::riding`, `hit_filter::riding_walls`).
- `CollisionWorld::floor_object` names the object under a point as `<block>/<part>` (`m11_00_00_00/o002500_0005`), since 200 object part names repeat between the Outskirts and Castle blocks, and the live check records it per sample as `floor_object`.
- The play setup loads a connected world's stories once (`story::load_blocks`) for both the objects' collision and the map (`add_map`), which would otherwise evaluate each block's story a second time (about 0.9 s per block).
  A map played alone (`--no-connected-blocks`) has no story for its drawn parts, so its objects' collision does not use one either.
- Standing on an object leaves the map selection on the last map part Wolf stood on (`MapWorld::stand` only scores map collision parts, and objects have no display groups).
  In the live trunk walk below the map stayed on `h023300` from the spawn to the platform, with no block request change.

## Measured

On the pine `o002500_0005` (collisionType 0), the data test (`crates/sim/tests/it/object_collision.rs`) walks a body from the tip over the ravine along the curving trunk to the platform: 9.3 m on the tree, grounded throughout; without object collision there is no floor under the tip above the event walls.
The play world itself (`map_world::collision_world`, connected blocks and story included) has the floor at the tip on `m11_00_00_00/o002500_0005` too (`play_collision_world_stands_on_the_grapple_pine` in `crates/game/src/map_world.rs`).
Walking off the tip, it leaves the floor at x = -141.5, the trunk's end.

In the live game, spawned on the lower ground south of the ravine at (-140, -50, -132), idle 7 s, grapple, then walk the trunk with `--script-route`:

- Before: Wolf landed on the perch at (-141.58, -44.26, -123.86) (spawned on the east ledge), and his first step west dropped him through `h900000` at y = -74.2 and the fall return.
- After: he landed at (-137.09, -43.55, -121.83), stepped onto the trunk, walked down to its tip at x = -140.2 and back up to the platform at (-135.9, -42.6, -117.7), with every sample on `o002500_0005` until the platform's map collision and no fall.

## Unresolved

- The MSB scale is applied to the collision like the drawn model; whether the exe scales object physics is not traced.
- Keyframed bodies stay in the bind pose of the hkx; the exe moves them with the object's animation (swaying trees, doors opened by their authored initial animation).
- Broken states (`_1.hkx`), destruction at runtime and `RequestObjectDestruction` are not applied; every object is intact.
- Convex polytope triangles have no hit material.
- The arena worlds (`Arena::load`) and the wire's own collision (`CollisionWorld::load_map`, used for line of sight) do not load objects yet; `CollisionWorld::add_block_objects` is the call they would share.
- The body's floor is a point under the feet, so on a round trunk only the top band within the 50 degree slope limit holds Wolf (about 0.6 m wide on the pine), where the exe's 0.4 m capsule rests on the curved sides too.

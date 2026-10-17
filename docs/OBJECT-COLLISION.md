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
Another query tests them, below.

## The fall-prevention assist

Placed with the rideable pine over the start ravine is `o002501_0005` (collisionType 6): 52 triangles, all steep, standing along both sides of the trunk's top and across its tip, from about 0.5 m below the top to 2 m above it.
They are not a floor; they are the walls that keep Wolf on the trunk.

They are on only while Wolf rides the tree:

- All eight `o002501_*` parts are in entity group 1106200, which `m11_00_00_00` event 0 hands to common event 20005560.
- Event 20005560 (restart on end): `DeactivateObject(group, off)`, wait for `IfCharacterHasSpeffect(10000, 4100, has)`, `DeactivateObject(group, on)`, wait for the effect to end, restart.
  So the story's first visit leaves the walls off, and they come on while Wolf has SpEffect 4100.
- SpEffectParam 4100 is ヒットマテリアル判定_乗れるオブジェ (hit material judgment, rideable object), `effectEndurance` 0.06 s, spCategory 20.
- `HitMaterialSpecialSettingParam` row 8, 乗れるオブジェ, has `SpEffectId` 4100 (rows 6 and 7 give 108010 steep slope roof and 108020 deep waterside).
- The rideable pines' triangles all carry hit material 808, and other ids follow the same pattern: 201, 204, 208 (row 2, no camera), 501, 504, 508 (row 5), 616 (row 6).
  So a material id is the special setting times 100 plus the HitMtrlParam row; the exe code that splits it and applies the effect is not traced.
- Standing on the tree refreshes 4100 every step, so it ends 0.06 s after he leaves it.

- Group names (`FUN_140932c00`): 92 落下防止アシストヒット(地上移動＆地上アクション) (fall-prevention assist hit, ground movement and ground actions), 93 the same for ground actions only, 94, 95 and 96 the sphere casts for 92, for 93, and for both, 113 and 114 the player's and enemies' fall-prevention throw-follow proxies.
- In the collision matrix `FUN_140933390`, group 92 meets only 94, 96 and 113, and group 93 only 95, 96, 113 and 114: no capsule, ray or other body.
- The player script asks for the assist every frame: `_UpdateFallProtection` (`c0000_transition.lua`, called from the `Update` in `c0000.lua`) sends `act(3016, 0)` in states whose `ENABLE_FALL_PROTECT_WALL_STRONG` flag is set and `act(3016, 1)` where `ENABLE_FALL_PROTECT_WALL_WEAK` is set (`g_paramHkbState` in `c0000_cmsg.lua`).
  Of 1310 states, 770 set both, 90 only the strong one (walk and run loops, item and sub-weapon moves, crouch moves) and 450 neither: every jump, fall, wire, hang, cover, swim, dive, damage-flight and event state, and `SPRINT_LOOP`.
  It skips the request in cover, hang, wire fall, cling, swim, dive and aged-fall styles, in the wire-ready states, and while SpEffect behaviour ref 6 (`TAE_DISABLE_FALL_PROTECT`) is on; refs 11 and 12 switch off the strong and weak request.
- `FUN_140bc6800` reads the two request bits (character `+0x1ff8`, first module, `+0x20`): bit 0 alone casts with group 94, bit 1 alone with 95, both with 96, and none casts nothing.
  So `act(3016, 0)` is taken as bit 0 (group 92) and `act(3016, 1)` as bit 1 (group 93); the act's own handler is not traced.
- `FUN_140bc58b0` runs it on the body's move each physics step (callers `FUN_140bcc570` and `FUN_140bccd50`) and stores whether a wall was met at `+0xac0`.
- `FUN_140bc68d0` casts the body along the move (`FUN_1418f73c0`) with that filter, collects the hit triangles' planes, slides the move along the nearest one, and then, for each plane the move heads into (the hit is ahead, and the move runs along the plane's face normal), cuts the move so the body stays `FallAssistMinRadius` from it, or stops it when it is already that close.
  A plane the body is in front of is skipped, so the walls are one-sided: they keep a body in and let one outside walk in.
- `FallAssistMaxRadius` and `FallAssistMinRadius` (physics module `+0xae8` and `+0xaec`, debug menu `FUN_140bc0280`) are 0.4 and 0.16 m (constructor `FUN_140bbb010`).
  `FUN_140bca6d0` also corrects the final position against the stored walls when the body has stopped.

What the simulation does:

- The story interpreter (`world_events`) records a startup wait on one of Wolf's SpEffects that the first visit never meets, with the object and collision writes after it (`PlayerEffectSwitch`); a part those writes switch on while the story leaves it off gets `PartState::while_player_has` (4100 for the eight `o002501_*`).
- The play world (`map_world::collision_world`) adds those objects as `ObjectSwitch::WhilePlayerHas(4100)`: in the grid, but off until `CollisionWorld::set_player_effect` turns them on.
- `Duel::apply_floor_effect` gives Wolf the SpEffect of his floor's special setting (`material / 100`) each step he is grounded on it, with its `effectEndurance`, and tells his ground which floor effects he has.

- `hit_filter::object_mask` gives type 6 the `FALL_ASSIST_MOVE` bit (group 92) and type 11 `FALL_ASSIST_ACTION` (group 93), and map parts with hit filter 28 (group 92, `h900400` and `h900410` in the Outskirts) get `FALL_ASSIST_MOVE` too; they join the world as helpers no capsule, ray or floor query sees.
- The player adapter records `act(3016, arg)` per tick (`PlayerEnv::fall_assist`, cleared before each script update), and `BodyInput::fall_assist` hands the bits to `Body::step`, which moves through `Ground::slide_assisted`.
- `CollisionWorld::slide_for` then also keeps the feet `FALL_ASSIST_RADIUS` (0.16 m) on the inner side of those walls (`hold_inside`), in sub-steps of 0.08 m so none is stepped across.
- The walls are tested at the feet: where the pine's walls cross the trunk's top they meet its edges (at the start of its curve, 1.5 m apart, with the walkable top from x = -137.0 to -135.7).
  The height the exe tests at is not traced; higher up the tip's walls lean over the trunk by up to 0.6 m, so a test along the whole capsule pinned Wolf in place there.
- The export mirrors X without reversing the triangle order, so in Bevy space the pine's walls face the trunk (all 52 normals point at it); that side is the inner one.
  In the game's space they face out, which is what the exe's test needs to keep a body in.

Sprinting: `SPRINT_LOOP` sends no request, while the step and sprint-start states before it do, so a sprint into the wall is held only until the loop starts.
The user saw this in the real game (held for one to two seconds, then off the trunk); issue #32 tracks it.
The simulation's sprint at the wall stays in its start clips and is held, so it does not show the release yet.

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
- `CollisionWorld::add_objects` adds the parts the story keeps enabled: types 0 as ordinary collision, 1 and 2 as collision for the player and enemies only, 3 and 7 for the camera only, 6 and 11 as fall-prevention walls (above), and leaves the rest out (`hit_filter::object_group`, `hit_filter::object_mask`).
- The play world (`map_world::collision_world`) adds every block's objects (`CollisionWorld::add_block_objects`), skipping those the first-visit story has switched off (`DeactivateObject`, `docs/MAP-COMPLETENESS.md`), except those switched by one of Wolf's SpEffects (above).
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

Fall prevention (`the_fall_prevention_walls_hold_wolf_on_the_trunk` and `wolf_stays_on_the_trunk_until_he_jumps_off` in the same file):

- In the play world with the story applied (`play_world_holds_wolf_on_the_grapple_pine` in `crates/game/src/map_world.rs`), walking from the trunk's middle west, east and up the trunk holds him on it; without the floor effect the walls stay off and he falls west and east.
  Toward the tip with the camera at yaw 0 he reaches the right flank and still drops (Unresolved).
- With the player script driving Wolf from (-136.0, -43.06, -120.0), pushing the stick west for 2 s holds him at x = -136.99, grounded throughout, the script asking for the assist each frame; a jump from there carries him over the wall to x = -145.4 and down into the ravine.
- A body walking square to the trunk at 1.5 and 4.8 m/s, from halfway along each stretch toward its left side (seen from the tip), and both ways near the tip, stays on the pine; without the walls it walks off every time.
- Toward the tip's end it stops at x = -141.06, short of the end at -141.5.
- From beside the trunk, where the grapple sets Wolf down (-137.09, -43.55, -121.83), he walks in onto it and along to the platform.
- Toward the right side past the tip he still drops: there the walls stand where the trunk is steeper than 50 degrees (below).

## Unresolved

- The MSB scale is applied to the collision like the drawn model; whether the exe scales object physics is not traced.
- Keyframed bodies stay in the bind pose of the hkx; the exe moves them with the object's animation (swaying trees, doors opened by their authored initial animation).
- Broken states (`_1.hkx`), destruction at runtime and `RequestObjectDestruction` are not applied; every object is intact.
- Convex polytope triangles have no hit material.
- The arena worlds (`Arena::load`) and the wire's own collision (`CollisionWorld::load_map`, used for line of sight) do not load objects yet; `CollisionWorld::add_block_objects` is the call they would share.
- The body's floor is a point under the feet, so on a round trunk only the top band within the 50 degree slope limit holds Wolf (about 0.6 m wide on the pine), where the exe's 0.4 m capsule rests on the curved sides too.
  On the pine's right flank past the tip (seen from the tip) the fall-prevention walls stand outside that band, so Wolf is held by the wall but has no floor and drops off (3 m in the data test); a capsule-shaped floor would close it.
- The fall-prevention walls: the act 3016 handler itself, the height the exe's cast tests at, the plane slide's exact order, the `+0xa2` case that stops a body within `FallAssistMinRadius`, and `FUN_140bca6d0`'s end-of-step correction are not traced or applied; enemies' `FallPreventionAssist` (`c9997.lua`, `act(3016, 1)`) is not wired.

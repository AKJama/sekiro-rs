# Map collision hit filters

Every MSB collision part has a hit filter (`hit_filter_id`, byte 0 of the part's hit parameters).
Ordinary parts use 8.
The helper parts named `h9xxxxx` use other values, and until 2026-10-16 the simulation treated all of them as floor and wall.
In the Ashina Outskirts ravine north of start 0 that put Wolf on `h900000` (filter 15) at y = -74.2, standing on nothing drawn.
This page records what the exe does with each filter and what the simulation now applies (`crates/sim/src/hit_filter.rs`).
Addresses are RVAs in the 1.06 exe (`re/dump/manifest.json`), read with the headless Ghidra project.

## From hit filter to collision group

`FUN_1409b5a50` reads the filter byte from the part's hit parameters (the debug writer `FUN_140c19460` prints the same byte as `hit_parts_param__hit_filter_id`).
`FUN_1409b58b0` turns it into a Havok collision group, the low 7 bits of a body's filter info.
Its two flags pick the resource: the high-resolution `h` binder gets group 38 for ordinary parts, the low-resolution `l` binder 39 (or 40 for its middle variant).

The group names come from the debug formatter `FUN_140932c00` ("Group[%s]"), which indexes a table of 120 UTF-16 names at `0x142a03848` and on.
The English is a translation.

| Hit filter | Group | Group name | Player | Enemy | Camera | Outskirts parts | Castle parts |
|---|---|---|---|---|---|---|---|
| 8 and unlisted | 38 / 39 / 40 | ハイ / ロウ / ミドルあたりマップ (high / low / middle map) | yes (39, 40) | yes (39, 40) | yes (39, 40) | 155 | 192 |
| 9 | 41 | マップのカメラあたり (map camera hit) | no | no | yes | `h900900` | `h900900` |
| 11, 19 | 42 | 敵だけが当たる壁当たり (wall only enemies hit) | no | yes | no | `h900300`, `h900200`, `h900201` | `h900300`-`h900302`, `h900200`, `h900201` |
| 12-15, 20, 21, 23 | 44 | マップアクションイベント壁 (map action event wall) | no, contact reaction | no | no | `h900001` (13), `h900000` (15) | `h900001`, `h900011` (13), `h900000`, `h900010` (15) |
| 16 | 46 | 水面 (water surface) | no | yes | yes | | `h900500` |
| 17 | 109 | 描画グループコピー (draw group copy) | no | no | no | | |
| 18 | 51 | 敵レイキャストだけあたる壁 (wall only enemy raycasts hit) | no | no | no | | |
| 22 | 43 | キャラのカプセルだけ当たる壁当たり (wall only character capsules hit) | yes | yes | no | `h900600` | `h900600`, `h900601`, `h900800` |
| 24 | 68 | なににも当たらない (hits nothing) | no | no | no | | |
| 25 | 85 | 壁張り付き中当たり (hit while wall-hugging) | while wall-hugging (group 86) | no | no | `h900100`, `h900101` | |
| 26 | 88 | 壁ジャンプ出来ない壁 (wall that cannot be wall-jumped) | no | no | no | | `h901000` |
| 27 | 91 | 水泳当たり (swim hit) | yes | yes | no | | 12 `h0xxxxx` parts |
| 28 | 92 | 落下防止アシストヒット(地上移動＆地上アクション) (fall-prevention assist, ground movement and actions) | no | no | no | `h900400`, `h900410` | `h900400`, `h900401` |
| 29 | 79 | 弾丸しか当たらない壁 (wall only bullets hit) | no | no | no | | |

## Which groups collide

`NS_SPRJ::FrpgCollisionFilter` (vtable `0x142a03800`) holds a 128 x 128 bit matrix at `+0x20`, 16 bytes per group.
Its pair test `FUN_140936940` checks the bit of one body's group in the other's row (and rejects two bodies of the same nonzero subgroup).
`FUN_140933390` fills the matrix once: group 0's row all set, then 783 symmetric enables through `FUN_140932ac0`, one disable through `FUN_140932940`, and the batch helpers `FUN_1409325c0`, `FUN_140932480`, `FUN_1409326e0`, `FUN_1409367f0` and `FUN_1409368a0`.
Replaying its arguments from the disassembly gives:

- The player's capsule, group 1 (プレイヤー), collides with map groups 39, 40, 43 and 91, and not with 38, 41, 42, 44, 46, 51, 68, 79, 85, 88, 92 or 109.
- Enemies and bosses, groups 2 (敵) and 5 (ボス) and their map-only variants 7 and 10, collide with 39, 40, 42, 43, 46 and 91: they also stand on water surfaces and stop at enemy-only walls.
- The camera ray, group 48 (カメラ), collides with 39, 40, 41 and 46.
- Group 44 collides only with group 45 (マップアクション壁収集用, the map action wall collector) and group 82 (a map event detection ray), so nothing solid stands on or bumps into an event wall.

So the player walks on the low-resolution binder in the exe, and so does the simulation (#189): the player, enemies and the camera all collide with `collision_lo.bin` (`crates/sim/src/lo_collision.rs`, `docs/FORMATS.md`).
The helper parts keep their fixed groups whichever binder their body comes from, so their `l` copies are the ones characters meet, and the story's part removal and the helper stripping read the parts' ranges in `collision_lo.bin`.
`l` models a flight of stairs as a ramp where `h` has real treads, so Wolf rises and descends a flight smoothly (#104).
On the long castle stairs past start 3's moat gate his height changes by at most 0.047 m a tick at a run and never against the way he walks; on `h` a riser lifted him up to 0.29 m in one tick (`play_collision_world_walks_stairs_smoothly`).
In the exe the treads are left to foot IK, which meets `h`.
The game draws the same (#276, `crates/game/src/foot_ik.rs`): the body stays on `l`, and each drawn foot is cast down onto the `h` floors of the tiles around Wolf, the pose drops by the lower foot's offset and two-bone IK bends each leg.
On the Stairway idol platform (start 3) `l` is one flat 24-triangle slab of part `h007600` at y -48.026, while `h` and the drawn ground dip up to 0.22 m below it; with foot IK the idle feet meet `h` within 2 cm (`foot_ik_puts_the_feet_on_the_idol_platform_craters`).
The gains are c0000's `hkbFootIkControlsModifier` (on/off 0.2, ground ascending 0.2, descending 1); its `hkbFootIkDriverInfo` (ray lengths, leg limits) is not in the behbnd, so those are chosen.
A block exported before `collision_lo.bin` existed falls back to `collision.bin`, with a log line naming `sekiro-extract map <block> --lo-collision`.
Parts with an `h` model and no `l` model, m11_00's `h900700` and m11_01's `h012400`, are met by no character and no camera in the exe and get an empty range; m11_00's `h009500` has no model in either binder.

## Wall-hug collision

Group 85 (壁張り付き中当たり, hit filter 25) collides only with itself and group 86.
`FUN_1407df900` sets a character's collision group (`+0x910`) to 86 (0x56) and its IK ray's to 87 (0x57, the wall-hug IK ray) while `FUN_140b7d5c0` on `chr+0x1ff8 -> +0x128` is true, the wall-hug state.
Group 86 collides with 1, 2, 17, 18, 24 to 27, 32, 39, 40, 43, 71, 75, 85 and 102 (`FUN_140933390`'s enables).
So a wall-hugging body stands on the low and middle map, the capsule-only walls and the objects like a walking one, and on the wall-hug parts too.

In m11_00's snake pit the `l` copy of `h900100` is a flat slab under the narrow ledge (variation 10) of cover wall `h013600`: its top lies within 0.1 m below the ledge's edge, it is about half a metre thick, and it reaches more than 1.2 m out from the wall.
On `l` the ledge itself has no floor under the docked feet (0.7 m out) along about half of chain 3, in gaps up to 3.7 m, over the pit's fall-return trigger.
The simulation gives the wall-hug parts the `hit_filter::WALL_HUG` bit and the cover search the `Ground::hug_floor` query, which adds them; so the hug crosses the ledge to the far end of chain 4 on the slab, and a walking Wolf, which does not meet group 85, falls into the pit there (`snake_pit_wall_hug_reaches_the_far_end_on_the_hug_slab` and `snake_pit_ledge_has_no_walking_floor` in `crates/sim/tests/it/cover.rs`).

Leaving the hug there, Wolf is a group-1 capsule again (`FUN_140b7d5c0` is true only while the docking type at `+0x10` is 1), so he stands on what `l` has under his 0.4 m capsule.
Where the ledge reaches within the capsule's rim (0.35 m) of the docked feet, he rests on its lip with nothing under his centre (19 of the ledge's points at 0.1 m spacing); where `l` has no ledge within the capsule's radius at all, as over the gap of about 2.7 m from chain 3's second edge into its third, he drops (34 points; `snake_pit_hug_exit_rests_on_the_lip`).
Over that gap `l` has only near-vertical wall faces (normal y under 0.1) at the ledge's height, so nothing a capsule can stand on.
Not checked in the running game: whether the exe keeps the docking type through the cover-end animation, which would only delay the drop.

Cover edges whose docked feet (0.7 m out, `DOCKING_TRANSLATION_OFFSET_COVER`) have a floor on `h` and none at the edge's height on `l`, over the nine maps, after the swim-hit rule:

| Map | Edge | Part | `l` at the feet |
|---|---|---|---|
| m11_00 | 525 | `h006400` (variation 1) | nothing within 1.2 m of the edge's height (`h` has its material-30 floor 0.2 m below) |
| m11_00 | 893 | `h010500` (variation 3) | a floor at 3 of 9 points, too few for the search |
| m11_02 | 175 | `h004400` (variation 3) | `h004400`'s `l` hull 0.56 m above the edge on one side, nothing on the other |
| m20 | 209 | `h001900` (variation 2) | nothing (`h` has `h001900` at the edge's height and `h002500` 0.5 m below) |

The low-resolution hulls end short of these edges, so the exe's capsule has no floor there either; they are left undockable.
m25's `h004200` and `h016200`, lost while deep material counted as water on ordinary parts, dock again.

## Event wall contact reactions

`FUN_1409edf20` runs when a character touches a part, switching on its hit filter:

- 13: when the toucher is the local player (`FUN_1409f3430`: `WorldChrMan+0x88`) and three state checks pass, sets `WorldChrMan+0x98` to 1. Untraced further.
- 14: sets bit 2 of a word at `chr+0x50 -> +0x9c`. Untraced.
- 15: when the state checks and `FUN_140b975f0(.., 1)` pass, sets `chr+0x1a48` to 1.0, `chr+0x1a4c` to -2.0 and bit 8 of `chr+0x1a14`.
  `FUN_140b975f0` is the fall-dead routine (it logs `SprjPlaylog_ChrFallDead`).
  For a character that is not a player (`chr` vtable `+0x1a8` false) it always sets HP to 0 (through the HP setter `FUN_140bd64e0`) and damage type 6, and returns true, so an NPC dies and fades (#234, `crates/sim/src/npc_fall.rs`).
  For the player it takes a share of 0.4 (`DAT_143b01958`) of a value read by `FUN_1405a9e40` (likely max HP) off his HP and sets damage type 14 (`FALL_DEAD_RETURN`), and returns false; when HP is too low, it kills him as an NPC.
  The per-character update `FUN_1409f78a0` calls it too, when the physics module's `FUN_140bc2210` reports a fall of 30 s airborne or 1500 m below the last stance, or `+0x8e8` (10 s airborne with the height changing by at most 0.02 m a step).
  `FUN_1409f6b20` adds `0x1a4c * dt` to `0x1a48` each frame and multiplies the model's draw alpha by it, so the model fades out over 0.5 s.
  When it reaches 0 with bit 8 set, it sets the physics module's `0x92b`, and `FUN_140bcc420` then sets `0x92a`, which holds the body.
- 16: spawns an effect at the contact point. 17: `FUN_1409f5820`. 20: a flag at `chr+0x1a12`. 21: a flag, then a warp to a point found from the part's parameter plus 100 (`FUN_1406d5f00`). 23: a virtual call with `0x140000001`. 27: `FUN_1409ee830`.

Filter 15 under a pit is the fall return's trigger: Wolf falls through it, fades out, and is held, and the script's `FALL_DEAD_RETURN` route (damage type 14, `docs/MOBILITY.md`) brings him back.

## What the simulation applies

- `hit_filter::strip_helpers` takes every part the player does not collide with out of the walkable collision when a map, an arena or the wire's collision loads (`map_world::collision_world`, `Arena::load`, `CollisionWorld::load_map`).
  Removed triangles collapse to a point, so the parts' triangle ranges stay valid.
  An arena strips them after its disabled, event and hidden parts are taken out, so those do not return as helpers, and crops them to its radius like the rest.
- The switched event collision `h900900` (entity 1124860, `arena_c::take_event_collision`) has hit filter 9, so while on it holds only the camera.
- The removed parts that enemies or the camera collide with stay in the world, tagged with who: enemies and bosses query it through `collision::EnemyGround` (filters 11, 16 and 19), and `CollisionWorld::camera_raycast` (the play camera) stops at filters 9 and 16.
  The player's queries skip them all.
  The camera-only parts stay helpers on `l` too: Castle's switched event collision `h900900` (filter 9, 52,877 `l` triangles) and Outskirts' wall-hug slabs (filter 25) stand no one (`low_resolution_helpers_are_not_floors` in `hit_filters.rs`).
- Only the swim hit (filter 27, group 91) is water.
  The exe swims on contact with it: env 3025 (水泳あたりに接触しているか？, "is touching the swim hit") is set by `FUN_1409ee830`, the filter-27 case of `FUN_1409edf20`, not by the material of a floor.
  So `strip_helpers` takes the swim hits out of the walkable mesh into `HelperCollision::swim`, and a deep-water material (HitMtrlParam rows 22, 36 and 38) on one is the plane bodies swim at, not a floor (`CollisionWorld::solid`).
  In `l` every swim hit carries the deep material 22; in `h` they have none or a dry one, so a block without `collision_lo.bin` has no swim planes.
- Every other part is solid for whoever it holds whatever its material (#189).
  `l` puts the deep material 22 on ordinary ground too, where `h` has sand: m25's shore `h004200` at (116.0, 104.9, 408.1) lies 1.3 m over the swim hit `h021500`, and Wolf stands on it (`deep_material_on_ordinary_shore_is_dry_ground` in `swim.rs`).
  The water surface (filter 16) carries it as well, and the Guardian Ape stands on m17's `h900702`.
  Where `l` has only the swim hit, a body meets water where `h` had a floor, as in the exe: at m17's (-68.5, 292.0) `h`'s floor is at -74.3, while `l` has the swim hit `h002600` at -74.5 over a bed at -76.1.
  Counting `h`'s dry floors that `l` turns into deep water at 1 m spacing, these spots are 1,069 in m10, 30 in m11_02, 106 in m13, 304 in m15, 583 in m17, 11 in m20 and 641 in m25.
  In m13's dungeon pool the `l` water surface (`h900100`, filter 16 there) lies 1 m below `h`'s; the player swims at the swim hit, which he does collide with.
- Fall-prevention parts (hit filter 28, group 92) stay as walls only the fall-prevention assist sees: they hold Wolf at cliff edges while his script asks for it (`docs/OBJECT-COLLISION.md`, #119).
- Event walls (group 44) become contact triggers (`CollisionWorld::trigger_contact`, tested along the capsule's path at its bottom, middle and top).
  Contact with filter 15 starts the fall return: after the 0.5 s fade Wolf is returned to where he last stood, the existing return path.
  Every contact is logged (`helper collision: ...`) and kept in `Duel::helper_events`; reactions other than 15's are only recorded.
- Two existing tests had relied on helpers holding Wolf.
  The Owl arena fall (`owl_arena.rs`) used to land and die on `h900011` (filter 13) 62 m below the roof; Wolf now falls through it and `h900010` (filter 15) and is returned, alive.
  On `l` (#189) these walls are tilted planes rather than `h`'s flat sheets: under the test's point, 80 m east of the Owl, Wolf passes `h900011` at y = 2.5, `h900001` (13) at -1.8 and `h900010` at -2.2.
  An arena keeps an event wall whole when any of it reaches the arena's radius: `l`'s walls are 9 large triangles each, and cropped per triangle the part under that point was lost, so the placeholder void depth returned Wolf instead.
  The Guardian Ape deathblow test (`guardian_ape.rs`) stood Wolf on the pond's water surface helper (filter 16); the pond is deep water (hit material 22), so the test now takes the fight to the dry bank.
- Where an authored filter-15 wall lies below a falling Wolf, the placeholder void depth waits for it.
- A foe (the enemy of any duel) that meets a filter-15 wall dies, fades over the same 0.5 s and is then held (#234); a foe airborne 30 s or 1500 m below its last stance dies too.
- `crates/sim/tests/it/hit_filters.rs` counts the parts by hit filter over the whole of m11_00_00_00 and m11_01_00_00, asserts which the player collides with, and checks the ravine.
  The live test (`crates/game/tests/it/live_play.rs`) drops Wolf into the ravine with `--live-tour drop:x,y,z`.

- Placed objects get their groups from `ObjectParam.collisionType` instead of a hit filter; `docs/OBJECT-COLLISION.md` has that table and what the player collides with.

## Measured

Dropping from (-137.6, -44.3, -124.5) in the live game with Outskirts start 0:

- Before: Wolf landed on `h900000` at y = -74.18 and stood there, grounded, with no drawn ground within 20 m.
- After: Wolf passes `h900000` at y = -74.18 (fall return starts), then `h900001` (filter 13) at y = -79.04, falls to y = -94.5 during the 0.5 s fade, and is returned to where he last stood (the spawn, in the single-stop run).

## Unresolved

- The fall return's HP cost (above) is not applied, and where the exe puts the character back and what `0x92a` does beyond holding the body are untraced; the return uses the last stance on collision, as before.
- The extra condition `FUN_140b975f0(.., 1)` for filter 15 and the other reactions (13, 14, 20, 21, 23, 27) are not decoded or applied.
- The fade itself is not drawn: `Duel::fall_return_opacity` exposes it, but the renderer has no model opacity yet.
- The camera ray in the exe passes through filters 22 and 27, which the simulation's camera ray still hits; wire and other rays use the player's set.
- Which group the player's floor ray (`FloorHitCollisionFilter`, physics module `+0xf8`) uses is not traced; group 44 only meets groups 45 and 82, so it cannot be its floor.
- The render gate's spot sampling (`map_sweep.rs`) still samples every collision part, helpers included.

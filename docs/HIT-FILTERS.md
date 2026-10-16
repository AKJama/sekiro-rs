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
| 25 | 85 | 壁張り付き中当たり (hit while wall-hugging) | no | no | no | `h900100`, `h900101` | |
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
- The fall-prevention assist hits, groups 92 and 93, meet only the assist's sphere casts (94 and 96 for 92, 95 and 96 for 93) and the throw-follow proxies 113 and 114 (`docs/OBJECT-COLLISION.md`).
- Group 44 collides only with group 45 (マップアクション壁収集用, the map action wall collector) and group 82 (a map event detection ray), so nothing solid stands on or bumps into an event wall.

So the player walks on the low-resolution binder in the exe; the simulation keeps using the exported high-resolution one as its stand-in.

## Event wall contact reactions

`FUN_1409edf20` runs when a character touches a part, switching on its hit filter:

- 13: when the toucher is the local player (`FUN_1409f3430`: `WorldChrMan+0x88`) and three state checks pass, sets `WorldChrMan+0x98` to 1. Untraced further.
- 14: sets bit 2 of a word at `chr+0x50 -> +0x9c`. Untraced.
- 15: when the state checks and `FUN_140b975f0(.., 1)` pass, sets `chr+0x1a48` to 1.0, `chr+0x1a4c` to -2.0 and bit 8 of `chr+0x1a14`.
  `FUN_1409f6b20` adds `0x1a4c * dt` to `0x1a48` each frame and multiplies the model's draw alpha by it, so the model fades out over 0.5 s.
  When it reaches 0 with bit 8 set, it sets the physics module's `0x92b`, and `FUN_140bcc420` then sets `0x92a`, which holds the body.
- 16: spawns an effect at the contact point. 17: `FUN_1409f5820`. 20: a flag at `chr+0x1a12`. 21: a flag, then a warp to a point found from the part's parameter plus 100 (`FUN_1406d5f00`). 23: a virtual call with `0x140000001`. 27: `FUN_1409ee830`.

Filter 15 under a pit is the fall return's trigger: Wolf falls through it, fades out, and is held, and the script's `FALL_DEAD_RETURN` route (damage type 14, `docs/MOBILITY.md`) brings him back.

## What the simulation applies

- `hit_filter::strip_helpers` takes every part the player does not collide with out of the walkable collision when a map, an arena or the wire's collision loads (`map_world::collision_world`, `Arena::load`, `CollisionWorld::load_map`).
  Removed triangles collapse to a point, so the triangle ranges in `layout.json` stay valid.
  An arena strips them after its disabled, event and hidden parts are taken out, so those do not return as helpers, and crops them to its radius like the rest.
- The switched event collision `h900900` (entity 1124860, `arena_c::take_event_collision`) has hit filter 9, so while on it holds only the camera.
- The removed parts that enemies or the camera collide with stay in the world, tagged with who: enemies and bosses query it through `collision::EnemyGround` (filters 11, 16 and 19), and `CollisionWorld::camera_raycast` (the play camera) stops at filters 9 and 16.
  The player's queries skip them all.
- Fall-prevention parts (hit filter 28, group 92) stay as walls only the fall-prevention assist sees: they hold Wolf while his script asks for it (`docs/OBJECT-COLLISION.md`).
- Event walls (group 44) become contact triggers (`CollisionWorld::trigger_contact`, tested along the capsule's path at its bottom, middle and top).
  Contact with filter 15 starts the fall return: after the 0.5 s fade Wolf is returned to where he last stood, the existing return path.
  Every contact is logged (`helper collision: ...`) and kept in `Duel::helper_events`; reactions other than 15's are only recorded.
- Two existing tests had relied on helpers holding Wolf.
  The Owl arena fall (`owl_arena.rs`) used to land and die on `h900011` (filter 13) 62 m below the roof; Wolf now falls through it and `h900010` (filter 15) and is returned, alive.
  The Guardian Ape deathblow test (`guardian_ape.rs`) stood Wolf on the pond's water surface helper (filter 16); the pond is deep water (hit material 22), so the test now takes the fight to the dry bank.
- Where an authored filter-15 wall lies below a falling Wolf, the placeholder void depth waits for it.
- `crates/sim/tests/it/hit_filters.rs` counts the parts by hit filter over the whole of m11_00_00_00 and m11_01_00_00, asserts which the player collides with, and checks the ravine.
  The live test (`crates/game/tests/it/live_play.rs`) drops Wolf into the ravine with `--live-tour drop:x,y,z`.

- Placed objects get their groups from `ObjectParam.collisionType` instead of a hit filter; `docs/OBJECT-COLLISION.md` has that table and what the player collides with.

## Measured

Dropping from (-137.6, -44.3, -124.5) in the live game with Outskirts start 0:

- Before: Wolf landed on `h900000` at y = -74.18 and stood there, grounded, with no drawn ground within 20 m.
- After: Wolf passes `h900000` at y = -74.18 (fall return starts), then `h900001` (filter 13) at y = -79.04, falls to y = -94.5 during the 0.5 s fade, and is returned to where he last stood (the spawn, in the single-stop run).

## Unresolved

- The fall return's HP cost, where the exe puts the character back, and what `0x92a` does beyond holding the body are untraced; the return uses the last stance on collision, as before.
- The extra condition `FUN_140b975f0(.., 1)` for filter 15 and the other reactions (13, 14, 20, 21, 23, 27) are not decoded or applied.
- The fade itself is not drawn: `Duel::fall_return_opacity` exposes it, but the renderer has no model opacity yet.
- The camera ray in the exe passes through filters 22 and 27, which the simulation's camera ray still hits; wire and other rays use the player's set.
- Which group the player's floor ray (`FloorHitCollisionFilter`, physics module `+0xf8`) uses is not traced; group 44 only meets groups 45 and 82, so it cannot be its floor.
- The render gate's spot sampling (`map_sweep.rs`) still samples every collision part, helpers included.

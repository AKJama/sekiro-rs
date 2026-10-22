# Wolf's mobility: grapple, ledges, wall hug, air control, falls, swimming

Research for the traversal rewrite, written so a coding agent can implement from it.
Nothing here is code.
Date of the research: 2026-10-11.
Game build: Steam v1.6.0.0.

Tags used on every claim:

- **[V]** verified against a file, offset, id or tool run named next to it.
- **[I]** inferred from patterns in the data; plausible but not proven.
- **[?]** unknown.

"HKS" below means the decompiled player script in `re/hks/c0000*.lua` (the extracted bytecode is `cache/raw/action/script/c0000*.hks`).
Line numbers refer to those decompiled files.

## 1. Summary

- Grapple points are not MSB regions or events.
  They are placed objects whose model carries a one-record `e<model>.edge` file [V: `cache/raw/obj/o003610.objbnd.d/e003610.edge`, 80 bytes].
  We have the bytes in `cache/` but no reader that applies them.
- Ledge hang and wall hug use the map's `.edgebhd/.edgebdt` edge files (record kind 2 and kind 3, see section 3).
  The edge format is the same one the grapple objects use.
- The player script already contains every transition for all five mechanics.
  Our runtime answers `0` for every query that gates them, so none of them can start [V: `crates/sim/src/player.rs` `answer_env`, the `_ => 0.0` arm].
- Fall control is an exe table, not a param.
  FwLo's RustSekiro (MIT) already traced it; we have not wired it.
- Fall death has real data: `HitMtrlParam.fallDeathDist`.
  Fall damage amount does not.
  The units of `env(224)` are centimetres, and our sim feeds metres [V: section 5].
- FwLo's tree has nothing for grapple, ledge, climb, wall hug or swimming [V: section 8].

## 2. What the runtime returns today

Source: `crates/sim/src/player.rs`, `Adapter::answer_env`, `Adapter::act`.
No env override is set anywhere outside tests [V: grep of `overrides` in `crates/`].

| Query | Meaning (HKS name) | Today | Needed |
|---|---|---|---|
| env 200 | is falling | `body.falling()` (airborne and vy < 0) | ok |
| env 201, 248 | landed, really landed | `body.grounded` | ok |
| env 224 | fall height | metres from `Body::last_fall_height` | centimetres (section 5) |
| env 3003 | vertical speed | falls to `0.0` | cm/s, negative when falling |
| env 3000 | grapple can be fired | `0.0` | true when a target is in range |
| env 3011 | docking end type | `0.0` | 0 none, 1 left end, 2 right end |
| env 3017 | docking edge variation id | `0.0` | edge variation of the current edge |
| env 3020 | touching a wall-jump wall | `docking::wall_contact` while airborne, plus the `ContactWallAngle` variable (section 6.1) | exe contact check not traced |
| env 3025 | touching a swim volume | `0.0` | water plane contact |
| env 3038, 3039 | can dive, can surface | `0.0` | water state |
| env 3043, 3044, 3045 | can start wall hug, ground hang, air hang | edge searches in `docking.rs`; 3045 is the air catch type (section 6.1) | exe search boxes not traced |
| env 3046 | candidate edge variation (arg 1 = hang) | `0.0` | edge variation |
| env 3048, 3049, 3059, 3060 | hang corner moves | `0.0` | corner geometry |
| env 3050 | can climb up from hang | `0.0` | top surface test |
| env 3051, 3052 | obstacle at docking start, end | `0.0` | edge ends blocked |
| env 3055, 3061 | init pose, safe-position return | `0.0` (which equals `STAND`, not `NONE` = -1) | see section 5 |
| act 3016 | fall-prevention assist | walls of groups 92 (arg 0) and 93 (arg 1) hold the move (`docs/OBJECT-COLLISION.md`) | ok |
| act 3018 | set wire action in progress | recorded, no effect | wire state flag |
| act 3023 | pre-computed movement (type, anim) | recorded, no effect | root-motion pre-evaluation |
| act 3026, 3027, 3028 | run wall hug, ground hang, air hang start tests | recorded, no effect | trigger the env 3043 to 3045 searches |
| act 3030 | show action guide | recorded, no effect | HUD prompt (ActionGuideParam 10, 11, 20 to 22) |
| act 3035 | prefer grapple-able edges | recorded, no effect | grapple target bias |
| act 3011, 3037 | underwater, on water | recorded, no effect | water flags |

The grapple button already exists: `arm::GRAPPLE = 3` in `crates/sim/src/input.rs`, bound to key F in `crates/game/src/play.rs`, and `grapple` works in script files (`crates/sim/src/demo.rs`).
`ACTION_ARM_WALL_HANG = 12` and `ACTION_ARM_SP_MOVE = 5` (dodge) are the other buttons the hang code reads; `arm 12` is not wired to any key [V: `re/hks/c0000_define.lua` lines 7 to 17].
Action unlock 4 (`SUB_WEAPONE`) is in our default unlocked set, so env(3033, 4) passes; unlock 14 (wire move attack) does not [V: `PlayerEnv::default`].

## 3. Edge data: the shared format

### 3.1 The `.edge` record format

Known from STATUS.md and re-verified here by parsing 284 map edge files and every object edge file.

- 16-byte header: `u32 version = 4, u32 count, u32 owner number, u32 0` [V].
  The owner number is the collision model number for map files (200 in `m11_00_00_00_000200.edge`) and the object number for object files (3610 in `e003610.edge`).
- Then `count` records of 64 bytes [V: file size = 16 + 64 * count for every map file, no exceptions].
  Layout: two vec4 endpoints (`x y z 1`), one vec4 (zero for map edges, a small offset vector for wire objects), `u32 chain id`, `u32 flags`, 8 bytes padding.
- Flags byte 0 is the edge kind: 1, 2 or 3 [V: histogram below].
- Flags byte 1 is the variation, and its value sets match HKS constants exactly:
  - kind 2: values 0 and 1 only. These are `HANG_EDGE_TYPE_DISABLE_HOLD_FOOT` and `ENABLE_HOLD_FOOT` (`c0000_define.lua` lines 404 to 405) [I, strong: identical value set].
  - kind 3: values 0 to 3, and 10. These are `COVER_EDGE_TYPE_*` disable look, free look, left only, right only, and peek-move (lines 446 to 453) [I, strong].
  - kind 1 on objects: 0, 1, 2, 3, 5, 20, 24, 40, 41. These are `WireVariationParam` row ids [I, strong: ids 20, 24, 40, 41 exist and carry the matching `chrPhysicsHomingIdOffset`, section 4.2].
- Flags bytes 2 and 3 (0, 1, 5, 10 to 14) are unknown [?].
  On kites they run 1, 10, 11, 12, 13, 14, which is suggestive of per-end cover/peek variation.
- Endpoints are in the owner's local space.
  For map edges apply the collision part's MSB transform; for objects apply the placed part's transform.
  In Bevy space, mirror local X first, then apply the mirrored part matrix: `p' = (S * part.matrix() * S) * (S * p)`, where `S = scale(-1, 1, 1)` [V: the edge extractor uses exactly the collision exporter transform].
  This equals `rotate(q, (-px * sx, py * sy, pz * sz)) + t` with the layout's mirrored transform.
  The m11_01 `h019900` chain-7 segment from (77.7366, 29.5121, 242.8321) to (83.7762, 29.5392, 249.2599) has upper collision 0.0505 m above its midpoint and a drop on its outward side [V: `crates/sim/tests/docking.rs` and reviewed `target/alt/mobility/hang-side.png`].

### 3.2 Counts from `cache/raw/map/<id>/<id>.edgebhd`

Decoded with a scratch Python script that reads the BHF4 header and decompresses KRAK with the install's `oo2core_6_win64.dll` (read only).

| Map | Edge files | Records | Chains | Kind 1 | Kind 2 (hang) | Kind 3 (cover) |
|---|---:|---:|---:|---:|---:|---:|
| m11_00_00_00 | 137 | 1562 | 503 | 4 | 584 | 974 |
| m11_01_00_00 | 147 | 2289 | 838 | 0 | 963 | 1326 |

STATUS.md's "16 files, 255 edges" is the partial extract under `re/edge/` (16 files of m11_00), not the whole map [V: `ls re/edge`, 16 entries].
Kind 2 variation split on m11_01: 880 with variation 0, 83 with variation 1.
The new extractor places all 2289 m11_01 records, but only 1486 m11_00 records (579 hang, 907 cover) have a collision owner placed by its MSB.
Unplaced collision models are omitted from `edges.json`; the binder parser tests still verify the complete raw counts above.
The 4 kind-1 records on m11_00 belong to raw collision owner `h060000` with variation 0 and 10 [V].
That owner has no placed collision part in the current MSB, so `grapple.json` retains these records with null world positions and their authored `local_position`; the runtime excludes them from targeting [V: `crates/formats/tests/grapple.rs`].

### 3.3 `.wall` files

`map/<id>/<id>.wallbhd/.wallbdt` holds one `h<number>.wall` per collision part (190 on m11_01) [V].
Header: `u32 version = 3, u32 1, u32 0x48, u32 0, u32 number, ...` followed by sizes and sections [V: hex of `re/wall/h000100.wall.dcx`].
The content is undecoded [?].
HKS has `STYLE_TYPE_CLING = 10` and `DOCKING_TARGET_TYPE_CLING_POLYGON = 3` (`c0000_define.lua` line 409), so these are probably cling polygons for wall climbing, not wall hug [I].

## 4. Grappling hook

### 4.1 Player HKS

States and events (from `re/hks/c0000_hkb_outline.txt`, `c0000_transition.lua`) [V]:

- Start conditions: env(1106, 3) pressed (or the buffered add-action input), env(3033, 4) unlocked, **env(3000) true**, and `g_forceCrouch == FALSE` for the ground version (`BEH_A_GROUND_WIRE_SHOOT` at line 6251).
  The air version (`BEH_A_AIR_WIRE_SHOOT`, line 6154) has the same without the crouch test.
- Ground shot: `BEH_A_GROUND_WIRE_SHOOT` calls act(127), act(3018, 0), sets `WireMoveStartIndex`, then fires `W_GroundWireShoot` (or `W_SwimWireMoveReady` when swimming) (lines 4418 to 4426).
  Hang and cover edge ends can also fire `W_AirWireShoot` after env(3000) (lines 3124 and 3165).
- `WireMoveStartIndex` comes from `GetWireMoveStartIndex(state)`: 0 up slope, 1 vertical, 2 horizontal [V: define lines 382 to 384].
  Its inputs are not visible in the script (engine side) [?].
- Graph states [V]: `GroundWireShoot` (clip 202000), `GroundWireMoveStart` selector (202010 up slope, 202011 up, 202012 vertical), `AirWireShoot` (202100), `AirWireMoveStart` selector (202110, 202110, 202112), `WireMove` (CMSG 202020, plays the free-fall loop clip a000_201030), `LandWireMove` (202035), `WireFallLandReady` (202045), `WireMoveWireShoot` (202600) and `WireMoveWireMoveStart` (202610) for a chained second hook, `WireMoveWire2Shoot` (202700), `AirBreakFallWireShoot` (202300), `AirWireMoveStartFromBreakFall` (202310), `SwimWireMoveReady` (528000), `SwimWireMoveStart` (202410), `WireFallDeflect` (203400), `WireFallAttackCombo1` (309000), `LandWireFallAttackCombo1` (309050).
- Style: `STYLE_TYPE_WIRE_FALL` (the airborne grapple travel) calls act(3032), act(3035) and act(3023, 5, 202600 or 202700) each frame, sets throw states and is kept alive by act(3018, 1) (lines 626 to 632, 815 to 818).
- Landing: `BEH_R_LAND_WIRE` (and `LAND_WIRE_READY` when env(3003) <= -1000 and env(248)) fires the `LandWireMove` family.
- Every shoot and move-start state calls **act(3023, type, anim)** with types 2 to 6 and the clip ids above.
  The engine evaluates the clip's total root motion in advance so the travel can be scaled to reach the target [I: act name "pre-computed movement", types listed in `c0000_define.lua` lines 524 to 545].
- Unused by `c0000.hks` but present in the exe's act list: act 3003 "face nearest wire point", act 3017 "wire obstacle check setting" [V: `re/ext/meow_act.txt`].

### 4.2 TAE events on the grapple clips

Dumped with `target/alt/release/examples/tae_probe.exe a000_<id>` [V].

| TAE type | Name | Where | Arguments |
|---|---|---|---|
| 900 | WireSyncWithTarget_Start | 202000 at 0.467 to 0.633 s, 202100 at 0.367 to 0.600 s | float 50.0 |
| 901 | WireSyncWithTarget_Travel | move-start clips 202010/11/12/110/112, 0 to 0.5 or 0.6 s | none |
| 902 | WireLookAtTarget | shoot and travel windows | none |
| 903 | WireEventUnk01 | 0.1 to 0.2 s on move starts | int 950 |
| 904 | WireSlideToStartPoint | 202010 at 0.5 to 0.667 s | none |
| 905 | WireDisableFalling | 202100 at 0 to 0.5 s | none |
| 920 | ChrPhysicsVelocityChange | travel start: row **1100** (202010 at 0.667 s, 202011/12/110/112 at 0.167 to 0.5 s) | row id |
| 921 | unnamed | shoot clips at 0 to 0.2 s | int 4, int 300000000 |
| 700 | CustomLookAtTwistModifier | modifiers 140 and 150, `TargetType 5: Wire` | head and spine twist |
| 715 | OverrideWeaponModelLocation | whole clip | `WeaponModelType 3: Grappling Hook`, dummy poly 144 |
| 717 | WireModelEventUnk | whole clip | 1 |
| ChrActionFlag 27 | SetNoGravity | 202000 over 0 to 0.633 s | none |

Our TAE reader (`crates/sim/src/tae.rs`) only acts on 920 here; 900 to 905, 921, 715 (hook model) and 717 are not used [V: grep].

### 4.3 Params

- `ChrPhysicsVelocityChangeParam` row 1100 "wire": all velocity terms 0, `fallType` **2** (Wire), `homingTargetType` 4, `homingId` 300000000.
  Rows 1101 "ground wire acceleration" (horizontal 30, vertical 30, fallType 2) and 1102 "aerial wire deceleration" (zeros) are system rows [V: `cache/params/ChrPhysicsVelocityChangeParam.json`].
- `ChrPhysicsHomingParam` 300000000 "PC wire_default": offset (0, 0, 0), `fallCorrectionGuaranteeArrival` 2, `fallCorrectionTurn` 1, `initialVelocityCorrection` 1.
  The per-target rows are `300000000 + WireVariationParam.chrPhysicsHomingIdOffset`: 300000020 "cannot ride default" offset (0, -2.5, 7.5), 300000024 offset (0, 3, 2), 300000040 kite valley offset (0, -30, 30), 300000041 kite castle offset (0, -22, 25) [V: ids and names match the variation rows].
  `fallCorrectionGuaranteeArrival` 2 vs 1 is undefined in our data [?]; FwLo reads 1 as "re-aim the horizontal velocity each frame so the fall ends on the target" [V: his `src/player.rs` plunge code, used for the plunge deathblow].
- `WireVariationParam` (45 rows): `targetMarkerType` (0 rideable marker, 1 not rideable), `extendWireActionDistance`, `recognitionAngleUp/Down/Left/Right`, `chrPhysicsHomingIdOffset`.
  Rows used by m11 objects: 0, 1, 3 (ride, extension 0), 5 (ride, extension 10), 20 (cannot ride), 24 (cannot ride, castle snake valley), 40 and 41 (kites, extension 30) [V].
  Row 1 and 5: 90 degrees all round; row 3: 90 up and down, 180 left and right.
  Row 1 has extension 0; row 5 has extension 10 [V: extracted rows].
- `WirePointSearchParam` (20 rows), default row 0: act near 3 m, act far 15 m, find far 25 m, filter box height 10.2 up and 10.2 down, width 14.3, soft-lock box 10.2 and 7.15 [V].
  Castle rows: 11100 far 25 and find 50 (cavalry boss map), 11110 far 50, 11500 far 20, 11510 far 28, 11520 far 15 with filter height down 30.
- `WireSetParam` (24 rows) lists up to 20 search-row ids per area; row 0 is `[0, 100, 10100]`, all identical to the default distances.
  Rows for the castle: 1100200, 1100210, 1100220 (m11_00), 1101200, 1101210, 1101220 (m11_01) [V].
  **How the engine selects a set is unknown** [?]: the ids appear nowhere in the m11_01 MSB or EMEVD (the hits in m11_00 are item and entity ids that collide by coincidence, checked by location).
  The MSB does contain regions named "wire set change", "wire point switch" and "wire extension" in m11_01 (Event_ワイヤーセット変更領域_本城内堀 and similar), so sets are probably switched by EMEVD on region entry [I].
- Exe tunables visible as debug-menu strings (values not extracted): `Game.Player.WireAction.RaycastElevationDeg`, `HitChangeDistance_XZ/_Y`, `WireLowOrbitHorizontalAcceleration`, `WireHighOrbitHorizontalAcceleration`, `WireAccelerationSphereCastRadius`, `ConsecutiveWire minDist/angleY/angleUp/angleDown/extraTime`, `WireDramaticAngle` [V: `re/exports/strings.tsv`, referenced by functions at 0x9771f0, 0xbb45c0, 0xbb5250, 0xefccc0].
  Source files named in the exe: `Game/WireAction/EdgeDataMap.cpp`, `ActionEdgeMng.cpp`, `TempEdgeDataMap.cpp`, so the engine loads edge data into one edge manager shared by grapple and docking [V: strings at 0x2b45fb0 to 0x2b46c10].
- Fall control for the travel: fall type 2 "Wire" (section 5).

### 4.4 Where the grapple points are

STATUS.md's lead said "object `o000100` carries dummy 90 and is very likely the grapple point".
That is wrong: `ObjectParam` row 100 is "Item drawstring" (a loot pouch) [V: `cache/params/ObjectParam.json`].
No grapple model carries a FLVER dummy at all [V: `dummy_census.exe` on `o003610`, `o003600`, `o003500`, `o003660`, `o003000` prints no dummies].

The real data:

- `ObjectParam` rows named "(wire Pt)" or "wire point": 45 rows, among them Root 3000, 3500, 3510, 3520, Shachi 3600, 3610, Wooden shark 3650, 3660, Fossil bone 3710 [V].
- Their objbnds hold an `e<number>.edge` with one kind-1 record (a 4 cm segment, flags 0x101: kind 1, variation 1) [V].
  Other models with kind-1 edges: snow pines 2500 to 2610 (rideable and not), kite 2700 (6 records, variations 40 and 41, flags byte 2 = 1 and 10 to 14), Takebashi bridges 115410 to 115430, decorative tiles 115750 to 115780, giant shachihoko 115790, snake fittings 3200 and 3210, 3250, turret ornaments 115720 and 115730 [V].
- `o115500` "Root (wire point)" has no edge file, so it is not counted [V: listing shows only flver and hkx].
- The object's model origin is the anchor; the first vec4 gives the point in model space (for `o003610`: (0.02, 0.315, -0.05) to (-0.02, 0.315, -0.05), third vec (0, -0.15, -0.3)) [V]; the third vector is probably the landing offset or reference direction [?].

**Counts, from the MSB object parts plus object edge files** (placed objects whose model has a kind-1 edge; events may disable some at runtime [?]):

| Map | Placed grapple objects | Kind-1 edges | Of which dedicated "wire Pt" models | Pines | Kites | Other |
|---|---:|---:|---:|---:|---:|---:|
| m11_01_00_00 (Ashina Castle) | 172 | 224 | 120 | 7 | 2 | 43 |
| m11_00_00_00 (Ashina Outskirts) | 237 | 360 (+4 in collision `h060000`) | 187 | 31 | 0 | 19 |

Edges by `WireVariationParam` id: m11_01 { 1: 196, 5: 2, 0: 4, 3: 10, 40: 8, 41: 4 }; m11_00 { 1: 343, 5: 5, 3: 6, 20: 3, 24: 3 } [V].
Both counts include objects far from the player route; no reachability was checked.
The 120 dedicated points on m11_01 are the 79 `o003610` and 26 `o003600` Shachi plus 11 `o003500`, 3 `o003000` and 1 `o003020` roots [V].

### 4.5 What we extract and what is missing

- Extracted today: MSB object parts (`crates/formats/src/msb.rs`, `crates/extract/src/maps/mod.rs`) with model name and transform in `layout.json`; the param tables including all three Wire tables (`cache/params/Wire*.json`) [V].
- Steps 2 and 3 now add the shared `.edge` reader, placed `edges.json`, object/collision `grapple.json`, and TAE 900 to 905 / 921 timing and argument retention.
- Missing: region-triggered wire-set changes, authored object disablement, native wire kinematics, hook attachment, and complete edge-end semantics for kinds 1 to 3.
  The MSB loader now reads environment-probe regions for visuals, but traversal region/event logic remains unavailable.

### 4.6 How the game picks the hook (issue #178)

Traced in Ghidra on the Steam 1.6.0.0 exe, 2026-10-21; addresses are image addresses.
The search lives in `WireActionTargetMng` (constructor 0x140ef94e0, debug menu 0x140efccc0).
Its per-frame update 0x140efaf80 copies the `WirePointSearchParam` row into the manager and calls the search 0x140efd8e0 [V].

- **Camera only.** The search reads the player camera's matrix (character +0x1190) and builds a check ray from it [V].
  The ray starts 0.8 m in front of the camera (constant at 0x14328917c) and runs along the camera's forward axis for the pass's far distance [V: 0x140efb2c0, 0x140efb560].
  The forward axis is tilted up by the config `Game.Player.WireAction.RaycastElevationDeg`, whose getter (0x1419fdc40) returns 0 when the config is not set [I].
- **Player back filter.** For each kind-1 edge, the horizontal angle between Wolf-to-edge and the check ray's direction must be at most 75 degrees [V: 0x140eeda30; the manager stores 105 at +0x148, the angle between edge-to-Wolf and the ray, and 45 at +0x14c while locked on].
  The test is skipped when the edge is straight above or below Wolf [V].
- **Distance bands.** 3D distance from Wolf: under `actionDistanceNear` is ignored, up to `actionDistanceFar` plus the variation's `extendWireActionDistance` is usable (Ready), up to `findDistanceFar` plus the extension is shown but not usable (Distant) [V: 0x140eeda30, 0x140efd8e0].
- **In front of the camera.** The closest points between the check-ray segment and the edge must have a ray parameter above 0: the hook lies beyond the plane 0.8 m in front of the camera [V: 0x140efb6f0 with the segment-segment closest-point routine 0x141a54680].
- **View box.** With view filtering on (+0x134, on by default) the hook, projected to the screen, must fall inside a box 15.8 m in front of the camera (0.8 m plus the 15 m view filtering distance at +0xb4) [V: 0x140efbc60].
  The box reaches `filteringDistanceWidth` (14.3 m) to each side along the horizon, and `filteringDistanceHeightUpper` and `Lower` (10.2 m) up and down along the camera's up axis [V].
  So in view space a hook needs |x / depth| at most 14.3 / 15.8 and y / depth between -10.2 / 15.8 and 10.2 / 15.8.
  The width is a half-extent on each side, not the whole width.
- **Ranking.** Survivors are sorted by the screen distance between the projected hook and the projected check ray, with x scaled by the aspect ratio, so by the angle from the screen centre [V: comparator 0x140efeb60].
  The first one in that order whose obstacle raycast (0x140efba50) is clear is marked [V].
- **Ready before Distant.** Under a manager flag (bit 29, purpose unknown [?]) the search runs two passes, the usable band first and the shown-only band second; otherwise one pass over both bands [V].
  We always prefer usable hooks, which is the two-pass behaviour.
- **Soft lock.** While the marked hook still passes the back filter, the narrower soft-lock box (`softlockDistanceWidth` 7.15 m, heights 10.2 m), the distance bands and the raycast, it stays marked and no new search runs [V: 0x140efd8e0; soft lock +0x136 and re-search on leaving the action range +0x138 are both on by default].
- **Recognition angles.** `WireVariationParam.recognitionAngle*` are tested against the edge's own direction, not the camera; 180 means no limit [V: 0x140eeda30].
  FwLo's RustSekiro reads them the same way.
- **Marker and pull.** The search keeps one target; the HUD marker and the wire both use it [V: the state 1 Ready or 2 Distant is stored with the single target].

The runtime follows this in `crates/sim/src/wire.rs`: the back filter (75 degrees, 135 locked on), the distance bands with the extension on both, the 0.8 m front plane, the view box, the ranking with the sight ray cast down the sorted survivors until one is clear, Ready before Distant, and the soft lock for a usable hook.
Its camera is built inside the simulation, so scripted and replayed runs pick the same hook at any frame rate: the follow camera orbiting a focus `CameraDirector::focus_height` (1.5 m by default) above Wolf's feet at `CameraDirector::distance` (4.5 m by default), turned to the step's `InputFrame::camera_yaw` and `camera_pitch`, and pulled in front of walls as the camera on screen is.
The hook the HUD marks (`preview`) is the only one the wire can fire at (`candidate`).
FwLo's `grapple_target` (RustSekiro `src/map.rs`, commit 292655c) has the same camera-plane, nearest-to-centre and soft-lock structure; his box takes the width as the whole width.
What is not reproduced is listed in [FIDELITY.md](FIDELITY.md#grapple-target-search-issue-178).

## 5. Air control, falling, fall death and respawn

### 5.1 Fall-control table (from FwLo)

An exe table at `0x143b09ce0`, one row per `ChrPhysicsVelocityChangeParam.fallType`, found and verified live by FwLo (`docs/kb/movement.md`, `src/data.rs` `FALL_TYPES`, MIT) [V from his source and notes; not re-traced by us].

| fallType | Name | Horizontal accel (m/s2) | Vertical accel (m/s2) | Stick accel | Stick max speed | Steerable |
|---:|---|---:|---:|---:|---:|---|
| 0 | Normal | -0.1 | -13.72 | 10 | 2 | yes |
| 1 | Normal uncontrollable | -0.1 | -13.72 | 10 | 2 | no |
| 2 | Wire | -15 | -9.8 | 9 | 6 | yes |
| 3 | Decelerating | -0.1 | -13.72 | 10 | 2 | yes |

Total gravity is Havok world gravity 9.8 plus the vertical term: 23.52 m/s2 for types 0, 1, 3 and 19.6 m/s2 for the wire type [V: FwLo measured 23.52 over five recorded jumps; `GRAVITY` in `crates/sim/src/body.rs` already uses it].
Param usage: of 174 velocity-change rows, 63 use fallType 0, 108 use 1 and 3 use 2 (rows 1100 to 1102, the wire) [V: histogram].
Row 3810 "air speed maintenance, not adjustable" (fallType 1) is applied by the fall-death clips 199100 and 199120 [V: TAE dump], so a falling death cannot be steered.

FwLo's reading of the exe's fall module (`FUN_140bac120`) [V as his claim, unverified by us]:

1. One horizontal air velocity `v` decays along its direction by `hAccel * dt` (skipped once slower than one step).
2. The stick adds `a = stickAccel * stick * dt`, split along and across `v`.
   The across part always applies; the along part applies in full when braking and, when accelerating, only up to `stickMax - |v|`.
3. A fast launch can therefore be steered and braked but never pushed faster.

His known gap: how stick input combines with the launch velocity is modelled, not traced [V: his movement.md].

**Our gap** [V]: `crates/sim/src/body.rs` integrates only `GRAVITY` and the launch velocity.
`VelocityChange` in `crates/sim/src/tae.rs` has no `fall_type` field.
`BodyInput` has no stick.
Horizontal air velocity is constant; PROGRESS.md lists "Air control: not started".

Other jump rows with `fallType` 1 and homing: positioning jumps 111 to 118 (7.68 m/s, 7.712 up, homing id 10000 when locked on), wall jumps 1000 to 1011, 1111 to 1118, 1200 to 1218, storm jumps 980 to 982, kick jumps 2002 to 2111.
Those belong to the wall-jump work, not here.

### 5.2 Fall height, landing clips and thresholds

- `env(224)` is multiplied by 0.01 before comparison with metres (`local fallHeight = env(224) * 0.01`, `c0000_transition.lua` lines 1770 and 2122) and compared raw with `WALL_JUMP_ENABLE_FALL_HEIGHT = 2000` (line 6023) [V].
  So env 224 is in centimetres [I, strong].
  Our sim returns metres [V: `character.rs` line 541], so `fallHeight` is 100 times too small and no stiff landing can ever trigger.
- Thresholds in metres [V: `c0000_define.lua` lines 211 to 214]: short stiff landing 5 (3 after a positioned jump), long stiff landing 20, long stiff water landing 20.
  Landing clips: `LandFreeFall` 200020, `LandFreeFallLongStiff` 200021, `FreeFall` loop 200010, `SelfFallStart` 200000.
- env 3003 (vertical speed) is cm/s: `LAND_READY_ENABLE_FALL_SPEED = -2000`, `LAND_WIRE_READY_ENABLE_FALL_SPEED = -1000` (lines 312 to 313) [I, same logic].
  `BEH_R_LAND` needs `env(3003) <= 0`, which our stub of 0 satisfies; the hard "land ready" states need a real value.
- `BEH_R_FALL` (free fall start) requires env(200) and `g_beforeFireLand == -1` (line 5684).

### 5.3 Fall death and fall damage

- `HitMtrlParam` has `disableFallDamage` (u8), `fallDeathDist` (u32, default 40) and `fallDamageRate` (f32, 0 to 1).
  The def descriptions say: landing on this material from a height at or above `fallDeathDist` kills the player instantly (PC only); `fallDamageRate` multiplies the fall damage (PC only); `disableFallDamage` removes it for PCs and enemies [V: `cache/refs/paramdex/Defs/HitMtrlParam.xml`].
  Unit of `fallDeathDist` is not stated; the default 40 and the 20 m long-stiff threshold suggest metres [I].
- Values [V: `cache/params/HitMtrlParam.json`]: every material 40 except water planes 22 (dive plane), 36 (swim slope) and 38 (water by grass swim) at 80 with `fallDamageRate` 0.5, and material 40 "cloud" with `disableFallDamage` 1.
  Walkable water (20, 21, 35, 37) and swamp keep 40.
  There are 34 hit materials in m11 collision, and `Ground::material()` already exposes them for footsteps (`crates/sim/src/body.rs`).
- **The HP amount of fall damage is not in any param.**
  The exe has a `FallDamage` function (strings.tsv: string at 0x2a28878, referenced by functions 0xa32e30 and 0xa15b20) and `GameData.DisableFallDamage` [V: names only; not decompiled in `re/exports/decomp/`] [?].
  `SpEffectParam.fallDamageRate`, `NpcParam.fallDamageDump` and `CharaInitParam.red_Falldam` (defined as damage absorbed when another character is landed on) exist as multipliers [V].
  FwLo has no fall damage [V: section 8].
- Fall reactions in the graph: `FallHitDmgSmall` (clips 102100, 102101), `FallHitDmgLargeStart/Loop` (102300, 102310), `LandFallHitDmgLarge` (102320, 102321), `FallDeathDmgStart/Loop` (121000, 121010, 121011), `LandFallDeathDmg` (112020, 112021), `FallDeathStart/Loop` (199100, 199110), `DeadFallStart`, electro variants, plus `GroundFallReturn` (711210) and `FallDeadReturnStart` (199120) [V: outline].
  The script never fires the entry events of `FallHitDmgSmall/Large`, so the engine sends them from fall-damage code [I].
- Death routing in the script (`BEH_R_DEATH`, lines 761 to 835): `damageType == 6` (`DEATH_FALLING`) fires `W_FallDeathStart`; `14` (`FALL_DEAD_RETURN`) fires `W_FallDeadReturnStart`; `15` and `16` (`LAND_DEAD_RETURN`, `LAND_DEAD`) fire the land death starts (water variants when env(3025)); free-fall styles otherwise use damage level and direction to pick `W_FallDeathDmgStart` [V].
  Return types after a fall-return death: `SAFE_POS_RETURN_TYPE_STAND/CROUCH/SWIM/DIVE` via env(3061), consumed by `InitState` (lines 175 to 200) [V].
  How Sekiro decides a pit fall is a death is not in the script but in the exe's fall-dead routine `FUN_140b975f0`: a filter-15 event wall, or 30 s airborne, or 1500 m below the last stance; an NPC always dies of it (`docs/HIT-FILTERS.md`, #234) [V: decompiled].
- Our current stand-in: `Duel::fall_respawn_depth` puts a character that falls a set depth below its start back at the start, standing, arena mode only (`crates/sim/src/duel.rs` lines 155 and 1397; `arena.rs` line 273) [V].
  `docs/DEATHBLOW.md` and PROGRESS.md mark Wolf death and respawn done for combat deaths; the fall path is separate.
- The exe also has debug-menu names for air docking: `AirDockingPredictionRadius`, `AirDockingPredictionStartTime`, `DockingTranslationOffset_Hang`, `DockingTranslationOffset_Cover`, `DockingStickAngle` [V: strings.tsv; values not extracted].

## 6. Ledge hang, shimmy, climb up

### 6.1 Player HKS

All [V] from `c0000_transition.lua` unless marked.

- Entry from the ground (`BEH_A_GROUND_HANG_START`, line 6199): `env(3044)` (can start ground hang), the wall-hang button (`env(1106, 12)`) or the add-action input, `env(3008)` (debug dash) false, not in sub-weapon guard.
  The script first calls act(3027) every frame while the style allows it, which asks the engine to run the ground-hang test; env(3044) then reports the result; the HUD prompt is act(3030, 20, 12) "Hanging" (ActionGuideParam 20).
  The event is chosen from `hkbGetVariable("HangStartEdgeAngle")` and `env(3046, 1)` (edge variation): back (-45 to 45 degrees), left (45 to 135), right (-135 to -45), else forward, each in a "foot hold" version when the variation is 1 (lines 4573 to 4603).
  Events: `W_StandToHang`, `W_StandToHang1Back/Left/Right` and the `...FootHold...` forms.
- Entry from the air (`BEH_A_AIR_HANG_START`): env(3045) is the air-hang type: 0 disabled, 1 short range, 2 long range [V: define 411 to 413].
  Left side when the angle is in -180 to -45, right in 45 to 180 (define 306 to 309); events `W_FreeFallToHang[FootHold][LongRange][Left|Right]` (lines 4604 to 4650).
  Gating: TAE SpEffect refs `ENABLE_AIR_HANG` (105) or `ENABLE_AIR_HANG_GRAB` (5), or env(1105), plus the wall-hang button held (`env(1108, 12) > 0`).
  Air grab states: `AirHangGrabStart/Loop/End` (clips 217700, 217710, 217720).
- Hang idle: `Hang` style (`STYLE_TYPE_HANG`), clip `HangIdle` 20000 (`HangFootHoldIdle` 20010).
  Each frame the style calls act(3023, 11, 217500) when variation is 0 else act(3023, 11, 217600) [pre-computed root motion of the climb-up clip; line 584 to 589] and `SetVariable("Selector_DockingTargetEdgeType", 0 or 1)`.
- Shimmy: `BEH_A_DOCKING_MOVE_START/STOP` with stick angle `DockingStickAngle` between 30 and 150 degrees (`DOCKING_MOVE_INPUT_ANGLE_MIN/MAX`), blocked by env(3051) (start end) or env(3052) (end end); `Selector_DockingMoveDir` 0 left, 1 right.
  Hang moves: `HangMove` clips (20202, 20203, `HangMoveL/R`), `HangMoveStart/End`, `HangFootHoldMoveStart/Loop/End`.
  The graph has 105 `CustomDockingGenerator` nodes [V: `hkb-dump.exe`], so the engine's docking module (`CSChrDockingModule`, exe string at 0x2a7f1f8) moves the body along the edge while the generator plays the clip [I].
- Corners: `BEH_A_HANG_CORNER_MOVE` with env(3048) outer left, 3049 outer right, 3059 inner left, 3060 inner right; events `W_HangCornerMove[FootHold]Left/Right` and `W_HangInsideCornerMove...` (clips 20402, 20403, 20412, 20413, 20422, 20423, 20432, 20433).
- Climb up: `BEH_A_HANG_CLIMB` on the wall-hang button or jump, requires env(3050); event `W_HangToStand` (variation 0, clip 217500) or `W_HangFootHoldToStand` (217600), then a buffered follow-up.
- Drop: `BEH_A_HANG_FALL` on the dodge button (`env(1106, 5)`): `W_HangToFreeFall` (217520) or `W_HangFootHoldToFreeFall` (217620).
- Hang jump: `HangJumpReady` (201304) and `HangFootHoldJumpReady` (201305), then wall-jump rows (`Selector_WallContactType` F).
- Docking break (`BEH_R_DOCKING_BREAK`) exits the style on damage or contact.
- Attacks from hang exist (`HangSubAttackCombo1`, `HangAttackCombo1` 314010, eavesdrop clips); out of scope for the first pass.

### 6.1 Wall jump and air catch (issue #16)

Built on the existing ground hang, adapted from Fireguy32's RustSekiro wall jump and air catch (FFwLo/RustSekiro PR 1).
- env 3020: `docking::wall_contact` probes 0.8 m out from the chest (1.0 m over the feet) in eight directions round the facing with `Ground::slide`, while airborne and not docked.
  The nearest wall's angle from the facing, positive to the right, goes to the hkb variable `ContactWallAngle`; the script turns it into `Selector_WallContactType` [V: `c0000_transition.lua` `BEH_A_WALL_JUMP`].
  The 0.8 m reach and the probe height are not traced (the exe's contact check).
- The script gates the wall jump itself: TAE ref 104 `ENABLE_WALL_JUMP_CANCEL`, falling (env 3003 < 0), fall height under 20 m, the jump button, and `g_wallJumpCount`, which the script sets to `WALL_JUMP_COUNT_MAX` (1) on `W_WallJumpReady` and resets in `_LandReset`.
  So one wall jump a flight comes from the script, not from an engine counter as in the RustSekiro port.
  The take-off speed is the WallJumpStart clip's own TAE 920 row.
- env 3045: when the script asks (act 3028), `Docking::air_search` takes the nearest hang edge whose lip is 1.0 to 2.6 m over the feet, with the body 0.3 m inside to 2.0 m out from its face and beside its span: type 1 (short) up to 1.0 m out, else type 2 (long).
  The values are Fireguy32's, not traced.
  The catch reuses the ground hang's docking (`Docking::step`), so HangIdle, the climb and the drop work as before.
- Hang edges now include variation 1 (`HANG_EDGE_TYPE_ENABLE_HOLD_FOOT`): the FootHold clips (catch 2172xx, idle 20010, climb 217600, drop 217620).
  The wall after the grapple tree from m11_00 start 0 has such an edge (`h023000` chain 2, lip y -32.8, about 5.7 m over the floor in front).
- FreeFallToHang (217120) is 3.3 s long and hands over to HangIdle at its end; the FootHold catch (217220) is short.
- Hang placement is read from the clips [V: bone and root-motion samples of the cached clips].
  HangIdle (20000) and HangFootHoldIdle (20010) both hold the hands 1.55 m over the root and 0.635 m in front of it, so the root hangs 1.55 m under and 0.635 m out from the lip (`HANG_DEPTH`, `HANG_OUTWARD`; they were 1.5 m and 0.35 m placeholders, which put the body in the face).
  The pelvis is 0.55 m (plain) or 0.18 m (foot hold) in front of the root.
  HangToStand and HangFootHoldToStand (217500, 217600) carry the root 1.10 m forward and 1.60 m up, so the climb ends 0.465 m in from the lip (`CLIMB_INWARD`, was 0.25 m).
- Only the hang starts (2171xx plain, 2172xx foot hold) dock Wolf; the air grab (AirHangGrab*, 2177xx) is a free fall and no longer docks him if a lip comes in reach mid-grab.
- HangFootHoldIdle (20010) counts as the docked idle like HangIdle: standby, and Shift / Space are not held back by the ground TAE windows.
- `PLAY_LOG_DOCKING=1` logs env 3044, 3045, 3020, 3050, the edge variation, standby, the requested buttons and the state each step.
- Not built: hug wall jump (CoverToWallJumpReady), hang jump (HangJumpReady), shimmy, corners, peek, hang and hug deathblows.

### 6.2 TAE on the hang clips

Probed `a000_217120`, `217130`, `217500`, `217600`, `217520`, `218103`, `218490` [V].
Only i-frames (951), engine flags 910 `NewEngineFlag1` and 913 `NewEngineFlag4` (217600 over 0 to 0.167 s), and a turn-speed event (224) appear; there is no SetNoGravity and no 920.
The climb-up clips have ChrActionFlags for cancels from 0.5 to 0.833 s [V: 217500 dump].
So the vertical motion comes from the clip's root motion, which the engine reaches onto the ledge top using the act(3023) pre-computation [I].

### 6.3 Data and params

- Edge data: kind-2 records (section 3). Variation 0 vs 1 = foot hold [I, strong].
- `ObjectParam.isEnableWallJump`, `doesStopPlayerDockedMove` apply to objects (155 and 686 of 1090 rows set) [V].
- `EquipParamGoods.enable_Hang` and `enable_CantClimbHang` gate goods [V: columns exist].
- Ledge reach heights: **[?]**.
  No param gives them; the 3044 and 3045 tests (distance, angle, ledge height) live in the exe (docking module) [I].
- FwLo has nothing here [V].

## 7. Wall hug

- Entry: `BEH_A_COVER_START` needs env(3043), env(3008) false, not force-crouch, SpEffect ref 9 (`DISABLE_COVER_START`) absent, and the wall-hang button (line 6194ff); act(3026) runs the test each frame; prompt act(3030, 10, 12) (ActionGuideParam 10 "Stick", 11 "Leave").
  Style `STYLE_TYPE_COVER`; setting the variable `Selector_Environment` from env(3017): peek types 10 to 13 select `SELECTOR_COVER_ENVIRONMENT_PEAK`, others normal (lines 520 to 523) [V].
- Entry events and clips: `StandToCover2Back` 218103, `StandToCover2Left` 218104, `StandToCover2Right` 218105, `...Left180` 218106, `...Right180` 218107, `MoveToCoverLeft/Right`; idle `CoverIdle` 21000; `CoverMoveStart`, `CoverMove`, `CoverWalkEnd`, `CoverRunEnd`; look-out `CoverActionStart/Loop/End` (peek, `BEH_A_COVER_LOOK_*`, direction from env(3011): 1 left end, else right); end `CoverEnd` 218490 and `CoverEndFromCoverLook` 218491 [V].
- Edge variation rules: 0 no look, 1 free look, 2 left look only, 3 right look only, 10 to 13 the same with peek-move [V: define 446 to 453].
  The look direction restrictions come from env(3011) and env(3017) (lines 6289 to 6300).
- Cover actions: eavesdrop, throw stone, throw pottery, sub-weapon from cover (`CoverEavesdrop*`, `CoverActionItem*`, `CoverActionSubAttack*`).
- Edge data: kind-3 records (section 3), 974 on m11_00 and 1326 on m11_01 [V].
  Door objects carry kind-3 edges with their own variations (door objects 111300 to 111850: `(kind 3, var 0 to 3, hi 1 or 10)`), so door-side wall hug is part of the same data [V].
- Collision: `ObjectParam.isDamageCover` (1 on chests) is a different "cover" meaning [V].
- Unknown: the engine test for env(3043) (distance and facing to the edge); `DockingTranslationOffset_Cover` numeric offset [?].

## 8. Climbing (ladders and cling)

- The player script contains no ladder state [V: zero "ladder" matches in all four decompiled files and the graph outline].
  `ObjectParam.isLadder` is set on 4 rows and none is placed on m11_00 or m11_01 [V].
  Ladders are therefore probably `ObjActParam` interactions with their own animations, like doors [I].
- `STYLE_TYPE_CLING` (10) exists with throw-state setters only; no CMSG states carry it [V].
  `DOCKING_TARGET_TYPE_CLING_POLYGON` (3) and the `.wall` files are the likely data [I].
- There is no free climbing in the script.
  Treat "climbing" as ladders plus cling polygons and defer it.

## 9. Swimming (brief)

- Script styles: `STYLE_TYPE_SWIM` and `STYLE_TYPE_DIVE` (plus aging variants).
  States: `SwimIdle/Move`, `SwimToDive`, `DiveMove*`, `DiveToSwim`, kick moves, `SwimBackStep`, attacks and guards in water (`HKB_STATE_SWIM_*`, `HKB_STATE_DIVE_*`) [V: outline and `c0000.lua` lines 647 to 695].
  Transitions: `BEH_R_STAND_MOVE_TO_SWIM`, `BEH_R_LAND_WATER`, `BEH_R_SWIM_TO_STAND_MOVE`, `BEH_R_DIVE_TO_SWIM`.
  Dive up and down use arms 29 and 30, acceleration arm 28, dive unlock `ACTION_UNLOCK_TYPE_DIVE = 0` [V].
- Engine facts the script reads: env(3025) touching a swim volume, env(3038) can dive, env(3039) can surface, act(3011) underwater, act(3037) on water, SpEffect ref `IN_WATERSIDE_AREA` 110005; speeds come from `_DiveSpeedUpdate` and the clips [V].
- Data: collision hit materials 20 and 21 (flat and walkable water), 22 (dive plane), 35 (walkable slope), 36 (swim slope), 37 and 38 (water with grass walkable and swim), 45 to 47 (underwater floors), 23 and 24 (poison swamp) [V: `HitMtrlParam`].
  Jump rows for water: 221, 240, 241 (dolphin jumps).
  Water landing fall height: 20 m for the long stiff water landing; 80 m death distance on dive planes [V].
- Maps with water in our baseline: m10, m13, m15, m17 (MAP-BASELINE.md notes "water approximate") [V]; materials in `collision.bin` were not scanned [?].

## 10. FwLo's RustSekiro

Checked with `gh api` against `FFwLo/RustSekiro` (MIT; no part of `tools/sekiro-dearxan` was read) [V].

- Has: the fall-control table (`src/data.rs` `FALL_TYPES`, exe 0x143b09ce0), the jump arc from TAE 920 plus fall type, the horizontal air-velocity model with stick (`src/player.rs` about lines 2560 to 2600), plunge deathblow homing (`ChrPhysicsHomingParam 200000300`, `fallCorrectionGuaranteeArrival`), exe-derived gravity 23.52 m/s2 and the 5-jump recording.
- Does not have: grapple, wire, ledge, hang, climb, wall hug, swimming, fall damage, fall death.
  `docs/HANDOFF.md` line 379 lists "wall hug, hanging (limit types)" as undone and `docs/HANDOFF_PLAN.md` line 140 tells agents to skip "swimming, hanging, wire".
  A grep of `src/*.rs` and `docs/kb/*.md` for grapple, wire, ledge, hang, climb, swim, cling, cover and fall damage finds only unrelated hits.
  His player falls to `ground_y + capsule` and has no kill height.

Credit the table and the air-velocity description in `CREDITS.md` and in the code that uses them.

## 11. Implementation plan

Order: air control and fall respawn, ledge hang, grapple, wall hug, climbing, swimming.
Every step: keep `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` clean; build with `CARGO_TARGET_DIR=target/alt` and `-j 6`.
Scripted runs: `sekiro-rs --play --map <id> --no-enemy --script FILE --shots <step>:<path> --exit`, `--spawn x,y,z,yaw,pitch`; script lines are `frames stick_x stick_y button [yaw_offset]` with buttons `jump`, `dodge`, `grapple`, and `-` for none (`crates/game/scripts/m11_ledge_jump.txt`).
The yaw offset turns the camera, which starts at the spawn yaw, so a forward stick runs along world yaw `spawn + yaw_offset` (format and aiming rule in `crates/sim/src/demo.rs`).
Yaw convention: forward is `(-sin yaw, -cos yaw)`, positive yaw turns left [V: `Body::forward`].
Spawn coordinates below are Bevy-space values from `layout.json`.

### Step 1. Air control and fall respawn

- Files: `crates/sim/src/tae.rs` (add `fall_type` to `VelocityChange`, read `fallType`), `crates/sim/src/body.rs` (fall table, stick, per-type gravity, horizontal air velocity), `crates/sim/src/character.rs` (feed the stick vector and fall type into `BodyInput`; env 224 in centimetres; env 3003 in cm/s), `crates/sim/src/player.rs` (env 3003 arm), `crates/sim/src/duel.rs` and `crates/game/src/play.rs` (fall death and respawn in play, not only arenas), `crates/sim/src/fight.rs` (inject damage type 6 or 14 for the death clips).
- Reads: `ChrPhysicsVelocityChangeParam.fallType`, FwLo's four-row table, `HitMtrlParam` `fallDeathDist` and `disableFallDamage` through `Ground::material`.
- Unknown or risky: how stick combines with the launch velocity (modelled, not traced); the unit of `fallDeathDist` (m assumed); the HP formula of fall damage (none; do not invent one, implement only instant death at `fallDeathDist` and the stiff-landing clips); what kills a fall into a void (use a placeholder kill depth below the lowest collision and mark it as such); when `FallHitDmgSmall/Large` fire.
  Changing env 224 to centimetres will newly trigger `LandFreeFallLongStiff` and the 5 m short stiff landing, which existing tests may not expect.
- Scripted tests:
  1. Air steering, `--map m11_00_00_00 --spawn -142.6,-44.0,125.0,180,0 --no-enemy` with a copy of `m11_ledge_jump.txt` that holds the stick left for 20 frames after the jump.
     Expected: landing point sideways from the straight jump by roughly 0.1 to 1 m, forward speed decays by 0.1 m/s2 only; a unit test in `body.rs` checks the table arithmetic.
  2. Stiff landing, `--map m11_00_00_00 --start 4 --no-enemy`, walk off the ledge the audit found (about 16.8 m drop, MAP-BASELINE.md) with `PLAY_LOG_ALL=1`.
     Expected: env 224 about 1680 on landing, `LandFreeFall` (200020) with the short stiff variant; a 20 m or higher drop plays `LandFreeFallLongStiff` (200021).
  3. Fall death and respawn, `--map m25_00_00_00 --start 0` and walk into the ravine (MAP-BASELINE.md: no ground supports it).
     Expected: Wolf enters `FallDeathStart` (199100) at the kill depth, the death flow runs, and he respawns at the start within the existing respawn delay.

**Status (2026-10-11):** Step 1 is implemented in the simulation and map play.
Velocity changes retain `fallType` for the airborne phase; the FwLo table drives gravity, decay, braking, across steering and the along-speed cap.
HKS env 224 returns centimetres and env 3003 returns centimetres per second.
Landing death reads the landing material's `fallDeathDist` and `disableFallDamage`, injects DEATH_FALLING (6), and uses the existing six-second respawn path.
A fall into the void, past a clearly marked placeholder of 40 m below the lowest supporting collision (or below spawn without collision), returns Wolf to where he last stood on collision instead of killing him; the fall return's HP cost is untraced, so none is taken.
The camera snaps back with Wolf on respawn.
The m11 straight jump lands at (-142.60, -42.60, 131.24), while `m11_air_steer.txt` lands at (-141.82, -42.68, 131.20), a 0.78 m sideways offset.
The steering replay waits through JumpReady and launch before holding left for 20 airborne frames, to avoid changing the launch direction.
The start-4 replay `m11_stiff_landing.txt` walks backward for 180 frames and reaches `LandFreeFall` (200020) with env 224 = 1284.9 cm; the intermediate landing splits its 17.75 m total descent into separate falls.
A deterministic 16.8 m sim fixture checks env 224 = 1680 and the same short landing clip.
The original audit's 270-degree route continued to a roughly 169 m landing and lethal fall, so its 16.8 m intermediate drop is not a reproduced single landing height.
The m25 start-0 replay `m25_fall_death.txt` falls into the ravine (about 35 m of void below the last ledge) and is returned to the last safe stance at (-68.25, 167.81, -7.61) without dying.
Screenshots were reviewed under `target/alt/mobility/`: `straight.png`, `steer.png`, `stiff.png`, `fall-death.png` and `respawn.png`.
Final captures add `--simple-ai` to bypass the default Lua driver's automatic session report writes.
One concurrent map replay hit Vulkan DeviceLost; its steering capture was rerun alone and passed.
The existing camera chase still lags during very fast descents; respawn now snaps the camera and starts a fresh pose at spawn.
`fallDeathDist` in metres remains an assumption from the research, the air-stick combination remains modelled, and authored void kill/return volumes remain undecoded.
Update 2026-10-16: the authored return trigger is decoded for one case: map collision parts with hit filter 15 are event walls whose contact fades Wolf out and starts the fall return (`docs/HIT-FILTERS.md`); the placeholder depth stays for pits without one.
The Owl arena fall test now expects the landing death on the ground 62 m below the roof and the respawn at the start.
Since 2026-10-16 that "ground" is known to be the filter-13 event wall `h900011`, which the player never stands on; the test now expects the filter-15 fall return to the start instead (`docs/HIT-FILTERS.md`).

### Step 2. Ledge hang

- Files: new `crates/formats/src/edge.rs` (parser), `crates/extract/src/maps/mod.rs` (write map edges transformed to Bevy space into `cache/maps/<id>/edges.bin` or `layout.json`), `crates/sim/src/` new `docking.rs` (edge manager: nearest hang edge, angle, tops, ends, corners), `crates/sim/src/player.rs` (env 3011, 3017, 3043 to 3052, 3059 to 3061; act 3026 to 3028 trigger the searches, 3030 raises a prompt), `crates/sim/src/input.rs` and `crates/game/src/play.rs` (bind arm 12), `crates/sim/src/body.rs` (docked body: position follows the edge, not collision).
- Reads: kind-2 edge records (variation 0 and 1), collision floor above the edge for env 3050, `HangStartEdgeAngle` hkb variable (script reads it, engine writes it), clips 217xxx, 20xxx.
- Unknown or risky: all distance and angle thresholds for env 3044 and 3045 (exe); the transform of docked position relative to the edge (`DockingTranslationOffset_Hang`); act(3023) root-motion scaling; whether kind 2 also carries "air hang" targets (env 3045 types 1 and 2); the meaning of flags bytes 2 and 3.
  Start with variation 0, ground entry, no corners.
- Scripted test: `--map m11_01_00_00 --spawn 82.9,29.8,254.3,15,0 --no-enemy` (start 2 of the castle map; the nearest hang edge `h019900` chain 7 runs from (77.7, 29.5, 242.8) to (83.8, 29.5, 249.3), 5 m ahead; geometry not yet checked on screen, so first log env 3044 along the walk).
  Script: walk forward 60 frames, release, press arm 12, wait, press arm 12 again.
  Expected: env 3044 true near the edge, `W_StandToHang*` fired, `HangIdle` (20000) active, the climb-up press plays `HangToStand` (217500) and Wolf ends standing within 0.3 m of the edge on the upper floor.

**Status (2026-10-11):** Ground entry, idle, climb and drop are implemented for kind 2, variation 0.
The parser retains both endpoints, the third vector, chain, owner and all flag bytes.
The extractor writes placed map records to `cache/maps/<id>/edges.json`; normals are absent in the source map records, so the runtime derives the outward side from collision.
For an edges-only refresh, set `SEKIRO_EDGES_ONLY=1` and run `target/alt/release/sekiro-extract.exe map <id>`.
The normal map export also writes edges.
Ground entry supports approaching the edge from the upper floor, or from a lower floor with hands near the top.
The upper-floor case is necessary for the researched roof edge, whose top is near the standing player's feet.
HKS still selects the entry event using `HangStartEdgeAngle`, enters `HangIdle`, and selects climb or drop.
Acts 3026 to 3028 enable the corresponding precomputed search answers for that tick; cover and air entry remain disabled.
Env 3011 reports the selected segment's endpoint proximity, 3017 and 3046 report variation, 3044 reports the ground search, and 3050 requires collision above the active edge.
Both docking movement ends are blocked, all corner queries return false, and safe-position return 3061 returns NONE (-1).
While docked, position follows the selected closest point and hang offset, with gravity and collision pushes bypassed.
Ground entry immediately anchors the body; the entry clip's root warp timing and rotational alignment are not traced.
Climb progress comes from the real 217500 root track projected onto its total displacement and fitted to the upper floor; the body settles on that floor when the clip exits.
This is an approximation of act 3023 precomputation, not a traced docking generator.
Drop releases docking at `HangToFreeFall` (217520) and resumes normal gravity.
HangIdle accepts action, jump and dodge through its HKS style even though its TAE carries only turn/prosthetic flags rather than ground execute windows.
Arm 12 has no traced TAE execute-window mapping, so its request is offered directly to HKS.
Live action is E or gamepad Y; scripts use `action` through the play/input extension.
Act 3030 supplies the Hang, Climb and Drop HUD guides.

Placeholder constants, not traced from the exe: horizontal reach 1.0 m, upper-floor height tolerance 0.4 m, lower-entry hand height 1.5 m with the same tolerance, facing limit 45 degrees, hang offset 1.5 m down and 0.35 m outward, climb destination 0.25 m inward, floor-top tolerance 0.3 m, endpoint margin 0.2 m, outward floor probe 0.6 m, minimum outward drop 0.5 m, upper-side tolerance 0.1 m, and fallback climb duration 0.833 s when root motion is missing.
The constant offset is the untraced `DockingTranslationOffset_Hang` approximation; no corners, shimmy, variation-1 foot hold, or air grabs are claimed.
Upper floor presence is checked, but full standing-capsule and hand IK clearance are not implemented.

The requested start-2 replay with yaw 15 and 60 forward frames never set env 3044 and stopped at (83.0565, 29.0828, 251.6671), outside the researched segment's range.
The successful route uses the upper side of the same segment: `--spawn 83.307,30.6,243.649,133.216,0` and `crates/game/scripts/m11_01_ledge_hang.txt` (30 forward frames, 15 released, action, 150 idle, action, 120 released).
Env 3044 becomes true at steps 43 to 45; step 45 fires `W_StandToHang` (217100), the idle clip is 020000, and step 196 fires `W_HangToStand` (217500).
The selected point is (80.7562, 29.5256, 246.0457), the hang body position is (80.5011, 28.0256, 246.2854), and the final standing position is (80.9384, 29.5759, 245.8745), 0.255 m from the edge.
Logs: `target/alt/docking-original-play.log`, `docking-hang-play.log` and `docking-side-fallback-play.log`.
The final prompt-enabled replay is `target/alt/docking-final-play.log`.
The companion `m11_01_ledge_drop.txt` fires `W_HangToFreeFall` at step 196, releases docking immediately, and ends standing on the lower roof at (80.5011, 20.9834, 246.2854); see `target/alt/docking-drop-play.log` and `target/alt/mobility/hang-drop.png`.
Reviewed screenshots: `target/alt/mobility/hang-side.png` and `hang-climbed-side.png` show hands on the eave, the body outside the wall, and feet on the upper floor after climbing.
The real-data regression also checks both hand bones are within 0.1 m of collision at the grab.
The approach-side `hang-idle.png` is obscured by the roof; `--extra hang-view` supplies an outside inspection camera through play.rs only.
One capture attempt encountered a concurrent environment-probe cubemap validation error (one layer supplied to a six-layer view); the successful outside captures use `--no-env-probe`.
The exported player model still carries its sword in the hand while hanging; weapon location changes are outside this docking slice.
Validation: `cargo fmt --all --check`, workspace Clippy with `-D warnings`, and the alternate release game build pass with four jobs.
The final release sim/formats suite passes 401 tests with two pre-existing ignored tests, including edge counts, docking, mobility and owl_arena.
Game unit tests pass; the full game suite retains the permitted pre-existing `map_tangents` failure (only 0.004 of tangent directions agree).

### Step 3. Grappling hook

- Files: `crates/formats/src/edge.rs` (shared with step 2), `crates/extract/src/maps/mod.rs` (write grapple points: object transform applied to the object edge, variation, model, placed name), `crates/sim/src/` new `wire.rs` (target search with `WirePointSearchParam` through the camera's view box, section 4.6, and travel), `crates/sim/src/tae.rs` (decode 900 to 905 and 921, hook visibility 715), `crates/sim/src/body.rs` (fall type 2, travel velocity), `crates/sim/src/player.rs` (env 3000, 3003, act 3018, 3023, 3035), `crates/game/src/` (a target marker for `targetMarkerType`, hook model on dummy 144, optional rope line).
- Reads: object edge files, MSB object parts, `WireVariationParam`, `WirePointSearchParam` row 0 (3 to 15 m act, 25 m find, each plus the variation's `extendWireActionDistance`), `ChrPhysicsHomingParam 300000000 + offset`, velocity rows 1100 to 1102, TAE of 202000 to 202710.
- Unknown or risky: how the engine picks the search row and wire set (section 4.3); travel kinematics (what TAE 900 and 901 actually drive; the homing parameters need a fit to FwLo-style recordings); the `fallCorrectionGuaranteeArrival` 1 vs 2 difference; the screen-space filter box (needs the camera); target selection and soft lock; landing on rideable vs non-rideable points (variation 20 homes to an offset below and beyond the point); event-disabled objects; `GetWireMoveStartIndex` inputs.
  Ship in two slices: (a) ground shot to a rideable Shachi with a scaled root-motion arrival, (b) air shot and chained hook.
- Scripted test: `--arena owl --passive --god --spawn 103.5,54.0,243.6,139,0 --script crates/game/scripts/grapple_shachi.txt --shots ... --exit` (castle top, map within 60 m).
  The point is `o003610_0014` at about (97.1, 52.8, 251.0), 9.9 m from the spawn (inside the 3 to 15 m window), yaw 139 faces it.
  Script: `30 0 0 -`, `1 0 0 grapple`, `120 0 0 -`.
  Expected: env 3000 true before the press, `W_GroundWireShoot` then `GroundWireMoveStart` (clip 202010 to 202012), TAE 901 window starts, and about 1.5 s later Wolf stands within 0.5 m of the point.
  Negative test: `--spawn 103.5,54.0,243.6,-41,0` (facing away) must not fire.
  Counts to assert in a unit test: `layout` reports 172 objects and 224 edges on m11_01, 237 and 360 plus 4 on m11_00.

**Status (2026-10-11):** Slice (a), ground shot and arrival, is implemented.
The shared edge exporter writes `grapple.json` using the same mirrored part matrix as collision.
The two m11 maps were re-extracted: castle has 172 grapple objects / 224 points; outskirts has 237 objects / 360 points plus four unplaced collision records.
The raw/MSB census and extracted census have regression tests.
`o003610_0014` transforms to (97.10218, 52.843166, 250.97510), with collision floor 52.54494, 0.29823 m below.
The runtime reads WirePointSearchParam row 0's near/far/find distances and its filtering and soft-lock boxes, and WireVariationParam's extension, marker type and homing id offset.
Since issue #178 the search is the exe's (section 4.6): the camera's back filter, view box, front plane and soft lock, survivors ranked by the angle from the screen centre, and a chest ray to the authored point.
The camera is the follow camera rebuilt in the simulation from the step's `InputFrame::camera_yaw` and `camera_pitch` (level when a caller gives no pitch).
WireVariationParam's recognition angles are not applied (see [FIDELITY.md](FIDELITY.md#grapple-target-search-issue-178)), and region-driven wire-set changes are not reconstructed.
The LOS ray ignores only the last 0.1 m at the anchor, as a placeholder allowance for its own surface.
Env 3000 exposes a selected target; env 3003 retains the step-1 cm/s answer.
Act 3018 tracks the wire action flag, cleared on release; act 3023 pre-evaluates the requested clip's wire interval as a travel-window fallback, and act 3035 marks the wire preference.
This slice searches kind-1 points only, with no docking/chained-hook bias yet.
HKS selects GroundWireShoot and GroundWireMoveStart without injected events.
TAE 900 to 905 and 921 retain their timings and raw argument bytes and are logged with `PLAY_LOG_WIRE=1`.
TAE 901 supplies the travel interval; 920 retains the real row-1100 fall type 2.
Travel currently fits a straight path to the 901 interval, 0.5 s on clip 202012, with gravity suspended while wire owns movement.
This linear fit is a placeholder for native act-3023 root pre-evaluation and homing kinematics, not a traced root-motion warp.
Missing travel windows fall back to distance / 20 m/s.
Collision sweeps at feet and capsule-centre heights stop travel on a hit and release to normal physics; a complete swept-capsule solver remains unimplemented.
Rideable arrival is at the authored point, then the HKS landing clip settles onto nearby collision.
Nonrideable arrival reads ChrPhysicsHomingParam targetOffsetX/Y/Z, using a placeholder approach-facing frame for horizontal offsets.
Variation 20 therefore arrives 2.5 m below and 7.5 m beyond the anchor; its release and retained downward velocity are tested synthetically.
Air shots and chained hooks remain disabled for this slice.
F and the existing L2/LT binding fire grapple; R2/RT is already prosthetic use and remains so.
The selected target has an 18 px projected UI ring, green for rideable and amber for nonrideable.
Hook-model attachment and the optional rope are not implemented.

The requested (103.5, 54.0, 243.6), yaw 139 replay does not select a target: collision blocks the chest ray at 4.34167 m of its 10.04281 m path.
Its negative yaw -41 replay also stays Idle with env 3000 false.
The collision-backed positive route is `--spawn 104.03038,53.234768,254.9751,60,0` with the same `grapple_shachi.txt` script.
Use `--arena owl --simple-ai --passive --god --objects --enemy-distance 1000` to show the ornament and keep the passive Owl out of the capture.
Env 3000 is true before step 30; that press fires W_GroundWireShoot / 202000.
Step 61 enters GroundWireMoveStart / 202012, and step 90 reaches the anchor exactly: 0.5 s travel and 1.0 s after the press.
LandWireMove / 202035 follows, and the final body position is (97.10218, 52.56432, 250.97510), 0.27885 m from the anchor, grounded.
The deterministic roof test also verifies yaw -41 at the original spawn and the obstructed yaw-139 original spawn.
Turning away on the clear roof can select another nearby point that is then in the camera's view; it must not be treated as a universal negative case.
Logs and reviewed before/mid/after captures are under `target/alt/mobility/grapple-*`.
The captures show the ornament, airborne movement and arrival on the roof, though the native hook model and rope remain absent.
Validation passes with `CARGO_TARGET_DIR=target/alt` and four jobs: formatting, workspace/all-target Clippy with `-D warnings`, all requested release sim/formats/extract tests (418 passed, two existing ignored) and the release game build.
The final test log is `target/alt/mobility/grapple-suite-v2.log`; final scripted logs are `grapple-final-v2.log` and `grapple-negative-v2.log` in the same directory.

### Step 3(b). Solved flight, air shots, chains and equipment (2026-10-11)

This section supersedes slice (a)'s linear travel and its air/chain/equipment limitations.
FwLo's MIT research in `target/alt/fwlo/grapple.md`, sections 3 and 6, supplies the engine facts used here; `target/alt/fwlo/docking.md` supplies docking context.
Our implementation is original Rust, with no FwLo source copied.
The existing FwLo credit in `CREDITS.md` includes these facts.

TAE 920 row 1100 launches the solved arc under fall type 2's total gravity of 19.6 m/s2.
The initial velocity is displacement divided by flight time plus half the gravity times flight time in Y.
Each tick recomputes horizontal velocity from the remaining displacement and remaining time, and integrates Y with the exact constant-acceleration displacement.
The arrival time comes from the playing clip's TAE CameraSlot 19 start minus its row-1100 launch time: 0.833333 s on 202011/202012, 0.666667 s on 202010 and the air/chain starts.
This preserves the authored camera-slot boundary without fitting flight to TAE 901.
The synthetic 10 m horizontal / 5 m upward fixture arrives within 0.000005 m at 0.83 s and rises above the straight-line midpoint.
Collision rays at the four existing capsule heights stop both the 904 slide and arc movement; the ray endpoint has a 1 mm numerical allowance to prevent crossing a wall at an exact tick boundary.
This remains a set of ray sweeps, not a complete swept capsule.

TAE 900 moves the rope tip using its float speed in m/s; the authored value is 50.
TAE 901 holds the tip at the hook, and TAE 902 re-aims Wolf during its active window.
The chain shots use the authored 2700 deg/s turn rate, versus 1080 deg/s for ordinary shoot windows.
TAE 904 applies row 1101's +30 horizontal / +30 vertical blend before launch.
Its horizontal curve is linear; the EaseIn curve id 10 is currently fitted with a quadratic, since the copied notes identify the curve id but do not give its evaluation formula.
TAE 905 suppresses falling only during its active air/chain shoot window.
TAE 921's wire homing selection uses the selected variation's 300000000-based homing row and its retained offset.
No-ride offsets are transformed in the approach frame; variation 20 flies 7.5 m beyond and 2.5 m below the hook and retains its velocity for the subsequent wire fall.

Search now runs while jumping, falling and flying, excludes the currently hooked point, and offers the next candidate to HKS.
The engine writes WireDramaticAngle from the hook elevation; HKS retains its >50 / >15 degree WireMoveStartIndex selection rule and stores the ground selector at shoot entry.
Air grapple requests can execute in the jump's TAE 119 window as well as the existing movement window.
HKS drives AirWireShoot, WireMoveWireShoot and WireMoveWire2Shoot, and their real move-start selectors.
The tests reach both chain shoot clips, 202600 and 202700, without injected behavior events.
Leaving wire movement for an attack, guard or jump clip releases wire control and preserves the current momentum for the ordinary body path.
Attack and guard tests enable their actual unlocks 14 and 15; the controller does not grant these skills.
Env 3003 remains cm/s, while env 248 can predict contact during the next downward tick independently of env 201's completed landing.
The fast no-ride fixture reaches WireFallLandReady at -2078 cm/s, then LandWireMove on contact.

A ridden landing supplies a virtual perch within 0.6 m horizontally of its landing spot, because the grapple point often sits beside the object's walkable top rather than on it.
When terrain is within 0.6 m vertically of the spot it supplies the perch height instead, so ordinary roof landings settle onto collision.
Walking outside the perch footprint resumes falling unless there is collision under Wolf.
The perch ends once Wolf is outside that footprint (walked or knocked off, docked, warped) or fires the next grapple shot, so it never holds him up again at that spot (#230).
A jump straight up keeps it, since he lands back on it.
This is FwLo's RustSekiro rule for `Player::perch`; the real game has no perch, as it stands Wolf on the ridden object's own collision.
Since 2026-10-16 the play world also loads the placed objects' own collision (`docs/OBJECT-COLLISION.md`), so after landing on a rideable pine Wolf walks along its trunk.

The model exporter now writes `cache/models/wp_a_9500.glb` and `cache/anim/wp_a_9500/` when exporting Wolf.
It exports the original WP_A_9500 model, its 80-bone skeleton, 77 mesh joints and all 16 available anibnd clips, including 202000 through 202710 and a999.
The actual decoded chain ends at BornEnd; the copied notes' Born79 name is not present in our skeleton.
TAE 715 model type 3 supplies dummy 144 and visibility.
An opt-in `--extra skinned-rope` preview samples the matching equipment animation, aims the sampled chain and scales the complete model by distance / 4.0.
Visual QA found that this does not correctly warp the animated cable to the hook: the hook size changes and large curls remain beside Wolf.
The default renderer therefore draws the allowed simple line directly from dummy 144 to the simulated tip during the TAE 715 window.
The full model and clips remain exported for completing native chain-only deformation and TAE 717 placement later.
Native skinned rope placement is an explicit gap, and the rejected model preview captures are not evidence of fidelity.

HangIdle 020000 carries TAE 720 byte pairs 01/01 and 00/01 throughout the clip, with the same sheath windows in the early climb; it carries no 233 draw-mask or 715 weapon-location event.
The exported katana was permanently attached to dmyblade:1, explaining the blade across the roof in the original screenshot.
The renderer now applies the right-hand sheath window to the existing dmy:147 scabbard frame and restores the hand attachment outside it.
This is an interpretation of the authored 720 sheath command; its full CultType enum and all equipment states are not traced.
The matched before/after files are `target/alt/mobility/grapple2-hang-before.png` and `grapple2-hang-after.png`.
The after view has clear hands and no sword across the tiles; the climb replay still finishes on the upper roof.

The scripted ground, jump-air and chain routes use `--arena owl --simple-ai --passive --god --objects --enemy-distance 1000 --spawn 104.03038,53.234768,254.9751,60,0` and the three `grapple2_*.txt` scripts.
The chain's second hook is o115790_7001 at (93.31853, 53.09239, 254.30566), found in the exported m11_01 grapple points.
Final run logs and screenshots use the `target/alt/mobility/grapple2-` prefix.
The ground and air routes land on the Shachi roof at (97.10218, 52.56432, 250.97510), 0.279 m from the anchor.
The chain reaches the second ornament and remains perched within 0.05 m of its landing spot.
The ground replay enters 202000 at tick 30, 202012 at tick 61 and LandWireMove / 202035 at tick 141.
The air replay jumps at tick 30, enters AirWireShoot / 202100 at tick 55, 202112 at tick 84 and LandWireMove at tick 134.
The chain replay enters WireMoveWireShoot / 202600 at tick 120, 202610 at tick 139 and LandWireMove at tick 189.
The final line-renderer logs are `grapple2-ground-line.log`, `grapple2-air-line.log` and `grapple2-chain-line.log`.
All nine matching `grapple2-{ground,air,chain}-{rope,flight,roof,perch}.png` captures were inspected.
The chain's rope and flight views show the cable leaving the prosthetic and reaching the ornament, with a plausible airborne arc.
The ground and air views show plausible flight above the tiles and stable roof landings; their cable is mostly occluded by the roof and Wolf from this camera.
The perch view shows Wolf standing on the second ornament, without drifting onto the lower roof.
The remaining search-box, region-driven wire-set, object enablement, enemy hook, break-fall and swim-wire gaps are unchanged.

Final validation passes with `CARGO_TARGET_DIR=target/alt`, `CARGO_BUILD_JOBS=6`, `-j 6` where supported and offline Cargo: `cargo fmt --all --check`, workspace/all-target Clippy with `-D warnings`, the release sim/formats/extract suite and the release game build.
The suite reports 433 passed and two existing ignored tests, including all 15 wire tests; the standalone wire diagnostic replay also passes all 15.
Final logs are `grapple2-fmt-pass.log`, `grapple2-clippy-final-v3.log`, `grapple2-suite-pass.log`, `grapple2-build-pass.log` and `grapple2-wire-final.log` under `target/alt/mobility/`.
Concurrent Genichiro and shadow-budget failures cleared on retry without modifying their implementation files.
Integration edits outside the assigned files are limited to `sim/src/character.rs` (wire advance, angle, landing-ready and perch hooks), `game/src/character.rs` (bind a rig by its own first bone rather than Wolf's Master) and `extract/src/models/mod.rs` (tool module declaration and Wolf-export call).

### Step 4. Wall hug

- Files: shares `edge.rs` and `docking.rs` from step 2; `crates/sim/src/player.rs` (env 3043, 3011, 3051, 3052, 3017 for kind 3); `input.rs` arm 12 as in step 2.
- Reads: kind-3 edge records and their variation (0 to 3, 10 to 13), the `Selector_Environment` and `Selector_CoverLookDir` variables, clips 218103 to 218107, 21000, 218490.
- Unknown or risky: the engine test for env(3043); wall-contact placement; `DockingTranslationOffset_Cover`; peek (look) movement needs the camera; door edges (doors can open or close at runtime).
  Start with idle and exit, then move along the edge, then look-out.
- Scripted test: `--map m11_00_00_00 --spawn 174.4,-69.1,68.2,-33,0 --no-enemy`.
  The nearest cover edge `h018700` chain 6 (variation 0) runs from (175.9, -68.7, 64.6) to (176.3, -68.7, 66.5), 2.6 m away.
  Script: walk forward until the wall, press arm 12.
  Expected: env 3043 true at the wall, `StandToCover2*` plays (218103 to 218107), then `CoverIdle` (21000) with `Selector_Environment` normal; a second press plays `CoverEnd` (218490).

**Status (2026-10-11):** Kind-3 cover supports variations 0 to 3 and 10 to 13 through the existing HKS.
The nearest reachable, faced edge wins the shared action-button search, so E / gamepad Y selects cover or ledge hang by the candidate kind.
Act 3026 enables env 3043 for the tick; acts 3027 and 3028 retain the existing hang search gates.
Env 3017 supplies the authored variation and env 3011 supplies the endpoint in the docking frame, which faces the wall for hang and cover alike.
With Wolf's back to the wall, the script's left is his own right: CoverMoveL (a000_021402) moves the root to source -X while HangMoveL (a000_020202) moves it to source +X, and the m11_00 left-look-only wall h018700 chain 1 has its open corner at its left end in this frame (issue #172).
Env 3051 and 3052 test capsule sliding toward the respective end for an obstacle rather than treating every cover end as blocked.
HKS sets `Selector_Environment` to normal or peek movement and `Selector_CoverLookDir` from the permitted endpoint.
Entry uses `CoverStartEdgeAngle`, plays StandToCover2Back / 218103 for the verified approach, then CoverIdle / 021000.
Docked movement projects the playing movement clip's root distance onto the selected segment, follows its supporting collision floor and clamps to its endpoints.
The camera-relative stick supplies `DockingStickAngle`, with the HKS convention of positive right in the same wall-facing frame, and ordinary free-movement turning is suspended while docked.
Sliding accepts only the HKS along-wall input cone, strictly between 30 and 150 degrees from Wolf's facing, so wall-normal input cannot slide through a residual movement clip.
HKS handles the cover move start/loop/end states and the look restrictions for each variation.
The real look-out clips are 205010/205011, 205020/205021 and 205030/205031; they retain docking at the endpoint and do not slide along the wall.
Action plays CoverEnd / 218490 or CoverEndFromCoverLook / 218491 and releases docking to ordinary locomotion.
Stick away offers the same action request to HKS rather than injecting an exit event.

FwLo's MIT `target/alt/fwlo/docking.md` reports the graph's initial `DockingTranslationOffset_Cover` as (0, 0, 0.7), with its edge-frame sign unresolved.
`DOCKING_TRANSLATION_OFFSET_COVER` applies that magnitude 0.7 m outward from the authored segment as a documented placeholder frame interpretation.
Other placeholder thresholds are horizontal reach 1.0 m, facing within 45 degrees toward the wall, supporting-floor height difference at most 0.4 m, edge-to-floor height tolerance 0.8 m, inward wall probe 1.0 m, probe lift 0.6 m above the ordinary capsule, collision displacement tolerance 0.05 m, endpoint/obstacle probe margin 0.2 m and away-stick cone 30 degrees.
The lifted probe rejects kerbs and floor contacts, and a zero-motion capsule slide rejects placements inside collision.
Entry anchors immediately rather than reconstructing the docking generator's blend-in/root warp.
Sliding uses the authored root-distance magnitude along the segment rather than a traced native generator evaluation; peek retains the same anchor and plays the authored pose without a reconstructed camera-dependent docking offset.
Door activation/object collision, connected-segment corner transfers, cover attacks/items/eavesdrop, native blend/IK and camera-dependent peek displacement remain gaps.

The requested chain-6 approach was also inspected from the outside: collision still makes env 3043 true and it enters cover, but the rendered position is beside a low terrace edge with no full wall behind Wolf, and variation 0 disables look-out.
`target/alt/mobility/cover-requested-final.png` records this collision/render geometry discrepancy and is not positive visual evidence of a wall hug.
The verified full-wall route is nearby on `h018700` chain 1, variation 2, from (170.90704, -68.97208, 62.02407) to (173.68359, -68.71230, 61.28944).
Use `--play --map m11_00_00_00 --spawn 172.70456,-68.85,63.21088,14.819877,0 --no-enemy --simple-ai --script crates/game/scripts/m11_cover.txt --exit`.
For outside visual inspection, add `--extra hang-view --no-env-probe --enemy-distance 1000`.
The script walks toward the wall, releases the stick, presses action, waits in idle, slides left to the permitted endpoint, holds the peek and presses action to exit.
The collision-backed regression enters 218103 at tick 28, idles at tick 51, moves at tick 176, starts the left peek at tick 209, loops in 205020 at tick 232, exits in 218491 at tick 386 and restores Idle at tick 405.
Its docked displacement is approximately 1.26 m and zero-motion capsule checks stay below 0.05 m throughout.
Synthetic HKS tests cover all eight variations' look restrictions and environment selectors, action and away-stick exits, retained peek docking and restored locomotion.
Direct tests cover endpoint clamps, missing walls, bad heights, reach/facing, unsupported variations and independent end obstacles; the ledge-hang tests remain unchanged.
The final `--simple-ai` replay matches those ticks and restores Idle at (173.58183, -68.71567, 61.57926).
Reviewed captures are `target/alt/mobility/cover-idle.png`, `cover-slide.png`, `cover-peek.png` and `cover-exit.png`, with the final log in `cover-final.log`.
They show Wolf's back facing the visible building wall, his body outside it, lateral movement and the endpoint lean pose followed by ordinary standing.
The boots appear slightly above the rendered snow in parts of the cover pose, and the lean is not proof of the native camera-dependent lookout displacement; floor/render alignment and native foot IK remain visual gaps.
The existing inactive-enemy HUD label remains visible in the no-enemy replay.
Final validation passes offline with `CARGO_TARGET_DIR=target/alt`, `CARGO_BUILD_JOBS=6` and `-j 6` where supported: `cargo fmt --all --check`, workspace/all-target Clippy with `-D warnings`, the release game build and the complete release sim/formats suite.
The suite reports 431 passed and two existing ignored tests, including five cover tests and four unchanged ledge-hang tests.
Final check logs are `cover-fmt-latest.log`, `cover-clippy-latest.log`, `cover-build-latest.log` and `cover-suite-pass.log` under `target/alt/mobility/`.
Play integration changes are limited to cover action-guide text and docking diagnostics; existing E / gamepad Y bindings and script `action` input are reused.

**Cover sword follow-up (2026-10-11):** The reviewed `cover-peek.png` exposed a drawn katana in Wolf's right hand.
Unlike HangIdle, CoverIdle / 021000, CoverActionLoop / 205020 and StandToCover2Back / 218103 have no TAE 720 sheath command [V: local `tae_probe`].
Active kind-3 docking now uses the existing hang fix's dmy:147 scabbard attachment, restoring the hand attachment on release.
This cover-mode sheath retention is an inferred equipment policy, not an authored 720 event added to those clips.
The matched `target/alt/mobility/cover-sword-before.png` and `cover-sword-after.png` at step 270 show the blade removed from the right hand; the replay still exits normally.
Both captures were inspected; concurrent fog changes alter the distant background, but the pose and camera match.

### Step 5. Climbing

- Files: `crates/formats/src/` new `wall.rs` only after the `.wall` layout is decoded; ladders through the object-action path (`ObjActParam`) once a ladder placement is found.
- Reads: `.wall` files, `ObjectParam.isLadder`, `ObjActParam`, ActionGuide.
- Unknown or risky: everything; `STYLE_TYPE_CLING` has no states, ladders are absent from m11.
  Do not start before steps 1 to 4 ship.
- Scripted test: none until a ladder or cling surface is located in `m10`, `m13`, `m15`, `m17`, `m20` or `m25` (scan `ObjectParam.isLadder` models in each MSB first).

### Step 6. Swimming

- Files: `crates/sim/src/collision.rs` (water hit materials 20 to 24, 35 to 38, 45 to 47 as a volume and surface height), `crates/sim/src/player.rs` (env 3025, 3038, 3039, act 3011 and 3037), `crates/sim/src/body.rs` (buoyant movement, dive up and down), `crates/sim/src/character.rs`.
- Reads: `HitMtrlParam`, clips for `SwimMove` and `DiveMove`, `SP_EF_REF_IN_WATERSIDE_AREA` 110005.
- Unknown or risky: speeds (from root motion and `_DiveSpeedUpdate`), water surface height source (collision plane or a region), fall into water (`W_LandWaterFreeFall*`, long stiff at 20 m), underwater camera and fog.
- Scripted test: pick the first start in `m13_00_00_00`, `m15_00_00_00` or `m17_00_00_00` whose collision under the feet reports material 22, 36 or 38 (not yet scanned), jump in, and expect `W_LandWaterFreeFall` then `SwimMove` states.

**Status (2026-10-11):** Surface swimming, diving, vertical movement, surfacing and shore exit are implemented through the shipped HKS.
HitMtrlParam names were verified locally: 20/21/35/37 are walkable water, 22/36/38 are deep surfaces, 23/24 are walkable poison swamp, and 45/46/47 are solid underwater floors.
`Ground::water_surface` queries authored water triangles independently of terrain, returning their interpolated height and material.
The collision world excludes only 22/36/38 from solid floor, capsule pushes and camera/LOS rays; ordinary water, swamp and underwater floors retain solid collision.
Water surfaces below solid terrain do not start swimming, and underwater-floor materials alone do not invent a surface or volume.
Env 3025 reports deep-water contact, 3038 permits diving and 3039 reports surface proximity; waterside reference 110005 is supplied near water.
Acts 3037 and 3011 select surface or underwater body integration for that tick.
Surface movement pins the body origin to the collision plane, while horizontal swimming and vertical diving use authored root deltas without a hand-tuned speed.
The HKS `_DiveSpeedUpdate` retains its own MoveSpeedIndex selection.
Underwater env 201/248 report seabed contact rather than buoyant support, allowing HKS down movement until the solid floor.
Dive movement clamps to that floor and the surface, while water jump launches retain their authored TAE velocity changes and normal airborne integration.
TAE flag 28 alone no longer restricts inputs: SwimIdle's persistent physics marker previously prevented moving after surfacing.
Arm 30 is bound to C / gamepad L3, arm 29 to Space / A and arm 28 to Shift / B, sharing the existing logical crouch, jump and dodge buttons.
Scripts accept `dive`, `surface` and `swim-boost` aliases; rising with Space can select the HKS dolphin-jump surface exit.
Swim arms have no reconstructed TAE execute-window mapping and are offered directly to HKS, which retains its state gates.
Shallow-water transitions restore ground locomotion, and the real map route jumps onto the shore using SwimJumpReady and LandSwimKickMoveJump.
Falling bodies contact the water plane before the seabed, preserving the material-22 80 m fall-death threshold and its existing fall-damage metadata.
Real HKS tests verify the 25 m and 60 m long-stiff water landing, survival at 60 m, death at 85 m, up/down movement, seabed clamps, surface return and movement onto shallow land.
The HP formula for nonlethal fall damage remains untraced, as in step 1; the 0.5 water fallDamageRate is not used to invent one.

The local census found 198 material-22 triangles in m13, with a usable pool beside player start 1 at surface Y = -60.729.
The closest material-22 triangle to that start is underneath a bank, so material proximity alone is not a usable entry route.
Run `--play --map m13_00_00_00 --spawn 467,-59,346,-45,0 --no-enemy --simple-ai --enemy-distance 1000 --no-env-probe --script crates/game/scripts/m13_swim.txt --exit`.
This is the Abandoned Dungeon, not Hirata Estate.
The replay enters StandMoveToSwim at about step 127, SwimMoveStart at 157, SwimToDive at 346 and DiveMoveDownLoop at 386.
It reaches the local floor at about Y = -62.36, rises through DiveJump / DiveToAir, lands back in water and resumes SwimIdle at surface Y = -60.729.
A second swim jump and westward input reach LandSwimKickMoveJump and restore ground movement, ending near (469.10, -59.99, 341.81).
Reviewed captures are `target/alt/mobility/swim-{shore,surface,dive,resurface,exit}.png`; the final log is `swim-play-final.log`.
The captures show the swimming pose, a lower dive position and standing on the shore, but do not establish correct visible immersion: Wolf appears above the pale rendered water in the surface view, and the resurface camera pulls very close against the rock.
The body origin matches the authoritative collision surface in logs and tests; render/collision water alignment, native swim body offsets, transparent water, underwater fog/refraction and native camera behavior remain gaps.
Water rendering would require the concurrently owned map/material files, which this task does not edit.

Placeholder tolerances are 0.01 m contact/shore clearance, 0.05 m surface proximity and 0.5 m waterside proximity; these are numerical/modelled allowances, not exe-traced thresholds.
The water column is inferred from the deep plane's horizontal footprint and solid terrain below it; authored volume bottoms, overlapping multi-level water selection and native water docking/root warp remain unreconstructed.
The existing capsule stack is retained, without reconstructing a swimming-specific collider or foot/hand IK.
No swimming attacks/guards, poison effects, ladder/cling work or native swim-wire fidelity is claimed by this step.
All Cargo validation uses offline mode, `CARGO_TARGET_DIR=target/alt`, `CARGO_BUILD_JOBS=6` and `-j 6` where supported.
Check logs use `target/alt/mobility/swim-{fmt,clippy,suite,build-final}.log`.
Formatting, workspace/all-target Clippy with `-D warnings`, the release sim/formats suite and the release game build all pass.
The complete suite reports 445 passed and two existing ignored, including six swimming, six mobility, five cover, four docking and fifteen wire tests.

Biggest unknowns overall: the wire set and search row per area, native rope chain deformation and the EaseIn curve formula, the exe tests behind env 3000 and 3043 to 3050, fall damage HP and the void kill height, edge flags bytes 2 and 3, and whether the object transform in section 3.1 matches the rendered map (check one point against a screenshot first).

# The Great Shinobi Owl

Notes for the Owl fight: Wolf against the Great Shinobi Owl, with the Owl running his own HKS script, behaviour graph, TAE, params and Lua battle AI, on the flat floor (milestone 1) and on the Ashina Castle top where the game places him (milestone 2, "The castle top" below).
Run it with `cargo run --release -p sekiro-game -- --play --enemy owl` (or `--enemy c5060`); `duel-sim --enemy c5060` and `cargo test -p sekiro-sim --test owl` run it headless.

## Which character he is

The Great Shinobi Owl is **c5060**, 忍軍の長 ("chief of the shinobi army"), not c5070.
c5070 is 長の梟 ("the chief's owl"), his pet bird: 31 clips, NpcParam 50700000 and NpcThinkParam 50709000/50709300, placed only in the Hirata Estate layout (`m10_00_00_00`, parts `c5070_0000`/`c5070_0001`, beside the Owl (Father) parts `c5060_0001`/`c5060_0002`).
`--enemy c5070` is refused with this explanation.

| Fact | Value | Source |
|---|---|---|
| Model, anims, script, graph | `chr/c5060.chrbnd`, `c5060.anibnd` (155 clips, 32 aliases, 185 TAE animations), `action/script/c5060.hks`, `c5060.behbnd/c9997.hkx` (his own copy of the shared NPC graph) | archives |
| Bones | 120 (skeleton), 102 joints in the model | `sekiro-extract anims c5060`, `models c5060` |
| Castle placement | `m11_01_00_00` part `c5060_0000`, entity 1110850, at (-104.50, 54.01, 244.06), rotation Y 46 degrees, event flag 8301 | MSB, `cargo run -p sekiro-sim --example msb_enemies -- m11_01_00_00 c50` |
| Conversation copy | part `c5060_0001`, NpcParam 50600080, think 3, at (-109.28, 54.04, 239.66) | MSB |
| NpcParam | 50600000 "Ninja Army Chief Honjo, when the Ninja Army attacks" | MSB part, Paramdex names |
| NpcThinkParam | 50601000 "Old Ninja Army Chief, Honjo": `logicId` 506000, `battleGoalID` 506001 | MSB part |
| AI scripts | `script/m11_01_00_00.luabnd`: `506000_logic.lua`, `506001_battle.lua` (`GOAL_NingunOsa_506001_Battle`); also `506000_battle.lua`, which the castle row does not use | luabnd, think row |

`m11_01_00_00` is Ashina Castle in its invaded world state; `m11_00_00_00` (the map the game crate exports today) has no Owl.
His bird is not in the castle fight.

### NpcParam 50600000

| Field | Owl | Soldier (10100000) |
|---|---|---|
| `hp` | 1916 | 195 |
| `stamina` (posture) | 660 | 75 |
| `staminaRecoverBaseVel` | 60 | 20 |
| `staminaGuardDef` | 50 | 0 |
| `ninsatuNum` (deathblow marks) | 2 | 0 (one) |
| `behaviorVariationId` | 50601 | 10100 |
| `hitRadius` / `hitHeight` | 0.7 / 2.5 | 0.4 / 1.8 |
| `staminaControlParamId` | 5060000 | 1000100 |
| Resident SpEffects | 200004 (AI find/battle state), 221000, 230611 | 200030, 221002, 118000, 118200, 118400, 230610 |
| `lockDist` | 10 | 0 |
| `disableInitializeDead` | 1 | 0 |

The BehaviorParam rows exist for variation 50600 too, but the castle Owl reads 50601 (`250601xxx`).

## His attacks

From his TAE `AttackBehavior` (type 1) and `BulletBehavior` (type 2) events, BehaviorParam `250601<judge>` and AtkParam_Npc / Bullet rows.
Damage is AtkParam `atkPhys`, posture `atkStam`.
"Perilous" means AtkParam `disableGuard_vsGuardAttribute0` (Wolf's katana cannot block it); "no deflect" adds `disableJustGuard_vsGuardAttribute0`.

| Action | Judges and timing | What |
|---|---|---|
| 3000, 3001, 3002 | 100 at 0.57 s, 101, 102 | Kesa slash combo (112 / 96 / 144 HP) |
| 3003 | 110 at 0.97 s | Jumping forward roll, slash down |
| 3004, 3024 | 120 | Low sweep, **perilous, no deflect** (jump it); warning dummy bullet 980 / 985 |
| 3005 | 130 | Front kick (1 HP, 30 posture) |
| 3007 | bullet 131 | Kick into a fan of 5 shuriken (15 degrees apart, 48 HP each) |
| 3008, 3013, 3068 | 140, 141 | Shoulder charge (鉄山靠), then a launching slash |
| 3009, 3014, 3025 | bullet 150 | Shuriken (32 HP, 40 m/s, SpEffect 113000 slows posture recovery) |
| 3010, 3064 | bullets 190 to 194 | Firecracker (sparks; their attack row is a dummy, the effect comes from child bullets) |
| 3011, 3012 | 170 | Rushing slash |
| 3015 | 180 at 0.83 s | Rushing slash, curving left |
| 3016, 3063 | 171, 172 | Rushing slash follow-up, two hits |
| 3017 | 199, bullets 250 to 254 | Explosive slash (爆炎斬り) |
| 3018 | bullet 150, then 200 | Shuriken, then a dash slash |
| 3019 | 111 | Jump slash into a spin |
| 3026, 3027, 3091 | 210 | Charged heavy downswing (208 HP, 72 posture) |
| 3028, 3029 | 280, 290 | Spin slashes after the downswing |
| 3030 (plays 3007's clip) | bullets 700, 730, 740 | Poison ball throw (three rows) |
| 3031, 3032 | bullet 710 | Recovery-sealing throw (SpEffect 3506090) |
| 3033 | 310, 311 | Shinobi stab, rushing thrust: **perilous**, deflectable dummy 311 for Mikiri |
| 3034 | 320, 330, 331 | Shinobi stab from a jump: **perilous** first hit, then two falling cuts |
| 3035, 3036, 3038 | 220, 230, 260 | Owl warp: falling slash, low falling slash, charged slash |
| 3037, 3040 | 800 | Grab ("collapse throw") start; ThrowParam 25060000 (`a000_004100` on him, `a000_600110` on Wolf) |
| 3041 | 810 | Mikiri-style grab start, **perilous**; ThrowParam 25060100 (`4110` / `600120`) |
| 3044, 3046, 3047, 3093 | 350 to 353, 230, 260, 360, 220 | Warp sequences |
| 3060, 3061, 3065, 3066 | 101, 500, 520, 510 | Counters after being deflected (the "Kengeki" acts) |
| 3069 | bullet 720 | Smoke screen |
| 3090 | 300 | Turn into a low sweep after a strong deflect, **perilous, no deflect** |
| 3100 to 3103 | | Guard and deflect (one clip, `a000_003100`) |
| 5201, 5202, 5203 | | Back step, side steps |

## His Lua AI

`506000_logic.lua` observes SpEffects 107710 (on himself), 110060 and 110015 (on Wolf), runs `COMMON_HiPrioritySetup` and `COMMON_EzSetup` with the boss flag, and queues battle goal 506001.
`506001_battle.lua` scores up to 26 acts by distance and state, with cool-downs per animation, then queues them as `ComboAttackTunableSpin` / `ComboRepeat` / `ComboFinal` chains.
Every act also does `goal:TimingSetNumber(5, GetNumber(5) + n, AI_TIMING_SET__ACTIVATE)`: AI number 5 is a pressure score that the deflect counters (`Kengeki_Activate`) read (at 22 and 25 he answers a deflect with a heavier act).
Its `Interrupt` answers parry timing (`Parry`: guard 3100, deflect 3101 with a probability that grows with his consecutive guards, special deflect 3102, dodge 5201), shuriken impacts, Wolf's posture break, item use (gourd) and SpEffect activations.

What we see in a headless 60 s run against a passive Wolf (seed 7, `owl_fights_a_passive_wolf_for_a_minute`):

| Action | Times |
|---|---:|
| 3000 / 3001 / 3002 combo | 4 / 4 / 2 |
| 3003 jumping slash | 1 |
| 3005 kick | 7 |
| 3009 shuriken | 1 |
| 3011 rushing slash | 2 |
| 3014 shuriken | 1 |
| 3031 recovery-sealing throw | 3 |
| 3063 rushing follow-up | 3 |
| 5203 side step | 3 |

From 12 m, with Wolf backing away, he throws shuriken that hit for 32 HP (`owl_throws_shuriken_from_range`).

## Deathblows and his two lives

ThrowParam rows for Wolf against him sit at `1506xyzz`, `DefChrId` 5060, and Wolf's clips use `atkAnimOffset` 221 (`a221_*`, as FwLo's deathblow table also lists).

| Rows | Kind | Wolf | Owl | Sorb |
|---|---|---|---|---|
| 15060000 / 15060001 | Posture break, front | `a221_500000` / `a221_510000` | `a000_012000` | `defSorbDmyId` 230 |
| 15060110 / 15060111 | Posture break, behind | `a221_501200` / `a221_511200` | `a000_013200` | |
| 15060180 / 15060181 | Finishing blow (トドメ, "todome") | `a221_501700` / `a221_511700` | `a000_013700` | `defSorbDmyId` 235 |
| 15060190 | Mikiri counter | `a221_511800` | `a000_013800` | |
| 15060010 to 15060012, 15060120 to 15060122 | Deflect and Mikiri variants | | | |

How the two lives work in the data:

- `ninsatuNum` 2 is the mark count; the engine keeps HP at 1 while marks remain (`Vitality::set_hp`, read from code).
- The first deathblow plays `a000_012000`, in which he gets back up: at 4.4 s its TAE adds SpEffect 300610 "HP reset" (`changeHpRate` -100, full HP) and switches to StaminaControlParam type 7 (row 5060000: minimum and maximum 100 %, so posture refills).
  Its `ThrowDef12000` state in his graph has no death branch.
- The finishing pair's defender clip `a000_013700` is itself the death: the shared NPC script's `ThrowDef13700` sets `STATE_DEATH` and goes to the death idle when it ends, and the clip adds SpEffect 3506050 "immortality release" and fades him out from 9.2 s.
  There is no `a000_013701`.

The rows use `defSorbDmyId` (dummy 230 sits 1.3 m in front of him, facing back) with `atkSorbDmyId` 0, the reverse of the soldier's rows; the duel holds him at the inverse of that frame in front of Wolf.

## What the reimplementation does

- `EnemySpec` (`crates/sim/src/enemy.rs`) names the enemy: model, NpcParam, NpcThinkParam, script bundle.
  `Duel::load_with`, the AI loader, `duel-sim --enemy` and the game's `--enemy` flag use it; the soldier stays the default.
- His HKS (`c9997.hks` then `c5060.hks`) and graph run unchanged; a 60 s fight produces no script errors.
- His Lua AI runs unchanged; three engine features were added for it, all generic (see `docs/AI.md`):
  goal `TimingSetNumber` / `TimingSetTimer`, SpEffect activation and end interrupts for observed effects, and the AI cancel envs 107 / 118 / 120 that let a queued combo attack, attack or step cut the current animation at its TAE windows (flags 23, 86, 79).
  Before the last one, every NPC combo waited for the previous animation to end and paused in idle for a step.
- Hits, blocks, deflects and posture go through the existing combat rules; bullets with damage (his shuriken) fly as projectiles (`crates/sim/src/bullets.rs`).
- Deathblows: the first mark plays the get-up deathblow and restores HP and posture from the data; the last mark uses the finishing pair; a deathblow cannot start while he is frozen in hit stop or before his script has read the breaking hit, or while he is still in the previous throw's defender clip.

## Inferred

- He starts in the battle target state (`EnemySpec::starts_in_battle`): the castle fight starts after a cutscene, and his NpcParam carries the battle-state SpEffect 200004 as a resident effect.
  Without it his first logic run sees the find state and walks at Wolf for 5 s (`goalAction_ToFind` 2).
- The finishing pair (suffixes 180 / 181) is chosen whenever the next deathblow spends the last mark and the enemy has such rows; the engine's actual rule is not traced.
  The evidence is that its defender clip is a death state while the ordinary one gets up again.
- `changeHpRate` applies once when a TAE SpEffect starts.
- Observed SpEffects already active when first observed do not raise an activation interrupt.
- Which TAE window accepts which queued request: combo goals (registered with `ENABLE_COMBO_ATK_CANCEL`) go to env 107 under flag 23, other non-step actions to env 120 under flag 86, steps (`5xxx`) to env 118 under flag 79.
  The env meanings are from the community env list; the pairing with the flags follows FwLo's RustSekiro notes (`docs/kb/enemy.md`).
- Bullets fly straight at `initVellocity`, aimed at Wolf's chest within `lockShootLimitAng` of the shooter's facing, a fan of `numShoot` spaced `shootAngleInterval` degrees, and their hits do not freeze the shooter.

## Unknown engine queries he made

| Query | When | What we answer |
|---|---|---|
| `goal:TimingSetNumber`, `goal:TimingSetTimer` | every act | Implemented (sets AI number / timer on activation or success) |
| `GetSpecialEffectActivateInterruptType`, `...InactivateInterruptType` | SpEffect interrupts | Implemented (the effect being dispatched) |
| `AddObserveSpecialEffectAttribute` | logic and battle goal | Implemented (was a no-op) |
| `IsInsideTargetRegion` | battle acts | Default false (also unknown for the soldier) |
| `GetLatestSoundTargetRank`, `CheckDoesExistPathWithSetPoint` | the find-state approach, now skipped by starting in battle | Default 0 / false |
| `TeamHelp_ValidateCall`, `TeamHelp_ValidateReply`, `IsValidPlatoon`, `GetMovePointNumber`, `GetPrevMovePointNumber`, `GetMovePointWaitTime`, `GetMovePointAnimId`, `goal:GetLastResult` | non-battle logic after Wolf dies | Default false / 0 |

## Not done yet

- **Phase 2.**
  After the first deathblow the game's map event script plays his heat-up (SpEffect 3506080 "action judgment, HU animation", acts with `EndureAttack` 20006 to 20008) and gives him SpEffect 200051 "behaviour pattern change 1", which switches some acts (for example 3030 poison instead of 3007).
  `event/m11_01_00_00.emevd` references 200051 and 3506080 next to his entity 1110850; it needs an EMEVD reader.
- His grabs (3037, 3040, 3041 into ThrowParam 25060000 / 25060100) do not throw Wolf: NPC-on-player throws are not modelled.
- Firecrackers, explosions, poison and smoke: their bullets' attack rows are dummies and the effects come from child bullets (`HitBulletID`), SpEffects and map hits.
- Perilous-attack warning (the 211000 "dangerous attack" VFX from the dummy bullets) and Mikiri / jump counters on Wolf's side.
- The Owl rarely guards or deflects Wolf's slashes in our runs (2 blocks in a minute of slashing).
  His `Parry` deflects with a chance of 50 % per consecutive guard; the count rises on SpEffect 200250, which his weak guard reactions (`a000_008500`, `8501`, `8504`) add, through the shared table-logic interrupt now that observed SpEffects raise interrupts.
  Why he guards so seldom is not checked yet (his `Parry` also needs Wolf inside `GetDist_Parry` and both front cones).
  Until 2026-10-10 his blocks also played the hit flinch `a000_008010` instead of a guard reaction: his script indexes `guard_damage_table` by env 258, his NpcParam `guardLevel` (4; "which guard motion receives the enemy's attack"), which we answered 0. That env meaning is inferred from the param description and the table's shape, not traced in the exe.
- Owl warp transparency (3506030) and the smoke vanish are not rendered; bullets are drawn as small spheres.
- After the finishing blow the duel respawns both characters 6 s after the death, before his 12 s death clip ends.

## The castle top (`--arena owl`)

Milestone 2: the same fight where the game places him, on the top floor of the Ashina Castle keep in `m11_01_00_00` (the invaded castle).
The fight floor is the keep's open top room with its tatami and broken shutters, not the outer roof tiles; Wolf falls onto the roof below if he goes over the railing.

```powershell
cargo run --release -p sekiro-extract -- map m11_01_00_00
cargo run --release -p sekiro-game -- --arena owl
cargo run --release -p sekiro-game -- --arena owl --full-map
cargo run --release -p sekiro-game -- --arena owl --script crates/game/scripts/owl_arena.txt --shots 25:cache/screens/arena_owl_start.png,120:cache/screens/arena_owl_approach.png,400:cache/screens/arena_owl_fight.png --exit
cargo test -p sekiro-sim --test owl_arena
```

`--arena owl` implies `--play` and `--enemy owl`; the other play flags (`--lock-on`, `--script`, `--hide`, `--objects`, ...) work as usual.

### How an arena is described

An arena is one row of `ARENAS` in `crates/sim/src/arena.rs`: map id, the boss's MSB enemy part, the enemy, a radius, Wolf's player start and map piece models to hide.
`ArenaLayout::resolve` reads the boss's transform and the start from the map's `layout.json`; `Arena::load` crops the hit collision to the triangles whose horizontal bounding box reaches within the radius of the boss, and `Arena::place` puts both characters on it.
The game crate spawns only the map pieces whose placed bounding box reaches inside the same circle (`MapScene::within`); `--full-map` loads the whole map with the same placements.
The next boss (Genichiro on the castle roof, for example) is one new row.

| Field | Owl |
|---|---|
| Map | `m11_01_00_00` (its own 3636 pieces of 791 models; nothing is shared with `m11_00_00_00`) |
| Boss | part `c5060_0000` at game (-104.50, 54.01, 244.06), Y rotation 46 degrees; Bevy (104.50, 54.01, 244.06), yaw -46 degrees (x and yaw negate, `docs/FORMATS.md`) |
| Radius | 60 m: the top room is about 25 m across, so the room, its railings and the roofs around and below it are all inside; 1553 of 3622 piece instances and 470 455 of 1 656 384 collision triangles |
| Wolf | player start `c0000_0003`, game (-120.34, 54.05, 234.23), 18.9 m from the Owl on the same floor, turned to face him |
| Hidden | `m015072`, the burnt version of the top floor (see below) |

### What the layout says, and what is inferred

- The MSB has two player starts on that floor: `c0000_0003` (18.9 m away, beside the Sculptor's Idol object, so most likely the idol's rest point) and `c0000_0010` (1 m from the Owl, likely an event warp).
  `c0000_0003` is the arena start; which one the game uses after the fight's cutscene is not traced.
- Wolf is turned to face the Owl instead of taking the start's MSB yaw (-46.8 degrees in game space), which points him away from the Owl under our yaw convention.
  The Owl keeps his MSB yaw, which faces Wolf's start within about 12 degrees.
  Our convention (yaw 0 faces -Z, the X mirror negates the angle) is consistent for geometry, but whether a character placed with an MSB yaw really faces that way in the game is not checked against a capture.
- Two map pieces at the same transform hold the castle top floor: `m015070` (entity 1114550, intact tatami with fallen shutters) and `m015072` (entity 1114551, burnt and dark).
  Shown together they z-fight in polygon-shaped patches.
  They are the only event-controlled map pieces in the layout; the map event script picks one.
  The arena hides `m015072`; that the Owl fight uses the intact floor is inferred (the burnt one fits a later story state), and decoding `event/m11_01_00_00.emevd` would settle it.
- Objects stay off as in `--play --map` (`--objects` adds those within the radius; on this floor that adds the Sculptor's Idol behind Wolf's start).
- Fall death is not modelled: a character that falls 30 m below its start is put back there (`Duel::fall_respawn_depth`), which only happens off the cropped collision.

### Checks

- `crates/sim/tests/owl_arena.rs` loads the arena collision (skips when the map or the Owl is not extracted), places both, and runs his real AI for 30 s against a passive Wolf: neither drops more than 2 m below his start, and the Owl attacks at least 5 times (13 attacks and 11 hits on Wolf with seed 7).
  A second test drops Wolf off the cropped collision and checks that he comes back to his start.
- Scripted runs (`crates/game/scripts/owl_arena.txt`) show both standing on the tatami, the Owl's shuriken and slashes, Wolf's blocks and deflects, and the camera under the room's ceiling; `--full-map` renders the same frames.
- Load time from launch to the first simulated frame (warm file cache, release build): about 4 s for the arena and 6 s for the full map; the map assets themselves load in 1.5 s and 3 s.
  The collision crop takes 0.1 s.

### Not done yet on the castle top

- The floor shows a single albedo layer like every map (`docs/FORMATS.md`, Known approximations); lighting is the default sun, not the castle top's draw params.
- The object `o115923` (entity 1111723) has no model in the archives and is skipped by the map export.
- Phase 2, throws on Wolf and firecrackers: see "Not done yet" above, and the steps below.

## Next steps

1. Decode the EMEVD bank used around entity 1110850 (boss health bar 905060, the phase-2 SpEffect 200051, the heat-up check on 3506080) and run his phase change from it; it also says which castle top floor (`m015070` or `m015072`) the fight shows.
2. NPC-on-player throws (his grabs 3037, 3040, 3041), firecracker and poison bullets.
3. The boss brain is in place (`docs/BRAIN.md`, `--brain scripted`): next, a hosted decision-model controller behind the same `BossBrain` trait.

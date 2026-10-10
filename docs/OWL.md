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
| 3010, 3064 | bullets 190 to 194 | Firecrackers: five lie lit 0.8 s, then bang (48 HP, unblockable, large stagger; see "Tools") |
| 3011, 3012 | 170 | Rushing slash |
| 3015 | 180 at 0.83 s | Rushing slash, curving left |
| 3016, 3063 | 171, 172 | Rushing slash follow-up, two hits |
| 3017 | 199, bullets 250 to 254 | Explosive slash (爆炎斬り): 208 HP fire slash; the powder bullets are looks ("Tools") |
| 3018 | bullet 150, then 200 | Shuriken, then a dash slash |
| 3019 | 111 | Jump slash into a spin |
| 3026, 3027, 3091 | 210 | Charged heavy downswing (208 HP, 72 posture) |
| 3028, 3029 | 280, 290 | Spin slashes after the downswing |
| 3030 (plays 3007's clip) | bullets 700, 730, 740 | Poison ball throw: three rows of poison floor ("Tools") |
| 3031, 3032 | bullet 710 | Recovery-sealing throw: a thrown bullet, not a grab (SpEffect 3506090 blocks healing for 30 s; see "Grabs") |
| 3033 | 310, 311 | Shinobi stab, rushing thrust: **perilous**, deflectable dummy 311 for Mikiri |
| 3034 | 320, 330, 331 | Shinobi stab from a jump: **perilous** first hit, then two falling cuts |
| 3035, 3036, 3038 | 220, 230, 260 | Owl warp: falling slash, low falling slash, charged slash |
| 3037, 3040 | 800 | Grab ("collapse throw"): ThrowParam 25060000, `a000_004100` on him, `a221_600110` on Wolf, 192 HP (see "Grabs") |
| 3041 | 810 | "See-through" counter grab, unblockable and undeflectable by AtkParam (no warning in the data): ThrowParam 25060100, `a000_004110` / `a221_600120`, 32 + 208 HP |
| 3044, 3046, 3047, 3093 | 350 to 353, 230, 260, 360, 220 | Warp sequences |
| 3060, 3061, 3065, 3066 | 101, 500, 520, 510 | Counters after being deflected (the "Kengeki" acts) |
| 3069 | bullet 720 | Smoke screen: blinds Wolf within 5 m, drops the lock-on ("Tools") |
| 3090 | 300 | Turn into a low sweep after a strong deflect, **perilous, no deflect** |
| 3100 to 3103 | | Guard and deflect (one clip, `a000_003100`) |
| 5201, 5202, 5203 | | Back step, side steps |

## Grabs

His three grab acts are enemy throws on Wolf: an ordinary attack window whose AtkParam row has `throwFlag` 1 (a throw start) catches Wolf, and a ThrowParam row then pairs the Owl's throw clip with Wolf's thrown clip.
The pattern follows FwLo's notes on the Ashina General's neck grab (RustSekiro `docs/kb/guard-deflect.md`, `damage.md`); the values below are the Owl's own data.
Code: `crates/sim/src/grab.rs`, wired in `Duel` (`try_enemy_throw`, `step_enemy_throw`); tests `crates/sim/tests/owl_grab.rs`.

### The acts and their windows

| Act | Clip | Grab window (TAE `AttackBehavior`) | Judge, AtkParam | Used by his castle AI |
|---|---|---|---|---|
| 3040 | `a000_003040` (2.2 s) | 0.567 to 0.767 s; `BoostRootMotionToReachTarget` 0.5 to 0.7 s brings him to 0.9 m of Wolf (between 0.8 and 2.0 m) | 800 崩し投げ_始動 ("collapse throw, start"), 50601800 | yes: an act in `506001_battle.lua` (its constants include 4 and 6, likely its distances) |
| 3041 | `a000_003041` (1.7 s) | 0.067 to 0.233 s, with `IFrames_DuringAction` (951) on him from 0 to 0.233 s | 810 見切り投げ_始動 ("see-through throw, start"), 50601810 | yes: an act, and his parry interrupt (next to his guard 3100, deflects 3101/3102 and back step 5201), so it is a counter to Wolf's attacks (read from the compiled scripts' constants, not decompiled) |
| 3037 | `a000_003037` (2.7 s) | 1.0 to 1.133 s, after fading out (opacity 0 from 0.27 to 0.7 s): a warp grab | 800 | no: only `506000_battle.lua` (the unused castle battle goal and Hirata) |

The grab hitbox of both start rows is one sphere of radius 0.4 m on dummy 16, which is on his **left foot** (`L_Foot`).

### AtkParam rows

| Row | `throwFlag` | `throwTypeId` | Damage | Notes |
|---|---|---|---|---|
| 50601800 (judge 800) | 1 | 0 | `atkPhys` 32, but a throw start deals none (see below) | not `disableGuard` / `disableJustGuard`: the throw flag, not the guard flags, makes it ignore a guard |
| 50601810 (judge 810) | 1 | 1 | `atkPhys` 1 | `disableGuard_vsGuardAttribute0` 1 and `disableJustGuard_vsGuardAttribute0` 1 (perilous by the AtkParam rule) |
| 50601801 (judge 801) | 2 | | **192 HP**, posture 12 (`directAtkStamDamage`) | the collapse throw's stab |
| 50601811 (judge 811) | 2 | | **32 HP**, `excessDmgKeepHp` 1 (cannot kill) | the see-through throw's first cut |
| 50601812 (judge 812) | 2 | | **208 HP**, posture 12 | the see-through throw's second cut |
| 50601813 (judge 813) | 0 | | none | "sword flash dummy" `AttackBehavior` in `a000_004110` at 1.367 s, for the trail effect |

None of the six rows has an SpEffect (`spEffectId0..4` are all -1): a grab applies no status.

### ThrowParam rows

| Field | 25060000 (崩し投げ) | 25060100 (見切り投げ) |
|---|---|---|
| `AtkChrId` / `DefChrId` | 5060 / 0 (the player) | 5060 / 0 |
| Owl's clip (`atkAnimOffset` 0) | `a000_004100` (6.0 s) | `a000_004110` (8.67 s) |
| Wolf's clip (`defAnimOffset` 221) | `a221_600110` (6.0 s) | `a221_600120` (8.83 s) |
| `Dist`, `DiffAngMin..Max`, `diffAngMyToDef` | 10 m, 0..180, 180 | same |
| `upperYRange` / `lowerYRange` | 10 / 10 | same |
| `atkSorbDmyId` / `defSorbDmyId` | 233 / 0 | 234 / 0 |
| Sorb dummy | 1.0 m in front of the Owl, facing back | 1.5 m in front, facing back |
| `adsrobModelPosInterpolationTime` | 0.5 s | 0.5 s |
| `isTurnAtker`, `throwKind`, `throwTypeId` | 1, 1000000, 4100 | 1, 1000010, 4110 |

The range is wide open (10 m, every angle): the hitbox contact decides, not the row.
How the start row finds its ThrowParam row is inferred: the id is `20000000 + chr * 1000 + throwTypeId * 100` for both rows (`ThrowTable::npc_row`).

Wolf's thrown clips are in `chr/c0000_c5060.anibnd` (Wolf's per-enemy archive; also `a221_600111`, `600121` and the one-frame `610111..610123`), already unpacked by `sekiro-extract unpack` and decoded by `anims c0000`; their TAE is in c0000's own TAE.

### The throw clips

| Event | Owl `a000_004100` | Owl `a000_004110` | Wolf `a221_600110` | Wolf `a221_600120` |
|---|---|---|---|---|
| Damage (`ThrowAttackBehavior`, TAE 304) | judge 801 at 1.333 s | judges 811 at 1.433 s, 812 at 2.633 s | | |
| Invulnerability | 951, 0 to 4.7 s | 951, 0 to 6.4 s | 952 `IFrames_ThrowAtkStillHurts`, 0 to 4.0 s | 952, 0 to 6.7 s |
| Turning | turn speed 0 throughout | turn speed 0 throughout | flag 7 (no turning) to 2.7 s, then turn speed 0 | flag 7 to 5.9 s, then 0 |
| Death branch | | | `ChrActionFlag` 69 at 3.833 s | flag 69 at 6.667 s |
| Get-up cancels | AI combo/attack/step windows from 5.433 s | from 7.9 s | rolls (flag 25) from 2.7 s; dodge (26) and jump (119) from 4.0 s; attack from 5.3 s; move (11) from 5.8 s | dodge and jump from 6.7 s; move from 8.7 s |
| SpEffects on Wolf | | | 110030 (0 to 5.3 s), 9281 and 9481 (0 to 4.0 s), 109450 (0 to 5.8 s), 110145 (from 0.4 s), the get-up refs 100300/100346/100306 (4.0 to 5.3 s) | the same pattern, shifted |

The death clips `a221_600111` (3.17 s) and `600121` (3.0 s) are reached through Wolf's graph event `W_ThrowDefDeathStart600110` / `600120`; they carry `IFrames_MistRaven` and SpEffect 110060.

### Perilous, and the counters

- No grab clip spawns a "dangerous attack" warning bullet: his warning dummies are judges 980 (low sweep), 985 (zigzag dive), 986 (turning low sweep) and 987 (Shinobi stab), in 3004, 3024, 3090 and 3033 only.
  So the data shows no red kanji for any of his grabs.
  810 is perilous by its AtkParam guard flags, but its window opens 0.067 s into a counter act, too early for any warning.
- Guarding and deflecting do not stop a grab: a `throwFlag` 1 contact starts the throw whatever Wolf's guard state (as in Sekiro, and as FwLo models it for the General's grab).
- Getting out: be out of reach of his foot sphere when the window opens (step back or away), step through it (Wolf's steps `a000_201xxx` carry TAE 954 i-frame types 2 and 7 for 0 to 0.27 s; type 7 is the grab type per FwLo), or be in the air (FwLo's rule: no grab on an airborne Wolf).
- The "recovery-sealing throw" 3031 (and 3032) is **not a grab**: it throws bullet 710 (`a000_003031` at 0.633 s, Bullet 50601710 with child bullet 50600711), whose effect is SpEffect 3506090 "Recovery Seal": `effectEndurance` 30 s, `stateInfo` 910, `spCategory` 193, icon 2206, VFX 40100, on the opposing target (Wolf).
  It is the healing gourd block of the fight and belongs to the bullet and status work (`crates/sim/src/bullets.rs`, `status.rs`), not to throws.

### What the reimplementation does

1. Hit detection treats a `throwFlag` 1 window like any other capsule, but only a grounded Wolf without invulnerability or step i-frame type 7 can be caught (`hits::can_be_grabbed`); a passed-through grab is not spent.
2. In `Duel::step` the contact does not go through the hit rules (no damage, guard or deflect): `try_enemy_throw` picks the row, checks its range, turns the Owl to Wolf (`isTurnAtker`), fires `W_ThrowAtk4100` into his graph and `W_ThrowDef600110` into Wolf's, and drops any knockback.
3. Wolf is absorbed onto dummy 233 over 0.5 s as the pair stood when the clips started, through collision (the other character gives way at a wall), with the deathblow's `AbsorbSlide`; then both play their own root motion.
   No push-apart and no knockback while the throw holds him.
4. At each `ThrowAttackBehavior` time of the Owl's clip (following his clip time, so hit stop delays it and leaving the clip cancels it) the row is resolved as a direct hit: 192 HP and 12 posture for 801, with `excessDmgKeepHp` honoured (811 never kills).
   Wolf's script gets no damage reaction; his thrown clip already shows it.
5. TAE 952 now counts as full invulnerability, so nothing else hits Wolf while thrown (his clip's own SpEffects play as usual).
6. If the damage empties his HP, his script keeps seeing 1 HP until the clip's flag 69 (3.833 s), where the duel fires `W_ThrowDefDeathStart600110` and he plays `a221_600111`; then the usual death and respawn.
   Otherwise he gets up in the clip and the throw ends when he leaves it (its cancels from 2.7 s, or its end at 6.0 s).
7. Testing aid `--force-act <id>` (`Duel::force_act`): the enemy requests the act whenever it is free, on top of its AI or alone with `--passive`.

```powershell
cargo test -p sekiro-sim --test owl_grab
cargo run --release -p sekiro-game -- --arena owl --passive --god --force-act 3040 --spawn 101.23,53.95,247.33,135,0 --enemy-distance 1.6 --script crates/game/scripts/owl_grab.txt --shots 36:cache/screens/grab_rail_036.png,120:cache/screens/grab_rail_120.png,330:cache/screens/grab_rail_330.png --exit
```

Measured (`owl_grab.rs`): Wolf (320 HP at level 1) is caught 0.57 s into 3040, both clips start the next step, the stab lands 1.33 s later for 192 HP and 12 posture, he is invulnerable in the clip, and he is free again at 6.0 s; held in guard or tapping guard he is still caught; stepping back at 0.2 s or jumping at 0.37 s avoids it; 3041 at 1.2 m catches him with row 25060100 and lands 32 then 208; at 100 HP the throw kills and plays `a221_600111`; with Wolf backed 0.45 m from the castle-top railing no step of either crosses geometry and both end on the floor.
Screenshots: `cache/screens/grab_open_*.png` (open floor), `grab_rail_*.png` (at the railing: the Owl kneeling on Wolf at the stab, Wolf getting up), `grab_guard_*.png` (guarding Wolf caught).

### Inferred

- The ThrowParam id rule from `throwTypeId` (two rows fit it).
- A throw start does no damage of its own (its `atkPhys` 32 is unused) and a contact the row's range refuses is ignored; FwLo's General grab does the same.
- No grab on an airborne Wolf (FwLo); step i-frame type 7 as the grab type (FwLo); the other i-frame types still do nothing.
- TAE 952 as full invulnerability against everything but the throw's own damage (from its name and that it sits only on thrown clips).
- The animation offset type the Owl's `ThrowAtk` state reads is set to `atkAnimOffset` (0) like the deathblow's defender type 15; his clips are named directly in his graph, so it has no visible effect.
- The held pair stands closer than both body radii (0.55 to 0.75 m) for most of the clip: that is the two clips' own root motion from the absorbed start, as with deathblows.

## Tools

His thrown tools are bullets (`crates/sim/src/bullets.rs`): a TAE `BulletBehavior` event names a judge, BehaviorParam `250601<judge>` (`refType` 1) names the Bullet row, and the row names its attack, SpEffects and child bullets (`HitBulletID` where it ends, `intervalCreateBulletId` while it lives).
Status effects they leave are in `crates/sim/src/status.rs`.
All numbers below are the data's; "inferred" marks what is ours.
Run one on demand with `--arena owl --passive --god --force-act <act> --spawn 104.5,54.0,244.06,135,0 --enemy-distance 4 --script crates/game/scripts/owl_tools.txt --shots ... --exit`; tests in `crates/sim/tests/owl_tools.rs` force them with `BrainKind::Repeat`.

### How a bullet flies

- Speed `initVellocity`; within `dist` metres it uses `accelInRange` and `gravityInRange`, past it `accelOutRange` and `gravityOutRange` (def: "distance where decay starts").
  Speed is clamped to `minVellocity..maxVellocity` while an acceleration acts (inferred).
- Fan: `numShoot` bullets, `shootAngle` + k * `shootAngleInterval` degrees horizontally and `shootAngleXZ` + k * `shootAngleXInterval` degrees of elevation (positive up: the recovery powder is thrown up at +75, the smoke bomb down at -60).
  Aimed at Wolf's chest when he is within `lockShootLimitAng` of the facing, else along the facing (after FwLo's RustSekiro); elevations add to the aim (inferred).
  The random spreads (`shootAngleYMaxRandom`, `lifeRandomRange`) are left out so runs repeat.
- Hit radius `hitRadius`, growing to `hitRadiusMax` over `spreadTime`.
  A bullet ends on a character unless `isPenetrate`, on the floor under it unless `isPenetrateMap` (walls are not tested, inferred), and when `life` runs out; its `HitBulletID` child starts there in all three cases (`launchConditionType` 0 on all his rows), along the parent's travel direction (inferred), never below the floor.
- `dmgHitRecordLifeTime`: seconds before the same target can be hit again (0: once); `isUseSharedHitList`: the bullets of one shot (one TAE event, children included) share that record, so five firecrackers hit Wolf once.
- `damageDamp`, `staminaDamp`, `knockbackDamp`: percent lost per second past `dist` (def); linear (inferred).
  His shuriken (100 %/s past 5 m) deal 29 to 30 of their 32 HP at 9 m.
- An AtkParam with `opposeTarget` 0 hits no character: row 1, "[System] dummy bullets (do not hit characters, hit maps)", is the attack of every carrier and of the explosive slash's flames.
- A bullet's `spEffectId0..4` go on the character it hits, unless the hit was guarded (inferred).
  A contact whose attack deals nothing, has `dmgLevel` 0 and no `atkBehaviorId` (the poison floor) only applies its SpEffects: no guard test, no hit reaction (inferred).

### Each tool

| Act | Bullets | What it does in the data | Now |
|---|---|---|---|
| 3009, 3014, 3025 shuriken | 150 | 40 m/s, past 5 m -40 m/s² and 2.5 m/s² down, 0.1 to 0.25 m over 0.07 s; AtkParam 50600150: 32 HP, posture 12, blockable; SpEffect 113000 (`staminaRecoverChangeSpeed` -5 for 10 s) | Flies, decays, slows Wolf's posture recovery for 10 s |
| 3007 kick into shuriken | 131 | Five at -30, -15, 0, 15, 30 degrees, 48 HP each, same SpEffects | As above |
| 3010, 3064 firecrackers | 190, child 195, grandchild 200, flames 201 | Five thrown at 70, 40, 10, -20, -50 degrees, 25 degrees down, 16 m/s for 0.2 s through the map; each lies lit 0.8 s (195, dummy attack), then bangs: 200, 1.8 m for 0.15 s, AtkParam 50600160 (24 physical + 24 fire = 48 HP, posture 12, `dmgLevel` 3, `spAttribute` 2 fire, cannot be blocked or deflected by Wolf's katana), with two 1.8 m flames (201, dummy) at 0.01 and 0.11 s; shared hit record, 0.5 s; SpEffects 100392 / 107623 are for the Loaded Umbrella (not modelled). 191 to 194 name Bullet rows that do not exist | Wolf takes one bang (48 HP, 12 posture) through his guard and plays `StandDamageLarge`; env 285 reads 2 (fire), so the script adds its fire blend; running away as he throws avoids it |
| 3017 explosive slash | slash judge 199; bullets 250 to 254, children 255 to 259, grandchild 260, flames 261 | The slash: AtkParam 50601199, 104 physical + 104 fire = 208 HP, posture 60, blow-away (`dmgLevel` 10). The five powder bullets and their 1.5 m / 2.5 m flames all use AtkParam 1: looks only | Slash as before (`StandDamageExLargeBlowStart`); the powder and flames fly and burn without hitting |
| 3030 poison balls (phase 2) | 700, 730, 740, children 701 / 731 / 741, grandchildren 702 / 732 / 742 | Three balls (25, 11, 7 m/s; 9.8 m/s² past 1 m) drop five poison drops each (100 m/s² down) that leave poison floor: 2 m, 18 s, re-hit every 0.15 s, SpEffect 9019 (`poizonAttackPower` 30, `replaceSpEffectId` 9050). AtkParam 50600700 deals nothing | A Wolf standing in it builds poison: 200 resistance (CalcCorrectGraph 110, flat 200; armour `resistPoison` 0) / 30 = 7 contacts, about a second; then 9050: 0.25 % of max HP + 12 every 1.0 s for 39.9 s (12 HP a second for Wolf), behaviour ref 500 (`SP_EF_REF_POISON`), and `disablePoison` stops new build-up while it runs; the meter falls 5 a second (TentativePlayerParam `ResistRecoverPoint_Poision_Player`). No hit reaction. Running away avoids it |
| 3031, 3032 recovery-sealing powder | 710, child 711 | Thrown up (+75 degrees, 4 m/s, 9.8 then 6 m/s² down) for 1 s, then a cloud: 0.1 m growing to 3.2 m over 1 s, 1.7 s, SpEffects 3506090 (`stateInfo` 910, 30 s, targets the opposing character: healing items are sealed) and 3506091 (its on-screen text). AtkParam 50600710 deals nothing | Wolf inside gets 3506090 for 30 s (HUD "Recovery sealed"); no item use exists yet, so the seal has nothing to block. Running away avoids it |
| 3069 smoke screen | 720, child 721 | Thrown 60 degrees down at 80 m/s; on the floor the dazzle: 5 m for 0.06 s, AtkParam 50600721 (no damage, unblockable, `atkBehaviorId` 17). The Owl adds SpEffect 200602 (`stateInfo` 350, "cannot be locked on", 1 s) at 1.17 s | Wolf's script reads `env(334, 17)` (`BEHAVIOR_IDENTIFICATION_VALUE_BLINDING`) with damage level 0 and plays `StandDamageBlinding` (`a000_192600`), also while guarding; the lock-on drops and cannot be taken for that second (that a held lock is released is inferred). Running away avoids it |
| 980 to 987 | | Perilous-attack warning dummies (2 m, 0.1 s, AtkParam 8, SpEffect 211000 "dangerous attack VFX") | Fly and touch Wolf with no effect; not drawn (the kanji warning is not done) |

Not in his kit: burn.
The Owl's fire hits (firecracker, explosive slash) add no burn SpEffect (the 91xx "burning damage" rows), so fire here is fire damage and the fire blend only.

### Inferred

- Children start along the parent's travel, never below the floor; child elevation is the child row's own angle.
- SpEffects of a guarded bullet hit are not applied; effects-only contacts skip the guard test.
- Poison: the meter's result is `replaceSpEffectId` (the def calls it "added when the lifetime ends"); the first poison tick comes one interval after it starts; the tick truncates.
- The five drops of each poison ball (`numShoot` 5, `EmittePosType` 8) start at the same point; what type 8 spreads them by is not known.
- Stand-in looks (`crates/game/src/bullet_fx.rs`): shuriken small silver balls; carriers (thrown or lit firecrackers, balls, powder) small orange balls; the bang a bright sphere of 1.8 m; flames faint orange spheres; poison floor a purple disc of 2 m; smoke a grey 5 m sphere that fades over 1.2 s; the recovery cloud a violet sphere growing to 3.2 m.
  Screenshots: `cache/screens/tool_*.png`.

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
- Tools: healing items do not exist yet, so the recovery seal blocks nothing; the warp after the firecrackers (3506021) and the smoke vanish are not done; bullets do not hit walls ("Tools").
- Perilous-attack warning (the 211000 "dangerous attack" VFX from the dummy bullets) and Mikiri / jump counters on Wolf's side.
- The Owl rarely guards or deflects Wolf's slashes in our runs (2 blocks in a minute of slashing).
  His `Parry` deflects with a chance of 50 % per consecutive guard; the count rises on SpEffect 200250, which his weak guard reactions (`a000_008500`, `8501`, `8504`) add, through the shared table-logic interrupt now that observed SpEffects raise interrupts.
  Why he guards so seldom is not checked yet (his `Parry` also needs Wolf inside `GetDist_Parry` and both front cones).
  Until 2026-10-10 his blocks also played the hit flinch `a000_008010` instead of a guard reaction: his script indexes `guard_damage_table` by env 258, his NpcParam `guardLevel` (4; "which guard motion receives the enemy's attack"), which we answered 0. That env meaning is inferred from the param description and the table's shape, not traced in the exe.
- Owl warp transparency (3506030) is not rendered; bullets and their areas are simple stand-in shapes, not the game's FXR effects.
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
- Phase 2: see "Not done yet" above, and the steps below.

## Next steps

1. Decode the EMEVD bank used around entity 1110850 (boss health bar 905060, the phase-2 SpEffect 200051, the heat-up check on 3506080) and run his phase change from it; it also says which castle top floor (`m015070` or `m015072`) the fight shows.
2. His tools work ("Tools"); his grabs throw Wolf ("Grabs"), the boss brain's action list does not offer the grabs yet.
3. The boss brain is in place (`docs/BRAIN.md`, `--brain scripted`): next, a hosted decision-model controller behind the same `BossBrain` trait.

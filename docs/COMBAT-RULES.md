# Combat rules from the executable

What `sekiro.exe` 1.6.0.0 does when an attack touches a character, recovered by static analysis of the game code in Ghidra, from the author's own copy of the game.
The implementation is `crates/sim/src/combat/`, original Rust written from these notes.
No decompiled code is reproduced here or in the source; dumps stay under `re/`.

## Confidence labels

- **code**: the arithmetic, order and rounding were read from the named function.
- **inferred**: the shape was read from code, but which param field feeds a value was inferred, usually by matching record offsets to Paramdex offsets or by matching values.
- **def**: from the Paramdex Japanese description.
- **community**: from a community source; not checked in the executable.
- **data**: counted in the extracted params.

RVAs are relative to image base 0x140000000.

## Where things live

| Thing | Location | Confidence |
|---|---|---|
| HP, max HP | `SprjChrDataModule` +0x130, +0x134 (int) | code |
| Remaining posture, max posture | +0x148, +0x14C (int) | code |
| Posture history (trailing value) | +0x258 (int) | code |
| Deathblows left | +0x25C (int), set from NpcParam `ninsatuNum` at spawn | code (`bd6ae0`) |
| Recovery carry, last frame gain, speed | +0x15C (float), +0x22C, +0x230 | code (debug display `bd7630`) |
| No-posture-loss flag | +0x228 bit 4, or the global debug flag | code (`bd5c80`) |
| Active StaminaControl type | action module +0xB0C, -1 when none | code (`bd4e20`, `a04850`) |
| Damage type for env 202 | action module +0x5C | code (`b6bd40`, `b698d0`) |

Posture is stored as the amount *remaining*.
A fresh character has remaining equal to max (`bd6ae0` calls the setter with max).
The bar on screen is max minus remaining.
A break is remaining below 1.

## The hit record

When an attack connects, `10b6880` builds the hit record the defender's damage module consumes.
It looks up the attack's AtkParam row (record +0x50 kind, +0x54 id) and BehaviorParam row (+0x4C), and fills the slots below (read from code).

| Slot | Value | Source |
|---|---|---|
| +0x34 | Posture base on a deflect: attacker virtual 0x1E8 with the flag set (thunk `847860` loads 1) | `850f00` |
| +0x38 | Posture base on a block: virtual 0x1E8 with the flag clear | `8476f0` |
| +0x3C | Posture base on a direct hit: virtual 0x1F0 | `841b10` |
| +0x40 | Repel attack power | `84c650` |
| +0x44 | Extra value for record kind 64 | `844290` |
| +0x24 | `dmgLevel` | AtkParam +0x7A |
| +0xEC bit 1 | `disableStaminaAttack` | AtkParam +0x86 bit 1 |
| +0xEC bit 6 | `isGhostAtk`: cannot be guarded unless the defender passes a character check | AtkParam +0x86 bit 6 |
| +0xED bit 0 | `isDisableNoDamage` | AtkParam +0x86 bit 7 |

The posture virtuals for NPC attackers (`a12b20`, `a12f90` into `847d50`) return `directAtkStamDamage`, `repelLostStamDamage` or `atkStam` times the product of the attacker's SpEffect `staminaAttackRate` (`bfe3b0`, `c057d0` reads SpEffect +0xC8).
For player weapon attacks (`a24420`, `a25d70` into `847e10`) the base is `(staminaAttackPowerRate * field + correction / 100 * attackBaseStamina * reinforce) * reinforce * staminaAttackRate effects`, with `directAtkStamCorrection` for a direct hit, `atkStamCorrection` otherwise and `reinforce` the ReinforceParamWeapon `staminaAtkRate`.
The repel attack power for NPC attacks is `guardAtkRate` (`a13630`, `84c760`).
For player weapon attacks it is `trunc(guardAtkRateCorrection / 100 * weapon attackBaseRepel)` plus a stat bonus capped at 10 that Sekiro weapons do not give, minus 5 or 10 in two hand states; failing the weapon's stat requirements gives TentativePlayerParam `LowStatus_AtkGuardBreak` (5) instead (`a288a0`, `84c7b0`).
The Kusabimaru (`attackBaseRepel` 10) with the common correction 300 (349 of 443 AtkParam_Pc rows) gives 30.
Record slot +0x188 multiplies all three posture bases again in `844070` and `8439a0`; its writer was not found.

## Wolf's attack power

`--attack-power N` selects 1 to 99, default 1, in both the game and `duel-sim`.
`--progression castle-roof` selects AP 6; an explicit `--attack-power` overrides it regardless of argument order.
Invalid values, unknown presets and missing values are rejected before loading the fight.

**data**: attack power selects `SpEffectParam` row `160000 + N`, named Player Growth_Core Level N.
The row's `physicsAttackRate`, `magicAttackRate`, `fireAttackRate`, `thunderAttackRate`, `darkAttackRate` and `staminaAttackRate` all contain the same multiplier M.
The runtime reads these rows directly; it does not encode the following piecewise summary as a replacement table:

| AP | M |
|---|---|
| 1 to 14 | `1 + 0.25 * (AP - 1)` |
| 15 to 27 | `4.25 + 0.10 * (AP - 14)` |
| 28 to 51 | `5.55 + 0.05 * (AP - 27)` |
| 52 | `6.775` (data exception) |
| 53 to 99 | `6.75 + 0.01 * (AP - 51)` |

For HP, each element starts with `AtkParam.attackPower + weapon.attackBase * reinforce.attackRate * AtkParam.correction / 100`, then the active element attack rates multiply it before guard cuts and final integer truncation.
For posture, `staminaAttackRate` enters the existing player weapon posture base before defender cuts, attribute rates and integer truncation, including the checkpoint damage estimate.
The selected effect is resident, survives fighter resets, and replaces the previous AP effect rather than stacking.
Its `defStaminaAttackRate` also participates in the already-traced deflect feedback formula.

The starting CharaInitParam 10010 weapon is EquipParamWeapon 5100, with element bases 40, reinforceTypeId 1000 and zero `correctStrength`, `correctAgility`, `correctMagic`, `correctFaith` and `correctType_*`.
ReinforceParamWeapon 1000 has physical and stamina attack rates 1; reinforcement and weapon stat-correction graphs do not implement Memory progression.
The first slash's AtkParam_Pc physical correction is 200%, giving base HP damage 80, and its direct posture base is 30.
A 300% physical correction instead gives 120, as on the thrust.
CalcCorrectGraph 500/501/504 remains at Prayer Necklace growth 1 (320 HP, 120 posture, 30/s recovery); attack power must not feed those capacity/recovery graphs.
Prayer Necklace progression remains outside this setting.

**community cross-check**: [Attack Power](https://sekiro-shadows-die-twice.fandom.com/wiki/Attack_Power) reports the opening slash as 80 at AP1 and 340 at AP14, matching these rows.
[Memories](https://sekiro-shadows-die-twice.fandom.com/wiki/Memories) identifies the boss Memories that raise AP.
The AP6 castle-roof preset represents consuming the five required-route Memories (Gyoubu, Genichiro, Folding Screen Monkeys, Guardian Ape and Corrupted Monk).
Optional Lady Butterfly and Headless Ape Memories can put Wolf at AP7 or AP8 before Great Shinobi Owl; the preset is a reproducible route choice, not a universal measured average or a full save-state preset.
FwLo's local RustSekiro clone was checked (`docs/kb/damage.md`, `src/player.rs`); it leaves the attack-power graph unknown and supplies no traced AP formula to borrow.
The effect selection is supported by the extracted row names and the community cross-check; the progression-to-effect attachment has not been independently traced in the executable.

Real HKS/TAE slash regression (`crates/sim/tests/attack_power.rs`):

| AP | Soldier HP / posture | Guarding Owl HP / posture |
|---|---|---|
| 1 | 80 / 30 | 0 / 15 |
| 5 | 160 / 60 | 0 / 30 |
| 10 | 260 / 97 | 0 / 48 |

The soldier normally has 195 HP, so AP10 actually removes at most 195 HP; the formula regression raises his HP to avoid this existing `keep_hp` cap.
Posture damage is the computed hit amount before the posture meter clamps it to its floor.
The AP6 guarded-pressure regression requests one slash every 36 ticks for 600 ticks: 11 guarded contacts, remaining posture after hits `[628, 611, 613, 559, 560, 565, 566, 512, 513, 522, 523]`, lowest 512 of 660.
AP1's same cadence bottoms at 624, so progression now builds Owl's posture under this scripted pressure, but does not break it within 10 seconds.
Guard recovery remains the extracted 60/s; reaction clips, deflects and combo variation account for the varying per-contact loss and recovery intervals.
All 99 extracted growth rows are checked against the piecewise summary in a separate regression.

## Order of one hit

The defender's damage module runs `b6f690` per hit record, which calls in order:

1. `b6c880`: guard test, deflect test, posture damage and HP damage numbers.
2. `b6e6a0`: applies HP and posture, decides posture break, guard break and deathblow readiness, computes the attacker's posture damage for the break flag.
3. Later stages: knockback (`b6bfe0`), damage type selection (`b6bd40`), events.

The attacker's damage module runs `b698d0` for the same contact, which picks the attacker's reaction code and applies the attacker's own posture damage.

## Hit, block or deflect

| Rule | Source | Confidence |
|---|---|---|
| A guard is only possible while the defender is in a guarding state and not in a guard-disabled state. | `b6c880` | code |
| The guard arc passes when the dot product of the defender's facing and the attack's travel direction is below `cos(guardAngle + 180 degrees)`, so `guardAngle` is a half-angle. | `b6c880` (constants 180 and pi/180) | code |
| `guardAngle` 0 means a threshold of 0, a 90 degree half-angle. The Kusabimaru has 0, the soldier 60. | `b6c880` | code |
| NPCs read `guardAngle` from NpcParam, the player from the weapon. | `a13820`, `a295c0` | code |
| `isAllDirGuard` skips the arc test. | `b6c880` | code |
| The arc test runs before the block or deflect decision, so it gates deflects too: a press inside the deflect window while facing away is a direct hit. | `b6c880`; also traced by FwLo's RustSekiro, `docs/kb/guard-deflect.md` | code |
| Repel defence of a guarding NPC is the guard row's `guardBreakRate` times the product of `guardDefFlickPowerRate` over active SpEffects with `stateInfo` 158 or 204, truncated. | `a137a0`, `84cd80`, `bfccc0` | code |
| Repel defence of the guarding player is `guardBreakCorrection / 100` times the weapon's `guardBaseRepel` times a guard effect rate, plus a stat bonus capped at 10 that Sekiro does not use. | `a28da0`, `84ce20` | code, stat term inferred unused |
| Repel defence of a non-guarding NPC is NpcParam `defFlickPower`. | `a137e0` | code |
| Every active SpEffect's `defFlickPower` overrides repel defence when it is higher. | `bfc440` | code |
| Repel defence below the attack's repel power is a block; at or above it is a deflect. | `b6c880` | code |
| The attack's repel power is record +0x40: `guardAtkRate` for NPC attacks, a weapon formula for the player (see the hit record). | `10b6880`, `84c650` | code |
| A block becomes a direct hit when `disableGuard_vsGuardAttribute[g]` is set, where `g` is the guard row's `guardAttribute` (0 or 1). | `b6c880`, table at 0x142a7ce28 | code |
| A deflect becomes a direct hit when `disableJustGuard_vsGuardAttribute[g]` is set. | `b6c880`, table at 0x142a7ce38 | code |
| A hit on an armoured body part is guarded whatever the defender is doing when the attack row's `guardAtkRate` is below the part row's `guardBreakRate`; the part row (NpcParam `partsAtkParamId1..8`, chosen by the part group 1 to 8 of the hit collision body) then acts as the guard row. | `b6c880`, `1058380`, `10c66a0` | code |
| On an unguarded hit on a body part, the part's damage group sets the HP and posture rates (record +0x210, +0x214) through damage module virtuals 0x30 and 0x38; group 31 has extra handling. | `b6c880` | code, rates not traced |

Worked numbers: ordinary NPC attacks have `guardAtkRate` 30 (2154 of 2396 AtkParam_Npc rows).
The just-guard window effect 105010 has `stateInfo` 158 and `defFlickPower` 40, so a guard inside the window deflects.
Outside the window the Kusabimaru's `guardBaseRepel` is 20, so the guard blocks.
Rows with `guardAtkRate` 100 (213 rows) can never be deflected by the 40 override.
Every AtkParam_Pc row has `guardAtkRate` 0, but player weapon attacks use the weapon formula (30 for the Kusabimaru), so a guarding NPC deflects the player when its guard row's `guardBreakRate` times its effect rate reaches 30.
Armoured parts compare the raw `guardAtkRate`, so any part with a positive `guardBreakRate` guards against the player.

The arc rule in one line: guarded only when `dot(defender facing, direction to the attacker) > cos(half-angle)`; Wolf's half-angle is 90 degrees (the front half), the soldier's 60.
FwLo's RustSekiro (`docs/kb/guard-deflect.md`) reads Wolf's half-angle from the `ShieldBlock` TAE flag's ArgB (90 in every Wolf guard animation).
Our trace of the player's virtual (`a295c0`) reads it from the weapon's `guardAngle` instead, and ArgB is the guard judge here (the soldier's ArgB is 900, which cannot be an angle), so the sim keeps the weapon value.
Both give 90 degrees for Wolf.

## Perilous attacks

Perilous attacks are the unblockable flags, not a separate system.
In AtkParam_Npc, 238 rows block both guard types for both block and deflect (grabs), 60 rows block only the katana (guard type 0) for both (the Loaded Umbrella still blocks them), and 111 rows forbid only blocking with the katana, so they must be deflected.
The Mikiri counter intercepts a counterable thrust contact before ordinary hit resolution.
Code: `crates/sim/src/perilous.rs`, `crates/sim/src/hits.rs` (`AttackData::perilous`), the duel; tests `crates/sim/tests/perilous.rs`.

### The warning (å±)

All from the data (**data**, except where marked):

1. The clip fires a TAE `BulletBehavior` with a judge from 980 to 989 a little before the blow: soldier sweep `a000_003020` at 0.2 s (blade 1.067 s), soldier thrust `a000_003013` at 0.0 s, Owl sweeps `a000_003004` / `3024` / `3090` at 0.0 s (judges 980, 985, 986), Owl Shinobi stab `a000_003033` at 0.667 s (987).
2. Its BehaviorParam row names a "dummy bullet, dangerous attack sign" (soldier 10001600, Owl 50601980 to 50601987): 2 m radius, 0.1 s, AtkParam 8 (no damage), `EmittePosType` 7, `generationMaxDistWhenOwner` 5 m (soldier) or 9 m (Owl), SpEffect 211000.
   We read `EmittePosType` 7 as "made at the target" (**inferred**: only these rows use it, and a maximum distance from the owner only makes sense for a bullet made away from him), so no warning is made when Wolf is farther than that from the attacker.
3. SpEffect 211000 "dangerous enemy attack VFX" (0.1 s) only lands on the player (`effectTargetPCOnlyTarget`, `effectTargetOpposeTarget`); its `vfxId` 48100 is SpEffectVfxParam "æ¼¢å­—_ã€Œå±ã€", effect 600035 (`sfx/sfxbnd_commoneffects`, `f000600035.fxr`) at Wolf's dummy 220 (`Spine2`).
   So the symbol is on Wolf, not over the attacker.
4. sekiro-rs reports the warning (`DuelReport::perilous`) when a bullet hit on Wolf carries an effect with `vfxId` 48100, and the HUD draws a red å± over Wolf for 0.8 s with a Japanese Windows font (`combat_hud::update_perilous`), as FwLo's RustSekiro does; the effect's texture and timing (the FXR) are not decoded, and no sound for it was found (FwLo has none either).

What to do about it is not in the warning: FwLo maps judges 980 / 982 / 983 to sweep / thrust / grab, which holds for the Ashina General but not in general (the Owl's sweeps use 980, 985 and 986).
Here the kind comes from the clip's next attack window: a grab (`throwFlag` 1), a sweep (katana can neither block nor deflect it), or a thrust (can deflect) (**inferred** split; every perilous sweep in these two characters is undeflectable and every perilous thrust deflectable).

### Jumping a sweep, and the jump counter

- TAE 954 `IFrameType` 1 ("Sweeps") on Wolf's jumps (from the jump start), his air kicks and the first 0.27 s of his steps lets a sweep pass unspent (FwLo's rule for the attacks with a sweep warning; ours uses the AtkParam kind above).
  General (2) and thrust (6) i-frames are still not applied.
- A jump press in the air is Wolf's own HKS air kick `a000_213100`, whose `PCBehavior` (TAE 307) judge 901 is BehaviorParam_PC 901, AtkParam_Pc 901 "kick in the air": `directAtkStamDamage` 10, `staminaPhysicsAttribute` 7 (anti-ground), dummies 612 to 614 (fixed in Wolf's frame on a root-level helper node, like the throw dummies).
  sekiro-rs now detects the damaging `PCBehavior` rows (the kicks 900, 901, 903); the contact-impact rows on every jump and landing (200 to 222, no damage) and the hand and foot effect probes (280 to 286) are left out (ours).
- Every perilous sweep puts SpEffect 400200 "jump kick effective when posture is low" on its attacker for most of the clip (soldier `3020` 0.0 to 2.667 s, Owl `3004` 0.133 to 1.567 s; the tutorial soldier's `3014` has 310601), `stateInfo` 912, `defAntiGroundStaminaDmgRate` 8 (40 on 310601).
  The existing posture rule (per-attribute rate by `staminaPhysicsAttribute`, **code**, above) then makes the kick's posture damage 10 x 8 = 80: the soldier's 75 breaks (`TrunkCollapseFront`, deathblow open), the Owl (660) takes 80.
  The Owl's sweeps also carry 3506001, which replaces his damage levels for the hit reaction.
- The kick off the enemy (`AirKickEnemyJumpStart*`) is Wolf's HKS: it needs env 2004 "swing is hitting" while the kick's SpEffect 100334 (reference 204) is up.
  The duel now answers env 2004 true from the step one of Wolf's attacks connects until his clip changes (how long it stays true is **inferred**); the script reads it only with reference 204 or a combat art's jump reference.
- `DuelReport::jump_counter` says when an anti-ground hit landed on an attacker with a `stateInfo` 912 effect.
- So a jump that is too early (landed before the blade) or too late (the blade arrived before the jump start) is hit, and a kick before or after the attacker's 912 window is an ordinary 10-posture kick, all from the TAE windows; there is no timing constant of ours in it.

### Mikiri Counter

Implemented for the soldier and Great Shinobi Owl on 11 October 2026.
FwLo's RustSekiro (MIT, local clone; the requested pull failed because GitHub was unreachable) supplied the flag-125/contact rule, the `staminaDamageAttackHitParry` field, and the ThrowParam suffix 190 / posture-break suffix 120 selection.
His knowledge base describes these as data findings; the break-pair choice also has live c1010 observations in `docs/kb/damage.md`.
The following values were checked against our own extracted v1.6 data, rather than using his General-specific animation offsets.

| Source | Finding | Confidence |
|---|---|---|
| Wolf TAE `a000_213301` (forward step; neutral `213300` aliases it) | ChrActionFlag 125 `MikiriCounter` and SpEffect 100254 (reference 16) run from 0.000 to 0.400 s, 12 frames at 30 Hz. The counter reads the flag; no timing constant or input-buffer change. | data |
| Soldier TAE `a000_003013` | Damaging judge 230 and counter-only judges 940 / 941 run from 0.767 to 0.933 s. | data |
| Soldier BehaviorParam 210100230 / 210100940 / 210100941 | AtkParam 10100230 / 10100940 / 10100941. The real thrust has `atkType` 2, katana `disableGuard` 1, `disableJustGuard` 0, and `staminaDamageAttackHitParry` 90. The category-6 dummy rows deal no damage, carry the same 90, and have wider capsules. | data |
| Owl TAE `a000_003033` | Damaging judge 310 runs 1.467 to 1.867 s; counter-only judge 311 runs 1.333 to 1.867 s. | data |
| Owl BehaviorParam 250600310 / 250600311 | AtkParam 50600310 / 50600311. The real thrust has the same type/guard/parry fields as the soldier, including 90 posture; the category-6 dummy deals nothing. Row 50601310 is the corresponding variation copy. | data |
| ThrowParam 11010190 / 15060190 | Surviving-posture pair: Wolf `a200_511800` / `a221_511800`, enemy `ThrowDef13800`; absorb dummy 522 / 523, interpolation 0.5 s. | data |
| ThrowParam 11010120 / 15060120 | Posture-break pair: Wolf `a200_511100` / `a221_511100`, enemy `ThrowDef13100`; absorb dummy 520 / 523, interpolation 0.5 s. | data; selecting it on projected empty posture follows RustSekiro's observed rule |
| `c0000.hks`, `c0000_cmsg` and `c0000.hkx` | No dedicated named Mikiri branch: native `W_ThrowAtk511800` / `W_ThrowAtk511100` events enter the graph's `NewThrowAtk_SM` states. HKS runs normal `UpdateState`, clears input via act 9101 and does throw bookkeeping. | script/graph |
| Shared NPC HKS and each enemy's `c9997.hkx` | `W_ThrowDef13800` / `W_ThrowDef13100` enter `ThrowDef_SM`; `ThrowDefCommonFunction` asks env 273 for its animation id and env 206 for throw state. The duel supplies these for the pair; env 339 remains the existing clip-end signal. | script/graph; native event injection inferred |

Counter-only contacts are recognised by category 6, zero attack power, type 0, no on-hit effects or special behaviour id, and positive parry posture, and associated with the clip's upcoming perilous thrust (**inferred** classification).
The type and behaviour checks preserve Owl's zero-damage blinding hit (type 1, behaviour id 17), which is not a counter dummy.
They are not ordinary hits, and are not spent before flag 125 opens.
A damaging thrust contact can also start the pair if its `atkType` is 2 and its katana guard flags identify a perilous thrust, following RustSekiro's rule (**inferred** engine predicate).
Ordinary blockable thrusts are therefore not countered by that fallback; a dedicated counter dummy associated with a blockable attack is not yet identified or enabled.
The damaging blade and dummy contacts are checked before resolving any damage that step, so a counter cancels Wolf's incoming hit and its on-hit effects.
No sweep, grab, backward step or side step can select a Mikiri pair.
The enemy must also be in Wolf's forward hemisphere at contact, so an unlocked forward step away from a thrust cannot counter it (**inferred** direction gate).

The counter applies the contacted row's 90 posture directly through the existing posture setter, without HP damage; the soldier's 75 breaks and Owl's 660 does not.
Both play the real pair with the existing absorb mechanism, and the hold ends when Wolf leaves the counter clip through its data-driven cancels or clip end.
The pair's initial facing and absorb behaviour use the existing throw convention (**inferred**); the native counter implementation has not been traced in Ghidra.
Learned-skill `attackHitParryStaminaAttackRate` bonuses are not modelled here.
The posture-break follow-up deathblow pairs (suffixes 121 / 122) are not automatically chained; this implements the counter and reaction, not that optional execution.
Mikiri does not ask env 2004; that remains the swing-hit result for the jump counter described above.
Tests replay real HKS, graph, animation and hit detection: forward steps at ticks 35 / 75 into forced 3013 / 3033 at 2 m counter for 90 with both reaction states and no Wolf hit; early, late, backward, side, sweep and ordinary slash cases do not, and the pair releases normally.

## Posture damage to the defender

| Outcome | Base | Source | Confidence |
|---|---|---|---|
| Direct hit | `directAtkStamDamage` | `844070`, record +0x3C | code |
| Block | `repelLostStamDamage` | `8439a0`, record +0x38 | code |
| Deflect | `atkStam` | `8439a0`, record +0x34 | code |

| Rule | Source | Confidence |
|---|---|---|
| The base already includes the attacker's `staminaAttackRate` effects (see the hit record) and is multiplied again by record +0x188. | `844070`, `8439a0` | code; +0x188 writer not found |
| Then by the defender's per-attribute rate chosen by `staminaPhysicsAttribute`: 1 slash, 2 light hit, 3 thrust, 4 neutral, 5 deathblow, 6 heavy hit, 7 anti-ground, 8 anti-air, 9 light shoot, 10 to 12 attributes A to C, anything else 1.0. | `844660` (NpcParam +0x27C on), `10ce8d0` (SpEffect +0x328 on) | code |
| SpEffect per-attribute rates: on a direct hit all effects except `stateInfo` 158, 204, 110 and 300 multiply; on a block only 158 effects; on a deflect only 204 effects. | `bfafa0` | code |
| On a block or deflect an NPC applies a guard cut of `clamp((1 + guardRow.guardStaminaCutRate / 100) * staminaGuardDef * product of guardStaminaCutRate over 158 or 204 effects, 0, 100)` percent, then adds the guard behaviour's BehaviorParam `stamina`. | `840550`, `bfcd40` | code, guard row identity inferred |
| The player's guard cut is `clamp((weaponDef * staminaGuardDefRate + stat bonus + 1) * (1 + guardRow.guardStaminaCutRate / 100) * product of guardStaminaCutRate over 158 or 204 effects, 0, 100)` percent, where `weaponDef` is `staminaGuardDef` on a block and `staminaJustGuardDef` on a deflect and the stat bonus comes from `staminaGuardDef_MaxCorrect` through CalcCorrectGraph 163 (0 on the Kusabimaru). The guard behaviour's `stamina` is added as for NPCs. | `a24180`, `840870`, `845d80`, `bfcde0` | code |
| In an unidentified guard state (a value of 2 or 3 at PlayerIns +0x2140, +8) the stat bonus is scaled by 1.5 and the result by 0.7 for weapon category 12 or 0.9 otherwise; a further percent at PlayerIns +0x21EC applies when a module flag is set. | `840870`, `a24180` | code, states not identified |
| The value is truncated to int, multiplied by a player-versus-enemy correction product and the hit part's rate, and truncated again. | `b6c880`, `8480a0` | code |
| `disableStaminaAttack` zeroes the applied posture damage but the break test uses the value projected before zeroing. | `b6e6a0` | code |

With the soldier's slash (AtkParam_Npc 10100100) and neutral multipliers, Wolf takes 18 on a direct hit.
On a block the constant 1 in the player's cut makes it `trunc(0.99 * 18)` = 17, and on a deflect `trunc(0.99 * 9)` = 8, before any guard behaviour cost.

## Posture damage to the attacker

| Outcome | Value | Source | Confidence |
|---|---|---|---|
| Direct hit | `directAtkStamDamage_Attacker` | `842790` | code |
| Blocked, or the block broke the guard | `repelVictoryStamDamage_Attacker` | `842790` | code |
| Deflected | `trunc(repelLostStamDamage_Attacker * weapon rate * product of defStaminaAttackRate over the defender's 204 effects)` | `842790`, `bfc490` | code |

The weapon rate is the defender's weapon `passiveStaminaAtkRate` times a reinforcement rate, 1.0 for NPC defenders.
The attacker's module applies the result with the same posture setter (`b698d0`).
Soldier deflected three times in a row: 45, then 22 (45 * 0.5 = 22.5 truncated), then 11 (11.25), for 105020, 105021 and 105022; 105023 would give 5.

## The posture setter

All posture changes go through `bd6710` (remaining = absolute target) and its helper `bd4e20`.

| Rule | Source | Confidence |
|---|---|---|
| A decrease is ignored while the no-posture-loss flag is set. | `bd6710` | code |
| The new value is clamped to `[floor, max]`. | `bd4e20` | code |
| The floor is NpcParam `maxDebtStamina` for NPCs (soldier -30) and TentativePlayerParam `DebtSp` for the player (0). | `bd4e20`, param table 0x7E at +0x4C | code |
| With a StaminaControl type active, the value is also clamped between `minRatio * max / 100` and `maxRatio * max / 100` (integer division). | `bd4e20`, `10cf390`, `10cf4b0` | code |
| The history value drops by the full loss, or with the combat flag only by `ceil(0.2 * loss)`, rises to the new value on a gain, and never exceeds max. The 0.8 is a global at 0x143b0a068. | `bd6710` | code |
| Max posture is `trunc(flat + rate * base)` from SpEffects, and remaining is clamped to `[-100, max]` after a change. HP does not enter it. | `bd60f0`, `bfd500`, `bfd4c0` | code, field roles inferred |

## Breaks, deathblows and HP protection

| Rule | Source | Confidence |
|---|---|---|
| On a direct hit with positive posture damage, a projected remaining below 1 is a posture break. | `b6e6a0`, record +0x1C6 | code |
| On a block, a projected remaining below 1 is a guard break. | `b6e6a0`, record +0x1C4 | code |
| On a deflect no break is recorded for the defender, even when its posture reaches the floor; the next block or hit breaks it. | `b6e6a0` | code |
| HP below 2 with at least one deathblow left also counts as a break (deathblow ready). | `b6e6a0` | code |
| HP is clamped to `[0, max]`; a drop below 1 from a positive value is held at 1 while deathblows remain or a no-death condition holds (no-dead flag, SpEffect state 143, a player-only state 235, debug). | `bd64e0`, `bd5df0` | code |
| A deathblow hit spends one deathblow before HP is applied. | `b6e6a0` (record +0x28 equal to 5) | code, marker inferred |
| `excessDmgKeepHp` caps HP damage so HP stays at that value; at or below it the damage is 0. | `b6e6a0` | code |
| When the attacker's remaining posture minus its repel damage is below 1, the contact is flagged as breaking the attacker. | `b6e6a0`, record +0x1CF | code |

What happens after a break (the deathblow window length, the recovery from a broken state) is driven by the reaction animations, HKS and TAE, not by this code as far as traced.

A guard break opens a deathblow just as a direct hit's break does.
The block leaves the defender's posture empty, and the guard-break reaction carries the same deathblow-ready SpEffect as the posture-break stagger: Emma's script answers damage type 1001 with `GuardBreakLeft`, whose clip `a000_008551` keeps SpEffect 220420 on, the effect the soldier's `TrunkCollapse` clip `a000_008300` holds while he can be deathblown (see `docs/DEATHBLOW.md`).
So the sim's hit report (`HitResult::posture_broke`) counts a guard break as a posture break of the defender, alongside direct-hit breaks and attackers broken by a deflect or block (#192).

## Damage type codes (env 202)

Defender, in the engine's order of precedence (`b6bd40`):

| Code | When |
|---|---|
| 9, 7 | Scripted special states (not modelled) |
| 2 | Dead |
| 1002 | Record kind 63 (not modelled) |
| 1004 | Guard break with damage level 4, 7 or 10 |
| 1005 | Guard break with damage level 6 |
| 1028 | Deflected, attacker broken, attack has `doesBreakRepelStamDamage` |
| 3 | Deflected otherwise |
| 1001 | Guard break |
| 12 | Blocked through the clash path |
| 1028 | Blocked, attacker broken, `doesBreakRepelStamDamage` |
| 3 | Blocked otherwise |
| 1027 | Direct hit with a posture break or HP deathblow readiness |
| 99999 or 10 | Ordinary hit, the reaction then follows damage level |

Attacker whose attack was guarded (`b698d0`), keyed by the attack's BehaviorParam `category`:

| Outcome | Default | Category 2 | 6 | 7 | 8 |
|---|---|---|---|---|---|
| Blocked | 1017 | 1018 | 1019 | 1020 | 1021 |
| Deflected | 1000 | 1003 | 1008 | 1009 | 1010 |
| Deflected, clash path | 1022 | 1023 | 1024 | 1025 | 1026 |

A deflect that breaks the attacker with `doesBreakRepelStamDamage` reports 1033.
Wall impacts report 1006 or 1007 (`b71ef0`).
These match the HKS branches in `docs/HKS-COMBAT.md`: 1000, 1003 and 1033 select the hard-deflected recoil and 1017, 1018 the easy one.

## HP damage

| Rule | Source | Confidence |
|---|---|---|
| Five elements (physical, magic, fire, lightning, dark) each go through an attack-versus-defence curve, are multiplied by seven per-element multiplier arrays and clamped at 0, then summed. | `840eb0` | code |
| The curve is piecewise quadratic in `attack / defence` with knots 0.12, 1, 2.5 and 8; its five output percents are zero-initialised globals at 0x143d5c100..114 that nothing writes (only the curve reads them, and a debug menu labelled "growth defence settings" binds them in `848c50`), so the curve returns the attack unchanged and defence has no effect. | `840ce0`, `848c50` | code |
| A positive total below 1 rounds up to 1. | `840eb0` | code |
| The damage manager (`6dba10`) passes the total through unchanged unless a map event registered a damage override for the defender's hit part, in which case it replaces the damage and the damage levels. | `6dba10` | code |
| The total is then multiplied by the hit part's rate and a repel cut (when repel defence met the attack's repel power), then truncated. | `b6c880`, `841c80` | code, repel cut source not traced |
| A blocking NPC's physical multiplier is `(100 - k * physGuardCutRate * (1 + guardRow.guardRate / 100)) / 100`, where `k = 1 + NpcParam <attr>GuardCutRate / 100` for the attack's `atkAttribute` (slash, blow, thrust, neutral, deathblow, heavy hit, anti-ground, anti-air, light shoot, A, B, C). | `845ed0`, `10c60d0` | code |
| The guarding player's physical multiplier is `(100 - k * (cut * reinforce physicsGuardCutRate + stat bonus) * durability * (1 + guardRow.guardRate / 100) * special) / 100`, with `cut` the weapon's `physGuardCutRate` on a block or `physJustGuardCutRate` on a deflect, `k = 1 + weapon <attr>GuardCutRate / 100`, durability 1.0, 0.7 or 0.5 by weapon durability (Sekiro weapons have none) and `special` a factor for blocks in an unidentified state. | `8460d0`, `10d1b40`, `841bd0` | code |
| Damage at least 1.5 times max HP sets an extra flag. | `b6e6a0` | code |

The soldier's slash (80 physical) on a defender with neutral multipliers deals 80.
The Kusabimaru (`physGuardCutRate` and `physJustGuardCutRate` 100) lets none through; the soldier's own block has `k` = 2 from `slashGuardCutRate` 100, so it also blocks everything.

## Posture recovery

| Rule | Source | Confidence |
|---|---|---|
| Speed per second is `product of staminaRecoverSpeedRate over active effects * (base + sum of staminaRecoverChangeSpeed) * anim percent / 100 * character scale`, and times `staminaRecoverRatio / 100` when a StaminaControl type is active. | `a04850`, `bfe360`, `c05740` | code; the sum and the anim percent source inferred |
| NPC base is NpcParam `staminaRecoverBaseVel`. | `a13a70` | code |
| The player's base is CalcCorrectGraph 504 ("Stamina recovery speed") at the player game data value +0x248, truncated: 30 at 1, rising linearly to 105 at 11 and above. Which progression value +0x248 is was not identified. | `a2a5d0`, `a2a680`, `844cc0`, `850b10` | code |
| CalcCorrectGraph evaluation: input capped at the last breakpoint, the first segment whose upper breakpoint is at least the input, output `g0 + t^e * (g1 - g0)` for `e >= 0` or `g0 + (1 - (1 - t)^(-e)) * (g1 - g0)` for `e < 0`, clamped to the segment. | `850b10` | code |
| Each frame the engine adds `speed * dt` to a float carry, applies the whole part through the posture setter and keeps the fraction. | `a04850`, `5460b0`, `9e6bf0` | code |
| The update is skipped while an action-state flag (bit 21 of +0x88) is set. The only writer in the executable (a scan of every `OR [reg + 0x88], 0x200000`, `BTS` and `AND 0xffdfffff` against that word) is the HKS command handler `b32410` case 0x6e, reached from `b4e520`: `act(110)`, community name `SetStaminaRecoveryDisabled`. The TAE ChrActionFlag handler `b51b70` writes +0x88 bits 1, 2, 4 to 12, 15 to 20 and 22 to 24, never 21. None of the shipped HKS chunks (`c0000`, `c9997`, `c1010`, `c5060` and the other 77 in `action/script`) calls `act(110)`, so no vanilla character is ever affected. Sim: `CombatRules::recover_with_state`, `PlayerEnv::recovery_disabled`. That the engine clears the flag every frame is inferred. | `a04850`, `b32410`, `b4e520`, `b51b70`; script disassembly | code; per-frame clearing inferred |
| A guarding character with neither TAE 225 nor a StaminaControlParam type on the guard clip recovers at full speed. The Owl's `a000_003100` and `a000_003101` carry neither, so at full HP he recovers at his `staminaRecoverBaseVel` of 60 per second while guarding, after 0.3 s of type 0 (0%) in the guard-hit reaction `a000_008501`. At AP1 a guarded Wolf slash costs him 14 to 15 of 660, so a slash every 0.6 s is refunded within about 0.25 s of each hit (`guarding_owl_under_pressure`). The soldier's guard has TAE 225 at 25%, so the same pressure breaks him in three hits (`guarding_soldier_under_pressure_breaks`). | NpcParam 50600000, c5060 and c1010 TAE | data |
| The StaminaControl row is NpcParam `staminaControlParamId` for NPCs and row 0 for the player. | `bd58c0` | code |
| The type comes from the current animation (TAE event 960). | Smithbox annotation | community |
| No separate post-damage delay timer exists in this path; pauses come from the animation's type (type 0 is 0% in rows 0 and 1000100). | `a04850` | code for the absence in this function |
| HP lowers recovery only through `conditionHp` SpEffects, which all multiply when active. | `bfe360`, def of `conditionHp` | code for the product, def for the gate |
| TAE 225 `SetSPRegenRatePercent` on the playing animation multiplies recovery by its percent; the sim feeds it in as the anim percent above. Soldier (c1010): 25% in its guard `a100_003100`, 0% in several attacks and reactions; FwLo reports 33% on the Samurai General's guard and deflect animations. | event and effect traced by FwLo's RustSekiro, `docs/kb/guard-deflect.md` | community; that it is the anim percent byte is inferred |

Player row 0: type 1 is 67%, type 5 pins posture full (min 100%), type 10 is 200%.
Soldier row 1000100: type 2 is 33%, type 4 200%, type 6 pins posture at 30%, type 7 keeps it full, type 10 200%.

Soldier numbers (base 20, resident effects 300600, 300601, 300602 gated at 80%, 60% and 40% HP with rates 0.6, 0.5 and 1/3):

| Soldier HP | Active gated effects | Recovery per second |
|---|---|---|
| 195 (100%) | none | 20 |
| 150 (77%) | 300600 | 12 |
| 110 (56%) | 300600, 300601 | 6 |
| 60 (31%) | all three | 2 |

## Invulnerability

| Rule | Source | Confidence |
|---|---|---|
| TAE 950 (`IFrames_MistRaven`, flag word +0x70 bit 1) and 951 (`IFrames_DuringAction`, bit 0) make the character fully invulnerable while active: no attack lands. 950 covers all of Wolf's knock-down, launch, death and revival animations (for example `a000_100400` frames 0 to 18), 951 the deathblow throws. | traced by FwLo's RustSekiro, `docs/kb/damage.md` | community |
| An attack window that passes through an invulnerable character is not spent: it can still hit once the invulnerability ends. | FwLo's RustSekiro, `docs/kb/guard-deflect.md` (said of dodges) | community, inferred for 950/951 |

The check is `hits::can_be_hit`, run for both characters in `HitDetector::detect` before the capsule test.
TAE 954 typed i-frames (`IFrameType` general, thrust, grab, sweep) are decoded but not applied yet.

## Knockback

| Rule | Source | Confidence |
|---|---|---|
| The push distance is AtkParam `knockbackDist_DirectHit`, `knockbackDist_Guard` or `knockbackDist_JustGuard` (metres) for a hit, block or deflect. | def; FwLo's RustSekiro, `docs/kb/damage.md` | def |
| An NPC defender cuts it by NpcParam `knockbackRate_vsPlayer_*` percent against the player's attacks (`knockbackRate_vsEnemy_*` against other characters): `dist * (1 - rate / 100)`. The description calls these cut rates; the soldier's are all 0. | def; FwLo's RustSekiro | def |
| TAE 226 `SetKnockbackPercent` on the defender's own animation multiplies the distance at the start of the push (module +0x40 = percent / 100). Wolf: Combo1 (`a050_300000`) 0% at its hit frames, Combo2 50%, Combo3 to 5 30%. | traced by FwLo's RustSekiro (`ba34a0`, `ba37e0`), `docs/kb/damage.md` | community |
| Timing comes from the defender's KnockBackParam row: hold speed `v` for `<class>_ContTime`, then fall linearly to 0 over `<class>_DecTime`, with `v = dist / (cont + dec / 2)`. | FwLo's RustSekiro, `docs/kb/damage.md` | community |
| The row is NpcParam `knockbackParamId` for NPCs (0 on every row) and EquipParamProtector `knockbackParamId` for Wolf (1, "For PC", on every outfit piece). The sim takes the highest over CharaInitParam 10010's four armour pieces; which piece the engine reads is not traced. | found in the data by FwLo's RustSekiro | inferred |
| The class comes from the outcome and `dmgLevel`. Direct hits: 1 small `damage_S`, 2 middle `damage_M`, 3 large `damage_L`, 4 ex-large and 10 ex-blast `damage_BlowM`, 5 push `damage_Push`, 6 fling `damage_Strike`, 7 small blow `damage_BlowS`, 8 minimum `damage_Min`, 9 upper `damage_Uppercut`, 11 breath `damage_Breath`. Blocks and deflects use the size of the player's guard reaction: `guard_L` for levels 3, 4, 6, 7 and 9, `guard_LL` for 10 and 11, `guard_S` otherwise. | matched by name, guard sizes as in FwLo's RustSekiro | inferred; the engine's choice in `b6bfe0` is not read |
| The push moves the defender away from the attacker on the ground plane, through its collision (`Ground::slide`), so walls stop it. | | inferred |

Not modelled: Wolf's armour `knockBackCutRate_*` and weapon `knockBackCutRate_*_Guard` (0 for Wolf), the leftover push handed to the other character when the pushed one is blocked (`ba34b0`, from FwLo's notes), and `guardBrake` timing on guard breaks.

## Hit stop

Each character has a hit stop module (`SprjChrHitStopModule`, ChrIns module bag +0x90).
Its update (`b9d1b0`) turns the frame time into the character's own time, and the character update (`9f78a0`) runs every module on that scaled time, so a stopped character's animation, script, TAE, root motion and pushes all wait together.
FwLo's RustSekiro (`docs/kb/guard-deflect.md`, `src/combat.rs`) pointed at the two AtkParam fields; it freezes both sides fully for the field's seconds on direct hits, which the code below refines.

| Rule | Source | Confidence |
|---|---|---|
| A contact starts the hit stops from the defender's damage module (`b6f690` calls `b6e5a0` after the damage and posture stages), with no test of the outcome, so blocked and deflected contacts count as well as hits. | `b6f690`, `b6e5a0` | code; that every guarded contact reaches `b6f690` is inferred |
| Nothing starts when all five attack powers of the hit record (+0x00 to +0x10) are 0 or less. | `b9cd90`, `b9cfb0` | code; not modelled, the attacks we resolve always carry attack power |
| The attacker's length is AtkParam `hitStopTime` (+0x1C) times a scale from the defender's NpcParam `hitStopType` (+0x14A): 0 none, 1 the engine's weak rate 0.5, 2 full, 3 full only while an attacker action state is set (ActionFlag +0x54 bit 3, not identified; no NpcParam row uses 3, we treat it as none). A player defender counts as type 2. | `b9cd90`, `b9d850`, defaults in `a51800` (+0x124 = 0.5) | code, fields matched by Paramdex offset |
| The defender's length is AtkParam `hitStopTime_Defencer` (+0x1CC) when its own NpcParam `hitStopType_Defencer` (+0x125) is 1, else none. A player defender always takes it. | `b9cfb0`, `b9d8c0` | code, fields matched by Paramdex offset |
| TAE `ChrActionFlag` 41 on the defender (community name "HasSpace_Hks305") forces both scales to full unless the attack is `isArrowAtk`. Wolf, the soldier and the Owl never set it. | `b51b70` (sets ActionFlag +0x70 bit 7), `b60ec0`, `b60f10` | code |
| Units are seconds; no cap. Wolf's sword rows have 0.1 s for both; NPC rows have 0 for both except 50000980/50001980 (1.2 s) and 50000981/50001981 (0.5 s) for the attacker. | AtkParam_Pc, AtkParam_Npc | data |
| NpcParam types: 1258 rows 0 and 0 (the soldier 10100000 and the Owl 50600000 among them), type 1 on the Yasha Ape, Okiryu and a "hit stop test, a little" Samurai General row, type 2 on the Great Serpent rows and a "do it properly" test row; `hitStopType_Defencer` is 0 on every row. So Wolf's hits on the soldier and the Owl stop nobody, and NPC hits on Wolf stop nobody either. | NpcParam | data |
| A start holds the character's clock at zero for 0.06 s, then eases it back over the hit stop's length: each frame advances `dt * (elapsed / length)^2`, so 0.1 s costs about 0.127 s of the character's time. The frame that ends the hold advances nothing. An ease-in phase before the hold has length 0. | `b9d1b0`, `b9dad0`; defaults in `a51800` (+0x12C = 0, +0x130 = 0.06) | code; the defaults change only through a debug menu (inferred) |
| A new start replaces a hit stop already running. | `b9dad0` | code |
| A debug flag (+0x11D of the same global) forces the attacker's stop for type 0 and replaces the ease length with +0x134 (0.04); the module's own debug byte (+0x2D) 2 disables it and 1 forces it. Off in a normal game. | `b9cd90`, `b9d1b0`, `b9d3b0` | code |
| A player's hit stop can also start a pad rumble (AtkParam `HitRumbleId*`). | `b9d930` | code, not modelled |

The sim runs hit stop in the fixed-step duel (`Duel::step`, `HitStop` in `crates/sim/src/combat/knockback.rs`), so it is deterministic.
Button presses are still buffered while a character is stopped (ours).
A bullet's hit does not stop its distant shooter (ours; every NPC bullet row has 0 anyway).
Tests: `hit_stop_holds_then_eases_out`, `a_new_hit_stop_restarts_and_zero_starts_nothing` and `hit_stop_scales_by_type` in the sim crate; `no_hit_stop_against_the_soldier` and `hit_stop_holds_both_clocks_then_eases_back` (the soldier's row set to type 2 and 1) in `crates/sim/tests/combat_rules.rs`; `drawn_clip_time_holds_through_hit_stop` in `crates/game/src/play.rs` for the drawn frames.

Before 2026-10-10 every direct hit by Wolf froze both characters fully for 0.1 s, which in the Owl fight gave about nine 100 ms freezes per combo the real game does not have.

## Third-party claims

| Claim | Status |
|---|---|
| Ashina Samurai General NpcParam 10219000: HP 1918, posture 600, regeneration 60 per second (Discord) | Agreed with the params: `hp` 1918, `stamina` 600, `staminaRecoverBaseVel` 60. The 60 is the base before the HP-gated effects (the same 300600 to 300602) and the animation type, so actual regeneration is lower below 80% HP. |
| Deflect windows 6, 3, 2 and 0 frames for consecutive deflects (Discord) | Agreed with the extracted TAE data at 30 frames per second: effect 105010 spans about 0.200, 0.100 and 0.067 seconds in guard animations 203000, 203005 and 203006 and is absent from 203007 (sekiro-deflection `docs/COMBAT-RULES-RESEARCH.md`). The executable compares repel values, so the window is exactly the time 105010 is active. |
| `defStaminaAttackRate` 1, 0.5, 0.25 for consecutive deflects (Discord) | Agreed, and the engine multiplies it into the posture sent back to the attacker (`bfc490`). There is a fourth step, 105023 at 0.125. |
| Posture recovery scales with HP | Agreed in effect, but it is data-driven through `conditionHp` SpEffects rather than an engine formula. |
| Xu060113/SekiroCraft-Passthrough: `bd6710` takes an absolute remaining value with a history flag | Agreed (`bd6710`, `bd4de0` callers pass `current + delta`). |
| borgCode/SekiroTool: ChrData HP at +0x130/+0x134, posture at +0x148/+0x14C | Agreed. |

## Open questions

- The writer of record +0x188 (a second posture multiplier).
- How `guardCutCancelRate` enters the HP guard cut; it is passed to the guard cut virtual but its use was not traced.
- The player guard states behind the 0.7 / 0.9 posture factor and the `special` HP factor, and the PlayerIns +0x21EC percent.
- The repel cut source in `841c80` (a damage-module virtual for unguarded hits, a param table 0x80 value at +0x1C for guarded ones).
- The progression value at player game data +0x248 that drives the player's base recovery, and the writer of the animation recovery percent byte (action state +0x14) and the no-recovery flag (+0x88 bit 21).
- The part damage-group rates (damage module virtuals 0x30 and 0x38) and group 31.
- The guard-break reaction flag (record +0x1C5, a character virtual).
- Inclusivity of the `conditionHp` gate.

## Tools

`tools/ghidra/Callers.java` lists callers and callees, `tools/ghidra/ReadData.java` prints values at RVAs, `tools/ghidra/FindScalar.java` finds instructions using given constants and `tools/ghidra/Disasm.java` dumps a function's instructions.
All four write only under `re/`.

## Lock release on target death (2026-10-11)

The duel already cleared a dead target, and env 1118 correctly became false, but a held stick kept MoveDirection from the last locked strafe.
HKS seeds this selector on movement start; the free movement steering then turned the body while its old sideways clip kept moving it sideways.
release_lock now clears the target, sets the move selector to forward, clears walk twist and publishes the released HKS lock flag together.
Deathblow start uses the same release, lethal deathblows reject relocking during the finishing pair, and an HP death releases the target in the contact step before the camera reads it.
The next character tick follows the free movement path; act 2018 still releases only the facing angle of a living held lock, as before.
The game orbit reads player.lock_target, so it also stops following the dead enemy without a separate camera flag.
No attack timing, buffering, cancels, hit stop or other movement tuning changed.

The real-script regression holds a sideways stick across an HP death for 0.5 s, checks the HKS lock query, refuses a dead lock and locks again after reset.
Before the fix the displacement was [0.545, 1.543] m in X/Z; after it, [2.165, -0.308] m, aligned with the camera-relative right stick.
Test: dead_target_releases_lock_and_camera_relative_movement in crates/sim/tests/duel.rs.
The soldier pressure regression and its one landed hit, one block and one deflect are documented in docs/AI.md; it does not reproduce a posture break in four seconds and requires no forced guard rule.
## Guard return audit and held movement (2026-10-11)

The normal-AI, non-god soldier replay in `soldier_guard_return_matches_wolfs_real_attack_data` reproduces an ordinary block at tick 64 (Wolf AtkParam_Pc 5000011) and a deflect at tick 98 (5000050).
Both rows have `repelVictoryStamDamage_Attacker = 0` and `repelLostStamDamage_Attacker = 0`; Wolf's remaining posture stays 120 before and after those contacts.
The local executable trace of `842790` is stored in `re/exports/decomp/842943.c` (its header identifies 842790).
It reads the contacted attack row's signed fields at +0x1B0 for a block and +0x1B2 for a deflect, with a deflector weapon/reinforcement multiplier and `defStaminaAttackRate` on the deflect branch.
It does not replace these fields with the defender's `staminaAttackRate` or `guardBreakRate`; those fields serve posture attack scaling and repel comparison elsewhere.
The zero result is therefore consistent with the extracted attack data and existing executable research, and no hand-tuned posture return has been added.
The general deflect-attacker formula remains covered by the nonzero NPC slash tests (45, 22, 11, 5 for successive Wolf deflect multipliers).
A claim that these ordinary Wolf slashes must build nonzero posture when deflected still needs a matched recording from the original game or evidence of an additional engine path.
ThrowParam is not involved, and the defender's no-break-on-deflect rule is unchanged.

The earlier lock release reset MoveDirection and walk twist, but free_move_start_turn still skipped an already moving Wolf.
A held backward stick across target HP death travelled 1.683 m along the camera-relative direction and 0.600 m sideways in the next half second.
A lock release now clears the remembered travel heading and queues a free-movement start turn even when the stick stays held.
It waits through restricted reactions and deathblow throws until movement is permitted, keeping the drawn turn smooth through the existing lower-body twist.
The four stick directions at three camera yaws all align in the following half second, with sideways drift below 0.000002 m in the regression.
The raw-input replay `crates/game/scripts/held_move_kill.txt` also holds W across a real soldier deathblow; its log shows the released lock and subsequent camera-relative movement.
Attack timing, hit stop and input buffering are unchanged.

## Deflecting projectiles

A projectile owns its guard/deflect contact; its shooter supplies damage scaling but receives no character recoil, hit stop, posture feedback or attacker posture-break result.
Melee contacts retain their existing attacker handling.
Use the TAE `BulletBehavior` / BehaviorParam `refType=1` contact source: `isArrowAtk=0` on all 249 existing NPC attack rows referenced by Bullet, so it cannot identify projectiles.

**data**: Owl act 3014 uses Bullet 50601150 -> AtkParam_Npc 50600150: `guardAtkRate=30`, both katana disable-guard flags 0, `deflectedAction=justDeflectedAction=0`, `isDisableParry=1`, `atkAttribute=9`, `spAttribute=1`.
The separate new parry-state test described by `isDisableParry` does not forbid ordinary just guard.
Wolf's posture bases are `atkStam=6` on deflect, `repelLostStamDamage=12` on block and `directAtkStamDamage=12` on hit; all three `_Attacker` feedback fields are 0.
Genichiro arrow 71000500 has the same permissions and zero attacker feedback, with Wolf's bases 9 / 18 / 18.
Existing guard/effect cuts, Bullet `staminaDamp` and truncation still apply: the 6 m regression records Wolf posture damage 5 / 11 for Owl and 8 / 17 for Genichiro (deflect / block), zero guarded HP damage, and uninterrupted shooter clips.

Owl's fan 50600131 and firecracker 50600160 contain `repelLostStamDamage_Attacker=30`; feedback to the contacting projectile does not redirect to its distant owner.
His firecracker still disables katana block and deflect; poison/seal effect contacts, smoke dazzle and lightning discharge retain their existing rules.
Tomoe lightning Bullet 71100521 is a projectile, while lightning sword AtkParam 71100361 is melee; guarded grounded lightning still electrifies Wolf.

**inferred ownership**: the zero Owl/Genichiro attacker-reaction selectors support the rule, but the exact native bullet-to-owner notification dispatch has not been independently traced here.
The old resolver unconditionally sent character category recoil codes 1008/1019 to the owner even when attacker posture damage was zero.
Sources and the complete per-tool/arrow audit, including Paramdex descriptions and FwLo's MIT notes, are in [Projectile guard parameter audit](PROJECTILE-GUARD.md).
The local primary sources are [AtkParam_Npc](../cache/params/AtkParam_Npc.json), [Bullet](../cache/params/Bullet.json), and Paramdex [AtkParam definitions](../cache/refs/paramdex/Defs/AtkParam.xml).
Regression evidence: `crates/sim/tests/projectile_guard.rs`, `crates/sim/tests/lightning.rs` and `target/alt/projectile-baseline.log`.

## Super armour and interruption (2026-10-11)

Interruption is a damage-motion decision, separate from HP and posture damage.
The local v1.6 executable's `84ff70` calls `bfd980` to choose the active SpEffect with `spCategory = 1001` and the lowest `categoryPriority`, then indexes its `dmgLv_*` fields (`10cea20`).
Guard contacts and level zero bypass replacement (`bdbac0`).
The replacement values are enums, not HKS damage levels: `10b5100` and the table at RVA `2b979e8` translate enums 1 through 12 to levels `[0, 8, 1, 2, 3, 7, 4, 6, 9, 5, 10, 11]`.
Level 8 is minimum/additive damage, so a sword hit can deal HP and posture without changing the full-body clip.
The simulation applies this replacement after HP/posture resolution and before delivering the incoming damage to HKS.

| Source | Meaning and audited values |
| --- | --- |
| TAE 0, ChrActionFlag 24 | Disables damage motion; `b51b70`, `bdbb40` and `b68d00` publish level zero and suppress knockback, while HP/posture remain active. |
| TAE 66/67/401, AddSpEffect | Timed category-1001 replacement windows. 5350 makes every positive level minimum; 5358 leaves large/blow-off reactions; 5359 leaves blow-off reactions. |
| SpEffect 5325 / 5311 / 5320 | Genichiro / Owl / soldier resident damage-motion tables, priority 200. Middle sword hits remain level 2. |
| SpEffect 5375 | Boss resident, priority 150, `stateInfo = 120`: valid only while a super-armour meter is active, with a small-blow/kick exception. It is not unconditional immunity. |
| NpcParam `superArmorDurability`, `superArmorRecoverCorrection` | All three tested rows (71001000, 50600000, 10100000) have durability 0 and recovery correction 0.4. `bdb440`/`bdbaf0` require positive base plus added durability and unbroken remaining armour. |
| SpEffect `changeSuperArmorPoint`, `saReceiveDamageRate` | `stateInfo = 155` adds armour capacity (`bfe4a0`/`bdb690`); received armour damage is scaled by `saReceiveDamageRate` (`bdb470`). The audited attack-window effects and these NPC residents add no capacity. |
| NpcParam `toughness`, SpEffect `toughnessDamageCutRate` | All three rows have toughness 0. Native `be4e30` requires positive toughness, remaining toughness and action-module flag +0x70 bit 34; `be5bf0` consumes hit toughness damage with the effect multiplier. Zero base toughness supplies no extra resistance here. |
| AtkParam_Pc `atkSuperArmor`, `atkSuperArmorCorrection` | Wolf's opening slash 5000010 has 0 and 100%; charged thrust 5000000 has correction 200%. These do not create defender armour. |
| TAE 795, DS3Poise | Template fields `ToughnessParamID`, `ToughnessType`, `DamageRatio`; no such event on the sampled Genichiro 3015/3010/3044/3007/3025 clips. No DS3-labelled event is assumed to grant Sekiro armour. |

The shipped NPC HKS `c9997.hks` functions `IsDamagedAerial` and `ExecDamageDefault` supply another resistance path.
A hit is aerial when env(201) is false or SpEffect reference 1000060 is present.
Middle/large/push/fling/small-blow hits then require animation 8140 or 8141 before firing an aerial full-body flinch.
The existence query must include TAE aliases: Genichiro 8140/8141 alias 8010/8011, so missing standalone .bin files do not mean the aerial reactions are absent.
The ordinary ground branch instead chooses 8010/8011/8012-style directional flinches.
Guard/deflect and posture-break reaction codes still follow the existing HKS branches.

### Audited attack windows

Times are seconds in the extracted `a000_<act>` clip, not wall-clock time.
These are reaction rules, not a blanket immunity for every attack labelled perilous.

| Enemy / act | Resistance data |
| --- | --- |
| Genichiro 3015 diving slash | No special replacement SpEffect or 795; ordinary aerial hits select existing 8140/8141 aliases and interrupt. No native armour gate has been verified for this move. Its strike window is 1.033-1.200. |
| Genichiro 3010 | Two spinning slashes, physics marker 28 at 0.200-0.967. Flag 28 alone is not armour. |
| Genichiro 3044 jumping volley | 5900 (aerial damage reference 1000060) at 0.400-2.133; 5355 at 0.333-2.133. 5355 alone still allows middle damage. |
| Genichiro 3007 | No category-1001 timed effect in the audited clip; do not infer immunity from its act number. |
| Genichiro 3025 perilous thrust | 5358 at 0.000-0.867. |
| Genichiro other 5358 commitments | 3021: 0-0.833; 3028: 0-4.733; 3040: 0-3.300; 3041: 0-0.900; 3062: 0-1.200; 3067: 0-2.233. |
| Genichiro 3030 protected aerial strike | 5350 at 0.067-1.167; 5900 (aerial damage reference 1000060) at 0.133-1.167; attack judge 350 at 1.100-1.167. |
| Genichiro 5350 / 5359 | 3026: 0.067-2.500 and 3030: 0.067-1.167 (5350); 3092: 0-4.033 (5359). |
| Owl aerial 5350 windows | 3003: 0.300-0.967; 3006: 0.067-0.667; 3007/3030: 0-0.900; 3029: 0.433-0.900; 3035: 0-0.733; 3036: 0-0.400; 3038: 0.300-0.967; 3046: 0.167-0.567; 3047: 0.500-1.100; 3092: 0.900-1.367. |
| Owl 5358 commitments | 3008: 0-0.700; 3010: 0.600-1.000; 3013: 0-0.633; 3026: 0-1.733; 3027: 0-1.800; 3039: 0.767-1.400; 3064: 0.333-0.700; 3068: 0-0.533; 3069: 0-1.633; 3091: 0-3.000. |
| Owl 3044 / 3093 | Flag 24 at 0.900-1.467, 2.100-3.300, 3.933-4.533 / 0.767-2.033, 2.333-2.867, 3.300-3.700. |
| Owl 3040 / 3069 | 5359 at 0.500-0.800 / additional 5350 at 1.467-1.733. |
| Soldier commitments | 5358 on 3010: 0-0.867, 3031: 0-5.000, 3032: 0-3.400; 5359 on 3013: 0-0.967; flag 24 on 3011: 0.967-1.367 and 3080: 0-2.433. |

The snap to ground came from reporting only `Body.grounded` to env(201): these NPC animations lift their skeleton while their physics root stays on the floor.
The wrong grounded branch replaced the jump with DamageMiddle, discarding the authored elevation; a legitimate aerial hit now selects DamageAerialFront/Back and retains height.
NPC support now compares the lowest of both extracted foot targets with the lowest starting foot, with a 0.15 m contact tolerance.
This support reconstruction is an engine approximation, not a recovered native tolerance.
It changes the aerial damage query without forcing the normal falling graph to cancel the authored jump.
When incoming damage actually replaces that full-body clip, its support elevation transfers to the body, which falls and lands through the existing graph and collision integration.
Positive durability/toughness meters for other NPC data remain unimplemented; the audited three NPCs do not activate them.
The requested unconditional protection for 3015 is unresolved: adding it without another verified source would invent a per-move rule.
The protected aerial regression uses 3026 at 1.5 s, inside its actual 5350 window, and verifies its later strike.


### Aerial reaction and landing data

Genichiro's aerial-back reaction 8141 aliases HKX 8011; the front reaction 8140 aliases 8010.
Their TAE 920 events select velocity row 5000 at 0.000-0.033 s and row 5100 at 0.033-0.067, 0.067-0.100 and 0.100-0.133 s.
Action flag 51 at 0.000-0.133 prevents premature fall cancellation, and SpEffect 5910 carries landing-decision reference 1000070.
The scripted interrupted 3015 run selects DamageAerialBack and then LandDownward (7220).
Support-height transfer starts with zero vertical speed; retaining the authored pre-contact vertical momentum and reconstructing native animation-root physics remain approximations.

### Mikiri deathblow follow-up (2026-10-12)

AtkParam_Npc 71000250, 50600310 / 50600311 and 10100230 / 10100940 / 10100941 carry base Mikiri posture damage 90 in staminaDamageAttackHitParry.
A projected break selects ThrowParam 17100120 / 15060120 / 11010120, pairing Wolf's a227_511100 / a221_511100 / a200_511100 with the enemy's 13100 counter reaction.
The boss Wolf clips import a201_511100, whose ChrActionFlag 154 opens deathblow cancellation at 1.200 s, as does a200_511100.
Genichiro's counter reaction holds posture at zero with StaminaControl type 5 until 1.500 s, then type 6 restores 30 percent, so waiting for the defender clip to end loses the opening.
A Mikiri pair now blocks deathblows only until Wolf's flag 154 opens, allows the counter reaction to be interrupted, and releases its slide when the deathblow takes ownership.
Leaving the counter through another action also clears its defender-clip lock; ordinary deathblow get-up locks retain their existing checks.
The extracted sweep bonus remains SpEffect 400200, stateInfo 912, defAntiGroundStaminaDmgRate 8, multiplying AtkParam_Pc 901's directAtkStamDamage 10 with staminaPhysicsAttribute 7.
At AP1 a landed in-window stomp takes 80 posture; the castle-roof AP6 effect 160006 has staminaAttackRate 2.25, giving 180 before the target meter's floor clamp.
The reported reduced damage on a landed sweep counter was not reproduced on Genichiro's 3067, Owl's 3004 or the soldier's 3020; the counter regressions measure the parameter-derived damage and the actual meter loss.

# Combat rules from the executable

What `sekiro.exe` 1.6.0.0 does when an attack touches a character, recovered by static analysis of the game code in Ghidra, from the author's own copy of the game.
The implementation is `crates/sim/src/combat/`, original Rust written from these notes.
No decompiled code is reproduced here or in the source; dumps stay under `re/`.

## Confidence labels

- **code**: the arithmetic, order and rounding were read from the named function.
- **inferred**: the shape was read from code, but which param field feeds a value was inferred, usually by matching record offsets to Paramdex offsets or by matching values.
- **def**: from the Paramdex Japanese description.
- **community**: from a community source; not checked in the executable.

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

Perilous attacks are the unblockable flags, not a separate system.
In AtkParam_Npc, 238 rows block both guard types for both block and deflect (grabs), 60 rows block only the katana (guard type 0) for both (the Loaded Umbrella still blocks them), and 111 rows forbid only blocking with the katana, so they must be deflected.
The Mikiri counter is not in this path; see `docs/HKS-COMBAT.md`.

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
| The update is skipped while an action-state flag (bit 21 of +0x88) is set. | `a04850` | code, meaning inferred |
| The StaminaControl row is NpcParam `staminaControlParamId` for NPCs and row 0 for the player. | `bd58c0` | code |
| The type comes from the current animation (TAE event 960). | Smithbox annotation | community |
| No separate post-damage delay timer exists in this path; pauses come from the animation's type (type 0 is 0% in rows 0 and 1000100). | `a04850` | code for the absence in this function |
| HP lowers recovery only through `conditionHp` SpEffects, which all multiply when active. | `bfe360`, def of `conditionHp` | code for the product, def for the gate |

Player row 0: type 1 is 67%, type 5 pins posture full (min 100%), type 10 is 200%.
Soldier row 1000100: type 2 is 33%, type 4 200%, type 6 pins posture at 30%, type 7 keeps it full, type 10 200%.

Soldier numbers (base 20, resident effects 300600, 300601, 300602 gated at 80%, 60% and 40% HP with rates 0.6, 0.5 and 1/3):

| Soldier HP | Active gated effects | Recovery per second |
|---|---|---|
| 195 (100%) | none | 20 |
| 150 (77%) | 300600 | 12 |
| 110 (56%) | 300600, 300601 | 6 |
| 60 (31%) | all three | 2 |

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

# Params that drive Sekiro combat

This is a short map of which PARAM tables and fields matter for combat, by internal Paramdex field name.
Sekiro's data calls posture "stamina", so every `*Stam*` or `stamina*` field below is posture unless stated.
Use `sekiro-extract params` to write every table to `cache/params/<Table>.json`, and `sekiro_formats::param` to read them in code.

## How sure we are

Each meaning carries one of these tags.

- **[def]**: the Japanese display name or description in the Paramdex def (soulsmods/Paramdex `SDT/Defs`, commit `ff7245e5`), translated by us.
  These come from the developers' own field labels, so they are the strongest source short of the executable.
- **[community]**: Smithbox English annotations (vawser/Smithbox `Assets/PARAM/SDT/Param Annotations/English`, commit `cbd477a8`) or community wikis.
  Useful, but written by modders and sometimes wrong.
- **[data]**: a value read from the player's own files and checked by `crates/formats/tests/params.rs`.
  This proves the value and the link between rows, not how the engine uses it.
- Nothing here is **[native]** yet.
  No field's arithmetic has been traced in `sekiro.exe`, so formulas, rounding, clamps and ordering are all open.

Where the Japanese def and the English annotation disagree, the def wins until the executable says otherwise.

## The chain from an attack to its numbers

1. A TAE attack event on an animation carries a behavior judge id.
2. `BehaviorParam` (NPCs) or `BehaviorParam_PC` (the player) has a row whose `variationId` matches the character's variation and whose `behaviorJudgeId` matches the event.
   For NPCs the variation is `NpcParam.behaviorVariationId`; for the player it is `EquipParamWeapon.behaviorVariationId` [community].
3. That row's `refType` picks the target table and `refId` the row: attack (`AtkParam_Npc` or `AtkParam_Pc`), bullet, or special effect [community].
4. The attack row's `spEffectId0`..`spEffectId4` are applied to the target on contact [community].

Verified example [data]: soldier `NpcParam` 10100000 has `behaviorVariationId` 10100.
`BehaviorParam` 210100100 has `variationId` 10100, `behaviorJudgeId` 100, `refType` 0 and `refId` 10100100.
`AtkParam_Npc` 10100100 exists and holds the numbers below.

## HP damage

| Table | Field | Meaning | Source |
|---|---|---|---|
| AtkParam | `atkPhys` | NPC only: base physical damage of the attack | [def] |
| AtkParam | `atkPhysCorrection` | PC only: multiplier on the weapon's base physical attack | [def] |
| EquipParamWeapon | `attackBasePhysics` | Weapon base physical attack | [def] |
| AtkParam | `atkAttribute`, `spAttribute` | Physical and special attribute of the hit (slash, thrust, ...) | [def] |
| NpcParam | `hp` | Maximum HP | [def] [data: 195 for 10100000] |
| NpcParam | `def_phys`, `def_slash`, `def_lightHit`, `def_thrust` | Defence values | [def] |
| NpcParam | `slashDamageCutRate`, `thrustDamageCutRate`, `neutralDamageCutRate`, ... | Incoming damage multipliers by attribute | [def] |
| NpcParam | `physGuardCutRate`, `slashGuardCutRate`, ... | Damage cut while guarding | [def] |
| EquipParamWeapon | `physGuardCutRate`, `physJustGuardCutRate` | Damage cut when guarding and when deflecting with this weapon | [def] |
| SpEffectParam | `physicsAttackRate`, `physicsAttackPowerRate` | Attacker-side damage and attack-power multipliers | [def] |
| SpEffectParam | `slashDamageCutRate`, `defPlayerDmgCorrectRate_Physics`, ... | Defender-side multipliers | [def] |
| AtkParam | `guardCutCancelRate` | Ignores part of the defender's guard cut (-100 to 100) | [def] |
| AtkParam | `excessDmgKeepHp` | HP kept when damage would overshoot it | [def] |
| AtkParam | `dmgLevel`, `dmgLevel_vsPlayer` | Which hit reaction plays | [def] |

## Posture damage

The attack row carries separate posture values for each contact outcome, for both sides.
The direction words come from the Japanese descriptions, where "attacker wins" (弾き勝ち) means the attack beat the guard and "attacker loses" (弾き負け) means it was deflected.

| Field (AtkParam) | Def meaning | Smithbox says | 10100100 [data] |
|---|---|---|---|
| `directAtkStamDamage` | Posture attack on a direct (unguarded) hit | Same | 18 |
| `repelLostStamDamage` | Posture attack on the defender when the attacker **wins** the contest, so a held guard | "blocked hits" | 18 |
| `atkStam` | Posture attack on the defender when the attacker **loses** the contest, so a deflect | "Flat Posture damage" | 9 |
| `directAtkStamDamage_Attacker` | Posture the attacker itself takes on a direct hit | Same | 0 |
| `repelVictoryStamDamage_Attacker` | Posture the attacker takes when it wins the contest | "when blocked" | 0 |
| `repelLostStamDamage_Attacker` | Posture the attacker takes when it is deflected | "when deflected" | 45 |
| `staminaDamageAttackHitParry` | Posture the attacker takes when its attack is contact-parried (Mikiri counter) | Same | 90 |

Note that `repelLostStamDamage` means the attacker won, despite its name; that is the def's description, not a typo here.

Modifiers on posture damage:

| Table | Field | Meaning | Source |
|---|---|---|---|
| AtkParam | `atkStamCorrection` | PC only: multiplier on guarded posture attack | [def] |
| AtkParam | `directAtkStamCorrection` | PC only: multiplier on direct-hit posture attack | [def] |
| AtkParam | `staminaPhysicsAttribute` | Which defender posture-damage rate applies (slash, thrust, ...) | [def] |
| AtkParam | `disableStaminaAttack` | Runs the break check but does not actually reduce posture | [def] |
| EquipParamWeapon | `attackBaseStamina` | Weapon base posture attack | [community] |
| EquipParamWeapon | `staminaGuardDef`, `staminaJustGuardDef` | Posture defence when guarding and deflecting with this weapon | [def] |
| EquipParamWeapon | `staminaAttackPowerRate`, `passiveStaminaAtkRate` | Posture attack multiplier; posture dealt to NPCs when deflecting | [community] |
| NpcParam | `slashStaminaDmgRate`, `thrustStaminaDmgRate`, `ninsatuStaminaDmgRate`, ... | Incoming posture multipliers by attribute | [def] |
| NpcParam | `staminaGuardDef` | Posture attack cut while guarding [%] | [def] |
| SpEffectParam | `staminaAttackRate` | Attacker posture attack multiplier | [def] |
| SpEffectParam | `defSlashStaminaDmgRate`, ... | Defender incoming posture multipliers | [def] |
| SpEffectParam | `atkPlayerDmgCorrectRate_Stamina`, `defEnemyDmgCorrectRate_Stamina`, ... | Player-versus-enemy posture corrections | [def] |
| SpEffectParam | `guardStaminaCutRate` | Guard posture cut multiplier | [def] |
| AtkParam | `guardStaminaCutRate` | Correction to the guard posture cut set on weapon or NPC | [def] |

## Deflect (just guard) and guard

| Table | Field | Meaning | Source |
|---|---|---|---|
| SpEffectParam | `defStaminaAttackRate` | On a successful deflect, multiplies the attack's `repelLostStamDamage_Attacker`, the posture sent back to the attacker | [def] |
| SpEffectParam | `attackHitParryStaminaAttackRate` | On a Mikiri counter, multiplies `staminaDamageAttackHitParry` | [def] |
| SpEffectParam | `stateInfo` | Hard-coded behaviour selector for the effect | [community] |
| AtkParam | `guardAttribute` and `disableGuard_vsGuardAttribute0/1`, `disableJustGuard_vsGuardAttribute0/1` | Unblockable or undeflectable against a guard type; 0 is the Kusabimaru, 1 the Loaded Umbrella | [def] for the flags, [community] for which tool is which |
| AtkParam | `isAllDirGuard` | Any guard counts regardless of guard angle | [def] |
| AtkParam | `isDisableParry` | Disables the contact-parry check | [def] |
| AtkParam | `deflectAction`, `deflectedAction`, `justDeflectAction`, `justDeflectedAction` | Reaction behaviour on guard and deflect, for each side | [def] |
| AtkParam | `knockbackDist_Guard`, `knockbackDist_JustGuard` | Knockback when guarded and deflected | [def] |
| AtkParam | `guardAtkRate`, `guardBreakRate` | NPC only: repel attack and repel defence used to decide whether an attack is repelled | [def] |
| NpcParam | `guardAngle`, `guardLevel`, `defFlickPower`, `flickDamageCutRate` | Guard arc, guard reaction size, repel defence, damage cut on repel | [def] |
| NpcParam | `knockbackRate_vsPlayer_JustGuard`, ... | Knockback cut on guard and deflect | [def] |
| EquipParamWeapon | `guardAngle`, `guardLevel`, `guardBaseRepel`, `attackBaseRepel` | Weapon guard arc, reaction size and repel values | [def] |

Verified deflect effects [data]: `SpEffectParam` 105020, 105021, 105022 and 105023 have `defStaminaAttackRate` 1, 0.5, 0.25 and 0.125, all with `stateInfo` 204 and `effectEndurance` 0.
Row 105010 has `stateInfo` 158.
The earlier sekiro-deflection project found these on player guard animations 203000, 203005 and 203006 (TAE events), and the community wiki labels 105010 as the just-guard judgement effect and 10502x as successive deflects [community].
So the first deflect sends back 45 posture to the soldier for attack 10100100, the next 22.5, then 11.25, before any other modifier or rounding; the composition is not yet confirmed in the executable.

## Posture recovery

| Table | Field | Meaning | Source |
|---|---|---|---|
| NpcParam | `stamina` | Maximum posture | [def] [data: 75 for 10100000] |
| NpcParam | `staminaRecoverBaseVel` | Base recovery in points per second | [def] [data: 20] |
| NpcParam | `maxDebtStamina` | Lowest posture can go below zero ("debt") | [def] |
| NpcParam | `staminaControlParamId` | Row in `StaminaControlParam` | [def] [data: 1000100] |
| StaminaControlParam | `staminaRecoverRatio_forTypeNNN` | Recovery percent for recovery type NNN (0 to 15) | [def] |
| StaminaControlParam | `staminaMaxRatio_forTypeNNN`, `staminaMinRatio_forTypeNNN` | Upper and lower posture limits for that type | [def] |
| SpEffectParam | `staminaRecoverSpeedRate` | Multiplies recovery speed | [def] |
| SpEffectParam | `staminaRecoverChangeSpeed` | Adds a flat amount to recovery speed | [def] |
| SpEffectParam | `changeStaminaPoint`, `changeStaminaRate` | Direct posture change while the effect runs | [def] |
| SpEffectParam | `conditionHp`, `conditionHpRate` | HP threshold that gates the effect | [def] |
| SpEffectParam | `maxStaminaRate`, `maxSpIncrease` | Maximum posture multiplier and flat increase | [def] |

The recovery type is chosen per animation by a TAE event (event 960 per the Smithbox annotation [community]).
Verified [data]: `StaminaControlParam` 1000100 has type 0 at 0% and type 10 at 200%.
The soldier's resident effects in `spEffectID28`..`30` are 300600, 300601 and 300602, gated at `conditionHp` 80, 60 and 40 with `staminaRecoverSpeedRate` 0.6, 0.5 and 1/3.
That matches the known rule that posture recovers slower at lower HP, but the native composition is unverified.

## Deathblow and break

| Table | Field | Meaning | Source |
|---|---|---|---|
| NpcParam | `ninsatuNum` | Deathblows needed (boss health bars) | [def] |
| SpEffectParam | `recoveRremainNinsatsuNum` | Restores remaining deathblows | [def] |
| AtkParam | `doesBreakRepelStamDamage` | When a deflect empties posture, whether both sides go into the break reactions | [def] [data: 1 for 10100100] |
| NpcParam | `ninsatsuDamageRate`, `def_ninsatsu`, `ninsatuStaminaDmgRate` | Deathblow damage and posture multipliers | [def] |
| SpEffectParam | `ninsatsuAttackPower`, `ninsatsuAttackPowerRate`, `atkNinsatsuDmgRate`, `defNinsatsuDmgRate` | Deathblow damage modifiers | [def] |
| ThrowParam | `AtkChrId`, `DefChrId`, `atkAnimId`, `defAnimId`, `Dist`, `DiffAngMin`, `DiffAngMax`, `throwKind` | Pairs attacker and victim animations for grabs and deathblows, with range and angle checks | [def] |
| AtkParam | `throwFlag`, `throwTypeId` | Marks a hit as a grab and names its type | [def] |
| NpcParam | `superArmorDurability`, `toughness` | Poise-like values; their role in Sekiro is unclear | [def] |

What triggers a deathblow window after posture break, and how long it lasts, is not in these tables as far as we know.
It is likely in HKS script, TAE events and the executable.

## In code

- Generic access: `ParamSet::load(dir, defs_dir)?.table("AtkParam_Npc")?.row(10100100)?.int("atkStam")?`.
- Typed access for the simulation: `sekiro_formats::param::typed::CombatParams::from_set(&set)?` gives `AtkParam`, `BehaviorParam`, `SpEffectParam`, `NpcParam`, `ThrowParam`, `EquipParamWeapon` and `StaminaControlParam` tables with snake-case fields.
- Field meanings above are also on the typed struct fields as doc comments.

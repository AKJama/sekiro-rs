# Projectile guard parameter audit

**data**: TAE `BulletBehavior` -> BehaviorParam `refType=1` -> Bullet `atkId_Bullet` identifies a projectile contact, including child explosions; `isArrowAtk` is 0 on all the Owl and Genichiro bullet attack rows below.
Across the full Bullet table, all 249 referenced existing AtkParam_Npc rows have `isArrowAtk=0`; 41 have nonzero guarded-reaction selectors and 81 have nonzero attacker-deflect posture bases, so neither flag nor feedback value alone identifies the contact's owner.
Wolf still uses the ordinary guard arc, `guardAtkRate` and `disableGuard_vsGuardAttribute0/1` / `disableJustGuard_vsGuardAttribute0/1` tests.
All non-dummy rows below have `guardAtkRate=30`, `guardBreakRate=0`, `isDisableParry=1`, and `deflectedAction=justDeflectedAction=0`; the similarly named `isDisableParry` does not forbid Wolf's just guard.
The guard/just-guard flag pairs below are for katana attribute 0; attribute 1 is allowed except on smoke dazzle 50600721, which disables both.
`disableStaminaAttack=0` throughout these non-dummy rows; `doesBreakRepelStamDamage=1` on the Owl rows and 0 on Genichiro's rows, and describes attacker break handling rather than permission to deflect.

| AtkParam_Npc | Tool / Bullet rows | Wolf direct / block / deflect posture bases | Attacker deflect base | Katana disable block / deflect | `atkAttribute` / `spAttribute` | `deflectAction` / `justDeflectAction` |
|---|---|---|---|---|---|---|
| 50600150 | Owl acts 3009/3014/3025; 50600150, 50601150 | 12 / 12 / 6 | 0 | 0 / 0 | 9 / 1 | 1 / 1 |
| 50600131 | Owl five-shuriken fan; 50600131, 50601131 | 12 / 12 / 6 | 30 | 0 / 0 | 9 / 1 | 1 / 1 |
| 50600160 | Firecracker explosions 50600168, 50601168, 50600200, 50601200 | 12 / 12 / 6 | 30 | 1 / 1 | 4 / 2 | 2 / 2 |
| 50600700 | Poison carriers/drops/floor 700-702, 730-732, 740-742 (50600/50601 prefixes) | 0 / 0 / 0 | 0 | 0 / 0 | 4 / 1 | 2 / 2 |
| 50600710, 50600720 | Recovery seal 710-711; smoke carrier 720 (same prefixes) | 0 / 0 / 0 | 0 | 0 / 0 | 4 / 1 | 2 / 2 |
| 50600721 | Smoke dazzle 721 (same prefixes) | 0 / 0 / 0 | 0 | 1 / 1 | 4 / 1 | 2 / 2 |
| 71000500, 71000502, 71000533 | Castle arrows; Bullet 71000540 also uses 71000500 | 18 / 18 / 9 | 0 | 0 / 0 | 9 / 1 | 1 / 1; 2 / 2 on 502, 533 |
| 71000501, 71000503, 71000531, 71000532 | Castle arrows, corresponding Bullet ids | 6 / 6 / 3 | 0 | 0 / 0 | 9 / 1 | 1 / 1; 2 / 2 on 503, 532 |
| 71000510, 71000520 | Strong arrow / blue lightning, corresponding Bullet ids | 42 / 42 / 21 | 0 | 0 / 0 | 9 / 1; 4 / 10 on 520 | 2 / 2; 1 / 1 on 520 |
| 71000530 | Jump volley arrow, corresponding Bullet id | 12 / 12 / 6 | 0 | 0 / 0 | 9 / 1 | 2 / 2 |
| 71100500-71100503 | Tomoe arrows; Bullet 71100530-533 and 71100540 also use 71100500 | 18 / 18 / 9 | 0 | 0 / 0 | 9 / 1 | 1 / 1; 2 / 2 on 502-503 |
| 71100510 | Tomoe strong arrow, corresponding Bullet id | 60 / 60 / 30 | 0 | 0 / 0 | 9 / 1 | 2 / 2 |
| 71100521 | Tomoe act 3039 blue lightning, Bullet 71100521 | 0 / 0 / 0 | 0 | 0 / 0 | 4 / 10 | 1 / 1 |

All these rows have zero `directAtkStamDamage_Attacker` and `repelVictoryStamDamage_Attacker`.
Dummy AtkParams 0/1/8 have no posture damage, `guardAtkRate=100` and both guard types disabled; 1 has `opposeTarget=0` and supplies carriers/visual flames, including the explosive slash's powder.
Bullet 71100520 references absent AtkParam 71100520; the installed castle lightning act uses 71100521 instead.
The separate lightning sword attack 71100361 is a melee contact and retains character attacker handling; [Genichiro lightning](GENICHIRO.md#castle-lightning-2026-10-11) documents its discharge/reversal effects.

**rule / inferred ownership**: a guarded projectile is repelled itself; its distant owner receives no character recoil, hit stop, posture feedback or attacker posture-break result.
Nonzero `repelLostStamDamage_Attacker` on the fan and firecracker rows describes the contacting attacker and is not evidence for transferring damage to its owner.
The exact native bullet-to-owner notification dispatch has not been independently traced here; the extracted zero attacker reaction selectors support this rule, while the simulation previously unconditionally sent melee category codes 1008/1019 to the owner.
Melee character contacts retain their existing recoil and feedback arithmetic.
Wolf's bases remain `directAtkStamDamage` on hit, `repelLostStamDamage` on block and `atkStam` on deflect, followed by existing guard/effect multipliers and integer truncation.
Owl's shuriken Bullet `staminaDamp=100` reduces those bases beyond its range; the 6 m real-script regression records 5 posture on deflect and 11 on block, versus Genichiro's arrow 8 and 17 after the existing guard calculation and integer truncation.
Wolf's guard calculation includes the traced 1-percent cut bias, so even an undamped base of 6 truncates to 5 and 12 to 11.
Bullet `guardRangeType` selects where the successful guard is judged (**def**); it is 0 on Owl's shuriken and 1 on Genichiro's arrow/lightning rows, but the native interpretation of those enum values remains untraced here.
Bullet `sfxId_Flick` is the guard or just-guard contact effect (**def**): 650603 on Owl's shuriken, 671004 on Genichiro's ordinary arrow and -1 on Tomoe's lightning bolt.
Effects-only poison/seal contacts and lightning discharge remain on their existing paths; a grounded lightning guard still electrifies Wolf.

Sources: local extracted [AtkParam_Npc](../cache/params/AtkParam_Npc.json), [Bullet](../cache/params/Bullet.json), [BehaviorParam](../cache/params/BehaviorParam.json), and Paramdex [AtkParam definitions](../cache/refs/paramdex/Defs/AtkParam.xml) (`deflectedAction`, `justDeflectedAction`, guard flags and the six posture fields).
Projectile fields are described in Paramdex [Bullet definitions](../cache/refs/paramdex/Defs/BulletParam.xml).
**def**: Paramdex describes `deflectedAction` as behavior when the attack is guarded and `justDeflectedAction` as behavior when it is just guarded.
Its `isDisableParry` description concerns the separate new parry-state contact test; the ordinary guard/just-guard permissions are the four attribute flags above.
FwLo's MIT [enemy notes](../target/alt/fwlo/enemy.md) confirm NPC bullet lookup and additive melee bounds, and [handoff](../target/alt/fwlo/HANDOFF.md) records projectile flick effects; neither establishes owner recoil or supplies a conflicting traced posture rule.
Regression evidence: `crates/sim/tests/projectile_guard.rs`, `crates/sim/tests/lightning.rs` and `target/alt/projectile-baseline.log`.

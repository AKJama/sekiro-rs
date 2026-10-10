# Deathblows

How Sekiro's deathblow (忍殺, "ninsatsu") works in the game's data, worked out for Wolf against the Ashina soldier (c1010), and how sekiro-rs reproduces it.
Code: `crates/sim/src/deathblow.rs` (rules, Bevy-free), `crates/game/src/deathblow_demo.rs` (arena demo), tests in `crates/sim/tests/deathblow_data.rs`.

## The short version

1. The soldier's posture breaks: he plays the stagger `a000_008300` (behaviour graph state `TrunkCollapseFront`; `8301` is the back version, `8310` the large one).
2. Wolf presses attack while within a ThrowParam *start* row's range and angles: he plays a lunge (`a200_500000`, or `a200_502500` when close).
3. At the lunge's grab event (0.27 s) the matching *body* row is checked; if it holds, both play the paired clips: Wolf `a200_510000`, the soldier `a000_012000`.
4. The soldier is moved onto Wolf's dummy poly 267 (1.2 m in front of Wolf, facing him) as Wolf stands at the body clip's first frame, sliding onto it over 0.5 s; from then on both play their own root motion.
5. The blade lands at 0.90 s (Wolf's `ThrowAttackBehavior` event); for an ordinary enemy that kills.
6. A killed soldier leaves `a000_012000` at 1.67 s for the death clip `a000_012001`, which continues the same pose and ends on the ground.
7. When Wolf's clip ends (2.67 s), or earlier when its TAE lets him move and he does, the pair ends; the soldier finishes his clip either way.

## ThrowParam

A deathblow is a pair of ThrowParam rows.
The *start* row has an attacker animation only and decides whether the deathblow can begin.
The *body* row pairs attacker and defender animations and is checked when the start clip's grab event fires.
Its `throwKind` is the start row's plus 100 (40000 then 40100), which is how the two are linked in the data.

Player-versus-enemy rows sit at `11xx yzzz`: the enemy model in the middle (`01` for c1010, with the variation digit `y`: 0 standard, 1 "Varier", 2 tutorial copy) and the row kind in the last three digits.
`DefChrId` holds the enemy model number (1010) and `AtkChrId` is 0 for the player.
Rows for the standard soldier:

| Row | Kind | Attacker | Defender | Dist | DiffAng | MyToDef | Sorb dummy |
|---|---|---|---|---|---|---|---|
| 11010000 | Posture-break start, far | a200_500000 | - | 3.0 | 90..180 | 35 | - |
| 11010001 | Posture-break body, far | a200_510000 | a000_012000 | 2.0 | 0..180 | 180 | 267 |
| 11010005 | Posture-break start, near | a200_502500 | - | 1.2 | 90..180 | 35 | - |
| 11010006 | Posture-break body, near | a200_512500 | a000_014500 | 2.0 | 0..180 | 180 | 267 |
| 11010020 | Stealth start, behind | a200_500200 | - | 2.0 | 0..90 | 35 | - |
| 11010021 | Stealth body, behind | a200_510200 | a000_012200 | 2.0 | 0..180 | 180 | 249 |
| 11010110 | Posture-break start, behind | a200_501200 | - | 2.0 | 0..90 | 35 | - |
| 11010111 | Posture-break body, behind | a200_511200 | a000_013200 | 2.0 | 0..180 | 180 | 249 |
| 11010030 | Plunge (from above) | a200_510300 | a000_012300 | 20 | 0..180 | 180 | - |

The offsets reach the behaviour graphs as animation offset types: Wolf's throw states (`a000_5xxxxx` CMSGs) list candidates `a200_` to `a249_` and pick by offset type 17, the enemy graph's `ThrowDef*` CMSGs by type 15.
The duel sets type 17 to the row's `atkAnimOffset` and type 15 to `defAnimOffset` before firing the throw events (the mapping is inferred from the candidate lists).
Until 2026-10-10 type 17 was never set, so the Owl's deathblow played the soldier's 2.0 s `a200_510000` on Wolf against the Owl's 6.6 s `a000_012000`: Wolf was free (and steerable) after 2 s while the Owl was still held on him.

Animation names are `a{atkAnimOffset:03}_{atkAnimId:06}` for the attacker (offset 200, the player's throw category) and `a{defAnimOffset:03}_{defAnimId:06}` for the defender (offset 0).
Other rows cover deathblows from walls, ledges, after a fall, kicks and so on; they follow the same pattern.

Field meanings, as implemented (from the values; not all confirmed against the engine):
- `Dist`: maximum horizontal distance between the two.
- `upperYRange`, `lowerYRange`: allowed height of the defender above and below the attacker.
- `DiffAngMin`..`DiffAngMax`: the angle between the defender's *back* direction and the direction from the defender to the attacker, in degrees.
  Front deathblows use 90..180 (the attacker is in front), back and stealth ones 0..90.
- `diffAngMyToDef`: the attacker must face the defender within this many degrees.
- `isTurnAtker`: the start row turns the attacker to face the defender.
- `atkSorbDmyId`: the attacker dummy poly the defender is snapped ("absorbed") to; `defSorbDmyId` 0 means the defender's root.
- `adsrobModelPosInterpolationTime`: seconds over which the defender slides onto the dummy (0.5).
- `throwFollowingType` 1 on the body rows: the defender follows the attacker while the throw lasts.

## Alignment

c0000 carries the sorb dummies on a root-level helper node (`投げ＆新ファントムテスト`, identity transform) with no attach bone, so they are fixed in Wolf's frame:
- Dummy 267: 1.2 m in front of Wolf, forward vector pointing back at him. The front deathblow's defender stands there, facing Wolf.
- Dummy 249: 0.5 m in front, forward vector pointing away. Back and stealth deathblows put the defender there, back to Wolf.
- Dummy 293: like 267, used by the deflect-counter rows.

The absorb places one character on the other's dummy once, as the pair stands when the body clip starts (its frame 0), spread over `adsrobModelPosInterpolationTime`; after that each plays its own root motion (in the clip's start frame) and nobody is held to anybody.
This follows FwLo's RustSekiro (`docs/kb/damage.md`), which matched it against live recordings of the soldier's and the General's deathblows.
Until 2026-10-10 sekiro-rs instead held the defender on the dummy of Wolf's *live* pose every step, which made the defender follow every turn and step of Wolf's (the "Owl spins with Wolf" bug) and teleported it, through walls when Wolf stood near one.

Which side moves: with `atkSorbDmyId` set (the soldier) the defender goes onto Wolf's dummy; with `atkSorbDmyId` 0 and `defSorbDmyId` set (the Owl, dummy 230) Wolf goes onto the Owl's dummy.
The second is inferred from the fields, not traced.

Walls: every absorb step moves the character through its collision (`Ground::slide`), as root motion, knockback and the push-apart between the two do.
What a wall stops, the other character gives way instead (through its own collision), so the pair keeps its spacing without either ending up inside geometry.
That shift is ours; the engine's handling (shifting the pair, or failing the throw check) is not traced.
While the enemy still plays the throw's defender clip, the push-apart moves only Wolf, so walking into the getting-up Owl does not shove him (ours, after FwLo's "throw pairs skip the push-apart").
A knockback left over from the breaking hit is dropped when the deathblow starts; it used to resume when the pair ended and slide the defender 0.4 m in one step.

## The defender's side

The enemy behaviour graph (`c9997.hkx`) has a `ThrowDef<id>` state per defender animation and a `ThrowDefDeath<id+1>` state for the death version.
The engine drives these states; the soldier's HKS script (`c1010.lua`) only sets throwable flags.
`a000_012001` is not a replacement for `012000` from the start: its first frame equals `012000` at 1.67 s (zero bone gap), where `012000` has a one-frame `ChrActionFlag` 69 event.
So a lethal deathblow plays `012000` up to 1.67 s and branches there; a non-lethal one plays `012000` to the end, where the enemy gets up (flags from 3.0 s on).
The near version has the same pair (`014500`, `014501`); its branch point was not checked.
Wolf's script sees the same structure from the other side: `W_ThrowDefDeathStart` and `W_ThrowKill<animId>` events, with the throw animation id from `env(273)` and "opponent died from throw" from `env(274)`.

## Damage and death

Wolf's `a200_510000` fires `ThrowAttackBehavior` (BehaviorJudgeID 600) at 0.90 s.
With the Kusabimaru that resolves to BehaviorParam_PC 105000600 and AtkParam_Pc 5000600: a throw attack (`throwFlag` 2) with `atkPhysCorrection` 60000, a damage multiplier no ordinary enemy survives.
The lunge's grab event is a `CommonBehavior` with the same judge id, resolving to AtkParam_Pc 600 (`throwFlag` 1, no damage).
NpcParam `ninsatuNum` is the number of deathblow marks; the soldier's row 10100000 has 0, which sekiro-rs reads as one deathblow (bosses carry 2 or 3).
That reading is an inference from the data.

## Posture break

The soldier's posture is NpcParam `stamina` (75 for row 10100000); when it runs out the graph plays a `TrunkCollapse` state.
`a000_008300` lasts 3.0 s, pushes him back 0.5 m, and keeps SpEffect 220420 (stateInfo 352) on for its first 2.5 s, which is the likely "can be deathblown" state; its recovery flags start at 2.5 s.
The demo takes the deathblow 0.6 s into the stagger.

## What the sim implements

`sekiro_sim::deathblow`:
- `ThrowTable` and `ThrowRow`: the rows above, with `in_range` for distance, height and both angles.
- `select(table, chr, variation, situation, attacker, defender)`: near front, far front, then back-of-broken for a broken enemy; the stealth pair for an unaware one.
- `ThrowTimes::from_tae_db`: the grab window (`CommonBehavior`) and damage time (`ThrowAttackBehavior`) from Wolf's decoded TAE, and the death branch time from the enemy's, with no file reads.
  `ThrowTimes::load` reads the same from `cache/anim/<chr>/tae.json`; parsing c0000's 23 MB export took about 0.45 s, which the fight used to do at every deathblow start (a visible freeze), so the duel now uses the decoded TAE and a test checks both agree.
- `DummyFrame::from_flver`: the sorb dummy as a placement in Wolf's frame.
- `Deathblow`: the timeline (`Start`, `Body`, `Done`, or `Missed`), returning per step the clips to start, the absorb share to apply (`AbsorbStep`), and when the blow lands and kills; `release` ends the pair when Wolf leaves his body clip.
- `AbsorbSlide`: the absorb itself (the correction worked out once at the paired clips' frame 0, handed out over `adsrobModelPosInterpolationTime`), shared with enemy throws on Wolf.
- `ThrowTable::npc_row`: the row an NPC's throw start begins against the player (below).

## Bosses with more than one mark

The Great Shinobi Owl (c5060, `ninsatuNum` 2) shows how the data handles several deathblows; details and rows in `docs/OWL.md`.

- While marks remain, HP stops at 1 (`Vitality::set_hp`), so the first deathblow leaves him at 1 HP with one mark.
- His ordinary front deathblow clip `a000_012000` gets him back up: at 4.4 s its TAE adds SpEffect 300610 "HP reset" (`changeHpRate` -100) and switches to StaminaControlParam type 7, whose row pins posture at 100 %.
  `CombatRules::apply_effect_hp` applies `changeHpRate` once when a TAE SpEffect starts.
- On the last mark the duel picks the finishing pair (suffixes 180 / 181, "todome") when the enemy has one; its defender clip `a000_013700` is itself the death (the shared script's `ThrowDef13700` sets `STATE_DEATH`), so the duel lets the script see HP 0 from the blow on instead of waiting for a death clip.
  That the last mark selects this pair is inferred.
- His rows snap Wolf to one of the Owl's dummies (`defSorbDmyId` 230, 1.3 m in front of him) with `atkSorbDmyId` 0; the absorb moves Wolf there (`DummyFrame::inverse` gives the relation from Wolf's side).
- Wolf's clips are the `a221_` set (ThrowParam `atkAnimOffset` 221): the body clip `a221_510000` is 5.1 s long, turns are disabled (TAE flag 7) to 5.1 s, and its move and action cancels open at 4.0 s, so from 4.0 s the stick may walk Wolf away while the Owl finishes getting up (to 6.6 s) on his own.
  Neither set (`a200_`, `a221_`) has a kill follow-up clip (`a2xx_5xxxx1`, which FwLo found only in the `a201_` set).
- A deathblow waits until the enemy has reacted to the breaking hit (not frozen in hit stop, damage read by its script), and does not start again while the enemy still plays the last throw's defender clip.
  Without the first rule, a throw event queued during the hit stop was overridden by the script's stagger.

## Enemy throws on Wolf

The same ThrowParam machinery runs the other way for enemy grabs (the Great Shinobi Owl's 3040 and 3041; details and every value in `docs/OWL.md`, "Grabs").
Code: `crates/sim/src/grab.rs`; tests `crates/sim/tests/owl_grab.rs`.
The pattern is FwLo's (RustSekiro `docs/kb/guard-deflect.md`, the Ashina General's neck grab); the Owl's values are his own data.

- The start is not a ThrowParam start row but an ordinary TAE `AttackBehavior` window whose AtkParam row has `throwFlag` 1.
  Its capsule is detected like any hit; its contact starts the throw instead of hitting, so guarding and deflecting do not matter.
  A step's TAE 954 i-frame type 7 (on Wolf's steps for 0.27 s), an airborne Wolf (FwLo) or invulnerability let it pass, unspent.
- The row: `AtkChrId` the enemy's model number, `DefChrId` 0, id `20000000 + chr * 1000 + throwTypeId * 100` from the AtkParam's `throwTypeId` (inferred from the Owl's two rows, 25060000 and 25060100).
  Its range is checked as for deathblows (the Owl's rows allow 10 m and every angle, so the contact decides).
- The pair: the enemy's graph gets `W_ThrowAtk<atkAnimId>` (clip `a{atkAnimOffset}_{atkAnimId}`, `a000_004100`), Wolf's `W_ThrowDef<defAnimId>` (`a221_600110`; Wolf's per-enemy thrown clips live in `chr/c0000_cXXXX.anibnd`, here `c0000_c5060`).
  With `atkSorbDmyId` set (233 on the Owl: 1.0 m in front of him, facing back) Wolf is absorbed onto the enemy's dummy with `AbsorbSlide`, through collision, then both play their own root motion; no push-apart or knockback while held.
- The damage: each `ThrowAttackBehavior` (TAE 304) in the enemy's clip names a `throwFlag` 2 AtkParam row, resolved as a direct hit (HP, posture, `excessDmgKeepHp`) without a reaction, at the enemy's own clip time.
- Wolf's thrown clips carry TAE 952 `IFrames_ThrowAtkStillHurts`, now full invulnerability against everything but the throw's damage.
- A lethal throw keeps Wolf's script at 1 HP until his clip's `ChrActionFlag` 69, then fires `W_ThrowDefDeathStart<defAnimId>` (death clip `defAnimId + 1`), the same structure as the enemy's deathblow death branch; otherwise he gets up in the clip and the throw ends when he leaves it.
- `--force-act <id>` (`Duel::force_act`) makes the enemy request an act whenever it is free, to test grabs on demand.

## Gaps

- The engine's own throw search (which candidates it considers each frame, priorities between rows) is not reproduced; `select` tries a fixed order.
- The posture-break eligibility (SpEffect 220420 window) is not checked yet; the demo triggers inside it.
- Stealth, plunge, ledge and wall rows are selected by `select` only for the stealth case; their extra conditions (crouching, falling, wall dummies) are not modelled.
- The arena demo (`--arena --deathblow`) still passes "one mark left"; the duel tracks marks through the combat rules.
- The soldier's death clip ends in a ragdoll in the game (`RagdollReviveTime` from 3.67 s); here it just plays out.
- Sound, blood decals (`DecalParamID_DummyPoly`) and the deathblow camera are ignored.
- The models are not kept apart: near the railing the Owl's arm can rest on or through the top rail while both capsules stay clear of it (`cache/screens/deathblow_owlrail_*.png`).

## Tests

`crates/sim/tests/owl_grab.rs` (enemy throws on Wolf): the Owl's forced grabs catch a passive, guarding or deflecting Wolf with the data's rows, clips and damage, miss a stepping or jumping Wolf, kill into the death clip, and do not clip at the castle-top railing.

`crates/sim/tests/deathblow_lock.rs` (Owl, castle roof):
- `deathblow_ignores_player_steering`: stick and camera turning through the whole throw change nothing until Wolf's move cancel at 4.0 s, and nothing on the Owl at all.
- `deathblow_keeps_owl_attached_to_the_throw`: no jumps in the pair's relative placement.
- `deathblow_near_a_wall_does_not_clip`, `deathblow_with_wolf_backed_to_a_wall_does_not_clip`: no step crosses map geometry or jumps, both end on a floor, and with Wolf backed to the wall the pair keeps its 1.10 m spacing (the Owl gives way).

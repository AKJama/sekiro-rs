# Deathblows

How Sekiro's deathblow (忍殺, "ninsatsu") works in the game's data, worked out for Wolf against the Ashina soldier (c1010), and how sekiro-rs reproduces it.
Code: `crates/sim/src/deathblow.rs` (rules, Bevy-free), `crates/game/src/deathblow_demo.rs` (arena demo), tests in `crates/sim/tests/deathblow_data.rs`.

## The short version

1. The soldier's posture breaks: he plays the stagger `a000_008300` (behaviour graph state `TrunkCollapseFront`; `8301` is the back version, `8310` the large one).
2. Wolf presses attack while within a ThrowParam *start* row's range and angles: he plays a lunge (`a200_500000`, or `a200_502500` when close).
3. At the lunge's grab event (0.27 s) the matching *body* row is checked; if it holds, both play the paired clips: Wolf `a200_510000`, the soldier `a000_012000`.
4. The soldier is held on Wolf's dummy poly 267, 1.2 m in front of Wolf and facing him, sliding onto it over 0.5 s.
5. The blade lands at 0.90 s (Wolf's `ThrowAttackBehavior` event); for an ordinary enemy that kills.
6. A killed soldier leaves `a000_012000` at 1.67 s for the death clip `a000_012001`, which continues the same pose and ends on the ground.
7. When Wolf's clip ends (2.67 s) the soldier is released and finishes the death clip under his own root motion.

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

The defender is held on the dummy for the whole body clip rather than playing its own root motion from an aligned start.
Measured on the data, the soldier's own root motion in `012000`/`012001` drifts up to 0.9 m from Wolf's dummy over the throw (`defender_root_motion_versus_sorb_lock` prints it), so following its root motion would pull the two apart.
After Wolf's clip ends, the soldier's own root motion takes over.

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
- `ThrowTimes::load`: the grab window and damage time from Wolf's TAE, and the death branch time from the enemy's TAE (`cache/anim/<chr>/tae.json`).
- `DummyFrame::from_flver`: the sorb dummy as a placement in Wolf's frame.
- `Deathblow`: the timeline (`Start`, `Body`, `Done`, or `Missed`), returning per step the clips to start, the defender's placement, and when the blow lands and kills.

## Gaps

- The engine's own throw search (which candidates it considers each frame, priorities between rows) is not reproduced; `select` tries a fixed order.
- The posture-break eligibility (SpEffect 220420 window) is not checked yet; the demo triggers inside it.
- Stealth, plunge, ledge and wall rows are selected by `select` only for the stealth case; their extra conditions (crouching, falling, wall dummies) are not modelled.
- Deathblow marks and HP are not tracked by a combat model yet; the demo passes "one mark left".
- The soldier's death clip ends in a ragdoll in the game (`RagdollReviveTime` from 3.67 s); here it just plays out.
- Sound, blood decals (`DecalParamID_DummyPoly`) and the deathblow camera are ignored.

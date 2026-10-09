# Enemy AI

Enemies in Sekiro think with compiled Lua scripts that ship with the game.
`crates/sim/src/lua_ai/` runs those scripts unchanged and re-implements the engine side they call into.
Everything below about the engine comes from static analysis of the game code, combined with how the scripts use each call; tags say which.

- **code**: read from the executable.
- **scripts**: implied by how the shipped scripts call or use something.
- **ours**: our reconstruction where neither settles it.

## Where the scripts are

| File | Contents |
|---|---|
| `script/aicommon.luabnd` | 104 shared chunks: constants (`ai_define.lua`, `goal_list.lua`), the table-goal helpers (`table_ai_common.lua`), shared battle functions and the script parts of common goals (attack, combo, approach, turn, ...) |
| `script/aicommon.luabnd/aiCommon.luainfo` | Goal and logic id to callback name, such as 2100 to `Attack` |
| `script/aicommon.luabnd/aiCommon.luagnl` | List of global names |
| `script/mXX_XX_XX_XX.luabnd` | Per-map copies of each enemy's logic and battle goal |

The Ashina soldier with one-handed sword (NpcParam 10100000) uses NpcThinkParam 10100000 ("Ochimusha, one-handed sword"), with `logicId` and `battleGoalID` 101000.
Its files are `101000_logic.lua` and `101000_battle.lua` (goal `GOAL_Ochimusha_katate_101000_Battle`).
The copies in `m11_00_00_00`, `m11_01_00_00`, `m11_02_00_00` and `m13_00_00_00` are byte-identical.
Other think rows of the same soldier use `battleGoalID` 101010 (the "eight phases" stance variant) and so on.

## The bytecode

The chunks are standard Lua 5.0 bytecode: signature `ESC Lua`, version 0x50, little endian, 4-byte int, 8-byte size_t, 4-byte instructions with 6/8/9/9-bit fields and 8-byte double numbers (scripts).
They are not HavokScript, so the HKS VM in `crates/hks` cannot run them.
`lua_ai/lua50.rs` is an independent Lua 5.0 loader and interpreter written from the public Lua 5.0 format and instruction set.
It covers all 35 opcodes, closures and upvalues, varargs through the `arg` table and the small part of the standard library the scripts use (`math.rad`, `math.sin`, `table.getn`, `table.insert`, `print`).
There are no metatables, coroutines or string functions in the AI scripts.
`loadstring` is used once, by `GOAL_COMMON_If`, always to build a call to `OnIf_<goal id>`; it is supported for exactly that pattern.

The DSLuaDecompiler build under `re/ext` turns the chunks into readable Lua for study; the output stays in `re/ai/` and is not tracked.

## How the engine drives the scripts

Engine objects reach the scripts as two handles: the AI (`ai`, first argument everywhere) and a goal.
Their methods are registered with the scripting layer by name; for example `IsBattleState` is bound in a registration routine at RVA `85920` to the native at `5e4f40` (code).

### Goals

A goal has a kind (`GOAL_COMMON_Attack` = 2100, the soldier's battle goal 101000, ...), a lifetime in seconds (-1 for unlimited), numbered parameters and a queue of subgoals that run front first (scripts).
A goal kind is implemented in one of three ways:

| Kind | How it is found | Callbacks |
|---|---|---|
| Table goal | `RegisterTableGoal(id, name)` creates a Lua table `Goal` and calls the engine's `REGISTER_GOAL` | The script helpers `ActivateTableGoal`, `UpdateTableGoal`, `TerminateTableGoal`, `InterruptTableGoal_Common` look the table up and call `Goal.Activate`, `Goal.Update`, ... (scripts) |
| Script goal | Named in `.luainfo` | Globals `<Name>_Activate`, `<Name>_Update`, `<Name>_Terminate`, `<Name>_Interupt` (scripts) |
| Native goal | Implemented by the engine | The executable has a goal class per kind: `SprjGoalWait`, `SprjGoalStay`, `SprjGoalMoveToSomewhere` (and route variants), `SprjGoalGuard`, `SprjGoalSidewayMove`, `SprjGoalKeepDist`, `SprjGoalLeaveTarget`, `SprjGoalSpinStep`, `SprjGoalParry`, `SprjGoalApproachStep`, `SprjGoalCommonAttack`, ... (code, from the class names) |

`CommonAttack` (2200) has empty script callbacks, so its behaviour is the native class.

Our goal update (ours, shaped by the scripts):

1. On the first update the goal is activated.
2. The front subgoal is updated. When it ends it is removed; a failure fails the parent and clears its queue.
3. The goal's own update runs: the table or script `Update`, or the native goal.
4. If the last subgoal just succeeded and the update returned "continue", the goal succeeds. The script goals (`Attack_Update`, `ComboFinal_Update`, ...) always return "continue" and rely on this.
5. When the lifetime runs out, waiting-style native goals (Wait, Guard, SidewayMove) succeed and everything else fails.

### Logic and the top goal

Every AI has a top goal (kind 0).
When the top goal's queue is empty, or a script called `Replanning`, the queue is cleared and the logic runs: `ExecTableLogic(ai, logicId)`, which calls `Logic.Main` of the table registered by `RegisterTableLogic` (scripts).
`Logic.Main` uses `AddTopGoal` to queue goals; for the soldier, the shared `COMMON_EzSetup` queues the battle goal (`battleGoalID` from NpcThinkParam) once the state transition reactions are done.

The battle goal's `Activate` scores each of its acts by distance, the target's state and cool-downs, picks one with the engine's random numbers, and the act queues subgoals such as approach, then a single attack or a combo (`ComboAttackTunableSpin` 3000 then `ComboFinal` 3001).
`Goal.Update` is `Update_Default_NoSubGoal`, so the battle goal ends when its subgoals are done and the logic runs again.

### AI target state

The engine keeps a target state on the AI: 0 none, 1 caution, 2 find, 3 battle (`AI_TARGET_STATE__*`).
`IsBattleState` returns state == 3 and `IsFindState` state == 2, read from a state object at AI +0x7B30, field +0x158 (code, `6056e0`, `605e20`).
`IsChangeState` compares that field with the previous state at +0x15C (code, `6057b0`).
The engine also marks the state with SpEffects 200001 (non-combat caution), 200002 (combat caution) and 200004 (find or battle) (SpEffectParam names), which the scripts test with `HasSpecialEffectId`.
Ours: a visible target moves the state to find on the next logic run and to battle on the one after; the previous state is updated after each logic run, and 200004 counts as active from find on.
With that, the soldier plays its notice animation 401040 once and then fights.

### Interrupts

Events (target attacking, damage, guard results, ...) are offered to the logic's and goals' `Interrupt` functions while `IsInterupt(type)` reports the event (scripts).
Ours: the logic is offered first (`InterruptTableLogic_Common`), then the active goals from the deepest up, until one returns true.
Two sources are wired: the rising edge of `AiWorld::parry_timing` fires `INTERUPT_FindAttack` (1) and `INTERUPT_ParryTiming` (24), and `AiWorld::damaged` fires `INTERUPT_Damaged` (2).
The soldier's battle goal answers parry timing through `Common_Parry`: with SpEffect 221002 (parry rank C) it guards with action 3100, or deflects with 3101 as its consecutive-guard count rises.
The guard is an `EndureAttack` goal that requests action 3100, not the generic guard action 9910.

## Native goals as implemented

| Goal | Ours |
|---|---|
| Wait, Stay, WaitCancelTiming | Stand, face the target given in parameter 0, succeed at end of life |
| MoveToSomewhere and variants | Move toward parameter 0 (walk if parameter 4 is true) until within parameter 2 metres |
| Guard | Request action parameter 0 for the lifetime |
| SidewayMove and variants | Strafe around parameter 0 (parameter 1: 0 right, 1 left) at walk speed |
| KeepDist | Move to bring the distance inside parameters 1..2 |
| LeaveTarget | Back away until parameter 1 metres |
| CommonAttack, SpinStep, Parry, ApproachStep | Face the target, request action parameter 0 until the animation plays (failing after 2 s), then wait for it to end; a combo step (CommonAttack parameter 6 true) succeeds as soon as `AiWorld::combo_window` opens |
| Team commands, caution approaches, ConfirmCautionTarget | Succeed immediately |

## The AI object's methods

About 230 distinct method names appear in the scripts.
Implemented with sim answers: goal queue control (`AddTopGoal`, `AddSubGoal`, `AddSubGoal_Front`, `ClearSubGoal`, `GetTopGoal`, `HasGoal`, `Replanning`), parameters and lifetimes, distances and angles (`GetDist`, `GetDist_Point`, `IsInsideTarget`, `IsInsideTargetEx`, `IsLookToTarget`, `GetToTargetAngle`), random numbers, HP and posture rates, SpEffect checks, `IsTargetGuard`, numbers, string-indexed numbers, AI and goal timers, attack cool-down ages, NpcThinkParam fields (`GetExcelParam`), the state queries above, `TurnTo` and `DoEzAction`.
Navigation probes (`GetExistMeshOnLineDistSpecifyAngleEx`, `CheckDoesExistPath`) answer "free floor" (ours; the arenas are flat).
Any other method is logged once as `AiEvent::Unknown` and answers false for `Is*`, `Has*` and `Check*`, 0 for `Get*`, and nothing otherwise.
In a 20-second soldier fight only `IsInsideTargetRegion` falls through to that default.

Cone tests use half the given angle on each side of the direction (ours; the scripts pass 90, 120, 180).

## Using it

```rust
let think = ThinkParams::from_row(&params.table("NpcThinkParam")?.row(10100000)?)?;
let mut brain = AiBrain::load(cache, "m11_00_00_00.luabnd.d", think, seed)?;
// each step:
let control = brain.think(AiWorld { dt, me, target: Some(player), hit_radius, current_anim,
    combo_window, home, parry_timing, damaged });
npc.npc = control; // as the stand-in brain does
for event in brain.drain_log() { /* goal starts and ends, actions, interrupts */ }
```

`AiWorld` needs, per step:

- position, yaw, HP, posture and active SpEffects of the soldier and of the player, and whether the player guards;
- `current_anim`: the playing full-body clip's id (`anim_id_from_clip("a000_003000")` gives 3000);
- `combo_window`: the clip has reached the TAE window where the next attack is accepted;
- `parry_timing`: the player's attack is about to land on the soldier (a TAE attack window with the soldier in range), and `damaged`.

The returned `NpcControl` carries the requested action (attack codes 3000 and up, guard 3100, deflect 3101, notice 401040), the move level and direction, the facing, and the battle behaviour reference 1000003.

## Not done yet

- Interrupts other than parry timing, FindAttack and damage; observed SpEffect activation interrupts (`AddObserveSpecialEffectAttribute`) are not raised.
- Kengeki (sword clash) acts, platoon and team behaviour, navigation, sound targets, non-battle behaviour, home return.
- The engine's exact goal update order, lifetime rules and interrupt order are ours; checking them against the native goal classes in the executable is the next step.
- The scripts use the engine's random numbers; ours are a seeded xorshift, so act choices differ from the game run for run.

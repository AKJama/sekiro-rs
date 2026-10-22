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

`goal:TimingSetNumber(slot, value, timing)` and `goal:TimingSetTimer(...)` attach a deferred `SetNumber` / `SetTimer` on the AI to a goal; `timing` is `AI_TIMING_SET__ACTIVATE` (0) or `AI_TIMING_SET__UPDATE_SUCCESS` (1) (`ai_define.lua`).
The value is the one the script computed when it attached it (scripts); ours applies it just before the goal's `Activate`, or when the goal ends with success.
The Owl's acts use it to keep a pressure score in AI number 5 (`docs/OWL.md`).

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
For map foes a held sound target with no seen target is caution (issue #218, docs/FIDELITY.md).
The alert anims add these marks in their TAE (c1010's 1010 adds 200001), and the last one stays on the AI until another replaces it (`AiState::ai_mark`).
With that, the soldier plays its notice animation 401040 once and then fights.
An enemy whose `EnemySpec::starts_in_battle` is set (the Owl, after his opening cutscene) starts in the battle state (ours, `AiBrain::start_in_battle`).

### Interrupts

Events (target attacking, damage, guard results, ...) are offered to the logic's and goals' `Interrupt` functions while `IsInterupt(type)` reports the event (scripts).
Ours: the logic is offered first (`InterruptTableLogic_Common`), then the active goals from the deepest up, until one returns true.
Three sources are wired: the rising edge of `AiWorld::parry_timing` fires `INTERUPT_FindAttack` (1) and `INTERUPT_ParryTiming` (24), `AiWorld::damaged` fires `INTERUPT_Damaged` (2), and a new sound target fires `INTERUPT_ChangeSoundTarget` (46).
Parry timing is the game's own signal: the player's attack animations carry TAE action flag 63 (for Wolf's first slash `a050_300000` at 0.6 s, the frames before the blade arrives), and the duel raises `parry_timing` while that flag is active on Wolf's current clip (traced by FwLo's RustSekiro, `docs/kb/guard-deflect.md`).
The AI reads the world before Wolf steps, so it hears the flag one step after it opens.
There is no range or facing check on the engine side: the script makes them itself.
The soldier's battle goal answers parry timing through `Common_Parry(ai, goal, 50, 25, 0, 3102)` (`common_func_NTC.lua`): it needs the player in its front 90 degree cone, itself in the player's front 90 degree cone and within `GetDist_Parry` (the player's weapon reach constant, `PC_ATTACK_DIST_*` by weapon SpEffect) to guard or deflect; one metre further out it may back-step instead.
With SpEffect 221002 (parry rank C) it guards with action 3100, or deflects with 3101 as its consecutive-guard count rises.
`FindAttack` firing together with `ParryTiming` is ours; its own source in the engine is not traced.

The scripts also ask to be told about SpEffects: `AddObserveSpecialEffectAttribute(target, id)`.
Ours: on the step an observed SpEffect turns on, `INTERUPT_ActivateSpecialEffect` (43) is dispatched with `GetSpecialEffectActivateInterruptType` answering its id; on the step it turns off, `INTERUPT_InactivateSpecialEffect` (44) with `GetSpecialEffectInactivateInterruptType`.
An effect already active when it is first observed raises nothing.
This is what drives the consecutive-guard count of `Common_Parry` and the Owl's `Parry`: the weak guard reactions `a000_008500`/`8501` add SpEffect 200250, and the shared table-logic interrupt (`table_ai_common.lua`) counts it.
The guard is an `EndureAttack` goal that requests action 3100, not the generic guard action 9910.

Watched areas: `AddObserveArea(slot, observer, target, dir, angle, dist)` keeps an area around the AI (the observer is always `TARGET_SELF` in the scripts); `IsInsideObserve(slot)` answers whether the target is in its cone and distance, `DeleteObserve(slot)` drops it.
Ours: `INTERUPT_Inside_ObserveArea` (16) is dispatched on every step a target is inside a watched area, not only on entering; the Owl's heat-up handler checks a timer when it fires, so a Wolf already standing next to him must still raise it once the timer allows (`docs/OWL.md`, "Phase 2").

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
| ConfirmCautionTarget | CommonAttack of parameter 0 (when set), then Wait for parameter 2 seconds; on Terminate in caution with no new sound, the sound target is cleared (FwLo's trace) |
| Team commands, caution approaches | Succeed immediately |

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
- `parry_timing`: the player's current clip has TAE action flag 63 active, and `damaged`.

The returned `NpcControl` carries the requested action (attack codes 3000 and up, guard 3100, deflect 3101, notice 401040), the move level and direction, the facing, whether the request comes from a combo goal, and the battle behaviour reference 1000003.

### How a request reaches the NPC script

The shared NPC script (`c9997.hks`) reads the request as `env(106)` when it is free (idle, moving).
Inside an attack or step it calls `ExecAIAction` with its cancel flag set, which reads a different env per kind of cancel instead: 107 combo action, 120 attack-cancel action, 118 step action, plus 123, 124 and 125 (endure, anti-backstab, immediate reaction; env names from the community env list in `re/ext/meow_env.txt`).
Env 123 is the request of an `EndureAttack` goal, whose `Activate` calls `SetEnableEndureCancel_forGoal` (`attack_endure.lua`); `ExecAIAction` takes it before every other cancel, and no TAE flag gates it in the template, so ours offers it in any state that asks (set by the duel as an env override; the Owl's heat-up escapes 20006 to 20008, guards and deflects are endure requests).
Env 125 now carries the action while the installed `AttackImmediateAction` wrapper has enabled it with `SetEnableImmediateAction_forGoal`, until its termination callback clears it.
The lifecycle and HKS request connection are reconstructed from the installed Lua and shared HKS dispatcher, not a traced native cancellation handler.
The Butterfly regression exercises interruption into every illusion's release animation and its damaging Bullet children.
Env 124 remains unhosted.
The NPC TAEs mark where an animation may be cut for a queued request with action flags 23 ("end if AI combo attack queued"), 86 (attack), 79 (step) and 78 (move).
Ours (`Character::fill_env`): env 107 is the request while flag 23 is on and the request comes from a goal registered with `ENABLE_COMBO_ATK_CANCEL` (`ComboRepeat`, `ComboFinal`); env 120 is any non-step request while flag 86 is on; env 118 a step (`5xxx`) while flag 79 is on.
The pairing of flags and requests follows FwLo's RustSekiro notes (`docs/kb/enemy.md`).
Before this, every NPC combo waited for the previous animation to end and stood in idle for a step between attacks.

### Choosing acts from outside (the act hook)

`AiBrain::enable_act_hook` lets an outside controller (the boss brain, `docs/BRAIN.md`) choose a battle goal's acts.
It keeps the scripts' shared `Common_Battle_Activate` under another global name and puts a host function in its place; that function queues the outside `ActChoice` (an `Approach_Act_Flex` approach, then the attack goals) on the battle goal, unless the script already queued a reaction or weighted one of the configured reflex acts, and otherwise calls the script's own function unchanged (ours).
Interrupts, the deflect counters and everything before the act pick still run in the scripts.
External attack approaches are deferred until that step activates, so `Approach_Act_Flex` sees the distance after earlier tactics.
The private plan wrappers use negative ids outside the game's goal-id space.
`deflect` intercepts only FindAttack/ParryTiming while ready and offers the battle goal's real interrupt callback a local queue, preserving the attack chain after a parry response.
It does not alter probabilities, ranks, parry intervals or combat resolution.
`dodge_back` reads the target's visible attack state (`AiWorld::target_attacking`, including wind-up), requests the table's back-step action within 2.5 m, and leaves other reactions live.
Both readiness goals succeed at their 1.5 s cap.

## Not done yet

- Interrupts other than parry timing, FindAttack, damage, observed SpEffects and watched areas (shoot impact, guard results, item use, observe regions and dummy areas).
- Env 124 (anti-backstab cancels).
- `GetEventRequest` answers -1 (the map events' requests to the AI are not wired).
- Kengeki (sword clash) acts, platoon and team behaviour, navigation, indication and memory targets, non-battle behaviour.
- The engine's exact goal update order, lifetime rules and interrupt order are ours; checking them against the native goal classes in the executable is the next step.
- The scripts use the engine's random numbers; ours are a seeded xorshift, so act choices differ from the game run for run.

## Soldier pressure playtest (2026-10-11)

The requested normal-Lua replay at 1.6 m presses attack every eight fixed frames for four seconds, holding forward to stay in range.
It already passes on the current runtime before any gameplay edit: direct hit at tick 26, block at tick 64, deflect at tick 98, and no posture break.
The first hit lands during notice action 401040.
At ticks 55 and 90, parry-timing interrupts queue EndureAttack 2260 and CommonAttack 2200 with actions 3100 and 3101.
The observed guard effect raises interrupt 43, updates the shared consecutive-guard count, and the soldier then requests counter action 3008 at tick 101.
No unanswered AI calls occur in this replay; the trace is logs/soldier-pressure-before.log.

Evidence is in the local scripts: 101000_battle.lua Goal.Interrupt calls Common_Parry for INTERUPT_ParryTiming, while its observed effects drive replanning and Kengeki counters.
It has no dedicated INTERUPT_Damaged or INTERUPT_GuardBreak branch that unconditionally guards after a fixed number of direct hits.
Common_Parry checks battle effect 200004, parry rank 221002, a 0.1 s interval, facing cones and weapon reach before requesting guard.
GOAL_COMMON_Guard 2101 with action 9910 also appears in Act23, but the mid-string reflex uses EndureAttack with action 3100 instead.
The NPC request reaches env 123 without an extra TAE cancel gate, as described above.
IncomingDamage reports NpcParam guardLevel 4 on both guarded contacts; env 258 is already wired by the earlier Owl stun-lock fix.
The guard reactions activate SpEffect 200250, so the consecutive-guard feedback also works.
The soldier's guard TAE recovery scaling and NpcParam guard rates are already covered by combat_rules tests, and both hit-stop scales are zero for this row.
No guard probability, reaction timing, hit stop or player input logic was changed.

Regression: soldier_guards_mashed_slashes in crates/sim/tests/combo_on_hit.rs.
This does not reproduce the reported stun lock and does not prove every approach angle or playtest configuration works.
A matching replay from the failing setup is still needed to locate any remaining defect; the extracted data does not support saying the soldier never guards mid-string.

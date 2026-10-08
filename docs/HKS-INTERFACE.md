# HKS engine interface

How Sekiro's character scripts talk to the engine, measured from the installed v1.6.0.0 scripts.
Everything here is original description of facts (ids, counts, shapes); no script text is reproduced.
Regenerate the raw numbers with `cargo run -p sekiro-hks --bin hks-inventory` (writes to stdout; keep the output under `re/`).

## The format in brief

`action/script/*.hks` is HavokScript bytecode, a Lua 5.1 derivative compiled by Havok's compiler.
The files are big-endian, use 32-bit floats as the only number type and carry full debug info (function names, line numbers, local names).
The reader is `crates/formats/src/hks.rs`; `crates/formats/tests/hks_disasm.rs` parses and disassembles all 81 installed scripts (20,516 functions, 493,743 instructions) and checks every `DATA` word sits where the encoding says it must.

Layout notes that are easy to get wrong:

- The header is followed by a type table (`TNIL` ... `TSTRUCT`, 13 entries) before the first prototype.
- A prototype header is `upvalues, params, vararg byte, max stack, unknown u32, code count`, then the code is aligned to a 4-byte file offset.
- Constants are tagged nil (0), boolean (1), number (3, f32) or string (4, 64-bit length including the terminator).
- After the constants comes a debug flag (always 1 in the shipped files), then line-info count, local count, upvalue count, line range, source, name, the line table, locals and upvalue names, and finally the child prototypes.
- An instruction is `op:7 | B:8 | C:9 | A:8` from the high bit; `Bx` is the 17 bits of B and C and `sBx = Bx - 65535`.
- Bit 8 of C marks a constant (Lua's RK encoding); B is always a register except in the `_BK` opcodes, where it is a constant index.
- `DATA` words are operands, not instructions: one follows each `GETGLOBAL_MEM` (an inline cache slot), two follow each `GETTABLE_S` and `GETFIELD_R1`, and one per upvalue follows each `CLOSURE` (A=1 binds a parent register, Bx is the register).

## Opcodes

All 92 HKS opcode numbers are known (`Op` in `hks.rs`).
Only 45 occur in any shipped script, and 42 in the player set (`c0000*.hks`).
None of the HKS-only structure opcodes (`NEWSTRUCT`, `SETSLOT*`, `GETSLOT*`, `CHECKTYPE*`, `INTRINSIC_*`) is used, and neither are `CLOSE`, `TESTSET`, `NOT`, `CALL`, `CALL_C`, `CALL_M` or plain `GETGLOBAL`.

| Opcode | Player set | All scripts | Lua 5.1 equivalent |
|---|---:|---:|---|
| GETGLOBAL_MEM | 18,371 | 112,168 | GETGLOBAL plus a one-word cache slot |
| DATA | 19,033 | 112,896 | none; operand word, executed as a no-op |
| LOADK | 11,954 | 53,386 | LOADK |
| RETURN | 7,223 | 47,809 | RETURN |
| SETGLOBAL | 6,545 | 23,523 | SETGLOBAL |
| JMP | 5,975 | 31,632 | JMP |
| CALL_I | 5,702 | 54,341 | CALL |
| CLOSURE | 4,248 | 20,435 | CLOSURE, upvalue bindings as DATA instead of MOVE/GETUPVAL |
| EQ | 4,030 | 28,936 | EQ with B a register |
| NEWTABLE | 1,617 | 1,735 | NEWTABLE |
| SETTABLE_S | 1,608 | 1,610 | SETTABLE with B a register |
| SETLIST | 1,596 | 1,634 | SETLIST, 50 fields per flush |
| MOVE | 414 | 772 | MOVE |
| GETTABLE_S | 285 | 299 | GETTABLE plus two cache words |
| CALL_I_R1 | 261 | 1,211 | CALL |
| LT | 163 | 196 | LT |
| LE | 128 | 199 | LE |
| DIV | 78 | 88 | DIV |
| LT_BK | 69 | 100 | LT with a constant on the left |
| GETUPVAL | 60 | 60 | GETUPVAL |
| SUB | 43 | 52 | SUB |
| GETFIELD_R1 | 40 | 59 | GETTABLE with a constant key, plus two cache words |
| ADD | 27 | 179 | ADD |
| LE_BK | 15 | 22 | LE with a constant on the left |
| LOADNIL | 14 | 18 | LOADNIL |
| CONCAT | 14 | 67 | CONCAT |
| MUL | 8 | 14 | MUL |
| MUL_BK | 7 | 12 | MUL with a constant on the left |
| TFORLOOP | 7 | 7 | TFORLOOP |
| FORPREP, FORLOOP | 6 each | 11 each | FORPREP, FORLOOP |
| SETTABLE_S_BK | 5 | 16 | SETTABLE with a constant key |
| SELF | 4 | 5 | SELF |
| UNM, LEN | 4 each | 4, 9 | UNM, LEN |
| TEST | 3 | 34 | TEST |
| POW_BK | 3 | 3 | POW with a constant base |
| MOD | 2 | 4 | MOD |
| VARARG | 2 | 2 | VARARG |
| SETFIELD | 1 | 76 | SETTABLE with a constant key |
| TAILCALL_I | 1 | 2 | TAILCALL |
| TEST_R1 | 1 | 1 | TEST |
| ADD_BK, SUB_BK | 0 | 4, 1 | ADD, SUB with a constant on the left |
| LOADBOOL | 0 | 100 | LOADBOOL |

Every used opcode maps one to one onto Lua 5.1 semantics; the differences are encoding only (operand layout, constant-left variants, cache words, closure binding words).
The `_R1` variants behave like their base opcode for an interpreter.

## Entry points the engine calls

| Function | Count | When |
|---|---:|---|
| `Initialize` | 1 | Once after the four player chunks are loaded |
| `Update` | 1 | Every frame |
| `<State>_onUpdate` | 1,334 | Every frame for each active behaviour-graph state that has a script hook |
| `<State>_onActivate`, `<State>_onDeactivate` | 1,342, 1,341 | When that state is entered or left |
| `ThrowScript_onPreUpdate` | 1 | Throw script node pre-update |
| `ModifiersLayer_onGenerate`, `Master_onGenerate`, `Move_onGenerate`, `SwimMove_onGenerate` | 4 | Generator callbacks (foot IK and movement blending) |

Most `_onUpdate` hooks (1,287 of 1,334) just call the shared `UpdateState(state id)`, which runs `Control` (per-state upkeep), `Validate` (pick the next behaviour) and `FireStateEndEvent` (leave a state whose animation ended).
`InitState` is called from the update hook of the behaviour graph's initial state, on map entry.
Each chunk ends by giving `_G` a metatable whose `__index` returns a no-op function, so any undefined global read yields a callable no-op rather than nil.
The scripts rely on this: 27 globals read by the player set are never defined (22 are stale `HKB_STATE_*` names), and comparisons against them are simply false.

## Engine functions

These are every global the 81 scripts read that is neither defined by a script nor part of Lua's standard library.

| Function | Player call sites | Shape | Meaning |
|---|---:|---|---|
| `env(id, ...)` | 1,605 | id, then 0 or 1 argument; returns a number | Condition query, see below |
| `act(id, ...)` | 683 | id, then 0 to 2 arguments | Command, see below |
| `hkbGetVariable(name)` | 195 | literal behaviour variable name (38 distinct) | Read a Havok behaviour variable |
| `hkbFireEvent(name)` | 2 | event name | Send an event to the behaviour graph; reached through the script wrappers `FireEvent` (891 sites, also issues `act(9101)`) and `FireEventNoReset` (203 sites) |
| `hkbIsNodeActive(name)` | 1 | node name | Whether a behaviour node is active; wrapper `IsNodeActive` (4 sites) |
| `hkbSetVariable(name, value)` | 0 (5 in `modifier.hks`) | name, value | Write a behaviour variable; the player instead uses `act(148, name, value)` via the `SetVariable` wrapper (564 sites) |
| `hkbGetBoneModelSpace`, `hkbGetOldBoneModelSpace`, `hkbSetBoneModelSpace`, `hkbIsBoneValid` | 1, 2, 1, 1 | bone name, transform objects with methods | Foot IK in the generator callbacks |

Standard library use is tiny: `math.abs` (50 sites), `math.random` (3), `table.insert` (4), `ipairs` (6), `pairs` (1, in dead code), `setmetatable` (4) and `collectgarbage` (1).

`env` and `act` take either a number or, in two places, a Japanese command name string that the engine resolves through its own name table.
Those two resolve to env 3064 and act 3041.

## env ids used by the player scripts

Names are English renderings of the engine's own Japanese command names, as listed in Meowmaritus's Sekiro dumps (see Sources).
Count is call sites in `c0000*.hks`; arguments are the extra arguments after the id.
Every id below has a name in the engine table, so none is fully unknown; "meaning" beyond the name, especially return encodings, still needs the Ghidra pass.

| id | Sites | Args | Name (translated) | Notes |
|---:|---:|---|---|---|
| 105 | 1 | 1 | Get command id from event | |
| 113 | 2 | 0 | Item-use menu opening | |
| 115 | 2 | 0 | Item-use menu open | |
| 200 | 1 | 0 | Is falling | |
| 201 | 4 | 0 | Is landed | |
| 202 | 21 | 0 | Received damage type | Compared against the `DAMAGE_TYPE_*` set; drives every damage and guard reaction |
| 205 | 11 | 0 | Normal damage passes during throw | |
| 206 | 4 | 0 | Is in a throw | |
| 207 | 18 | 0 | Weapon switch state | Arm style; 0 means weapon sheathed (non-combat) |
| 222 | 4 | 0 | Received damage direction | |
| 224 | 4 | 0 | Fall height | |
| 225 | 29 | hand | Equipped weapon category | Main weapon must be category 50 for katana actions |
| 231 | 4 | 0 | Item animation type | |
| 233 | 2 | 0 | Can use item | |
| 236 | 21 | 0 | Damage level | `DAMAGE_LEVEL_*`: small, middle, large, ex-large, push, fling, minimum, ... |
| 237 | 2 | 0 | Guard level action | Greater than zero gates the full guard-damage reaction |
| 248 | 10 | 0 | Really landed | |
| 256 | 4 | 0 | Took any damage | |
| 273 | 35 | 0 | Throw animation id | |
| 274 | 1 | 0 | Opponent died from throw | |
| 276 | 1 | 0 | Self died from throw | |
| 277 | 1 | 0 | Throw escape succeeded | |
| 285 | 25 | 0 | Special attribute of the hit | Element: fire, lightning, blue lightning |
| 333 | 3 | 0 | Delta time | |
| 334 | 14 | identification value | Behaviour identification value present | Special guard reactions 1 to 4, blinding, blast, storm back jump |
| 337 | 18 | 0 | Throw alignment in progress | |
| 339 | 6 | layer | Animation ended | |
| 345 | 29 | hand | Weapon special category | Combat art type |
| 349 | 2 | 0 | Damage motion disabled | |
| 1000 | 3 | 0 | HP | |
| 1007 | 4 | 0 | Is COM player | |
| 1105 | 28 | 0 | Is in a standby state | Lets an action end early |
| 1106 | 142 | action arm (18 distinct) | Action request | The input query: true on the frame an action button is requested |
| 1108 | 54 | action arm (9 distinct) | Action duration | How long the action button has been held; greater than zero means held |
| 1112 | 11 | 0 | Throw requested | |
| 1116 | 8 | SpEffect id | Has SpEffect | |
| 1118 | 31 | 0 | Is locked on | |
| 1119 | 5 | 0 | Attack direction | |
| 1121 | 4 | 0 | Knockback distance | |
| 2000 | 48 | 0 | Can cancel into movement | |
| 2004 | 2 | 0 | Swing is hitting | |
| 3000 | 8 | 0 | Grapple can be fired | |
| 3003 | 6 | 0 | Vertical speed | |
| 3008 | 21 | 0 | Debug dash moving | |
| 3011 | 5 | 0 | On a docking target end point | |
| 3017 | 25 | 0 | Docking edge variation id | |
| 3018 | 3 | 0 | Guard-repelled behaviour | How the player's own blocked attack bounces (normal block by the target) |
| 3019 | 3 | 0 | Just-guard-repelled behaviour | Same, when the target perfectly deflected |
| 3020 | 1 | 0 | Touching a wall-jump wall | |
| 3025 | 5 | 0 | Touching a swim volume | |
| 3027 | 1 | hand | Next slot weapon category | |
| 3028 | 7 | 0 | Fully dead | |
| 3029 | 1 | 0 | Docked edge invalid | |
| 3031 | 3 | 0 | Throw event frame | |
| 3032 | 10 | 0 | Damage direction front or back | |
| 3033 | 29 | unlock type (13 distinct) | Action unlocked | Skill and progression gates; main weapon unlock is type 5 |
| 3035 | 15 | action arm | Action can be performed | |
| 3036 | 689 | behaviour reference id (194 distinct) | SpEffect with behaviour reference id active | The main state query: TAE windows and passive effects are SpEffects with a behaviour reference id |
| 3037 | 3 | 0 | Can leave crouch collision | |
| 3038, 3039 | 2, 5 | 0 | Can dive, can surface | |
| 3040 | 2 | 0 | Map visibility type | |
| 3043, 3044, 3045 | 2, 2, 4 | 0 | Can start wall hug, ground hang, air hang | |
| 3046 | 2 | edge type | Candidate docking edge variation id | |
| 3048, 3049, 3059, 3060 | 2 each | 0 | Hang corner moves (outer left, outer right, inner left, inner right) possible | |
| 3050 | 3 | 0 | Can climb up from hang | |
| 3051, 3052 | 4 each | 0 | Obstacle at docking start or end | |
| 3053, 3054 | 2, 7 | 0 | Talk parameter and talk EzState behaviour reference ids | |
| 3055 | 1 | 0 | Saved state on load | Initial pose |
| 3056 | 2 | 0 | Guard-repel behaviour | Deflect direction when the player blocks |
| 3057 | 2 | 0 | Just-guard-repel behaviour | Deflect direction when the player perfectly deflects |
| 3058 | 2 | 0 | Lip-sync request | |
| 3061 | 1 | 0 | Safe-position return state | |
| 3063 | 76 | index 0 to 2 | Variable change value | |
| 3064 (by name) | 20 | behaviour reference id | SpEffect with behaviour reference id active, counting lifetime extension strictly | Only used for the aging (Dragonrot / old age) reference |

## act ids used by the player scripts

| id | Sites | Args | Name (translated) | Notes |
|---:|---:|---|---|---|
| 101 | 15 | flag | Movement switch | Called every frame from `Update` |
| 123 | 2 | 0 | Open item-use menu | |
| 127 | 35 | 0 | Close item-use menu | Issued when an action starts |
| 135 | 33 | 0 | Force-stop throw animation | |
| 136 | 6 | state | Set throw state | |
| 138 | 10 | 0 | Set event action allowed | |
| 139 | 1 | 0 | Request throw animation stop | |
| 141 | 29 | damage flag (10 distinct) | Set damage animation type | Guard small, large, ex-large, guard break, fling, ... |
| 147 | 11 | 0 | Allow equipment change from menu | |
| 148 | 1 | name, value | Set behaviour variable | The body of the `SetVariable` wrapper (564 sites) |
| 150 | 3 | 0 | Set item animation in progress | |
| 154 | 150 | hand | Weapon parameter source hand | Called before every weapon action and guard reaction |
| 155 | 5 | interrupt type | Notify AI of attack type | |
| 156, 157 | 1 each | 0 | Set, clear auto-aim target | |
| 159 | 22 | angle | Turn to face attacker | Front (0) or back (180) |
| 160, 161 | 17, 16 | throwable state | Set throwable state, attacker side and defender side | |
| 2002 | 17 | SpEffect id | Apply SpEffect | |
| 2015 | 3 | angle, angle | Lock-on turn-forbidden angles | |
| 2018 | 13 | 0 | Release lock-on angle fix | |
| 2019 | 3 | 0 | Turn to lock target immediately | |
| 2024 | 4 | 0 | Decide item to use | |
| 3004 | 1 | SpEffect id | Apply SpEffect to lock target | |
| 3011, 3037 | 1 each | 0 | Underwater, on water | |
| 3016 | 2 | flag | Fall-prevention assist | |
| 3018 | 5 | flag | Set wire action in progress | |
| 3019 | 7 | flag | Set lock homing action in progress | |
| 3023 | 32 | type, animation id | Set animation for pre-computed movement | |
| 3025 | 30 | angle | Face given direction | |
| 3026, 3027, 3028 | 1 each | 0 | Run wall-hug, ground-hang, air-hang start tests | |
| 3029 | 7 | talk state | Set talk animation state | |
| 3030 | 8 | guide, action | Show action guide | |
| 3032 | 6 | 0 | Disable suction | |
| 3034 | 22 | exec type | Action button executable | |
| 3035 | 2 | 0 | Prefer grapple-able edges | |
| 3036 | 1 | 0 | Disable floor snap extension at high speed | |
| 3041 (by name) | 1 | 0 | Reset warp-type input acceptance | |
| 9000 | 1 | text | Debug log output | |
| 9100 | 8 | 0 | Idle | |
| 9101 | 140 | 0 | Reset input acceptance | Every `FireEvent` issues this |
| 9102 | 3 | 0 | Set event animation in progress | |
| 9103 | 2 | flag | AI attack state | |
| computed | 2 | varies | | The id is computed at run time in `Control` |

## Open questions for the Ghidra pass

- Return encodings of the query ids above, especially 202 (damage type), 236 (damage level), 237 (guard level action), 3018, 3019, 3056 and 3057 (deflect behaviours), 1106 and 1108 (input buffering and hold time units).
- How the engine produces damage type 1028 (named for the attacker's stamina reaching zero while guarded) and 1027 (damage break), which separate posture break from normal hits.
- Which behaviour reference ids each TAE event's SpEffect carries; the scripts only see the reference id through env 3036.
- The exact engine call order per frame (`Update` before or after state hooks, and whether `_onActivate` runs before the first `_onUpdate`).

## Sources

- katalash, [DSLuaDecompiler](https://github.com/katalash/DSLuaDecompiler) (MIT), revision c27340ab, and horkrux's HKS format notes it credits: file layout and opcode numbering.
  Built locally (retargeted to .NET 8) to decompile the installed scripts into `re/hks/`; nothing from it is in tracked code.
- Meowmaritus, Sekiro env and act command name dumps ([env](https://gist.github.com/Meowmaritus/5a9b4bd0ab4e3bd8b929f6bd956f11bc), [act](https://gist.github.com/Meowmaritus/ee346c03a95dcd061d121c2689960102)): id to engine name.
- iitsigor, [SekiroHKS](https://github.com/iitsigor/SekiroHKS) revision 3e2098e9 (no licence; consulted for names only) and vawser's [Elden Ring HKS notes](https://github.com/vawser/ER-Documentation): cross-checks of command naming.

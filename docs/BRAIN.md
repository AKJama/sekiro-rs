# The boss brain

A pluggable "boss brain" for an enemy with a Lua battle goal: his high-level choices can come from his real Lua AI (the default) or from an outside controller, including a hosted model.
Two enemies have action tables: the Great Shinobi Owl (c5060) and the Ashina soldier with a one-handed sword (c1010), the testbed for whether a hosted model plays better than the game's AI.
The code is in `crates/sim/src/brain/`, with a small hook in the Lua goal runtime (`crates/sim/src/lua_ai/runtime.rs`, `ActHook`).

```powershell
cargo run --release -p sekiro-game -- --arena owl --brain scripted --brain-log
cargo run --release -p sekiro-game -- --play --enemy soldier --brain scripted --brain-memory on
cargo run --release -p sekiro-sim --bin duel-sim -- --enemy soldier --brain scripted --brain-log --steps 1800
cargo test -p sekiro-sim --test owl_brain --test soldier_brain
```

`--brain lua` (the default) leaves his Lua AI unchanged; `--brain scripted` runs the deterministic test controller with that enemy's own test plans; `--brain-log` writes a JSONL log to `cache/brain_logs/`; `--brain-memory off|on|<path>` keeps the memory of every fight (on by default for hosted brains, see "Memory across fights").
`--force-act <id>` (the enemy requests that act whenever he is free) works for either enemy.

## Design

The controller sets the **plan**, the game keeps the **reflexes**.

- A plan is 1 to 3 of the boss's own action ids (the animation ids his Lua acts request), run in order as one act.
- Reflexes stay with the Lua goal runtime and keep working while a plan runs. For the Owl: his `Goal.Parry` (guard 3100, deflect 3101, special deflect 3102, back step 5201), shuriken reactions, the deflect counters (`Kengeki_Activate`), punishing a heal, turning to a Wolf behind him. For the soldier: `Common_Parry` (guard, deflect, back step), his deflect counters (`Kengeki_Activate`, 3050 to 3076) and his answers to Wolf lying down. For both, everything their HKS script does (staggers, getting up).
- The observation is fair and mirrored: only what the player can see of the boss, and the same about Wolf.
- Outcome memory lives in our code, because hosted models keep nothing between calls: every request carries this fight's tally and the memory of every fight with Wolf's habits, which is also kept on disk across sessions.
- One request is always in flight while the current act runs; if no answer is ready when the next act starts, the Lua AI's own choice runs (the boss never waits).

### Where a plan enters his AI

The Owl's battle goal (`GOAL_NingunOsa_506001_Battle`, `506001_battle.lua`) and the soldier's (`GOAL_Ochimusha_katate_101000_Battle`, `101000_battle.lua`) do, on every activation:

1. reactions first: `Kengeki_Activate` (counters after being deflected) returns early, and `Common_ActivateAct` may queue a reaction (a heal punish, a step away);
2. weights for each of his acts by distance and state;
3. `Common_Battle_Activate(ai, goal, weights, acts, after, params)`, the shared helper that picks one act and runs it.

`AiBrain::enable_act_hook` keeps the script's `Common_Battle_Activate` under another global name and puts a host function in its place.
That host function is a *slot*: if a plan is waiting and the moment is not a reaction, it queues the plan on the battle goal instead of calling the script's pick; otherwise it calls the script's own function unchanged.
A moment is a reaction when the script already queued subgoals in step 1, or when it weighted one of the boss's *reflex acts*:

- the Owl: Act21, which turns to punish a heal or to face a Wolf behind him, and Act40, his answer to SpEffect 110125;
- the soldier: Act23 (strafe), Act25 (back away) and Act27 (wait and strafe), which his script weighs before any distance band, when Wolf is down (SpEffect 109031), on SpEffects 3170200 and 3101540, or with no path to Wolf.

A deflect counter never reaches a slot at all, because `Kengeki_Activate` returns first.
The soldier's interrupt also replans when Wolf goes down (SpEffect 109031, after his leaping slash 3003 for example), so a plan that knocks Wolf down is cut short by his own back step or retreat (Act24, Act25), as one of his own acts would be.

A plan runs exactly as one of his acts would:

- an approach with the scripts' own `Approach_Act_Flex`, to the distance of the act that opens with that action (less his hit radius, as his acts do);
- a single attack as `AttackTunableSpin`; a chain as `ComboAttackTunableSpin`, `ComboRepeat`, ..., `ComboFinal`; a step as `SpinStep`;
- each attack sets the AI number its act sets: for the Owl, number 5, his pressure score read by `Kengeki_Activate`; for the soldier, number 5 is set to 1 by his rushes (3010, 3011), which his script then stops weighing from afar.

So animations, TAE cancel windows, the combo-cancel envs and reactions go through the same goals and the same native `CommonAttack` as his own choices.
When a reflex interrupts a plan (a `Parry` clears the battle goal's subgoals and queues a guard), the rest of the plan is dropped, as with his own acts.

With `--brain lua` the hook is still installed but never has a plan, and the script runs unchanged; `lua_brain_leaves_his_choices_unchanged` (in both `owl_brain.rs` and `soldier_brain.rs`) checks that a fight takes the same action sequence with and without it.

## The observation

One JSON object per request, about 450 to 500 bytes, always under 1 KB.
Numbers are rounded to two decimals, angles to whole degrees.

| Field | Meaning |
|---|---|
| `t` | Session clock, seconds |
| `boss`, `wolf` | One object per side, below |
| `dist` | Horizontal distance, metres |
| `dy` | Wolf's height above the boss, metres |
| `wolf_bearing` | Where Wolf is from the boss's facing, degrees, positive to the left |
| `boss_bearing` | Where the boss is from Wolf's facing, degrees, positive to the left |
| `locked_on` | Wolf is locked on to the boss |
| `since_boss_hit` | Seconds since the boss last landed an unguarded hit on Wolf (`null`: never, this fight) |
| `since_wolf_hit` | Seconds since Wolf last landed an unguarded hit on the boss |
| `plan` | The plan running now (empty while the Lua AI's own act runs) |

Each side:

| Field | Meaning |
|---|---|
| `hp` | HP left, fraction of max |
| `posture` | Posture damage built up, fraction of max (the bar the game shows: 0 calm, 1 broken) |
| `lives` | Deathblow marks left for the boss (2 at the start for the Owl, 1 for the soldier); Wolf's own (1: resurrection is not modelled) |
| `state` | Leaf name of the behaviour state playing, such as `Attack3005`, `GroundAttackCombo1Release`, `StandDamageMiddle` |
| `act` | The boss only: his current action id (attack, step or guard animation) |
| `attacking` | Wolf: his clip is in the window the game signals to enemies as an incoming attack (TAE flag 63); the boss: an attack action plays |
| `guarding` | In a guard (TAE flag 3) |
| `airborne` | Not on the ground |
| `staggered` | In a damage or recoil reaction (behaviour state group `*Damage*`) |
| `deathblow_open` | Can be deathblown now (posture broken, or 1 HP with marks left) |

There are no item counts, AI numbers, params, cool-downs or other hidden state.

## The actions

Each boss has a `BossActions` table (`brain::actions`): his name for the instructions, the action ids with labels, approach distances and AI numbers, his reflex acts, the most actions per plan (3), named combos offered to hosted models as one choice, his back steps (for the habits) and the scripted controller's test plans.
`BossActions::for_chr` picks the table by character id; `Duel::set_brain` refuses an enemy without one.

### The Great Shinobi Owl (c5060)

`brain::actions::OWL`; approach distances and pressure come from the act in `506001_battle.lua` that uses each id, descriptions from his TAE and AtkParam rows (`docs/OWL.md`).

| Id | What | Approach (m) | Kind |
|---|---|---:|---|
| 3000 | Kesa slash, combo opener | 4.8 | Attack |
| 3001 | Second slash | 5.8 | Attack |
| 3002 | Third slash, combo finisher | 4.8 | Attack |
| 3003 | Jumping forward roll, slash down | 10.7 | Attack |
| 3005 | Front kick (posture only) | 2.8 | Attack |
| 3009 | Shuriken | 9.7 | Attack |
| 3011 | Rushing slash | 11 | Attack |
| 3014 | Second shuriken | 9.7 | Attack |
| 3015 | Rushing slash curving left | 9 | Attack |
| 3018 | Shuriken, then a dash slash | 11 | Attack |
| 3031 | Recovery-sealing throw | 5.5 | Attack |
| 3032 | Recovery-sealing throw follow-up | 9 | Attack |
| 3063 | Rushing follow-up, two hits | 4.8 | Attack |
| 5201 | Back step | | Step |
| 5202 | Side step (his act picks it with Wolf on his right) | | Step |
| 5203 | Side step (his act picks it with Wolf on his left) | | Step |

The tool acts 3010 (firecrackers), 3017 (explosive slash), 3030 (poison balls) and 3069 (smoke screen) are in the vocabulary since their bullets work (`docs/OWL.md`, "Tools"); their approach distances are ours, from where their bullets land, and their pressure is 0 until their acts are read.
Left out on purpose: warps (3035 to 3047; the transparency is not rendered), grabs (3037, 3040, 3041; they now throw Wolf, `docs/OWL.md` "Grabs", but are not in the vocabulary yet), phase-2 acts other than the poison balls 3030, the perilous sweeps of Act32 and Act33, and the deflect counters (3060 to 3066 with the firecracker counter 3064, 3090, 3091), which stay reactions; the smoke screen 3069 is offered as an act.
Combo: `3000+3001+3002` (three-slash combo).
Scripted test plans: the kick 3005, then the recovery-sealing throw 3031 once two kicks were deflected.

### The Ashina soldier (c1010)

`brain::actions::SOLDIER`, from his battle goal 101000 (`101000_battle.lua` in his AI bundle `m11_00_00_00.luabnd`, NpcThinkParam 10100000).
Approach distances are the base distance of the act that uses each id (his acts add 2 m when Wolf is outside his front 120 degrees, `karaburiDist`; a plan does not); labels from his BehaviorParam rows (the judge ids in his TAE, `2101000xx`) and AtkParam rows (`101001xx`, `101002xx`).

| Id | What | Approach (m) | Act | AI number |
|---|---|---:|---|---|
| 3000 | Sweeping slash, combo opener | 2.5 | Act01 | |
| 3001 | Second sweeping slash (follows 3000) | 2.5 | Act01 | |
| 3002 | Running sweeping slash | 7 | Act02 | |
| 3003 | Leaping slash | 7.1 | Act03 | |
| 3005 | Kick (posture only, 1 HP) | 3.7 | Act05 | |
| 3007 | Rising slash (follows the kick) | 3.7 | Act05 | |
| 3008 | Flurry of three slashes | 4.7 | Act06 | |
| 3009 | Stone throw | none | Act07 | |
| 3010 | Pouncing leap (1 HP) | 5 | Act08 | 5 = 1 |
| 3011 | Running lariat, then a pounce (1 HP) | 7.6 | Act09 | 5 = 1 |
| 3012 | Left sweep, then an overhead slam | 3.5 | Act11 | |
| 3013 | Perilous thrust (cannot be guarded; deflect or counter it) | 5 | Act14 | |
| 3014 | Perilous low sweep (cannot be guarded or deflected; jump) | 3.5 | Act15 | |
| 5201 | Back step | | Act24 | |
| 5211 | Short back step (his act uses it within 4 m) | | Act24 | |

- 3001 and 3007 only follow 3000 and 3005 in his acts; as openers they use their act's distance (ours).
- The stone throw's act never approaches, so neither does a plan opening with it.
- Left out: 3004 and 3006 (Act04) and the side steps 5202 and 5203 (Act22), which his script names but his TAE (`c1010.tae`) has no clips for (the script is shared with other soldier variants); 3013 toward an event point (Act10) and 3030 (Act12, a dodge used only in the combat tutorial); the deflect counters (3050 to 3076, `Kengeki01` to `Kengeki08`), which stay reactions.
- Combos: `3000+3001` (both sweeping slashes, his Act01) and `3005+3007` (kick, then rising slash, his Act05).
- Scripted test plans: both sweeping slashes, then the kick and rising slash once two openers were deflected.

A plan with an unknown id (for the soldier also an id his script names without a clip, such as 3004), more than 3 actions or none is refused and logged, and the Lua AI picks that act.

## The tally

`brain::Tally` keeps one *exchange* per action of each side in the current fight: the boss's attack and step animations (3000 to 3099, 5200 to 5299), and each of Wolf's attacks from the step his clip raises the parry-timing flag (63) until the clip changes, keyed by its behaviour state name shortened for requests (`GroundAttackCombo1Release` becomes `AttackCombo1`: the `Ground` prefix and `Release` are dropped).
An exchange stays open for a punish window of 1 s after its clip ends, then counts.

| Count | Meaning |
|---|---|
| `n` | Exchanges |
| `hit` | One of its hits landed unguarded |
| `blocked` | The other side guarded one of its hits |
| `deflected` | The other side deflected one of its hits |
| `whiffed` | None of the three (dodged, out of range, interrupted before contact) |
| `punished` | Its side took an unguarded hit during it or within 1 s after |

Each flag counts at most once per exchange; hits that arrive after the clip ended (a shuriken in flight) count for that side's latest exchange.
The request carries this fight's counts per key (`tally.boss`, `tally.wolf`, most used first) as arrays `[n, hit, blocked, deflected, whiffed, punished]`; the same counts over every fight are in `memory.boss` and `memory.wolf`, so a controller can notice "the player deflects 3005" and "the player has adapted" (this fight differs from all fights).

```json
{"boss":{"3005":[8,7,0,0,1,2],"3031":[1,0,0,0,1,0]},
 "wolf":{"AttackCombo2":[1,1,0,0,0,1]}}
```

A fight runs from the start, or a respawn, until either side dies (when the duel schedules its respawn; a boss with deathblow marks left gets up from a deathblow and fights on).
Then the tally and the habits go into the memory, and the next fight starts with a fresh tally.

## Memory across fights

`brain::Memory` sums the tally and Wolf's habits over every fight against one enemy, with the fight count and who won.
It lives in `logs/memory/<chr>.json` (the root whitelist `.gitignore` ignores `logs/`), is loaded when the brain is set up and saved at the end of each fight (a death), at every respawn, every 15 s of fight (so a crash keeps most of it) and when the game exits (the driver's `Drop`); an unfinished fight is saved as a fight with no winner.
Saves go through a temporary file, so a crash never leaves half a file.

`--brain-memory off` keeps memory for the session only (across deaths, not on disk); `on` uses `logs/memory/<chr>.json`; a path ending in `.json` is the file; any other path is a folder holding `<chr>.json`.
Without the flag, hosted brains (`decisions`, `responses`) use the default file and `lua` and `scripted` keep session memory only.
The same flag works in `duel-sim`.

The file format, version 1 (an example after two short soldier fights in `soldier_brain.rs`, rounded):

```json
{
  "version": 1,
  "chr": "c1010",
  "fights": 2,
  "boss_wins": 1,
  "wolf_wins": 0,
  "boss": {"3000": [18, 1, 0, 0, 17, 0], "3008": [1, 0, 0, 0, 1, 0]},
  "wolf": {"AttackCombo1": [14, 2, 12, 0, 0, 0], "EasyDeflectedAtk": [20, 0, 7, 13, 0, 0]},
  "habits": {
    "seconds": 30.7,
    "range_seconds": [29.2, 1.5, 0.0],
    "attack_dist": {"sum": 41.9, "n": 36},
    "after": {"guarded": {"n": 32, "to": {"attack": 32}, "attack_dist": {"sum": 29.4, "n": 32}}},
    "answers": {"3000": {"n": 18, "to": {"hit": 1, "none": 17}}, "3008": {"n": 1, "to": {"none": 1}}},
    "punish": {"3000": {"sum": 13.1, "n": 15}},
    "heals": {"hp": {"sum": 0.0, "n": 0}, "at_low": 0, "low_spells": 1},
    "jumps": 0
  }
}
```

A missing file starts fresh with a note on stderr.
An unusable file (bad JSON, another `version`, or another enemy's `chr`) starts fresh with a warning and is moved aside to `<name>.json.corrupt`, so the next save does not destroy it.
Every field but `version` and `chr` has a default, so a file from a build with fewer fields still loads.

## Wolf's habits

`brain::habits` watches Wolf the way the boss could (mirrored fairness): his position, behaviour state, guard and airborne flags, his HP bar and the hits, never his buttons.

| Habit | How it is measured |
|---|---|
| Attack distance | Distance when each of his attacks starts (when his swing is signalled, TAE flag 63) |
| Range | Seconds close (under 3 m), mid (3 to 6 m) and far (over 6 m) |
| After a trigger | His first response within 1.2 s after `deflect` (he deflected a boss hit), `guard` (he guarded one) and `guarded` (the boss guarded or deflected his hit), and within 2 s of the start of a boss `back_step` (5201 for the Owl, 5201 and 5211 for the soldier): `attack` (with its distance), `dodge`, `jump`, `heal`, else at the window's end `guard` (still guarding), `back` (1.5 m further away) or `wait` |
| Answers per boss action | The first hit of the exchange: `deflect`, `guard`, `hit` (the boss's hit landed), `counter` (Wolf's unguarded hit landed first); without one, `dodge` or `jump` if he did, else `none` |
| Punish speed per boss action | Seconds from the action's end to his next attack, if within 1.5 s |
| Heals | HP at each heal (an `Item*` state), heals below half HP, and how often his HP fell below half |
| Jumps | Jumps started (a `*Jump*` state, or airborne outside a damage state) |

Dodges are `*Step*` states other than `*FromStep*` and attacks; these state rules are ours, from the names in `c0000.hks`.

The request carries a compact summary (`HabitSummary`): only entries with at least 2 samples, most sampled first, rounded.

```json
"habits":{"attack_m":3.0,"range_pct":[66,34,0],
 "after":{"back_step":"attack 100% from 3.0m n10","deflect":"guard 100% n10"},
 "vs":{"3000":"deflect 100% n10","3005":"dodge 50%, jump 50% n2"},
 "punish_s":{"5201":0.4},"jumps_pm":1.6}
```

(from `habits_from_a_scripted_wolf`: a Wolf who always attacks from 3 m 0.4 s after the soldier's back step and holds his guard after deflecting).

## The request and its size

```json
{"id":12,"obs":{...},"tally":{"boss":{...},"wolf":{...}},
 "memory":{"fights":2,"boss_wins":1,"wolf_wins":0,"boss":{...},"wolf":{...},"habits":{...}}}
```

`memory` counts the earlier fights (`fights`, `boss_wins`, `wolf_wins`; the rest were unfinished) and sums the counts and habits over every fight, this one included.
The whole request is kept within 1200 bytes of compact JSON (`REQUEST_BUDGET`): `BrainRequest::fit` drops the least telling entries (fewest samples) until it fits, in this order, each map down to 4, 3, 2, 1 and 0 entries: `memory.boss`, `memory.wolf`, the habits' `punish_s`, `vs` and `after`, then `tally.wolf` and `tally.boss`; last the single habit values and the memory itself.
Measured (`requests_fit_the_budget`, `memory_survives_a_death_and_a_new_session`): a typical request with one or two earlier fights is 750 to 940 bytes and needs no trimming; a worst case (123 fights, every soldier action and habit filled, the widest observation) is 2818 bytes before and 1172 after, keeping two entries per map and every single value.

## Timing model

Per duel step (`BrainDriver`):

1. Poll the controller.
   A plan that arrives is checked against the action list and handed to the act hook, where it waits for the next slot.
2. The Lua AI thinks.
   At a slot the waiting plan is queued, unless the moment is a reaction (then it keeps waiting); with no plan waiting, the Lua AI's own pick runs.
3. After any slot the next request goes out at once, so the controller thinks while the current act runs.
   At most one request is in flight; a new one is sent only when nothing is in flight and no plan is waiting.
4. After the step, action changes and hits feed the tally and the habits; a death ends the fight, a respawn starts the next.

A plan whose observation is older than `max_staleness` (6 s) when it would run is dropped and a fresh request goes out.
A 3-action plan with an approach can take 5 s, so the observation behind the next plan is naturally one act old.
A controller that answers `None` leaves the next act to the Lua AI.

`ScriptedBrain` models its latency on the fight clock, so tests are deterministic: with a 4 s delay (longer than his acts) most slots fall back to the Lua AI, and the Owl is never idle longer than his own AI makes him (`slow_brain_falls_back_to_his_lua_ai_without_freezing`); a controller that never answers leaves every act to the soldier's AI (`lua_picks_when_no_answer_comes`).

## The log

With `--brain-log`, one file per session: `cache/brain_logs/<unix ms>-<chr>-<brain>.jsonl`.

| `ev` | Fields |
|---|---|
| `start` | `brain`, `chr`, `max_plan`, `actions` (id and label) |
| `request` | `t`, `request` (`id`, `obs`, `tally`, `memory`): exactly what the controller saw |
| `reply` | `t`, `id`, `plan` (or `null`), `latency` |
| `slot` | `t`, `by` (`plan`, `lua`, `reflex`), `plan` |
| `outcome` | `t`, `side`, `key`, `start`, `end`, `hit`, `blocked`, `deflected`, `punished`, `whiffed` |
| `fight_end` | `t`, `winner` (`boss`, `wolf`, `none`), `fights` (in the memory now) |
| `fight_start` | `t` (after a respawn) |
| `stale`, `invalid`, `late` | `t`, `id` (and `error` for `invalid`) |

Requests and replies are enough to replay a controller's decisions offline; the slots and outcomes show what each decision led to.

## Adding a controller

Implement `brain::BossBrain`:

```rust
pub trait BossBrain {
    fn name(&self) -> &'static str;
    fn plans(&self) -> bool { true }
    fn submit(&mut self, request: &BrainRequest);
    fn poll(&mut self, now: f32) -> Option<BrainReply>;
}
```

- `submit` must not block; `poll` returns the answer to the request in flight once it is ready (with the request's `id`).
- A controller that needs time (a network call) owns a worker thread: `submit` sends the serialized request down a `std::sync::mpsc` channel, the worker does the call and sends back `(id, Option<Plan>)`, and `poll` does `try_recv`.
  The duel is not `Send` (the script VMs use `Rc`), but the controller's channel ends are, so the worker never touches the simulation.
- Add it to `BrainKind` and its `by_name` / `build(chr)` so `--brain <name>` picks it.

Another boss needs its own `BossActions` table (name, action ids, approach distances, AI numbers, reflex acts, combos, back steps, scripted test plans) and a `BossActions::for_chr` entry; the driver, observation, tally, habits and hosted requests are the same for every boss.

## Hosted model controller

`sekiro-brain-net` implements `HostedBrain` with Decisions and Responses backends.
The game CLI builds a boxed `BossBrain` directly (`sekiro_brain_net::from_cli(name, model, chr)`, which offers the enemy's own `BossActions`), keeping `sekiro-sim` independent of HTTP and credentials.
Only the worker sends HTTP: `submit` serializes compact request JSON and uses an unbounded channel, while `poll` uses `try_recv` and returns the original request id.
One rustls `ureq` agent and its connection pool live for the fight.
Redirects and environment proxies are disabled.
The global per-request deadline defaults to 1.5 s and covers connection setup, headers and the response body.
On drop, the channel closes, queued work is discarded, and the worker finishes its in-flight call under that deadline before joining.

### Flags and configuration

```powershell
$env:CARGO_TARGET_DIR = 'target/codex'
cargo run --release -p sekiro-game -- --arena owl --brain decisions
cargo run --release -p sekiro-game -- --arena owl --brain responses --brain-model gpt-6-luna
cargo run --release -p sekiro-game -- --play --enemy soldier --brain decisions --brain-memory logs/memory
```

These commands authorize real calls when the player runs them; development tests make no provider API calls.
`lua`, `scripted`, `--force-act`, and the independent simulation `--brain-log` remain available.
Hosted brains remember every fight in `logs/memory/<chr>.json` unless `--brain-memory off` is given (see "Memory across fights").
`--simple-ai` retains precedence over the boss controller.
`HostedConfig::from_env` checks `OPENAI_API_KEY` in the environment first, then only the `OPENAI_API_KEY=` setting in the working directory `.env`.
Surrounding single or double quotes on the dotenv value are accepted.
No real key or `.env` was inspected during implementation or tests.
`ApiKey` prints `***` in Debug; configuration diagnostics redact the key, and errors never include HTTP headers, bodies or transport details.
Authorization headers are marked sensitive.
The `log` dependency caps its macros at Debug at compile time because `ureq-proto` Trace dumps raw wire bytes, including credentials.
This disables Trace from libraries using `log`; the game's separate `tracing` macros are unaffected.
A missing key or controller startup failure prints a fixed diagnostic and falls back to the Lua brain without panicking.
Library callers can override `backend`, `model`, `base_url` (default `https://api.openai.com`), `timeout`, `log_path`, and `api_key`.
The default model is `gpt-6-luna` for both backends.

### Verified API contract (2026-10-10)

Sources: [Decisions create reference](https://developers.openai.com/api/reference/resources/decisions/methods/create), [Decisions guide](https://developers.openai.com/api/docs/guides/decisions), and [Responses create reference](https://developers.openai.com/api/reference/resources/responses/methods/create).
Decisions uses text `input` and a `questions` array; a choice question has 2 to 255 unique `choices[].value` values with optional descriptions.
Its response contains `model`, `answers` and `usage`; the named answer has type `choice` with a `choice` value, or type `refusal`.
Responses structured output is under `text.format`; its assistant message content contains `output_text` or `refusal` parts.
Both backends extract `usage.input_tokens` and `usage.output_tokens` when supplied, including usage on failed answers.
The docs establish these field names; account access and this model's acceptance of `none`, strict structured output, and a 16-token cap on Responses have not been live-verified.

Both requests use the same instructions, action ids and labels from the enemy's `BossActions`, his named combos (`3000+3001+3002` for the Owl; `3000+3001` and `3005+3007` for the soldier), and `lua` to leave an act to his own AI.
The instructions name the enemy (`BossActions::name`), explain the input in two sentences (the observation, this fight's tally arrays, and the memory of every fight with Wolf's habits) and invite baiting and punishing his habits.
Below are the complete request bodies for the Owl with placeholder observation, tally and memory objects; actual `input` contains the compact serialization of `BrainRequest`.
Bodies are emitted as compact JSON; indentation here is for reading.

### Decisions request

`POST {base_url}/v1/decisions`

```json
{
  "model": "gpt-6-luna",
  "input": "{\"id\":1,\"obs\":{},\"tally\":{},\"memory\":{}}",
  "questions": [
    {
      "type": "choice",
      "name": "boss_action",
      "instructions": "You are the Great Shinobi Owl fighting Wolf. Pick your next action. Input: obs is a fair view of both fighters; tally counts this fight per action as [n, hit, blocked, deflected, whiffed, punished]; memory holds every fight so far (earlier fights and wins, the same counts, and Wolf's habits: his attack distance, time at each range, what he does after deflects, guards and your back steps, how he answers and how fast he punishes each of your actions). Bait his habits and punish them. Reflexes (guarding, deflecting, counters and reactions) are automatic. Choose lua to leave this act to your own AI. Actions:\n3000: kesa slash, combo opener\n3001: second slash\n3002: third slash, combo finisher\n3003: jumping forward roll, slash down\n3005: front kick (posture only)\n3009: shuriken\n3010: firecrackers scattered in front (unblockable bangs)\n3011: rushing slash\n3014: second shuriken\n3015: rushing slash curving left\n3017: explosive slash (fire)\n3018: shuriken then dash slash\n3030: poison balls, three rows of poison floor (phase 2)\n3031: recovery-sealing throw\n3032: recovery-sealing throw follow-up\n3063: rushing follow-up, two hits\n3069: smoke screen (blinds, breaks lock-on)\n5201: back step\n5202: side step (his act picks it with Wolf on his right)\n5203: side step (his act picks it with Wolf on his left)\n3000+3001+3002: three-slash combo",
      "choices": [
        {
          "value": "3000",
          "description": "kesa slash, combo opener"
        },
        {
          "value": "3001",
          "description": "second slash"
        },
        {
          "value": "3002",
          "description": "third slash, combo finisher"
        },
        {
          "value": "3003",
          "description": "jumping forward roll, slash down"
        },
        {
          "value": "3005",
          "description": "front kick (posture only)"
        },
        {
          "value": "3009",
          "description": "shuriken"
        },
        {
          "value": "3010",
          "description": "firecrackers scattered in front (unblockable bangs)"
        },
        {
          "value": "3011",
          "description": "rushing slash"
        },
        {
          "value": "3014",
          "description": "second shuriken"
        },
        {
          "value": "3015",
          "description": "rushing slash curving left"
        },
        {
          "value": "3017",
          "description": "explosive slash (fire)"
        },
        {
          "value": "3018",
          "description": "shuriken then dash slash"
        },
        {
          "value": "3030",
          "description": "poison balls, three rows of poison floor (phase 2)"
        },
        {
          "value": "3031",
          "description": "recovery-sealing throw"
        },
        {
          "value": "3032",
          "description": "recovery-sealing throw follow-up"
        },
        {
          "value": "3063",
          "description": "rushing follow-up, two hits"
        },
        {
          "value": "3069",
          "description": "smoke screen (blinds, breaks lock-on)"
        },
        {
          "value": "5201",
          "description": "back step"
        },
        {
          "value": "5202",
          "description": "side step (his act picks it with Wolf on his right)"
        },
        {
          "value": "5203",
          "description": "side step (his act picks it with Wolf on his left)"
        },
        {
          "value": "3000+3001+3002",
          "description": "three-slash combo"
        },
        {
          "value": "lua",
          "description": "leave it to his own AI"
        }
      ]
    }
  ]
}
```

The client reads the one answer named `boss_action` and accepts `answers[0].choice`, such as `"3005"`.
A typed refusal is a failure, rather than an intentional `lua` choice.

### Responses request

`POST {base_url}/v1/responses`

```json
{
  "model": "gpt-6-luna",
  "reasoning": {
    "effort": "none"
  },
  "instructions": "You are the Great Shinobi Owl fighting Wolf. Pick your next action. Input: obs is a fair view of both fighters; tally counts this fight per action as [n, hit, blocked, deflected, whiffed, punished]; memory holds every fight so far (earlier fights and wins, the same counts, and Wolf's habits: his attack distance, time at each range, what he does after deflects, guards and your back steps, how he answers and how fast he punishes each of your actions). Bait his habits and punish them. Reflexes (guarding, deflecting, counters and reactions) are automatic. Choose lua to leave this act to your own AI. Actions:\n3000: kesa slash, combo opener\n3001: second slash\n3002: third slash, combo finisher\n3003: jumping forward roll, slash down\n3005: front kick (posture only)\n3009: shuriken\n3010: firecrackers scattered in front (unblockable bangs)\n3011: rushing slash\n3014: second shuriken\n3015: rushing slash curving left\n3017: explosive slash (fire)\n3018: shuriken then dash slash\n3030: poison balls, three rows of poison floor (phase 2)\n3031: recovery-sealing throw\n3032: recovery-sealing throw follow-up\n3063: rushing follow-up, two hits\n3069: smoke screen (blinds, breaks lock-on)\n5201: back step\n5202: side step (his act picks it with Wolf on his right)\n5203: side step (his act picks it with Wolf on his left)\n3000+3001+3002: three-slash combo",
  "input": [
    {
      "role": "user",
      "content": "{\"id\":1,\"obs\":{},\"tally\":{},\"memory\":{}}"
    }
  ],
  "text": {
    "format": {
      "type": "json_schema",
      "name": "boss_action",
      "strict": true,
      "schema": {
        "type": "object",
        "properties": {
          "action": {
            "type": "string",
            "enum": [
              "3000",
              "3001",
              "3002",
              "3003",
              "3005",
              "3009",
              "3010",
              "3011",
              "3014",
              "3015",
              "3017",
              "3018",
              "3030",
              "3031",
              "3032",
              "3063",
              "3069",
              "5201",
              "5202",
              "5203",
              "3000+3001+3002",
              "lua"
            ]
          }
        },
        "required": [
          "action"
        ],
        "additionalProperties": false
      }
    }
  },
  "max_output_tokens": 16,
  "store": false
}
```

The client requires a completed response with one assistant output text object, such as `{"action":"3000+3001+3002"}`.
Every numeric component is checked against the boss action table, and plans longer than `max_plan` are refused.
`lua` returns `plan: None` and counts as answered.
Timeouts, HTTP failures, transport failures, refusals, malformed JSON, incomplete responses, invalid answers and invalid actions return `plan: None` and count by failure reason.
An unknown value is never copied into diagnostics or logs.
When an answer has not arrived at an act slot, `BrainDriver` runs the Lua choice without waiting; its existing staleness and reflex rules continue to apply.

### Hosted telemetry log

The game enables one separate JSONL file at `logs/brain-<unix nanoseconds>.jsonl` for either hosted backend.
The root whitelist `.gitignore` ignores `logs/`.
Each completed call appends and flushes one line:

```json
{"id":1,"backend":"decisions","model":"gpt-6-luna","round_trip_ms":42.5,"status":"answered","failure":null,"chosen":"3005","usage":{"input_tokens":450,"output_tokens":0}}
```

Failed calls have `status: "failed"`, `chosen: null`, and a failure such as `"timeout"`, `"refusal"`, `"unknown_action"`, or `{"http":429}`.
Logs contain no key, headers, raw provider payloads or observation; the separate simulation log remains available with `--brain-log`.
`HostedBrain::stats()` returns requests, answered calls, failures by reason, mean/max round-trip milliseconds, summed input/output tokens and log write errors.
Keep `stats_handle()` before boxing the controller to inspect the same stats later on the game thread.
Log write failures stop further logging and increment `log_errors` without discarding a valid plan.

### Offline verification

All HTTP tests bind a mock to `127.0.0.1:0` and supply a fake key.
They check both request bodies, action labels, ids, combo parsing, Lua answers, refusals, HTTP 429/500, malformed JSON, unknown ids, incomplete/invalid answers, usage totals, timing, redaction, nonblocking submit and bounded drop.
Two sequential Decisions requests are checked to reuse exactly one accepted TCP connection.
A unit test compares both complete request examples above against the actual request builder.
Credential-source tests use in-memory dotenv text and never read the real environment key or `.env`.

## Inferred and ours

- The slot is `Common_Battle_Activate`: every table battle goal of this generation calls it once after weighting its acts (scripts); replacing its pick there keeps the reactions before it.
- The reflex acts (Act21, Act40) and what counts as a reaction (subgoals already queued) are our reading of `506001_battle.lua`.
- A plan's approach runs when the distance is more than 3 m beyond the target distance and has longer lifetimes than his acts (3 s walking, 5 s running, against his 1.5 s and 3 s): his script only picks an act at a distance that suits it, while a plan may start from anywhere.
- Plan chains use 9999 as every step's success distance (his acts sometimes use shorter ones to break off a combo when Wolf backs away).
- `staggered` from the behaviour state group name, and Wolf's attacks from TAE flag 63, are ours.
- The soldier's reflex acts (Act23, Act25, Act27) are our reading of `101000_battle.lua`: they are the acts his script weighs only as reactions, before any distance band.
- The habit rules (the windows, the range bands, `back` at 1.5 m, the state-name rules for dodges, jumps and heals, the first hit of an exchange as its answer) are ours.
- A fight ends when the duel schedules its respawn; an unfinished fight (the game closed) counts as a fight with no winner.

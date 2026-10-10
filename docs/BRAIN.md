# The boss brain

A pluggable "boss brain" for the Great Shinobi Owl (c5060): his high-level choices can come from his real Lua AI (the default) or from an outside controller, including a hosted model.
The code is in `crates/sim/src/brain/`, with a small hook in the Lua goal runtime (`crates/sim/src/lua_ai/runtime.rs`, `ActHook`).

```powershell
cargo run --release -p sekiro-game -- --arena owl --brain scripted --brain-log
cargo run --release -p sekiro-sim --bin duel-sim -- --enemy owl --brain scripted --brain-log --steps 1800
cargo test -p sekiro-sim --test owl_brain
```

`--brain lua` (the default) leaves his Lua AI unchanged; `--brain scripted` runs the deterministic test controller; `--brain-log` writes a JSONL log to `cache/brain_logs/`.

## Design

The controller sets the **plan**, the game keeps the **reflexes**.

- A plan is 1 to 3 of the Owl's own action ids (the animation ids his Lua acts request), run in order as one act.
- Reflexes stay with the Lua goal runtime and keep working while a plan runs: his `Goal.Parry` (guard 3100, deflect 3101, special deflect 3102, back step 5201), shuriken reactions, the deflect counters (`Kengeki_Activate`), punishing a heal, turning to a Wolf behind him, and everything his HKS script does (staggers, getting up).
- The observation is fair and mirrored: only what the player can see of the Owl, and the same about Wolf.
- Outcome memory lives in our code, because hosted models keep nothing between calls; every request carries the tally.
- One request is always in flight while the current act runs; if no answer is ready when the next act starts, the Lua AI's own choice runs (the boss never waits).

### Where a plan enters his AI

His battle goal (`GOAL_NingunOsa_506001_Battle`, `506001_battle.lua`) does, on every activation:

1. reactions first: `Kengeki_Activate` (counters after being deflected) returns early, and `Common_ActivateAct` may queue a reaction (a heal punish, a step away);
2. weights for each of his acts by distance and state;
3. `Common_Battle_Activate(ai, goal, weights, acts, after, params)`, the shared helper that picks one act and runs it.

`AiBrain::enable_act_hook` keeps the script's `Common_Battle_Activate` under another global name and puts a host function in its place.
That host function is a *slot*: if a plan is waiting and the moment is not a reaction, it queues the plan on the battle goal instead of calling the script's pick; otherwise it calls the script's own function unchanged.
A moment is a reaction when the script already queued subgoals in step 1, or when it weighted one of the boss's *reflex acts* (for the Owl, Act21, which turns to punish a heal or to face a Wolf behind him, and Act40, his answer to SpEffect 110125).
A deflect counter never reaches a slot at all, because `Kengeki_Activate` returns first.

A plan runs exactly as one of his acts would:

- an approach with the scripts' own `Approach_Act_Flex`, to the distance of the act that opens with that action (less his hit radius, as his acts do);
- a single attack as `AttackTunableSpin`; a chain as `ComboAttackTunableSpin`, `ComboRepeat`, ..., `ComboFinal`; a step as `SpinStep`;
- each attack sets AI number 5 (his pressure score, read by `Kengeki_Activate`) as his act does.

So animations, TAE cancel windows, the combo-cancel envs and reactions go through the same goals and the same native `CommonAttack` as his own choices.
When a reflex interrupts a plan (his `Parry` clears the battle goal's subgoals and queues a guard), the rest of the plan is dropped, as with his own acts.

With `--brain lua` the hook is still installed but never has a plan, and the script runs unchanged; `lua_brain_leaves_his_choices_unchanged` checks that a fight takes the same action sequence with and without it.

## The observation

One JSON object per request, about 450 to 500 bytes, always under 1 KB.
Numbers are rounded to two decimals, angles to whole degrees.

| Field | Meaning |
|---|---|
| `t` | Fight time, seconds |
| `owl`, `wolf` | One object per side, below |
| `dist` | Horizontal distance, metres |
| `dy` | Wolf's height above the Owl, metres |
| `wolf_bearing` | Where Wolf is from the Owl's facing, degrees, positive to the left |
| `owl_bearing` | Where the Owl is from Wolf's facing, degrees, positive to the left |
| `locked_on` | Wolf is locked on to the Owl |
| `since_owl_hit` | Seconds since the Owl last landed an unguarded hit on Wolf (`null`: never) |
| `since_wolf_hit` | Seconds since Wolf last landed an unguarded hit on the Owl |
| `plan` | The plan running now (empty while the Lua AI's own act runs) |

Each side:

| Field | Meaning |
|---|---|
| `hp` | HP left, fraction of max |
| `posture` | Posture damage built up, fraction of max (the bar the game shows: 0 calm, 1 broken) |
| `lives` | Deathblow marks left for the Owl (2 at the start); Wolf's own (1: resurrection is not modelled) |
| `state` | Leaf name of the behaviour state playing, such as `Attack3005`, `GroundAttackCombo1Release`, `StandDamageMiddle` |
| `act` | The Owl only: his current action id (attack, step or guard animation) |
| `attacking` | Wolf: his clip is in the window the game signals to enemies as an incoming attack (TAE flag 63); the Owl: an attack action plays |
| `guarding` | In a guard (TAE flag 3) |
| `airborne` | Not on the ground |
| `staggered` | In a damage or recoil reaction (behaviour state group `*Damage*`) |
| `deathblow_open` | Can be deathblown now (posture broken, or 1 HP with marks left) |

There are no item counts, AI numbers, params, cool-downs or other hidden state.

## The actions

The plan vocabulary for the Owl (`brain::actions::OWL`); approach distances and pressure come from the act in `506001_battle.lua` that uses each id, descriptions from his TAE and AtkParam rows (`docs/OWL.md`).

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
A plan with an unknown id, more than 3 actions or none is refused and logged, and the Lua AI picks that act.

## The tally

`brain::Tally` keeps one *exchange* per action of each side: the Owl's attack and step animations (3000 to 3099, 5200 to 5299), and each of Wolf's attacks from the step his clip raises the parry-timing flag (63) until the clip changes, keyed by its behaviour state name.
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
The request carries the counts per key over the whole fight (`owl`, `wolf`) and over the last 10 exchanges of each side (`owl_recent`, `wolf_recent`), as arrays `[n, hit, blocked, deflected, whiffed, punished]`:

```json
{"owl":{"3005":[8,7,0,0,1,2],"3031":[1,0,0,0,1,0]},
 "owl_recent":{"3005":[8,7,0,0,1,2]},
 "wolf":{"GroundAttackCombo2Release":[1,1,0,0,0,1]},
 "wolf_recent":{"GroundAttackCombo2Release":[1,1,0,0,0,1]}}
```

So a controller can notice "the player deflects 3005" (high `deflected` overall) and "the player has adapted" (recent counts differ from overall).
The tally survives Wolf's death and the respawn: it is the memory of the whole session against this boss.

## Timing model

Per duel step (`BrainDriver`):

1. Poll the controller.
   A plan that arrives is checked against the action list and handed to the act hook, where it waits for the next slot.
2. The Lua AI thinks.
   At a slot the waiting plan is queued, unless the moment is a reaction (then it keeps waiting); with no plan waiting, the Lua AI's own pick runs.
3. After any slot the next request goes out at once, so the controller thinks while the current act runs.
   At most one request is in flight; a new one is sent only when nothing is in flight and no plan is waiting.
4. After the step, action changes and hits feed the tally.

A plan whose observation is older than `max_staleness` (6 s) when it would run is dropped and a fresh request goes out.
A 3-action plan with an approach can take 5 s, so the observation behind the next plan is naturally one act old.
A controller that answers `None` leaves the next act to the Lua AI.

`ScriptedBrain` models its latency on the fight clock, so tests are deterministic: with a 4 s delay (longer than his acts) most slots fall back to the Lua AI, and the Owl is never idle longer than his own AI makes him (`slow_brain_falls_back_to_his_lua_ai_without_freezing`).

## The log

With `--brain-log`, one file per fight: `cache/brain_logs/<unix ms>-c5060-<brain>.jsonl`.

| `ev` | Fields |
|---|---|
| `start` | `brain`, `chr`, `max_plan`, `actions` (id and label) |
| `request` | `t`, `request` (`id`, `obs`, `tally`): exactly what the controller saw |
| `reply` | `t`, `id`, `plan` (or `null`), `latency` |
| `slot` | `t`, `by` (`plan`, `lua`, `reflex`), `plan` |
| `outcome` | `t`, `side`, `key`, `start`, `end`, `hit`, `blocked`, `deflected`, `punished`, `whiffed` |
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
- Add it to `BrainKind` and its `by_name` / `build` so `--brain <name>` picks it.

Another boss needs its own `BossActions` table (action ids, approach distances, pressure changes, reflex acts) and a `BossActions::for_chr` entry.

## Hosted model controller

`sekiro-brain-net` implements `HostedBrain` with Decisions and Responses backends.
The game CLI builds a boxed `BossBrain` directly, keeping `sekiro-sim` independent of HTTP and credentials.
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
```

These commands authorize real calls when the player runs them; development tests make no provider API calls.
`lua`, `scripted`, `--force-act`, and the independent simulation `--brain-log` remain available.
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

Both requests use the same instructions, action ids and labels from `BossActions`, the three-slash combo when its ids are available, and `lua` to leave an act to his own AI.
Below are the complete request bodies with placeholder observation and tally objects; actual `input` contains the compact serialization of `BrainRequest`.
Bodies are emitted as compact JSON; indentation here is for reading.

### Decisions request

`POST {base_url}/v1/decisions`

```json
{
  "model": "gpt-6-luna",
  "input": "{\"id\":1,\"obs\":{},\"tally\":{}}",
  "questions": [
    {
      "type": "choice",
      "name": "owl_action",
      "instructions": "You are the Great Shinobi Owl fighting Wolf. Pick the next action using only the fair observation and outcome tally. Reflexes (guarding, deflecting, counters and reactions) are automatic. Choose lua to leave this act to your own AI. Actions:\n3000: kesa slash, combo opener\n3001: second slash\n3002: third slash, combo finisher\n3003: jumping forward roll, slash down\n3005: front kick (posture only)\n3009: shuriken\n3010: firecrackers scattered in front (unblockable bangs)\n3011: rushing slash\n3014: second shuriken\n3015: rushing slash curving left\n3017: explosive slash (fire)\n3018: shuriken then dash slash\n3030: poison balls, three rows of poison floor (phase 2)\n3031: recovery-sealing throw\n3032: recovery-sealing throw follow-up\n3063: rushing follow-up, two hits\n3069: smoke screen (blinds, breaks lock-on)\n5201: back step\n5202: side step (his act picks it with Wolf on his right)\n5203: side step (his act picks it with Wolf on his left)\n3000+3001+3002: three-slash combo",
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

The client reads the one answer named `owl_action` and accepts `answers[0].choice`, such as `"3005"`.
A typed refusal is a failure, rather than an intentional `lua` choice.

### Responses request

`POST {base_url}/v1/responses`

```json
{
  "model": "gpt-6-luna",
  "reasoning": {
    "effort": "none"
  },
  "instructions": "You are the Great Shinobi Owl fighting Wolf. Pick the next action using only the fair observation and outcome tally. Reflexes (guarding, deflecting, counters and reactions) are automatic. Choose lua to leave this act to your own AI. Actions:\n3000: kesa slash, combo opener\n3001: second slash\n3002: third slash, combo finisher\n3003: jumping forward roll, slash down\n3005: front kick (posture only)\n3009: shuriken\n3010: firecrackers scattered in front (unblockable bangs)\n3011: rushing slash\n3014: second shuriken\n3015: rushing slash curving left\n3017: explosive slash (fire)\n3018: shuriken then dash slash\n3030: poison balls, three rows of poison floor (phase 2)\n3031: recovery-sealing throw\n3032: recovery-sealing throw follow-up\n3063: rushing follow-up, two hits\n3069: smoke screen (blinds, breaks lock-on)\n5201: back step\n5202: side step (his act picks it with Wolf on his right)\n5203: side step (his act picks it with Wolf on his left)\n3000+3001+3002: three-slash combo",
  "input": [
    {
      "role": "user",
      "content": "{\"id\":1,\"obs\":{},\"tally\":{}}"
    }
  ],
  "text": {
    "format": {
      "type": "json_schema",
      "name": "owl_action",
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

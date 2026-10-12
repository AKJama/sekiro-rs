# Boss brain roadmap

Why the boss brain is changing, what each change is for, and how we will know it worked.
Read this before changing the brain; add a dated entry when a step lands or a hypothesis is settled.
Mechanics live in `docs/BRAIN.md`; the evidence fix is in `docs/BRAIN-LEARNING.md`.

## The problem (2026-10-12)

After 57 fights against Genichiro (c7100) with the local checkpoint chooser, the player still took the first deathblow almost every fight and was learning faster than the boss.
The Owl (c5060) had felt far stronger after a similar number of fights.
Our reading: the Owl's brain settled early into a good local minimum and kept exploiting it, while Genichiro settled into a bad one and never left it.

What the evidence showed:

- 3025 (his jump attack) was Mikiri-countered 29 times in 60 uses (48%), under the old 60% veto, so it was never held back.
- 3076 whiffed 115 of 119 uses and still kept about half its weight.
- Only whiffs counted as failures; Mikiri, jump counters, deflects and full guards did not.
- The coach saw only the last 3 fights, so a move failing across the whole run was invisible to it.
- The checkpoint bandit was learning (about 160 updates), but it only switches plan sets at checkpoints, so it could not stop a single bad move.
- The checkpoint design came from the paid Decisions API; a free local chooser has no reason to decide so rarely.

## Principles

- Anything that is a counted fact is computed by deterministic code from the memory and logs, never by a model: outcome rates, the moves table, trends.
- The models only judge: the coach turns facts into a plan and lessons.
- Judge a move by its failure rate relative to the other moves available in the same situation, after a small minimum sample (3 uses), not by a fixed count or percentage.
- Every outcome that is bad for the boss is a failure: Mikiri and jump counters, deflects, full guards, whiffs, dodged sweeps, damage taken.
- Reward what kills the player: win and win fast; break Wolf's posture and convert the break with a hit (a break that is not converted is mildly bad); then damage per minute; then take little damage; never reward stalling.
- Explore hard early, exploit what trends well, keep a small exploration floor so the boss never becomes predictable.
- Lua keeps reactions and counter callbacks; the learned layers only choose among the attacks Lua would otherwise pick from.
- No second coach model: a review mode of the same coach does the "manager" step.

## The layers

| Layer | Decides | How often | Cost |
|---|---|---|---|
| Lua AI | Reactions, counters, spacing, fallback acts | Every frame | Free |
| Per-move bandit | The next attack or plan in the current situation | Every 2 attacks | Free, local |
| Checkpoint bandit | Which plan set to play | At checkpoints | Free, local |
| Coach | Plans, priors, bans, confidence and exploration level | After each fight, every 30 s mid-fight | One model call |
| Coach review mode | Is the strategy working, lessons, what to test next | Every 5 fights and when the trend stalls | One model call |

## Steps

| Step | What | Status |
|---|---|---|
| 1 | Classify every outcome; per-attack evidence after 3 samples; remove automatic bans; memory version 4 | Done in e964560 (`docs/BRAIN-LEARNING.md`) |
| 2 | Per-move contextual bandit; break capitalisation reward; coach confidence and exploration controls; `--brain-moves local\|off` | Done in e964560; offline replay: countered moves under 5% from fight 4 |
| 3 | Deterministic moves table and 10-fight trend digest in the coach input; the coach prompt audited to match | Done in e964560 (coach input budget now 8 KB) |
| 4 | Coach review mode with accumulated lessons | Done in e964560 |
| 5 | `--brain-stage` presets for Genichiro, like the Owl's, from the frozen snapshots below | To do |
| 6 | Playtest A/B: new brain against the frozen ones, same player | To do |

## Frozen brains

Snapshots are copies; play from them only through `--brain-stage` (which works on a temporary copy), never by copying them back into `logs/memory/`.

| Snapshot | What it is |
|---|---|
| `logs/datasets/owl-2026-10-11-golden`, `-alive`, `-outsmarted` | The Owl presets the player recorded (`--brain-stage early\|alive\|outsmarted`) |
| `logs/datasets/owl-2026-10-12-before-move-bandit` | The strong Owl brain as it stood before steps 1 to 4 |
| `logs/datasets/genichiro-2026-10-12-local-minimum` | Genichiro after 57 fights, stuck in the bad local minimum |

## How we will know it worked

- Offline: in the replay experiment, a move the synthetic player always Mikiri-counters falls below about 5% of the boss's attacks within 3 to 5 fights, probes still happen, and the boss still fights coherently.
- In play: starting from the frozen Genichiro, his use of 3025 and 3076 drops within a few fights, and the player's first-deathblow rate falls over the next 10 to 20 fights.
- The Owl does not get worse: the new brain started from the frozen Owl keeps or improves his kill time.

## Open questions

- Does the per-move bandit escape a bad local minimum on its own, or does the coach's review need to raise exploration first?
- What context buckets are coarse enough to learn fast but fine enough to find the quick punisher after a break?
- How often should review mode run when the trend is improving: is every 5 fights wasteful then?

- Is the local chooser missing the "intuition" a model brings when picking plan sets? Test: Genichiro from scratch with `--checkpoint decisions` and its own memory folder (`logs/memory-decisions`), about 40 fights, against the 57 local-chooser fights.
- Later: make the coach an agent with read-only query tools over the deterministic tables (top moves by posture damage, most deflected or Mikiri'd moves, best combos, least used), so it can investigate instead of reading one fixed digest.
- Coach input grew from about 6 to 9 thousand tokens a call on 2026-10-11 to about 19 thousand on 2026-10-12 (about 40% cached); find out how much is the game's message and how much the CLI's own prompt.

## Log

- 2026-10-12: Problem found from the player's 57 Genichiro fights; steps 1 to 4 started; Owl and Genichiro brains frozen.
- 2026-10-12: Steps 1 to 4 landed in e964560; the player is running the Decisions-from-scratch test on the old exe (`target/play`); the new brain is in `target/play-next`.

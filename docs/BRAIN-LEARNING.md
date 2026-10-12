# Attack learning investigation, 12 October 2026

The main defect was uneven attack feedback, below the checkpoint chooser.
The checkpoint bandit had learned; it could not retire a Lua counter animation that every set continued to permit.
The current Decisions checkpoint path does not read coach notes or attack evidence, so the claimed advantage from reading those notes is not supported by its request builder.

## Evidence and limits

All source evidence under `logs/` was read only.
The initial c7100 player read contained 40 fights and 36 boss wins; the shared file contained 39 fights and 36 boss wins.
These totals do not establish whether Wolf took the first deathblow on every fight, since historical records do not retain individual boss mark losses.
The live player file advanced to 44 fights during investigation, so subsequent totals must not be mistaken for the initial 40-fight snapshot.
The stable report `brain-session-1791827648012.md` covers seventeen completed Genichiro fights, not all forty.

`brain-*.jsonl` is provider-call telemetry, including usage, latency, status and a chosen value when applicable, rather than the simulation's per-exchange `outcome` log.
Session reports aggregate actions across the session and list individual fight metrics, but do not associate each attack outcome with a fight.
Consequently the supplied files cannot reconstruct exact move success/failure counts for each of all forty fights, nor conclusively attach every reported jump to a specific Lua callback.
New fight records and reports now retain per-phase action counts for future comparisons.

## What chooses the next attack

| Layer | Choice | Previous evidence influence | New evidence influence |
|---|---|---|---|
| Lua battle goal | Weights ordinary acts by distance, state and effects | Unchanged native weights when no plan owns a slot | Still Lua-owned; offered plans override eligible ordinary slots |
| Lua reactions and Kengeki | Selects legal counter callbacks before an ordinary plan slot | Counter/deflect rates and hits after three uses; coach counter multipliers | Same recent attack signal as plans; explicit coach weights and avoids still apply |
| Coach game plan | Opener, named sets, weighted plans, avoids, counter weights and break responses | Between-fight and mid-fight reasoning sees counts and its previous notes | Also receives mature recent rates and instructions to encode actionable notes in executable fields |
| Local checkpoint bandit | Selects a named set, equivalent to keep when already active | Four-count coach prior plus discounted reward posterior | Also scores the set's constituent attack quality and honours a matching when hint |
| Local plan sampler | Samples a plan within the active set at each eligible ordinary slot | Coach weights, temperature, current-fight repelled streaks and untried bonus | Also persistent per-player attack multipliers, weakest component for a chain |
| Break response | Samples the coach's immediate on_wolf_break response | Separate plan-use and contact evidence | Same per-attack weighting when alternatives are offered; explicit response policy remains |

`BrainDriver::before_think` installs counter weights, and `pick_from_active` supplies ordinary planned attacks.
`ask_if_ready` selects checkpoint sets; it does not select each animation or override a reaction slot.
`GamePlan::validate` removes explicit avoided plans, and `apply_boss_bans` extends shared exclusions to offered chains.
The initial three main Genichiro sets all gave `3016` weight 5 out of 10, so switching between them still left half their base plan probability on the same move.
The saved `counter_weights` mostly remained 1, independently of those set changes.

## Counter and jump evidence

The initial shared c7100 action counts included:

| Animation | Uses | Clean-hit flag | Deflected | Whiffed | Punished | Mikiri | Jump counter |
|---|---:|---:|---:|---:|---:|---:|---:|
| 3025, perilous thrust | 60 | 19 | 1 | 11 | 3 | 29 | 0 |
| 3062, back-step thrust | 69 | 3 | 1 | 60 | 1 | 5 | 0 |
| 3067, perilous finisher | 11 | 2 | 0 | 3 | 7 | 0 | 6 |
| 3076, Kengeki callback 14 animation | 119 | 1 | 1 | 115 | 0 | 0 | 0 |

Flags can overlap within a multi-hit animation, so columns are not mutually exclusive counts.
The seventeen-fight report alone records 3025 with 21 uses, 13 hit flags, eight counters and zero whiffs, and 3076 with 42 uses, zero hits and 39 whiffs.
The earlier nine-fight report records 3025 with 13 uses and eleven whiffs before dedicated counter recording, so those old whiffs cannot retrospectively be relabelled Mikiri.

`Tally::record(Countered)` marks the latest boss exchange; `Outcome::whiffed` intentionally excludes any successful counter.
This separation is correct, but the old all-whiff rule could never trigger on an accurately recorded counter.
The original rule in this checkout required nine all-whiff uses, while a later coach-written exclusion cites ten out of ten planned whiffs.
Neither was a prerequisite for rationally reducing a move's probability.

The checkout already had a three-use, 60% counter-rate veto and plan failure bans; it was not wholly blind to Mikiri.
However, 3025's aggregate counter rate was 29/60, below that veto threshold, and the prior Lua counter multiplier ignored whiffs, guards and punishment.
Starting from 29/60, eighteen additional perfectly countered uses would be needed to reach the former 60% veto threshold.
For 3076's actual saved counts it was approximately 0.5000 despite 115 whiffs in 119 uses.
The new bounded legacy prior yields a 0.01 multiplier immediately, without pretending those whiffs were counters.
Ordinary sampling previously had no persistent attack-level multiplier, and its streaks reset at each fight.
Plan-span counts also include reactions: the saved 3016 plan has ten countered exchanges while the actual 3016 animation has zero.
That is evidence about what happened while a plan was active, rather than proof that the draw slash itself was Mikiri-able; the new weights use actual animation keys.
Shared counts were preferred to personal counts in the previous counter evidence lookup; recent personal observations now dominate bounded transfer from other profiles.

The real-data replay forces 3025 and 3076 separately through the headless engine and logs the actual clips and contact/counter attribution.
Forcing a single animation is not replaying the complete conditional Lua Kengeki callback, so it cannot prove the player's entire jump sequence from aggregate reports.
The callback map and observed counts identify 3076 and 3025 as the relevant jump/counter candidates; only 3025 has substantial recorded Mikiri evidence in the supplied forty-fight data.

## The bandit's actual learning

The initial profile contains approximately 162.10 discounted arm updates, with 36.56 success mass and 125.54 failure mass across ten contexts.
The largest context, wolf_posture_25, contains 55.81 effective updates, start contains 23.49, and wolf_posture_50 contains 22.25.
These are decayed posterior masses, not exact historical choice counts.
The stable seventeen-fight report records 119 choices and 118 settled rewards; forty rewards are zero and the mean settled reward is 0.278.
Another short report records five settled choices, four zero rewards and mean 0.013.

At wolf_posture_25 the main set posterior means moved from the default 0.5 to roughly 0.234 through 0.260.
The start counter_pressure arm has mean about 0.382 using the latest coach prior 0.65, and the wolf_posture_50 advance arm about 0.526 with prior 0.65.
The posteriors therefore moved meaningfully, but many sets remained similarly poor and retained common attacks and counter permissions.

Reward is clamped Wolf HP/posture progress and breaks minus boss HP/posture damage, counters and fully deflected plan uses.
Activity gates reject void fights rather than learning from an idle player.
The four-second reward horizon can end before an attack or counter resolves; zero reward also cannot distinguish different failure severities.
The persistent per-attack signal now updates at exchange settlement regardless of a checkpoint reward window, and directly changes attack weights and set quality.
The reward formula itself is unchanged.

## Decisions comparison

In coach mode `BrainDriver::ask_if_ready` constructs a brief containing why, current set, boss HP/posture, Wolf HP/posture and distance.
It explicitly supplies an empty tally and no memory.
`HostedBrain::submit` sends that brief, and `chooser_body` offers set names/descriptions and keep.
Coach notes, attack counts and recent habits are absent; there is no numeric guarantee about a model's semantic interpretation of a description.
Direct `--brain decisions` is a different path that does receive fitted tally, memory and habits and chooses attacks, rather than coach sets.

Owl's retained legacy profile has 41 fights and 40 boss wins, with recent kill times averaging 50.4 seconds, ranging from 22.6 to 80.3 seconds.
Its outsmarted archive at 21 fights has ten recent wins averaging 39.8 seconds, with one 18.3-second kill.
Its current local profile has 63 fights and 61 wins, with recent wins averaging 44.3 seconds and ranging from 22.5 to 69.1 seconds.
These are different snapshots, player states and engine revisions, not a controlled A/B test.
They do not establish that Decisions consistently killed Wolf under twenty seconds or that the local chooser made Owl less lethal.
The offline experiment establishes a specific feedback defect and recovery, not equivalence to an uncalled hosted model.

## Offline regression experiment

The deterministic experiment uses 64 seeds, six fights and thirty sampled action slots per fight.
Wolf always Mikiri-counters 3025 and is hit by the alternative 3000; the coach provides a deliberately thrust-heavy 9:1 set and a slash-heavy inverse set.
There are no game-data mutations, coach calls or provider requests.
The local experiment deliberately gives every reward window zero to test the per-attack signal's independence from bandit rewards.

The compared pickers are a counter-blind weighted baseline, the previous uncapped current-fight streak sampler, the persistent outcome sampler, and the outcome-aware local chooser plus sampler.
This isolates selection components; it does not emulate every historical driver ban or fabricate a Decisions answer.
Persistent sampling must fall below 5% thrust use by fight two, and both learned paths must still choose it during subsequent fights.

| Picker | Fight 1 countered uses / 1920 | Fight 2 | Fight 6 |
|---|---:|---:|---:|
| Counter-blind weighted baseline | 1714, 89.27% | 1743, 90.78% | 1689, 87.97% |
| Previous current-fight streak sampler | 467, 24.32% | 484, 25.21% | 468, 24.38% |
| Persistent outcome sampler | 200, 10.42% | 84, 4.38% | 65, 3.39% |
| Outcome-aware local chooser and sampler | 173, 9.01% | 24, 1.25% | 15, 0.78% |

Across fights two through six, the persistent sampler still probes the thrust 374 times and the local path 83 times out of 9600 slots each.
Thus the regression checks suppression and continuing exploration, rather than a hard ban.
Separate tests cover all recorded outcome kinds, changing player behaviour, bounded shared transfer, legacy versions 1 through 3, structured coach hints/preferences and the actual 3076 historical multiplier.
The controlled real-data replay records 3025's first contact at clip time 0.733 seconds and one successful Mikiri, with zero hits and zero whiffs for that exchange.
Forced 3076 instead contacts at 0.433 seconds and lands a clean hit in that replay; it is not itself recorded as the countered thrust.
This confirms why animation identities and full Lua sequence provenance must not be inferred from a session-level plan label.

See [BRAIN.md](BRAIN.md), "Recent per-attack evidence", for exact outcome severities, decay, safety minimum and exploration floors.
Test output and final validation are kept under `target/alt-fk/`.

Validation used `CARGO_TARGET_DIR=target/alt-fk`, three Cargo jobs and offline mode throughout.
`cargo test --offline --release -p sekiro-sim -p sekiro-brain-net -j 3 -- brain coach` passed 150 tests with one existing slow test ignored.
The final `--lib brain::` rerun passed all 80 tests after consolidating direct and layered memory migration and prioritising current failures during coach-budget trimming.
`cargo clippy --workspace --all-targets -j 3 -- -D warnings` passed after those changes, with Cargo networking disabled.
Only the two new Rust files were checked with rustfmt; shared files were not reformatted.
The relevant transcripts are `brain-checks.txt`, `brain-final-unit.txt` and `brain-clippy-final.txt` under the permitted target directory.

## Manual follow-up

Repeat a short c7100 session with a fixed player profile and consistent inputs, and inspect the new per-fight attack table for 3025 and 3076.
Check first-deathblow frequency separately from final boss wins and compare first-phase footage at the same player power and progression.
Verify that the coach encodes its notes as plan weights, avoids, counter weights and checkpoint preferences, rather than only changing prose.
Check whether Lua offers alternatives when the repeated callback occurs; its mandatory single-candidate and all-zero fallback branches remain outside plan selection authority.
Occasional probes are intentional, while explicit coach vetoes can remove a move where Lua permits alternatives.

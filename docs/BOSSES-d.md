# Group d boss fights, pass 2

Earlier sessions checked the ghost Monk and True Monk; the current continuation implements the sanctuary encounter.
True Monk's three marks and the Old Dragons opening are playable in the simulation, with remaining fidelity limits listed below.
No group-d encounter is claimed fully DONE under the every-mechanic and visual-verification definition.
Existing arena staging is documented in [ARENAS-d.md](ARENAS-d.md).
The earlier missing c5000 and Dragon model notes are stale: those GLBs are present on 12 October 2026.
No game data, research artefacts, existing logs or git state were changed.

## Corrupted Monk ghost, one phase

`--arena mibu-bridge` selects c5000, NpcParam 50001000 and boss entity 1500800 through the existing arena and placed registry.
NpcParam supplies 3513 HP, 990 posture, one deathblow mark and behavior variation 50001.
The selected think row runs the installed 500000 logic and battle goal from the Mibu Lua bundle.
The existing character loader uses c5000's own HKS, NPC behavior graph, skeleton, animation clips and TAE.
Hit capsules are read from AtkParam and the raw FLVER dummies, with no new guessed hit shapes or attack timings.
The Lua attack smoke initially failed in the actual arena with an attempt to index nil at `TimingSetNumber`.
The shipped battle goal chains a goal setter and a timing setter, so host goal setters now return their goal handle.
`SetLifeEndSuccess` also retains the script's explicit boolean and applies it on goal timeout, while goals without that setter preserve their existing timeout behavior.
With that correction the normal Lua run performs five attacks and lands three contacts on idle god-mode Wolf.

The shared Mikiri path initially rejected the Monk's counter because it required an attacker dummy on Wolf.
ThrowParam 15000190 instead specifies attacker dummy 0, defender dummy 235, Wolf offset 211 and defender offset 0.
The counter now selects the defender dummy from the Monk's raw FLVER and absorbs Wolf onto that frame, using the same ownership convention as existing deathblows and grabs.
The posture-breaking alternative 15000120 uses the same dummy ownership and follows this path too, but that alternative has not received a separate Monk test.
Act 3015's real warning bullet reports a thrust, and the timed forward step enters Mikiri row 15000190 for 90 posture damage.
Act 3005's warning reports a sweep, and two scripted jump presses produce the real HKS jump counter for 80 posture damage.
An ordinary naginata strike can be blocked by holding guard or deflected by pressing guard near contact, with positive player posture damage and the real attacker deflect response.
The guard test uses immortal Wolf rather than god mode, so the damage rules remain active.
The ghost row's loaded attacks contain no grab-start attack, and its BulletBehavior rows are warning or dummy behaviors rather than invented damaging projectiles.
This is an audit of the selected row, not a claim to have exercised every move in the boss's complete repertoire.

Scripted Wolf slashes break the boss's posture and the existing selector requests pair 15000180 and 15000181.
The simulation spends its single mark, reaches zero HP, and the actual map event sets defeat flag 9306 and removes the fog wall.
The graphical replay reveals that this is not a faithful execution pair: the requested defender a000_013700 is an empty TAE entry with no decoded HKX or alias, and the boss enters ordinary death before Wolf finishes.
The available a100_013700 belongs to another animation group and was not substituted without evidence that the ghost uses it.
The test intentionally checks map progression without asserting this currently unsupported pair as the correct native selection.
The test lowers posture to one before the breaking slash to keep this regression short, but the breaking hit, attack input, throw selection, paired animation and victory progression run through the real simulation.
The boss bar is the map's 905000 bar.
There is no invented second phase or get-up refill for this one-mark ghost fight.

`--brain scripted` now retains the character's own Lua AI with a diagnostic when no scripted plan table exists for that character.
Previously that option attempted to use the Owl's plan table and then failed when installing it on c5000.
The Mibu fixture exercises this fallback and asserts that no external planning driver is installed.
The four focused tests cover Lua combat, guard and deflect, both perilous counters, and single-mark removal and map victory.
The shared regression run includes all four tests and the existing Owl, Genichiro, brain tactics, attack power, posture recovery and perilous tests.
It passes 115 tests with one existing slow sweep ignored, in 68.41 seconds.

### Remaining fidelity gaps

This phase is not claimed fully DONE under the task's every-mechanic and visual-verification definition.
`GetHpLastTarget` still uses the unknown-method fallback rather than reconstructed AI target history.
The first reproduction also logged `IsInsideTargetRegion`, but another agent supplied installed-MSB region queries during this shared-tree session and the later Monk run no longer logs that method.
Event 11505814's numeric snap-seed counter still requires unsupported event-value increment and comparison instructions 2003[31] and 3[12].
The three-use illusion-disruption threshold and removal of resident effect 6023 therefore remain unimplemented.
No snap-seed inventory or use input was added.
Primary ForceAnimationPlayback and SetCharacterAIState staging instructions are not fully hosted by combat, so the entrance appearance animation and AI enable boundary are not claimed faithful.
The continuation now reads the sticky TAE 231 character message register, which also enables the True Monk phantom event sequence.
The paired execution test logs the existing bounded wall-clearance exhaustion and retains authored alignment, so complete wall and floor clearance during that pair is not proved.
The shared throw-space reservation, NPC airborne support, animation blending, bullet aiming and seeded random choices retain the approximations documented in [FIDELITY.md](FIDELITY.md), [AI.md](AI.md) and [DEATHBLOW.md](DEATHBLOW.md).
The reviewed replay renders the ghost opaque with normal character shading rather than the native illusion appearance.
The complete ghost execution selection and defender animation remain incorrect, as demonstrated by the second graphical replay.
The full attack repertoire and finishing camera sequence still require comparison with the installed game.
Production weather, particles and extra lighting are pass 3 work.

## True Corrupted Monk, three phases

The missing `cache/models/c5005.glb` is now present, and no further extraction was needed for this boss.
`--arena fountainhead-bridge` selects entity 2500850, c5000:50000000 and its own installed 500000 Lua goal.
The registry, animation donor resolution and raw AtkParam/FLVER capsule loading already supply the correct rows, so no duplicate EnemySpec or hand-authored attack table was added.

### Phase 1

The real c5000 model, skeleton, HKS, graph, TAE and root motion run on the staged bridge collision.
The first-phase arena test observes three natural Lua attacks and two contacts on idle god-mode Wolf, with no Lua or HKS errors and boss bar 905000.
The initial row supplies 2555 HP, 840 posture and three deathblow marks.
A scripted sword hit breaks posture and begins ThrowParam 15000000/15000001, with Wolf a211_510000 and Monk a000_012000.
Its TAE get-up applies 275000 and HP reset, producing 1916 maximum HP and 672 maximum posture from the original NPC stats and extracted rates.
The fight-through checks full HP after recovery and exactly one spent mark.
The short posture fixture sets remaining posture to one before the breaking hit and positions Wolf in range; it is not a claim of a natural unassisted kill.

### Phase 2

Event 12505961 sets 12505952, supplies AI command 1 and enables all three placed c5005 phantoms.
Each phantom runs its own installed Lua and c5000 donor HKS, graph, skeleton, clips and TAE, retaining its c5005 model and params.
The event host reads their SpEffects and sticky messages and routes the event's AI commands and replan requests independently.
The collision world is shared, and Wolf advances once while each phantom resolves its own authored attacks against that frame.
The natural Lua fight-through observes a000_003032, all three phantom attack requests and two contacts during the illusion sequence, without forcing the Monk's illusion action.
The map supplies their warp regions and command slots 10/20/30, and their a200_003011, a200_003006 and a200_003023 clips are observed.
The renderer reads the simulated phantom positions and clip clocks rather than leaving them at staged poses.
The second ordinary deathblow removes the next mark and plays a000_012000.

### Phase 3 and defeat

Event 12505962 supplies command 2 and EzState 20010 after the throw effect clears.
The installed HKS/AI path plays a000_020010 and a000_020015, whose animation effect selects the a100 set.
TAE effect 275001 and HP reset produce 2714 maximum HP and 840 maximum posture, with full HP after the transition.
Natural phase-three Lua attacks make two contacts in the fight-through.
The final pair uses ThrowParam 15000180/15000181 while preserving the active defender animation group, so a100_013700 plays instead of the empty a000_013700 entry.
Wolf's TAE 936 message 10 requests the map continuation, but the defender clip also owns message 40 at 4.733333 seconds.
The continuation now preserves that authored message before accepting EzState 20200 or ordinary HP-zero death, avoiding the reproduced premature interruption.
The test observes a100_020200, flag 12505956 and the enable of centipede entity 2500852, then defeat flag 9309 and zero remaining marks.
The renderer now applies the script's one-shot short warp to the Monk's animated dummy 260 and honors the explicit centipede 20000 playback despite its disabled ordinary animation state.
Request arbitration and that forced-playback precedence are inferred from the installed event and TAE timelines, rather than traced native scheduling rules.

### Checks and remaining gaps

The three-phase replay is deliberately ignored in the normal tier because it takes about nineteen seconds on this machine.
The counter fixture separately checks thrust 3015 and sweep 3005 in the a000 and a100 sets using Wolf's real HKS dodge, Mikiri and jump inputs, and passes all four cases.
It isolates the move set with the authored 200031 effect; the fight-through separately proves the event-driven transition.
The counter fixture is also in the slow tier because it loads four complete phantom casts.
The first version of that fixture incorrectly attempted to jump straight into a late event state; its failed offset assertion was replaced with explicit move-set isolation.

`GetHpLastTarget` remains an unknown-method fallback, so AI target history is approximate.
The full phase-three breath, terror, grab and projectile repertoire and every guard/deflect contact are not certified by these tests.
The shared status engine now has terror support from another agent, but this session does not claim a Monk terror-threshold replay.
The optional tree/plunging second-phase deathblow, complete native entrance and finishing camera are not verified.
The phantom renderer uses the existing threshold visibility behavior for TAE opacity, rather than smooth native illusion shading.
The final pair still reports bounded wall-clearance exhaustion and retains authored alignment.
The real-game comparison needed to close these limits has not been run.
Production weather, particles and extra lighting remain pass 3 work.

## Divine Dragon and Old Dragons, two encounter phases

All four requested GLBs and animation bundles are present, and there is no current missing-extract blocker to report.
This encounter is not playable end to end and is not claimed DONE.
The installed-data tests now include a normal-start sword replay of the entire Old Dragons opening and an isolated native Dragon finishing pair.

### Old Dragons

The opening bar is the hidden c5300:53000000 controller, entity 2500810, with 1000 HP in the extracted NpcParam.
The fighting placements use c5300:53000010, 53000011 and 53000012, each with 320 HP and their own selected animation group.
All twenty-five placed fighters run separate shipped Lua, HKS, skeleton, root motion, TAE, AtkParam hit shapes and params against a shared Wolf.
The supporting c5320 and three opening c5310 trees retain separate character and event state.
MSB entity groups route the script's AI enables and command 30 into the cast.
The selected target advances Wolf once, and the other actors resolve their contacts against that same player frame.
The renderer follows each actor's simulated pose and clip clock, with map-authored masks and visibility.
Event 12505803 applies SpEffect 3530020 to the controller after a placed white dragon reaches HP 1; its extracted changeHpRate is 4 percent.
The script then waits for death and revival, changes the actor's effects and masks, increments event value 12505828 and uses character instruction 2004[11].
Numeric comparisons 3[12], increments 2003[31] and flag-range conditions now run against the actual event-flag bitfields.
Each 3530020 application is a separate pulse, including several white deaths in one frame.
Later revivals pass the script's ten-death comparison and receive its black-dragon masks, quarter-posture effect 3530030 and half-HP effect 3530501.
Those maximum-stat effects are read from params and compose rather than replacing each other.
The 2004[11] instruction sets TARGET_EVENT to the actual c5320 actor for the Lua queries, rather than inventing shared damage.
Resurrection eligibility is inferred from the death clip's TAE 932/103, with the refill and a000_020000 request at the death-idle boundary.
The native engine's exact scheduling and use of action 103 are not recovered, so that policy remains an explicit approximation.
The first complete sword replay observed 113 outgoing contacts, two incoming contacts and 23 hosted revivals, depleted the controller through the actual 4% effects, and entered c5200 with 3600 HP and event value 23.
It used god-mode Wolf, movement and attack inputs from the normal sanctuary start, with no actor-position, boss-health, posture, AI or event-flag edits.
The separate placed-row probe still checks its own Lua attacks and contact.
The opening has no deathblow marks on its aggregate bar.
The HUD keeps this bar visible independently of the hidden controller's character fade.
The full replay is explicitly slow and ignored by default.
Old Dragons are not fidelity-DONE: poison status coverage remains incomplete, the opening tree grapple and area execution are not hosted, and native resurrection scheduling remains inferred.
No fabricated summed-health rule, timed victory or single-dragon replacement was introduced.

### Divine Dragon

The retry probe raises authored retry flag 12505882, allowing the existing map event to hand over to entity 2500800 and its c5200:52000000 Lua.
The extended probe runs the actual Dragon attacks, TAE bullets and contacts, while attempting the authored tree and lightning interactions.
Natural entry from defeated Old Dragons is independently proved by the sword replay.
Five c5310 tree helpers now have separate Lua/HKS state, event commands, dummy-relative character warps and animated object attachments.
Their dynamic wire points read the active SpEffect's variation, target dummy and landing dummy ids from params.
The current installed tree wire effect uses variation 53100 and dummy 600 for both endpoints.
Pass 1 omitted 12505891, 12505894, 12505911, 12505916, 12505921 and 12505926, which now run with all five placed slots where applicable.
These are the actual rise, retreat, lightning-target selection, warning and charging events, not a separate boss state machine.
Map-authored shots resolve their BehaviorParam from either c5200 or c5310 and their origin from the named source's posed dummy.
Random-flag selection is reproducible and uniform over the authored range, with the native random generator and scheduling still inferred.
The helpers' 531001 logic waits in Main and reacts to map commands in Interrupt.
The shared Lua host now raises the shipped INTERUPT_EventRequest when command slots change, rather than exposing only GetEventRequest.
The extended replay observes all five helpers request their own 20000 rise action and enter the 5025 hold state, with no Lua or HKS errors.
The imported motion is valid: c5310 a000_020000 uses the installed a100_003000 HKX alias while retaining its own TAE.
However, no active effect carrying chrWireVariationNo appears during the sixty-second replay, and the exported grapple table contains no corresponding sanctuary tree points.
The unresolved engine path is the authored tree's wire-target activation from object and character state, with 2005[19] still skipped.
The exact role of that opcode in wire registration has not been recovered, so it is a candidate dependency rather than a demonstrated sole cause.
The current character-effect wire host therefore publishes no live target in that replay, which records zero grapple activations, zero lightning charges, zero reversals and unchanged Dragon HP 3600.
The observed Dragon portion still performs ten real attacks, emits 36 authored bullets and lands eight contacts.
This precise interaction gap blocks natural lightning damage and the subsequent collapse/head-traversal sequence.
The only Dragon finishing ThrowParam is 15200090, with Wolf a233_510900, defender a000_012900 and defender absorption dummy 230.
The installed a000_021000 collapse exposes TAE 932 action unlock 104, which now gates this row separately from ordinary posture-broken selection.
Range and absorption use the posed eye dummy, rather than the Dragon's distant root position.
An isolated fixture requests the installed collapse, places Wolf at that dummy and presses attack, and both native paired clips play.
It reaches effect 201000, the map's HP-zero and 9.7-second waits, victory command 2003[74] and defeat flag 9310.
The fixture proves the native finishing pair and map ending, not natural traversal from lightning damage to the eye.
The remaining full lightning transfer, reversal, low-HP collapse loop and head traversal must still pass the natural encounter replay before this phase can be called playable end to end.
The shared generic lightning-return path still substitutes a ten-metre beam for missing weapon dummy 10141, which is not a verified Dragon return path.
During the opening the supporting c5200 model is hidden while the map disables its animation, preventing its waiting head from appearing among the Old Dragons.
That visibility mapping is inferred from the staged event states; complete native character draw-flag hosting remains open.
No fabricated reversal damage, instant victory or automatic final deathblow was added.

### Sanctuary continuation verification, 12 October 2026

New engine files are `crates/sim/src/old_dragons.rs`, `dragon_trees.rs`, `dragon_finish.rs` and `event_values.rs`.
Small additive hooks are in `duel.rs`, `arena.rs`, `events.rs`, `lib.rs`, `lua_ai/runtime.rs`, `arenas/depths_palace.rs`, `arenas/scene.rs`, `crates/game/src/arena_d_scene.rs` and `crates/game/src/play.rs`.
The existing `crates/sim/tests/it/divine_dragon.rs` contains the four focused tests and keeps its existing module registration.
The completed Monk source and test files were not edited.
The installed MSB, EMEVD, params, Lua bytecode, skeleton dummies, TAE and animation aliases provide the encounter ids and mechanics above.
FwLo's MIT notes remain a read-only cross-check of character identities, animation groups and the special Dragon finishing row; no FwLo code was copied.

```powershell
$env:CARGO_TARGET_DIR='target/alt-d'
$env:CARGO_NET_OFFLINE='true'
cargo test --release -p sekiro-sim --test it -j 3 divine_dragon -- --include-ignored --nocapture --test-threads 1
cargo test --release -p sekiro-sim --test it -j 3 -- divine_dragon_retry owl genichiro brain_tactics attack_power posture_recovery --nocapture
rustfmt --edition 2024 --check crates/sim/src/event_values.rs crates/sim/src/old_dragons.rs crates/sim/src/dragon_finish.rs crates/sim/src/dragon_trees.rs crates/sim/tests/it/divine_dragon.rs
cargo fmt --all --check
cargo clippy --workspace --all-targets -j 3 -- -D warnings
cargo build --release -p sekiro-game -j 3
```

The first combined Dragon verification passes all four tests in 114.52 seconds, including both slow fixtures.
The strengthened final command `cargo test --release -p sekiro-sim --test it -j 3 divine_dragon -- --include-ignored --test-threads 1` passes all four fixtures in 120.29 seconds.
It additionally asserts white-to-black conversion, all five native tree holds, attached objects and absence of actor Lua errors.
The regression after the event-request interrupt change passes 100 tests, with two slow cases ignored, in 43.47 seconds.
The new engine and Dragon test files pass direct rustfmt checking.
Workspace formatting inspection reports shared and concurrent-file differences; those files were not globally reformatted or reordered.
Workspace/all-target Clippy initially encountered the other agent's private ape HUD types and redundant Sword Saint test conversions, then passes in 11.95 seconds after their owners resolved them.
The release game build initially fails at the private ape HUD type used by `arena_c_scene.rs:84`, then passes in 3 minutes 10 seconds on retry.
After the source-verified empty-object fix, workspace Clippy passes again in 13.82 seconds and the release game build passes in 3 minutes 15 seconds.
The subsequent HUD and waiting-model changes compile in the pre-capture release game build taking 2 minutes 38 seconds.
The final release game build after graphical review and the small formatting correction passes in 2 minutes 26 seconds.
The latest workspace Clippy run and two retries after waiting exit 1 at another agent's `fight.rs:921` hp_damage argument-count lint.
That file was left untouched under the shared-tree instruction despite the earlier successful workspace checks.
Those unrelated files were left to their owners throughout.
Earlier test builds temporarily encountered concurrent `headless_ape.rs` and `sword_saint.rs` type errors, which disappeared on retry without editing them.
One overlapping test build hit Windows LNK1104 because the integration executable was still running; subsequent integration builds and runs were serialized.
Before graphical replay, saves were backed up read-only to `target/alt-d/save-backup-dragon-20261012/S0000.sl2`, preserving all 11928528 bytes.
The first graphical start exits 1 at `missing extracted sanctuary object o259900`.
Its source `cache/raw/obj/o259900.objbnd.d/o259900.flver` is only 336 bytes and has zero meshes, so the scene loader now verifies that source and skips drawing the empty helper.
That is a staging-loader fix for an authored empty object, with no invented mesh or asset extraction.
The corrected graphical replay exits 0 and the reviewed step-300 and step-600 captures show the real animated Old Dragons, Wolf's sword contact and the named 1000-HP bar without deathblow marks.
The separate `--play --arena` replay also exits 0, produces both captures and shows the same combatants and health bar in the reviewed step-600 image.
Its log records Wolf's AtkParam 5000010 hit for 80 HP and 30 posture, then two incoming AtkParam 53000130 contacts for 16 HP each.
The waiting Divine Dragon head is absent after the animation-state visibility fix.
These short graphical captures establish opening combat and presentation, while the complete hand-over is proved separately by the simulation replay.
They still show heavily washed-out materials, an excessively bright sky, sparse shadows and cloth restarts; production weather, particles and extra lighting belong to pass 3, and these captures do not certify visual fidelity.

```powershell
target/alt-d/release/sekiro-rs.exe --arena fountainhead-sanctuary --brain scripted --brain-memory off --god --script target/alt-d/dragon-arena-input.txt --shots 300:target/alt-d/dragon-arena-fixed-300.png,600:target/alt-d/dragon-arena-fixed-600.png --exit
target/alt-d/release/sekiro-rs.exe --play --arena fountainhead-sanctuary --brain scripted --brain-memory off --god --script target/alt-d/dragon-arena-input.txt --shots 300:target/alt-d/dragon-play-300.png,600:target/alt-d/dragon-play-600.png --exit
```

The original input file holds 180 idle frames, 90 forward frames, then eighteen repetitions of three attack frames and 37 guard frames.
Replay inputs, save backups and captures stay under `target/alt-d`.
No cache/re writes, git changes, existing-log writes, network or external model/API calls were used.

## Sources and changed files

The implementation is original Rust built on the repository's existing Owl and Genichiro paths.
FwLo's read-only MIT `target/ref-fwlo/enemy.md` and `enemies.md` identified the chained-goal-setter pitfall and corroborated the Monk's finishing-pair pattern.
`deathblow_groups.md` cross-checks Wolf's a211 group, while `boss_events.py`, `boss_scripts.py`, `boss_arenas.py` and `map.md` were read as event and placement cross-checks.
The actual ids, dummy ownership, attack parameters and map victory checks above come from this installation's read-only extracted params, layouts, FLVER, TAE and EMEVD.
No FwLo code or Lua bytecode was copied into tracked source or documentation.

The new test file is `crates/sim/tests/it/corrupted_monk.rs`, registered additively in `crates/sim/tests/it/main.rs`.
Small shared fixes are in `crates/sim/src/lua_ai/runtime.rs`, `crates/sim/src/duel.rs` and `crates/sim/src/brain/mod.rs`.
No duplicate EnemySpec or per-boss hit table was added because those already resolve from the staged arena and installed data.
Documentation changes are this file and the dated entry in `STATUS.md`.

## Commands and results

All Cargo commands use `CARGO_TARGET_DIR=target/alt-d`, offline dependency resolution and three build jobs.
`cargo test --release -p sekiro-sim --test it -j 3 corrupted_monk_ -- --nocapture` first reproduced the Lua crash, then the missing Mikiri, and subsequently passed all four tests in 7.72 seconds after the runtime fixes.
After graphical review narrowed the execution test to map progression, the final focused run passes all four tests in 3.65 seconds.
The final regression command below includes the strengthened non-god guard test and the scripted-controller fallback check.

```powershell
$env:CARGO_TARGET_DIR='target/alt-d'
$env:CARGO_NET_OFFLINE='true'
cargo test --release -p sekiro-sim --test it -j 3 -- corrupted_monk owl genichiro brain_tactics attack_power posture_recovery perilous --nocapture
cargo run --release -p sekiro-formats --example emevd_dump -j 3 -- cache/raw/event/m15_00_00_00.emevd 11505800 11505810 11505812 11505814
rustfmt --edition 2024 --check crates/sim/tests/it/corrupted_monk.rs
cargo fmt --all --check
cargo clippy --workspace --all-targets -j 3 -- -D warnings
cargo build --release -p sekiro-game -j 3
```

The regression passes 115 tests, with zero failures and one existing slow case ignored.
The event dump succeeds and the new test file passes direct rustfmt checking.
The final workspace formatting check reports only ordering of the shared integration-test and Lua modules; those tables were not reordered.
An earlier check also reported a concurrent MapRegions assignment, subsequently formatted by its owner.
Workspace/all-target Clippy passes with warnings denied; the first successful retry finishes in 46.37 seconds and the final check finishes in 19.16 seconds.
Its first attempt encountered another agent's temporary `play.rs` call to the not-yet-added `Duel::enemy_bar_marks`, which was left to its owner and then retried.
The release game build passes, taking 35 minutes 8 seconds for the fresh dependency build and 2 minutes 37 seconds for the final incremental build.
Saves were backed up read-only to `target/alt-d/save-backup-group-d-20261012-153710` before any graphical attempt.

### Graphical replays

Both corrected commands below exit successfully and create the requested captures.

```powershell
target/alt-d/release/sekiro-rs.exe --arena mibu-bridge --brain scripted --brain-memory off --god --lock-on --script target/alt-d/monk-idle.txt --shots 300:target/alt-d/mibu-idle-300.png,600:target/alt-d/mibu-idle-600.png --exit
target/alt-d/release/sekiro-rs.exe --play --arena mibu-bridge --brain scripted --brain-memory off --god --passive --break-posture --lock-on --spawn 160.8627,-395.04,1427.3798,0,0 --enemy-distance 1.6 --script target/alt-d/monk-finish.txt --shots 180:target/alt-d/mibu-finish-180.png,600:target/alt-d/mibu-finish-600.png --exit
```

The idle input is `900 0 0 -`, and the finishing input is twenty repetitions of `1 0 0 attack` followed by `39 0 0 -`.
These original replay inputs and the PNGs stay in `target/alt-d`.
The initial finishing invocation used `--spawn 160.8627,-395.04,1427.3798,0` and exited 1 because the shared parser requires the fifth pitch value; the corrected invocation above passes.
The idle capture at step 300 shows the textured Monk and naginata standing opposite Wolf in the actual Mibu arena, with the named one-mark bar and HP 3513.
The game logs all 223 Monk rig bones bound, all 13 Monk cloths simulated, and authored attacks 3035, 3036 and 3037 landing through AtkParam 50001351 and 50001352.
The step-600 idle capture shows Wolf launched down the adjacent passage, with the Monk still visible at a distance; this does not prove faithful knockback or fog collision through every reaction.
The finishing capture at step 180 shows Wolf in a211_511700 but the Monk already absent and reporting a000_010001 at HP 0, confirming the unsupported defender clip and premature death.
The step-600 finishing capture has no boss bar and Wolf has recovered control, proving presentation of the current map ending rather than a correct native execution.
Ghost opacity and the finishing sequence are explicitly rejected as fidelity-complete visual results.

## Continuation verification, 12 October 2026

Continuation source files are `crates/sim/src/true_monk.rs` and `crates/sim/src/monk_phantoms.rs`, with small scoped hooks in `duel.rs`, `arena.rs`, `lib.rs`, the TAE message runtime and the existing `crates/game/src/arena_d_scene.rs` support renderer.
The two boss test files are `crates/sim/tests/it/true_monk.rs` and `crates/sim/tests/it/divine_dragon.rs`, registered additively in `main.rs`.
The continuation preserves the ghost work and updates this document and `STATUS.md`.
FwLo's `enemy.md` and `enemies.md` cross-check animation-group effects, phantom command slots and sticky message behavior; `deathblow_groups.md` corroborates the a211 Monk pair family.
The implementation and phase numbers above come from the local params, TAE, MSB and EMEVD, without copying FwLo code.

All commands below use offline dependencies, `target/alt-d` and three build jobs.
The final combined group-d run passes all five tests in 25.67 seconds, including both ignored Monk fixtures, the separate phantom-contact assertion and the strengthened Dragon-contact assertion.
An earlier natural three-phase-only run passes in 18.80 seconds, before the strengthened centipede-enable assertion.
The two Dragon diagnostic tests pass in 2.96 seconds, observing actual attacks and contacts rather than certifying a complete encounter.
Workspace/all-target Clippy passes with warnings denied in 15.01 seconds after the owners resolved the concurrent monkey warning and headless-ape compile error.
A subsequent final check encounters another agent's temporary private-interface errors in `crates/game/src/ape_hud.rs`; that file was left untouched for its owner.
The first release game build passes in 3 minutes 38 seconds, and the final incremental release game build passes in 3 minutes 23 seconds.
Direct rustfmt checking passes for all four group-d source/test files listed in its command.
Workspace formatting inspection reports shared and concurrent-file formatting differences, which were left untouched under the shared-tree rule.
The initial regression run reports 95 passed, eight failed and three ignored in 50.72 seconds, with the failures confined to Owl phase-two, wall-grab, fade and sweep tests.
After the required wait, the narrow retry passes all eleven Owl phase-two, wall-grab, fade and sweep tests in 5.75 seconds on the updated shared tree; no unrelated file was fixed by this session.

```powershell
$env:CARGO_TARGET_DIR='target/alt-d'
$env:CARGO_NET_OFFLINE='true'
cargo test --release -p sekiro-sim --test it -j 3 true_monk -- --include-ignored --nocapture
cargo test --release -p sekiro-sim --test it -j 3 divine_dragon -- --nocapture
cargo test --release -p sekiro-sim --test it -j 3 -- true_monk owl genichiro brain_tactics attack_power posture_recovery divine_dragon --nocapture
cargo test --release -p sekiro-sim --test it -j 3 -- owl_phase2 owl_grab::grab_near_a_wall_does_not_clip owl_warp::wolf_cannot_hit_him_while_he_is_faded_out perilous::jump_kick_on_the_owls_sweep_takes_eight_times_the_posture
cargo clippy --workspace --all-targets -j 3 -- -D warnings
cargo build --release -p sekiro-game -j 3
rustfmt --edition 2024 --check crates/sim/src/true_monk.rs crates/sim/src/monk_phantoms.rs crates/sim/tests/it/true_monk.rs crates/sim/tests/it/divine_dragon.rs
cargo fmt --all --check
```

Saves were copied read-only into `target/alt-d/save-backup-group-d-20261012-170758` before graphical runs.
Both replay commands below exit successfully with scripted guard input `720 0 0 guard`, real Lua retained by the scripted-option fallback and brain memory disabled.

```powershell
target/alt-d/release/sekiro-rs.exe --arena fountainhead-bridge --brain scripted --brain-memory off --god --lock-on --script target/alt-d/true-monk-continuation-input.txt --shots 300:target/alt-d/true-monk-continuation-300.png,600:target/alt-d/true-monk-continuation-600.png --exit
target/alt-d/release/sekiro-rs.exe --play --arena fountainhead-bridge --brain scripted --brain-memory off --god --lock-on --spawn -77.12,156.8,96.4,0,0 --enemy-distance 2.4 --script target/alt-d/true-monk-continuation-input.txt --shots 180:target/alt-d/true-monk-close-180.png,360:target/alt-d/true-monk-close-360.png --exit
```

The staged-start replay logs real blocked AtkParam 50000010 contact at tick 527 and the correctly named three-mark bar, but the doorway fog heavily obscures the Monk.
The reviewed close replay at tick 180 shows the textured c5000 model and naginata in a000_003006, Wolf's actual block reaction and the three-mark bar on the bridge.
Its debug positioning is explicit and does not verify native approach or entrance choreography.
Washed-out bridge materials, fog opacity and cloth-reset warnings remain visible limitations.
No graphical phase-two, phase-three or centipede finishing capture is claimed by these first-phase replays.
No cache/re, existing logs, git state, network or external model/API calls were used.

The final combined command is `cargo test --release -p sekiro-sim --test it -j 3 -- true_monk divine_dragon --include-ignored`, and all five tests pass.
The final Clippy retry still reports the same `ape_hud.rs` private-interface errors after the required wait, despite the earlier successful check.
The successful game build preceded that concurrent HUD edit.
The final `cargo build --release -p sekiro-game -j 3` retry exits 1 because the private `ape_hud::Panel` type reaches the public update system registration in `arena_c_scene.rs:84`.

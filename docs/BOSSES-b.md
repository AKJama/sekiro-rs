# Group b fight pass

## Continuation, 12 October 2026

This section supersedes the previous session's gap list below.
Guardian Ape's ordinary headless pair now exposes the separate front continuation 15100509, plays a100_012090, sends message 30 and reaches the map's defeat flag 9304 without restarting an ordinary pair.
The scoped host drops the waterfall's stale phase-one 20200/710203 request after the headless think switch, preserving the authored finishing pair.
The ordinary deathblow cannot reopen while that false corpse is waiting for its continuation.
TAE 233 now retains its authored draw-mask changes, including the decapitation and revival head groups, and the renderer applies them to both simulated apes.
The companion's initial row mask is restored when its simulation resets.
The installed-data replay still covers real Lua contacts before and after the decapitation, the twelve-second map wait, a000_020000 revival, 3872 HP and 1254 posture.
The scream now applies terror through SpEffectParam's legacy registIllness field, with resistance from CalcCorrectGraph 111 and recovery from TentativePlayerParam ResistRecoverPoint_Desease_Player.
The actual scream TAE emits Bullet 51000580 with SpEffect 9220, rather than the separate 9216 test attack.
At full buildup the source effect's changeHpRate supplies the fatal HP change and replaceSpEffectId supplies the status indicator.
The near/far scripted replay proves terror buildup and death at four metres, and survival outside the ten-metre bullet radius at twenty metres.
The HUD shows the terror percentage.
The resistance dispatcher, equipment modifiers, terror prevention tools and exact native status timing remain incompletely recovered.
Sword action 3000 in the headless group has tested ordinary guard and timed deflect contacts.
Sweep action 3009 has a tested authored perilous warning and jump counter.
Living action 3014 emits the installed dung Bullet 51000290 with poison effect 3510121.
That test proves the TAE shot and payload, not a natural aimed hit.
Forced low-grab fixtures at 1.6 and 3 metres did not produce a paired contact, so neither grab family is claimed verified.
The corresponding Wolf a230_600200 and a230_600380 clips and NPC throw rows 25100000/25100100 are present, so this is an unresolved contact/throw-host investigation rather than a missing extraction.
The Loaded Spear centipede pull and firecracker-specific fear are still not implemented as playable equipment interactions.
PlayerEnv currently exposes the basic unlocked styles and fixed weapon categories, without a data-driven prosthetic inventory/equip selector for those interactions.
Repeated sword-deflect collapse, every attack variation and the back finishing continuation remain unverified.
Guardian Ape's two phases and finishing continuation are playable, but the strict every-attack/every-counter DONE definition is not met.

### Headless Ape in apes-burrow

The main ape retains entity 1700850, NpcParam/think 51000100, 2240 HP, 750 posture and two marks.
The brown companion is now an independent combatant loaded directly from MSB entity 1700852, NpcParam 51000010 and think 51000011.
It has 1600 HP, 480 posture and one mark, and runs logic 510000 with battle 510001 through its own Lua, HKS, TAE, root motion, dummy rig, damage and posture runtime.
c5001 entity 1700851 remains the finishing centipede and is not substituted for the brown ape.
The real 11705880 summon drives visibility and appearance animation 20020.
Map instruction 2004[11] assigns TARGET_EVENT, whose installed ai_define value is 20, between the two apes.
Both Lua hosts receive that target's live position, HP, posture and SpEffects.
This matters because the brown battle script reads its partner's 3510900-series effects and otherwise mostly circles and approaches.
The selected ape advances the single Wolf once, while the other resolves contacts on that same frame.
Selection currently follows the nearer living enabled opponent, and a paired execution or grab retains its owning opponent until release.
This nearest-opponent selection is a host approximation, not the native manual target-switch algorithm.
Map commands, event flags, kills, immortality and timed effects reach the companion, and event dead/HP/message queries read its real combat state.
Reset reloads the companion and reuses the arena collision.
The renderer now follows the companion's simulated transform and clip clock, and displays its independent vitality and posture bars.
The first non-final headless pair 15100500/15100501 spends one mark and refills HP through a100_012000's SpEffect 300610 at 3.5 seconds.
Its simultaneous 275101 effect has maxHpRate and maxStaminaRate equal to 1, so it does not change the installed 2240/750 maxima.
The slow scripted arena replay uses an actual first deathblow, then lets the main ape's Lua event request reach region 1702871 and the real map wait summon the companion.
It records independent Lua contacts, the brown ape's living 15100180/15100181 execution, the main ape's separate headless continuation, message 30, defeat flag 9307 and both fog removals.
Wolf and the two apes are repositioned onto the main placement's open helper floor for the isolated contact check, and HP is reduced to offer the ordinary deathblow windows.
That controlled fixture is not a natural-input full-HP playthrough or proof of every attack's reach.
The event target, appearance timing and region arrival are not replaced by test flags or a direct warp.
The optional Mortal Blade severance, ThrowParam 15100590, a100_012900, finishing centipede and flag 9314 are not fight-through verified.
The general game has no complete inventory/unlock dispatcher for that optional execution.
Companion-specific firecracker response, every cooperative attack, ally-to-ally system-bullet delivery and manual lock switching remain unverified or incomplete.
Both burrow combat phases are playable and tested, but neither is declared DONE under the requested fidelity definition.

### Cave movement and remaining order

The burrow replay reproduced Wolf oscillating between Swim and SwimToStandMoveInSitu on its helper floor.
A material-22 plane is approximately fifteen centimetres above that floor, and the original query treated any positive depth as swimmable.
The water query now rejects an intervening solid floor above the body and water depth within the existing locomotion step height.
Body landing and water-mode movement use the same depth restriction.
Using step height as the shallow-water threshold is inferred and remains a fidelity approximation pending recovery of the native water-volume/depth rule.
The temporary ape-only gravity workaround was removed once that underlying movement problem was fixed.
The original floor-aware throw reservation still reports exhausted clearance in some paired placements.
Full animated-mesh clearance, unsupported character-part states and native camera sub-IDs remain gaps from the previous session.
Father and Inner Father have not been started while the ape fidelity blockers remain unresolved.
No requested model or animation extraction is missing for the work validated here.
The limitations above require engine hosting or verification work, not an owner-run extraction command.
No FwLo code was copied.
The read-only MIT notes enemy.md, enemies.md, deathblow_groups.md and boss_scripts.py supplied cross-checks of throw families, phase/cast structure and event-target pitfalls.
All IDs, row values and live timings were checked against the local params, TAE, Lua constant tables, MSB and EMEVD.

### Continuation files and validation

New source is crates/sim/src/headless_ape.rs and crates/game/src/ape_hud.rs, with the new boss test in crates/sim/tests/it/headless_ape.rs.
Small shared edits connect the companion in duel.rs, arena.rs and tests/it/main.rs, expose TARGET_EVENT in lua_ai/runtime.rs and capture its assignment in arena_c.rs.
The companion renderer is wired in arena_c_scene.rs, and play.rs adds the terror status label.
Status buildup and fatal resolution are in status.rs and fight.rs, with the cave-water correction in character.rs and body.rs.
The additive draw-mask event decoder and persistent state are in tae.rs.
The Guardian Ape test file now requires its real final continuation, one ordinary headless pair, message 30, terror range, sword guard/deflect and dung-shot payload.
The slow two-ape replay is explicitly ignored in the normal tier because it exceeds ten seconds.
All Cargo commands use target/alt-b, three jobs and offline mode.
The final installed-data ape run passed all seven tests, including the slow two-ape replay, in 34.33 seconds.
Workspace Clippy and the release game build passed after the owners resolved transient shared-tree compile errors.
The Owl phase-two regressions briefly failed during a concurrent SpEffect-category change, then passed on retry without edits to that owner's implementation.
The requested regression filters passed: Owl 60, Genichiro 15, brain_tactics 19 with one existing slow test ignored, attack_power 3, posture_recovery 6 and swim 6.
Direct rustfmt checks passed for the two new source files and the two ape test files.
Workspace cargo fmt --all --check still reports concurrent shared-file formatting and module-order differences; those files were not globally reformatted.
The graphical watering-hole and apes-burrow launches both returned exit code 0 using the fresh release executable.
The first passive watering-hole replay remained idle and does not establish a graphical phase transition.
The active watering-hole replay recorded Lua attacks but did not reach the headless phase with the simple attack-mashing script.
The first burrow replay spent its first mark and played a100_012000, but its forty-second attack-mashing run did not provide a clear two-ape capture.
An additional sixty-five-second burrow replay stopped attacking after the first mark and returned exit code 0, but its captures still do not visually establish the companion's arrival.
The captures expose heavy staged haze and cave-camera wall obstruction, and are startup/combat evidence rather than a visual-fidelity sign-off.
Local evidence is under target/alt-b/ape-continuation, outside tracked source.

The exact successful verification commands were run with the following environment.

```powershell
$env:CARGO_TARGET_DIR='target/alt-b'
$env:CARGO_NET_OFFLINE='true'
cargo test --release -p sekiro-sim --test it -j 3 'ape::' -- --include-ignored
cargo test --release -p sekiro-sim --test it -j 3 owl
cargo test --release -p sekiro-sim --test it -j 3 genichiro
cargo test --release -p sekiro-sim --test it -j 3 brain_tactics
cargo test --release -p sekiro-sim --test it -j 3 attack_power
cargo test --release -p sekiro-sim --test it -j 3 posture_recovery
cargo test --release -p sekiro-sim --test it -j 3 swim
cargo clippy --workspace --all-targets -j 3 -- -D warnings
cargo build --release -p sekiro-game -j 3
rustfmt --edition 2024 --check crates/sim/src/headless_ape.rs crates/game/src/ape_hud.rs crates/sim/tests/it/guardian_ape.rs crates/sim/tests/it/headless_ape.rs
```

The diagnostic cargo check -p sekiro-sim -j 3 passed.
The looking-only cargo fmt --all --check returned 1 for the shared formatting differences described above.
Transient builds failed in other agents' character, sword_saint and demon edits and were retried without modifying their code.

The successful arena-launch commands below wrote their own new output files under target/alt-b and ran without hosted model calls or player-memory writes.
The screenshot names are capture checkpoints, not claims that a phase or finish occurred at that step.

```powershell
& target/alt-b/release/sekiro-rs.exe --play --arena watering-hole --brain scripted --brain-memory off --god --lock-on --passive --break-posture --spawn 654.3921,-295.61334,722.50586,0,0 --enemy-distance 3 --script target/alt-b/ape-continuation/attack.txt --shots 300:target/alt-b/ape-continuation/guardian-first.png,1500:target/alt-b/ape-continuation/guardian-headless.png,2300:target/alt-b/ape-continuation/guardian-finish.png --exit
& target/alt-b/release/sekiro-rs.exe --play --arena watering-hole --brain scripted --brain-memory off --god --lock-on --break-posture --spawn 654.3921,-295.61334,722.50586,0,0 --enemy-distance 3 --script target/alt-b/ape-continuation/attack.txt --shots 300:target/alt-b/ape-continuation/guardian-active-first.png,1500:target/alt-b/ape-continuation/guardian-active-headless.png,2300:target/alt-b/ape-continuation/guardian-active-finish.png --exit
& target/alt-b/release/sekiro-rs.exe --arena apes-burrow --brain scripted --brain-memory off --god --lock-on --break-posture --spawn 187.54846,-377.73804,671.2771,0,0 --enemy-distance 3 --script target/alt-b/ape-continuation/attack.txt --shots 300:target/alt-b/ape-continuation/burrow-first.png,1500:target/alt-b/ape-continuation/burrow-pair.png,2300:target/alt-b/ape-continuation/burrow-finish.png --exit
& target/alt-b/release/sekiro-rs.exe --arena apes-burrow --brain scripted --brain-memory off --god --lock-on --break-posture --spawn 187.54846,-377.73804,671.2771,0,0 --enemy-distance 3 --script target/alt-b/ape-continuation/summon.txt --shots 1800:target/alt-b/ape-continuation/burrow-summon-1800.png,3200:target/alt-b/ape-continuation/burrow-summon-3200.png --exit
```

attack.txt repeats one attack frame and forty-four idle frames for 2,430 frames.
summon.txt repeats that segment for 900 frames and then waits 3,000 frames.
The save folder was backed up under target/alt-b/ape-continuation/save-backup before these executable runs, and the installed game itself was not launched.

## Previous session record

This pass starts with Guardian Ape in `watering-hole` and does not declare any encounter DONE.
The ordered Headless Ape and Father work remains outstanding until the first encounter meets the requested fight definition.
The current cache contains `models/c5100.glb`, `models/c5001.glb` and `models/c5070.glb`, superseding the missing-model observations in the arena-staging notes.
Cache and research directories remain read-only, and no network or git-state operation was used.

## Guardian Ape, entity 1700800

The existing registry and arena select c5100, NpcParam 51000000, initial think 51000000 and the m17 Lua bundle.
NpcThinkParam selects logic 510000 and battle 510000 initially, then logic 510000 and battle 510010 under headless think 51000100.
The model, skeleton, HKS, graph, TAE, root motion, dummy-based hit shapes, BehaviorParam attacks and Lua goals use the existing data readers rather than newly tuned attacks.
The installed initial row has 3520 HP, 1140 posture, one mark, hit radius 2.2 m and height 5.2 m.
The new tests use watering-hole map collision and scripted Wolf inputs, with god mode to isolate boss behavior.
They position Wolf near the actual boss placement for combat rather than waiting at the approach above the pool.
They are not graphical verification or a natural-input playthrough of every attack.

### Phase one and headless revival

The initial reproduction selected ThrowParam 15100180/15100181 and permanently killed the ape before the map's revival could restore HP.
The generic deathblow damage path clears immortality, whereas the waterfall encounter must retain the false corpse until its authored revival.
The scoped waterfall rule preserves 1 HP and immortality after that decapitation without changing the one-mark NpcParam row.
The defender's a000_013700 TAE sends message 20 at 6.133333 seconds.
Event 11705821 waits twelve seconds, requests HKS command 20000 and switches think to 51000100.
The a000_020000 revival clip applies SpEffect 401000, whose changeHpRate is -100, rather than a hand-written timed refill.
That authored refill also releases the simulation's dying lock-on state.
The first focused checks proved an actual Lua-driven attack reaches Wolf and that the decapitation reaches the delayed refill instead of ending the encounter.
The expanded arena replay also reaches a Lua-driven headless hit after revival.
Phase one is not declared fully faithful because every grab, projectile, response and attack has not been separately verified.
The normal Lua replay reports the existing unanswered `ai:GetHpLastTarget` query.

### Phase two and final execution

Headless deathblows select the extracted front pair 15100500/15100501 or back pair 15100610/15100611, using Wolf's a235 clips and the ape's matching absorption dummy.
The persistent a100 animation group is added to the row's defender offset before reading branch timing or starting the paired animation.
This avoids overwriting the headless group with the living ape's a000 throw clips.
Pair selection and offset composition are inferred from the extracted ThrowParam and TAE timelines rather than verified against the executable.
The test exercises the front headless pair on map collision.
The expanded replay initially interrupted Wolf's body clip because it opened the next deathblow while Wolf was still recovering from an AI hit.
Allowing recovery before the next deathblow produces the real front pair's strike and reaches defeat flag 9304.
This proves the existing dead-state victory path, but it does not prove the separate final paired continuation.
ThrowParam 15100509 and the a100_012090 timeline describe a separate continuation, including event message 30 and SpEffect 201000.
The ordinary headless pair is not a substitute for that continuation.
`--brain scripted --brain-memory off` retains the extracted Lua AI for c5100 when no external action table exists, with an explicit diagnostic and no controller requests.
The shared host exposes paired player event requests, but this pass has not verified c5100's continuation or the centipede execution through that path.
Revival SpEffect 275100 supplies maxHpRate and maxStaminaRate from params, giving 3872 HP and 1254 posture before SpEffect 401000 refills HP.
The application order and integer truncation are inferred; the native effect dispatcher has not been traced here.
The headless scream's terror build-up and fatal resolution are not implemented by the current status runtime, which supports poison and the separate electric-shock path.
Every projectile, grab, sweep counter and sword-deflect response remains unverified, rather than claimed functional from resource loading alone.
The Loaded Spear centipede pull, firecracker-specific fear and repeated sword-deflect collapse are not covered by these checks.
The arena replay reports that paired-path clearance can be exhausted and retains authored alignment, so full animated-mesh clearance is also not claimed.
The staged arena's unsupported character-part state instructions, false-victory banner and camera sub-IDs remain presentation gaps, and this pass has not visually verified the headless mesh variant.

## Headless Ape, entity 1700850, apes-burrow

No burrow fight implementation or fight-through verification is delivered in this session.
The existing stage selects c5100 with NpcParam and think 51000100, which has 2240 HP, 750 posture and two marks.
The existing interpreter enables the companion presentation, but no independent combat host is wired for the brown ape in this encounter.
The installed MSB identifies the brown companion as c5100_0002, entity 1700852, NpcParam 51000010 and think 51000011.
c5001_0001, entity 1700851, NpcParam 50011000 and think 0, is the finishing centipede rather than the second combat ape.
A faithful second phase requires the companion's independent AI, animation, hit detection and health, as well as the main ape's final execution and severance sequence.
The surviving headless deathblow also applies maximum-stat effect 275101, which this pass does not implement.
Neither burrow phase is DONE.

## Owl Father and Inner Father, entities 1000900 and 1000990

No Father fight implementation or phase verification is delivered in this session.
The existing Hirata arena wiring and castle Owl implementation are not proof of a faithful Father encounter.
Both phases in `hirata-father` and both phases in `hirata-inner-father` remain outstanding, including their event transitions and independently driven bird mechanics.

## Sources and changed files

The phase and stat facts above come from the local NpcParam, ThrowParam, SpEffectParam, c5100 and Wolf TAE exports, and the installed m17 EMEVD.
The read-only FwLo MIT notes `target/ref-fwlo/deathblow_groups.md` cross-check c5100's a230 family, and `enemies.md` cross-checks the distinction between Guardian and Headless Ape script rows.
Those notes do not establish completion of this implementation, and no FwLo code was copied.
New files are `crates/sim/src/guardian_ape.rs`, `crates/sim/tests/it/guardian_ape.rs` and this document.
Small additive shared edits register the module and test in `lib.rs` and `tests/it/main.rs`, and connect the scoped deathblow rules in `duel.rs`.
The one-dependency fix in `crates/game/src/arena_c_scene.rs` removes a startup scheduling cycle without reordering the shared main update chain.
The support update now follows draw masks and actor synchronization, and still precedes animation; changed support masks can apply on the next update.
`STATUS.md` records the session outcome.

## Validation

All build and test commands use `target/alt-b`, three jobs and Cargo offline mode.
The final Guardian Ape filter passes both tests in 2.85 seconds.
The Owl filter passes 60 tests in 32.69 seconds, and the Genichiro filter passes.
The brain-tactics filter passes 19 tests with one existing slow test ignored, in 23.16 seconds.
Attack-power passes three tests in 5.11 seconds, and posture-recovery passes six tests in 6.38 seconds.
The complete filter loop exits 0.
Workspace/all-target Clippy with warnings denied passes after the scheduling fix.
The final Clippy retry passes in 7.80 seconds, and the final release game build passes in 2 minutes 51 seconds.
The initial full release game build passes in 30 minutes 43 seconds, and the scheduling-fix rebuild passes in 3 minutes 26 seconds.
Direct rustfmt checks pass for the two new Rust files and the touched group-c support scene file.
Workspace formatting inspection reports prohibited shared module reordering and concurrent formatting changes in other agents' files; those files were not reformatted.
Two intermediate test builds encountered concurrent missing-method/import errors in Butterfly and folding-screen-monkey tests, and were retried without editing those files.
One later Clippy run caught an unused assignment in the concurrent Butterfly test; the one-minute wait and retry passed without changing that file.

```powershell
$env:CARGO_TARGET_DIR='target/alt-b'
$env:CARGO_NET_OFFLINE='true'
foreach ($filter in @('guardian_ape::','owl','genichiro','brain_tactics','attack_power','posture_recovery')) {
    cargo test --release -p sekiro-sim --test it -j 3 $filter -- --nocapture
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
cargo clippy --workspace --all-targets -j 3 -- -D warnings
cargo build --release -p sekiro-game -j 3
rustfmt --edition 2024 --check crates/sim/src/guardian_ape.rs crates/sim/tests/it/guardian_ape.rs crates/game/src/arena_c_scene.rs
cargo fmt --all --check
```

The graphical command initially exited 1 because of the support-actor schedule cycle, then exited 0 after the dependency fix.
The inspected capture `target/alt-b/guardian-ape/initial.png` shows the animated Guardian Ape in attack 3021, the named one-mark boss bar, Wolf's damage reaction and recorded AtkParam damage of 80 and 64 HP.
It proves first-phase graphical startup and Lua-driven attack presentation, rather than a visually verified full two-phase fight.
The image has washed-out fog/exposure and low-contrast labels, and the run reports fur-cloth resets; it is not a finished visual match.
No weather, particle or additional-lighting work was added.
The short idle input script and save backup are under `target/alt-b/guardian-ape`, outside tracked source.

```powershell
target/alt-b/release/sekiro-rs.exe --play --arena watering-hole --brain scripted --brain-memory off --god --lock-on --spawn 654.3921,-295.61334,725.50586,0,0 --enemy-distance 6 --script target/alt-b/guardian-ape/idle.txt --shots 330:target/alt-b/guardian-ape/initial.png --exit
```

The script contains `360 0 0 -`, keeping Wolf passive for six seconds.

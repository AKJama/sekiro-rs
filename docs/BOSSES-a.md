# Group-a boss combat, pass 2

Session date: 12 October 2026.
This is an incomplete combat pass, and none of the three bosses is claimed DONE.
The latest continuation hosts Butterfly illusion release and native descent and tests Gyoubu's two mounted bars without per-frame position holding.
Complete illusion waves remain blocked by pursuit homing, temporary-point navigation, summon recreation and Snap Seed, with a concrete late-wave floor fall recorded below.
The required boss models are now present, superseding the missing-model observations in the arena staging documents.
No missing extraction has been identified for the Butterfly paths exercised here.
All cache, raw game data and FwLo references were read-only.

## Lady Butterfly, phase 1

The existing registry and `hirata-hall` row resolve c5090, NpcParam 50900000, think 50900000 and MSB entity 1000800 from m10_00_00_00.
The installed NpcParam supplies 2073 base HP, 600 base posture, one mark and behavior variation 50900 for this entity.
The HUD now sums the first entity's remaining mark and the second entity's NpcParam mark, displaying two marks before the hand-over and one afterward.
Combat vitality and the two entities' individual deathblow counters remain separate.
Both phases use the real c5090 skeleton, clips, HKS graph, TAE and Lua bundle through the existing simulation.
The natural Lua replay exposed `GetDistXZ` returning the unknown-method fallback of zero.
That call now uses the runtime's horizontal target-distance calculation.
The scripted-input arena test observes real Lua decisions and damaging AtkParam contacts on the actual Hirata collision.
Observed decisions include 3000, 3001, 3002, 3003, 3013, 3014, 3016, 3024, 3240 and 5201.
Observed landed AtkParam rows include 50900000, 50900010, 50900011, 50900020, 50900030, 50900130, 50900140 and 50900160.
The separate deathblow regression uses real player attack inputs with a deliberately reduced posture meter, then exercises ThrowParam 15090000/15090001 and the EMEVD enable of entity 1000810.
This reduced-meter regression proves the paired throw and hand-over paths, not a complete naturally won fight.
Every attack, grab, projectile, perilous warning and jump-counter variant has not been independently verified.
Inherited AI approximations remain, including GetDistY/GetDistYSigned returning zero in the shared runtime rather than the actors' actual height difference.
No graphical fidelity or real-game comparison is claimed by the headless tests.
Both CLI forms were also run with scripted guard inputs and memory disabled, with visible Butterfly and Wolf models and the two-mark HUD captured in target/alt-a/butterfly-240.png.
The visible smoke run reports all 129 Butterfly bones bound and all four of her cloth simulations loaded.
This is a phase-one placement and animation smoke capture, not graphical verification of the complete fight or its second phase.

## Lady Butterfly, phase 2

The existing second arena row selects NpcParam 50900001 and entity 1000810, with the same base HP, posture and one remaining mark.
Its resident SpEffects include 200031 for animation group 100 and 200051 for the phase-two battle pattern.
The old generic suffix-based throw selection used the first entity's ordinary deathblow for this phase.
The new Butterfly module selects installed finishing rows 15090680/15090681, which pair Wolf's a244_501700/a244_511700 with the defender's 13700 execution.
The throw retains the second entity's live animation group, selecting its extracted a100_013700 clip rather than the unavailable a000_013700 name.
The choice of this finishing family for the second entity is inferred from those params and event 11005855, rather than a traced native throw-condition dispatcher.
TAE 936 with leading argument 10 now supplies the player event message 10 required by the ending event.
Other TAE 936 operations remain uninterpreted.
The message interpretation was cross-checked against the MIT notes in target/ref-fwlo/enemies.md and the actual a244_511700 timeline, whose message window begins at 2.9666667 seconds.
The shared event host now publishes player EzState requests to HKS as well as boss requests, so the map's 710205 execution request has a host.
The generic native throw selection remains approximate as documented in DEATHBLOW.md.
The second-entity descent and region gate are still bypassed by the pass-one event join at instruction 18.
The eight c1510 illusions have a c1500 animation donor in NpcParam, and both the c1510 model and c1500 animation extraction are present.
Their supporting actors now run simultaneous Lua/HKS combat and param-driven attack contacts against the same Wolf frame.
Shared AI event flags 19952081/19952082 now have a setter and propagate between the boss, illusions and map events.
Illusion-to-projectile conversion remains incomplete despite the authored release pulse reaching the cast.
The need for the shared AI numbers was cross-checked against target/ref-fwlo/enemy.md, which identifies Butterfly's 509000 AI and illusion goal 151010.
Snap Seed interaction with the illusion actors has not been implemented or tested in this pass.
These are gameplay gaps that must be completed before effects work can make the fight production-ready.
Phase 2 and the complete Butterfly encounter are not DONE.

## Gyoubu Oniwa

The continuation audits both mounted bars and adds a last-mark finishing selector for installed ThrowParam 15080170/15080171.
The arena's already-completed intro now skips its opening AI command 1 as well as its opening wait, avoiding a stale entrance command in an active fight.
The installed event and throw structure provides two mounted bars and a finishing execution, without a separate on-foot combat phase in the reviewed data.
No independent on-foot combat phase was invented.
Model cache/models/c5080.glb and animation skeleton cache/anim/c5080/skeleton.bin are present.
Native mounted movement, complete attack/counter coverage and unconstrained throw approach remain gaps before any complete-fight claim.

## Demon of Hatred

Phases 1, 2 and 3 were not changed or verified in this session.
The existing demon arena remains the pass-one baseline.
Model cache/models/c7020.glb and animation skeleton cache/anim/c7020/skeleton.bin are present.
Attack-driven hazards, all phase transitions and the final paired execution remain to be audited against actual data before any complete-fight claim.

## Changed files and verification

New Butterfly-specific logic is in crates/sim/src/butterfly.rs.
The two focused cache-backed tests are in crates/sim/tests/it/butterfly.rs, registered additively in tests/it/main.rs.
Shared changes are additive hooks in lib.rs and duel.rs, the GetDistXZ method in lua_ai/runtime.rs, the message-10 decoder in tae.rs and the HUD mark query in crates/game/src/play.rs.
No unrelated shared modules were reordered or reformatted.
No commits, git state changes, network calls or external brain calls were used.

```powershell
$env:CARGO_TARGET_DIR='target/alt-a'
$env:CARGO_NET_OFFLINE='true'
cargo run --release -p sekiro-sim --bin duel-sim -j 3 -- --enemy c5090:50900000 --steps 1200 --brain-memory off
cargo run --release -p sekiro-formats --example emevd_dump -j 3 -- cache/raw/event/m10_00_00_00.emevd 11005800 11005812 11005856 11005857 11005850 11005851 11005855 11005860
cargo test --release -p sekiro-sim --test it -j 3 butterfly:: -- --nocapture
foreach ($filter in 'butterfly::','owl','genichiro','brain_tactics','attack_power','posture_recovery') {
    cargo test --release -p sekiro-sim --test it -j 3 $filter -- --nocapture
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
rustfmt --edition 2024 --check crates/sim/src/butterfly.rs crates/sim/tests/it/butterfly.rs
cargo fmt --all --check
cargo clippy --workspace --all-targets -j 3 -- -D warnings
cargo build --release -p sekiro-game -j 3
Set-Content -LiteralPath 'target/alt-a/butterfly-smoke.txt' -Value '360 0 0 guard'
& .\target\alt-a\release\sekiro-rs.exe --play --arena hirata-hall --brain-memory off --brain scripted --god --script target/alt-a/butterfly-smoke.txt --shots 240:target/alt-a/butterfly-240.png --window-size 1280x720 --exit
& .\target\alt-a\release\sekiro-rs.exe --arena hirata-hall --brain-memory off --brain scripted --god --script target/alt-a/butterfly-smoke.txt --window-size 1280x720 --exit
```

The initial duel replay passed and reproduced the unknown GetDistXZ query, real guard/deflect contacts and Wolf's death to Butterfly's attacks.
The EMEVD example passed and supplied the reviewed phase, immortality, illusion and execution event identities.
Earlier regression iterations exposed an incorrect test assumption that resident 200051 would appear in the transient TAE-effect list, which was corrected to check the resident params.
The final two Butterfly tests passed in 5.09 seconds, including mark counts before and after each strike, the second entity's finishing rows, defender a100_013700, player a000_710205 execution and map defeat flag 9302.
Direct rustfmt checking of both new Rust files passed.
The workspace formatting check returned 1 for appended shared module ordering and a concurrent region-host expression in duel.rs, which were left untouched under the shared-tree rule.
Workspace Clippy passed with warnings denied on its final retry in 38.16 seconds.
Its earlier attempt saw stale simulation metadata while the HUD API was being added, and the retry rebuilt that metadata without a source workaround.
The final Owl filter passed all 60 tests in 36.16 seconds after the HUD and finishing-group changes.
The release game build passed in 31 minutes 38 seconds from a cold target directory.
Both six-second graphical CLI smoke commands returned 0, using the character's own Lua AI because c5090 has no external scripted-plan table.
The screenshot was opened and visually checked for the primary models and two-mark HUD.
Original saves were backed up to target/alt-a/butterfly-save-backup-20261012-160206 before those graphical runs, without modifying the originals.
The first queued regression retry encountered another agent's temporarily missing screen_monkeys.rs module, which was left untouched and retried after a minute.
The final incremental release game build passed in 2 minutes 54 seconds after the concurrent module was restored.
All requested regression filters passed, with only the existing slow parameter sweep ignored.

| Filter | Passed | Ignored | Runtime |
|---|---:|---:|---:|
| butterfly:: | 2 | 0 | 5.09 s |
| owl | 60 | 0 | 36.16 s |
| genichiro | 15 | 0 | 9.66 s |
| brain_tactics | 19 | 1 | 22.19 s |
| attack_power | 3 | 0 | 4.94 s |
| posture_recovery | 6 | 0 | 5.04 s |

These overlapping test filters are not a count of distinct tests or completed boss fights.

## Continuation: illusion combat and mounted Gyoubu

The continuation remains an incomplete pass, with no boss claimed DONE.
No missing cache extraction was found, so there is no extraction command that resolves the native-runtime blockers below.
The local FwLo `enemy.md` notes supplied the cross-check that 19952081/19952082 are shared event flags rather than per-actor AI numbers.
The installed compiled Lua constant tables independently confirm Butterfly writes these flags and illusion goal 151010 reads them.
All implementation is original Rust, and the reference scripts and extracted game data remained read-only.

### Butterfly phase 1

Existing real Lua attacks, separate vitality, mark display, first paired deathblow and entity hand-over are retained.
The new supporting encounter reloads with the primary boss on retry and hand-over, with the arena collision retained.
The new shared-player recovery guard prevents each illusion from advancing Wolf's posture recovery again in the same frame.

### Butterfly phase 2

Eight placed actors 1000820 through 1000827 load their own MSB think identities, real Lua goals, c1500 animation donor, c1510 model dummies, TAE and NpcParam combat data.
Each uses Wolf's already-advanced pose and combat state, with independent enemy animation clocks, hit detection, bullets and damage reactions.
The supporting-cast renderer reads these live simulation poses instead of independently advancing an event-only animation.
The summon regression feeds the actual map event's SpEffect 200060 condition, rather than substituting made-up summon commands.
It moves the cast into a contact fixture after spawning, so it proves actual attack timing and contact damage rather than faithful spawn navigation.
An authored action 3020 emits Bullet 50900921, whose installed expanding radius and rehit interval apply SpEffect 3509070 to the illusion cast.
The installed Bullet `spEffectIDForShooter` is also read and applied once per emitted primary bullet, allowing the boss's Lua observers to see its release effect.
Shared Lua writes are published to event state, and `GetDistY`/`GetDistYSigned` now use the actors' origin-height difference instead of a constant zero.
The origin choice and cross-actor system-pulse targeting are inferred engine reconstructions, not traced native targeting dispatch.
The focused probe resolves ordinary authored regions 1002830 and 1002860 but cannot resolve 1002890 or the Lua's event-relative points 9992800, 9992810, 9992930 and 9992960 through the complete local MSB host.
These require a native temporary/event-relative point host; they are not evidence of a missing map export.
The shipped goal table identifies `GOAL_COMMON_ToTargetWarp` as 2037, currently an immediately successful unsupported native goal.
The probe also identifies `GOAL_COMMON_ApproachSettingDirection` as 2060, which has no native implementation in this runtime.
Native illusion warp/release, conversion into the full homing projectile attack, re-summoning of killed actors and Snap Seed interaction remain incomplete.
The attempted native descent entry played a100_021000 and reached AI command 10 with the correct player region, but stayed in its looping event state instead of completing a playable descent.
That attempt was reverted, retaining the earlier instruction-18 entry fallback instead of leaving phase 2 stuck.
The map's clear of resident SpEffect 3509210, event-relative targeting and immediate-action env hosting still need native verification.
The pulse replay also observes unhosted `SetEnableImmediateAction_forGoal` and `ClearEnableImmediateAction_forGoal` calls.
Both existing paired deathblows and map defeat remain regression-covered, but phase 2 and the complete encounter are not DONE.

### Gyoubu bar 1

The registry already selects c5080, NpcParam 50800000, think 50800000 and entity 1100800 on m11_00_00_00.
Installed base HP is 2552, posture is 660 and the initial mark count is two.
The active-arena entry now respects seeded flag 11100801 before issuing command 1, retaining the actual boss-bar suffix of event 11105810.
Real Lua, skeleton, root motion, TAE and param-driven attack contacts run on the staged castle-gate collision.
The deathblow regression uses real attack presses and a reduced posture meter, holding initial pair positioning during the start clip to isolate authored throw timing.
It exercises rows 15080000/15080001 and the get-up HP refill, rather than claiming a naturally won full fight.
Without that positioning fixture, an observed first mounted pair can miss its body range after root motion and collision displacement.
Native mounted approach, clearance and absorption therefore remain an explicit gameplay gap.

### Gyoubu bar 2 and defeat

The second bar remains the same mounted c5080 actor, with the Lua-visible remaining/max mark values changed by the first real throw.
Last-mark selection uses installed rows 15080170/15080171 and the a225 animation family instead of the generic selector's first-phase fallback.
The finishing-family choice is inferred from the params and finishing clips, without a traced native throw-condition dispatcher.
The second-bar replay reproduced a panic in a000_003043, whose authored TAE 760 bounds are minimum 6 and maximum 4 with reference distance 4.
The shared lunge calculation now applies the original lower and upper bounds in sequence without Rust's ordered-bound assertion.
Upper-bound precedence for this inverted interval is an honest approximation until the native handler is traced.
The normal ordered-bound behaviour is preserved and is checked by the existing Owl/Genichiro combat regressions.
Directional mounted goal 2060, sound-target rank, effect-observer deletion, grapple/prosthetic reactions, every perilous/counter variant and full graphical phase progression are not all verified.
Gyoubu is not claimed DONE.

### Demon phases 1, 2 and 3

All three phases remain at their previous staged baseline.
Demon combat work was not started while Butterfly's native blockers and the Gyoubu audit remain unresolved.

### Continuation files

New source is `crates/sim/src/butterfly_illusions.rs` and `crates/sim/src/gyoubu.rs`.
The read-only diagnostic is `crates/sim/examples/butterfly_ai_probe.rs`.
Tests extend the existing Butterfly file and add `crates/sim/tests/it/gyoubu.rs`, registered additively in main.rs.
Shared hooks are small additions in duel.rs, arena.rs, arena_b.rs, lib.rs, lua_ai/runtime.rs, homing.rs and game/src/arena_d_scene.rs.
The two heavier Butterfly multi-actor/paired-execution checks are explicitly ignored as slow and were run with `--include-ignored`.

## Further continuation: native release and descent (2026-10-12)

This section supersedes the earlier immediate-action, projectile-emission, descent-bypass and Gyoubu position-holding observations.
No boss is claimed DONE under the complete-fidelity definition.
No cache extraction is missing for these exercised paths, and an extraction command cannot resolve the remaining runtime omissions.
The shared-plan network claim was not repeated because this session explicitly forbids network and publishing.

### Butterfly phase 1

The existing real-Lua attack and first paired-deathblow paths are retained.
The phase hand-over now transfers event 11005812's program counter and active wait rather than restarting its already-executed setup.
Only that event is continued, and only when handing over to NpcParam 50900001.

### Butterfly phase 2

The starting release regression failed because it disabled Butterfly's Lua until the forced release, while still requiring a shared flag published by that Lua.
The fixture now runs Butterfly's Lua, enters the original phase-two region gate, and checks all eight actors' release clips and installed damaging Bullet children.
The installed common `attack_immediate_action.lua` enables immediate action at activation and clears it at termination.
The installed c9997 HKS dispatcher reads that action through env 125, which now has a host connection.
This is an original engine reconstruction from those scripts, without copied script or decompiler output and without a traced native cancellation handler.
All eight actors 1000820 through 1000827 play a000_003031 and emit Bullet families 15000800/801/802, 15000810/811/812 and 15000820/821/822.
The earlier assertion that no damaging projectile conversion occurs is superseded by this emission check.
The shared Bullet engine still does not consume homingAngle, homingAngleX, homingBeginDist and hormingStopRange, so the third-stage pursuit is not faithful and emission alone is not a complete projectile-attack proof.
Installed pursuit rows 15000802/812/822 specify horizontal 42, vertical 50 and stop range 3, while the initial and delayed children have distinct flight and lifetime values.
Native temporary-point warps, character recreation through the summon generator, complete re-summoning after kills and Snap Seed interaction remain missing.
Their authored instructions and temporary point ids are present in the cache, so invented target positions or additional extraction would not fix the host omissions.

The primary Butterfly now consumes the map's forced animation as an HKS event instead of leaving it in renderer-only scene state.
EMEVD SetCharacterAIState is decoded, with its primary-combat host scoped to Butterfly in this pass.
Butterfly's ClearSpEffect also removes its NpcParam-resident copy, allowing event 11005812 to clear entrance effect 3509210.
Resident effects restore on fight reset, with persistent attack-power selection retained.
The instruction-18 phase-two bypass is removed.
The unchanged event now waits for Wolf in region 1002830 and boss SpEffect 5028, sends AI command 10, starts the named bar and clears that command when SpEffect 5450 ends.
The native replay observes a100_021000, action 3290, a100_003290, a100_003292, command -1 and phase flag 11005802, followed by ordinary combat decisions.
The entrance-region fixture projects Wolf onto the actual collision floor because region 1002830's volume origin is below that floor.
An earlier fixture placed him at that volume origin, causing a fall and misleading later contact failures; the fixture now resets velocity and grounded state at the resolved floor.
The paired hand-over exposed a clear of the second entity's entrance effect before the first throw released the primary slot, which is now retained as map character-effect state and applied to the incoming combatant.
The final release replay still observes actors 1000824 and 1000825 below the hall floor later in the encounter, so complete spawn navigation and collision safety remain a concrete failing gameplay path despite the scoped contact/emission assertions passing.
This proves the event and animation path headlessly, not exact canopy placement, collision clearance or graphical descent fidelity.
The paired-deathblow regression now requires that entrance/descent path before the second execution.
Butterfly remains incomplete because pursuit homing, summon recreation/warps, Snap Seed and comprehensive attack/counter coverage are unresolved.

### Gyoubu, both mounted bars

The installed fight has two mounted bars; no separate on-foot combat phase was found or invented.
Both bars' natural Lua attacks land on the staged castle-gate collision.
The previous per-frame position-holding fixture was removed, and the tested first deathblow and final execution still succeed through root motion and collision.
The regression retains one initial close-range placement and a reduced posture meter to isolate each paired throw, so it does not prove a naturally won unconstrained fight.
The first pair restores 2552 HP, retains one mark and raises 11105802; the final pair plays a225_511610/a000_013610 and reaches map defeat 9301.
The observed first-bar AtkParam contacts include 50800280, 50800300, 50800350, 50800351, 50800990 and 50800991.
The observed second-bar contacts include 50800000, 50800001, 50800030, 50800101, 50800300, 50800302 and 50800991.
The shared throw-clearance reconstruction made the unheld fixture pass, but its exact native rule remains approximate as documented in FIDELITY.md.
Directional mounted goal 2060, sound-target rank, effect-observer deletion, grapple/prosthetic reactions, full perilous/counter coverage and graphical phase progression remain unverified or unhosted.
Gyoubu is playable and tested headlessly in both bars, but is not DONE.

### Demon, phases 1 through 3

Demon was not changed or fight-through tested by this continuation while Butterfly's remaining native-runtime omissions block completing the requested boss order.
No Demon fight-through or phase fidelity is claimed.

### Files and commands in this continuation

Changed source is butterfly.rs, lua_ai/runtime.rs, duel.rs, events.rs, fight.rs, arenas_a.rs and arenas/scene.rs, using small local edits without reordering shared files.
Changed own tests are tests/it/butterfly.rs and tests/it/gyoubu.rs.
The read-only butterfly_ai_probe example now supports --code, --replay and --descent, including actual event logs and goal parameters.
Documentation changes are this file, AI.md, FIDELITY.md and STATUS.md.
No git-state changes, network calls, cache/re writes or external controllers were used.

All Cargo commands use the following environment and three jobs.

```powershell
$env:CARGO_TARGET_DIR='target/alt-a'
$env:CARGO_NET_OFFLINE='true'
cargo test --release -p sekiro-sim --test it -j 3 butterfly_illusion -- --include-ignored --nocapture
cargo run --release -p sekiro-sim --example butterfly_ai_probe -j 3 -- --replay
cargo run --release -p sekiro-sim --example butterfly_ai_probe -j 3 -- --code cache/raw/script/aicommon.luabnd.d/attack_immediate_action.lua
cargo run --release -p sekiro-sim --example butterfly_ai_probe -j 3 -- --descent
cargo test --release -p sekiro-sim --test it -j 3 gyoubu:: -- --nocapture
cargo test --release -p sekiro-sim --test it -j 3 butterfly:: -- --include-ignored --nocapture
foreach ($filter in 'gyoubu::','owl','genichiro','brain_tactics','attack_power','posture_recovery') {
    cargo test --release -p sekiro-sim --test it -j 3 $filter
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
cargo test --release -p sekiro-sim --test it -j 3 posture_recovery
rustfmt --edition 2024 --check crates/sim/src/butterfly.rs crates/sim/tests/it/butterfly.rs crates/sim/tests/it/gyoubu.rs crates/sim/examples/butterfly_ai_probe.rs
cargo fmt --all --check
cargo clippy --workspace --all-targets -j 3 -- -D warnings
cargo build --release -p sekiro-game -j 3
```

The initial release reproduction failed after 12.93 seconds on the missing shared flag; the immediate-action correction and enabled-Lua fixture passed in 13.02 seconds with every release actor observed.
The first Gyoubu replay without position holding passed in 4.74 seconds, and the next replay passed in 5.87 seconds.
The corrected descent diagnostic returned zero and completed the unchanged event, with its animation and request evidence stated above.
An intermediate diagnostic compilation failed on an incorrect Emevd parse call and was corrected to Emevd::parse.
The first broad regression batch passed Butterfly, Gyoubu and all 60 Owl-filter cases before encountering an intermediate resident-clone type-inference error, which was corrected with an explicit Vec<i32> annotation.
The first final Clippy attempt encountered another agent's temporarily missing step_ape_encounter hook and was left to its owner under the shared-tree rule.
Subsequent Clippy and game-build attempts encounter private Panel/Fill types exposed by ape_hud::update at crates/game/src/ape_hud.rs:49 and its scheduling call at arena_c_scene.rs:84.
These other-agent files were not changed, and the checks were retried after the required wait.
Direct rustfmt checking of all four group-a Rust files passes.
The workspace formatting inspection returns 1 for shared/concurrent formatting and appended module ordering, which were left untouched under the shared-tree rule.
No new graphical game run or real-game comparison was performed in this continuation.
The final Butterfly run tightens the emission assertion to primary bullets' first tick, observes only a000_003031 as the source clip on all eight actors, and requires all nine child-family rows.
The final regression loop passes Gyoubu, Owl, Genichiro, brain tactics and attack power, then encounters another agent's new borrow errors at old_dragons.rs:72 and :127 before posture-recovery execution.
The remaining filter passes independently after the required wait, without editing that file.

| Final filter/check | Passed | Ignored | Result/runtime |
|---|---:|---:|---|
| butterfly:: with --include-ignored | 3 | 0 | 18.28 s |
| gyoubu:: | 1 | 0 | 5.16 s |
| owl | 60 | 0 | 22.70 s |
| genichiro | 15 | 0 | 9.24 s |
| brain_tactics | 19 | 1 | 23.07 s |
| attack_power | 3 | 0 | 5.53 s |
| posture_recovery | 6 | 0 | 6.23 s |
| Direct rustfmt, four group-a files | - | - | Exit 0 |
| cargo fmt --all --check | - | - | Exit 1, shared/concurrent differences |
| Workspace/all-target Clippy | - | - | Exit 101, concurrent ape HUD privacy errors |
| Release game build | - | - | Exit 101, concurrent ape HUD privacy errors |

These overlapping filters are not a count of distinct tests or complete fights.

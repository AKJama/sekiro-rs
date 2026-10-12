# Folding Screen Monkeys - pass 2 continuation

The four targets now have independent combat simulation in `--arena senpou-screens` and `--play --arena senpou-screens`.
Real Wolf contacts and paired deathblows kill them, change their screens and reach the map's four-target victory.
The encounter is **not DONE** under the requested fidelity criteria because sensory acquisition, roof navigation, the room-door interaction and several set-piece details remain incomplete.
The hidden target has a normal-arena-start sword replay with all four Lua instances active and no actor-placement, HP or posture changes.
The four-target victory replay uses controlled catches on an authored floor with Lua enabled and is not a complete natural puzzle walkthrough.

## Identity and animation source

Entities 2000801 through 2000804 are c1260_0000 through c1260_0003 in m20_00_00_00.
Their NpcParam and initial NpcThinkParam rows are 12600000, 12600010, 12600020 and 12600030.
All four initial think rows select logic and battle goal 126000 from m20_00_00_00.luabnd.d.
Their raw HP values are 249, 248, 247 and 5, with zero boss marks, a 0.3 m collision radius and a 1 m height.
They are four simultaneous targets, not four sequential boss lives.
Entity 2000800 is the hidden c1000_0009 controller, row 12600050, with 100 HP and bar name 901260.
The registry resolves its display name through the actual event NPC-name text rather than c1000's ordinary name, and the HUD shows zero collective deathblow marks.

The previous investigation established that c1260.anibnd.d is skeleton-only, with no animation HKX or TAE members.
All four placed rows explicitly select normalChangeAnimChrId 1250 and normalChangeTexChrId 1259.
The existing donor resolver retains the c1260 model and identity while loading c1250's skeleton, HKS, graph, clips and TAE.
The donor has 307 decoded clips, zero reported failures and 143 bones.
Donor and variant have matching bone names and parent links but differing bind poses.
The retained integration test checks all four placed identities and an HKS-selected decoded clip with matching TAE.
This resource check does not establish visually correct skinning for every pose.

## Per-target and outcome coverage

| Target | Implemented and checked | Missing or approximate |
|---|---|---|
| 2000801, row 12600000, visual monkey | Own Character, params, hit detector, Lua state, donor clips and TAE; actual scripted contacts and paired deathblow; independent screen and shared-bar reduction | Exact sight and darkness acquisition, room-door action, native roof routes and stealth eligibility |
| 2000802, row 12600010, hearing monkey | Same independent combat/death path; entity-specific think changes; bell input plays Wolf's 710700 and reaches its real event and region-gated 20011 stun request | Sound propagation/history, bell absorption/object animation, every stun/get-up frame and native roof routes |
| 2000803, row 12600020, alarm monkey | Same independent combat/death path; own Lua/effects; authored event changes to the other monkeys' think rows | Alarm audio/perception feedback, native roof routes and complete sensory-state timing |
| 2000804, row 12600030, hidden monkey | Real sword hit kills its 5 HP row; its screen changes without ending the encounter; zero-alpha camouflage comes from VFX params; authored player-dummy following is hosted | Footprints, exact material fading/reveal, full targeting/perception history and complete following-loop checks |
| Apparitions 2000805-2000809 and 2000811-2000814 | Authored enables lazily load independent c1250 Character, params, Lua and combat; models follow simulation; one native spawn produces Lua attacks and a Wolf contact; victory sends real kill commands | All spawn tiers/death flags, terror fidelity, shared projectile conversion/presentation and native navigation |
| Controller and victory | Actual deaths drive screen slots; effect 3126010 takes 26 percent of max HP per application; 12005800 waits for all four deaths and awards 9305 | Complete post-victory transport, reward/inventory handling and original cinematic playback |

The visual/hearing/alarm roles are cross-checked from resident effects 5020, 5021 and 5022 and their region/event logic.
The hidden target's resident effect 3126040 selects VFX row 50340, whose installed camouflage has zero minimum alpha and zero ending distance.
Camouflage currently switches visibility rather than reproducing the native material fade.
The screen slot order is 2000801 to 2001830/2001840, 2000802 to 2001832/2001842, 2000803 to 2001831/2001841 and 2000804 to 2001833/2001843.

## Combat and event implementation

Duel::screen_monkeys owns independent opponent runtimes and transfers the one Wolf, his combat state, pending reactions, hit stop, camera state and input edges between them.
Only the selected opponent advances Wolf's HKS, status clock and camera, while other opponents resolve against his already advanced frame.
An active paired throw retains its target until release.
Nearest-living-opponent selection is an approximation of the native lock-on and deathblow candidate search.
NPC clips, capsule shapes, root motion, TAE windows, HP/posture damage and throws use the existing data-driven Duel machinery.
No boss-specific attack timings, damage values, shapes, bullets, grabs or perilous counters were invented.
Not every donor attack, bullet, grab or counter has been replayed.
Wolf projectile simulation remains per-opponent and is not a verified shared-projectile solution.
Auxiliary attack audio, decals and projectile rendering are not yet a complete multi-actor presentation path.

Death, HP, SpEffects, event messages, regions, AI-state and entity-distance queries reach the relevant loaded actor.
AI commands, replans, think changes and HKS execution requests are routed independently, and each actor reads the encounter's event flags.
SetEventMoveTarget resolves the selected MSB region and POINT_EVENT 102 reads its cached position.
CheckDoesExistPathWithSetPoint now reaches the existing path-query host instead of returning the unknown-method default false.
That host still assumes reachable floor, so it does not reconstruct native roof navigation.
Condition-group skip/end commands are hosted, retaining concurrent group-d refinements.
Screen health effects are applied separately so simultaneous deaths do not collapse into one timed-effect edge.
The observed controller sequence is 100, 74, 48, 22 and 0, from the installed 26 percent rate.
The map's defeat event remains the authority for final victory.

Event 12005873's 2004[42] warp is hosted for its actual player target and dummy 169, using Wolf's live FLVER dummy frame and the existing collision world.
The surrounding animation-disable/enable commands reach the hidden monkey's runtime.
The warp position comes from the rig rather than an invented trailing offset.
The remaining native warp/floor-copy and perception semantics are not claimed fully reconstructed.

## Bell and remaining puzzle blockers

The bell is MSB object 2001800, model o205101, ObjActParam 205101 and ActionButtonParam 2000691.
Its MSB object-action record uses activation event 12000825.
The action volume uses the object's actual FLVER dummy 170, MSB transform, 1.5 m radius, 1 m height and -2 m base-height offset.
A rising E/Y action in that volume reaches condition 5[2] and event 12005825 sets 12005826.
A distant action does not activate it, and the HUD displays an E/Y bell prompt when its volume is available.
Event 12005860 then requests 20011 for entity 2000802 in region 2002553, retaining its authored waits and subsequent 21000/20012 requests.
The test checks activation and the stun request, not every paired gesture and get-up frame.
Bell activation now plays the installed player object-action animation 710700 through its existing W_Event710700 graph event.
Player absorption onto the bell's ObjActParam dummy remains unimplemented.
ObjActParam's angle restrictions and final absorption-distance check remain incomplete.

The room-door action record uses entity 2001504, ObjActParam 999922, activation event 12000810 and flag 62000705.
Its paired animation/activation host remains unimplemented.
Native room GPARAM transitions, the Hall bell inventory reset and sound-target acquisition remain incomplete.
ClearEnemyTarget and several history/team methods retain the general runtime's incomplete behavior.
GetTeamRecordCount is still reported as an unknown query by the hidden target's battle goal.
MoveToSomewhere has no native roof path/link planner even when Lua selects a genuine MSB destination.
These are engine blockers to a faithful natural puzzle replay, not missing c1260 animations.

## Extraction and visual state

The required c1000, c1250 and c1260 character GLBs are present.
The m89 export is present, but this session still found no maps/m89_00_00_99/models/m900085.glb and no m900085 entry in sky.json or its exported layout.
The raw m900085 FLVER is present under its original map binder.
The authored sky group remains 8907085, and graphical startup explicitly reports the missing illusion sky and uses m900010 as a fallback.
No cache file was modified or re-exported by this session.
The owner-run command below is the full export after repairing the exporter if it continues to omit this collisionless sky placement.
Repeated anims c1260 cannot produce donor animations or repair the engine blockers.

```powershell
$env:CARGO_TARGET_DIR='target/alt-e'
$env:CARGO_NET_OFFLINE='true'
Remove-Item Env:SEKIRO_MAP_SKY_ONLY -ErrorAction SilentlyContinue
Remove-Item Env:SEKIRO_MAP_LIGHTS_ONLY -ErrorAction SilentlyContinue
Remove-Item Env:SEKIRO_MAP_ENV_ONLY -ErrorAction SilentlyContinue
cargo run --release -p sekiro-extract -j 3 -- map m89_00_00_99
```

## Verification and sources

The donor test checks every placed row, matching rig names/parents and an HKS-selected clip with TAE.
The independent-Lua test checks the hidden monkey's authored following warp, rings the bell, checks Wolf's real gesture and the hearing monkey's stun request.
The normal-start hidden-target test uses only movement, camera and attack inputs to kill that target, requires the other three to remain alive, and checks shared health and the delayed screen change.
It then checks that Wolf returns to his last safe arena floor after a void fall.
The victory test uses controlled floor catches and scripted Wolf attacks with every target's Lua enabled, without writing HP, posture, death flags or controller health.
It exercises paired deathblows for the larger targets and the original final victory event after the fourth death.
The slow apparition replay checks an authored screen death and region-triggered spawn, then its own Lua attacks and actual contact with Wolf.
Its initial failure reproduced an unhandled CheckDoesExistPathWithSetPoint call selecting unreachable-target action 3021 instead of the offensive chain.
After the method host was added, the replay recorded five Lua attack requests and one Wolf contact in 12.88 seconds.

FwLo's MIT enemies.md and deathblow_groups.md under target/ref-fwlo cross-check c1260's region-scripted encounter and the a202/a200 throw families.
No reference code was copied.
Local MSB, EMEVD, compiled Lua constants, NpcParam, NpcThinkParam, ObjActParam, ActionButtonParam, SpEffectParam, SpEffectVfxParam and donor animation data supply the identities and numbers.
No game script, extracted asset or decompiler output is tracked.
Temporary original inspection code was removed after its read-only checks.

All Cargo commands use CARGO_TARGET_DIR=target/alt-e, CARGO_NET_OFFLINE=true and three build jobs.

```powershell
cargo run --release -p sekiro-formats --example emevd_dump -j 3 -- cache/raw/event/m20_00_00_00.emevd 12005800 12005813 12005820 12005825 12005855 12005856 12005857 12005858 12005859 12005860
cargo test --release -p sekiro-sim --test it -j 3 folding_screen_monkeys -- --nocapture
cargo test --release -p sekiro-sim --test it -j 3 folding_screen_monkeys -- --include-ignored --nocapture
cargo test --release -p sekiro-sim --test it -j 3 folding_screen_monkeys_apparition -- --include-ignored --nocapture
cargo test --release -p sekiro-sim --test it -j 3 owl
cargo test --release -p sekiro-sim --test it -j 3 genichiro
cargo test --release -p sekiro-sim --test it -j 3 brain_tactics
cargo test --release -p sekiro-sim --test it -j 3 attack_power
cargo test --release -p sekiro-sim --test it -j 3 posture_recovery
cargo test --release -p sekiro-sim --test it -j 3 senpou_screens
cargo clippy --workspace --all-targets -j 3 -- -D warnings
cargo build --release -p sekiro-game -j 3
cargo fmt --all --check
rustfmt --edition 2024 --check crates/sim/src/screen_monkeys.rs crates/sim/src/screen_monkeys_interactions.rs crates/sim/tests/it/folding_screen_monkeys.rs
```

The continuation's verification results and graphical commands are recorded below.

## Files changed

New logic is in crates/sim/src/screen_monkeys.rs and screen_monkeys_interactions.rs.
The existing crates/sim/tests/it/folding_screen_monkeys.rs gains combat, bell, following, victory and slow apparition coverage and retains its prior module registration.
Small shared hooks are in crates/sim/src/duel.rs, arena.rs, events.rs, registry.rs, lua_ai/runtime.rs and lua_ai/regions.rs.
Presentation hooks are in crates/game/src/play.rs and arena_d_scene.rs.
Documentation changes are this file, the dated STATUS.md section and a fidelity cross-reference.
No cache/re writes, existing-log writes, network calls, model/API calls, commits, pushes or git-state changes were made.

## Continuation findings, 12 October 2026

The first installed-data replay failed before the hidden monkey's authored warp because the encounter's 3[3] distance host used region-condition polarity.
In/out radius uses 0 for inside and 1 for outside, and 12005873 requires outside 30 m before its dummy-169 warp, then outside 5 m before enabling animation again.
The corrected host reaches the actual map command in the regression instead of forcing a warp in the fixture.
The replay also exposed auxiliary Duels retaining Wolf's initial origin placeholder as their void-return fallback.
Each auxiliary spawn now receives the encounter's real Wolf start, while the shared last-safe-floor state remains authoritative.
The existing shared-player transfer helper is visible within the duel module so the concurrently implemented brown ape can reuse it.
Auxiliary Lua event-flag writes are published back to the encounter before later actors and map events run.
The monkey battle chunk inspected here does not itself write event flags, so that publication is plumbing rather than evidence of another puzzle mechanic.

The hidden monkey is independently playable and tested from the normal arena start.
The three visible monkeys remain combat-playable in controlled catches, including their real paired deathblows, with their own Lua AI running throughout.
Their sensory puzzle and full roof pursuit are not claimed complete.
The four deaths still reach the actual map defeat event and flag 9305, rather than a synthetic victory condition.

The remaining blockers were checked against installed data rather than presumed missing c1260 animation extracts.
The visual, hearing, alarm and hidden think rows have different eye ranges of 70, 45, 50 and 180 metres for perception, but the current general AI host treats a supplied Wolf target as visible regardless of those fields.
ClearEnemyTarget remains a no-op, and GetTeamRecordCount is still unknown in the native monkey battle replay.
Consequently the darkness regions cannot yet reliably clear acquisition and maintain the map's requested AI state 0.
Inventing blindness or hearing thresholds from the role labels would not repair that native acquisition/history dispatcher.
The raw Senpou navigation binder is present at cache/raw/map/m20_00_00_00/m20_00_00_00.nvmhktbnd.d, but no decoded navigation surface/link extract or runtime planner exists.
MoveToSomewhere therefore has real MSB destinations and collision physics but no native roof-link route, and CheckDoesExistPathWithSetPoint still assumes reachability.

The room door is placed o201040_8000, entity 2001504, with its static GLB present.
Its raw object skeleton, animations and TAE are present under cache/raw/obj/o201040.objbnd.d/o201040.anibnd.d.
Decoded object skeleton/clips are absent, and the existing sekiro-extract anims command only searches raw/chr character binders.
No existing extraction command can produce this object-animation cache yet; an object-animation exporter and paired ObjAct activation/absorption host are required before its installed 700220/1 pair can run.
The same object-animation gap affects the bell's installed object animation 10, although Wolf's 710700 clip is present and now plays.
No fake open-door flag or replacement object motion was added.

The requested m89 re-export was inspected again: m900085 is still absent from sky.json, layout.json and models/m900085.glb, although its raw FLVER and authored group are present.
The documented full map export remains the owner-run command after repairing the sky export omission; it was not run here.
Sky rendering remains pass-3 scope and does not prevent the tested hidden-target combat.

This continuation edits only screen_monkeys.rs, folding_screen_monkeys.rs, this document, STATUS.md and the fidelity cross-reference.
The larger file list above describes the implementation already present when this continuation began.

## Continuation verification

All commands below use target/alt-e and three Cargo jobs, with Cargo network access disabled.
The full installed-data monkey run passed all five tests, including the ignored apparition replay, in 17.18 seconds with no skips.
The apparition replay recorded five native Lua attack requests and one actual contact with Wolf.
The victory fixture killed the four targets with 3, 7, 7 and 3 sword contacts respectively, retained their Lua brains, and reached controller health 74, 48, 22 and 0 and defeat flag 9305.
The hidden-target replay also checks that the other three targets remain alive and Wolf's void return stays on his last safe temple floor.
After graphical verification, that test was tightened to the same fixed 175-degree turn and sword inputs, without reading the hidden actor's position for steering.
The tightened test passed in 10.10 seconds while concurrent compilation was running and is assigned to the ignored slow tier.
Its final explicit include-ignored run passed again in 7.73 seconds.

The release game build passed in 3 minutes 32 seconds.
The later end-of-session release build failed on the concurrent ape HUD's private Panel signature at arena_c_scene.rs:84, matching the latest lint failure.
The final retry after the shared-tree wait failed on the same private Panel/Fill boundary, with no changes to the other agent's HUD.
Workspace/all-target Clippy initially passed with warnings denied after removing a redundant InputFrame update in the new test.
Later concurrent ape_hud.rs changes introduced private Panel/Fill types in its pub(super) update signature, and the latest Clippy retry failed at that unrelated renderer boundary.
The ape HUD was left untouched under the shared-tree ownership rule.
The required regression filters passed Owl 60/60, Genichiro 15/15, brain_tactics 19/19 with its full sweep ignored, attack_power 3/3, posture_recovery 6/6 and senpou_screens 1/1.
Direct rustfmt checks passed for the monkey source, interaction source and test file.
The workspace formatting check returned 1 for concurrent/shared files, including arena_c_scene.rs, arena_d_scene.rs, duel.rs, Lua runtime, status/TAE code and the additive integration module list.
Those files were not reformatted or reordered.
The first tests caught the inverted distance host; subsequent assertions were corrected to respect the screen event's installed 0.5-second delay and the engine's last-safe-floor return rather than requiring the initial spawn.
Two intermediate builds failed on a concurrent brown-ape caller of the private shared-player helper; after retrying, visibility was extended in our monkey module without editing the brown-ape file.

Saves were backed up locally under target/alt-e/monkeys-continuation-20261012/save-backup before graphical runs.
Both arena CLI forms opened, rendered and exited successfully with scripted input and brain memory off.
The first two facing fixtures did not kill a target and are not claimed as combat evidence.
The final fixed turn uses the MSB entry's source yaw 235 degrees and the CLI's camera-relative script convention.
That replay hit the hidden target at step 128 with AtkParam_Pc 5000012 for its remaining 5 HP and 30 posture, and the inspected step-180 capture shows controller HP 74 and its changed screen.
The capture is target/alt-e/monkeys-continuation-20261012/turn-after-sword.png.
The revealed corpse is skinned and visible, but the image is washed out and contains the existing debug HUD and cloth-reset warnings.
This is combat/presentation evidence for one target, not a complete visual-fidelity claim or a full puzzle walkthrough.

```powershell
$env:CARGO_TARGET_DIR='target/alt-e'
$env:CARGO_NET_OFFLINE='true'
cargo test --release -p sekiro-sim --test it -j 3 folding_screen_monkeys -- --include-ignored --nocapture
cargo test --release -p sekiro-sim --test it -j 3 folding_screen_monkeys_hidden_target -- --include-ignored --nocapture
foreach ($filter in @('owl','genichiro','brain_tactics','attack_power','posture_recovery','senpou_screens')) {
    cargo test --release -p sekiro-sim --test it -j 3 $filter
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
cargo clippy --workspace --all-targets -j 3 -- -D warnings
cargo build --release -p sekiro-game -j 3
cargo fmt --all --check
rustfmt --edition 2024 --check crates/sim/src/screen_monkeys.rs crates/sim/src/screen_monkeys_interactions.rs crates/sim/tests/it/folding_screen_monkeys.rs
target/alt-e/release/sekiro-rs.exe --play --arena senpou-screens --brain scripted --brain-memory off --script target/alt-e/monkeys-continuation-20261012/hidden-catch.txt --shots 20:target/alt-e/monkeys-continuation-20261012/normal-start.png,180:target/alt-e/monkeys-continuation-20261012/after-sword.png --exit
target/alt-e/release/sekiro-rs.exe --arena senpou-screens --brain scripted --brain-memory off --script target/alt-e/monkeys-continuation-20261012/hidden-catch-facing.txt --shots 180:target/alt-e/monkeys-continuation-20261012/facing-after-sword.png --exit
target/alt-e/release/sekiro-rs.exe --arena senpou-screens --brain scripted --brain-memory off --script target/alt-e/monkeys-continuation-20261012/hidden-catch-turn.txt --shots 180:target/alt-e/monkeys-continuation-20261012/turn-after-sword.png --exit
```

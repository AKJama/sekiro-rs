# Group c boss combat - 12 October 2026

This is the combat pass following arena staging at b082d8a.
All research reads the installed cache without modifying it.
The staging documents' missing-model statements are historical: c7400, c5430 and c5400 GLBs are now present.
No boss in this report is claimed to meet the user's complete graphical and every-attack DONE standard solely from these headless checks.

## Emma, the Gentle Blade - one phase

The existing registry and `shura` arena already select c7400, NpcParam 74000010, think 74000000, entity 1110900 and part c7400_0001.
The shared loader runs the installed 740000 Lua logic and battle chunks from m11_01_00_00.luabnd.d, c9997/c7400 HKS, the behavior graph, skeleton, clips, root motion and TAE.
Hit shapes come from her own FLVER dummy rig and BehaviorParam/AtkParam rows; no custom timing, hit shape or damage table was introduced.
Her data gives 1595 HP, 750 posture and one deathblow mark, with boss-bar name 907400.
The idle-roof regression records five Lua attack requests and five contacts, with no Lua or HKS errors.
The pressure regression records nine blocks and four deflects against Wolf's scripted sword inputs.

TAE a000_003005 opens judge 50 at 1.633 seconds and counter judge 940 at 1.600 seconds.
TAE a000_003013 opens judge 130 at 0.900 seconds and counter judge 940 at 0.867 seconds.
Both thrusts accept a timed forward step through ThrowParam 17400190 and inflict 90 counter posture damage without an incoming contact.
The sweep a000_003006 opens judge 60 at 1.067 seconds, warns through judge 982 and contacts a standing Wolf.
The same sweep misses a jumping Wolf in the counter regression; that assertion proves evasion, not a successful kick-off counter.
The grab a000_003025 opens judge 250 at 1.800 seconds and warns through judge 987.
It catches a guarding Wolf through ThrowParam 27400000, plays the paired throw and applies judge 251's 128 HP damage at 2.300 seconds of a000_004100.
The ordinary shared bullet runtime supplies the perilous warnings; these tests do not substitute a warning timer.

The fight-through regression uses Wolf's real attack input to break posture, then starts the real deathblow pair.
Only the initial remaining-posture value is lowered to keep the test focused and short; HP, mark consumption, throw timing and event messages are not synthesized.
The actual map script raises 9315 and hands over to c5430 with two marks after Emma's finishing animation and cinematic boundary.
The prior staging test's direct HP/mark mutation is not used as combat completion evidence.

The new engine change supplies Lua `IsInsideTargetRegion` with the same installed MSB sphere, cylinder and box volumes that map events use.
The original run logged that query as unknown and answered false; the updated idle run has no unanswered AI query.
A real-data method-boundary regression tests Wolf inside roof region 1112612 and then outside it, checking true and false through the Lua host interface.
The map context is installed whenever a reviewed boss AI loads, including reset and sequential hand-over.
Unsupported region shapes, unknown region ids and absent map contexts still answer false, so this is not a complete MSB region dispatcher.

No graphical replay or comparison with a recording of the original game has been performed in this session.
The tests verify the two thrusts, sweep, grab, sword pressure and final hand-over; they do not enumerate and validate every normal sword action or every combo branch.
Sweep kick-off, grab escape and lethal-grab branches are not covered by Emma-specific regressions.
Animation blending, native goal scheduling, navigation, unimplemented AI interrupt categories and native cutscene playback retain the shared limitations in AI.md, DEATHBLOW.md and FIDELITY.md.
The timed fade used for Emma-to-Isshin remains the staging cinematic approximation.
These limits prevent declaring the entire Shura sequence or every Emma behavior fully DONE.

## Isshin Ashina - two phases

The installed roof placement is c5430_0000, entity 1110920, NpcParam and think 54300000.
NpcParam normalChangeAnimChrId is 5400, so c5430 uses c5400's skeleton, HKS, graph, clips and TAE while retaining its own model and combat row.
His original phase data and map-script wiring are retained.
The first real fight-through attempt records phase-one Lua actions 3016, 3029 and 3041 with three contacts, and phase-two actions 3029, 3032 and 3080 with six contacts.
The observed capacities are 1600 HP and 750 posture in both phases, with two initial marks and bar 905401.
After the first scripted posture break and real deathblow, his shipped get-up restores HP and the event script raises 11115922.
The first phase initially logged GetHpLastTarget as an unanswered query from the installed shared 102000 logic.
The host now returns Wolf's actual HP from AiWorld in these single-target duels, and the checked runs have no unanswered AI query in either phase.
Selecting and retaining a last target across target switches remains unreconstructed; the reviewed duels retain Wolf as their sole enemy target.
The first complete deathblow attempt fails because queued event 20200 interrupts ThrowDef13700 before its message 50, leaving the boss in DeathIdleDefault without defeat flag 9316.
The local trace confirms Wolf Event710206 and boss Event20200 both begin, rather than the requests merely being absent.
The new isshin.rs helper defers command 20200 while the current installed TAE still has a message-50 window ahead or active.
This request-ownership reconstruction is inferred, not traced native code, and adds no replacement duration or synthetic victory flag.
The finishing TAE is a000_013700, with message 40 at the start and message 50 from 11.000 to 11.100 seconds.
The shared player finishing-request and TAE-936 support was added concurrently by another agent and is not claimed as group-c work.
The initial test incorrectly treated a242_511700.bin as a required physical file; its installed alias is a243_511700 and the corrected test resolves aliases through ClipLibrary.
The separate fire helper is entity 1110921, part c1000_0006, NpcParam 10006000 and think 0.
Its model and skeleton extracts are present.
Event 11115944 responds to boss SpEffect 3540510 by requesting 35 behavior-254000382 shots from MSB points 1112960 through 1112994.
Those 2003[5] instructions were previously skipped; they are shot-emission points, not helper warps.
The new isshin_fire.rs path emits Bullet 54000382 and its installed child/interval definitions at those points through the normal bullet simulation.
Static c1000 ownership survives both child and interval launches, with its own placed NpcParam 10006000 supplying combat scaling rather than Isshin's animation effects.
The event emitter's temporary combat-source substitution is restored after each contact, so it does not replace the boss's vitality or posture.
Emitter stats and source transforms are prepared during fight-event initialization, keeping registry and MSB reads out of the attack frame.
The current host supports this fight's region source, dummy -1 and zero angle overrides; other event shooters, character dummies and nonzero overrides remain unhosted.
The complete installed MSB contains 33 of the 35 event-referenced points, 1112962 through 1112994; 1112960 and 1112961 are unresolved and are skipped rather than assigned invented positions.
Whether those two references acquire a native runtime source outside this MSB remains unverified, so the host is not claimed to reproduce all 35 emissions.
No extraction command is requested for those ids because the required original MSB and EMEVD files are present; this is an unresolved source-reference question rather than a missing export file.
The focused fire regression forces the original wind-up act 3032, whose TAE adds 3540510 at 3.000 seconds, and checks 33 parent shots, a damaging child contact at an original source point and preservation of both source and boss capacities.
Its parent Bullet 54000382 lasts 1.5 seconds and starts child 54000383, whose 0.1-second attack is AtkParam 54000380 with the 1.5-metre param radius.
Act 3080 is the subsequent sword flurry, not the floor-fire trigger; the real phase-two Lua run selects both 3032 and 3080.
The counter regression Mikiri-counters standalone thrust 3026 through row 15430190 for 90 posture damage, jumps over sweep 3005 and catches a guarding Wolf with grab 3104 through row 25430000 for 256 throw damage.
The combined kick/thrust act 3003 was not validated as a Mikiri fixture because the initial contact interrupted that replay; no counter probability or timing was changed to force it to pass.
The final fight-through consumes both marks through actual Wolf attacks and throw animations, restores HP after the first mark, and raises 9316 and the real map defeat command after the final message.
Graphical fire effects and an original-game comparison remain unverified, so phase two is not declared DONE.
Every fire-attack variant, complete One Mind branch behavior, jump kick-off, grab escapes and lethal-grab branches still require further verification before either phase can be declared fully DONE.

## Isshin, the Sword Saint - four phases including Tomoe

| Phase | Implemented and checked | Remaining verification |
| --- | --- | --- |
| 1 - final Tomoe, one mark | Own Lua, c7110 model/TAE, damage contacts, real paired deathblow and map hand-over in the continuous replay; graphical arena capture | Complete Mortal Blade, combo, projectile and counter coverage |
| 2 - Isshin sword, three marks | Own Lua, c5400 model/TAE, 2240 HP/660 posture, Mikiri, sweep evasion, actual first mark/get-up and spear/storm hand-over; graphical start capture | Complete guard/deflect and sword-branch coverage; natural graphical hand-over |
| 3 - Isshin spear/pistol, two marks | Native a100 group and phase think, Lua contacts, pistol damage, spear Mikiri/sweep evasion and second real get-up | Full gun branches, jump kick-off and natural graphical phase replay |
| 4 - Isshin lightning, one mark | Own Lua lightning selection, airborne capture/reversal damage, last mark, persistent kneeling prompt, separate Mortal Blade execution and real map defeat | Other lightning variant, full attack coverage and complete graphical finishing presentation |
| Inner rematch, three marks | Separate entity/think/bar, own Lua in every phase, both get-ups, delayed execution and original 9347 completion branch | Inner-exclusive attack/counter coverage, Gauntlet manager and full graphical finish |

The existing `silvergrass` row begins with final Tomoe entity 1120830 and hands over to story Isshin entity 1120860.
`silvergrass-genichiro` is the distinct opening prologue encounter, not the final Tomoe row.
`silvergrass-isshin` selects the story Isshin fight directly.
Entity 1120890 is the rematch placement, distinct from the story row selected by the staged arenas.
The continuation's initial real-Lua replay observes final Tomoe acts 3014, 3015 and 3085 with three contacts, 2556 HP, 750 posture and one mark.
Direct story Isshin observes acts 3004, 3027 and 3028 with three contacts, 2240 HP, 660 posture and three marks.
Both initial phases report no Lua/HKS errors and no unanswered AI query.
The first completion attempt exposed c5400's second non-final deathblow selecting a000_012000, whose TAE clears the spear animation offset, instead of a100_012000.
The small shared throw hook now retains c5400's installed defender animation group, as already done for other multi-set bosses.
Sword Saint's a100_013700 finishing pair carries message 20 at 13.000 seconds; the following a100_012900 continuation carries defeat message 30 at 11.633 seconds.
The reused finishing helper now waits for that message-20 window for c5400, while retaining Ashina's message-50 policy for c5430.
The request hand-off remains inferred engine hosting rather than a native executable trace.
The new sword_saint.rs integration file uses scripted Wolf attacks and reduced remaining posture on the arena floor to exercise actual mark consumption and phase scripts.
No HP, marks, SpEffects or victory flags are injected by its completion fixture.
The later real-Lua replay observes spear-phase acts 3000, 3012, 3020, 3023, 3027 and 3028 with three contacts, and last-phase acts 3000, 3004, 3005, 3012, 3013, 3015, 3021 and lightning 3056 with three contacts.
Both phases restore 2240 HP, retain 660 posture capacity and report no unanswered AI query.
The first complete finishing replay reaches flag 9312 through the separate row 15400590, with Wolf's a243_510900 and Isshin's a100_012900.
The first fixture incorrectly approached that row from the front; its actual ThrowParam requires the rear 45-degree cone and a maximum 3.1-metre distance.
The Mortal Blade continuation has no separate grab-start row, so it uses the existing direct-body timing host instead of inventing a lunge.
The final replay verifies the persistent Event21000 kneeling loop, its SpEffect 220400 and a six-second delay before Wolf executes the separate Mortal Blade pair.
The HKS-visible HP floor preserves the execution sequence after the final mark while combat HP remains zero; this is an inferred engine request/death-ownership model.
The two queue requests are 20200 for the kneeling transition and 21000 for the waiting execution state, not interchangeable death clips.
The installed c9997 Event20200 update does not poll the throw-transition request, so the small boss-specific host delivers queued 21000 to the graph when the transition's authored action flag 152 opens at 5.300 seconds.
The real c5400 Event21000 HKS update then owns the looping a100_021000 and the execution opening; no replacement animation, duration or victory flag is introduced.
Sword-set act 3026 and spear-set act 3016 both use Mikiri row 15400190 and inflict 90 posture damage without an incoming contact.
Sword sweep 3005 and spear sweep 3006 warn through the normal bullet path and miss a jumping Wolf; jump kick-off remains unverified.
Spear-set act 3025 emits the installed pistol Bullet 54000810 or 54000811 through the ordinary bullet simulator and damages Wolf at six metres.
Act 3056 captures lightning airborne at six metres and a real Wolf attack reverses it, applies shock 9445 and reduces Isshin's HP.
The three-metre lightning fixture missed rather than demonstrating an engine defect, and no hit shape or param was enlarged to make it pass.
Final Tomoe and opening prologue Genichiro retain their own installed Lua under the requested scripted CLI mode instead of the castle action-plan table.
The rematch identity now retains entity 1120890's placed think 54030000, rather than Registry's first same-NpcParam placement.
The new optional `silvergrass-inner-isshin` row exposes that placement without changing any original arena's selection.
Its event source is m11_02_54_01, its bar is 905403, its spear think is 54030100 and its rematch completion flag is 9347 under the original 2310 branch.
Its event and AI region queries resolve the original Reservoir MSB, since the rematch changes event scripts rather than providing a separate map export.
The rematch regression runs its own Lua through all three marks, retains a100 across both get-ups, waits at the execution prompt and reaches defeat flag 9347 through the real message 30.
The reviewed story tests cover all four phases continuously, including final Tomoe's actual paired deathblow and map-driven hand-over rather than direct mark mutation.
The rematch HUD now reads the installed English name for bar 905403, Inner Isshin, instead of inheriting story Isshin's same-NpcParam name.
Native Lua attacks, model posing, named bars and the correct opening marks were captured through both arena CLI forms for final Tomoe, story Isshin, prologue Genichiro and the separate Inner start.
The forced-3015 capture shows an a100 pose but retains three marks and is not evidence of a naturally completed second phase or a full graphical fight-through.
No phase is declared fully DONE without original-game comparison, complete move/branch coverage and verified full finishing presentation.
The shared native navigation, goal scheduler, interruption and animation blending approximations remain in force.
Complete gun combo branches, both lightning variants, sweep kick-off, every deflect reaction and the shared c5400 grab row's applicability to these Lua goals remain unverified.
Double-jump fixtures at 1.6, 2.4 and 4.0 metres entered AirKick during the real sweep but did not contact the lunging boss, so they do not establish a successful kick-off or justify changing the hit shapes.
Counter fixtures now project both actors onto their actual collision floors; no mesh radius, root motion or combat param was changed to force a contact.
The shared paired-clearance solver still reports exhausted clearance in a focused deathblow fixture and retains authored alignment, so obstruction handling is approximate even though completion succeeds.
The rendered field has bright, sparse vegetation and a debug HUD; this combat pass did not tune scene materials, weather or lighting.
The native rematch return-to-hub, rewards and Gauntlet manager are not a campaign implementation; the supported completion path is its original 2310 branch to 9347.
Sword Saint's param family contains grab AtkParam 54000280, but this session does not infer that Sword Saint's own Lua selects Ashina's grab solely because they share c5400.

## Sources and verification

The primary evidence is the local MSB layout, installed TAE and params, original Lua bytecode and the simulation's scripted reports.
FwLo's read-only MIT enemy.md and deathblow_groups.md were cross-checked for animation-donor and throw-family expectations.
His enemies.md and boss_events.py describe the final throw/event-message pitfall and reinforce that a load smoke or direct mark mutation is insufficient fight-completion evidence.
His boss_scripts.py, boss_arenas.py and map.md were reviewed for the distinction between fight setup, phase script and arena placement.
No reference implementation code was copied.

All Cargo commands use `$env:CARGO_TARGET_DIR='target/alt-c'`, `$env:CARGO_NET_OFFLINE='true'` and `-j 3`.
`cargo test --release -p sekiro-sim --test it -j 3 'emma::' -- --nocapture` passes all six tests in 9.85 seconds.
The later full regression run passes the same six Emma tests in 12.33 seconds while other agents compile concurrently; this is aggregate suite time, not a single test duration.
`cargo test --release -p sekiro-sim --test it -j 3 'isshin_ashina::' -- --nocapture` passes all three tests, most recently in 11.49 seconds with no unanswered AI query.
The same three tests passed in 6.85 seconds immediately before the last-target HP host addition.
The isolated `isshin_ashina::isshin_ashina_lua_fights_both_marks_and_finishes_from_map_messages` run with `--nocapture --test-threads=1` passes in 9.00 seconds, so the complete fight remains in the normal tier.
The required regression filters are `owl`, `genichiro`, `brain_tactics::`, `attack_power::` and `posture_recovery::`, using the same release test command and three jobs.
Owl passes 60 tests in 20.66 seconds, Genichiro passes 15 in 9.56 seconds, brain tactics passes 19 in 23.08 seconds with its existing slow sweep ignored, and attack power passes three in 4.75 seconds.
Posture recovery passes all four tests in 7.36 seconds, completing the required regression filter run with no failures.
The initial two-test run passed in 5.56 seconds and exposed IsInsideTargetRegion as the only unanswered idle query.
The initial region regression failed because its direct host call omitted Lua's implicit self argument; the corrected method-boundary test passes.
Direct rustfmt is used only on this session's new Rust files.
All five new Rust files pass direct `rustfmt --edition 2024 --check`.
Workspace formatting inspection reports appended module ordering and concurrently unfinished edits in other agents' files; only this session's added code was formatted, without reordering shared module lists.
Workspace/all-target Clippy passed before the concurrent ape HUD changes, including a 15.05-second pass with the fire-emitter bridge.
The first release game build passed in 32 minutes 22 seconds while filling the new target directory's Bevy dependencies.
Later final Clippy and game-build attempts encounter private Panel/Fill types in another agent's ape_hud.rs and its system registration in arena_c_scene.rs.
Those files remain untouched under the shared-tree instruction, and the failed final attempts are not represented as successful checks.
The final scoped `cargo clippy -p sekiro-sim --all-targets -j 3 -- -D warnings` passes in 15.47 seconds, confirming the simulation and its tests are warning-free despite the game-side blocker.
An earlier transient compile failure in headless_ape.rs also disappeared after waiting for the owner to expose swap_screen_player.

```powershell
$env:CARGO_TARGET_DIR='target/alt-c'
$env:CARGO_NET_OFFLINE='true'
cargo test --release -p sekiro-sim --test it -j 3 'emma::' -- --nocapture
cargo test --release -p sekiro-sim --test it -j 3 'isshin_ashina::' -- --nocapture
cargo test --release -p sekiro-sim --test it -j 3 'isshin_ashina::isshin_ashina_lua_fights_both_marks_and_finishes_from_map_messages' -- --nocapture --test-threads=1
cargo test --release -p sekiro-sim --test it -j 3 owl -- --nocapture
cargo test --release -p sekiro-sim --test it -j 3 genichiro -- --nocapture
cargo test --release -p sekiro-sim --test it -j 3 'brain_tactics::' -- --nocapture
cargo test --release -p sekiro-sim --test it -j 3 'attack_power::' -- --nocapture
cargo test --release -p sekiro-sim --test it -j 3 'posture_recovery::' -- --nocapture
cargo check -p sekiro-sim --tests -j 3
cargo clippy --workspace --all-targets -j 3 -- -D warnings
cargo clippy -p sekiro-sim --all-targets -j 3 -- -D warnings
cargo build --release -p sekiro-game -j 3
rustfmt --edition 2024 --check crates/sim/src/isshin.rs crates/sim/src/isshin_fire.rs crates/sim/src/lua_ai/regions.rs crates/sim/tests/it/emma.rs crates/sim/tests/it/isshin_ashina.rs
cargo fmt --all --check
cargo run --release -p sekiro-formats --example emevd_dump -j 3 -- cache/raw/event/m11_01_00_00.emevd 11115941 11115942 11115944
cargo run --release -p sekiro-sim --example msb_parts -j 3 -- m11_01_00_00
target/alt-c/release/examples/emevd_dump.exe cache/raw/event/m11_01_00_00.emevd --find 1112960
```

The event and part research commands pass and are read-only.
Additional temporary original Rust probes under target/alt-c trace finishing states and inspect the installed logic's HP-query call through the existing Lua bytecode reader.
They were compiled with `rustc --edition 2024`, `-L dependency=target/alt-c/release/deps` and `--extern sekiro_sim=<the matching libsekiro_sim rlib>`, with their output binaries also under target/alt-c.
The first finishing trace reproduces missing victory, a diagnostic request deferral reaches victory, and the tracked regression verifies the actual implementation without HP or flag injection.
No graphical run, model/API call, network request, cache/re mutation or git operation was performed.

## Changed files

The new production module is `crates/sim/src/lua_ai/regions.rs`.
The finishing-request policy lives in the new `crates/sim/src/isshin.rs`.
The Shura fire-emitter loading and point-shot bridge live in `crates/sim/src/isshin_fire.rs`.
Small additive registrations and hooks are in `crates/sim/src/lua_ai/mod.rs`, `runtime.rs`, `duel.rs` and `crates/sim/tests/it/main.rs`.
The event-shot decoder and bullet-source propagation add small hooks to `events.rs` and `bullets.rs`, while `fight.rs` adds static-emitter contact resolution with independent param scaling.
Emma's six focused regressions live in `crates/sim/tests/it/emma.rs`.
The Isshin Ashina fight-through regression lives in `crates/sim/tests/it/isshin_ashina.rs`, with additive registration in the shared test module table and lib.rs.
This document and the dated STATUS.md entry record the evidence and limits.
No cache, re, existing logs, game install, save or git-state write was performed.

## Sword Saint continuation verification - 12 October 2026

All continuation Cargo commands use `CARGO_TARGET_DIR=target/alt-c`, `CARGO_NET_OFFLINE=true` and three build jobs.
The final focused suite has four tests: a continuous four-phase story fight, the three-phase Inner fight, the two-set perilous fixtures and pistol/lightning behavior.
The continuous story fight passed alone in 8.41 seconds and remains in the normal tier.
The four-test run after projecting counter fixtures onto collision passed in 10.15 seconds aggregate, and the later HUD-name run passed in 19.01 seconds aggregate while builds and graphical replays ran concurrently.
Both completion fixtures use real Wolf attacks, reduced remaining posture and explicit short-range placement; they are not proof of natural difficulty or a complete no-assistance victory.
The full story fixture carries Wolf through Tomoe's real throw, map hand-over, all Isshin marks, both get-ups, the persistent final prompt and the separate execution.
Each tested Lua phase produces at least three distinct authored attacks and three contacts without Lua/HKS errors or unanswered AI queries.
The required combined regression command passes 102 tests with one existing slow test ignored in 105.44 seconds, including the Ashina regression for the reused helper.
The simulation/all-target Clippy check passes with warnings denied.
Workspace/all-target Clippy passes after the other agent repairs the shared ape HUD visibility.
The release game build passes in 1 minute 38 seconds after the same repair, and the final HUD-name build passes in 3 minutes 09 seconds while other agents continue compiling.
Earlier build/lint attempts hit private ape Panel, Fill and Deathblow types, temporarily private brown-deathblow access and a concurrent private BehaviorRuntime graph read in divine_dragon.rs.
Those files were left to their owners and the checks were retried rather than repaired by group c.
Direct rustfmt checks cover this continuation's two modules and its integration file; workspace formatting inspection still shows concurrently edited shared files and additive module ordering.
No workspace formatting rewrite is performed.
The standard AppData Sekiro saves were copied read-only into a fresh ignored target/alt-c backup before graphical execution, without modifying the five original files.
Only the reimplementation was run, with scripted control, god mode and brain memory disabled.
The original game, online services, model APIs, cache/re, existing logs and git state were not modified or invoked.

```powershell
$env:CARGO_TARGET_DIR='target/alt-c'
$env:CARGO_NET_OFFLINE='true'
cargo test --release -p sekiro-sim --test it -j 3 'sword_saint::' -- --nocapture
cargo test --release -p sekiro-sim --test it -j 3 sword_saint_four_phases -- --nocapture --test-threads=1
cargo test --release -p sekiro-sim --test it -j 3 sword_saint_thrust -- --nocapture
cargo test --release -p sekiro-sim --test it -j 3 -- owl genichiro brain_tactics:: attack_power:: posture_recovery:: isshin_ashina:: --test-threads=3
cargo clippy -p sekiro-sim --all-targets -j 3 -- -D warnings
cargo clippy --workspace --all-targets -j 3 -- -D warnings
cargo build --release -p sekiro-game -j 3
rustfmt --edition 2024 --check crates/sim/src/sword_saint.rs crates/sim/src/isshin.rs crates/sim/tests/it/sword_saint.rs
cargo fmt --all --check
```

The earlier `sword_saint_thrust` double-jump probes fail to establish kick-off, and the retained test honestly checks Mikiri and sweep evasion rather than weakening a kick-off assertion into a claimed success.
The final no-kick-off suite passes after that coverage distinction; the missing counter coverage remains documented above.
Research uses the read-only EMEVD dump executable for m11_02_00_00 and m11_02_54_01 events 11125830, 11125842, 11125860, 11125870 and 11125872-11125877.
The original HKS disassembler is run read-only on c9997.hks and c5400.hks to inspect Event20200, Event21000, EventCommonFunction and throw-transition request dispatch.
The relevant original reference cross-check is FwLo's enemies.md animation-group/get-up description and his deathblow_groups.md a242 family; final pair 15400590 and rematch events are verified from this checkout's own params and EMEVD rather than copied from his code.

The idle capture input is a single `900 0 0 -` line under target/alt-c.
The following graphical commands exit successfully and save their images under target/alt-c.

```powershell
target/alt-c/release/sekiro-rs.exe --arena silvergrass-isshin --god --brain scripted --brain-memory off --script target/alt-c/sword-saint-idle.txt --shots 500:target/alt-c/sword-saint-sword.png --exit --window-size 1200x800
target/alt-c/release/sekiro-rs.exe --play --arena silvergrass --god --brain scripted --brain-memory off --script target/alt-c/sword-saint-idle.txt --shots 750:target/alt-c/sword-saint-tomoe.png --exit --window-size 1200x800
target/alt-c/release/sekiro-rs.exe --play --arena silvergrass-genichiro --god --brain scripted --brain-memory off --script target/alt-c/sword-saint-idle.txt --shots 750:target/alt-c/silvergrass-prologue.png --exit --window-size 1200x800
target/alt-c/release/sekiro-rs.exe --arena silvergrass-isshin --god --brain scripted --brain-memory off --force-act 3015 --script target/alt-c/sword-saint-idle.txt --shots 500:target/alt-c/sword-saint-spear.png --exit --window-size 1200x800
target/alt-c/release/sekiro-rs.exe --play --arena silvergrass-inner-isshin --god --brain scripted --brain-memory off --script target/alt-c/sword-saint-idle.txt --shots 750:target/alt-c/sword-saint-inner.png --exit --window-size 1200x800
```

The first Inner image exposed the inherited story name, which prompted the native-FMG HUD correction and its regression assertion.
The repeated command changes only its screenshot destination to target/alt-c/sword-saint-inner-fixed.png for verification against the rebuilt executable.
That corrected run exits with code 0, and the inspected image shows Inner Isshin with 2240 HP, three marks and real 54000420 damage contacts.
The final workspace/all-target Clippy run passes in 17.09 seconds with warnings denied, and direct rustfmt checks pass for all three continuation Rust files.
Tomoe's capture logs real AtkParam 71100120 damage, the prologue capture logs 71000370 damage and a real paired grab, and Inner's capture logs its own 54000420 damage.
The source modules changed in this continuation are sword_saint.rs and isshin.rs.
Small shared additions are in registry.rs, enemy.rs, arena.rs, arena_c.rs, duel.rs, lua_ai/regions.rs, lib.rs and tests/it/main.rs.
The new boss integration file is tests/it/sword_saint.rs, and the documentation changes are this file and STATUS.md.
No earlier Emma or Ashina behavior is reimplemented in this continuation.

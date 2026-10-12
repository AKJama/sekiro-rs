# Group c: Reservoir and Sunken Valley arenas

The rows below come from the installed MSB layouts and `cache/raw/event/<map>.emevd`, with fog definitions read through common events 20005820 and 20005825.
No game data was copied into source or changed in cache.
The arena code, event registrations and headless staging are implemented, but missing character GLBs prevent graphical completion of Isshin and both ape encounters in this checkout.
Boss attack fidelity and simultaneous companion combat remain separate work.

## Ashina Reservoir silvergrass

Map: `m11_02_00_00`, with a 100 m collision radius around the selected boss and the normal full-map scenery renderer.
`--arena silvergrass` starts final-game Genichiro, Way of Tomoe, entity 1120830, part `c7110_0000`, NpcParam 71101000 and think 71100000.
`--arena silvergrass-isshin` starts story Isshin, entity 1120860, part `c5400_0000`, NpcParam and think 54000000.
Isshin 1120890, part `c5400_0001`, uses the same NpcParam with rematch think 54030000 and is disabled by story event 11125370, so it is deliberately not the story boss.
Both final-game entries put Wolf at MSB region 1122800, the warp used by cutscenes 11020030 and 11020040 in events 11125840 and 11125830.
The exported player parts are outside the silvergrass fight; their positions are not substituted for that authored warp.
Final-game story flags include 8302, 9300, defeated time bosses 9303-9306 and cumulative time flags 9800-9802, selecting night at 22 h.
Final Tomoe starts with 11125831; direct Isshin starts with 9311, 11120830, 11125830 and 11125861.
Final Tomoe runs 11125840 from Label2, 11125830 and 11125842.
Isshin runs 11125870 from Label1, 11125860, 11125872, 11125873, 11125874, 11125875 and 11125877.
The Tomoe row carries both fog event 11125841 and successor fog event 11125871, which refer to the same asset 1121800 and SFX 10 with different lifecycles.
The first lifecycle is start 11125831 and end 11125830; Isshin's is start 11125861 and end 9312.
Neither fog event starts common 20005825; the plane blocker is derived from the fog asset's extracted bounds.
Event 50 disables collision 1124460 and 1124420 and the alternative 1124410-1124414 collision set for the final encounter.
Event 11125877 independently switches collision 1124860, `h900900`, off/on on Wolf's messages 90/100, without changing the entrance fog.
Event 11125872 changes Isshin's think row to 54000100, hides model group 1127820, requests GPARAM sub-ID 30 over five seconds and changes the map's storm SFX.
Event 11125860 restores group 1127820 and GPARAM sub-ID 20 after the finishing message and victory delay.
The final intro's sky group 8907075 selects the extracted `m900075` sky; the renderer now loads this previously omitted model.
GPARAM changes use the extracted `m11_02_0930.gparam` storm and `m11_02_0920.gparam` clear variants and currently snap rather than interpolate over the requested five seconds.
The 09 file family is inferred from the map's sole extracted sub-ID-30 preset, rather than a traced native file-selection algorithm.
The test resolves all three story starts against real collision, loads each simulation and its boss bar/fog, advances Tomoe's completed-mark state through the actual simulation hand-over to Isshin 1120860 with bar 905400, and checks Isshin's storm flag, model state and GPARAM request.
A separate small geometry regression proves the finishing boundary switches independently of fog.
Missing graphical dependency: `cache/models/c5400.glb`, required by both direct Isshin and the continuous Tomoe-to-Isshin fight.
The available c7110 simulation and model load, but a complete fight through Isshin cannot be shown until that GLB exists.
Native cutscene playback, story rewards, paired player event animations and exact storm blending remain approximations or separate engine work.

### Opening Genichiro at the same field

`--arena silvergrass-genichiro` selects entity 1120800, part `c7100_0000`, NpcParam and think 71000000.
Wolf starts at MSB region 1122801, the post-intro warp in event 11125810 and cutscene 11020010.
The stage joins 11125810 after its intro, preserving its HP-bar setup and actual fight suffix, and runs outcome event 11125800 and boundary immortality event 11125812.
Flags 8306, 11120810 and 11125810 describe the started prologue encounter.
Fog event 11125811 places asset 1121800, SFX 10, start 11125810 and end 9300, without a 20005825 collision.
The prologue swaps collision 1124400-1124404 off and 1124410-1124414 on, as event 11125810 does.
Common event 100 selects environment 0 and sky group 8907040 under 8306 without the cumulative time flags, so the stage uses the authored 0 h light and `m900040` sky.
The morning lighting named in event 11125800 belongs to the post-fight warp to the temple, not the fight's initial lighting.
Both c7100's model and simulation assets are present; the focused headless run proves placement, stable Wolf floor, the named boss bar and standing fog.
The original prologue's story continuation and cutscenes are not a playable campaign implementation.

## Sunken Valley watering hole

`--arena watering-hole` uses map `m17_00_00_00`, Guardian Ape entity 1700800, part `c5100_0000`, NpcParam and initial think 51000000, with a 110 m collision radius.
Wolf starts at exported player part `c0000_0009`, entity 1700978, on the approach overlooking the watering hole; `c0000_0004` is essentially on top of the ape and is not a fight-entry start.
This player-part choice is a reviewed approach-stage choice, not a cutscene warp claim.
The canonical post-Genichiro route seeds defeated flag 9303, choosing afternoon at 12 h; Guardian Ape can be reached in other campaign orders.
Flags 11700801 and 11705801 describe the already-triggered encounter.
Fog event 11705811 places asset 1701800 with SFX 12, start 11705801 and end 9304, without a 20005825 boundary.
The running events are 11705810, 11705800, 11705820-11705827, 11705833 and 11705835.
11705810 retains the real setup before its entry wait and joins the fight suffix after that wait.
11705821 waits twelve seconds after message 20, requests command 20000, changes the think row to 51000100 and changes the phase SpEffects.
11705835 handles the false-victory bar timing; 11705800 handles true victory, defeat flag 9304 and fog removal.
The finishing centipede is entity 1700801, part `c5001_0000`, and remains a supporting actor rather than replacing the ape in the duel.
The test proves both character starts have hit collision, Wolf remains on his approach floor, the event bar/fog start, the phase wait is respected, and true victory clears the fog.
Missing graphical dependencies are `cache/models/c5100.glb` and `cache/models/c5001.glb`.
No graphical ape placement is claimed while its GLB is missing, although the original simulation assets load successfully.
Finishing centipede dummy warps and animations are wired but cannot be visually checked without the GLB; player event animation, character-part state instructions, custom camera sub-IDs and the false-victory banner are not fully rendered by the current engine.
No Guardian Ape attacks were authored or tuned in this task.

## Sunken Valley ape's burrow

`--arena apes-burrow` uses map `m17_00_00_00`, Headless Ape entity 1700850, part `c5100_0001`, NpcParam and think 51000100, with an 80 m collision radius.
Wolf starts at player part `c0000_0007`, entity 1700976, on the burrow's real helper collision near the boss.
The stage seeds defeated flags 9303 and 9304 for afternoon at 12 h and leaves severance flag 9314 off.
Common event 912 enables story flag 8303 after 9304 while 9314 is off; the stage seeds that same state with 11700851 and 11705851.
Fog event 11705861 places two assets, 1701850 and 1701851, each with SFX 12, start 11705851 and end 9307.
There is no common 20005825 boundary in that event.
The running events are 11705860, 11705850, 11705870-11705873, 11705880 and 11705884.
11705860 retains the real setup and joins after the player-entry wait.
11705880 requests the main ape's event-driven reposition, waits for region 1702871 and three seconds, then enables brown ape entity 1700852, part `c5100_0002`, NpcParam 51000010, think 51000011, and its appearance animation 20020.
The renderer retains that companion as a separate posed actor with event visibility, animation and draw masks, and 11705884 can dismiss it.
It has no simultaneous combat simulation in this staging task.
The finishing centipede is entity 1700851, part `c5001_0001`, with the same missing model as the watering-hole centipede.
The test proves both starts have real collision, both fog walls and the bar start, the first-mark event produces the companion enable/animation commands, victory raises 9307 and removes the fog, and finishing message 40 raises 9314.
Missing graphical dependencies are `cache/models/c5100.glb` and `cache/models/c5001.glb`.
Both centipede placements have renderer registrations and use the event's one-shot warp to the main ape's animated dummy 260, but their missing GLB prevents visual verification.
Character-part state instructions, paired finishing player animation and camera sub-IDs remain engine gaps, and both apes' attack fidelity is separate work.

## Validation

All Cargo commands use `$env:CARGO_TARGET_DIR='target/alt-c'` and four build jobs.
The focused test command is `cargo test --release -p sekiro-sim --test it -j 4 '::group_c_' -- --nocapture`.
Initial placement/phase verification passed all three arena tests in 9.53 seconds before the final boundary and companion assertions were added.
The release game build command is `cargo build --release -p sekiro-game -j 4`.
The required lint command is `cargo clippy --workspace --all-targets -j 4 -- -D warnings`.
Formatting inspection uses `cargo fmt --all --check`; only group-c new files are formatted with direct `rustfmt --edition 2024` calls, and shared module reordering is deliberately not performed.
One early test filter, `arena_c`, also matched a concurrent Shura test's name and reported its unrelated warp assertion failure; the filter was tightened to `::group_c_`.
Early Clippy attempts encountered the concurrently removed `EventScripts.scene` field in `arenas_a.rs`; that code was left to its owner and later became compilable.
The final focused run passes four tests in 8.32 seconds, with zero failures or ignored tests.
The final workspace/all-target Clippy run passes with warnings denied, and the final release game build passes in 1 minute 58 seconds.
Direct `rustfmt --edition 2024 --check` passes for all eight group-c new Rust files.
The final workspace formatting inspection reports only the shared integration-test module ordering, which is left additive as required by the shared-tree rules.
The installed-data storm assertion caught and fixed the GPARAM command's packed 32-bit area/block layout before final verification.

Startup checks ran the following commands, all with expected exit code 1 before any window or controller was created.

```powershell
target/alt-c/release/sekiro-rs.exe --arena silvergrass --brain scripted --brain-memory off
target/alt-c/release/sekiro-rs.exe --arena silvergrass-isshin --brain scripted --brain-memory off
target/alt-c/release/sekiro-rs.exe --play --arena watering-hole --brain scripted --brain-memory off
target/alt-c/release/sekiro-rs.exe --play --arena apes-burrow --brain scripted --brain-memory off
```

Both silvergrass commands report `models/c5400.glb` missing; both ape commands report `models/c5100.glb` and `models/c5001.glb` missing.
These are asset-blocker checks, not graphical staging runs.
No screenshots, model/API calls, cache/re writes, commits, network access or git-state changes were made.

## Changed files

New staging source: `crates/sim/src/arena_c.rs`, `arena_c_regions.rs`, `arena_c_collision.rs` and `crates/game/src/arena_c_scene.rs`.
New tests: `crates/sim/tests/it/silvergrass_arena.rs`, `watering_hole_arena.rs`, `apes_burrow_arena.rs` and `arena_c_support.rs`.
Additive shared registrations and hooks: `crates/sim/src/arena.rs`, `events.rs`, `lib.rs`, `duel.rs`, `collision.rs`, `lua_ai/runtime.rs` and `crates/sim/tests/it/main.rs`.
Additive game hooks: `crates/game/src/play.rs` and `sky.rs`.
Documentation: this file, `STATUS.md` and the group-c approximation section in `docs/FIDELITY.md`.

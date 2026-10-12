# Castle gate and Shura arenas

Staging reads the local MSB layouts, hit collision and `cache/raw/event/<map>.emevd`, without changing extracted files.
The review used `emevd_dump` and the MSB region/part readers, rather than copied game scripts.
`--arena gyoubu`, `--arena demon` and `--arena shura` use the same full-map scenery loader and collision crop as the castle-top arenas.
`--play --arena <name>` selects the same rows.
The two Outskirts rows use a 120 m collision radius around their distinct boss placements, covering the battlefield approach and three fog assets.
Shura uses the roof's existing 60 m collision radius.

## Gyoubu Oniwa

Map is `m11_00_00_00`, boss entity 1100800, part `c5080_0000`, NpcParam 50800000.
Wolf starts at the battlefield's southern approach part `c0000_0008`, at `[122.400, -71.820, -1.056]` in Bevy space.
The southern approach is the reviewed start choice; Gyoubu's script tests entry region 1102810 without warping Wolf to a specific start.
The nearby `c0000_0006` and duplicate `c0000_0011` are beside the boss's gate placement and are not the approach used here.
Story time is morning, with none of the common event 820 time-advancing bosses beaten.
Flags 11100801, 11105801 and 11105803 stage the completed introduction and active fight.
Event 11105810 joins at instruction 4, retains character enable and immortality, and bypasses its introduction message/time wait when 11100801 is already on.
Defeat event 11105800 joins at instruction 5, avoiding its pre-fight character disable.
Events run are 11105800, 11105810, 11105812, 11105820, 11105830, 11105840, 11105841, 11105842 and 11105845.
They provide the boss bar 905081, mark flag 11105802, immortality branches, effect swaps and defeat flag 9301.
Fog event 11105811 initializes common event 20005820 for assets 1101800, 1101801 and 1101802, with effects 11, 12 and 12.
All three walls use start flag 11105803 and end flag 9301; this fight declares no common 20005825 collision boundary.
The test loads the real duel on map collision, checks both start floors, starts the boss bar without needing the approach trigger, and exercises defeat and release of all three walls.
The introduction presentation, camera sub-ID switches and combat-dependent region effects are not reconstructed by these tests.
The boss's attacks, mounted movement and character-specific draw masks remain the separate fight implementation task.

## Demon of Hatred

Map is `m11_00_00_00`, boss entity 1100900, part `c7020_0000`, NpcParam 70200000.
Wolf starts at `c0000_0008`, which exactly matches region 1102957 used by arrival event 11105980.
Story time is night, using the four common time-advancing defeat flags and 9802.
Flags 8302, 9301, 11100901 and 11105901 select late Outskirts, defeated Gyoubu, the retry encounter and the active boss fight.
Event 11105910 runs its retry setup, including immortality, then bypasses the completed arena-entry wait at instruction 27.
Events run are 11105900, 11105910, 11105912, 11105920, 11105921, 11105922, 11105924, 11105970, 11105971 and 11105972.
They provide boss bar 907021, second-mark flag 11105902, final-mark effect changes, finishing requests and defeat flag 9313.
The actual character-state condition 4[21] now waits through the shared EventHost interface instead of being skipped as a command.
Fog event 11105911 uses assets 1101800, 1101801 and 1101802, with start 11105901 and end 9313, and no 20005825 boundary.
The event-controlled idol object 1101955 is hidden during the fight and enabled by the defeat script when objects are rendered.
The test checks the arrival warp against Wolf's MSB start, real duel loading, night, three active walls, both mark transitions and scripted defeat.
The first-visit soldier encounter and travel interaction are bypassed by the documented retry state.
Attack-driven flame hazards, final paired execution and camera sub-ID behaviour remain the separate boss fight implementation task.

## Shura: Emma and Isshin Ashina

Map is `m11_01_00_00`, Emma entity 1110900, part `c7400_0001`, NpcParam 74000010.
Isshin is entity 1110920, part `c5430_0000`, NpcParam 54300000, with animation donor c5400.
Wolf starts at the exact cutscene warp region 1112812 used by both Shura setup events.
Player-start part `c0000_0010` is the nearest part, 0.46 m away, and is resolved before the native story warp overrides it.
Emma starts in evening from the four defeated time-advancing bosses; Isshin's event 11115930 and floor event 11115943 select `SetLighting(4)`, night.
Emma's initial flags are 8300, 8301, 9303, 11115722, 71110020, 1270 and 1276.
Emma events are 11115900, 11115910 from Label2, 11115930 from instruction 20 after the already-satisfied roof-entry conditions, and 11115943.
Boss bar 907400 appears after the setup wait; Emma's defeat sets 9315 and 11115900, then event 11115930 requests cutscene 11010080 and enables Isshin.
The existing sequential boss hand-over resolves Isshin's own MSB placement and the cutscene's Wolf warp.
The end-to-end test exposed the harness's automatic three-second enemy-death retry interrupting Emma's 9.5-second defeat script.
Map-driven fights now wait for their event progression after a boss dies; player death still retries, and fights without map events retain automatic retries.
Isshin's initial row adds 11115901, 11115921, 1290 and 1296, and stages event 11115930 at instruction 40 after its cutscene.
Isshin events are 11115920, 11115930, 11115932, 11115941, 11115942, 11115943 and 11115944.
They provide boss bar 905401, mark flag 11115922, finishing immortality removal on message 40, defeat on message 50 and final defeat flag 9316.
Emma fog event 11115911 uses asset 1111800 and boundary collision 1114800, with wall start 11115722 and end 11115900.
Isshin fog event 11115931 uses the same asset and collision with start 11115921 and end 11110100, which is the later ending state rather than his defeat flag.
The distinct fog rows prevent Emma's defeat flag from removing Isshin's gate.
Event 11115943 hides burnt group 1117551 during Emma, then shows it and hides tatami group 1117550 when 11115921 is on.
The renderer reads entity and group IDs from the original MSB, so the exported layout does not need additional fields.
The placed fire effect 1114419 resolves to FXR 811201 from the MSB's type-5 effect region.
Emma setup effects 1114837, 1114838 and 1114839 likewise resolve from the MSB and stop in the transition event.
The test exercises the actual duel hand-over as well as bar/mark/defeat event commands, the two fog flag lifecycles, the story warp and the tatami-to-burnt scene state.
Cutscenes use the existing timed fade placeholder.
Isshin's attack-controlled secondary fire character 1110921 and its region warps in event 11115944 remain the separate attack/hazard task; the event itself is registered.

## Local asset blockers and verification limits

The maps, collision, raw MSBs, EMEVDs, Lua bundles and animation donors needed by these rows are present.
`cache/models/c5080.glb`, `cache/models/c7020.glb`, `cache/models/c7400.glb` and `cache/models/c5430.glb` are missing.
Those missing model extracts prevent a complete graphical demonstration with the bosses standing visibly at their placements.
Graphical startup reports the missing files before creating a window or starting a brain controller.
No model substitute or extraction was written because this task requires treating cache as read-only.
The tests prove simulation startup and event staging, not boss attack fidelity or a visually verified full-map render.
The placed Shura effects are read from installed FXRs through the existing effect player, whose unsupported rendering features retain its documented limitations.

## Verification and changed files

Each group-b test passed individually: Gyoubu in 2.18 s, Demon in 2.54 s and Shura in 5.02 s.
The existing Genichiro two-mark and Tomoe hand-over regression also passed, in 6.19 s.
Workspace/all-target Clippy passes with warnings denied, and the release game build passes.
The first game build encountered a concurrent `arena_c_scene.rs` import error; that file was left untouched, and the later retry passed.
The initial broad `arena_` test filter also selected three concurrent group-d tests; all six passed, and subsequent runs used the exact group-b filters.
The final workspace formatting check reports only module ordering in the shared integration-test main file; the appended modules were preserved as required by the shared-tree rules.
Both new source modules and all three new test files pass direct rustfmt checks.

```powershell
$env:CARGO_TARGET_DIR='target/alt-b'
$env:CARGO_NET_OFFLINE='true'
cargo test --release -p sekiro-sim --test it -j 4 gyoubu_arena -- --nocapture
cargo test --release -p sekiro-sim --test it -j 4 demon_arena -- --nocapture
cargo test --release -p sekiro-sim --test it -j 4 shura_arena -- --nocapture
cargo test --release -p sekiro-sim --test it -j 4 castle_genichiro_loses_two_marks_and_reaches_map_handover -- --nocapture
cargo clippy --workspace --all-targets -j 4 -- -D warnings
cargo build --release -p sekiro-game -j 4
rustfmt --edition 2024 --check crates/sim/src/arena_b.rs crates/game/src/arena_b.rs crates/sim/tests/it/gyoubu_arena.rs crates/sim/tests/it/demon_arena.rs crates/sim/tests/it/shura_arena.rs
cargo fmt --all --check
target/alt-b/release/sekiro-rs.exe --arena gyoubu --brain scripted --brain-memory off
target/alt-b/release/sekiro-rs.exe --play --arena demon --brain scripted --brain-memory off
target/alt-b/release/sekiro-rs.exe --play --arena shura --brain scripted --brain-memory off
```

The three graphical entry checks exit 1 with the exact missing-model paths and the correct initial story hour; they open no window and start no brain controller.
New files are `crates/sim/src/arena_b.rs`, `crates/game/src/arena_b.rs`, and the three `crates/sim/tests/it/{gyoubu,demon,shura}_arena.rs` tests, plus this document.
Small shared-file edits are in `crates/sim/src/{arena,events,duel,lib}.rs`, `crates/sim/tests/it/main.rs`, and `crates/game/src/{main,map,play}.rs`.
`STATUS.md` records the staging milestone and asset blockers.
No commits, git-state operations, screenshots, network or model calls, or protected data/log writes were used.

# Hirata and Senpou arena staging

These rows use the local, unmodified MSB and EMEVD extracts.
Neither map nor its raw event script is missing.
The current cache lacks required character models, so none of these arenas is reported as fully DONE visually.
The headless checks can still validate their story placements, collision and event staging.
Boss combat AI, attacks and auxiliary combat simulation remain separate work.

## Hirata main hall - `hirata-hall`

The map is `m10_00_00_00`, with a 60 m collision radius.
Lady Butterfly begins as entity 1000800, part `c5090_0000`, NpcParam 50900000 and think 50900000.
Her second character is entity 1000810, part `c5090_0001`, NpcParam 50900001 and the same think row.
Wolf starts at player part `c0000_0006`, entity 1000974, at Bevy position `[231, -788, 578]`.
This stages the retry after the opening cinematic, with flags 11000801 and 11005801 set.
Event 11005810 is joined at its post-cutscene camera/setup instruction 15.
Event 11005812 waits for the first character's death before disabling it and enabling the second character.
The second row joins 11005812 at instruction 18, after its descent and region gate, to retain the bar and phase flag sequence.
Event 11005856 controls the first character's immortality, while 11005857 controls the second character's finishing immortality.
The second row also runs 11005800, 11005820, 11005850, 11005851, 11005855, 11005858, 11005860, 11005870, 11005880 and 11005881.
All eight illusion slots and both camera-event slots are instantiated from their real InitializeEvent arguments.
The illusions are MSB entities 1000820 through 1000827, parts `c1510_0000` through `c1510_0007`, using the c1500 animation donor.
Common event 20005820 is read from fog event 11005811 and places asset 1001800, SFX 11, start flag 11005801 and end flag 11000805.
There is no corresponding 20005825 boundary start in this fight's fog event.
Common event 100 selects the fixed Hirata past-night environment variant 0, represented by hour 0 and sky `m900060`.
Constructor calls to common events 20004114 and 20004115 select the original scenery and collision groups with 8304 off.
Their inactive MSB parts are hidden by exact part name, and inactive collision triangles are disabled while retaining the exported triangle ranges.
The supporting-cast renderer retains the second Butterfly and illusion actors separately from the primary opponent.
The cache is missing `cache/models/c5090.glb` and `cache/models/c1510.glb`.
The primary c5090 and supporting c1500 animation extracts are present.
Auxiliary AI status, distance/visibility conditions, illusion projectile conversion and the native cinematic descent are not fully hosted by the arena engine.
Those gaps and the missing models prevent a claim that the full illusion phase is playable or that this arena is DONE.

## Hirata Father - `hirata-father` and `hirata-inner-father`

Both rows use `m10_00_00_00`, a 60 m collision radius and Wolf's `c0000_0006` start.
Story Father is entity 1000900, part `c5060_0001`, NpcParam 50601010 and think 50600000.
Inner Father is entity 1000990, part `c5060_0002`, the same NpcParam and think 50630000.
Both boss parts have Bevy position `[226.374, -787.841, 575.669]` and yaw -116.328 degrees.
The explicit arena selection preserves Inner Father's think identity rather than resolving the shared NpcParam to the first Father placement.
Story Father reads `cache/raw/event/m10_00_00_00.emevd`.
Inner Father reads `cache/raw/event/m10_00_50_61.emevd`, while its physical placement and map remain in `m10_00_00_00`.
The unrelated `m10_00_50_90` script is the Butterfly gauntlet and is not used for Inner Father.
Both rows start with 8304 and 11005914 set, and join 11005910 after Label2.
Both run 11005900, 11005912, 11005920, 11005921, 11005922, 11005923, 11005930, 11005931, 11005935 and 11005936 from their respective scripts.
These events retain the boss bar, phase effect 200051, finishing immortality, defeat and bird enable/vanish/return requests.
Story Father's bird is entity 1000901, part `c5070_0000`, and Inner Father's bird is entity 1000991, part `c5070_0001`.
The birds begin hidden, and their enable commands do not replace the boss combatant.
Fog event 11005911 reads common 20005820 with asset 1001800, SFX 11 and start flag 11005914.
Its end flag is 9317 for story Father and 9346 for Inner Father.
There is no corresponding 20005825 boundary start in either selected fog event.
Both use Hirata's fixed past-night environment variant 0, hour 0 and sky `m900060`.
Common constructor events 20004114 and 20004115 select the later-memory scenery and collision with 8304 on.
The primary c5060 model and animations and the bird's c5070 animations are present.
The cache is missing `cache/models/c5070.glb`.
Bird attachment, synchronized combat and AI-driven vanish/return timing are not a verified playable fight here.
The missing bird model and unverified supporting fight behavior prevent a full DONE claim.

## Senpou illusion temple - `senpou-screens`

The map is `m20_00_00_00`, with a 160 m collision radius covering the separate hidden temple and all four placements.
Entity 2000800 is the invisible health controller, part `c1000_0009`, NpcParam 12600050 and think 10000000.
Its MSB position is `[857.336, -813.970, 573.064]`, below the temple, and it is not a visible monkey or a floor-supported fighting character.
The four real monkeys are entities 2000801 through 2000804, parts `c1260_0000` through `c1260_0003`, with NpcParam rows 12600000, 12600010, 12600020 and 12600030.
Their animation donor is c1250, so a separate `cache/anim/c1260` extract is not required.
The supporting apparition placements are entities 2000805 through 2000809 and 2000811 through 2000814.
There is no player part in the hidden temple in the exported layout.
Wolf therefore uses MSB warp region 2002200, named by entrance event 12005210, at `[965.605, -820.845, 542.801]`.
The start is read from the original MSB without modifying the layout extract.
Flags 8400, 9200, 12000801 and 12005801 seed the entrance-completed, unbeaten fight.
The selected story route has Genichiro defeated, flag 9303, giving afternoon hour 12 through common events 820 and 100.
Entrance event 12005210 selects illusion sky group 8907085, model `m900085`, independently of the normal Senpou story sky.
The raw m89 MSB verifies that group on part `m900085_0000`, entity 8904085, even though the exported m89 layout and sky library omit it.
The supported sky/object state before its entrance cutscene is seeded from that event's instructions 11 through 25.
Event 12005810 is joined at instruction 29 and shows controller boss bar 901260 after one second.
Event 12005813's native gravity-disable instruction keeps the hidden health controller suspended at its placement without adding a fake collision floor.
The row also runs 12005800, 12005813, 12005820, 12005825, 12005826, 12005855 through 12005860, 12005827, 12005828, 12005835 through 12005838, 12005870, 12005873, 12005875, 12005890, 12005895 and 12005900.
All four 12005820 slots select intact screens 2001830 through 2001833 and hide their defeated alternatives 2001840 through 2001843.
The screen slots remain independent rather than repeating only the first InitializeEvent argument list.
Objects named directly by the selected event instructions are staged without enabling unrelated whole-map objects.
The raw map script contains no common 20005820 or 20005825 starts for this fight, so there are no invented fog walls or fog flags.
The cache is missing `cache/models/c1000.glb`, `cache/models/c1250.glb` and `cache/models/c1260.glb`.
It also lacks the `m900085` record in `cache/maps/m89_00_00_99/sky.json` and `cache/maps/m89_00_00_99/models/m900085.glb`.
The c1000 and c1250 animation extracts and the temple map collision are present.
The optional sky loader reports the missing illusion sky and keeps a fallback until the authored asset is supplied.
Action-button activation for the bell, AI-status conditions, native room GPARAM transitions, shared four-monkey health and auxiliary combat are not fully hosted.
Unavailable interaction conditions wait instead of automatically ringing the bell or extinguishing the room lights.
The models, authored sky and interaction gaps prevent this arena from being reported as DONE.

## Verification

The three focused integration tests are registered in the existing `it` binary.
They skip when their map collision extract is absent.
They resolve real placements, check the collision floor at Wolf and floor-supported boss starts, read real fog flags, execute the real boss-bar setup and perform a simulation tick when the primary animation donor exists.
The Butterfly check also requires the second character to remain disabled while the first lives and to be enabled after its death.
The Senpou check requires all four independent intact-screen states, the authored sky-group selection and no bell activation without an action-button host.
It also steps the actual controller for half a second and requires its height to remain stable with gravity disabled by 12005813.
The hidden controller is intentionally excluded from the boss-floor assertion.
No combat effectiveness, pixel-perfect rendering or completed four-monkey defeat is claimed by these checks.

Validation commands use `$env:CARGO_TARGET_DIR='target/alt-a'` and four build jobs.
The final command outcomes are recorded in the session report and STATUS.md.
No cache/re writes, game-install writes, screenshots, network calls, real brain calls, commits or Git operations were used.

### Commands and final results

```powershell
$env:CARGO_TARGET_DIR='target/alt-a'
$env:CARGO_NET_OFFLINE='true'
cargo build --release -p sekiro-formats --example emevd_dump -j 4
cargo test --release -p sekiro-sim --test it -j 4 hirata -- --nocapture
cargo test --release -p sekiro-sim --test it -j 4 senpou_screens -- --nocapture
cargo clippy --workspace --all-targets -j 4 -- -D warnings
cargo build --release -p sekiro-game -j 4
cargo fmt --all --check
rustfmt --edition 2024 --check crates/sim/src/arenas_a.rs crates/sim/tests/it/arenas_a_support.rs crates/sim/tests/it/hirata_hall_arena.rs crates/sim/tests/it/hirata_father_arena.rs crates/sim/tests/it/senpou_screens_arena.rs
foreach ($arenaName in 'hirata-hall','hirata-father','hirata-inner-father','senpou-screens') {
    & .\target\alt-a\release\sekiro-rs.exe --play --arena $arenaName --brain-memory off --brain scripted --script builtin --exit
    Write-Output "$arenaName exit code: $LASTEXITCODE"
}
```

The formats example build passed and its read-only dumps supplied the event review above.
The final Hirata command passed both tests in 9.05 seconds, including both Father identities.
The Senpou command passed its test in 5.03 seconds.
Workspace Clippy passed after concurrent compile/lint repairs, with no suppressed warnings added by this task.
The final release game build passed in 2 minutes 11 seconds after its dependencies were built.
The workspace formatting check returned 1 solely for sorting the appended modules in `crates/sim/tests/it/main.rs`; the shared-tree rule forbids that reorder.
Direct rustfmt checking of all five new group-a Rust files returned 0.
Each scripted CLI startup returned 1 before opening a window, reporting exactly the missing character models listed in its arena section.
Earlier retries of these same build/check commands encountered concurrent edits in arena_c, arena_c_scene and the shared scene API, plus a corrected group-a Clippy conversion warning.
No unrelated arena source was repaired by this task.

### Files changed by this task

The new staging logic is `crates/sim/src/arenas_a.rs`.
The new tests are `crates/sim/tests/it/arenas_a_support.rs`, `hirata_hall_arena.rs`, `hirata_father_arena.rs` and `senpou_screens_arena.rs`.
Small shared simulation hooks are in `crates/sim/src/arena.rs`, `events.rs`, `lib.rs`, `duel.rs`, `body.rs`, `story.rs` and `crates/sim/tests/it/main.rs`.
Small shared presentation/CLI hooks are in `crates/game/src/main.rs`, `play.rs`, `map.rs`, `arena_d_scene.rs` and `sky.rs`.
Documentation changes are this file and `STATUS.md`.

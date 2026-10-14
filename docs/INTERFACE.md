# Start and pause menus

No arguments or `--menu` opens a Bevy UI panel with Segoe UI, keyboard, mouse and gamepad navigation.
Existing flags still dispatch directly to their original modes.
Explore map is the first and default mode, with only Map and Start number settings.
The Map row starts selected, so Enter/A immediately explores the selected start.
Number keys 1-9 and 0 select the ten listed maps directly; Enter then starts exploration.
Only extracted maps appear in the map cycle, and changing maps resets the start number to zero.
Start number wraps through that map's authored player starts.
Explore enables map objects and uses the per-map data's first-visit story state, with no hour or story override.
The menu has no Hour row; `--hour` and `--story` remain direct CLI testing controls.
Fight choices follow Explore in the Mode selector and retain their existing settings.
Up/down or the gamepad stick/D-pad select a row; left/right change values; clicking a row advances its value; Enter/A confirms Start or a pause action.
Select Enemy id or Player name and type with the keyboard; Backspace removes a character.
Enemy ids also increment with left/right for gamepad entry.
Enemy id, map/start and brain stage controls are grey when their parent choice does not use them.
Attack power accepts 1-99 via left/right; left from 1 returns to the castle-roof preset.
Owl stages override the chosen controller as documented in `BRAIN.md`; they are unavailable for other fight selections.
Arena falls back to the flat floor for enemies without a named arena.
The map list includes the baseline table's m89 sky library, which has no authored floor.
No enemy and Explore emit `--no-enemy`, which omits the opponent model, target, name and health bar.
The separate `--passive` CLI testing aid retains its opponent while disabling AI.
Explore and direct `--no-enemy` runs show only the controls and contextual action guides in the top-left HUD by default.
F3 toggles the debug text and is listed in the controls line; exploration debug text contains player details only.

The menu emits CLI arguments and validates them through the shared `play_options` builder.
Controller worker construction is deferred until play, so validation never opens hosted telemetry or sends model requests.
Preferences are saved best-effort to `%LOCALAPPDATA%/sekiro-rs/menu.json` (or `$XDG_CONFIG_HOME/sekiro-rs/menu.json`).
An internal JSON argument also carries the choices through relaunch, so a failed preference write does not lose them when returning to the menu.

Esc or gamepad Start pauses virtual time, fixed simulation and live camera/input, and frees the cursor.
Resume restores time and cursor capture.
Back to start menu and Start use the permitted `current_exe` relaunch fallback.
Fight setup currently constructs the non-Send simulation, per-map asset root, animation libraries, map collision and audio plugin before running the app; these are not state-scoped resources or entities.
Replacing this architecture with in-process teardown would require a wider lifecycle refactor.
The app drops before the next process starts, releasing characters, map, audio and brain workers through the same path as Quit.
The existing `BrainDriver::drop` prints the session report and saves the unfinished fight when it normally would on quit.
Returning preserves menu choices, cache override and window size; it does not continue a scripted replay.

## Offline capture driver

`--menu-script FILE` reads rendered-frame actions, one `<frame> <action>` per line.
Actions are `up`, `down`, `left`, `right`, `enter`, `start`, `pause`, `back`, `quit`, `shot:<path>`, and `set:<row>:<value>`.
`enter` confirms from the selected row, while `start` selects the Start action before confirming.
Rows are Mode 0, Enemy id 1, Place 2, Map 3, Start 4, AI 5, Stage 6, Overlay 7, AP 8, Player 9 and Player name 10.
`--menu-fight-script FILE` forwards a separate driver to the launched fight.
`--menu-return-script FILE` forwards a driver to the returned start menu.
Capture runs use `--simple-ai` to avoid model calls and brain memory/session writes.
Capture files and temporary scripts are under `target/alt/menu/`.

## Verification, 11 October 2026

All these PNGs were opened and visually inspected.

| Capture under `target/alt/menu/` | Judgement |
| --- | --- |
| `start-1600.png` | Soldier picker fits with generous margins, readable Segoe UI and grey unused controls. |
| `start-1280.png` | Owl arena / coach / alive / overlay choices fit at 720p; the help stays on one line and stage is enabled. |
| `fight.png` | Menu Start loads Wolf and the soldier on the existing textured flat-floor setup. |
| `pause.png` | Short help and all three actions fit; the dark translucent panel stays readable over the paused fight. |
| `returned.png` | Back restores the picker with the previously chosen god mode. |
| `persisted.png` | A fresh process restores arbitrary enemy/row, map start 2, AP 9, hour 18, immortal mode and player name tester from disk. |

Persistence was verified with LOCALAPPDATA redirected to `target/alt/menu/config-test` as a writable test fixture.
Writing the normal user destination could not be verified in this sandbox; its failure was harmless and the relaunch payload still restored choices.
All rendered fight captures used `--simple-ai`, with no model calls or brain session writes.
The final checks passed: formatting, workspace/all-target Clippy with warnings denied, the release build, 369 sim tests and 26 game tests, using only `target/alt`, offline Cargo and six jobs.
The four menu equivalence tests compare every options field, resolving the stage's unique temporary memory paths by their contents rather than directory names.
Physical gamepad input and live coach requests remain untested here.

## Explore verification, 14 October 2026

The old menu used short map IDs that the map loader does not accept.
The menu now emits full IDs and migrates saved short IDs without changing map loading rules.
All nine menu tests pass, including direct keyboard shortcuts through the live update system, authored start wrapping, all ten extracted maps' options and first-visit lighting selection, retained fight options and the distinction between absent and passive opponents.
Unset hours are compared with `to_bits()` because the CLI default is NaN.
The completed follow-up game-crate release run passes 87 unit tests and six integration tests, with two existing slow tests ignored.
The no-enemy regression checks repeated lock requests, overlapping bodies, attacks, walking beyond the enemy restart deadline and normal player death/restart.
The live HUD-system regression checks F3 on/off and hidden enemy panels, names and targeting hints.
Builds and checks use `target/alt-mn` and four Cargo jobs.

Accepted and visually inspected captures are `target/alt-mn/captures/explore-menu.png`, `outskirts-menu.png`, `fight-menu.png`, `castle-start.png` and `outskirts-walk.png`.
Castle starts with Enter; Outskirts starts with Left then Enter from the Castle selection.
Both raw WASD replays produce movement in all four directions with lock-on off, and the settled scenes include map objects.
The first Outskirts gameplay shot is rejected because it contains only the sky while scene loading settles.
The Castle file named `castle-walk.png` was captured before the walking segment under host contention; walking evidence comes from the completed input log.
The fixtures redirect LOCALAPPDATA into `target/alt-mn/settings` and use simple AI, so normal menu preferences, brain memory and session reports are untouched.

The follow-up authorizes enemy spawn and HUD changes in `crates/game`.
`--no-enemy` now omits the actor and encounter-specific spawns, rejects lock-on and auto-aim, and hides every enemy HUD field.
The adapter for the simulation's mandatory opponent slot suppresses contacts and enemy-death restarts while preserving Wolf's normal death/restart behavior.
Explore starts with debug text hidden, and F3 toggles player debug details without exposing enemy details.
The extracted m89 sky library has no collision floor and cannot support ordinary walking.

The follow-up replaces and visually inspects `castle-start.png` and `outskirts-walk.png` with the opponent and its HUD absent and debug details hidden.
Both CLI runs include `--no-enemy --lock-on`, repeated raw Q presses, map objects and the map's default story selection; neither records a locked-on frame.
The Outskirts replay walks in all four directions.
These captures use the existing `--no-connected-blocks` compatibility switch and a process-local 64 MiB Rust thread stack to isolate concurrent connected-map loading and its IO-stack overflow.
No map loading or rendering implementation is changed for this workaround.
The final check after concurrent source/data updates passes 100 unit tests, with two slow tests ignored, and five integration tests.
`map_tangents::exported_tangents_match_mikktspace` fails on the regenerated map data with handedness agreement 0.695; its rendering/data path is outside this job's authorized scope.
Final game all-target release Clippy with warnings denied and workspace formatting pass.

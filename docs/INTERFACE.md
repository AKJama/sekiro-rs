# Start and pause menus

No arguments or `--menu` opens a Bevy UI panel with Segoe UI, keyboard, mouse and gamepad navigation.
Existing flags still dispatch directly to their original modes.
Up/down or the gamepad stick/D-pad select a row; left/right change values; clicking a row advances its value; Enter/A confirms Start or a pause action.
Select Enemy id or Player name and type with the keyboard; Backspace removes a character.
Enemy ids also increment with left/right for gamepad entry.
Enemy id, map/start and brain stage controls are grey when their parent choice does not use them.
Attack power accepts 1-99 via left/right; left from 1 returns to the castle-roof preset.
Owl stages override the chosen controller as documented in `BRAIN.md`; they are unavailable for other fight selections.
Arena falls back to the flat floor for enemies without a named arena.
The map list includes the baseline table's m89 sky library, which has no authored floor.
No enemy maps to the existing `--no-enemy` flag, which currently leaves a passive rendered opponent.

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
Actions are `up`, `down`, `left`, `right`, `start`, `pause`, `back`, `quit`, `shot:<path>`, and `set:<row>:<value>`.
Rows are Fight 0, Enemy id 1, Place 2, Map 3, Start 4, AI 5, Stage 6, Overlay 7, AP 8, Hour 9, Player 10 and Player name 11.
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

---
name: live-test
description: How to test a sekiro-rs change in the live game - ask the user to playtest first, and when an agent runs the game itself, wait for the full load, spawn where a player would stand, steer with route waypoints and prove in the log that the thing under test was hit. Use before any live-game run, playtest request or scripted walk in this repo.
---

# Live test

Data tests are the main regression check; a live run shows the change works the way the player meets it.
The user's own playtest is the acceptance test, and an agent's scripted run never replaces it.

## 1. Ask the user first

Ask whether the user is at the PC to playtest.
If they are:
1. Copy the branch's release binary to `target/play/sekiro-rs.exe`.
2. Give the run command in PowerShell syntax with the full path to the repo, for example `& <repo>\target\play\sekiro-rs.exe` (no `&&`, no `/c/` paths).
3. Give a short checklist of specific things to try and questions to answer, most important first, for example "grapple to the tree and walk the trunk: do you stay on it?".

Run the game as an agent only when the user is away, and say in the report that the run is not acceptance.

## 2. Running the game as an agent

- Only one agent runs the game at a time, and at most two game windows are open in total (the user may have one).
- Build into the shared agent folder, `CARGO_TARGET_DIR=target/agent`, never `target/play`.
- Use `--live-check <summary.json>` so the run records load times, frame times and 4 Hz samples of Wolf (see `crates/game/src/live_check.rs`).
- Close the window when the run ends; kill a stuck one by its exact PID only.

## 3. Wait for the full load

Give no input until the log's `ready` time has passed, plus about 3 s.
Start the `--script` with an idle line (for example `420 0 0 -`, 7 s) and check in the summary that `map_loaded_s` is not after `sim_started_s`.
Confirm Wolf is grounded through the wait before the first real input.

## 4. Start where a player would

Spawn with `--spawn x,y,z,yaw,0` on ground a player would really stand on before doing the thing under test, not on the target itself.
For example, to test a tree reached by grapple, start on the ground below and grapple to it.
Check the first samples: a spawn that drops Wolf into a fall return before any input is a bad spawn.

## 5. Steer with a route, not timed presses

Scripted sticks are relative to the camera, so a timed "forward for 2 s" drifts off a narrow path.
Use `--script-route x,z;x,z;...` instead: once the `--script` ends, Wolf walks to each point in turn, steered toward it in world space every step.
- Points are map x, z; take them from the collision or the target's own geometry, about 0.25 to 0.5 m apart on narrow paths.
- A point counts as reached within 0.25 m, and the log prints `script route: reached point N`.
- After 3600 steps without finishing, the route is abandoned with a `script route abandoned` warning; treat that run as failed.
- The live check waits for the route before it ends the run.

A route only walks, so a run, sprint or jump in a set direction is a `--script` row aimed by its yaw column (format in `crates/sim/src/demo.rs`).
- The column turns the camera, which starts at the `--spawn` yaw: a forward stick (`0 1`) goes along world yaw `spawn + column`, and `0 -1` goes the opposite way.
- Yaw 0 faces -z and positive yaw turns left (+90 faces -x); to aim along a world direction `(dx, dz)`, use the column `atan2(-dx, -dz)` in degrees minus the spawn yaw.
- Set `PLAY_LOG_CAMERA=1` to log each step's `cam`, `stick_yaw` and `travel` yaw, and check the travel matches before trusting the run.

## 6. Prove the target was hit

The log must show the run actually tested the change, or it proves nothing.
- Every route point logs `reached point N`.
- The samples show the state under test, for example `grounded` true, `floor_object` naming the object being walked on (once object collision from #6 is in), and no fall (`fall_cm` 0).
- Turn the proof into assertions in `crates/game/tests/it/live_play.rs` when the case should stay covered.

Report the load time, settled frame times and the proof lines, and keep screenshots under `D:\sekiro-rs-media\progress\<date>\<job>\`.

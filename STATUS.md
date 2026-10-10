# Status

Last updated 10 October 2026.
Read this first in a new session, then `AGENTS.md` and `docs/DESIGN.md`.
How much of the game is done overall: `docs/PROGRESS.md`.

## Where things stand

| Area | State | Docs |
|---|---|---|
| Archives, formats | Whole install unpacks to `cache/raw` in under a minute | `docs/FORMATS.md` |
| Models | Wolf (c0000 + parts) and Ashina soldier (c1010) export as textured, skinned GLB; enemy weapons via draw masks | `docs/FORMATS.md` |
| Animation | All c0000/c1010 clips decode; TAE parsed and decoded; clip aliases resolved | `docs/FORMATS.md` |
| Params | All 186 tables load from Paramdex defs; typed combat tables | `docs/PARAMS.md` |
| Player behaviour | Real `c0000.hks` runs in our HKS VM against the real behaviour graph; TAE cancel windows, blending, lock-on | `docs/HKS-INTERFACE.md`, `docs/HKS-COMBAT.md` |
| Movement | Run, sprint, jump (running jump fixed), step, crouch; root motion; walks the real m11 map with floor/wall collision | `docs/FORMATS.md` (Walking on the map) |
| Combat | Engine rules for hit/block/deflect, posture both ways, breaks, recovery; HP/posture HUD | `docs/COMBAT-RULES.md` |
| Deathblows | Real ThrowParam pairs; posture break -> deathblow -> death; Wolf death and respawn | `docs/DEATHBLOW.md` |
| Enemy | Soldier runs its own HKS script, behaviour graph and real Lua AI (Lua 5.0 VM + goal runtime); real combo cancels | `docs/AI.md` |
| Great Shinobi Owl | c5060 (not c5070, his bird) with his own HKS, graph, TAE, params and Lua AI on the flat floor and on the Ashina Castle top where the game places him (`--arena owl`); shuriken bullets; two deathblows with the get-up and finishing pairs | `docs/OWL.md` |
| Boss brain | The Owl's act choices can come from an outside controller (`--brain lua\|scripted`, `--brain-log`): plans of his own action ids run through his battle goal, reflexes stay with his Lua AI, mirrored observation and an outcome tally per action | `docs/BRAIN.md` |
| Audio | The game's own FMOD Ex runtime from the install plays the game's event projects (`main`, the enemy's `cXXXX`, the map area's): TAE sounds with floor-material footsteps and armour rustle, hit, block and deflect clangs from the hit-effect params, listener on the camera with the events' own 3D rolloff; decoded samples only as a fallback | `docs/AUDIO.md` |
| Map | m11_00_00_00 (Ashina Outskirts and Castle) and m11_01_00_00 (the invaded castle): pieces, textures, normal maps, collision; boss arenas load the pieces and collision within a radius | `docs/FORMATS.md` (Maps) |

## How to run

```powershell
./tools/fetch-refs.ps1
cargo run --release -p sekiro-extract -- unpack
cargo run --release -p sekiro-extract -- models
cargo run --release -p sekiro-extract -- anims c0000 c1010 c5060
cargo run --release -p sekiro-extract -- params
cargo run --release -p sekiro-extract -- map m11_00_00_00
cargo run --release -p sekiro-extract -- map m11_01_00_00
cargo run --release -p sekiro-game -- --play --map m11_00_00_00 --start 3
cargo run --release -p sekiro-game -- --arena owl
```

Useful flags: `--play` alone uses a flat floor; `--enemy owl` (or `c5060`) fights the Great Shinobi Owl instead of the soldier; `--arena owl` fights him on the castle top where the game places him (only the map within 60 m; `--full-map` loads all of it; `docs/OWL.md`, "The castle top"); in play, `--hide <model prefixes>` skips map piece models and `--objects` adds MSB objects; `--lock-on`, `--no-enemy`, `--simple-ai`, `--script FILE --shots step:path --exit` for scripted runs with screenshots; `--input FILE` replays raw keys and mouse through the live input path and logs camera and movement per frame and step (`crates/game/src/input_replay.rs`); `--arena --deathblow` for the deathblow demo; `--viewer <glb>` and `--map <id>` for inspection; `--game <dir>` (or `SEKIRO_DIR`) points at the install whose FMOD DLLs play the sounds; `--volume 0.0..1.0` sets the master volume (default 0.5), and in play minus and equals (or `[` and `]`) change it in 10% steps with a short on-screen readout.
Testing aids: `--break-posture` holds the enemy's posture at 1 point outside deathblows, so Wolf's next landed hit breaks it (quick deathblow tests, also for the Owl's second life); `--spawn x,y,z,yaw,pitch` now also works in an arena, putting Wolf there and the enemy `--enemy-distance` metres (default 5) in front of him, for example next to the castle-top railing (`--arena owl --passive --god --break-posture --spawn 101.23,53.95,247.33,135,0 --enemy-distance 1.6`); `--god` makes Wolf take no HP or posture damage (hits, reactions, hit stop and knockback unchanged), so long replays are not cut short by his death; `--passive` (same as `--no-enemy`) keeps the enemy standing there with his AI off; `PLAY_LOG_COMBO=1` logs one `combo` line per step (attack button, buffered presses, Wolf's state, clip and clip time, both hit stops, the step's hits, the enemy's state and clip, distance).

## In progress when the session paused

- Grappling hook, ledge hang, Mikiri counter and perilous attacks (player/enemy behaviour).
- Fall death and respawn, FLVER tangents to cut the 14 s map load, map lighting from the game's draw params, foliage alpha.
- Audio (2026-10-10, FMOD path built, to be confirmed by ear): sounds at the TAE events' dummy polys, map reverb, music. Coverage and loudness numbers: `docs/AUDIO.md`.

Uncommitted files in the working tree belong to these tasks; check `git status` and build before committing.

### Leads for grapple, ledges and Mikiri (found, not built)

- Ledges: `map/<id>/<id>.edgebhd/.edgebdt` hold one `.edge` file per collision part (16 files, 255 edges on m11). Header `u32 version=4, u32 count, u32 collision number, u32 0`, then 64-byte records: two vec4 endpoints, a zero vec4, `u32 chain id`, `u32 flags` (low byte 2 or 3, second byte 0..3 or 10, meaning unknown), 8 bytes padding. Endpoints are in the collision part's local space; apply its layout transform. `.wallbhd/.wallbdt` is likely wall-hug data, undecoded.
- Grapple: map pieces have no dummies; object `o000100` (placed 67 times on m11) carries dummy 90 and is very likely the grapple point. Search rules: WirePointSearchParam row 0 (act 3 to 15 m, find to 25 m, filter and soft-lock boxes); WireSetParam picks search rows per area; WireVariationParam (angles, marker type, homing offset) is probably set per placed object in MSB fields the parser still skips.
- Mikiri: find the soldier's perilous thrusts (AtkParam rows that disable guarding) and the warning effect; the counter likely uses an attack-cancel window plus a ThrowParam row, wired like deathblows in `crates/sim/src/duel.rs`.
- Study tools: `crates/sim/examples/bxf_list.rs`, `dummy_census.rs`.

## Decision needed from the user

- 62 of 174 sound banks (`sm*`, `smain`, `vm*`, `xm*`, `rm*`: area SFX, voices, music, cutscenes) are encrypted FSB that the game's FMOD cannot open without the game's key. Footsteps, hits, clangs, Wolf's and the Owl's sounds are in the plain banks and play; the encrypted ones stay silent. Reading them means decrypting with a key from the game; ask before touching them (see `docs/AUDIO.md`).

## Known issues

- Fixed 2026-10-10, to be confirmed in play: combos on the Owl froze and jittered. Every direct hit froze both characters for 0.1 s, which the game does not do against the Owl (his NpcParam hit stop types are 0), and the drawn animation slid backwards during the freeze; the fight also hitched about 2 s in, on its first step and on first-shown clips. See "Frame pacing" below.
- Fixed 2026-10-10, to be confirmed in play: during the Owl's deathblow the stick turned Wolf and the Owl's body followed; near walls the pair clipped in; the game froze for about half a second as a deathblow started. Causes: Wolf's throw states pick their clip by animation offset type 17, which was never set to ThrowParam `atkAnimOffset` (221 for the Owl), so Wolf played the soldier's 2.0 s `a200_510000` and was free after 2 s while the Owl (6.6 s clip) was still held on him; the defender was held on the dummy of Wolf's live pose every step, teleported and through walls; a knockback left from the breaking hit resumed when the pair ended; the push-apart ignored collision; and every deathblow start parsed c0000's 23 MB `tae.json` (0.45 s). Now the offset types are set from the row, the absorb places one character once at the body clip's frame 0 and both play their own root motion (after FwLo's RustSekiro), all of it moves through collision (a blocked absorb shifts the other character instead, inferred), and throw times come from the decoded TAE (deathblow-start frames now 16 to 18 ms, were a 250 ms clamped frame). Wolf can walk out from 4.0 s, where his clip's TAE opens the move cancel. `docs/DEATHBLOW.md`, tests `crates/sim/tests/deathblow_lock.rs`, screenshots `cache/screens/deathblow_{steer,owlwall,wolfwall,owlrail,wolfrail}_*.png`.
- Fixed 2026-10-10, to be confirmed by ear: nearly no sounds in the Owl fight and volume that jumped near walls and corners. The Owl's bank was never decoded and Bevy's spatial audio scaled every sound by the camera's distance with no minimum. Sounds now play through the game's own FMOD (`crates/fmod`, `crates/game/src/audio.rs`): all 55 of the Owl's TAE sound keys play, and the events' own rolloff is flat out to about 5 m. The hit-sound lookup also used the wrong param fields and letter; fixed from FwLo's exe trace. `docs/AUDIO.md`.

- Fixed 2026-10-10, to be confirmed in play: locked on, diagonal walks and runs (W+A, S+D, ...) went straight forward, back or sideways. The script picks one of four strafe clips by 60 degree cones (`MoveDirection`), and the engine applied that clip's straight root motion; the game's exe also sets `TwistMasterAngle` to the stick's angle minus the clip's direction, and the graph turns the Master bone by it and turns Spine..Spine2 back (WalkTwist, traced by FwLo's RustSekiro, `docs/kb/movement.md`). Now `Character::walk_twist` eases to that angle at 0.1 rad per 1/30 s, locked walk and run root motion is turned by its target (so travel follows the stick even while the twist catches up after a cone change; our choice, not traced), and the renderer turns the legs by it with the chest kept on the target (`apply_walk_twist` in `crates/game/src/character.rs`). The forward diagonals that "turned away" were the same bug: running straight at a target 6 m away, Wolf ran through it and the lock-on facing swung round. Tests: the `locked_diagonal_*` cases in `crates/sim/tests/camera_turn_movement.rs` (target moved to 20 m, since holding 45 degrees off the target bearing spirals in and the test has no enemy body) and `walk_twist_turns_the_hips_and_keeps_the_shoulders` in the game crate; app run: `--arena owl --passive --god --input FILE` with W+A then S+D logs travel 0 degrees off the stick and `walk_twist` near -48 and -42 (`cache/screens/diag_*.png`, camera behind Wolf).
- Mouse look was far too sensitive (reported 2026-10-10); `MOUSE_SENSITIVITY` in play.rs is now 0.0012 radians per mouse count (was 0.0035), not yet confirmed by the user.
- Fixed 2026-10-10, to be confirmed in play: movement "inverted / tank-like" after turning the camera. Free move starts picked the locked-on strafe clips (back, left, right) from the stick's angle to Wolf's facing while steering turned him to the stick, so he side-stepped or ran backwards; now a free move start turns the body to the stick and keeps the turn as the lower-body twist (`TwistLowerRootAngle`, drawn as a smooth turn), so the script picks the forward start or its 180 degree turn-and-go. That snap-and-twist model is inferred from the script and graph, not traced in the exe. Locked-on strafes now follow the stick (engine `MoveDirection` with the WalkTwist cones, and bound graph selectors re-read when a variable changes). Tests: `crates/sim/tests/camera_turn_movement.rs`.
- Fixed 2026-10-10, to be confirmed in play: mashing attack on the Owl "repeated the first slash and stun-locked him". Wolf's part is the game's: when the Owl guards (`a000_003100`), Wolf's blocked slash recoils (`EasyDeflected`) and the script's follow-up is the quick `EasyDeflectedAtk` (`a050_305000`). The bug was the Owl's: env 258 (his NpcParam `guardLevel`, 4) read 0, so his script's guard table gave no guard reaction and every blocked slash played the hit flinch `a000_008010`, with no guard window and no consecutive-guard count, and he never got out to counter. He now plays `GuardDamageSmall` (`a000_008501`) and counters. Tests: `crates/sim/tests/combo_on_hit.rs`; app check: `PLAY_LOG_COMBO=1 sekiro-rs --arena owl --god --input FILE` with W held and left click mashed.
- Fixed 2026-10-10: after Esc freed the mouse, the camera ignored it until Esc again (clicking did nothing); a click now takes the mouse back (and does not attack). Mouse motion during map loading no longer turns the start view. Wolf and the enemy are drawn between simulation steps, so frames with no or two steps no longer stall or jump.
- Fixed 2026-10-10, to be confirmed in play: locked on, holding a direction and dodge stepped that way, then sprinted forwards or backwards. Causes: the engine ignored the script's `act(2018)` (release the lock-on angle lock, asked on every sprinting frame), so steering kept turning Wolf to the target and the script's locked-sprint brake (|MoveAngle| >= 135) fired over and over; root motion samples are displacements in the clip's start frame but were applied as if in the current facing, so a sprint start that turns 90 or 180 degrees (a000_00115x) carried Wolf sideways; `act(3025)` turned the wrong way and ran before `act(2019)`, so diagonal step tilts went off the stick; the locked action angles (Kick/Jump/Attack/SubAttackAngle) were measured from the old facing although the script reads them right after facing the target. `TurnAngle` is now measured from the root-motion travel direction (the facing when still), which lets the back sprint start (a000_001152) play instead of braking into the sprint stop slide toward the enemy; that model is inferred from the script and clips, not traced in the exe. Tests: `crates/sim/tests/camera_turn_movement.rs` (locked step, diagonal tilt, locked sprint in four directions, release).
- The lock-on camera loses Wolf while he sprints locked on: he leaves the frame, or the camera ends up inside him near the castle-top railing (`cache/screens/locksprint_*.png`, 2026-10-10). Camera framing and collision, not movement.
- Wolf's max HP/posture come from CalcCorrectGraph at progression level 1; the real progression input is not yet traced.
- Player HP damage formula is inferred (weapon attack power x correction), not traced.
- Single-clip rendering: layered upper/lower-body states show one clip (FwLo's RustSekiro plays them with the graph's bone weights; next to port).
- Auto-aim has no line-of-sight check; knockback column choice per damage level is matched by name, not traced.
- Shading is plain PBR; map lighting is hand-picked; 3 map textures from other areas are missing.
- Deathblow needs lock-on to meet the angle limit.

## Next steps

- Deathblow presentation (user, 2026-10-10): the deathblow camera (TAE 155 SetLockParamID -> CameraSetParam slot 41 -> CameraParam 4000), camera shake (TAE RumbleCam 144-147 -> rumblebnd camera_NNN.hkx), then particle effects (FFX/FXR: blood on deathblows and hits, sparks on clashes, Owl's smoke and firecrackers). See FwLo's docs/kb/camera.md for the camera rules.
1. Owl on the castle top (`docs/OWL.md`, "Next steps"): the fight runs there (`--arena owl`); next decode the map event script (`event/m11_01_00_00.emevd`) for his phase 2 and for which top floor variant shows, then NPC-on-player throws, firecracker and poison bullets.
2. Finish the three in-progress tasks above.
3. Validation against the real game: record posture/HP/animation traces in a real fight (read-only) and compare with `duel-sim` output.
4. More enemies from the m11 layout (43 soldiers placed in the MSB).
5. Combat arts, prosthetics and items only after the core loop is solid.

## Publishing

The repo is public at github.com/AKJama/sekiro-rs.
Local git hooks (`.git/hooks/pre-commit`, `commit-msg`) reject personal paths, emails, co-author lines and anything under `cache/` or `re/`; they are not part of the repo, so re-create them on a new clone.
Write commands in docs as plain `cargo`, never a full user path.

## Frame pacing (2026-10-10)

`PLAY_LOG_PERF=1` writes one CSV row per rendered frame to `cache/perf/` (`crates/game/src/perf_log.rs`: wall time, steps, drawn and simulated clip times, hit stops, hits, sounds, and where a slow frame went: main schedule, fixed steps, clip loads, render-world time per render set, pipeline counts).
`tools/perf/owl-replays.ps1 -Exe <exe> -Tag <tag>` replays the same raw input against the Owl (passive, real AI, and a no-contact control with the Owl 1000 m away, each run twice: cold, then warm); `uv run tools/perf/summary.py -v --fail-over 50 cache/perf/<tag>-*warm.csv` prints median, p95, p99, max and every frame over 25 ms with what happened in it, and exits 1 if a frame without a hit takes over 50 ms.
That warm-run check is manual (it needs the game window on a desktop); run it after renderer, loading or audio changes.

Clean runs on an idle machine (RTX 5070 Ti, Vulkan, vsync 60 Hz; gameplay frames only):

| Run | Before (c1782f8): median / p95 / p99 / max ms, frames over 25 ms | After |
|---|---|---|
| passive cold | 16.67 / 17.77 / 18.96 / 57.19, 4 | 16.68 / 17.39 / 17.97 / 20.06, 0 |
| passive warm | 16.66 / 17.72 / 18.80 / 52.54, 3 | 16.67 / 17.43 / 18.32 / 21.16, 0 |
| AI cold | 16.67 / 17.56 / 18.60 / 57.05, 2 | 16.68 / 17.25 / 17.79 / 18.15, 0 |
| AI warm | 16.68 / 17.47 / 18.12 / 52.16, 4 | 16.69 / 17.34 / 18.66 / 30.09, 1 |
| control cold | 16.68 / 17.23 / 17.89 / 52.37, 3 | 16.67 / 17.15 / 18.12 / 30.11, 1 |
| control warm | 16.69 / 17.19 / 18.18 / 55.63, 4 | 16.69 / 17.13 / 17.65 / 18.63, 0 |

The earlier 42 to 52 ms medians in `cache/perf/REPORT.md` were another agent compiling on the same machine.
What the slow frames were, and what changed:

- The contact freezes were hit stop, which the game does not apply in this fight: the defender's NpcParam `hitStopType` scales the attacker's stop and is 0 on the Owl and the soldier, and `hitStopType_Defencer` is 0 on every NPC. Hit stop now follows the exe (a 0.06 s hold, then a quadratic ease over the AtkParam time, scaled per NpcParam type; `docs/COMBAT-RULES.md`, "Hit stop"), so Wolf's hits on the Owl stop nobody.
- Drawn clip time ran backwards during hit stop (by up to 11 ms) because the frames between steps trailed the last step by a fixed step length even when the character's clock had stopped. They now trail by the character's own advance in that step (`StepReport::advanced`), and crossfades advance on the character's clock, so a stopped character holds exactly still and resumes without a jump (test `drawn_clip_time_holds_through_hit_stop`). The backward frames outside clip restarts went from 25 and 10 (passive and AI warm) to 0.
- Every run had a 35 ms and a 52 to 57 ms frame about 1.9 s after the map finished loading, also with no input and with sound off: the render world waits that long for the next swapchain image, with no CPU work, pipeline compile, clip or sound load in it; with map pieces hidden or on the flat floor it does not happen, and with the start held 3 s it still came 1.9 s after loading (cause in the driver or presentation, not found). The fight now starts 2.5 s after the map has loaded (`MAP_SETTLE_SECS` in `play.rs`), so the stall falls in the loading time.
- The fight's first step took 95 ms: the hit-sound tables were read on first use. They and the enemy's FMOD project (c5060.fev, which loaded a few frames into the fight) now load at start.
- Clips shown for the first time were read and decoded on that frame (26 to 55 ms on cold runs, for the Owl's damage and attack clips). Once the fight runs, a background thread decodes all of both characters' clips (0.4 s for Wolf's 1433, no measurable memory change).
- Left: one 30 ms frame in some runs with 16 ms in the main schedule and nothing else in it; the replay writes a stderr line per frame and step into a PowerShell redirect, which likely blocks now and then (not seen without the replay logs, not proven).

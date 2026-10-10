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
| Deathblows | Real ThrowParam pairs; posture break -> deathblow -> death; Wolf death and respawn; the deathblow camera from the clips' TAE 151 / 153 / 155 | `docs/DEATHBLOW.md` |
| Enemy | Soldier runs its own HKS script, behaviour graph and real Lua AI (Lua 5.0 VM + goal runtime); real combo cancels | `docs/AI.md` |
| Great Shinobi Owl | c5060 (not c5070, his bird) with his own HKS, graph, TAE, params and Lua AI on the flat floor and on the Ashina Castle top where the game places him (`--arena owl`); shuriken bullets; two deathblows with the get-up and finishing pairs; his grabs (3040, 3041) throw Wolf from the data (ThrowParam 25060000 / 25060100); phase 2 (heat-up, SpEffect 200051, boss bar, defeat) runs from the map's own event script (`event/m11_01_00_00.emevd`) | `docs/OWL.md` |
| Owl's tools | Bullets with child bullets, flight from the Bullet params, area and lingering hits, shared hit records; poison build-up and damage over time, recovery seal, blinding, lock-on break; warp clips fade him out, unhittable, when forced (his castle AI never warps); simple stand-in looks and a HUD status line | `docs/OWL.md` (Tools) |
| Boss brain | The Owl's and the Ashina soldier's act choices can come from an outside controller (`--brain lua\|scripted\|decisions\|responses`, `--brain-log`): plans of his own action ids run through his battle goal, reflexes stay with his Lua AI, mirrored observation, this fight's outcome tally, and a memory of every fight with Wolf's habits kept in `logs/memory/<chr>.json` (`--brain-memory`) | `docs/BRAIN.md` |
| Audio | The game's own FMOD Ex runtime from the install plays the game's event projects (`main`, the enemy's `cXXXX`, the map area's): TAE sounds with floor-material footsteps and armour rustle, hit, block and deflect clangs from the hit-effect params, listener on the camera with the events' own 3D rolloff; decoded samples only as a fallback | `docs/AUDIO.md` |
| Map | All ten archive maps extracted and loadable; 69 starts across nine gameplay maps walk; m89 is sky-only | `docs/MAP-BASELINE.md`, `docs/FORMATS.md` |

## Map baseline commands

Use `target/alt/release/sekiro-rs.exe --map <id>` for the viewer and add `--play` to walk.
The alternate build leaves the user's default release executable alone.

| `--map` ID | Area | Suggested `--start` |
| --- | --- | ---: |
| m10_00_00_00 | Hirata Estate | 0 |
| m11_00_00_00 | Ashina Outskirts | 3 |
| m11_01_00_00 | Ashina Castle | 2 |
| m11_02_00_00 | Ashina Reservoir | 0 |
| m13_00_00_00 | Abandoned Dungeon | 0 |
| m15_00_00_00 | Mibu Village / Ashina Depths | 3 |
| m17_00_00_00 | Sunken Valley / Sunken Valley Passage | 0 |
| m20_00_00_00 | Senpou Temple, Mt. Kongo | 0 |
| m25_00_00_00 | Fountainhead Palace | 1 |
| m89_00_00_99 | Sky-dome library, no floor | Viewer only |

Map extraction has zero failed models after handling the castle's HKX-only object binder.
The sky library now retains its collisionless zero-group pieces and resolves its cinematic GPARAM.
Initial viewer cameras now clip against nearby collision; the castle and Mibu obstructed starts were verified after rebuilding.
Final alternate release build, workspace Clippy, workspace formatting, formats/extractor release tests and both map regressions passed; details and screenshots are in `docs/MAP-BASELINE.md`.

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
Testing aids: `--break-posture` holds the enemy's posture at 1 point outside deathblows, so Wolf's next landed hit breaks it (quick deathblow tests, also for the Owl's second life); `--spawn x,y,z,yaw,pitch` now also works in an arena, putting Wolf there and the enemy `--enemy-distance` metres (default 5) in front of him, for example next to the castle-top railing (`--arena owl --passive --god --break-posture --spawn 101.23,53.95,247.33,135,0 --enemy-distance 1.6`); `--god` makes Wolf take no HP or posture damage (hits, reactions, hit stop and knockback unchanged), so long replays are not cut short by his death; `--passive` (same as `--no-enemy`) keeps the enemy standing there with his AI off; `--force-act <id>` makes the enemy request that act whenever he is free (with `--passive`, only that act; `--force-act 3040` is the Owl's grab, script `crates/game/scripts/owl_grab.txt`); the Owl's tools on demand: `--arena owl --passive --god --force-act 3010 --spawn 104.5,54.0,244.06,135,0 --enemy-distance 4 --script crates/game/scripts/owl_tools.txt --shots ... --exit` (3010 firecrackers, 3017 explosive slash, 3030 poison, 3031 recovery seal, 3069 smoke; always give `--shots` paths under the repo, a shot that cannot be saved keeps the run from exiting); `PLAY_LOG_COMBO=1` logs one `combo` line per step (attack button, buffered presses, Wolf's state, clip and clip time, both hit stops, the step's hits, the enemy's state and clip, distance).

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

- Done 2026-10-10, to be confirmed in play: three features after FwLo's RustSekiro video (his code adapted under MIT, `CREDITS.md`). (1) The deathblow camera: Wolf's deathblow clips drive it through TAE 155 (CameraSetParam slot 41, CameraParam 4000), 153 (distance, 1.5 m at the soldier's blow) and 151 (focus on dummy 142 of his animated camera bone, with look limits; the Owl's deathblow), and it hands back on the params' own chase rates (`docs/DEATHBLOW.md`, "Camera"). (2) The perilous kanji: the warning dummy bullet (judge 980 to 989) is made at Wolf within 5 m (soldier) or 9 m (Owl), its SpEffect 211000 shows VFX 48100 "å±" at Wolf's dummy 220, and the HUD draws a red å± over Wolf with a Windows Japanese font (the game's FXR is not decoded; no sound found). (3) The jump counter: Wolf's jumps carry sweep i-frames (TAE 954 type 1), his air kick (PCBehavior 901, anti-ground) is now a real attack, the sweeps' SpEffect 400200 multiplies its posture damage by 8 (the soldier's posture breaks), and env 2004 lets his script kick off the enemy (`docs/COMBAT-RULES.md`, "Perilous attacks"). Tests `crates/sim/tests/perilous.rs`, `deathblow_camera.rs`; scripts `crates/game/scripts/soldier_deathblow.txt`, `soldier_jump_counter.txt`; screenshots `cache/screens/dbcam_soldier_*.png`, `dbcam_owl2_*.png`, `kanji_soldier_*.png`.

- Done 2026-10-11, to be tried in play (user's first real soldier fight: the model chose the leaping slash 3003 twelve times although Wolf deflected it 5 of 5): the boss brain now probes instead of repeating. (1) Plans are 1 to 3 steps, each an attack or a movement *tactic* run through the engine's own native goals (hold, guard, walk_in, run_in, strafe, back_off, retreat, keep_range, relative to Wolf, each with a lifetime cap); the soldier and the Owl offer 73 and 75 choices (his actions, combos, 15 tactics, about 40 curated chains), text form such as `walk_in4>hold2>3013`. (2) Request modes `slot` (as before), `plan_end` and `checkpoints` (hosted default: only at fight start, half HP, a lost deathblow mark, Wolf punishing twice or deflecting three times in a row; a fight takes 1 to 5 requests); a reply is a plan set (the Decisions probabilities) the boss samples locally at every act, with a penalty for options repelled repeatedly this fight and a bonus for untried ones (temperature, seed by environment variables). (3) Per-fight metrics for Wolf (perfect-deflect and guard rate, hits taken, HP and posture lost, punishes, kill time) are kept in the memory file (version 2, last 10 fights), sent as a trend, and printed as one summary line per fight; `--brain lua --brain-memory on` saves the baseline to `logs/memory/<chr>.lua.json`. (4) Nothing is requested or logged once a fighter is dead; the hosted worker warms the route (DNS and TCP, no request) and its timeout is 3 s. `docs/BRAIN.md`, tests `crates/sim/tests/brain_tactics.rs`, `crates/sim/src/brain/tests.rs`, `crates/brain-net`. Not verified: any live API call (probability field names come from the docs), how the tactics look in play (distances and caps are tested, the animation blends are not).
- Done 2026-10-10, to be tried in play: the Ashina soldier (c1010) can be driven by the boss brain, as a testbed for whether a hosted model plays better than the game's AI. His action table (`brain::actions::SOLDIER`, from `101000_battle.lua`, his BehaviorParam and AtkParam rows) has 13 attacks and 2 back steps with his acts' approach distances, his rushes' AI number, combos `3000+3001` and `3005+3007`, and reflex acts 23, 25 and 27; his script also names 3004, 3006, 5202 and 5203, which his TAE has no clips for, so they are refused. The driver, observation (`boss`, `boss_bearing`, `since_boss_hit`), tally and hosted requests are generic now (question `boss_action`, instructions naming the enemy). Memory: each fight's tally and Wolf's habits (attack distance, range, his response after deflects, guards and back steps, his answer to and punish speed for each boss action, heals, jumps) go into `logs/memory/<chr>.json` (version 1) at each death, respawn, every 15 s and on exit; requests carry this fight and all fights within 1200 bytes. Try: `.\target\release\sekiro-rs.exe --play --enemy soldier --brain scripted --brain-memory on --brain-log`. `docs/BRAIN.md`, tests `crates/sim/tests/soldier_brain.rs`, `crates/sim/src/brain/tests.rs`, `crates/brain-net`.
- Fixed and checked in play 2026-10-10: the soldier's missing sword was a stale combat draw mask exported before `anims`, leaving group 0 hidden.
  Model extraction now reads weapon-draw TAE 233 directly from the unpacked character TAE, so `models` can run before `anims`.
  Regenerated `cache/models/c1010.glb`; both sword meshes follow `Kodachi` under `R_Hand` with unit scale and now have inherited visibility.
  Regression: `models::tests::soldier_combat_mask_does_not_require_extracted_animations` in `sekiro-extract`.
  Checked screenshots: `cache/screens/sword_duel_before.png`, `sword_duel_after.png`, `sword_roof_verified.png`, `sword_roof_attack.png`, and `sword_owl_{012,040,110,250}.png` (Owl's sword and warp hiding still work).
  Final alternate release captures also checked: `cache/screens/sword_final_duel.png` and `sword_final_owl_{012,040,110,250}.png`.
  Temporary entity diagnostics, removed from source, remain in `cache/screens/sword_mask_before.err` and `sword_roof_verified.err`.
  Release build, extractor regression, full sim release suite and workspace Clippy passed in `target/alt` with six build jobs.
  Touched Rust files pass formatting; the workspace formatting check still reports other agents' active brain, camera and perilous changes.

- Done 2026-10-10, to be confirmed in play: the Owl's phase 2. An EMEVD reader (`crates/formats/src/emevd.rs`, names from DarkScript3's Sekiro EMEDF) and a small event interpreter (`crates/sim/src/events.rs`) run his fight's own events: the first deathblow's get-up raises event message 20 (TAE 231), event 11115870 hides the boss bar, sends HKS command 506000 (heat-up `a000_020005`, then the kneeling loop `a000_021003`) and gives SpEffect 200051; his AI escapes with smoke (20006 when Wolf comes within 2 m, 20007 when Wolf attacks, 20008 after five loops), which resets his HP and brings the bar back; with 200051 his kick follow-up is 3030 poison instead of 3007; the finishing clip's message 50 handles the defeat. New engine answers: env 105 (event command), 206 (in a throw), 123 (AI endure cancel), observe areas, and melee AtkParam SpEffects on the defender. Open: while he begs at 1 HP our duel allows a deathblow (unknown in the game); the heat-up's judge 910 blows Wolf far away, so 20006 rarely fires in practice; boss music and map effects are not run (`docs/OWL.md`, "Phase 2").
- Done 2026-10-10: the Owl's warp and vanish. Finding: in the data the castle Owl (c5060, battle goal 506001, map events m11_01_00_00) never vanishes or teleports; the firecrackers' SpEffect 3506021 "attacking warp" is only acted on by the Hirata event script (Owl (Father) swapping with his bird), and his castle AI never asks for his fade-out clips. Those clips (3037, 3039, 3044, 3092, 3093) now work when forced: TAE 193 fades him out and in, ChrActionFlag 94 makes him unhittable while gone (same exe bit as TAE 951), TAE 760 carries him toward Wolf, and the renderer hides him with a grey puff where he goes and comes back. Try `--arena owl --passive --god --force-act 3044 --enemy-distance 8`. `docs/OWL.md` "Warp and vanish", tests `crates/sim/tests/owl_warp.rs`, screenshots `cache/screens/warp_3044_*.png`.
- Done 2026-10-10, to be confirmed in play: the Owl's grabs (3040 collapse throw, 3041 see-through counter) played his lunge but never caught Wolf. Enemy throws on Wolf now run from the data: a `throwFlag` 1 contact (his left-foot sphere) starts ThrowParam 25060000 / 25060100, Wolf is absorbed onto the Owl's dummy 233 / 234 and plays his thrown clip from `c0000_c5060.anibnd` (`a221_600110` / `600120`), the damage lands at the Owl's `ThrowAttackBehavior` events (192 HP; 32 then 208), Wolf is invulnerable in the clip (TAE 952) and gets up in it, or dies into `a221_600111`. Guard and deflect do not stop it; stepping out of reach, step i-frames (type 7) and jumping do. No grab has a warning bullet in the data, so no red kanji is shown. `docs/OWL.md` "Grabs", `docs/DEATHBLOW.md` "Enemy throws on Wolf", tests `crates/sim/tests/owl_grab.rs`, screenshots `cache/screens/grab_{open,rail,guard}_*.png`.
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

- Deathblow presentation (user, 2026-10-10): the deathblow camera is done; next camera shake (TAE RumbleCam 144-147 -> rumblebnd camera_NNN.hkx), then particle effects (FFX/FXR: blood on deathblows and hits, sparks on clashes, Owl's smoke and firecrackers). See FwLo's docs/kb/camera.md for the camera rules.
1. Owl on the castle top (`docs/OWL.md`, "Next steps"): the fight runs there (`--arena owl`) with his tools, grabs and phase 2 (from the map's event script, which also confirms the intact tatami floor `m015070`); next the boss music switch (encrypted banks) and a capture to check the heat-up against.
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

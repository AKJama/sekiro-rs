# Status

Last updated 11 October 2026.
Read this first in a new session, then `AGENTS.md` and `docs/DESIGN.md`.
How much of the game is done overall: `docs/PROGRESS.md`.

## Visuals steps 0 and 1 (2026-10-11)

Read-only GPARAM/MSB dumps and hour comparisons are in `target/alt/visuals/`.
MSB probe regions, HDR BC6H extraction, Lambert cubes, spatial selection, hour variants and camera environment lighting are implemented and all three requested maps re-extracted.
The intensity is 9000/pi cd/m2; `--no-env-probe` preserves the old ambient fallback.
Six matched screenshot pairs show warmer evening and cooler interior shade; the probes contain nearby scenery, so they light the world but are not shown as the sky (`--probe-sky` shows them for debugging).
See `docs/VISUALS-PLAN.md` step 1 for approximations and the capture names.
Formatting, workspace Clippy, 64 release formats/extract tests (two ignored), 14 game unit tests and the alternate release build pass.
The pre-existing map_tangents regression still fails with direction agreement 0.004.
Matched frame medians stay at 16.6-16.8 ms with a 60 Hz cap; measured probe deltas range from -0.034 to +0.076 ms.
No commit or protected-path write was made.
## Mobility step 2: ledge hang (2026-10-11)

Kind-2 variation-0 map ledges now support ground grab, HangIdle, root-track-fitted climb up and drop through the existing HKS transitions.
The new edge reader and extractor write placed `edges.json` records for m11_00 and m11_01; collision confirms the researched h019900 chain-7 roof edge.
E / gamepad Y is the action button; play scripts accept `action`.
The original start-2 route stayed outside reach; the verified upper-side route at (83.307, 30.6, 243.649), yaw 133.216, finishes at (80.9384, 29.5759, 245.8745), 0.255 m from the edge.
Reviewed captures are `target/alt/mobility/hang-side.png` and `hang-climbed-side.png`; `docs/MOBILITY.md` records the route, placeholders and remaining docking gaps.
The drop replay fires HangToFreeFall and lands on the lower roof; `target/alt/mobility/hang-drop.png` shows the released fall.
Formatting, workspace Clippy, 401 release sim/formats tests (two existing ignored tests) and the alternate release game build pass; game unit tests pass and the pre-existing map_tangents failure remains.
All commands use `target/alt` and four jobs; no commits or protected-path writes were made.

## Mobility step 1 (2026-10-11)

Air control uses the four fall types, world-space stick, per-type gravity, decay, braking and the along-speed cap (`docs/MOBILITY.md`, step 1).
HKS fall height and vertical speed now use cm and cm/s, so the stiff landings play.
Landing from at least `HitMtrlParam.fallDeathDist` (metres assumed) injects DEATH_FALLING and uses the six-second respawn path; falling off the Owl's roof onto the ground 62 m below is such a death.
A fall into the void (a placeholder 40 m below the lowest collision) returns Wolf to where he last stood on collision, as Sekiro's fall return does; its HP cost is in the exe and not traced, so none is taken.
Reviewed replays under `target/alt/mobility/` show a 0.78 m sideways landing offset and a 1284.9 cm short stiff landing at m11 start 4 (the route has an intermediate landing; a 16.8 m sim regression verifies 1680 cm and clip 200020).
Formatting, workspace Clippy, all sim tests and the alternate release build pass; the game's pre-existing `map_tangents` failure remains.

## Boss brain token observability and budgets (2026-10-11)

Tokens are the primary metric in fight summaries, session reports and the recording overlay.
Subscription Codex/Claude usage is labelled as a plan proxy, with no computed or displayed dollar estimate.
API-key requests retain billed USD estimates from the editable `config/brain-prices.toml`, with standard OpenAI model/pricing and Decisions source URLs checked on 11 October.
The price table is whitelisted for tracking; unknown API models show unknown USD.
The task-specified Haiku seed retains an explicit cache-pricing TODO and is never applied to subscription Claude.

Codex JSON completed-turn events supply tokens while `-o` supplies the final plan.
Reported CLI usage now survives an invalid plan, so failures cannot bypass the token budget.
Telemetry keeps legacy fields alongside GenAI semantic names and `sekiro.*` extras, including billing and the plan-proxy flag.
Fight summaries print both fight and session totals; the report Cost section groups kind/model and separates plan tokens from API tokens and billed USD.
F8 displays the session totals and persistent budget notice in a smaller overlay footer.

`--brain-token-budget N` counts all reported input plus output tokens; `--brain-budget USD` counts only billed API usage.
Both default to no limit and zero stops immediately.
Budgeted admission is serialized across the decision and coach workers, and queued work is rechecked before starting a call.
After a limit is reached, the current fight retains its plan and the next uses Lua AI.
Unknown billed prices stop a USD-budgeted session; a final completed call can overshoot a limit because usage is known after completion.

Workspace Clippy with `-D warnings`, `cargo fmt --all` and the final formatting check are clean.
The requested release sim/brain-net library, binary and integration suite passed all 368 tests across 37 targets; its log is `target/alt/brain-cost-release-tests.log`.
Offline regressions cover realistic fake Codex events and final-message separation, cached/reasoning subsets, unknown prices, queued-call stopping, subscription token/USD separation, failed-plan token accounting, current-plan retention and next-fight Lua fallback.
All Cargo commands used `target/alt` and six jobs (fmt uses `CARGO_BUILD_JOBS=6`).
No model/API calls, credential reads, real model CLI execution beyond `codex exec --help`, protected memory/dataset writes, cache/re deletion or links, worktrees or commits were made.
The overlay footer was checked through compilation and driver/format tests; a newly rendered window was not inspected in this session.
The pre-existing game `map_tangents` test was outside the requested suite.

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
| Boss brain | The Owl's and the Ashina soldier's act choices can come from an outside controller (`--brain lua\|scripted\|decisions\|responses`, `--brain-log`): plans of his own action ids run through his battle goal, reflexes stay with his Lua AI, mirrored observation, this fight's outcome tally, and shared boss knowledge in `logs/memory/<chr>.boss.json` and Wolf's habits/history in `<chr>.player-<name>.json` (`--brain-memory`, `--player`) | `docs/BRAIN.md` |
| Audio | The game's own FMOD Ex runtime from the install plays the game's event projects (`main`, the enemy's `cXXXX`, the map area's): TAE sounds with floor-material footsteps and armour rustle, hit, block and deflect clangs from the hit-effect params, listener on the camera with the events' own 3D rolloff; decoded samples only as a fallback | `docs/AUDIO.md` |
| Map | All ten archive maps extracted and loadable; 69 starts across nine gameplay maps walk; m89 is sky-only | `docs/MAP-BASELINE.md`, `docs/FORMATS.md` |

## Combo-aware boss session (2026-10-11)

Boss brain installation now computes a move sheet from the loaded TAE/clip/AtkParam data, including attack groups inside offered tactic plans.
Coach and mid-review input contain hit times/counts, guard/deflect posture forecasts from the shared combat resolver, clip duration, recovery, conditional cancel windows and nominal capsule/root-motion reach.
The generated Owl example and window-query mapping are in `docs/BRAIN.md`, "Combo-aware move sheet and break log".
Complete fitted Owl coach inputs measured 3916 to 4067 bytes, within the 4096-byte budget, with the current combo and break evidence retained.

Every Wolf posture break records the boss action/combo and hit index/count, state, remaining clip-clock wait, actual response start/delay and outcome.
Per-action/hit conversion totals reach the coach and mid-fight review; session reports include the totals and individual break records.
`break_conversion_delay` is mean seconds to the first landed conversion per converted break (null without one), in metrics, trend, coach result and scorecard.

The normal middle-hit Owl response already used the real TAE window in the baseline.
A repeated-current-attack response reproduced a 14.8-second late start because the runtime mistook the old clip for a new response.
It now tracks the clip clock and keeps combo cancellation enabled until the new attack starts.
Both the ordinary and repeated punish start 0.300 seconds after the middle hit, through the 0.667-second TAE window before the 2.733-second natural end.
The fixture with cancel flags removed only from an in-memory timeline starts at the first free frame, 2.400 seconds after the break.
Long data-defined recoveries also retain the queued response past the ordinary queue deadline.
Coach guidance asks for last-hit or fast-single breaks, a `finisher` set for `wolf_posture_75`, and review of failed conversions.

The final requested release sim/brain-net suite passed 360 tests across 37 targets, including the concurrent cost-accounting integration.
Final targeted Owl verification passed all 12 cases after adding the attack group inside a tactic plan, including AP1/AP6 guard/deflect forecast equality and frame-boundary assertions.
Unit regressions cover allowed/conditional/expired windows, all break outcomes, mean conversion delay and legacy defaults.
Workspace Clippy with `-D warnings` and formatting are clean.
Logs are `target/alt/combo-final-tests.log`, `combo-owl-tests.log`, `combo-unit.log` and `combo-clippy.log`.
All Cargo commands used `target/alt` and six jobs (fmt uses `CARGO_BUILD_JOBS=6`).
No model/API calls, real model CLIs, credential reads, protected memory/dataset writes, cache/re deletion or links, worktrees or commits were made.
Attack timings, hit stop and Wolf's input buffer were unchanged.

## Recording brain overlay and stages (2026-10-11)

`--brain-overlay` starts the top-right recording panel; it is off by default and F8 toggles it.
It shows the controller/model, fight/player, active set and description, current plan step and move label, checkpoint choice and age, coach thinking/notes/principles, recent bans and the post-fight scorecard.
Coach notes last eight seconds, bans five seconds and scorecards six seconds.
The wrapped, fixed-width panel uses a dark translucent background and Segoe UI, hides the debug readout while visible, and reserves space from the floating boss bars.
The driver view is independent of decision logging.

`--brain-stage lua|early|alive|outsmarted` selects the recording baseline or an isolated coach snapshot.
Lua has no memory; early uses `owl-2026-10-11-golden`, alive uses `owl-2026-10-11-alive`, and outsmarted uses `owl-2026-10-11-outsmarted`, each from `logs/datasets/<dataset>/c5060.json`.
Coach stages copy and migrate into a unique retained temporary directory, print its path, and direct all memory writes there, overriding normal brain and memory flags.
Missing/invalid datasets produce a clear error.
Named players receive the snapshot's habits/history without counting its boss evidence twice.
`--window-size WIDTHxHEIGHT` sets the play window size (default 1600x900, minimum 800x600).
PowerShell commands for each stage are in `docs/BRAIN.md`, "Recording overlay and stage presets".

Final workspace Clippy with `-D warnings`, formatting and diff whitespace checks passed.
The requested release sim/brain-net library, binary and integration suite passed all 353 tests, and all 11 game binary tests passed.
Offline tests cover transient card timing, newest coach notes, actual move labels, shared ban reasons, missing datasets, source preservation and migration of all three real stage snapshots into temporary copies.
The pre-existing game `map_tangents` test was not part of the requested suite and was left alone.
All Cargo commands used `target/alt` and six jobs; fmt uses `CARGO_BUILD_JOBS=6` because it has no `-j` option.
The final release executable is `target/alt/release/sekiro-rs.exe`; the user's release and preview executables were not replaced.

Scripted Owl fights with memory off were checked at 1920x1080 and 1280x720; every run completed under a 90-second cap.
Reviewed final screenshots: `target/alt/overlay-review/brain-1080-final-150.png`, `brain-1080-final-300.png`, `brain-720-final-150.png` and `brain-720-final-300.png` in that directory.
The raw-input F8 replay (`crates/game/scripts/brain_overlay_toggle.txt`) verified default off, on and off again, reviewed in `target/alt/overlay-review/toggle-off.png`, `toggle-on.png` and `toggle-off-again.png`.
Initial flat-floor screenshots `target/alt/overlay-review/brain-1080-150.png` and `brain-1080-300.png` were also reviewed before the font and debug-readout polish.
The requested suite log is `target/alt/overlay-review/final-sim-net-tests.log`.
No real model/API calls, real model CLIs, credential reads, protected memory/dataset writes, cache/re deletions or links, worktrees or commits were made.
Live hosted coach calls were deliberately not exercised; coach events were tested offline.

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

Useful flags: `--attack-power N` selects Wolf's Memory attack power (1 to 99, default 1); `--progression castle-roof` selects AP6, overridden by explicit `--attack-power`; both also work in `duel-sim`; `--play` alone uses a flat floor; `--enemy owl` (or `c5060`) fights the Great Shinobi Owl instead of the soldier; `--arena owl` fights him on the castle top where the game places him (only the map within 60 m; `--full-map` loads all of it; `docs/OWL.md`, "The castle top"); in play, `--hide <model prefixes>` skips map piece models and `--objects` adds MSB objects; `--lock-on`, `--no-enemy`, `--simple-ai`, `--script FILE --shots step:path --exit` for scripted runs with screenshots; `--input FILE` replays raw keys and mouse through the live input path and logs camera and movement per frame and step (`crates/game/src/input_replay.rs`); `--arena --deathblow` for the deathblow demo; `--viewer <glb>` and `--map <id>` for inspection; `--game <dir>` (or `SEKIRO_DIR`) points at the install whose FMOD DLLs play the sounds; `--volume 0.0..1.0` sets the master volume (default 0.5), and in play minus and equals (or `[` and `]`) change it in 10% steps with a short on-screen readout.
Testing aids: `--break-posture` holds the enemy's posture at 1 point outside deathblows, so Wolf's next landed hit breaks it (quick deathblow tests, also for the Owl's second life); `--spawn x,y,z,yaw,pitch` now also works in an arena, putting Wolf there and the enemy `--enemy-distance` metres (default 5) in front of him, for example next to the castle-top railing (`--arena owl --passive --god --break-posture --spawn 101.23,53.95,247.33,135,0 --enemy-distance 1.6`); `--god` makes Wolf take no HP or posture damage (hits, reactions, hit stop and knockback unchanged), so long replays are not cut short by his death; `--passive` (same as `--no-enemy`) keeps the enemy standing there with his AI off; `--force-act <id>` makes the enemy request that act whenever he is free (with `--passive`, only that act; `--force-act 3040` is the Owl's grab, script `crates/game/scripts/owl_grab.txt`); the Owl's tools on demand: `--arena owl --passive --god --force-act 3010 --spawn 104.5,54.0,244.06,135,0 --enemy-distance 4 --script crates/game/scripts/owl_tools.txt --shots ... --exit` (3010 firecrackers, 3017 explosive slash, 3030 poison, 3031 recovery seal, 3069 smoke; always give `--shots` paths under the repo, a shot that cannot be saved keeps the run from exiting); `PLAY_LOG_COMBO=1` logs one `combo` line per step (attack button, buffered presses, Wolf's state, clip and clip time, both hit stops, the step's hits, the enemy's state and clip, distance).

## In progress when the session paused

- Grappling hook and ledge hang (player/enemy behaviour).
- Fall death and respawn, FLVER tangents to cut the 14 s map load, map lighting from the game's draw params, foliage alpha.
- Audio (2026-10-10, FMOD path built, to be confirmed by ear): sounds at the TAE events' dummy polys, map reverb, music. Coverage and loudness numbers: `docs/AUDIO.md`.

Uncommitted files in the working tree belong to these tasks; check `git status` and build before committing.

### Leads for grapple and ledges (found, not built)

- Ledges: `map/<id>/<id>.edgebhd/.edgebdt` hold one `.edge` file per collision part (16 files, 255 edges on m11). Header `u32 version=4, u32 count, u32 collision number, u32 0`, then 64-byte records: two vec4 endpoints, a zero vec4, `u32 chain id`, `u32 flags` (low byte 2 or 3, second byte 0..3 or 10, meaning unknown), 8 bytes padding. Endpoints are in the collision part's local space; apply its layout transform. `.wallbhd/.wallbdt` is likely wall-hug data, undecoded.
- Grapple: map pieces have no dummies; object `o000100` (placed 67 times on m11) carries dummy 90 and is very likely the grapple point. Search rules: WirePointSearchParam row 0 (act 3 to 15 m, find to 25 m, filter and soft-lock boxes); WireSetParam picks search rows per area; WireVariationParam (angles, marker type, homing offset) is probably set per placed object in MSB fields the parser still skips.
- Mikiri and perilous warning are implemented; see the counter session below and `docs/COMBAT-RULES.md`.
- Study tools: `crates/sim/examples/bxf_list.rs`, `dummy_census.rs`.

## Decision needed from the user

- 62 of 174 sound banks (`sm*`, `smain`, `vm*`, `xm*`, `rm*`: area SFX, voices, music, cutscenes) are encrypted FSB that the game's FMOD cannot open without the game's key. Footsteps, hits, clangs, Wolf's and the Owl's sounds are in the plain banks and play; the encrypted ones stay silent. Reading them means decrypting with a key from the game; ask before touching them (see `docs/AUDIO.md`).

## Mikiri Counter session (2026-10-11)

Soldier act 3013 and Owl act 3033 now counter when Wolf steps forward into their thrust during his real TAE ChrActionFlag 125 window, 0.000 to 0.400 s.
The incoming blow and on-hit effects are cancelled, and the contacted AtkParam's `staminaDamageAttackHitParry` deals 90 posture to the enemy.
ThrowParam suffix 190 plays Wolf's per-enemy `511800` counter and enemy `ThrowDef13800`; suffix 120 plays `511100` / `ThrowDef13100` when the damage breaks posture (the soldier by default).
The pair uses its actual absorb dummy and 0.5 s interpolation, and releases through Wolf's existing animation cancels or end, with env 273 / 206 supplied to the enemy throw hook.
No input buffering, movement, attack timing, hit stop or combo logic changed, and no fixed delay was added.
FwLo's RustSekiro supplied the rule and pair selection, checked against our extracted data and credited once in its existing CREDITS section; the requested pull failed because this environment could not connect to GitHub.
Details, data versus inferred rules, and limitations (latent skill multipliers and automatic posture-break deathblow follow-up) are in `docs/COMBAT-RULES.md`, "Mikiri Counter".
The real-data reproduction first failed with Wolf hit at tick 46; the fixed 2 m replays counter for both enemies and verify their reaction states, Wolf's animation, no incoming damage, and release.
The ten perilous tests also cover early/late, backward/side, unlocked stepping away, sweeps and ordinary slashes, and confirm the 0.400 s data window and counter dummy rows.
The full release sim suite passed (268 tests); workspace Clippy with `-D warnings`, formatting and diff whitespace checks passed.
Owl's smoke blinding hit remains a normal hit and its existing regression passes.
Logs: `cache/mikiri-tests-final.log`, `cache/mikiri-focused.log`; scripted demo: `crates/game/scripts/soldier_mikiri.txt`.
The game release executable was built in `target/alt`.
The soldier demo completed in about eight seconds under a 55 s cap, saving `cache/screens/mikiri_soldier_55.png` and `mikiri_soldier_70.png`.
Both screenshots were inspected; the blade stomp is visible, and `cache/mikiri-game.err.log` confirms `ThrowAtk511100` / `ThrowDef13100` from tick 47 with no hit stop.
All Cargo builds and tests used `target/alt` and six jobs; cargo fmt uses `CARGO_BUILD_JOBS=6` because it has no `-j` option.
No commits, links, junctions, protected brain edits or game-install writes were made.

```powershell
.\target\alt\release\sekiro-rs.exe --play --passive --god --force-act 3013 --lock-on --enemy-distance 2 --script crates/game/scripts/soldier_mikiri.txt --shots 55:cache/screens/mikiri_soldier_55.png,70:cache/screens/mikiri_soldier_70.png --exit
```

## Known issues

- Done 2026-10-11, to be tried in play: the boss **coach** (`--brain coach`, `docs/BRAIN.md` "The coach"). Between fights a reasoning model (`--coach claude|codex|api`, default `api` = Responses, `gpt-6-luna`, low effort, 45 s; `claude` = slim `claude -p --model claude-haiku-5-5`, no API key, `--coach-model`, `--coach-effort`) reads the fight summary (about 4 KB: result, trend, per-plan and per-set outcomes, Wolf's habits, the plan played, its own notes) and writes a versioned game plan (opener, 2 to 4 named plan sets, start set, avoid, notes), validated against the boss's actions. Fight 1 with no memory starts with his own Lua AI and can be reviewed mid-fight; the plan arriving during the respawn plays from fight 2 (opener, then the start set), a late one a fight later, a failure keeps the old plan, and the newest plan is saved in `logs/memory/<chr>.json`. In the fight only the Decisions API picks a set at the checkpoints, with a tiny request (about 100 tokens of numbers plus the set names). The console prints the request and a 3 to 5 line summary of each plan. Against reward hacking: winning faster is the first goal; new metrics `attacks/min`, `damage/min` and `passive_share` (a fight above 35% passive failed), and hard limits in the driver (an attack at least every 6 s, one retreat per 8 s, plans need an attack). Also fixed: an attack after `retreat12` or a strafe swung at empty air; it now turns to face Wolf first (30 degree rule). Tests `crates/sim/tests/coach.rs`, `crates/sim/src/brain/tests.rs`, `crates/brain-net/tests/offline.rs` (mock servers and a fake CLI only). Not verified: any real API, `claude` or `codex` call (flag names are from `--help` output), that the strict schema with `$defs` is accepted, that 6000 output tokens suffice for `medium` reasoning, how the plans look in play.

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

## Collision-hole audit (2026-10-11)

The headless tool is `crates/sim/examples/collision_holes.rs`; results and top-ten lists are in `docs/COLLISION-HOLES.md`.
All nine walking maps have JSON under `cache/holes/`, with screenshots and review notes under `cache/screens/holes/` and `cache/holes/<map>.review.json`.
The final audit found 208,241 raw render/collision discrepancies, reproduced 119 with Body, retained 12 after visual review and rejected four visual false positives.
Unreviewed and inconclusive cases remain unconfirmed; raw counts are not gameplay defect counts.
There are 360 real-movement checks and 18,432 seeded capsule probes, each with 64 moves and subdivisions no longer than 0.1 m.
Floor sampling uses 0.5 m cells, slopes up to 50 degrees, a 0.3 m collision tolerance and nearby collision plus line-of-sight filtering.
Body floor confirmation requires more than five ticks over 5 m below the specific render witness while within 1 m horizontally, preventing unrelated ledge falls from confirming a shallow gap.
Large unlabelled background meshes required a conservative collision-neighbourhood voxel mask, and render walls use a BVH to keep memory around 2 GB per worker rather than duplicating triangles across a grid.
The final full scans plus latest cached validation passes total 1,283.46 seconds (21.39 minutes).
The distributed pipeline ran from 23:17:48 to 23:34:20 UTC on 10 October (16.53 minutes to the last full scan), using three gpt-6-luna workers with two Rayon threads each; screenshot review, tuning pilots and build checks are additional time.
Mibu review cameras use `--groups display --hide m9,m8` and stable Body eye positions to avoid occluding far/helper pieces; the headless audit still loads all map pieces.
Confirmed here means a Body reproduction plus a visually retained surface; reachability is a nearby-surface/start-distance proxy, and enclosure parity does not prove watertight solid geometry.

Rerun from the repo root:

```powershell
$env:CARGO_TARGET_DIR='target/alt'
$env:CARGO_BUILD_JOBS='6'
$env:RAYON_NUM_THREADS='6'
& "$env:USERPROFILE\.cargo\bin\cargo.exe" build --release -p sekiro-sim --example collision_holes -j 6
.\target\alt\release\examples\collision_holes.exe cache/maps --spacing 0.5 --probes 2048
.\target\alt\release\examples\collision_holes.exe --report
```

The six release tool tests pass, including the missing floor patch, solid floor and 100 m thin-wall sweep.
Final `cargo clippy --workspace --all-targets -j 6 -- -D warnings` and `cargo fmt --all -- --check` pass with the alternate target configuration above; the tool file was formatted separately to avoid altering protected concurrent edits.
A broader workspace test attempt during concurrent brain edits stopped on a temporary `BrainRequest.brief` compile error; the subsequent workspace Clippy check passes, and no full-workspace test success is claimed.
The known pre-existing `map_tangents` failure was not changed.
Tracked task changes are the new example, dev-only Rayon dependency in `crates/sim/Cargo.toml` and Cargo.lock, `docs/COLLISION-HOLES.md`, and this status section.
No collision engine code, protected brain files, game-install files, links, junctions or commits were changed by this audit.

The concurrent brain agent's commit 9809964 included this shared status section; this audit issued no commit, and the collision tool, dependency changes and report remain uncommitted.

## Playtest fixes (2026-10-11)

No commits were made.
Coach selection now defaults to the Responses API with gpt-6-luna, configurable low reasoning effort and a 45 s timeout; strict schema, store:false and usage accounting are retained.
Claude receives our replacement system prompt separately from its compact stdin summary, an empty strict MCP config, no settings sources, safe mode to skip CLAUDE.md/customizations without changing subscription authentication, low effort, no tools or persistence, and model claude-haiku-5-5.
Codex ignores user config (including configured MCP servers), uses low reasoning effort and retains its ephemeral read-only invocation.
All flags were checked only against installed CLI help; Claude's dynamic-section flag is ignored with a replacement system prompt and was omitted.
Exact argument vectors and the empty MCP file are checked through the fake CLI; no real API requests, real authenticated CLI runs or credential reads were performed.
Live coach latency, context size and provider acceptance remain unverified.

Checkpoint requests now prefer posture_half at 50% posture damage and posture_critical at 85%, re-arming below 25%, with half_hp retained as a once-per-fight fallback.
A critical crossing supersedes an unsent half-posture or HP warning; a jump over both thresholds requests critical directly.
Coach hints, instructions and docs include these events; regression tests cover two cycles, exact boundaries and fallback.

Lock-on already dropped its target on HP death, but MoveDirection retained the last strafe while a stick stayed held, steering a sideways clip along the free body's facing.
The common release now resets that selector and walk twist, publishes the HKS lock flag and is used on deathblow start as well as death, manual unlock and smoke lock breaks.
Lethal finishing pairs reject relocking, and HP deaths release the camera target in the contact step.
The failing real-script replay moved [0.545, 1.543] m in X/Z over 0.5 s after death; the fix moves [2.165, -0.308] m for a camera-relative right stick.
Tests cover the HKS lock query, dead-target refusal, held-stick free movement, locking after reset and relock attempts during/after a lethal deathblow.

The normal-Lua soldier pressure replay at 1.6 m with attack presses every eight frames for four seconds already passes before a gameplay change: one landed hit (tick 26), one block (64), one deflect (98), no posture break.
His Lua requests EndureAttack guard 3100 and deflect 3101, the guard reactions expose guardLevel 4 and SpEffect 200250, and his next counter request is 3008.
There are no unanswered AI calls in the replay; baseline evidence is logs/soldier-pressure-before.log, and soldier_guards_mashed_slashes retains the regression.
No forced guard, input, attack timing, hit stop or combo/cancel change was added.
The reported soldier stun lock remains unreproduced in this setup; an exact failing replay is needed to establish any remaining root cause.
Details are in docs/BRAIN.md, docs/AI.md and docs/COMBAT-RULES.md.

Validation: cargo fmt --all and its check, workspace/all-targets Clippy with -D warnings, and the requested release sekiro-sim plus sekiro-brain-net lib/bin/integration suites passed (304 tests).
All Cargo invocations used target/alt and six build jobs; fmt uses CARGO_BUILD_JOBS because it has no -j option.
Logs: logs/playtest-fixes-tests.log and logs/playtest-fixes-clippy.log.
No rendered game session was run; real scripts, graphs, Lua AI, TAE and params were exercised through the deterministic duel tests.
No cache/re links, junctions, worktrees, deletions or game-install writes were made.

## Combat fundamentals and actionable checkpoints (2026-10-11)

Added `docs/COMBAT-FUNDAMENTALS.md`: sourced base mechanics in original words, a rule-by-rule code/test audit and six fight-impact-ranked fidelity gaps.
The largest gaps are general/thrust dodge i-frames, unverified complete poise/guard-counter parity, and player progression scaling; combat mismatches were documented without changing simulation rules.
Checkpoint posture events are now `pressure` at about 6 estimated Wolf hits left and `pressure_critical` at about 3, with no pressure warning below 2 hits.
The estimate uses Wolf's first combo TAE judge and the same AtkParam, weapon, reinforcement, enemy defence and active-effect posture formulas as the duel.
Each warning fires once per posture cycle and re-arms when remaining posture recovers above 70%; HP-half is retained as a fallback.
Requests are suppressed during enemy deathblow openings, posture breaks, damage staggers, throw/get-up reactions, death and respawn.
Queued pressure waits through a brief stagger and is revalidated before sending; deathblow openings discard obsolete pressure.
`mark_lost` only fires with another mark remaining: none for the soldier, one after Owl's first deathblow and get-up.
Coach instructions now include a 447-character combat paragraph; the schema, hint names and `docs/BRAIN.md` use the new events.
The real-data offline regression first demonstrated the old percentage timing and later caught a get-up request before Owl could act; the final soldier/Owl deathblow regression passes with suppression through the throw state.
Validation passed: `cargo fmt --all`, `cargo fmt --all --check`, workspace/all-targets Clippy with `-D warnings`, and the requested release sim plus brain-net lib/bin/integration suites (305 tests).
All Cargo commands used `CARGO_TARGET_DIR=target/alt` and six jobs; fmt uses `CARGO_BUILD_JOBS=6` because it has no `-j` flag.
Final release test log: `target/alt/combat-fundamentals-final-tests.log`.
No commits, real model/API calls, real Claude/Codex CLI runs, credential reads, game sessions, game-install writes, worktrees, or cache/re links, junctions or deletions were made.

## Plan spacing, deflect and timed back steps (2026-10-11)

Plan attack approaches now evaluate the real Lua `Approach_Act_Flex` range check when the attack step activates, after earlier tactics, instead of while the whole plan is queued.
The soldier's action-table distances already match `101000_battle.lua`; his hit radius was subtracted once and NPC TAE 760 lunges already work.
The actual range reproduction is `retreat12>3000` with stationary Wolf initially 1.6 m away: before, attack start 4.57 m, first active frame 3.19 m, observed capsule/root-travel reach 4.29 m, no contact; after, start 2.02 m, first active 0.80 m, reach 3.92 m, contact.
The six reported plans contact stationary Wolf at 2.5, 4 and 6 m even on the original runtime, so their historical whiff rates were not reproduced as pure stationary range failures.
Owl's firecrackers now use `506001_battle.lua` Act07's 3.5 m approach and reset pressure number 5 to zero instead of the previous approximation.

A second bug inflated the evidence: movement, guard and back-step exchanges were counted as whiffed attacks because they contacted nothing.
They still contribute usage and punishment but cannot whiff in the tally, per-plan/per-set summaries or outcome logs.
The 15 s offline `walk_in4>3000+3001` fight runs four plans and lands eight attacks: its count changes from [12,8,0,0,4,0] to [12,8,0,0,0,0].
Existing memory files were not read or rewritten; historical totals retain their old false whiffs.

Both bosses offer `deflect` and `dodge_back` and five new attack chains each; all chains are listed in `docs/BRAIN.md`.
`deflect` faces Wolf for up to 1.5 s and routes the real parry-timing interrupt through the boss's own battle callback into a local queue, preserving the following attack after its guard/deflect response.
The soldier's rank, facing, weapon reach, interval and consecutive-guard roll still choose guard 3100 versus deflect 3101; the duel's guard judge, repel defence, active effects and attack data still decide the contact.
Owl uses his own parry callback.
`dodge_back` watches Wolf's visible attack wind-up within 2.5 m and requests soldier 5211 or Owl 5201, then continues the attack chain.
The soldier's pressure replay guards then requests 3000, finishes readiness after 36 ticks, and its timed short step gains 0.86 m and finishes after 38 ticks before requesting 3002.
Both idle waits finish after 90 ticks at 60 Hz.
Both caps feed anti-stall lead time; `dodge_back` also uses the eight-second retreat allowance.

The coach summary now includes per-plan mean [attack-start distance, observed melee reach] in metres, rounded to two decimals.
Reach samples actual animated dummy-poly capsules, AtkParam radii, root travel and Wolf's body radius; projectile-only and interrupted-before-active attacks have no melee sample.
The radial envelope and averages are diagnostics, with direction, timing and target movement still relevant.
Ranges trim with plan outcomes and the worst-case summary stays inside 4096 bytes.
Updated `docs/BRAIN.md` and `docs/AI.md` document the mechanics, limitations, chains and before/after evidence.

Validation passed: cargo fmt --all and its check, workspace/all-targets Clippy with -D warnings, and the requested release sim plus brain-net lib/bin/integration suite (310 tests).
The final coach wording was also rechecked through the complete offline brain-net suite.
The range regression exercises all thirteen soldier attacks and nine mixed plans at 1.6, 2.5, 4 and 6 m (88 cases), plus dedicated retreat-whiff, reactive soldier/Owl pressure, timeout, telemetry/budget and non-attack-whiff tests.
The Owl tally test now checks contact/whiff outcomes only for attacks and separately verifies that back steps cannot whiff.
Logs: `target/alt/plan-range-before.log`, `plan-retreat-before.log`, `plan-combat-tests-final.log`, `plan-protocol-tests.log`, and `plan-combat-clippy.log`.
All Cargo commands used target/alt and six jobs; fmt uses CARGO_BUILD_JOBS=6 because it has no -j option.
This task made no commits, real model/API calls, real Claude/Codex CLI runs, credential reads, live game runs, game-install writes, worktrees, or cache/re links, junctions or deletions.
Input buffering, attack timing, hit stop and Wolf's movement were not changed by this task.
Concurrent --immortal edits in the game and Duel were left intact and are separate from this work.
## Five playtest fixes (11 October 2026)

Raw duel input now determines whether a fight is valid: at least three relevant presses in the first ten seconds and any relevant held input during at least 40% of its duration.
Void fights print `fight void: no player input` and do not enter history, tally, habits, wins, plan failure evidence or post-fight coach calls; current-fight autosaves also exclude inactive evidence.
The window requests focus and cursor lock on the first input frame and re-grabs on the first click inside it.
Windows can still refuse foreground activation.
The no-input integration test uses a temporary memory file and fake coach; active-fight save tests remain green.

The normal-AI, non-god posture reproduction sampled a soldier block at tick 64 (Wolf attack 5000011) and deflect at tick 98 (5000050).
Both attack rows specify zero attacker posture return, and Wolf's remaining posture stayed 120 to 120.
The executable research confirms the attack-row return fields and the deflect multiplier path, so no unsupported nonzero return damage was introduced.
Nonzero NPC return tests still cover 45, 22, 11 and 5 posture damage.
Matched original-game contact footage remains unverified; the audit and limitation are recorded in docs/COMBAT-RULES.md.

Held-stick movement retained the old heading because free_move_start_turn skipped an already moving Wolf after lock release.
Release now clears the remembered heading and queues the camera-relative turn when movement is permitted, including after deathblow recovery.
The reproduction's backward-stick sideways drift was 0.600 m in half a second; all four stick directions at three camera yaws now have less than 0.000002 m sideways drift.
The raw-key replay is crates/game/scripts/held_move_kill.txt.
The final live run held W through a soldier deathblow and recovery, logged lock false and zero direction error, and completed in under ten seconds (exact PID 13736, 60-second cap).
Its first-frame log reports focused true and cursor Locked.
The screenshot target/alt/screens/held_move_kill_final.png shows the soldier at zero HP during the deathblow.
Saves were backed up to the local temporary sekiro-playtest-saves-20261011-015359 directory before game sessions.

Per-plan uses, deflected_all and fatal counters persist in coach memory.
Three uses with a summed failure rate of at least 0.6 exclude the plan from sampling and openers, including across subsequent fights.
Fatal attribution covers lost marks, death within three seconds of the final attack exchange and a punished posture break, including death during the final attack.
Only a validated coach retry with a reason re-allows a plan; historical counts remain intact and new failures can ban it again.
Void fights roll back provisional bans and never persist their failures.
Coach summaries include banned choices and packed failure counters within 4096 bytes, tested with the entire known vocabulary banned.
Codex defaults are gpt-6.1-sol and medium effort, with both CLI overrides tested through a fake executable.

Coach and checkpoint instructions reward HP plus posture damage per minute, fewer perfect deflects and survival while passive_share is below 0.35.
Engaged_time appears in metrics, scorecards, trends and coach input; it is duration below that passivity threshold and zero otherwise.
Retreat allowance is once per five seconds and permits retreat chains followed by an attack, while preserving the six-second attack rule and passivity tracking.
The checkpoint body still fits its 1200-byte limit.

Validation passed: cargo fmt --all and --check, workspace/all-targets Clippy with -D warnings, the requested release sim and brain-net lib/bin/integration suite (321 tests), and the release game build.
All Cargo work used target/alt and six jobs; fmt used CARGO_BUILD_JOBS=6 because it has no -j option.
Logs are target/alt/playtest-clippy-final.log, playtest-tests-final.log, playtest-build-final.log and playtest-game-final.err.log.
Updated docs/BRAIN.md and docs/COMBAT-RULES.md document behavior, evidence and the remaining original-game posture parity question.
No commits, real model/API calls, real Claude/Codex CLI runs, credential reads, protected memory-log edits, game-install writes, worktrees or cache/re links, junctions or deletions were made.
Attack timing, hit stop and the input buffer were unchanged.

## Owl-driven brain session (2026-10-11)

Boss-only pressure checkpoints and aggregate Lua plan counts hid Wolf's break opportunities and the individual moves' performance.
The read-only golden Owl dataset confirms five boss wins, six 3040 grabs that all whiffed, four punished grabs and 127/64/49 Lua acts in its last three fights.
Wolf posture checkpoints now fire at 25%, 50% and 75% damage once per cycle, re-arm below 15%, and report breaks without requesting during deathblows, boss breaks, death or respawn.
Metrics, scorecards, trends and coach input now include Wolf breaks, hits within 2.5 seconds after a break, the fraction of breaks punished, peak Wolf posture and boss-win time to kill Wolf.
Multiple hits after one break do not inflate the punish rate above 100%, and the breaking blow is excluded.

The coach can author one to three weighted on_wolf_break responses, validated against the normal action vocabulary and avoid list.
The fighter replaces Owl's Act40 grab follow-up locally and retains the response through delayed Lua replanning until its first attack starts or the punish window expires.
A single response attack uses the existing ComboFinal cancellation path and preserves the current animation's TAE windows.
The real-data scripted slash-to-break regression starts response 3000 after 18 ticks (0.300 seconds); the same setup without a response plan requests Lua grab 3040 after two ticks.
Response outcomes persist separately and have independent on_wolf_break:<plan> failure evidence, with the existing three-use/60% ban and reasoned retry rule.

Combined Lua weights are capped at 25% per set; the coach is instructed to mix at least three different attacks, with warnings for narrower legacy sets.
Coach input now reports individual action outcomes, including Lua choices and zero-count untried actions, plus measured attack distance versus reach.
Up to six principles of 90 characters each persist in the game plan, return on every coach call and print in plan summaries.
The coach input limit is 8192 bytes to retain the new evidence and lessons; the in-fight chooser still uses its tiny existing input.
F9 and normal game exit print a session action/range table and fight trend and append to logs/brain-session-<timestamp>.md.
Report tests use temporary files, and the game enables reports for the real Lua AI as well as controlled brains.

Brain validation passed: owned-file formatting, shared workspace formatting checks, all-target Clippy with -D warnings and the complete requested release sim/brain-net lib/bin/integration suite (333 tests).
The final shared checks include the parallel posture agent's updates; all Cargo work used target/alt and six jobs.
Logs: target/alt/brain-break-final.log, brain-unit-final.log, brain-tests-final.log, brain-clippy-final.log, brain-clippy-core.log, brain-clippy-changed-tests.log and brain-fmt-final.log.
Live model plan quality, rendered F9 behavior and parity against the original game's fights remain unverified.
The brain work did not edit combat/* or posture_recovery.rs, commit, call a real model/API/CLI, open credentials, alter protected memory/dataset logs, create worktrees or cache/re links, or delete cache/re contents.
Attack timing, hit stop and the input buffer were unchanged by the brain changes.

## Guarded-pressure posture recovery (2026-10-11)

Report: the Owl's posture barely rises while he guards.
Finding: the engine's one "no recovery" flag is HKS `act(110)` (`SetStaminaRecoveryDisabled`, bit 21 of the action state's +0x88; its only writer is the HKS command handler `b32410`, reached from `b4e520`).
No shipped script calls it, so it is not the cause; the sim now honours it anyway (`CombatRules::recover_with_state`, `PlayerEnv::recovery_disabled`, `act(110)` in `player.rs`), for NPCs and Wolf alike.
There is no post-damage delay timer in the recovery update (`a04850`); pauses come from TAE 960 types and TAE 225.
The Owl's guard `a000_003100` has neither, so at full HP he recovers 60 per second while guarding (0.3 s of 0% in the guard-hit reaction `a000_008501`), and a guarded Wolf slash costs him only 14 to 15 of 660.
Sim numbers (Wolf slashing a forced guard every 0.6 s for 10 s): Owl 660 -> 646 -> 646 -> 654 -> 624 -> 634 -> 646, never below 624 and back to about 650 within 0.5 s of each hit; the soldier (guard recovers at 25% of 20) 75 -> 45 -> 15 -> 0, broken in three hits.
No recovery rule was changed, because the data and code say this is what the game does at full HP; whether the live Owl builds faster is unverified and the next suspects are guarded posture damage and progression scaling (`docs/COMBAT-FUNDAMENTALS.md`, mismatch 4).
Tests: `crates/sim/tests/posture_recovery.rs`, `act_110_disables_posture_recovery` in `player.rs`.

## Mid-fight coach reviews (2026-10-11)

Coach mode now reviews after 45 seconds of fight time since the last send, or early after a surviving boss mark loss, two Wolf posture breaks, or all plans in the active set being banned.
The first 20 seconds, inactive-player fights, posture breaks, deathblows and respawn block sends; one coach call is outstanding at most, and a pending post-fight review supersedes a mid-fight trigger.
`--coach-interval <secs>` defaults to 45; 0 disables every mid-fight trigger.
Mid requests use `phase: "mid"`, partial fight metrics and outcomes first, elapsed time, both fighters' HP/posture, boss marks and the current plan, within 4096 compact JSON bytes.
Validated replies wait for the next non-reflex decision slot, preserve existing attack goals and skip their opener, while being saved through the normal memory path.
Console and JSONL telemetry identify the phase and reason; fight summaries count mid reviews and their completed latency.
The objective is winning faster, breaking Wolf posture and hitting after, dealing HP plus posture damage per minute, then taking little HP plus posture damage and limiting Wolf perfect deflects.
Passive share above 0.35 fails regardless; engaged time is information and only a small losing-fight tiebreaker.
`boss_damage_taken_per_min` is persisted in metrics and included in scorecards, trends and coach results.
Offline regressions cover the 120-second 45/90 cadence and post call, mark loss at 30 seconds resetting the next timer to 75, serialized slow calls with post-review priority, deferred plan application and opener skipping, the 20-second floor, zero disabling, request budgeting and mock HTTP mid-phase telemetry.
Validation passed: cargo fmt --all and --check, workspace all-target Clippy with -D warnings, and the full requested release sim/brain-net lib/bin/integration suite (340 tests).
All Cargo work used target/alt and six jobs; logs are target/mid-final.log and target/mid-clippy-final-pass.log.
The worst-case input regression fitted 4078 bytes under the 4096-byte limit.
The all-banned check excludes Lua fallback choices so they cannot hide a failed coached set.
No commits, real model/API/CLI calls, credential reads, protected log edits, combat edits, worktrees, cache/re links or cache/re deletions were made by this task.

## Wolf's attack power (2026-10-11)

Both the game and `duel-sim` accept `--attack-power 1..99` (default 1) and `--progression castle-roof` (AP6, required-route Memories); explicit AP overrides the preset.
Damage reads the real `SpEffectParam` Player Growth_Core row `160000 + AP`: element `*AttackRate` fields for HP and `staminaAttackRate` for posture before truncation.
All 99 rows are checked, including the AP52 exception (6.775, followed by 6.77 at AP53); no hand-written progression table drives gameplay.
CalcCorrectGraph 500/501/504 still describes Wolf's separate Prayer Necklace growth, fixed at 1, rather than being fed Memory attack power.
The local RustSekiro clone was checked and leaves its AP formula unresolved; the extracted rows and community Attack Power/Memory references are documented in `docs/COMBAT-RULES.md`.
Combat fundamentals mismatch 3 is resolved for attack power; full Prayer Necklace progression and live-game parity remain outside this setting.
Real-script opening slash HP/posture at AP1/5/10: soldier 80/30, 160/60, 260/97 before the remaining-HP cap (195 actual HP damage at AP10 on a fresh soldier); guarding Owl 0/15, 0/30, 0/48.
At AP6, the existing ten-second pressure cadence (one attack press every 0.6 s) lands 11 guarded contacts and lowers Owl's remaining posture to 512/660, versus 624/660 at AP1, without changing his 60/s guard recovery.
The posture builds but does not break within this run.
Validation: workspace/all-targets Clippy with -D warnings, cargo fmt --all and its check, and the full requested release sim/brain-net lib/bin/integration suite passed (344 tests).
The alternate release game executable was built; actual CLI checks rejected invalid/missing values, and duel-sim confirmed explicit AP5 overrides castle-roof with a one-tick passive run.
Logs: target/alt/ap-baseline.log, ap-focused.log, ap-focused-final.log, ap-suite.log, ap-clippy-final.log, ap-game-build.log and ap-cli.log.
All Cargo commands used CARGO_TARGET_DIR=target/alt and six jobs; fmt uses CARGO_BUILD_JOBS=6 because it has no -j flag.
No commits, real model/API calls, authenticated CLI runs, credential reads, protected-log edits, game sessions, game-install writes, worktrees, or cache/re links, junctions or deletions were made.

```powershell
$env:CARGO_TARGET_DIR = 'target/alt'
$env:CARGO_BUILD_JOBS = '6'
& "$env:USERPROFILE\.cargo\bin\cargo.exe" run --release -j 6 -p sekiro-game -- --arena owl --progression castle-roof
```

## Owl's phase-2 deathblow protection (2026-10-11)

Reproduced with a real first deathblow in the sim: attack during `a000_020005` immediately started the finishing pair 15060180 / 15060181.
Deathblow selection now rejects the active TAE judgment marker 3506080, which covers the heat-up, kneeling loop and first 0.233 s of the escape.
The red mark and input both use this check; ordinary slashes stay available.
The transition also carries ChrActionFlag 24 throughout: native `b51b70` sets action +0x88 bit 16, `bdbb40` reads it, and `b6e6a0` stores it in damage record +0x1CA.
Implemented its traced damage-level-zero and no-knockback behavior (`b68d00`, `b6bfe0`) and the HKS `env(349)` answer, so a landed slash cannot replace the protected animation with posture collapse.
Owl's unchanged castle Lua detects Wolf's attack through `INTERUPT_ParryTiming`, checks 3506080 and its parry range, and requests escape 20007.
The escape restores HP and posture from its TAE, returns the phase-2 boss bar and keeps one mark; posture break and HP depletion both still open the finishing deathblow afterward.
The user's real-game confirmation replaces the previous uncertainty about a deathblow while begging in `docs/OWL.md`.
That document separates the exe-traced damage-motion rule from the marker-based transition gate; the complete native throw-defence/ThrowKind condition dispatcher remains unreconstructed and is recorded in `docs/FIDELITY.md`.
The real-data regressions test no red mark or second deathblow in both transition clips, landed ordinary slashes preserving the transition, Lua escape 20007, and the finishing blow after HP depletion alongside the existing posture-break victory.
Focused `owl_phase2`, `owl` and `deathblow_lock` tests passed.
Final validation: `cargo fmt --all`, workspace/all-targets Clippy with `-D warnings`, and the full requested release sim lib/bin/integration suite passed (313 tests, zero failures).
Full suite log: `cache/tests/owl-transition-final.log`; local native research: `re/exports/decomp/owl_transition/`; parameter evidence: `cache/tests/owl-transition-params.txt`.
All Cargo commands used `CARGO_TARGET_DIR=target/alt` and six jobs; fmt uses `CARGO_BUILD_JOBS=6` because it has no `-j` option.
No commits, real model/API calls, protected-log edits, game sessions, game-install writes, worktrees, or cache/re links, junctions or deletions were made.

## Shared boss knowledge and player profiles (2026-10-11)

Boss knowledge is now shared across player profiles, with counted action/plan/break-response evidence, reasoned bans, six universal principles and the best known sanitized game plan with its results.
`--player <name>` selects isolated habits, history, notes, player principles, local failure bans and the current tuned plan; the default is `default`.
A fresh player starts immediately with the shared plan at 20 recorded fights, while thinner knowledge uses the Lua baseline.
Mid-fight review defaults to 30 seconds, with the interval flag retained.
Coach requests separate boss knowledge and player profiles under the same 4096-byte budget; responses add `player_principles`, `boss_principles` and `boss_bans`.
Old `principles` replies remain readable as player principles.

First load copies legacy memory to a byte-identical `.v2-backup.json` and creates `.boss.json` plus `.player-default.json`, retaining the original.
Unclassified legacy principles remain local until the coach explicitly promotes universal lessons.
Temporary-copy migration preserved the outsmarted Owl dataset's 21 fights, 20 boss wins, one Wolf win and 656 action/plan exchanges, and the alive dataset's seven fights, six boss wins, one Wolf win and 281 action/plan exchanges.
Both migrations retained all break-response counts and backup bytes, and repeated loads did not inflate totals.
New offline regressions cover rich/thin knowledge in a real extracted-data duel, shared coach lessons, isolated habits and bans, repeated/stale saves and full-layer Unicode budget pressure.
Details and file examples are in `docs/BRAIN.md`.
No commits, model calls, real coach CLIs, game runs, links or junctions were made, and protected memory/dataset files were untouched.
All Cargo commands used `target/alt` and six build jobs; formatting uses `CARGO_BUILD_JOBS=6` because cargo fmt has no `-j` option.Formatting, workspace Clippy with `-D warnings`, and the requested release sim/brain-net lib/bin/integration suite passed (351 tests across 37 targets).
The final full-layer budget regression includes duplicated boss lessons/bans inside the played plan and confirms they are removed before fitting.
Validation logs: `target/alt/shared-memory-tests-final.log`; no `map_tangents` exception was needed for these checks.

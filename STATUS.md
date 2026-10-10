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

- Fixed 2026-10-10, to be confirmed by ear: nearly no sounds in the Owl fight and volume that jumped near walls and corners. The Owl's bank was never decoded and Bevy's spatial audio scaled every sound by the camera's distance with no minimum. Sounds now play through the game's own FMOD (`crates/fmod`, `crates/game/src/audio.rs`): all 55 of the Owl's TAE sound keys play, and the events' own rolloff is flat out to about 5 m. The hit-sound lookup also used the wrong param fields and letter; fixed from FwLo's exe trace. `docs/AUDIO.md`.

- Mouse look was far too sensitive (reported 2026-10-10); `MOUSE_SENSITIVITY` in play.rs is now 0.0012 radians per mouse count (was 0.0035), not yet confirmed by the user.
- Fixed 2026-10-10, to be confirmed in play: movement "inverted / tank-like" after turning the camera. Free move starts picked the locked-on strafe clips (back, left, right) from the stick's angle to Wolf's facing while steering turned him to the stick, so he side-stepped or ran backwards; now a free move start turns the body to the stick and keeps the turn as the lower-body twist (`TwistLowerRootAngle`, drawn as a smooth turn), so the script picks the forward start or its 180 degree turn-and-go. That snap-and-twist model is inferred from the script and graph, not traced in the exe. Locked-on strafes now follow the stick (engine `MoveDirection` with the WalkTwist cones, and bound graph selectors re-read when a variable changes). Tests: `crates/sim/tests/camera_turn_movement.rs`.
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

1. Owl on the castle top (`docs/OWL.md`, "Next steps"): the fight runs there (`--arena owl`); next decode the map event script (`event/m11_01_00_00.emevd`) for his phase 2 and for which top floor variant shows, then NPC-on-player throws, firecracker and poison bullets.
2. Finish the three in-progress tasks above.
3. Validation against the real game: record posture/HP/animation traces in a real fight (read-only) and compare with `duel-sim` output.
4. More enemies from the m11 layout (43 soldiers placed in the MSB).
5. Combat arts, prosthetics and items only after the core loop is solid.

## Publishing

The repo is public at github.com/AKJama/sekiro-rs.
Local git hooks (`.git/hooks/pre-commit`, `commit-msg`) reject personal paths, emails, co-author lines and anything under `cache/` or `re/`; they are not part of the repo, so re-create them on a new clone.
Write commands in docs as plain `cargo`, never a full user path.

# Genichiro Ashina on the castle keep

The castle fight uses `c7100:71001000`, not a final-game Way of Tomoe row.
The placement is `m11_01_00_00/c7100_0000`, entity 1110800, at Bevy coordinates `[107.829, 54.044, 235.440]`, yaw 39.616 degrees.
NpcParam gives 1598 HP, 660 posture and `ninsatuNum = 2`.
NpcThinkParam 71000000 selects logic and battle goal 710000 from the castle's `m11_01_00_00.luabnd.d`.
The boss name comes from the installed English NPC FMG, and the map shows boss bar 907100.

## Resources and generalisation

The registry reads extracted MSB layouts, NpcParam, NpcThinkParam, available Lua bundles and NPC names.
An omitted row selects the most frequently placed row, breaking ties toward the lower id.
Explicit selection uses `--enemy cXXXX:row`; `soldier` and `owl` remain compatibility aliases.
Missing scripts in the placement's map bundle fall back to the first sorted bundle containing both think scripts.
That fallback is deterministic but is not a reconstruction of the native bundle selection algorithm.

`normalChangeAnimChrId` selects the HKS, graph, skeleton, clips and TAE donor while model identity and NpcParam remain the variant's own.
Model extraction also searches the texture donor and family texture binders.
Draw groups come from the selected row, with the existing weapon-draw TAE override.
The extractor's default animation selection is Wolf plus all placed donors; default model selection is Wolf plus all placed characters.

SpEffects whose `stateInfo` is 270 through 274 select CMSG offset type 15, choosing a000 through a400.
The selected group persists after the short TAE effect window, and AI sees the selected category choice instead of competing resident choices.
An Isshin fixture verifies a100 clip and TAE selection after the effect expires.
Genichiro's extracted c7100 clips are a000: his final castle phase is a different character, rather than an a100 swap.

## Deathblows and map events

The reviewed event rows in `arena.rs` are keyed by character and NpcParam row.
Event 11115810 joins after its opening cutscene label and starts the c7100 boss bar and immortality.
The first deathblow removes one mark, restores HP through the existing throw/TAE path, and Lua receives the remaining and maximum marks.
After the second mark, event 11115820 waits for Wolf alive, requests cutscene 11010020, clears its temporary effects and disables 1110800 while enabling 1110801.
The generic entity hand-over resolves that placed character from the registry and replaces the active opponent after the throw pair finishes.
Wolf's simulation, health and arena survive, and resetting restores the original c7100 fight.
The renderer reloads the enemy model, draw groups and animation library when the selected opponent changes.

The enabled placement is `c7110_0001`, castle NpcParam 71100000, think 71100000, at `[103.509, 53.989, 243.125]`, yaw -52 degrees.
Its data gives 2556 HP, 900 posture and one mark; the map changes the boss bar to 907110 and sets flag 11115802.
This is the castle Tomoe phase, distinct from final-game NpcParam 71101000.
Events 11115831 and 11115800 lift immortality at the finishing event message and handle final boss defeat.

## Verification and running

The placed roster smoke reports 102 characters and 1754 placements, with 102 loading and zero unsupported after donor extraction.
This proves resource loading, not complete combat fidelity for every character.
The idle-Wolf Lua test records 17 attacks and 19 contacts over 60 seconds, then 24 attacks after the first mark and seven in the c7110 phase.
The arrow test records Bullet row 71000500 and seven contacts through the shared bullet runtime.
Additional tests exercise both c7100 marks, the map hand-over, final victory and reset.

```powershell
$env:CARGO_TARGET_DIR='target/alt'
$env:CARGO_BUILD_JOBS='6'
cargo run --release -p sekiro-extract -j 6 -- anims c7100 c7110
cargo run --release -p sekiro-extract -j 6 -- models c7100:71001000 c7110:71100000
target/alt/release/sekiro-rs.exe --arena genichiro --simple-ai --god --brain-memory off --lock-on --script crates/game/scripts/genichiro_idle.txt --shots 300:target/alt/genichiro/idle-005.png,1800:target/alt/genichiro/idle-030.png --exit
target/alt/release/sekiro-rs.exe --arena genichiro --simple-ai --god --brain-memory off --passive --break-posture --spawn 107.829,54.044,237.04,0,0 --enemy-distance 1.6 --script crates/game/scripts/genichiro_deathblows.txt --shots 210:target/alt/genichiro/deathblow-1.png,650:target/alt/genichiro/deathblow-after-1.png,810:target/alt/genichiro/deathblow-2.png,1500:target/alt/genichiro/deathblow-tomoe.png --exit
```

The reviewed five-second idle image shows textured c7100 on the keep floor, his back bow, and the named two-mark boss bar.
The thirty-second image shows his textured armor and katana clearly, but the camera is too close after repeated attacks push Wolf against a wall.
The deathblow images show the throw pair and one remaining mark, then the separately loaded bare-chested c7110 model, bow, katana and named one-mark bar on the same roof.
Lighting remains dark under the roof, and wall camera framing needs further work.

### Offline brain verification (2026-10-11)

`genichiro_brain.rs` installs the scripted controller with persistence off, except for the isolated `%TEMP%` memory test.
The sixty-second idle god-mode run starts 52 brain-chosen attacks across 18 plans, with zero invalid plans.
The bow plan produces Bullet rows 71000500 and 71000530 through 71000533.
The move sheet includes all 24 actions and the three-hit sword and seven-hit Floating Passage timelines, with real TAE cancels and AtkParam-derived posture costs.
Other regressions cover unchanged Lua choices, plan-set pacing, posture checkpoint re-arming, character-specific boss/player memory and the continuous c7110 brain hand-over and numbered reset.

The forty-second game replay uses `--arena genichiro --brain scripted --brain-memory off --brain-overlay --god --lock-on`, with its input under `target/alt/genichiro/brain-idle.txt`.
Reviewed screenshots are `target/alt/genichiro/brain-005.png`, `brain-010.png`, `brain-020.png` and `brain-040.png`.
The panel is readable and shows the selected combo with its active step and Genichiro's move label; the five-second frame still shows his initial Lua bow follow-up before the scripted slot.
The twenty- and forty-second captures show the brain's sword chain, but repeated knockback traps Wolf near the roof edge and causes excessive camera proximity.
The boss armor is dark under the roof; camera framing, lighting and the existing FMOD event-83 diagnostic remain visual/audio gaps outside this brain change.
No model/API or real coach CLI call was used, and saves were backed up under `target/alt/genichiro/save-backup-20261011-brain` before the replay.

## Remaining gaps

Lightning uses the installed attacks and HKS states; the unresolved collision and status details are listed below.
Cutscene playback and story rewards remain unimplemented; the castle hand-over uses the timed fade placeholder and authored MSB warp described below.
Only one enemy is active at a time, with sequential event hand-over; simultaneous enemies and allies remain future work.
Castle c7100 supports the planning brain through `brain::actions::GENICHIRO`, including sword combos, perilous finishers, bow shots, jumping arrow volleys and existing tactics.
See `docs/BRAIN.md`, "Genichiro Ashina (c7100)", for the data-derived act table and exclusions.
The second-mark hand-over preserves the same brain session and coach, switches to the Lua-derived c7110 action table, and continues the same fight.
Tomoe includes the three lightning acts, sword and bow plans, tactics and phase-specific counter evidence; reset reinstalls the c7100 hook for the next numbered fight.
See `docs/BRAIN.md`, "Continuous Genichiro brain and counter learning", for phase storage, strict move-identity pooling and Lua counter constraints.
Lua still reports unimplemented `GetHpLastTarget` and `IsInsideTargetRegion` queries using the existing fallback.
Boss event discovery and all-boss arena generation remain incomplete; this session adds reviewed event and arena rows.

FwLo's MIT notes supplied the donor, animation-set and castle hand-over facts; the implementation here is original Rust.

## Hand-over cinematic placeholder (2026-10-11)

The EMEVD PlayCutsceneAndWarpPlayer instruction requests cutscene 11010020, method 16, player 10000 and point 1112813 in m11_01_00_00.
The point's MSB region base data gives Wolf position [115.906, 54.03, 234.886] and world yaw 126.6 degrees after the usual mirrored-X conversion.
The queued cinematic waits for any playing deathblow pair to release, fades to black over 0.5 s, applies the warp and event continuation while fully black, holds black for 0.75 s, then fades back over 0.5 s.
Enabling Tomoe uses his authored MSB part placement [103.509, 53.989, 243.125], yaw -52 degrees.
Combat, AI, status timers, animation clocks and gameplay input are paused for the full fade; input buffers are cleared at its boundaries.
Keyboard, mouse and gamepad buttons, including pause, are suppressed during the fade, and camera input is ignored.
The renderer requires two black presentation updates before synchronously loading the replacement opponent, then holds the fade until the new rig is ready.
That loading stall may lengthen wall-clock black time; the simulation's cinematic timing stays deterministic.
A cutscene without a resolved warp keeps both characters' positions, with a diagnostic if the requested point cannot be resolved.
This is a cinematic placeholder, not native cutscene playback or a recovered native camera sequence.
Owl event 11115870 has no cutscene instruction: its HKS heat-up, kneeling and escape continue playing normally rather than receiving this fade.

The hand-over replay uses `crates/game/scripts/genichiro_tomoe_fade.txt` with `--arena genichiro --god --simple-ai --brain-memory off --passive --break-posture --spawn 107.829,54.044,237.04,0,0 --enemy-distance 1.6`.
The inspected `target/alt/polish/tomoe-final-{before,fade-out,black,fade-in,after}.png` sequence shows the old boss before fading, complete black including the HUD, and Wolf facing Tomoe at the authored placements afterward.
The swap is hidden; the earlier deathblow camera can still approach the roof eaves before the cinematic starts.
The real-data regression holds movement/attack/jump/lock input throughout the pause, verifies no combat or animation advancement, and checks that a cinematic without a warp keeps both positions.

## Castle lightning (2026-10-11)

The castle Tomoe row is `c7110:71100000`, with behavior variation 71100.
Its `711000_battle.lua` Act13/17 requests bow 3039 without approach, Act18 requests 3043 at 8.3 m, and Act19 requests 3042 at 3.5 m.
Castle c7100 has those animation names but no corresponding Lua functions or lightning BehaviorParam rows, so its planning vocabulary remains unchanged.

| Act | TAE attack | Params |
|---|---|---|
| 3039 | Bullet judge 521 at 1.867 s, dummy 24 | BehaviorParam 271100521 -> Bullet 71100521 -> AtkParam_Npc 71100521 |
| 3042 | Melee judge 361 at 1.667 to 1.900 s | BehaviorParam 271100361 -> AtkParam_Npc 71100361 |
| 3043 | Melee judge 361 at 2.000 to 2.167 s | Same melee row; its warning bullet is separate |

The bow bolt has speed 80 m/s, life 2 s, initial radius 0.3 m and homing angle 0.
The slash is a melee attack, not a replacement bolt.
Both attacks have 16 thunder power and `spAttribute` 10 (blue/weak lightning), and neither disables guard or just guard for Wolf's katana.
Both leave SpEffect 9492 even on a guarded or deflected contact.
The bow's effect is on Bullet; the slash's effect is on AtkParam.
SpEffect 5390 and 5900/400240 accompany the charge animation; their native FXR is represented by blue sparks.

9492 lasts 30 seconds and has stateInfo 356 and behaviorRefId 511.
It cycles through hit effect 9412 to resistance-damage row 9432, which adds 200 in the legacy `registFreeze` field, specifies 30% max HP plus 160, and removes 15% max posture plus 120 remaining stamina.
The runtime holds the charge while airborne and discharges on ground contact.
9445 is the 1.5-second status indicator; 9450 supplies behavior reference 593 for Wolf's shipped shock reaction.
This is a single discharge, not periodic damage over time.
The full resistance threshold, scaling and native stateInfo 356 dispatcher remain unreconstructed: a full charge triggers directly in this implementation.

Wolf's HKS reads env 202 (damage type), 236 (level), 285 (element), 201/248 (landing), 1106/1108 (attack input), and 3036 (effect reference).
Airborne weak lightning selects `W_AirDamageElectroChargeWeakStart`, whose clip is `a000_102930`.
Attack input with reference 511 selects `W_AirWeakElectroReceiveAttack`, clip `a050_308910`, with landing continuation `a050_308911`.
Strong lightning uses reference 510 and `W_AirElectroReceiveAttack` instead.
No new HKS act id is required; existing act 159 is the damage absorption direction, not the reversal trigger.
Charged input acceptance is inferred: a held attack is made available to HKS while airborne, excluding an already-running reversal.
TAE 920 reads hop row 1612; TAE 922 reads velocity scale row 1615, whose active-window application is inferred.

The weak return's TAE starts judges 280/281 at 0.133 s: AtkParam_Pc 5000280 is the sword blade and 5000281 is the lightning capsule with radius 1.2 m and effect 9400.
9400 names resistance-damage row 9420: 200 electric build-up, 67% max HP plus 320, and 5% max posture plus 180 remaining stamina removed.
TAE effects 9505/9506 at 0.100 s lead to discharge/erase 9507/9508, 9500 and resistance reset 9440; the runtime clears the charge at that authored release.
The return is an AtkParam attack, rather than an invented Bullet row.
The current weapon rig lacks its endpoint dummy 10141, so a facing 10 m beam stands in for that capsule endpoint; radius, timing, effect and damage come from the data.
Native dummy placement and exact return reach remain a placeholder.

Stand-ins are flag-free jagged bright lines for the bolt (with a 0.15 s visual linger), blue charge sparks, blue material tint and small sparks while charged/electrified, and a short white beam/flash during reversal.
FwLo's MIT hand-off notes confirm lightning is a rank-3 reaction and describe Divine Dragon reversal availability; Genichiro's ids and mechanics above were read from this installation.

```powershell
target/alt/release/sekiro-rs.exe --arena genichiro --genichiro-phase tomoe --god --simple-ai --brain-memory off --passive --force-act 3039 --lock-on --spawn 107.829,54.044,241.44,0,0 --enemy-distance 6 --script crates/game/scripts/genichiro_lightning.txt --shots 100:target/alt/genichiro/lightning-charge.png,122:target/alt/genichiro/lightning-bolt.png,136:target/alt/genichiro/lightning-reversal.png --exit
```

The scripted castle replay catches the bow at step 117 and returns AtkParam 5000281 at step 130, taking Genichiro from 2556 to 524 HP and removing 225 posture.
The grounded guard replay records a blocked bolt with 272 HP and 120 posture damage from discharge, then Wolf's `GroundDamageElectricShockWeak` reaction and the Electrified HUD line.
Screenshots are `target/alt/genichiro/lightning-charge.png`, `lightning-bolt.png`, `lightning-reversal.png`, `lightning-after.png` and `lightning-ground.png`.
Visual inspection confirms the blue sparks/tint and white return beam are readable stand-ins; the incoming beam is thin and the castle scene is washed out, so this is functional FX verification rather than a finished visual match.
The selected bolt capture is step 122 of a replay with grading and auto exposure disabled; earlier step 115/118 captures transiently lacked visible actor meshes and were rejected.
The five real-data lightning regressions cover grounded and guarded bow hits, airborne capture, landing without returning, successful HKS reversal, and both lightning slashes.

## Aerial interruption (2026-10-11)

Act 3015's jump does not carry a DS3Poise/795 event or a special damage-motion replacement effect.
The NPC script's aerial branch requires flinch clips 8140/8141; these exist as aliases of 8010/8011.
The authored foot lift now makes env(201) false and a legitimate flinch falls through the aerial reaction graph instead of snapping to the floor.
No native source granting 3015 uninterrupted armour has been verified; the original attack is still interruptible in this implementation.
The data-protected aerial case is 3026, whose 5350 window lasts 0.067-2.500 s and preserves its later strike after a damaging hit.
See `COMBAT-RULES.md`, "Super armour and interruption", for the native category-1001 replacement table, perilous windows and the support-height approximation.

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
After the second mark, event 11115820 waits for Wolf alive, completes the cutscene boundary, clears its temporary effects and disables 1110800 while enabling 1110801.
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
Other regressions cover unchanged Lua choices, plan-set pacing, posture checkpoint re-arming, character-specific boss/player memory and the c7110 hand-over back to Lua.

The forty-second game replay uses `--arena genichiro --brain scripted --brain-memory off --brain-overlay --god --lock-on`, with its input under `target/alt/genichiro/brain-idle.txt`.
Reviewed screenshots are `target/alt/genichiro/brain-005.png`, `brain-010.png`, `brain-020.png` and `brain-040.png`.
The panel is readable and shows the selected combo with its active step and Genichiro's move label; the five-second frame still shows his initial Lua bow follow-up before the scripted slot.
The twenty- and forty-second captures show the brain's sword chain, but repeated knockback traps Wolf near the roof edge and causes excessive camera proximity.
The boss armor is dark under the roof; camera framing, lighting and the existing FMOD event-83 diagnostic remain visual/audio gaps outside this brain change.
No model/API or real coach CLI call was used, and saves were backed up under `target/alt/genichiro/save-backup-20261011-brain` before the replay.

## Remaining gaps

Lightning damage, airborne lightning capture and Lightning Reversal need dedicated systems.
Cutscenes are treated as completed boundaries; their playback, region warp and story rewards are not implemented.
Only one enemy is active at a time, with sequential event hand-over; simultaneous enemies and allies remain future work.
Castle c7100 supports the planning brain through `brain::actions::GENICHIRO`, including sword combos, perilous finishers, bow shots, jumping arrow volleys and existing tactics.
See `docs/BRAIN.md`, "Genichiro Ashina (c7100)", for the data-derived act table and exclusions.
The second-mark hand-over drops the c7100 driver and returns castle c7110 to its own Lua; lightning is unavailable to the planning vocabulary.
Lua still reports unimplemented `GetHpLastTarget` and `IsInsideTargetRegion` queries using the existing fallback.
Boss event discovery and all-boss arena generation remain incomplete; this session adds reviewed event and arena rows.

FwLo's MIT notes supplied the donor, animation-set and castle hand-over facts; the implementation here is original Rust.

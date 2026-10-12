# Group s fight pass: Demon of Hatred

This pass wires the Demon of Hatred into its arena (`--arena demon`, `--play --arena demon`) and checks each phase on the installed data.
Cache, `re/` and git state were not written.
FwLo's MIT notes under `target/ref-fwlo/` were used as a cross-check and are named where they were relied on.
Dated 2026-10-12.

## Identity and data

The boss is `c7020:70200000` at `m11_00_00_00/c7020_0000`, entity 1100900, boss bar 907021.
NpcParam gives 3195 HP, 1050 posture and `ninsatuNum` 3, so there are three health bars with two non-final marks and one final mark.
NpcThinkParam 70200000 selects logic and battle goal 702000 from `m11_00_00_00.luabnd.d`, which the shared Lua runtime runs unchanged.
The hurt capsule is 2.8 m radius and 4.5 m high; the body collision radius `chrHitRadius` is 1.4 m.
c7020 has only the a000 animation group (94 clips, TAE, skeleton, `c0000_c7020` paired clips for Wolf), so no a100 to a400 set exists and the anim-set SpEffects 200030/200031 select nothing new.
The model `cache/models/c7020.glb` is present.
No extract was missing for this boss.

## What was built

The fight runs on the shared machinery (clips, TAE, root motion, BehaviorParam attacks, bullets, deathblow pairs, map events, Lua AI) plus these additions.

- **SpEffect categories** (`crates/sim/src/sp_category.rs`).
  Clip 20001 adds SpEffect 3702002 (category 100, priority 4) and the Lua's `HasSpecialEffectId(3702001)` must turn false.
  An enemy's newly applied lasting effect now replaces active effects of the same `spCategory` and `categoryPriority`.
  Equal priority is the rule because the Demon's residents 200030 (3), 200004 (1), 221002 (20) and 3702001 (4) share category 100 and coexist; this is inferred, not traced in the executable.
  FwLo's notes record the same overwrite for the anim-set effects.
  The duel applies it to the Demon only; other bosses' regressions are tuned without it.
- **TAE effects with `effectEndurance` -1 stay on** until a map event clears them (`TaeRuntime::keep_endless`), for the Demon only.
  The retry opener 3056 adds SpEffect 5022 in a 0.24 s window and the Lua tests 5022 for the rest of the fight; before this the Demon repeated the opener forever.
  Other characters' clips add -1 state effects too (200000, 200004, 275060 and so on) and keep their window-bound behaviour until someone verifies them.
- **Defender HP cut rates** (`ActiveEffect::hp_cut_*`, `fight.rs`).
  SpEffect 300331 (HP damage multiplier 0) now makes the owner immune; 300321 and 300341 already cut posture.
- **Arrival gate** (`crates/sim/src/bosses/demon.rs`, `arena_b.rs`, `Arena::place`).
  The staged state is the retry encounter (flag 11100901).
  `Arena::place` removes the staged fight flag 11105901, so event 11105910 holds the Demon's animation disabled until Wolf and he are inside region 1102910 (about 53 m out), then sets 11105901, raises the three fog walls and shows bar 907021.
  Until then the duel freezes his animation and AI; Wolf's death resets the encounter-local flags 11105901, 11105902 and 11100900 so the wait repeats.
  A flat `--play --enemy c7020:70200000` duel has no region to enter, so it keeps the staged flag and fights at once (`off_the_arena_the_demon_fights_at_once`).
- **Kneeling is not interrupted**: after a non-final mark the AI does not act while the defender clip `a000_012000` (alias `a000_013200` from behind) plays; its TAE 300610 refills HP at 5.7 s.
- **Todome**: HKS would replace `Event20200` with the generic death clip on its first frame, so the script keeps seeing 1 HP while `Event20200` runs (`execution_owns_death`).
- **Throw pair on sloped ground**: the wall-clearance shift of a deathblow pair now re-grounds both bodies within a step height; before, the Demon hung over a lower floor and a fall start cancelled his kneel clip.
- **Body radius** (`Combatant::body_radius`): only the Demon separates from Wolf by `chrHitRadius` (1.4 m) instead of the 2.8 m hurt radius.
  His `GetDist` and `GetDist_Point` subtract that radius (`AiState::dist_offset`), which FwLo's static notes give for the engine; which radius field the engine reads is not traced.
  Other enemies keep their old behaviour because the Owl posture regression is tuned to it.

## Phase 1 (bar 3 of 3, SpEffect 3702001 table)

Implemented: the arrival gate and walls; the opener 3056 once, then the Lua's 3702001 table; every attack the AI can ask for lands on Wolf.
Checked by `the_demon_waits_frozen_until_wolf_enters_the_arena_then_opens_with_the_spin` (frozen idle, bar, three walls, one 3056, 5022 persists, reset refreezes), `phase_one_fights_wolf_with_the_map_ai` (90 s against an idle invulnerable Wolf: at least 8 hits and 8 distinct attacks, no unanswered Lua query, no script error) and `demon_arena_starts_at_night_waits_for_wolf_and_runs_mark_and_defeat_events`.
The slow `every_attack_the_demon_ai_requests_reaches_wolf` forces each of 41 acts at up to six ranges and requires a non-800 contact.
The slow `the_demons_projectiles_use_the_bullet_rows_of_his_attacks` requires the bullet rows 126, 122, 270, 350/364/440, 222, 321, 460, 450/452, 231/234, 320 and 300 to fly and reach Wolf, with warning rows 980 to 982.
`guarding_wolf_takes_posture_damage_instead_of_the_slash` checks a block or deflect of 3000.
`damage_alone_leaves_the_demon_at_one_hp_and_opens_the_deathblow` checks the immortality of event 11105910 (HP stops at 1 and the deathblow opens).
The perilous attacks are sweeps: AtkParam 180/181 (act 3025), 200/202 (act 3036) are unblockable and not just-guardable, with warning bullets 980 and 981; act 3008 also carries bullet 982.
`a_jump_passes_the_perilous_sweep_and_the_kanji_warns_first` finds a jump timing that takes no damage from 3025, and checks the warning precedes the blow.
The data has no thrust or grab for the Demon: no ThrowParam besides the deathblow rows, so there is no mikiri counter, and no sweep carries SpEffect 400200, so Wolf's jump kick has no posture bonus.
Missing or approximate:

- TAE 705 `FacingAngleCorrection` is not implemented for any enemy; 20 Demon events use it (acts 3001, 3008, 3013, 3015, 3019, 3025, 3026, 3034, 3038, 3046, 3048, 3052, 3092 and the 8010 to 8013 reactions), so the curvature of his attacks is approximate.
- Act 3026 is the far-range dash only the phase-3 interrupt requests (`702000_battle.lua`, Wolf more than 20 m away); its sweep passes 0.78 m short of Wolf at best, so it never lands here.
- Fire hazards are the shared bullet runtime: ground flame effects, glows, camera shakes and the rest of the FXR work belong to the effects pass.
- Event 11105923 and the first-visit introduction (priest soldiers 1105405, AI command 10 and 20, SpEffects 300320, 300330, 3702006, team 28, clip 20000, camera sub IDs 510 to 513 from events 11105970 to 11105972) are not run; the retry state bypasses them as documented in `ARENAS-b.md`.
- EMEVD 2004[50](entity, 220 or 221, flag) has no known meaning and is not executed; the Demon's behaviour does not need it here.
- The exact thresholds of `GetDist` are a FwLo-based choice, see above.
- The Lua's `SpaceCheck` asks the engine's navmesh (`GetExistMeshOnLineDistSpecifyAngleEx`), which the shared runtime answers with open floor everywhere, so the Demon steps and leaps where the original would see an edge; a six-minute soak with Wolf teleported beside him ends on lower terraces of the battlefield, never in the void (`soak` checks the height above the lowest collision).

## Phase 2 (after the first mark, 2 bars left)

Implemented: the front pair ThrowParam 17020000/17020001 (rear pair 17020110/17020111 from behind, `a000_013200` aliased to `a000_012000`); the kneel; flag 11105902 from event 11105912; HP back to 3195 through the clip's own SpEffect 300610; Act31 requests the 17 s roar `a000_020001`, whose TAE adds 3702002 (replacing 3702001), 3702050 and the 277020/277021 effects; the roar is immune to HP and posture damage until its 10.13 s effects end; the second table's attacks follow.
Checked by `the_first_mark_kneels_refills_then_the_roar_replaces_the_phase_effect`, `the_roar_makes_the_demon_immune_to_hp_and_posture_damage`, `a_mark_from_behind_uses_the_rear_pair_and_the_aliased_kneel` and the slow `phases_two_and_three_use_their_own_act_tables`.
Missing or approximate: the roar's camera (sub ID 513 via 3702072), its flames and sound; the heat-up SpEffects 277020 to 277023 change nothing in data (rows are no-ops, FwLo's `enemies.md` reaches the same conclusion), so no stat change is modelled; the Demon's posture after the get-up is whatever the shared posture rules give.

## Phase 3 (after the second mark, 1 bar left)

Implemented: the same pair and kneel; event 11105912 adds SpEffect 3702005 at one bar (FwLo's `boss_events.py` and `enemies.md` name the same effect), which the Lua reads to switch to its last table (fire waves 3036, 3043 and 3044 with bullets 350 to 366, 440, 450 and 452, the 3092 and 3093 moves).
Checked by the same phase test (acts, bullets, hits and no unanswered query in each phase).
Missing: the big fire shockwaves (bullet rows 233, 234, 451 and 452, radius up to 40 m) hit Wolf by the shared bullet rule; whether a jump clears them in the original is not verified here.

## Final mark and defeat

Implemented: ThrowParam 17020180/17020181 (FwLo's `deathblow_groups.md` lists the 180/181 finishing pair for c7020); the defender clip `a000_013700` raises message 10; with no bars left, 3702050 on the Demon and Wolf in a throw, event 11105921 requests `Event20200` for the Demon and `Event710207` for Wolf, which both play; defeat event 11105900 sets 9313 and 11100900, releases the three fog walls and re-enables the idol after three seconds; the Demon ends in the dead pose `a000_010000`.
Checked by `the_last_mark_plays_the_todome_and_defeats_the_demon` (rows, `a000_013700`, `a000_020200`, Wolf `a000_710207`, flag 9313, walls 0, final dead pose) and `demon_arena_starts_at_night_waits_for_wolf_and_runs_mark_and_defeat_events` (event flags with a stub host).
Missing: the one-frame idle between `Event20200` and the death clip (the script's own `IdleTransition`); the original's banner timing; the idol object's rendering is the arena stage's.
Event 11105922 (message 20 to command 21000) only fires if the death clip plays to its 16 s mark, which the Todome path cuts, so it is registered but unexercised.

## Commands

```powershell
$env:CARGO_TARGET_DIR='target/alt-s'
cargo test --release -p sekiro-sim --test it -j 3 -- demon
cargo test --release -p sekiro-sim --test it -j 3 -- --ignored demon
cargo test --release -p sekiro-sim --test it -j 3 -- owl genichiro brain_tactics attack_power posture_recovery perilous super_armour combat_rules player_guard projectile_guard deathblow guardian_ape gyoubu butterfly duel soldier isshin emma
cargo clippy --workspace --all-targets -j 3 -- -D warnings
cargo build --release -p sekiro-game -j 3
target/alt-s/release/sekiro-rs.exe --play --arena demon --brain scripted --brain-memory off
```

## Changed files

New: `crates/sim/src/bosses/demon.rs`, `crates/sim/src/sp_category.rs`, `crates/sim/tests/it/demon.rs`.
Small shared edits: `crates/sim/src/{lib,duel,arena,arena_b,tae,character,fight,hits}.rs`, `crates/sim/src/combat/{effects,params}.rs`, `crates/sim/src/lua_ai/runtime.rs`, `crates/sim/tests/it/{main,demon_arena}.rs`, `docs/ARENAS-b.md`, `STATUS.md`.

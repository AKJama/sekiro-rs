# Enemies and bosses: generalisation plan

Goal: any enemy or boss runs the way the Ashina soldier (c1010) and the Great Shinobi Owl (c5060) do now, from the game's own data, with no per-character code.
This is research only: no code was changed to write it.
Dated 2026-10-11.

Claim markers: **[V]** verified in our data or code (source named), **[F]** taken from FwLo's repo (not re-checked against the exe), **[I]** inferred.
FwLo's repo is `FFwLo/RustSekiro` (MIT), read through `gh api` only; `tools/sekiro-dearxan` was not opened.
Facts below are recorded in our own words; credit belongs in `CREDITS.md` when a value is used.

## 1. What is generic and what is hardcoded today

### 1.1 Already generic (reuse as is)

- **Scripts and graph.** `Character::load_npc` loads `c9997.hks` plus the character's own `<chr>.hks`, driven by `<chr>.behbnd.d/c9997.hkx` (`crates/sim/src/character.rs:271-281`) [V].
- **Every shipped HKS script loads.** `every_script_main_chunk_loads` in `crates/hks/tests/player_script.rs:183` covers all 81 files in `action/script` [V: test comment, not re-run here].
- **Params by row.** `Combatant::npc(cache, chr, npc_param, skeleton)` reads NpcParam; hits resolve `BehaviorParam 200000000 + behaviorVariationId*1000 + judge` (`crates/sim/src/hits.rs:7`, `:476`); bullets, SpEffects and ThrowParam by `defChrId` (`EnemySpec::throw_chr`, `enemy.rs:77`) are all row-driven [V].
- **Deathblow marks.** `Vitality::deathblows_left` is seeded from NpcParam `ninsatuNum` (`crates/sim/src/combat/hp.rs:117`, `params.rs:236`) [V].
- **Lua AI.** `AiBrain::load(cache, bundle, think, seed)` runs any `<id>_logic.lua` and `<id>_battle.lua` from a map bundle (`duel.rs`, `load_ai`) [V].
- **Map events.** `crates/sim/src/events.rs` is a general EMEVD interpreter (flags, boss bar, SpEffects, EzState commands) [V: `docs/OWL.md` "Phase 2"].
- **Arena crop.** `ArenaLayout::resolve` and `crop` take any MSB enemy part name (`crates/sim/src/arena.rs`) [V].
- **Draw masks.** The GLB carries `drawMask` and `combatDrawMask` extras applied by `apply_draw_masks` (`crates/game/src/draw_mask.rs`) [V].

### 1.2 Hardcodes to remove (file:line)

| # | Where | What is fixed to one character | Needed instead |
|---|---|---|---|
| 1 | `crates/sim/src/enemy.rs:28-48` | `EnemySpec::soldier()` and `owl()` hold chr, NpcParam row, think row, bundle and `starts_in_battle` as literals | Build a spec from data: placement + NpcParam + NpcThinkParam |
| 2 | `crates/sim/src/enemy.rs:55-66` | `by_name` accepts only `c1010`, `c5060`, `soldier`, `owl`; `c5070` is refused by name | Any `cXXXX[:row]` in the roster; keep the two aliases |
| 3 | `crates/sim/src/enemy.rs:70-75` | `display_name` has two names, else "Enemy" | NpcName text (FMG) or the roster file |
| 4 | `crates/sim/src/enemy.rs:33,46` | `ai_bundle` is a literal map bundle | The bundle of the map that places the enemy, with a search fallback |
| 5 | `crates/sim/src/character.rs:271-281`, `crates/sim/src/tae.rs:367`, `duel.rs:310` | `load_npc(chr)` reads `<chr>.hks`, `<chr>.behbnd.d`, `<chr>.anibnd.d` | Use the animation character (NpcParam `normalChangeAnimChrId`) for script, graph, TAE and clips; keep the model of `chr` |
| 6 | `crates/sim/src/arena.rs:75-109` | `OWL_EVENTS` (entity 1110850, flags 8301 and 9303, five event ids) and `boss_events("c5060")` | Derive the event set from the map's EMEVD, or a small tracked data file per boss |
| 7 | `crates/sim/src/arena.rs:111-127` | `ARENAS` has one row (`owl`: map, part `c5060_0000`, start `c0000_0003`, hidden model `m015072`) | Arena from the enemy's own MSB placement, with data overrides |
| 8 | `crates/sim/src/duel.rs:50,635` | `OWL_HEAT_UP = 3506080` makes deathblows unavailable | A data rule (step 6); keep until then, behind the registry |
| 9 | `crates/sim/src/duel.rs:297`, `crates/sim/src/bin/duel-sim.rs:35`, `crates/game/src/main.rs:126` | The default enemy is `EnemySpec::soldier()` | Fine as a default, but no code path other enemies cannot take |
| 10 | `crates/sim/src/brain/actions.rs:138,248,390`, `brain/mod.rs:266,348` | Boss-brain action tables exist for OWL and SOLDIER only; the fallback is `&OWL` | Out of scope: other enemies keep their Lua AI (`set_brain` already errors cleanly, `duel.rs:405`) |
| 11 | `crates/game/src/brain_stage.rs:26-114` | Datasets and stages are `c5060` only | Same: brain work, not enemy bring-up |
| 12 | `crates/extract/src/models/mod.rs:28` | `DEFAULT_IDS = c1010, c0000, c5060`; `cache/models` holds only those [V] | Export all roster chrs |
| 13 | `crates/extract/src/main.rs:66` | `anims` defaults to c0000, c1010, c5060; `cache/anim` holds only those [V] | Default to the roster |
| 14 | `crates/extract/src/models/mod.rs:120-122` | The draw mask comes from NpcParam row `<model number>*10000`, which is not a placed row for most chrs (c1010 places 10101300, 10100000 and others) [V] | Mask per placed row, chosen at spawn |
| 15 | `crates/extract/src/models/mod.rs:135-150` | The combat mask reads TAE of clip 1040 (`a000_001040`) for every chr | Per chr: find the draw-weapon clip, or none |
| 16 | `crates/extract/src/models/mod.rs:208-220` | Textures come only from `<chr>.texbnd` | Add `normalChangeTexChrId` and family packs (c1011 uses 1019, c1021 uses 1029 [V: NpcParam rows]) |
| 17 | `crates/game/src/arena.rs:33-250`, `deathblow_demo.rs:25,101-123` | The legacy flat `--arena` demo is soldier-only (`Soldier`, `c1010`, `SOLDIER_CHR`) | Leave or delete; `--play --enemy` is the real path |
| 18 | `crates/sim/src/duel.rs:108,114` and 68 `fighters[` uses in `crates/sim/src` | One `Character` enemy and `fighters: [Combatant; 2]` | The structural limit for multi-enemy encounters (step 9) |

### 1.3 Things that look hardcoded but are not

- `deathblow.rs:714-765` literal rows (11010000 and so on) are test fixtures.
- Soldier and Owl mentions in `perilous.rs` and `hits.rs` are doc comments; the logic reads the rows [V].
- `starts_in_battle` is only a flag on the spec (`enemy.rs:23`); its Owl value is documented as inferred (`enemy.rs:19-22`).

### 1.4 Gaps that block other enemies, found while auditing

- **Variants without their own animation.** 37 of 102 placed characters have no `behbnd` or `anibnd`; they borrow another character's through NpcParam `normalChangeAnimChrId` (c1011 and c1012 use c1010, c1021 uses c1020, c5430 uses c5400, and so on) [V: `cache/params/NpcParam.json`, `cache/raw/chr`].
  `load_npc("c1011")` would fail today (hardcode 5).
- **Anim sets a000 to a400.** FwLo found that SpEffects 200030-200034 (stateInfo 270-274) choose the `a000` to `a400` clip group, and that NPC TAE ids carry the group (`100003001` is `a100_003001`) [F].
  We have no handler (grep finds the id only in a test), and the Owl and soldier use `a000` only [V]; every phase-2 boss (Isshin, Genichiro, Monk, Demon) needs it [F].
- **Finishing pair.** Our notes call the 180/181 finishing pair on the last mark "inferred" (`docs/DEATHBLOW.md`); FwLo lists 180/181 pairs for c5000, c5060, c5100, c5400, c5430, c7020 and c7110 [F].
- **Behaviour breadth.** `GetEventRequest`, platoon, navigation and stealth are "not done yet" (`docs/AI.md:173-180`) and matter more for patrol enemies than for duels.

## 2. What FwLo did (https://github.com/FFwLo/RustSekiro)

### 2.1 Roster and per-enemy data

- A Rust extractor subcommand `npcs` writes `extracted/enemies/roster.json` (every chr the MSB places, with each placement's NpcParam and NpcThinkParam rows) and one `extracted/enemies/<chr>.json` per chr (character data, its own param rows, its roster entry) [F: `tools/extract.ps1`, `src/data.rs` `EnemyFile`].
- At run time `EnemyFile::foe(chr, npc_row)` picks the configured NpcParam row (else the most placed one), the think row from a placement that uses that row, and `animChr` from the roster entry [F: `src/data.rs`].
- `animChr` is NpcParam `normalChangeAnimChrId` and `texChr` is `normalChangeTexChrId`; model textures come from the chr, the texture chr and the family packs `c<first 3 digits>8` and `9` [F: `tools/extract.ps1`, `src/data.rs:685`].
  We confirm the animation chr rule on our data (section 3) [V].
- He reports 102 placed characters [F: `docs/reports/enemies.md`]; our own count from the ten layouts is also 102 [V].
- Config selects the enemy: `[enemy] chr`, `npc_row`, and a `group` list for several at once [F: `config.toml`, `HANDOFF.md` 4.10].

### 2.2 Per-enemy special cases he still has

- Boss characters have no `ThrowDefDeath` clips at all (c5000, c7100); a non-final mark plays ThrowDef only [F: `docs/kb/enemy.md`].
- Anim-set overwrite: a TAE-added anim-set SpEffect (spCategory 100) replaces the resident one [F].
- NPC bullets, resident SpEffects `spEffectID0-31` that open the 950/951 TAE gates, and grabs through `ThrowParam AtkChrId = chr` with `throwKind = 1000000 + throwTypeId*10`, which he calls a data pattern whose exe match is not traced [F].
- Team hostility (teams 6 vs 24, 29 indiscriminate) is read from NpcParam row names; the exe table is not traced [F].
- Hand-written scripted fights: the Great Serpent c5010 (region checks answered false), the Divine Dragon collapse loop (m25 52000000) and the Monk's phantoms (c5005) [F].

### 2.3 Multi-phase bosses and several deathblow marks

- With `ninsatuNum` above 1, a non-final deathblow plays the ThrowDef reaction only, then the boss gets up with full HP and empty posture; he lists the refill as untraced [F].
  Our Owl work found where the refill lives in data (the clip's TAE adds SpEffect 300610 "HP reset", `changeHpRate -100`) and applies it (`docs/DEATHBLOW.md:116-117`) [V], so we are ahead on this point.
- Boss battle scripts read `GetNinsatsuNum < GetNinsatsuMaxNum` as "a deathblow was taken" [F].
- Phases are map events: `tools/boss_events.py` reads every wait on `4[37] IF Number of Character Health Bars` on a boss and the actions after it into `boss_events.json` (16 rules, 12 NpcParam rows) [F].
  Actions are SetSpEffect, ClearSpEffect, ForceAnimationPlayback, EzState request, AI command (`2004[17]`), re-plan and enabling another character.
- `tools/boss_scripts.py` treats every character with a `2003[11] Display Boss Health Bar` as a boss and exports that fight's event instances and its "cast" (Tomoe, the Monk's phantoms, Owl's double) [F].
- Hand-over: when a boss dies or is disabled, the main enemy swaps to a living cast member with its own script (Butterfly 1000800 to 1000810, Genichiro to Tomoe) [F].
- The Todome (last deathblow) plays the 180/181 pair then `Event20200`; the map's 0-bar event gives Wolf `Event7102xx` (only 710205, 710206 and 710207 exist in `c0000.hkx`) [F].

### 2.4 Boss arenas

- `tools/boss_arenas.py` runs the map exporter once per boss row, centred on the boss's MSB entity id, radius 60 m, LOD 20 and 40, writing `map_boss_<row>.*` and `map_boss_arenas.json` (row to stem); a boss within 40 m of another's cast shares that arena [F: `docs/kb/map.md` "Every boss in its own arena"].
- Sizes are 60 to 130 MB per arena and about 400 MB for the castle top; he deletes each unpacked map afterwards (2 GB each) [F].
- Collision under the centre must draw something: the Headless Ape stands on a helper collision with draw group 0, so the nearest drawable floor is taken [F].
- Objects are drawn from `obj/*.objbnd` with the same draw-group rule; the Divine Dragon arena is almost all objects; the objects' own collision is not in his hit mesh [F].
- Cast positions are taken in the boss part's frame and placed relative to the boss (`src/enemy/events.rs`) [F].
- Our approach already differs: layouts, collision and pieces are extracted once for all ten maps and cropped per arena at run time (`sim::arena::crop`) [V]. No per-boss export is needed.

### 2.5 Testing method (worth copying)

- `all_enemies_smoke` (ignored test, env `SHINOBI_ENEMIES=` filter): per chr, loads the AI goal and anims, requires a first hit on Wolf and a deathblow pair [F].
- `all_enemies_full`: per enemy and per mark, fight until two hits land, posture-break, deathblow; the last must kill [F].
- Result on 2026-10-11: 94 of 110 pass (the 110 includes extra boss script rows) [F].

### 2.6 What he says is broken or not done

- Non-fighters: lookout gong c1120 and the shop clerks that share its AI (c7540, c7550, c7560, c7590, c7600), nuns c7440 and c7450, merchants c7420 and c7430, Hanbei c1012, handmaidens c1110 and c1111, c5021, Kuro c7200 [F].
- Scripted or special: c1260 folding-screen monkeys (arena regions), c5300 row 53000000, c5410 cutscene Isshin, c7300 (friendly), c1320 carp, c1340 (underwater only), c1460 kites, c1550 row 15501007 "hidden for directing" (loops stance 3040) [F].
- c1300 Aging status (belongs to the status system); the Great Serpent c5010 stays a set piece [F].
- Region heights for the Monk's phantoms, the exe's team table, map collision for bullets, homing and attached bullets [F].
- Caveat on "every enemy and boss": it means they load and pass a headless check, not that each fight matches the game; he lists 16 failing rows himself [F].

## 3. Roster from our extracted data

Method (read-only, from `cache/`): the ten `cache/maps/*/layout.json` give the placements (enemy parts with model, `npc_param`, `think_param`, `entity_id`); `cache/params/NpcParam.json` and `NpcThinkParam.json` give rows; `cache/raw/script/*.luabnd.d` list Lua files; `cache/raw/action/script/*.hks` list scripts; `cache/raw/chr` lists binders; `cache/raw/msg/engus/item.msgbnd.d/NPC*.fmg` gives names (decoded with a throwaway script, nothing committed).
Boss status: a character is a boss when an EMEVD `2003[11] DisplayBossHealthBar` instruction (arguments resolved through the event-start arguments) names one of its entity ids [V: scan of the 30 files in `cache/raw/event`].

### 3.1 Totals

- 102 placed characters (1754 placements) across the ten maps, excluding Wolf (c0000) and two objects [V].
- 65 have their own `behbnd`, `anibnd` and HKS script; 37 are model-only variants that name an animation character [V].
- Each of the 102 has at least one think row whose battle script exists in some map bundle [V].
  But 30 characters are placed in some map with a think row whose scripts are not in that map's own bundle (for example c5060 with row 50630000 in m11_01) [V].
  Which bundle supplies them in the game is unknown [I: another bundle, or unused rows].
- `cache/anim` and `cache/models` hold c0000, c1010 and c5060 only [V]; every other chr's binders are unpacked under `cache/raw/chr` but not decoded.
- Wolf has per-enemy deathblow animation binders `c0000_cXXXX.anibnd` for 36 characters; 29 own-animation placed characters (including c1010) have none and use the shared sets [V], which fits the soldier's `atkAnimOffset 200` [V: `docs/DEATHBLOW.md`].
- Map keys below: `m10` Hirata Estate, `m11` Ashina Outskirts (m11_00), `m11_01` Ashina Castle, `m11_02` Ashina Reservoir, `m13` Abandoned Dungeon, `m15` Mibu Village / Ashina Depths, `m17` Sunken Valley, `m20` Senpou Temple, `m25` Fountainhead.

### 3.2 Bosses (14 characters, 24 boss-bar entities)

Columns: maps, HP and deathblow marks (`ninsatuNum`) of the main row, boss-bar entity ids.
All have Lua AI and an own HKS script unless noted.

| chr | Boss (NpcName) | Maps | HP / marks | Entities | Notes |
|---|---|---|---|---|---|
| c5060 | Great Shinobi - Owl; Owl (Father); Inner Father | m10, m11_01 | 2552 / 2 | 1110850 (castle), 1000900, 1000990 | **Done** for the castle. The father rows use row 50601010 (m10). |
| c5090 | Lady Butterfly | m10 | 2073 / 1 | 1000800, 1000810 | The second part is its own entity (hand-over). |
| c5080 | Gyoubu Oniwa | m11 | 2552 / 2 | 1100800 | Mounted fight [I]. |
| c7020 | Demon of Hatred | m11 | 3195 / 3 | 1100900 | FwLo: last phase by SpEffect 3702005 [F]. |
| c7100 | Genichiro Ashina | m11_01, m11_02 | 1598 / 2 | 1110800, 1110890, 1120800 | Hands over to c7110 (Tomoe). Same castle top as the Owl. |
| c7110 | Genichiro, Way of Tomoe | m11_01, m11_02 | 2556 / 1 | 1110801, 1110891, 1120830 | Cast member of c7100. |
| c7400 | Emma, the Gentle Blade | m11, m11_01, m11_02 | 2235 / 1 | 1110900 | Row 74000010 is the boss. |
| c5430 | Isshin Ashina (old man) | m11, m11_01 | 2552 / 2 (row 54300000) | 1110920 | Model only; animations of c5400 [V]. |
| c5400 | Isshin, the Sword Saint | m11_02 | 2240 / 3 | 1120860, 1120890 | Anim sets by SpEffect [F]. |
| c5000 | Corrupted Monk | m15, m25 | 3513 / 1 (m15 row 50001000); 3 (m25 row 50000000) | 1500800, 2500850 | Phantoms are c5005 (model only, animations of c5000). |
| c5100 | Guardian Ape | m17 | 3520 / 1; row 51000100 has 2 | 1700800, 1700850 | Two rows. |
| c5200 | Divine Dragon | m25 | 3600 / 1 | 2500800 | Scripted collapse loop [F]. |
| c5300 | Old Dragons of the Tree | m25 | 320 / 0 | 2500810 | 26 placements, one bar. |
| c1000 | Folding Screen Monkeys (the bar sits on a c1000 dummy, row 12600050) | m20 | 9999 / 0 | 2000800 | The fight itself is c1260 [I]. |

Boss-like without a bar: c5010 Great Serpent (hp 9999, 1 mark, region driven), c5410 Isshin (cutscene), c5320 Okiryu large (hp 2560, 1 mark).

### 3.3 Minibosses

The NPC name table has a name `910000 + chr number` for each of these characters [V for the names].
No NpcParam row carries those name ids and the EMEVD scan found no bar for them, so how the bar gets its name is not known [I that they are minibosses].

| chr | Name | Placements | Maps | HP / marks (row max) | Own HKS | Own anim |
|---|---|---|---|---|---|---|
| c1020 | Ashina generals (911020 to 911026) | 10 | m11, m11_01, m11_02 | 1273 / 2 on rows 10200000, 10200010, 10202000; 0 on others | yes | yes |
| c1021 | General Naomori Kawarada | 2 | m11_02 | 1598 / 2 | no | via c1020 |
| c1040 | Long-arm Centipede | 2 | m17, m20 | 1274 / 2 | yes | yes |
| c1050 | Shinobi Hunting Monk | 8 | m10, m11_01 | 1276 / 2 | yes | yes |
| c1060 | Shinobi Hunter Enshin of Misen | 5 | m20 | 1114 / 0 | yes | yes |
| c1070 | Tokujiro, Juzou, Shigekichi | 4 | m10, m11, m15 | 1917 / 2 | yes | yes |
| c1080 | Shichimen Warrior | 3 | m13, m17, m25 | 1596 / 2 | yes | yes |
| c1130 | Armored Warrior | 1 | m20 | 1915 / 0 | yes | yes |
| c1191 | Snake Eyes Shirafuji | 6 | m15, m17, m25 | 1597 / 2 | no | via c1190 |
| c1300 | Mist Noble | 30 | m10, m15, m25 | 77 / 2 on the boss rows | yes | yes |
| c1310 | Okami Leader Shizu | 54 | m25 | 558 / 1 | yes | yes |
| c1350 | Headless | 4 | m11, m15, m17 | 1915 / 2 | yes | yes |
| c1370 | Blazing Bull | 1 | m11_01 | 2232 / 1 | yes | yes |
| c1400 | Ashina Elite (Jinsuke Saze, Ujinari Mizuo) | 33 | m10, m11_01, m15, m17, m20, m25 | 638 / 2 on elite rows | yes | yes |
| c1470 | Lone Shadow (four named) | 31 | m10, m11_01, m11_02, m15 | 953 / 2 | yes | yes |
| c5020 | Chained Ogre | 3 | m11, m11_01 | 2560 / 2 | yes | yes |
| c5021 | Chained Ogre (m25) | 1 | m25 | 2552 / 2 | no | via c5020 |
| c7000 | O'Rin of the Water | 1 | m15 | 1434 / 2 | yes | yes |

Marks are per row, not per chr: most placements of the same chr have 0 marks, so the registry must key on (chr, row) [V: the c1020 rows above].

### 3.4 Regular fighters (own HKS and anim unless noted; all have Lua AI)

Names are the NpcParam row names, machine translated; identities are inferred.

| chr | Row name (inferred identity) | Placements | Maps |
|---|---|---|---|
| c1010 | Ashina soldier family (sword, spear, matchlock rows; **done**) | 162 | m11, m11_01, m11_02, m13 |
| c1011 | Ashina soldier variant (tutorial) | 10 | m11_02 (animation of c1010) |
| c1030 | Centipede (close quarters) | 18 | m17, m20 |
| c1100 | Gecko-like small enemy, hp 17 | 75 | seven maps |
| c1140 | Earth-wall soldier | 4 | m17 |
| c1150 | Dog (hp 28) | 65 | m10, m11, m11_01, m11_02, m15, m20 |
| c1151 | Dog variant | 33 | m25 (animation of c1150) |
| c1180 | Mallet man (hp about 1913, 0 marks) | 19 | m10, m11, m11_01, m11_02, m20 |
| c1181 | Mallet zombie variant | 6 | m15 (animation of c1180) |
| c1200, c1210 | Sokushinbutsu monks | 4, 5 | m20 |
| c1211, c1220 | Illusory Hall Monk (hp 5, 194) | 62, 42 | m13, m15, m20 |
| c1240 | Shamo (chicken) | 20 | m10, m11, m15 |
| c1250 | Monkey (hp 74, bare hands) | 98 | m11_01, m15, m17, m20, m25 |
| c1260 | Folding Screen Monkey | 4 | m20 (animation of c1250) |
| c1320 | Carp | 80 | seven maps (FwLo: not a fighter [F]) |
| c1360, c1361 | Rappa-type soldiers (hp about 396) | 21, 17 | m11, m11_01, m11_02, m17, m20 |
| c1450, c1460 | Nightjar ninja, kite | 72, 2 | m10, m11_01, m15, m17, m20, m25 |
| c1500, c1501, c1510, c1520 | Villager zombies (hp 79; ambush and grab rows) | 117, 15, 8, 10 | m11_02, m13, m15, m17, m20, m25 (variants use c1500's animation) |
| c1550 | Bandit (hp 198) | 130 | m10, m15, m20 |
| c1700 | Castle samurai (hp 397) | 56 | m11, m11_01 |
| c5001, c5040, c5050, c5070, c5310 | Props and phantoms (hp 9999): bug monk, straw doll, carp, Owl's bird c5070, tentacle | 3, 1, 5, 2, 16 | m10, m17, m25 |
| c5005 | Corrupted Monk phantom | 3 | m25 (animation of c5000) |
| c7021 | Demon of Hatred dummy | 8 | m11 |

### 3.5 Non-combat, NPC and dummy characters

- Placeholder dummies: c1000 and c1001 (hp 9999, 45 and 88 placements).
- Hanbei c1012 and c1013; handmaidens c1110 and c1111 and the NPCs built on them (c7440, c7450, c7520, c7530).
- The lookout c1120 and the memorial-mob shop clerks (c7540, c7550, c7560, c7590, c7600).
- Merchants and conversation NPCs: c7010, c7200, c7210, c7300, c7401, c7410, c7420, c7430, c7460, c7470, c7480, c7490, c7500, c7510, c1321.
- Several NPCs reuse a fighter's animation (c7410 uses c1020's, c7510 uses c1360's) [V: `normalChangeAnimChrId`], so they may fight when provoked [I].
- Shipped but not placed: c1212, c5061, c5063, c5403, c7103 (HKS present; likely second-phase or "inner" forms) [V for existence, I for purpose].

## 4. Plan

Order is by dependency; each step ends runnable.
"Test" uses the headless `duel-sim` (`crates/sim/src/bin/duel-sim.rs`) and ignored cache-backed integration tests in `crates/sim/tests`, as the Owl work does.
Per `CLAUDE.local.md`, Codex can take steps 1, 3 and 4 once a spec is written; steps 2, 5 and 6 need research or judgement.

### Step 0: Load smoke over all 102 characters (a measuring tool, first)

- **Why:** we do not know which of the 65 HKS scripts, graphs, TAEs and AI rows break in our VM; FwLo's smoke test made his work tractable [F].
- **Files:** new `crates/sim/tests/enemy_roster.rs` (ignored, env `SEKIRO_ENEMIES=c1020,c5400`) using `Character::load_npc`, `TaeDb` and `AiBrain::load` only.
  First run `sekiro-extract anims` for all roster chrs; the owner of `cache/` does that, agents never write there.
- **Risk:** extraction time and `cache/anim` size; some binders may fail to decode (new HKX variants), and that is the finding.
- **Test:** one line per chr (script loads, graph loads, clip count, AI goal resolves) and a failure list that becomes the backlog.

### Step 1: Enemy registry from data

- **What:** `EnemyRoster` (new `crates/sim/src/roster.rs`) built from `cache/maps/*/layout.json`, NpcParam, NpcThinkParam and the NPC name FMG.
  It yields `EnemySpec { chr, anim_chr, tex_chr, npc_param, think, ai_bundle, starts_in_battle, display_name, marks, placements }` and replaces the literals at `enemy.rs:28-75`.
  `soldier` and `owl` stay as aliases.
- **Row choice:** the most placed row, or `cXXXX:ROW`; the think row from a placement that uses that row, as FwLo does [F]; when the row's think scripts are absent from the placing map's bundle, search all bundles and log it.
- **Bundle:** the bundle of the placing map (the soldier's `101000_*.lua` exists in four bundles [V]).
- **`starts_in_battle`:** true for boss-bar characters; for others derive it from the think row (`BattleStartDist`, `goalAction_ToFind`) and keep the soldier's current value [I]; the Owl value stays as it is (regression).
- **Also:** a small FMG reader in `crates/formats` (groups of id ranges and UTF-16 string offsets; layout checked on `NPC名.fmg` [V]), or write names into a roster JSON at extract time.
- **Risk:** placements reference NpcParam rows that do not exist (for example 11103000) and think rows that do not exist (10002000) [V]; skip them with a warning.
- **Test:** unit tests that skip without `cache/`: `by_name("c1020")` picks row 10202060 (most placed) and resolves bundle and think; `owl()` and `soldier()` equal their old literals (existing tests at `enemy.rs:95-98` keep passing).

### Step 2: Animation character (variants and phase sets)

- **What:** `Character::load_npc(cache, chr, anim_chr)`: script `c9997.hks` plus `<anim_chr>.hks`, graph `<anim_chr>.behbnd.d/c9997.hkx`, TAE and clips of `anim_chr`, skeleton and model of `chr` when it has its own, else `anim_chr`'s [I: variants ship a chrbnd with a model but no anibnd; confirm the skeleton matches].
- **Files:** `character.rs:271-281`, `tae.rs:367`, `duel.rs:310,484`, `hits.rs:467` (the flver used for dummies).
- **Then anim sets:** implement the SpEffect 200030-200034 (stateInfo 270-274) clip-group choice and TAE-id groups (`a100_003001`) [F].
  Isshin, the Monk, Genichiro and the Demon switch sets by phase, so this is the biggest unknown for phase-2 bosses; use c5400 as the test case.
- **Risk:** `AnimOffsets::default()` is used for every NPC today; NpcParam `animIdOffset` fields exist [V] and may matter.
- **Test:** `duel-sim --enemy c1011` runs the soldier's graph with c1011's model; a unit test asserts `anim_chr(c1021) == c1020`; an ignored test per variant chr loads.

### Step 3: Extraction for every roster character

- **What:** `anims` and `models` default to the roster (`main.rs:66`, `models/mod.rs:28`).
  The GLB keeps all `#NN#` mesh groups and stores no row-specific mask; the runtime applies `modelDispMask0..31` of the spawned row plus that chr's weapon-draw event.
  Include `normalChangeTexChrId` and family texture packs [F].
- **Files:** `crates/extract/src/models/mod.rs:120-220`, `crates/extract/src/anims.rs`, `crates/game/src/draw_mask.rs`, `crates/game/src/play.rs`.
- **Risk:** disk and time; unusual materials; the mask-per-row change touches the soldier and Owl look (regression: compare screenshots).
- **Test:** `cargo test -p sekiro-extract` fixtures like `soldier_combat_mask_does_not_require_extracted_animations`, generalised; a screenshot of three roster chrs in the viewer.

### Step 4: `--enemy cXXXX[:row]` for any id

- **What:** `duel-sim --enemy`, `sekiro-rs --play --enemy` and `--arena` accept registry names; errors list near matches; the HUD name comes from the registry (`play.rs:1138`).
- **Files:** `enemy.rs` (`by_name`), `crates/sim/src/bin/duel-sim.rs`, `crates/game/src/main.rs:112-127`, `crates/game/src/play.rs`.
- **Test:** Step 0's smoke extended into a scripted duel per enemy: `duel-sim --enemy cXXXX --steps 1800` must show the AI taking at least one act and a first hit on Wolf, or a documented "no attack" reason (the non-fighters of section 2.6). Wolf stays passive and immortal (`Duel::immortal`).

### Step 5: Arenas from the MSB

- **What:** derive an `ArenaSpec` for any placed boss or miniboss: map and part from the registry placement (the boss-bar entity picks the main placement when a chr has several), Wolf's start by a rule (nearest `c0000_*` start more than N metres away, else `FALLBACK_DISTANCE` in front), radius 60 m (also FwLo's value [F]).
  Keep a tracked override table for what the layout cannot say (`hidden_models` such as the castle's `m015072`, a preferred start).
- **Files:** `crates/sim/src/arena.rs:111-127`, `crates/game/src/play.rs` (map load by arena), `crates/game/src/main.rs:105-111`.
- **Events:** replace `OWL_EVENTS` with events found in the map script: collect the `2003[11]` bar events for the boss entity (as the scan in section 3 does) and the events that name that entity.
  Write them to a tracked JSON of ids only, generated by a new extractor command and reviewed per boss.
  Keep the hand list for the Owl as a regression.
- **Risk:** event-state props (tatami against burnt floor on the castle top, `arena.rs:121-125`) are per fight and cannot be derived without simulating flags; story-flag preconditions (`flags: [8301, 9303]`) must be recorded per boss [V for the Owl].
- **Test:** an ignored test per boss arena: layout resolves, Wolf's start has a collision floor (the `map_collision.rs` pattern), the boss part exists; extend `owl_arena.rs`; a screenshot per arena with `--shots`.

### Step 6: Several deathblow marks, phase rules and finishing pairs for any boss

- **What:** verify marks on rows with 2 or 3 (c5400, c5000, c7020, c7100) through the existing path (`hp.rs`, `Duel::deathblow_rows`).
  Then make "the interrupted deathblow gets him back up" generic: the refill is the TAE-added "HP reset" SpEffect in the defender clip [V for the Owl]; check that Isshin and Genichiro use the same mechanism before assuming it [I].
- **`OWL_HEAT_UP`:** replace `duel.rs:50,635` with a data rule once its generic form is known.
  This is a spike: find which SpEffect field marks "transition judgment" (`docs/FIDELITY.md` says the native check is traced for the Owl only).
- **Finishing pair:** select 180/181 on the last mark from ThrowParam rows whose `defChrId` is the enemy; FwLo lists seven characters that have them [F]. It stays **[I]** until traced.
- **Phase events:** feed `GetEventRequest` and the 4[37] bars-left waits through the event interpreter (done for the Owl, `docs/OWL.md` "Phase 2").
  Enable and disable hand-overs (Genichiro to Tomoe, the Monk's phantoms) belong to step 9.
- **Files:** `duel.rs`, `crates/sim/src/deathblow.rs`, `crates/sim/src/events.rs`, `crates/sim/src/combat/hp.rs`.
- **Test:** per boss, a scripted duel that posture-breaks and deathblows N times: marks go N, N-1, and so on; the boss is alive with full HP after each non-final blow and dead after the last, as FwLo's `all_enemies_full` checks [F]. `owl_phase2.rs` is the model.

### Step 7: Roster smoke and fight-through, then fix the backlog

- **What:** the two FwLo-style ignored tests: `smoke` (loads, acts, first hit) and `full` (every mark).
  Run all 102 and triage failures into data we misread, missing engine features (statuses, anim sets, grabs) and scripted set pieces.
  Update `docs/PROGRESS.md` counts.
- **Risk:** large; keep the backlog in `STATUS.md`.
- **Test:** the tests are the deliverable; aim at a count (FwLo reached 94 of 110) rather than "done".

### Step 8: Mechanics that unlock groups of enemies

In rough value order, each with a representative from section 5:

1. Enemy grabs on Wolf for non-boss rows (zombie ground grab, ThrowParam `AtkChrId = chr`): the Owl's grabs already work (`grab.rs`, `docs/OWL.md` "Grabs") [V]; check the throwKind rule on zombies [F: pattern only].
2. NPC bullets for ranged rows (matchlock, bows): `bullets.rs` exists [V].
3. Non-humanoid skeletons and movement (monkeys, dogs, centipedes, the Ape); the capsule from NpcParam `hitHeight` and `hitRadius` is in the data [V].
4. Status effects from enemies (burn, poison, terror, Aging): poison exists from the Owl's tools [V]; the rest belongs to the status work in `docs/PROGRESS.md`.
5. Team hostility (invaders against Ashina): needs a team table, not traced anywhere yet [F].

### Step 9: Multi-enemy encounters (later)

- **What:** replace `Duel { enemy: Character, fighters: [Combatant; 2] }` with a list of foes, each with its own Character, Combatant, `AiBrain`, throw state and brain slot; Wolf's targeting (lock-on, auto-aim candidates, deathblow target) over the list; per-foe HUD.
- **Files:** `crates/sim/src/duel.rs` (1659 lines, 68 `fighters[` uses across `crates/sim/src`), `fight.rs`, `hits.rs`, `crates/game/src/play.rs`, `crates/game/src/audio.rs`.
- **Order inside:**
  (a) two soldiers, no teams;
  (b) a boss with an add (Monk phantoms: c5005 with c5000's animation, parked by default);
  (c) hand-over (Genichiro to Tomoe, Butterfly's second entity) driven by the interpreter's enable and disable;
  (d) patrol groups and platoon AI, which is `docs/AI.md` "Not done yet".
- **Risk:** the biggest refactor in the plan, and it touches the boss-brain driver, which assumes one enemy. Do it after steps 0 to 7.
- **Test:** headless `duel-sim` with two `--enemy c1010`: both act, hits reach the right target, one death does not end the fight; then the Genichiro-to-Tomoe hand-over replayed from the map events.

## 5. Suggested first five after the soldier and the Owl

Chosen for being common (many placements), different from each other, and cheap in new assets once steps 1 to 4 exist.

1. **c1020 Ashina general** (10 placements across three castle maps, hp 1273, 2 marks on three rows).
   Same family as the soldier (same graph, sword combos) and it has Wolf's own deathblow binder `c0000_c1020`.
   It is the cheapest test of multiple marks on a non-boss, and was FwLo's first enemy [F].
   Test: two marks, posture break twice, killed on the second.
2. **c1500 villager zombie** (117 placements, five maps, hp 79, plus c1501, c1510 and c1520 that use its animation).
   The most common enemy after soldiers; tests the variant mechanism (step 2), unarmed grabs and the hidden ground ambush (ThrowParam 21500200 [F]), and low-HP posture rules.
3. **c1250 monkey** (98 placements, five maps, hp 74; c1260 as a variant).
   A non-humanoid skeleton and animal movement; tests step 3 and the capsule from NpcParam.
4. **c1180 mallet man** (19 placements, five maps, hp about 1913 with 0 marks, Wolf binder `c0000_c1180`).
   Heavy, large hit radius: tests damage reduction, deathblow on a high-HP enemy and attack hit areas bigger than a sword.
5. **c1400 Ashina elite** (33 placements, six maps, 2 marks on elite rows, quick sword).
   A miniboss with a different move set from the soldier; exercises per-row differences (rows 14000000 against 14005000 [V]), perilous warnings (`perilous.rs`) and Wolf's counters.

Why not a boss next: bosses need arenas, events and anim sets (steps 2, 5, 6), which the Owl has proven only for himself.
The first extra boss should be c5400 Isshin (3 marks, anim sets, its own arena on m11_02) or c7100 Genichiro (same castle top as the Owl, hand-over to Tomoe, which exercises step 9).
Suggested boss order: c5100 Guardian Ape (m17, two rows with 1 and 2 marks), c7100 then c7110, c5400, c5000, c7020.

## 6. Open questions

- How the engine picks the AI bundle for a placement whose think scripts are not in that map's bundle (30 characters) [I].
- How miniboss bars get their 91xxxx names (not on NpcParam; not in the EMEVD boss-bar instruction in the 30 event files) [I].
- Whether the "HP reset" SpEffect refill and the "transition judgment" marker are generic across bosses or Owl-specific.
- Whether model-only variants always share the skeleton of their animation character.
- The exe's team hostility table (FwLo does not have it either).

## 7. Sources

- Ours: `STATUS.md`, `docs/PROGRESS.md`, `docs/AI.md`, `docs/OWL.md`, `docs/DEATHBLOW.md`, `docs/FIDELITY.md`, `CLAUDE.local.md`; code paths are quoted with file:line above.
- Data (read only): `cache/maps/*/layout.json`, `cache/params/NpcParam.json`, `NpcThinkParam.json`, `cache/raw/chr`, `cache/raw/script`, `cache/raw/action/script`, `cache/raw/event/*.emevd`, `cache/raw/msg/engus/item.msgbnd.d`.
- FwLo (MIT): `docs/kb/enemy.md`, `docs/kb/map.md`, `docs/reports/enemies.md`, `docs/HANDOFF.md` sections 4.9 to 4.11, `tools/boss_arenas.py`, `tools/boss_scripts.py`, `tools/boss_events.py`, `tools/extract.ps1`, `src/data.rs`, `src/enemy/events.rs`, `src/map.rs`, `config.toml`.

# Combat fundamentals

Base sword combat only: no prosthetics, combat arts or special items.
Community observations are identified below; exact engine values come from our local executable/param research in [COMBAT-RULES.md](COMBAT-RULES.md).

## Vitality, posture and deathblows

Vitality is HP; an unguarded sword hit damages both HP and posture, while a blocked attack still builds posture damage. [Activision](https://support.activision.com/sekiro/articles/sekiro-shadows-die-twice-game-mechanics)
Posture measures the ability to maintain a fighting stance; filling its damage bar breaks that stance and briefly exposes an enemy to a deathblow. [Activision](https://support.activision.com/sekiro/articles/sekiro-shadows-die-twice-game-mechanics)
A deathblow is a finishing attack at the red body marker, and the red marks above an enemy's HP indicate how many deathblows are needed. [Activision](https://support.activision.com/sekiro/articles/sekiro-shadows-die-twice-game-mechanics)
A one-life enemy dies from that finisher; a multi-life boss survives until the last mark is spent, so breaking posture is an opening rather than an automatic kill. [Activision](https://support.activision.com/sekiro/articles/sekiro-shadows-die-twice-game-mechanics)
Wolf's own posture break leaves him vulnerable rather than offering him an enemy-style deathblow; a safety roll gets him back up. [Activision](https://support.activision.com/sekiro/articles/sekiro-shadows-die-twice-game-mechanics)

## Recovery

Lower HP slows posture recovery, making sustained pressure easier to preserve. [Activision](https://support.activision.com/sekiro/articles/sekiro-shadows-die-twice-game-mechanics)
Holding guard between incoming blows speeds recovery; guarding does not categorically stop it. [Combat wiki](https://sekiro-shadows-die-twice.fandom.com/wiki/Category:Combat#Posture_and_Vitality)
For Wolf, community guidance reports guard recovery at twice idle/running speed. [Community guide](https://ezzygamestats.com/games/digital-games/2019-games/sekiro-shadows-die-twice/guides/combat-tips-and-tricks.html)
Attack and reaction animations can suppress recovery; there is no universal rule that every guarding or attacking frame has the same recovery rate. [Executable research](COMBAT-RULES.md#posture-recovery)
Our extracted soldier data gives recovery multipliers 0.6, 0.5 and 1/3 below 80%, 60% and 40% HP; they compound to 0.6, 0.3 and 0.1 of base speed, rather than universal 75/50/25% breakpoints. [Executable and param research](COMBAT-RULES.md#posture-recovery)

## Guard and deflect, in either direction

Holding guard blocks ordinary physical sword damage but costs the guarder posture; timing guard just before contact deflects and damages the attacker's posture. [Combat wiki](https://sekiro-shadows-die-twice.fandom.com/wiki/Category:Combat#Deflection), [ordinary block discussion](https://www.reddit.com/r/Sekiro/comments/1fdxoih)
A deflector still takes some posture damage, usually less than on a block; these rules also apply when an enemy blocks or deflects Wolf. [Executable research](COMBAT-RULES.md#posture-damage-to-the-defender)
Ordinary physical blocks have no HP chip in the baseline mode; elemental/status attacks can behave differently, and the optional charmless difficulty introduces block chip while preserving clean deflects. [Community chip discussion](https://www.reddit.com/r/Sekiro/comments/shna5x/damage_from_deflection/)
A successful deflect cannot break the deflector's posture even at a full damage bar; a subsequent block or direct hit can. [Community guide](https://ezzygamestats.com/games/digital-games/2019-games/sekiro-shadows-die-twice/guides/combat-tips-and-tricks.html), [executable research](COMBAT-RULES.md#breaks-deathblows-and-hp-protection)
An overloaded guard breaks and leaves a reaction opening. [Executable research](COMBAT-RULES.md#breaks-deathblows-and-hp-protection)
Our animation research gives Wolf's repeated guard-press windows as 0.200, 0.100, 0.067 and 0 seconds; mashing guard progressively removes the deflect window. [Local data comparison](COMBAT-RULES.md#third-party-claims)

## Enemy pressure and hit reactions

Sword-fighting enemies commonly block a short attack string, then deflect and counter; continuing to mash attack after the deflect risks being interrupted by that counter. [Community hit-reaction analysis](https://www.reddit.com/r/Sekiro/comments/ycdp3m)
There is no universal N: the same analysis reports castle blue-robed samurai blocking the first hit and deflecting the second, while a comment describes two blocks before retaliation as common. [Community hit-reaction analysis](https://www.reddit.com/r/Sekiro/comments/ycdp3m)
Treat that count as an enemy-specific observation, not an engine constant.
An unguarded hit often staggers a swordfighter and permits another swing; poise or hyper-armour means a particular enemy/action continues through the hit instead. [Community hit-reaction analysis](https://www.reddit.com/r/Sekiro/comments/ycdp3m)
Ordinary exposed wind-ups can be interrupted, but protected attack sequences can trade through Wolf's slash, and a deflect does not guarantee the enemy's combo has ended. [Community hit-reaction analysis](https://www.reddit.com/r/Sekiro/comments/ycdp3m), [Combat wiki](https://sekiro-shadows-die-twice.fandom.com/wiki/Category:Combat#Consecutive_Deflections)

## Perilous attacks

A sweep cannot be blocked or deflected: jump it, then press jump again over the enemy for a posture-damaging stomp. [Combat wiki](https://sekiro-shadows-die-twice.fandom.com/wiki/Category:Combat#Sweep_Attacks)
A thrust cannot be ordinarily blocked but can be deflected; the unlockable Mikiri counter steps into it and stomps the weapon for greater posture damage. [Combat wiki](https://sekiro-shadows-die-twice.fandom.com/wiki/Category:Combat#Thrust_Attacks)
Avoid ordinary grabs with a timed dodge or by moving out of reach; guard is not the answer. [Combat wiki](https://sekiro-shadows-die-twice.fandom.com/wiki/Category:Combat#Grab_Attacks)
Read the animation: the same red warning covers different attack kinds. [Community counter guide](https://www.reddit.com/r/Sekiro/comments/vmax38)
In our soldier/Owl data, sweep stomps deal 10 x 8 = 80 posture and Mikiri deals 90, with a 0.400 s forward-step window; these are attack/data-specific numbers. [Counter research](COMBAT-RULES.md#perilous-attacks)

## Our engine vs these rules

Paths below are relative to this document.
Matches mean implemented and covered, not a claim of complete frame-exact fidelity.

| Rule | State and evidence |
|---|---|
| HP and posture damage, posture breaks | Matches: [fight.rs](../crates/sim/src/fight.rs), [posture.rs](../crates/sim/src/combat/posture.rs); `posture_floor_and_debt`, `repeated_deflects_break_the_soldier` in [combat tests](../crates/sim/src/combat/tests.rs); progression scaling remains approximate; |
| Deathblows and multiple marks | Matches: [duel.rs](../crates/sim/src/duel.rs), [hp.rs](../crates/sim/src/combat/hp.rs); `deflect_chain_breaks_the_soldier_then_deathblow` in [duel tests](../crates/sim/tests/duel.rs), `owl_takes_two_deathblows` in [Owl tests](../crates/sim/tests/owl.rs); |
| HP-dependent and animation-dependent recovery, including guard | Matches the extracted formulas: [recovery.rs](../crates/sim/src/combat/recovery.rs), `CombatRules::recover_with_state`; `soldier_recovery_by_hp` in combat tests, `posture_recovery_is_scaled_by_tae_225` in [combat integration tests](../crates/sim/tests/combat_rules.rs) and `guarding_owl_under_pressure`, `guarding_soldier_under_pressure_breaks`, `recovery_disabled_flag_skips_the_update` in [posture recovery tests](../crates/sim/tests/posture_recovery.rs); The action-state skip flag is `act(110)` and is implemented, but no shipped script calls it; |
| Guard/deflect in both directions, chip, guard break | Matches ordinary baseline sword outcomes: `CombatRules::resolve`; `guard_outcomes_against_the_soldier_slash`, `deflect_never_breaks_the_defender` in combat tests and `guards_and_deflects_work_both_ways` in Owl tests; Charmless chip state is unidentified/unimplemented; |
| Enemy blocking, deflecting and countering under attack pressure | Partial: real HKS reactions, TAE effects and Lua reflex acts run through [character.rs](../crates/sim/src/character.rs) and [lua_ai](../crates/sim/src/lua_ai); `his_reflexes_still_interrupt_a_tactic_plan` in [brain tactics tests](../crates/sim/tests/brain_tactics.rs); `soldier_guards_mashed_slashes` in [combo tests](../crates/sim/tests/combo_on_hit.rs) reproduces hit, block, deflect and a counter request under attack spam; No universal block count is imposed; full per-enemy sequences still lack live-game parity; |
| Sweeps and stomp, thrust deflect and Mikiri | Matches basic counters: [perilous.rs](../crates/sim/src/perilous.rs), [duel.rs](../crates/sim/src/duel.rs); `forward_step_counters_the_thrust`, `jump_kick_on_the_soldiers_sweep_breaks_his_posture`, `jump_kick_on_the_owls_sweep_takes_eight_times_the_posture` in [perilous tests](../crates/sim/tests/perilous.rs); learned-skill posture bonuses and automatic follow-up deathblow remain missing; |
| Grab/dodge and ordinary evasions | Partial: Owl throws run from ThrowParam, sweep/grab typed i-frames work; [hits.rs](../crates/sim/src/hits.rs) explicitly lacks general and thrust typed i-frames; [Owl grab tests](../crates/sim/tests/owl_grab.rs) cover paired throws, not full evasion fidelity; |
| Stagger, poise and interruptible actions | Partial: [damage_type.rs](../crates/sim/src/combat/damage_type.rs) supplies reactions to real HKS; attack/recoil cancels are data-driven; No independently verified complete poise/hyper-armour model or per-action interruption parity suite exists; |

### Mismatches ranked by fight impact

1. **General/thrust dodge i-frames missing**: evasions that should avoid a blow may instead take it, changing survivability and counter openings.
2. **Poise/interruption and exact guard-counter sequences not fully verified**: these determine who owns the next swing; this is a fidelity gap, not evidence that every such action is wrong.
3. **Player attack-power/progression scaling approximate**: fixed growth level and absent attack-power stat change HP damage, fight length and recovery pressure ([fight.rs](../crates/sim/src/fight.rs)).
4. **Recovery action-state skip flag: resolved, and not the cause of a fast-recovering guard.** The flag (bit 21 of the action state's +0x88) is set only by HKS `act(110)` (`SetStaminaRecoveryDisabled`), which no shipped script calls, so it never suppresses recovery in the vanilla game; the sim honours it for completeness ([COMBAT-RULES.md](COMBAT-RULES.md#posture-recovery)). A guarding Great Shinobi Owl really recovers at his full 60 per second from the data (his guard clip has no TAE 225 or recovery type), so a Wolf slash every 0.6 s, which costs him 14 to 15 of 660, never builds his posture in our sim. Whether the live game builds it faster is unverified; the remaining suspects are on the damage side (guarded posture damage, progression scaling in item 3) rather than the recovery rule.
5. **Learned Mikiri bonuses and automatic break-to-deathblow follow-up missing**: counter success works, but upgraded posture payoff and continuation differ.
6. **Charmless block-chip mode missing**: ordinary baseline matches; this matters when supporting the optional harder mode.

These are findings only; this session does not change the combat simulation rules.

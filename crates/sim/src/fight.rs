//! HP, posture and hit resolution for a fight, on the engine rules in [`crate::combat`].
//!
//! [`CombatRules`] keeps one [`Fighter`] per character (HP, the deathblow counter and the posture
//! meter) and turns each [`HitEvent`] into numbers and the reaction codes the two scripts read:
//!
//! 1. Guard test: the defender's TAE `ShieldBlock` flag names its guard judge, whose AtkParam row
//!    is the guard row; the arc comes from the weapon (Wolf) or NpcParam `guardAngle` (NPCs); the
//!    repel defence from the guard row, the weapon and the active SpEffects (`defFlickPower` 40
//!    of the just-guard window 105010 makes a deflect). See [`crate::combat::hit`].
//! 2. Posture damage to the defender and back to the attacker, with the defender's deflect
//!    effects 105020..105023 halving what a chain of deflects sends back.
//! 3. HP damage through the element sum and the guard cuts.
//! 4. The damage codes (99999, 3, 1001, 1027, 1028, 2 for the defender; 1000, 1008, 1017,
//!    1019, 1033 for the attacker).
//!
//! Every frame [`CombatRules::recover`] regains posture at the character's speed, scaled by the
//! StaminaControlParam type of its current animation (TAE 960) and its HP-gated SpEffects.
//!
//! Inferred here, not read from the executable (see `docs/COMBAT-RULES.md` for what is):
//! - The player's physical attack power is EquipParamWeapon `attackBasePhysics` times
//!   ReinforceParamWeapon `physicsAtkRate` times AtkParam_Pc `atkPhysCorrection / 100` (the
//!   Dark Souls convention; AtkParam_Pc `atkPhys` is 0). The Kusabimaru's slash is 40 * 300% =
//!   120. The attack-power stat, which scales this in the game, is not modelled.
//! - Wolf's maximum HP, posture and posture recovery are CalcCorrectGraph 500, 501 and 504 at
//!   [`PLAYER_GROWTH_LEVEL`]; which progression value the engine feeds them is open.
//! - The NPC's per-attribute `<attr>DamageCutRate` on unguarded hits is not applied (all 1.0 on
//!   the soldier except dark).

use crate::character::Character;
use crate::combat::damage_type::{DefenderFacts, attacker_reaction, defender_reaction};
use crate::combat::effects::{ActiveEffect, attacker_stamina_attack_rate};
use crate::combat::hit::{
    GuardInput, HitOutcome, npc_guard_repel, player_attack_repel, player_guard_repel,
    resolve_hit_with_repel,
};
use crate::combat::hp::{
    Vitality, attribute_guard_factor, element_damage, final_hp_damage, keep_hp, npc_guard_cut,
    player_guard_cut,
};
use crate::combat::params::{
    AttackProfile, CalcCorrectGraph, NpcCombat, StaminaControl, WeaponGuard,
};
use crate::combat::posture::{
    ControlRange, DefenderPosture, PostureMeter, apply_defender_posture, attacker_breaks,
    attacker_posture_damage, defender_posture_damage, defender_posture_damage_from_base,
    player_posture_base,
};
use crate::combat::recovery::{
    RecoveryAccumulator, RecoveryInputs, player_base_speed, posture_recovery_per_second,
};
use crate::hits::{AttackData, Combatant, HitEvent, HitResult, load_table};
use crate::player::IncomingDamage;
use sekiro_formats::param::Table;
use std::collections::HashMap;
use std::path::Path;

/// The progression value fed to CalcCorrectGraph 500/501/504 for Wolf (inferred: 1 is the
/// graphs' first breakpoint, giving 320 HP, 120 posture and 30 posture per second).
pub const PLAYER_GROWTH_LEVEL: f32 = 1.0;

/// CalcCorrectGraph rows: maximum HP, maximum posture, posture recovery speed.
const GRAPH_MAX_HP: i32 = 500;
const GRAPH_MAX_POSTURE: i32 = 501;
const GRAPH_RECOVERY: i32 = 504;

/// ReinforceParamWeapon multipliers of the equipped weapon level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reinforce {
    /// physics, magic, fire, thunder, dark attack rates.
    pub attack: [f32; 5],
    pub stamina_atk: f32,
    pub stamina_guard_def: f32,
    pub physics_guard_cut: f32,
    pub passive_stamina_atk: f32,
}

impl Default for Reinforce {
    fn default() -> Self {
        Self {
            attack: [1.0; 5],
            stamina_atk: 1.0,
            stamina_guard_def: 1.0,
            physics_guard_cut: 1.0,
            passive_stamina_atk: 1.0,
        }
    }
}

/// What kind of character a fighter is, with the params its side of the rules reads.
#[derive(Debug, Clone)]
pub enum FighterKind {
    Player {
        weapon: Box<WeaponGuard>,
        /// EquipParamWeapon `attackBase{Physics,Magic,Fire,Thunder,Dark}`.
        attack_base: [f32; 5],
        reinforce: Reinforce,
    },
    Npc(Box<NpcCombat>),
}

/// One character's HP and posture.
#[derive(Debug, Clone)]
pub struct Fighter {
    pub vitality: Vitality,
    pub posture: PostureMeter,
    /// Posture recovery speed of the last frame, points per second.
    pub recovery_rate: f32,
    pub kind: FighterKind,
    recovery: RecoveryAccumulator,
    recovery_base: f32,
    control: Option<StaminaControl>,
    /// Always-on SpEffects (NpcParam `spEffectID0..`), still subject to their HP gates.
    resident: Vec<i32>,
    start: (Vitality, PostureMeter),
}

impl Fighter {
    /// Back to full HP and posture.
    pub fn reset(&mut self) {
        (self.vitality, self.posture) = self.start;
        self.recovery = RecoveryAccumulator::default();
    }

    /// HP left as a fraction of max.
    pub fn hp_fraction(&self) -> f32 {
        self.vitality.hp as f32 / self.vitality.max_hp.max(1) as f32
    }

    /// Posture *damage* as a fraction of max: 0 is a calm meter, 1 is broken. This is what the
    /// game's bar shows, growing from the centre outward.
    pub fn posture_fraction(&self) -> f32 {
        ((self.posture.max - self.posture.remaining) as f32 / self.posture.max.max(1) as f32)
            .clamp(0.0, 1.0)
    }

    /// Always-on SpEffect ids (NpcParam `spEffectID0..`).
    pub fn resident(&self) -> &[i32] {
        &self.resident
    }

    pub fn dead(&self) -> bool {
        self.vitality.hp <= 0
    }

    fn control_range(&self, stamina_type: Option<i32>) -> Option<ControlRange> {
        let kind = usize::try_from(stamina_type?).ok()?;
        ControlRange::new(self.control.as_ref()?, kind, self.posture.max)
    }
}

/// HP, posture and the hit rules for the characters of one fight.
pub struct CombatRules {
    pub fighters: Vec<Fighter>,
    effects: HashMap<i32, ActiveEffect>,
}

fn param_dirs(cache: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
    (
        cache.join("raw/param/gameparam/gameparam.parambnd.d"),
        cache.join("refs/paramdex/Defs"),
    )
}

fn row_f32(t: Option<&Table>, id: i32, field: &str) -> Option<f32> {
    let row = t?.find(id)?;
    row.f32(field)
        .ok()
        .or_else(|| row.int(field).ok().map(|v| v as f32))
}

impl CombatRules {
    /// Builds a fighter for each combatant from the params in `cache`. Index 0 is the player
    /// (weapon `param_id`), the rest NPCs (NpcParam `param_id`).
    pub fn load(cache: &Path, combatants: &[&Combatant]) -> Self {
        let (pd, dd) = param_dirs(cache);
        let sp = load_table(&pd, &dd, "SpEffectParam", "SpEffect");
        let npcs = load_table(&pd, &dd, "NpcParam", "NpcParam");
        let weapons = load_table(&pd, &dd, "EquipParamWeapon", "EquipParamWeapon");
        let reinforce = load_table(&pd, &dd, "ReinforceParamWeapon", "ReinforceParamWeapon");
        let control = load_table(&pd, &dd, "StaminaControlParam", "StaminaControlParam");
        let graphs = load_table(&pd, &dd, "CalcCorrectGraph", "CalcCorrectGraph");
        let tentative = load_table(&pd, &dd, "TentativePlayerParam", "TentativePlayerParam");

        let effects = sp
            .as_ref()
            .map(|t| {
                t.rows()
                    .filter_map(|r| ActiveEffect::from_row(&r).ok().map(|e| (r.id(), e)))
                    .collect()
            })
            .unwrap_or_default();
        let control_row = |id: i32| {
            control
                .as_ref()?
                .find(id)
                .and_then(|r| StaminaControl::from_row(&r).ok())
        };
        let graph = |id: i32| {
            graphs
                .as_ref()?
                .find(id)
                .and_then(|r| CalcCorrectGraph::from_row(&r).ok())
        };

        let fighters = combatants
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let fighter = |vit: Vitality,
                               posture: PostureMeter,
                               recovery_base,
                               control,
                               resident,
                               kind| Fighter {
                    vitality: vit,
                    posture,
                    recovery_rate: 0.0,
                    kind,
                    recovery: RecoveryAccumulator::default(),
                    recovery_base,
                    control,
                    resident,
                    start: (vit, posture),
                };
                if i == 0 {
                    let w = weapons.as_ref().and_then(|t| t.find(c.param_id));
                    let weapon = w
                        .as_ref()
                        .and_then(|r| WeaponGuard::from_row(r).ok())
                        .expect("EquipParamWeapon row of the player's weapon");
                    let base = |f: &str| w.as_ref().and_then(|r| r.int(f).ok()).unwrap_or(0) as f32;
                    let attack_base = [
                        base("attackBasePhysics"),
                        base("attackBaseMagic"),
                        base("attackBaseFire"),
                        base("attackBaseThunder"),
                        base("attackBaseDark"),
                    ];
                    let rid = base("reinforceTypeId") as i32;
                    let rf = |f: &str| row_f32(reinforce.as_ref(), rid, f).unwrap_or(1.0);
                    let reinforce = Reinforce {
                        attack: [
                            rf("physicsAtkRate"),
                            rf("magicAtkRate"),
                            rf("fireAtkRate"),
                            rf("thunderAtkRate"),
                            rf("darkAtkRate"),
                        ],
                        stamina_atk: rf("staminaAtkRate"),
                        stamina_guard_def: rf("staminaGuardDefRate"),
                        physics_guard_cut: rf("physicsGuardCutRate"),
                        passive_stamina_atk: rf("passiveStaminaAtkRate"),
                    };
                    let level = |id: i32, fallback: f32| {
                        graph(id).map_or(fallback, |g| g.eval(PLAYER_GROWTH_LEVEL).trunc())
                    };
                    let hp = level(GRAPH_MAX_HP, 320.0) as i32;
                    let posture = level(GRAPH_MAX_POSTURE, 120.0) as i32;
                    let recovery = graph(GRAPH_RECOVERY)
                        .map_or(30.0, |g| player_base_speed(&g, PLAYER_GROWTH_LEVEL));
                    let debt = row_f32(tentative.as_ref(), 0, "DebtSp").unwrap_or(0.0) as i32;
                    fighter(
                        Vitality::new(hp, 0),
                        PostureMeter::full(posture, debt),
                        recovery,
                        control_row(0),
                        Vec::new(),
                        FighterKind::Player {
                            weapon: Box::new(weapon),
                            attack_base,
                            reinforce,
                        },
                    )
                } else {
                    let npc = npcs
                        .as_ref()
                        .and_then(|t| t.find(c.param_id))
                        .and_then(|r| NpcCombat::from_row(&r).ok())
                        .expect("NpcParam row");
                    fighter(
                        Vitality::new(npc.hp, npc.deathblows),
                        PostureMeter::full(npc.max_posture, npc.max_debt_stamina),
                        npc.stamina_recover_base_vel,
                        control_row(npc.stamina_control_param_id as i32),
                        npc.sp_effect_ids
                            .iter()
                            .copied()
                            .filter(|&id| id > 0)
                            .collect(),
                        FighterKind::Npc(Box::new(npc)),
                    )
                }
            })
            .collect();
        Self { fighters, effects }
    }

    /// The SpEffects acting on fighter `i`: its animation's plus its resident ones, each only
    /// while its HP gates are open.
    pub fn active_effects(&self, i: usize, character: &Character) -> Vec<ActiveEffect> {
        let f = &self.fighters[i];
        character
            .tae_frame()
            .sp_effects
            .iter()
            .chain(f.resident.iter())
            .filter_map(|id| self.effects.get(id))
            .filter(|e| e.hp_gate_open(f.vitality.hp, f.vitality.max_hp))
            .cloned()
            .collect()
    }

    /// Regains posture for fighter `i` over `dt` seconds (read from code, see
    /// [`crate::combat::recovery`]). The animation's control type also clamps the meter, which
    /// is how a posture break ends at a fixed value.
    pub fn recover(&mut self, i: usize, character: &Character, dt: f32) {
        let effects = self.active_effects(i, character);
        let stamina_type = character.tae_frame().stamina_type;
        let f = &mut self.fighters[i];
        let mut inputs = RecoveryInputs::base(f.recovery_base);
        inputs.control_ratio = stamina_type
            .zip(f.control.as_ref())
            .map(|(t, c)| c.recover_ratio(t.max(0) as usize));
        let rate = posture_recovery_per_second(inputs, &effects);
        f.recovery_rate = rate;
        let gain = f.recovery.tick(rate, dt);
        let control = f.control_range(stamina_type);
        if f.dead() {
            return;
        }
        let target = f.posture.remaining + gain;
        f.posture.set(target, false, control);
    }

    /// Resolves one contact between `chars[hit.attacker]` and `chars[hit.defender]`, applying
    /// HP and posture damage to both fighters.
    pub fn resolve(
        &mut self,
        hit: &HitEvent,
        chars: [&Character; 2],
        combatants: [&Combatant; 2],
    ) -> HitResult {
        let (a, d) = (hit.attacker, hit.defender);
        let Some(atk) = hit.attack.profile.clone() else {
            return HitResult::default();
        };
        let (attacker, defender) = (chars[a], chars[d]);
        let effects_a = self.active_effects(a, attacker);
        let effects_d = self.active_effects(d, defender);
        let frame_d = defender.tae_frame();
        let guard_row: Option<&AttackData> = frame_d
            .guard_judge
            .and_then(|j| combatants[d].attacks.get(&j));
        let guard_profile = guard_row.and_then(|g| g.profile.as_ref());

        // 1. Guard test.
        let fd = &self.fighters[d];
        let (guard_angle, base_repel) = match &fd.kind {
            FighterKind::Player { weapon, .. } => (
                weapon.guard_angle,
                player_guard_repel(
                    guard_profile.map_or(100, |g| g.guard_break_correction),
                    weapon.guard_base_repel,
                    &effects_d,
                ),
            ),
            FighterKind::Npc(npc) => (
                npc.guard_angle,
                npc_guard_repel(guard_profile.map_or(0, |g| g.guard_break_rate), &effects_d),
            ),
        };
        let guard = GuardInput {
            guarding: frame_d.flag(crate::hits::FLAG_GUARD),
            forward: defender.body.forward(),
            guard_angle,
            base_repel,
            guard_attribute: guard_profile.map_or(0, |g| g.guard_attribute),
        };
        let repel_power = match &self.fighters[a].kind {
            FighterKind::Player { weapon, .. } if atk.guard_atk_rate_correction > 0 => {
                player_attack_repel(&atk, weapon.attack_base_repel)
            }
            _ => atk.guard_atk_rate as i32,
        };
        let resolution =
            resolve_hit_with_repel(&atk, repel_power, hit.direction, &guard, &effects_d);
        let outcome = resolution.outcome;
        let deflect = outcome == HitOutcome::Deflect;

        // 2. Posture damage to the defender.
        let attack_rate = attacker_stamina_attack_rate(&effects_a);
        let mut defender_side = match &self.fighters[d].kind {
            FighterKind::Player {
                weapon, reinforce, ..
            } => DefenderPosture::player(weapon, deflect, reinforce.stamina_guard_def),
            FighterKind::Npc(npc) => DefenderPosture {
                attribute_rate: npc.stamina_dmg_rates.get(atk.stamina_attribute),
                guard_def: npc.stamina_guard_def as f32,
                ..DefenderPosture::default()
            },
        };
        if let Some(g) = guard_profile {
            defender_side.guard_row_cut_rate = g.guard_stamina_cut_rate;
        }
        defender_side.guard_behavior_stamina = guard_row.map_or(0, |g| g.behavior_stamina);
        let posture_damage = match &self.fighters[a].kind {
            FighterKind::Player {
                weapon, reinforce, ..
            } => {
                let base =
                    player_posture_base(outcome, &atk, weapon, reinforce.stamina_atk, attack_rate);
                defender_posture_damage_from_base(outcome, base, &atk, &defender_side, &effects_d)
            }
            FighterKind::Npc(_) => {
                defender_posture_damage(outcome, &atk, attack_rate, &defender_side, &effects_d)
            }
        };
        let control_d = self.fighters[d].control_range(frame_d.stamina_type);
        let contact = apply_defender_posture(
            &mut self.fighters[d].posture,
            outcome,
            posture_damage,
            &atk,
            control_d,
        );
        let guard_broken = contact.broke && outcome == HitOutcome::Block;

        // 3. Posture damage back to the attacker.
        let weapon_rate = match &self.fighters[d].kind {
            FighterKind::Player {
                weapon, reinforce, ..
            } => weapon.passive_stamina_atk_rate * reinforce.passive_stamina_atk,
            FighterKind::Npc(_) => 1.0,
        };
        let attacker_damage =
            attacker_posture_damage(outcome, &atk, guard_broken, weapon_rate, &effects_d);
        let attacker_broke =
            outcome.guarded() && attacker_breaks(&self.fighters[a].posture, attacker_damage);
        let control_a = self.fighters[a].control_range(attacker.tae_frame().stamina_type);
        self.fighters[a].posture.take(attacker_damage, control_a);

        // 4. HP damage.
        let hp_damage = self.hp_damage(a, d, &atk, &hit.attack, outcome, guard_profile);
        let fd = &mut self.fighters[d];
        let hp_damage = keep_hp(fd.vitality.hp, hp_damage, atk.excess_dmg_keep_hp);
        fd.vitality.take(hp_damage, false);
        let dead = fd.dead();
        let broke = outcome == HitOutcome::Hit && (contact.broke || fd.vitality.deathblow_ready());

        // 5. Reaction codes.
        let defender_code = defender_reaction(DefenderFacts {
            outcome,
            dead,
            guard_broken,
            guard_break_reaction: false,
            damage_level: hit.attack.damage_level.max(0) as u32,
            broke,
            attacker_broke,
            does_break_repel_stam_damage: atk.does_break_repel_stam_damage,
        })
        .code();
        let attacker_code = attacker_reaction(
            outcome,
            guard_broken,
            hit.attack.category,
            false,
            attacker_broke,
            atk.does_break_repel_stam_damage,
        );

        let (direction, front_back) = hit_direction(defender, hit.direction);
        let level = hit.attack.damage_level.max(1);
        HitResult {
            defender: IncomingDamage {
                damage_type: defender_code,
                level,
                direction,
                front_back,
                guard_level: if outcome.guarded() { 1 } else { 0 },
                repel_dir: if outcome.guarded() { 2 } else { 0 },
                ..IncomingDamage::default()
            },
            attacker: attacker_code.map(|c| IncomingDamage {
                damage_type: c.0,
                level: 1,
                just_repelled: i32::from(deflect),
                repelled: i32::from(!deflect),
                ..IncomingDamage::default()
            }),
            hp_damage,
            posture_damage: contact.applied,
            attacker_posture_damage: attacker_damage,
            deflected: deflect,
            guarded: outcome.guarded(),
            posture_broke: broke || attacker_broke,
        }
    }

    /// HP damage of `atk` from fighter `a` on fighter `d` for `outcome`.
    fn hp_damage(
        &self,
        a: usize,
        d: usize,
        atk: &AttackProfile,
        data: &AttackData,
        outcome: HitOutcome,
        guard: Option<&AttackProfile>,
    ) -> i32 {
        let attack = match &self.fighters[a].kind {
            FighterKind::Player {
                attack_base,
                reinforce,
                ..
            } => std::array::from_fn(|i| {
                atk.attack_power[i]
                    + attack_base[i] * reinforce.attack[i] * data.atk_corrections[i] as f32 * 0.01
            }),
            FighterKind::Npc(_) => atk.attack_power,
        };
        let mut mult = [1.0; 5];
        if outcome.guarded() {
            // `atkAttribute` 1.. indexes slash, blow, thrust, ... (0 means none).
            let attr = (atk.atk_attribute as usize).checked_sub(1);
            let guard_rate = guard.map_or(0, |g| g.guard_rate);
            mult[0] = match &self.fighters[d].kind {
                FighterKind::Player {
                    weapon, reinforce, ..
                } => {
                    let rate = attr
                        .and_then(|i| weapon.guard_cut_attribute_rates.get(i))
                        .map_or(0.0, |&r| r as f32);
                    let cut = if outcome == HitOutcome::Deflect {
                        weapon.phys_just_guard_cut_rate
                    } else {
                        weapon.phys_guard_cut_rate
                    };
                    player_guard_cut(
                        cut,
                        reinforce.physics_guard_cut,
                        attribute_guard_factor(rate),
                        guard_rate,
                    )
                }
                FighterKind::Npc(npc) => {
                    let rate = attr
                        .and_then(|i| npc.guard_cut_attribute_rates.get(i))
                        .map_or(0.0, |&r| r as f32);
                    npc_guard_cut(
                        npc.phys_guard_cut_rate,
                        guard_rate,
                        attribute_guard_factor(rate),
                    )
                }
            };
        }
        let total = element_damage(attack, [0.0; 5], mult, 1.0);
        final_hp_damage(total, 1.0, 1.0)
    }

    /// Ends fighter `d` with a deathblow: spends a deathblow mark and takes all its HP (the
    /// deathblow's own damage is whatever is left, as every Sekiro deathblow on a last mark).
    pub fn deathblow(&mut self, d: usize) {
        let v = &mut self.fighters[d].vitality;
        let hp = v.hp;
        v.cannot_die = false;
        v.take(hp, true);
        if v.deathblows_left == 0 {
            v.set_hp(0);
        }
    }

    /// Whether fighter `d` can be deathblown now: posture broken, or down to 1 HP with marks
    /// left (read from code, see [`Vitality::deathblow_ready`]).
    pub fn deathblow_open(&self, d: usize) -> bool {
        let f = &self.fighters[d];
        !f.dead() && (f.posture.broken() || f.vitality.deathblow_ready())
    }

    pub fn reset(&mut self) {
        for f in &mut self.fighters {
            f.reset();
        }
    }
}

/// Hit direction codes for the defender's script: `(direction, front_back)`. Direction is 0
/// left, 1 right, 2 front, 3 back as the defender sees the attacker; front_back 0 front, 1 back.
fn hit_direction(defender: &Character, travel: [f32; 2]) -> (i32, i32) {
    let [fx, fz] = defender.body.forward();
    // From the defender toward the attacker.
    let (ix, iz) = (-travel[0], -travel[1]);
    let front = fx * ix + fz * iz;
    let right = fz * ix - fx * iz;
    let from_front = front >= 0.0;
    let direction = if front.abs() >= right.abs() {
        if from_front { 2 } else { 3 }
    } else if right > 0.0 {
        1
    } else {
        0
    };
    (direction, if from_front { 0 } else { 1 })
}

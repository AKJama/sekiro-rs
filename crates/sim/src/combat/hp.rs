//! HP damage and the HP side of deathblows.

use super::trunc;

/// Attack-versus-defence curve for one damage element (`840ce0`).
///
/// Read from code: a piecewise-quadratic absorption curve over `attack / defence` with knots at
/// 0.12, 1, 2.5 and 8 (globals at 0x143b01918..24) and absorption percents plus a flat share in
/// five globals at 0x143d5c100..114. In the analysed code those five are all zero, which makes the
/// curve return the attack unchanged, so this function does that. Whether the game fills them at
/// run time (they live in zero-initialised data) is open; NPC defence values are mostly 0 anyway.
pub fn defence_curve(attack: f32, _defence: f32) -> f32 {
    attack
}

/// HP damage from per-element attack powers and per-element multipliers (`840eb0`).
///
/// Read from code: each of the five elements (physical, magic, fire, lightning, dark) is
/// `max(0, defence_curve(attack, defence) * product of its multipliers)`; the five are summed and
/// multiplied by `scale` (critical and special-hit factors, 1.0 normally). A positive total below
/// 1 is rounded up to 1. The caller truncates after further multipliers (see [`final_hp_damage`]).
///
/// Which params fill the seven multiplier arrays is only partly traced: they include the
/// defender's attribute cut rates (`<attr>DamageCutRate`), attacker and defender SpEffect rates
/// and, when guarded, the guard cut from [`npc_guard_cut`] (inferred).
pub fn element_damage(
    attack: [f32; 5],
    defence: [f32; 5],
    multipliers: [f32; 5],
    scale: f32,
) -> f32 {
    let mut total = 0.0;
    for i in 0..5 {
        total += (defence_curve(attack[i], defence[i]) * multipliers[i]).max(0.0);
    }
    let total = total * scale;
    if total < 0.0 {
        0.0
    } else if total < 1.0 {
        total.ceil()
    } else {
        total
    }
}

/// The guard HP multiplier of a blocking NPC for physical damage (`845ed0`).
///
/// Read from code: `(100 - attribute_factor * physGuardCutRate * (1 + guardRate / 100)) / 100`,
/// where `guardRate` is from the guard action's AtkParam row. `attribute_factor` comes from a
/// per-attribute NpcParam lookup that Ghidra could not recover; 1.0 is assumed (inferred). A
/// negative result becomes 0 in [`element_damage`]. The player's guard cut (`8460d0`) was not
/// traced; using the weapon's `physGuardCutRate` (block) or `physJustGuardCutRate` (deflect) in
/// the same formula is our assumption.
pub fn npc_guard_cut(phys_guard_cut_rate: f32, guard_rate: i16, attribute_factor: f32) -> f32 {
    (100.0 - attribute_factor * phys_guard_cut_rate * (guard_rate as f32 * 0.01 + 1.0)) * 0.01
}

/// Final HP damage of one hit (`b6c880`): the element total times the part's damage-group rate
/// and the repel cut (`841c80`, applied when the defender's repel defence met the attack's repel
/// power), truncated. Read from code for the order and the truncation.
pub fn final_hp_damage(element_total: f32, part_rate: f32, repel_cut: f32) -> i32 {
    trunc(element_total * part_rate * repel_cut)
}

/// Lowers HP damage so that HP does not drop below `keep` (AtkParam `excessDmgKeepHp`).
///
/// Read from code (`b6e6a0`): when HP is above `keep` and the hit would end at or below it, the
/// damage becomes `hp - keep`; when HP is already at or below `keep` the damage becomes 0.
/// `keep` 0 leaves damage alone except that it cannot exceed HP.
pub fn keep_hp(hp: i32, damage: i32, keep: u16) -> i32 {
    let keep = keep as i32;
    if keep < hp {
        if hp - damage <= keep {
            hp - keep
        } else {
            damage
        }
    } else {
        0
    }
}

/// HP and the deathblow counter of a character (`SprjChrDataModule` +0x130, +0x134, +0x25C).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vitality {
    pub hp: i32,
    pub max_hp: i32,
    /// Deathblows still needed (NpcParam `ninsatuNum` at spawn, read from code `bd6ae0`).
    pub deathblows_left: i32,
    /// Something else forbids death (no-dead flag, SpEffect state 143, debug). Read from code
    /// (`bd5df0`).
    pub cannot_die: bool,
}

impl Vitality {
    pub fn new(max_hp: i32, deathblows: i32) -> Self {
        let max_hp = max_hp.max(1);
        Self {
            hp: max_hp,
            max_hp,
            deathblows_left: deathblows,
            cannot_die: false,
        }
    }

    /// Whether HP may not reach 0: any deathblows left, or `cannot_die` (read from code, `bd5df0`).
    pub fn protected(&self) -> bool {
        self.cannot_die || self.deathblows_left > 0
    }

    /// Sets HP (`bd64e0`): clamped to `[0, max]`; if it would drop below 1 from a positive value
    /// while [`Vitality::protected`], it stays at 1. Read from code.
    pub fn set_hp(&mut self, value: i32) {
        let old = self.hp;
        self.hp = value.clamp(0, self.max_hp);
        if self.hp < 1 && self.protected() && old > 0 {
            self.hp = 1;
        }
    }

    /// Applies HP damage from a hit (`b6e6a0`): a deathblow hit first spends one deathblow, then
    /// the damage is applied through [`Vitality::set_hp`]. Read from code; which record value marks
    /// a deathblow hit (5 in slot +0x28) is inferred.
    pub fn take(&mut self, damage: i32, is_deathblow: bool) {
        if is_deathblow && self.deathblows_left > 0 {
            self.deathblows_left -= 1;
        }
        self.set_hp(self.hp - damage);
    }

    /// The HP side of the break test in `b6e6a0`: HP below 2 with at least one deathblow left
    /// counts like an empty posture meter, so a protected enemy knocked to 1 HP becomes
    /// deathblow-ready (damage type 1027 on a direct hit). Read from code.
    pub fn deathblow_ready(&self) -> bool {
        self.hp < 2 && self.deathblows_left >= 1
    }
}

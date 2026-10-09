//! Deciding whether an attack hits, is blocked, or is deflected.
//!
//! Source: the defender's damage module, `b6c880` (the first stage of applying one hit record,
//! called from `b6f690`). The steps, in the engine's order:
//!
//! 1. The defender must be in a guarding state, and the arc test must pass (or the attack must
//!    ignore direction with `isAllDirGuard`). Read from code.
//! 2. The defender's repel defence is computed (guarding or not) and raised to the highest
//!    `defFlickPower` of its active SpEffects. Read from code (`842fb0`, `bfc440`).
//! 3. Repel defence below the attack's repel power means a block, unless the attack is
//!    unblockable for the defender's guard type, in which case it is a direct hit.
//!    Repel defence at or above it means a deflect, unless the attack is undeflectable for that
//!    guard type, in which case it is a direct hit. Read from code.
//!
//! So the deflect window works by raising repel defence: the window effect 105010 has
//! `defFlickPower` 40, ordinary NPC attacks have `guardAtkRate` 30, and the Kusabimaru's own guard
//! repel is 20 (`guardBaseRepel`). Inside the window 40 >= 30 deflects; outside 20 < 30 blocks.

use super::effects::{ActiveEffect, guard_flick_power_rate, repel_with_overrides};
use super::params::AttackProfile;
use super::trunc;

/// How one hit landed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitOutcome {
    /// Unguarded (or unguardable) hit.
    Hit,
    /// Blocked: guarding, repel defence below the attack's repel power.
    Block,
    /// Deflected (just guard): guarding, repel defence at least the attack's repel power.
    Deflect,
}

impl HitOutcome {
    pub fn guarded(self) -> bool {
        !matches!(self, Self::Hit)
    }
}

/// The defender's guard at the moment of contact.
#[derive(Debug, Clone, PartialEq)]
pub struct GuardInput {
    /// The defender is in a guarding state (engine flag word bit 2 at +0x70 of the action module).
    pub guarding: bool,
    /// Defender facing on the ground plane (unit length not required).
    pub forward: [f32; 2],
    /// Guard half-angle in degrees: NpcParam `guardAngle` for NPCs, EquipParamWeapon
    /// `guardAngle` for the player. 0 means the default 90 degrees (read from code).
    pub guard_angle: i16,
    /// Repel defence before SpEffect overrides: see [`npc_guard_repel`] and [`player_guard_repel`]
    /// while guarding, and NpcParam `defFlickPower` (0 for the player) while not.
    pub base_repel: i32,
    /// Guard type of the guard action's AtkParam row (`guardAttribute`): 0 katana, 1 umbrella.
    pub guard_attribute: u8,
}

/// Result of [`resolve_hit`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitResolution {
    pub outcome: HitOutcome,
    /// The repel defence that was compared, after overrides.
    pub repel: i32,
    /// The guard test passed but the attack's flags turned it into a direct hit.
    pub guard_bypassed: bool,
}

/// Whether the guard arc covers an attack travelling along `incoming` (from attacker toward the
/// defender, ground plane).
///
/// Read from code (`b6c880`): with both vectors normalised, the hit is inside the arc when
/// `dot(forward, incoming) < cos((angle + 180) degrees)`, that is the angle between the defender's
/// facing and the direction back toward the attacker is below `angle`. A zero angle uses
/// threshold 0, a 90 degree half-angle.
pub fn in_guard_arc(forward: [f32; 2], incoming: [f32; 2], guard_angle: i16) -> bool {
    let norm = |v: [f32; 2]| {
        let len = (v[0] * v[0] + v[1] * v[1]).sqrt();
        if len == 0.0 {
            [0.0, 0.0]
        } else {
            [v[0] / len, v[1] / len]
        }
    };
    let f = norm(forward);
    let d = norm(incoming);
    let dot = f[0] * d[0] + f[1] * d[1];
    let threshold = if guard_angle != 0 {
        ((guard_angle as f32 + 180.0) * 0.017_453_3).cos()
    } else {
        0.0
    };
    dot < threshold
}

/// Repel defence of a guarding NPC: the guard action's AtkParam `guardBreakRate` times the
/// product of `guardDefFlickPowerRate` over active 158/204 effects, truncated.
/// Read from code (`a137a0`, `84cd80`, `bfccc0`).
pub fn npc_guard_repel(guard_row_break_rate: u16, effects: &[ActiveEffect]) -> i32 {
    trunc(guard_row_break_rate as f32 * guard_flick_power_rate(effects))
}

/// Repel defence of the guarding player: the guard AtkParam_Pc `guardBreakCorrection` percent
/// times the weapon's `guardBaseRepel` times the guard effect rate, truncated.
///
/// Read from code (`a28da0`, `84ce20`), with two simplifications: the engine first zeroes the
/// value when the player's stats are below the weapon's requirements and then adds a stat bonus
/// of up to 10; Sekiro has no such stats, so both are left out (inferred).
pub fn player_guard_repel(
    guard_break_correction: u16,
    weapon_guard_base_repel: u8,
    effects: &[ActiveEffect],
) -> i32 {
    let correction = guard_break_correction as f32 * 0.01;
    trunc(correction * weapon_guard_base_repel as f32 * guard_flick_power_rate(effects)).max(0)
}

/// Decides between hit, block and deflect for one contact.
///
/// `incoming` is the attack's travel direction (attacker toward defender) on the ground plane.
/// `defender_effects` are the defender's active SpEffects.
///
/// Not modelled (open, see `docs/COMBAT-RULES.md`): the attack-versus-attack clash path that
/// forces a guard through `guardAtkRate`/`guardBreakRate` of two attack rows, the position-based
/// arc test used for bullets, and the per-target filter `c03aa0` on effects.
pub fn resolve_hit(
    attack: &AttackProfile,
    incoming: [f32; 2],
    guard: &GuardInput,
    defender_effects: &[ActiveEffect],
) -> HitResolution {
    let covers = guard.guarding
        && (attack.is_all_dir_guard || in_guard_arc(guard.forward, incoming, guard.guard_angle));
    let repel = repel_with_overrides(guard.base_repel, defender_effects);
    if !covers {
        return HitResolution {
            outcome: HitOutcome::Hit,
            repel,
            guard_bypassed: false,
        };
    }
    let attr = guard.guard_attribute as usize;
    let flag = |flags: &[bool; 2]| attr < 2 && flags[attr];
    let (outcome, bypassed) = if repel < attack.guard_atk_rate as i32 {
        if flag(&attack.disable_guard) {
            (HitOutcome::Hit, true)
        } else {
            (HitOutcome::Block, false)
        }
    } else if flag(&attack.disable_just_guard) {
        (HitOutcome::Hit, true)
    } else {
        (HitOutcome::Deflect, false)
    };
    HitResolution {
        outcome,
        repel,
        guard_bypassed: bypassed,
    }
}

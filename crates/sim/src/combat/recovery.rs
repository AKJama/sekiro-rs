//! Posture recovery over time.
//!
//! Source: the per-frame character update `a04850` (ChrIns), reached from `a16580` (EnemyIns) and
//! `a37560` (PlayerIns), with the base speed from the virtuals `a13a70` (enemy) and `a2a5d0`
//! (player). Read from code unless noted:
//!
//! ```text
//! rate = effects_speed_rate * (base_speed + effects_change_speed) * anim_scale [* control% / 100]
//! accumulator += rate * dt
//! gain = trunc(accumulator); accumulator -= gain; posture += gain (same setter as damage)
//! ```
//!
//! `anim_scale` is a percent byte of the character's action state times a per-character float
//! (`+0x10d0`); both are 100% / 1.0 unless an animation changes them (inferred: their writers were
//! not traced). The control percent is `staminaRecoverRatio_forTypeNNN` of the character's
//! StaminaControlParam row for the type the current animation selects (TAE event 960 per
//! Smithbox, community); with no type active the factor is skipped. The whole update is skipped
//! while a flag of the action state is set (bit 21 of +0x88, inferred to be an animation-driven
//! "no recovery").
//!
//! There is no separate timer for "delay after damage" in this code; pauses come from the
//! animation's control type (type 0 is 0% for both the player row 0 and the soldier row
//! 1000100). HP affects recovery only through SpEffects gated by `conditionHp`, such as the
//! soldier's resident 300600 / 300601 / 300602 (0.6, 0.5, 1/3 at 80%, 60%, 40% HP); all active
//! ones multiply.

use super::effects::{ActiveEffect, recover_change_speed, recover_speed_rate};
use super::trunc;

/// Inputs to [`posture_recovery_per_second`] besides SpEffects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecoveryInputs {
    /// NpcParam `staminaRecoverBaseVel` for NPCs; the player's comes from another virtual (not
    /// traced).
    pub base_speed: f32,
    /// Animation percent byte (100 = normal), inferred default.
    pub anim_percent: u8,
    /// Per-character recovery scale, inferred default 1.0.
    pub chr_scale: f32,
    /// `staminaRecoverRatio` of the active control type, or `None` when no type is active.
    pub control_ratio: Option<u32>,
}

impl RecoveryInputs {
    pub fn base(base_speed: f32) -> Self {
        Self {
            base_speed,
            anim_percent: 100,
            chr_scale: 1.0,
            control_ratio: None,
        }
    }
}

/// Recovery speed in posture points per second, as the engine computes it before integrating.
/// `effects` must already be filtered to the ones whose HP gates are open.
pub fn posture_recovery_per_second(inputs: RecoveryInputs, effects: &[ActiveEffect]) -> f32 {
    let mut scale = inputs.anim_percent as f32 * 0.01 * inputs.chr_scale;
    if let Some(ratio) = inputs.control_ratio {
        scale = scale * ratio as f32 * 0.01;
    }
    let speed = inputs.base_speed + recover_change_speed(effects) as f32;
    recover_speed_rate(effects) * speed * scale
}

/// The "Stamina Recover[point/s]" value the engine's debug display shows: the per-second speed
/// truncated (read from code, `a04850`).
pub fn displayed_recovery(rate: f32) -> i32 {
    trunc(rate)
}

/// The fractional carry the engine keeps between frames (`SprjChrDataModule` +0x15C).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RecoveryAccumulator {
    pub carry: f32,
}

impl RecoveryAccumulator {
    /// Advances by `dt` seconds at `rate` points per second and returns the whole points to add.
    /// Read from code (`a04850`, `5460b0`, `9e6bf0`).
    pub fn tick(&mut self, rate: f32, dt: f32) -> i32 {
        let total = rate * dt + self.carry;
        let whole = trunc(total);
        self.carry = total - whole as f32;
        whole
    }
}

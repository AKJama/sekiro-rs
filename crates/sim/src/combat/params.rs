//! Combat inputs built from param rows.
//!
//! These read generic [`Row`]s by Paramdex field name so the combat code does not depend on which
//! fields the typed tables happen to expose.

use sekiro_formats::Result;
use sekiro_formats::param::Row;

use super::effects::{ActiveEffect, StaminaAttribute, StaminaRates};

/// The parts of an AtkParam row (NPC or PC) that hit resolution and damage read.
#[derive(Debug, Clone, PartialEq)]
pub struct AttackProfile {
    pub id: i32,
    /// `guardAtkRate`: repel attack power. A guarding defender whose repel defence is at least
    /// this deflects instead of blocking. Inferred: the record slot compared in `b6c880` is filled
    /// from this field (all neighbouring slots match AtkParam offsets).
    pub guard_atk_rate: u16,
    /// Physical, magic, fire, lightning and dark attack power (`atkPhys` .. `atkDark`).
    pub attack_power: [f32; 5],
    /// `directAtkStamDamage`: posture damage to the defender on a direct hit.
    pub direct_stam: i16,
    /// `repelLostStamDamage`: posture damage to the defender when it blocks.
    pub blocked_stam: i16,
    /// `atkStam`: posture damage to the defender when it deflects.
    pub deflected_stam: i16,
    /// `directAtkStamDamage_Attacker`.
    pub attacker_direct_stam: i16,
    /// `repelVictoryStamDamage_Attacker`: posture the attacker takes when blocked.
    pub attacker_blocked_stam: i16,
    /// `repelLostStamDamage_Attacker`: posture the attacker takes when deflected.
    pub attacker_deflected_stam: i16,
    /// `staminaPhysicsAttribute`.
    pub stamina_attribute: StaminaAttribute,
    pub disable_stamina_attack: bool,
    pub does_break_repel_stam_damage: bool,
    pub is_all_dir_guard: bool,
    /// `disableGuard_vsGuardAttribute0/1`: cannot be blocked by guard type 0 (katana) or 1.
    pub disable_guard: [bool; 2],
    /// `disableJustGuard_vsGuardAttribute0/1`: cannot be deflected by guard type 0 or 1.
    pub disable_just_guard: [bool; 2],
    pub excess_dmg_keep_hp: u16,
    /// `guardBreakRate`: repel defence of this row when it is a guard action's attack row.
    pub guard_break_rate: u16,
    /// `guardBreakCorrection`: PC guard rows, percent applied to the weapon's base repel.
    pub guard_break_correction: u16,
    /// `guardStaminaCutRate`: correction to the guard posture cut when this is a guard row.
    pub guard_stamina_cut_rate: i16,
    /// `guardRate`: correction to the HP guard cut when this is a guard row.
    pub guard_rate: i16,
    /// `guardAttribute`: guard type of this row when it is a guard action (0 katana, 1 umbrella).
    pub guard_attribute: u8,
}

impl AttackProfile {
    pub fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row.id(),
            guard_atk_rate: row.u16("guardAtkRate")?,
            attack_power: [
                row.int("atkPhys")? as f32,
                row.int("atkMag")? as f32,
                row.int("atkFire")? as f32,
                row.int("atkThun")? as f32,
                row.int("atkDark")? as f32,
            ],
            direct_stam: row.i16("directAtkStamDamage")?,
            blocked_stam: row.i16("repelLostStamDamage")?,
            deflected_stam: row.i16("atkStam")?,
            attacker_direct_stam: row.i16("directAtkStamDamage_Attacker")?,
            attacker_blocked_stam: row.i16("repelVictoryStamDamage_Attacker")?,
            attacker_deflected_stam: row.i16("repelLostStamDamage_Attacker")?,
            stamina_attribute: StaminaAttribute(row.u8("staminaPhysicsAttribute")?),
            disable_stamina_attack: row.bool("disableStaminaAttack")?,
            does_break_repel_stam_damage: row.bool("doesBreakRepelStamDamage")?,
            is_all_dir_guard: row.bool("isAllDirGuard")?,
            disable_guard: [
                row.u8("disableGuard_vsGuardAttribute0")? != 0,
                row.u8("disableGuard_vsGuardAttribute1")? != 0,
            ],
            disable_just_guard: [
                row.u8("disableJustGuard_vsGuardAttribute0")? != 0,
                row.u8("disableJustGuard_vsGuardAttribute1")? != 0,
            ],
            excess_dmg_keep_hp: row.u16("excessDmgKeepHp")?,
            guard_break_rate: row.u16("guardBreakRate")?,
            guard_break_correction: row.u16("guardBreakCorrection")?,
            guard_stamina_cut_rate: row.i16("guardStaminaCutRate")?,
            guard_rate: row.i16("guardRate")?,
            guard_attribute: row.u8("guardAttribute")?,
        })
    }
}

/// The parts of an NpcParam row that combat reads.
#[derive(Debug, Clone, PartialEq)]
pub struct NpcCombat {
    pub id: i32,
    pub hp: i32,
    /// `stamina`: maximum posture.
    pub max_posture: i32,
    pub stamina_recover_base_vel: f32,
    /// `maxDebtStamina`: lowest remaining posture (zero or negative).
    pub max_debt_stamina: i32,
    pub stamina_control_param_id: u32,
    /// `ninsatuNum`: deathblows needed.
    pub deathblows: i32,
    pub guard_angle: i16,
    /// `defFlickPower`: repel defence while not guarding.
    pub def_flick_power: u16,
    pub stamina_guard_def: u8,
    pub phys_guard_cut_rate: f32,
    pub stamina_dmg_rates: StaminaRates,
    /// Resident SpEffects `spEffectID0..31` (unused slots are -1).
    pub sp_effect_ids: Vec<i32>,
}

impl NpcCombat {
    pub fn from_row(row: &Row) -> Result<Self> {
        const RATES: [&str; 12] = [
            "slashStaminaDmgRate",
            "lightHitStaminaDmgRate",
            "thrustStaminaDmgRate",
            "neutralStaminaDmgRate",
            "ninsatuStaminaDmgRate",
            "heavyHitStaminaDmgRate",
            "antiGroundStaminaDmgRate",
            "antiAirStaminaDmgRate",
            "lightShootStaminaDmgRate",
            "attriAStaminaDmgRate",
            "attriBStaminaDmgRate",
            "attriCStaminaDmgRate",
        ];
        let mut rates = [1.0; 12];
        for (r, name) in rates.iter_mut().zip(RATES) {
            *r = row.f32(name)?;
        }
        let mut sp_effect_ids = Vec::with_capacity(32);
        for i in 0..32 {
            sp_effect_ids.push(row.i32(&format!("spEffectID{i}"))?);
        }
        Ok(Self {
            id: row.id(),
            hp: row.int("hp")? as i32,
            max_posture: row.int("stamina")? as i32,
            stamina_recover_base_vel: row.int("staminaRecoverBaseVel")? as f32,
            max_debt_stamina: row.int("maxDebtStamina")? as i32,
            stamina_control_param_id: row.u32("staminaControlParamId")?,
            deathblows: row.int("ninsatuNum")? as i32,
            guard_angle: row.i16("guardAngle")?,
            def_flick_power: row.u16("defFlickPower")?,
            stamina_guard_def: row.u8("staminaGuardDef")?,
            phys_guard_cut_rate: row.f32("physGuardCutRate")?,
            stamina_dmg_rates: StaminaRates(rates),
            sp_effect_ids,
        })
    }
}

impl ActiveEffect {
    pub fn from_row(row: &Row) -> Result<Self> {
        const RATES: [&str; 12] = [
            "defSlashStaminaDmgRate",
            "defLightHitStaminaDmgRate",
            "defThrustStaminaDmgRate",
            "defNeutralStaminaDmgRate",
            "defNinsatuStaminaDmgRate",
            "defHeavyHitStaminaDmgRate",
            "defAntiGroundStaminaDmgRate",
            "defAntiAirStaminaDmgRate",
            "defLightShootStaminaDmgRate",
            "defAttriAStaminaDmgRate",
            "defAttriBStaminaDmgRate",
            "defAttriCStaminaDmgRate",
        ];
        let mut rates = [1.0; 12];
        for (r, name) in rates.iter_mut().zip(RATES) {
            *r = row.f32(name)?;
        }
        Ok(Self {
            id: row.id(),
            state_info: row.u16("stateInfo")?,
            def_flick_power: row.u8("defFlickPower")?,
            guard_def_flick_power_rate: row.f32("guardDefFlickPowerRate")?,
            guard_stamina_cut_rate: row.f32("guardStaminaCutRate")?,
            def_stamina_attack_rate: row.f32("defStaminaAttackRate")?,
            def_stamina_dmg_rates: StaminaRates(rates),
            stamina_recover_speed_rate: row.f32("staminaRecoverSpeedRate")?,
            stamina_recover_change_speed: row.i32("staminaRecoverChangeSpeed")?,
            condition_hp: row.f32("conditionHp")?,
            condition_hp_rate: row.f32("conditionHpRate")?,
            max_stamina_rate: row.f32("maxStaminaRate")?,
        })
    }
}

/// The guard-related parts of an EquipParamWeapon row.
#[derive(Debug, Clone, PartialEq)]
pub struct WeaponGuard {
    pub id: i32,
    /// `guardAngle`: half-angle of the guard arc in degrees; 0 means the default 90.
    pub guard_angle: i16,
    pub guard_base_repel: u8,
    pub stamina_guard_def: i16,
    pub stamina_just_guard_def: i16,
    pub phys_guard_cut_rate: f32,
    pub phys_just_guard_cut_rate: f32,
    /// `passiveStaminaAtkRate`: multiplies the posture sent back to an attacker on a deflect.
    pub passive_stamina_atk_rate: f32,
}

impl WeaponGuard {
    pub fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row.id(),
            guard_angle: row.i16("guardAngle")?,
            guard_base_repel: row.u8("guardBaseRepel")?,
            stamina_guard_def: row.i16("staminaGuardDef")?,
            stamina_just_guard_def: row.i16("staminaJustGuardDef")?,
            phys_guard_cut_rate: row.f32("physGuardCutRate")?,
            phys_just_guard_cut_rate: row.f32("physJustGuardCutRate")?,
            passive_stamina_atk_rate: row.f32("passiveStaminaAtkRate")?,
        })
    }
}

/// One StaminaControlParam row: per recovery type (0..15) a recovery percent and the upper and
/// lower posture limits in percent of max.
#[derive(Debug, Clone, PartialEq)]
pub struct StaminaControl {
    pub id: i32,
    pub recover_ratio: [u32; 16],
    pub max_ratio: [u16; 16],
    pub min_ratio: [u16; 16],
}

impl StaminaControl {
    pub fn from_row(row: &Row) -> Result<Self> {
        let mut out = Self {
            id: row.id(),
            recover_ratio: [100; 16],
            max_ratio: [100; 16],
            min_ratio: [0; 16],
        };
        for i in 0..16 {
            out.recover_ratio[i] = row.u32(&format!("staminaRecoverRatio_forType{i:03}"))?;
            out.max_ratio[i] = row.u16(&format!("staminaMaxRatio_forType{i:03}"))?;
            out.min_ratio[i] = row.u16(&format!("staminaMinRatio_forType{i:03}"))?;
        }
        Ok(out)
    }

    /// Recovery percent for `kind`, or 100 for an unknown type (read from code, `10cf5c0`).
    pub fn recover_ratio(&self, kind: usize) -> u32 {
        self.recover_ratio.get(kind).copied().unwrap_or(100)
    }
}

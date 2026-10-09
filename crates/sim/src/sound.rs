//! Which sounds a simulation step plays, by the game's sound event names.
//!
//! Two sources, both from the game's data (see `docs/AUDIO.md`):
//!
//! - TAE `PlaySound_*` events (types 128 to 132) of the clips playing, collected by
//!   [`crate::tae`] as (sound type, id) and named with [`crate::tae::sound_name`];
//! - hit, block and deflect sounds from the combat outcome, looked up in `HitEffectSeParam`
//!   (hits and blocks) and `HitEffectSeJustGuardParam` (deflects): the row is the defender's
//!   material, the column the attacker's material, attack kind and size. These are `s` sounds.

use std::collections::HashMap;
use std::path::Path;

use sekiro_formats::param::ParamSet;

use crate::character::Character;
use crate::hits::{HitEvent, HitResult};
use crate::tae::sound_name;

/// One sound to play.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundCue {
    /// Sound event name, such as `c101001001` or `s999999980`.
    pub name: String,
    /// 0 the player, 1 the enemy.
    pub actor: usize,
    pub position: [f32; 3],
}

/// Material names of the `<Material>_<Kind>_<Size>` columns, in `atkMaterial_forSe` order.
const MATERIALS: [&str; 15] = [
    "Iron",
    "Fire",
    "Wood",
    "Body",
    "Eclipse",
    "Energy",
    "None",
    "Dmy1",
    "Dmy2",
    "Dmy3",
    "Maggot",
    "Wax",
    "FireFlame",
    "EclipseGas",
    "EnergyStrong",
];

/// The `<Material>_<Kind>_<Size>` column for an attack.
fn column(material: u8, attribute: u8, size: u8) -> String {
    let m = MATERIALS.get(material as usize).copied().unwrap_or("Iron");
    // atkAttribute: 1 slash, 2 blow, 3 thrust (Paramdex); anything else counts as slash.
    let kind = match attribute {
        2 => "Blow",
        3 => "Thrust",
        _ => "Slash",
    };
    let size = match size {
        1 => "L",
        2 => "LL",
        _ => "S",
    };
    format!("{m}_{kind}_{size}")
}

/// The attack fields the sound lookup needs.
#[derive(Debug, Clone, Copy, Default)]
struct AttackSe {
    material: u8,
    attribute: u8,
    size: u8,
}

/// Hit-sound tables and the materials of the two duel characters.
#[derive(Debug, Clone, Default)]
pub struct HitSounds {
    hit: HashMap<i32, HashMap<String, i64>>,
    just_guard: HashMap<i32, HashMap<String, i64>>,
    attacks: [HashMap<i32, AttackSe>; 2],
    /// Material rows: [player, enemy] guarding weapon (`defSeMaterial1`) and body.
    pub guard_material: [i32; 2],
    pub body_material: [i32; 2],
}

fn table(set: &ParamSet, name: &str) -> HashMap<i32, HashMap<String, i64>> {
    let Ok(t) = set.table(name) else {
        return HashMap::new();
    };
    t.rows()
        .map(|r| {
            let fields = r
                .fields()
                .filter_map(|(d, v)| Some((d.name.clone(), v.as_i64()?)))
                .collect();
            (r.id(), fields)
        })
        .collect()
}

fn attacks(set: &ParamSet, name: &str) -> HashMap<i32, AttackSe> {
    let Ok(t) = set.table(name) else {
        return HashMap::new();
    };
    t.rows()
        .map(|r| {
            let u = |f: &str| r.u8(f).unwrap_or(0);
            (
                r.id(),
                AttackSe {
                    material: u("atkMaterial_forSe"),
                    attribute: u("atkAttribute"),
                    size: u("atkSize"),
                },
            )
        })
        .collect()
}

impl HitSounds {
    /// Loads the tables for Wolf with the Kusabimaru (EquipParamWeapon 5000) against the Ashina
    /// soldier (NpcParam 10100000).
    pub fn load(param_dir: &Path, defs_dir: &Path) -> Option<Self> {
        let set = ParamSet::load(param_dir, defs_dir).ok()?;
        let weapon_mat = set
            .table("EquipParamWeapon")
            .ok()
            .and_then(|t| t.find(5000))
            .and_then(|r| r.int("defSeMaterial1").ok())
            .unwrap_or(101) as i32;
        let soldier = set.table("NpcParam").ok().and_then(|t| t.find(10100000));
        let soldier_body = soldier
            .as_ref()
            .and_then(|r| r.int("materialSe1").ok())
            .unwrap_or(114) as i32;
        Some(Self {
            hit: table(&set, "HitEffectSeParam"),
            just_guard: table(&set, "HitEffectSeJustGuardParam"),
            attacks: [attacks(&set, "AtkParam_Pc"), attacks(&set, "AtkParam_Npc")],
            // The soldier guards with an iron weapon (row 100, "weapon iron", as its attack
            // rows' defSeMaterial1); Wolf's body row is 12, "flesh and blood" (ours).
            guard_material: [weapon_mat, 100],
            body_material: [12, soldier_body],
        })
    }

    pub fn load_from_cache(cache: &Path) -> Option<Self> {
        Self::load(
            &cache.join("raw/param/gameparam/gameparam.parambnd.d"),
            &cache.join("refs/paramdex/Defs"),
        )
    }

    /// The `s` sound for one hit, if the tables name one.
    pub fn cue(&self, hit: &HitEvent, result: &HitResult) -> Option<SoundCue> {
        let name = self.lookup(
            hit.attacker,
            hit.attack.atk_id,
            hit.defender,
            result.deflected,
            result.guarded,
        )?;
        Some(SoundCue {
            name,
            actor: hit.defender,
            position: hit.position,
        })
    }

    /// The sound event name for an attack (AtkParam row `atk_id` of `attacker`: 0 player,
    /// 1 enemy) meeting `defender`, deflected, blocked or landing.
    pub fn lookup(
        &self,
        attacker: usize,
        atk_id: i32,
        defender: usize,
        deflected: bool,
        guarded: bool,
    ) -> Option<String> {
        let atk = self.attacks[attacker.min(1)]
            .get(&atk_id)
            .copied()
            .unwrap_or_default();
        let col = column(atk.material, atk.attribute, atk.size);
        let d = defender.min(1);
        let (table, row) = if deflected {
            (&self.just_guard, self.guard_material[d])
        } else if guarded {
            (&self.hit, self.guard_material[d])
        } else {
            (&self.hit, self.body_material[d])
        };
        let id = *table.get(&row)?.get(&col)?;
        (id > 0).then(|| sound_name(5, id as i32))
    }
}

/// The TAE sound cues that started on `ch` this step.
pub fn tae_cues(actor: usize, ch: &Character) -> Vec<SoundCue> {
    ch.tae_frame()
        .sounds
        .iter()
        .map(|&(kind, id)| SoundCue {
            name: sound_name(kind, id),
            actor,
            position: ch.body.position,
        })
        .collect()
}

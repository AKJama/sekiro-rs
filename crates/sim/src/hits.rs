//! Attack hit detection: weapon capsules from TAE attack windows against body capsules.
//!
//! The data trail, per attack:
//!
//! 1. A TAE `AttackBehavior` event on the attacker's clip opens a window with a behaviour judge
//!    id.
//! 2. `BehaviorParam` (NPCs, id `200000000 + NpcParam.behaviorVariationId * 1000 + judge`) or
//!    `BehaviorParam_PC` (the player, `100000000 + EquipParamWeapon.behaviorVariationId * 1000 +
//!    judge`) names the `AtkParam` row (`refType` 0).
//! 3. The AtkParam row's `hitN_DmyPoly1`/`hitN_DmyPoly2`/`hitN_Radius` describe capsules between
//!    two dummy polys. For NPCs the dummies are on the character model (the soldier's `Kodachi`
//!    bone carries 10 and 11); for the player they are on the weapon model (`WP_A_0300` dummies
//!    100 and 120 on the blade), held at the character's dummy 1 on `R_Weapon`.
//! 4. While the window is open the capsule is swept from the previous step's pose to this one
//!    and tested against the defender's vertical body capsule. Each window hits a defender once.
//!
//! The result is a [`HitEvent`]. [`crate::fight::CombatRules`] turns it into HP and posture
//! damage and the reaction codes both scripts read, on the engine rules in [`crate::combat`].

use crate::character::Character;
use crate::combat::params::AttackProfile;
use crate::player::IncomingDamage;
use crate::tae::{TaeKind, TimedEvent};
use glam::{Mat4, Vec3};
use sekiro_formats::anim::Skeleton;
use sekiro_formats::flver::{self, Flver};
use sekiro_formats::param::{RawParam, Row, Table};
use sekiro_formats::paramdef::ParamDef;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

/// Dummy polys of a character (and its held weapon) relative to animated skeleton bones.
#[derive(Debug, Clone, Default)]
pub struct DummyRig {
    /// Reference id -> list of (Havok bone index, offset from that bone, source space).
    dummies: HashMap<i16, Vec<(usize, Mat4)>>,
}

fn node_world(nodes: &[flver::Node]) -> Vec<Mat4> {
    let mut out: Vec<Option<Mat4>> = vec![None; nodes.len()];
    fn resolve(i: usize, nodes: &[flver::Node], out: &mut [Option<Mat4>], depth: usize) -> Mat4 {
        if let Some(m) = out[i] {
            return m;
        }
        let local = nodes[i].local_matrix();
        let p = nodes[i].parent;
        let m = if p >= 0 && (p as usize) < nodes.len() && depth < nodes.len() {
            resolve(p as usize, nodes, out, depth + 1) * local
        } else {
            local
        };
        out[i] = Some(m);
        m
    }
    (0..nodes.len())
        .map(|i| resolve(i, nodes, &mut out, 0))
        .collect()
}

fn dummy_matrix(d: &flver::Dummy, blade: bool) -> Mat4 {
    let fwd = Vec3::from(d.forward).normalize_or(Vec3::Z);
    let up = Vec3::from(d.upward).normalize_or(Vec3::Y);
    let (x, y, z) = if blade {
        // The held part's -Y along the dummy's forward (how the katana sits in the hand).
        let y = -fwd;
        let z = (up - y * up.dot(y)).normalize_or(Vec3::Z);
        (y.cross(z), y, z)
    } else {
        let z = fwd;
        let x = up.cross(z).normalize_or(Vec3::X);
        (x, z.cross(x), z)
    };
    Mat4::from_cols(
        x.extend(0.0),
        y.extend(0.0),
        z.extend(0.0),
        Vec3::from(d.position).extend(1.0),
    )
}

fn parent_world(world: &[Mat4], d: &flver::Dummy) -> Mat4 {
    usize::try_from(d.parent_bone)
        .ok()
        .and_then(|p| world.get(p).copied())
        .unwrap_or(Mat4::IDENTITY)
}

impl DummyRig {
    /// Dummies of a character model, attached to the Havok bones of the same name.
    pub fn from_flver(model: &Flver, skeleton: &Skeleton) -> Self {
        let world = node_world(&model.nodes);
        let mut rig = DummyRig::default();
        for d in &model.dummies {
            let Some(attach) = usize::try_from(d.attach_bone).ok() else {
                continue;
            };
            let Some(node) = model.nodes.get(attach) else {
                continue;
            };
            let Some(bone) = skeleton.bone_index(&node.name) else {
                continue;
            };
            let frame = parent_world(&world, d) * dummy_matrix(d, false);
            let offset = world[attach].inverse() * frame;
            rig.dummies
                .entry(d.reference_id)
                .or_default()
                .push((bone, offset));
        }
        rig
    }

    /// Adds a held weapon's dummies: the weapon model sits at the character's dummy
    /// `holder_dummy` (blade along its forward), following that dummy's attach bone.
    pub fn add_weapon(
        &mut self,
        model: &Flver,
        skeleton: &Skeleton,
        weapon: &Flver,
        holder_dummy: i16,
    ) -> bool {
        let world = node_world(&model.nodes);
        let Some(h) = model
            .dummies
            .iter()
            .find(|d| d.reference_id == holder_dummy)
        else {
            return false;
        };
        let Some(attach) = usize::try_from(h.attach_bone).ok() else {
            return false;
        };
        let Some(bone) = model
            .nodes
            .get(attach)
            .and_then(|n| skeleton.bone_index(&n.name))
        else {
            return false;
        };
        let holder = world[attach].inverse() * parent_world(&world, h) * dummy_matrix(h, true);
        let wworld = node_world(&weapon.nodes);
        for d in &weapon.dummies {
            let frame = parent_world(&wworld, d) * dummy_matrix(d, false);
            self.dummies
                .entry(d.reference_id)
                .or_default()
                .push((bone, holder * frame));
        }
        true
    }

    /// World positions of every dummy with `id`, given source-space bone model matrices and the
    /// body placement (`place * mirror`).
    pub fn positions(&self, id: i16, model: &[Mat4], to_world: Mat4) -> Vec<Vec3> {
        self.dummies
            .get(&id)
            .into_iter()
            .flatten()
            .filter_map(|(b, off)| {
                model
                    .get(*b)
                    .map(|m| to_world.transform_point3((*m * *off).w_axis.truncate()))
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.dummies.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dummies.is_empty()
    }
}

/// One hit capsule of an attack: two dummy polys and a radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HitShape {
    pub dummy_a: i16,
    pub dummy_b: i16,
    pub radius: f32,
}

/// One behaviour judge: its AtkParam row (capsules plus the combat fields) and the BehaviorParam
/// fields the rules read. Guard actions are judges too (the TAE `ShieldBlock` flag names them).
#[derive(Debug, Clone, PartialEq)]
pub struct AttackData {
    pub atk_id: i32,
    pub shapes: Vec<HitShape>,
    /// `dmgLevel`: which hit reaction plays (1 small, 2 middle, 3 large, ...).
    pub damage_level: i32,
    pub atk_phys: i32,
    pub atk_stam: i32,
    /// `atk{Phys,Mag,Fire,Thun,Dark}Correction`: percent of the weapon attack power a player
    /// attack uses.
    pub atk_corrections: [i32; 5],
    /// The combat fields of the AtkParam row.
    pub profile: Option<AttackProfile>,
    /// BehaviorParam `category` (picks the attacker's recoil code family).
    pub category: u8,
    /// BehaviorParam `stamina`: for a guard judge, posture added to every guarded hit.
    pub behavior_stamina: i32,
}

impl AttackData {
    fn from_rows(row: &Row, behavior: &Row) -> Self {
        let mut me = Self::from_row(row);
        me.category = behavior.int("category").unwrap_or(0) as u8;
        me.behavior_stamina = behavior.int("stamina").unwrap_or(0) as i32;
        me
    }

    fn from_row(row: &Row) -> Self {
        let int = |f: &str| row.int(f).unwrap_or(0) as i32;
        let shapes = (0..4)
            .filter_map(|i| {
                let a = int(&format!("hit{i}_DmyPoly1"));
                let b = int(&format!("hit{i}_DmyPoly2"));
                let r = row.f32(&format!("hit{i}_Radius")).unwrap_or(0.0);
                (a > 0 && r > 0.0).then_some(HitShape {
                    dummy_a: a as i16,
                    dummy_b: if b > 0 { b as i16 } else { a as i16 },
                    radius: r,
                })
            })
            .collect();
        Self {
            atk_id: row.id(),
            shapes,
            damage_level: int("dmgLevel"),
            atk_phys: int("atkPhys"),
            atk_stam: int("atkStam"),
            atk_corrections: [
                int("atkPhysCorrection"),
                int("atkMagCorrection"),
                int("atkFireCorrection"),
                int("atkThunCorrection"),
                int("atkDarkCorrection"),
            ],
            profile: AttackProfile::from_row(row).ok(),
            category: 0,
            behavior_stamina: 0,
        }
    }
}

pub(crate) fn load_table(
    param_dir: &Path,
    defs_dir: &Path,
    file: &str,
    def: &str,
) -> Option<Table> {
    let raw = RawParam::parse(std::fs::read(param_dir.join(format!("{file}.param"))).ok()?).ok()?;
    let def = ParamDef::load(&defs_dir.join(format!("{def}.xml"))).ok()?;
    Table::new(file, &raw, Arc::new(def), None).ok()
}

/// What a character attacks with: its dummy rig, behaviour variation and body size.
#[derive(Debug, Clone, Default)]
pub struct Combatant {
    pub rig: DummyRig,
    /// `behaviorVariationId` of the NPC or of the player's weapon.
    pub variation: i32,
    /// Body capsule radius and height, metres.
    pub radius: f32,
    pub height: f32,
    /// Attacks by behaviour judge id.
    pub attacks: HashMap<i32, AttackData>,
    /// The row the combat rules read: EquipParamWeapon of the player's right-hand weapon, or
    /// the NPC's NpcParam.
    pub param_id: i32,
}

impl Combatant {
    /// The player with the starting katana (CharaInitParam 10010).
    pub fn player(cache: &Path, skeleton: &Skeleton) -> Self {
        let (pd, dd) = param_dirs(cache);
        let read = |p: &Path| std::fs::read(p).ok().and_then(|b| flver::parse(&b).ok());
        let mut c = Combatant {
            radius: 0.4,
            height: 1.7,
            ..Combatant::default()
        };
        let Some(model) = read(&cache.join("raw/chr/c0000.chrbnd.d/c0000.flver")) else {
            return c;
        };
        c.rig = DummyRig::from_flver(&model, skeleton);
        let init = load_table(&pd, &dd, "CharaInitParam", "CharaInitParam");
        let weapons = load_table(&pd, &dd, "EquipParamWeapon", "EquipParamWeapon");
        let wep = init
            .as_ref()
            .and_then(|t| t.find(10010))
            .and_then(|r| r.int("equip_Wep_Right").ok())
            .and_then(|id| weapons.as_ref()?.find(id as i32));
        if let Some(w) = wep {
            c.variation = w.int("behaviorVariationId").unwrap_or(0) as i32;
            c.param_id = w.id();
            let model_id = w.int("equipModelId").unwrap_or(0);
            let name = format!("WP_A_{model_id:04}");
            let path = cache.join(format!(
                "raw/parts/{}.partsbnd.d/{name}.flver",
                name.to_ascii_lowercase()
            ));
            if let Some(blade) = read(&path) {
                c.rig.add_weapon(&model, skeleton, &blade, 1);
            }
        }
        // Rows missing for the weapon's own variation fall back to its hundred (5001 -> 5000),
        // where the katana's ordinary attacks live (observed in the data, as in DS3).
        let base = c.variation / 100 * 100;
        c.attacks = load_attacks(
            &pd,
            &dd,
            "BehaviorParam_PC",
            "AtkParam_Pc",
            100_000_000,
            base,
        );
        if base != c.variation {
            c.attacks.extend(load_attacks(
                &pd,
                &dd,
                "BehaviorParam_PC",
                "AtkParam_Pc",
                100_000_000,
                c.variation,
            ));
        }
        c
    }

    /// An NPC by its NpcParam row (the soldier is 10100000).
    pub fn npc(cache: &Path, chr: &str, npc_param: i32, skeleton: &Skeleton) -> Self {
        let (pd, dd) = param_dirs(cache);
        let mut c = Combatant {
            radius: 0.4,
            height: 1.7,
            ..Combatant::default()
        };
        if let Some(model) =
            std::fs::read(cache.join(format!("raw/chr/{chr}.chrbnd.d/{chr}.flver")))
                .ok()
                .and_then(|b| flver::parse(&b).ok())
        {
            c.rig = DummyRig::from_flver(&model, skeleton);
        }
        if let Some(row) = load_table(&pd, &dd, "NpcParam", "NpcParam").and_then(|t| {
            t.find(npc_param).map(|r| {
                (
                    r.int("behaviorVariationId").unwrap_or(0) as i32,
                    r.f32("hitRadius").unwrap_or(0.4),
                    r.f32("hitHeight").unwrap_or(1.7),
                )
            })
        }) {
            (c.variation, c.radius, c.height) = row;
            c.param_id = npc_param;
        }
        c.attacks = load_attacks(
            &pd,
            &dd,
            "BehaviorParam",
            "AtkParam_Npc",
            200_000_000,
            c.variation,
        );
        c
    }
}

fn param_dirs(cache: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
    (
        cache.join("raw/param/gameparam/gameparam.parambnd.d"),
        cache.join("refs/paramdex/Defs"),
    )
}

fn load_attacks(
    pd: &Path,
    dd: &Path,
    behavior: &str,
    atk: &str,
    base: i32,
    variation: i32,
) -> HashMap<i32, AttackData> {
    let mut out = HashMap::new();
    let (Some(b), Some(a)) = (
        load_table(pd, dd, behavior, "BehaviorParam"),
        load_table(pd, dd, atk, "AtkParam"),
    ) else {
        return out;
    };
    let first = base + variation * 1000;
    for row in b.rows() {
        let id = row.id();
        if !(first..first + 1000).contains(&id) || row.int("refType").unwrap_or(-1) != 0 {
            continue;
        }
        if let Some(r) = a.find(row.int("refId").unwrap_or(-1) as i32) {
            out.insert(id - first, AttackData::from_rows(&r, &row));
        }
    }
    out
}

/// One contact between an attack and a defender.
#[derive(Debug, Clone, PartialEq)]
pub struct HitEvent {
    pub attacker: usize,
    pub defender: usize,
    pub judge: i32,
    pub attack: AttackData,
    /// Contact point, world space.
    pub position: [f32; 3],
    /// Horizontal direction from attacker to defender, unit length.
    pub direction: [f32; 2],
}

/// The outcome of a hit for both sides' scripts.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HitResult {
    pub defender: IncomingDamage,
    pub attacker: Option<IncomingDamage>,
    pub hp_damage: i32,
    /// Posture taken by the defender.
    pub posture_damage: i32,
    /// Posture sent back to the attacker (deflects and blocks).
    pub attacker_posture_damage: i32,
    pub deflected: bool,
    pub guarded: bool,
    /// The defender (direct hit) or the attacker (deflected or blocked) had its posture broken.
    pub posture_broke: bool,
}

/// ChrActionFlag type for "guarding" (`ShieldBlock`). Its ArgB is the behaviour judge of the
/// guard action (90 for Wolf's sword guard, 900 for the soldier's), whose AtkParam row is the
/// guard row the rules read.
pub const FLAG_GUARD: i32 = 3;

/// Closest distance between segments `p1-q1` and `p2-q2`.
pub fn segment_distance(p1: Vec3, q1: Vec3, p2: Vec3, q2: Vec3) -> f32 {
    let d1 = q1 - p1;
    let d2 = q2 - p2;
    let r = p1 - p2;
    let a = d1.dot(d1);
    let e = d2.dot(d2);
    let f = d2.dot(r);
    let (s, t);
    if a <= 1e-9 && e <= 1e-9 {
        return r.length();
    }
    if a <= 1e-9 {
        s = 0.0;
        t = (f / e).clamp(0.0, 1.0);
    } else {
        let c = d1.dot(r);
        if e <= 1e-9 {
            t = 0.0;
            s = (-c / a).clamp(0.0, 1.0);
        } else {
            let b = d1.dot(d2);
            let denom = a * e - b * b;
            let mut s0 = if denom > 1e-9 {
                ((b * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut t0 = (b * s0 + f) / e;
            if t0 < 0.0 {
                t0 = 0.0;
                s0 = (-c / a).clamp(0.0, 1.0);
            } else if t0 > 1.0 {
                t0 = 1.0;
                s0 = ((b - c) / a).clamp(0.0, 1.0);
            }
            s = s0;
            t = t0;
        }
    }
    ((p1 + d1 * s) - (p2 + d2 * t)).length()
}

/// Tracks attack windows across steps and finds contacts.
#[derive(Debug, Default)]
pub struct HitDetector {
    /// Previous step's capsule ends per (attacker, judge, shape index).
    last: HashMap<(usize, i32, usize), (Vec3, Vec3)>,
    /// (attacker, judge, window start, defender) already hit in the current window.
    done: HashSet<(usize, i32, u32, usize)>,
}

impl HitDetector {
    /// Checks every open attack window of every character against every other character.
    pub fn detect(&mut self, chars: &[&Character], fighters: &[&Combatant]) -> Vec<HitEvent> {
        let mut hits = Vec::new();
        let mut live_windows = HashSet::new();
        let mut live_shapes = HashSet::new();
        for (ai, attacker) in chars.iter().enumerate() {
            let windows: Vec<TimedEvent> = attacker
                .tae_frame()
                .attacks
                .iter()
                .filter(|e| matches!(e.kind, TaeKind::Attack { .. }))
                .cloned()
                .collect();
            if windows.is_empty() {
                continue;
            }
            let model = attacker.bone_source_matrices();
            let to_world = attacker.source_to_world();
            for w in windows {
                let TaeKind::Attack { judge, .. } = w.kind else {
                    continue;
                };
                let Some(attack) = fighters[ai].attacks.get(&judge) else {
                    continue;
                };
                let window_key = (ai, judge, w.start.to_bits());
                live_windows.insert(window_key);
                for (si, shape) in attack.shapes.iter().enumerate() {
                    let rig = &fighters[ai].rig;
                    let (Some(a), Some(b)) = (
                        rig.positions(shape.dummy_a, &model, to_world)
                            .first()
                            .copied(),
                        rig.positions(shape.dummy_b, &model, to_world)
                            .first()
                            .copied(),
                    ) else {
                        continue;
                    };
                    let key = (ai, judge, si);
                    live_shapes.insert(key);
                    let (pa, pb) = self.last.insert(key, (a, b)).unwrap_or((a, b));
                    for (di, defender) in chars.iter().enumerate() {
                        if di == ai || self.done.contains(&(ai, judge, w.start.to_bits(), di)) {
                            continue;
                        }
                        let f = fighters[di];
                        let base = Vec3::from_array(defender.body.position);
                        let axis = (
                            base + Vec3::Y * f.radius,
                            base + Vec3::Y * (f.height - f.radius).max(f.radius),
                        );
                        // Sweep: the blade now, before, and the two arcs its ends traced.
                        let d = segment_distance(a, b, axis.0, axis.1)
                            .min(segment_distance(pa, pb, axis.0, axis.1))
                            .min(segment_distance(pa, a, axis.0, axis.1))
                            .min(segment_distance(pb, b, axis.0, axis.1));
                        if d <= shape.radius + f.radius {
                            self.done.insert((ai, judge, w.start.to_bits(), di));
                            let dir = Vec3::new(
                                base.x - attacker.body.position[0],
                                0.0,
                                base.z - attacker.body.position[2],
                            )
                            .normalize_or(Vec3::Z);
                            let mid = (a + b) * 0.5;
                            hits.push(HitEvent {
                                attacker: ai,
                                defender: di,
                                judge,
                                attack: attack.clone(),
                                position: mid.to_array(),
                                direction: [dir.x, dir.z],
                            });
                        }
                    }
                }
            }
        }
        self.last.retain(|k, _| live_shapes.contains(k));
        self.done
            .retain(|(a, j, s, _)| live_windows.contains(&(*a, *j, *s)));
        hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_distances() {
        let d = segment_distance(
            Vec3::new(-1.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 0.5),
            Vec3::new(0.0, 2.0, 0.5),
        );
        assert!((d - 0.5).abs() < 1e-5);
        let parallel = segment_distance(Vec3::ZERO, Vec3::Y, Vec3::X, Vec3::X + Vec3::Y);
        assert!((parallel - 1.0).abs() < 1e-5);
    }
}

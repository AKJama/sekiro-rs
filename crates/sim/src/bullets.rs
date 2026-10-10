//! Bullets: thrown and shot attacks (the Owl's shuriken), as the data describes them.
//!
//! The data trail, per bullet:
//!
//! 1. A TAE `BulletBehavior` event (type 2) on the shooter's clip names a behaviour judge and
//!    the dummy poly it leaves from.
//! 2. `BehaviorParam` row `200000000 + behaviorVariationId * 1000 + judge` with `refType` 1
//!    names a `Bullet` param row.
//! 3. The Bullet row gives the speed (`initVellocity`), lifetime (`life`, seconds), hit radius,
//!    how many leave at once (`numShoot`) and their horizontal fan (`shootAngle`, then
//!    `shootAngleInterval` degrees per bullet), and `atkId_Bullet`, the AtkParam_Npc row its hit
//!    deals.
//!
//! Ours (not traced): bullets fly straight at constant speed; each leaves aimed at the target's
//! chest when the target is within `lockShootLimitAng` of the shooter's facing (the bullet
//! auto-aim FwLo's RustSekiro describes for Wolf's shuriken, `docs/kb/enemy.md`), else along the
//! facing; a bullet ends on its first character hit or when its life runs out. Bullets whose
//! attack row deals nothing (firecracker sparks, poison and smoke balls, the perilous-attack
//! warning dummies) are not flown: their effects come from child bullets, SpEffects and map
//! hits that are not modelled.

use std::collections::{HashMap, HashSet};

use glam::Vec3;

use crate::character::Character;
use crate::hits::{AttackData, Combatant, HitEvent, can_be_hit, segment_distance};
use crate::tae::TaeKind;

/// One Bullet param row, with the attack its hits deal.
#[derive(Debug, Clone, PartialEq)]
pub struct BulletDef {
    pub id: i32,
    pub attack: AttackData,
    /// Metres per second.
    pub speed: f32,
    /// Seconds.
    pub life: f32,
    pub hit_radius: f32,
    pub count: i32,
    /// Horizontal angle of the first bullet and the step between bullets, degrees.
    pub first_angle: f32,
    pub angle_step: f32,
    /// The bullet aims at the target when it is within this many degrees of the facing.
    pub aim_limit: f32,
}

impl BulletDef {
    /// Reads a Bullet row; `None` for bullets whose attack deals no HP or posture damage.
    pub fn from_row(row: &sekiro_formats::param::Row, attack: AttackData) -> Option<Self> {
        if attack.atk_phys <= 0 && attack.atk_stam <= 0 {
            return None;
        }
        let f = |name: &str| row.f32(name).unwrap_or(0.0);
        let i = |name: &str| row.int(name).unwrap_or(0) as i32;
        Some(Self {
            id: row.id(),
            attack,
            speed: f("initVellocity"),
            life: f("life"),
            hit_radius: f("hitRadius").max(0.05),
            count: i("numShoot").max(1),
            first_angle: i("shootAngle") as f32,
            angle_step: i("shootAngleInterval") as f32,
            aim_limit: i("lockShootLimitAng") as f32,
        })
    }
}

/// A bullet in flight.
#[derive(Debug, Clone, PartialEq)]
pub struct Bullet {
    pub shooter: usize,
    pub judge: i32,
    pub position: Vec3,
    pub velocity: Vec3,
    pub life: f32,
    pub radius: f32,
    attack: AttackData,
}

/// Every bullet in flight, and which bullet windows already fired.
#[derive(Debug, Default)]
pub struct Bullets {
    pub live: Vec<Bullet>,
    /// (shooter, judge, window start) of bullet events already fired.
    fired: HashSet<(usize, i32, u32)>,
}

/// World yaw of a direction (the simulation's convention, yaw 0 facing -Z).
fn yaw_of(dir: Vec3) -> f32 {
    (-dir.x).atan2(-dir.z)
}

fn dir_of(yaw: f32, pitch: f32) -> Vec3 {
    Vec3::new(
        -yaw.sin() * pitch.cos(),
        pitch.sin(),
        -yaw.cos() * pitch.cos(),
    )
}

fn angle_diff(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(std::f32::consts::TAU);
    if d > std::f32::consts::PI {
        d - std::f32::consts::TAU
    } else {
        d
    }
}

impl Bullets {
    /// Fires the bullets whose TAE window opened this step on any character, aimed at the
    /// other character.
    pub fn spawn(&mut self, chars: &[&Character], fighters: &[&Combatant]) {
        let mut live_windows = HashSet::new();
        for (si, shooter) in chars.iter().enumerate() {
            for e in &shooter.tae_frame().attacks {
                let TaeKind::Bullet { judge, dummy_poly } = e.kind else {
                    continue;
                };
                let key = (si, judge, e.start.to_bits());
                live_windows.insert(key);
                if !self.fired.insert(key) {
                    continue;
                }
                let Some(def) = fighters[si].bullets.get(&judge) else {
                    continue;
                };
                let model = shooter.bone_source_matrices();
                let to_world = shooter.source_to_world();
                let body = Vec3::from_array(shooter.body.position);
                let origin = i16::try_from(dummy_poly)
                    .ok()
                    .and_then(|d| {
                        fighters[si]
                            .rig
                            .positions(d, &model, to_world)
                            .first()
                            .copied()
                    })
                    .unwrap_or(body + Vec3::Y * 1.2);
                let facing = shooter.body.yaw;
                // Aim at the first other character's chest when it is in front.
                let target = chars
                    .iter()
                    .enumerate()
                    .find(|(ti, _)| *ti != si)
                    .map(|(ti, t)| {
                        Vec3::from_array(t.body.position) + Vec3::Y * fighters[ti].height * 0.6
                    });
                let (yaw, pitch) = match target {
                    Some(t) => {
                        let to = t - origin;
                        let yaw = yaw_of(to);
                        let flat = (to.x * to.x + to.z * to.z).sqrt();
                        if angle_diff(yaw, facing).abs().to_degrees() <= def.aim_limit {
                            (yaw, to.y.atan2(flat))
                        } else {
                            (facing, 0.0)
                        }
                    }
                    None => (facing, 0.0),
                };
                for k in 0..def.count {
                    let offset = (def.first_angle + def.angle_step * k as f32).to_radians();
                    // Positive shoot angles turn to the shooter's right (ours).
                    let dir = dir_of(yaw - offset, pitch);
                    self.live.push(Bullet {
                        shooter: si,
                        judge,
                        position: origin,
                        velocity: dir * def.speed,
                        life: def.life,
                        radius: def.hit_radius,
                        attack: def.attack.clone(),
                    });
                }
            }
        }
        self.fired.retain(|k| live_windows.contains(k));
    }

    /// Moves every bullet by `dt` seconds and returns the hits on characters other than each
    /// bullet's shooter.
    pub fn advance(
        &mut self,
        dt: f32,
        chars: &[&Character],
        fighters: &[&Combatant],
    ) -> Vec<HitEvent> {
        let mut hits = Vec::new();
        self.live.retain_mut(|b| {
            let from = b.position;
            let to = from + b.velocity * dt;
            b.position = to;
            b.life -= dt;
            for (di, defender) in chars.iter().enumerate() {
                if di == b.shooter || !can_be_hit(defender.tae_frame()) {
                    continue;
                }
                let f = fighters[di];
                let base = Vec3::from_array(defender.body.position);
                let axis = (
                    base + Vec3::Y * f.radius,
                    base + Vec3::Y * (f.height - f.radius).max(f.radius),
                );
                if segment_distance(from, to, axis.0, axis.1) <= b.radius + f.radius {
                    let flat = Vec3::new(b.velocity.x, 0.0, b.velocity.z).normalize_or(Vec3::Z);
                    hits.push(HitEvent {
                        attacker: b.shooter,
                        defender: di,
                        judge: b.judge,
                        attack: b.attack.clone(),
                        position: to.to_array(),
                        direction: [flat.x, flat.z],
                    });
                    return false;
                }
            }
            b.life > 0.0
        });
        hits
    }

    pub fn clear(&mut self) {
        self.live.clear();
        self.fired.clear();
    }
}

/// Bullet definitions of an NPC's behaviour variation by judge id.
pub fn load_bullets(
    behaviors: &sekiro_formats::param::Table,
    bullets: &sekiro_formats::param::Table,
    attacks: &sekiro_formats::param::Table,
    base: i32,
    variation: i32,
) -> HashMap<i32, BulletDef> {
    let mut out = HashMap::new();
    let first = base + variation * 1000;
    for row in behaviors.rows() {
        let id = row.id();
        if !(first..first + 1000).contains(&id) || row.int("refType").unwrap_or(-1) != 1 {
            continue;
        }
        let Some(bullet) = bullets.find(row.int("refId").unwrap_or(-1) as i32) else {
            continue;
        };
        let Some(atk) = attacks.find(bullet.int("atkId_Bullet").unwrap_or(-1) as i32) else {
            continue;
        };
        if let Some(def) = BulletDef::from_row(&bullet, AttackData::from_rows(&atk, &row)) {
            out.insert(id - first, def);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directions_follow_the_yaw_convention() {
        // Yaw 0 faces -Z; yaw of that direction is 0 again.
        let d = dir_of(0.0, 0.0);
        assert!((d - Vec3::NEG_Z).length() < 1e-6);
        assert!((yaw_of(dir_of(1.0, 0.0)) - 1.0).abs() < 1e-5);
        assert!((angle_diff(0.1, std::f32::consts::TAU - 0.1) - 0.2).abs() < 1e-5);
    }
}

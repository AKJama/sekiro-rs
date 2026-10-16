//! Authored wire points and solved wire flight, using FwLo's MIT engine research.
use crate::{
    body::{Body, Ground},
    collision::CollisionWorld,
    input::angle_diff,
};
use glam::Vec3;
use serde::Deserialize;
use std::{collections::HashMap, path::Path};

#[derive(Debug, Clone, Deserialize)]
pub struct Point {
    pub position: Option<[f32; 3]>,
    pub variation: u32,
    pub model: String,
    pub source_part: String,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Search {
    pub action_distance_near: f32,
    pub action_distance_far: f32,
    pub find_distance_far: f32,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Variation {
    pub id: u32,
    pub target_marker_type: u8,
    pub extend_wire_action_distance: f32,
    pub recognition_angle_up: f32,
    pub recognition_angle_down: f32,
    pub recognition_angle_left: f32,
    pub recognition_angle_right: f32,
    pub chr_physics_homing_id_offset: u32,
}
#[derive(Debug, Clone)]
pub struct Target {
    pub index: usize,
    pub point: Vec3,
    pub arrival: Vec3,
    pub rideable: bool,
}
#[derive(Default)]
pub struct Wire {
    /// Camera downward pitch, radians; headless callers use the body's horizontal view.
    pub view_pitch: f32,
    pub points: Vec<Point>,
    pub search: Option<Search>,
    pub variations: HashMap<u32, Variation>,
    pub offsets: HashMap<u32, Vec3>,
    pub collision: Option<CollisionWorld>,
    pub candidate: Option<Target>,
    pub active: Option<Target>,
    start: Option<Vec3>,
    elapsed: f32,
    duration: f32,
    arrived: bool,
    clip: String,
    pub tip: Option<Vec3>,
    pub rope_dummy: Option<i16>,
    /// Virtual support for authored rideable ornaments absent from terrain collision.
    pub perch: Option<Vec3>,
}
impl Wire {
    pub fn flight_time(&self) -> Option<(f32, f32)> {
        (self.start.is_some() || self.arrived).then_some((self.elapsed, self.duration))
    }

    pub fn load(cache: &Path, map: &Path) -> Result<Self, String> {
        let read = |p: &Path| -> Result<serde_json::Value, String> {
            serde_json::from_slice(&std::fs::read(p).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())
        };
        let rows = |name: &str| -> Result<Vec<serde_json::Value>, String> {
            read(&cache.join(format!("params/{name}.json")))?["rows"]
                .as_array()
                .cloned()
                .ok_or_else(|| format!("missing {name} rows"))
        };
        let search = rows("WirePointSearchParam")?
            .into_iter()
            .find(|r| r["id"] == 0)
            .ok_or("missing wire search row 0")?;
        let variations: Vec<Variation> = rows("WireVariationParam")?
            .into_iter()
            .map(serde_json::from_value)
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;
        let mut offsets = HashMap::new();
        for row in rows("ChrPhysicsHomingParam")? {
            let id = row["id"].as_u64().unwrap_or(0) as u32;
            offsets.insert(
                id,
                Vec3::new(
                    row["targetOffsetX"].as_f64().unwrap_or(0.0) as f32,
                    row["targetOffsetY"].as_f64().unwrap_or(0.0) as f32,
                    row["targetOffsetZ"].as_f64().unwrap_or(0.0) as f32,
                ),
            );
        }
        Ok(Self {
            points: serde_json::from_value(read(&map.join("grapple.json"))?)
                .map_err(|e| e.to_string())?,
            search: Some(serde_json::from_value(search).map_err(|e| e.to_string())?),
            variations: variations.into_iter().map(|v| (v.id, v)).collect(),
            offsets,
            // The player's collision: helper parts the player passes through do not stop the
            // line of sight or the flight (`crate::hit_filter`).
            collision: Some(CollisionWorld::load_map(map)?),
            ..Self::default()
        })
    }
    pub fn search(&mut self, body: &Body, camera_yaw: f32) {
        self.candidate = None;
        let Some(s) = &self.search else { return };
        let origin = Vec3::from(body.position);
        let mut best = (f32::INFINITY, f32::INFINITY);
        for (index, p) in self.points.iter().enumerate() {
            if self.active.as_ref().is_some_and(|t| t.index == index) {
                continue;
            }
            let Some(v) = self.variations.get(&p.variation) else {
                continue;
            };
            let Some(position) = p.position else { continue };
            let point = Vec3::from(position);
            let delta = point - origin;
            let distance = delta.length();
            if distance < s.action_distance_near
                || distance > s.find_distance_far
                || distance > s.action_distance_far + v.extend_wire_action_distance
            {
                continue;
            }
            let yaw = (-delta.x).atan2(-delta.z);
            let horizontal = angle_diff(yaw, camera_yaw).to_degrees();
            let vertical = (delta.y.atan2(delta.x.hypot(delta.z)) + self.view_pitch).to_degrees();
            if horizontal > v.recognition_angle_left
                || -horizontal > v.recognition_angle_right
                || vertical > v.recognition_angle_up
                || -vertical > v.recognition_angle_down
            {
                continue;
            }
            let chest = origin + Vec3::Y * 1.2;
            let ray = point - chest;
            if self.collision.as_ref().is_some_and(|w| {
                w.raycast(chest, ray.normalize(), (ray.length() - 0.1).max(0.0))
                    .is_some()
            }) {
                continue;
            }
            // Placeholder: prefer view-centre angle, then distance; screen boxes are not reconstructed.
            let score = (horizontal.hypot(vertical), distance);
            if score >= best {
                continue;
            }
            best = score;
            let offset = self
                .offsets
                .get(&(300000000 + v.chr_physics_homing_id_offset))
                .copied()
                .unwrap_or_default();
            let forward = Vec3::new(delta.x, 0.0, delta.z).normalize_or_zero();
            let right = Vec3::new(-forward.z, 0.0, forward.x);
            self.candidate = Some(Target {
                index,
                point,
                arrival: point + right * offset.x + Vec3::Y * offset.y + forward * offset.z,
                rideable: v.target_marker_type == 0,
            });
        }
    }
    pub fn step(
        &mut self,
        body: &mut Body,
        animation: Option<&str>,
        travel_window: Option<(f32, f32)>,
        dt: f32,
        ground: &dyn Ground,
    ) -> bool {
        self.advance(body, animation, 0.0, None, travel_window, dt, ground)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn advance(
        &mut self,
        body: &mut Body,
        animation: Option<&str>,
        time: f32,
        tae: Option<&crate::tae::TaeAnim>,
        travel_window: Option<(f32, f32)>,
        dt: f32,
        ground: &dyn Ground,
    ) -> bool {
        use crate::tae::TaeKind;
        let shot = animation.is_some_and(|a| {
            matches!(
                a,
                "a000_202000" | "a000_202100" | "a000_202600" | "a000_202700"
            )
        });
        let moving = animation.is_some_and(|a| {
            matches!(
                a,
                "a000_202010"
                    | "a000_202011"
                    | "a000_202012"
                    | "a000_202110"
                    | "a000_202112"
                    | "a000_202610"
                    | "a000_202710"
                    | "a000_202020"
                    | "a000_201030"
            )
        });
        let changed = self.clip != animation.unwrap_or_default();
        self.clip = animation.unwrap_or_default().to_owned();
        if shot && changed {
            self.active = self.candidate.clone();
            self.arrived = false;
            self.start = None;
            self.elapsed = 0.0;
            self.duration = 0.0;
            self.tip = Some(Vec3::from(body.position) + Vec3::Y);
        }
        if !shot && !moving {
            self.active = None;
            self.start = None;
            self.tip = None;
            self.rope_dummy = None;
            return false;
        }
        let Some(target) = self.active.clone() else {
            return false;
        };
        let event = |kind| {
            tae.and_then(|a| {
                a.events.iter().find(|e| {
                    e.start <= time
                        && time < e.end
                        && matches!(e.kind, TaeKind::Wire { kind: k, .. } if k == kind)
                })
            })
        };
        self.rope_dummy = event(715).and_then(|e| {
            let TaeKind::Wire { args, .. } = &e.kind else {
                return None;
            };
            let model = i32::from_le_bytes(args.get(..4)?.try_into().ok()?);
            if model != 3 {
                return None;
            }
            Some(i16::from_le_bytes(args.get(4..6)?.try_into().ok()?))
        });
        if event(901).is_some() {
            self.tip = Some(target.point);
        } else if let Some(e) = event(900) {
            let TaeKind::Wire { args, .. } = &e.kind else {
                unreachable!()
            };
            let speed = args
                .get(..4)
                .and_then(|b| b.try_into().ok())
                .map(f32::from_le_bytes)
                .unwrap_or(50.0);
            let tip = self.tip.get_or_insert(Vec3::from(body.position) + Vec3::Y);
            let d = target.point - *tip;
            *tip += d.normalize_or_zero() * (speed * dt).min(d.length());
        }
        if event(902).is_some() {
            let d = target.point - Vec3::from(body.position);
            let yaw = (-d.x).atan2(-d.z);
            let speed: f32 = if matches!(animation, Some("a000_202600" | "a000_202700")) {
                2700.0
            } else {
                1080.0
            };
            body.yaw +=
                angle_diff(yaw, body.yaw).clamp(-speed.to_radians() * dt, speed.to_radians() * dt);
        }
        if self.arrived {
            return body.grounded && target.rideable;
        }
        body.fall_type = 2;
        if shot {
            if event(905).is_some() || animation == Some("a000_202000") {
                body.velocity = [0.0; 3];
                return true;
            }
            return false;
        }
        let launch = tae
            .and_then(|a| {
                a.events
                    .iter()
                    .find(|e| matches!(e.kind, TaeKind::VelocityChange(1100)))
            })
            .map(|e| e.start);
        if self.start.is_none() && launch.is_some_and(|t| time < t) {
            // FwLo traced row 1101 and Linear/EaseIn curve ids; the latter's shape is a fit.
            if let Some(e) = event(904) {
                let blend = ((time - e.start) / (e.end - e.start)).clamp(0.0, 1.0);
                let d = target.point - Vec3::from(body.position);
                let horizontal = Vec3::new(d.x, 0.0, d.z).normalize_or_zero();
                body.velocity =
                    (horizontal * 30.0 * blend + Vec3::Y * 30.0 * blend * blend).to_array();
                let current = Vec3::from(body.position);
                let delta = Vec3::from(body.velocity) * dt;
                let hit = self.sweep(current, delta);
                body.position = (current
                    + hit.map_or(delta, |d| delta.normalize_or_zero() * (d - 0.05).max(0.0)))
                .to_array();
                if hit.is_some() {
                    self.release();
                    body.grounded = false;
                }
            }
            return true;
        }
        self.start.get_or_insert_with(|| {
            self.elapsed = 0.0;
            let authored_arrival = tae
                .and_then(|a| {
                    a.events
                        .iter()
                        .find(|e| matches!(e.kind, TaeKind::CameraSlot(19)))
                })
                .zip(launch)
                .map(|(e, start)| e.start - start);
            self.duration = authored_arrival
                .unwrap_or_else(|| {
                    if tae.is_some() {
                        if matches!(animation, Some("a000_202011" | "a000_202012")) {
                            0.83
                        } else {
                            0.67
                        }
                    } else {
                        travel_window.map_or(0.83, |(a, b)| b - a)
                    }
                })
                .max(dt);
            let d = target.arrival - Vec3::from(body.position);
            body.velocity = (d / self.duration + Vec3::Y * (9.8 * self.duration)).to_array();
            Vec3::from(body.position)
        });
        let remaining = (self.duration - self.elapsed).max(0.0);
        let step = if remaining <= dt + 1e-6 {
            remaining
        } else {
            dt
        };
        let current = Vec3::from(body.position);
        let mut velocity = Vec3::from(body.velocity);
        if remaining > 0.0 {
            velocity.x = (target.arrival.x - current.x) / remaining;
            velocity.z = (target.arrival.z - current.z) / remaining;
        }
        let next = current + velocity * step - Vec3::Y * (9.8 * step * step);
        velocity.y -= 19.6 * step;
        body.velocity = velocity.to_array();
        self.elapsed += step;
        let delta = next - current;
        // Sweep feet and capsule centres, stopping before any collision rather than tunnelling.
        let hit = self.sweep(current, delta);
        if let Some(hit) = hit {
            body.position =
                (current + delta.normalize_or_zero() * (hit - 0.05).max(0.0)).to_array();
            self.release();
            body.grounded = false;
            return true;
        }
        body.position = next.to_array();
        body.grounded = false;
        body.fall_top = next.y;
        if self.elapsed >= self.duration {
            self.arrived = true;
            self.start = None;
            body.grounded = target.rideable;
            if body.grounded {
                self.perch = Some(Vec3::new(
                    next.x,
                    ground
                        .floor(next.x, next.z, next.y + 0.1)
                        .filter(|y| (next.y - y).abs() <= 0.6)
                        .unwrap_or(next.y),
                    next.z,
                ));
                body.velocity = [0.0; 3];
            }
            body.last_fall_height = 0.0;
        }
        true
    }

    fn release(&mut self) {
        self.active = None;
        self.start = None;
        self.tip = None;
        self.rope_dummy = None;
    }

    fn sweep(&self, current: Vec3, delta: Vec3) -> Option<f32> {
        self.collision.as_ref().and_then(|w| {
            [0.05, 0.4, 1.2, 1.55]
                .into_iter()
                .filter_map(|h| {
                    w.raycast(
                        current + Vec3::Y * h,
                        delta.normalize_or_zero(),
                        delta.length() + 0.001,
                    )
                })
                .min_by(f32::total_cmp)
        })
    }
}

/// The ridden landing spot supplies floor only inside its 0.6 m footprint.
pub struct PerchGround<'a> {
    pub terrain: &'a dyn Ground,
    pub perch: Option<Vec3>,
}
impl Ground for PerchGround<'_> {
    fn height(&self, x: f32, z: f32) -> Option<f32> {
        self.floor(x, z, f32::INFINITY)
    }
    fn floor(&self, x: f32, z: f32, top: f32) -> Option<f32> {
        let terrain = self.terrain.floor(x, z, top);
        let perch = self
            .perch
            .filter(|p| (p.x - x).hypot(p.z - z) <= 0.6 && p.y <= top)
            .map(|p| p.y);
        match (terrain, perch) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        }
    }
    fn slide(&self, feet: [f32; 3], delta: [f32; 2]) -> [f32; 2] {
        self.terrain.slide(feet, delta)
    }
    fn slide_assisted(&self, feet: [f32; 3], delta: [f32; 2], walls: u8) -> [f32; 2] {
        self.terrain.slide_assisted(feet, delta, walls)
    }
    fn set_player_effect(&self, effect: i32, on: bool) {
        self.terrain.set_player_effect(effect, on)
    }
    fn max_step(&self) -> f32 {
        self.terrain.max_step()
    }
    fn material(&self, x: f32, z: f32, top: f32) -> Option<u32> {
        self.terrain.material(x, z, top)
    }
}

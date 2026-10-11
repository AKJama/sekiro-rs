//! Authored wire points and the first ground-shot traversal slice.
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
}
impl Wire {
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
            collision: Some(CollisionWorld::load(&map.join("collision.bin"))?),
            ..Self::default()
        })
    }
    pub fn search(&mut self, body: &Body, camera_yaw: f32) {
        self.candidate = None;
        if self.active.is_some() || !body.grounded {
            return;
        }
        let Some(s) = &self.search else { return };
        let origin = Vec3::from(body.position);
        let mut best = (f32::INFINITY, f32::INFINITY);
        for (index, p) in self.points.iter().enumerate() {
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
        let shot = animation == Some("a000_202000");
        let moving = animation.is_some_and(|a| {
            matches!(
                a,
                "a000_202010" | "a000_202011" | "a000_202012" | "a000_201030"
            )
        });
        if shot && self.active.is_none() {
            self.active = self.candidate.clone();
            self.arrived = false;
        }
        if !shot && !moving {
            self.active = None;
            self.start = None;
            return false;
        }
        let Some(target) = self.active.clone() else {
            return false;
        };
        if self.arrived {
            return target.rideable;
        }
        body.velocity = [0.0; 3];
        body.fall_type = 2;
        if shot {
            return true;
        }
        let start = *self.start.get_or_insert_with(|| {
            self.elapsed = 0.0;
            self.duration = travel_window
                .map_or_else(
                    || Vec3::from(body.position).distance(target.arrival) / 20.0,
                    |(a, b)| b - a,
                )
                .max(dt);
            Vec3::from(body.position)
        });
        self.elapsed += dt;
        let next = start.lerp(target.arrival, (self.elapsed / self.duration).min(1.0));
        let current = Vec3::from(body.position);
        let delta = next - current;
        // Sweep feet and capsule centres, stopping before any collision rather than tunnelling.
        let hit = self.collision.as_ref().and_then(|w| {
            [0.05, 0.4, 1.2, 1.55]
                .into_iter()
                .filter_map(|h| {
                    w.raycast(
                        current + Vec3::Y * h,
                        delta.normalize_or_zero(),
                        delta.length(),
                    )
                })
                .min_by(f32::total_cmp)
        });
        if let Some(hit) = hit {
            body.position =
                (current + delta.normalize_or_zero() * (hit - 0.05).max(0.0)).to_array();
            self.active = None;
            self.start = None;
            body.grounded = false;
            return true;
        }
        body.position = next.to_array();
        body.grounded = false;
        body.fall_top = next.y;
        if self.elapsed >= self.duration {
            self.arrived = true;
            self.start = None;
            body.grounded = target.rideable
                && ground
                    .floor(next.x, next.z, next.y + 0.1)
                    .is_some_and(|y| next.y - y <= 0.5);
            body.last_fall_height = 0.0;
        }
        true
    }
}

//! Collision-backed action-edge selection and body docking.
use crate::body::{Body, Ground};
use crate::input::angle_diff;
use glam::Vec3;
use serde::Deserialize;
use std::path::Path;

// Placeholder, not traced: the exe owns these search and placement tunables.
pub const GROUND_REACH: f32 = 1.0;
pub const TOP_TOLERANCE: f32 = 0.4;
pub const HAND_HEIGHT: f32 = 1.5;
pub const FACING_LIMIT: f32 = 45.0;
pub const HANG_DEPTH: f32 = 1.5;
pub const HANG_OUTWARD: f32 = 0.35;
pub const CLIMB_INWARD: f32 = 0.25;
pub const FLOOR_TOLERANCE: f32 = 0.3;
pub const END_MARGIN: f32 = 0.2;
pub const OUTWARD_PROBE: f32 = 0.6;
pub const MIN_DROP: f32 = 0.5;
pub const SIDE_TOLERANCE: f32 = 0.1;
pub const CLIMB_FALLBACK_DURATION: f32 = 0.833;

#[derive(Debug, Clone, Deserialize)]
pub struct MapEdge {
    pub kind: u8,
    pub variation: u8,
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub source_part: String,
    pub chain: u32,
}

impl MapEdge {
    pub fn closest(&self, position: Vec3) -> (Vec3, f32) {
        let a = Vec3::from(self.start);
        let delta = Vec3::from(self.end) - a;
        let horizontal = Vec3::new(delta.x, 0.0, delta.z);
        let t = if horizontal.length_squared() > 1e-8 {
            ((position - a).dot(horizontal) / horizontal.length_squared()).clamp(0.0, 1.0)
        } else {
            0.0
        };
        (a + delta * t, t)
    }
}

#[derive(Debug, Clone)]
pub struct Target {
    pub edge: usize,
    pub point: Vec3,
    pub outward: Vec3,
    pub floor: Vec3,
    pub end: u8,
    pub angle: f32,
}

impl Target {
    pub fn hang_position(&self) -> Vec3 {
        self.point + self.outward * HANG_OUTWARD - Vec3::Y * HANG_DEPTH
    }
    pub fn yaw(&self) -> f32 {
        self.outward.x.atan2(self.outward.z)
    }
}

#[derive(Debug, Default)]
pub struct Docking {
    pub edges: Vec<MapEdge>,
    pub candidate: Option<Target>,
    pub active: Option<Target>,
    climb_start: Option<Vec3>,
}

impl Docking {
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let edges = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        Ok(Self {
            edges,
            ..Self::default()
        })
    }

    /// Infer the outward side from an upper floor on one side and a drop on the other.
    pub fn target(&self, index: usize, position: Vec3, ground: &dyn Ground) -> Option<Target> {
        let edge = self.edges.get(index)?;
        if edge.kind != 2 || edge.variation != 0 {
            return None;
        }
        let (point, t) = edge.closest(position);
        let d = Vec3::from(edge.end) - Vec3::from(edge.start);
        let side = Vec3::new(-d.z, 0.0, d.x).try_normalize()?;
        for outward in [side, -side] {
            let top = point - outward * CLIMB_INWARD;
            let Some(y) = ground.floor(top.x, top.z, point.y + FLOOR_TOLERANCE) else {
                continue;
            };
            if (y - point.y).abs() > FLOOR_TOLERANCE {
                continue;
            }
            let outside = point + outward * OUTWARD_PROBE;
            if ground
                .floor(outside.x, outside.z, point.y + FLOOR_TOLERANCE)
                .is_some_and(|h| h >= point.y - MIN_DROP)
            {
                continue;
            }
            let length = Vec3::from(edge.end).distance(Vec3::from(edge.start));
            return Some(Target {
                edge: index,
                point,
                outward,
                floor: Vec3::new(top.x, y, top.z),
                end: if t * length < END_MARGIN {
                    1
                } else if (1.0 - t) * length < END_MARGIN {
                    2
                } else {
                    0
                },
                angle: 0.0,
            });
        }
        None
    }

    pub fn search(&mut self, body: &Body, ground: &dyn Ground) {
        self.candidate = None;
        if !body.grounded || self.active.is_some() {
            return;
        }
        let p = Vec3::from(body.position);
        let mut nearest = GROUND_REACH;
        for i in 0..self.edges.len() {
            let edge = &self.edges[i];
            if edge.kind != 2 || edge.variation != 0 {
                continue;
            }
            let (point, _) = edge.closest(p);
            let offset = p - point;
            if offset.x.hypot(offset.z) > nearest
                || (offset.y.abs() > TOP_TOLERANCE
                    && (offset.y + HAND_HEIGHT).abs() > TOP_TOLERANCE)
            {
                continue;
            }
            let Some(mut target) = self.target(i, p, ground) else {
                continue;
            };
            let offset = p - target.point;
            let distance = Vec3::new(offset.x, 0.0, offset.z).length();
            let from_top =
                offset.y.abs() <= TOP_TOLERANCE && offset.dot(target.outward) <= SIDE_TOLERANCE;
            let from_below = (target.point.y - (p.y + HAND_HEIGHT)).abs() <= TOP_TOLERANCE
                && offset.dot(target.outward) >= 0.0;
            if distance > nearest || !(from_top || from_below) {
                continue;
            }
            // yaw is measured around +Y, forward is (-sin yaw, -cos yaw).
            let desired = if from_top {
                (-target.outward.x).atan2(-target.outward.z)
            } else {
                target.yaw()
            };
            if angle_diff(body.yaw, desired).abs().to_degrees() > FACING_LIMIT {
                continue;
            }
            target.angle = angle_diff(target.yaw(), body.yaw).to_degrees();
            nearest = distance;
            self.candidate = Some(target);
        }
    }

    /// Returns true while docking owns movement, bypassing gravity and collision pushes.
    pub fn step(
        &mut self,
        body: &mut Body,
        animation: Option<&str>,
        time: f32,
        root_progress: Option<f32>,
    ) -> bool {
        let name = animation.unwrap_or("");
        if self.active.is_none()
            && name.starts_with("a000_217")
            && !matches!(name, "a000_217500" | "a000_217520")
        {
            self.active = self.candidate.clone();
        }
        let Some(target) = &self.active else {
            return false;
        };
        if name == "a000_217520" {
            body.grounded = false;
            body.velocity = [0.0; 3];
            body.fall_top = body.position[1];
            self.active = None;
            self.climb_start = None;
            return false;
        }
        let position = if name == "a000_217500" {
            let start = *self.climb_start.get_or_insert(Vec3::from(body.position));
            // Precomputed climb displacement is fitted to the collision floor; progress
            // follows the clip's root track, with a time fallback for missing tracks.
            start.lerp(
                target.floor,
                root_progress
                    .unwrap_or(time / CLIMB_FALLBACK_DURATION)
                    .clamp(0.0, 1.0),
            )
        } else if self.climb_start.is_some() {
            let floor = target.floor;
            body.position = floor.to_array();
            body.grounded = true;
            body.velocity = [0.0; 3];
            body.last_fall_height = 0.0;
            self.active = None;
            self.climb_start = None;
            return true;
        } else if name.starts_with("a000_217") || name.starts_with("a000_020") {
            target.hang_position()
        } else {
            self.active = None;
            return false;
        };
        body.position = position.to_array();
        body.yaw = target.yaw();
        body.velocity = [0.0; 3];
        body.grounded = false;
        body.fall_top = position.y;
        true
    }
}

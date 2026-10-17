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
// Placeholder edge-frame interpretation of DockingTranslationOffset_Cover.
// FwLo's MIT docking notes report (0, 0, 0.7); the native frame sign is unknown.
pub const DOCKING_TRANSLATION_OFFSET_COVER: f32 = 0.7;
pub const COVER_HEIGHT_TOLERANCE: f32 = 0.8;
pub const COVER_WALL_PROBE: f32 = 1.0;
pub const COVER_PROBE_LIFT: f32 = 0.6;
pub const COVER_BLOCK_TOLERANCE: f32 = 0.05;
pub const COVER_AWAY_ANGLE: f32 = 30.0;
/// Air hang (env 3045): how far out from the face a fall or jump catches the lip, short range
/// (type 1) and long range (type 2), and the band of lip heights over the feet. Not traced (the
/// exe's search); Fireguy32's RustSekiro values.
pub const AIR_HANG_SHORT: f32 = 1.0;
pub const AIR_HANG_LONG: f32 = 2.0;
pub const AIR_HANG_BAND: (f32, f32) = (1.0, 2.6);
/// c0000_define `AIR_HANG_TYPE_SHORT_LANGE` and `AIR_HANG_TYPE_LONG_LANGE`.
pub const AIR_HANG_TYPE_SHORT: u8 = 1;
pub const AIR_HANG_TYPE_LONG: u8 = 2;
/// The climb-up and drop clips: `HangToStand` / `HangFootHoldToStand` and `HangToFreeFall` /
/// `HangFootHoldToFreeFall` (variation 0 / 1).
const CLIMB_CLIPS: [&str; 2] = ["a000_217500", "a000_217600"];
const DROP_CLIPS: [&str; 2] = ["a000_217520", "a000_217620"];

/// Hang edges: kind 2, variation 0 (`HANG_EDGE_TYPE_DISABLE_HOLD_FOOT`) or 1 (`ENABLE_HOLD_FOOT`).
fn is_hang(edge: &MapEdge) -> bool {
    edge.kind == 2 && edge.variation <= 1
}

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
    /// env 3045 for the airborne candidate: 0 none, [`AIR_HANG_TYPE_SHORT`] or [`AIR_HANG_TYPE_LONG`].
    pub air_hang_type: u8,
    climb_start: Option<Vec3>,
}

impl Docking {
    pub fn is_cover(&self, target: &Target) -> bool {
        self.edges[target.edge].kind == 3
    }

    fn cover_target(&self, index: usize, position: Vec3, ground: &dyn Ground) -> Option<Target> {
        let edge = &self.edges[index];
        if edge.kind != 3 || !matches!(edge.variation, 0..=3 | 10..=13) {
            return None;
        }
        let (point, t) = edge.closest(position);
        let d = Vec3::from(edge.end) - Vec3::from(edge.start);
        let side = Vec3::new(-d.z, 0.0, d.x).try_normalize()?;
        for outward in [side, -side] {
            if (position - point).dot(outward) < 0.0 {
                continue;
            }
            let feet = point + outward * DOCKING_TRANSLATION_OFFSET_COVER;
            let y = ground.floor(feet.x, feet.z, position.y + TOP_TOLERANCE)?;
            if (point.y - y).abs() > COVER_HEIGHT_TOLERANCE
                || (position.y - y).abs() > TOP_TOLERANCE
            {
                continue;
            }
            let clearance = ground.slide([feet.x, y, feet.z], [0.0; 2]);
            if clearance[0].hypot(clearance[1]) > COVER_BLOCK_TOLERANCE {
                continue;
            }
            let delta = -outward * COVER_WALL_PROBE;
            // Reject kerbs and floor contacts: cover needs a wall above the step zone.
            let slid = ground.slide([feet.x, y + COVER_PROBE_LIFT, feet.z], [delta.x, delta.z]);
            if Vec3::new(slid[0] - delta.x, 0.0, slid[1] - delta.z).length() < COVER_BLOCK_TOLERANCE
            {
                continue;
            }
            let length = Vec3::new(d.x, 0.0, d.z).length();
            // Endpoint labels are relative to Wolf facing away from the wall.
            let start_is_left = d.dot(Vec3::new(outward.z, 0.0, -outward.x)) < 0.0;
            let end = if t * length < END_MARGIN {
                if start_is_left { 1 } else { 2 }
            } else if (1.0 - t) * length < END_MARGIN {
                if start_is_left { 2 } else { 1 }
            } else {
                0
            };
            return Some(Target {
                edge: index,
                point,
                outward,
                floor: Vec3::new(feet.x, y, feet.z),
                end,
                angle: 0.0,
            });
        }
        None
    }

    pub fn cover_blocked(&self, target: &Target, ground: &dyn Ground, left: bool) -> bool {
        let direction =
            Vec3::new(target.outward.z, 0.0, -target.outward.x) * if left { 1.0 } else { -1.0 };
        let delta = direction * END_MARGIN;
        let slid = ground.slide(target.floor.to_array(), [delta.x, delta.z]);
        Vec3::new(slid[0] - delta.x, 0.0, slid[1] - delta.z).length() > COVER_BLOCK_TOLERANCE
    }

    /// Cover displacement uses the authored movement clip's root distance.
    pub fn cover_step(
        &mut self,
        body: &mut Body,
        animation: Option<&str>,
        stick_yaw: Option<f32>,
        root_distance: f32,
        ground: &dyn Ground,
    ) -> bool {
        let name = animation.unwrap_or("");
        let entry = name.starts_with("a000_21810") || matches!(name, "a000_218111" | "a000_218112");
        if self.active.is_none() && entry {
            self.active = self.candidate.clone().filter(|t| self.is_cover(t));
        }
        let Some(target) = self.active.as_ref().filter(|t| self.is_cover(t)) else {
            return false;
        };
        if matches!(name, "a000_218490" | "a000_218491")
            || !(entry
                || name.starts_with("a000_021")
                || name.starts_with("a000_022")
                || matches!(
                    name,
                    "a000_205010"
                        | "a000_205011"
                        | "a000_205020"
                        | "a000_205021"
                        | "a000_205030"
                        | "a000_205031"
                ))
        {
            self.active = None;
            body.grounded = true;
            body.velocity = [0.0; 3];
            return false;
        }
        let mut target = target.clone();
        // Only locomotion clips slide. Look-out stays anchored at the endpoint.
        let id = name
            .strip_prefix("a000_")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        if (21100..21700).contains(&id)
            && let Some(yaw) = stick_yaw
            // HKS DOCKING_MOVE_INPUT_ANGLE_MIN/MAX: only the along-wall cone moves.
            && {
                let angle = angle_diff(yaw, target.yaw() + std::f32::consts::PI).abs().to_degrees();
                angle > 30.0 && angle < 150.0
            }
        {
            let edge = &self.edges[target.edge];
            let tangent = (Vec3::from(edge.end) - Vec3::from(edge.start)).normalize();
            let stick = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
            let next = target.point + tangent * root_distance * stick.dot(tangent).signum();
            let (point, _) = edge.closest(next);
            if let Some(updated) = self.cover_target(
                target.edge,
                Vec3::new(point.x, target.floor.y, point.z)
                    + target.outward * DOCKING_TRANSLATION_OFFSET_COVER,
                ground,
            ) {
                target = updated;
            }
        }
        body.position = target.floor.to_array();
        body.yaw = angle_diff(target.yaw() + std::f32::consts::PI, 0.0);
        body.velocity = [0.0; 3];
        body.grounded = true;
        body.last_fall_height = 0.0;
        self.active = Some(target);
        true
    }
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
        if !is_hang(edge) {
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
        self.air_hang_type = 0;
        if self.active.is_some() {
            return;
        }
        if !body.grounded {
            self.air_search(body, ground);
            return;
        }
        let p = Vec3::from(body.position);
        let mut nearest = GROUND_REACH;
        for i in 0..self.edges.len() {
            let edge = &self.edges[i];
            if edge.kind == 3 {
                let (point, _) = edge.closest(p);
                let distance = Vec3::new(p.x - point.x, 0.0, p.z - point.z).length();
                if distance <= nearest
                    && let Some(mut target) = self.cover_target(i, p, ground)
                    && angle_diff(body.yaw, target.yaw()).abs().to_degrees() <= FACING_LIMIT
                {
                    target.angle =
                        angle_diff(target.yaw() + std::f32::consts::PI, body.yaw).to_degrees();
                    nearest = distance;
                    self.candidate = Some(target);
                }
                continue;
            }
            if !is_hang(edge) {
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

    /// The air catch (env 3045): a hang edge whose lip is in [`AIR_HANG_BAND`] over the feet,
    /// with the body off its face within [`AIR_HANG_LONG`]; the nearest one wins. Hands the
    /// wall jump and plain falls into the ground hang's own docking.
    fn air_search(&mut self, body: &Body, ground: &dyn Ground) {
        let p = Vec3::from(body.position);
        let mut nearest = f32::INFINITY;
        for i in 0..self.edges.len() {
            let edge = &self.edges[i];
            if !is_hang(edge) {
                continue;
            }
            let (point, t) = edge.closest(p);
            let up = point.y - p.y;
            if !(AIR_HANG_BAND.0..=AIR_HANG_BAND.1).contains(&up)
                || !(0.0..=1.0).contains(&t)
                || Vec3::new(p.x - point.x, 0.0, p.z - point.z).length() > AIR_HANG_LONG + 0.5
            {
                continue;
            }
            let Some(mut target) = self.target(i, p, ground) else {
                continue;
            };
            let off = (p - target.point).dot(target.outward);
            // Beside the edge's span, not past its ends.
            let a = Vec3::from(edge.start);
            let d = Vec3::from(edge.end) - a;
            let along = Vec3::new(d.x, 0.0, d.z);
            let s = (p - a).dot(along) / along.length_squared().max(1e-8);
            if !(-0.3..=AIR_HANG_LONG).contains(&off) || !(0.0..=1.0).contains(&s) || off >= nearest
            {
                continue;
            }
            target.angle = angle_diff(target.yaw(), body.yaw).to_degrees();
            nearest = off;
            self.air_hang_type = if off > AIR_HANG_SHORT {
                AIR_HANG_TYPE_LONG
            } else {
                AIR_HANG_TYPE_SHORT
            };
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
            && !CLIMB_CLIPS.contains(&name)
            && !DROP_CLIPS.contains(&name)
        {
            self.active = self.candidate.clone();
        }
        let Some(target) = &self.active else {
            return false;
        };
        if self.is_cover(target) {
            return false;
        }
        if DROP_CLIPS.contains(&name) {
            body.grounded = false;
            body.velocity = [0.0; 3];
            body.fall_top = body.position[1];
            self.active = None;
            self.climb_start = None;
            return false;
        }
        let position = if CLIMB_CLIPS.contains(&name) {
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

/// Wall jump (c0000_transition `BEH_A_WALL_JUMP`): how far out from the chest a wall counts as
/// touched. Not traced: the exe's contact check behind env 3020; a 0.8 m probe in eight
/// directions round the facing, as in Fireguy32's RustSekiro port.
pub const WALL_REACH: f32 = 0.8;
/// Height of the wall-contact probe over the feet (the chest).
pub const WALL_PROBE_LIFT: f32 = 1.0;
/// Shortfall of the probe below which nothing counts as touched.
pub const WALL_TOLERANCE: f32 = 0.05;

/// The nearest wall within [`WALL_REACH`] of the chest of a body at `feet` facing `yaw`: the
/// angle in degrees from the facing to it, positive to the right. This is what the engine hands
/// the HKS as `ContactWallAngle` and answers env 3020 with; the script turns it into
/// `Selector_WallContactType` (-45..45 F, beyond +-135 B, -135..-45 L, else R).
pub fn wall_contact(feet: Vec3, yaw: f32, ground: &dyn Ground) -> Option<f32> {
    let chest = [feet.x, feet.y + WALL_PROBE_LIFT, feet.z];
    (0..8)
        .filter_map(|k| {
            let ray_yaw = yaw - k as f32 * std::f32::consts::FRAC_PI_4;
            // yaw is measured around +Y, forward is (-sin yaw, -cos yaw).
            let dir = Vec3::new(-ray_yaw.sin(), 0.0, -ray_yaw.cos());
            let delta = dir * WALL_REACH;
            let slid = ground.slide(chest, [delta.x, delta.z]);
            let shortfall = WALL_REACH - Vec3::new(slid[0], 0.0, slid[1]).dot(dir);
            (shortfall > WALL_TOLERANCE)
                .then(|| (-angle_diff(ray_yaw, yaw).to_degrees(), shortfall))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(angle, _)| angle)
}

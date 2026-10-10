//! Camera framing and chase rates from the game's normal camera params.

use crate::hits::load_table;
use glam::Vec3;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraParams {
    /// LockCamParam row 0 `camDistTarget`.
    pub cam_dist: f32,
    /// LockCamParam row 0 `chrOrgOffset_Y`.
    pub chr_org_offset_y: f32,
    /// LockCamParam row 0 `camFovY`.
    pub cam_fov_y: f32,
    /// LockCamParam row 0 `chrLockRangeMaxRadius`.
    pub lock_range: f32,
    /// LockCamParam row 0 `lockRotXShiftRatio`.
    pub lock_rot_x_shift_ratio: f32,
    /// CameraParam row 0 `chrTransChaseRateXZ_ForNormal`.
    pub chr_trans_chase_rate_xz: f32,
    /// CameraParam row 0 `chrTransChaseRateY_ForNormal`.
    pub chr_trans_chase_rate_y: f32,
    /// CameraParam row 0 `rotRangeXMin`.
    pub rot_range_x_min: f32,
    /// CameraParam row 0 `rotRangeXMax`.
    pub rot_range_x_max: f32,
    /// CameraParam row 0 `rotRangeXAtLockMin`.
    pub rot_range_x_at_lock_min: f32,
    /// CameraParam row 0 `rotRangeXAtLockMax`.
    pub rot_range_x_at_lock_max: f32,
    /// CameraParam row 0 `rotRangeLerpBeginHeight`.
    pub rot_range_lerp_begin_height: f32,
    /// CameraParam row 0 `rotRangeLerpEndHeight`.
    pub rot_range_lerp_end_height: f32,
    /// CameraParam row 0 `rotSpeed_MaxX`.
    pub rot_speed_max_x: f32,
    /// CameraParam row 0 `rotSpeed_MaxY`.
    pub rot_speed_max_y: f32,
    /// CameraParam row 0 `lockRotChaseRateX`.
    pub lock_rot_chase_rate_x: f32,
    /// CameraParam row 0 `lockRotChaseRateY`.
    pub lock_rot_chase_rate_y: f32,
    /// CameraParam row 0 `lockTgtPosRate`.
    pub lock_tgt_pos_rate: f32,
}

impl Default for CameraParams {
    #[allow(clippy::approx_constant)] // Preserve the literal values stored in the params.
    fn default() -> Self {
        Self {
            cam_dist: 4.5,
            chr_org_offset_y: 1.5,
            cam_fov_y: 43.0,
            lock_range: 30.0,
            lock_rot_x_shift_ratio: 0.45,
            chr_trans_chase_rate_xz: 0.2,
            chr_trans_chase_rate_y: 0.3,
            rot_range_x_min: -0.6981317,
            rot_range_x_max: 1.3962634,
            rot_range_x_at_lock_min: -0.5235988,
            rot_range_x_at_lock_max: 0.5235988,
            rot_range_lerp_begin_height: 0.5,
            rot_range_lerp_end_height: 3.0,
            rot_speed_max_x: 2.9321532,
            rot_speed_max_y: 3.6651914,
            lock_rot_chase_rate_x: 0.6,
            lock_rot_chase_rate_y: 0.3,
            lock_tgt_pos_rate: 0.7,
        }
    }
}

fn param_dirs(cache: &Path) -> (PathBuf, PathBuf) {
    (
        cache.join("raw/param/gameparam/gameparam.parambnd.d"),
        cache.join("refs/paramdex/Defs"),
    )
}

impl CameraParams {
    /// Loads row 0 of both tables, falling back per field when data is missing.
    pub fn load(cache: &Path) -> Self {
        let (pd, dd) = param_dirs(cache);
        let lock = load_table(&pd, &dd, "LockCamParam", "LockCamParam");
        let camera = load_table(&pd, &dd, "CameraParam", "CameraParam");
        let lock = lock.as_ref().and_then(|t| t.find(0));
        let camera = camera.as_ref().and_then(|t| t.find(0));
        let defaults = Self::default();
        Self {
            cam_dist: lock
                .as_ref()
                .and_then(|r| r.f32("camDistTarget").ok())
                .unwrap_or(defaults.cam_dist),
            chr_org_offset_y: lock
                .as_ref()
                .and_then(|r| r.f32("chrOrgOffset_Y").ok())
                .unwrap_or(defaults.chr_org_offset_y),
            cam_fov_y: lock
                .as_ref()
                .and_then(|r| r.f32("camFovY").ok())
                .unwrap_or(defaults.cam_fov_y),
            lock_range: lock
                .as_ref()
                .and_then(|r| r.f32("chrLockRangeMaxRadius").ok())
                .unwrap_or(defaults.lock_range),
            lock_rot_x_shift_ratio: lock
                .as_ref()
                .and_then(|r| r.f32("lockRotXShiftRatio").ok())
                .unwrap_or(defaults.lock_rot_x_shift_ratio),
            chr_trans_chase_rate_xz: camera
                .as_ref()
                .and_then(|r| r.f32("chrTransChaseRateXZ_ForNormal").ok())
                .unwrap_or(defaults.chr_trans_chase_rate_xz),
            chr_trans_chase_rate_y: camera
                .as_ref()
                .and_then(|r| r.f32("chrTransChaseRateY_ForNormal").ok())
                .unwrap_or(defaults.chr_trans_chase_rate_y),
            rot_range_x_min: camera
                .as_ref()
                .and_then(|r| r.f32("rotRangeXMin").ok())
                .unwrap_or(defaults.rot_range_x_min),
            rot_range_x_max: camera
                .as_ref()
                .and_then(|r| r.f32("rotRangeXMax").ok())
                .unwrap_or(defaults.rot_range_x_max),
            rot_range_x_at_lock_min: camera
                .as_ref()
                .and_then(|r| r.f32("rotRangeXAtLockMin").ok())
                .unwrap_or(defaults.rot_range_x_at_lock_min),
            rot_range_x_at_lock_max: camera
                .as_ref()
                .and_then(|r| r.f32("rotRangeXAtLockMax").ok())
                .unwrap_or(defaults.rot_range_x_at_lock_max),
            rot_range_lerp_begin_height: camera
                .as_ref()
                .and_then(|r| r.f32("rotRangeLerpBeginHeight").ok())
                .unwrap_or(defaults.rot_range_lerp_begin_height),
            rot_range_lerp_end_height: camera
                .as_ref()
                .and_then(|r| r.f32("rotRangeLerpEndHeight").ok())
                .unwrap_or(defaults.rot_range_lerp_end_height),
            rot_speed_max_x: camera
                .as_ref()
                .and_then(|r| r.f32("rotSpeed_MaxX").ok())
                .unwrap_or(defaults.rot_speed_max_x),
            rot_speed_max_y: camera
                .as_ref()
                .and_then(|r| r.f32("rotSpeed_MaxY").ok())
                .unwrap_or(defaults.rot_speed_max_y),
            lock_rot_chase_rate_x: camera
                .as_ref()
                .and_then(|r| r.f32("lockRotChaseRateX").ok())
                .unwrap_or(defaults.lock_rot_chase_rate_x),
            lock_rot_chase_rate_y: camera
                .as_ref()
                .and_then(|r| r.f32("lockRotChaseRateY").ok())
                .unwrap_or(defaults.lock_rot_chase_rate_y),
            lock_tgt_pos_rate: camera
                .as_ref()
                .and_then(|r| r.f32("lockTgtPosRate").ok())
                .unwrap_or(defaults.lock_tgt_pos_rate),
        }
    }

    pub fn pitch_limits(&self, locked: bool, height_gap: f32) -> (f32, f32) {
        if !locked {
            return (self.rot_range_x_min, self.rot_range_x_max);
        }
        let t = ((height_gap.abs() - self.rot_range_lerp_begin_height)
            / (self.rot_range_lerp_end_height - self.rot_range_lerp_begin_height))
            .clamp(0.0, 1.0);
        (
            self.rot_range_x_at_lock_min
                + t * (self.rot_range_x_min - self.rot_range_x_at_lock_min),
            self.rot_range_x_at_lock_max
                + t * (self.rot_range_x_max - self.rot_range_x_at_lock_max),
        )
    }

    pub fn lock_target_pitch(&self, focus: Vec3, target: Vec3) -> f32 {
        let p = focus + self.lock_tgt_pos_rate * (target - focus);
        let half = self.cam_fov_y.to_radians() * self.lock_rot_x_shift_ratio * 0.5;
        let delta = p - focus;
        let ratio = self.cam_dist * half.sin() / delta.length();
        let a = if ratio <= 1.0 {
            ratio.asin()
        } else {
            std::f32::consts::FRAC_PI_2
        };
        let e = delta.y.atan2(delta.x.hypot(delta.z));
        a + half - e
    }
}

/// Fraction of the gap closed at the game's 30 Hz camera chase rate.
pub fn chase(rate: f32, dt: f32) -> f32 {
    1.0 - (1.0 - rate).powf(dt * 30.0)
}

/// Matches the facing convention of Character::yaw_to.
pub fn lock_target_yaw(focus: Vec3, target: Vec3) -> f32 {
    let delta = target - focus;
    (-delta.x).atan2(-delta.z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::approx_constant)] // Check the exact param defaults.
    fn camera_defaults() {
        let p = CameraParams::default();
        assert_eq!(p.cam_dist, 4.5);
        assert_eq!(p.chr_org_offset_y, 1.5);
        assert_eq!(p.cam_fov_y, 43.0);
        assert_eq!(p.lock_range, 30.0);
        assert_eq!(p.lock_rot_x_shift_ratio, 0.45);
        assert_eq!(p.chr_trans_chase_rate_xz, 0.2);
        assert_eq!(p.chr_trans_chase_rate_y, 0.3);
        assert_eq!(p.rot_range_x_min, -0.6981317);
        assert_eq!(p.rot_range_x_max, 1.3962634);
        assert_eq!(p.rot_range_x_at_lock_min, -0.5235988);
        assert_eq!(p.rot_range_x_at_lock_max, 0.5235988);
        assert_eq!(p.rot_range_lerp_begin_height, 0.5);
        assert_eq!(p.rot_range_lerp_end_height, 3.0);
        assert_eq!(p.rot_speed_max_x, 2.9321532);
        assert_eq!(p.rot_speed_max_y, 3.6651914);
        assert_eq!(p.lock_rot_chase_rate_x, 0.6);
        assert_eq!(p.lock_rot_chase_rate_y, 0.3);
        assert_eq!(p.lock_tgt_pos_rate, 0.7);
    }

    #[test]
    fn camera_chase() {
        assert!((chase(0.2, 1.0 / 30.0) - 0.2).abs() < 1e-5);
        assert!((chase(0.2, 2.0 / 30.0) - 0.36).abs() < 1e-5);
    }

    #[test]
    fn camera_pitch_limits() {
        let p = CameraParams::default();
        let free = (p.rot_range_x_min, p.rot_range_x_max);
        assert_eq!(p.pitch_limits(false, 0.0), free);
        assert_eq!(
            p.pitch_limits(true, 0.0),
            (p.rot_range_x_at_lock_min, p.rot_range_x_at_lock_max)
        );
        assert_eq!(p.pitch_limits(true, 3.0), free);
    }

    #[test]
    fn camera_lock_target_pitch() {
        let p = CameraParams::default();
        let focus = Vec3::new(0.0, 1.5, 0.0);
        let target = Vec3::new(0.0, 1.5, -2.5);
        let flat = p.lock_target_pitch(focus, target);
        assert!((0.55..0.65).contains(&flat));
        assert!(p.lock_target_pitch(focus, target + Vec3::Y * 2.0) < flat);
    }

    #[test]
    fn camera_lock_target_yaw() {
        assert_eq!(lock_target_yaw(Vec3::ZERO, Vec3::NEG_Z), 0.0);
        assert_eq!(
            lock_target_yaw(Vec3::ZERO, Vec3::X),
            -std::f32::consts::FRAC_PI_2
        );
    }

    #[test]
    fn camera_cache() {
        let cache = Path::new("../../cache");
        if cache.join("params/LockCamParam.json").is_file() {
            assert_eq!(CameraParams::load(cache).lock_range, 30.0);
        }
    }
}

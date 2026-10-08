//! The character's body in the world: position, facing, velocity and ground contact.
//!
//! World space is Bevy's (right-handed, Y up). A character with yaw 0 faces -Z, and positive yaw
//! turns it to the left (counter-clockwise seen from above). Animation root motion arrives in
//! the game's source space and is mirrored on X here, like the clips the renderer plays.
//!
//! Movement on the ground comes only from root motion; turning comes from root-motion yaw plus
//! steering toward the stick at the TAE turn speed. In the air, a velocity set by a TAE velocity
//! change (jumps) integrates under gravity. Ground is a [`Ground`] query so a collision mesh can
//! replace the flat plane later.

use crate::input::angle_diff;
use crate::tae::VelocityChange;

/// Downward acceleration in m/s². An assumption tuned so the game's jump velocity (9.64 m/s up)
/// gives an apex near 2.3 m and lands as the jump-start clip ends; the engine value is unknown.
pub const GRAVITY: f32 = 20.0;

/// Something the character can stand on.
pub trait Ground {
    /// Height of the walkable surface below `(x, z)`, if any.
    fn height(&self, x: f32, z: f32) -> Option<f32>;
}

/// An infinite flat plane.
#[derive(Debug, Clone, Copy, Default)]
pub struct FlatGround {
    pub height: f32,
}

impl Ground for FlatGround {
    fn height(&self, _x: f32, _z: f32) -> Option<f32> {
        Some(self.height)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Body {
    pub position: [f32; 3],
    pub yaw: f32,
    pub velocity: [f32; 3],
    pub grounded: bool,
    /// Highest point of the current airborne phase.
    pub fall_top: f32,
    /// Height fallen on the last landing (env 224).
    pub last_fall_height: f32,
}

impl Default for Body {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            yaw: 0.0,
            velocity: [0.0; 3],
            grounded: true,
            fall_top: 0.0,
            last_fall_height: 0.0,
        }
    }
}

/// What moves the body this tick.
#[derive(Debug, Clone, Default)]
pub struct BodyInput {
    /// Root-motion delta in the character's local source space: `[x, y, z, yaw]`.
    pub root_motion: [f32; 4],
    /// World yaw to steer toward, if the stick is pushed.
    pub steer_to: Option<f32>,
    /// Steering speed in degrees per second (0 disables steering).
    pub turn_speed: f32,
    /// A velocity change that starts this tick.
    pub velocity_change: Option<VelocityChange>,
}

impl Body {
    /// Unit forward vector for the current yaw.
    pub fn forward(&self) -> [f32; 2] {
        [-self.yaw.sin(), -self.yaw.cos()]
    }

    /// True while airborne and moving down.
    pub fn falling(&self) -> bool {
        !self.grounded && self.velocity[1] < 0.0
    }

    pub fn step(&mut self, input: &BodyInput, dt: f32, ground: &dyn Ground) {
        // Root motion: mirror X (and so the yaw) into Bevy space, then rotate by facing.
        let [rx, _ry, rz, ryaw] = input.root_motion;
        let (lx, lz) = (-rx, rz);
        let (s, c) = self.yaw.sin_cos();
        // rotate_y(yaw) applied to (lx, 0, lz).
        let wx = c * lx + s * lz;
        let wz = -s * lx + c * lz;
        self.position[0] += wx;
        self.position[2] += wz;
        self.yaw -= ryaw;

        if let Some(target) = input.steer_to
            && input.turn_speed > 0.0
        {
            let max = input.turn_speed.to_radians() * dt;
            self.yaw += angle_diff(target, self.yaw).clamp(-max, max);
        }
        self.yaw = angle_diff(self.yaw, 0.0);

        if let Some(v) = input.velocity_change {
            let [fx, fz] = self.forward();
            let a = v.horizontal_angle.to_radians();
            let (sa, ca) = a.sin_cos();
            let (dx, dz) = (fx * ca - fz * sa, fx * sa + fz * ca);
            let h = [
                self.velocity[0] * v.horizontal_scale + dx * v.horizontal,
                self.velocity[2] * v.horizontal_scale + dz * v.horizontal,
            ];
            self.velocity = [h[0], self.velocity[1] * v.vertical_scale + v.vertical, h[1]];
            if self.velocity[1] > 0.0 {
                self.grounded = false;
                self.fall_top = self.position[1];
            }
        }

        let floor = ground.height(self.position[0], self.position[2]);
        if self.grounded {
            match floor {
                Some(h) if self.position[1] - h < 0.3 => self.position[1] = h,
                _ => {
                    self.grounded = false;
                    self.fall_top = self.position[1];
                }
            }
            if self.grounded {
                return;
            }
        }
        self.velocity[1] -= GRAVITY * dt;
        for (p, v) in self.position.iter_mut().zip(self.velocity) {
            *p += v * dt;
        }
        self.fall_top = self.fall_top.max(self.position[1]);
        if let Some(h) = floor
            && self.velocity[1] <= 0.0
            && self.position[1] <= h
        {
            self.position[1] = h;
            self.velocity = [0.0; 3];
            self.grounded = true;
            self.last_fall_height = self.fall_top - h;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_motion_moves_forward_and_jump_lands() {
        let ground = FlatGround::default();
        let mut b = Body::default();
        // One metre of forward root motion (source -Z) at yaw 0 moves toward -Z.
        b.step(
            &BodyInput {
                root_motion: [0.0, 0.0, -1.0, 0.0],
                ..BodyInput::default()
            },
            1.0 / 30.0,
            &ground,
        );
        assert!((b.position[2] + 1.0).abs() < 1e-5);

        let jump = VelocityChange {
            horizontal_scale: 0.0,
            vertical_scale: 0.0,
            horizontal: 6.4,
            vertical: 9.64,
            horizontal_angle: 0.0,
        };
        b.step(
            &BodyInput {
                velocity_change: Some(jump),
                ..BodyInput::default()
            },
            1.0 / 60.0,
            &ground,
        );
        assert!(!b.grounded);
        let mut frames = 1;
        while !b.grounded && frames < 600 {
            b.step(&BodyInput::default(), 1.0 / 60.0, &ground);
            frames += 1;
        }
        let airtime = frames as f32 / 60.0;
        assert!((0.9..1.05).contains(&airtime), "airtime {airtime}");
        assert!(b.last_fall_height > 2.0 && b.last_fall_height < 2.6);
        assert!(b.position[2] < -6.0, "jumped forward: {:?}", b.position);
    }

    #[test]
    fn steering_is_rate_limited() {
        let ground = FlatGround::default();
        let mut b = Body::default();
        b.step(
            &BodyInput {
                steer_to: Some(std::f32::consts::FRAC_PI_2),
                turn_speed: 90.0,
                ..BodyInput::default()
            },
            0.5,
            &ground,
        );
        assert!((b.yaw - std::f32::consts::FRAC_PI_4).abs() < 1e-5);
    }
}

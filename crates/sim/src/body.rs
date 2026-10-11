//! The character's body in the world: position, facing, velocity and ground contact.
//!
//! World space is Bevy's (right-handed, Y up). A character with yaw 0 faces -Z, and positive yaw
//! turns it to the left (counter-clockwise seen from above). Animation root motion arrives in
//! the game's source space and is mirrored on X here, like the clips the renderer plays.
//!
//! Movement on the ground comes only from root motion; turning comes from root-motion yaw plus
//! steering toward the stick at the TAE turn speed. In the air, a velocity set by a TAE velocity
//! change (jumps) integrates under gravity. Ground is a [`Ground`]: a flat plane, or the map's
//! hit collision ([`crate::collision::CollisionWorld`]), which also blocks horizontal moves.

use crate::input::angle_diff;
use crate::tae::VelocityChange;
use glam::Vec2;

/// Exe fall-control table values from FwLo's RustSekiro (MIT), `src/data.rs`.
#[derive(Debug, Clone, Copy)]
pub struct FallControl {
    pub horizontal_accel: f32,
    pub vertical_accel: f32,
    pub stick_accel: f32,
    pub stick_max: f32,
    pub steerable: bool,
}

pub const FALL_TYPES: [FallControl; 4] = [
    FallControl {
        horizontal_accel: -0.1,
        vertical_accel: -13.72,
        stick_accel: 10.0,
        stick_max: 2.0,
        steerable: true,
    },
    FallControl {
        horizontal_accel: -0.1,
        vertical_accel: -13.72,
        stick_accel: 10.0,
        stick_max: 2.0,
        steerable: false,
    },
    FallControl {
        horizontal_accel: -15.0,
        vertical_accel: -9.8,
        stick_accel: 9.0,
        stick_max: 6.0,
        steerable: true,
    },
    FallControl {
        horizontal_accel: -0.1,
        vertical_accel: -13.72,
        stick_accel: 10.0,
        stick_max: 2.0,
        steerable: true,
    },
];

/// Horizontal air integration described by FwLo's RustSekiro (MIT).
fn air_velocity(mut v: Vec2, stick: Vec2, control: FallControl, dt: f32) -> Vec2 {
    let decay = control.horizontal_accel.abs() * dt;
    if v.length() > decay {
        v -= v.normalize() * decay;
    }
    if control.steerable {
        let a = stick.clamp_length_max(1.0) * control.stick_accel * dt;
        let speed = v.length();
        if speed > 1e-6 {
            let direction = v / speed;
            let along = a.dot(direction);
            let across = a - direction * along;
            v += across
                + direction
                    * if along <= 0.0 {
                        along
                    } else {
                        along.min((control.stick_max - speed).max(0.0))
                    };
        } else {
            v += a.clamp_length_max((control.stick_max - speed).max(0.0));
        }
    }
    v
}

/// Downward acceleration in m/s²: Havok world gravity (9.8) plus the extra vertical
/// acceleration of the normal fall type (13.72) from the exe's fall-control table. Measured
/// in the running game as a constant 23.52 m/s² over five jumps (traced by FwLo's
/// RustSekiro, `docs/kb/movement.md`); a 9.64 m/s jump rises about 2 m and lands after 0.82 s.
pub const GRAVITY: f32 = 9.8 + 13.72;

/// Something the character can stand on.
pub trait Ground {
    /// Lowest walkable collision at this horizontal point, for the placeholder void boundary.
    fn lowest_floor(&self, x: f32, z: f32) -> Option<f32> {
        let mut lowest = self.height(x, z)?;
        while let Some(next) = self.floor(x, z, lowest - 0.01) {
            if next >= lowest {
                break;
            }
            lowest = next;
        }
        Some(lowest)
    }
    /// Height of the walkable surface below `(x, z)`, if any.
    fn height(&self, x: f32, z: f32) -> Option<f32>;

    /// The walkable surface under `(x, z)` that is no higher than `top` (multi-level ground).
    fn floor(&self, x: f32, z: f32, top: f32) -> Option<f32> {
        let _ = top;
        self.height(x, z)
    }

    /// The horizontal displacement a body at `feet` actually makes when it tries to move by
    /// `delta` (walls stop it and it slides along them). Unobstructed by default.
    fn slide(&self, feet: [f32; 3], delta: [f32; 2]) -> [f32; 2] {
        let _ = feet;
        delta
    }

    /// Highest ledge a grounded body steps onto, and the furthest it follows the floor down
    /// before it starts falling.
    fn max_step(&self) -> f32 {
        0.3
    }

    /// The hit material (HitMtrlParam row: 1 cobblestone, 4 wood, 16 tile, 28 tatami, ...) of
    /// the floor under `(x, z)` no higher than `top`, when the ground knows it. Footstep
    /// sounds are chosen by it.
    fn material(&self, x: f32, z: f32, top: f32) -> Option<u32> {
        let _ = (x, z, top);
        None
    }
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
    /// Selected by the latest velocity change, reset on landing or walking off a ledge.
    pub fall_type: u8,
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
            fall_type: 0,
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
    /// World-space horizontal movement stick, with magnitude at most one.
    pub stick: Vec2,
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
        let mut travel = [wx, wz];
        self.yaw -= ryaw;

        if let Some(target) = input.steer_to
            && input.turn_speed > 0.0
        {
            let max = input.turn_speed.to_radians() * dt;
            self.yaw += angle_diff(target, self.yaw).clamp(-max, max);
        }
        self.yaw = angle_diff(self.yaw, 0.0);

        if let Some(v) = input.velocity_change {
            self.fall_type = v.fall_type;
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

        if !self.grounded {
            let control = FALL_TYPES
                .get(self.fall_type as usize)
                .copied()
                .unwrap_or(FALL_TYPES[0]);
            let v = air_velocity(
                Vec2::new(self.velocity[0], self.velocity[2]),
                input.stick,
                control,
                dt,
            );
            self.velocity[0] = v.x;
            self.velocity[2] = v.y;
            travel[0] += self.velocity[0] * dt;
            travel[1] += self.velocity[2] * dt;
        }
        let [mx, mz] = ground.slide(self.position, travel);
        self.position[0] += mx;
        self.position[2] += mz;

        let step = ground.max_step();
        if self.grounded {
            // Follow the floor up or down by a step, plus what a 50 degree slope rises over
            // this tick's travel, so a fast tick on a ramp or stairs keeps contact.
            let reach = step + (mx * mx + mz * mz).sqrt() * 1.2;
            let floor = ground.floor(self.position[0], self.position[2], self.position[1] + reach);
            match floor {
                Some(h) if self.position[1] - h < reach => self.position[1] = h,
                _ => {
                    self.fall_type = 0;
                    self.grounded = false;
                    self.fall_top = self.position[1];
                }
            }
            if self.grounded {
                return;
            }
        }
        // Exact constant-acceleration step, so the arc does not depend on the tick rate.
        let previous_y = self.position[1];
        let gravity = 9.8
            + FALL_TYPES
                .get(self.fall_type as usize)
                .unwrap_or(&FALL_TYPES[0])
                .vertical_accel
                .abs();
        self.position[1] += self.velocity[1] * dt - 0.5 * gravity * dt * dt;
        self.velocity[1] -= gravity * dt;
        self.fall_top = self.fall_top.max(self.position[1]);
        // Land on the highest floor the feet passed through this tick, or one up to a step
        // above them (a stair tread or ledge the body was carried into while airborne).
        let floor = ground.floor(self.position[0], self.position[2], previous_y + step);
        if let Some(h) = floor
            && self.velocity[1] <= 0.0
            && self.position[1] <= h
        {
            self.position[1] = h;
            self.velocity = [0.0; 3];
            self.grounded = true;
            self.fall_type = 0;
            self.last_fall_height = self.fall_top - h;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: Vec2, expected: Vec2) {
        assert!(
            (actual - expected).length() < 1e-5,
            "{actual:?} != {expected:?}"
        );
    }

    #[test]
    fn air_decay_does_not_flip_small_velocity() {
        close(
            air_velocity(Vec2::new(3.0, 4.0), Vec2::ZERO, FALL_TYPES[0], 1.0),
            Vec2::new(2.94, 3.92),
        );
        close(
            air_velocity(Vec2::new(0.05, 0.0), Vec2::ZERO, FALL_TYPES[0], 1.0),
            Vec2::new(0.05, 0.0),
        );
    }

    #[test]
    fn grounded_stick_does_not_add_air_velocity() {
        let mut b = Body::default();
        b.step(
            &BodyInput {
                stick: Vec2::X,
                root_motion: [0.0, 0.0, -1.0, 0.0],
                ..Default::default()
            },
            0.1,
            &FlatGround::default(),
        );
        assert_eq!(b.position, [0.0, 0.0, -1.0]);
        assert_eq!(b.velocity, [0.0; 3]);
    }

    #[test]
    fn air_braking_and_across_steer_apply_above_cap() {
        close(
            air_velocity(Vec2::new(6.0, 0.0), -Vec2::X, FALL_TYPES[0], 0.1),
            Vec2::new(4.99, 0.0),
        );
        close(
            air_velocity(Vec2::new(6.0, 0.0), Vec2::Y, FALL_TYPES[0], 0.1),
            Vec2::new(5.99, 1.0),
        );
        close(
            air_velocity(Vec2::new(6.0, 0.0), Vec2::Y, FALL_TYPES[1], 0.1),
            Vec2::new(5.99, 0.0),
        );
    }

    #[test]
    fn air_along_acceleration_caps_only_speed_gain() {
        close(
            air_velocity(Vec2::new(1.9, 0.0), Vec2::X, FALL_TYPES[0], 0.1),
            Vec2::new(2.0, 0.0),
        );
        close(
            air_velocity(Vec2::new(6.0, 0.0), Vec2::X, FALL_TYPES[0], 0.1),
            Vec2::new(5.99, 0.0),
        );
        close(
            air_velocity(Vec2::ZERO, Vec2::X, FALL_TYPES[0], 1.0),
            Vec2::new(2.0, 0.0),
        );
        close(
            air_velocity(Vec2::new(8.0, 0.0), Vec2::X, FALL_TYPES[2], 0.1),
            Vec2::new(6.5, 0.0),
        );
    }

    #[test]
    fn selected_fall_type_persists_until_landing() {
        let mut b = Body {
            position: [0.0, 10.0, 0.0],
            grounded: false,
            fall_top: 10.0,
            ..Default::default()
        };
        let input = BodyInput {
            stick: Vec2::X,
            velocity_change: Some(VelocityChange {
                fall_type: 2,
                horizontal_scale: 0.0,
                vertical_scale: 0.0,
                horizontal: 0.0,
                vertical: 0.0,
                horizontal_angle: 0.0,
            }),
            ..Default::default()
        };
        let ground = FlatGround::default();
        b.step(&input, 0.1, &ground);
        assert!((b.velocity[1] + 1.96).abs() < 1e-5);
        b.step(&BodyInput::default(), 0.1, &ground);
        assert_eq!(b.fall_type, 2);
        assert!((b.velocity[1] + 3.92).abs() < 1e-5);
        while !b.grounded {
            b.step(&BodyInput::default(), 0.1, &ground);
        }
        assert_eq!(b.fall_type, 0);
    }

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
            fall_type: 0,
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
        // The running game: 0.82 s in the air and a 1.98 m rise for this launch.
        assert!((0.8..0.86).contains(&airtime), "airtime {airtime}");
        assert!(
            (1.94..2.02).contains(&b.last_fall_height),
            "rise {}",
            b.last_fall_height
        );
        assert!(b.position[2] < -5.0, "jumped forward: {:?}", b.position);
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

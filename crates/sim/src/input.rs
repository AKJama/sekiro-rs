//! Player input: logical buttons and the movement stick, translated into what the script reads.
//!
//! The script never sees keys. It asks the engine:
//!
//! - `env(1106, arm)`: is the action requested now. The engine keeps a press for a short buffer
//!   and only reports it while the current animation allows that action (TAE cancel windows, see
//!   [`crate::tae::cancel`]). Starting an action calls `act(9101)`, which clears the buffer.
//! - `env(1108, arm)`: how long the button has been held. The script compares it with 0 and, for
//!   sprinting in swamps, with 200, so milliseconds is our working unit (unverified).
//! - Behaviour variables the engine writes every frame: `MoveSpeedLevel` (0 to 2, stick
//!   magnitude), `TurnAngle` and `MoveAngle` (degrees from the character's facing to the stick
//!   direction, positive to the right), and the per-action copies `JumpAngle`, `AttackAngle`,
//!   `JumpStickLevel`, `AttackStickLevel` and so on.
//!
//! Action-arm ids (`ACTION_ARM_*` in the define script): attack 0, prosthetic 1, guard 2, grapple
//! 3, jump 4, step/dodge 5, item 7, crouch 15, combat art 32. Sekiro's dodge button steps on a
//! press and sprints while held, which the script reads as 1106 and 1108 of arm 5.

use crate::tae::{TaeFrame, cancel};
use std::collections::HashMap;

/// Action-arm ids, as the script names them.
pub mod arm {
    pub const ATTACK: i32 = 0;
    pub const PROSTHETIC: i32 = 1;
    pub const GUARD: i32 = 2;
    pub const GRAPPLE: i32 = 3;
    pub const JUMP: i32 = 4;
    pub const DODGE: i32 = 5;
    pub const ITEM: i32 = 7;
    pub const CROUCH: i32 = 15;
    pub const COMBAT_ART: i32 = 32;
}

/// Seconds a press stays requested while waiting for its cancel window (assumed).
pub const INPUT_BUFFER: f32 = 0.4;

/// Logical buttons, true while held.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Buttons {
    pub attack: bool,
    pub guard: bool,
    pub jump: bool,
    /// Step on press, sprint while held.
    pub dodge: bool,
    pub crouch: bool,
    pub grapple: bool,
    pub prosthetic: bool,
    pub item: bool,
    pub combat_art: bool,
    pub lock_on: bool,
}

impl Buttons {
    fn arms(&self) -> [(i32, bool); 9] {
        [
            (arm::ATTACK, self.attack),
            (arm::GUARD, self.guard),
            (arm::JUMP, self.jump),
            (arm::DODGE, self.dodge),
            (arm::CROUCH, self.crouch),
            (arm::GRAPPLE, self.grapple),
            (arm::PROSTHETIC, self.prosthetic),
            (arm::ITEM, self.item),
            (arm::COMBAT_ART, self.combat_art),
        ]
    }
}

/// One frame of player input.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct InputFrame {
    /// Movement stick: x to the right, y forward, magnitude up to 1 (keyboard gives 1).
    pub move_stick: [f32; 2],
    /// Camera yaw in radians: the camera looks along `rotate_y(camera_yaw) * -Z`.
    pub camera_yaw: f32,
    pub buttons: Buttons,
}

/// The cancel-window flag that lets an action interrupt the current animation.
pub fn cancel_flag(action: i32) -> Option<i32> {
    match action {
        arm::ATTACK => Some(cancel::ATTACK),
        arm::PROSTHETIC => Some(cancel::PROSTHETIC),
        arm::GUARD => Some(cancel::GUARD),
        arm::JUMP => Some(cancel::JUMP),
        arm::DODGE => Some(cancel::DODGE),
        arm::ITEM => Some(cancel::ITEM),
        arm::COMBAT_ART => Some(cancel::COMBAT_ART),
        arm::CROUCH => Some(cancel::MOVE),
        _ => None,
    }
}

/// Press buffering and hold times across frames.
#[derive(Debug, Default)]
pub struct InputState {
    prev: Buttons,
    /// Seconds since each buffered press.
    pending: HashMap<i32, f32>,
    /// Milliseconds each button has been held.
    held_ms: HashMap<i32, f32>,
    pub frame: InputFrame,
}

impl InputState {
    /// Takes this frame's input.
    pub fn update(&mut self, frame: &InputFrame, dt: f32) {
        for age in self.pending.values_mut() {
            *age += dt;
        }
        self.pending.retain(|_, age| *age <= INPUT_BUFFER);
        let prev = self.prev.arms();
        for (i, (action, down)) in frame.buttons.arms().into_iter().enumerate() {
            if down && !prev[i].1 {
                self.pending.insert(action, 0.0);
            }
            if down {
                *self.held_ms.entry(action).or_insert(0.0) += dt * 1000.0;
            } else {
                self.held_ms.remove(&action);
            }
        }
        self.prev = frame.buttons;
        self.frame = *frame;
    }

    /// `act(9101)`: the script started an action; buffered presses are spent.
    pub fn reset_requests(&mut self) {
        self.pending.clear();
    }

    /// `env(1106, action)` given the current TAE windows.
    pub fn requested(&self, action: i32, tae: &TaeFrame) -> bool {
        if !self.pending.contains_key(&action) {
            return false;
        }
        if !tae.restricted {
            return true;
        }
        match cancel_flag(action) {
            Some(flag) => tae.flag(flag),
            None => tae.flag(cancel::MOVE),
        }
    }

    pub fn pending_actions(&self) -> impl Iterator<Item = i32> + '_ {
        self.pending.keys().copied()
    }

    /// `env(1108, action)` in milliseconds.
    pub fn held_ms(&self, action: i32) -> f32 {
        self.held_ms.get(&action).copied().unwrap_or(0.0)
    }

    pub fn held(&self) -> impl Iterator<Item = (i32, f32)> + '_ {
        self.held_ms.iter().map(|(a, t)| (*a, *t))
    }

    /// Stick magnitude, clamped to 1.
    pub fn stick_level(&self) -> f32 {
        let [x, y] = self.frame.move_stick;
        (x * x + y * y).sqrt().min(1.0)
    }

    /// World-space yaw of the stick direction (same convention as the camera), if pushed.
    pub fn stick_world_yaw(&self) -> Option<f32> {
        let [x, y] = self.frame.move_stick;
        if x * x + y * y < 0.01 {
            return None;
        }
        // Stick forward is the camera forward; stick right turns clockwise seen from above.
        Some(self.frame.camera_yaw + (-x).atan2(y))
    }
}

/// Signed smallest angle `to - from`, radians in (-pi, pi].
pub fn angle_diff(to: f32, from: f32) -> f32 {
    let mut d = (to - from) % std::f32::consts::TAU;
    if d > std::f32::consts::PI {
        d -= std::f32::consts::TAU;
    } else if d <= -std::f32::consts::PI {
        d += std::f32::consts::TAU;
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn press_buffers_until_window_and_reset() {
        let mut s = InputState::default();
        let mut f = InputFrame::default();
        f.buttons.attack = true;
        s.update(&f, 1.0 / 30.0);
        let free = TaeFrame::default();
        assert!(s.requested(arm::ATTACK, &free));
        let mut busy = TaeFrame {
            restricted: true,
            ..TaeFrame::default()
        };
        assert!(!s.requested(arm::ATTACK, &busy));
        busy.flags.insert(cancel::ATTACK);
        assert!(s.requested(arm::ATTACK, &busy));
        s.reset_requests();
        assert!(!s.requested(arm::ATTACK, &free));
        assert!(s.held_ms(arm::ATTACK) > 30.0);
    }

    #[test]
    fn stick_yaw_follows_camera() {
        let mut s = InputState::default();
        s.update(
            &InputFrame {
                move_stick: [0.0, 1.0],
                camera_yaw: 0.5,
                ..InputFrame::default()
            },
            0.0,
        );
        assert!((s.stick_world_yaw().unwrap() - 0.5).abs() < 1e-6);
        s.update(
            &InputFrame {
                move_stick: [1.0, 0.0],
                camera_yaw: 0.0,
                ..InputFrame::default()
            },
            0.0,
        );
        // Right is a clockwise (negative) yaw.
        assert!((s.stick_world_yaw().unwrap() + std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        assert!((angle_diff(3.0, -3.0) - (6.0 - std::f32::consts::TAU)).abs() < 1e-6);
    }
}

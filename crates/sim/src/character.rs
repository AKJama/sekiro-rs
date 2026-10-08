//! The playable character: input, script and behaviour graph, TAE events and the body.
//!
//! [`PlayerCharacter::tick`] runs one simulation step:
//!
//! 1. Input: buffered presses and hold times, stick direction relative to the camera.
//! 2. What the engine tells the script: movement variables, action requests gated by the
//!    previous step's TAE cancel windows, active behaviour reference ids, ground contact.
//! 3. The script and behaviour graph ([`PlayerBehavior::tick`]).
//! 4. TAE events of the clips now playing, which feed the next step's answers.
//! 5. The body: root motion of the full-body clip, steering at the TAE turn speed, jump
//!    velocity changes and gravity.

use crate::behavior::ClipDurations;
use crate::body::{Body, BodyInput, FlatGround, Ground};
use crate::clips::ClipLibrary;
use crate::input::{InputFrame, InputState, angle_diff};
use crate::player::{LoadError, PlayerBehavior, TickReport};
use crate::tae::{PlayingClip, TaeDb, TaeFrame, TaeRuntime, cancel};
use sekiro_formats::hkb::NodeId;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// Steering speed when no TAE `SetTurnSpeed` is active, degrees per second (assumed).
pub const DEFAULT_TURN_SPEED: f32 = 540.0;

/// One step's outcome, for HUDs, logs and tests.
#[derive(Debug, Clone, Default)]
pub struct StepReport {
    pub behavior: TickReport,
    /// Full-body clip and its time after the step.
    pub animation: Option<String>,
    pub anim_time: f32,
    pub position: [f32; 3],
    pub yaw: f32,
    pub grounded: bool,
    pub sp_effects: Vec<i32>,
    pub behavior_refs: Vec<i32>,
    pub action_flags: Vec<i32>,
    pub requested: Vec<i32>,
}

pub struct PlayerCharacter {
    pub behavior: PlayerBehavior,
    pub clips: ClipLibrary,
    pub tae: Arc<TaeDb>,
    pub tae_runtime: TaeRuntime,
    pub body: Body,
    pub input: InputState,
    pub ground: Box<dyn Ground>,
    /// Clip times before the last step, by clip node.
    last_times: HashMap<NodeId, (String, f32)>,
}

impl PlayerCharacter {
    /// Loads Wolf from the extracted cache and settles him into the idle stance.
    pub fn load_from_cache(cache: &Path) -> Result<Self, LoadError> {
        let tae = Arc::new(TaeDb::load_from_cache(cache, "c0000"));
        let clips = ClipLibrary::new(cache.join("anim/c0000"));
        for (id, src) in tae.hkx_aliases() {
            clips.add_alias(id, src);
        }
        let behavior = PlayerBehavior::load(
            &cache.join("raw/action/script"),
            &cache.join("raw/chr/c0000.behbnd.d/c0000.hkx"),
            Box::new(clips.clone()),
        )?;
        let mut me = Self {
            behavior,
            clips,
            tae,
            tae_runtime: TaeRuntime::default(),
            body: Body::default(),
            input: InputState::default(),
            ground: Box::new(FlatGround::default()),
            last_times: HashMap::new(),
        };
        me.behavior.env.standby = Some(true);
        me.behavior.env.move_cancel = Some(true);
        Ok(me)
    }

    /// The TAE state computed by the last step.
    pub fn tae_frame(&self) -> &TaeFrame {
        &self.tae_runtime.frame
    }

    fn set_engine_variables(&mut self) {
        let level = self.input.stick_level();
        let turn = self
            .input
            .stick_world_yaw()
            .map(|y| angle_diff(y, self.body.yaw).to_degrees())
            .unwrap_or(0.0);
        let rt = &mut self.behavior.runtime;
        // The engine's stick level runs 0..2; full keyboard or stick deflection runs.
        rt.set_variable("MoveSpeedLevel", level * 2.0);
        for name in ["TurnAngle", "TurnAngleReal", "MoveAngle", "MoveAngleReal"] {
            rt.set_variable(name, turn);
        }
        for name in ["JumpAngle", "AttackAngle", "SubAttackAngle", "KickAngle"] {
            rt.set_variable(name, turn);
        }
        for name in [
            "JumpStickLevel",
            "AttackStickLevel",
            "SubAttackStickLevel",
            "KickStickLevel",
        ] {
            rt.set_variable(name, level);
        }
    }

    fn fill_env(&mut self, dt: f32) {
        let tae = &self.tae_runtime.frame;
        let env = &mut self.behavior.env;
        env.requested = self
            .input
            .pending_actions()
            .filter(|a| self.input.requested(*a, tae))
            .collect();
        env.held_ms = self.input.held().collect();
        env.sp_effect_refs = tae.behavior_refs.clone();
        // Free animations (no action flags at all) are standby; otherwise the move-cancel window
        // marks both standby and move cancel. Ended clips count as standby too.
        let ended = self
            .behavior
            .runtime
            .main_clip()
            .is_some_and(|c| c.at_end());
        let move_window = !tae.restricted || tae.flag(cancel::MOVE) || ended;
        env.standby = Some(move_window);
        env.move_cancel = Some(move_window);
        env.landed = self.body.grounded;
        env.falling = self.body.falling();
        env.fall_height = self.body.last_fall_height;
        env.dt = dt;
        // Lock-on targeting is not simulated yet.
        env.locked_on = false;
    }

    /// Runs one step of `dt` seconds with this frame's input.
    pub fn tick(&mut self, frame: &InputFrame, dt: f32) -> StepReport {
        self.input.update(frame, dt);
        self.set_engine_variables();
        self.fill_env(dt);
        let requested: Vec<i32> = self.behavior.env.requested.iter().copied().collect();
        let resets_before = self.behavior.env.input_resets.get();

        let behavior = self.behavior.tick(dt);

        if self.behavior.env.input_resets.get() != resets_before {
            self.input.reset_requests();
        }
        if let Some(deg) = self.behavior.env.face_turn.take() {
            self.body.yaw = angle_diff(self.body.yaw + deg.to_radians(), 0.0);
        }

        // Clips now playing, with the time each had before this step (0 if it just started).
        let main_node = self.behavior.runtime.main_clip().map(|c| c.node);
        let mut playing: Vec<(NodeId, PlayingClip)> = self
            .behavior
            .runtime
            .clips()
            .map(|c| {
                let prev = match self.last_times.get(&c.node) {
                    Some((name, t)) if *name == c.animation && *t <= c.time => *t,
                    _ => 0.0,
                };
                (
                    c.node,
                    PlayingClip {
                        animation: c.animation.clone(),
                        prev_time: prev,
                        time: c.time,
                    },
                )
            })
            .collect();
        playing.sort_by_key(|(n, _)| (Some(*n) != main_node, *n));
        self.tae_runtime.update(
            &self.tae,
            &playing.iter().map(|(_, c)| c.clone()).collect::<Vec<_>>(),
            dt,
        );
        self.last_times = playing
            .iter()
            .map(|(n, c)| (*n, (c.animation.clone(), c.time)))
            .collect();

        // Body.
        let main = playing
            .iter()
            .find(|(n, _)| Some(*n) == main_node)
            .map(|(_, c)| c.clone());
        let root_motion = main
            .as_ref()
            .and_then(|c| {
                let clip = self.clips.clip(&c.animation)?;
                let rm = clip.root_motion.as_ref()?;
                let end = clip.duration;
                let a = rm.sample(c.prev_time.min(end));
                let b = rm.sample(c.time.min(end));
                Some([b[0] - a[0], b[1] - a[1], b[2] - a[2], b[3] - a[3]])
            })
            .unwrap_or_default();
        let tae = &self.tae_runtime.frame;
        let turn_speed = if tae.flag(cancel::DISABLE_TURN) {
            0.0
        } else {
            tae.turn_speed.unwrap_or(DEFAULT_TURN_SPEED)
        };
        let velocity_change = tae
            .velocity_changes
            .iter()
            .find_map(|id| self.tae.velocity_changes.get(id).copied());
        let body_input = BodyInput {
            root_motion,
            steer_to: self.input.stick_world_yaw(),
            turn_speed,
            velocity_change,
        };
        self.body.step(&body_input, dt, self.ground.as_ref());

        let mut refs: Vec<i32> = tae.behavior_refs.iter().copied().collect();
        refs.sort_unstable();
        let mut flags: Vec<i32> = tae.flags.iter().copied().collect();
        flags.sort_unstable();
        StepReport {
            animation: main.as_ref().map(|c| c.animation.clone()),
            anim_time: main.map(|c| c.time).unwrap_or(0.0),
            behavior,
            position: self.body.position,
            yaw: self.body.yaw,
            grounded: self.body.grounded,
            sp_effects: tae.sp_effects.clone(),
            behavior_refs: refs,
            action_flags: flags,
            requested,
        }
    }

    /// Clip length lookup shared with the renderer.
    pub fn clip_duration(&mut self, animation: &str) -> Option<f32> {
        self.clips.duration(animation)
    }

    /// Steps idle (no input) until the character stands idle, up to `max_steps`.
    pub fn settle(&mut self, dt: f32, max_steps: usize) -> usize {
        for i in 0..max_steps {
            if self
                .behavior
                .runtime
                .main_state_path()
                .last()
                .is_some_and(|s| s == "StandIdle")
            {
                return i;
            }
            self.tick(&InputFrame::default(), dt);
        }
        max_steps
    }
}

//! A simulated character: script and behaviour graph, TAE events, the body, and its control
//! (the player's input or an NPC's brain).
//!
//! [`Character::tick`] runs one simulation step:
//!
//! 1. Control: the player's buffered presses, hold times and stick, or the NPC brain's
//!    requested action and movement.
//! 2. What the engine tells the script: movement variables, action requests gated by the
//!    previous step's TAE cancel windows, active behaviour reference ids, ground contact, lock-on
//!    and the hit being reacted to.
//! 3. The script and behaviour graph ([`PlayerBehavior::tick`]).
//! 4. TAE events of the clips now playing, which feed the next step's answers.
//! 5. The body: root motion of the full-body clip, steering at the TAE turn speed, jump
//!    velocity changes and gravity.

use crate::behavior::{AnimOffsets, ClipDurations};
use crate::body::{Body, BodyInput, FlatGround, Ground};
use crate::clips::ClipLibrary;
use crate::input::{InputFrame, InputState, angle_diff};
use crate::player::{IncomingDamage, LoadError, PlayerBehavior, TickReport};
use crate::tae::{PlayingClip, TaeDb, TaeFrame, TaeRuntime, cancel};
use glam::{Mat4, Vec3};
use sekiro_formats::anim::Skeleton;
use sekiro_formats::hkb::NodeId;
use std::collections::{HashMap, HashSet};
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
    /// True when the full-body clip started (or restarted) this step.
    pub clip_started: bool,
    /// Crossfade into the full-body clip in seconds (TAE `Blend`), when it just started.
    pub blend_in: f32,
}

/// What an NPC's brain asks of its body this step. The NPC script reads these as the engine's
/// AI interface: `env(106)` for the action and the `MoveSpeedLevel` / `MoveAngle` variables.
#[derive(Debug, Clone, Default)]
pub struct NpcControl {
    /// The requested action (env 106): an animation-like code such as 3000 for attack
    /// `a000_003000`, 9910 for guard, or 0 for none.
    pub action: i32,
    /// Movement speed level: 0 stop, up to 0.75 walk, above that run.
    pub move_level: f32,
    /// World yaw to move along, if moving.
    pub move_yaw: Option<f32>,
    /// World yaw to face (usually toward the target).
    pub face_yaw: Option<f32>,
    /// Behaviour reference ids the AI state contributes (battle, caution, ...).
    pub refs: HashSet<i32>,
}

/// Which brain drives the character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlKind {
    Player,
    Npc,
}

pub struct Character {
    pub chr: String,
    pub kind: ControlKind,
    pub behavior: PlayerBehavior,
    pub clips: ClipLibrary,
    pub tae: Arc<TaeDb>,
    pub tae_runtime: TaeRuntime,
    pub body: Body,
    /// Player input state (unused for NPCs).
    pub input: InputState,
    /// NPC control for the next step (unused for the player).
    pub npc: NpcControl,
    /// Lock-on target position, set by the world (player only).
    pub lock_target: Option<[f32; 3]>,
    /// A hit to react to on the next step; cleared after it.
    pub pending_damage: Option<IncomingDamage>,
    /// The Havok skeleton the clips animate (source space).
    pub skeleton: Arc<Skeleton>,
    pub ground: Box<dyn Ground>,
    /// Clip times before the last step, by clip node.
    last_times: HashMap<NodeId, (String, f32)>,
    last_main: Option<PlayingClip>,
}

/// The player.
pub type PlayerCharacter = Character;

fn load_skeleton(cache: &Path, chr: &str) -> Arc<Skeleton> {
    std::fs::read(cache.join(format!("anim/{chr}/skeleton.bin")))
        .ok()
        .and_then(|b| Skeleton::from_bytes(&b).ok())
        .map(Arc::new)
        .unwrap_or_else(|| {
            Arc::new(Skeleton {
                name: chr.to_owned(),
                bones: Vec::new(),
            })
        })
}

impl Character {
    fn assemble(
        cache: &Path,
        chr: &str,
        kind: ControlKind,
        scripts: &[&str],
        graph: &str,
        offsets: AnimOffsets,
    ) -> Result<Self, LoadError> {
        let tae = Arc::new(TaeDb::load_from_cache(cache, chr));
        let clips = ClipLibrary::new(cache.join("anim").join(chr));
        for (id, src) in tae.hkx_aliases() {
            clips.add_alias(id, src);
        }
        let script_paths: Vec<_> = scripts
            .iter()
            .map(|s| cache.join("raw/action/script").join(s))
            .collect();
        let mut behavior = PlayerBehavior::load_scripts(
            &script_paths,
            &cache.join("raw/chr").join(graph),
            Box::new(clips.clone()),
            offsets,
        )?;
        behavior.env.standby = Some(true);
        behavior.env.move_cancel = Some(true);
        behavior.env.existing_anims = std::rc::Rc::new(tae.anim_ids().collect());
        Ok(Self {
            chr: chr.to_owned(),
            kind,
            behavior,
            clips,
            tae,
            tae_runtime: TaeRuntime::default(),
            body: Body::default(),
            input: InputState::default(),
            npc: NpcControl::default(),
            lock_target: None,
            pending_damage: None,
            skeleton: load_skeleton(cache, chr),
            ground: Box::new(FlatGround::default()),
            last_times: HashMap::new(),
            last_main: None,
        })
    }

    /// Loads Wolf from the extracted cache.
    pub fn load_from_cache(cache: &Path) -> Result<Self, LoadError> {
        let scripts: Vec<&str> = sekiro_hks::PLAYER_SCRIPTS.to_vec();
        Self::assemble(
            cache,
            "c0000",
            ControlKind::Player,
            &scripts,
            "c0000.behbnd.d/c0000.hkx",
            AnimOffsets::player_default(),
        )
    }

    /// Loads an NPC such as `c1010` (Ashina soldier): the shared NPC script `c9997.hks`, then
    /// its own script, driven by the shared NPC behaviour graph its `behbnd` references.
    pub fn load_npc(cache: &Path, chr: &str) -> Result<Self, LoadError> {
        let own = format!("{chr}.hks");
        Self::assemble(
            cache,
            chr,
            ControlKind::Npc,
            &["c9997.hks", &own],
            &format!("{chr}.behbnd.d/c9997.hkx"),
            AnimOffsets::default(),
        )
    }

    /// The TAE state computed by the last step.
    pub fn tae_frame(&self) -> &TaeFrame {
        &self.tae_runtime.frame
    }

    /// World yaw from this character toward `p`.
    pub fn yaw_to(&self, p: [f32; 3]) -> f32 {
        let dx = p[0] - self.body.position[0];
        let dz = p[2] - self.body.position[2];
        (-dx).atan2(-dz)
    }

    fn set_engine_variables(&mut self) {
        let rt = &mut self.behavior.runtime;
        match self.kind {
            ControlKind::Player => {
                let level = self.input.stick_level();
                // Stick direction relative to facing, positive to the left.
                let turn = self
                    .input
                    .stick_world_yaw()
                    .map(|y| angle_diff(y, self.body.yaw).to_degrees())
                    .unwrap_or(0.0);
                // The engine's stick level runs 0..2; full keyboard or stick deflection runs.
                rt.set_variable("MoveSpeedLevel", level * 2.0);
                for name in [
                    "TurnAngle",
                    "TurnAngleReal",
                    "MoveAngle",
                    "MoveAngleReal",
                    "JumpAngle",
                    "AttackAngle",
                    "SubAttackAngle",
                    "KickAngle",
                ] {
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
            ControlKind::Npc => {
                // The NPC script reads MoveAngle with positive to the right (`GetMoveDir`).
                let angle = self
                    .npc
                    .move_yaw
                    .map(|y| -angle_diff(y, self.body.yaw).to_degrees())
                    .unwrap_or(0.0);
                rt.set_variable("MoveSpeedLevel", self.npc.move_level);
                rt.set_variable("MoveAngle", angle);
                rt.set_variable("TurnAngle", angle);
            }
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
        if self.kind == ControlKind::Npc {
            env.sp_effect_refs.extend(self.npc.refs.iter().copied());
            env.ai_action = self.npc.action;
        }
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
        env.locked_on = self.lock_target.is_some();
        env.damage = self.pending_damage.take();
    }

    /// Runs one step of `dt` seconds. `frame` is the player's input; NPCs read [`Self::npc`].
    pub fn tick(&mut self, frame: &InputFrame, dt: f32) -> StepReport {
        if self.kind == ControlKind::Player {
            self.input.update(frame, dt);
        }
        self.set_engine_variables();
        self.fill_env(dt);
        let requested: Vec<i32> = self.behavior.env.requested.iter().copied().collect();
        let resets_before = self.behavior.env.input_resets.get();

        let behavior = self.behavior.tick(dt);
        self.behavior.env.damage = None;

        if self.behavior.env.input_resets.get() != resets_before {
            self.input.reset_requests();
        }
        if let Some(deg) = self.behavior.env.face_turn.take() {
            self.body.yaw = angle_diff(self.body.yaw + deg.to_radians(), 0.0);
        }
        if self.behavior.env.face_lock_target.take()
            && let Some(t) = self.lock_target
        {
            self.body.yaw = self.yaw_to(t);
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
        let steer_to = match self.kind {
            // Locked on, the character keeps facing the target and strafes.
            ControlKind::Player => match self.lock_target {
                Some(t) => Some(self.yaw_to(t)),
                None => self.input.stick_world_yaw(),
            },
            ControlKind::Npc => self.npc.face_yaw,
        };
        let body_input = BodyInput {
            root_motion,
            steer_to,
            turn_speed,
            velocity_change,
        };
        self.body.step(&body_input, dt, self.ground.as_ref());

        let mut refs: Vec<i32> = tae.behavior_refs.iter().copied().collect();
        refs.sort_unstable();
        let mut flags: Vec<i32> = tae.flags.iter().copied().collect();
        flags.sort_unstable();
        let clip_started = main.as_ref().is_some_and(|c| c.prev_time == 0.0);
        let blend_in = main
            .as_ref()
            .filter(|_| clip_started)
            .and_then(|c| self.tae.blend_in(&c.animation))
            .unwrap_or(0.0);
        self.last_main = main.clone();
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
            clip_started,
            blend_in,
        }
    }

    /// Clip length lookup shared with the renderer.
    pub fn clip_duration(&mut self, animation: &str) -> Option<f32> {
        self.clips.duration(animation)
    }

    /// Source-space model matrices of every skeleton bone for the full-body clip as of the last
    /// step. Map points to the world with [`Self::source_to_world`].
    pub fn bone_source_matrices(&self) -> Vec<Mat4> {
        let Some(main) = &self.last_main else {
            return Vec::new();
        };
        let Some(clip) = self.clips.clip(&main.animation) else {
            return Vec::new();
        };
        let pose = clip.sample(main.time.min(clip.duration), &self.skeleton);
        self.skeleton.model_space(&pose)
    }

    /// Source model space to Bevy world space: mirror X, then place at the body.
    pub fn source_to_world(&self) -> Mat4 {
        Mat4::from_translation(Vec3::from_array(self.body.position))
            * Mat4::from_rotation_y(self.body.yaw)
            * Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0))
    }

    /// The name of the state the full-body machine is in.
    pub fn state(&self) -> String {
        self.behavior
            .runtime
            .main_state_path()
            .last()
            .cloned()
            .unwrap_or_default()
    }

    /// Steps without input until the character stands idle (`StandIdle` for the player, an
    /// idle state for NPCs), up to `max_steps`.
    pub fn settle(&mut self, dt: f32, max_steps: usize) -> usize {
        for i in 0..max_steps {
            let s = self.state();
            if s == "StandIdle" || (self.kind == ControlKind::Npc && s.starts_with("Idle")) {
                return i;
            }
            self.tick(&InputFrame::default(), dt);
        }
        max_steps
    }
}

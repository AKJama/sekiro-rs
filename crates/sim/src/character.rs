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
use crate::homing::{ACT_STOP_AUTO_AIM, ACT_UPDATE_AUTO_AIM, AutoAimLimits, reach_scale};
use crate::input::{InputFrame, InputState, angle_diff};
use crate::player::{IncomingDamage, LoadError, PlayerBehavior, TickReport};
use crate::tae::{PlayingClip, TaeDb, TaeFrame, TaeRuntime, cancel};
use glam::{Mat4, Vec3};
use sekiro_formats::anim::Skeleton;
use sekiro_formats::hkb::NodeId;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

/// Steering speed when no TAE `SetTurnSpeed` is active, degrees per second: the exe's turn
/// controller default (traced by FwLo's RustSekiro, `docs/kb/movement.md`).
pub const DEFAULT_TURN_SPEED: f32 = 720.0;

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
    /// Seconds this character's own clock advanced in the step: the step's length, less while a
    /// hit stop holds or eases it (0 while held). The renderer draws between steps with it.
    pub advanced: f32,
    /// How far the model is faded out by the playing clips' TAE 193 events: 0 fully drawn (the
    /// default), 1 not drawn at all (the Owl's warps; 1 minus [`crate::tae::TaeFrame::opacity`]).
    pub fade: f32,
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
    /// The action comes from a combo goal (one the scripts registered with
    /// `ENABLE_COMBO_ATK_CANCEL`), so a TAE "AI combo attack queued" window (flag 23) accepts it.
    pub combo: bool,
}

/// TAE action flags that end an NPC animation for a queued AI action: 23 combo attack, 86
/// attack, 79 step ("End If AI ... Queued" in the TAE template).
pub mod ai_cancel {
    pub const COMBO: i32 = 23;
    pub const STEP: i32 = 79;
    pub const ATTACK: i32 = 86;
}

/// Step actions of the shared NPC script (`a000_005xxx`).
fn is_step_action(action: i32) -> bool {
    (5000..6000).contains(&action)
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
    /// Living enemies the attack auto-aim may pick, set by the world before each step
    /// (player only).
    pub aim_candidates: Vec<[f32; 3]>,
    /// The attack auto-aim target, an index into [`Self::aim_candidates`] (player only).
    pub homing: Option<usize>,
    /// LockCamParam limits of the auto-aim.
    pub auto_aim: AutoAimLimits,
    /// The character an NPC fights (Wolf), set by the world; its attacks aim at it.
    pub npc_target: Option<[f32; 3]>,
    /// A hit to react to on the next step; cleared after it.
    pub pending_damage: Option<IncomingDamage>,
    /// The Havok skeleton the clips animate (source space).
    pub skeleton: Arc<Skeleton>,
    pub ground: Box<dyn Ground>,
    /// The lower-body twist in degrees (the engine's `TwistLowerRootAngle`, positive after a
    /// turn to the left): how far the visible body still lags behind [`Body::yaw`] after a
    /// free move start turned the character to the stick. Unwinds at the default turn speed.
    pub twist: f32,
    /// The locked-on walk twist in degrees, positive with the legs turned to the right (the
    /// exe's WalkTwist output, the graph's `TwistMasterAngle`): how far the legs and the
    /// travel turn off the directional walk or run clip playing so the move goes along the
    /// stick. Eases toward [`Self::walk_twist_target`] at [`WALK_TWIST_RATE`]. The renderer turns
    /// the Master bone by it and turns Spine..Spine2 back, so the chest keeps facing the target.
    pub walk_twist: f32,
    /// What the walk twist eases toward: the stick's angle from the clip's direction while a
    /// locked-on walk or run plays with the stick pushed, else 0. Root motion is turned by it.
    pub walk_twist_target: f32,
    /// World yaw of the last step's root-motion travel, when the clip carried the body faster
    /// than [`HEADING_MIN_SPEED`]; `TurnAngle` is measured from it (else from the facing).
    heading: Option<f32>,
    /// Clip times before the last step, by clip node.
    last_times: HashMap<NodeId, (String, f32)>,
    last_main: Option<PlayingClip>,
}

/// The locked-on strafe class for a stick `angle` degrees from the facing (positive right),
/// given the `current` class: forward within 55 degrees (65 while already forward), back past
/// 125 (115 while already back), otherwise left (2) or right (3).
fn strafe_direction(current: i32, angle: f32) -> i32 {
    let (front, back) = match current {
        0 => (65.0, 125.0),
        1 => (55.0, 115.0),
        _ => (55.0, 125.0),
    };
    if angle.abs() < front {
        0
    } else if angle.abs() > back {
        1
    } else if angle < 0.0 {
        2
    } else {
        3
    }
}

/// The clip direction of each locked-on strafe class (`MoveDirection` 0 forward, 1 back, 2
/// left, 3 right), degrees from the facing, positive right.
const STRAFE_CLIP_ANGLE: [f32; 4] = [0.0, 180.0, -90.0, 90.0];

/// How fast the walk twist eases toward its target, degrees per second: 0.1 rad per 1/30 s,
/// the exe's SpinJointBehavior on the Master joint in constant-rate mode (traced by FwLo's
/// RustSekiro, `docs/kb/movement.md`).
pub const WALK_TWIST_RATE: f32 = 0.1 * 30.0 * (180.0 / std::f32::consts::PI);

/// SpEffect behaviour reference ids the TAE puts on walk, run and sprint animations
/// (`SP_EF_REF_TAE_MOVING_WALK/RUN/SPRINT`; the script's `GetLocomotionType`).
const MOVING_REFS: [i32; 3] = [90, 91, 92];

/// Root-motion speed in m/s above which the travel direction, not the facing, is what
/// `TurnAngle` is measured from (a walk start already moves faster).
const HEADING_MIN_SPEED: f32 = 1.0;

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
            aim_candidates: Vec::new(),
            homing: None,
            auto_aim: AutoAimLimits::load(cache),
            npc_target: None,
            pending_damage: None,
            skeleton: load_skeleton(cache, chr),
            ground: Box::new(FlatGround::default()),
            twist: 0.0,
            walk_twist: 0.0,
            walk_twist_target: 0.0,
            heading: None,
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

    /// Where this character's attacks aim (the exe's attack target point): Wolf's lock-on
    /// target, else his attack auto-aim target; an NPC's combat target.
    pub fn attack_target(&self) -> Option<[f32; 3]> {
        match self.kind {
            ControlKind::Player => self.lock_target.or_else(|| {
                self.homing
                    .and_then(|i| self.aim_candidates.get(i).copied())
            }),
            ControlKind::Npc => self.npc_target,
        }
    }

    /// Runs the script and behaviour graph for one step, watching the script's auto-aim
    /// calls (`act(156)` refresh, `act(157)` stop) for the player.
    fn tick_behavior(&mut self, dt: f32) -> TickReport {
        if self.kind != ControlKind::Player {
            return self.behavior.tick(dt);
        }
        // Reuse the engine-call log to see the script's act calls, keeping any outer log.
        let outer = self.behavior.call_log.replace(Vec::new());
        let report = self.behavior.tick(dt);
        let calls = std::mem::replace(&mut self.behavior.call_log, outer).unwrap_or_default();
        let act = |id: i32| {
            calls.iter().any(|c| {
                c.function == "act" && c.args.first().and_then(|v| v.as_number()) == Some(id as f32)
            })
        };
        if act(ACT_STOP_AUTO_AIM) {
            self.homing = None;
        }
        if act(ACT_UPDATE_AUTO_AIM) && self.lock_target.is_none() {
            let aim = self.input.stick_world_yaw().unwrap_or(self.body.yaw);
            self.homing = self
                .auto_aim
                .pick(self.body.position, aim, &self.aim_candidates);
        }
        if let Some(log) = self.behavior.call_log.as_mut() {
            log.extend(calls);
        }
        report
    }

    /// World yaw from this character toward `p`.
    pub fn yaw_to(&self, p: [f32; 3]) -> f32 {
        let dx = p[0] - self.body.position[0];
        let dz = p[2] - self.body.position[2];
        (-dx).atan2(-dz)
    }

    /// The yaw the character is drawn with: the body's facing minus the lower-body twist that
    /// has not unwound yet, so a free move start turns the model smoothly while the body (and
    /// its root motion) already points along the stick.
    pub fn visual_yaw(&self) -> f32 {
        angle_diff(self.body.yaw - self.twist.to_radians(), 0.0)
    }

    /// Free movement (not locked on, no attack target): when the stick is pushed while Wolf is
    /// not already walking, running or sprinting and may start moving, the body turns to the
    /// stick at once and the turn becomes the lower-body twist. The script then sees the stick
    /// ahead (`TurnAngle` 0, so the forward move start) and the twist (`TwistLowerRootAngle`,
    /// so its 180 degree turn-and-go start past 135 degrees). The directional move starts,
    /// loops and stops it would otherwise pick from `TurnAngle` (back, left, right) are the
    /// locked-on strafes; played in free movement they carried Wolf sideways or backwards
    /// while steering turned him to face the stick.
    fn free_move_start_turn(&mut self) {
        let tae = &self.tae_runtime.frame;
        let moving = MOVING_REFS.iter().any(|r| tae.behavior_refs.contains(r));
        let ended = self
            .behavior
            .runtime
            .main_clip()
            .is_some_and(|c| c.at_end());
        let may_move = !tae.restricted || tae.flag(cancel::MOVE) || ended;
        // A TAE that stops turning (the sprint quick turn's wind-up) turns by root motion.
        let may_turn = !tae.flag(cancel::DISABLE_TURN) && tae.turn_speed != Some(0.0);
        if self.kind != ControlKind::Player
            || self.attack_target().is_some()
            || !self.body.grounded
            || moving
            || !may_move
            || !may_turn
        {
            return;
        }
        let Some(stick) = self.input.stick_world_yaw() else {
            return;
        };
        let turn = angle_diff(stick, self.body.yaw);
        self.body.yaw = angle_diff(stick, 0.0);
        self.twist = angle_diff((self.twist.to_radians()) + turn, 0.0).to_degrees();
    }

    /// The locked-on walk twist (exe WalkTwist, facts from FwLo's RustSekiro,
    /// `docs/kb/movement.md`): while a locked-on walk or run plays with the stick pushed, the
    /// script's directional clip covers only its 60 degree cone (`MoveDirection`), so the exe
    /// sets `TwistMasterAngle` to the stick's angle minus the clip's direction. The graph's
    /// twist modifiers turn the Master bone (the legs and the travel) by it and turn the spine
    /// back, so a diagonal runs along the stick with the chest on the target. Anything else
    /// (stick released, sprinting, attacking) lets it return to 0. Called after the TAE update,
    /// so the walk and run behaviour refs are this step's.
    fn update_walk_twist(&mut self, lock_angle_released: bool, dt: f32) {
        let tae = &self.tae_runtime.frame;
        let walking = tae.behavior_refs.contains(&MOVING_REFS[0])
            || tae.behavior_refs.contains(&MOVING_REFS[1]);
        let stick = self.input.stick_world_yaw();
        self.walk_twist_target = match stick {
            Some(stick)
                if self.kind == ControlKind::Player
                    && self.lock_target.is_some()
                    && !lock_angle_released
                    && self.body.grounded
                    && walking =>
            {
                let angle = -angle_diff(stick, self.body.yaw).to_degrees();
                let class = self
                    .behavior
                    .runtime
                    .variable("MoveDirection")
                    .unwrap_or(0.0) as usize;
                let clip = STRAFE_CLIP_ANGLE.get(class).copied().unwrap_or(0.0);
                angle_diff((angle - clip).to_radians(), 0.0).to_degrees()
            }
            _ => 0.0,
        };
        let step = WALK_TWIST_RATE * dt;
        self.walk_twist += (self.walk_twist_target - self.walk_twist).clamp(-step, step);
    }

    fn set_engine_variables(&mut self) {
        let lock_yaw = self.lock_target.map(|t| self.yaw_to(t));
        let rt = &mut self.behavior.runtime;
        match self.kind {
            ControlKind::Player => {
                rt.set_variable("TwistLowerRootAngle", self.twist);
                // Radians, positive with the legs turned right (FwLo's RustSekiro, actor.rs).
                rt.set_variable("TwistMasterAngle", self.walk_twist.to_radians());
                let level = self.input.stick_level();
                // Stick direction relative to `from` in degrees, positive to the RIGHT: the script
                // quick-turns right for TurnAngle > 0 and picks the right strafe for MoveAngle > 0.
                let stick_from = |from: f32| {
                    self.input
                        .stick_world_yaw()
                        .map(|y| -angle_diff(y, from).to_degrees())
                        .unwrap_or(0.0)
                };
                let turn = stick_from(self.body.yaw);
                // `TurnAngle` is measured from where the body is travelling (the facing when it
                // stands still or walks in place). In free movement they are the same. Locked on
                // they differ: a back step travels toward the stick while Wolf faces the target,
                // and the script brakes a locked sprint start when |TurnAngle| >= 135
                // (`BEH_A_GROUND_MOVE_STOP`, SprintStartFromStep reads TurnAngle). Measured from
                // the facing, the back sprint start (a000_001152, turning 180 degrees by root
                // motion) could never play: every locked back sprint braked into the sprint stop
                // slide toward the enemy. Inferred from the script and clips, not traced in the exe.
                let turn_from_travel = stick_from(self.heading.unwrap_or(self.body.yaw));
                // The locked-on action directions (step, jump, attack) are read right after the
                // script turns Wolf to the target (`act(2019)` in `_fireGroundStep` and the jump
                // starts), and pick a direction relative to that new facing, so they are measured
                // from the target. Measured from the old facing, a step out of a locked sprint or
                // quick turn (Wolf facing away) went the wrong way once he snapped to the target.
                let action = lock_yaw.map_or(turn, stick_from);
                // The engine's stick level runs 0..2; full keyboard or stick deflection runs.
                rt.set_variable("MoveSpeedLevel", level * 2.0);
                for name in ["TurnAngle", "TurnAngleReal"] {
                    rt.set_variable(name, turn_from_travel);
                }
                for name in ["MoveAngle", "MoveAngleReal"] {
                    rt.set_variable(name, turn);
                }
                for name in ["JumpAngle", "AttackAngle", "SubAttackAngle", "KickAngle"] {
                    rt.set_variable(name, action);
                }
                for name in [
                    "JumpStickLevel",
                    "AttackStickLevel",
                    "SubAttackStickLevel",
                    "KickStickLevel",
                ] {
                    rt.set_variable(name, level);
                }
                // Locked on, the engine keeps the strafe direction (`MoveDirection`, an engine
                // variable the move loop's selectors read: 0 forward, 1 back, 2 left, 3 right)
                // following the stick; the script only seeds it when a move starts. The exe's
                // WalkTwist classifier uses 60 degree cones with 5 degrees of hysteresis (traced
                // by FwLo's RustSekiro, `docs/kb/movement.md`). Without it, a strafe kept its
                // first direction: right, then left on the stick, still went right.
                if self.lock_target.is_some() && level > 0.0 {
                    let current = rt.variable("MoveDirection").unwrap_or(0.0) as i32;
                    rt.set_variable("MoveDirection", strafe_direction(current, turn) as f32);
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
            // Which cancel window offers the queued action to the attack's update (ours: the
            // window flag must be on and the request must be of the matching kind).
            let a = self.npc.action;
            let open = |flag: i32, ok: bool| if ok && a > 0 && tae.flag(flag) { a } else { 0 };
            env.ai_cancel_actions = [
                open(ai_cancel::COMBO, self.npc.combo),
                open(ai_cancel::ATTACK, !is_step_action(a)),
                open(ai_cancel::STEP, is_step_action(a)),
            ];
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
            self.input.update(frame, dt, &self.tae_runtime.frame);
            self.free_move_start_turn();
        }
        self.set_engine_variables();
        self.fill_env(dt);
        let requested: Vec<i32> = self.behavior.env.requested.iter().copied().collect();
        let resets_before = self.behavior.env.input_resets.get();

        let behavior = self.tick_behavior(dt);
        self.behavior.env.damage = None;

        if self.behavior.env.input_resets.get() != resets_before {
            self.input.reset_requests();
        }
        // The script always faces the target (act 2019) before tilting off it (act 3025): the
        // locked step and positioning jump tilts are relative to the target direction.
        if self.behavior.env.face_lock_target.take()
            && let Some(t) = self.lock_target
        {
            self.body.yaw = self.yaw_to(t);
        }
        if let Some(deg) = self.behavior.env.face_turn.take() {
            self.body.yaw = angle_diff(self.body.yaw - deg.to_radians(), 0.0);
        }
        let lock_angle_released = self.behavior.env.release_lock_angle.take();

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
        let mut root_motion = main
            .as_ref()
            .and_then(|c| {
                let clip = self.clips.clip(&c.animation)?;
                let rm = clip.root_motion.as_ref()?;
                let end = clip.duration;
                // A clip entered from the previous clip's end event starts after this step's
                // advance, at time 0. Taking its first `dt` of motion keeps the body moving
                // through the switch instead of standing still for a step (a hitch every time
                // the run start hands over to the run loop).
                let time = if c.prev_time == 0.0 && c.time == 0.0 {
                    dt
                } else {
                    c.time
                };
                let a = rm.sample(c.prev_time.min(end));
                let b = rm.sample(time.min(end));
                // Samples are displacements in the clip's start frame. Turn this step's delta
                // into the root's own frame (undo the clip's yaw so far); the body then applies
                // it along its facing. A clip that turns while moving (SprintStartFromStep_L
                // turns 90 degrees in 0.2 s and travels 5.8 m along its start frame's side) would
                // otherwise carry Wolf sideways once he has turned.
                let (dx, dz) = (b[0] - a[0], b[2] - a[2]);
                let (sin, cos) = (-a[3]).sin_cos();
                Some([
                    cos * dx + sin * dz,
                    b[1] - a[1],
                    -sin * dx + cos * dz,
                    b[3] - a[3],
                ])
            })
            .unwrap_or_default();
        // Locked-on walks and runs travel along the clip's motion turned by the walk twist, so
        // a diagonal goes along the stick, not along the nearest of the four strafe clips.
        // Turned by the target (the stick's angle off the clip) rather than the eased value:
        // when the class changes, the clip switches at once, and the eased twist would carry
        // the new clip off the stick until it caught up.
        self.update_walk_twist(lock_angle_released, dt);
        if self.walk_twist_target != 0.0 {
            // Source X is mirrored (as in `Body::step`), so a turn to the right is a positive
            // turn about source Y: x' = x cos + z sin, z' = -x sin + z cos.
            let (sin, cos) = self.walk_twist_target.to_radians().sin_cos();
            let [x, _, z, _] = root_motion;
            root_motion[0] = cos * x + sin * z;
            root_motion[2] = -sin * x + cos * z;
        }
        let tae = &self.tae_runtime.frame;
        // TAE 760: lengthen or shorten the attack's travel so it arrives at its target.
        let target = self.attack_target();
        if let (Some(r), Some(t)) = (tae.reach_target.as_ref(), target) {
            let scale = reach_scale(r, self.body.position, t);
            root_motion[0] *= scale;
            root_motion[2] *= scale;
        }
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
            // TAE 703: the swing keeps its direction (what makes side-steps work).
            _ if tae.fixed_rotation => None,
            // Locked on, the character keeps facing the target and strafes; an attack with an
            // auto-aim target turns toward it the same way. A sprint releases the lock-on angle
            // lock (act 2018) so Wolf turns to run along the stick, still locked on.
            ControlKind::Player => match target {
                Some(_) if lock_angle_released && self.lock_target.is_some() => {
                    self.input.stick_world_yaw()
                }
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
        // Where the clip carries the body: the root-motion offset from the facing (source X is
        // mirrored, as in `Body::step`), turned by the facing it is applied along.
        let [rx, _, rz, _] = root_motion;
        self.heading = (rx.hypot(rz) > HEADING_MIN_SPEED * dt && self.body.grounded)
            .then(|| angle_diff(self.body.yaw + rx.atan2(-rz), 0.0));
        self.body.step(&body_input, dt, self.ground.as_ref());
        // The lower body catches up with the facing at the default turn speed.
        let unwind = DEFAULT_TURN_SPEED * dt;
        self.twist -= self.twist.clamp(-unwind, unwind);

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
            advanced: dt,
            fade: 1.0 - tae.opacity(),
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

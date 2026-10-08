//! The player character driven by the game's own HKS scripts and behaviour graph.
//!
//! Each tick runs, in order:
//!
//! 1. `Update()`.
//! 2. `<State>_onUpdate()` for every active state that has such a hook.
//! 3. Queued behaviour events are applied; `_onDeactivate` hooks of left states and then
//!    `_onActivate` hooks of entered states run. Hooks may fire more events; this repeats up to
//!    [`MAX_EVENT_ROUNDS`] times.
//! 4. Clips that have ended send the end event their CMSG asks for (see
//!    [`BehaviorRuntime::queue_clip_end_events`]), and step 3 repeats for those.
//! 5. Active clips advance by `dt`.
//!
//! The order of steps 1 to 3 within a frame is our best reading of the engine, not verified.
//! Engine queries the runtime can answer (animation end, behaviour variables, node activity)
//! come from [`BehaviorRuntime`]; the rest come from [`PlayerEnv`], which a fuller simulation
//! will fill from TAE events, SpEffects, params and physics.

use crate::behavior::{AnimOffsets, BehaviorRuntime, ClipDurations, ClipState, HookCall};
use sekiro_formats::hkb::Graph;
use sekiro_hks::host::CallRecord;
use sekiro_hks::{CommandId, Host, PLAYER_SCRIPTS, Value, Vm, VmError};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const MAX_EVENT_ROUNDS: usize = 8;

// env ids, see docs/HKS-INTERFACE.md.
pub const ENV_LANDED: i32 = 201;
pub const ENV_ARM_STYLE: i32 = 207;
pub const ENV_WEAPON_CATEGORY: i32 = 225;
pub const ENV_REALLY_LANDED: i32 = 248;
pub const ENV_ANIM_END: i32 = 339;
pub const ENV_HP: i32 = 1000;
pub const ENV_STANDBY_STATE: i32 = 1105;
pub const ENV_ACTION_REQUEST: i32 = 1106;
pub const ENV_ACTION_HELD: i32 = 1108;
pub const ENV_MOVE_CANCEL: i32 = 2000;
pub const ENV_ACTION_UNLOCKED: i32 = 3033;
pub const ENV_SP_EFFECT_REF: i32 = 3036;
pub const ENV_CAN_RELEASE_CROUCH: i32 = 3037;
pub const ACT_SET_VARIABLE: i32 = 148;
pub const ACT_RESET_INPUT: i32 = 9101;
pub const ACT_FACE_DIRECTION: i32 = 3025;
pub const ENV_FALLING: i32 = 200;
pub const ENV_FALL_HEIGHT: i32 = 224;
pub const ENV_DT: i32 = 333;
pub const ENV_LOCKED_ON: i32 = 1118;

/// Action-arm ids used as the argument of env 1106 and 1108.
pub mod arm {
    pub const ATTACK: i32 = 0;
    pub const GUARD: i32 = 2;
    pub const JUMP: i32 = 4;
    pub const STEP: i32 = 5;
}

/// The engine-side facts the script asks about that the behaviour runtime cannot answer.
#[derive(Debug, Clone)]
pub struct PlayerEnv {
    pub hp: f32,
    pub landed: bool,
    /// Right and left hand weapon motion categories (50 is the katana).
    pub weapon_category: [i32; 2],
    /// env 207; 0 is sheathed (non-combat), 1 one-handed.
    pub arm_style: i32,
    /// Actions requested this frame (env 1106), by action-arm id.
    pub requested: HashSet<i32>,
    /// How long each action button has been held, in milliseconds (env 1108). The unit is
    /// unverified; the scripts only compare against zero and against each other.
    pub held_ms: HashMap<i32, f32>,
    /// Active behaviour reference ids (env 3036). Will come from TAE events and SpEffects.
    pub sp_effect_refs: HashSet<i32>,
    /// Unlocked action types (env 3033).
    pub unlocked: HashSet<i32>,
    /// Airborne and moving down (env 200).
    pub falling: bool,
    /// Height of the last fall in metres (env 224).
    pub fall_height: f32,
    /// Frame time in seconds (env 333).
    pub dt: f32,
    /// Lock-on active (env 1118).
    pub locked_on: bool,
    /// env 1105 ("standby state") from TAE cancel windows; `None` uses the clip-end
    /// approximation.
    pub standby: Option<bool>,
    /// env 2000 ("can cancel into movement") from TAE cancel windows; `None` as above.
    pub move_cancel: Option<bool>,
    /// Counts `act(9101)` (reset input acceptance) calls; the input layer clears its buffer.
    pub input_resets: std::cell::Cell<u32>,
    /// `act(3025, degrees)`: turn to face a direction relative to the current facing
    /// (positive to the left). The body applies and clears it.
    pub face_turn: std::cell::Cell<Option<f32>>,
    /// Fixed answers for any other `(id, first argument)`; `None` matches any argument.
    pub overrides: Vec<(i32, Option<i32>, f32)>,
}

impl Default for PlayerEnv {
    fn default() -> Self {
        Self {
            hp: 100.0,
            landed: true,
            weapon_category: [50, 70],
            arm_style: 1,
            requested: HashSet::new(),
            held_ms: HashMap::new(),
            sp_effect_refs: HashSet::new(),
            // Main weapon (5) and the basics; progression unlocks are a save-file matter.
            unlocked: [0, 1, 2, 3, 4, 5].into_iter().collect(),
            falling: false,
            fall_height: 0.0,
            dt: 1.0 / 30.0,
            locked_on: false,
            standby: None,
            move_cancel: None,
            input_resets: std::cell::Cell::new(0),
            face_turn: std::cell::Cell::new(None),
            overrides: Vec::new(),
        }
    }
}

/// What happened in one tick.
#[derive(Debug, Clone, Default)]
pub struct TickReport {
    pub frame: u64,
    pub state_path: Vec<String>,
    pub animation: Option<String>,
    pub anim_time: f32,
    pub events: Vec<String>,
    pub hooks: Vec<String>,
    pub script_errors: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("reading {0}: {1}")]
    Io(PathBuf, std::io::Error),
    #[error(transparent)]
    Format(#[from] sekiro_formats::Error),
    #[error(transparent)]
    Vm(#[from] VmError),
}

/// The engine side of the HKS interface for one character.
struct Adapter<'a> {
    rt: &'a mut BehaviorRuntime,
    env: &'a PlayerEnv,
    events: &'a mut Vec<String>,
    log: Option<&'a mut Vec<CallRecord>>,
    /// The state whose hook is running, if any.
    hook_state: Option<&'a str>,
}

fn num(v: Option<&Value>) -> Option<i32> {
    v.and_then(Value::as_number).map(|n| n as i32)
}

fn truth(b: bool) -> f32 {
    if b { 1.0 } else { 0.0 }
}

impl Adapter<'_> {
    fn answer_env(&self, id: CommandId<'_>, args: &[Value]) -> f32 {
        let CommandId::Number(id) = id else {
            // Named command 3064: the aging check. No aging in this simulation.
            return 0.0;
        };
        let arg = num(args.first());
        if let Some((_, _, v)) = self
            .env
            .overrides
            .iter()
            .find(|(i, a, _)| *i == id && (a.is_none() || *a == arg))
        {
            return *v;
        }
        let env = self.env;
        match id {
            ENV_FALLING => truth(env.falling),
            ENV_LANDED | ENV_REALLY_LANDED => truth(env.landed),
            ENV_FALL_HEIGHT => env.fall_height,
            ENV_DT => env.dt,
            ENV_LOCKED_ON => truth(env.locked_on),
            ENV_ARM_STYLE => env.arm_style as f32,
            ENV_WEAPON_CATEGORY => match arg {
                Some(1) => env.weapon_category[0] as f32,
                Some(0) => env.weapon_category[1] as f32,
                _ => 0.0,
            },
            ENV_ANIM_END => truth(self.rt.anim_ended(arg.unwrap_or(0), self.hook_state)),
            ENV_HP => env.hp,
            // Approximation until TAE cancel windows are wired: a state counts as standby once
            // its animation has ended or when it loops.
            ENV_STANDBY_STATE if env.standby.is_some() => truth(env.standby == Some(true)),
            ENV_MOVE_CANCEL if env.move_cancel.is_some() => truth(env.move_cancel == Some(true)),
            ENV_STANDBY_STATE | ENV_MOVE_CANCEL => truth(
                self.hook_state
                    .and_then(|s| self.rt.state_clip(s))
                    .or_else(|| self.rt.main_clip())
                    .is_some_and(|c| c.at_end() || c.looping),
            ),
            ENV_ACTION_REQUEST => truth(arg.is_some_and(|a| env.requested.contains(&a))),
            ENV_ACTION_HELD => arg
                .and_then(|a| env.held_ms.get(&a).copied())
                .unwrap_or(0.0),
            ENV_ACTION_UNLOCKED => truth(arg.is_some_and(|a| env.unlocked.contains(&a))),
            ENV_SP_EFFECT_REF => truth(arg.is_some_and(|a| env.sp_effect_refs.contains(&a))),
            ENV_CAN_RELEASE_CROUCH => 1.0,
            _ => 0.0,
        }
    }

    fn record(&mut self, function: &str, args: Vec<Value>, result: Vec<Value>) {
        if let Some(log) = self.log.as_deref_mut() {
            log.push(CallRecord {
                function: function.to_owned(),
                args,
                result,
            });
        }
    }
}

fn with_id(id: CommandId<'_>, args: &[Value]) -> Vec<Value> {
    let mut all = vec![match id {
        CommandId::Number(n) => Value::Number(n as f32),
        CommandId::Name(s) => Value::str(s),
    }];
    all.extend_from_slice(args);
    all
}

impl Host for Adapter<'_> {
    fn env(&mut self, id: CommandId<'_>, args: &[Value]) -> Value {
        let v = Value::Number(self.answer_env(id, args));
        self.record("env", with_id(id, args), vec![v.clone()]);
        v
    }

    fn act(&mut self, id: CommandId<'_>, args: &[Value]) -> Value {
        if id == CommandId::Number(ACT_FACE_DIRECTION)
            && let Some(deg) = args.first().and_then(Value::as_number)
        {
            self.env.face_turn.set(Some(deg));
        }
        if id == CommandId::Number(ACT_RESET_INPUT) {
            self.env.input_resets.set(self.env.input_resets.get() + 1);
        }
        if id == CommandId::Number(ACT_SET_VARIABLE)
            && let (Some(name), Some(value)) =
                (args.first(), args.get(1).and_then(Value::as_number))
        {
            self.rt.set_variable(&name.to_display(), value);
        }
        self.record("act", with_id(id, args), Vec::new());
        Value::Nil
    }

    fn call(&mut self, name: &str, args: &[Value]) -> Vec<Value> {
        let first = args.first().map(Value::to_display).unwrap_or_default();
        let result = match name {
            "hkbFireEvent" => {
                self.events.push(first.clone());
                self.rt.fire_event(&first);
                Vec::new()
            }
            "hkbGetVariable" => vec![Value::Number(self.rt.variable(&first).unwrap_or(0.0))],
            "hkbSetVariable" => {
                if let Some(v) = args.get(1).and_then(Value::as_number) {
                    self.rt.set_variable(&first, v);
                }
                Vec::new()
            }
            "hkbIsNodeActive" => vec![Value::Bool(self.rt.is_node_active(&first))],
            _ => Vec::new(),
        };
        self.record(name, args.to_vec(), result.clone());
        result
    }
}

pub struct PlayerBehavior {
    pub vm: Vm,
    pub runtime: BehaviorRuntime,
    pub env: PlayerEnv,
    durations: Box<dyn ClipDurations>,
    frame: u64,
    /// When set, every engine call of the next ticks is appended here.
    pub call_log: Option<Vec<CallRecord>>,
}

impl PlayerBehavior {
    /// Loads the player scripts from `script_dir`, the behaviour graph from `behavior_hkx` and
    /// clip lengths through `durations`, then runs `Initialize` and the initial state hooks.
    pub fn load(
        script_dir: &Path,
        behavior_hkx: &Path,
        durations: Box<dyn ClipDurations>,
    ) -> Result<Self, LoadError> {
        let read = |p: &Path| std::fs::read(p).map_err(|e| LoadError::Io(p.to_owned(), e));
        let graph: Graph = sekiro_formats::hkb::parse(&read(behavior_hkx)?)?;
        let runtime = BehaviorRuntime::new(Arc::new(graph), AnimOffsets::player_default());
        let mut me = Self {
            vm: Vm::new(),
            runtime,
            env: PlayerEnv::default(),
            durations,
            frame: 0,
            call_log: None,
        };
        me.vm.seed(1);
        let mut events = Vec::new();
        for chunk in PLAYER_SCRIPTS {
            let data = read(&script_dir.join(chunk))?;
            let mut host = Adapter {
                rt: &mut me.runtime,
                env: &me.env,
                events: &mut events,
                log: None,
                hook_state: None,
            };
            me.vm.load(&mut host, chunk, &data)?;
        }
        let mut errors = Vec::new();
        me.call(&[("Initialize".to_owned(), None)], &mut events, &mut errors);
        let hooks = me.runtime.initial_hooks();
        me.run_hooks(&hooks, &mut events, &mut errors, &mut Vec::new());
        me.settle(&mut events, &mut errors, &mut Vec::new());
        Ok(me)
    }

    /// Loads from the standard extracted cache layout under `cache_root`.
    pub fn load_from_cache(cache_root: &Path) -> Result<Self, LoadError> {
        Self::load(
            &cache_root.join("raw/action/script"),
            &cache_root.join("raw/chr/c0000.behbnd.d/c0000.hkx"),
            Box::new(
                crate::clips::ClipLibrary::new(cache_root.join("anim/c0000"))
                    .with_tae_dir(&cache_root.join("raw/chr/c0000.anibnd.d")),
            ),
        )
    }

    /// Calls each `(function, hook state)` that exists in the scripts.
    fn call(
        &mut self,
        calls: &[(String, Option<String>)],
        events: &mut Vec<String>,
        errors: &mut Vec<String>,
    ) {
        for (name, state) in calls {
            if !self.vm.has_function(name) {
                continue;
            }
            let mut host = Adapter {
                rt: &mut self.runtime,
                env: &self.env,
                events,
                log: self.call_log.as_mut(),
                hook_state: state.as_deref(),
            };
            if let Err(e) = self.vm.call_global(&mut host, name, &[]) {
                errors.push(format!("{name}: {e}"));
            }
        }
    }

    fn run_hooks(
        &mut self,
        hooks: &[HookCall],
        events: &mut Vec<String>,
        errors: &mut Vec<String>,
        ran: &mut Vec<String>,
    ) {
        let calls: Vec<(String, Option<String>)> = hooks
            .iter()
            .map(|h| (h.function_name(), Some(h.state.clone())))
            .filter(|(n, _)| self.vm.has_function(n))
            .collect();
        ran.extend(calls.iter().map(|(n, _)| n.clone()));
        self.call(&calls, events, errors);
    }

    fn settle(
        &mut self,
        events: &mut Vec<String>,
        errors: &mut Vec<String>,
        ran: &mut Vec<String>,
    ) {
        for _ in 0..MAX_EVENT_ROUNDS {
            if !self.runtime.has_pending_events() {
                break;
            }
            let hooks = self.runtime.process_events(self.durations.as_mut());
            self.run_hooks(&hooks, events, errors, ran);
        }
    }

    /// Runs one frame of `dt` seconds.
    pub fn tick(&mut self, dt: f32) -> TickReport {
        let mut events = Vec::new();
        let mut errors = Vec::new();
        let mut ran = Vec::new();
        self.call(&[("Update".to_owned(), None)], &mut events, &mut errors);
        let updates = self.runtime.update_hooks();
        self.run_hooks(&updates, &mut events, &mut errors, &mut ran);
        self.settle(&mut events, &mut errors, &mut ran);
        // Clips that ended and were not already handled by the script send their end events.
        if self.runtime.queue_clip_end_events() > 0 {
            events.push("(clip end)".to_owned());
            self.settle(&mut events, &mut errors, &mut ran);
        }
        self.runtime.advance(dt);
        let clip: Option<ClipState> = self.runtime.main_clip().cloned();
        let report = TickReport {
            frame: self.frame,
            state_path: self.runtime.main_state_path(),
            animation: clip.as_ref().map(|c| c.animation.clone()),
            anim_time: clip.map(|c| c.time).unwrap_or(0.0),
            events,
            hooks: ran,
            script_errors: errors,
        };
        self.frame += 1;
        report
    }

    /// The animation playing in the full-body slot.
    pub fn animation(&self) -> Option<&str> {
        self.runtime.main_clip().map(|c| c.animation.as_str())
    }
}

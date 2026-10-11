//! [`BrainDriver`]: runs a [`BossBrain`] alongside the boss's Lua AI in a duel.
//!
//! Timing model, per duel step:
//!
//! 1. Poll the controller. An answer is a *plan set* (one plan is a set of one): a weighted
//!    list of plans. The driver samples the next plan from it locally (the controller's
//!    probabilities, a penalty for options that were deflected or guarded repeatedly this
//!    fight and a bonus for untried ones, see [`super::sampler`]) and hands it to the Lua
//!    runtime's act hook, where it waits for the next *slot*: the next time his battle goal
//!    picks an act.
//! 2. The Lua AI thinks. At a slot the waiting plan is queued instead of the script's pick,
//!    unless the moment is a reaction; with no plan waiting, the script's own pick runs (the
//!    fallback: the boss never waits for a controller).
//! 3. After a slot, a request goes out according to the [`RequestMode`]: `slot` after every
//!    slot, `plan_end` when the last step of the running plan starts, `checkpoints` only at
//!    events (fight start, half HP, a lost deathblow mark, Wolf punishing twice or deflecting
//!    three times in a row; each once per fight, a lost mark each time). One request is in
//!    flight at a time. In `slot` and `plan_end` an answer is used for one plan; in
//!    `checkpoints` the set persists and the boss keeps cycling through it, sampled again at
//!    every slot, until the next answer.
//! 4. After the step, hits and action changes feed the [`Tally`], Wolf's habits and metrics.
//!
//! A single-shot plan whose answer is older than [`BrainDriver::max_staleness`] when it would
//! run is dropped and a fresh request is sent.
//!
//! A *fight* runs from the start (or a respawn) until either side dies. Its tally, habits and
//! record ([`FightRecord`]: Wolf's metrics, the choices made, tokens, latency) then go into the
//! [`Memory`] of every fight, which is saved to its file (if any) at the end of each fight,
//! every [`AUTOSAVE_SECS`] and when the driver is dropped (the game exits). Once a fighter is
//! dead nothing is requested or logged until the next fight starts.

use std::collections::{BTreeMap, HashSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::json;

use super::actions::BossActions;
use super::coach::{
    CallStats, CoachInput, CoachReply, CoachRequest, FightLine, GamePlan, INPUT_BUDGET, KEEP,
    SetStat,
};
use super::habits::{HabitTracker, WolfFrame};
use super::limits::{self, Pacing};
use super::memory::{
    BossKnowledge, FightEnd, LayerPaths, Loaded, Memory, MemorySummary, PlanFailures,
};
use super::metrics::{FightRecord, MetricsTracker, Trend, percentile};
use super::observation::observe;
use super::plan::{Plan, Step, Weighted};
use super::sampler::{self, Context, EXCLUDE_STREAK, Pick, Rng, Streaks};
use super::tally::{
    CountMap, Counts, Exchange, Side, Tally, TallyEvent, merge_counts, ranked_counts,
};
use super::{BossBrain, BrainReply, BrainRequest, REQUEST_BUDGET, RequestMode};
use crate::duel::{Duel, DuelReport, FLAG_PARRY_TIMING};
use crate::hits::FLAG_GUARD;
use crate::lua_ai::{ActSlot, AiBrain, anim_id_from_clip};

/// Seconds of fight between saves of an unfinished fight.
pub const AUTOSAVE_SECS: f32 = 15.0;

/// Estimated Wolf hits remaining: enough lead time to change the next exchange.
pub const PRESSURE_HITS: f32 = 6.0;
/// Urgent warning, still leaving at least two hits for a decision to matter.
pub const PRESSURE_CRITICAL_HITS: f32 = 3.0;
/// Below this estimate, no pressure warning leaves enough time to act.
pub const PRESSURE_MIN_HITS: f32 = 2.0;
/// Re-arm both warnings after remaining posture recovers above 70% of maximum.
pub const PRESSURE_REARM: f32 = 0.70;

/// Counters for tests and the end-of-fight summary.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BrainStats {
    pub requests: u32,
    pub replies: u32,
    /// Slots that ran the controller's plan.
    pub planned: u32,
    /// Slots where no plan was waiting, so the Lua AI's own act ran.
    pub fallback: u32,
    /// Slots the Lua AI kept as reactions.
    pub reflex: u32,
    /// Plans dropped as too old, and refused as invalid.
    pub stale: u32,
    pub invalid: u32,
    /// Fights that ended (a death), and memory saves that failed.
    pub fights: u32,
    pub save_errors: u32,
}

/// The plan waiting for its slot.
#[derive(Debug, Clone)]
struct Ready {
    /// When the answer behind it was asked for (a single-shot plan goes stale), or when it was
    /// picked (a set's plan).
    since: f32,
    plan: Plan,
    /// The game plan's opener (put back if it goes stale).
    opener: bool,
    /// Which part of the game plan it came from: `opener`, a set name, or `-`.
    label: String,
}

/// A coach request in flight.
#[derive(Debug, Clone, Copy)]
struct CoachFlight {
    id: u64,
    sent: f32,
    mid: bool,
    fight: u32,
}

/// A request in flight.
#[derive(Debug, Clone, Copy)]
struct InFlight {
    id: u64,
    sent: f32,
    fight: u32,
}

/// Which checkpoint events have fired this fight (see [`RequestMode::Checkpoints`]).
#[derive(Debug, Clone, Default)]
struct Checkpoints {
    wolf_thresholds: [bool; 3],
    wolf_broken: bool,
    previous_posture: f32,
    pressure: bool,
    pressure_critical: bool,
    half_hp: bool,
    marks: Option<i32>,
    punish_run: u32,
    punished_twice: bool,
    deflect_run: u32,
    deflected_thrice: bool,
}

impl Checkpoints {
    fn on_wolf(&mut self, posture: f32, broken: bool) -> (Vec<&'static str>, bool) {
        if posture < 0.15 {
            self.wolf_thresholds = [false; 3];
        }
        let edge = broken && !self.wolf_broken;
        self.wolf_broken = broken;
        let mut events = Vec::new();
        for (i, (level, name)) in [
            (0.25, "wolf_posture_25"),
            (0.5, "wolf_posture_50"),
            (0.75, "wolf_posture_75"),
        ]
        .into_iter()
        .enumerate()
        {
            if posture >= level && !self.wolf_thresholds[i] {
                self.wolf_thresholds[i] = true;
                events.push(name);
            }
        }
        if edge {
            events.push("wolf_broken");
        }
        (events, edge)
    }
    fn on_vitals(&mut self, posture: f32, hp: f32, hits: Option<f32>) -> Option<&'static str> {
        let growing = posture > self.previous_posture;
        if 1.0 - posture > PRESSURE_REARM && 1.0 - self.previous_posture <= PRESSURE_REARM {
            self.pressure = false;
            self.pressure_critical = false;
        }
        self.previous_posture = posture;
        if growing
            && !self.pressure_critical
            && hits.is_some_and(|h| (PRESSURE_MIN_HITS..=PRESSURE_CRITICAL_HITS).contains(&h))
        {
            self.pressure_critical = true;
            self.pressure = true;
            Some("pressure_critical")
        } else if growing
            && !self.pressure
            && hits.is_some_and(|h| (PRESSURE_MIN_HITS..=PRESSURE_HITS).contains(&h))
        {
            self.pressure = true;
            Some("pressure")
        } else if !self.half_hp && hp <= 0.5 {
            self.half_hp = true;
            Some("half_hp")
        } else {
            None
        }
    }

    /// A hit resolved: the checkpoint it raises, if any.
    fn on_hit(&mut self, attacker: Side, guarded: bool, deflected: bool) -> Option<&'static str> {
        match attacker {
            Side::Wolf if !guarded && !deflected => {
                self.punish_run += 1;
                if self.punish_run >= 2 && !self.punished_twice {
                    self.punished_twice = true;
                    return Some("punished_twice");
                }
            }
            Side::Boss => {
                let mut raised = None;
                if deflected {
                    self.deflect_run += 1;
                    if self.deflect_run >= 3 && !self.deflected_thrice {
                        self.deflected_thrice = true;
                        raised = Some("deflected_thrice");
                    }
                } else {
                    self.deflect_run = 0;
                }
                if !guarded && !deflected {
                    self.punish_run = 0;
                }
                return raised;
            }
            Side::Wolf => {}
        }
        None
    }
}

/// What is counted about the fight in progress.
#[derive(Debug, Clone, Default)]
struct FightState {
    break_plans: CountMap,
    action_ranges: BTreeMap<String, (u32, f32, f32)>,
    uses: Vec<PlanUse>,
    failures: BTreeMap<String, PlanFailures>,
    /// Completed melee samples: count, sum of starting distances, sum of measured reaches.
    ranges: BTreeMap<String, (u32, f32, f32)>,
    range_open: Option<AttackRange>,
    metrics: MetricsTracker,
    requests: u32,
    decisions: u32,
    lua_acts: u32,
    choices: BTreeMap<String, u32>,
    /// Request to answer, milliseconds.
    latencies: Vec<f32>,
    tokens_at_start: (u64, u64),
    stats_at_start: [CallStats; 2],
    /// Plan runs `(start, plan text, set label)` in order, for attributing exchanges.
    runs: Vec<(f32, String, String)>,
    /// Outcomes per plan text and per set label this fight.
    by_plan: CountMap,
    by_set: BTreeMap<String, Counts>,
    /// Seconds each set was the active one.
    set_secs: BTreeMap<String, f32>,
}

#[derive(Debug, Clone)]
struct PlanUse {
    break_response: bool,
    failed: bool,
    text: String,
    start: f32,
    last_attack: Option<f32>,
    attacks: u32,
    deflected: u32,
    fatal: bool,
    settled: bool,
}

#[derive(Debug, Clone)]
struct AttackRange {
    action: String,
    plan: String,
    start: [f32; 3],
    distance: f32,
    reach: Option<f32>,
}

impl FightState {
    fn close_range(&mut self) {
        if let Some(sample) = self.range_open.take()
            && let Some(reach) = sample.reach
        {
            let (n, distance, total_reach) = self.ranges.entry(sample.plan).or_default();
            *n += 1;
            *distance += sample.distance;
            *total_reach += reach;
            let (n, distance, total_reach) = self.action_ranges.entry(sample.action).or_default();
            *n += 1;
            *distance += sample.distance;
            *total_reach += reach;
        }
    }
}

pub struct BrainDriver {
    overlay_events: OverlayEvents,
    session_records: Vec<FightRecord>,
    session_actions: CountMap,
    session_ranges: BTreeMap<String, (u32, f32, f32)>,
    session_report_path: Option<PathBuf>,
    break_pending: bool,
    cancel_break: bool,
    wolf_events: std::collections::VecDeque<&'static str>,
    banned: HashSet<String>,
    bans_at_start: HashSet<String>,
    retry_baselines: BTreeMap<String, PlanFailures>,
    brain: Box<dyn BossBrain>,
    actions: &'static BossActions,
    /// This fight's tally.
    pub tally: Tally,
    /// Wolf's habits in this fight.
    pub habits: HabitTracker,
    /// Every earlier fight (loaded from the memory file, plus the fights of this session).
    pub memory: Memory,
    pub boss_knowledge: BossKnowledge,
    pub player: String,
    layer_paths: Option<LayerPaths>,
    memory_path: Option<PathBuf>,
    last_save: f32,
    /// The last step's length, seconds.
    step_dt: f32,
    /// The fight ended (a death) and the respawn has not come yet.
    fight_over: bool,
    /// Fights started so far (an answer to a request of an earlier fight is dropped).
    fight_no: u32,
    pub stats: BrainStats,
    /// Seconds a single-shot answer may age before its plan runs; older plans are dropped.
    pub max_staleness: f32,
    /// Session clock, seconds.
    pub time: f32,
    /// When requests go out.
    pub mode: RequestMode,
    /// Sampling temperature (see [`super::sampler`]); `SEKIRO_BRAIN_TEMPERATURE` sets it.
    pub temperature: f32,
    rng: Rng,
    streaks: Streaks,
    /// Plan texts chosen this fight.
    tried: HashSet<String>,
    in_flight: Option<InFlight>,
    /// The controller's latest answer, sampled for each act.
    set: Vec<Weighted>,
    /// When the set's request was sent.
    set_time: f32,
    /// A set stays for many picks (checkpoints) rather than one.
    persist: bool,
    ready: Option<Ready>,
    /// A pick (a plan or the Lua AI's act) is waiting for the next slot.
    picked: bool,
    /// `slot` and `plan_end`: a request is due.
    want_request: bool,
    /// `checkpoints`: the event that makes the next request due.
    pending_why: Option<&'static str>,
    checkpoints: Checkpoints,
    next_id: u64,
    /// The plan running now (empty: the Lua AI's own act).
    pub running: Plan,
    /// Every plan that started, in order, for tests.
    pub history: Vec<Plan>,
    /// Seconds since each side last landed an unguarded hit `[boss, wolf]`.
    since_hit: [Option<f32>; 2],
    boss_open: Option<i32>,
    wolf_open: Option<String>,
    /// A tactic's exchange is open in the tally.
    tactic_open: bool,
    fight: FightState,
    /// The record of the last finished fight, for tests and the summary.
    pub last_record: Option<FightRecord>,
    log: Option<BufWriter<File>>,
    log_path: Option<PathBuf>,
    /// The controller is a coach (see [`super::coach`]).
    coach_mode: bool,
    /// Apply the hard limits of [`super::limits`] when sampling plans (on for hosted brains).
    pub anti_stall: bool,
    /// Attack and retreat timing for those limits.
    pub pacing: Pacing,
    /// The game plan being played (coach mode).
    pub game_plan: Option<GamePlan>,
    /// The newest plan that arrived; played from the next fight on.
    next_plan: Option<GamePlan>,
    /// Index of the plan set being played.
    active_set: Option<usize>,
    /// When the active set became active.
    set_since: f32,
    /// The opener still to run this fight.
    opener: Option<Plan>,
    coach_flights: Vec<CoachFlight>,
    next_coach_id: u64,
    /// The id of the newest coach reply used, so an older one never replaces it.
    newest_coach: u64,
    /// Coach calls that completed since the fight began: calls, tokens, milliseconds.
    coach_done: [u64; 5],
    pub coach_interval: f32,
    last_coach_sent: f32,
    review_breaks: u32,
    mid_pending: Option<&'static str>,
    post_pending: Option<CoachInput>,
    mid_plan: Option<GamePlan>,
    staged_mid: Option<GamePlan>,
    mid_done: [u64; 2],
    set_banned_seen: bool,
}

/// Text and transient cards for the game's recording HUD.
#[derive(Debug, Default, Clone)]
pub struct BrainOverlay {
    pub fight: u32,
    pub set: String,
    pub running: String,
    pub label: String,
    pub checkpoint: Option<String>,
    pub coach: Option<String>,
    pub thinking: bool,
    pub principles: Vec<String>,
    pub ban: Option<String>,
    pub scorecard: Option<String>,
}

#[derive(Default)]
struct OverlayEvents {
    fights_before: u32,
    step: usize,
    checkpoint: Option<(f32, String)>,
    coach: Option<(f32, String)>,
    ban: Option<(f32, String)>,
    scorecard: Option<(f32, String)>,
}

impl BrainDriver {
    pub fn new(brain: Box<dyn BossBrain>, actions: &'static BossActions) -> Self {
        let env = |name: &str| std::env::var(name).ok();
        let coach_mode = brain.coaches();
        let anti_stall = brain.default_mode() == RequestMode::Checkpoints;
        let mode = if coach_mode {
            RequestMode::Checkpoints
        } else {
            env("SEKIRO_BRAIN_MODE")
                .and_then(|m| RequestMode::by_name(&m))
                .unwrap_or_else(|| brain.default_mode())
        };
        let temperature = env("SEKIRO_BRAIN_TEMPERATURE")
            .and_then(|t| t.parse().ok())
            .unwrap_or(1.0);
        let seed = env("SEKIRO_BRAIN_SEED")
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(1, |d| d.as_nanos() as u64)
            });
        Self {
            overlay_events: OverlayEvents::default(),
            session_records: Vec::new(),
            session_actions: Default::default(),
            session_ranges: Default::default(),
            session_report_path: None,
            break_pending: false,
            cancel_break: false,
            wolf_events: Default::default(),
            banned: HashSet::new(),
            bans_at_start: HashSet::new(),
            retry_baselines: BTreeMap::new(),
            brain,
            actions,
            tally: Tally::default(),
            habits: HabitTracker::new(actions.back_steps),
            memory: Memory::new(actions.chr),
            boss_knowledge: BossKnowledge::new(actions.chr),
            player: "default".into(),
            layer_paths: None,
            memory_path: None,
            last_save: 0.0,
            step_dt: 0.0,
            fight_over: false,
            fight_no: 1,
            stats: BrainStats::default(),
            max_staleness: 6.0,
            time: 0.0,
            mode,
            temperature,
            rng: Rng::new(seed),
            streaks: Streaks::default(),
            tried: HashSet::new(),
            in_flight: None,
            set: Vec::new(),
            set_time: 0.0,
            persist: false,
            ready: None,
            picked: false,
            want_request: true,
            pending_why: (!coach_mode).then_some("start"),
            checkpoints: Checkpoints::default(),
            next_id: 1,
            running: Plan::default(),
            history: Vec::new(),
            since_hit: [None, None],
            boss_open: None,
            wolf_open: None,
            tactic_open: false,
            fight: FightState::default(),
            last_record: None,
            log: None,
            log_path: None,
            coach_mode,
            anti_stall,
            pacing: Pacing::new(0.0),
            game_plan: None,
            next_plan: None,
            active_set: None,
            set_since: 0.0,
            opener: None,
            coach_flights: Vec::new(),
            next_coach_id: 1,
            newest_coach: 0,
            coach_done: [0; 5],
            coach_interval: super::coach::MID_COACH_INTERVAL,
            last_coach_sent: 0.0,
            review_breaks: 0,
            mid_pending: None,
            post_pending: None,
            mid_plan: None,
            staged_mid: None,
            mid_done: [0; 2],
            set_banned_seen: false,
        }
    }

    pub fn brain_name(&self) -> &'static str {
        self.brain.name()
    }

    /// A read-only recording view, independent of logging and persistence.
    pub fn overlay(&self) -> BrainOverlay {
        let events = &self.overlay_events;
        let plan = self.memory.coach.plan.as_ref().or(self.game_plan.as_ref());
        let set = self
            .game_plan
            .as_ref()
            .and_then(|p| p.sets.get(self.active_set?));
        let step = self.running.steps.get(events.step);
        let label = match step {
            Some(Step::Act(id)) => self
                .actions
                .actions
                .iter()
                .find(|a| a.id == self.boss_open.unwrap_or(*id))
                .or_else(|| self.actions.actions.iter().find(|a| a.id == *id))
                .map(|a| a.label.to_string())
                .unwrap_or_default(),
            Some(Step::Tactic(t)) => t.label(),
            None => self
                .boss_open
                .and_then(|id| self.actions.actions.iter().find(|a| a.id == id))
                .map(|a| a.label.to_string())
                .unwrap_or_default(),
        };
        let running = if self.running.is_empty() {
            "Lua".into()
        } else {
            self.running
                .steps
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let text = match s {
                        Step::Act(id) => id.to_string(),
                        Step::Tactic(t) => t.token(),
                    };
                    if i == events.step {
                        format!("[{text}]")
                    } else {
                        text
                    }
                })
                .collect::<Vec<_>>()
                .join(" > ")
        };
        let recent = |event: &Option<(f32, String)>, seconds: f32| {
            event
                .as_ref()
                .filter(|(t, _)| self.time - t < seconds)
                .map(|(_, text)| text.clone())
        };
        BrainOverlay {
            fight: self.overlay_events.fights_before + self.fight_no,
            set: set
                .map(|s| format!("{}: {}", s.name, s.description))
                .unwrap_or_else(|| {
                    if self.brain.name() == "scripted" {
                        "scripted: deterministic test plans".into()
                    } else if self.set.is_empty() {
                        "Lua".into()
                    } else {
                        "Controller plan set".into()
                    }
                }),
            running,
            label,
            checkpoint: events
                .checkpoint
                .as_ref()
                .map(|(t, text)| format!("{text} · {:.1} s ago", self.time - t)),
            coach: recent(&events.coach, 8.0),
            thinking: !self.coach_flights.is_empty(),
            principles: plan
                .map(|p| {
                    p.boss_principles
                        .iter()
                        .map(|s| format!("Boss · {s}"))
                        .chain(p.principles.iter().map(|s| format!("Player · {s}")))
                        .collect()
                })
                .unwrap_or_default(),
            ban: recent(&events.ban, 5.0),
            scorecard: recent(&events.scorecard, 6.0),
        }
    }

    pub fn actions(&self) -> &'static BossActions {
        self.actions
    }

    /// Seeds the sampler (tests; `SEKIRO_BRAIN_SEED` otherwise).
    pub fn seed(&mut self, seed: u64) {
        self.rng = Rng::new(seed);
    }

    /// Remembers across sessions in `path` (`memory::default_path` is
    /// `logs/memory/<chr>.json`): loads it now (a missing or unusable file starts fresh, with a
    /// line on stderr) and saves to it from then on.
    pub fn use_memory(&mut self, path: PathBuf) -> Loaded {
        self.use_player_memory(path, "default")
    }

    pub fn use_player_memory(&mut self, path: PathBuf, player: &str) -> Loaded {
        let chr = self.actions.chr;
        if let Err(e) = super::memory::player_name(player) {
            return Loaded::Corrupt(e);
        }
        let paths = LayerPaths::new(&path, player);
        let existed = paths.player.exists();
        let (boss, memory) = match paths.load(chr) {
            Ok(layers) => layers,
            Err(e) => {
                eprintln!("warning: memory layers: {e}; persistence disabled");
                return Loaded::Corrupt(e.to_string());
            }
        };
        let how = if existed || memory.fights > 0 {
            Loaded::Read
        } else {
            Loaded::Missing
        };
        self.player = player.into();
        self.boss_knowledge = boss;
        self.layer_paths = Some(paths);
        eprintln!(
            "boss knowledge: {} fights from {} players; player profile: {} ({} fights)",
            self.boss_knowledge.evidence.fights,
            self.boss_knowledge.summary().players,
            player,
            memory.fights
        );
        match &how {
            Loaded::Read => eprintln!(
                "brain memory: {} earlier fights against {chr} from {}",
                memory.fights,
                path.display()
            ),
            Loaded::Missing => eprintln!(
                "brain memory: none yet at {}, starting fresh",
                path.display()
            ),
            Loaded::Corrupt(why) => eprintln!(
                "warning: brain memory {} is unusable ({why}); starting fresh, the old file is kept as {}.corrupt",
                path.display(),
                path.display()
            ),
        }
        self.memory = memory;
        self.overlay_events.fights_before = self.memory.fights;
        self.apply_boss_bans();
        for (text, f) in &self.memory.coach.failures {
            if f.banned() {
                self.banned.insert(text.clone());
                eprintln!(
                    "banned: {text} ({} uses, {:.0}% deflected/fatal/break failure)",
                    f.uses,
                    f.rate() * 100.0
                );
            }
        }
        self.bans_at_start = self.banned.clone();
        self.memory_path = Some(path);
        if self.coach_mode {
            // The newest plan of an earlier session plays from the first fight (validated
            // again: the file may be from another build).
            let saved = self
                .memory
                .coach
                .plan
                .take()
                .or_else(|| self.boss_knowledge.starter());
            if let Some(saved) = saved {
                let v = saved.validate(self.actions);
                for w in &v.warnings {
                    eprintln!("coach: saved game plan: {w}");
                }
                if v.usable() {
                    self.overlay_events.coach = Some((
                        self.time,
                        format!("coach (pre-fight, 0 s): {}", v.plan.notes),
                    ));
                    self.memory.coach.plan = Some(v.plan.clone());
                    self.game_plan = Some(v.plan);
                }
            }
            self.install_plan();
            if self.game_plan.is_some() {
                eprintln!(
                    "coach: playing the saved game plan from fight {} on",
                    self.memory.fights + 1
                );
            }
        }
        how
    }

    /// Coach mode: sets up the fight that is starting from [`Self::game_plan`]: the opener, the
    /// start set, and the `start` checkpoint (with no plan nothing is requested and his own AI
    /// plays).
    fn install_plan(&mut self) {
        self.opener = None;
        self.active_set = None;
        self.set.clear();
        self.pending_why = None;
        let Some(plan) = self.game_plan.clone() else {
            return;
        };
        self.persist = true;
        self.opener = plan.opener_plan(self.actions);
        if let Some(i) = plan.start_index() {
            self.active_set = Some(i);
            self.set = plan.weighted(i, self.actions);
            self.set_since = self.time;
            self.pending_why = Some("start");
        }
    }

    /// Name of the plan set being played (coach mode).
    pub fn active_set_name(&self) -> Option<&str> {
        let plan = self.game_plan.as_ref()?;
        Some(plan.sets.get(self.active_set?)?.name.as_str())
    }

    /// Switches to the named set (coach mode); the next act is sampled from it. An opener
    /// still waiting stays.
    fn switch_set(&mut self, name: &str) -> bool {
        let Some(plan) = self.game_plan.as_ref() else {
            return false;
        };
        let Some(i) = plan.set_index(name) else {
            return false;
        };
        if self.active_set == Some(i) {
            return true;
        }
        let set = plan.weighted(i, self.actions);
        self.close_set_time();
        self.active_set = Some(i);
        self.set = set;
        if self
            .ready
            .as_ref()
            .is_none_or(|r| !r.opener && r.label != "on_wolf_break")
        {
            self.ready = None;
            self.picked = false;
        }
        true
    }

    /// Adds the time the active set has been played to this fight's seconds.
    fn close_set_time(&mut self) {
        if let Some(name) = self.active_set_name().map(str::to_string) {
            *self.fight.set_secs.entry(name).or_default() += (self.time - self.set_since).max(0.0);
        }
        self.set_since = self.time;
    }

    pub fn memory_path(&self) -> Option<&Path> {
        self.memory_path.as_deref()
    }

    fn apply_boss_bans(&mut self) {
        for ban in &self.boss_knowledge.bans {
            if self.banned.insert(ban.choice.clone()) {
                self.overlay_events.ban = Some((
                    self.time,
                    format!(
                        "banned {}: {}",
                        ban.choice,
                        ban.reason.trim_start_matches("shared: ")
                    ),
                ));
            }
            for choice in self.actions.choices() {
                if choice
                    .value
                    .split(['>', '+'])
                    .any(|step| step == ban.choice)
                {
                    self.banned.insert(choice.value);
                }
            }
        }
    }

    /// Enable exit/F9 reports in the game; offline tests do not write reports.
    pub fn enable_session_report(&mut self, logs: &Path) {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        self.session_report_path = Some(logs.join(format!("brain-session-{stamp}.md")));
    }

    pub fn session_report(&self) -> String {
        let mut actions = self.session_actions.clone();
        let mut ranges = self.session_ranges.clone();
        if !self.fight_over && self.fight.metrics.player_active(self.time) {
            let (tally, _) = self.fight_snapshot();
            merge_counts(&mut actions, tally.all(Side::Boss));
            for (key, &(n, distance, reach)) in &self.fight.action_ranges {
                let entry = ranges.entry(key.clone()).or_default();
                entry.0 += n;
                entry.1 += distance;
                entry.2 += reach;
            }
        }
        let mut report = format!(
            "# Brain session: {}\n\n| action | n | hit | blocked | deflected | whiffed | punished | distance / reach (m) |\n|---|---:|---:|---:|---:|---:|---:|---|\n",
            self.actions.chr
        );
        for (key, c) in actions {
            let range = ranges.get(&key).map_or("-".into(), |&(n, d, r)| {
                format!("{:.2} / {:.2}", d / n as f32, r / n as f32)
            });
            report.push_str(&format!(
                "| {key} | {} | {} | {} | {} | {} | {} | {range} |\n",
                c.n, c.hit, c.blocked, c.deflected, c.whiffed, c.punished
            ));
        }
        report.push_str("\n| fight | winner | duration (s) | time to kill Wolf (s) | Wolf posture breaks | hits after break | break punish rate | Wolf posture peak | Wolf damage/min | boss damage taken/min | passive share |\n|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");
        for (i, r) in self.session_records.iter().enumerate() {
            let m = r.metrics;
            report.push_str(&format!(
                "| {} | {} | {:.1} | {} | {} | {} | {:.0}% | {:.0}% | {:.0}% | {:.0}% | {:.0}% |\n",
                i + 1,
                r.winner,
                m.duration,
                m.time_to_kill_wolf
                    .map_or("-".into(), |t| format!("{t:.1}")),
                m.wolf_posture_breaks,
                m.hits_after_break,
                m.break_punish_rate * 100.0,
                m.wolf_posture_peak * 100.0,
                m.damage_per_min(),
                m.boss_damage_taken_per_min(),
                m.passive_share() * 100.0
            ));
        }
        if !self.fight_over && self.fight.metrics.player_active(self.time) {
            let m = self.fight.metrics.snapshot(self.time);
            report.push_str(&format!(
                "| current | unfinished | {:.1} | - | {} | {} | {:.0}% | {:.0}% | {:.0}% | {:.0}% | {:.0}% |\n",
                m.duration,
                m.wolf_posture_breaks,
                m.hits_after_break,
                m.break_punish_rate * 100.0,
                m.wolf_posture_peak * 100.0, m.damage_per_min(), m.boss_damage_taken_per_min(), m.passive_share() * 100.0
            ));
        }
        report
    }

    pub fn print_session_report(&self) -> std::io::Result<()> {
        let report = self.session_report();
        eprintln!("{report}");
        if let Some(path) = &self.session_report_path {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?;
            file.write_all(report.as_bytes())?;
        }
        Ok(())
    }

    /// The memory as it would be saved now: every earlier fight, plus this one as unfinished
    /// if anything happened in it.
    pub fn memory_now(&self) -> Memory {
        let mut m = self.memory.clone();
        if !self.fight_over && !self.tally.is_empty() && self.fight.metrics.player_active(self.time)
        {
            let (tally, habits) = self.fight_snapshot();
            m.add_fight(&tally, &habits.habits, FightEnd::Unfinished);
        }
        m
    }

    /// Saves [`Self::memory_now`] to the memory file, if there is one.
    pub fn save_memory(&mut self) {
        self.last_save = self.time;
        let Some(path) = self.memory_path.clone() else {
            return;
        };
        let now = self.memory_now();
        self.boss_knowledge.update(&self.player, &self.memory);
        self.apply_boss_bans();
        let saved = self.layer_paths.as_ref().map_or_else(
            || now.save(&path).map(|()| self.boss_knowledge.clone()),
            |p| p.save(&self.boss_knowledge, &now),
        );
        match saved {
            Ok(boss) => {
                self.boss_knowledge = boss;
                self.apply_boss_bans();
            }
            Err(e) => {
                self.stats.save_errors += 1;
                eprintln!("brain memory: saving {} failed: {e}", path.display());
            }
        }
    }

    /// What a request carries: every fight so far, this one included (the fight count and
    /// wins are the earlier fights'), and Wolf's recent play.
    pub fn memory_summary(&self) -> MemorySummary {
        let mut m = self.memory.clone();
        if !self.fight_over && self.fight.metrics.player_active(self.time) {
            let (tally, habits) = self.fight_snapshot();
            m.add_data(&tally, &habits.habits);
        }
        let now = (!self.fight_over).then(|| self.fight.metrics.snapshot(self.time));
        m.summary_with(now.as_ref())
    }

    /// Copies of this fight's tally and habits with everything still open settled.
    fn fight_snapshot(&self) -> (Tally, HabitTracker) {
        let mut tally = self.tally.clone();
        tally.flush(self.time);
        let mut habits = self.habits.clone();
        habits.flush(self.time);
        (tally, habits)
    }

    /// Writes a JSONL log of requests, replies, slots and outcomes into `dir` (created if
    /// missing), one file per session; returns its path.
    pub fn log_to(&mut self, dir: &Path) -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(dir)?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        let path = dir.join(format!(
            "{stamp}-{}-{}.jsonl",
            self.actions.chr,
            self.brain.name()
        ));
        self.log = Some(BufWriter::new(File::create(&path)?));
        self.log_path = Some(path.clone());
        let choices: Vec<_> = self
            .actions
            .choices()
            .iter()
            .map(|c| json!({"value": c.value, "label": c.label}))
            .collect();
        self.write(json!({
            "ev": "start",
            "brain": self.brain.name(),
            "mode": self.mode.name(),
            "chr": self.actions.chr,
            "max_plan": self.actions.max_plan,
            "choices": choices,
        }));
        Ok(path)
    }

    pub fn log_path(&self) -> Option<&Path> {
        self.log_path.as_deref()
    }

    fn write(&mut self, line: serde_json::Value) {
        self.write_line(&line);
    }

    /// Writes any serializable line (serialized directly, so `f32` fields stay short).
    fn write_line<T: Serialize>(&mut self, line: &T) {
        if let Some(w) = self.log.as_mut() {
            let ok = serde_json::to_writer(&mut *w, line).is_ok()
                && w.write_all(b"\n").is_ok()
                && w.flush().is_ok();
            if !ok {
                eprintln!("brain log: write failed, logging stops");
                self.log = None;
            }
        }
    }

    /// Advances the clock; call once at the start of every duel step.
    pub fn tick(&mut self, dt: f32) {
        self.time += dt;
        self.step_dt = dt;
        for s in self.since_hit.iter_mut().flatten() {
            *s += dt;
        }
        if self.coach_mode {
            while let Some(reply) = self.brain.poll_coach(self.time) {
                self.take_coach_reply(reply);
            }
            if self.coach_flights.is_empty()
                && let Some(input) = self.post_pending.take()
            {
                self.submit_coach_input(input, false);
            }
        }
    }

    /// A coach answer arrived (at any time, also during a respawn): a usable plan waits for the
    /// next fight to start; a failure or a plan with nothing valid keeps the previous one.
    fn take_coach_reply(&mut self, reply: CoachReply) {
        let Some(at) = self.coach_flights.iter().position(|f| f.id == reply.id) else {
            return;
        };
        let flight = self.coach_flights.remove(at);
        if flight.mid && flight.fight == self.fight_no {
            self.mid_done[1] += u64::from(reply.latency_ms);
        }
        let waited = self.time - flight.sent;
        self.coach_done[0] += 1;
        for (i, t) in reply.tokens.iter().enumerate() {
            self.coach_done[1 + i] += t;
        }
        self.coach_done[4] += u64::from(reply.latency_ms);
        let tokens = format!(
            "tokens {} in / {} out, {} reasoning, {:.1} s",
            reply.tokens[0],
            reply.tokens[1],
            reply.tokens[2],
            f64::from(reply.latency_ms) / 1000.0
        );
        let Some(raw) = reply.plan else {
            eprintln!(
                "coach: no game plan ({}); keeping the previous plan ({tokens})",
                reply.error.as_deref().unwrap_or("failed")
            );
            self.write(
                json!({"ev": "coach_failed", "t": ms(self.time), "id": reply.id,
                "error": reply.error, "waited": ms(waited)}),
            );
            return;
        };
        let v = raw.validate(self.actions);
        for w in &v.warnings {
            eprintln!("coach: warning: {w}");
        }
        if !v.usable() {
            eprintln!(
                "coach: the game plan had nothing valid; keeping the previous plan ({tokens})"
            );
            self.write(
                json!({"ev": "coach_failed", "t": ms(self.time), "id": reply.id,
                "error": "nothing valid", "warnings": v.warnings}),
            );
            return;
        }
        if reply.id < self.newest_coach {
            return;
        }
        self.newest_coach = reply.id;
        self.overlay_events.coach = Some((
            self.time,
            format!(
                "coach ({}, {:.0} s): {}",
                if flight.mid {
                    "mid-fight"
                } else {
                    "post-fight"
                },
                waited,
                v.plan.notes
            ),
        ));
        self.boss_knowledge.learn(&v.plan);
        for retry in &v.plan.retry {
            if self
                .boss_knowledge
                .bans
                .iter()
                .any(|b| b.choice == retry.choice)
            {
                continue;
            }
            self.banned.remove(&retry.choice);
            self.bans_at_start.remove(&retry.choice);
            // New evidence is evaluated against new uses after the explicit retry.
            self.retry_baselines
                .insert(retry.choice.clone(), self.plan_failures(&retry.choice));
            let response = format!("on_wolf_break:{}", retry.choice);
            self.banned.remove(&response);
            self.bans_at_start.remove(&response);
            self.retry_baselines
                .insert(response.clone(), self.plan_failures(&response));
        }
        let application = if flight.mid && flight.fight == self.fight_no && !self.fight_over {
            "next decision slot"
        } else {
            "next fight"
        };
        eprintln!(
            "coach: game plan arrived after {waited:.1} s, applied at {application} ({tokens})"
        );
        for line in v.plan.summary_lines() {
            eprintln!("  {line}");
        }
        self.write(
            json!({"ev": "coach_plan", "t": ms(self.time), "id": reply.id,
            "waited": ms(waited), "tokens": reply.tokens, "plan": v.plan,
            "warnings": v.warnings}),
        );
        self.memory.coach.plan = Some(v.plan.clone());
        self.apply_boss_bans();
        if flight.mid && flight.fight == self.fight_no && !self.fight_over {
            self.mid_plan = Some(v.plan);
        } else {
            self.next_plan = Some(v.plan);
        }
        self.save_memory();
    }

    /// Before the Lua AI thinks: installs the act hook, collects an answer, picks the plan for
    /// the next slot and drops a stale one.
    pub fn before_think(&mut self, ai: &mut AiBrain, hit_radius: f32) {
        ai.enable_act_hook(self.actions.reflex_acts);
        if self.mid_plan.is_some() && !self.fight_over {
            self.ready = None;
            self.picked = false;
            if let Some(hook) = ai.state.act_hook.as_mut() {
                hook.choice = None;
            }
        }
        if self.cancel_break {
            self.cancel_break = false;
            if let Some(hook) = ai.state.act_hook.as_mut() {
                if hook.break_followup.take().is_some() {
                    hook.choice = None;
                }
                hook.break_response = false;
            }
            if self
                .ready
                .as_ref()
                .is_some_and(|r| r.label == "on_wolf_break")
            {
                self.ready = None;
                self.picked = false;
            }
        }
        if let Some(reply) = self.brain.poll(self.time) {
            self.take_reply(reply);
        }
        if self.fight_over {
            if let Some(hook) = ai.state.act_hook.as_mut() {
                hook.break_followup = None;
            }
            self.ready = None;
        }
        // A plan that is no longer waiting (a new fight began) must not run.
        if self.ready.is_none()
            && let Some(hook) = ai.state.act_hook.as_mut()
        {
            hook.choice = None;
            if hook.break_followup.is_none() {
                hook.break_response = false;
            }
        }
        if self.fight_over {
            return;
        }
        if self.break_pending {
            self.break_pending = false;
            let choices: Vec<_> = self
                .game_plan
                .as_ref()
                .map(|p| p.on_wolf_break.as_slice())
                .unwrap_or_default()
                .iter()
                .filter(|w| {
                    !self.banned.contains(&w.plan)
                        && !self.banned.contains(&format!("on_wolf_break:{}", w.plan))
                })
                .filter_map(|w| {
                    Some(Weighted {
                        plan: Some(self.actions.parse(&w.plan).ok()?),
                        weight: w.weight,
                    })
                })
                .collect();
            let ctx = Context {
                streaks: &Streaks::default(),
                tried: &HashSet::new(),
                temperature: self.temperature,
            };
            if let Pick::Plan(i) = sampler::pick(&choices, &ctx, &mut self.rng)
                && let Some(plan) = choices[i].plan.clone()
                && let Ok(mut choice) = self.actions.act_for_plan(&plan, hit_radius)
                && let Some(hook) = ai.state.act_hook.as_mut()
            {
                // A break response is a follow-up: request it through the existing combo
                // cancel path, preserving the animation's own TAE windows.
                if let Some(first) = choice.goals.first_mut()
                    && first.goal == "GOAL_COMMON_AttackTunableSpin"
                {
                    first.goal = "GOAL_COMMON_ComboFinal";
                    first.params.truncate(6);
                }
                hook.break_followup = Some(choice.clone());
                hook.break_until = ai.state.time + 2.5;
                hook.choice = Some(choice);
                hook.break_response = true;
                self.ready = Some(Ready {
                    since: self.time,
                    plan,
                    opener: false,
                    label: "on_wolf_break".into(),
                });
                self.picked = true;
            }
        }
        let stale = self.ready.as_ref().is_some_and(|r| {
            self.time - r.since > self.max_staleness || self.banned.contains(&r.plan.value())
        });
        if stale {
            let ready = self.ready.take();
            if let Some(r) = ready.as_ref().filter(|r| r.opener) {
                // The opener is not lost: it runs at the next slot.
                self.opener = Some(r.plan.clone());
            }
            let plan = ready.map(|r| r.plan);
            if let Some(hook) = ai.state.act_hook.as_mut() {
                hook.choice = None;
                hook.break_response = false;
            }
            self.picked = false;
            if !self.persist {
                self.stats.stale += 1;
                self.want_request = true;
                self.write(json!({"ev": "stale", "t": ms(self.time),
                    "plan": plan.map(|p| p.value())}));
            }
        }
        if ai
            .state
            .act_hook
            .as_ref()
            .is_some_and(|h| h.break_followup.is_some())
        {
            self.picked = true;
        }
        if !self.picked
            && (!self.set.is_empty()
                || self.opener.is_some()
                || self.mid_plan.is_some()
                || self.staged_mid.is_some())
        {
            self.pick_next(ai, hit_radius);
        }
    }

    /// An answer arrived: it becomes the plan set.
    fn take_reply(&mut self, reply: BrainReply) {
        let sent = match self.in_flight {
            Some(f) if f.id == reply.id => {
                self.in_flight = None;
                f
            }
            _ => {
                // An answer to a request we no longer wait for.
                if !self.fight_over {
                    self.write(json!({"ev": "late", "t": ms(self.time), "id": reply.id}));
                }
                return;
            }
        };
        if self.fight_over || sent.fight != self.fight_no {
            return;
        }
        self.stats.replies += 1;
        let latency = (self.time - sent.sent) * 1000.0;
        self.fight.latencies.push(latency);
        if self.coach_mode {
            // The chooser's answer: a set name, or `keep`. A failed call keeps the set too.
            let chosen = reply.choice.as_deref().unwrap_or(KEEP);
            let previous = self.active_set;
            let switched = chosen != KEEP && self.switch_set(chosen);
            if let Some((time, checkpoint)) = self.overlay_events.checkpoint.as_mut() {
                *time = self.time;
                *checkpoint = format!(
                    "{} -> {}",
                    checkpoint.split(" -> ").next().unwrap_or("checkpoint"),
                    if switched && previous != self.active_set {
                        format!("set {chosen}")
                    } else {
                        "keep".into()
                    }
                );
            }
            self.write(json!({
                "ev": "reply", "t": ms(self.time), "id": reply.id, "choice": chosen,
                "switched": switched, "set": self.active_set_name(),
                "latency": ms(latency / 1000.0),
            }));
            self.want_request = false;
            return;
        }
        let set = if reply.set.is_empty() {
            vec![Weighted {
                plan: reply.plan.clone(),
                weight: 1.0,
            }]
        } else {
            reply.set
        };
        self.write(json!({
            "ev": "reply", "t": ms(self.time), "id": reply.id,
            "plan": reply.plan.as_ref().map(Plan::value),
            "options": set.len(), "latency": ms(latency / 1000.0),
        }));
        self.persist = self.mode == RequestMode::Checkpoints;
        if let Some((time, checkpoint)) = self.overlay_events.checkpoint.as_mut() {
            *time = self.time;
            *checkpoint = format!(
                "{} -> plan set ({} choices)",
                checkpoint.split(" -> ").next().unwrap_or("checkpoint"),
                set.len()
            );
        }
        self.set = set;
        self.set_time = sent.sent;
        self.ready = None;
        self.picked = false;
        self.want_request = false;
    }

    /// Samples the plan for the next slot from the set and hands it to the act hook.
    fn pick_next(&mut self, ai: &mut AiBrain, hit_radius: f32) {
        if let Some(plan) = self.mid_plan.take().or_else(|| self.staged_mid.take()) {
            // Prepare the next slot without replacing the currently running plan or goals.
            let old_plan = self.game_plan.replace(plan.clone());
            let old_set = self.set.clone();
            let old_active = self.active_set;
            let old_since = self.set_since;
            let old_opener = self.opener.clone();
            let old_why = self.pending_why;
            self.install_plan();
            self.opener = None;
            self.pick_from_active(ai, hit_radius);
            self.game_plan = old_plan;
            self.set = old_set;
            self.active_set = old_active;
            self.set_since = old_since;
            self.opener = old_opener;
            self.pending_why = old_why;
            self.staged_mid = Some(plan);
        } else {
            self.pick_from_active(ai, hit_radius);
        }
    }

    fn pick_from_active(&mut self, ai: &mut AiBrain, hit_radius: f32) {
        let opener = self
            .opener
            .take()
            .filter(|p| !self.banned.contains(&p.value()));
        let is_opener = opener.is_some();
        let plan = if let Some(plan) = opener {
            Some(plan)
        } else {
            let ctx = Context {
                streaks: &self.streaks,
                tried: &self.tried,
                temperature: self.temperature,
            };
            // The hard limits (see `limits`): only eligible plans are sampled; with none, his
            // own AI's act runs.
            let eligible: Vec<usize> = (0..self.set.len())
                .filter(|&i| {
                    self.set[i]
                        .plan
                        .as_ref()
                        .is_none_or(|p| !self.banned.contains(&p.value()))
                        && (!self.anti_stall
                            || self.set[i]
                                .plan
                                .as_ref()
                                .is_none_or(|p| self.pacing.eligible(p, self.time, self.actions)))
                })
                .collect();
            if self.set.is_empty() {
                return;
            }
            let view: Vec<Weighted> = eligible.iter().map(|&i| self.set[i].clone()).collect();
            if view.is_empty() {
                None
            } else {
                match sampler::pick(&view, &ctx, &mut self.rng) {
                    Pick::Nothing => return,
                    Pick::Lua => None,
                    Pick::Plan(i) => view[i].plan.clone(),
                }
            }
        };
        if !self.persist && !is_opener {
            self.set.clear();
        }
        self.picked = true;
        let Some(plan) = plan else { return };
        let label = if is_opener {
            "opener".to_string()
        } else {
            self.active_set_name().unwrap_or("-").to_string()
        };
        match self.actions.act_for_plan(&plan, hit_radius) {
            Ok(choice) => {
                if let Some(hook) = ai.state.act_hook.as_mut() {
                    hook.choice = Some(choice);
                }
                let since = if self.persist {
                    self.time
                } else {
                    self.set_time
                };
                self.ready = Some(Ready {
                    since,
                    plan,
                    opener: is_opener,
                    label,
                });
            }
            Err(e) => {
                self.stats.invalid += 1;
                self.write(json!({"ev": "invalid", "t": ms(self.time), "error": e.to_string()}));
            }
        }
    }

    /// Whether nothing should be requested or logged: a fighter is dead.
    fn quiet(&self, duel: &Duel) -> bool {
        self.fight_over
            || duel.respawn_in.is_some()
            || duel.rules.fighters[0].dead()
            || duel.rules.fighters[1].dead()
    }

    fn checkpoint_blocked(&self, duel: &Duel) -> bool {
        self.quiet(duel)
            || duel.deathblow.is_some()
            || duel.deathblow_available()
            || duel.rules.deathblow_open(1)
            || duel
                .enemy
                .behavior
                .runtime
                .main_state_path()
                .first()
                .is_some_and(|s| s.contains("Damage") || s.contains("Throw"))
    }

    fn hits_to_break(&self, duel: &Duel) -> Option<f32> {
        let anim = duel.player.tae.anim("a050_300000")?;
        let attack = anim.events.iter().find_map(|e| match e.kind {
            crate::tae::TaeKind::Attack { judge, .. } => duel.fighters[0].attacks.get(&judge),
            _ => None,
        })?;
        let damage = duel
            .rules
            .wolf_posture_per_hit(attack, [&duel.player, &duel.enemy])?;
        Some(duel.rules.fighters[1].posture.remaining as f32 / damage)
    }

    /// After the Lua AI thought: notes who chose any act that started and sends the next
    /// request when one is due.
    pub fn after_think(&mut self, ai: &mut AiBrain, duel: &Duel) {
        let (slots, started) = ai
            .state
            .act_hook
            .as_mut()
            .map(|h| (std::mem::take(&mut h.slots), std::mem::take(&mut h.started)))
            .unwrap_or_default();
        let quiet = self.quiet(duel);
        for (_, slot) in slots {
            if self.tactic_open {
                self.tally
                    .record(self.time, TallyEvent::End { side: Side::Boss });
                self.tactic_open = false;
            }
            if !matches!(slot, ActSlot::Reflex)
                && let Some(plan) = self.staged_mid.take()
            {
                self.close_set_time();
                self.game_plan = Some(plan);
                self.install_plan();
                self.opener = None;
            }
            let by = match slot {
                ActSlot::External => {
                    let ready = self.ready.take();
                    let label = ready.as_ref().map(|r| r.label.clone()).unwrap_or_default();
                    self.running = ready.map(|r| r.plan).unwrap_or_default();
                    self.overlay_events.step = 0;
                    if limits::is_retreat(&self.running, self.actions) {
                        self.pacing.last_retreat = Some(self.time);
                    }
                    self.history.push(self.running.clone());
                    self.stats.planned += 1;
                    self.picked = false;
                    let value = self.running.value();
                    self.tried.insert(value.clone());
                    if !quiet {
                        self.fight.decisions += 1;
                        self.fight
                            .runs
                            .push((self.time, value.clone(), label.clone()));
                        self.fight.uses.push(PlanUse {
                            break_response: label == "on_wolf_break",
                            failed: false,
                            text: if label == "on_wolf_break" {
                                format!("on_wolf_break:{value}")
                            } else {
                                value.clone()
                            },
                            start: self.time,
                            last_attack: None,
                            attacks: 0,
                            deflected: 0,
                            fatal: false,
                            settled: false,
                        });
                        *self.fight.choices.entry(value).or_default() += 1;
                    }
                    "plan"
                }
                ActSlot::Script => {
                    self.running = Plan::default();
                    self.stats.fallback += 1;
                    self.fight.lua_acts += 1;
                    self.picked = false;
                    if !quiet {
                        self.fight
                            .runs
                            .push((self.time, "lua".into(), "lua".into()));
                    }
                    "lua"
                }
                ActSlot::Reflex => {
                    self.running = Plan::default();
                    self.stats.reflex += 1;
                    self.fight.lua_acts += 1;
                    if !quiet {
                        self.fight
                            .runs
                            .push((self.time, "lua".into(), "lua".into()));
                    }
                    "reflex"
                }
            };
            match self.mode {
                RequestMode::Slot => self.want_request = true,
                RequestMode::PlanEnd if by != "plan" => self.want_request = true,
                _ => {}
            }
            if !quiet {
                let plan = self.running.value();
                self.write(json!({"ev": "slot", "t": ms(self.time), "by": by, "plan": plan}));
            }
        }
        for (_, step) in started {
            self.overlay_events.step = step;
            if quiet {
                continue;
            }
            if let Some(Step::Tactic(t)) = self.running.steps.get(step) {
                // A tactic is an exchange of its own in the tally (Wolf hitting him during it
                // is a punish).
                let key = t.token();
                self.tally.record(
                    self.time,
                    TallyEvent::Start {
                        side: Side::Boss,
                        key,
                    },
                );
                self.tactic_open = true;
            }
            if self.mode == RequestMode::PlanEnd && step + 1 >= self.running.len() {
                self.want_request = true;
            }
        }
        if quiet
            || !self.brain.plans()
            || self.in_flight.is_some()
            || (self.mode == RequestMode::Checkpoints && self.checkpoint_blocked(duel))
        {
            return;
        }
        if self.mode == RequestMode::Checkpoints {
            if self.pending_why.is_none() {
                self.pending_why = self.wolf_events.pop_front();
            }
            let limit = match self.pending_why {
                Some("pressure") => Some(PRESSURE_HITS),
                Some("pressure_critical") => Some(PRESSURE_CRITICAL_HITS),
                _ => None,
            };
            if let Some(limit) = limit
                && self
                    .hits_to_break(duel)
                    .is_none_or(|hits| !(PRESSURE_MIN_HITS..=limit).contains(&hits))
            {
                self.pending_why = None;
            }
        }
        let why = match self.mode {
            RequestMode::Slot | RequestMode::PlanEnd => {
                (self.want_request && self.ready.is_none() && (!self.picked || self.set.is_empty()))
                    .then_some(None)
            }
            RequestMode::Checkpoints => self.pending_why.map(Some),
        };
        let Some(why) = why else { return };
        self.want_request = false;
        self.pending_why = None;
        let (active, sets) = if self.coach_mode {
            // Checkpoints only choose among a game plan's sets: without a plan (the first
            // fight, or a coach that failed) no call is made and his own AI plays.
            let plan = self.game_plan.as_ref().filter(|p| p.sets.len() >= 2);
            let Some(plan) = plan else { return };
            (self.active_set_name().map(str::to_string), plan.offers())
        } else {
            (None, Vec::new())
        };
        let obs = observe(duel, self.time, self.since_hit, &self.running.value());
        let (tally, memory, brief) = if self.coach_mode {
            // The chooser sees a few numbers and the set names, nothing else (the habits and
            // the history are the coach's): about 100 tokens.
            let r2 = |x: f32| (f64::from(x) * 100.0).round() / 100.0;
            let brief = json!({"why": why, "set": active, "boss_hp": r2(obs.boss.hp),
                "boss_posture": r2(obs.boss.posture), "wolf_hp": r2(obs.wolf.hp),
                "wolf_posture": r2(obs.wolf.posture), "dist": r2(obs.dist)});
            (Default::default(), None, Some(brief.to_string()))
        } else {
            (self.tally.summary(), Some(self.memory_summary()), None)
        };
        let mut request = BrainRequest {
            id: self.next_id,
            obs,
            tally,
            memory,
            why,
            active,
            sets,
            brief,
            excluded: self.excluded(),
        };
        request.fit(REQUEST_BUDGET);
        self.next_id += 1;
        self.stats.requests += 1;
        self.fight.requests += 1;
        self.in_flight = Some(InFlight {
            id: request.id,
            sent: self.time,
            fight: self.fight_no,
        });
        if let Some(why) = request.why {
            self.overlay_events.checkpoint = Some((self.time, format!("{why} -> thinking")));
        }
        self.brain.submit(&request);
        if self.log.is_some() {
            self.write_line(&RequestLine {
                ev: "request",
                t: ms(self.time),
                request: &request,
            });
        }
    }

    /// The offered plans whose steps were deflected or guarded [`EXCLUDE_STREAK`] times in a
    /// row this fight.
    fn excluded(&self) -> Vec<String> {
        self.actions
            .choices()
            .into_iter()
            .filter(|c| {
                self.actions
                    .parse(&c.value)
                    .is_ok_and(|p| self.streaks.worst(&p) >= EXCLUDE_STREAK)
            })
            .map(|c| c.value)
            .collect()
    }

    /// After the duel step: action changes and hits into the tally and the habits; a death
    /// ends the fight, a respawn starts the next.
    pub fn after_step(&mut self, duel: &Duel, report: &DuelReport, input: &crate::InputFrame) {
        if report.respawned {
            self.start_fight();
        }
        if self.fight_over {
            return;
        }
        let t = self.time;
        self.fight.metrics.input_tick(input, t, self.step_dt);
        let mut events = Vec::new();
        // The boss's actions: his attack and step animations (guards and deflects are
        // reactions, not actions).
        let boss = duel
            .enemy
            .behavior
            .runtime
            .main_clip()
            .and_then(|c| anim_id_from_clip(&c.animation))
            .filter(|a| (3000..3100).contains(a) || (5200..5300).contains(a));
        if boss != self.boss_open || (boss.is_some() && report.enemy.clip_started) {
            self.fight.close_range();
            events.push(match boss {
                Some(a) => TallyEvent::Start {
                    side: Side::Boss,
                    key: a.to_string(),
                },
                None => TallyEvent::End { side: Side::Boss },
            });
            if boss.is_some_and(|a| a < 3100) {
                if let Some((_, plan, _)) = self.fight.runs.last() {
                    let start = duel.enemy.body.position;
                    let wolf = duel.player.body.position;
                    self.fight.range_open = Some(AttackRange {
                        action: boss.unwrap().to_string(),
                        plan: plan.clone(),
                        start,
                        distance: ((wolf[0] - start[0]).powi(2) + (wolf[2] - start[2]).powi(2))
                            .sqrt(),
                        reach: None,
                    });
                }
                // An attack step started (a back step does not count).
                self.fight.metrics.attack_started(t);
                self.pacing.last_attack = t;
                if let Some(use_) = self.fight.uses.last_mut()
                    && self.fight.runs.last().is_some_and(|r| r.0 == use_.start)
                {
                    use_.last_attack = Some(t);
                    use_.attacks += 1;
                }
            }
            self.boss_open = boss;
            // An attack or step starting closes an open tactic; its end closes nothing more.
            if boss.is_some() {
                self.tactic_open = false;
            }
        }
        if let Some(sample) = self.fight.range_open.as_mut()
            && let Some(reach) =
                duel.fighters[1].active_reach(&duel.enemy, sample.start, duel.fighters[0].radius)
        {
            sample.reach = Some(sample.reach.unwrap_or(0.0).max(reach));
        }
        // Wolf's attacks: from the step his clip raises the parry-timing flag (the signal the
        // game gives enemies) until the clip changes.
        let clip = duel
            .player
            .behavior
            .runtime
            .main_clip()
            .map(|c| c.animation.clone());
        if self.wolf_open.is_some() && (self.wolf_open != clip || report.player.clip_started) {
            events.push(TallyEvent::End { side: Side::Wolf });
            self.wolf_open = None;
        }
        let state = report
            .player
            .behavior
            .state_path
            .last()
            .cloned()
            .unwrap_or_default();
        if self.wolf_open.is_none() && duel.player.tae_frame().flag(FLAG_PARRY_TIMING) {
            let key = if state.is_empty() {
                clip.clone().unwrap_or_default()
            } else {
                wolf_key(&state)
            };
            events.push(TallyEvent::Start {
                side: Side::Wolf,
                key,
            });
            self.wolf_open = clip;
        }
        let mut punished_break = false;
        let wolf = &duel.rules.fighters[0];
        let (wolf_events, wolf_broke) = self
            .checkpoints
            .on_wolf(wolf.posture_fraction(), wolf.posture.broken());
        for why in wolf_events {
            if !self.wolf_events.contains(&why) {
                self.wolf_events.push_back(why);
            }
        }
        let breaking_hit = wolf_broke
            .then(|| {
                report.hits.iter().position(|(hit, result)| {
                    result.posture_broke
                        && (if hit.defender == 0 {
                            matches!(result.defender.damage_type, 1001 | 1004 | 1005)
                        } else {
                            result.attacker.as_ref().is_some_and(|damage| {
                                matches!(damage.damage_type, 1001 | 1004 | 1005)
                            })
                        })
                })
            })
            .flatten();
        if wolf_broke && breaking_hit.is_none() {
            self.fight.metrics.wolf_break(t);
        }
        if wolf_broke
            && !self.quiet(duel)
            && duel.deathblow.is_none()
            && !duel.rules.deathblow_open(1)
        {
            self.break_pending = true;
        }
        for (index, (hit, result)) in report.hits.iter().enumerate() {
            if breaking_hit == Some(index) {
                self.fight.metrics.wolf_break(t);
            }
            if (result.posture_broke && duel.rules.fighters[1].posture.broken())
                || (hit.attacker == 0 && matches!(result.defender.damage_type, 1001 | 1004 | 1005))
            {
                punished_break = true;
                self.fatal_plans(t, true);
            }
            let attacker = Side::from_index(hit.attacker);
            events.push(TallyEvent::Hit {
                attacker,
                guarded: result.guarded,
                deflected: result.deflected,
            });
            if !result.guarded && !result.deflected {
                // since_hit is [boss, wolf].
                self.since_hit[if attacker == Side::Boss { 0 } else { 1 }] = Some(0.0);
            }
            if attacker == Side::Boss {
                // The blow that caused the break is not its follow-up.
                if breaking_hit != Some(index) && !result.guarded && !result.deflected {
                    self.fight.metrics.post_break_hit(t);
                }
                self.fight
                    .metrics
                    .boss_hit(result.guarded, result.deflected);
            }
            if let Some(why) = self
                .checkpoints
                .on_hit(attacker, result.guarded, result.deflected)
            {
                self.pending_why.get_or_insert(why);
            }
        }
        let wolf = &duel.rules.fighters[0];
        self.fight
            .metrics
            .wolf(wolf.hp_fraction(), wolf.posture_fraction());
        self.fight.metrics.passive_tick(t, self.step_dt);
        let boss_fighter = &duel.rules.fighters[1];
        self.fight
            .metrics
            .boss(boss_fighter.hp_fraction(), boss_fighter.posture_fraction());
        let hits = self.hits_to_break(duel);
        if let Some(why) = self.checkpoints.on_vitals(
            boss_fighter.posture_fraction(),
            boss_fighter.hp_fraction(),
            hits,
        ) {
            // A request may still be in flight when pressure grows. Keep the newest
            // posture warning instead of sending an obsolete half-meter/HP warning later.
            if (why == "pressure_critical"
                && matches!(self.pending_why, Some("pressure" | "half_hp")))
                || (why == "pressure" && self.pending_why == Some("half_hp"))
            {
                self.pending_why = Some(why);
            } else {
                self.pending_why.get_or_insert(why);
            }
        }
        let marks = boss_fighter.vitality.deathblows_left.max(0);
        let lost_mark = self.checkpoints.marks.is_some_and(|m| marks < m);
        if boss_fighter.dead() || lost_mark {
            self.fatal_plans(t, false);
        }
        if marks > 0 && self.checkpoints.marks.is_some_and(|m| marks < m) {
            self.pending_why = Some("mark_lost");
            self.mid_pending.get_or_insert("mark_lost");
        }
        self.checkpoints.marks = Some(marks);
        if self.quiet(duel) || duel.deathblow.is_some() || duel.rules.deathblow_open(1) {
            self.wolf_events.clear();
            self.break_pending = false;
            self.cancel_break = true;
            // A lost mark is useful after the get-up; pressure during a deathblow opening is obsolete.
            self.pending_why = self
                .pending_why
                .filter(|why| matches!(*why, "mark_lost" | "start"));
        }
        for e in &events {
            self.tally.record(t, e.clone());
        }
        let (p, e) = (duel.player.body.position, duel.enemy.body.position);
        let frame = WolfFrame {
            t,
            dt: self.step_dt,
            dist: ((p[0] - e[0]).powi(2) + (p[2] - e[2]).powi(2)).sqrt(),
            state: &state,
            airborne: !duel.player.body.grounded,
            guarding: duel.player.tae_frame().flag(FLAG_GUARD),
            hp: duel.rules.fighters[0].hp_fraction(),
        };
        self.habits.step(&frame, &events);
        let done = self.tally.advance(t);
        self.settle(&done);
        if boss_fighter.dead() || lost_mark || punished_break {
            self.fatal_plans(t, punished_break);
        }
        self.settle_plans(false);
        self.log_outcomes(t, &done);
        self.review_mid_fight(duel);
        // The duel schedules its respawn exactly when the fight is over: a death outside a
        // deathblow (a boss with marks left gets up from his).
        if duel.respawn_in.is_some() {
            self.end_fight(if duel.rules.fighters[0].dead() {
                FightEnd::BossWon
            } else {
                FightEnd::WolfWon
            });
        } else if self.memory_path.is_some() && t - self.last_save >= AUTOSAVE_SECS {
            self.save_memory();
        }
    }

    /// Finished boss exchanges update the streaks the sampler penalises: an attack is repelled
    /// when it was deflected or guarded and never landed, a tactic when Wolf hit him during it.
    fn settle(&mut self, done: &[Exchange]) {
        self.attribute(done);
        for x in done.iter().filter(|x| x.side == Side::Boss) {
            let o = &x.outcome;
            let repelled = if x.key.parse::<i32>().is_ok() {
                (o.deflected || o.blocked) && !o.hit
            } else {
                o.punished
            };
            self.streaks.record(&x.key, repelled);
        }
    }

    fn plan_failures(&self, text: &str) -> PlanFailures {
        let mut f = self
            .memory
            .coach
            .failures
            .get(text)
            .copied()
            .unwrap_or_default();
        if !self.fight_over
            && let Some(now) = self.fight.failures.get(text)
        {
            f.uses += now.uses;
            f.deflected_all += now.deflected_all;
            f.fatal += now.fatal;
            f.failed += now.failed;
        }
        f
    }

    fn fatal_plans(&mut self, now: f32, posture_break: bool) {
        let active = self.fight.runs.last().map(|r| r.0);
        for use_ in &mut self.fight.uses {
            if !use_.settled
                && (use_
                    .last_attack
                    .is_some_and(|last| now - last <= 3.0 + 1e-3)
                    || (posture_break && active == Some(use_.start)))
            {
                use_.fatal = true;
            }
        }
    }

    fn settle_plans(&mut self, flush: bool) {
        let active = self.fight.runs.last().map(|r| r.0);
        for use_ in &mut self.fight.uses {
            if use_.settled
                || (!flush
                    && (active == Some(use_.start)
                        || self.time - use_.last_attack.unwrap_or(use_.start) <= 3.0 + 1e-3))
            {
                continue;
            }
            use_.settled = true;
            let f = self.fight.failures.entry(use_.text.clone()).or_default();
            f.uses += 1;
            f.deflected_all += u32::from(use_.attacks > 0 && use_.attacks == use_.deflected);
            f.fatal += u32::from(use_.fatal);
            f.failed += u32::from(use_.failed && !use_.fatal && use_.attacks != use_.deflected);
        }
        // Provisional outcomes never teach the sampler while the player is inactive.
        if !self.fight.metrics.player_active(self.time) {
            return;
        }
        let keys: HashSet<_> = self
            .memory
            .coach
            .failures
            .keys()
            .chain(self.fight.failures.keys())
            .cloned()
            .collect();
        for text in keys {
            let mut f = self.plan_failures(&text);
            if let Some(base) = self.retry_baselines.get(&text) {
                f.uses = f.uses.saturating_sub(base.uses);
                f.deflected_all = f.deflected_all.saturating_sub(base.deflected_all);
                f.fatal = f.fatal.saturating_sub(base.fatal);
                f.failed = f.failed.saturating_sub(base.failed);
            }
            if f.banned() && self.banned.insert(text.clone()) {
                let reasons = [
                    (f.deflected_all, "fully deflected"),
                    (f.fatal, "fatal"),
                    (f.failed, "whiffed/punished"),
                ]
                .into_iter()
                .filter(|(n, _)| *n > 0)
                .map(|(n, reason)| format!("{n}/{} {reason}", f.uses))
                .collect::<Vec<_>>()
                .join(", ");
                self.overlay_events.ban = Some((self.time, format!("banned {text}: {reasons}")));
                eprintln!(
                    "banned: {text} ({} uses, {:.0}% deflected/fatal/break failure)",
                    f.uses,
                    f.rate() * 100.0
                );
            }
        }
    }

    /// Counts each finished boss exchange for the plan run it began under (the latest plan or
    /// Lua act that started at or before it), and that run's set.
    fn attribute(&mut self, done: &[Exchange]) {
        for x in done.iter().filter(|x| x.side == Side::Boss) {
            let Some((start, plan, set)) = self
                .fight
                .runs
                .iter()
                .rev()
                .find(|(start, _, _)| *start <= x.start + 1e-3)
                .cloned()
            else {
                continue;
            };
            if x.key
                .parse::<i32>()
                .is_ok_and(|a| (3000..3100).contains(&a))
                && let Some(use_) = self.fight.uses.iter_mut().find(|u| u.start == start)
                && !use_.settled
            {
                // The last attack's end, rather than the beginning of its wind-up, anchors
                // the post-plan punishment window. It remains live after a new run starts.
                use_.last_attack = Some(
                    use_.last_attack
                        .unwrap_or(x.start)
                        .max(x.end.unwrap_or(self.time)),
                );
                use_.deflected +=
                    u32::from(x.outcome.deflected && !x.outcome.hit && !x.outcome.blocked);
                if use_.break_response {
                    use_.failed |= x.whiffed() || x.outcome.punished;
                }
            }
            let mut one = Counts::default();
            one.merge(&Counts {
                n: 1,
                hit: u32::from(x.outcome.hit),
                blocked: u32::from(x.outcome.blocked),
                deflected: u32::from(x.outcome.deflected),
                whiffed: u32::from(x.whiffed()),
                punished: u32::from(x.outcome.punished),
            });
            if set == "on_wolf_break" {
                self.fight
                    .break_plans
                    .entry(plan.clone())
                    .or_default()
                    .merge(&one);
            }
            self.fight.by_plan.entry(plan).or_default().merge(&one);
            self.fight.by_set.entry(set).or_default().merge(&one);
        }
    }

    /// Ends the fight: settles it, adds it and its record to the memory and saves.
    pub fn end_fight(&mut self, end: FightEnd) {
        if self.fight_over {
            return;
        }
        let t = self.time;
        let done = self.tally.flush(t);
        self.fight.close_range();
        self.attribute(&done);
        if end == FightEnd::WolfWon {
            self.fatal_plans(t, false);
        }
        self.settle_plans(true);
        self.close_set_time();
        self.log_outcomes(t, &done);
        self.habits.flush(t);
        self.fight_over = true;
        self.stats.fights += 1;
        let winner = match end {
            FightEnd::BossWon => "boss",
            FightEnd::WolfWon => "wolf",
            FightEnd::Unfinished => "none",
        };
        let punishes = self
            .tally
            .all(Side::Boss)
            .values()
            .map(|c| c.punished)
            .sum();
        let mut metrics = self
            .fight
            .metrics
            .finish(t, punishes, end == FightEnd::WolfWon);
        metrics.time_to_kill_wolf = (end == FightEnd::BossWon).then_some(metrics.duration);
        let usage = self.brain.usage();
        let record = FightRecord {
            winner: winner.to_string(),
            metrics,
            decisions: self.fight.decisions,
            lua_acts: self.fight.lua_acts,
            requests: self.fight.requests,
            choices: self.fight.choices.clone(),
            tokens: [
                usage.0.saturating_sub(self.fight.tokens_at_start.0),
                usage.1.saturating_sub(self.fight.tokens_at_start.1),
            ],
            latency_ms: [
                percentile(&self.fight.latencies, 0.5).round() as u32,
                percentile(&self.fight.latencies, 0.9).round() as u32,
            ],
            brain: format!("{}/{}", self.brain.name(), self.mode.name()),
            reasoning: self.brain.call_stats()[0]
                .since(&self.fight.stats_at_start[0])
                .reasoning,
            coach: self.coach_done,
            mid_coach: self.mid_done,
            sets: self.fight.set_secs.clone(),
        };
        eprintln!("{}", record.summary_line());
        self.overlay_events.scorecard = Some((
            self.time,
            format!(
                "{} wins · kill Wolf {} · {} posture breaks · deflect {}",
                record.winner,
                record
                    .metrics
                    .time_to_kill_wolf
                    .map(|s| format!("{s:.1} s"))
                    .unwrap_or_else(|| "n/a".into()),
                record.metrics.wolf_posture_breaks,
                record
                    .metrics
                    .deflect_rate()
                    .map(|r| format!("{:.0}%", r * 100.0))
                    .unwrap_or_else(|| "n/a".into())
            ),
        ));
        self.write(json!({"ev": "fight_end", "t": ms(t), "winner": winner,
            "void": record.metrics.void, "fights": self.memory.fights + u32::from(!record.metrics.void), "summary": record.summary_line()}));
        self.write_line(&json!({"ev": "fight_summary", "t": ms(t), "record": record}));
        if !record.metrics.void {
            self.session_records.push(record.clone());
            merge_counts(&mut self.session_actions, self.tally.all(Side::Boss));
            for (action, &(n, distance, reach)) in &self.fight.action_ranges {
                let entry = self.session_ranges.entry(action.clone()).or_default();
                entry.0 += n;
                entry.1 += distance;
                entry.2 += reach;
            }
            self.memory.add_fight(&self.tally, &self.habits.habits, end);
            self.memory.push_record(record.clone());
            merge_counts(&mut self.memory.coach.plans, &self.fight.by_plan);
            merge_counts(&mut self.memory.coach.break_plans, &self.fight.break_plans);
            for (text, f) in &self.fight.failures {
                let all = self.memory.coach.failures.entry(text.clone()).or_default();
                all.uses += f.uses;
                all.deflected_all += f.deflected_all;
                all.fatal += f.fatal;
                all.failed += f.failed;
            }
        } else {
            self.banned = self.bans_at_start.clone();
            for (text, base) in &mut self.retry_baselines {
                let committed = self
                    .memory
                    .coach
                    .failures
                    .get(text)
                    .copied()
                    .unwrap_or_default();
                base.uses = base.uses.min(committed.uses);
                base.deflected_all = base.deflected_all.min(committed.deflected_all);
                base.fatal = base.fatal.min(committed.fatal);
                base.failed = base.failed.min(committed.failed);
            }
        }
        if self.coach_mode && !record.metrics.void {
            self.boss_knowledge.update(&self.player, &self.memory);
            if let Some(plan) = &self.game_plan {
                self.boss_knowledge.consider(plan, &self.memory);
            }
            self.send_coach(&record);
        }
        self.last_record = Some(record);
        self.save_memory();
    }

    /// The coach's input for the fight just over (called after its record is in the memory).
    pub fn coach_input(&self, record: &FightRecord) -> CoachInput {
        let m = &record.metrics;
        let pct = |r: Option<f32>| (r.unwrap_or(0.0) * 100.0).round() as u32;
        let top = |map: &CountMap| ranked_counts(map);
        let mut sets: BTreeMap<String, SetStat> = BTreeMap::new();
        for (name, c) in &self.fight.by_set {
            sets.entry(name.clone()).or_default().c = *c;
        }
        for (name, s) in &self.fight.set_secs {
            sets.entry(name.clone()).or_default().secs = (s * 10.0).round() / 10.0;
        }
        let mut input = CoachInput {
            boss_knowledge: self.boss_knowledge.summary(),
            phase: "post".into(),
            elapsed: None,
            vitals: None,
            marks: None,
            on_wolf_break: self.fight.break_plans.clone(),
            actions: {
                let mut actions = self.tally.all(Side::Boss).clone();
                for action in self.actions.actions {
                    actions.entry(action.id.to_string()).or_default();
                }
                actions
            },
            action_range: self
                .fight
                .action_ranges
                .iter()
                .map(|(key, &(n, distance, reach))| {
                    (
                        key.clone(),
                        [
                            (distance / n as f32 * 100.0).round() / 100.0,
                            (reach / n as f32 * 100.0).round() / 100.0,
                        ],
                    )
                })
                .collect(),
            principles: self
                .memory
                .coach
                .plan
                .as_ref()
                .map(|p| p.principles.clone())
                .unwrap_or_default(),
            banned: {
                let mut list: Vec<_> = self
                    .banned
                    .iter()
                    .filter(|choice| {
                        !self.boss_knowledge.bans.iter().any(|ban| {
                            *choice == &ban.choice
                                || choice.split(['>', '+']).any(|step| step == ban.choice)
                        })
                    })
                    .cloned()
                    .collect();
                list.sort();
                list
            },
            failures: self
                .memory
                .coach
                .failures
                .iter()
                .map(|(k, f)| (k.clone(), [f.uses, f.deflected_all, f.fatal, f.failed]))
                .collect(),
            fight: self.memory.fights,
            result: FightLine {
                boss_damage_taken_per_min: m.boss_damage_taken_per_min(),
                wolf_posture_breaks: m.wolf_posture_breaks,
                hits_after_break: m.hits_after_break,
                break_punish_rate: m.break_punish_rate,
                wolf_posture_peak: m.wolf_posture_peak,
                time_to_kill_wolf: m.time_to_kill_wolf,
                won: record.winner.clone(),
                secs: m.duration.round() as u32,
                defl: pct(m.deflect_rate()),
                guard: pct(m.guard_rate()),
                hits: m.hits_taken,
                hp: (m.hp_lost * 100.0).round() as u32,
                posture: (m.posture_gained * 100.0).round() as u32,
                punishes: m.punishes,
                apm: m.attacks_per_min().round() as u32,
                dpm: m.damage_per_min().round() as u32,
                passive: (m.passive_share() * 100.0).round() as u32,
                engaged_time: m.engaged_time.round() as u32,
            },
            trend: Trend::new(&self.memory.history, None),
            plans: top(&self.fight.by_plan),
            range: self
                .fight
                .ranges
                .iter()
                .map(|(plan, &(n, distance, reach))| {
                    let round = |x: f32| (x / n as f32 * 100.0).round() / 100.0;
                    (plan.clone(), [round(distance), round(reach)])
                })
                .collect(),
            sets,
            wolf: top(self.tally.all(Side::Wolf)),
            overall: top(&self.memory.coach.plans),
            habits: self.memory.habits.summary(),
            played: self.game_plan.clone(),
            notes: self
                .memory
                .coach
                .plan
                .as_ref()
                .map(|p| p.notes.clone())
                .unwrap_or_default(),
        };
        // The notes of the plan played are the notes asked for again: send them once.
        if let Some(p) = input.played.as_mut()
            && p.notes == input.notes
        {
            p.notes.clear();
        }
        if let Some(p) = input.played.as_mut()
            && p.principles == input.principles
        {
            p.principles.clear();
        }
        input
    }

    /// Sends the fight summary to the coach; the plan comes back whenever it is ready.
    fn send_coach(&mut self, record: &FightRecord) {
        if !self.brain.plans() || self.fight.runs.is_empty() {
            return;
        }
        self.mid_pending = None;
        self.mid_plan = None;
        self.staged_mid = None;
        let input = self.coach_input(record);
        if !self.coach_flights.is_empty() {
            self.post_pending = Some(input);
            return;
        }
        self.submit_coach_input(input, false);
    }

    fn submit_coach_input(&mut self, mut input: CoachInput, mid: bool) {
        input.phase = if mid { "mid" } else { "post" }.into();
        let bytes = input.fit(INPUT_BUDGET);
        if bytes > INPUT_BUDGET {
            return;
        }
        let Ok(text) = serde_json::to_string(&input) else {
            return;
        };
        let request = CoachRequest {
            id: self.next_coach_id,
            fight: self.fight_no,
            input: text,
        };
        self.next_coach_id += 1;
        self.coach_flights.push(CoachFlight {
            id: request.id,
            sent: self.time,
            mid,
            fight: self.fight_no,
        });
        if mid {
            self.mid_done[0] += 1;
        }
        self.last_coach_sent = self.time;
        self.review_breaks = self.fight.metrics.metrics.wolf_posture_breaks;
        if !mid {
            eprintln!(
                "coach: fight {} over, sent the fight summary ({bytes} bytes); the new game plan plays from the next fight if it arrives during the respawn, else the one after",
                self.fight_no
            );
        }
        self.write(json!({"ev": "coach_request", "kind": "coach", "phase": input.phase, "t": ms(self.time), "id": request.id,
            "bytes": bytes, "input": serde_json::from_str::<serde_json::Value>(&request.input).ok()}));
        self.brain.submit_coach(&request, self.time);
    }

    fn review_mid_fight(&mut self, duel: &Duel) {
        if !self.coach_mode || self.coach_interval <= 0.0 || self.fight_over {
            return;
        }
        let metrics = self.fight.metrics.snapshot(self.time);
        if metrics
            .wolf_posture_breaks
            .saturating_sub(self.review_breaks)
            >= 2
        {
            self.mid_pending.get_or_insert("wolf_broken_twice");
        }
        let set_banned = self.active_set_banned();
        if set_banned && !self.set_banned_seen {
            self.mid_pending.get_or_insert("set_banned");
        }
        self.set_banned_seen = set_banned;
        if self.time - self.last_coach_sent + 1e-3 >= self.coach_interval {
            self.mid_pending.get_or_insert("timer");
        }
        if metrics.duration < 20.0
            || metrics.void
            || self.checkpoint_blocked(duel)
            || duel.rules.fighters.iter().any(|f| f.posture.broken())
            || !self.coach_flights.is_empty()
            || !self.brain.plans()
        {
            return;
        }
        let Some(why) = self.mid_pending.take() else {
            return;
        };
        self.close_set_time();
        let (tally, habits) = self.fight_snapshot();
        let record = FightRecord {
            metrics: super::metrics::FightMetrics {
                punishes: tally.all(Side::Boss).values().map(|c| c.punished).sum(),
                ..metrics
            },
            ..FightRecord::default()
        };
        let mut input = self.coach_input(&record);
        for (key, counts) in tally.all(Side::Boss) {
            input.actions.insert(key.clone(), *counts);
        }
        input.wolf = ranked_counts(tally.all(Side::Wolf));
        input.habits = habits.habits.summary();
        input.fight = self.fight_no;
        input.vitals = Some([
            duel.rules.fighters[0].hp_fraction(),
            duel.rules.fighters[0].posture_fraction(),
            duel.rules.fighters[1].hp_fraction(),
            duel.rules.fighters[1].posture_fraction(),
        ]);
        input.marks = Some(duel.rules.fighters[1].vitality.deathblows_left);
        input.elapsed = Some(metrics.duration);
        eprintln!(
            "coach: mid-fight review at {:.1} s (why: {why})",
            metrics.duration
        );
        self.submit_coach_input(input, true);
    }

    fn active_set_banned(&self) -> bool {
        self.set.iter().any(|w| w.plan.is_some())
            && self
                .set
                .iter()
                .filter_map(|w| w.plan.as_ref())
                .all(|p| self.banned.contains(&p.value()))
    }

    /// Starts the next fight (after a respawn): a fresh tally and habits; the memory keeps
    /// everything before.
    pub fn start_fight(&mut self) {
        if !self.fight_over && !self.tally.is_empty() {
            self.end_fight(FightEnd::Unfinished);
        }
        self.tally = Tally::default();
        self.habits = HabitTracker::new(self.actions.back_steps);
        self.since_hit = [None, None];
        self.boss_open = None;
        self.wolf_open = None;
        self.tactic_open = false;
        self.fight_over = false;
        self.fight_no += 1;
        self.streaks.clear();
        self.tried.clear();
        self.set.clear();
        self.ready = None;
        self.picked = false;
        self.want_request = true;
        self.running = Plan::default();
        self.checkpoints = Checkpoints::default();
        self.wolf_events.clear();
        self.break_pending = false;
        self.cancel_break = true;
        self.pending_why = Some("start");
        self.fight = FightState {
            metrics: MetricsTracker::new(self.time),
            tokens_at_start: self.brain.usage(),
            stats_at_start: self.brain.call_stats(),
            ..FightState::default()
        };
        self.bans_at_start = self.banned.clone();
        self.coach_done = [0; 5];
        self.mid_done = [0; 2];
        self.last_coach_sent = self.time;
        self.review_breaks = 0;
        self.mid_pending = None;
        self.mid_plan = None;
        self.staged_mid = None;
        self.set_banned_seen = false;
        self.pacing = Pacing::new(self.time);
        if self.coach_mode {
            // The newest plan that has arrived plays; with none, the previous one stays.
            if let Some(plan) = self.next_plan.take() {
                self.game_plan = Some(plan);
            }
            self.install_plan();
            match (&self.game_plan, self.active_set_name()) {
                (Some(_), Some(set)) => {
                    eprintln!(
                        "coach: fight {} plays the coach's game plan, start set {set}",
                        self.fight_no
                    )
                }
                (Some(_), None) => {
                    eprintln!(
                        "coach: fight {} plays the coach's opener, then his own AI",
                        self.fight_no
                    )
                }
                (None, _) => eprintln!(
                    "coach: no game plan yet, fight {} is his own AI",
                    self.fight_no
                ),
            }
        }
        self.write(json!({"ev": "fight_start", "t": ms(self.time)}));
    }

    fn log_outcomes(&mut self, t: f32, done: &[Exchange]) {
        if self.log.is_none() {
            return;
        }
        for x in done {
            self.write_line(&OutcomeLine {
                ev: "outcome",
                t: ms(t),
                whiffed: x.whiffed(),
                exchange: x,
            });
        }
    }
}

impl Drop for BrainDriver {
    /// The game is closing: keep an unfinished fight in the memory file.
    fn drop(&mut self) {
        if self.session_report_path.is_some()
            && let Err(e) = self.print_session_report()
        {
            eprintln!("brain session report: {e}");
        }
        if self.memory_path.is_some() && !self.fight_over && !self.tally.is_empty() {
            self.save_memory();
        }
    }
}

/// Wolf's attack state as a tally key, shortened for requests: `GroundAttackCombo1Release`
/// becomes `AttackCombo1`.
pub fn wolf_key(state: &str) -> String {
    let short = state
        .strip_prefix("Ground")
        .unwrap_or(state)
        .replace("Release", "");
    if short.is_empty() {
        state.to_string()
    } else {
        short
    }
}

/// Seconds rounded to milliseconds, for the log.
fn ms(x: f32) -> f64 {
    (f64::from(x) * 1000.0).round() / 1000.0
}

#[derive(Serialize)]
struct RequestLine<'a> {
    ev: &'static str,
    t: f64,
    request: &'a BrainRequest,
}

#[derive(Serialize)]
struct OutcomeLine<'a> {
    ev: &'static str,
    t: f64,
    whiffed: bool,
    #[serde(flatten)]
    exchange: &'a Exchange,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_tracks_steps_and_expires_cards_without_logging() {
        let mut d = learning_driver();
        d.running = d.actions.parse("walk_in4>hold2>3013").unwrap();
        d.overlay_events.step = 1;
        d.overlay_events.checkpoint = Some((3.0, "wolf_posture_50 -> keep".into()));
        d.overlay_events.ban = Some((5.0, "banned 3010: 9/9 whiffs".into()));
        d.overlay_events.scorecard = Some((5.0, "boss wins".into()));
        d.memory.coach.plan = Some(GamePlan {
            principles: vec!["Wolf guards after a kick".into()],
            boss_principles: vec!["Kick closes distance".into()],
            ..Default::default()
        });
        d.overlay_events.coach = Some((5.0, "coach (mid-fight, 14 s): press".into()));
        d.coach_flights.push(CoachFlight {
            id: 1,
            sent: 5.0,
            mid: true,
            fight: 1,
        });
        let view = d.overlay();
        assert_eq!(view.running, "walk_in4 > [hold2] > 3013");
        assert!(view.checkpoint.unwrap().ends_with("2.0 s ago"));
        assert!(view.thinking && view.coach.is_some() && view.ban.is_some());
        assert!(view.principles[0].starts_with("Boss ·"));
        assert!(view.principles[1].starts_with("Player ·"));
        d.time = 10.0;
        assert!(d.overlay().ban.is_none());
        assert!(d.overlay().scorecard.is_some());
        d.time = 11.0;
        assert!(d.overlay().scorecard.is_none());
        d.time = 13.0;
        assert!(d.overlay().coach.is_none());
        assert_eq!(d.overlay().principles.len(), 2);
        d.running = Plan::default();
        assert_eq!(d.overlay().running, "Lua");
        let mut owl = BrainDriver::new(Box::new(crate::brain::LuaBrain), &crate::brain::OWL);
        owl.running = owl.actions.parse("3009").unwrap();
        owl.boss_open = Some(3014);
        assert_eq!(owl.overlay().label, "second shuriken");
    }

    #[test]
    fn overlay_uses_newest_accepted_coach_note_and_shared_ban_reason() {
        let mut d = learning_driver();
        d.coach_flights.push(CoachFlight {
            id: 2,
            sent: 1.0,
            mid: true,
            fight: 1,
        });
        d.take_coach_reply(CoachReply {
            id: 2,
            plan: Some(GamePlan {
                opener: "3005".into(),
                notes: "new lesson".into(),
                ..Default::default()
            }),
            ..Default::default()
        });
        assert_eq!(
            d.overlay().coach.as_deref(),
            Some("coach (mid-fight, 4 s): new lesson")
        );
        d.coach_flights.push(CoachFlight {
            id: 1,
            sent: 1.0,
            mid: true,
            fight: 1,
        });
        d.take_coach_reply(CoachReply {
            id: 1,
            plan: Some(GamePlan {
                opener: "3005".into(),
                notes: "old lesson".into(),
                ..Default::default()
            }),
            ..Default::default()
        });
        assert!(d.overlay().coach.unwrap().contains("new lesson"));
        d.boss_knowledge.bans.push(super::super::coach::Avoid {
            choice: "3000".into(),
            reason: "shared: 9/9 whiffs".into(),
        });
        d.apply_boss_bans();
        assert_eq!(d.overlay().ban.as_deref(), Some("banned 3000: 9/9 whiffs"));
    }

    fn layer_test_path() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sekiro-layers-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("c1010.json")
    }

    #[test]
    fn new_player_uses_rich_shared_plan_and_bans_but_not_other_player_habits() {
        let path = layer_test_path();
        let mut a = learning_driver();
        a.coach_mode = true;
        a.use_player_memory(path.clone(), "alice");
        a.memory.fights = 21;
        a.memory.habits.jumps = 77;
        a.memory.coach.failures.insert(
            "3000".into(),
            PlanFailures {
                uses: 9,
                fatal: 9,
                ..Default::default()
            },
        );
        a.memory.boss.insert(
            "3003".into(),
            Counts {
                n: 9,
                whiffed: 9,
                ..Default::default()
            },
        );
        a.coach_flights.push(CoachFlight {
            id: 1,
            sent: a.time,
            mid: false,
            fight: a.fight_no,
        });
        a.take_coach_reply(CoachReply {
            id: 1,
            plan: Some(GamePlan {
                opener: "3005".into(),
                boss_principles: vec!["kick has short recovery".into()],
                principles: vec!["Alice jumps".into()],
                notes: "Alice is aggressive".into(),
                ..Default::default()
            }),
            ..Default::default()
        });
        a.boss_knowledge
            .consider(a.memory.coach.plan.as_ref().unwrap(), &a.memory);
        a.save_memory();
        let mut b = learning_driver();
        b.coach_mode = true;
        b.use_player_memory(path.clone(), "bob");
        assert_eq!(b.game_plan.as_ref().unwrap().opener, "3005");
        assert_eq!(b.opener.as_ref().unwrap().value(), "3005");
        assert!(b.banned.contains("3003"));
        assert!(!b.banned.contains("3000"));
        assert_eq!(b.memory.habits.jumps, 0);
        let request = serde_json::to_value(b.coach_input(&FightRecord::default())).unwrap();
        assert_eq!(
            request["boss_knowledge"]["principles"][0],
            "kick has short recovery"
        );
        assert!(!request.to_string().contains("Alice"));
        b.memory.fights = 1;
        b.save_memory();
        b.save_memory();
        assert_eq!(b.boss_knowledge.evidence.fights, 22);
        assert_eq!(b.boss_knowledge.summary().players, 2);
        // A stale Alice driver must not overwrite Bob's durable contribution.
        a.save_memory();
        assert_eq!(a.boss_knowledge.evidence.fights, 22);
        let paths = LayerPaths::new(&layer_test_path(), "bob");
        let existing = LayerPaths::new(&path, "bob");
        let (mut boss, profile) = existing.load("c1010").unwrap();
        boss.players.get_mut("alice").unwrap().fights = 2;
        boss.update("bob", &profile);
        paths.save(&boss, &profile).unwrap();
        let mut c = learning_driver();
        c.coach_mode = true;
        c.use_player_memory(paths.legacy, "charlie");
        assert!(c.game_plan.is_none() && c.opener.is_none());
        assert!(c.banned.contains("3003"));
    }

    #[test]
    fn a_lua_fallback_does_not_hide_a_banned_active_set() {
        let mut d = learning_driver();
        d.set = vec![
            Weighted {
                plan: Some(d.actions.parse("3000").unwrap()),
                weight: 1.0,
            },
            Weighted {
                plan: None,
                weight: 0.25,
            },
        ];
        assert!(!d.active_set_banned());
        d.banned.insert("3000".into());
        assert!(d.active_set_banned());
        d.set.push(Weighted {
            plan: Some(d.actions.parse("3001").unwrap()),
            weight: 1.0,
        });
        assert!(!d.active_set_banned());
    }

    #[test]
    fn coach_keeps_lessons_and_reports_lua_actions_separately() {
        let mut d = learning_driver();
        d.memory.coach.plan = Some(GamePlan {
            principles: vec!["kick 3005 lands".into()],
            ..Default::default()
        });
        d.game_plan = d.memory.coach.plan.clone();
        d.tally.record(
            0.0,
            TallyEvent::Start {
                side: Side::Boss,
                key: "3005".into(),
            },
        );
        d.tally.record(
            0.5,
            TallyEvent::Hit {
                attacker: Side::Boss,
                guarded: false,
                deflected: false,
            },
        );
        d.tally.flush(3.0);
        d.fight.action_ranges.insert("3005".into(), (2, 4.0, 6.0));
        let mut input = d.coach_input(&FightRecord::default());
        assert_eq!(input.principles, ["kick 3005 lands"]);
        assert!(input.played.as_ref().unwrap().principles.is_empty());
        assert_eq!(input.actions["3005"].hit, 1);
        assert_eq!(input.actions["3000"].n, 0);
        assert_eq!(input.action_range["3005"], [2.0, 3.0]);
        assert!(input.fit(INPUT_BUDGET) <= INPUT_BUDGET);
        assert_eq!(input.principles, ["kick 3005 lands"]);
        let encoded = serde_json::to_string(&d.memory).unwrap();
        let decoded: Memory = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.coach.plan.unwrap().principles, ["kick 3005 lands"]);
    }

    #[test]
    fn session_report_contains_action_ranges_and_break_trend_and_appends_at_exit() {
        let mut d = learning_driver();
        d.session_actions.insert(
            "3000".into(),
            Counts {
                n: 2,
                hit: 1,
                whiffed: 1,
                ..Default::default()
            },
        );
        d.session_ranges.insert("3000".into(), (2, 8.0, 6.0));
        d.session_records.push(FightRecord {
            winner: "boss".into(),
            metrics: super::super::metrics::FightMetrics {
                duration: 31.0,
                time_to_kill_wolf: Some(31.0),
                wolf_posture_breaks: 2,
                hits_after_break: 1,
                break_punish_rate: 0.5,
                wolf_posture_peak: 1.0,
                ..Default::default()
            },
            ..Default::default()
        });
        let report = d.session_report();
        assert!(report.contains("| 3000 | 2 | 1 | 0 | 0 | 1 | 0 | 4.00 / 3.00 |"));
        assert!(report.contains("| 1 | boss | 31.0 | 31.0 | 2 | 1 | 50% | 100% |"));
        d.enable_session_report(&std::env::temp_dir());
        let path = d.session_report_path.clone().unwrap();
        d.print_session_report().unwrap();
        drop(d);
        let written = std::fs::read_to_string(&path).unwrap();
        assert_eq!(written.matches("# Brain session:").count(), 2);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn wolf_posture_checkpoints_are_once_per_cycle_and_break_is_an_edge() {
        let mut c = Checkpoints::default();
        assert_eq!(c.on_wolf(0.26, false).0, ["wolf_posture_25"]);
        assert!(c.on_wolf(0.3, false).0.is_empty());
        assert_eq!(
            c.on_wolf(0.76, false).0,
            ["wolf_posture_50", "wolf_posture_75"]
        );
        assert_eq!(c.on_wolf(1.0, true), (vec!["wolf_broken"], true));
        assert!(c.on_wolf(1.0, true).0.is_empty());
        assert!(c.on_wolf(0.2, false).0.is_empty());
        assert!(c.on_wolf(0.14, false).0.is_empty());
        assert_eq!(
            c.on_wolf(0.5, false).0,
            ["wolf_posture_25", "wolf_posture_50"]
        );
    }

    #[test]
    fn failed_break_responses_are_banned_independently_of_regular_uses() {
        let mut d = learning_driver();
        for i in 0..3 {
            let start = i as f32;
            d.fight
                .runs
                .push((start, "3000".into(), "on_wolf_break".into()));
            d.fight.uses.push(PlanUse {
                break_response: true,
                failed: false,
                text: "on_wolf_break:3000".into(),
                start,
                last_attack: Some(start),
                attacks: 1,
                deflected: 0,
                fatal: false,
                settled: false,
            });
            d.attribute(&[Exchange {
                side: Side::Boss,
                key: "3000".into(),
                start,
                end: Some(start + 0.5),
                outcome: super::super::tally::Outcome {
                    punished: i == 1,
                    hit: i == 2,
                    ..Default::default()
                },
            }]);
        }
        d.settle_plans(true);
        assert!(d.banned.contains("on_wolf_break:3000"));
        assert!(!d.banned.contains("3000"));
        assert_eq!(d.fight.break_plans["3000"].n, 3);
        assert_eq!(d.fight.break_plans["3000"].hit, 1);
        assert_eq!(d.fight.break_plans["3000"].whiffed, 2);
        assert_eq!(d.fight.break_plans["3000"].punished, 1);
    }

    fn learning_driver() -> BrainDriver {
        let mut d = BrainDriver::new(Box::new(crate::brain::LuaBrain), &crate::brain::SOLDIER);
        d.time = 5.0;
        d.fight.metrics.input_tick(
            &crate::InputFrame {
                move_stick: [0.0, 1.0],
                buttons: crate::Buttons {
                    attack: true,
                    guard: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            0.1,
            5.0,
        );
        d
    }

    #[test]
    fn a_fight_that_becomes_inactive_discards_its_provisional_learning() {
        let mut d = learning_driver();
        d.fight.failures.insert(
            "3000".into(),
            PlanFailures {
                failed: 0,
                uses: 3,
                deflected_all: 3,
                fatal: 0,
            },
        );
        d.settle_plans(false);
        assert!(d.banned.contains("3000"));
        d.time = 20.0;
        d.end_fight(FightEnd::BossWon);
        assert!(d.last_record.as_ref().unwrap().metrics.void);
        assert!(d.banned.is_empty());
        assert!(d.memory.coach.failures.is_empty());
        assert!(d.memory.history.is_empty());
        assert_eq!(d.memory.fights, 0);
    }

    #[test]
    fn losing_plan_is_banned_across_fights_and_only_a_reasoned_retry_reopens_it() {
        let mut d = learning_driver();
        d.memory.coach.failures.insert(
            "3000".into(),
            PlanFailures {
                failed: 0,
                uses: 2,
                deflected_all: 1,
                fatal: 0,
            },
        );
        d.fight.runs = vec![
            (0.0, "3000".into(), "press".into()),
            (1.0, "lua".into(), "lua".into()),
        ];
        d.fight.uses.push(PlanUse {
            break_response: false,
            failed: false,
            text: "3000".into(),
            start: 0.0,
            last_attack: Some(0.1),
            attacks: 1,
            deflected: 0,
            fatal: false,
            settled: false,
        });
        d.settle(&[Exchange {
            side: Side::Boss,
            key: "3000".into(),
            start: 0.1,
            end: Some(0.7),
            outcome: super::super::tally::Outcome {
                deflected: true,
                ..Default::default()
            },
        }]);
        d.settle_plans(false);
        assert!(d.banned.contains("3000"));
        assert_eq!(d.fight.failures["3000"].deflected_all, 1);
        d.end_fight(FightEnd::BossWon);
        assert_eq!(d.memory.coach.failures["3000"].uses, 3);
        assert_eq!(d.memory.coach.failures["3000"].deflected_all, 2);
        let mut input = d.coach_input(d.last_record.as_ref().unwrap());
        assert_eq!(input.banned, ["3000"]);
        assert_eq!(input.failures["3000"], [3, 2, 0, 0]);
        assert!(input.fit(INPUT_BUDGET) <= INPUT_BUDGET);
        d.start_fight();
        assert!(d.banned.contains("3000"));
        for (id, reason) in [(1, ""), (2, "Try after a bait to change the timing")] {
            d.coach_flights.push(CoachFlight {
                id,
                sent: d.time,
                mid: false,
                fight: d.fight_no,
            });
            d.take_coach_reply(CoachReply {
                id,
                plan: Some(GamePlan {
                    opener: "3000".into(),
                    retry: vec![super::super::coach::Avoid {
                        choice: "3000".into(),
                        reason: reason.into(),
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            });
            assert_eq!(d.banned.contains("3000"), reason.is_empty());
        }
        // A retry preserves all historical counts; three new failing uses ban it again.
        assert_eq!(d.memory.coach.failures["3000"].uses, 3);
        d.fight.failures.insert(
            "3000".into(),
            PlanFailures {
                failed: 0,
                uses: 3,
                deflected_all: 2,
                fatal: 0,
            },
        );
        d.time = 10.0;
        d.fight.metrics.input_tick(
            &crate::InputFrame {
                move_stick: [0.0, 1.0],
                buttons: crate::Buttons {
                    attack: true,
                    guard: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            5.1,
            5.0,
        );
        d.settle_plans(false);
        assert!(d.banned.contains("3000"));
    }

    #[test]
    fn fatal_window_and_punished_posture_break_are_attributed_once() {
        let mut d = learning_driver();
        d.fight.runs.push((0.0, "3000".into(), "press".into()));
        d.fight.uses.push(PlanUse {
            break_response: false,
            failed: false,
            text: "3000".into(),
            start: 0.0,
            last_attack: Some(1.0),
            attacks: 1,
            deflected: 0,
            fatal: false,
            settled: false,
        });
        d.fatal_plans(4.1, false);
        assert!(!d.fight.uses[0].fatal);
        d.fatal_plans(4.0, false);
        assert!(d.fight.uses[0].fatal);
        d.fight.uses[0].fatal = false;
        d.fatal_plans(4.1, true);
        assert!(d.fight.uses[0].fatal);
        d.settle_plans(true);
        d.settle_plans(true);
        assert_eq!(d.fight.failures["3000"].fatal, 1);
        assert_eq!(d.fight.failures["3000"].uses, 1);
        assert_eq!(d.fight.failures["3000"].deflected_all, 0);
    }

    #[test]
    fn dying_during_the_last_attack_is_fatal_when_the_fight_flushes() {
        let mut d = learning_driver();
        d.fight.runs.push((0.0, "3000".into(), "press".into()));
        d.fight.uses.push(PlanUse {
            break_response: false,
            failed: false,
            text: "3000".into(),
            start: 0.0,
            last_attack: Some(1.0),
            attacks: 1,
            deflected: 0,
            fatal: false,
            settled: false,
        });
        d.tally.record(
            1.0,
            TallyEvent::Start {
                side: Side::Boss,
                key: "3000".into(),
            },
        );
        d.end_fight(FightEnd::WolfWon);
        assert_eq!(d.memory.coach.failures["3000"].fatal, 1);
    }

    #[test]
    fn posture_checkpoints_rearm_and_leave_time_to_act() {
        let mut c = Checkpoints::default();
        assert_eq!(c.on_vitals(0.4, 1.0, Some(6.1)), None);
        assert_eq!(c.on_vitals(0.41, 1.0, Some(6.0)), Some("pressure"));
        assert_eq!(c.on_vitals(0.5, 1.0, Some(3.1)), None);
        assert_eq!(c.on_vitals(0.6, 1.0, Some(3.0)), Some("pressure_critical"));
        assert_eq!(c.on_vitals(0.3, 1.0, Some(6.0)), None);
        assert_eq!(c.on_vitals(0.6, 1.0, Some(2.0)), None);
        assert_eq!(c.on_vitals(0.29, 1.0, Some(8.0)), None);
        assert_eq!(c.on_vitals(0.6, 1.0, Some(2.0)), Some("pressure_critical"));
        let mut c = Checkpoints::default();
        assert_eq!(c.on_vitals(0.9, 1.0, Some(1.9)), None);
        assert_eq!(c.on_vitals(0.9, 0.4, None), Some("half_hp"));
    }
    #[test]
    fn wolf_landing_two_hits_in_a_row_raises_one_checkpoint() {
        let mut c = Checkpoints::default();
        assert_eq!(c.on_hit(Side::Wolf, false, false), None);
        assert_eq!(c.on_hit(Side::Wolf, false, false), Some("punished_twice"));
        // Once per fight.
        assert_eq!(c.on_hit(Side::Wolf, false, false), None);
        // A boss hit that lands breaks the run; a hit he guards does not count as a landing.
        let mut c = Checkpoints::default();
        c.on_hit(Side::Wolf, false, false);
        c.on_hit(Side::Boss, false, false);
        assert_eq!(c.on_hit(Side::Wolf, false, false), None);
        c.on_hit(Side::Wolf, true, false);
        assert_eq!(c.on_hit(Side::Wolf, false, false), Some("punished_twice"));
    }

    #[test]
    fn three_deflects_in_a_row_raise_one_checkpoint() {
        let mut c = Checkpoints::default();
        assert_eq!(c.on_hit(Side::Boss, true, true), None);
        assert_eq!(c.on_hit(Side::Boss, true, true), None);
        // A guard that is not a deflect breaks the run.
        assert_eq!(c.on_hit(Side::Boss, true, false), None);
        assert_eq!(c.on_hit(Side::Boss, true, true), None);
        assert_eq!(c.on_hit(Side::Boss, true, true), None);
        assert_eq!(c.on_hit(Side::Boss, true, true), Some("deflected_thrice"));
        assert_eq!(c.on_hit(Side::Boss, true, true), None);
    }
}

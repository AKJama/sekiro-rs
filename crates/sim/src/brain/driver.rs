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
use super::habits::{HabitTracker, WolfFrame};
use super::memory::{FightEnd, Loaded, Memory, MemorySummary};
use super::metrics::{FightRecord, MetricsTracker, percentile};
use super::observation::observe;
use super::plan::{Plan, Step, Weighted};
use super::sampler::{self, Context, EXCLUDE_STREAK, Pick, Rng, Streaks};
use super::tally::{Exchange, Side, Tally, TallyEvent};
use super::{BossBrain, BrainReply, BrainRequest, REQUEST_BUDGET, RequestMode};
use crate::deathblow::deathblow_marks;
use crate::duel::{Duel, DuelReport, FLAG_PARRY_TIMING};
use crate::hits::FLAG_GUARD;
use crate::lua_ai::{ActSlot, AiBrain, anim_id_from_clip};

/// Seconds of fight between saves of an unfinished fight.
pub const AUTOSAVE_SECS: f32 = 15.0;

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
    half_hp: bool,
    marks: Option<i32>,
    punish_run: u32,
    punished_twice: bool,
    deflect_run: u32,
    deflected_thrice: bool,
}

impl Checkpoints {
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
    metrics: MetricsTracker,
    requests: u32,
    decisions: u32,
    lua_acts: u32,
    choices: BTreeMap<String, u32>,
    /// Request to answer, milliseconds.
    latencies: Vec<f32>,
    tokens_at_start: (u64, u64),
}

pub struct BrainDriver {
    brain: Box<dyn BossBrain>,
    actions: &'static BossActions,
    /// This fight's tally.
    pub tally: Tally,
    /// Wolf's habits in this fight.
    pub habits: HabitTracker,
    /// Every earlier fight (loaded from the memory file, plus the fights of this session).
    pub memory: Memory,
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
}

impl BrainDriver {
    pub fn new(brain: Box<dyn BossBrain>, actions: &'static BossActions) -> Self {
        let env = |name: &str| std::env::var(name).ok();
        let mode = env("SEKIRO_BRAIN_MODE")
            .and_then(|m| RequestMode::by_name(&m))
            .unwrap_or_else(|| brain.default_mode());
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
            brain,
            actions,
            tally: Tally::default(),
            habits: HabitTracker::new(actions.back_steps),
            memory: Memory::new(actions.chr),
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
            pending_why: Some("start"),
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
        }
    }

    pub fn brain_name(&self) -> &'static str {
        self.brain.name()
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
        let chr = self.actions.chr;
        let (memory, how) = Memory::load(&path, chr);
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
        self.memory_path = Some(path);
        how
    }

    pub fn memory_path(&self) -> Option<&Path> {
        self.memory_path.as_deref()
    }

    /// The memory as it would be saved now: every earlier fight, plus this one as unfinished
    /// if anything happened in it.
    pub fn memory_now(&self) -> Memory {
        let mut m = self.memory.clone();
        if !self.fight_over && !self.tally.is_empty() {
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
        if let Err(e) = self.memory_now().save(&path) {
            self.stats.save_errors += 1;
            eprintln!("brain memory: saving {} failed: {e}", path.display());
        }
    }

    /// What a request carries: every fight so far, this one included (the fight count and
    /// wins are the earlier fights'), and Wolf's recent play.
    pub fn memory_summary(&self) -> MemorySummary {
        let mut m = self.memory.clone();
        if !self.fight_over {
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
    }

    /// Before the Lua AI thinks: installs the act hook, collects an answer, picks the plan for
    /// the next slot and drops a stale one.
    pub fn before_think(&mut self, ai: &mut AiBrain, hit_radius: f32) {
        ai.enable_act_hook(self.actions.reflex_acts);
        if let Some(reply) = self.brain.poll(self.time) {
            self.take_reply(reply);
        }
        if self.fight_over {
            self.ready = None;
        }
        // A plan that is no longer waiting (a new fight began) must not run.
        if self.ready.is_none()
            && let Some(hook) = ai.state.act_hook.as_mut()
        {
            hook.choice = None;
        }
        if self.fight_over {
            return;
        }
        let stale = self
            .ready
            .as_ref()
            .is_some_and(|r| self.time - r.since > self.max_staleness);
        if stale {
            let plan = self.ready.take().map(|r| r.plan);
            if let Some(hook) = ai.state.act_hook.as_mut() {
                hook.choice = None;
            }
            self.picked = false;
            if !self.persist {
                self.stats.stale += 1;
                self.want_request = true;
                self.write(json!({"ev": "stale", "t": ms(self.time),
                    "plan": plan.map(|p| p.value())}));
            }
        }
        if !self.picked && !self.set.is_empty() {
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
        self.set = set;
        self.set_time = sent.sent;
        self.ready = None;
        self.picked = false;
        self.want_request = false;
    }

    /// Samples the plan for the next slot from the set and hands it to the act hook.
    fn pick_next(&mut self, ai: &mut AiBrain, hit_radius: f32) {
        let ctx = Context {
            streaks: &self.streaks,
            tried: &self.tried,
            temperature: self.temperature,
        };
        let pick = sampler::pick(&self.set, &ctx, &mut self.rng);
        let plan = match pick {
            Pick::Nothing => return,
            Pick::Lua => None,
            Pick::Plan(i) => self.set[i].plan.clone(),
        };
        if !self.persist {
            self.set.clear();
        }
        self.picked = true;
        let Some(plan) = plan else { return };
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
                self.ready = Some(Ready { since, plan });
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
            let by = match slot {
                ActSlot::External => {
                    let ready = self.ready.take();
                    self.running = ready.map(|r| r.plan).unwrap_or_default();
                    self.history.push(self.running.clone());
                    self.stats.planned += 1;
                    self.picked = false;
                    let value = self.running.value();
                    self.tried.insert(value.clone());
                    if !quiet {
                        self.fight.decisions += 1;
                        *self.fight.choices.entry(value).or_default() += 1;
                    }
                    "plan"
                }
                ActSlot::Script => {
                    self.running = Plan::default();
                    self.stats.fallback += 1;
                    self.fight.lua_acts += 1;
                    self.picked = false;
                    "lua"
                }
                ActSlot::Reflex => {
                    self.running = Plan::default();
                    self.stats.reflex += 1;
                    self.fight.lua_acts += 1;
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
        if quiet || !self.brain.plans() || self.in_flight.is_some() {
            return;
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
        let mut request = BrainRequest {
            id: self.next_id,
            obs: observe(duel, self.time, self.since_hit, &self.running.value()),
            tally: self.tally.summary(),
            memory: Some(self.memory_summary()),
            why,
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
    pub fn after_step(&mut self, duel: &Duel, report: &DuelReport) {
        if report.respawned {
            self.start_fight();
        }
        if self.fight_over {
            return;
        }
        let t = self.time;
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
            events.push(match boss {
                Some(a) => TallyEvent::Start {
                    side: Side::Boss,
                    key: a.to_string(),
                },
                None => TallyEvent::End { side: Side::Boss },
            });
            self.boss_open = boss;
            // An attack or step starting closes an open tactic; its end closes nothing more.
            if boss.is_some() {
                self.tactic_open = false;
            }
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
        for (hit, result) in &report.hits {
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
        let boss_fighter = &duel.rules.fighters[1];
        if !self.checkpoints.half_hp && boss_fighter.hp_fraction() <= 0.5 {
            self.checkpoints.half_hp = true;
            self.pending_why.get_or_insert("half_hp");
        }
        let marks = deathblow_marks(boss_fighter.vitality.deathblows_left);
        if self.checkpoints.marks.is_some_and(|m| marks < m) {
            self.pending_why.get_or_insert("mark_lost");
        }
        self.checkpoints.marks = Some(marks);
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
        self.log_outcomes(t, &done);
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

    /// Ends the fight: settles it, adds it and its record to the memory and saves.
    pub fn end_fight(&mut self, end: FightEnd) {
        if self.fight_over {
            return;
        }
        let t = self.time;
        let done = self.tally.flush(t);
        self.log_outcomes(t, &done);
        self.habits.flush(t);
        self.memory.add_fight(&self.tally, &self.habits.habits, end);
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
        let metrics = self
            .fight
            .metrics
            .finish(t, punishes, end == FightEnd::WolfWon);
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
        };
        eprintln!("{}", record.summary_line());
        self.write(json!({"ev": "fight_end", "t": ms(t), "winner": winner,
            "fights": self.memory.fights, "summary": record.summary_line()}));
        self.write_line(&json!({"ev": "fight_summary", "t": ms(t), "record": record}));
        self.memory.push_record(record.clone());
        self.last_record = Some(record);
        self.save_memory();
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
        self.pending_why = Some("start");
        self.fight = FightState {
            metrics: MetricsTracker::new(self.time),
            tokens_at_start: self.brain.usage(),
            ..FightState::default()
        };
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
                whiffed: x.outcome.whiffed(),
                exchange: x,
            });
        }
    }
}

impl Drop for BrainDriver {
    /// The game is closing: keep an unfinished fight in the memory file.
    fn drop(&mut self) {
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

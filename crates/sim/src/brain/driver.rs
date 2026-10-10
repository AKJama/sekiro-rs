//! [`BrainDriver`]: runs a [`BossBrain`] alongside the boss's Lua AI in a duel.
//!
//! Timing model, per duel step:
//!
//! 1. Poll the controller. A plan that arrives is checked against the boss's action list and
//!    handed to the Lua runtime's act hook, where it waits for the next *slot*: the next time
//!    his battle goal picks an act.
//! 2. The Lua AI thinks. At a slot the waiting plan is queued instead of the script's pick,
//!    unless the moment is a reaction; with no plan waiting, the script's own pick runs (the
//!    fallback: the boss never waits for a controller).
//! 3. After a slot, the next request goes out at once, so the controller thinks while the
//!    current act runs. One request is in flight at a time.
//! 4. After the step, hits and action changes feed the [`Tally`].
//!
//! A plan whose observation is older than [`BrainDriver::max_staleness`] when it would run is
//! dropped and a fresh request is sent.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::json;

use super::actions::BossActions;
use super::observation::observe;
use super::tally::{Exchange, Side, Tally, TallyEvent};
use super::{BossBrain, BrainRequest, Plan};
use crate::duel::{Duel, DuelReport, FLAG_PARRY_TIMING};
use crate::lua_ai::{ActSlot, AiBrain, anim_id_from_clip};

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
}

/// The waiting plan, with the request it answers.
#[derive(Debug, Clone)]
struct Ready {
    id: u64,
    obs_time: f32,
    plan: Plan,
}

pub struct BrainDriver {
    brain: Box<dyn BossBrain>,
    actions: &'static BossActions,
    pub tally: Tally,
    pub stats: BrainStats,
    /// Seconds an observation may age before its plan runs; older plans are dropped.
    pub max_staleness: f32,
    /// Fight clock, seconds.
    pub time: f32,
    in_flight: Option<(u64, f32)>,
    ready: Option<Ready>,
    want_request: bool,
    next_id: u64,
    /// The plan running now (empty: the Lua AI's own act).
    pub running: Vec<i32>,
    /// Every plan that started, in order, for tests.
    pub history: Vec<Vec<i32>>,
    /// Seconds since each side last landed an unguarded hit `[owl, wolf]`.
    since_hit: [Option<f32>; 2],
    owl_open: Option<i32>,
    wolf_open: Option<String>,
    log: Option<BufWriter<File>>,
    log_path: Option<PathBuf>,
}

impl BrainDriver {
    pub fn new(brain: Box<dyn BossBrain>, actions: &'static BossActions) -> Self {
        Self {
            brain,
            actions,
            tally: Tally::default(),
            stats: BrainStats::default(),
            max_staleness: 6.0,
            time: 0.0,
            in_flight: None,
            ready: None,
            want_request: true,
            next_id: 1,
            running: Vec::new(),
            history: Vec::new(),
            since_hit: [None, None],
            owl_open: None,
            wolf_open: None,
            log: None,
            log_path: None,
        }
    }

    pub fn brain_name(&self) -> &'static str {
        self.brain.name()
    }

    /// Writes a JSONL log of requests, replies, slots and outcomes into `dir` (created if
    /// missing), one file per fight; returns its path.
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
        let actions: Vec<_> = self
            .actions
            .actions
            .iter()
            .map(|a| json!({"id": a.id, "label": a.label}))
            .collect();
        self.write(json!({
            "ev": "start",
            "brain": self.brain.name(),
            "chr": self.actions.chr,
            "max_plan": self.actions.max_plan,
            "actions": actions,
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

    /// Advances the fight clock; call once at the start of every duel step.
    pub fn tick(&mut self, dt: f32) {
        self.time += dt;
        for s in self.since_hit.iter_mut().flatten() {
            *s += dt;
        }
    }

    /// Before the Lua AI thinks: installs the act hook, collects an answer and drops a stale
    /// plan.
    pub fn before_think(&mut self, ai: &mut AiBrain, hit_radius: f32) {
        ai.enable_act_hook(self.actions.reflex_acts);
        if let Some(reply) = self.brain.poll(self.time) {
            let obs_time = match self.in_flight.take() {
                Some((id, t)) if id == reply.id => t,
                other => {
                    // An answer to a request we no longer wait for.
                    self.in_flight = other;
                    self.write(json!({"ev": "late", "t": ms(self.time), "id": reply.id}));
                    return;
                }
            };
            self.stats.replies += 1;
            let plan = reply.plan.map(|p| p.actions);
            self.write(json!({
                "ev": "reply", "t": ms(self.time), "id": reply.id, "plan": plan,
                "latency": ms(self.time - obs_time),
            }));
            if let Some(plan) = plan {
                match self.actions.act_for(&plan, hit_radius) {
                    Ok(choice) => {
                        if let Some(hook) = ai.state.act_hook.as_mut() {
                            hook.choice = Some(choice);
                        }
                        self.ready = Some(Ready {
                            id: reply.id,
                            obs_time,
                            plan: Plan { actions: plan },
                        });
                    }
                    Err(e) => {
                        self.stats.invalid += 1;
                        self.write(json!({
                            "ev": "invalid", "t": ms(self.time), "id": reply.id,
                            "error": e.to_string(),
                        }));
                    }
                }
            }
        }
        if let Some(r) = &self.ready
            && self.time - r.obs_time > self.max_staleness
        {
            let id = r.id;
            self.ready = None;
            if let Some(hook) = ai.state.act_hook.as_mut() {
                hook.choice = None;
            }
            self.stats.stale += 1;
            self.want_request = true;
            self.write(json!({"ev": "stale", "t": ms(self.time), "id": id}));
        }
    }

    /// After the Lua AI thought: notes who chose any act that started and sends the next
    /// request when one is due.
    pub fn after_think(&mut self, ai: &mut AiBrain, duel: &Duel) {
        let slots = ai
            .state
            .act_hook
            .as_mut()
            .map(|h| std::mem::take(&mut h.slots))
            .unwrap_or_default();
        for (_, slot) in slots {
            let by = match slot {
                ActSlot::External => {
                    let ready = self.ready.take();
                    self.running = ready.map(|r| r.plan.actions).unwrap_or_default();
                    self.history.push(self.running.clone());
                    self.stats.planned += 1;
                    "plan"
                }
                ActSlot::Script => {
                    self.running.clear();
                    self.stats.fallback += 1;
                    "lua"
                }
                ActSlot::Reflex => {
                    self.running.clear();
                    self.stats.reflex += 1;
                    "reflex"
                }
            };
            self.want_request = true;
            let plan = self.running.clone();
            self.write(json!({"ev": "slot", "t": ms(self.time), "by": by, "plan": plan}));
        }
        if self.brain.plans()
            && self.want_request
            && self.in_flight.is_none()
            && self.ready.is_none()
        {
            self.want_request = false;
            let request = BrainRequest {
                id: self.next_id,
                obs: observe(duel, self.time, self.since_hit, &self.running),
                tally: self.tally.summary(),
            };
            self.next_id += 1;
            self.stats.requests += 1;
            self.in_flight = Some((request.id, self.time));
            self.brain.submit(&request);
            if self.log.is_some() {
                self.write_line(&RequestLine {
                    ev: "request",
                    t: ms(self.time),
                    request: &request,
                });
            }
        }
    }

    /// After the duel step: action changes and hits into the tally.
    pub fn after_step(&mut self, duel: &Duel, report: &DuelReport) {
        let t = self.time;
        // The boss's actions: his attack and step animations (guards and deflects are
        // reactions, not actions).
        let owl = duel
            .enemy
            .behavior
            .runtime
            .main_clip()
            .and_then(|c| anim_id_from_clip(&c.animation))
            .filter(|a| (3000..3100).contains(a) || (5200..5300).contains(a));
        if owl != self.owl_open || (owl.is_some() && report.enemy.clip_started) {
            let event = match owl {
                Some(a) => TallyEvent::Start {
                    side: Side::Owl,
                    key: a.to_string(),
                },
                None => TallyEvent::End { side: Side::Owl },
            };
            self.tally.record(t, event);
            self.owl_open = owl;
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
            self.tally.record(t, TallyEvent::End { side: Side::Wolf });
            self.wolf_open = None;
        }
        if self.wolf_open.is_none() && duel.player.tae_frame().flag(FLAG_PARRY_TIMING) {
            let key = report
                .player
                .behavior
                .state_path
                .last()
                .cloned()
                .unwrap_or_else(|| clip.clone().unwrap_or_default());
            self.tally.record(
                t,
                TallyEvent::Start {
                    side: Side::Wolf,
                    key,
                },
            );
            self.wolf_open = clip;
        }
        for (hit, result) in &report.hits {
            let attacker = Side::from_index(hit.attacker);
            self.tally.record(
                t,
                TallyEvent::Hit {
                    attacker,
                    guarded: result.guarded,
                    deflected: result.deflected,
                },
            );
            if !result.guarded && !result.deflected {
                // since_hit is [owl, wolf].
                self.since_hit[if attacker == Side::Owl { 0 } else { 1 }] = Some(0.0);
            }
        }
        for x in self.tally.advance(t) {
            if self.log.is_some() {
                self.write_line(&OutcomeLine {
                    ev: "outcome",
                    t: ms(t),
                    whiffed: x.outcome.whiffed(),
                    exchange: &x,
                });
            }
        }
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

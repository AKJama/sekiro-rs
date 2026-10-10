//! A pluggable boss brain: the boss's high-level choices (which of his own actions to string
//! together next) can come from his Lua AI or from an outside controller, while his reflexes
//! (deflecting, guarding, reactions, getting up) stay with the Lua goal runtime.
//!
//! - [`Observation`] is what a controller sees: only what the player can see, mirrored.
//! - [`Tally`] is the fight's outcome memory, kept here because hosted models remember nothing.
//! - A [`Plan`] is 1 to 3 of the boss's own action ids ([`actions::OWL`]); it runs through his
//!   battle goal exactly like one of his acts (`lua_ai::ActHook`).
//! - [`BossBrain`] is the controller, with a non-blocking submit / poll API; [`BrainDriver`]
//!   keeps one request in flight and falls back to the Lua AI's own act when no answer is
//!   ready, so the boss never freezes.
//!
//! See `docs/BRAIN.md`.

pub mod actions;
pub mod driver;
pub mod observation;
pub mod tally;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

pub use actions::{BossAction, BossActions, OWL};
pub use driver::{BrainDriver, BrainStats};
pub use observation::{FighterView, Observation};
pub use tally::{Counts, Exchange, Outcome, Side, Tally, TallyEvent, TallySummary};

/// What the boss does next: his own action ids, run in order as one act.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub actions: Vec<i32>,
}

impl Plan {
    pub fn new(actions: &[i32]) -> Self {
        Self {
            actions: actions.to_vec(),
        }
    }
}

/// One question to a controller: the observation and the outcome memory.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BrainRequest {
    /// Increasing per fight; the reply carries it back.
    pub id: u64,
    pub obs: Observation,
    pub tally: TallySummary,
}

/// A controller's answer.
#[derive(Debug, Clone, PartialEq)]
pub struct BrainReply {
    /// The request answered.
    pub id: u64,
    /// The plan, or `None` to leave this act to the Lua AI.
    pub plan: Option<Plan>,
}

/// A boss controller. Calls never block: a controller that needs time (a hosted model) works
/// on a background thread and answers through [`BossBrain::poll`].
pub trait BossBrain {
    /// Short name for logs (`lua`, `scripted`).
    fn name(&self) -> &'static str;
    /// Whether it ever plans; `false` leaves every act to the Lua AI and sends no requests.
    fn plans(&self) -> bool {
        true
    }
    /// Takes a request. The driver keeps at most one in flight.
    fn submit(&mut self, request: &BrainRequest);
    /// The answer to the request in flight, once ready. `now` is the fight clock in seconds
    /// (deterministic controllers model their latency on it; real ones ignore it).
    fn poll(&mut self, now: f32) -> Option<BrainReply>;
}

/// Today's behaviour: the Lua AI chooses every act.
#[derive(Debug, Clone, Copy, Default)]
pub struct LuaBrain;

impl BossBrain for LuaBrain {
    fn name(&self) -> &'static str {
        "lua"
    }
    fn plans(&self) -> bool {
        false
    }
    fn submit(&mut self, _request: &BrainRequest) {}
    fn poll(&mut self, _now: f32) -> Option<BrainReply> {
        None
    }
}

/// A deterministic test controller: opens with [`ScriptedBrain::opener`] every time until its
/// first action has been deflected [`ScriptedBrain::switch_after_deflects`] times, then uses
/// [`ScriptedBrain::fallback_plan`]. Answers [`ScriptedBrain::delay`] seconds of fight time after
/// each request, to exercise the timeout path.
#[derive(Debug, Clone)]
pub struct ScriptedBrain {
    pub opener: Plan,
    pub fallback_plan: Plan,
    pub switch_after_deflects: u32,
    /// Artificial decision latency, seconds of fight time.
    pub delay: f32,
    pending: Option<(u64, f32, Plan)>,
}

impl Default for ScriptedBrain {
    /// Kick (3005); after two deflected kicks, the recovery-sealing throw (3031).
    fn default() -> Self {
        Self::new(Plan::new(&[3005]), Plan::new(&[3031]), 2, 0.1)
    }
}

impl ScriptedBrain {
    pub fn new(opener: Plan, fallback_plan: Plan, switch_after_deflects: u32, delay: f32) -> Self {
        Self {
            opener,
            fallback_plan,
            switch_after_deflects,
            delay,
            pending: None,
        }
    }

    /// The plan this controller picks for a tally.
    pub fn choose(&self, tally: &TallySummary) -> Plan {
        let deflected = self
            .opener
            .actions
            .first()
            .and_then(|a| tally.owl.get(&a.to_string()))
            .map_or(0, |c| c.deflected);
        if deflected >= self.switch_after_deflects {
            self.fallback_plan.clone()
        } else {
            self.opener.clone()
        }
    }
}

impl BossBrain for ScriptedBrain {
    fn name(&self) -> &'static str {
        "scripted"
    }
    fn submit(&mut self, request: &BrainRequest) {
        let plan = self.choose(&request.tally);
        self.pending = Some((request.id, request.obs.t + self.delay, plan));
    }
    fn poll(&mut self, now: f32) -> Option<BrainReply> {
        let (id, ready_at, _) = self.pending.as_ref()?;
        if now + 1e-4 < *ready_at {
            return None;
        }
        let id = *id;
        let (_, _, plan) = self.pending.take()?;
        Some(BrainReply {
            id,
            plan: Some(plan),
        })
    }
}

/// Which controller a fight uses (`--brain lua|scripted`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BrainKind {
    #[default]
    Lua,
    Scripted,
}

impl BrainKind {
    pub fn by_name(name: &str) -> Result<Self, String> {
        match name.to_ascii_lowercase().as_str() {
            "lua" => Ok(Self::Lua),
            "scripted" => Ok(Self::Scripted),
            other => Err(format!("unknown brain {other}: known are lua, scripted")),
        }
    }

    pub fn build(self) -> Box<dyn BossBrain> {
        match self {
            Self::Lua => Box::new(LuaBrain),
            Self::Scripted => Box::new(ScriptedBrain::default()),
        }
    }
}

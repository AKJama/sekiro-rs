//! The score to beat: how well Wolf played each fight, as numbers the boss's controller is
//! asked to make worse, kept per fight (the last [`HISTORY`] in the memory file) and summed up
//! as a compact trend in every request.
//!
//! Definitions, all about Wolf, over one fight (from its start or a respawn until a death):
//!
//! - `deflected`, `guarded`, `hits_taken`: how each of the boss's hits on Wolf was resolved (a
//!   deflect, a guard that was not a deflect, or an unguarded hit that landed). The perfect-
//!   deflect rate is `deflected / (deflected + guarded + hits_taken)`, the guard rate
//!   `guarded` over the same total.
//! - `hp_lost`: the boss's damage to Wolf, as a fraction of Wolf's max HP, summed over drops
//!   (healing does not take it back).
//! - `posture_gained`: the boss's posture damage to Wolf, as a fraction of his posture bar,
//!   summed over rises (recovery does not take it back).
//! - `punishes`: the boss's actions (attacks and tactics) Wolf hit during or within a second
//!   after them (the tally's `punished`).
//! - `kill_time`: seconds Wolf took to kill the boss, when he did.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Fights kept in the memory file.
pub const HISTORY: usize = 10;
/// Fights in a request's trend.
pub const TREND_FIGHTS: usize = 3;

/// Wolf's numbers for one fight.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FightMetrics {
    pub duration: f32,
    pub deflected: u32,
    pub guarded: u32,
    pub hits_taken: u32,
    pub hp_lost: f32,
    pub posture_gained: f32,
    pub punishes: u32,
    pub kill_time: Option<f32>,
}

impl FightMetrics {
    /// The boss's hits that were resolved against Wolf.
    pub fn resolved(&self) -> u32 {
        self.deflected + self.guarded + self.hits_taken
    }

    fn rate(&self, n: u32) -> Option<f32> {
        (self.resolved() > 0).then(|| n as f32 / self.resolved() as f32)
    }

    /// Perfect-deflect rate, if the boss landed or had hits resolved at all.
    pub fn deflect_rate(&self) -> Option<f32> {
        self.rate(self.deflected)
    }

    pub fn guard_rate(&self) -> Option<f32> {
        self.rate(self.guarded)
    }
}

/// Collects [`FightMetrics`] as a fight goes.
#[derive(Debug, Clone, Default)]
pub struct MetricsTracker {
    pub metrics: FightMetrics,
    start: f32,
    prev_hp: Option<f32>,
    prev_posture: Option<f32>,
}

impl MetricsTracker {
    pub fn new(start: f32) -> Self {
        Self {
            start,
            ..Self::default()
        }
    }

    /// One of the boss's hits on Wolf was resolved.
    pub fn boss_hit(&mut self, guarded: bool, deflected: bool) {
        if deflected {
            self.metrics.deflected += 1;
        } else if guarded {
            self.metrics.guarded += 1;
        } else {
            self.metrics.hits_taken += 1;
        }
    }

    /// Wolf's HP and posture (fractions) after a step.
    pub fn wolf(&mut self, hp: f32, posture: f32) {
        if let Some(p) = self.prev_hp {
            self.metrics.hp_lost += (p - hp).max(0.0);
        }
        if let Some(p) = self.prev_posture {
            self.metrics.posture_gained += (posture - p).max(0.0);
        }
        self.prev_hp = Some(hp);
        self.prev_posture = Some(posture);
    }

    /// The numbers so far, at fight time `now`.
    pub fn snapshot(&self, now: f32) -> FightMetrics {
        FightMetrics {
            duration: now - self.start,
            ..self.metrics
        }
    }

    /// The fight is over at `now`: the final numbers.
    pub fn finish(&self, now: f32, punishes: u32, wolf_won: bool) -> FightMetrics {
        let mut m = self.snapshot(now);
        m.punishes = punishes;
        m.kill_time = wolf_won.then_some(m.duration);
        m
    }
}

/// One finished fight as the memory file keeps it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FightRecord {
    /// `boss`, `wolf` or `none` (unfinished).
    pub winner: String,
    pub metrics: FightMetrics,
    /// Plans of the controller that ran.
    pub decisions: u32,
    /// Acts the Lua AI chose (no plan waiting) or kept as reflexes.
    pub lua_acts: u32,
    /// Requests sent in the fight.
    pub requests: u32,
    /// Plans run, by text form (`lua` for the Lua AI's own acts is not counted).
    pub choices: BTreeMap<String, u32>,
    /// Input and output tokens used in the fight.
    pub tokens: [u64; 2],
    /// Median and 90th percentile of the time from a request to its answer, milliseconds.
    pub latency_ms: [u32; 2],
    /// The brain and request mode of the fight (`decisions/checkpoints`).
    pub brain: String,
}

impl FightRecord {
    /// One line for the log and the console.
    pub fn summary_line(&self) -> String {
        let m = &self.metrics;
        let pct = |r: Option<f32>| r.map_or("-".to_string(), |r| format!("{:.0}%", r * 100.0));
        let mut top: Vec<_> = self.choices.iter().collect();
        top.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        let hist = top
            .iter()
            .take(6)
            .map(|(k, n)| format!("{k}x{n}"))
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "fight over: {} won in {:.1}s ({}); {} plans, {} lua acts, {} requests [{}]; wolf deflect {} guard {} hits taken {} hp lost {:.0}% posture {:.0}% punishes {}; tokens {}/{}; latency p50 {} ms p90 {} ms",
            match self.winner.as_str() {
                "boss" => "boss",
                "wolf" => "Wolf",
                _ => "nobody",
            },
            m.duration,
            self.brain,
            self.decisions,
            self.lua_acts,
            self.requests,
            hist,
            pct(m.deflect_rate()),
            pct(m.guard_rate()),
            m.hits_taken,
            m.hp_lost * 100.0,
            m.posture_gained * 100.0,
            m.punishes,
            self.tokens[0],
            self.tokens[1],
            self.latency_ms[0],
            self.latency_ms[1],
        )
    }
}

/// The `p`th percentile (0 to 1) of unsorted values, 0 for none.
pub fn percentile(values: &[f32], p: f32) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut v = values.to_vec();
    v.sort_by(f32::total_cmp);
    v[((v.len() - 1) as f32 * p).round() as usize]
}

/// Wolf's recent play as a controller sees it, last fights last: percentages and counts per
/// fight, plus this fight so far.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Trend {
    /// Perfect-deflect rate, percent.
    pub defl: Vec<u32>,
    /// Guard rate, percent.
    pub guard: Vec<u32>,
    /// Hits the boss landed on Wolf.
    pub hits: Vec<u32>,
    /// The boss's damage to Wolf, percent of his max HP.
    pub hp: Vec<u32>,
    /// Seconds Wolf took to kill the boss (`null`: the boss won or it was unfinished).
    pub kill_s: Vec<Option<u32>>,
    /// This fight so far: deflect percent and hits landed on Wolf.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub now: Option<[u32; 2]>,
}

fn pct(r: Option<f32>) -> u32 {
    (r.unwrap_or(0.0) * 100.0).round() as u32
}

impl Trend {
    /// The trend over the last [`TREND_FIGHTS`] records and this fight's numbers so far.
    pub fn new(history: &[FightRecord], now: Option<&FightMetrics>) -> Self {
        let recent = &history[history.len().saturating_sub(TREND_FIGHTS)..];
        Self {
            defl: recent
                .iter()
                .map(|r| pct(r.metrics.deflect_rate()))
                .collect(),
            guard: recent.iter().map(|r| pct(r.metrics.guard_rate())).collect(),
            hits: recent.iter().map(|r| r.metrics.hits_taken).collect(),
            hp: recent
                .iter()
                .map(|r| (r.metrics.hp_lost * 100.0).round() as u32)
                .collect(),
            kill_s: recent
                .iter()
                .map(|r| r.metrics.kill_time.map(|k| k.round() as u32))
                .collect(),
            now: now.map(|m| [pct(m.deflect_rate()), m.hits_taken]),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.defl.is_empty() && self.now.is_none()
    }

    /// Drops the oldest fight (to shrink a request); true if one went.
    pub fn trim(&mut self) -> bool {
        if self.defl.len() <= 1 {
            return false;
        }
        self.defl.remove(0);
        self.guard.remove(0);
        self.hits.remove(0);
        self.hp.remove(0);
        self.kill_s.remove(0);
        true
    }
}

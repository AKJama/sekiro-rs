//! Outcome memory: how each of the boss's actions and each of Wolf's attacks ended, overall
//! and over the last few exchanges. Hosted models keep nothing between calls, so this is the
//! fight's memory, sent with every request.
//!
//! An *exchange* is one action of one side, from its start until its clip ends, plus a punish
//! window after it. Its outcome flags:
//!
//! - `hit`: one of its hits landed unguarded;
//! - `blocked`: the other side guarded one of its hits;
//! - `deflected`: the other side deflected one of its hits;
//! - `whiffed`: none of the above (dodged, out of range, interrupted before contact);
//! - `punished`: its side took an unguarded hit during it or within the punish window after.
//!
//! Hits that arrive after the clip ended (a shuriken in flight) count for the side's latest
//! exchange.

use std::collections::{BTreeMap, VecDeque};

use serde::{Serialize, Serializer};

/// Which side an exchange belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    /// The boss (index 1 in the duel).
    Owl,
    /// The player (index 0).
    Wolf,
}

impl Side {
    pub fn index(self) -> usize {
        match self {
            Side::Wolf => 0,
            Side::Owl => 1,
        }
    }

    pub fn from_index(i: usize) -> Self {
        if i == 0 { Side::Wolf } else { Side::Owl }
    }

    pub fn other(self) -> Self {
        match self {
            Side::Wolf => Side::Owl,
            Side::Owl => Side::Wolf,
        }
    }
}

/// How one exchange ended.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Outcome {
    pub hit: bool,
    pub blocked: bool,
    pub deflected: bool,
    pub punished: bool,
}

impl Outcome {
    pub fn whiffed(&self) -> bool {
        !(self.hit || self.blocked || self.deflected)
    }
}

/// One action of one side.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Exchange {
    pub side: Side,
    /// The boss's action id, or the name of Wolf's attack state.
    pub key: String,
    #[serde(serialize_with = "ser_ms")]
    pub start: f32,
    #[serde(serialize_with = "ser_ms_opt")]
    pub end: Option<f32>,
    #[serde(flatten)]
    pub outcome: Outcome,
}

/// Counts over exchanges: `[n, hit, blocked, deflected, whiffed, punished]` in JSON.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub n: u32,
    pub hit: u32,
    pub blocked: u32,
    pub deflected: u32,
    pub whiffed: u32,
    pub punished: u32,
}

impl Counts {
    fn add(&mut self, o: &Outcome) {
        self.n += 1;
        self.hit += u32::from(o.hit);
        self.blocked += u32::from(o.blocked);
        self.deflected += u32::from(o.deflected);
        self.whiffed += u32::from(o.whiffed());
        self.punished += u32::from(o.punished);
    }
}

impl Serialize for Counts {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        [
            self.n,
            self.hit,
            self.blocked,
            self.deflected,
            self.whiffed,
            self.punished,
        ]
        .serialize(s)
    }
}

/// The tally as a controller sees it: per action, overall and over the recent exchanges.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct TallySummary {
    /// Boss action id -> counts, every exchange of the fight.
    pub owl: BTreeMap<String, Counts>,
    /// The same over the last [`Tally::recent_len`] boss exchanges.
    pub owl_recent: BTreeMap<String, Counts>,
    /// Wolf's attack (state name) -> counts.
    pub wolf: BTreeMap<String, Counts>,
    pub wolf_recent: BTreeMap<String, Counts>,
}

/// Something that happened, as the tally needs it.
#[derive(Debug, Clone, PartialEq)]
pub enum TallyEvent {
    /// A side started an action (ending its previous one).
    Start { side: Side, key: String },
    /// A side's action ended without a new one.
    End { side: Side },
    /// A hit from `attacker`'s side resolved.
    Hit {
        attacker: Side,
        guarded: bool,
        deflected: bool,
    },
}

/// The running tally.
#[derive(Debug, Clone)]
pub struct Tally {
    /// Seconds after an action's end during which a hit on its side still punishes it.
    pub punish_window: f32,
    /// How many exchanges per side the recent counts cover.
    pub recent_len: usize,
    open: [Option<Exchange>; 2],
    /// Ended exchanges still inside their punish window, oldest first.
    closing: Vec<Exchange>,
    all: [BTreeMap<String, Counts>; 2],
    recent: [VecDeque<Exchange>; 2],
}

impl Default for Tally {
    fn default() -> Self {
        Self {
            punish_window: 1.0,
            recent_len: 10,
            open: [None, None],
            closing: Vec::new(),
            all: [BTreeMap::new(), BTreeMap::new()],
            recent: [VecDeque::new(), VecDeque::new()],
        }
    }
}

impl Tally {
    /// Applies an event at fight time `time`.
    pub fn record(&mut self, time: f32, event: TallyEvent) {
        match event {
            TallyEvent::Start { side, key } => {
                self.close(side, time);
                self.open[side.index()] = Some(Exchange {
                    side,
                    key,
                    start: time,
                    end: None,
                    outcome: Outcome::default(),
                });
            }
            TallyEvent::End { side } => self.close(side, time),
            TallyEvent::Hit {
                attacker,
                guarded,
                deflected,
            } => {
                if let Some(x) = self.latest_mut(attacker) {
                    if deflected {
                        x.outcome.deflected = true;
                    } else if guarded {
                        x.outcome.blocked = true;
                    } else {
                        x.outcome.hit = true;
                    }
                }
                if !guarded && !deflected {
                    let victim = attacker.other();
                    if let Some(x) = self.open[victim.index()].as_mut() {
                        x.outcome.punished = true;
                    }
                    let window = self.punish_window;
                    let recent = |x: &&mut Exchange| {
                        x.side == victim && x.end.is_some_and(|end| time - end <= window)
                    };
                    for x in self.closing.iter_mut().filter(recent) {
                        x.outcome.punished = true;
                    }
                }
            }
        }
    }

    /// Moves exchanges whose punish window has passed into the counts; returns them.
    pub fn advance(&mut self, time: f32) -> Vec<Exchange> {
        let window = self.punish_window;
        let (done, keep): (Vec<_>, Vec<_>) = self
            .closing
            .drain(..)
            .partition(|x| time - x.end.unwrap_or(time) >= window);
        self.closing = keep;
        for x in &done {
            let i = x.side.index();
            self.all[i]
                .entry(x.key.clone())
                .or_default()
                .add(&x.outcome);
            self.recent[i].push_back(x.clone());
            while self.recent[i].len() > self.recent_len {
                self.recent[i].pop_front();
            }
        }
        done
    }

    /// The side's action in progress.
    pub fn current(&self, side: Side) -> Option<&Exchange> {
        self.open[side.index()].as_ref()
    }

    /// Counts for one key of one side, over the whole fight.
    pub fn counts(&self, side: Side, key: &str) -> Counts {
        self.all[side.index()].get(key).copied().unwrap_or_default()
    }

    pub fn summary(&self) -> TallySummary {
        let recent = |i: usize| {
            let mut m: BTreeMap<String, Counts> = BTreeMap::new();
            for x in &self.recent[i] {
                m.entry(x.key.clone()).or_default().add(&x.outcome);
            }
            m
        };
        TallySummary {
            owl: self.all[1].clone(),
            owl_recent: recent(1),
            wolf: self.all[0].clone(),
            wolf_recent: recent(0),
        }
    }

    fn close(&mut self, side: Side, time: f32) {
        if let Some(mut x) = self.open[side.index()].take() {
            x.end = Some(time);
            self.closing.push(x);
        }
    }

    /// The side's open exchange, or its most recently ended one.
    fn latest_mut(&mut self, side: Side) -> Option<&mut Exchange> {
        let i = side.index();
        if self.open[i].is_some() {
            return self.open[i].as_mut();
        }
        self.closing.iter_mut().rev().find(|x| x.side == side)
    }
}

fn ms(x: f32) -> f64 {
    (f64::from(x) * 1000.0).round() / 1000.0
}

fn ser_ms<S: Serializer>(x: &f32, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_f64(ms(*x))
}

fn ser_ms_opt<S: Serializer>(x: &Option<f32>, s: S) -> Result<S::Ok, S::Error> {
    x.map(ms).serialize(s)
}

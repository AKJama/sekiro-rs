//! Map event scripts (EMEVD) at run time: a small interpreter for the instructions boss fights
//! use, so a fight's phase changes, boss bar and defeat come from the game's own event script.
//!
//! The game runs every event of `event/<map>.emevd` once per frame. Each event is a list of
//! instructions (decoded by [`sekiro_formats::emevd`], named by [`sekiro_formats::emedf`]):
//!
//! - Conditions (`If...`, banks 0 to 4) take a condition group as their first argument. Group 0
//!   ("MAIN") blocks the event until the condition holds; a positive group `n` collects
//!   conditions that must all hold (AND), a negative one conditions of which one must hold (OR).
//!   `IfConditionGroup` tests a group, either blocking (MAIN) or as a condition of another
//!   group. Once a MAIN condition passes, the collected groups are cleared.
//! - Control flow: skips, gotos to `Label<n>` instructions, ends (end type 0 ends the event,
//!   1 restarts it), and waits in seconds or frames.
//! - Commands on characters, flags, the boss health bar and the like, which the interpreter
//!   hands to its caller as [`EventCommand`]s.
//!
//! That semantics follows the community documentation (DarkScript3, SoulsFormats); the exact
//! per-frame order of the engine's event loop is not traced. We run only the events a fight
//! names (see [`crate::arena::BossEvents`]), and characters other than the ones the caller maps
//! are unknown: their conditions are false and commands on them are dropped.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use sekiro_formats::emedf::{self, Arg};
use sekiro_formats::emevd::{Emevd, Instruction};

/// The entity id the event scripts use for the player.
pub const PLAYER_ENTITY: i32 = 10000;

/// What an event asks of the world.
#[derive(Debug, Clone, PartialEq)]
pub enum EventCommand {
    /// `SetSpEffect` (2004[8]).
    SetSpEffect { entity: i32, id: i32 },
    /// `ClearSpEffect` (2004[21]).
    ClearSpEffect { entity: i32, id: i32 },
    /// `EzstateInstructionRequest` (2004[6]): a command the character's HKS script reads with
    /// `env(105, slot)` (the NPC script's `ExecEventTrandition` plays the matching event
    /// animation: 506000 is the Owl's heat-up `W_Event20005`).
    EzStateRequest { entity: i32, command: i32, slot: u8 },
    /// `DisplayBossHealthBar` (2003[11]): shows or hides the boss bar with an NpcName text id.
    BossBar {
        shown: bool,
        entity: i32,
        slot: i16,
        name: i32,
    },
    /// `SetCharacterImmortality` (2004[12]): HP stops at 1.
    Immortality { entity: i32, on: bool },
    /// `SetCharacterInvincibility` (2004[15]): no damage at all.
    Invincibility { entity: i32, on: bool },
    /// `SetCharacterHPBarDisplay` (2004[30]): the small bar over the character's head.
    HpBarDisplay { entity: i32, on: bool },
    /// `HandleBossDefeat` (2003[12]): the boss is beaten (banner, music end, rewards).
    BossDefeat { entity: i32 },
}

/// What the events may ask about the world. Unknown entities answer `None`, which makes the
/// condition false.
pub trait EventHost {
    fn has_sp_effect(&self, entity: i32, id: i32) -> Option<bool>;
    /// Whether the character's animation raises event message `id` (TAE 231
    /// `SetEzStateRequestID`), the condition `IfCharacterHasEventMessage` (4[8]) waits for.
    fn has_event_message(&self, entity: i32, id: i32) -> Option<bool>;
    fn is_dead(&self, entity: i32) -> Option<bool>;
    fn hp(&self, entity: i32) -> Option<i32>;
}

/// What happened, for logs and tests.
#[derive(Debug, Clone, PartialEq)]
pub enum EventLog {
    Ended(i64),
    Restarted(i64),
    Flag {
        flag: i32,
        on: bool,
    },
    Command {
        event: i64,
        command: EventCommand,
    },
    /// An instruction the interpreter does not run (listed once per instruction).
    Skipped {
        event: i64,
        bank: i32,
        id: i32,
    },
}

/// One event to run and where it starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventEntry {
    pub id: i64,
    /// Start right after `Label<n>` instead of at the top (to join a fight that is already
    /// under way, after the parts of its event that play the intro).
    pub from_label: Option<u8>,
}

#[derive(Debug, Clone)]
struct Running {
    id: i64,
    code: Vec<Instruction>,
    entry: usize,
    pc: usize,
    /// Seconds or frames left of the current wait.
    wait: Option<f32>,
    groups: HashMap<i8, Vec<usize>>,
    ended: bool,
}

impl Running {
    fn reset(&mut self) {
        self.pc = self.entry;
        self.wait = None;
        self.groups.clear();
        self.ended = false;
    }
}

/// The events of one fight, their flags and log.
#[derive(Debug, Clone)]
pub struct EventScripts {
    running: Vec<Running>,
    /// Event flags that are on. They persist over a death and reload, as the game's do.
    pub flags: HashSet<i32>,
    pub log: Vec<EventLog>,
    skipped: HashSet<(i64, i32, i32)>,
}

/// Instructions run per event and step before the event is made to wait (a loop guard).
const STEP_BUDGET: usize = 512;

fn label_code(ins: &Instruction) -> Option<i32> {
    (ins.bank == 1014).then_some(ins.id)
}

fn args(ins: &Instruction) -> Vec<Arg> {
    emedf::decode(ins)
        .map(|(_, a)| a.into_iter().map(|(_, v)| v).collect())
        .unwrap_or_default()
}

fn compare(cmp: i64, a: i64, b: i64) -> bool {
    match cmp {
        0 => a == b,
        1 => a != b,
        2 => a > b,
        3 => a < b,
        4 => a >= b,
        5 => a <= b,
        _ => false,
    }
}

impl EventScripts {
    /// Prepares `entries` from `file`, each with its parameters filled in from the first
    /// `InitializeEvent` that starts it.
    pub fn new(file: &Emevd, entries: &[EventEntry]) -> Result<Self, String> {
        let mut running = Vec::new();
        for e in entries {
            let ev = file
                .event(e.id)
                .ok_or_else(|| format!("event {} is not in the script", e.id))?;
            let start = file.starts_of(e.id).into_iter().next();
            let code = ev.resolve(start.as_ref().map_or(&[][..], |s| &s.args));
            let entry = match e.from_label {
                None => 0,
                Some(n) => {
                    code.iter()
                        .position(|i| label_code(i) == Some(n as i32))
                        .ok_or_else(|| format!("event {} has no Label{n}", e.id))?
                        + 1
                }
            };
            running.push(Running {
                id: e.id,
                code,
                entry,
                pc: entry,
                wait: None,
                groups: HashMap::new(),
                ended: false,
            });
        }
        Ok(Self {
            running,
            flags: HashSet::new(),
            log: Vec::new(),
            skipped: HashSet::new(),
        })
    }

    /// [`EventScripts::new`] from `cache/raw/event/<map>.emevd`.
    pub fn load(cache: &Path, map: &str, entries: &[EventEntry]) -> Result<Self, String> {
        let path = cache.join(format!("raw/event/{map}.emevd"));
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let file = Emevd::parse(&bytes).map_err(|e| e.to_string())?;
        Self::new(&file, entries)
    }

    /// Starts every event over (after the fight reloads), keeping the flags.
    pub fn restart(&mut self) {
        for r in &mut self.running {
            r.reset();
        }
    }

    pub fn flag(&self, id: i32) -> bool {
        self.flags.contains(&id)
    }

    pub fn set_flag(&mut self, id: i32, on: bool) {
        let changed = if on {
            self.flags.insert(id)
        } else {
            self.flags.remove(&id)
        };
        if changed {
            self.log.push(EventLog::Flag { flag: id, on });
        }
    }

    /// Whether event `id` has ended.
    pub fn ended(&self, id: i64) -> bool {
        self.running.iter().any(|r| r.id == id && r.ended)
    }

    /// Runs every event for one step of `dt` seconds; returns the commands they issued.
    pub fn step(&mut self, dt: f32, host: &dyn EventHost) -> Vec<EventCommand> {
        let mut out = Vec::new();
        for k in 0..self.running.len() {
            let mut r = std::mem::replace(
                &mut self.running[k],
                Running {
                    id: 0,
                    code: Vec::new(),
                    entry: 0,
                    pc: 0,
                    wait: None,
                    groups: HashMap::new(),
                    ended: true,
                },
            );
            self.run(&mut r, dt, host, &mut out);
            self.running[k] = r;
        }
        out
    }

    fn eval_group(&self, r: &Running, group: i8, host: &dyn EventHost) -> bool {
        let Some(list) = r.groups.get(&group) else {
            return false;
        };
        if list.is_empty() {
            return false;
        }
        let mut each = list.iter().map(|&i| self.eval(r, &r.code[i], host));
        if group > 0 {
            each.all(|v| v)
        } else {
            each.any(|v| v)
        }
    }

    /// Evaluates condition instruction `ins` (its group argument aside).
    fn eval(&self, r: &Running, ins: &Instruction, host: &dyn EventHost) -> bool {
        let a = args(ins);
        let i = |k: usize| a.get(k).map_or(0, |v| v.as_i64());
        match ins.code() {
            (0, 0) => self.eval_group(r, i(2) as i8, host) == (i(1) != 0),
            (3, 0) => self.flag(i(3) as i32) == (i(1) != 0),
            // DarkScript3's death state: 0 alive, 1 dead.
            (4, 0) => host.is_dead(i(1) as i32) == Some(i(2) == 1),
            (4, 5) => host.has_sp_effect(i(1) as i32, i(2) as i32) == Some(i(3) != 0),
            (4, 8) => host.has_event_message(i(1) as i32, i(2) as i32) == Some(i(3) != 0),
            (4, 14) => host
                .hp(i(1) as i32)
                .is_some_and(|hp| compare(i(2), hp as i64, i(3))),
            _ => false,
        }
    }

    fn is_condition(ins: &Instruction) -> bool {
        matches!(
            ins.code(),
            (0, 0) | (3, 0) | (4, 0) | (4, 5) | (4, 8) | (4, 14)
        )
    }

    fn goto(r: &mut Running, label: i64) {
        let n = label as i32;
        let after = r.code[r.pc + 1..]
            .iter()
            .position(|i| label_code(i) == Some(n))
            .map(|p| p + r.pc + 1);
        let at = after.or_else(|| r.code.iter().position(|i| label_code(i) == Some(n)));
        match at {
            Some(at) => r.pc = at + 1,
            None => r.pc = r.code.len(),
        }
    }

    fn end(&mut self, r: &mut Running, end_type: i64) {
        if end_type == 1 {
            r.reset();
            self.log.push(EventLog::Restarted(r.id));
        } else {
            r.ended = true;
            self.log.push(EventLog::Ended(r.id));
        }
    }

    fn run(&mut self, r: &mut Running, dt: f32, host: &dyn EventHost, out: &mut Vec<EventCommand>) {
        let mut dt_left = dt;
        for _ in 0..STEP_BUDGET {
            if r.ended {
                return;
            }
            let Some(ins) = r.code.get(r.pc).cloned() else {
                r.ended = true;
                self.log.push(EventLog::Ended(r.id));
                return;
            };
            let a = args(&ins);
            let i = |k: usize| a.get(k).map_or(0, |v| v.as_i64());
            let f = |k: usize| a.get(k).map_or(0.0, |v| v.as_f32());
            if Self::is_condition(&ins) {
                let group = i(0) as i8;
                if group == 0 {
                    if !self.eval(r, &ins, host) {
                        return;
                    }
                    r.groups.clear();
                } else {
                    r.groups.entry(group).or_default().push(r.pc);
                }
                r.pc += 1;
                continue;
            }
            let mut next = r.pc + 1;
            let mut command = None;
            match ins.code() {
                // EndUnconditionally(end type).
                (1000, 4) => {
                    self.end(r, i(0));
                    if !r.ended {
                        return; // a restart waits for the next step
                    }
                    continue;
                }
                // GotoIfConditionGroupStateUncompiled(label, state, group).
                (1000, 101) => {
                    if self.eval_group(r, i(2) as i8, host) == (i(1) != 0) {
                        Self::goto(r, i(0));
                        continue;
                    }
                }
                // GotoUnconditionally(label).
                (1000, 103) => {
                    Self::goto(r, i(0));
                    continue;
                }
                // WaitFixedTimeSeconds / WaitFixedTimeFrames (one frame per step).
                (1001, 0) | (1001, 1) => {
                    let left = r
                        .wait
                        .get_or_insert(if ins.id == 0 { f(0) } else { i(0) as f32 });
                    if *left > 0.0 {
                        *left -= if ins.id == 0 { dt_left } else { 1.0 };
                        dt_left = 0.0;
                        if *left > 0.0 {
                            return;
                        }
                    }
                    r.wait = None;
                }
                // WaitForEventFlag(state, type, flag).
                (1003, 0) => {
                    if self.flag(i(2) as i32) != (i(0) != 0) {
                        return;
                    }
                }
                // SkipIfEventFlag(lines, state, type, flag).
                (1003, 1) => {
                    if self.flag(i(3) as i32) == (i(1) != 0) {
                        next += i(0) as usize;
                    }
                }
                // EndIfEventFlag(end type, state, type, flag).
                (1003, 2) => {
                    if self.flag(i(3) as i32) == (i(1) != 0) {
                        self.end(r, i(0));
                        if !r.ended {
                            return;
                        }
                        continue;
                    }
                }
                // GotoIfEventFlag(label, state, type, flag).
                (1003, 101) => {
                    if self.flag(i(3) as i32) == (i(1) != 0) {
                        Self::goto(r, i(0));
                        continue;
                    }
                }
                (1014, _) => {}
                // SetEventFlag / SetNetworkconnectedEventFlag(flag, state: 0 off, 1 on, 2 toggle).
                (2003, 2) | (2003, 62) => {
                    let flag = i(0) as i32;
                    let on = match i(1) {
                        2 => !self.flag(flag),
                        s => s != 0,
                    };
                    self.set_flag(flag, on);
                }
                // BatchSetNetworkconnectedEventFlags(first, last, state).
                (2003, 63) => {
                    let (a0, b0) = (i(0).min(i(1)) as i32, i(0).max(i(1)) as i32);
                    for flag in a0..=b0 {
                        self.set_flag(flag, i(2) != 0);
                    }
                }
                (2003, 11) => {
                    command = Some(EventCommand::BossBar {
                        shown: i(0) != 0,
                        entity: i(1) as i32,
                        slot: i(2) as i16,
                        name: i(3) as i32,
                    })
                }
                (2003, 12) => {
                    command = Some(EventCommand::BossDefeat {
                        entity: i(0) as i32,
                    })
                }
                (2004, 6) => {
                    command = Some(EventCommand::EzStateRequest {
                        entity: i(0) as i32,
                        command: i(1) as i32,
                        slot: i(2) as u8,
                    })
                }
                (2004, 8) => {
                    command = Some(EventCommand::SetSpEffect {
                        entity: i(0) as i32,
                        id: i(1) as i32,
                    })
                }
                (2004, 21) => {
                    command = Some(EventCommand::ClearSpEffect {
                        entity: i(0) as i32,
                        id: i(1) as i32,
                    })
                }
                (2004, 12) => {
                    command = Some(EventCommand::Immortality {
                        entity: i(0) as i32,
                        on: i(1) != 0,
                    })
                }
                (2004, 15) => {
                    command = Some(EventCommand::Invincibility {
                        entity: i(0) as i32,
                        on: i(1) != 0,
                    })
                }
                (2004, 30) => {
                    command = Some(EventCommand::HpBarDisplay {
                        entity: i(0) as i32,
                        on: i(1) != 0,
                    })
                }
                (bank, id) => {
                    if self.skipped.insert((r.id, bank, id)) {
                        self.log.push(EventLog::Skipped {
                            event: r.id,
                            bank,
                            id,
                        });
                    }
                }
            }
            if let Some(c) = command {
                self.log.push(EventLog::Command {
                    event: r.id,
                    command: c.clone(),
                });
                out.push(c);
            }
            r.pc = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sekiro_formats::emevd::{Event, RestBehavior};

    fn ins(bank: i32, id: i32, words: &[i32]) -> Instruction {
        Instruction {
            bank,
            id,
            args: words.iter().flat_map(|w| w.to_le_bytes()).collect(),
            layer: None,
        }
    }

    /// A condition with a group byte, then padding, then words (the 4[x] layout).
    fn cond(bank: i32, id: i32, group: i8, rest: &[u8]) -> Instruction {
        let mut args = vec![group as u8, 0, 0, 0];
        args.extend_from_slice(rest);
        Instruction {
            bank,
            id,
            args,
            layer: None,
        }
    }

    fn bytes(words: &[i32]) -> Vec<u8> {
        words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    /// IfCharacterHasEventMessage(group, entity, message, has=1, cmp=0, count=1.0).
    fn has_message(group: i8, entity: i32, message: i32) -> Instruction {
        let mut rest = bytes(&[entity, message]);
        rest.extend_from_slice(&[1, 0, 0, 0]);
        rest.extend_from_slice(&1.0f32.to_le_bytes());
        cond(4, 8, group, &rest)
    }

    /// IfCharacterHasSpeffect(group, entity, id, has, cmp=0, count=1.0).
    fn has_effect(group: i8, entity: i32, id: i32, has: bool) -> Instruction {
        let mut rest = bytes(&[entity, id]);
        rest.extend_from_slice(&[has as u8, 0, 0, 0]);
        rest.extend_from_slice(&1.0f32.to_le_bytes());
        cond(4, 5, group, &rest)
    }

    #[derive(Default)]
    struct World {
        messages: HashSet<i32>,
        effects: HashSet<i32>,
    }

    impl EventHost for World {
        fn has_sp_effect(&self, entity: i32, id: i32) -> Option<bool> {
            (entity == 7).then(|| self.effects.contains(&id))
        }
        fn has_event_message(&self, entity: i32, id: i32) -> Option<bool> {
            (entity == 7).then(|| self.messages.contains(&id))
        }
        fn is_dead(&self, _: i32) -> Option<bool> {
            Some(false)
        }
        fn hp(&self, _: i32) -> Option<i32> {
            Some(1)
        }
    }

    fn file(code: Vec<Instruction>) -> Emevd {
        Emevd {
            events: vec![Event {
                id: 1,
                rest: RestBehavior::Default,
                instructions: code,
                parameters: Vec::new(),
            }],
            ..Emevd::default()
        }
    }

    #[test]
    fn waits_on_main_conditions_and_or_groups() {
        let code = vec![
            has_message(0, 7, 20),
            ins(2004, 8, &[7, 200051]),
            // WaitFixedTimeSeconds(1.0)
            ins(1001, 0, &[f32::to_bits(1.0) as i32]),
            has_message(-1, 7, 60),
            has_effect(-1, 7, 3506080, false),
            // IfConditionGroup(MAIN, on, OR group 1)
            Instruction {
                bank: 0,
                id: 0,
                args: vec![0, 1, 0xFF, 0],
                layer: None,
            },
            ins(2003, 2, &[11110860, 1]),
        ];
        let mut ev = EventScripts::new(
            &file(code),
            &[EventEntry {
                id: 1,
                from_label: None,
            }],
        )
        .unwrap();
        let mut w = World::default();
        assert!(ev.step(0.1, &w).is_empty());
        w.messages.insert(20);
        w.effects.insert(3506080);
        let c = ev.step(0.1, &w);
        assert_eq!(
            c,
            vec![EventCommand::SetSpEffect {
                entity: 7,
                id: 200051
            }]
        );
        w.messages.clear();
        for _ in 0..12 {
            ev.step(0.1, &w);
        }
        assert!(
            !ev.flag(11110860),
            "the OR group held while the effect was on"
        );
        w.effects.clear();
        ev.step(0.1, &w);
        assert!(ev.flag(11110860));
        assert!(ev.ended(1));
    }

    #[test]
    fn flags_labels_and_restarts() {
        let code = vec![
            // GotoIfEventFlag(label 0, off, 0, 5)
            ins(1003, 101, &[0x0000_0000, 5]),
            ins(2003, 2, &[6, 1]),
            ins(1014, 0, &[]),
            ins(2003, 2, &[5, 1]),
            // EndUnconditionally(restart)
            ins(1000, 4, &[1]),
        ];
        let mut ev = EventScripts::new(
            &file(code),
            &[EventEntry {
                id: 1,
                from_label: Some(0),
            }],
        )
        .unwrap();
        let w = World::default();
        ev.step(0.1, &w);
        assert!(ev.flag(5));
        ev.step(0.1, &w);
        assert!(!ev.flag(6), "started after the label, so 6 is never set");
        assert!(!ev.ended(1), "restarted, not ended");
    }
}

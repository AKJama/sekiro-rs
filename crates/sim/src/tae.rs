//! TAE (TimeAct) events at run time: what the playing animations say right now.
//!
//! Each animation has a timeline of events. The ones the simulation uses:
//!
//! - `ChrActionFlag` (type 0): cancel windows and state flags, by flag type. The flag types that
//!   gate actions are listed in [`cancel`].
//! - `AddSpEffect` (types 66, 67, 401): applies a SpEffect while the event is active. The effect's
//!   `SpEffectParam.behaviorRefId` is what the scripts query with `env(3036, ref)`.
//! - `SetTurnSpeed` (type 224): turn speed in degrees per second while active.
//! - `ChrPhysicsVelocityChange` (type 920): sets the body velocity from
//!   `ChrPhysicsVelocityChangeParam` when the event starts (jumps).
//! - `AttackBehavior` (1), `BulletBehavior` (2), `PCBehavior` (307) and `IFrames` (954): kept for
//!   combat.
//!
//! Event names and parameter layouts come from DSAnimStudio's Sekiro template
//! (`cache/refs/TAE.Template.SDT.xml`); action-flag meanings are DS3-era community labels checked
//! against which animations carry them, not against the exe.

use sekiro_formats::param::{RawParam, Table};
use sekiro_formats::paramdef::ParamDef;
use sekiro_formats::tae::{self, AnimHeader, Template};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

/// ChrActionFlag types that open action windows, by the action they allow.
pub mod cancel {
    /// Interrupt into movement (stick) allowed.
    pub const MOVE: i32 = 11;
    /// Step / dodge allowed.
    pub const DODGE: i32 = 26;
    /// Turning disabled while active.
    pub const DISABLE_TURN: i32 = 7;
    /// Main weapon attack (R1) allowed: combo windows.
    pub const ATTACK: i32 = 115;
    /// Guard (L1) allowed.
    pub const GUARD: i32 = 117;
    /// Combat art (L2 in the DS3 naming) allowed.
    pub const COMBAT_ART: i32 = 118;
    /// Jump allowed.
    pub const JUMP: i32 = 119;
    /// Prosthetic (R2 in the DS3 naming) allowed.
    pub const PROSTHETIC: i32 = 137;
    /// Item use allowed.
    pub const ITEM: i32 = 31;
    /// Weapon or prosthetic switch allowed.
    pub const SWITCH: i32 = 32;
}

/// One decoded event of interest.
#[derive(Debug, Clone, PartialEq)]
pub enum TaeKind {
    ActionFlag(i32),
    SpEffect(i32),
    TurnSpeed(f32),
    VelocityChange(i32),
    Attack {
        judge: i32,
        attack_type: i32,
    },
    Bullet {
        judge: i32,
        dummy_poly: i32,
    },
    PcBehavior {
        judge: i32,
    },
    IFrames(i32),
    /// `Blend` (16): crossfade into this animation over the event's length.
    Blend,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimedEvent {
    pub start: f32,
    pub end: f32,
    pub kind: TaeKind,
}

impl TimedEvent {
    pub fn active_at(&self, t: f32) -> bool {
        t >= self.start && t < self.end
    }
}

/// One animation's TAE entry.
#[derive(Debug, Clone, Default)]
pub struct TaeAnim {
    pub hkx_source: i64,
    pub events: Arc<Vec<TimedEvent>>,
}

/// Raw animations from every `*.tae` in an extracted `anibnd`, by full id.
pub fn read_tae_dir(dir: &Path) -> HashMap<i64, ParsedAnim> {
    let mut out = HashMap::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("tae") {
            continue;
        }
        let Ok(t) = std::fs::read(&path)
            .map_err(|_| ())
            .and_then(|d| tae::parse(&d).map_err(|_| ()))
        else {
            continue;
        };
        let category =
            tae::category_from_file_name(&path.file_name().unwrap_or_default().to_string_lossy());
        for a in t.animations {
            let id = tae::full_id(category, a.id);
            let events_from = match &a.header {
                Some(AnimHeader::ImportOther { from, .. }) => Some(tae::full_id(category, *from)),
                _ => None,
            };
            out.insert(
                id,
                ParsedAnim {
                    hkx_source: tae::full_id(category, a.hkx_source()),
                    events_from,
                    events: a.events,
                },
            );
        }
    }
    out
}

/// An animation as read from a TAE file, before decoding.
pub struct ParsedAnim {
    pub hkx_source: i64,
    /// Set when the animation takes its events from another one.
    pub events_from: Option<i64>,
    pub events: Vec<tae::TaeEvent>,
}

/// What a SpEffect means to the simulation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpEffectInfo {
    /// `behaviorRefId`, answered by `env(3036)`; 0 for none.
    pub behavior_ref: i32,
    /// Seconds the effect lasts after it is applied; 0 means only while the TAE event is active.
    pub duration: f32,
    pub state_info: i32,
}

/// A velocity change for jumps (`ChrPhysicsVelocityChangeParam`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VelocityChange {
    pub horizontal_scale: f32,
    pub vertical_scale: f32,
    pub horizontal: f32,
    pub vertical: f32,
    pub horizontal_angle: f32,
}

fn load_table(param_dir: &Path, defs_dir: &Path, file: &str, def: &str) -> Option<Table> {
    let raw = RawParam::parse(std::fs::read(param_dir.join(format!("{file}.param"))).ok()?).ok()?;
    let def = ParamDef::load(&defs_dir.join(format!("{def}.xml"))).ok()?;
    Table::new(file, &raw, Arc::new(def), None).ok()
}

/// All TAE timelines of one character plus the params the events refer to.
#[derive(Default)]
pub struct TaeDb {
    anims: HashMap<i64, TaeAnim>,
    pub sp_effects: HashMap<i32, SpEffectInfo>,
    pub velocity_changes: HashMap<i32, VelocityChange>,
}

impl TaeDb {
    /// Loads timelines from an extracted `anibnd` directory, decoding them with the template, and
    /// the SpEffect and velocity-change params from the extracted `gameparam` directory.
    pub fn load(anibnd_dir: &Path, template: &Template, param_dir: &Path, defs_dir: &Path) -> Self {
        let parsed = read_tae_dir(anibnd_dir);
        let mut anims = HashMap::with_capacity(parsed.len());
        for (&id, a) in &parsed {
            let events = match a.events_from.and_then(|f| parsed.get(&f)) {
                Some(src) => &src.events,
                None => &a.events,
            };
            let decoded: Vec<TimedEvent> =
                events.iter().filter_map(|e| decode(template, e)).collect();
            anims.insert(
                id,
                TaeAnim {
                    hkx_source: a.hkx_source,
                    events: Arc::new(decoded),
                },
            );
        }
        let mut db = TaeDb {
            anims,
            ..TaeDb::default()
        };
        if let Some(t) = load_table(param_dir, defs_dir, "SpEffectParam", "SpEffect") {
            for row in t.rows() {
                db.sp_effects.insert(
                    row.id(),
                    SpEffectInfo {
                        behavior_ref: row.i32("behaviorRefId").unwrap_or(0),
                        duration: row.f32("effectEndurance").unwrap_or(0.0),
                        state_info: row.int("stateInfo").unwrap_or(0) as i32,
                    },
                );
            }
        }
        if let Some(t) = load_table(
            param_dir,
            defs_dir,
            "ChrPhysicsVelocityChangeParam",
            "ChrPhysicsVelocityChangeParam",
        ) {
            for row in t.rows() {
                db.velocity_changes.insert(
                    row.id(),
                    VelocityChange {
                        horizontal_scale: row.f32("horizontalVelocityScale").unwrap_or(1.0),
                        vertical_scale: row.f32("verticalVelocityScale").unwrap_or(1.0),
                        horizontal: row.f32("horizontalVelocityChange").unwrap_or(0.0),
                        vertical: row.f32("verticalVelocityChange").unwrap_or(0.0),
                        horizontal_angle: row.f32("horizontalVelocityAngle").unwrap_or(0.0),
                    },
                );
            }
        }
        db
    }

    /// Loads from the standard cache layout.
    pub fn load_from_cache(cache: &Path, chr: &str) -> Self {
        let template = std::fs::read_to_string(cache.join("refs/TAE.Template.SDT.xml"))
            .ok()
            .and_then(|s| Template::parse_xml(&s).ok());
        match template {
            Some(t) => Self::load(
                &cache.join(format!("raw/chr/{chr}.anibnd.d")),
                &t,
                &cache.join("raw/param/gameparam/gameparam.parambnd.d"),
                &cache.join("refs/paramdex/Defs"),
            ),
            None => Self::default(),
        }
    }

    pub fn len(&self) -> usize {
        self.anims.len()
    }

    pub fn is_empty(&self) -> bool {
        self.anims.is_empty()
    }

    pub fn anim(&self, name: &str) -> Option<&TaeAnim> {
        self.anims.get(&tae::anim_id(name)?)
    }

    /// Full ids of every animation with a TAE entry (what env 1114 "does the animation exist"
    /// is answered from).
    pub fn anim_ids(&self) -> impl Iterator<Item = i64> + '_ {
        self.anims.keys().copied()
    }

    /// Crossfade length into `name`: the end of its `Blend` event that starts at 0, if any.
    pub fn blend_in(&self, name: &str) -> Option<f32> {
        self.anim(name)?
            .events
            .iter()
            .find(|e| e.kind == TaeKind::Blend && e.start <= 0.001)
            .map(|e| e.end)
    }

    /// Full id to HKX source id, for every animation that borrows another's HKX.
    pub fn hkx_aliases(&self) -> impl Iterator<Item = (i64, i64)> + '_ {
        self.anims
            .iter()
            .filter(|(id, a)| **id != a.hkx_source)
            .map(|(id, a)| (*id, a.hkx_source))
    }
}

fn decode(template: &Template, e: &tae::TaeEvent) -> Option<TimedEvent> {
    let d = template.decode(e)?;
    let int = |name: &str| d.field(name).and_then(|v| v.as_int()).map(|v| v as i32);
    let kind = match e.event_type {
        0 => TaeKind::ActionFlag(int("FlagType")?),
        66 | 67 | 401 => TaeKind::SpEffect(int("SpEffectID")?),
        224 => TaeKind::TurnSpeed(d.field("TurnSpeed")?.as_f32()?),
        920 => TaeKind::VelocityChange(int("ChrPhysicsVelocityParam ID")?),
        1 => TaeKind::Attack {
            judge: int("BehaviorJudgeID")?,
            attack_type: int("AttackType").unwrap_or(0),
        },
        2 => TaeKind::Bullet {
            judge: int("BehaviorJudgeID")?,
            dummy_poly: int("DummyPolyID").unwrap_or(-1),
        },
        307 => TaeKind::PcBehavior {
            judge: int("BehaviorJudgeID")?,
        },
        954 => TaeKind::IFrames(int("IFrameType").unwrap_or(0)),
        16 => TaeKind::Blend,
        _ => return None,
    };
    Some(TimedEvent {
        start: e.start,
        end: e.end,
        kind,
    })
}

/// A clip being played, as the TAE runtime sees it.
#[derive(Debug, Clone)]
pub struct PlayingClip {
    pub animation: String,
    /// Time before and after this tick's advance.
    pub prev_time: f32,
    pub time: f32,
}

/// The TAE state for this tick.
#[derive(Debug, Clone, Default)]
pub struct TaeFrame {
    pub flags: HashSet<i32>,
    /// SpEffects applied by active events plus timed effects still running.
    pub sp_effects: Vec<i32>,
    pub behavior_refs: HashSet<i32>,
    /// Smallest active `SetTurnSpeed`, degrees per second.
    pub turn_speed: Option<f32>,
    /// Velocity changes whose event started this tick.
    pub velocity_changes: Vec<i32>,
    pub attacks: Vec<TimedEvent>,
    pub iframes: Vec<i32>,
    /// True if any playing clip has action flags at all. Clips without any (idle, locomotion
    /// loops) put no limits on actions.
    pub restricted: bool,
}

impl TaeFrame {
    pub fn flag(&self, f: i32) -> bool {
        self.flags.contains(&f)
    }
}

/// Tracks timed SpEffects across ticks and builds each [`TaeFrame`].
#[derive(Default)]
pub struct TaeRuntime {
    /// Effects with a duration, and their remaining seconds.
    timed: HashMap<i32, f32>,
    pub frame: TaeFrame,
}

impl TaeRuntime {
    /// Computes the frame for `clips` (the main clip first) after they advanced by `dt`.
    pub fn update(&mut self, db: &TaeDb, clips: &[PlayingClip], dt: f32) {
        for remaining in self.timed.values_mut() {
            *remaining -= dt;
        }
        self.timed.retain(|_, r| *r > 0.0);
        let mut f = TaeFrame::default();
        for clip in clips {
            let Some(anim) = db.anim(&clip.animation) else {
                continue;
            };
            let t = clip.time;
            for e in anim.events.iter() {
                if let TaeKind::ActionFlag(_) = e.kind {
                    f.restricted = true;
                }
                // Started this tick: the event's start lies in [prev_time, time). A clip that just
                // began reports prev_time 0, so events at 0 fire on its first tick.
                let started = e.start >= clip.prev_time && e.start < t.max(clip.prev_time + 1e-6);
                if !e.active_at(t) && !started {
                    continue;
                }
                match &e.kind {
                    TaeKind::ActionFlag(flag) => {
                        f.flags.insert(*flag);
                    }
                    TaeKind::SpEffect(id) => {
                        f.sp_effects.push(*id);
                        if let Some(info) = db.sp_effects.get(id)
                            && info.duration > 0.0
                            && started
                        {
                            self.timed.insert(*id, info.duration);
                        }
                    }
                    TaeKind::TurnSpeed(s) => {
                        f.turn_speed = Some(f.turn_speed.map_or(*s, |c: f32| c.min(*s)));
                    }
                    TaeKind::VelocityChange(id) => {
                        if started {
                            f.velocity_changes.push(*id);
                        }
                    }
                    TaeKind::Attack { .. }
                    | TaeKind::Bullet { .. }
                    | TaeKind::PcBehavior { .. } => {
                        f.attacks.push(e.clone());
                    }
                    TaeKind::IFrames(kind) => f.iframes.push(*kind),
                    TaeKind::Blend => {}
                }
            }
        }
        f.sp_effects.extend(self.timed.keys().copied());
        f.sp_effects.sort_unstable();
        f.sp_effects.dedup();
        for id in &f.sp_effects {
            if let Some(info) = db.sp_effects.get(id)
                && info.behavior_ref > 0
            {
                f.behavior_refs.insert(info.behavior_ref);
            }
        }
        self.frame = f;
    }
}

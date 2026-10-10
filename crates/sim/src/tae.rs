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
    /// Full invulnerability while active, like TAE 951: the exe's ChrActionFlag handler
    /// (`FUN_140b51b70`, case 0x5e) sets bit 0 of the action module's flag word +0x70, the bit
    /// FwLo's RustSekiro traced for 951 (`docs/kb/damage.md`). In the data only the Great
    /// Shinobi Owl's warp clips carry it, over each window in which he is faded out
    /// (`docs/OWL.md`, "Warp and vanish"). The DSAnimStudio label is `SetStartbit0`.
    pub const WARP_INVULNERABLE: i32 = 94;
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
    /// `IFrames_MistRaven` (950) and `IFrames_DuringAction` (951): full invulnerability. 950 sits
    /// on Wolf's knock-down, launch, death and revival animations, 951 on the deathblow throws.
    /// `IFrames_ThrowAtkStillHurts` (952) sits only on Wolf's being-thrown clips (`a2xx_600xxx`):
    /// no other attack lands while the throw holds him, and the throw's own damage still does
    /// (it is applied by the throw, not by hit detection; see [`crate::grab`]).
    Invulnerable,
    /// `BoostRootMotionToReachTarget` (760): scales root motion so an attack reaches its target.
    ReachTarget(ReachTarget),
    /// `SetKnockbackPercent` (226): scales the knockback this animation's character receives.
    KnockbackPercent(f32),
    /// `FixedRotationDirection` (703): the character stops turning toward its target.
    FixedRotation,
    /// `SetSPRegenRatePercent` (225): scales posture recovery.
    PostureRegenPercent(f32),
    /// `Blend` (16): crossfade into this animation over the event's length.
    Blend,
    /// `ChrActionFlag` 3 (`ShieldBlock`): guarding. ArgB is the behaviour judge of the guard
    /// action, whose AtkParam row is the guard row (the soldier's BehaviorParam for judge 900 is
    /// its "deflect" row, Wolf's judge 90 is "sword_guard").
    Guard {
        judge: i32,
    },
    /// `StaminaControlParam` (960): which `staminaRecoverRatio_forTypeNNN` posture recovery uses.
    StaminaControl(i32),
    /// `CommonBehavior` (5): a behaviour without a hitbox; in a deathblow start clip, the grab
    /// check.
    CommonBehavior {
        judge: i32,
    },
    /// `ThrowAttackBehavior` (304): where a throw's damage lands.
    ThrowAttack {
        judge: i32,
    },
    /// `PlaySound_*` (128 to 132): a sound of type `kind` (0 a, 1 c, ...; see
    /// [`sound_name`]) and number `id`.
    Sound {
        kind: i32,
        id: i32,
    },
    /// `SetOpacityKeyframe` (193): the model's opacity goes linearly from `from` at the event's
    /// start to `to` at its end (the Owl's warps fade him out and back in with three of these).
    Opacity {
        from: f32,
        to: f32,
    },
    /// `SetEzStateRequestID` (231): a message to the map's event scripts while the event is
    /// active. The scripts wait for it with `IfCharacterHasEventMessage` (EMEVD 4[8]); the Owl's
    /// first deathblow get-up raises 20, which starts his phase 2 (`docs/OWL.md`).
    EzStateRequest(i32),
}

/// The sound event name for a TAE sound type and id: the type letter followed by the id padded
/// to nine digits (type 1, id 101001001 gives `c101001001`). Letters from the DSAnimStudio
/// Sekiro template.
pub fn sound_name(kind: i32, id: i32) -> String {
    let letter = match kind {
        0 => 'a',
        1 => 'c',
        2 => 'f',
        3 => 'o',
        4 => 'p',
        5 => 's',
        6 => 'm',
        7 => 'v',
        8 => 'x',
        9 => 'b',
        10 => 'g',
        // Hit, guard and deflect sounds from HitEffectSeParam (exe sound type 12, traced by
        // FwLo's RustSekiro, `docs/kb/sound.md`).
        12 => 'z',
        _ => '?',
    };
    format!("{letter}{id:09}")
}

/// Fields of a `BoostRootMotionToReachTarget` (760) event, metres and degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReachTarget {
    pub reference_dist: f32,
    pub range_min: f32,
    pub range_max: f32,
    pub arrive_angle: f32,
    pub arrive_dist: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimedEvent {
    pub start: f32,
    pub end: f32,
    pub kind: TaeKind,
    /// The event's `StateInfo` (action flags, attacks, bullets, sounds, effects). When non-zero
    /// the engine applies the event only while an SpEffect with that `stateInfo` is active, so
    /// weapon-enchantment hits and upgraded-prosthetic sounds stay off by default.
    pub state_info: i32,
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
        0 if int("FlagType")? == 3 => TaeKind::Guard {
            judge: int("ArgB").unwrap_or(-1),
        },
        0 => TaeKind::ActionFlag(int("FlagType")?),
        960 => TaeKind::StaminaControl(int("StaminaRatioType")?),
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
        5 => TaeKind::CommonBehavior {
            judge: int("BehaviorJudgeID")?,
        },
        304 => TaeKind::ThrowAttack {
            judge: int("BehaviorJudgeID")?,
        },
        950..=952 => TaeKind::Invulnerable,
        760 if int("IsEnable").unwrap_or(1) != 0 => {
            let f = |name: &str| d.field(name).and_then(|v| v.as_f32()).unwrap_or(0.0);
            TaeKind::ReachTarget(ReachTarget {
                reference_dist: f("ReferenceDist"),
                range_min: f("EnableRangeMin"),
                range_max: f("EnableRangeMax"),
                arrive_angle: f("ArriveAngleFromTarget"),
                arrive_dist: f("ArriveDistFromTarget"),
            })
        }
        226 => TaeKind::KnockbackPercent(int("KnockbackPercent")? as f32),
        703 if int("IsEnable").unwrap_or(1) != 0 => TaeKind::FixedRotation,
        225 => TaeKind::PostureRegenPercent(int("RegenRatePercent")? as f32),
        16 => TaeKind::Blend,
        128..=132 => TaeKind::Sound {
            kind: int("SoundType")?,
            id: int("SoundID")?,
        },
        193 => {
            let f = |name: &str| d.field(name).and_then(|v| v.as_f32()).unwrap_or(1.0);
            TaeKind::Opacity {
                from: f("OpacityAtEventStart"),
                to: f("OpacityAtEventEnd"),
            }
        }
        231 => TaeKind::EzStateRequest(int("EzStateRequestID")?),
        _ => return None,
    };
    Some(TimedEvent {
        start: e.start,
        end: e.end,
        kind,
        state_info: int("StateInfo").unwrap_or(0),
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
    /// Guard judge of an active `ShieldBlock` flag (see [`TaeKind::Guard`]).
    pub guard_judge: Option<i32>,
    /// Posture recovery control type (TAE 960) of the first playing clip that sets one.
    pub stamina_type: Option<i32>,
    /// Velocity changes whose event started this tick.
    pub velocity_changes: Vec<i32>,
    pub attacks: Vec<TimedEvent>,
    pub iframes: Vec<i32>,
    /// True if any playing clip has action flags at all. Clips without any (idle, locomotion
    /// loops) put no limits on actions.
    pub restricted: bool,
    /// Sound events that started this tick: (sound type, sound id).
    pub sounds: Vec<(i32, i32)>,
    /// TAE 950 / 951 / 952, or ChrActionFlag 94 ([`cancel::WARP_INVULNERABLE`]): no hit lands
    /// at all.
    pub invulnerable: bool,
    /// Model opacity from TAE 193 (the lowest of the active events); `None` is fully opaque.
    /// See [`TaeFrame::opacity`].
    pub opacity: Option<f32>,
    /// TAE 231 messages to the event scripts, active or started this tick.
    pub ezstate_requests: Vec<i32>,
    /// TAE 760 of the first playing clip that has one.
    pub reach_target: Option<ReachTarget>,
    /// TAE 226 as a fraction (0.3 for 30 %); `None` is full knockback.
    pub knockback_scale: Option<f32>,
    /// TAE 703: stop tracking the target.
    pub fixed_rotation: bool,
    /// TAE 225 as a fraction; `None` is normal posture recovery.
    pub posture_regen_scale: Option<f32>,
}

impl TaeFrame {
    pub fn flag(&self, f: i32) -> bool {
        self.flags.contains(&f)
    }

    /// Model opacity, 0 (not drawn) to 1 (fully drawn).
    pub fn opacity(&self) -> f32 {
        self.opacity.unwrap_or(1.0)
    }
}

/// Tracks timed SpEffects across ticks and builds each [`TaeFrame`].
#[derive(Default)]
pub struct TaeRuntime {
    /// Effects with a duration, and their remaining seconds.
    timed: HashMap<i32, f32>,
    /// SpEffects the character always has (equipment, skills); they open `StateInfo` gates.
    pub resident: Vec<i32>,
    pub frame: TaeFrame,
}

impl TaeRuntime {
    /// Starts (or refreshes) a timed SpEffect from outside the animation, such as one a bullet
    /// left on the character ([`crate::status`]). It shows in the frame from the next update on.
    pub fn add_timed(&mut self, id: i32, seconds: f32) {
        let r = self.timed.entry(id).or_insert(0.0);
        *r = r.max(seconds);
    }

    /// Computes the frame for `clips` (the main clip first) after they advanced by `dt`.
    pub fn update(&mut self, db: &TaeDb, clips: &[PlayingClip], dt: f32) {
        for remaining in self.timed.values_mut() {
            *remaining -= dt;
        }
        self.timed.retain(|_, r| *r > 0.0);
        let states = self.active_states(db, clips);
        let mut f = TaeFrame::default();
        for clip in clips {
            let Some(anim) = db.anim(&clip.animation) else {
                continue;
            };
            let t = clip.time;
            for e in anim.events.iter() {
                if e.state_info != 0 && !states.contains(&e.state_info) {
                    continue;
                }
                if let TaeKind::ActionFlag(_) | TaeKind::Guard { .. } = e.kind {
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
                        if *flag == cancel::WARP_INVULNERABLE && e.active_at(t) {
                            f.invulnerable = true;
                        }
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
                    TaeKind::Invulnerable => f.invulnerable = true,
                    TaeKind::ReachTarget(r) => {
                        f.reach_target.get_or_insert(*r);
                    }
                    TaeKind::KnockbackPercent(p) => {
                        f.knockback_scale.get_or_insert(p / 100.0);
                    }
                    TaeKind::FixedRotation => f.fixed_rotation = true,
                    TaeKind::PostureRegenPercent(p) => {
                        f.posture_regen_scale.get_or_insert(p / 100.0);
                    }
                    // Read by the deathblow timeline (`deathblow::ThrowTimes`), not per frame.
                    TaeKind::Blend
                    | TaeKind::CommonBehavior { .. }
                    | TaeKind::ThrowAttack { .. } => {}
                    TaeKind::Guard { judge } => {
                        f.flags.insert(3);
                        f.guard_judge.get_or_insert(*judge);
                    }
                    TaeKind::Sound { kind, id } => {
                        if started {
                            f.sounds.push((*kind, *id));
                        }
                    }
                    TaeKind::StaminaControl(kind) => {
                        if e.active_at(t) {
                            f.stamina_type.get_or_insert(*kind);
                        }
                    }
                    TaeKind::Opacity { from, to } => {
                        if e.active_at(t) {
                            let span = (e.end - e.start).max(1e-6);
                            let k = ((t - e.start) / span).clamp(0.0, 1.0);
                            let o = from + (to - from) * k;
                            f.opacity = Some(f.opacity.map_or(o, |c: f32| c.min(o)));
                        }
                    }
                    TaeKind::EzStateRequest(id) => f.ezstate_requests.push(*id),
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

    /// `stateInfo` of every SpEffect active this tick: from the clips' SpEffect events, timed
    /// effects still running and resident effects.
    fn active_states(&self, db: &TaeDb, clips: &[PlayingClip]) -> HashSet<i32> {
        let mut ids: Vec<i32> = self.timed.keys().chain(&self.resident).copied().collect();
        for clip in clips {
            let Some(anim) = db.anim(&clip.animation) else {
                continue;
            };
            for e in anim.events.iter() {
                if let TaeKind::SpEffect(id) = e.kind
                    && e.active_at(clip.time)
                {
                    ids.push(id);
                }
            }
        }
        ids.iter()
            .filter_map(|id| db.sp_effects.get(id))
            .map(|info| info.state_info)
            .filter(|&s| s != 0)
            .collect()
    }
}

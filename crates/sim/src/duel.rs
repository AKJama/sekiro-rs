//! Wolf against one enemy: both characters, the enemy's brain, lock-on, hits with the engine's
//! HP and posture rules, deathblows, and a respawn after a death.

use crate::ai::SimpleBrain;
use crate::character::{Character, StepReport};
use crate::combat::knockback::{HitStop, Knockback};
use crate::deathblow::{
    AbsorbStep, Absorbed, Deathblow, DummyFrame, Phase, Placement, Situation, ThrowTable,
    ThrowTimes, deathblow_marks, select,
};
use crate::enemy::EnemySpec;
use crate::fight::CombatRules;
use crate::hits::{Combatant, HitDetector, HitEvent, HitResult};
use crate::input::{InputFrame, angle_diff};
use crate::lua_ai::{AiActor, AiBrain, AiWorld, ThinkParams, anim_id_from_clip};
use crate::player::LoadError;
use sekiro_formats::flver::{self, Flver};
use std::path::{Path, PathBuf};

/// Seconds between a death and both characters respawning at their start positions (a stand-in
/// for the game's death screen and reload).
pub const RESPAWN_DELAY: f32 = 6.0;

/// Seed for the AI's random numbers (fixed, so runs repeat).
const AI_SEED: u64 = 7;

/// TAE action flag on the player's attacks that tells the enemy AI to parry
/// (`INTERUPT_ParryTiming`), traced by FwLo's RustSekiro (`docs/kb/guard-deflect.md`).
pub const FLAG_PARRY_TIMING: i32 = 63;

/// Animation offset types that pick a throw clip by its ThrowParam offset: the player graph's
/// throw CMSGs (`a000_5xxxxx`, candidates `a200_` to `a249_`) use type 17 for `atkAnimOffset`
/// (200 for the soldier, 221 for the Owl), the NPC graph's `ThrowDef*` CMSGs type 15 for
/// `defAnimOffset`. The mapping is inferred from the graphs' candidate lists.
const THROW_ATK_OFFSET_TYPE: i64 = 17;
const THROW_DEF_OFFSET_TYPE: i64 = 15;

/// TAE action flags of NPC attacks that accept the next combo attack: 23 "AI combo attack
/// queued" and 86 "AI attack queued".
const COMBO_FLAGS: [i32; 2] = [23, 86];

/// A deathblow as the duel reports it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DeathblowReport {
    /// ThrowParam start and body rows, on the step the deathblow began.
    pub started: Option<(i32, i32)>,
    /// The blade went in this step.
    pub hit: bool,
    pub killed: bool,
}

/// One step of the duel.
#[derive(Debug, Clone, Default)]
pub struct DuelReport {
    pub player: StepReport,
    pub enemy: StepReport,
    pub hits: Vec<(HitEvent, HitResult)>,
    pub locked_on: bool,
    pub deathblow: DeathblowReport,
    /// Both characters were put back at their start this step.
    pub respawned: bool,
}

/// ThrowParam and both models' sorb dummies, for deathblows.
struct Throws {
    table: ThrowTable,
    model: Flver,
    /// The enemy's model, for rows that snap Wolf to one of its dummies (`defSorbDmyId`).
    enemy_model: Option<Flver>,
}

pub struct Duel {
    pub camera: crate::camera::CameraParams,
    pub player: Character,
    pub enemy: Character,
    /// The stand-in brain, used when [`Duel::ai`] is `None`.
    pub brain: SimpleBrain,
    /// The soldier's own AI (its compiled Lua battle goal); `None` uses [`Duel::brain`].
    pub ai: Option<AiBrain>,
    /// Hitboxes and attack data: index 0 the player, 1 the enemy.
    pub fighters: [Combatant; 2],
    pub detector: HitDetector,
    /// HP and posture of both, and the hit rules.
    pub rules: CombatRules,
    /// Whether the enemy brain runs (off for scripted player-only tests).
    pub enemy_active: bool,
    /// The deathblow in progress.
    pub deathblow: Option<Deathblow>,
    /// Seconds until the respawn after a death.
    pub respawn_in: Option<f32>,
    throws: Option<Throws>,
    cache: PathBuf,
    /// Which enemy this duel loaded.
    pub spec: EnemySpec,
    /// Steps since the deathblow switched the enemy to its death animation (0: not yet).
    defender_dying: u8,
    spawn: Option<[Placement; 2]>,
    lock_button: bool,
    attack_button: bool,
    /// The enemy took a hit last step (the AI's damage interrupt).
    enemy_damaged: bool,
    /// The defender clip of the last deathblow; no new deathblow starts while it plays (the
    /// enemy is in its throw-defence state, getting up from a non-lethal blow).
    last_throw_def_clip: Option<String>,
    /// Hit stop of the player and the enemy: a frozen character does not advance.
    pub hit_stop: [HitStop; 2],
    /// Pushes in progress on the player and the enemy.
    pub knockback: [Option<Knockback>; 2],
    /// Each character's last real step, reported again while it is frozen.
    last_steps: [StepReport; 2],
    /// Thrown and shot attacks in flight.
    pub bullets: crate::bullets::Bullets,
    /// A character that falls this far below its start is put back there (a stand-in for
    /// fall death, which is not modelled yet). `None` on the flat floor, where nobody falls.
    pub fall_respawn_depth: Option<f32>,
    /// The boss brain choosing the enemy's acts with its Lua AI (see [`crate::brain`]);
    /// `None` leaves the Lua AI alone.
    pub boss_brain: Option<crate::brain::BrainDriver>,
    /// Testing aid: the player takes no HP or posture damage (hits, reactions, hit stop and
    /// knockback are unchanged).
    pub god: bool,
}

fn placement(c: &Character) -> Placement {
    Placement {
        position: c.body.position,
        yaw: c.body.yaw,
    }
}

/// `a200_510000` -> 510000.
fn anim_id(clip: &str) -> Option<i32> {
    clip.split_once('_')?.1.parse().ok()
}

impl Duel {
    /// Wolf at the origin facing -Z and an Ashina soldier (`c1010`, NpcParam 10100000) `distance`
    /// metres in front of him, facing him.
    pub fn load(cache: &Path, distance: f32) -> Result<Self, LoadError> {
        Self::load_with(cache, distance, EnemySpec::soldier())
    }

    /// Like [`Duel::load`], against the enemy `spec`.
    pub fn load_with(cache: &Path, distance: f32, spec: EnemySpec) -> Result<Self, LoadError> {
        let (player, enemy) = Self::load_characters(cache, distance, &spec)?;
        let fighters = [
            Combatant::player(cache, &player.skeleton),
            Combatant::npc(cache, &spec.chr, spec.npc_param, &enemy.skeleton),
        ];
        let rules = CombatRules::load(cache, &[&fighters[0], &fighters[1]]);
        let ai = Self::load_ai(cache, &spec);
        let read_model = |chr: &str| {
            let bytes = std::fs::read(cache.join(format!("raw/chr/{chr}.chrbnd.d/{chr}.flver")));
            flver::parse(&bytes.ok()?).ok()
        };
        let throws = ThrowTable::load_from_cache(cache).ok().and_then(|table| {
            Some(Throws {
                table,
                model: read_model("c0000")?,
                enemy_model: read_model(&spec.chr),
            })
        });
        Ok(Self {
            camera: crate::camera::CameraParams::load(cache),
            player,
            enemy,
            brain: SimpleBrain::default(),
            ai,
            fighters,
            detector: HitDetector::default(),
            rules,
            enemy_active: true,
            deathblow: None,
            respawn_in: None,
            throws,
            cache: cache.to_path_buf(),
            spec,
            defender_dying: 0,
            spawn: None,
            lock_button: false,
            attack_button: false,
            enemy_damaged: false,
            last_throw_def_clip: None,
            hit_stop: [HitStop::default(); 2],
            knockback: [None; 2],
            last_steps: Default::default(),
            bullets: Default::default(),
            fall_respawn_depth: None,
            boss_brain: None,
            god: false,
        })
    }

    /// The enemy's own AI from the extracted scripts, or `None` (with a warning) when they
    /// are missing or fail to load.
    fn load_ai(cache: &Path, spec: &EnemySpec) -> Option<AiBrain> {
        let (pd, dd) = (
            cache.join("raw/param/gameparam/gameparam.parambnd.d"),
            cache.join("refs/paramdex/Defs"),
        );
        let think = crate::hits::load_table(&pd, &dd, "NpcThinkParam", "NpcThinkParam")?
            .find(spec.think)
            .and_then(|r| ThinkParams::from_row(&r).ok())?;
        match AiBrain::load(cache, &spec.ai_bundle, think, AI_SEED) {
            Ok(mut brain) => {
                if spec.starts_in_battle {
                    brain.start_in_battle();
                }
                Some(brain)
            }
            Err(e) => {
                eprintln!("{} AI not loaded, using the stand-in brain: {e}", spec.chr);
                None
            }
        }
    }

    /// Switches the soldier to the stand-in [`SimpleBrain`] (for comparison and for tests that
    /// need a predictable attacker).
    pub fn use_simple_ai(&mut self) {
        self.boss_brain = None;
        self.ai = None;
    }

    /// Puts a boss brain (see [`crate::brain`]) in charge of the enemy's acts; its Lua AI keeps
    /// the reflexes. `log` writes a JSONL log to `cache/brain_logs/` and returns its path.
    /// Fails for an enemy without an action table or without its Lua AI.
    pub fn set_brain(
        &mut self,
        brain: Box<dyn crate::brain::BossBrain>,
        log: bool,
    ) -> Result<Option<PathBuf>, String> {
        let actions = crate::brain::BossActions::for_chr(&self.spec.chr)
            .ok_or_else(|| format!("no boss brain action list for {}", self.spec.chr))?;
        if self.ai.is_none() {
            return Err(format!(
                "{} has no Lua AI to pair the brain with",
                self.spec.chr
            ));
        }
        let mut driver = crate::brain::BrainDriver::new(brain, actions);
        let path = if log {
            Some(
                driver
                    .log_to(&self.cache.join("brain_logs"))
                    .map_err(|e| format!("brain log: {e}"))?,
            )
        } else {
            None
        };
        self.boss_brain = Some(driver);
        Ok(path)
    }

    /// Whether Wolf's current attack carries the parry-timing flag ([`FLAG_PARRY_TIMING`]).
    /// Range and facing are the AI script's own checks: `Common_Parry` tests the parry
    /// distance (`GetDist_Parry`) and a front cone itself.
    fn parry_timing(&self) -> bool {
        self.player.tae_frame().flag(FLAG_PARRY_TIMING)
    }

    /// What the soldier's AI sees this step.
    fn ai_world(&self, dt: f32) -> AiWorld {
        let actor = |c: &Character, i: usize| {
            let f = &self.rules.fighters[i];
            let mut sp_effects = c.tae_frame().sp_effects.clone();
            sp_effects.extend_from_slice(f.resident());
            AiActor {
                position: c.body.position,
                yaw: c.body.yaw,
                hp: f.vitality.hp as f32,
                max_hp: f.vitality.max_hp as f32,
                posture: f.posture.remaining as f32,
                max_posture: f.posture.max as f32,
                sp_effects,
                guarding: c.tae_frame().flag(crate::hits::FLAG_GUARD),
            }
        };
        let main = self.enemy.behavior.runtime.main_clip();
        let frame = self.enemy.tae_frame();
        AiWorld {
            dt,
            me: actor(&self.enemy, 1),
            target: (!self.rules.fighters[0].dead()).then(|| actor(&self.player, 0)),
            hit_radius: self.fighters[1].radius,
            current_anim: main.and_then(|c| anim_id_from_clip(&c.animation)),
            combo_window: COMBO_FLAGS.iter().any(|&f| frame.flag(f))
                || main.is_some_and(|c| c.at_end()),
            home: self
                .spawn
                .map_or(self.enemy.body.position, |s| s[1].position),
            parry_timing: self.parry_timing(),
            damaged: self.enemy_damaged,
        }
    }

    fn load_characters(
        cache: &Path,
        distance: f32,
        spec: &EnemySpec,
    ) -> Result<(Character, Character), LoadError> {
        let mut player = Character::load_from_cache(cache)?;
        let mut enemy = Character::load_npc(cache, &spec.chr)?;
        enemy.body.position = [0.0, 0.0, -distance];
        enemy.body.yaw = std::f32::consts::PI;
        let dt = 1.0 / 60.0;
        player.settle(dt, 400);
        enemy.settle(dt, 400);
        Ok((player, enemy))
    }

    /// Puts both characters back where they stood on the first step, with fresh scripts, full
    /// HP and posture. Each keeps its ground.
    pub fn reset(&mut self) -> Result<(), LoadError> {
        let (mut player, mut enemy) = Self::load_characters(&self.cache, 4.0, &self.spec)?;
        std::mem::swap(&mut player.ground, &mut self.player.ground);
        std::mem::swap(&mut enemy.ground, &mut self.enemy.ground);
        if let Some([p, e]) = self.spawn {
            (player.body.position, player.body.yaw) = (p.position, p.yaw);
            (enemy.body.position, enemy.body.yaw) = (e.position, e.yaw);
        }
        self.player = player;
        self.enemy = enemy;
        self.rules.reset();
        self.detector = HitDetector::default();
        if self.ai.is_some() {
            self.ai = Self::load_ai(&self.cache, &self.spec);
        }
        self.enemy_damaged = false;
        self.hit_stop = [HitStop::default(); 2];
        self.knockback = [None; 2];
        self.deathblow = None;
        self.respawn_in = None;
        self.defender_dying = 0;
        self.last_throw_def_clip = None;
        self.bullets.clear();
        Ok(())
    }

    /// Lock on to the enemy if it is within range and roughly in front of the camera.
    fn update_lock(&mut self, input: &InputFrame) {
        let pressed = input.buttons.lock_on && !self.lock_button;
        self.lock_button = input.buttons.lock_on;
        let enemy_alive = !self.rules.fighters[1].dead();
        if !pressed {
            // Follow the target while locked on.
            if self.player.lock_target.is_some() {
                self.player.lock_target = enemy_alive.then_some(self.enemy.body.position);
            }
            return;
        }
        if self.player.lock_target.is_some() {
            self.player.lock_target = None;
            return;
        }
        let p = self.player.body.position;
        let e = self.enemy.body.position;
        let dist = ((e[0] - p[0]).powi(2) + (e[2] - p[2]).powi(2)).sqrt();
        if dist <= self.camera.lock_range && enemy_alive {
            self.player.lock_target = Some(e);
        }
    }

    /// Keeps the two bodies from overlapping: pushes both apart along the line between them,
    /// each through its own collision so the push never carries one into a wall.
    /// (Character-character collision; the engine uses the Havok character proxies.)
    /// While the enemy still plays the last throw's defender clip (getting up after a deathblow)
    /// only Wolf gives way, so walking into him does not shove him out of his animation (ours,
    /// after FwLo's RustSekiro, where throw pairs skip the push-apart).
    fn separate(&mut self) {
        let min = self.fighters[0].radius + self.fighters[1].radius;
        let (p, e) = (self.player.body.position, self.enemy.body.position);
        let (dx, dz) = (e[0] - p[0], e[2] - p[2]);
        let d = (dx * dx + dz * dz).sqrt();
        if d >= min || !self.player.body.grounded || !self.enemy.body.grounded {
            return;
        }
        let (nx, nz) = if d > 1e-4 {
            (dx / d, dz / d)
        } else {
            (0.0, -1.0)
        };
        let enemy_share = if self.enemy_in_throw_clip() { 0.0 } else { 0.5 };
        let overlap = min - d;
        for (c, share, sign) in [
            (&mut self.player, 1.0 - enemy_share, -1.0),
            (&mut self.enemy, enemy_share, 1.0),
        ] {
            if share <= 0.0 {
                continue;
            }
            let push = overlap * share * sign;
            let [mx, mz] = c.ground.slide(c.body.position, [nx * push, nz * push]);
            c.body.position[0] += mx;
            c.body.position[2] += mz;
        }
    }

    /// Whether the enemy still plays the last deathblow's defender clip (or its death clip).
    fn enemy_in_throw_clip(&self) -> bool {
        let Some(clip) = self.enemy.behavior.runtime.main_clip() else {
            return false;
        };
        let Some(def) = &self.last_throw_def_clip else {
            return false;
        };
        clip.animation == *def
            || anim_id(def)
                .zip(anim_id(&clip.animation))
                .is_some_and(|(d, c)| c == d + 1)
    }

    /// Whether pressing attack now would start a deathblow.
    pub fn deathblow_available(&self) -> bool {
        self.deathblow_rows().is_some()
    }

    fn deathblow_rows(&self) -> Option<(i32, i32)> {
        // The enemy must have reacted to the hit that broke it first: while it is frozen in hit
        // stop or has not read the damage yet, its script would still send it into the stagger
        // after the throw event (ours; the engine checks the throwable state the stagger sets).
        if self.deathblow.is_some()
            || self.rules.fighters[0].dead()
            || !self.rules.deathblow_open(1)
            || self.hit_stop[1].active()
            || self.enemy.pending_damage.is_some()
        {
            return None;
        }
        if let Some(clip) = &self.last_throw_def_clip
            && self
                .enemy
                .behavior
                .runtime
                .main_clip()
                .is_some_and(|c| &c.animation == clip)
        {
            return None;
        }
        let throws = self.throws.as_ref()?;
        let situation = if self.lethal_deathblow() {
            Situation::Final
        } else {
            Situation::PostureBroken
        };
        select(
            &throws.table,
            self.spec.throw_chr(),
            0,
            situation,
            &placement(&self.player),
            &placement(&self.enemy),
        )
        .map(|(s, b)| (s.id, b.id))
    }

    /// Whether a deathblow now would spend the enemy's last mark (NpcParam `ninsatuNum`, 0
    /// counting as one).
    fn lethal_deathblow(&self) -> bool {
        deathblow_marks(self.rules.fighters[1].vitality.deathblows_left) <= 1
    }

    /// Starts a deathblow on the enemy if one is open and Wolf stands in a ThrowParam range.
    /// Fires the throw events into both graphs, as the engine does (the scripts never start
    /// throws themselves).
    fn try_deathblow(&mut self) -> Option<(i32, i32)> {
        let rows = self.deathblow_rows()?;
        let throws = self.throws.as_ref()?;
        let row = |id: i32| throws.table.rows.iter().find(|r| r.id == id);
        let (start, body) = (row(rows.0)?, row(rows.1)?);
        let (attacker, defender) = (placement(&self.player), placement(&self.enemy));
        let defender_clip = body.defender_anim();
        let times = ThrowTimes::from_tae_db(
            &self.player.tae,
            &start.attacker_anim(),
            &body.attacker_anim(),
            defender_clip.as_deref().map(|d| (&*self.enemy.tae, d)),
        )?;
        let sorb = if body.atk_sorb_dummy <= 0 && body.def_sorb_dummy > 0 {
            // Wolf is snapped to the enemy's dummy; hold the enemy where that puts it.
            DummyFrame::from_flver(throws.enemy_model.as_ref()?, body.def_sorb_dummy)?.inverse()
        } else {
            DummyFrame::from_flver(&throws.model, body.atk_sorb_dummy)?
        };
        let (start, body) = (start.clone(), body.clone());
        let body_duration = self.player.clip_duration(&body.attacker_anim())?;
        let marks = self.rules.fighters[1].vitality.deathblows_left;
        // Whether the enemy's TAE has the death clip (env 1114's "does the animation exist").
        let death_clip = body
            .defender_death_anim()
            .and_then(|clip| sekiro_formats::tae::anim_id(&clip))
            .is_some_and(|id| self.enemy.behavior.env.existing_anims.contains(&id));
        let (mut db, first) = Deathblow::begin(
            (&start, &body),
            times,
            sorb,
            body_duration,
            deathblow_marks(marks),
        );
        db.death_clip = death_clip;
        if body.atk_sorb_dummy <= 0 && body.def_sorb_dummy > 0 {
            db.absorbed = Absorbed::Attacker;
        }
        self.last_throw_def_clip = body.defender_anim();
        if let Some(yaw) = db.start_facing(&attacker, &defender) {
            self.player.body.yaw = yaw;
        }
        if let Some(clip) = first.attacker_clip.as_deref().and_then(anim_id) {
            let rt = &mut self.player.behavior.runtime;
            rt.offsets
                .by_type
                .insert(THROW_ATK_OFFSET_TYPE, start.atk_anim_offset as u32);
            rt.fire_event(&format!("W_ThrowAtk{clip}"));
        }
        self.player.lock_target = None;
        // The throw takes over both bodies: a knockback left from the breaking hit must not
        // resume (and shove the enemy) once the pair ends.
        self.knockback = [None, None];
        self.deathblow = Some(db);
        self.defender_dying = 0;
        Some(rows)
    }

    /// Advances the deathblow in progress after both characters moved this step.
    fn step_deathblow(&mut self, dt: f32, report: &mut DeathblowReport) {
        let Some(db) = self.deathblow.as_mut() else {
            return;
        };
        let step = db.step(dt, &placement(&self.player), &placement(&self.enemy));
        if let Some(clip) = step.attacker_clip.as_deref().and_then(anim_id) {
            let rt = &mut self.player.behavior.runtime;
            rt.offsets
                .by_type
                .insert(THROW_ATK_OFFSET_TYPE, db.body.atk_anim_offset as u32);
            rt.fire_event(&format!("W_ThrowAtk{clip}"));
        }
        if let Some(clip) = &step.defender_clip {
            let death = db.body.defender_death_anim().as_deref() == Some(clip.as_str());
            if let Some(id) = anim_id(clip) {
                if death {
                    self.defender_dying = 1;
                }
                self.enemy
                    .behavior
                    .runtime
                    .offsets
                    .by_type
                    .insert(THROW_DEF_OFFSET_TYPE, db.body.def_anim_offset as u32);
                let event = if death {
                    format!("W_ThrowDefDeath{id}")
                } else {
                    format!("W_ThrowDef{id}")
                };
                self.enemy.behavior.runtime.fire_event(&event);
            }
        }
        let left_clip = db.phase == Phase::Body
            && step.attacker_clip.is_none()
            && db.time > dt
            && self
                .player
                .behavior
                .runtime
                .main_clip()
                .is_none_or(|c| c.animation != db.body.attacker_anim());
        if left_clip {
            // The pair ends when Wolf leaves his body clip (its TAE opens move and action
            // cancels before the end); the enemy plays its own clip out.
            db.release();
        }
        if step.hit {
            self.rules.deathblow(1);
            report.hit = true;
            report.killed = step.killed;
            if step.killed && !db.death_clip {
                // The defender clip is itself the death: the script may see HP 0 from now on.
                self.defender_dying = self.defender_dying.max(1);
            }
        }
        if matches!(db.phase, Phase::Done | Phase::Missed) {
            self.deathblow = None;
        }
        if let Some(a) = step.absorb {
            self.absorb(&a);
        }
    }

    /// Moves the absorbed character its share of the way onto the other's sorb dummy, through
    /// its collision. What a wall stops, the other character gives way instead (through its own
    /// collision), so the pair keeps its relation without either ending up inside geometry.
    /// That shift of the pair is inferred: the engine's handling is not traced.
    fn absorb(&mut self, a: &AbsorbStep) {
        let (moved, other) = match a.who {
            Absorbed::Attacker => (&mut self.player, &mut self.enemy),
            Absorbed::Defender => (&mut self.enemy, &mut self.player),
        };
        let want = [a.delta[0], a.delta[2]];
        let got = moved.ground.slide(moved.body.position, want);
        moved.body.position[0] += got[0];
        moved.body.position[2] += got[1];
        moved.body.yaw = angle_diff(moved.body.yaw + a.yaw, 0.0);
        let rest = [got[0] - want[0], got[1] - want[1]];
        if rest[0].hypot(rest[1]) > 1e-5 {
            let back = other.ground.slide(other.body.position, rest);
            other.body.position[0] += back[0];
            other.body.position[2] += back[1];
        }
    }

    /// Passes HP and posture to both scripts (env 1000, 1001, 2010). While a deathblow holds the
    /// enemy its script still sees it alive, so the throw animations (with their own death
    /// branch) play out instead of the script's ordinary death.
    fn publish_vitals(&mut self) {
        // The script sees HP 0 one step after the death event, once its graph is in the death state.
        let held = self.deathblow.is_some() && self.defender_dying < 2;
        for (i, (c, f)) in [&mut self.player, &mut self.enemy]
            .into_iter()
            .zip(&self.rules.fighters)
            .enumerate()
        {
            let hp = f.vitality.hp as f32;
            c.behavior.env.hp = if held && i == 1 { hp.max(1.0) } else { hp };
            c.behavior.env.posture = f.posture.remaining as f32;
            c.behavior.env.max_posture = f.posture.max as f32;
        }
    }

    /// Steps character `i` by `dt` seconds of its own time, or holds it frozen (hit stop) when
    /// `dt` is 0: no animation, script, TAE or body update, and no TAE one-shots (sounds,
    /// velocity changes) repeated. The player's buttons are still read, so a press during the
    /// freeze is buffered like any other.
    fn advance_character(
        &mut self,
        i: usize,
        input: &InputFrame,
        real_dt: f32,
        dt: f32,
    ) -> StepReport {
        let c = if i == 0 {
            &mut self.player
        } else {
            &mut self.enemy
        };
        if dt > 0.0 {
            let report = c.tick(input, dt);
            self.last_steps[i] = report.clone();
            return report;
        }
        if i == 0 {
            c.input.update(input, real_dt, &c.tae_runtime.frame);
        }
        let frame = &mut c.tae_runtime.frame;
        frame.sounds.clear();
        frame.velocity_changes.clear();
        let mut report = self.last_steps[i].clone();
        report.clip_started = false;
        report.blend_in = 0.0;
        report.advanced = 0.0;
        report.requested.clear();
        report.behavior.events.clear();
        report.behavior.hooks.clear();
        report.behavior.script_errors.clear();
        report
    }

    /// Moves character `i` along its knockback for `dt` seconds of its own time, through its
    /// collision so walls stop the push.
    fn push(&mut self, i: usize, dt: f32) {
        let Some(kb) = self.knockback[i].as_mut() else {
            return;
        };
        let delta = kb.advance(dt);
        if kb.finished() {
            self.knockback[i] = None;
        }
        let c = if i == 0 {
            &mut self.player
        } else {
            &mut self.enemy
        };
        let [mx, mz] = c.ground.slide(c.body.position, delta);
        c.body.position[0] += mx;
        c.body.position[2] += mz;
    }

    /// Puts a character that fell [`Duel::fall_respawn_depth`] below its start back at its
    /// start, standing still.
    fn respawn_fallen(&mut self) {
        let (Some(depth), Some(spawn)) = (self.fall_respawn_depth, self.spawn) else {
            return;
        };
        for (c, start) in [&mut self.player, &mut self.enemy].into_iter().zip(spawn) {
            if c.body.position[1] < start.position[1] - depth {
                c.body.position = start.position;
                c.body.yaw = start.yaw;
                c.body.velocity = [0.0; 3];
                c.body.grounded = true;
                c.body.fall_top = start.position[1];
            }
        }
    }

    pub fn step(&mut self, input: &InputFrame, dt: f32) -> DuelReport {
        if self.spawn.is_none() {
            self.spawn = Some([placement(&self.player), placement(&self.enemy)]);
        }
        let mut respawned = false;
        if let Some(t) = self.respawn_in.as_mut() {
            *t -= dt;
            if *t <= 0.0 && self.reset().is_ok() {
                respawned = true;
            }
        }
        self.update_lock(input);

        // The engine checks for a deathblow before the attack reaches the script.
        let mut input = *input;
        let attack_pressed = input.buttons.attack && !self.attack_button;
        self.attack_button = input.buttons.attack;
        let mut deathblow = DeathblowReport::default();
        if attack_pressed && let Some(rows) = self.try_deathblow() {
            deathblow.started = Some(rows);
            input.buttons.attack = false;
        }

        if let Some(d) = self.boss_brain.as_mut() {
            d.tick(dt);
        }
        let enemy_alive = !self.rules.fighters[1].dead();
        if self.enemy_active && enemy_alive && self.deathblow.is_none() {
            if self.ai.is_some() {
                let world = self.ai_world(dt);
                let mut driver = self.boss_brain.take();
                if let Some(mut ai) = self.ai.take() {
                    if let Some(d) = driver.as_mut() {
                        d.before_think(&mut ai, self.fighters[1].radius);
                    }
                    self.enemy.npc = ai.think(world);
                    if let Some(d) = driver.as_mut() {
                        d.after_think(&mut ai, self);
                    }
                    self.ai = Some(ai);
                }
                self.boss_brain = driver;
            } else {
                self.brain
                    .think(&mut self.enemy, self.player.body.position, dt);
            }
        } else {
            self.enemy.npc = Default::default();
        }
        self.publish_vitals();
        // Who each side aims at: auto-aim candidates for Wolf, Wolf for the soldier.
        self.player.aim_candidates = if enemy_alive {
            vec![self.enemy.body.position]
        } else {
            Vec::new()
        };
        self.enemy.npc_target =
            (!self.rules.fighters[0].dead()).then_some(self.player.body.position);
        // Hit stop: each character advances only by what is left of the step after its freeze.
        let dts = [self.hit_stop[0].split(dt), self.hit_stop[1].split(dt)];
        let player = self.advance_character(0, &input, dt, dts[0]);
        let enemy = self.advance_character(1, &InputFrame::default(), dt, dts[1]);
        if self.deathblow.is_none() {
            self.push(0, dts[0]);
            self.push(1, dts[1]);
        }
        if self.defender_dying > 0 {
            self.defender_dying = self.defender_dying.saturating_add(1);
        }
        self.step_deathblow(dt, &mut deathblow);
        if self.deathblow.is_none() && enemy_alive {
            self.separate();
        }
        self.respawn_fallen();

        if self.god {
            let f = &mut self.rules.fighters[0];
            f.vitality.cannot_die = true;
            f.posture.no_consume = true;
        }
        let mut found = self.detector.detect(
            &[&self.player, &self.enemy],
            &[&self.fighters[0], &self.fighters[1]],
        );
        let chars = [&self.player, &self.enemy];
        let fighters = [&self.fighters[0], &self.fighters[1]];
        self.bullets.spawn(&chars, &fighters);
        let bullet_hits = self.bullets.advance(dt, &chars, &fighters);
        let first_bullet_hit = found.len();
        found.extend(bullet_hits);
        let mut hits = Vec::new();
        self.enemy_damaged = false;
        for (k, hit) in found.into_iter().enumerate() {
            if self.deathblow.is_some() || self.rules.fighters.iter().any(|f| f.dead()) {
                break;
            }
            let from_bullet = k >= first_bullet_hit;
            let result = self.rules.resolve(
                &hit,
                [&self.player, &self.enemy],
                [&self.fighters[0], &self.fighters[1]],
            );
            let (att, def) = if hit.attacker == 0 {
                (&mut self.player, &mut self.enemy)
            } else {
                (&mut self.enemy, &mut self.player)
            };
            def.pending_damage = Some(result.defender);
            if let Some(back) = result.attacker {
                att.pending_damage = Some(back);
            }
            self.knockback[hit.defender] = result.knockback;
            // A bullet's hit does not freeze its distant shooter (ours).
            if !from_bullet {
                self.hit_stop[hit.attacker].start(result.hit_stop[0]);
            }
            self.hit_stop[hit.defender].start(result.hit_stop[1]);
            self.enemy_damaged |= hit.defender == 1 && !result.guarded;
            hits.push((hit, result));
        }
        self.rules.apply_effect_hp(0, self.player.tae_frame());
        self.rules.apply_effect_hp(1, self.enemy.tae_frame());
        if self.god {
            let v = &mut self.rules.fighters[0].vitality;
            v.hp = v.max_hp;
        }
        self.rules.recover(0, &self.player, dts[0]);
        self.rules.recover(1, &self.enemy, dts[1]);
        if self.respawn_in.is_none()
            && self.deathblow.is_none()
            && self.rules.fighters.iter().any(|f| f.dead())
        {
            self.respawn_in = Some(RESPAWN_DELAY);
        }
        let report = DuelReport {
            locked_on: self.player.lock_target.is_some(),
            player,
            enemy,
            hits,
            deathblow,
            respawned,
        };
        if let Some(mut d) = self.boss_brain.take() {
            d.after_step(self, &report);
            self.boss_brain = Some(d);
        }
        report
    }
}

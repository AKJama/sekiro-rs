//! Wolf against one enemy: both characters, the enemy's brain, lock-on, hits with the engine's
//! HP and posture rules, deathblows, and a respawn after a death.

use crate::ai::SimpleBrain;
use crate::character::{Character, StepReport};
use crate::deathblow::{
    Deathblow, DummyFrame, Phase, Placement, Situation, ThrowTable, ThrowTimes, deathblow_marks,
    select,
};
use crate::fight::{CombatRules, FighterKind};
use crate::hits::{Combatant, HitDetector, HitEvent, HitResult};
use crate::input::InputFrame;
use crate::lua_ai::{AiActor, AiBrain, AiWorld, ThinkParams, anim_id_from_clip};
use crate::player::LoadError;
use sekiro_formats::flver::{self, Flver};
use std::path::{Path, PathBuf};

/// Lock-on reach, metres (assumed; the game reads it from LockCamParam and the target's size).
pub const LOCK_RANGE: f32 = 15.0;

/// Seconds between a death and both characters respawning at their start positions (a stand-in
/// for the game's death screen and reload).
pub const RESPAWN_DELAY: f32 = 6.0;

/// The soldier's character id in ThrowParam (`defChrId`).
const SOLDIER_THROW_CHR: i32 = 1010;

/// The soldier's NpcThinkParam row and the map script bundle its battle goal lives in.
const SOLDIER_THINK: i32 = 10_100_000;
const SOLDIER_AI_BUNDLE: &str = "m11_00_00_00.luabnd.d";
/// Seed for the AI's random numbers (fixed, so runs repeat).
const AI_SEED: u64 = 7;

/// How long before Wolf's TAE attack window opens the AI is told an attack is coming (the
/// engine's parry-timing signal), seconds, and within what distance (assumed values).
const PARRY_LEAD: f32 = 0.3;
const PARRY_RANGE: f32 = 3.5;

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

/// ThrowParam and the player model's sorb dummies, for deathblows.
struct Throws {
    table: ThrowTable,
    model: Flver,
}

pub struct Duel {
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
    /// Steps since the deathblow switched the enemy to its death animation (0: not yet).
    defender_dying: u8,
    spawn: Option<[Placement; 2]>,
    lock_button: bool,
    attack_button: bool,
    /// The enemy took a hit last step (the AI's damage interrupt).
    enemy_damaged: bool,
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
        let (player, enemy) = Self::load_characters(cache, distance)?;
        let fighters = [
            Combatant::player(cache, &player.skeleton),
            Combatant::npc(cache, "c1010", 10_100_000, &enemy.skeleton),
        ];
        let rules = CombatRules::load(cache, &[&fighters[0], &fighters[1]]);
        let ai = Self::load_ai(cache);
        let throws = ThrowTable::load_from_cache(cache).ok().and_then(|table| {
            let bytes = std::fs::read(cache.join("raw/chr/c0000.chrbnd.d/c0000.flver")).ok()?;
            Some(Throws {
                table,
                model: flver::parse(&bytes).ok()?,
            })
        });
        Ok(Self {
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
            defender_dying: 0,
            spawn: None,
            lock_button: false,
            attack_button: false,
            enemy_damaged: false,
        })
    }

    /// The soldier's own AI from the extracted scripts, or `None` (with a warning) when they
    /// are missing or fail to load.
    fn load_ai(cache: &Path) -> Option<AiBrain> {
        let (pd, dd) = (
            cache.join("raw/param/gameparam/gameparam.parambnd.d"),
            cache.join("refs/paramdex/Defs"),
        );
        let think = crate::hits::load_table(&pd, &dd, "NpcThinkParam", "NpcThinkParam")?
            .find(SOLDIER_THINK)
            .and_then(|r| ThinkParams::from_row(&r).ok())?;
        match AiBrain::load(cache, SOLDIER_AI_BUNDLE, think, AI_SEED) {
            Ok(brain) => Some(brain),
            Err(e) => {
                eprintln!("soldier AI not loaded, using the stand-in brain: {e}");
                None
            }
        }
    }

    /// Switches the soldier to the stand-in [`SimpleBrain`] (for comparison and for tests that
    /// need a predictable attacker).
    pub fn use_simple_ai(&mut self) {
        self.ai = None;
    }

    /// Whether Wolf's attack is about to land on the soldier: a TAE attack window of his clip
    /// is open or opens within [`PARRY_LEAD`], and the soldier is within [`PARRY_RANGE`].
    fn parry_timing(&self) -> bool {
        let (p, e) = (self.player.body.position, self.enemy.body.position);
        if ((p[0] - e[0]).powi(2) + (p[2] - e[2]).powi(2)).sqrt() > PARRY_RANGE {
            return false;
        }
        let Some(clip) = self.player.behavior.runtime.main_clip() else {
            return false;
        };
        let Some(anim) = self.player.tae.anim(&clip.animation) else {
            return false;
        };
        let t = clip.time;
        anim.events.iter().any(|ev| {
            matches!(ev.kind, crate::tae::TaeKind::Attack { .. })
                && ev.end > t
                && ev.start <= t + PARRY_LEAD
        })
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

    fn load_characters(cache: &Path, distance: f32) -> Result<(Character, Character), LoadError> {
        let mut player = Character::load_from_cache(cache)?;
        let mut enemy = Character::load_npc(cache, "c1010")?;
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
        let (mut player, mut enemy) = Self::load_characters(&self.cache, 4.0)?;
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
            self.ai = Self::load_ai(&self.cache);
        }
        self.enemy_damaged = false;
        self.deathblow = None;
        self.respawn_in = None;
        self.defender_dying = 0;
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
        if dist <= LOCK_RANGE && enemy_alive {
            self.player.lock_target = Some(e);
        }
    }

    /// Keeps the two bodies from overlapping: pushes both apart along the line between them.
    /// (Character-character collision; the engine uses the Havok character proxies.)
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
        let push = (min - d) * 0.5;
        self.player.body.position[0] -= nx * push;
        self.player.body.position[2] -= nz * push;
        self.enemy.body.position[0] += nx * push;
        self.enemy.body.position[2] += nz * push;
    }

    /// Whether pressing attack now would start a deathblow.
    pub fn deathblow_available(&self) -> bool {
        self.deathblow_rows().is_some()
    }

    fn deathblow_rows(&self) -> Option<(i32, i32)> {
        if self.deathblow.is_some()
            || self.rules.fighters[0].dead()
            || !self.rules.deathblow_open(1)
        {
            return None;
        }
        let throws = self.throws.as_ref()?;
        select(
            &throws.table,
            SOLDIER_THROW_CHR,
            0,
            Situation::PostureBroken,
            &placement(&self.player),
            &placement(&self.enemy),
        )
        .map(|(s, b)| (s.id, b.id))
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
        let times = ThrowTimes::load(
            &self.cache,
            "c0000",
            &start.attacker_anim(),
            &body.attacker_anim(),
            body.defender_anim().as_deref().map(|d| ("c1010", d)),
        )?;
        let sorb = DummyFrame::from_flver(&throws.model, body.atk_sorb_dummy)?;
        let (start, body) = (start.clone(), body.clone());
        let body_duration = self.player.clip_duration(&body.attacker_anim())?;
        let marks = match &self.rules.fighters[1].kind {
            FighterKind::Npc(npc) => npc.deathblows,
            FighterKind::Player { .. } => 0,
        };
        let (db, first) = Deathblow::begin(
            (&start, &body),
            times,
            sorb,
            body_duration,
            deathblow_marks(marks),
        );
        if let Some(yaw) = db.start_facing(&attacker, &defender) {
            self.player.body.yaw = yaw;
        }
        if let Some(clip) = first.attacker_clip.as_deref().and_then(anim_id) {
            self.player
                .behavior
                .runtime
                .fire_event(&format!("W_ThrowAtk{clip}"));
        }
        self.player.lock_target = None;
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
            self.player
                .behavior
                .runtime
                .fire_event(&format!("W_ThrowAtk{clip}"));
        }
        if let Some(clip) = &step.defender_clip {
            let death = db.body.defender_death_anim().as_deref() == Some(clip.as_str());
            if let Some(id) = anim_id(clip) {
                if death {
                    self.defender_dying = 1;
                }
                let event = if death {
                    format!("W_ThrowDefDeath{id}")
                } else {
                    format!("W_ThrowDef{id}")
                };
                self.enemy.behavior.runtime.fire_event(&event);
            }
        }
        if let Some(p) = step.defender_placement {
            self.enemy.body.position = p.position;
            self.enemy.body.yaw = p.yaw;
        }
        if step.hit {
            self.rules.deathblow(1);
            report.hit = true;
            report.killed = step.killed;
        }
        if matches!(db.phase, Phase::Done | Phase::Missed) {
            self.deathblow = None;
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

        let enemy_alive = !self.rules.fighters[1].dead();
        if self.enemy_active && enemy_alive && self.deathblow.is_none() {
            if self.ai.is_some() {
                let world = self.ai_world(dt);
                if let Some(ai) = self.ai.as_mut() {
                    self.enemy.npc = ai.think(world);
                }
            } else {
                self.brain
                    .think(&mut self.enemy, self.player.body.position, dt);
            }
        } else {
            self.enemy.npc = Default::default();
        }
        self.publish_vitals();
        let player = self.player.tick(&input, dt);
        let enemy = self.enemy.tick(&InputFrame::default(), dt);
        if self.defender_dying > 0 {
            self.defender_dying = self.defender_dying.saturating_add(1);
        }
        self.step_deathblow(dt, &mut deathblow);
        if self.deathblow.is_none() && enemy_alive {
            self.separate();
        }

        let found = self.detector.detect(
            &[&self.player, &self.enemy],
            &[&self.fighters[0], &self.fighters[1]],
        );
        let mut hits = Vec::new();
        self.enemy_damaged = false;
        for hit in found {
            if self.deathblow.is_some() || self.rules.fighters.iter().any(|f| f.dead()) {
                break;
            }
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
            self.enemy_damaged |= hit.defender == 1 && !result.guarded;
            hits.push((hit, result));
        }
        self.rules.recover(0, &self.player, dt);
        self.rules.recover(1, &self.enemy, dt);
        if self.respawn_in.is_none()
            && self.deathblow.is_none()
            && self.rules.fighters.iter().any(|f| f.dead())
        {
            self.respawn_in = Some(RESPAWN_DELAY);
        }
        DuelReport {
            locked_on: self.player.lock_target.is_some(),
            player,
            enemy,
            hits,
            deathblow,
            respawned,
        }
    }
}

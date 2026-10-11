//! Wolf against one enemy: both characters, the enemy's brain, lock-on, hits with the engine's
//! HP and posture rules, deathblows, and a respawn after a death.

use crate::ai::SimpleBrain;
use crate::arena::BossEvents;
use crate::character::{Character, StepReport};
use crate::combat::knockback::{HitStop, Knockback};
use crate::deathblow::{
    AbsorbStep, Absorbed, Deathblow, DummyFrame, Phase, Placement, Situation, ThrowTable,
    ThrowTimes, deathblow_marks, select,
};
use crate::enemy::EnemySpec;
use crate::events::{EventCommand, EventHost, EventScripts, PLAYER_ENTITY};
use crate::fight::CombatRules;
use crate::grab::{EnemyThrow, GrabPhase};
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

#[derive(serde::Deserialize)]
struct FallMaterial {
    id: u32,
    #[serde(rename = "fallDeathDist")]
    fall_death_dist: u32,
    #[serde(rename = "disableFallDamage")]
    disable_fall_damage: u8,
}

fn load_fall_materials(cache: &Path) -> std::collections::HashMap<u32, FallMaterial> {
    #[derive(serde::Deserialize)]
    struct Materials {
        rows: Vec<FallMaterial>,
    }
    std::fs::read(cache.join("params/HitMtrlParam.json"))
        .ok()
        .and_then(|data| serde_json::from_slice::<Materials>(&data).ok())
        .map(|table| table.rows.into_iter().map(|row| (row.id, row)).collect())
        .unwrap_or_default()
}

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

/// SpEffect `stateInfo` 350: the character cannot be locked on to (SpEffect 200602 "cannot be
/// locked, seconds given, Ninja Army Chief").
const STATE_INFO_NO_LOCK: i32 = 350;

/// Owl's TAE transition judgment marker, active throughout the heat-up and kneeling loop.
const OWL_HEAT_UP: i32 = 3_506_080;

/// env 2004, "swing is hitting" ([`Duel::publish_swing_hit`]).
const ENV_SWING_HIT: i32 = 2004;

/// A deathblow as the duel reports it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DeathblowReport {
    /// ThrowParam start and body rows, on the step the deathblow began.
    pub started: Option<(i32, i32)>,
    /// The blade went in this step.
    pub hit: bool,
    pub killed: bool,
}

/// An enemy throw on Wolf as the duel reports it (see [`crate::grab`]).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GrabReport {
    /// ThrowParam row of a throw that caught Wolf this step.
    pub started: Option<i32>,
    /// The throw's damage that landed this step: (behaviour judge, HP, posture).
    pub hits: Vec<(i32, i32, i32)>,
}

/// One step of the duel.
#[derive(Debug, Clone, Default)]
pub struct DuelReport {
    pub player: StepReport,
    pub enemy: StepReport,
    pub hits: Vec<(HitEvent, HitResult)>,
    pub locked_on: bool,
    pub deathblow: DeathblowReport,
    pub grab: GrabReport,
    /// Both characters were put back at their start this step.
    pub respawned: bool,
    /// A perilous-attack warning (the red kanji) reached Wolf this step ([`crate::perilous`]).
    pub perilous: Option<crate::perilous::PerilousWarning>,
    /// Wolf's anti-ground attack (the air kick) landed on an enemy in a perilous sweep, whose
    /// "jump kick effective" SpEffect multiplied its posture damage this step.
    pub jump_counter: bool,
    /// ThrowParam row and posture damage of a Mikiri begun this step.
    pub mikiri: Option<(i32, i32)>,
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
    /// The camera as Wolf's animation drives it (the deathblow camera, TAE 151 / 153 / 155),
    /// updated every step.
    pub camera_director: crate::camera::CameraDirector,
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
    /// The enemy's throw on Wolf in progress.
    pub enemy_throw: Option<EnemyThrow>,
    mikiri: Option<crate::perilous::Mikiri>,
    /// Testing aid: the enemy requests this action (an animation id such as 3040) whenever it is
    /// not already playing it, on top of its brain (or alone with the brain off).
    pub force_act: Option<i32>,
    /// Seconds until the respawn after a death.
    pub respawn_in: Option<f32>,
    throws: Option<Throws>,
    cache: PathBuf,
    /// Which enemy this duel loaded.
    pub spec: EnemySpec,
    initial_spec: EnemySpec,
    pending_handover: Option<(i32, Option<i32>)>,
    /// Map-event cinematic boundary, queued until any active deathblow releases.
    pub cinematic: Option<crate::events::Cinematic>,
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
    /// Wolf's full-body clip in which one of his attacks connected: env 2004 ("swing is hitting")
    /// reads true while it still plays.
    swing_hit: Option<String>,
    /// Hit stop of the player and the enemy: a frozen character does not advance.
    pub hit_stop: [HitStop; 2],
    /// Pushes in progress on the player and the enemy.
    pub knockback: [Option<Knockback>; 2],
    /// Each character's last real step, reported again while it is frozen.
    last_steps: [StepReport; 2],
    /// Thrown and shot attacks in flight.
    pub bullets: crate::bullets::Bullets,
    lightning_returned: bool,
    /// Placeholder void depth below the lowest collision, or spawn without collision, past
    /// which Wolf is returned to where he last stood; `None` disables the return.
    pub fall_respawn_depth: Option<f32>,
    fall_materials: std::collections::HashMap<u32, FallMaterial>,
    /// Where Wolf last stood on collision, for the return after a fall into the void.
    last_safe: Option<([f32; 3], f32)>,
    /// The boss brain choosing the enemy's acts with its Lua AI (see [`crate::brain`]);
    /// `None` leaves the Lua AI alone.
    pub boss_brain: Option<crate::brain::BrainDriver>,
    /// Testing aid: the player takes no HP or posture damage (hits, reactions, hit stop and
    /// knockback are unchanged).
    pub god: bool,
    /// Testing aid (`--immortal`): Wolf cannot die (HP stops at 1) but his posture, guard breaks
    /// and HP-dependent recovery work as in the game, so mashing into guards and deflects
    /// can still break him.
    pub immortal: bool,
    /// The fight's map events (phase changes, boss bar, defeat), when the enemy has any
    /// ([`crate::arena::boss_events`]).
    pub events: Option<FightEvents>,
}

/// The state the map events keep for a fight (see [`crate::events`]).
#[derive(Debug, Clone)]
pub struct FightEvents {
    pub scripts: EventScripts,
    /// The boss's entity id in the events.
    pub boss_entity: i32,
    /// NpcName text id of the boss health bar, while it is shown.
    pub boss_bar: Option<i32>,
    /// The small HP bar over the boss's head is shown.
    pub hp_bar: bool,
    /// `HandleBossDefeat` ran.
    pub defeated: bool,
    /// An event command waiting for the boss's script (`env(105, slot)`): (slot, command).
    pub ezstate: Option<(u8, i32)>,
}

impl FightEvents {
    pub fn load(cache: &Path, spec: &BossEvents) -> Result<Self, String> {
        let mut scripts = EventScripts::load(cache, spec.map, spec.events)?;
        scripts.flags.extend(spec.flags.iter().copied());
        Ok(Self {
            scripts,
            boss_entity: spec.boss_entity,
            boss_bar: None,
            hp_bar: true,
            defeated: false,
            ezstate: None,
        })
    }

    /// The fight reloads: the events start over, their flags stay.
    fn restart(&mut self) {
        self.scripts.restart();
        self.boss_bar = None;
        self.hp_bar = true;
        self.defeated = false;
        self.ezstate = None;
    }
}

/// The env id of the event command a character's script reads (`env(105, slot)`, "command id
/// from the event" in the community env list).
const ENV_EVENT_COMMAND: i32 = 105;

/// The env id the NPC script asks "in a throw?" (`env(206)`): its `ThrowCommonFunction` accepts
/// event commands only then and skips damage and death. Ours: true for the enemy while a
/// deathblow holds him or he still plays its defender clip (inferred; not traced in the exe).
const ENV_IN_THROW: i32 = 206;

/// The env id of the AI's endure-cancel action (`env(123)`, "AI endure cancel type" in the
/// community env list): an `EndureAttack` goal's request, which the NPC script's
/// `ExecAIAction` takes before every other cancel.
const ENV_AI_ENDURE_ACTION: i32 = 123;

/// TAE action flag 78, "End If AI Move Queued": an NPC clip's move window.
const AI_MOVE_FLAG: i32 = 78;

/// How long an effect with no duration (`effectEndurance` 0, applied once) stays visible to the
/// scripts and the AI's SpEffect observers, in steps (ours).
const INSTANT_EFFECT_STEPS: f32 = 2.0;

/// The world as the map events see it: Wolf is entity 10000, the enemy the boss's entity id.
struct EventView<'a> {
    duel: &'a Duel,
    boss: i32,
}

impl EventView<'_> {
    fn who(&self, entity: i32) -> Option<usize> {
        match entity {
            PLAYER_ENTITY => Some(0),
            e if e == self.boss => Some(1),
            _ => None,
        }
    }

    fn character(&self, i: usize) -> &Character {
        if i == 0 {
            &self.duel.player
        } else {
            &self.duel.enemy
        }
    }
}

impl EventHost for EventView<'_> {
    fn cutscene_finished(&self, id: i32) -> bool {
        self.duel
            .cinematic
            .as_ref()
            .is_none_or(|c| c.id != id || c.applied)
    }
    fn has_sp_effect(&self, entity: i32, id: i32) -> Option<bool> {
        let i = self.who(entity)?;
        Some(
            self.character(i).tae_frame().sp_effects.contains(&id)
                || self.duel.rules.fighters[i].resident().contains(&id),
        )
    }

    fn has_event_message(&self, entity: i32, id: i32) -> Option<bool> {
        let i = self.who(entity)?;
        Some(self.character(i).tae_frame().ezstate_requests.contains(&id))
    }

    fn is_dead(&self, entity: i32) -> Option<bool> {
        Some(self.duel.rules.fighters[self.who(entity)?].dead())
    }

    fn hp(&self, entity: i32) -> Option<i32> {
        Some(self.duel.rules.fighters[self.who(entity)?].vitality.hp)
    }
    fn health_bars(&self, entity: i32) -> Option<i32> {
        Some(
            self.duel.rules.fighters[self.who(entity)?]
                .vitality
                .deathblows_left
                .max(0),
        )
    }
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
            camera_director: crate::camera::CameraDirector::load(cache),
            player,
            enemy,
            brain: SimpleBrain::default(),
            ai,
            fighters,
            detector: HitDetector::default(),
            rules,
            enemy_active: true,
            deathblow: None,
            enemy_throw: None,
            mikiri: None,
            force_act: None,
            respawn_in: None,
            throws,
            cache: cache.to_path_buf(),
            initial_spec: spec.clone(),
            pending_handover: None,
            cinematic: None,
            spec,
            defender_dying: 0,
            spawn: None,
            lock_button: false,
            attack_button: false,
            enemy_damaged: false,
            last_throw_def_clip: None,
            swing_hit: None,
            hit_stop: [HitStop::default(); 2],
            knockback: [None; 2],
            last_steps: Default::default(),
            bullets: Default::default(),
            lightning_returned: false,
            fall_respawn_depth: None,
            fall_materials: load_fall_materials(cache),
            last_safe: None,
            boss_brain: None,
            god: false,
            immortal: false,
            events: None,
        })
        .map(|mut duel| {
            duel.events = crate::arena::boss_events(&duel.spec).and_then(|spec| {
                FightEvents::load(cache, spec)
                    .map_err(|e| eprintln!("{}: map events not loaded: {e}", duel.spec.chr))
                    .ok()
            });
            duel
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
        if crate::brain::BossActions::for_chr(&self.spec.chr).is_none() && !brain.plans() && !log {
            return Ok(None);
        }
        let actions = crate::brain::BossActions::for_chr(&self.spec.chr)
            .ok_or_else(|| format!("no boss brain action list for {}", self.spec.chr))?;
        if self.ai.is_none() {
            return Err(format!(
                "{} has no Lua AI to pair the brain with",
                self.spec.chr
            ));
        }
        let mut driver = crate::brain::BrainDriver::new(brain, actions);
        driver.move_sheet = crate::brain::moves::build(self, actions);
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
            if c.kind == crate::character::ControlKind::Npc
                && sp_effects.iter().any(|id| {
                    c.tae
                        .sp_effects
                        .get(id)
                        .is_some_and(|info| (270..=274).contains(&info.state_info))
                })
            {
                let group = c.behavior.runtime.offsets.offset(15) / 100;
                sp_effects.retain(|id| {
                    c.tae.sp_effects.get(id).is_none_or(|info| {
                        !(270..=274).contains(&info.state_info)
                            || info.state_info as u32 == 270 + group
                    })
                });
                if let Some((&id, _)) = c
                    .tae
                    .sp_effects
                    .iter()
                    .find(|(_, info)| info.state_info as u32 == 270 + group)
                    && !sp_effects.contains(&id)
                {
                    sp_effects.push(id);
                }
            }
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
            current_anim_time: main.map_or(0.0, |c| c.time),
            combo_window: COMBO_FLAGS.iter().any(|&f| frame.flag(f))
                || main.is_some_and(|c| c.at_end()),
            home: self
                .spawn
                .map_or(self.enemy.body.position, |s| s[1].position),
            parry_timing: self.parry_timing(),
            target_attacking: self
                .player
                .behavior
                .runtime
                .main_state_path()
                .iter()
                .any(|s| s.contains("Attack")),
            damaged: self.enemy_damaged,
        }
    }

    fn load_characters(
        cache: &Path,
        distance: f32,
        spec: &EnemySpec,
    ) -> Result<(Character, Character), LoadError> {
        let mut player = Character::load_from_cache(cache)?;
        let registry = crate::registry::Registry::load(cache).ok();
        let anim_chr = registry.as_ref().map_or_else(
            || spec.chr.clone(),
            |r| r.anim_chr(&spec.chr, spec.npc_param),
        );
        let mut enemy = Character::load_npc_with_anim(cache, &spec.chr, &anim_chr)?;
        if let Some(row) = registry.as_ref().and_then(|r| r.npc_row(spec.npc_param)) {
            for i in 0..32 {
                if let Some(id) = row[format!("spEffectID{i}")]
                    .as_i64()
                    .and_then(|id| enemy.tae.sp_effects.get(&(id as i32)))
                    .filter(|info| (270..=274).contains(&info.state_info))
                {
                    enemy
                        .behavior
                        .runtime
                        .offsets
                        .by_type
                        .insert(15, (id.state_info as u32 - 270) * 100);
                }
            }
        }
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
        if self.spec != self.initial_spec {
            let spot = self.spawn.map_or(placement(&self.enemy), |s| s[1]);
            self.switch_enemy(self.initial_spec.clone(), spot)?;
        }
        self.pending_handover = None;
        self.cinematic = None;
        let (mut player, mut enemy) = Self::load_characters(&self.cache, 4.0, &self.spec)?;
        std::mem::swap(&mut player.ground, &mut self.player.ground);
        player.docking.edges = std::mem::take(&mut self.player.docking.edges);
        player.wire.points = std::mem::take(&mut self.player.wire.points);
        player.wire.search = self.player.wire.search.clone();
        player.wire.variations = self.player.wire.variations.clone();
        player.wire.offsets = self.player.wire.offsets.clone();
        player.wire.collision = self.player.wire.collision.take();
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
        self.enemy_throw = None;
        self.mikiri = None;
        self.respawn_in = None;
        self.defender_dying = 0;
        self.last_throw_def_clip = None;
        self.swing_hit = None;
        self.bullets.clear();
        self.lightning_returned = false;
        if let Some(ev) = self.events.as_mut() {
            ev.restart();
        }
        Ok(())
    }

    /// A map event hands the single active opponent slot to a separately placed character.
    /// Wolf, his health/status, the arena and the original reset point survive the hand-over.
    fn switch_enemy(&mut self, spec: EnemySpec, spot: Placement) -> Result<(), LoadError> {
        let mut next = Self::load_with(&self.cache, 6.0, spec)?;
        std::mem::swap(&mut next.enemy.ground, &mut self.enemy.ground);
        next.enemy.body.position = spot.position;
        next.enemy.body.yaw = spot.yaw;
        next.rules.fighters[0] = self.rules.fighters[0].clone();
        let flags = self.events.as_ref().map(|e| e.scripts.flags.clone());
        if let Some(events) = next.events.as_mut()
            && let Some(flags) = flags
        {
            events.scripts.flags.extend(flags);
        }
        self.enemy = next.enemy;
        self.last_steps[1] = StepReport {
            position: self.enemy.body.position,
            yaw: self.enemy.body.yaw,
            grounded: self.enemy.body.grounded,
            ..Default::default()
        };
        let [_, enemy] = next.fighters;
        self.fighters[1] = enemy;
        self.rules = next.rules;
        self.ai = if self.ai.is_some() { next.ai } else { None };
        self.spec = next.spec;
        self.events = next.events;
        self.throws = next.throws;
        self.enemy_active = true;
        self.last_throw_def_clip = None;
        self.enemy_damaged = false;
        self.detector = HitDetector::default();
        self.knockback[1] = None;
        self.hit_stop[1] = HitStop::default();
        self.bullets.clear();
        self.respawn_in = None;
        self.lightning_returned = false;
        self.defender_dying = 0;
        self.boss_brain = None;
        Ok(())
    }

    /// Lock on to the enemy if it is within range and roughly in front of the camera.
    fn update_lock(&mut self, input: &InputFrame) {
        let pressed = input.buttons.lock_on && !self.lock_button;
        self.lock_button = input.buttons.lock_on;
        let enemy_alive = !self.rules.fighters[1].dead()
            && !self.deathblow.as_ref().is_some_and(|d| d.lethal)
            && self.defender_dying == 0;
        // SpEffect stateInfo 350 "cannot be locked on" (the Owl's smoke screen adds 200602 for
        // 1 s) drops the lock and refuses a new one (that it releases a held lock is inferred).
        let unlockable = self.enemy.tae_frame().sp_effects.iter().any(|id| {
            self.enemy
                .tae
                .sp_effects
                .get(id)
                .is_some_and(|e| e.state_info == STATE_INFO_NO_LOCK)
        });
        if unlockable || !enemy_alive {
            self.release_lock();
            return;
        }
        if !pressed {
            // Follow the target while locked on.
            if self.player.lock_target.is_some() {
                self.player.lock_target = enemy_alive.then_some(self.enemy.body.position);
            }
            return;
        }
        if self.player.lock_target.is_some() {
            self.release_lock();
            return;
        }
        let p = self.player.body.position;
        let e = self.enemy.body.position;
        let dist = ((e[0] - p[0]).powi(2) + (e[2] - p[2]).powi(2)).sqrt();
        if dist <= self.camera.lock_range && enemy_alive {
            self.player.lock_target = Some(e);
        }
    }

    /// Drop the target and its strafe selector together. The HKS move loop only seeds
    /// MoveDirection when movement starts, so a held stick otherwise keeps the dead-target
    /// strafe clip while the body starts steering freely.
    fn release_lock(&mut self) {
        if self.player.lock_target.take().is_some() {
            self.player.release_move_lock();
        }
        self.player.behavior.env.locked_on = false;
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
        // Owl's TAE keeps the heat-up judgment effect on throughout the heat-up and kneel.
        // HP 1 here is the interrupted first deathblow's HP floor, not another opening.
        // Leave attacks enabled: his Lua ParryTiming interrupt uses this same effect to
        // request escape 20007, whose TAE restores HP and posture (docs/OWL.md, Phase 2).
        if self.enemy.tae_frame().sp_effects.contains(&OWL_HEAT_UP) {
            return None;
        }
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
        self.release_lock();
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
        let pair_start = step.attacker_clip.is_some() && db.phase == Phase::Body;
        let alignment = pair_start.then(|| {
            let p = placement(&self.player);
            let e = placement(&self.enemy);
            let mut aligned = [p.position, e.position];
            match db.absorbed {
                Absorbed::Defender => aligned[1] = db.sorb.placement(&p).position,
                Absorbed::Attacker => aligned[0] = db.sorb.inverse().placement(&e).position,
            }
            // Sorb helper Y is a model offset; the existing floor placement stays authoritative.
            aligned[0][1] = p.position[1];
            aligned[1][1] = e.position[1];
            aligned
        });
        let path = pair_start.then(|| throw_body_path(db, &self.player, &self.enemy));
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
        if let Some(aligned) = alignment {
            let starts = [self.player.body.position, self.enemy.body.position];
            let mut checks = vec![aligned, starts];
            // Include the fully absorbed endpoint even if the strike occurs before its end.
            checks.extend(path.unwrap_or_default());
            self.reserve_pair_path(&checks);
        }
        if let Some(a) = step.absorb {
            self.absorb(&a);
        }
    }

    fn reserve_pair(&mut self, mut aligned: [[f32; 3]; 2]) {
        let starts = [self.player.body.position, self.enemy.body.position];
        for i in 0..2 {
            aligned[i][1] = starts[i][1];
        }
        self.reserve_pair_path(&[starts, aligned]);
    }

    fn reserve_pair_path(&mut self, path: &[[[f32; 3]; 2]]) {
        if let Some(shift) = crate::collision::throw_pair_path_shift(
            [self.player.ground.as_ref(), self.enemy.ground.as_ref()],
            path,
        ) {
            if shift[0].hypot(shift[1]) > 0.01 {
                eprintln!(
                    "throw pair wall clearance: x={:.3} z={:.3}",
                    shift[0], shift[1]
                );
            }
            for c in [&mut self.player, &mut self.enemy] {
                c.body.position[0] += shift[0];
                c.body.position[2] += shift[1];
            }
        } else {
            eprintln!("throw pair clearance exhausted; retaining authored alignment");
        }
    }

    /// Applies authored absorption without independent wall pushes after reserving pair space.
    fn absorb(&mut self, a: &AbsorbStep) {
        // Wolf is the deathblow's attacker.
        self.slide_pair(a.who == Absorbed::Attacker, a);
    }

    /// [`Self::absorb`] for either character: `player_moves` says whether Wolf is the absorbed one.
    fn slide_pair(&mut self, player_moves: bool, a: &AbsorbStep) {
        let moved = if player_moves {
            &mut self.player
        } else {
            &mut self.enemy
        };
        moved.body.position[0] += a.delta[0];
        moved.body.position[2] += a.delta[2];
        moved.body.yaw = angle_diff(moved.body.yaw + a.yaw, 0.0);
    }

    /// Whether an enemy throw holds Wolf.
    fn throw_held(&self) -> bool {
        self.mikiri.is_some()
            || self
                .enemy_throw
                .as_ref()
                .is_some_and(|t| t.phase == GrabPhase::Held)
    }

    fn try_mikiri(&mut self, hit: &HitEvent) -> Option<(i32, i32)> {
        if hit.attacker != 1
            || hit.defender != 0
            || !self.player.tae_frame().flag(crate::perilous::FLAG_MIKIRI)
            || !(hit.attack.atk_type == 2
                && hit.attack.perilous() == Some(crate::hits::Perilous::Thrust)
                || crate::perilous::counter_dummy(&hit.attack)
                    && crate::perilous::upcoming_kind(&self.enemy, &self.fighters[1])
                        == Some(crate::hits::Perilous::Thrust))
        {
            return None;
        }
        let throws = self.throws.as_ref()?;
        let damage = hit.attack.mikiri_posture;
        let forward = placement(&self.player).forward();
        let to_enemy = [
            self.enemy.body.position[0] - self.player.body.position[0],
            self.enemy.body.position[2] - self.player.body.position[2],
        ];
        if forward[0] * to_enemy[0] + forward[1] * to_enemy[1] <= 0.0 {
            return None;
        }
        let broken = self.rules.fighters[1].posture.project(-damage, None) < 1;
        let row = throws
            .table
            .player_row(self.spec.throw_chr(), 0, if broken { 120 } else { 190 })?
            .clone();
        self.player.clip_duration(&row.attacker_anim())?;
        let sorb = DummyFrame::from_flver(&throws.model, row.atk_sorb_dummy)?;
        self.player.body.yaw = self.player.yaw_to(self.enemy.body.position);
        self.enemy.body.yaw = self.enemy.yaw_to(self.player.body.position);
        let slide = crate::deathblow::AbsorbSlide::new(
            Absorbed::Defender,
            &sorb,
            &placement(&self.player),
            &placement(&self.enemy),
            row.interpolation_time,
        );
        self.reserve_pair([
            self.player.body.position,
            sorb.placement(&placement(&self.player)).position,
        ]);
        self.rules.fighters[1].posture.take(damage, None);
        self.player
            .behavior
            .runtime
            .offsets
            .by_type
            .insert(THROW_ATK_OFFSET_TYPE, row.atk_anim_offset as u32);
        self.player
            .behavior
            .runtime
            .fire_event(&format!("W_ThrowAtk{}", row.atk_anim));
        self.enemy
            .behavior
            .runtime
            .offsets
            .by_type
            .insert(THROW_DEF_OFFSET_TYPE, row.def_anim_offset as u32);
        self.enemy
            .behavior
            .runtime
            .fire_event(&format!("W_ThrowDef{}", row.def_anim));
        self.last_throw_def_clip = row.defender_anim();
        self.knockback = [None, None];
        self.player.pending_damage = None;
        self.enemy.pending_damage = None;
        let result = (row.id, damage);
        self.mikiri = Some(crate::perilous::Mikiri {
            row,
            slide,
            started: false,
        });
        Some(result)
    }

    fn step_mikiri(&mut self, dt: f32) {
        let Some(mut pair) = self.mikiri.take() else {
            return;
        };
        let playing = self
            .player
            .behavior
            .runtime
            .main_clip()
            .is_some_and(|c| c.animation == pair.row.attacker_anim());
        if playing {
            pair.started = true;
        }
        if pair.started && !playing {
            return;
        }
        if let Some(a) = pair.slide.advance(dt) {
            self.absorb(&a);
        }
        self.mikiri = Some(pair);
    }

    /// [`Self::force_act`]: asks for the act whenever the enemy is not already in it or in a
    /// throw, facing Wolf.
    fn force_enemy_act(&mut self, enemy_alive: bool) {
        let Some(act) = self.force_act else {
            return;
        };
        let clip = format!("a000_{act:06}");
        let playing = self
            .enemy
            .behavior
            .runtime
            .main_clip()
            .is_some_and(|c| c.animation == clip);
        if !enemy_alive || playing || self.throw_held() || self.deathblow.is_some() {
            return;
        }
        self.enemy.npc.action = act;
        self.enemy.npc.combo = false;
        self.enemy.npc.face_yaw = Some(self.enemy.yaw_to(self.player.body.position));
    }

    /// Starts the enemy's throw on Wolf from a throw-start contact (AtkParam `throwFlag` 1):
    /// picks the ThrowParam row, checks its range, fires the throw events into both graphs and
    /// begins the absorb. Returns the row. A contact the row's range refuses does nothing
    /// (inferred: the engine's handling of a failed throw check is not traced).
    fn try_enemy_throw(&mut self, hit: &HitEvent) -> Option<i32> {
        if self.enemy_throw.is_some()
            || self.deathblow.is_some()
            || self.rules.fighters.iter().any(|f| f.dead())
        {
            return None;
        }
        let throws = self.throws.as_ref()?;
        let row = throws
            .table
            .npc_row(self.spec.throw_chr(), hit.attack.throw_type_id)?
            .clone();
        let (attacker, defender) = (placement(&self.enemy), placement(&self.player));
        if !row.in_range(&attacker, &defender) {
            return None;
        }
        let sorb = if row.atk_sorb_dummy > 0 {
            DummyFrame::from_flver(throws.enemy_model.as_ref()?, row.atk_sorb_dummy)?
        } else {
            // Only Wolf's dummy is named: the enemy stands on it.
            DummyFrame::from_flver(&throws.model, row.def_sorb_dummy)?.inverse()
        };
        let attacker = Placement {
            yaw: if row.turn_attacker {
                attacker.yaw_towards(defender.position)
            } else {
                attacker.yaw
            },
            ..attacker
        };
        let throw = EnemyThrow::begin(
            &row,
            &self.enemy.tae,
            &self.player.tae,
            sorb,
            &attacker,
            &defender,
        )?;
        self.enemy.body.yaw = attacker.yaw;
        let aligned = if row.atk_sorb_dummy <= 0 && row.def_sorb_dummy > 0 {
            [
                defender.position,
                sorb.inverse().placement(&defender).position,
            ]
        } else {
            [sorb.placement(&attacker).position, attacker.position]
        };
        self.reserve_pair(aligned);
        let rt = &mut self.enemy.behavior.runtime;
        rt.offsets
            .by_type
            .insert(THROW_DEF_OFFSET_TYPE, row.atk_anim_offset as u32);
        rt.fire_event(&format!("W_ThrowAtk{}", row.atk_anim));
        self.player
            .behavior
            .runtime
            .fire_event(&format!("W_ThrowDef{}", row.def_anim));
        // The throw takes over both bodies, as a deathblow does.
        self.knockback = [None, None];
        self.enemy_throw = Some(throw);
        Some(row.id)
    }

    /// Advances the enemy throw after both characters moved this step: the absorb, the damage at
    /// the enemy clip's `ThrowAttackBehavior` events, and the death branch.
    fn step_enemy_throw(&mut self, dt: f32, report: &mut GrabReport) {
        let Some(mut throw) = self.enemy_throw.take() else {
            return;
        };
        let clip = |c: &Character| {
            c.behavior
                .runtime
                .main_clip()
                .map(|c| (c.animation.clone(), c.time))
        };
        let (enemy_clip, wolf_clip) = (clip(&self.enemy), clip(&self.player));
        let step = throw.step(
            dt,
            enemy_clip.as_ref().map(|(c, t)| (c.as_str(), *t)),
            wolf_clip.as_ref().map(|(c, t)| (c.as_str(), *t)),
        );
        if let Some(a) = step.absorb {
            // Wolf is the throw's defender.
            self.slide_pair(a.who == Absorbed::Defender, &a);
        }
        for judge in step.hits {
            let Some(attack) = self.fighters[1].attacks.get(&judge).cloned() else {
                continue;
            };
            let (p, e) = (self.player.body.position, self.enemy.body.position);
            let (dx, dz) = (p[0] - e[0], p[2] - e[2]);
            let len = dx.hypot(dz).max(1e-4);
            let event = HitEvent {
                attacker: 1,
                defender: 0,
                judge,
                attack,
                position: p,
                direction: [dx / len, dz / len],
            };
            // HP and posture as a direct hit of the throw's row (`throwFlag` 2); the thrown clip
            // already shows the impact, so no reaction, knockback or hit stop.
            let result = self.rules.resolve(
                &event,
                [&self.player, &self.enemy],
                [&self.fighters[0], &self.fighters[1]],
            );
            report
                .hits
                .push((judge, result.hp_damage, result.posture_damage));
            throw.lethal |= self.rules.fighters[0].dead();
        }
        if step.death {
            self.player
                .behavior
                .runtime
                .fire_event(&format!("W_ThrowDefDeathStart{}", throw.row.def_anim));
        }
        if throw.phase != GrabPhase::Done {
            self.enemy_throw = Some(throw);
        }
    }

    /// Passes HP and posture to both scripts (env 1000, 1001, 2010). While a deathblow holds the
    /// enemy its script still sees it alive, so the throw animations (with their own death
    /// branch) play out instead of the script's ordinary death.
    /// env 2004 ("swing is hitting", `docs/HKS-INTERFACE.md`) for Wolf's script: true while the
    /// clip in which one of his attacks connected still plays. The script reads it only together
    /// with SpEffect reference 204 (the air kick's window for kicking off the enemy into
    /// `AirKickEnemyJumpStart`, as FwLo's RustSekiro `BEH_R_ENEMY_JUMP`) or a combat art's jump
    /// reference; that it stays true for the rest of the clip is ours.
    fn publish_swing_hit(&mut self) {
        let playing = self
            .player
            .behavior
            .runtime
            .main_clip()
            .map(|c| c.animation.clone());
        if self.swing_hit.is_some() && self.swing_hit != playing {
            self.swing_hit = None;
        }
        let env = &mut self.player.behavior.env;
        env.overrides.retain(|(id, _, _)| *id != ENV_SWING_HIT);
        if self.swing_hit.is_some() {
            env.overrides.push((ENV_SWING_HIT, None, 1.0));
        }
    }

    /// Notes Wolf's landed attacks of this step for env 2004, and returns whether one was an
    /// anti-ground hit (the air kick) on an enemy whose active SpEffects say "jump kick
    /// effective" (a perilous sweep, [`crate::perilous::STATE_INFO_JUMP_KICK_EFFECTIVE`]).
    fn note_player_hits(&mut self, hits: &[(HitEvent, HitResult)]) -> bool {
        let mut counter = false;
        for (hit, _) in hits.iter().filter(|(h, _)| h.attacker == 0) {
            self.swing_hit = self
                .player
                .behavior
                .runtime
                .main_clip()
                .map(|c| c.animation.clone());
            let anti_ground = hit
                .attack
                .profile
                .as_ref()
                .is_some_and(|p| p.stamina_attribute.0 == crate::perilous::STAMINA_ANTI_GROUND);
            counter |= anti_ground
                && self
                    .rules
                    .active_effects(hit.defender, self.chr(hit.defender))
                    .iter()
                    .any(|e| e.state_info == crate::perilous::STATE_INFO_JUMP_KICK_EFFECTIVE);
        }
        counter
    }

    /// Character `i`: 0 Wolf, 1 the enemy.
    fn chr(&self, i: usize) -> &Character {
        if i == 0 { &self.player } else { &self.enemy }
    }

    /// World position of dummy poly `id` of character `i` (0 Wolf, 1 the enemy) in its pose as
    /// of the last step: the camera's look-at target (Wolf's 142 in deathblows) and the kanji's
    /// spot (Wolf's 220).
    pub fn dummy_position(&self, i: usize, id: i16) -> Option<[f32; 3]> {
        let c = self.chr(i);
        self.fighters[i]
            .rig
            .positions(id, &c.bone_source_matrices(), c.source_to_world())
            .first()
            .map(|p| p.to_array())
    }

    fn publish_vitals(&mut self) {
        // The script sees HP 0 one step after the death event, once its graph is in the death state.
        let held = self.deathblow.is_some() && self.defender_dying < 2;
        // Wolf's script sees him alive until a lethal throw has his graph in its death state, so
        // the thrown clip's own death branch plays instead of the ordinary death.
        let thrown = self.enemy_throw.as_ref().is_some_and(|t| {
            t.phase == GrabPhase::Held
                && (!t.branched || !self.player.state().starts_with("ThrowDefDeath"))
        });
        for (i, (c, f)) in [&mut self.player, &mut self.enemy]
            .into_iter()
            .zip(&self.rules.fighters)
            .enumerate()
        {
            let hp = f.vitality.hp as f32;
            c.behavior.env.hp = if (held && i == 1) || (thrown && i == 0) {
                hp.max(1.0)
            } else {
                hp
            };
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

    /// Leaves a bullet's SpEffects on character `i`: build-up and timed effects in its status
    /// ([`crate::status`]), the timed ones also in its TAE runtime so its scripts and AI see them.
    fn apply_status(&mut self, i: usize, ids: &[i32]) {
        let applied = self.rules.apply_hit_effects(i, ids);
        let c = if i == 0 {
            &mut self.player
        } else {
            &mut self.enemy
        };
        for (id, seconds) in applied.timed {
            c.tae_runtime.add_timed(id, seconds);
        }
        // The weak/strong lightning hit effects point at resistance-damage rows.
        // Unlike poison this is a single discharge, not damage over time.
        if ids.contains(&9400) {
            self.discharge_lightning(i, 9420);
        }
        let grounded = if i == 0 {
            self.player.body.grounded
        } else {
            self.enemy.body.grounded
        };
        if grounded && let Some(id) = ids.iter().find(|id| (9490..=9495).contains(*id)) {
            self.discharge_lightning(i, 9430 + (*id - 9490));
        }
    }

    fn discharge_lightning(&mut self, i: usize, damage_effect: i32) {
        let Some(effect) = self.rules.statuses.get(damage_effect).copied() else {
            return;
        };
        let damage = effect.tick_damage(self.rules.fighters[i].vitality.max_hp);
        let f = &mut self.rules.fighters[i];
        f.vitality.set_hp(f.vitality.hp - damage);
        let posture = (f.posture.max as f32 * effect.change_stamina_rate / 100.0
            + effect.change_stamina_point as f32) as i32;
        f.posture.remaining = (f.posture.remaining + posture).clamp(0, f.posture.max);
        f.status.running.retain(|r| !(9490..=9499).contains(&r.id));
        let applied = self.rules.apply_hit_effects(i, &[9445]);
        let c = if i == 0 {
            &mut self.player
        } else {
            &mut self.enemy
        };
        for id in 9490..=9499 {
            c.tae_runtime.clear_timed(id);
        }
        for (id, seconds) in applied.timed {
            c.tae_runtime.add_timed(id, seconds);
        }
        // A one-tick behaviour notification selects the shipped shock reaction.
        c.tae_runtime.add_timed(9450, 2.0 / 60.0);
        c.pending_damage = Some(crate::player::IncomingDamage {
            damage_type: 99999,
            level: 3,
            element: 10,
            ..Default::default()
        });
    }

    fn step_lightning(&mut self) {
        for i in 0..2 {
            let c = if i == 0 { &self.player } else { &self.enemy };
            let released = c
                .tae_frame()
                .sp_effects
                .iter()
                .any(|id| matches!(id, 9505 | 9506 | 9500));
            let charge = self.rules.fighters[i]
                .status
                .running
                .iter()
                .find(|r| (9490..=9495).contains(&r.id))
                .map(|r| r.id);
            if let Some(id) = charge {
                if released {
                    self.rules.fighters[i].status.running.retain(|r| r.id != id);
                    let c = if i == 0 {
                        &mut self.player
                    } else {
                        &mut self.enemy
                    };
                    c.tae_runtime.clear_timed(id);
                } else if c.body.grounded {
                    self.discharge_lightning(i, 9430 + (id - 9490));
                }
            }
        }
    }

    /// Leaves a melee hit's AtkParam SpEffects on character `i`: as [`Self::apply_status`], and
    /// effects with no duration (applied once, such as the Owl's kick marker 3506000 on Wolf)
    /// stay visible for [`INSTANT_EFFECT_STEPS`] steps so the AI's observers see them.
    fn apply_hit_effects(&mut self, i: usize, ids: &[i32], dt: f32) {
        self.apply_status(i, ids);
        for &id in ids {
            let c = if i == 0 { &self.player } else { &self.enemy };
            if c.tae.sp_effects.get(&id).is_some_and(|e| e.duration == 0.0) {
                let c = if i == 0 {
                    &mut self.player
                } else {
                    &mut self.enemy
                };
                c.tae_runtime.add_timed(id, INSTANT_EFFECT_STEPS * dt);
            }
        }
    }

    /// Whether the enemy's health bar is up: always without map events; with them, while the
    /// events show the boss bar or the overhead bar.
    pub fn enemy_bar_shown(&self) -> bool {
        self.events
            .as_ref()
            .is_none_or(|e| e.boss_bar.is_some() || e.hp_bar)
    }

    /// What the enemy's script reads from the engine about events, throws and endure requests,
    /// before it runs: the waiting event command (`env(105, slot)`), "in a throw" (`env(206)`),
    /// the AI's endure-cancel action (`env(123)`), and, in an event animation, the move cancel
    /// and standby states without the clip-end stand-in (see below).
    fn publish_event_envs(&mut self, endure_action: Option<i32>) {
        let in_throw = self.deathblow.is_some() || self.enemy_in_throw_clip();
        let command = self.events.as_ref().and_then(|e| e.ezstate);
        // Event animations (the graph's `Event*` states, such as the Owl's heat-up) end through
        // their own state's end branch (`env(339)`, `W_Event21003` after `a000_020005`); their
        // TAE opens no move window. Our clip-end stand-in for env 2000 / 1105 would hand them to
        // the AI's turn or move at the end instead (ours).
        let frame = self.enemy.tae_frame();
        let event_anim = self.enemy.state().starts_with("Event")
            && frame.restricted
            && !frame.flag(crate::tae::cancel::MOVE)
            && !frame.flag(AI_MOVE_FLAG);
        let env = &mut self.enemy.behavior.env;
        env.overrides.retain(|(id, _, _)| {
            ![
                ENV_IN_THROW,
                273,
                ENV_EVENT_COMMAND,
                ENV_AI_ENDURE_ACTION,
                crate::player::ENV_MOVE_CANCEL,
                crate::player::ENV_STANDBY_STATE,
            ]
            .contains(id)
        });
        if in_throw {
            env.overrides.push((ENV_IN_THROW, None, 1.0));
        }
        if let Some(pair) = &self.mikiri {
            env.overrides.push((273, None, pair.row.def_anim as f32));
        }
        if let Some(a) = endure_action.filter(|&a| a > 0) {
            env.overrides.push((ENV_AI_ENDURE_ACTION, None, a as f32));
        }
        if event_anim {
            env.overrides
                .push((crate::player::ENV_MOVE_CANCEL, None, 0.0));
            env.overrides
                .push((crate::player::ENV_STANDBY_STATE, None, 0.0));
        }
        if let Some((slot, command)) = command {
            env.overrides
                .push((ENV_EVENT_COMMAND, Some(slot as i32), command as f32));
        }
    }

    /// Runs the map events for this step and applies what they ask.
    fn step_events(&mut self, dt: f32) {
        let Some(mut ev) = self.events.take() else {
            return;
        };
        // A waiting command is used up once the script played an event animation for it.
        if ev.ezstate.is_some()
            && self.last_steps[1]
                .behavior
                .events
                .iter()
                .any(|e| e.starts_with("W_Event"))
        {
            ev.ezstate = None;
        }
        let commands = ev.scripts.step(
            dt,
            &EventView {
                duel: self,
                boss: ev.boss_entity,
            },
        );
        let who = |entity: i32| match entity {
            PLAYER_ENTITY => Some(0),
            e if e == ev.boss_entity => Some(1),
            _ => None,
        };
        for c in commands {
            match c {
                EventCommand::Cutscene {
                    id,
                    point,
                    area,
                    block,
                    player,
                } => {
                    let warp = (player == PLAYER_ENTITY && point > 0)
                        .then(|| crate::events::warp_point(&self.cache, area, block, point))
                        .flatten();
                    if point > 0 && warp.is_none() {
                        eprintln!(
                            "cutscene {id}: unresolved MSB warp point {point}; keeping position"
                        );
                    }
                    self.cinematic = Some(crate::events::Cinematic {
                        id,
                        elapsed: 0.0,
                        warp,
                        applied: false,
                    });
                    self.player.input.reset_requests();
                }
                EventCommand::SetSpEffect { entity, id } => {
                    if let Some(i) = who(entity) {
                        let c = if i == 0 {
                            &mut self.player
                        } else {
                            &mut self.enemy
                        };
                        // `effectEndurance` -1 lasts until cleared (200051), 0 is applied once.
                        let seconds = match c.tae.sp_effects.get(&id).map(|e| e.duration) {
                            Some(d) if d < 0.0 => f32::INFINITY,
                            Some(d) if d > 0.0 => d,
                            _ => INSTANT_EFFECT_STEPS * dt,
                        };
                        c.tae_runtime.add_timed(id, seconds);
                    }
                }
                EventCommand::ClearSpEffect { entity, id } => {
                    if let Some(i) = who(entity) {
                        let character = if i == 0 {
                            &mut self.player
                        } else {
                            &mut self.enemy
                        };
                        character.tae_runtime.clear_timed(id);
                    }
                }
                EventCommand::EzStateRequest {
                    entity,
                    command,
                    slot,
                } if who(entity) == Some(1) => ev.ezstate = Some((slot, command)),
                EventCommand::BossBar {
                    shown,
                    entity,
                    name,
                    ..
                } if who(entity) == Some(1) => {
                    ev.boss_bar = shown.then_some(name);
                }
                EventCommand::HpBarDisplay { entity, on } if who(entity) == Some(1) => {
                    ev.hp_bar = on;
                }
                EventCommand::Immortality { entity, on } => {
                    if let Some(i) = who(entity) {
                        self.rules.fighters[i].vitality.cannot_die = on;
                    }
                }
                EventCommand::BossDefeat { entity } if who(entity) == Some(1) => {
                    ev.defeated = true;
                }
                EventCommand::CharacterEnabled { entity, enabled } if who(entity) == Some(1) => {
                    self.enemy_active = enabled;
                }
                EventCommand::CharacterEnabled {
                    entity,
                    enabled: true,
                } if who(entity).is_none() => {
                    self.pending_handover = Some((entity, None));
                }
                EventCommand::BossBar {
                    shown: true,
                    entity,
                    name,
                    ..
                } if self
                    .pending_handover
                    .is_some_and(|(next, _)| next == entity) =>
                {
                    self.pending_handover = Some((entity, Some(name)));
                }
                // Remaining commands require world or cinematic systems outside the duel.
                _ => {}
            }
        }
        self.events = Some(ev);
        if self.deathblow.is_none()
            && let Some((entity, bar)) = self.pending_handover.take()
        {
            let handover = crate::arena::boss_events(&self.spec).and_then(|events| {
                let roster = crate::registry::Registry::load(&self.cache).ok()?;
                let (spec, p) = roster.entity(events.map, entity).ok()?;
                Some((
                    spec,
                    Placement {
                        position: p.position,
                        yaw: p.yaw,
                    },
                ))
            });
            if let Some((spec, mut spot)) = handover {
                if self.cinematic.as_ref().is_some_and(|c| c.warp.is_none()) {
                    spot = placement(&self.enemy);
                }
                match self.switch_enemy(spec, spot) {
                    Ok(()) => {
                        if let Some(events) = self.events.as_mut() {
                            events.boss_bar = bar;
                        }
                    }
                    Err(error) => eprintln!("map hand-over to {entity} failed: {error}"),
                }
            }
        }
    }

    /// Routes lethal landings through DEATH_FALLING and returns Wolf to where he last stood
    /// after a fall into the void.
    fn kill_fallen(&mut self, was_grounded: bool) {
        // Keep the arena's existing NPC boundary reset; material fall death is PC-only.
        if let Some((depth, spawn)) = self.fall_respawn_depth.zip(self.spawn) {
            let start = spawn[1];
            if self.enemy.body.position[1] < start.position[1] - depth {
                self.enemy.body.position = start.position;
                self.enemy.body.yaw = start.yaw;
                self.enemy.body.velocity = [0.0; 3];
                self.enemy.body.grounded = true;
                self.enemy.body.fall_top = start.position[1];
                self.enemy.body.fall_type = 0;
            }
        }
        if self.rules.fighters[0].dead() {
            return;
        }
        let b = &self.player.body;
        let [x, y, z] = b.position;
        // Only a stance on real collision counts as safe ground.
        let on_floor = self
            .player
            .ground
            .floor(x, z, y + 0.5)
            .is_some_and(|floor| (y - floor).abs() < 0.3);
        if was_grounded && b.grounded && on_floor {
            self.last_safe = Some((b.position, b.yaw));
        }
        // Placeholder void boundary until authored kill/return volumes are decoded.
        // The lowest supporting collision takes precedence over spawn to allow deep paths.
        let void_fall = !b.grounded
            && self
                .fall_respawn_depth
                .zip(self.spawn)
                .is_some_and(|(depth, spawn)| {
                    let lowest = self
                        .player
                        .ground
                        .lowest_floor(x, z)
                        .unwrap_or(spawn[0].position[1]);
                    y < lowest - depth
                });
        if void_fall {
            // Sekiro returns Wolf to safe ground after a fall into the void instead of killing
            // him (the script's FALL_DEAD_RETURN route). The HP cost is in the exe and not
            // traced yet, so none is taken here.
            let (position, yaw) = self
                .last_safe
                .or_else(|| self.spawn.map(|s| (s[0].position, s[0].yaw)))
                .unwrap_or((b.position, b.yaw));
            let body = &mut self.player.body;
            body.position = position;
            body.yaw = yaw;
            body.velocity = [0.0; 3];
            body.grounded = true;
            body.fall_top = position[1];
            body.fall_type = 0;
            return;
        }
        if self.god || self.immortal {
            return;
        }
        let lethal_landing = !was_grounded && b.grounded && {
            let material = self.player.ground.material(x, z, y + 0.01).unwrap_or(1);
            self.fall_materials.get(&material).is_some_and(|m| {
                m.disable_fall_damage == 0 && b.last_fall_height >= m.fall_death_dist as f32
            })
        };
        if lethal_landing {
            self.rules.fighters[0].vitality.hp = 0;
            self.player.pending_damage = Some(crate::player::IncomingDamage {
                damage_type: 6,
                ..Default::default()
            });
        }
    }

    pub fn step(&mut self, input: &InputFrame, dt: f32) -> DuelReport {
        // Let an already playing pair finish before beginning a queued cinematic. Otherwise
        // pausing the pair would prevent the map's hand-over from ever becoming eligible.
        if self.deathblow.is_none() && self.cinematic.is_some() {
            let cinematic = self.cinematic.as_mut().unwrap();
            let was_black = cinematic.elapsed >= crate::events::Cinematic::FADE;
            cinematic.elapsed += dt;
            let apply = !cinematic.applied && was_black;
            if apply {
                cinematic.applied = true;
                if let Some(warp) = cinematic.warp {
                    self.player.body.position = warp.position;
                    self.player.body.yaw = warp.yaw;
                    self.player.body.velocity = [0.0; 3];
                    self.player.body.fall_top = warp.position[1];
                    self.player.body.grounded = true;
                }
                self.step_events(dt);
            }
            for c in [&mut self.player, &mut self.enemy] {
                c.tae_runtime.frame.sounds.clear();
                c.tae_runtime.frame.velocity_changes.clear();
            }
            let reports: [StepReport; 2] = std::array::from_fn(|i| {
                let mut r = self.last_steps[i].clone();
                let c = if i == 0 { &self.player } else { &self.enemy };
                r.position = c.body.position;
                r.yaw = c.body.yaw;
                r.advanced = 0.0;
                r.clip_started = false;
                r.requested.clear();
                r.behavior.events.clear();
                r.behavior.hooks.clear();
                r.behavior.script_errors.clear();
                r
            });
            if self
                .cinematic
                .as_ref()
                .is_some_and(|c| c.elapsed >= crate::events::Cinematic::DURATION)
            {
                self.cinematic = None;
                self.player.input = Default::default();
                self.attack_button = false;
                self.lock_button = false;
            }
            let [player, enemy] = reports;
            return DuelReport {
                player,
                enemy,
                ..Default::default()
            };
        }
        let raw_input = *input;
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
        let mut endure_action = None;
        if self.enemy_active && enemy_alive && self.deathblow.is_none() {
            if self.ai.is_some() {
                let world = self.ai_world(dt);
                let mut driver = self.boss_brain.take();
                if let Some(mut ai) = self.ai.take() {
                    if let Some(d) = driver.as_mut() {
                        d.before_think(&mut ai, self.fighters[1].radius);
                    }
                    let max_marks = match &self.rules.fighters[1].kind {
                        crate::fight::FighterKind::Npc(n) => n.deathblows,
                        _ => 0,
                    };
                    ai.state.ninsatsu =
                        (self.rules.fighters[1].vitality.deathblows_left, max_marks);
                    self.enemy.npc = ai.think(world);
                    endure_action = ai.state.request.endure.then_some(self.enemy.npc.action);
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
        self.force_enemy_act(enemy_alive);
        self.publish_event_envs(endure_action);
        self.publish_vitals();
        self.publish_swing_hit();
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
        let was_grounded = self.player.body.grounded;
        let held = self
            .deathblow
            .as_ref()
            .is_some_and(|d| d.phase == Phase::Body)
            || self.throw_held();
        self.player.throw_ignore_walls = held;
        self.enemy.throw_ignore_walls = held;
        let player = self.advance_character(0, &input, dt, dts[0]);
        let enemy = self.advance_character(1, &InputFrame::default(), dt, dts[1]);
        if self.deathblow.is_none() && !self.throw_held() {
            self.push(0, dts[0]);
            self.push(1, dts[1]);
        }
        if self.defender_dying > 0 {
            self.defender_dying = self.defender_dying.saturating_add(1);
        }
        self.step_deathblow(dt, &mut deathblow);
        let mut grab = GrabReport::default();
        self.step_enemy_throw(dt, &mut grab);
        self.step_mikiri(dt);
        if self.deathblow.is_none() && !self.throw_held() && enemy_alive {
            self.separate();
        }
        self.kill_fallen(was_grounded);

        if self.god {
            let f = &mut self.rules.fighters[0];
            f.vitality.cannot_die = true;
            f.posture.no_consume = true;
        }
        if self.immortal {
            self.rules.fighters[0].vitality.cannot_die = true;
        }
        let mut found = self.detector.detect(
            &[&self.player, &self.enemy],
            &[&self.fighters[0], &self.fighters[1]],
        );
        // The lightning capsule ends on weapon dummy 10141, absent from the
        // current rig. A facing beam substitutes for that endpoint until the
        // native weapon-dummy transform is reconstructed. Radius is AtkParam;
        // ten metres of reach is explicitly a stand-in.
        let reversal = self
            .player
            .tae_frame()
            .attacks
            .iter()
            .any(|e| matches!(e.kind, crate::tae::TaeKind::Attack { judge: 281, .. }));
        if !reversal {
            self.lightning_returned = false;
        }
        if reversal && !self.lightning_returned {
            self.lightning_returned = true;
            if let Some(attack) = self.fighters[0].attacks.get(&281).cloned() {
                let start = glam::Vec3::from_array(self.player.body.position) + glam::Vec3::Y;
                let [x, z] = self.player.body.forward();
                let end = start + glam::Vec3::new(x, 0.0, z) * 10.0;
                let feet = glam::Vec3::from_array(self.enemy.body.position);
                let radius = attack.shapes.first().map_or(0.0, |s| s.radius);
                if crate::hits::can_be_hit(self.enemy.tae_frame())
                    && crate::hits::segment_distance(
                        start,
                        end,
                        feet,
                        feet + glam::Vec3::Y * self.fighters[1].height,
                    ) <= radius + self.fighters[1].radius
                {
                    found.retain(|h| !(h.attacker == 0 && h.judge == 281));
                    found.push(HitEvent {
                        attacker: 0,
                        defender: 1,
                        judge: 281,
                        attack,
                        position: feet.to_array(),
                        direction: [x, z],
                    });
                }
            }
        }
        let chars = [&self.player, &self.enemy];
        let fighters = [&self.fighters[0], &self.fighters[1]];
        self.bullets.spawn(&chars, &fighters);
        let bullet_hits = self
            .bullets
            .advance(dt, &chars, &fighters, &*self.player.ground);
        let first_bullet_hit = found.len();
        // Contacts that only carry SpEffects (the poison floor) skip the hit rules (ours).
        let (effect_hits, bullet_hits): (Vec<_>, Vec<_>) =
            bullet_hits.into_iter().partition(|b| b.effects_only);
        let bullet_effects: Vec<Vec<i32>> =
            bullet_hits.iter().map(|b| b.sp_effects.clone()).collect();
        found.extend(bullet_hits.into_iter().map(|b| b.hit));
        let mut hits = Vec::new();
        // Counter contacts win over the damaging blade contact of the same step.
        let mikiri = if self.deathblow.is_none()
            && !self.throw_held()
            && !self.rules.fighters.iter().any(|f| f.dead())
        {
            found
                .iter()
                .take(first_bullet_hit)
                .find_map(|hit| self.try_mikiri(hit))
        } else {
            None
        };
        self.enemy_damaged = false;
        for (k, hit) in found.into_iter().enumerate() {
            if self.deathblow.is_some()
                || self.throw_held()
                || self.rules.fighters.iter().any(|f| f.dead())
            {
                break;
            }
            // A throw start grabs instead of hitting (guarding does not stop it).
            if crate::perilous::counter_dummy(&hit.attack) {
                continue;
            }
            if hit.attacker == 1 && hit.attack.throw_flag == 1 {
                grab.started = self.try_enemy_throw(&hit);
                continue;
            }
            let from_bullet = k >= first_bullet_hit;
            let chars = [&self.player, &self.enemy];
            let combatants = [&self.fighters[0], &self.fighters[1]];
            let mut result = if from_bullet {
                self.rules.resolve_bullet(&hit, chars, combatants)
            } else {
                self.rules.resolve(&hit, chars, combatants)
            };
            let after_contact = (
                self.rules.fighters[hit.defender].vitality.hp,
                self.rules.fighters[hit.defender].posture.remaining,
            );
            let (att, def) = if hit.attacker == 0 {
                (&mut self.player, &mut self.enemy)
            } else {
                (&mut self.enemy, &mut self.player)
            };
            def.pending_damage = Some(result.defender);
            // The AtkParam's SpEffects stay on a landed melee hit's defender (bullets carry
            // their own, below).
            let lightning = hit
                .attack
                .profile
                .as_ref()
                .is_some_and(|p| matches!(p.sp_attribute, 6 | 10));
            let melee_effects = if !from_bullet && (!result.guarded || lightning) {
                hit.attack.sp_effects.clone()
            } else {
                Vec::new()
            };
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
            // A bullet's SpEffects stay on a guarded hit's guard (inferred).
            if from_bullet && (!result.guarded || lightning) {
                self.apply_status(hit.defender, &bullet_effects[k - first_bullet_hit]);
            }
            if !melee_effects.is_empty() {
                self.apply_hit_effects(hit.defender, &melee_effects, dt);
            }
            result.hp_damage += after_contact.0 - self.rules.fighters[hit.defender].vitality.hp;
            result.posture_damage +=
                after_contact.1 - self.rules.fighters[hit.defender].posture.remaining;
            hits.push((hit, result));
        }
        let mut perilous = None;
        for b in effect_hits {
            let kanji = b.sp_effects.iter().any(|id| {
                self.player
                    .tae
                    .sp_effects
                    .get(id)
                    .is_some_and(|e| e.vfx == crate::perilous::KANJI_VFX)
            });
            if b.hit.defender == 0 && kanji {
                let a = b.hit.attacker;
                perilous = Some(crate::perilous::PerilousWarning {
                    attacker: a,
                    judge: b.hit.judge,
                    kind: crate::perilous::upcoming_kind(self.chr(a), &self.fighters[a]),
                });
            }
            if !self.rules.fighters.iter().any(|f| f.dead()) {
                self.apply_status(b.hit.defender, &b.sp_effects);
            }
        }
        self.step_lightning();
        let jump_counter = self.note_player_hits(&hits);
        self.rules.tick_status(0, dt);
        self.rules.tick_status(1, dt);
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
        self.step_events(dt);
        if self.rules.fighters[1].dead() {
            self.release_lock();
        }
        let report = DuelReport {
            locked_on: self.player.lock_target.is_some(),
            player,
            enemy,
            hits,
            deathblow,
            grab,
            respawned,
            perilous,
            jump_counter,
            mikiri,
        };
        let clip = self
            .player
            .behavior
            .runtime
            .main_clip()
            .map(|c| (c.animation.clone(), c.time));
        self.camera_director.update(
            clip.as_ref().map(|(a, t)| (a.as_str(), *t)),
            &self.player.tae,
            dt,
        );
        if let Some(mut d) = self.boss_brain.take() {
            d.after_step(self, &report, &raw_input);
            self.boss_brain = Some(d);
        }
        report
    }
}

/// Predicts the authored capsule paths through the paired clip, including its absorption.
/// Uses the same reference-frame delta integration as Character, without advancing scripts.
/// Floors and airborne velocity events are not predicted by this probe.
fn throw_body_path(db: &Deathblow, player: &Character, enemy: &Character) -> Vec<[[f32; 3]; 2]> {
    use crate::body::{Body, BodyInput, FlatGround};
    let placements = [placement(player), placement(enemy)];
    let clips = [
        player.clips.clip(&db.body.attacker_anim()),
        db.body
            .defender_anim()
            .and_then(|name| enemy.clips.clip(&name)),
    ];
    let mut bodies = placements.map(|p| Body {
        position: p.position,
        yaw: p.yaw,
        ..Body::default()
    });
    let mut slide = crate::deathblow::AbsorbSlide::new(
        db.absorbed,
        &db.sorb,
        &placements[0],
        &placements[1],
        db.body.interpolation_time,
    );
    let mut path = vec![bodies.clone().map(|b| b.position)];
    let end = db.body_duration;
    let mut time = 0.0;
    while time < end {
        let dt = (end - time).min(1.0 / 60.0);
        for i in 0..2 {
            let motion = clips[i]
                .as_ref()
                .and_then(|c| c.root_motion.as_ref())
                .map_or([0.0; 4], |rm| {
                    let a = rm.sample(time);
                    let b = rm.sample(time + dt);
                    let (sin, cos) = (-a[3]).sin_cos();
                    let (dx, dz) = (b[0] - a[0], b[2] - a[2]);
                    [
                        cos * dx + sin * dz,
                        b[1] - a[1],
                        -sin * dx + cos * dz,
                        b[3] - a[3],
                    ]
                });
            bodies[i].step(
                &BodyInput {
                    root_motion: motion,
                    ignore_walls: true,
                    ..Default::default()
                },
                dt,
                &FlatGround {
                    height: placements[i].position[1],
                },
            );
        }
        if let Some(a) = slide.advance(dt) {
            let i = if a.who == Absorbed::Attacker { 0 } else { 1 };
            bodies[i].position[0] += a.delta[0];
            bodies[i].position[2] += a.delta[2];
            bodies[i].yaw += a.yaw;
        }
        time += dt;
        path.push(bodies.clone().map(|b| b.position));
    }
    path
}

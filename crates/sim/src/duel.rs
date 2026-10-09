//! Wolf against one enemy: both characters, the enemy's brain, lock-on and hits.

use crate::ai::SimpleBrain;
use crate::character::{Character, StepReport};
use crate::hits::{Combatant, HitDetector, HitEvent, HitResolver, HitResult, SimpleResolver};
use crate::input::InputFrame;
use crate::player::LoadError;
use std::path::Path;

/// Lock-on reach, metres (assumed; the game reads it from LockCamParam and the target's size).
pub const LOCK_RANGE: f32 = 15.0;

/// One step of the duel.
#[derive(Debug, Clone, Default)]
pub struct DuelReport {
    pub player: StepReport,
    pub enemy: StepReport,
    pub hits: Vec<(HitEvent, HitResult)>,
    pub locked_on: bool,
}

pub struct Duel {
    pub player: Character,
    pub enemy: Character,
    pub brain: SimpleBrain,
    /// Hitboxes and attack data: index 0 the player, 1 the enemy.
    pub fighters: [Combatant; 2],
    pub detector: HitDetector,
    pub resolver: Box<dyn HitResolver>,
    /// Remaining HP (raw AtkParam damage until the combat rules are wired), player and enemy.
    pub hp: [i32; 2],
    /// Whether the enemy brain runs (off for scripted player-only tests).
    pub enemy_active: bool,
    lock_button: bool,
}

impl Duel {
    /// Wolf at the origin facing -Z and an Ashina soldier (`c1010`, NpcParam 10100000) `distance`
    /// metres in front of him, facing him.
    pub fn load(cache: &Path, distance: f32) -> Result<Self, LoadError> {
        let mut player = Character::load_from_cache(cache)?;
        let mut enemy = Character::load_npc(cache, "c1010")?;
        enemy.body.position = [0.0, 0.0, -distance];
        enemy.body.yaw = std::f32::consts::PI;
        let dt = 1.0 / 60.0;
        player.settle(dt, 400);
        enemy.settle(dt, 400);
        let fighters = [
            Combatant::player(cache, &player.skeleton),
            Combatant::npc(cache, "c1010", 10_100_000, &enemy.skeleton),
        ];
        Ok(Self {
            player,
            enemy,
            brain: SimpleBrain::default(),
            fighters,
            detector: HitDetector::default(),
            resolver: Box::new(SimpleResolver),
            hp: [800, 195],
            enemy_active: true,
            lock_button: false,
        })
    }

    /// Lock on to the enemy if it is within range and roughly in front of the camera.
    fn update_lock(&mut self, input: &InputFrame) {
        let pressed = input.buttons.lock_on && !self.lock_button;
        self.lock_button = input.buttons.lock_on;
        if !pressed {
            // Follow the target while locked on.
            if self.player.lock_target.is_some() {
                self.player.lock_target = Some(self.enemy.body.position);
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
        if dist <= LOCK_RANGE && self.hp[1] > 0 {
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

    pub fn step(&mut self, input: &InputFrame, dt: f32) -> DuelReport {
        self.update_lock(input);
        if self.enemy_active {
            self.brain
                .think(&mut self.enemy, self.player.body.position, dt);
        } else {
            self.enemy.npc = Default::default();
        }
        let player = self.player.tick(input, dt);
        let enemy = self.enemy.tick(&InputFrame::default(), dt);
        self.separate();
        let found = self.detector.detect(
            &[&self.player, &self.enemy],
            &[&self.fighters[0], &self.fighters[1]],
        );
        let mut hits = Vec::new();
        for hit in found {
            let (a, d) = (hit.attacker, hit.defender);
            let chars = [&self.player, &self.enemy];
            let result = self.resolver.resolve(&hit, chars[a], chars[d]);
            self.hp[d] = (self.hp[d] - result.hp_damage).max(0);
            let mut defender_damage = result.defender;
            if self.hp[d] == 0 && !result.guarded {
                defender_damage.damage_type = 2; // death
            }
            let (att, def) = if a == 0 {
                (&mut self.player, &mut self.enemy)
            } else {
                (&mut self.enemy, &mut self.player)
            };
            def.pending_damage = Some(defender_damage);
            if let Some(back) = result.attacker {
                att.pending_damage = Some(back);
            }
            hits.push((hit, result));
        }
        DuelReport {
            locked_on: self.player.lock_target.is_some(),
            player,
            enemy,
            hits,
        }
    }
}

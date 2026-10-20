//! Resting at a Sculptor's Idol (#128): Wolf kneels, prays, rests and stands up with the game's
//! own behaviour states (StatueAttachStart a000_710310, Loop 710320, Rest 710340, End 710330).
//! The engine only sends their events (`W_StatueAttach*`); c0000.hks moves on from Start and
//! Rest to the Loop, and from End to standing, by itself.
//!
//! Kneeling refills Vitality and makes the idol the checkpoint: the next respawn puts Wolf at
//! it. Rest refills him again and brings the map's foes back. After FwLo's RustSekiro
//! `src/idol.rs` (Fireguy32's idols): there kneeling leaves the foes alone and only Rest
//! revives them.
//! gap: no Healing Gourd or resurrection yet to refill, and the respawn stands Wolf up at the
//! idol instead of kneeling there first.

use super::{Duel, placement};
use crate::deathblow::Placement;

/// Where Wolf is at an idol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdolPhase {
    Kneeling,
    /// The prayer loop: the idol menu is open.
    Praying,
    Resting,
    Leaving,
}

impl Duel {
    /// Kneels Wolf at an idol, `at` his kneeling spot facing it, and `checkpoint` where he comes
    /// back after a death. False (and nothing changes) unless he stands or crouches, alive and
    /// on the ground.
    pub fn kneel_at_idol(&mut self, at: Placement, checkpoint: Placement) -> bool {
        let state = self.player.behavior.runtime.main_state_name();
        let free = state.starts_with("Stand") || state.starts_with("Crouch");
        if !free
            || self.player.behavior.runtime.has_pending_events()
            || !self.player.body.grounded
            || self.rules.fighters[0].dead()
            || self.respawn_in.is_some()
            || self.deathblow.is_some()
            || self.enemy_throw.is_some()
            || !self
                .player
                .behavior
                .runtime
                .fire_event("W_StatueAttachStart")
        {
            return false;
        }
        self.player.body.position = at.position;
        self.player.body.velocity = [0.0; 3];
        self.player.snap_yaw(at.yaw);
        self.player.lock_target = None;
        self.rules.fighters[0].reset();
        let enemy = self.spawn.map_or(placement(&self.enemy), |[_, e]| e);
        self.spawn = Some([checkpoint, enemy]);
        true
    }

    /// Rests at the idol Wolf prays at: Vitality refilled and the map's foes back.
    pub fn rest_at_idol(&mut self) {
        if self.praying()
            && self
                .player
                .behavior
                .runtime
                .fire_event("W_StatueAttachRest")
        {
            self.rules.fighters[0].reset();
            if let Some(mut population) = self.population.take() {
                self.respawn_foes(&mut population);
                self.population = Some(population);
            }
        }
    }

    /// Stands Wolf up from the idol he prays at.
    pub fn leave_idol(&mut self) {
        if self.praying() {
            self.player.behavior.runtime.fire_event("W_StatueAttachEnd");
        }
    }

    /// In the prayer loop with no other event on its way (a second press in the same frame).
    fn praying(&self) -> bool {
        self.idol_phase() == Some(IdolPhase::Praying)
            && !self.player.behavior.runtime.has_pending_events()
    }

    /// Where Wolf is at an idol, `None` when he is not at one.
    pub fn idol_phase(&self) -> Option<IdolPhase> {
        match self.player.behavior.runtime.main_state_name() {
            "StatueAttachStart" => Some(IdolPhase::Kneeling),
            "StatueAttachLoop" => Some(IdolPhase::Praying),
            "StatueAttachRest" => Some(IdolPhase::Resting),
            "StatueAttachEnd" => Some(IdolPhase::Leaving),
            _ => None,
        }
    }
}

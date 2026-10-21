//! Resting at a Sculptor's Idol (#128): Wolf kneels, prays, rests and stands up with the game's
//! own behaviour states (StatueAttachStart a000_710310, Loop 710320, Rest 710340, End 710330).
//! The engine only sends their events (`W_StatueAttach*`); c0000.hks moves on from Start and
//! Rest to the Loop, and from End to standing, by itself.
//!
//! The action button at an idol's kneeling spot kneels Wolf there (unless a map item is nearer,
//! `super::pickup`); while he is at it his own
//! input is held. Kneeling refills Vitality and makes the idol the checkpoint: the next respawn
//! puts Wolf at it. Rest refills him again and brings the map's foes back on the next step.
//! After FwLo's RustSekiro `src/idol.rs` (Fireguy32's idols): there kneeling leaves the foes
//! alone and only Rest revives them.
//! gap: no Healing Gourd or resurrection yet to refill, and the respawn stands Wolf up at the
//! idol instead of kneeling there first.
//!
//! The first kneel at an idol lights it (#152): its BonfireWarpParam `eventflagId` goes into the
//! lit flags, which a respawn keeps, and the screen is told once that it was found
//! ([`Duel::take_idol_found`], the game's "SCULPTOR'S IDOL FOUND" banner).

use std::collections::BTreeSet;

use super::Duel;
use super::pickup::ActionTarget;
use crate::deathblow::Placement;
use crate::input::InputFrame;

/// How near an idol's kneeling spot Wolf can kneel (m across, m up or down).
const REACH: f32 = 1.3;
const REACH_Y: f32 = 0.6;

/// Where Wolf is at an idol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdolPhase {
    Kneeling,
    /// The prayer loop: the idol menu is open.
    Praying,
    Resting,
    Leaving,
}

/// One idol of the play world.
#[derive(Debug, Clone, Copy)]
pub struct IdolSpot {
    /// Where Wolf kneels, facing it.
    pub kneel: Placement,
    /// Where a respawn stands him, facing away.
    pub checkpoint: Placement,
    /// Its BonfireWarpParam `eventflagId`, on once it is lit.
    pub flag: i32,
}

/// The play world's idols and Wolf's place at them.
#[derive(Debug, Default)]
pub(super) struct Idols {
    spots: Vec<IdolSpot>,
    /// The idol Wolf was sent to kneel at, until he stands again.
    at: Option<usize>,
    /// Where he comes back after a death, once he knelt at an idol.
    pub(super) checkpoint: Option<Placement>,
    action_held: bool,
    /// Rest was taken: the foes come back on the next population step.
    pub(super) revive: bool,
    /// The lit idols' flags.
    lit: BTreeSet<i32>,
    /// The idol lit last, until the screen takes it.
    found: Option<usize>,
}

impl Duel {
    pub fn set_idols(&mut self, spots: Vec<IdolSpot>) {
        self.idols.spots = spots;
    }

    /// The idol whose kneeling spot Wolf is at, if he can kneel now (the "Rest" guide).
    /// An item nearer than the idol takes the press instead ([`Duel::action_target`]).
    pub fn idol_in_reach(&self) -> Option<usize> {
        match self.action_target()? {
            ActionTarget::Idol(i) => Some(i),
            ActionTarget::Pickup(_) => None,
        }
    }

    /// The nearest idol whose kneeling spot Wolf's feet are at, and how far it is.
    pub(super) fn idol_near(&self) -> Option<(usize, f32)> {
        let feet = self.player.body.position;
        self.idols
            .spots
            .iter()
            .enumerate()
            .filter_map(|(i, s)| {
                let [x, y, z] = s.kneel.position;
                let d = (x - feet[0]).hypot(z - feet[2]);
                (d < REACH && (y - feet[1]).abs() < REACH_Y).then_some((i, d))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }

    /// The idol Wolf kneels or prays at.
    pub fn idol(&self) -> Option<usize> {
        self.idols.at
    }

    /// Standing or crouching, alive, on the ground and in no deathblow or throw.
    pub(super) fn can_act(&self) -> bool {
        let runtime = &self.player.behavior.runtime;
        let state = runtime.main_state_name();
        (state.starts_with("Stand") || state.starts_with("Crouch"))
            && !runtime.has_pending_events()
            && self.player.body.grounded
            && !self.rules.fighters[0].dead()
            && self.respawn_in.is_none()
            && self.deathblow.is_none()
            && self.enemy_throw.is_none()
    }

    /// Kneels Wolf on the action press at an idol and holds his input while he is at one: the
    /// frame the step uses instead of `input`, if any.
    pub(super) fn idol_input(&mut self, input: &InputFrame) -> Option<InputFrame> {
        let pressed = input.buttons.action && !self.idols.action_held;
        self.idols.action_held = input.buttons.action;
        // Off the idol once he stood up, something else took him (a hit, death), or the kneel
        // event was never taken.
        if self.idols.at.is_some()
            && self.idol_phase().is_none()
            && !self.player.behavior.runtime.has_pending_events()
        {
            self.idols.at = None;
        }
        if pressed {
            match self.action_target() {
                Some(ActionTarget::Idol(i)) => self.kneel_at(i),
                Some(ActionTarget::Pickup(p)) => self.pick_up(p),
                None => {}
            }
        }
        self.idols.at.map(|_| InputFrame {
            camera_yaw: input.camera_yaw,
            ..InputFrame::default()
        })
    }

    fn kneel_at(&mut self, i: usize) {
        if !self
            .player
            .behavior
            .runtime
            .fire_event("W_StatueAttachStart")
        {
            return;
        }
        let spot = self.idols.spots[i];
        self.player.body.position = spot.kneel.position;
        self.player.body.velocity = [0.0; 3];
        self.player.snap_yaw(spot.kneel.yaw);
        self.player.lock_target = None;
        self.rules.fighters[0].reset();
        self.idols.at = Some(i);
        self.idols.checkpoint = Some(spot.checkpoint);
        if self.idols.lit.insert(spot.flag) {
            self.idols.found = Some(i);
        }
    }

    /// Whether idol `i` is lit (Wolf knelt at it once).
    pub fn idol_lit(&self, i: usize) -> bool {
        self.idols
            .spots
            .get(i)
            .is_some_and(|s| self.idols.lit.contains(&s.flag))
    }

    /// The idol lit since the last call, once: the cue for the "idol found" banner.
    pub fn take_idol_found(&mut self) -> Option<usize> {
        self.idols.found.take()
    }

    /// Rests at the idol Wolf prays at: Vitality refilled, and the map's foes back.
    pub fn rest_at_idol(&mut self) {
        if self.praying()
            && self
                .player
                .behavior
                .runtime
                .fire_event("W_StatueAttachRest")
        {
            self.rules.fighters[0].reset();
            self.idols.revive = self.population.is_some();
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

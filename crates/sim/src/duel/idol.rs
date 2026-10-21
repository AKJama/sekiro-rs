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
//! The first kneel at an idol lights it (#152): its [`LitKey`] goes into the lit set, which a
//! respawn keeps, and Wolf stays at the idol in [`IdolPhase::Lighting`] while the game's
//! "SCULPTOR'S IDOL FOUND" banner plays ([`LIGHTING_SECS`]); only then does he pray and the
//! idol menu open. Kneeling at a lit idol goes straight to the prayer.

use std::collections::BTreeSet;

use super::Duel;
use super::pickup::ActionTarget;
use crate::deathblow::Placement;
use crate::input::InputFrame;

/// How near an idol's kneeling spot Wolf can kneel (m across, m up or down).
const REACH: f32 = 1.3;
const REACH_Y: f32 = 0.6;

/// How long lighting an idol holds Wolf before he prays (s): the `BonfireLit` clip of the text
/// effects in `menu/01_000_FE.gfx` (the "SCULPTOR'S IDOL FOUND" banner), played at 60 frames a
/// second from its `Normal` label on frame 1 to its last frame, 197.
pub const LIGHTING_SECS: f32 = (197.0 - 1.0) / 60.0;

/// Where Wolf is at an idol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdolPhase {
    Kneeling,
    /// The first kneel at an unlit idol, while its "found" banner plays ([`LIGHTING_SECS`]):
    /// he kneels, then prays, held and with the menu shut.
    Lighting,
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
    /// What marks it lit.
    pub lit: LitKey,
}

/// What marks an idol lit: its BonfireWarpParam `eventflagId`, or its bonfire entity id when it
/// has no flag, so no two idols share one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LitKey {
    Flag(i32),
    Entity(i32),
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
    /// The lit idols.
    lit: BTreeSet<LitKey>,
    /// Seconds since Wolf lit the idol he is at, while its banner plays.
    lighting: Option<f32>,
}

impl Duel {
    pub fn set_idols(&mut self, spots: Vec<IdolSpot>) {
        self.idols.spots = spots;
        // `at` indexes the old spots; the lit set is keyed by idol, not index, and stays.
        self.idols.at = None;
        self.idols.lighting = None;
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
    pub(super) fn idol_input(&mut self, input: &InputFrame, dt: f32) -> Option<InputFrame> {
        let pressed = input.buttons.action && !self.idols.action_held;
        self.idols.action_held = input.buttons.action;
        self.idols.lighting = self
            .idols
            .lighting
            .map(|t| t + dt)
            .filter(|t| *t < LIGHTING_SECS);
        // Off the idol once he stood up, something else took him (a hit, death), or the kneel
        // event was never taken.
        if self.idols.at.is_some()
            && self.idol_phase().is_none()
            && !self.player.behavior.runtime.has_pending_events()
        {
            self.idols.at = None;
            self.idols.lighting = None;
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
        self.idols.lighting = self.idols.lit.insert(spot.lit).then_some(0.0);
    }

    /// Seconds since Wolf lit the idol he is at, while he is [`IdolPhase::Lighting`]: the
    /// banner's clock.
    pub fn idol_lighting(&self) -> Option<f32> {
        self.idols
            .lighting
            .filter(|_| self.idol_phase() == Some(IdolPhase::Lighting))
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
        let phase = match self.player.behavior.runtime.main_state_name() {
            "StatueAttachStart" => IdolPhase::Kneeling,
            "StatueAttachLoop" => IdolPhase::Praying,
            "StatueAttachRest" => IdolPhase::Resting,
            "StatueAttachEnd" => IdolPhase::Leaving,
            _ => return None,
        };
        let lighting = self.idols.lighting.is_some()
            && matches!(phase, IdolPhase::Kneeling | IdolPhase::Praying);
        Some(if lighting { IdolPhase::Lighting } else { phase })
    }
}

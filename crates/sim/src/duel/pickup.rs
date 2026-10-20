//! Picking up map items (#136): the action button inside an item's ActionButtonParam reach takes
//! its lot into Wolf's kit and sets the lot's taken flag there, so the item stays gone through
//! Rest and death. The press goes to the nearer of an item and an idol's kneeling spot.
//! After FwLo's RustSekiro `src/items.rs` (Fireguy32's pickups): goods 1000 is a Spirit Emblem,
//! and there is no pickup animation (60070 has no clip).
//! gap: the lot is drawn with a fixed sequence, not the exe's random numbers (every m11_00
//! treasure lot is one 1000-point entry, so it does not matter there), and ActionButtonParam's
//! angle and dummy polygon (4201, a corpse's purse) are not checked.

use std::sync::Arc;

use super::Duel;
use crate::items::{ItemData, MapPickup};

/// The goods one pickup gave, for the "got" line.
#[derive(Debug, Clone, PartialEq)]
pub struct Got {
    /// The pickup's index in [`Duel::pickups`].
    pub pickup: usize,
    /// EquipParamGoods id and count added.
    pub goods: Vec<(i32, u32)>,
}

/// The play world's map items.
#[derive(Debug, Default)]
pub(super) struct Pickups {
    spots: Vec<MapPickup>,
    data: Arc<ItemData>,
    /// Pickups taken so far, so a screen can tell a new one.
    taken: u32,
    last: Option<Got>,
    draws: u32,
}

/// What the action button would act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionTarget {
    Idol(usize),
    Pickup(usize),
}

impl Duel {
    pub fn set_pickups(&mut self, spots: Vec<MapPickup>, data: Arc<ItemData>) {
        self.pickups.spots = spots;
        self.pickups.data = data;
    }

    pub fn pickups(&self) -> &[MapPickup] {
        &self.pickups.spots
    }

    /// Whether the item is still on the map (its lot's flag not taken).
    pub fn pickup_lies(&self, i: usize) -> bool {
        self.pickups
            .spots
            .get(i)
            .is_some_and(|p| p.lot.flag <= 0 || !self.kit.taken_flags.contains(&p.lot.flag))
    }

    /// The number of pickups taken, and the latest one's goods.
    pub fn last_pickup(&self) -> (u32, Option<&Got>) {
        (self.pickups.taken, self.pickups.last.as_ref())
    }

    /// The nearest item lying with Wolf's feet in its reach, and how far it is.
    fn pickup_near(&self) -> Option<(usize, f32)> {
        let feet = self.player.body.position;
        self.pickups
            .spots
            .iter()
            .enumerate()
            .filter(|(i, _)| self.pickup_lies(*i))
            .filter_map(|(i, p)| Some((i, p.reach.distance(p.position, feet)?)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }

    /// What the action button acts on now: the nearer of an idol in reach and an item.
    pub fn action_target(&self) -> Option<ActionTarget> {
        if self.idol().is_some() || !self.can_kneel() {
            return None;
        }
        match (self.idol_near(), self.pickup_near()) {
            (Some((_, di)), Some((p, dp))) if dp < di => Some(ActionTarget::Pickup(p)),
            (Some((i, _)), _) => Some(ActionTarget::Idol(i)),
            (None, Some((p, _))) => Some(ActionTarget::Pickup(p)),
            (None, None) => None,
        }
    }

    /// The item the action button would pick up (the "Pick Up Item" guide).
    pub fn pickup_in_reach(&self) -> Option<usize> {
        match self.action_target()? {
            ActionTarget::Pickup(p) => Some(p),
            ActionTarget::Idol(_) => None,
        }
    }

    /// Takes the item's lot into the kit and marks it taken.
    pub(super) fn pick_up(&mut self, i: usize) {
        let Some(spot) = self.pickups.spots.get(i) else {
            return;
        };
        let data = self.pickups.data.clone();
        let lot = spot.lot.clone();
        // A golden-ratio sequence in 0..1 stands in for the exe's random draw.
        self.pickups.draws += 1;
        let r = (self.pickups.draws as f32 * 0.618_034).fract();
        let mut goods = Vec::new();
        if let Some((id, n)) = lot.roll(0.0, r) {
            // The line names the lot's count even when some is over the carry limit.
            self.kit.add(id, n, data.max(id));
            goods.push((id, n));
        }
        if lot.flag > 0 {
            self.kit.taken_flags.insert(lot.flag);
        }
        eprintln!(
            "picked up {} (ItemLotParam {}): {goods:?}",
            spot.name, lot.id
        );
        self.pickups.taken += 1;
        self.pickups.last = Some(Got { pickup: i, goods });
    }
}

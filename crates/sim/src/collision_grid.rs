//! The collision grid's cells: triangles by cell over the horizontal plane, then by height
//! band, so a query near one height looks only at the triangles that reach it. Hot 4 m cells in
//! m11_00 hold 3,000 to 7,400 triangles from several storeys, and a floor under a point needs
//! only the few whose footprint holds it (#163).
//!
//! Each list keeps its triangles in triangle order, as the queries that depend on the order
//! need ([`super::CollisionWorld::gather_slide`] sorts what it gathers from several lists).

/// The height of a band. A power of two, so [`band_of`] and a band's bottom
/// (`band as f32 * BAND`) are exact.
pub(super) const BAND: f32 = 2.0;

/// The side of a floor cell ([`super::CollisionWorld::floor_cells`]): a power of two that
/// divides [`super::CELL`], so each floor cell lies in one grid cell.
pub(super) const FLOOR_CELL: f32 = 2.0;

/// The band holding height `y`.
pub(super) fn band_of(y: f32) -> i32 {
    (y * (1.0 / BAND)).floor() as i32
}

/// How far past its corners' box a floor query can read a height off a triangle spanning
/// `extent` (its largest side of that box) at coordinates up to `magnitude`: more than
/// [`super::height_on`]'s barycentric slack of 1e-5 per corner and float roundoff allow.
pub(super) fn slack(extent: f32, magnitude: f32) -> f32 {
    0.02 + 1e-4 * extent + 1e-6 * magnitude
}

/// The first and last band a triangle whose corners span heights `lo` to `hi` is filed in,
/// padded by [`slack`], so every height a floor query reads off the triangle lies within them.
pub(super) fn bands_of(lo: f32, hi: f32) -> (i32, i32) {
    let pad = slack(hi - lo, lo.abs().max(hi.abs()));
    (band_of(lo - pad), band_of(hi + pad))
}

/// Whether hit material `m` is a water surface's ([`super::View::water_surface`]).
pub(super) fn water_material(m: u32) -> bool {
    matches!(m, 20..=24 | 35..=38)
}

/// What one band of a cell holds.
pub(super) trait Slot: Default {
    /// Adds `other`'s triangles, numbered from `offset` on here, after this slot's.
    fn merge(&mut self, other: Self, offset: u32);
    /// Frees the room its lists grew beyond what they hold.
    fn shrink(&mut self);
}

impl Slot for Vec<u32> {
    fn merge(&mut self, other: Self, offset: u32) {
        self.extend(other.into_iter().map(|t| t + offset));
    }

    fn shrink(&mut self) {
        self.shrink_to_fit();
    }
}

/// One band of a grid cell.
#[derive(Default)]
pub(super) struct Band {
    /// Solid faces flat enough to stand on (whether they are live is asked per query).
    pub floors: Vec<u32>,
    /// Everything else.
    pub rest: Vec<u32>,
}

impl Band {
    pub fn lists(&self) -> [&[u32]; 2] {
        [&self.floors, &self.rest]
    }
}

impl Slot for Band {
    fn merge(&mut self, other: Self, offset: u32) {
        self.floors.merge(other.floors, offset);
        self.rest.merge(other.rest, offset);
    }

    fn shrink(&mut self) {
        self.floors.shrink();
        self.rest.shrink();
    }
}

/// A cell's bands, from the lowest any of its triangles reaches to the highest.
pub(super) struct Bands<T> {
    /// The band `slots[0]` is.
    base: i32,
    slots: Vec<T>,
}

impl<T> Default for Bands<T> {
    fn default() -> Self {
        Self {
            base: 0,
            slots: Vec::new(),
        }
    }
}

impl<T: Slot> Bands<T> {
    /// The bands from `lo` to `hi` the cell has, lowest first, with their numbers.
    pub fn range(&self, lo: i32, hi: i32) -> impl DoubleEndedIterator<Item = (i32, &T)> {
        let top = self.base.saturating_add(self.slots.len() as i32) - 1;
        let (lo, hi) = (lo.max(self.base), hi.min(top));
        (lo..=hi).map(move |b| (b, &self.slots[(b - self.base) as usize]))
    }

    /// Every band, each with its number.
    pub fn all(&self) -> impl DoubleEndedIterator<Item = (i32, &T)> {
        self.range(i32::MIN, i32::MAX)
    }

    /// Bands `lo` to `hi`, grown to hold them, to file a triangle in.
    pub fn span_mut(&mut self, (lo, hi): (i32, i32)) -> &mut [T] {
        if self.slots.is_empty() {
            self.base = lo;
        }
        if lo < self.base {
            let more = (self.base - lo) as usize;
            self.slots
                .splice(0..0, std::iter::repeat_with(T::default).take(more));
            self.base = lo;
        }
        let len = (hi - self.base + 1) as usize;
        if self.slots.len() < len {
            self.slots.resize_with(len, T::default);
        }
        &mut self.slots[(lo - self.base) as usize..len]
    }

    /// Adds `other`'s triangles, numbered from `offset` on here, after this cell's.
    pub fn merge(&mut self, other: Self, offset: u32) {
        if other.slots.is_empty() {
            return;
        }
        let top = other.base + other.slots.len() as i32 - 1;
        for (slot, more) in self.span_mut((other.base, top)).iter_mut().zip(other.slots) {
            slot.merge(more, offset);
        }
    }

    /// Frees the room the bands' lists grew beyond what they hold.
    pub fn shrink(&mut self) {
        self.slots.iter_mut().for_each(T::shrink);
        self.slots.shrink_to_fit();
    }

    /// The bands, lowest first.
    #[cfg(test)]
    pub fn slots(&self) -> &[T] {
        &self.slots
    }
}

/// One cell of the grid (4 m square, [`super::CELL`]).
#[derive(Default)]
pub(super) struct Column {
    pub bands: Bands<Band>,
    /// The triangles with a water material, at any height ([`super::View::water_surface`]).
    pub water: Vec<u32>,
    /// The triangles of helper and event parts, at any height: the only ones the
    /// fall-prevention assist's walls can be ([`super::CollisionWorld::gather_slide`]).
    pub parts: Vec<u32>,
}

impl Column {
    /// Adds `other`'s triangles, numbered from `offset` on here, after this cell's.
    pub fn merge(&mut self, other: Column, offset: u32) {
        self.bands.merge(other.bands, offset);
        self.water.merge(other.water, offset);
        self.parts.merge(other.parts, offset);
    }

    /// Frees the room the cell's lists grew beyond what they hold.
    pub fn shrink(&mut self) {
        self.bands.shrink();
        self.water.shrink();
        self.parts.shrink();
    }
}

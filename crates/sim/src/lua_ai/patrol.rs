//! MSB patrols, using FwLo's documented loop, ping-pong and one-way stepping.
use sekiro_formats::msb::{MapRegion, Msb, Part};

#[derive(Debug, Clone)]
pub struct Patrol {
    pub kind: i32,
    points: Vec<(i16, MapRegion)>,
    arrivals: Vec<(i16, i32, i32)>,
    pub current: i32,
    pub previous: i32,
    back: bool,
}

impl Patrol {
    pub fn from_msb(msb: &Msb, part: &Part) -> Option<Self> {
        let route = msb
            .patrols
            .get(usize::try_from(part.enemy.as_ref()?.walk_route_index).ok()?)?;
        let points: Vec<_> = route
            .regions
            .iter()
            .filter_map(|&i| Some((i, msb.regions.get(usize::try_from(i).ok()?)?.clone())))
            .collect();
        let current = if points.is_empty() || !(0..=5).contains(&route.kind) {
            -1
        } else {
            0
        };
        Some(Self {
            kind: route.kind,
            points,
            arrivals: route.arrivals.to_vec(),
            current,
            previous: -1,
            back: false,
        })
    }

    pub fn position(&self) -> Option<[f32; 3]> {
        let r = &self.points.get(usize::try_from(self.current).ok()?)?.1;
        Some([-r.position[0], r.position[1], r.position[2]])
    }

    pub fn arrival(&self, index: i32) -> Option<(i32, i32)> {
        let region = self.points.get(usize::try_from(index).ok()?)?.0;
        self.arrivals
            .iter()
            .find(|a| a.0 == region)
            .map(|a| (a.1, a.2))
    }

    pub fn ended(&self) -> bool {
        self.previous + 1 >= self.points.len() as i32
    }

    pub fn step(&mut self, rng: u64) {
        let n = self.points.len() as i32;
        if self.current < 0 || n == 0 {
            return;
        }
        self.previous = self.current;
        self.current = match self.kind {
            0 | 3 | 4 if n > 1 => (self.current + 1) % n,
            1 if n > 1 => {
                if (self.back && self.current == 0) || (!self.back && self.current == n - 1) {
                    self.back = !self.back;
                }
                self.current + if self.back { -1 } else { 1 }
            }
            2 if n > 1 => ((self.current as u64 + 1 + rng % (n as u64 - 1)) % n as u64) as i32,
            5 if self.current + 1 < n => self.current + 1,
            _ => -1,
        };
    }
}

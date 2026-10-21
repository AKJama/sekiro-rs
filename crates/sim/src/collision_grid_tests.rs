//! The height-banded grid (#163) answers every query exactly as the 4 m grid without bands
//! before it did, on the play world's real collision.
use std::collections::HashMap;

use rayon::prelude::*;

use super::*;
use crate::hit_filter::{FALL_ASSIST_ACTION, FALL_ASSIST_MOVE, RIDE_ASSIST_MOVE};

/// The grid before #163: each 4 m cell's triangles in triangle order, at every height, and the
/// queries as they scanned it.
struct Unbanded<'a> {
    world: &'a CollisionWorld,
    cells: HashMap<(i32, i32), Vec<u32>>,
}

impl<'a> Unbanded<'a> {
    fn new(world: &'a CollisionWorld) -> Self {
        let mut cells: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        for (t, [lo, hi]) in world.bounds.iter().enumerate() {
            let (x0, z0) = cell_of(lo.x, lo.z);
            let (x1, z1) = cell_of(hi.x, hi.z);
            for cx in x0..=x1 {
                for cz in z0..=z1 {
                    cells.entry((cx, cz)).or_default().push(t as u32);
                }
            }
        }
        Self { world, cells }
    }

    fn floor_triangle(&self, x: f32, z: f32, top: f32, who: u8) -> Option<(f32, u32)> {
        let w = self.world;
        let mut best: Option<(f32, u32)> = None;
        for &t in self.cells.get(&cell_of(x, z))? {
            if !w.walkable(t, who) {
                continue;
            }
            let [a, b, c] = w.corners(t);
            if let Some(h) = height_on(x, z, a, b, c)
                && h <= top
                && best.is_none_or(|(b, _)| h > b)
            {
                best = Some((h, t));
            }
        }
        best
    }

    fn ceiling(&self, feet: Vec3, rise: f32, who: u8) -> Option<f32> {
        let w = self.world;
        let centre = feet.y + HEAD_TOP - RADIUS;
        let high = feet.y + HEAD_TOP + rise;
        let line = Vec2::new(feet.x, feet.z);
        let reach = ceiling_reach();
        let (min, max) = (line - RADIUS, line + RADIUS);
        let (x0, z0) = cell_of(min.x, min.y);
        let (x1, z1) = cell_of(max.x, max.y);
        let mut best: Option<f32> = None;
        for cx in x0..=x1 {
            for cz in z0..=z1 {
                for &t in self.cells.get(&(cx, cz)).into_iter().flatten() {
                    let [lo, hi] = w.bounds[t as usize];
                    let (lo2, hi2) = (Vec2::new(lo.x, lo.z), Vec2::new(hi.x, hi.z));
                    if hi.y >= centre
                        && lo.y <= high
                        && line.distance_squared(line.clamp(lo2, hi2)) < reach * reach
                        && cell_of(lo.x.max(min.x), lo.z.max(min.y)) == (cx, cz)
                        && w.walkable(t, who)
                        && let Some(up) = w.rise_into(t, feet + Vec3::Y * (HEAD_TOP - RADIUS), rise)
                    {
                        best = Some(best.map_or(up, |b| b.min(up)));
                    }
                }
            }
        }
        best.map(|up| feet.y + up)
    }

    fn surfaces_at(&self, x: f32, z: f32) -> Vec<(f32, f32)> {
        let w = self.world;
        let mut out: Vec<(f32, f32)> = self
            .cells
            .get(&cell_of(x, z))
            .into_iter()
            .flatten()
            .filter(|&&t| w.live(t, PLAYER))
            .filter_map(|&t| {
                let [a, b, c] = w.corners(t);
                Some((height_on(x, z, a, b, c)?, w.normals[t as usize].y))
            })
            .collect();
        out.sort_by(|a, b| b.0.total_cmp(&a.0));
        out
    }

    fn water_surface(&self, x: f32, z: f32, top: f32, who: u8) -> Option<WaterSurface> {
        let w = self.world;
        self.cells
            .get(&cell_of(x, z))?
            .iter()
            .filter_map(|&t| {
                let material = w.materials[t as usize];
                if !matches!(material, 20..=24 | 35..=38) || !w.live(t, who) {
                    return None;
                }
                let [a, b, c] = w.corners(t);
                let height = height_on(x, z, a, b, c)?;
                (height <= top).then_some(WaterSurface { height, material })
            })
            .max_by(|a, b| a.height.total_cmp(&b.height))
    }

    fn ray(&self, origin: Vec3, dir: Vec3, max: f32, who: u8) -> Option<f32> {
        let w = self.world;
        let end = origin + dir * max;
        if max < 0.0 || !origin.is_finite() || !end.is_finite() {
            return None;
        }
        let (mut x, mut z) = cell_of(origin.x, origin.z);
        let (ex, ez) = cell_of(end.x, end.z);
        let axis = |start: f32, delta: f32, cell: i32| {
            if delta == 0.0 {
                return (0, f32::INFINITY, f32::INFINITY);
            }
            let step = if delta > 0.0 { 1 } else { -1 };
            let boundary = (cell as f32 + if step > 0 { 1.0 } else { 0.0 }) * CELL;
            (step, (boundary - start) / delta, CELL / delta.abs())
        };
        let (sx, mut tx, dx) = axis(origin.x, end.x - origin.x, x);
        let (sz, mut tz, dz) = axis(origin.z, end.z - origin.z, z);
        const ALONG: f32 = 1e-4;
        const PAD: f32 = 1e-3;
        let rise = end.y - origin.y;
        let mut enter = 0.0f32;
        let mut best: Option<f32> = None;
        for _ in 0..(u64::from(x.abs_diff(ex)) + u64::from(z.abs_diff(ez)) + 1) {
            let last = (x, z) == (ex, ez);
            if x == ex {
                tx = f32::INFINITY;
            }
            if z == ez {
                tz = f32::INFINITY;
            }
            let leave = tx.min(tz).clamp(enter, 1.0);
            if let Some(cell) = self.cells.get(&(x, z)) {
                let [y0, y1] = [(enter - ALONG).max(0.0), (leave + ALONG).min(1.0)]
                    .map(|t| origin.y + rise * t);
                let (low_y, high_y) = (y0.min(y1) - PAD, y0.max(y1) + PAD);
                for &t in cell {
                    let [lo, hi] = w.bounds[t as usize];
                    if lo.y > high_y || hi.y < low_y || !w.solid(t) || !w.live(t, who) {
                        continue;
                    }
                    let [a, b, c] = w.corners(t);
                    if let Some(d) = ray_triangle(origin, dir, a, b, c)
                        && d <= max
                        && best.is_none_or(|b| d < b)
                    {
                        best = Some(d);
                    }
                }
            }
            if last || best.is_some_and(|b| b < (leave - ALONG) * max - PAD) {
                break;
            }
            if tx < tz {
                x += sx;
                tx += dx;
            } else if tz < tx {
                z += sz;
                tz += dz;
            } else {
                x += sx;
                z += sz;
                tx += dx;
                tz += dz;
            }
            enter = leave;
        }
        best
    }

    fn slide(&self, feet: [f32; 3], delta: [f32; 2], who: u8, assist: u8, cull: bool) -> [f32; 2] {
        let w = self.world;
        let start = Vec3::from(feet);
        let d = Vec2::from(delta);
        let len = d.length();
        let coarse = ((len / (RADIUS * 0.5)).ceil() as usize).clamp(1, 32);
        let reach = RADIUS + len / coarse as f32 + 0.05;
        let min = Vec2::new(start.x.min(start.x + d.x), start.z.min(start.z + d.y)) - reach;
        let max = Vec2::new(start.x.max(start.x + d.x), start.z.max(start.z + d.y)) + reach;
        let low = start.y + SPHERES[0] - RADIUS - 0.01;
        let high = start.y + HEAD_TOP + 0.01;
        let pad = RADIUS + len;
        let (near_min, near_max) = (min - pad, max + pad);
        let (mut body, mut walls) = (Vec::new(), Vec::new());
        let (x0, z0) = cell_of(min.x, min.y);
        let (x1, z1) = cell_of(max.x, max.y);
        for cx in x0..=x1 {
            for cz in z0..=z1 {
                let Some(cell) = self.cells.get(&(cx, cz)) else {
                    continue;
                };
                body.extend(cell.iter().copied().filter(|&t| {
                    let [lo, hi] = w.bounds[t as usize];
                    hi.y >= low
                        && lo.y <= high
                        && (!cull
                            || hi.x >= near_min.x
                                && lo.x <= near_max.x
                                && hi.z >= near_min.y
                                && lo.z <= near_max.y)
                        && w.solid(t)
                        && w.live(t, who)
                }));
                if assist != 0 {
                    walls.extend(cell.iter().copied().filter(|&t| w.live(t, assist)));
                }
            }
        }
        if x0 != x1 || z0 != z1 {
            for tris in [&mut body, &mut walls] {
                tris.sort_unstable();
                tris.dedup();
            }
        }
        w.slide_among(start, d, coarse, &body, &walls)
    }
}

/// The play world's collision as `step_cost`'s test builds it: both Ashina blocks, helpers
/// stripped and added back, placed objects.
fn play_world() -> Option<CollisionWorld> {
    let cache = crate::cache::root();
    let blocks = ["m11_00_00_00", "m11_01_00_00"];
    if !blocks
        .iter()
        .all(|b| cache.join("maps").join(b).join("collision.bin").is_file())
    {
        eprintln!("skipping: no m11 collision in {}", cache.display());
        return None;
    }
    let mut mesh = CollisionMesh::default();
    let mut helpers = crate::hit_filter::HelperCollision::default();
    for b in blocks {
        let dir = cache.join("maps").join(b);
        let mut part =
            CollisionMesh::from_bytes(&std::fs::read(dir.join("collision.bin")).unwrap()).unwrap();
        let layout: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join("layout.json")).unwrap()).unwrap();
        helpers.extend(crate::hit_filter::strip_helpers(&mut part, &layout));
        mesh.extend(&part);
    }
    let mut world = CollisionWorld::from_mesh(&mesh);
    world.add_helpers(&helpers);
    for b in blocks {
        world
            .add_block_objects(&cache.join("maps").join(b), b, |_, who| who)
            .unwrap();
    }
    Some(world)
}

/// A small deterministic generator (xorshift64*).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// Uniform in `[lo, hi)`.
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn pick<T: Copy>(&mut self, of: &[T]) -> T {
        of[(self.next() % of.len() as u64) as usize]
    }
}

/// Every query at many places near the play world's triangles (picked uniformly, so mostly in
/// the dense cells, and a third of them on triangles spanning several bands), at heights from
/// below to above each, gives the same answer to the bit as the unbanded grid.
#[test]
fn banded_grid_answers_as_the_unbanded_one() {
    let Some(mut world) = play_world() else {
        return;
    };
    // A switched event wall the fall-prevention assist holds, among the helper parts.
    let wall = CollisionMesh {
        vertices: vec![
            [-257.0, -50.0, 6.0],
            [-253.0, -50.0, 6.0],
            [-253.0, -40.0, 6.0],
            [-257.0, -40.0, 6.0],
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
        materials: vec![0, 0],
    };
    world.append_event_part(1, &wall, FALL_ASSIST_MOVE | ENEMY, true);
    let world = &world;
    let unbanded = Unbanded::new(world);
    let bands: Vec<&grid::Band> = world
        .columns
        .values()
        .flat_map(|c| c.bands.slots())
        .collect();
    let squares: Vec<&Vec<u32>> = world.floor_cells.values().flat_map(|c| c.slots()).collect();
    // Heap bytes of the lists, with each list's header and the map's keys.
    let mb = |b: usize| b as f64 / (1 << 20) as f64;
    let list = |v: &Vec<u32>| v.capacity() * 4 + size_of::<Vec<u32>>();
    let columns = world.columns.values().map(|c| {
        c.bands
            .slots()
            .iter()
            .map(|b| list(&b.floors) + list(&b.rest))
            .sum::<usize>()
            + list(&c.water)
            + list(&c.parts)
            + size_of::<((i32, i32), grid::Column)>()
    });
    let floors = world.floor_cells.values().map(|c| {
        c.slots().iter().map(list).sum::<usize>() + size_of::<((i32, i32), grid::Bands<Vec<u32>>)>()
    });
    let unbanded_bytes = unbanded.cells.values().map(list).sum::<usize>()
        + unbanded.cells.len() * size_of::<(i32, i32)>();
    eprintln!(
        "{} triangles; unbanded {} entries in {} cells ({:.0} MB); banded {} entries in {} bands          of {} cells ({:.0} MB); floors {} entries in {} bands of {} squares ({:.0} MB)",
        world.triangle_count(),
        unbanded.cells.values().map(Vec::len).sum::<usize>(),
        unbanded.cells.len(),
        mb(unbanded_bytes),
        bands
            .iter()
            .map(|b| b.floors.len() + b.rest.len())
            .sum::<usize>(),
        bands.len(),
        world.columns.len(),
        mb(columns.sum()),
        squares.iter().map(|s| s.len()).sum::<usize>(),
        squares.len(),
        world.floor_cells.len(),
        mb(floors.sum()),
    );
    let tall: Vec<u32> = (0..world.triangle_count() as u32)
        .filter(|&t| {
            let [lo, hi] = world.bounds[t as usize];
            hi.y - lo.y > 2.0 * BAND
        })
        .collect();
    assert!(tall.len() > 1000, "{} tall triangles", tall.len());
    let all = world.triangle_count() as u32;
    let mismatches: Vec<String> = (0..12_000u64)
        .into_par_iter()
        .flat_map_iter(|i| {
            let mut rng = Rng(0x9e37_79b9_7f4a_7c15 ^ (i + 1).wrapping_mul(0xbf58_476d_1ce4_e5b9));
            let t = if i % 3 == 0 {
                rng.pick(&tall)
            } else {
                (rng.next() % u64::from(all)) as u32
            };
            let [lo, hi] = world.bounds[t as usize];
            let [mut x, mut z] = [
                rng.range(lo.x - 1.0, hi.x + 1.0),
                rng.range(lo.z - 1.0, hi.z + 1.0),
            ];
            // Some on a corner, where the triangles meeting there give equal floors.
            if i % 5 == 1 {
                let corner = world.corners(t)[(i % 3) as usize];
                [x, z] = [corner.x, corner.z];
            }
            let y = rng.range(lo.y - 3.0, hi.y + 3.0);
            let who = rng.pick(&[PLAYER, ENEMY, CAMERA]);
            let mut out = Vec::new();
            let mut check = |what: &str, same: bool, detail: String| {
                if !same {
                    out.push(format!("{what} at ({x}, {y}, {z}) for {who}: {detail}"));
                }
            };
            for top in [y, y + rng.range(0.0, 0.6), f32::INFINITY] {
                let (a, b) = (
                    world.floor_triangle(x, z, top, who),
                    unbanded.floor_triangle(x, z, top, who),
                );
                let bits = |f: Option<(f32, u32)>| f.map(|(h, t)| (h.to_bits(), t));
                check(
                    "floor",
                    bits(a) == bits(b),
                    format!("top {top}: {a:?} vs {b:?}"),
                );
            }
            let feet = [x, y, z];
            let angle = rng.range(0.0, std::f32::consts::TAU);
            let length = rng.pick(&[0.0, 0.05, 0.13, 0.4, 0.8, 2.0]) * rng.range(0.5, 1.0);
            let delta = [angle.sin() * length, angle.cos() * length];
            let assist = rng.pick(&[
                0,
                0,
                FALL_ASSIST_MOVE,
                FALL_ASSIST_ACTION,
                FALL_ASSIST_MOVE | RIDE_ASSIST_MOVE,
            ]);
            let cull = i % 8 != 0;
            let (a, b) = (
                world.slide_for(feet, delta, who, assist, cull),
                unbanded.slide(feet, delta, who, assist, cull),
            );
            check(
                "slide",
                a.map(f32::to_bits) == b.map(f32::to_bits),
                format!("{delta:?} assist {assist} cull {cull}: {a:?} vs {b:?}"),
            );
            let rise = rng.range(0.0, 3.0);
            let (a, b) = (
                world.ceiling_for(Vec3::from(feet), rise, who),
                unbanded.ceiling(Vec3::from(feet), rise, who),
            );
            check(
                "ceiling",
                a.map(f32::to_bits) == b.map(f32::to_bits),
                format!("rise {rise}: {a:?} vs {b:?}"),
            );
            let origin = Vec3::new(x, y + rng.range(0.0, 2.0), z);
            let dir = Vec3::new(
                rng.range(-1.0, 1.0),
                rng.range(-0.6, 0.6),
                rng.range(-1.0, 1.0),
            )
            .normalize_or(Vec3::X);
            let max = rng.pick(&[0.5, 4.0, 25.0, 60.0]);
            let (a, b) = (
                world.ray(origin, dir, max, who).0,
                unbanded.ray(origin, dir, max, who),
            );
            check(
                "ray",
                a.map(f32::to_bits) == b.map(f32::to_bits),
                format!("{origin} {dir} {max}: {a:?} vs {b:?}"),
            );
            let view = View { world, who };
            let (a, b) = (
                view.water_surface(x, z, y),
                unbanded.water_surface(x, z, y, who),
            );
            check("water", a == b, format!("{a:?} vs {b:?}"));
            if i % 16 == 0 {
                let (a, b) = (world.surfaces_at(x, z), unbanded.surfaces_at(x, z));
                check("surfaces", a == b, format!("{a:?} vs {b:?}"));
            }
            out
        })
        .collect();
    assert!(
        mismatches.is_empty(),
        "{} mismatches, first: {:#?}",
        mismatches.len(),
        &mismatches[..mismatches.len().min(5)]
    );
}

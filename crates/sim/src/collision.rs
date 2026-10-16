//! Map collision for character bodies: the hit collision exported by `sekiro-extract map`
//! (`cache/maps/<id>/collision.bin`, a triangle soup in Bevy space) behind a [`Ground`].
//!
//! Triangles are bucketed in a uniform grid over the horizontal plane (4 m cells), which suits
//! the queries a walking character makes: what is under my feet, and what do I bump into.
//!
//! - Floors: [`CollisionWorld::floor`] returns the highest walkable surface (normal within
//!   [`MAX_SLOPE_DEGREES`] of up) under a point, no higher than a given top. Bodies ask for
//!   the floor up to [`STEP_HEIGHT`] above their feet, so stairs and small ledges are stepped
//!   onto, and follow the floor down by the same amount before they start falling.
//! - Walls: the body is three spheres stacked above the step zone (radius [`RADIUS`]).
//!   [`CollisionWorld::slide`] moves them in sub-steps and pushes them out of every triangle
//!   horizontally, so the body stops at walls and slides along them, and steep slopes act as
//!   walls. Vertical pushes are left to the floor query.
//! - [`CollisionWorld::raycast`] serves cameras.
//!
//! The collision mesh does not say which side of a triangle is solid (its winding is not
//! consistent), so floors use `|normal.y|` and pushes go away from the closest point.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use glam::{Vec2, Vec3};
use sekiro_formats::hknp::CollisionMesh;

use crate::body::{Ground, WaterSurface};

/// Body radius in metres: the player's physics capsule is 1.5 m tall with a 0.4 m radius, read
/// from the running game by FwLo's RustSekiro (`docs/kb/movement.md`).
pub const RADIUS: f32 = 0.4;

/// Reserves a throw's aligned endpoint and interpolation path using the ordinary capsule
/// sweep. Every rejected displacement becomes a common translation of both characters.
/// This bounded alternating projection is an inferred wall-clearance rule, not an exe trace.
pub fn throw_pair_shift(
    grounds: [&dyn Ground; 2],
    starts: [[f32; 3]; 2],
    aligned: [[f32; 3]; 2],
) -> Option<[f32; 2]> {
    throw_pair_path_shift(grounds, &[starts, aligned])
}

/// As above, also reserving authored paired root-motion samples through the whole body clip.
pub fn throw_pair_path_shift(
    grounds: [&dyn Ground; 2],
    path: &[[[f32; 3]; 2]],
) -> Option<[f32; 2]> {
    pair_path_shift(grounds, path, 4.0, true)
}

/// Small corrections for actual paired movement, avoiding large relocations mid-clip.
pub fn throw_pair_step_shift(
    grounds: [&dyn Ground; 2],
    path: &[[[f32; 3]; 2]],
) -> Option<[f32; 2]> {
    pair_path_shift(grounds, path, 0.2, true)
}

/// Start/recovery travel already went through the controller's wall slide. Check its
/// floor support without re-sweeping the slide's curved path as a straight line.
pub(crate) fn throw_pair_floor_step_shift(
    grounds: [&dyn Ground; 2],
    path: &[[[f32; 3]; 2]],
) -> Option<[f32; 2]> {
    pair_path_shift(grounds, path, 0.2, false)
}

fn pair_path_shift(
    grounds: [&dyn Ground; 2],
    path: &[[[f32; 3]; 2]],
    limit: f32,
    sweep_motion: bool,
) -> Option<[f32; 2]> {
    if path.is_empty() {
        return Some([0.0; 2]);
    }
    let preferred = if sweep_motion {
        throw_wall_shift(grounds, path).unwrap_or([0.0; 2])
    } else {
        [0.0; 2]
    };
    if preferred[0].hypot(preferred[1]) <= limit
        && pair_path_clear(grounds, path, preferred, sweep_motion)
    {
        return Some(preferred);
    }
    // Try the blocking surface's normal first. A wide paired clip at a door can
    // need metres of clearance; probing every direction at every radius stalls play.
    if limit > 0.2
        && let Some(direction) = pair_clearance_direction(grounds, path)
    {
        for ring in 1..=(limit / 0.1).round() as usize {
            let radius = ring as f32 * 0.1;
            let shift = [
                preferred[0] + direction[0] * radius,
                preferred[1] + direction[1] * radius,
            ];
            if shift[0].hypot(shift[1]) <= limit
                && pair_path_clear(grounds, path, shift, sweep_motion)
            {
                return Some(shift);
            }
        }
    }
    // Search locally around the wall projection for supported pair space. This only
    // constrains authored paired placement; ordinary movement may still leave a ledge.
    for ring in 1..=(limit / 0.1).round() as usize {
        let radius = ring as f32 * 0.1;
        for direction in 0..16 {
            let angle = direction as f32 * std::f32::consts::TAU / 16.0;
            let shift = [
                preferred[0] + radius * angle.sin(),
                preferred[1] + radius * angle.cos(),
            ];
            if shift[0].hypot(shift[1]) <= limit
                && pair_path_clear(grounds, path, shift, sweep_motion)
            {
                return Some(shift);
            }
        }
    }
    None
}

fn pair_clearance_direction(grounds: [&dyn Ground; 2], path: &[[[f32; 3]; 2]]) -> Option<[f32; 2]> {
    for segment in path.windows(2) {
        for (i, ground) in grounds.into_iter().enumerate() {
            let mut from = segment[0][i];
            from[1] = ground
                .floor(from[0], from[2], from[1] + ground.max_step())
                .unwrap_or(from[1]);
            let want = [segment[1][i][0] - from[0], segment[1][i][2] - from[2]];
            let got = ground.slide(from, want);
            let correction = [got[0] - want[0], got[1] - want[1]];
            let length = correction[0].hypot(correction[1]);
            if length > 0.01 {
                return Some(correction.map(|v| v / length));
            }
        }
    }
    None
}

/// Check the real relocation as well as the translated animation, so a valid endpoint
/// on the other side of a thin wall or on a lower roof cannot be selected.
fn pair_path_clear(
    grounds: [&dyn Ground; 2],
    path: &[[[f32; 3]; 2]],
    shift: [f32; 2],
    sweep_motion: bool,
) -> bool {
    for (i, ground) in grounds.into_iter().enumerate() {
        let origin = path[0][i];
        let translate = |p: [f32; 3]| [p[0] + shift[0], p[1], p[2] + shift[1]];
        let mut from = origin;
        for (segment, to) in std::iter::once(translate(origin))
            .chain(path.iter().skip(1).map(|p| translate(p[i])))
            .enumerate()
        {
            let delta = [to[0] - from[0], to[2] - from[2]];
            let steps = (delta[0].hypot(delta[1]) / 0.1).ceil().max(1.0) as usize;
            for n in 0..=steps {
                let t = n as f32 / steps as f32;
                let p = [from[0] + delta[0] * t, origin[1], from[2] + delta[1] * t];
                // Support the capsule footprint, allowing ordinary roof seams and steps,
                // but never borrowing a floor metres below the throw's starting floor.
                for (dx, dz) in [(0.0, 0.0), (0.2, 0.0), (-0.2, 0.0), (0.0, 0.2), (0.0, -0.2)] {
                    if !ground
                        .floor(p[0] + dx, p[2] + dz, origin[1] + ground.max_step())
                        .is_some_and(|y| (y - origin[1]).abs() <= ground.max_step())
                    {
                        return false;
                    }
                }
            }
            let mut feet = from;
            feet[1] = ground
                .floor(from[0], from[2], origin[1] + ground.max_step())
                .unwrap_or(origin[1]);
            let got = if sweep_motion || (segment == 0 && shift[0].hypot(shift[1]) > 1e-4) {
                ground.slide(feet, delta)
            } else {
                delta
            };
            if (got[0] - delta[0]).hypot(got[1] - delta[1]) > 1e-4 {
                return false;
            }
            from = to;
        }
        if !sweep_motion && shift[0].hypot(shift[1]) > 1e-4 {
            let mut end = path.last().unwrap()[i];
            end[1] = ground
                .floor(end[0], end[2], origin[1] + ground.max_step())
                .unwrap_or(origin[1]);
            let got = ground.slide(end, shift);
            if (got[0] - shift[0]).hypot(got[1] - shift[1]) > 1e-4 {
                return false;
            }
        }
    }
    true
}

fn throw_wall_shift(grounds: [&dyn Ground; 2], path: &[[[f32; 3]; 2]]) -> Option<[f32; 2]> {
    let mut shift = [0.0; 2];
    for _ in 0..32 {
        let mut clear = true;
        for segment in path.windows(2) {
            let [starts, aligned] = [segment[0], segment[1]];
            for i in 0..2 {
                let mut from = [
                    starts[i][0] + shift[0],
                    starts[i][1],
                    starts[i][2] + shift[1],
                ];
                from[1] = grounds[i]
                    .floor(from[0], from[2], from[1] + grounds[i].max_step())
                    .unwrap_or(from[1]);
                let want = [aligned[i][0] - starts[i][0], aligned[i][2] - starts[i][2]];
                let got = grounds[i].slide(from, want);
                // The sweep's rejection points along the blocking surface normal, including
                // corner contacts. Moving both by it preserves the dummy-to-dummy relation.
                let correction = [got[0] - want[0], got[1] - want[1]];
                if correction[0].hypot(correction[1]) > 1e-4 {
                    shift[0] += correction[0];
                    shift[1] += correction[1];
                    clear = false;
                }
                // Also keep the unabsorbed character and the start of the slide outside walls.
                let mut from = [
                    starts[i][0] + shift[0],
                    starts[i][1],
                    starts[i][2] + shift[1],
                ];
                from[1] = grounds[i]
                    .floor(from[0], from[2], from[1] + grounds[i].max_step())
                    .unwrap_or(from[1]);
                let correction = grounds[i].slide(from, [0.0; 2]);
                if correction[0].hypot(correction[1]) > 1e-4 {
                    shift[0] += correction[0];
                    shift[1] += correction[1];
                    clear = false;
                }
            }
        }
        if clear {
            return Some(shift);
        }
        if shift[0].hypot(shift[1]) > 4.0 {
            return None;
        }
    }
    None
}
/// Highest ledge a walking body steps onto, and the furthest it follows the floor down.
pub const STEP_HEIGHT: f32 = 0.5;
/// Steepest walkable surface.
pub const MAX_SLOPE_DEGREES: f32 = 50.0;
/// Heights of the collision spheres' centres above the feet. The lowest sphere's bottom sits
/// at the step height, so stairs reach the floor query rather than the wall push.
const SPHERES: [f32; 3] = [STEP_HEIGHT + RADIUS, 1.2, 1.55];
const CELL: f32 = 4.0;

pub struct CollisionWorld {
    vertices: Vec<Vec3>,
    triangles: Vec<[u32; 3]>,
    normals: Vec<Vec3>,
    /// Hit material per triangle (`u32::MAX` when unknown).
    materials: Vec<u32>,
    cells: HashMap<(i32, i32), Vec<u32>>,
    min_walk_ny: f32,
    /// Triangles from this index on form the gate: collision that exists only while a map event
    /// has it enabled (a boss fog wall and its invisible boundary). `u32::MAX` when there is none.
    gate_from: u32,
    gate_open: AtomicBool,
    event_parts: Vec<(i32, std::ops::Range<u32>, AtomicBool)>,
}

fn cell_of(x: f32, z: f32) -> (i32, i32) {
    ((x / CELL).floor() as i32, (z / CELL).floor() as i32)
}

impl CollisionWorld {
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mesh = CollisionMesh::from_bytes(&bytes).map_err(|e| e.to_string())?;
        Ok(Self::from_mesh(&mesh))
    }

    /// Like [`CollisionWorld::load`], keeping only the triangles that reach within `radius`
    /// of `center` on the horizontal plane ([`crate::arena::crop`]).
    pub fn load_within(path: &Path, center: Vec3, radius: f32) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mesh = CollisionMesh::from_bytes(&bytes).map_err(|e| e.to_string())?;
        Ok(Self::from_mesh(&crate::arena::crop(&mesh, center, radius)))
    }

    pub fn from_mesh(mesh: &CollisionMesh) -> Self {
        Self::build(mesh, None)
    }

    /// [`CollisionWorld::from_mesh`] plus a gate: triangles that block only while
    /// [`CollisionWorld::set_gate`] has them on (off at first).
    pub fn from_mesh_with_gate(mesh: &CollisionMesh, gate: &CollisionMesh) -> Self {
        let mut all = mesh.clone();
        all.extend(gate);
        Self::build(&all, Some(mesh.triangle_count()))
    }

    /// Switches the gate's triangles on or off. They are shared by every holder of the world.
    pub fn set_gate(&self, on: bool) {
        self.gate_open.store(on, Ordering::Relaxed);
    }

    /// Whether the gate's triangles currently block.
    pub fn gate(&self) -> bool {
        self.gate_open.load(Ordering::Relaxed)
    }

    /// Whether triangle `t` takes part in queries: every ordinary one, a gate one while on.
    fn live(&self, t: u32) -> bool {
        if let Some((_, _, on)) = self
            .event_parts
            .iter()
            .find(|(_, range, _)| range.contains(&t))
        {
            return on.load(Ordering::Relaxed);
        }
        t < self.gate_from || self.gate_open.load(Ordering::Relaxed)
    }

    fn build(mesh: &CollisionMesh, gate_start: Option<usize>) -> Self {
        let mut gate_from = u32::MAX;
        let vertices: Vec<Vec3> = mesh.vertices.iter().map(|v| Vec3::from(*v)).collect();
        let mut triangles = Vec::with_capacity(mesh.indices.len() / 3);
        let mut normals = Vec::with_capacity(mesh.indices.len() / 3);
        let mut materials = Vec::with_capacity(mesh.indices.len() / 3);
        let mut cells: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        for (source, tri) in mesh.indices.as_chunks::<3>().0.iter().enumerate() {
            if gate_start == Some(source) {
                gate_from = triangles.len() as u32;
            }
            let [a, b, c] = tri.map(|i| vertices[i as usize]);
            let n = (b - a).cross(c - a);
            if n.length_squared() < 1e-12 {
                continue;
            }
            let t = triangles.len() as u32;
            triangles.push(*tri);
            normals.push(n.normalize());
            materials.push(mesh.materials.get(source).copied().unwrap_or(u32::MAX));
            let (x0, z0) = cell_of(a.x.min(b.x).min(c.x), a.z.min(b.z).min(c.z));
            let (x1, z1) = cell_of(a.x.max(b.x).max(c.x), a.z.max(b.z).max(c.z));
            for cx in x0..=x1 {
                for cz in z0..=z1 {
                    cells.entry((cx, cz)).or_default().push(t);
                }
            }
        }
        Self {
            vertices,
            triangles,
            normals,
            materials,
            cells,
            min_walk_ny: MAX_SLOPE_DEGREES.to_radians().cos(),
            gate_from,
            gate_open: AtomicBool::new(false),
            event_parts: Vec::new(),
        }
    }

    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    fn corners(&self, t: u32) -> [Vec3; 3] {
        self.triangles[t as usize].map(|i| self.vertices[i as usize])
    }

    /// Triangles in the cells overlapping a horizontal box, each once.
    fn gather(&self, min: Vec2, max: Vec2, out: &mut Vec<u32>) {
        out.clear();
        let (x0, z0) = cell_of(min.x, min.y);
        let (x1, z1) = cell_of(max.x, max.y);
        for cx in x0..=x1 {
            for cz in z0..=z1 {
                if let Some(c) = self.cells.get(&(cx, cz)) {
                    out.extend_from_slice(c);
                }
            }
        }
        if x0 != x1 || z0 != z1 {
            out.sort_unstable();
            out.dedup();
        }
    }

    /// The highest walkable surface under `(x, z)` whose height is at most `top`.
    pub fn floor(&self, x: f32, z: f32, top: f32) -> Option<f32> {
        self.floor_triangle(x, z, top).map(|(h, _)| h)
    }

    /// The hit material of the floor [`CollisionWorld::floor`] finds, when known.
    pub fn floor_material(&self, x: f32, z: f32, top: f32) -> Option<u32> {
        let (_, t) = self.floor_triangle(x, z, top)?;
        self.materials
            .get(t as usize)
            .copied()
            .filter(|&m| m != u32::MAX)
    }

    /// The highest walkable surface under `(x, z)` no higher than `top`: height and triangle.
    fn floor_triangle(&self, x: f32, z: f32, top: f32) -> Option<(f32, u32)> {
        let cell = self.cells.get(&cell_of(x, z))?;
        let mut best: Option<(f32, u32)> = None;
        for &t in cell {
            if matches!(self.materials[t as usize], 22 | 36 | 38)
                || self.normals[t as usize].y.abs() < self.min_walk_ny
                || !self.live(t)
            {
                continue;
            }
            let [a, b, c] = self.corners(t);
            if let Some(h) = height_on(x, z, a, b, c)
                && h <= top
                && best.is_none_or(|(b, _)| h > b)
            {
                best = Some((h, t));
            }
        }
        best
    }

    /// Every surface the vertical line through `(x, z)` crosses: (height, normal y), highest
    /// first. For debugging and tests.
    pub fn surfaces_at(&self, x: f32, z: f32) -> Vec<(f32, f32)> {
        let mut out: Vec<(f32, f32)> = self
            .cells
            .get(&cell_of(x, z))
            .into_iter()
            .flatten()
            .filter(|&&t| self.live(t))
            .filter_map(|&t| {
                let [a, b, c] = self.corners(t);
                Some((height_on(x, z, a, b, c)?, self.normals[t as usize].y))
            })
            .collect();
        out.sort_by(|a, b| b.0.total_cmp(&a.0));
        out
    }

    /// Distance along `dir` (unit length) to the first triangle within `max`.
    pub fn raycast(&self, origin: Vec3, dir: Vec3, max: f32) -> Option<f32> {
        let end = origin + dir * max;
        let mut tris = Vec::new();
        self.gather(
            Vec2::new(origin.x.min(end.x), origin.z.min(end.z)),
            Vec2::new(origin.x.max(end.x), origin.z.max(end.z)),
            &mut tris,
        );
        let mut best: Option<f32> = None;
        for t in tris {
            if matches!(self.materials[t as usize], 22 | 36 | 38) || !self.live(t) {
                continue;
            }
            let [a, b, c] = self.corners(t);
            if let Some(d) = ray_triangle(origin, dir, a, b, c)
                && d <= max
                && best.is_none_or(|b| d < b)
            {
                best = Some(d);
            }
        }
        best
    }

    /// Moves a body standing at `feet` by the horizontal `delta`, stopping at walls and
    /// sliding along them. Returns the horizontal displacement actually made.
    pub fn slide(&self, feet: [f32; 3], delta: [f32; 2]) -> [f32; 2] {
        let start = Vec3::from(feet);
        let d = Vec2::from(delta);
        let len = d.length();
        let steps = ((len / (RADIUS * 0.5)).ceil() as usize).clamp(1, 32);
        let mut p = start;
        let mut tris = Vec::new();
        let reach = RADIUS + len / steps as f32 + 0.05;
        self.gather(
            Vec2::new(p.x.min(p.x + d.x), p.z.min(p.z + d.y)) - reach,
            Vec2::new(p.x.max(p.x + d.x), p.z.max(p.z + d.y)) + reach,
            &mut tris,
        );
        for _ in 0..steps {
            p.x += d.x / steps as f32;
            p.z += d.y / steps as f32;
            for _ in 0..4 {
                let push = self.push_out(p, &tris);
                if push == Vec2::ZERO {
                    break;
                }
                p.x += push.x;
                p.z += push.y;
            }
        }
        [p.x - start.x, p.z - start.z]
    }

    /// Horizontal correction that takes the body's spheres out of the given triangles.
    fn push_out(&self, feet: Vec3, tris: &[u32]) -> Vec2 {
        let mut total = Vec2::ZERO;
        for h in SPHERES {
            let centre = feet + Vec3::Y * h;
            let mut push = Vec2::ZERO;
            for &t in tris {
                if matches!(self.materials[t as usize], 22 | 36 | 38) || !self.live(t) {
                    continue;
                }
                let [a, b, c] = self.corners(t);
                let q = closest_on_triangle(centre, a, b, c);
                let away = centre - q;
                let dist = away.length();
                if dist >= RADIUS {
                    continue;
                }
                // Push horizontally only; a contact straight above or below (beam, floor) has
                // no horizontal direction and is left to the floor query.
                let flat = Vec2::new(away.x, away.z);
                let dir = if flat.length_squared() > 1e-8 {
                    flat.normalize()
                } else {
                    let n = self.normals[t as usize];
                    let nf = Vec2::new(n.x, n.z);
                    if nf.length_squared() < 1e-4 {
                        continue;
                    }
                    nf.normalize()
                };
                // Depth along the horizontal direction needed to clear the triangle point.
                let needed = (RADIUS * RADIUS - away.y * away.y).max(0.0).sqrt();
                let depth = needed - flat.dot(dir);
                if depth > push.dot(dir) {
                    push += dir * (depth - push.dot(dir));
                }
            }
            if push.length_squared() > total.length_squared() {
                total = push;
            }
        }
        total
    }
}

impl Ground for CollisionWorld {
    fn water_surface(&self, x: f32, z: f32, top: f32) -> Option<WaterSurface> {
        self.cells
            .get(&cell_of(x, z))?
            .iter()
            .filter_map(|&t| {
                let material = self.materials[t as usize];
                if !matches!(material, 20..=24 | 35..=38) || !self.live(t) {
                    return None;
                }
                let [a, b, c] = self.corners(t);
                let height = height_on(x, z, a, b, c)?;
                (height <= top).then_some(WaterSurface { height, material })
            })
            .max_by(|a, b| a.height.total_cmp(&b.height))
    }
    fn height(&self, x: f32, z: f32) -> Option<f32> {
        self.floor(x, z, f32::INFINITY)
    }

    fn floor(&self, x: f32, z: f32, top: f32) -> Option<f32> {
        // Feet are not a point: look a few centimetres around too, so seams between triangles
        // do not drop the body.
        const R: f32 = 0.08;
        [(0.0, 0.0), (R, 0.0), (-R, 0.0), (0.0, R), (0.0, -R)]
            .into_iter()
            .filter_map(|(dx, dz)| CollisionWorld::floor(self, x + dx, z + dz, top))
            .reduce(f32::max)
    }

    fn slide(&self, feet: [f32; 3], delta: [f32; 2]) -> [f32; 2] {
        CollisionWorld::slide(self, feet, delta)
    }

    fn max_step(&self) -> f32 {
        STEP_HEIGHT
    }

    fn material(&self, x: f32, z: f32, top: f32) -> Option<u32> {
        if let Some(water) = self.water_surface(x, z, top)
            && water.deep()
            && self
                .floor(x, z, top)
                .is_none_or(|floor| floor < water.height)
        {
            return Some(water.material);
        }
        self.floor_material(x, z, top)
    }
}

impl<T: Ground + ?Sized> Ground for std::sync::Arc<T> {
    fn water_surface(&self, x: f32, z: f32, top: f32) -> Option<WaterSurface> {
        (**self).water_surface(x, z, top)
    }
    fn height(&self, x: f32, z: f32) -> Option<f32> {
        (**self).height(x, z)
    }

    fn floor(&self, x: f32, z: f32, top: f32) -> Option<f32> {
        (**self).floor(x, z, top)
    }

    fn slide(&self, feet: [f32; 3], delta: [f32; 2]) -> [f32; 2] {
        (**self).slide(feet, delta)
    }

    fn max_step(&self) -> f32 {
        (**self).max_step()
    }

    fn material(&self, x: f32, z: f32, top: f32) -> Option<u32> {
        (**self).material(x, z, top)
    }
}

/// Height of the triangle where the vertical line through `(x, z)` crosses it.
fn height_on(x: f32, z: f32, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let d = (b.z - c.z) * (a.x - c.x) + (c.x - b.x) * (a.z - c.z);
    if d.abs() < 1e-9 {
        return None;
    }
    let l1 = ((b.z - c.z) * (x - c.x) + (c.x - b.x) * (z - c.z)) / d;
    let l2 = ((c.z - a.z) * (x - c.x) + (a.x - c.x) * (z - c.z)) / d;
    let l3 = 1.0 - l1 - l2;
    const EPS: f32 = -1e-5;
    if l1 < EPS || l2 < EPS || l3 < EPS {
        return None;
    }
    Some(l1 * a.y + l2 * b.y + l3 * c.y)
}

/// Möller-Trumbore, both faces.
/// Returns the ray parameter `t >= 0` where `o + t * d` meets triangle `abc`.
pub fn ray_triangle(o: Vec3, d: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let e1 = b - a;
    let e2 = c - a;
    let p = d.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-9 {
        return None;
    }
    let inv = 1.0 / det;
    let s = o - a;
    let u = s.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = d.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t >= 0.0).then_some(t)
}

/// Closest point on triangle `abc` to `p` (Ericson, Real-Time Collision Detection 5.1.5).
fn closest_on_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let denom = 1.0 / (va + vb + vc);
    a + ab * (vb * denom) + ac * (vc * denom)
}

#[path = "arena_c_collision.rs"]
mod arena_c_collision;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::{Body, BodyInput};

    /// A floor at y 0, a 0.3 m step up at z < -4, a 1.5 m ledge at z < -8 and a wall at x = 3.
    fn test_world() -> CollisionWorld {
        let mut m = CollisionMesh::default();
        let mut quad = |p: [[f32; 3]; 4]| {
            let base = m.vertices.len() as u32;
            m.vertices.extend_from_slice(&p);
            m.indices
                .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
            m.materials.extend([0, 0]);
        };
        let floor = |y: f32, z0: f32, z1: f32| {
            [[-10.0, y, z0], [10.0, y, z0], [10.0, y, z1], [-10.0, y, z1]]
        };
        quad(floor(0.0, 10.0, -4.0));
        quad(floor(0.3, -4.0, -8.0));
        quad(floor(1.5, -8.0, -20.0));
        // Riser faces.
        quad([
            [-10.0, 0.0, -4.0],
            [10.0, 0.0, -4.0],
            [10.0, 0.3, -4.0],
            [-10.0, 0.3, -4.0],
        ]);
        quad([
            [-10.0, 0.3, -8.0],
            [10.0, 0.3, -8.0],
            [10.0, 1.5, -8.0],
            [-10.0, 1.5, -8.0],
        ]);
        // A wall along x = 3.
        quad([
            [3.0, -1.0, 10.0],
            [3.0, -1.0, -20.0],
            [3.0, 5.0, -20.0],
            [3.0, 5.0, 10.0],
        ]);
        CollisionWorld::from_mesh(&m)
    }

    fn walk(body: &mut Body, world: &CollisionWorld, local: [f32; 2], steps: usize) {
        for _ in 0..steps {
            body.step(
                &BodyInput {
                    // Source space: forward is -Z, and X is mirrored into Bevy space.
                    root_motion: [-local[0], 0.0, local[1], 0.0],
                    ..BodyInput::default()
                },
                1.0 / 60.0,
                world,
            );
        }
    }

    #[test]
    fn steps_up_small_ledges_but_not_tall_ones() {
        let world = test_world();
        let mut b = Body::default();
        // Walk forward (-Z) 0.08 m per tick for 2.5 s.
        walk(&mut b, &world, [0.0, -0.08], 150);
        assert!(
            (b.position[1] - 0.3).abs() < 1e-4,
            "on the step: {:?}",
            b.position
        );
        assert!(b.grounded);
        // Stopped by the 1.2 m riser at z = -8, one radius away.
        assert!(
            (b.position[2] + 8.0 - RADIUS).abs() < 0.05,
            "{:?}",
            b.position
        );
    }

    #[test]
    fn slides_along_walls() {
        let world = test_world();
        let mut b = Body::default();
        // Diagonal toward +X and -Z: the wall at x = 3 stops X, Z keeps going.
        walk(&mut b, &world, [0.06, -0.03], 100);
        assert!(
            (b.position[0] - (3.0 - RADIUS)).abs() < 0.03,
            "{:?}",
            b.position
        );
        assert!(b.position[2] < -2.5, "{:?}", b.position);
    }

    #[test]
    fn walks_off_edges_and_falls() {
        let world = test_world();
        let mut b = Body {
            position: [0.0, 1.5, -12.0],
            ..Body::default()
        };
        // Walk back (+Z) off the ledge at z = -8.
        walk(&mut b, &world, [0.0, 0.08], 90);
        assert!(b.grounded);
        assert!((b.position[1] - 0.3).abs() < 1e-4, "{:?}", b.position);
        assert!(b.last_fall_height > 1.0, "fell {}", b.last_fall_height);
    }

    #[test]
    fn raycast_hits_wall() {
        let world = test_world();
        let d = world.raycast(Vec3::new(0.0, 1.0, 0.0), Vec3::X, 10.0);
        assert!((d.unwrap() - 3.0).abs() < 1e-4);
    }

    #[test]
    fn floor_material_is_the_highest_floor_s() {
        // A cobblestone floor (1) under a wooden one (4) 1 m higher.
        let mut m = CollisionMesh::default();
        for (y, material) in [(0.0, 1), (1.0, 4)] {
            let base = m.vertices.len() as u32;
            m.vertices.extend_from_slice(&[
                [-5.0, y, -5.0],
                [5.0, y, -5.0],
                [5.0, y, 5.0],
                [-5.0, y, 5.0],
            ]);
            m.indices
                .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
            m.materials.extend([material, material]);
        }
        let world = CollisionWorld::from_mesh(&m);
        assert_eq!(world.floor_material(0.0, 0.0, 2.0), Some(4));
        assert_eq!(world.floor_material(0.0, 0.0, 0.5), Some(1));
        assert_eq!(Ground::material(&world, 0.0, 0.0, 0.5), Some(1));
        assert_eq!(world.floor_material(9.0, 0.0, 2.0), None);
    }
}

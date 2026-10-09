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

use glam::{Vec2, Vec3};
use sekiro_formats::hknp::CollisionMesh;

use crate::body::Ground;

/// Body radius in metres (an assumption; Sekiro's character capsule size is not read yet).
pub const RADIUS: f32 = 0.35;
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
    cells: HashMap<(i32, i32), Vec<u32>>,
    min_walk_ny: f32,
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

    pub fn from_mesh(mesh: &CollisionMesh) -> Self {
        let vertices: Vec<Vec3> = mesh.vertices.iter().map(|v| Vec3::from(*v)).collect();
        let mut triangles = Vec::with_capacity(mesh.indices.len() / 3);
        let mut normals = Vec::with_capacity(mesh.indices.len() / 3);
        let mut cells: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        for tri in mesh.indices.as_chunks::<3>().0 {
            let [a, b, c] = tri.map(|i| vertices[i as usize]);
            let n = (b - a).cross(c - a);
            if n.length_squared() < 1e-12 {
                continue;
            }
            let t = triangles.len() as u32;
            triangles.push(*tri);
            normals.push(n.normalize());
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
            cells,
            min_walk_ny: MAX_SLOPE_DEGREES.to_radians().cos(),
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
        let cell = self.cells.get(&cell_of(x, z))?;
        let mut best: Option<f32> = None;
        for &t in cell {
            if self.normals[t as usize].y.abs() < self.min_walk_ny {
                continue;
            }
            let [a, b, c] = self.corners(t);
            if let Some(h) = height_on(x, z, a, b, c)
                && h <= top
                && best.is_none_or(|b| h > b)
            {
                best = Some(h);
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
}

impl<T: Ground + ?Sized> Ground for std::sync::Arc<T> {
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
fn ray_triangle(o: Vec3, d: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
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
}

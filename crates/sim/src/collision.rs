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
use sekiro_formats::hknp::{CollisionMesh, ObjectCollision, ObjectCollisionPart};

use crate::body::{Ground, WaterSurface};
use crate::hit_filter::{CAMERA, ENEMY, PLAYER};

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

/// Furthest the grid part of the local pair search ([`pair_path_shift`]) looks around the wall
/// projection; the ring along the blocking surface's normal still goes out to the full limit.
const PAIR_SEARCH_REACH: f32 = 1.0;
/// How many whole-path checks' worth of ground queries one pair search may spend on candidate
/// shifts before it gives up and the throw falls back to per-step placement. Each candidate
/// checks the whole paired path, so an unbounded search at a cramped spot stalled a
/// deathblow's start for seconds (#156); a check that fails early costs less, so more
/// candidates than this usually get tried. The wall projection ([`throw_wall_shift`]) and
/// the clearance direction before the search are not counted.
const PAIR_SEARCH_CHECKS: usize = 20;
/// A ceiling on that budget, so a long throw path (each check costs more) still gives up within
/// a frame or two; a short doorway path needing its 17th candidate spends about 1,700.
const PAIR_SEARCH_MAX_QUERIES: usize = 4000;

/// A [`Ground`] that counts the floor and wall queries made through it. It forwards only what
/// [`pair_path_clear`] asks.
struct Counted<'a> {
    inner: &'a dyn Ground,
    queries: &'a std::cell::Cell<usize>,
}

impl Counted<'_> {
    fn count(&self) {
        self.queries.set(self.queries.get() + 1);
    }
}

impl Ground for Counted<'_> {
    fn height(&self, x: f32, z: f32) -> Option<f32> {
        self.count();
        self.inner.height(x, z)
    }

    fn floor(&self, x: f32, z: f32, top: f32) -> Option<f32> {
        self.count();
        self.inner.floor(x, z, top)
    }

    fn slide(&self, feet: [f32; 3], delta: [f32; 2]) -> [f32; 2] {
        self.count();
        self.inner.slide(feet, delta)
    }

    fn max_step(&self) -> f32 {
        self.inner.max_step()
    }
}

/// Sub-steps [`pair_path_clear`] samples the footprint at along a horizontal move `delta`.
fn path_steps(delta: [f32; 2]) -> usize {
    (delta[0].hypot(delta[1]) / 0.1).ceil().max(1.0) as usize
}

/// The ground queries one [`pair_path_clear`] of `path` makes when every sample passes, for
/// a small shift.
fn path_check_queries(path: &[[[f32; 3]; 2]]) -> usize {
    (0..2)
        .map(|i| {
            let segments = path
                .windows(2)
                .map(|w| path_steps([w[1][i][0] - w[0][i][0], w[1][i][2] - w[0][i][2]]));
            // The shift itself, then each segment: five footprint floors per sample, a floor
            // and a slide; then a floor and a slide for the end.
            std::iter::once(1)
                .chain(segments)
                .map(|steps| 5 * (steps + 1) + 2)
                .sum::<usize>()
                + 2
        })
        .sum()
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
    // Try the blocking surface's normal first. A wide paired clip at a door can
    // need metres of clearance; probing every direction at every radius stalls play.
    let normal_rings = (limit / 0.1).round() as usize;
    let normal = std::iter::once_with(|| {
        (limit > 0.2)
            .then(|| pair_clearance_direction(grounds, path))
            .flatten()
    })
    .flatten()
    .flat_map(move |direction| {
        (1..=normal_rings).map(move |ring| {
            let radius = ring as f32 * 0.1;
            [
                preferred[0] + direction[0] * radius,
                preferred[1] + direction[1] * radius,
            ]
        })
    });
    // Search locally around the wall projection for supported pair space. This only
    // constrains authored paired placement; ordinary movement may still leave a ledge.
    // Small step corrections search a fine grid; a relocation a coarse one within
    // `PAIR_SEARCH_REACH` (every 0.1 m in 16 directions out to 4 m was 640 path checks).
    let (spacing, directions) = if limit > 0.2 { (0.25, 8) } else { (0.1, 16) };
    let grid_rings = (limit.min(PAIR_SEARCH_REACH) / spacing).round() as usize;
    let grid = (1..=grid_rings).flat_map(move |ring| {
        let radius = ring as f32 * spacing;
        (0..directions).map(move |direction| {
            let angle = direction as f32 * std::f32::consts::TAU / directions as f32;
            [
                preferred[0] + radius * angle.sin(),
                preferred[1] + radius * angle.cos(),
            ]
        })
    });
    let queries = std::cell::Cell::new(0);
    let budget = (PAIR_SEARCH_CHECKS * path_check_queries(path)).min(PAIR_SEARCH_MAX_QUERIES);
    let counted = grounds.map(|inner| Counted {
        inner,
        queries: &queries,
    });
    let counted: [&dyn Ground; 2] = [&counted[0], &counted[1]];
    std::iter::once(preferred)
        .chain(normal)
        .chain(grid)
        .filter(|shift| shift[0].hypot(shift[1]) <= limit)
        .take_while(|_| queries.get() < budget)
        .find(|&shift| pair_path_clear(counted, path, shift, sweep_motion))
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
            let steps = path_steps(delta);
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
/// Havok character proxy default, FUN_14104d3d0 (FwLo's RustSekiro, map.rs).
pub const MAX_SLOPE_DEGREES: f32 = 60.0;
/// Heights of the collision spheres' centres above the feet. The lowest sphere's bottom sits
/// at the step height, so stairs reach the floor query rather than the wall push.
const SPHERES: [f32; 3] = [STEP_HEIGHT + RADIUS, 1.2, 1.55];
/// The player's physics capsule height (see [`RADIUS`]).
pub const CAPSULE_HEIGHT: f32 = 1.5;
/// The top of the body's top collision sphere above the feet: where its head meets a ceiling.
pub const HEAD_TOP: f32 = SPHERES[SPHERES.len() - 1] + RADIUS;
/// How close the fall-prevention assist lets a body come to its walls: `FallAssistMinRadius`,
/// physics module `+0xaec`, 0.16 m from the constructor `FUN_140bbb010` (`+0xae8`,
/// `FallAssistMaxRadius`, is 0.4). `FUN_140bc68d0` slides the body's move along the walls its
/// sphere cast meets and cuts it where the body would come nearer than this to one.
pub const FALL_ASSIST_RADIUS: f32 = 0.16;
const CELL: f32 = 4.0;
/// How far inside the head sphere's rim, seen from above, a ceiling must reach to stop a rise
/// ([`CollisionWorld::ceiling_for`]). The slide leaves a body pressed against a wall at
/// [`RADIUS`] from it, give or take roundoff, so the wall's top edge passes a hair inside the
/// rim; skipping such a face lets the sphere overlap it by at most this.
const CEILING_GRAZE: f32 = 1e-3;
/// `CollisionWorld::part_of` for a triangle in no helper or event part.
const ORDINARY: u16 = 0;
/// `CollisionWorld::part_of` bit marking an event part's triangle.
const EVENT_PART: u16 = 0x8000;

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
    /// Map event collision switched by entity: triangle range, who collides with it (as in
    /// `masked`) and whether it is on.
    event_parts: Vec<(i32, std::ops::Range<u32>, u8, AtomicBool)>,
    /// Helper parts (see [`crate::hit_filter`]): who collides with each
    /// ([`crate::hit_filter::PLAYER`], `ENEMY`, `CAMERA` bits). Every other triangle is for all.
    masked: Vec<u8>,
    /// Per triangle, the part [`CollisionWorld::live`] asks about: [`ORDINARY`], `1 + i` for
    /// `masked[i]` or [`EVENT_PART`]` | i` for `event_parts[i]`.
    part_of: Vec<u16>,
    /// Per triangle, the corner-wise minimum and maximum of its corners: its bounding box.
    bounds: Vec<[Vec3; 2]>,
    /// Map action event walls: untouchable triangles whose contact runs a reaction.
    triggers: Vec<TriggerPart>,
    /// Placed objects' triangle ranges ([`CollisionWorld::add_objects`]) and `<block>/<part>`
    /// names, in triangle order.
    objects: Vec<(std::ops::Range<u32>, String)>,
}

/// One event wall part, kept out of the walkable grid.
struct TriggerPart {
    hit_filter: u8,
    part: String,
    min: Vec3,
    max: Vec3,
    triangles: Vec<[Vec3; 3]>,
}

/// A body's contact with an event wall ([`CollisionWorld::trigger_contact`]).
#[derive(Debug, Clone, PartialEq)]
pub struct TriggerContact {
    pub hit_filter: u8,
    pub part: String,
    /// Where the body's path met the wall.
    pub point: Vec3,
}

/// A triangle's unit normal, `None` for one too thin to collide with (left out of the grid).
fn face_normal(a: Vec3, b: Vec3, c: Vec3) -> Option<Vec3> {
    let n = (b - a).cross(c - a);
    (n.length_squared() >= 1e-12).then(|| n.normalize())
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

    /// A map's `collision.bin` as the player collides with it: the helper parts its
    /// `layout.json` names leave the walkable mesh and join as helpers
    /// ([`crate::hit_filter::strip_helpers`], [`CollisionWorld::add_helpers`]). Without a
    /// layout it is [`CollisionWorld::load`].
    pub fn load_map(dir: &Path) -> Result<Self, String> {
        let path = dir.join("collision.bin");
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut mesh = CollisionMesh::from_bytes(&bytes).map_err(|e| e.to_string())?;
        let helpers = match std::fs::read(dir.join("layout.json")) {
            Ok(layout) => crate::hit_filter::strip_helpers(
                &mut mesh,
                &serde_json::from_slice(&layout).map_err(|e| e.to_string())?,
            ),
            Err(_) => Default::default(),
        };
        let mut world = Self::from_mesh(&mesh);
        world.add_helpers(&helpers);
        Ok(world)
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

    /// Whether triangle `t` takes part in `who`'s queries: every ordinary one (except for the
    /// fall-prevention assist's), a helper one when it holds `who`, an event one while on and
    /// holding `who`, a gate one while on.
    fn live(&self, t: u32, who: u8) -> bool {
        match self.part_of[t as usize] {
            ORDINARY => {
                who & (PLAYER | ENEMY | CAMERA) != 0
                    && (t < self.gate_from || self.gate_open.load(Ordering::Relaxed))
            }
            part if part & EVENT_PART != 0 => {
                let (_, _, mask, on) = &self.event_parts[usize::from(part & !EVENT_PART)];
                mask & who != 0 && on.load(Ordering::Relaxed)
            }
            part => self.masked[usize::from(part) - 1] & who != 0,
        }
    }

    /// Whether triangle `t` is a wall or floor at all: deep water's are neither
    /// ([`crate::body::deep_water`]).
    fn solid(&self, t: u32) -> bool {
        !crate::body::deep_water(self.materials[t as usize])
    }

    /// Whether triangle `t` is a solid face flat enough to stand on in `who`'s queries. The
    /// winding does not say which side is solid (see the module notes), so it is a floor from
    /// above and a ceiling from below.
    fn walkable(&self, t: u32, who: u8) -> bool {
        self.normals[t as usize].y.abs() >= self.min_walk_ny && self.solid(t) && self.live(t, who)
    }

    /// Records that the triangles `range` belong to `part` (see `part_of`).
    fn mark_part(&mut self, range: &std::ops::Range<u32>, part: u16) {
        self.part_of[range.start as usize..range.end as usize].fill(part);
    }

    /// Adds an event part over the just appended triangles `range` (see `event_parts`).
    pub(crate) fn add_event_part(
        &mut self,
        entity: i32,
        range: std::ops::Range<u32>,
        who: u8,
        on: bool,
    ) {
        let part = u16::try_from(self.event_parts.len())
            .ok()
            .filter(|&i| i & EVENT_PART == 0)
            .expect("too many event parts");
        self.mark_part(&range, EVENT_PART | part);
        self.event_parts
            .push((entity, range, who, AtomicBool::new(on)));
    }

    fn build(mesh: &CollisionMesh, gate_start: Option<usize>) -> Self {
        let mut gate_from = u32::MAX;
        let vertices: Vec<Vec3> = mesh.vertices.iter().map(|v| Vec3::from(*v)).collect();
        let mut triangles = Vec::with_capacity(mesh.indices.len() / 3);
        let mut normals = Vec::with_capacity(mesh.indices.len() / 3);
        let mut materials = Vec::with_capacity(mesh.indices.len() / 3);
        let mut bounds = Vec::with_capacity(mesh.indices.len() / 3);
        let mut cells: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        for (source, tri) in mesh.indices.as_chunks::<3>().0.iter().enumerate() {
            if gate_start == Some(source) {
                gate_from = triangles.len() as u32;
            }
            let [a, b, c] = tri.map(|i| vertices[i as usize]);
            let Some(n) = face_normal(a, b, c) else {
                continue;
            };
            let t = triangles.len() as u32;
            triangles.push(*tri);
            normals.push(n);
            materials.push(mesh.materials.get(source).copied().unwrap_or(u32::MAX));
            let (lo, hi) = (a.min(b).min(c), a.max(b).max(c));
            bounds.push([lo, hi]);
            let (x0, z0) = cell_of(lo.x, lo.z);
            let (x1, z1) = cell_of(hi.x, hi.z);
            for cx in x0..=x1 {
                for cz in z0..=z1 {
                    cells.entry((cx, cz)).or_default().push(t);
                }
            }
        }
        Self {
            part_of: vec![ORDINARY; triangles.len()],
            vertices,
            triangles,
            normals,
            materials,
            bounds,
            cells,
            min_walk_ny: MAX_SLOPE_DEGREES.to_radians().cos(),
            gate_from,
            gate_open: AtomicBool::new(false),
            event_parts: Vec::new(),
            masked: Vec::new(),
            triggers: Vec::new(),
            objects: Vec::new(),
        }
    }

    /// Adds the helper collision [`crate::hit_filter::strip_helpers`] took out of the
    /// player's: enemy and camera parts to the grid, queried only by those they hold
    /// ([`EnemyGround`], [`CollisionWorld::camera_raycast`]), and the event walls to the contact
    /// list.
    pub fn add_helpers(&mut self, helpers: &crate::hit_filter::HelperCollision) {
        for (&mask, mesh) in &helpers.parts {
            self.add_masked(mask, mesh);
        }
        for trigger in &helpers.triggers {
            let mesh = &trigger.mesh;
            let triangles: Vec<[Vec3; 3]> = mesh
                .indices
                .as_chunks::<3>()
                .0
                .iter()
                .map(|t| t.map(|i| Vec3::from(mesh.vertices[i as usize])))
                .collect();
            if triangles.is_empty() {
                continue;
            }
            let all = triangles.iter().flatten();
            let min = all.clone().copied().reduce(Vec3::min).unwrap();
            let max = all.copied().reduce(Vec3::max).unwrap();
            self.triggers.push(TriggerPart {
                hit_filter: trigger.hit_filter,
                part: trigger.part.clone(),
                min,
                max,
                triangles,
            });
        }
    }

    /// Adds the collision of a block's placed objects (`object_collision.bin`), each with the
    /// mask `mask` gives it from its part and [`crate::hit_filter::object_mask`] (0 leaves it
    /// out): objects everyone collides with as ordinary triangles, the rest as helpers of their
    /// mask. Each mask's objects join the grid as one mesh, and each object is recorded by its
    /// triangles in it as `<block>/<part>`, since part names repeat across blocks. Returns the
    /// parts and triangles added.
    pub fn add_objects(
        &mut self,
        block: &str,
        objects: &ObjectCollision,
        mut mask: impl FnMut(&ObjectCollisionPart, u8) -> u8,
    ) -> (usize, usize) {
        let mut by_mask: std::collections::BTreeMap<u8, Vec<&ObjectCollisionPart>> =
            Default::default();
        for part in &objects.parts {
            let who = crate::hit_filter::object_mask(part.collision_type);
            let who = if who == 0 { 0 } else { mask(part, who) };
            if who != 0 {
                by_mask.entry(who).or_default().push(part);
            }
        }
        let source = &objects.mesh;
        let (mut parts, first) = (0, self.triangles.len());
        // Source vertex -> its index in the group's mesh.
        let mut slot = vec![u32::MAX; source.vertices.len()];
        for (who, group) in by_mask {
            slot.fill(u32::MAX);
            let mut mesh = CollisionMesh::default();
            let mut ranges = Vec::with_capacity(group.len());
            for part in group {
                let start = mesh.triangle_count() as u32;
                for t in part.first as usize..(part.first + part.count) as usize {
                    let tri = &source.indices[t * 3..t * 3 + 3];
                    let [a, b, c] = [0, 1, 2].map(|k| Vec3::from(source.vertices[tri[k] as usize]));
                    // Left out here as the grid would, so the ranges count what it keeps.
                    if face_normal(a, b, c).is_none() {
                        continue;
                    }
                    for &i in tri {
                        if slot[i as usize] == u32::MAX {
                            slot[i as usize] = mesh.vertices.len() as u32;
                            mesh.vertices.push(source.vertices[i as usize]);
                        }
                        mesh.indices.push(slot[i as usize]);
                    }
                    mesh.materials
                        .push(source.materials.get(t).copied().unwrap_or(u32::MAX));
                }
                ranges.push((start..mesh.triangle_count() as u32, &part.name));
            }
            let range = self.add_masked(who, &mesh);
            for (r, name) in ranges {
                let r = range.start + r.start..range.start + r.end;
                self.objects.push((r, format!("{block}/{name}")));
                parts += 1;
            }
        }
        (parts, self.triangles.len() - first)
    }

    /// [`CollisionWorld::add_objects`] with a block's `object_collision.bin` in its map folder
    /// `dir`, if it has one (none adds nothing).
    pub fn add_block_objects(
        &mut self,
        dir: &Path,
        block: &str,
        mask: impl FnMut(&ObjectCollisionPart, u8) -> u8,
    ) -> Result<(usize, usize), String> {
        let path = dir.join("object_collision.bin");
        if !path.is_file() {
            return Ok((0, 0));
        }
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let objects = ObjectCollision::from_bytes(&bytes).map_err(|e| e.to_string())?;
        Ok(self.add_objects(block, &objects, mask))
    }

    /// Appends a mesh only those `mask` holds collide with ([`crate::hit_filter::PLAYER`],
    /// `ENEMY`, `CAMERA` bits); with all three bits it is ordinary collision.
    fn add_masked(&mut self, mask: u8, mesh: &CollisionMesh) -> std::ops::Range<u32> {
        let range = self.append(mesh);
        if mask != PLAYER | ENEMY | CAMERA && !range.is_empty() {
            self.masked.push(mask);
            let part = u16::try_from(self.masked.len())
                .ok()
                .filter(|&i| i & EVENT_PART == 0)
                .expect("too many helper parts");
            self.mark_part(&range, part);
        }
        range
    }

    /// The placed object (`<block>/<part>`, [`CollisionWorld::add_objects`]) a triangle belongs
    /// to, if it is an object's.
    pub fn object_at(&self, triangle: u32) -> Option<&str> {
        let i = self
            .objects
            .partition_point(|(range, _)| range.end <= triangle);
        self.objects
            .get(i)
            .filter(|(range, _)| range.contains(&triangle))
            .map(|(_, name)| name.as_str())
    }

    /// The placed object whose collision is the floor [`CollisionWorld::floor`] finds, if the
    /// floor there is an object's.
    pub fn floor_object(&self, x: f32, z: f32, top: f32) -> Option<&str> {
        self.object_at(self.floor_hit(x, z, top)?.1)
    }

    /// Appends a mesh's triangles to the grid; the caller records what they are.
    pub(crate) fn append(&mut self, mesh: &CollisionMesh) -> std::ops::Range<u32> {
        let part = Self::from_mesh(mesh);
        let vertex_offset = self.vertices.len() as u32;
        let first = self.triangles.len() as u32;
        let last = first + part.triangles.len() as u32;
        self.vertices.extend(part.vertices);
        self.triangles.extend(
            part.triangles
                .into_iter()
                .map(|tri| tri.map(|i| i + vertex_offset)),
        );
        self.normals.extend(part.normals);
        self.materials.extend(part.materials);
        self.bounds.extend(part.bounds);
        self.part_of.resize(self.triangles.len(), ORDINARY);
        for (cell, triangles) in part.cells {
            self.cells
                .entry(cell)
                .or_default()
                .extend(triangles.into_iter().map(|t| t + first));
        }
        first..last
    }

    /// The first event wall a body's capsule meets moving from feet `from` to feet `to`:
    /// the path is tested at the capsule's bottom, middle and top.
    pub fn trigger_contact(&self, from: Vec3, to: Vec3) -> Option<TriggerContact> {
        let travel = to - from;
        let length = travel.length();
        if length < 1e-4 {
            return None;
        }
        let dir = travel / length;
        let lo = from.min(to);
        let hi = from.max(to) + Vec3::Y * CAPSULE_HEIGHT;
        let mut best: Option<(f32, TriggerContact)> = None;
        for part in &self.triggers {
            if (hi.cmplt(part.min - RADIUS)).any() || (lo.cmpgt(part.max + RADIUS)).any() {
                continue;
            }
            for h in [0.05, CAPSULE_HEIGHT * 0.5, CAPSULE_HEIGHT - 0.05] {
                let origin = from + Vec3::Y * h;
                for &[a, b, c] in &part.triangles {
                    if let Some(d) = ray_triangle(origin, dir, a, b, c)
                        && d <= length
                        && best.as_ref().is_none_or(|(b, _)| d < *b)
                    {
                        best = Some((
                            d,
                            TriggerContact {
                                hit_filter: part.hit_filter,
                                part: part.part.clone(),
                                point: origin + dir * d,
                            },
                        ));
                    }
                }
            }
        }
        best.map(|(_, c)| c)
    }

    /// Whether an event wall with the fall return's hit filter lies below `(x, y, z)`.
    pub fn fall_return_below(&self, x: f32, y: f32, z: f32) -> bool {
        self.triggers
            .iter()
            .filter(|p| p.hit_filter == crate::hit_filter::FALL_RETURN_FILTER)
            .filter(|p| p.min.y < y && (p.min.x..=p.max.x).contains(&x))
            .filter(|p| (p.min.z..=p.max.z).contains(&z))
            .any(|p| {
                p.triangles
                    .iter()
                    .any(|&[a, b, c]| height_on(x, z, a, b, c).is_some_and(|h| h < y))
            })
    }

    /// Whether a body's capsule standing at `feet` is still within reach of an event wall part
    /// (its bounds widened by the capsule radius): one contact lasts until it leaves.
    pub fn near_trigger(&self, part: &str, feet: Vec3) -> bool {
        let lo = feet;
        let hi = feet + Vec3::Y * CAPSULE_HEIGHT;
        self.triggers.iter().any(|p| {
            p.part == part && !(hi.cmplt(p.min - RADIUS)).any() && !(lo.cmpgt(p.max + RADIUS)).any()
        })
    }

    /// Distance along `dir` (unit length) to the first triangle the camera ray stops at within
    /// `max`: [`CollisionWorld::raycast`] with the camera's helper parts instead of the player's.
    pub fn camera_raycast(&self, origin: Vec3, dir: Vec3, max: f32) -> Option<f32> {
        self.ray(origin, dir, max, CAMERA).0
    }

    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    fn corners(&self, t: u32) -> [Vec3; 3] {
        self.triangles[t as usize].map(|i| self.vertices[i as usize])
    }

    /// The highest walkable surface under `(x, z)` whose height is at most `top`.
    pub fn floor(&self, x: f32, z: f32, top: f32) -> Option<f32> {
        self.floor_hit(x, z, top).map(|(h, _)| h)
    }

    /// [`CollisionWorld::floor`] with the floor's triangle, for [`CollisionWorld::object_at`].
    pub fn floor_hit(&self, x: f32, z: f32, top: f32) -> Option<(f32, u32)> {
        self.floor_triangle(x, z, top, PLAYER)
    }

    /// The hit material of the floor [`CollisionWorld::floor`] finds, when known.
    pub fn floor_material(&self, x: f32, z: f32, top: f32) -> Option<u32> {
        self.floor_material_for(x, z, top, PLAYER)
    }

    fn floor_material_for(&self, x: f32, z: f32, top: f32, who: u8) -> Option<u32> {
        let (_, t) = self.floor_triangle(x, z, top, who)?;
        self.triangle_material(t)
    }

    /// Triangle `t`'s hit material, when known.
    fn triangle_material(&self, t: u32) -> Option<u32> {
        self.materials
            .get(t as usize)
            .copied()
            .filter(|&m| m != u32::MAX)
    }

    /// The highest walkable surface under `(x, z)` no higher than `top`: height and triangle.
    fn floor_triangle(&self, x: f32, z: f32, top: f32, who: u8) -> Option<(f32, u32)> {
        let cell = self.cells.get(&cell_of(x, z))?;
        let mut best: Option<(f32, u32)> = None;
        for &t in cell {
            if !self.walkable(t, who) {
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

    /// The highest the feet of `who`'s body at `feet` can rise by at most `rise` before its top
    /// sphere meets a ceiling over it: any surface flat enough to be a floor (the winding does
    /// not say which side is solid, see the module notes) above that sphere's centre. The sphere
    /// is swept straight up against each such face, its interior, edges and corners, so a beam's
    /// edge over the rim or a sloping underside stops it where it first touches (#170). Only a
    /// face the sphere rises into counts: not one it is already in at the start, nor one that
    /// reaches no more than [`CEILING_GRAZE`] inside its rim, such as the top edge of a wall the
    /// body is pressed against. When `Some`, it is below `feet.y + rise`.
    // TODO(#163): this scans whole 4 m cells; a height-banded grid would cut that.
    fn ceiling_for(&self, feet: Vec3, rise: f32, who: u8) -> Option<f32> {
        let centre = feet + Vec3::Y * (HEAD_TOP - RADIUS);
        let mut best: Option<f32> = None;
        for t in self.ceiling_candidates(feet, rise, who) {
            if let Some(up) = self.rise_into(t, centre, rise) {
                best = Some(best.map_or(up, |b| b.min(up)));
            }
        }
        best.map(|up| feet.y + up)
    }

    /// The triangles [`CollisionWorld::ceiling_for`] sweeps the head sphere against, each once:
    /// faces flat enough to be a ceiling, reaching from above the sphere's centre to below its
    /// top after `rise`, whose boxes come within the sphere's rim of its centre line.
    fn ceiling_candidates(&self, feet: Vec3, rise: f32, who: u8) -> impl Iterator<Item = u32> {
        let centre = feet.y + HEAD_TOP - RADIUS;
        let high = feet.y + HEAD_TOP + rise;
        let line = Vec2::new(feet.x, feet.z);
        let reach = RADIUS - CEILING_GRAZE;
        let (min, max) = (line - RADIUS, line + RADIUS);
        let (x0, z0) = cell_of(min.x, min.y);
        let (x1, z1) = cell_of(max.x, max.y);
        (x0..=x1)
            .flat_map(move |cx| (z0..=z1).map(move |cz| (cx, cz)))
            .filter_map(|key| Some((key, self.cells.get(&key)?)))
            .flat_map(move |(key, cell)| {
                cell.iter().copied().filter(move |&t| {
                    let [lo, hi] = self.bounds[t as usize];
                    let (lo2, hi2) = (Vec2::new(lo.x, lo.z), Vec2::new(hi.x, hi.z));
                    // Tested by the first cell of the scan that holds it.
                    hi.y >= centre
                        && lo.y <= high
                        && line.distance_squared(line.clamp(lo2, hi2)) < reach * reach
                        && cell_of(lo.x.max(min.x), lo.z.max(min.y)) == key
                        && self.walkable(t, who)
                })
            })
    }

    /// How far the sphere of [`RADIUS`] at `centre` rises, less than `rise`, before it first
    /// touches triangle `t`: the least rise at which it meets the face's interior, one of its
    /// edges or one of its corners, 0 if it touches it already. `None` when it does not touch it
    /// within `rise`, when it is already more than [`CEILING_GRAZE`] into it, or when the face
    /// reaches no further than that inside its rim, seen from above.
    fn rise_into(&self, t: u32, centre: Vec3, rise: f32) -> Option<f32> {
        let [a, b, c] = self.corners(t);
        let line = centre.with_y(0.0);
        let reach = RADIUS - CEILING_GRAZE;
        let q = closest_on_triangle(line, a.with_y(0.0), b.with_y(0.0), c.with_y(0.0));
        if q.distance_squared(line) >= reach * reach {
            return None;
        }
        let r2 = RADIUS * RADIUS;
        let mut first = f32::INFINITY;
        // The interior: from wholly below the face's plane up to touching it.
        let n = self.normals[t as usize];
        let n = if n.y < 0.0 { -n } else { n };
        let below = n.dot(centre - a);
        if below <= -RADIUS {
            let up = (-RADIUS - below) / n.y;
            let touch = centre + Vec3::Y * up + n * RADIUS;
            if height_on(touch.x, touch.z, a, b, c).is_some() {
                first = up;
            }
        }
        // The edges: the vertical line meets the cylinder of RADIUS round each, between its ends.
        for (p, e) in [(a, b - a), (b, c - b), (c, a - c)] {
            let ee = e.length_squared();
            let m = centre - p;
            let m_off = m - e * (m.dot(e) / ee);
            let y_off = Vec3::Y - e * (e.y / ee);
            let aa = y_off.length_squared();
            let bb = m_off.dot(y_off);
            let disc = bb * bb - aa * (m_off.length_squared() - r2);
            // A vertical edge is met first at a corner.
            if aa < 1e-9 || disc <= 0.0 {
                continue;
            }
            let root = disc.sqrt();
            let up = (-bb - root) / aa;
            let along = (m.dot(e) + up * e.y) / ee;
            if -bb + root > 0.0 && (0.0..=1.0).contains(&along) {
                first = first.min(up);
            }
        }
        // The corners.
        for p in [a, b, c] {
            let d2 = (p - centre).with_y(0.0).length_squared();
            if d2 < r2 {
                let (dy, half) = (p.y - centre.y, (r2 - d2).sqrt());
                if dy + half > 0.0 {
                    first = first.min(dy - half);
                }
            }
        }
        if first >= rise {
            return None;
        }
        if first < 0.0 {
            // Already touching: within the graze it stops the rise now; deeper, the body is in
            // the face already (a slope it walked under), and rising carries it out.
            let q = closest_on_triangle(centre, a, b, c);
            return (q.distance_squared(centre) >= reach * reach).then_some(0.0);
        }
        Some(first)
    }

    /// How many triangles a [`CollisionWorld::ceiling_for`] of `rise` from `feet` sweeps the
    /// head sphere against. Only for the step-cost test.
    #[doc(hidden)]
    pub fn ceiling_triangle_count(&self, feet: [f32; 3], rise: f32) -> usize {
        self.ceiling_candidates(Vec3::from(feet), rise, PLAYER)
            .count()
    }

    /// Every surface the vertical line through `(x, z)` crosses: (height, normal y), highest
    /// first. For debugging and tests.
    pub fn surfaces_at(&self, x: f32, z: f32) -> Vec<(f32, f32)> {
        let mut out: Vec<(f32, f32)> = self
            .cells
            .get(&cell_of(x, z))
            .into_iter()
            .flatten()
            .filter(|&&t| self.live(t, PLAYER))
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
        self.ray(origin, dir, max, PLAYER).0
    }

    /// The ray's `(hit, tested)`: the distance to the first hit, and how many triangles the ray
    /// was tested against.
    fn ray(&self, origin: Vec3, dir: Vec3, max: f32, who: u8) -> (Option<f32>, usize) {
        let end = origin + dir * max;
        if max < 0.0 || !origin.is_finite() || !end.is_finite() {
            return (None, 0);
        }
        // Walk the cells the ray crosses in order, testing in each only the triangles that reach
        // the ray's height over that cell, and stop once the nearest hit lies before the next
        // cell: any triangle not yet tested is met further along. Hot cells hold thousands of
        // triangles from other storeys, and gathering, sorting and testing all of them along a
        // sight ray blocked by the next wall cost about 1 ms a ray (#161).
        // TODO(#163): one shared candidate filter with `gather_slide`'s horizontal cull.
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
        // Cell crossings as fractions of the whole ray.
        let (sx, mut tx, dx) = axis(origin.x, end.x - origin.x, x);
        let (sz, mut tz, dz) = axis(origin.z, end.z - origin.z, z);
        // Roundoff margins: along the ray (as a fraction of it) and in height and distance (m).
        const ALONG: f32 = 1e-4;
        const PAD: f32 = 1e-3;
        let rise = end.y - origin.y;
        let mut enter = 0.0f32;
        let mut best: Option<f32> = None;
        let mut tested = 0;
        for _ in 0..(u64::from(x.abs_diff(ex)) + u64::from(z.abs_diff(ez)) + 1) {
            let last = (x, z) == (ex, ez);
            // Roundoff must not step an axis beyond the endpoint's cell.
            if x == ex {
                tx = f32::INFINITY;
            }
            if z == ez {
                tz = f32::INFINITY;
            }
            // 1 in the last cell, where both crossings are infinite.
            let leave = tx.min(tz).clamp(enter, 1.0);
            if let Some(cell) = self.cells.get(&(x, z)) {
                let [y0, y1] = [(enter - ALONG).max(0.0), (leave + ALONG).min(1.0)]
                    .map(|t| origin.y + rise * t);
                let (low_y, high_y) = (y0.min(y1) - PAD, y0.max(y1) + PAD);
                for &t in cell {
                    let [lo, hi] = self.bounds[t as usize];
                    if lo.y > high_y || hi.y < low_y || !self.solid(t) || !self.live(t, who) {
                        continue;
                    }
                    tested += 1;
                    let [a, b, c] = self.corners(t);
                    if let Some(d) = ray_triangle(origin, dir, a, b, c)
                        && d <= max
                        && best.is_none_or(|b| d < b)
                    {
                        best = Some(d);
                    }
                }
            }
            // Any triangle not yet tested lies past `leave`; the margin grows with the ray, as
            // the crossings add up roundoff cell by cell.
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
        (best, tested)
    }

    /// How many triangles a [`CollisionWorld::raycast`] tests the ray against. Only for the
    /// step-cost test (#161).
    #[doc(hidden)]
    pub fn ray_triangle_count(&self, origin: Vec3, dir: Vec3, max: f32) -> usize {
        self.ray(origin, dir, max, PLAYER).1
    }

    /// Moves a body standing at `feet` by the horizontal `delta`, stopping at walls and
    /// sliding along them. Returns the horizontal displacement actually made.
    pub fn slide(&self, feet: [f32; 3], delta: [f32; 2]) -> [f32; 2] {
        self.slide_for(feet, delta, PLAYER, 0, true)
    }

    /// [`CollisionWorld::slide`] without the horizontal cull of [`CollisionWorld::gather_slide`]:
    /// every triangle of the move's cells in the body's height band. Only for the test that the
    /// cull changes no slide (#161).
    #[doc(hidden)]
    pub fn slide_uncull(&self, feet: [f32; 3], delta: [f32; 2]) -> [f32; 2] {
        self.slide_for(feet, delta, PLAYER, 0, false)
    }

    /// How many triangles a [`CollisionWorld::slide`] of `delta` from `feet` tests the body
    /// against on each pass: those near the move, solid, and within the body's height band.
    /// Only for the step-cost test (#145).
    #[doc(hidden)]
    pub fn slide_triangle_count(&self, feet: [f32; 3], delta: [f32; 2]) -> usize {
        let (mut tris, mut walls) = (Vec::new(), Vec::new());
        self.gather_slide(
            Vec3::from(feet),
            Vec2::from(delta),
            [PLAYER, 0],
            &mut tris,
            &mut walls,
            true,
        );
        tris.len()
    }

    /// Gathers, each once, the triangles near a slide of `d` from `start` that `who[0]`'s body
    /// can touch into `body`, and the fall-prevention walls in `who[1]` (none for 0) into
    /// `walls`. Returns the slide's coarse sub-step count.
    ///
    /// `body` keeps only the solid triangles that reach the body's spheres' height band at
    /// `start`. A slide moves the body only horizontally, so any other triangle is one
    /// [`CollisionWorld::push_out`] would reject on every pass; hot cells hold thousands of
    /// them from other storeys (#145). The band is padded so roundoff cannot drop a contact.
    /// It also keeps only those whose horizontal bounds come near the move: the cells reach up
    /// to 4 m past it, and a long probe (`docking::wall_contact`'s eight 0.8 m slides) tested
    /// about 3k far triangles on each pass, about 20 ms a step (#161). `cull` false skips that
    /// horizontal cull, for [`CollisionWorld::slide_uncull`].
    // TODO(#163): one shared candidate filter with `ray`'s per-cell height band.
    fn gather_slide(
        &self,
        start: Vec3,
        d: Vec2,
        [who, assist]: [u8; 2],
        body: &mut Vec<u32>,
        walls: &mut Vec<u32>,
        cull: bool,
    ) -> usize {
        let len = d.length();
        let coarse = ((len / (RADIUS * 0.5)).ceil() as usize).clamp(1, 32);
        let reach = RADIUS + len / coarse as f32 + 0.05;
        let min = Vec2::new(start.x.min(start.x + d.x), start.z.min(start.z + d.y)) - reach;
        let max = Vec2::new(start.x.max(start.x + d.x), start.z.max(start.z + d.y)) + reach;
        let low = start.y + SPHERES[0] - RADIUS - 0.01;
        let high = start.y + SPHERES[SPHERES.len() - 1] + RADIUS + 0.01;
        // The horizontal keep box: the move's reach plus the move's length and a body radius
        // more. Push-outs carry the body off the straight line, along a slanted wall for one:
        // each sub-step's push only undoes what that sub-step pushed into a wall, so together
        // they drift it by at most about the move's length, plus up to a radius for a body that
        // starts pressed into a wall. Within that box the body meets every triangle it can touch.
        let pad = RADIUS + len;
        let (near_min, near_max) = (min - pad, max + pad);
        body.clear();
        walls.clear();
        let (x0, z0) = cell_of(min.x, min.y);
        let (x1, z1) = cell_of(max.x, max.y);
        for cx in x0..=x1 {
            for cz in z0..=z1 {
                let Some(cell) = self.cells.get(&(cx, cz)) else {
                    continue;
                };
                body.extend(cell.iter().copied().filter(|&t| {
                    let [lo, hi] = self.bounds[t as usize];
                    hi.y >= low
                        && lo.y <= high
                        && (!cull
                            || hi.x >= near_min.x
                                && lo.x <= near_max.x
                                && hi.z >= near_min.y
                                && lo.z <= near_max.y)
                        && self.solid(t)
                        && self.live(t, who)
                }));
                // The assist's walls are rare, so only a move near one pays for them.
                if assist != 0 {
                    walls.extend(cell.iter().copied().filter(|&t| self.live(t, assist)));
                }
            }
        }
        if x0 != x1 || z0 != z1 {
            for tris in [body, walls] {
                tris.sort_unstable();
                tris.dedup();
            }
        }
        coarse
    }

    /// [`CollisionWorld::slide`] for `who` that also keeps the body [`FALL_ASSIST_RADIUS`]
    /// from the fall-prevention walls in `assist` ([`crate::hit_filter::FALL_ASSIST_MOVE`],
    /// `FALL_ASSIST_ACTION` bits), which nothing else collides with.
    /// `cull` is [`CollisionWorld::gather_slide`]'s.
    fn slide_for(
        &self,
        feet: [f32; 3],
        delta: [f32; 2],
        who: u8,
        assist: u8,
        cull: bool,
    ) -> [f32; 2] {
        thread_local! {
            static SCRATCH: std::cell::RefCell<(Vec<u32>, Vec<u32>)> =
                const { std::cell::RefCell::new((Vec::new(), Vec::new())) };
        }
        let start = Vec3::from(feet);
        let d = Vec2::from(delta);
        let len = d.length();
        SCRATCH.with_borrow_mut(|(tris, walls)| {
            let coarse = self.gather_slide(start, d, [who, assist], tris, walls, cull);
            // Sub-steps of half the smallest clearance, so no wall is stepped across.
            let steps = if walls.is_empty() {
                coarse
            } else {
                ((len / (FALL_ASSIST_RADIUS * 0.5)).ceil() as usize).clamp(1, 64)
            };
            let mut p = start;
            for _ in 0..steps {
                p.x += d.x / steps as f32;
                p.z += d.y / steps as f32;
                for _ in 0..4 {
                    let mut push = self.push_out(p, tris);
                    // The assist's walls last, so they hold where both push.
                    if !walls.is_empty() {
                        push += self.hold_inside(p + Vec3::new(push.x, 0.0, push.y), walls);
                    }
                    if push == Vec2::ZERO {
                        break;
                    }
                    p.x += push.x;
                    p.z += push.y;
                }
            }
            [p.x - start.x, p.z - start.z]
        })
    }

    /// Horizontal correction that keeps the feet [`FALL_ASSIST_RADIUS`] on the inner side of
    /// the fall-prevention wall triangles `walls` they are near. The walls are tested at the feet (the
    /// height the exe tests at is not traced); the map's walls are bands from the floor at the
    /// edge to about 1 m above it. They are one-sided: `FUN_140bc68d0` cuts a move only along a
    /// hit triangle's face normal and only while the body is behind that face, so they keep a
    /// body in and let one outside walk in. The export's mirror into Bevy space reverses the
    /// winding, so the inner side is the one the Bevy-space normal points to.
    fn hold_inside(&self, feet: Vec3, walls: &[u32]) -> Vec2 {
        let mut push = Vec2::ZERO;
        for &t in walls {
            let [lo, hi] = self.bounds[t as usize];
            if feet.distance_squared(feet.clamp(lo, hi)) >= FALL_ASSIST_RADIUS * FALL_ASSIST_RADIUS
            {
                continue;
            }
            let [a, b, c] = self.corners(t);
            if feet.distance(closest_on_triangle(feet, a, b, c)) >= FALL_ASSIST_RADIUS {
                continue;
            }
            let n = self.normals[t as usize];
            let flat = Vec2::new(n.x, n.z);
            let inside = n.dot(feet - a);
            // Outside the wall, or a face too flat to push sideways from.
            if inside < 0.0 || flat.length_squared() < 0.09 {
                continue;
            }
            let dir = flat.normalize();
            let depth = (FALL_ASSIST_RADIUS - inside) / flat.length();
            if depth > push.dot(dir) {
                push += dir * (depth - push.dot(dir));
            }
        }
        push
    }

    /// Horizontal correction that takes the body's spheres out of the given triangles, which
    /// [`CollisionWorld::gather_slide`] has already limited to the solid ones the body can touch.
    fn push_out(&self, feet: Vec3, tris: &[u32]) -> Vec2 {
        let mut total = Vec2::ZERO;
        for h in SPHERES {
            let centre = feet + Vec3::Y * h;
            let mut push = Vec2::ZERO;
            for &t in tris {
                // The horizontal grid includes stacked floors and walls from other heights.
                // Exact sphere/AABB rejection preserves the narrow-phase result and avoids
                // expensive closest-point tests on geometry this capsule cannot touch.
                let [min, max] = self.bounds[t as usize];
                if centre.distance_squared(centre.clamp(min, max)) >= RADIUS * RADIUS {
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

/// The world as one kind of body sees it: which helper parts hold it.
struct View<'a> {
    world: &'a CollisionWorld,
    who: u8,
}

impl Ground for View<'_> {
    fn water_surface(&self, x: f32, z: f32, top: f32) -> Option<WaterSurface> {
        let w = self.world;
        w.cells
            .get(&cell_of(x, z))?
            .iter()
            .filter_map(|&t| {
                let material = w.materials[t as usize];
                if !matches!(material, 20..=24 | 35..=38) || !w.live(t, self.who) {
                    return None;
                }
                let [a, b, c] = w.corners(t);
                let height = height_on(x, z, a, b, c)?;
                (height <= top).then_some(WaterSurface { height, material })
            })
            .max_by(|a, b| a.height.total_cmp(&b.height))
    }

    fn height(&self, x: f32, z: f32) -> Option<f32> {
        self.floor(x, z, f32::INFINITY)
    }

    fn trigger_contact(&self, from: [f32; 3], to: [f32; 3]) -> Option<TriggerContact> {
        self.world.trigger_contact(from.into(), to.into())
    }

    fn near_trigger(&self, part: &str, feet: [f32; 3]) -> bool {
        self.world.near_trigger(part, feet.into())
    }

    fn fall_return_below(&self, x: f32, y: f32, z: f32) -> bool {
        self.world.fall_return_below(x, y, z)
    }

    fn floor(&self, x: f32, z: f32, top: f32) -> Option<f32> {
        self.floor_triangle(x, z, top).map(|(h, _)| h)
    }

    fn ceiling(&self, feet: [f32; 3], rise: f32) -> Option<f32> {
        self.world.ceiling_for(feet.into(), rise, self.who)
    }

    fn slide(&self, feet: [f32; 3], delta: [f32; 2]) -> [f32; 2] {
        self.slide_assisted(feet, delta, 0)
    }

    fn slide_assisted(&self, feet: [f32; 3], delta: [f32; 2], walls: u8) -> [f32; 2] {
        self.world.slide_for(feet, delta, self.who, walls, true)
    }

    fn max_step(&self) -> f32 {
        STEP_HEIGHT
    }

    fn material(&self, x: f32, z: f32, top: f32) -> Option<u32> {
        self.deep_water_over(x, z, top, self.floor(x, z, top))
            .or_else(|| self.world.floor_material_for(x, z, top, self.who))
    }

    fn floor_with_material(&self, x: f32, z: f32, top: f32) -> Option<(f32, Option<u32>)> {
        let (h, t) = self.floor_triangle(x, z, top)?;
        let material = self
            .deep_water_over(x, z, top, Some(h))
            .or_else(|| self.world.triangle_material(t));
        Some((h, material))
    }
}

impl View<'_> {
    /// The highest walkable surface under the feet at `(x, z)` no higher than `top`: height and
    /// triangle. Feet are not a point: it looks a few centimetres around too, so seams between
    /// triangles do not drop the body.
    fn floor_triangle(&self, x: f32, z: f32, top: f32) -> Option<(f32, u32)> {
        const R: f32 = 0.08;
        [(0.0, 0.0), (R, 0.0), (-R, 0.0), (0.0, R), (0.0, -R)]
            .into_iter()
            .filter_map(|(dx, dz)| self.world.floor_triangle(x + dx, z + dz, top, self.who))
            .reduce(|best, hit| if hit.0 > best.0 { hit } else { best })
    }

    /// The material of deep water over the floor `floor` at `(x, z)`: the body stands in that
    /// water rather than on the floor.
    fn deep_water_over(&self, x: f32, z: f32, top: f32, floor: Option<f32>) -> Option<u32> {
        self.water_surface(x, z, top)
            .filter(|water| water.deep() && floor.is_none_or(|floor| floor < water.height))
            .map(|water| water.material)
    }
}

/// Implements [`Ground`] for a type by forwarding every query to its [`View`].
macro_rules! ground_via_view {
    ($ty:ty, $view:ident) => {
        impl Ground for $ty {
            fn water_surface(&self, x: f32, z: f32, top: f32) -> Option<WaterSurface> {
                self.$view().water_surface(x, z, top)
            }

            fn height(&self, x: f32, z: f32) -> Option<f32> {
                self.$view().height(x, z)
            }

            fn trigger_contact(&self, from: [f32; 3], to: [f32; 3]) -> Option<TriggerContact> {
                self.$view().trigger_contact(from, to)
            }

            fn near_trigger(&self, part: &str, feet: [f32; 3]) -> bool {
                self.$view().near_trigger(part, feet)
            }

            fn fall_return_below(&self, x: f32, y: f32, z: f32) -> bool {
                self.$view().fall_return_below(x, y, z)
            }

            fn floor(&self, x: f32, z: f32, top: f32) -> Option<f32> {
                self.$view().floor(x, z, top)
            }

            fn ceiling(&self, feet: [f32; 3], rise: f32) -> Option<f32> {
                self.$view().ceiling(feet, rise)
            }

            fn slide(&self, feet: [f32; 3], delta: [f32; 2]) -> [f32; 2] {
                self.$view().slide(feet, delta)
            }

            fn slide_assisted(&self, feet: [f32; 3], delta: [f32; 2], walls: u8) -> [f32; 2] {
                self.$view().slide_assisted(feet, delta, walls)
            }

            fn max_step(&self) -> f32 {
                self.$view().max_step()
            }

            fn material(&self, x: f32, z: f32, top: f32) -> Option<u32> {
                self.$view().material(x, z, top)
            }

            fn floor_with_material(&self, x: f32, z: f32, top: f32) -> Option<(f32, Option<u32>)> {
                self.$view().floor_with_material(x, z, top)
            }
        }
    };
}

impl CollisionWorld {
    /// The world itself is the player's view.
    fn player_view(&self) -> View<'_> {
        View {
            world: self,
            who: PLAYER,
        }
    }
}

ground_via_view!(CollisionWorld, player_view);

/// A shared world as enemies and bosses see it: they also stand on water surfaces and stop at
/// enemy-only walls (hit filters 11, 16 and 19, [`crate::hit_filter`]).
#[derive(Clone)]
pub struct EnemyGround(pub std::sync::Arc<CollisionWorld>);

impl EnemyGround {
    fn enemy_view(&self) -> View<'_> {
        View {
            world: &self.0,
            who: ENEMY,
        }
    }
}

ground_via_view!(EnemyGround, enemy_view);

impl<T: Ground + ?Sized> Ground for std::sync::Arc<T> {
    fn water_surface(&self, x: f32, z: f32, top: f32) -> Option<WaterSurface> {
        (**self).water_surface(x, z, top)
    }
    fn height(&self, x: f32, z: f32) -> Option<f32> {
        (**self).height(x, z)
    }

    fn trigger_contact(&self, from: [f32; 3], to: [f32; 3]) -> Option<TriggerContact> {
        (**self).trigger_contact(from, to)
    }

    fn near_trigger(&self, part: &str, feet: [f32; 3]) -> bool {
        (**self).near_trigger(part, feet)
    }

    fn fall_return_below(&self, x: f32, y: f32, z: f32) -> bool {
        (**self).fall_return_below(x, y, z)
    }

    fn floor(&self, x: f32, z: f32, top: f32) -> Option<f32> {
        (**self).floor(x, z, top)
    }

    fn ceiling(&self, feet: [f32; 3], rise: f32) -> Option<f32> {
        (**self).ceiling(feet, rise)
    }

    fn slide(&self, feet: [f32; 3], delta: [f32; 2]) -> [f32; 2] {
        (**self).slide(feet, delta)
    }

    fn slide_assisted(&self, feet: [f32; 3], delta: [f32; 2], walls: u8) -> [f32; 2] {
        (**self).slide_assisted(feet, delta, walls)
    }

    fn max_step(&self) -> f32 {
        (**self).max_step()
    }

    fn material(&self, x: f32, z: f32, top: f32) -> Option<u32> {
        (**self).material(x, z, top)
    }

    fn floor_with_material(&self, x: f32, z: f32, top: f32) -> Option<(f32, Option<u32>)> {
        (**self).floor_with_material(x, z, top)
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
    fn ray_cell_traversal_matches_all_triangles_at_boundaries() {
        let world = test_world();
        for origin in [
            Vec3::new(-9.0, 1.0, -9.0),
            Vec3::new(8.0, 1.0, -8.0),
            Vec3::new(4.0, 1.0, 4.0),
            Vec3::new(-4.0, 1.0, -4.0),
        ] {
            for direction in [
                Vec3::X,
                -Vec3::X,
                Vec3::Z,
                -Vec3::Z,
                Vec3::X + Vec3::Z,
                Vec3::X - Vec3::Z,
                -Vec3::X + Vec3::Z,
                -Vec3::X - Vec3::Z,
                -Vec3::Y,
                // Sloped rays meet the floors and risers in a later cell than they start in,
                // which each cell's height band must keep.
                Vec3::new(0.4, -0.15, -1.0),
                Vec3::new(-1.0, -0.1, 0.3),
                Vec3::new(0.7, 0.05, -0.6),
            ] {
                let dir = direction.normalize();
                for max in [0.1, 25.0] {
                    let expected = (0..world.triangles.len() as u32)
                        .filter(|&t| world.live(t, PLAYER))
                        .filter_map(|t| {
                            let [a, b, c] = world.corners(t);
                            ray_triangle(origin, dir, a, b, c)
                        })
                        .filter(|&d| d <= max)
                        .min_by(f32::total_cmp);
                    assert_eq!(
                        world.raycast(origin, dir, max),
                        expected,
                        "{origin:?} {dir:?} max {max}"
                    );
                }
            }
        }
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

    /// A beam 0.25 m above the head, whose edge is 0.3 m out, over the head sphere's rim, stops
    /// the rise where the rim meets that edge (#170).
    #[test]
    fn a_beam_edge_over_the_head_rim_stops_the_rise() {
        let y = HEAD_TOP + 0.25;
        let mut m = CollisionMesh::default();
        m.vertices.extend_from_slice(&[
            [-2.0, y, -2.0],
            [2.0, y, -2.0],
            [2.0, y, -0.3],
            [-2.0, y, -0.3],
        ]);
        m.indices.extend([0, 1, 2, 0, 2, 3]);
        m.materials.extend([0, 0]);
        let world = CollisionWorld::from_mesh(&m);
        let top = world.ceiling_for(Vec3::ZERO, 1.0, PLAYER);
        let want = y - (RADIUS * RADIUS - 0.3 * 0.3).sqrt() - (HEAD_TOP - RADIUS);
        assert!(
            top.is_some_and(|t| (t - want).abs() < 1e-4),
            "{top:?}, want {want}"
        );
        // Clear of the rim, nothing stops it.
        assert_eq!(
            world.ceiling_for(Vec3::new(0.0, 0.0, 0.11), 1.0, PLAYER),
            None
        );
    }

    /// A world of the given quads, each two triangles.
    fn quads(quads: &[[[f32; 3]; 4]]) -> CollisionWorld {
        let mut m = CollisionMesh::default();
        for q in quads {
            let base = m.vertices.len() as u32;
            m.vertices.extend_from_slice(q);
            m.indices.extend([0, 1, 2, 0, 2, 3].map(|i| base + i));
            m.materials.extend([0, 0]);
        }
        CollisionWorld::from_mesh(&m)
    }

    /// An underside sloping down away from the centre line, its near edge 0.1 m out, meets the
    /// head sphere first on its face, before the edge nearest the line would.
    #[test]
    fn a_sloped_underside_stops_the_rise_at_its_first_touch() {
        let (y0, k) = (HEAD_TOP + 0.3, 0.7);
        let y = |x: f32| y0 - k * (x - 0.1);
        let world = quads(&[[
            [0.1, y(0.1), -3.0],
            [3.0, y(3.0), -3.0],
            [3.0, y(3.0), 3.0],
            [0.1, y(0.1), 3.0],
        ]]);
        let head = HEAD_TOP - RADIUS;
        let face = y0 + 0.1 * k - RADIUS * (1.0 + k * k).sqrt() - head;
        let edge = y0 - (RADIUS * RADIUS - 0.1 * 0.1).sqrt() - head;
        assert!(face < edge - 0.02, "{face} {edge}");
        let top = world.ceiling_for(Vec3::ZERO, 1.0, PLAYER);
        assert!(
            top.is_some_and(|t| (t - face).abs() < 1e-4),
            "{top:?}, want {face}"
        );
    }

    /// The top of a wall the slide leaves the body pressed against, its edge at the head
    /// sphere's rim, does not stop a jump up past it (ledge grabs and wall jumps); 0.1 m nearer,
    /// it does.
    #[test]
    fn a_wall_top_at_the_rim_does_not_stop_the_rise() {
        let y = 2.0;
        let world = quads(&[
            [
                [-3.0, 0.0, 0.0],
                [3.0, 0.0, 0.0],
                [3.0, 0.0, 3.0],
                [-3.0, 0.0, 3.0],
            ],
            [
                [-3.0, 0.0, 0.0],
                [3.0, 0.0, 0.0],
                [3.0, y, 0.0],
                [-3.0, y, 0.0],
            ],
            [
                [-3.0, y, -3.0],
                [3.0, y, -3.0],
                [3.0, y, 0.0],
                [-3.0, y, 0.0],
            ],
        ]);
        let [dx, dz] = world.slide([0.3, 0.0, 1.0], [0.0, -1.0]);
        let pressed = Vec3::new(0.3 + dx, 0.0, 1.0 + dz);
        assert!((pressed.z - RADIUS).abs() < 1e-3, "{pressed}");
        for feet in [pressed, Vec3::new(0.3, 0.0, RADIUS)] {
            assert_eq!(world.ceiling_for(feet, 1.0, PLAYER), None, "{feet}");
        }
        assert!(
            world
                .ceiling_for(Vec3::new(0.3, 0.0, 0.3), 1.0, PLAYER)
                .is_some()
        );
    }
}

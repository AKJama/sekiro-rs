//! Foot IK for Wolf, drawn only: the body stands on the `l` collision, the feet on `h` (#276).
//!
//! The exe's player capsule meets only the low-resolution `l` mesh, which models uneven ground
//! as flat slabs and flights of stairs as ramps (`docs/HIT-FILTERS.md`). Its foot IK
//! (Havok's `hkbFootIkModifier`, driven by the behaviour graph's `hkbFootIkControlsModifier`)
//! casts each foot down onto the high-resolution `h` mesh, so on the Stairway idol platform's
//! craters, up to 0.22 m below the `l` slab, the feet still meet the drawn ground.
//!
//! Each frame after the clip is posed ([`crate::character::animate`]):
//! 1. Each foot's error is the `h` ground height under its ankle less the body's feet height
//!    (the `l` floor), clamped to [`MAX_ERROR`]; none when nothing is hit within the ray.
//! 2. The errors ease toward that with the behaviour graph's gains: down at once
//!    (`groundDescendingGain` 1), up by `groundAscendingGain` 0.2 a step, and the whole IK in
//!    and out by `onOffGain` 0.2 (gains read from c0000's `hkbFootIkControlsModifier`; Havok
//!    applies them once per behaviour update, here per 1/60 s).
//! 3. The whole pose drops by the lower foot's error (never raised), at most [`MAX_PELVIS_DROP`].
//! 4. Each leg (thigh, calf, foot) is solved with two-bone IK so its ankle reaches the animated
//!    ankle moved by that leg's error; the foot keeps its animated orientation.
//!
//! The leg geometry and ray lengths of the game's `hkbFootIkDriverInfo` are not in c0000's
//! behbnd (it holds only the controls modifier), so the ray lengths and limits below are chosen,
//! not read.

use bevy::prelude::*;
use sekiro_formats::anim::Skeleton;
use sekiro_formats::hknp::CollisionMesh;
use sekiro_sim::collision::CollisionWorld;
use std::sync::{Arc, Mutex};

/// How far above the ankle the ray starts (the driver info's `raycastDistanceUp`; chosen).
pub const RAY_UP: f32 = 0.5;
/// How far below the body's feet the ray reaches (`raycastDistanceDown`; chosen).
pub const RAY_DOWN: f32 = 0.8;
/// The largest error a foot follows either way, metres.
pub const MAX_ERROR: f32 = 0.45;
/// The furthest the pose drops for the lower foot, metres.
pub const MAX_PELVIS_DROP: f32 = 0.4;

/// c0000's `hkbFootIkControlsModifier` gains (`FootIKControlsModifier` in `c0000.behbnd`).
pub const ON_OFF_GAIN: f32 = 0.2;
pub const GROUND_ASCENDING_GAIN: f32 = 0.2;
pub const GROUND_DESCENDING_GAIN: f32 = 1.0;

/// A gain given per behaviour step of 1/60 s, for a frame of `dt` seconds.
fn gain_for(gain: f32, dt: f32) -> f32 {
    if gain >= 1.0 {
        return 1.0;
    }
    1.0 - (1.0 - gain).powf((dt * 60.0).max(0.0))
}

/// One leg's bones in the skeleton.
#[derive(Debug, Clone, Copy)]
pub struct Leg {
    pub hip: usize,
    pub knee: usize,
    pub ankle: usize,
}

/// Wolf's legs, left then right; `None` if the skeleton lacks them.
pub fn legs(skeleton: &Skeleton) -> Option<[Leg; 2]> {
    let leg = |side: &str| {
        Some(Leg {
            hip: skeleton.bone_index(&format!("{side}_Thigh"))?,
            knee: skeleton.bone_index(&format!("{side}_Calf"))?,
            ankle: skeleton.bone_index(&format!("{side}_Foot"))?,
        })
    };
    Some([leg("L")?, leg("R")?])
}

/// The eased state of one character's foot IK.
#[derive(Component, Debug, Clone, Default)]
pub struct FootIk {
    /// Whether the IK should be on this frame (grounded, not swimming, perched or grappling).
    pub enabled: bool,
    /// The on/off weight, eased by [`ON_OFF_GAIN`].
    pub weight: f32,
    /// Each leg's eased error, metres (negative: the ground is lower than the feet).
    pub error: [f32; 2],
    /// The last frame's drop of the pose, metres, for logs and tests.
    pub drop: f32,
}

impl FootIk {
    /// Eases toward this frame's raw errors (`None`: the ray hit nothing).
    fn ease(&mut self, raw: [Option<f32>; 2], dt: f32) {
        let on = if self.enabled { 1.0 } else { 0.0 };
        self.weight += (on - self.weight) * gain_for(ON_OFF_GAIN, dt);
        if self.weight < 1e-3 && !self.enabled {
            self.weight = 0.0;
        }
        for (error, raw) in self.error.iter_mut().zip(raw) {
            let target = raw.unwrap_or(0.0).clamp(-MAX_ERROR, MAX_ERROR);
            let gain = if target > *error {
                GROUND_ASCENDING_GAIN
            } else {
                GROUND_DESCENDING_GAIN
            };
            *error += (target - *error) * gain_for(gain, dt);
        }
    }
}

/// Applies foot IK to the model-space pose `model` of a character drawn at `world`
/// (its feet at `world`'s origin on the play collision). `ground(x, z, top)` is the IK
/// ground's height under `(x, z)` at or below `top`. Returns whether the pose changed.
pub fn solve(
    skeleton: &Skeleton,
    legs: &[Leg; 2],
    model: &mut [Mat4],
    world: Mat4,
    ik: &mut FootIk,
    dt: f32,
    ground: impl Fn(f32, f32, f32) -> Option<f32>,
) -> bool {
    let feet = world.w_axis.y;
    let raw = legs.map(|leg| {
        if !ik.enabled {
            return None;
        }
        let ankle = world.transform_point3(model[leg.ankle].w_axis.truncate());
        let y = ground(ankle.x, ankle.z, ankle.y + RAY_UP)?;
        (y >= feet - RAY_DOWN).then_some(y - feet)
    });
    ik.ease(raw, dt);
    let error = ik.error.map(|e| e * ik.weight);
    if error.iter().all(|e| e.abs() < 1e-4) {
        ik.drop = 0.0;
        return false;
    }
    // Model space is world space turned about +Y, so heights carry over.
    let drop = error[0].min(error[1]).clamp(-MAX_PELVIS_DROP, 0.0);
    ik.drop = drop;
    let targets = legs.map(|leg| model[leg.ankle].w_axis.truncate());
    if drop != 0.0 {
        let by = Mat4::from_translation(Vec3::Y * drop);
        for m in model.iter_mut() {
            *m = by * *m;
        }
    }
    let side = world.inverse().transform_vector3(Vec3::X);
    for ((leg, target), error) in legs.iter().zip(targets).zip(error) {
        two_bone(skeleton, model, leg, target + Vec3::Y * error, side);
    }
    true
}

/// Turns the leg's thigh and calf so its ankle reaches `target` (model space), bending the knee
/// in the plane it already bends in (`side`, the character's side axis, when the leg is
/// straight), and gives the foot back its animated orientation.
pub fn two_bone(skeleton: &Skeleton, model: &mut [Mat4], leg: &Leg, target: Vec3, side: Vec3) {
    let at = |model: &[Mat4], i: usize| model[i].w_axis.truncate();
    let (h, k, a) = (
        at(model, leg.hip),
        at(model, leg.knee),
        at(model, leg.ankle),
    );
    let (upper, lower) = ((k - h).length(), (a - k).length());
    if upper < 1e-4 || lower < 1e-4 {
        return;
    }
    let foot = Quat::from_mat4(&model[leg.ankle]).normalize();
    let reach = (target - h)
        .length()
        .clamp((upper - lower).abs() + 1e-4, upper + lower - 1e-4);
    // The knee's bend axis: the plane of hip, knee and ankle, or the side axis if straight.
    let mut axis = (h - k).cross(a - k);
    if axis.length_squared() < 1e-10 {
        axis = side;
    }
    let axis = axis.normalize();
    let now = (h - k).angle_between(a - k);
    let want = ((upper * upper + lower * lower - reach * reach) / (2.0 * upper * lower))
        .clamp(-1.0, 1.0)
        .acos();
    turn_about(
        skeleton,
        model,
        leg.knee,
        k,
        Quat::from_axis_angle(axis, want - now),
    );
    let a = at(model, leg.ankle);
    let (from, to) = ((a - h).normalize(), (target - h).normalize());
    turn_about(
        skeleton,
        model,
        leg.hip,
        h,
        Quat::from_rotation_arc(from, to),
    );
    let now = Quat::from_mat4(&model[leg.ankle]).normalize();
    let a = at(model, leg.ankle);
    turn_about(skeleton, model, leg.ankle, a, foot * now.inverse());
}

/// Turns bone `root` and everything under it by `turn` about the model-space point `pivot`.
fn turn_about(skeleton: &Skeleton, model: &mut [Mat4], root: usize, pivot: Vec3, turn: Quat) {
    let by = Mat4::from_translation(pivot) * Mat4::from_quat(turn) * Mat4::from_translation(-pivot);
    crate::character::turn_subtree(skeleton, model, root, by);
}

/// Side of the square tiles the floors are found by, metres.
pub const TILE: f32 = 32.0;

/// The tile `(x, z)` is in.
pub fn tile_of(x: f32, z: f32) -> (i32, i32) {
    ((x / TILE).floor() as i32, (z / TILE).floor() as i32)
}

/// Triangles per chunk of [`BlockFloors`]' index.
const CHUNK: u32 = 1024;

/// Whether a triangle faces within 73 degrees of up (either way): the ground the feet meet.
fn is_floor([a, b, c]: [Vec3; 3]) -> bool {
    (b - a).cross(c - a).normalize_or_zero().y.abs() >= 0.3
}

/// Reads a postcard varint (LEB128) `u32` at `*at`.
fn varint(bytes: &[u8], at: &mut usize) -> Option<u32> {
    let mut value = 0u32;
    for shift in (0..35).step_by(7) {
        let byte = *bytes.get(*at)?;
        *at += 1;
        value |= u32::from(byte & 0x7f).checked_shl(shift)?;
        if byte & 0x80 == 0 {
            return Some(value);
        }
    }
    None
}

/// One block's `h` collision (`collision.bin`, a postcard [`CollisionMesh`]) read in place from
/// the mapped file, so its millions of triangles cost no memory of their own (#285).
/// The index keeps, per [`TILE`], the chunks of [`CHUNK`] triangles that hold a floor there,
/// and the byte offset of each chunk's indices, since the indices are varints.
///
/// [`CollisionMesh`]: sekiro_formats::hknp::CollisionMesh
pub struct BlockFloors {
    map: memmap2::Mmap,
    /// Byte offset of the first vertex (three `f32`s each) and the vertex count.
    vertices: (usize, u32),
    triangles: u32,
    /// Byte offset of the indices of triangle `CHUNK * k`.
    chunks: Vec<usize>,
    /// Triangle ranges that are not ground: parts the story switches off and helper parts the
    /// player does not collide with. Sorted and disjoint.
    skip: Vec<std::ops::Range<u32>>,
    tiles: std::collections::HashMap<(i32, i32), Vec<u32>>,
}

impl BlockFloors {
    /// Maps `path` and indexes its floors outside `skip` (sorted and disjoint).
    pub fn open(path: &std::path::Path, skip: Vec<std::ops::Range<u32>>) -> anyhow::Result<Self> {
        let file = std::fs::File::open(path)?;
        // SAFETY: the cache is written only by the extractor, never while the game runs.
        let map = unsafe { memmap2::Mmap::map(&file)? };
        let bad = || anyhow::anyhow!("{}: not a collision mesh", path.display());
        let mut at = 0;
        let count = varint(&map, &mut at).ok_or_else(bad)?;
        let vertices = (at, count);
        at += count as usize * 12;
        let indices = varint(&map, &mut at).ok_or_else(bad)?;
        if at > map.len() || indices % 3 != 0 {
            return Err(bad());
        }
        let mut block = Self {
            map,
            vertices,
            triangles: indices / 3,
            chunks: Vec::new(),
            skip,
            tiles: Default::default(),
        };
        let mut skip = 0;
        for t in 0..block.triangles {
            if t % CHUNK == 0 {
                block.chunks.push(at);
            }
            let tri = block.triangle(&mut at).ok_or_else(bad)?;
            while block.skip.get(skip).is_some_and(|r| r.end <= t) {
                skip += 1;
            }
            if block.skip.get(skip).is_some_and(|r| r.contains(&t)) || !is_floor(tri) {
                continue;
            }
            let (lo, hi) = bounds(tri);
            let chunk = t / CHUNK;
            for x in lo.0..=hi.0 {
                for z in lo.1..=hi.1 {
                    let chunks = block.tiles.entry((x, z)).or_default();
                    if chunks.last() != Some(&chunk) {
                        chunks.push(chunk);
                    }
                }
            }
        }
        block.tiles.shrink_to_fit();
        Ok(block)
    }

    /// The corners of the triangle whose indices start at `*at`, moving `*at` past them.
    fn triangle(&self, at: &mut usize) -> Option<[Vec3; 3]> {
        let mut corners = [Vec3::ZERO; 3];
        for corner in &mut corners {
            let i = varint(&self.map, at)?;
            if i >= self.vertices.1 {
                return None;
            }
            let start = self.vertices.0 + i as usize * 12;
            let xyz = self.map.get(start..start + 12)?;
            let f = |k: usize| f32::from_le_bytes([xyz[k], xyz[k + 1], xyz[k + 2], xyz[k + 3]]);
            *corner = Vec3::new(f(0), f(4), f(8));
        }
        Some(corners)
    }

    /// Appends to `mesh` the floors of this block over the tiles `lo..=hi`.
    pub(crate) fn floors_in(&self, lo: (i32, i32), hi: (i32, i32), mesh: &mut CollisionMesh) {
        let mut chunks: Vec<u32> = (lo.0..=hi.0)
            .flat_map(|x| (lo.1..=hi.1).map(move |z| (x, z)))
            .filter_map(|tile| self.tiles.get(&tile))
            .flatten()
            .copied()
            .collect();
        chunks.sort_unstable();
        chunks.dedup();
        for chunk in chunks {
            let first = chunk * CHUNK;
            let mut at = self.chunks[chunk as usize];
            let mut skip = self.skip.partition_point(|r| r.end <= first);
            for t in first..(first + CHUNK).min(self.triangles) {
                let Some(tri) = self.triangle(&mut at) else {
                    return;
                };
                while self.skip.get(skip).is_some_and(|r| r.end <= t) {
                    skip += 1;
                }
                if self.skip.get(skip).is_some_and(|r| r.contains(&t)) || !is_floor(tri) {
                    continue;
                }
                let (a, b) = bounds(tri);
                if a.0 > hi.0 || a.1 > hi.1 || b.0 < lo.0 || b.1 < lo.1 {
                    continue;
                }
                let base = mesh.vertices.len() as u32;
                mesh.vertices.extend(tri.map(<[f32; 3]>::from));
                mesh.indices.extend([base, base + 1, base + 2]);
                mesh.materials.push(u32::MAX);
            }
        }
    }
}

/// The tiles a triangle's bounding box spans.
fn bounds(tri: [Vec3; 3]) -> ((i32, i32), (i32, i32)) {
    let lo = tri.into_iter().reduce(Vec3::min).unwrap_or_default();
    let hi = tri.into_iter().reduce(Vec3::max).unwrap_or_default();
    (tile_of(lo.x, lo.z), tile_of(hi.x, hi.z))
}

/// The `h` floors of the play blocks ([`crate::map::world::foot_ik_ground`]). The two Ashina
/// blocks hold about 8 million floor triangles, 1.3 GB as one [`CollisionWorld`] and 350 MB as
/// a compact index, so they stay in the mapped cache files ([`BlockFloors`]) and the feet query
/// a world of the tiles around Wolf only ([`Floors::world_around`]).
#[derive(Default)]
pub struct Floors {
    pub blocks: Vec<BlockFloors>,
}

impl Floors {
    /// The floors of `tile` and the eight around it, so a point within [`TILE`] of `tile`
    /// finds its ground.
    pub fn world_around(&self, tile: (i32, i32)) -> CollisionWorld {
        let (lo, hi) = ((tile.0 - 1, tile.1 - 1), (tile.0 + 1, tile.1 + 1));
        let mut mesh = CollisionMesh::default();
        for block in &self.blocks {
            block.floors_in(lo, hi, &mut mesh);
        }
        CollisionWorld::from_mesh(&mesh)
    }
}

/// The world of the floors around one tile.
type Local = ((i32, i32), Arc<CollisionWorld>);

/// The ground the feet meet: the `h` floors of the tiles around Wolf ([`Floors`], indexed on a
/// thread at load, and read on that thread whenever he enters another tile, dropping the old
/// ones; no IK until the first is ready), and the placed objects of the play collision, which
/// `h` lacks.
#[derive(Resource)]
pub struct FootGround {
    local: Arc<Mutex<Option<Local>>>,
    follow: Mutex<Option<(i32, i32)>>,
    wanted: Option<std::sync::mpsc::Sender<(i32, i32)>>,
    pub play: Arc<CollisionWorld>,
}

impl FootGround {
    /// Indexes the `h` floors of `layouts` on a thread, which then builds the world around each
    /// tile [`FootGround::follow`] asks for.
    pub fn load(
        cache: &std::path::Path,
        layouts: &std::collections::BTreeMap<String, serde_json::Value>,
        story: &crate::map::story::MapStory,
        play: Arc<CollisionWorld>,
    ) -> Self {
        let local = Arc::new(Mutex::new(None));
        // Only the collision rows go to the thread: the whole layouts are about a gigabyte.
        let cache = cache.to_path_buf();
        let rows: std::collections::BTreeMap<String, serde_json::Value> = layouts
            .iter()
            .map(|(block, layout)| {
                let rows = serde_json::json!({ "collisions": layout["collisions"].clone() });
                (block.clone(), rows)
            })
            .collect();
        let off = crate::map::world::story_off_parts(layouts, story);
        let (wanted, asks) = std::sync::mpsc::channel::<(i32, i32)>();
        let slot = local.clone();
        let spawned = std::thread::Builder::new()
            .name("foot-ik-ground".into())
            .spawn(move || {
                let t = std::time::Instant::now();
                let floors = match crate::map::world::foot_ik_ground(&cache, &rows, &off) {
                    Ok(floors) => floors,
                    Err(e) => {
                        warn!("foot IK ground: {e:#}");
                        return;
                    }
                };
                drop((rows, off));
                // Told with the first tile: this can finish before the log is set up.
                let mut indexed = Some((floors.blocks.len(), t.elapsed()));
                while let Ok(mut tile) = asks.recv() {
                    while let Ok(newer) = asks.try_recv() {
                        tile = newer;
                    }
                    let t = std::time::Instant::now();
                    let world = Arc::new(floors.world_around(tile));
                    info!(
                        "foot IK ground: tile {tile:?}, {} triangles in {:.1} ms{}",
                        world.triangle_count(),
                        t.elapsed().as_secs_f64() * 1e3,
                        indexed.take().map_or(String::new(), |(n, t)| format!(
                            " ({n} h blocks indexed in {:.0} ms)",
                            t.as_secs_f64() * 1e3
                        ))
                    );
                    if let Ok(mut slot) = slot.lock() {
                        *slot = Some((tile, world));
                    }
                }
            });
        if let Err(e) = &spawned {
            warn!("foot IK ground thread: {e}");
        }
        Self {
            local,
            follow: Mutex::new(None),
            wanted: spawned.is_ok().then_some(wanted),
            play,
        }
    }

    /// A ground of one ready world, for tests.
    #[cfg(test)]
    pub fn fixed(world: CollisionWorld, play: Arc<CollisionWorld>) -> Self {
        Self {
            local: Arc::new(Mutex::new(Some(((0, 0), Arc::new(world))))),
            follow: Mutex::new(None),
            wanted: None,
            play,
        }
    }

    /// Asks for the world around `(x, z)`'s tile when it is a new one.
    pub fn follow(&self, x: f32, z: f32) {
        let tile = tile_of(x, z);
        let Ok(mut follow) = self.follow.lock() else {
            return;
        };
        if *follow != Some(tile)
            && let Some(wanted) = &self.wanted
        {
            *follow = Some(tile);
            let _ = wanted.send(tile);
        }
    }

    /// The feet's ground under `(x, z)` at or below `top`: `h`, or a placed object's top when
    /// one is higher.
    pub fn height(&self, x: f32, z: f32, top: f32) -> Option<f32> {
        let world = self.local.lock().ok()?.as_ref()?.1.clone();
        let h = world.floor(x, z, top);
        match self.play.floor_hit(x, z, top) {
            Some((y, t)) if self.play.object_at(t).is_some() => Some(h.map_or(y, |h| h.max(y))),
            _ => h,
        }
    }
}

/// Foot IK on the posed rigs, after [`crate::character::animate`]: moves the legs and rewrites
/// the joints of the bones it changed.
pub fn pose(
    time: Res<Time>,
    ground: Option<Res<FootGround>>,
    mut q: Query<(&mut crate::character::Rig, &Transform, &mut FootIk)>,
    mut joints: Query<&mut Transform, Without<crate::character::Rig>>,
) {
    let Some(ground) = ground else {
        return;
    };
    let dt = time.delta_secs();
    for (mut rig, tf, mut ik) in &mut q {
        if !rig.bound || rig.model.is_empty() {
            continue;
        }
        let rig = &mut *rig;
        let Some(legs) = legs(&rig.skeleton) else {
            continue;
        };
        let world = tf.to_matrix();
        ground.follow(world.w_axis.x, world.w_axis.z);
        if solve(
            &rig.skeleton,
            &legs,
            &mut rig.model,
            world,
            &mut ik,
            dt,
            |x, z, top| ground.height(x, z, top),
        ) {
            crate::character::write_joints(rig, &mut joints);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sekiro_formats::anim::{Bone, Trs};

    /// A pelvis with two legs of 0.45 m thigh, 0.45 m calf and a foot, knees a little bent
    /// forward (-Z), standing with the ankles 0.1 m above the feet.
    fn rig() -> (Skeleton, [Leg; 2], Vec<Mat4>) {
        let bone = |name: &str, parent: Option<u16>, t: [f32; 3]| Bone {
            name: name.into(),
            parent,
            local: Trs {
                translation: t,
                ..Trs::IDENTITY
            },
        };
        let mut bones = vec![bone("Pelvis", None, [0.0, 0.98, 0.0])];
        for (side, x) in [("L", 0.1), ("R", -0.1)] {
            let base = bones.len() as u16;
            bones.push(bone(&format!("{side}_Thigh"), Some(0), [x, 0.0, 0.0]));
            bones.push(bone(
                &format!("{side}_Calf"),
                Some(base),
                [0.0, -0.44, -0.08],
            ));
            bones.push(bone(
                &format!("{side}_Foot"),
                Some(base + 1),
                [0.0, -0.44, 0.08],
            ));
            bones.push(bone(
                &format!("{side}_Toe0"),
                Some(base + 2),
                [0.0, -0.08, -0.12],
            ));
        }
        let skeleton = Skeleton {
            name: "test".into(),
            bones,
        };
        let legs = legs(&skeleton).unwrap();
        let model = skeleton.model_space(&[]);
        (skeleton, legs, model)
    }

    fn settle(
        skeleton: &Skeleton,
        legs: &[Leg; 2],
        model: &[Mat4],
        world: Mat4,
        ground: impl Fn(f32, f32, f32) -> Option<f32> + Copy,
    ) -> (Vec<Mat4>, FootIk) {
        let mut ik = FootIk {
            enabled: true,
            ..Default::default()
        };
        let mut posed = model.to_vec();
        for _ in 0..120 {
            posed = model.to_vec();
            solve(
                skeleton,
                legs,
                &mut posed,
                world,
                &mut ik,
                1.0 / 60.0,
                ground,
            );
        }
        (posed, ik)
    }

    /// On flat ground at the feet the pose is left exactly as animated.
    #[test]
    fn flat_ground_leaves_the_pose() {
        let (skeleton, legs, model) = rig();
        let world =
            Mat4::from_rotation_translation(Quat::from_rotation_y(0.7), Vec3::new(3.0, -48.0, 1.0));
        let (posed, ik) = settle(&skeleton, &legs, &model, world, |_, _, _| Some(-48.0));
        assert_eq!(posed, model);
        assert_eq!(ik.drop, 0.0);
    }

    /// A foot over a dip 0.2 m deep and one over ground 0.1 m up: the pose drops 0.2 m, each
    /// ankle ends its own error above its animated height, the leg lengths hold and the feet keep
    /// their orientation.
    #[test]
    fn uneven_ground_moves_each_ankle_by_its_error() {
        let (skeleton, legs, model) = rig();
        let world = Mat4::from_translation(Vec3::new(0.0, 10.0, 0.0));
        let (posed, ik) = settle(&skeleton, &legs, &model, world, |x, _, _| {
            Some(if x > 0.0 { 9.8 } else { 10.1 })
        });
        assert!((ik.drop + 0.2).abs() < 1e-3, "drop {}", ik.drop);
        for (leg, error) in legs.iter().zip([-0.2, 0.1]) {
            let want = model[leg.ankle].w_axis.truncate() + Vec3::Y * error;
            let got = posed[leg.ankle].w_axis.truncate();
            assert!(got.distance(want) < 2e-3, "ankle {got} want {want}");
            let length = |m: &[Mat4], a: usize, b: usize| {
                m[a].w_axis.truncate().distance(m[b].w_axis.truncate())
            };
            assert!(
                (length(&posed, leg.hip, leg.knee) - length(&model, leg.hip, leg.knee)).abs()
                    < 1e-4
            );
            assert!(
                (length(&posed, leg.knee, leg.ankle) - length(&model, leg.knee, leg.ankle)).abs()
                    < 1e-4
            );
            let (a, b) = (
                Quat::from_mat4(&posed[leg.ankle]),
                Quat::from_mat4(&model[leg.ankle]),
            );
            assert!(a.angle_between(b) < 1e-3);
        }
    }
}

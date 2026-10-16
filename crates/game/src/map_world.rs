//! Bounded connected-block loading for Ashina exploration.
//!
//! Positions in layout v2 already include MSB MapOffset. No block-root transform is added.
//! The warm cache is the direct neighbourhood of the two Ashina blocks, not a recursive
//! graph walk. Demand and retirement are separate from cached asset ownership.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};
use bevy::camera::primitives::{Aabb, Frustum};
use bevy::prelude::*;
use sekiro_formats::hknp::CollisionMesh;
use serde_json::{Value, json};

use super::{GroupMode, MapFocus, Pending, groups8, vertical_hit, visibility};
use bevy::world_serialization::{WorldAssetRoot, WorldInstance, WorldInstanceSpawner};

pub(crate) type Mask = [u32; 8];
const ASHINA: [&str; 2] = ["m11_00_00_00", "m11_01_00_00"];

/// Explicit bootstrap marker, including arena sessions launched from the menu.
#[derive(Resource)]
pub(crate) struct ArenaMap;

pub(crate) fn enabled(id: &str) -> bool {
    ASHINA.contains(&id)
}

/// Material-renderer hook: layer URIs are relative to their own model GLB, not
/// the primary map. This also preserves Bevy's named asset-source identity.
pub(crate) fn material_texture_path(
    assets: &AssetServer,
    material: &Handle<StandardMaterial>,
    uri: &str,
) -> Option<String> {
    relative_material_texture(&assets.get_path(material.id())?, uri)
}

fn relative_material_texture(path: &bevy::asset::AssetPath<'_>, uri: &str) -> Option<String> {
    if !uri.starts_with("tex/") || uri.contains("..") {
        return None;
    }
    path.without_label()
        .resolve_embed_str(uri)
        .ok()
        .map(|p| p.to_string())
}

/// Executable target matching: signed negative block selects zero; other negative
/// fields bypass equality. The all-negative sentinel requests nothing.
pub(crate) fn targets(pattern: [u8; 4], inventory: &[String]) -> Vec<String> {
    if pattern == [255; 4] {
        return Vec::new();
    }
    inventory
        .iter()
        .filter(|id| {
            let Some(fields) = id.strip_prefix('m') else {
                return false;
            };
            let fields: Vec<_> = fields
                .split('_')
                .filter_map(|v| v.parse::<u8>().ok())
                .collect();
            fields.len() == 4
                && pattern.iter().enumerate().all(|(i, &p)| {
                    if (p as i8) < 0 {
                        i != 1 || fields[i] == 0
                    } else {
                        p == fields[i]
                    }
                })
        })
        .cloned()
        .collect()
}

pub(crate) fn union(into: &mut Mask, mask: Mask) {
    for (a, b) in into.iter_mut().zip(mask) {
        *a |= b;
    }
}

/// Condition is a group index, never a story flag.
pub(crate) fn display_mask(row: &Value) -> Mask {
    let data = &row["display_group"];
    let mut mask = groups8(&data["disp_groups"]);
    if let Some(bit) = data["condition"].as_i64().filter(|b| (0..256).contains(b)) {
        mask[bit as usize / 32] |= 1 << (bit as usize % 32);
    }
    mask
}

/// Loader preprocessing, retaining the editor's field names in JSON.
pub(crate) fn effective_masks(row: &Value) -> (Mask, Mask) {
    let masks = visibility::SourceMasks::from_row(row)
        .expect("validated MSB display data")
        .effective(row["kind"].as_str().unwrap_or_default());
    (masks.draw, masks.backread)
}

/// Stable block identity on every scene root, for jobs C/E/F and material routing.
#[derive(Component, Clone, Debug)]
pub(crate) struct BlockPart {
    pub(crate) block: String,
    pub(crate) part_index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RequestState {
    Dormant,
    Requested,
    Ready,
    Retiring,
}

pub(crate) struct Block {
    pub(crate) id: String,
    pub(crate) prefix: String,
    pub(crate) json: Value,
    pub(crate) collision: CollisionMesh,
    pub(crate) normal: Mask,
    pub(crate) additional: Mask,
    pub(crate) state: RequestState,
    pub(crate) handles: Vec<Handle<Gltf>>,
    assets_ready: bool,
    /// Distinct GLBs this block streams; it is ready once all of them have loaded.
    glb_total: usize,
    warm_mask: Mask,
    collisions: Vec<Surface>,
    retire_frames: u8,
    detail: BTreeMap<String, [bool; 3]>,
}

struct Surface {
    name: String,
    first: usize,
    count: usize,
    min: Vec2,
    max: Vec2,
    demand: Mask,
    backread: Mask,
}

/// C/E consume the per-block demand here; F uses BlockPart and its authored row.
/// Cached GLBs stay warm for the bounded neighbourhood after demand retires.
#[derive(Resource)]
pub(crate) struct MapWorld {
    pub(crate) blocks: BTreeMap<String, Block>,
    pub(crate) primary: String,
    pub(crate) selected: Option<(String, String)>,
    visibility_maps: BTreeMap<String, visibility::VisibilityMap>,
    requests: BTreeMap<String, visibility::BlockRequest>,
    lod: super::lod::LodTable,
    /// Fixed offscreen captures need only geometry intersecting their camera volumes
    /// and the renderer's existing caster budget. Interactive worlds never set this.
    pub(crate) capture_views: Option<CaptureViews>,
}

pub(crate) struct CaptureViews {
    pub(crate) views: Vec<(Frustum, Vec3)>,
    pub(crate) caster_distance: f32,
}

impl CaptureViews {
    fn includes(&self, min: Vec3, max: Vec3, transform: &Transform) -> bool {
        let bounds = Aabb::from_min_max(min, max);
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = Vec3::splat(f32::NEG_INFINITY);
        for x in [min.x, max.x] {
            for y in [min.y, max.y] {
                for z in [min.z, max.z] {
                    let corner = transform.transform_point(Vec3::new(x, y, z));
                    lo = lo.min(corner);
                    hi = hi.max(corner);
                }
            }
        }
        self.views.iter().any(|(frustum, eye)| {
            frustum.intersects_obb(&bounds, &transform.compute_affine(), true, false)
                || eye.distance_squared(eye.clamp(lo, hi)) <= self.caster_distance.powi(2)
        })
    }
}

#[derive(Component)]
pub(crate) struct DetailPart {
    state: super::lod::LodState,
    rule: super::lod::LodRule,
    scales: super::lod::LodScales,
    min: Vec3,
    max: Vec3,
    available: [bool; 3],
    slots: [Option<Entity>; 3],
    /// The model's GLB stream request (`None` for offscreen captures, which spawn every level
    /// up front), the level drawn now, and which levels have a scene instance.
    stream: Option<usize>,
    shown: usize,
    spawned: [bool; 3],
}

/// Submission weight retained for the eventual retail coverage shader.
#[derive(Component)]
pub(crate) struct DetailWeight(pub(crate) f32);

fn detail_levels(path: &Path) -> Result<[bool; 3]> {
    let mut file = std::fs::File::open(path)?;
    let mut header = [0; 20];
    file.read_exact(&mut header)?;
    anyhow::ensure!(
        &header[..4] == b"glTF" && &header[16..] == b"JSON",
        "invalid GLB header"
    );
    let size = u32::from_le_bytes(header[12..16].try_into()?) as usize;
    anyhow::ensure!(size <= 64 * 1024 * 1024, "GLB JSON exceeds metadata limit");
    let mut bytes = vec![0; size];
    file.read_exact(&mut bytes)?;
    let doc: Value = serde_json::from_slice(&bytes)?;
    if doc["extras"]["sekiro_lod_version"] != 1 {
        return Ok([true, false, false]);
    }
    Ok(std::array::from_fn(|i| {
        doc["scenes"][i]["nodes"]
            .as_array()
            .is_some_and(|n| !n.is_empty())
    }))
}

fn read_layout(dir: &Path) -> Result<Value> {
    let mut layout: Value = serde_json::from_slice(&std::fs::read(dir.join("layout.json"))?)?;
    if layout["layout_version"].as_u64().unwrap_or(0) >= 2 {
        return Ok(layout);
    }
    // Neighbours need not be re-exported. Read their metadata without changing the cache.
    let id = dir.file_name().and_then(|v| v.to_str()).context("map ID")?;
    let cache = dir.parent().and_then(Path::parent).context("cache root")?;
    let msb = sekiro_formats::msb::parse(&std::fs::read(
        cache.join(format!("raw/map/mapstudio/{id}.msb")),
    )?)?;
    let mut parts = Vec::new();
    let mut connections = Vec::new();
    for (index, p) in msb.parts.iter().enumerate() {
        let kind = format!("{:?}", p.kind);
        let key = match kind.as_str() {
            "MapPiece" => "pieces",
            "Object" => "objects",
            "Collision" => "collisions",
            _ => "",
        };
        let mut row = layout[key]
            .as_array()
            .and_then(|a| a.iter().find(|r| r["name"] == p.name))
            .cloned()
            .unwrap_or_else(|| json!({}));
        row["name"] = json!(p.name);
        let mirror = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
        let (scale, rotation, translation) =
            (mirror * p.matrix() * mirror).to_scale_rotation_translation();
        row["translation"] = json!(translation.to_array());
        row["rotation"] = json!(rotation.normalize().to_array());
        row["scale"] = json!(scale.to_array());
        row["model"] = json!(msb.model(p).map(|m| &m.name));
        row["part_index"] = json!(index);
        row["kind"] = json!(kind);
        row["entity_id"] = json!(p.entity_id);
        row["entity_group_ids"] = json!(p.entity_group_ids);
        row["lod_param_id"] = json!(p.lod_param_id);
        row["gparam"] = json!(p.gparam);
        row["display_data"] = p.display_data.as_ref().map(|d| json!({"display_groups":d.display_groups,"draw_groups":d.draw_groups,"collision_mask":d.collision_mask,"condition1":d.condition1,"condition2":d.condition2})).unwrap_or(Value::Null);
        row["display_group"] = p
            .display_group
            .as_ref()
            .map(|d| json!({"condition":d.condition,"disp_groups":d.disp_groups}))
            .unwrap_or(Value::Null);
        if let Some((ci, map_id)) = p.connect {
            row["connection"] = json!({"map_id":map_id,"source_collision":msb.collision_by_index(ci).map(|c| json!({"name":c.name}))});
            connections.push(row.clone());
        }
        parts.push(row);
    }
    layout["parts"] = json!(parts);
    layout["connections"] = json!(connections);
    Ok(layout)
}

fn map_inventory(cache: &Path) -> Result<Vec<String>> {
    let mut ids: Vec<_> = std::fs::read_dir(cache.join("maps"))?
        .filter_map(Result::ok)
        .filter(|e| e.path().join("layout.json").is_file())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    ids.sort();
    Ok(ids)
}

fn pattern(row: &Value) -> Option<[u8; 4]> {
    serde_json::from_value(row["connection"]["map_id"].clone()).ok()
}

fn neighbourhood(cache: &Path) -> Result<BTreeMap<String, Value>> {
    let inventory = map_inventory(cache)?;
    let mut layouts = BTreeMap::new();
    let mut wanted = BTreeSet::new();
    for id in ASHINA {
        let layout = read_layout(&cache.join("maps").join(id))?;
        wanted.insert(id.to_string());
        for row in layout["connections"].as_array().into_iter().flatten() {
            // Unbound rows cannot contribute demand.
            if row["connection"]["source_collision"].is_null() {
                continue;
            }
            if let Some(p) = pattern(row) {
                wanted.extend(targets(p, &inventory));
            }
        }
        layouts.insert(id.to_string(), layout);
    }
    for id in wanted {
        if let std::collections::btree_map::Entry::Vacant(entry) = layouts.entry(id) {
            let layout = read_layout(&cache.join("maps").join(entry.key()))?;
            entry.insert(layout);
        }
    }
    Ok(layouts)
}

/// Physics bootstrap uses the same bounded block inventory as rendering.
/// Collision/group records remain separate in MapWorld; only the broad-phase query
/// mesh is concatenated, without adding MapOffset or translating any vertex.
/// Each block's helper parts the player does not collide with (`sekiro_sim::hit_filter`)
/// leave the walkable mesh first and join the world as camera-only triangles or event walls.
pub(crate) fn collision_world(
    cache: &Path,
    id: &str,
) -> Result<sekiro_sim::collision::CollisionWorld> {
    let layouts = if !enabled(id) || std::env::args().any(|a| a == "--no-connected-blocks") {
        BTreeMap::from([(id.to_string(), read_layout(&cache.join("maps").join(id))?)])
    } else {
        neighbourhood(cache)?
    };
    let mut mesh = CollisionMesh::default();
    let mut helpers = sekiro_sim::hit_filter::HelperCollision::default();
    for (block, layout) in &layouts {
        let path = cache.join("maps").join(block).join("collision.bin");
        if path.is_file() {
            let mut part = CollisionMesh::from_bytes(&std::fs::read(path)?)?;
            helpers.extend(sekiro_sim::hit_filter::strip_helpers(&mut part, layout));
            mesh.extend(&part);
        }
    }
    eprintln!(
        "collision: helper triangles kept off the player by hit filter {:?}, {} event walls",
        helpers.removed,
        helpers.triggers.len()
    );
    let mut world = sekiro_sim::collision::CollisionWorld::from_mesh(&mesh);
    world.add_helpers(&helpers);
    Ok(world)
}

impl MapWorld {
    pub(crate) fn load(cache: &Path, primary: &str, prefix: &str) -> Result<Self> {
        let mut blocks = BTreeMap::new();
        for (id, json) in neighbourhood(cache)? {
            let dir = cache.join("maps").join(&id);
            let collision = if dir.join("collision.bin").is_file() {
                CollisionMesh::from_bytes(&std::fs::read(dir.join("collision.bin"))?)?
            } else {
                CollisionMesh::default()
            };
            let mut collisions = Vec::new();
            for row in json["parts"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|r| r["kind"] == "Collision")
            {
                let Some(range) = row["triangles"].as_array() else {
                    continue;
                };
                let first = range.first().and_then(Value::as_u64).unwrap_or(0) as usize;
                let count = range.get(1).and_then(Value::as_u64).unwrap_or(0) as usize;
                let Some(indices) = collision.indices.get(first * 3..(first + count) * 3) else {
                    anyhow::bail!("{id}: collision triangle range is invalid");
                };
                if indices.is_empty() {
                    continue;
                }
                let mut min = Vec2::splat(f32::MAX);
                let mut max = Vec2::splat(f32::MIN);
                for &i in indices {
                    let v = collision
                        .vertices
                        .get(i as usize)
                        .context("collision vertex index")?;
                    min = min.min(Vec2::new(v[0], v[2]));
                    max = max.max(Vec2::new(v[0], v[2]));
                }
                let (_, backread) = effective_masks(row);
                collisions.push(Surface {
                    name: row["name"].as_str().unwrap_or_default().to_string(),
                    first,
                    count,
                    min,
                    max,
                    demand: display_mask(row),
                    backread,
                });
            }
            let block_prefix = if id == primary {
                prefix.to_string()
            } else if prefix.is_empty() {
                format!("../{id}/")
            } else {
                format!("maps/{id}/")
            };
            let mut detail = BTreeMap::new();
            if enabled(&id) {
                for (model, entry) in json["models"].as_object().context("models table")? {
                    if model.starts_with('m')
                        && let Some(glb) = entry["glb"].as_str()
                    {
                        detail.insert(model.clone(), detail_levels(&dir.join(glb))?);
                    }
                }
            }
            blocks.insert(
                id.clone(),
                Block {
                    id,
                    prefix: block_prefix,
                    json,
                    collision,
                    collisions,
                    normal: [0; 8],
                    additional: [0; 8],
                    state: RequestState::Dormant,
                    handles: Vec::new(),
                    assets_ready: false,
                    glb_total: 0,
                    warm_mask: [0; 8],
                    retire_frames: 0,
                    detail,
                },
            );
        }
        info!(
            "connected world {primary}: bounded warm blocks {:?}",
            blocks.keys().collect::<Vec<_>>()
        );
        let mut visibility_maps = BTreeMap::new();
        for (id, block) in &blocks {
            if let Some(map) = visibility::VisibilityMap::from_layout(id, &block.json)? {
                visibility_maps.insert(id.clone(), map);
            }
        }
        let inventory: Vec<_> = blocks.keys().cloned().collect();
        let mut warm_masks = BTreeMap::<String, Mask>::new();
        for id in ASHINA {
            for row in blocks[id].json["connections"]
                .as_array()
                .into_iter()
                .flatten()
            {
                if row["connection"]["source_collision"].is_null() {
                    continue;
                }
                if let Some(p) = pattern(row) {
                    for target in targets(p, &inventory) {
                        union(warm_masks.entry(target).or_default(), display_mask(row));
                    }
                }
            }
        }
        for (id, block) in &mut blocks {
            block.warm_mask = if enabled(id) {
                [u32::MAX; 8]
            } else {
                warm_masks.get(id).copied().unwrap_or_default()
            };
        }
        let lod = super::lod::LodTable::load(cache, 6)?;
        for (id, block) in &blocks {
            if !enabled(id) {
                continue;
            }
            for row in block.json["parts"].as_array().context("canonical parts")? {
                if matches!(row["kind"].as_str(), Some("MapPiece" | "Object")) {
                    visibility::VisibilityPart::from_row(id, row)?;
                    if row["kind"] == "MapPiece" {
                        lod.map_piece(
                            row["lod_param_id"].as_u64().context("authored LOD ID")? as u8
                        )?;
                    }
                }
            }
        }
        Ok(Self {
            blocks,
            primary: primary.to_string(),
            selected: None,
            visibility_maps,
            requests: BTreeMap::new(),
            lod,
            capture_views: None,
        })
    }

    /// Recalculate all masks, so stale connection demand cannot leak across blocks.
    pub(crate) fn demand(&mut self, positions: &[Vec3], story: Option<&super::story::MapStory>) {
        for block in self.blocks.values_mut() {
            block.normal = [0; 8];
            block.additional = [0; 8];
        }
        let inventory: Vec<_> = self.blocks.keys().cloned().collect();
        let mut contributions = Vec::new();
        let mut participating = BTreeMap::<String, BTreeSet<String>>::new();
        self.selected = None;
        for &pos in positions {
            let mut hits = Vec::new();
            for (id, block) in &self.blocks {
                for (index, surface) in block.collisions.iter().enumerate() {
                    if story
                        .and_then(|s| s.maps.get(id))
                        .and_then(|s| s.part(&surface.name))
                        .is_some_and(|p| !p.enabled)
                    {
                        continue;
                    }
                    if pos.xz().cmplt(surface.min).any() || pos.xz().cmpgt(surface.max).any() {
                        continue;
                    }
                    let score = block.collision.indices
                        [surface.first * 3..(surface.first + surface.count) * 3]
                        .as_chunks::<3>()
                        .0
                        .iter()
                        .filter_map(|tri| {
                            let [a, b, c] =
                                tri.map(|i| Vec3::from(block.collision.vertices[i as usize]));
                            vertical_hit(pos, a, b, c)
                        })
                        .map(|h| if h >= -0.5 { h } else { 1000.0 - h })
                        .min_by(f32::total_cmp);
                    if let Some(score) = score {
                        hits.push((score, id.clone(), index));
                    }
                }
            }
            hits.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            let Some((best, _, _)) = hits.first() else {
                continue;
            };
            // Supporting/player/camera collisions can coexist at a seam. The 0.5 m
            // tie band is an explicit occupancy approximation pending job E's policy.
            for (_, id, index) in hits.iter().take_while(|h| h.0 <= best + 0.5) {
                let block = &self.blocks[id];
                let surface = &block.collisions[*index];
                participating
                    .entry(id.clone())
                    .or_default()
                    .insert(surface.name.clone());
                if self.selected.is_none() {
                    self.selected = Some((id.clone(), surface.name.clone()));
                }
                contributions.push((id.clone(), surface.demand, surface.backread));
                for row in block.json["connections"].as_array().into_iter().flatten() {
                    if row["connection"]["source_collision"]["name"].as_str() != Some(&surface.name)
                    {
                        continue;
                    }
                    if let Some(p) = pattern(row) {
                        for target in targets(p, &inventory) {
                            contributions.push((target, display_mask(row), [0; 8]));
                        }
                    }
                }
            }
        }
        for (id, normal, additional) in contributions {
            let block = self.blocks.get_mut(&id).expect("inventory target");
            union(&mut block.normal, normal);
            union(&mut block.additional, additional);
        }
        self.requests.clear();
        for (id, names) in participating {
            if let Some(map) = self.visibility_maps.get(&id) {
                for (target, request) in map.requests(
                    names.iter().map(String::as_str),
                    |source, _| {
                        !story
                            .and_then(|s| s.maps.get(&id))
                            .and_then(|s| s.part(source))
                            .is_some_and(|p| !p.enabled)
                    },
                    |target| self.blocks.contains_key(target),
                ) {
                    self.requests.entry(target).or_default().merge(request);
                }
            }
        }
    }
}

pub(crate) fn update(
    mut world: ResMut<MapWorld>,
    assets: Res<AssetServer>,
    focus: Query<&Transform, With<MapFocus>>,
    cameras: Query<&Transform, With<Camera3d>>,
    mut last: Local<Option<Vec<Vec3>>>,
    story: Res<super::story::MapStory>,
) {
    let positions: Vec<_> = focus
        .iter()
        .chain(cameras.iter())
        .map(|t| t.translation)
        .collect();
    if last.as_ref() != Some(&positions) {
        world.demand(&positions, Some(&story));
        *last = Some(positions);
    }
    for block in world.blocks.values_mut() {
        // Streaming requests a block's GLBs over time; it is ready once every one has loaded.
        block.assets_ready = block.assets_ready
            || (block.handles.len() >= block.glb_total
                && block
                    .handles
                    .iter()
                    .all(|h| assets.is_loaded_with_dependencies(h.id())));
        let demanded = block.normal != [0; 8] || block.additional != [0; 8];
        let next = if demanded {
            block.retire_frames = 0;
            if block.assets_ready {
                RequestState::Ready
            } else {
                RequestState::Requested
            }
        } else if matches!(block.state, RequestState::Ready | RequestState::Requested) {
            block.retire_frames = 2;
            RequestState::Retiring
        } else if block.retire_frames > 0 {
            block.retire_frames -= 1;
            RequestState::Retiring
        } else {
            RequestState::Dormant
        };
        if next != block.state {
            info!(
                "block {} {:?} -> {:?}, normal={:08x?}, additional={:08x?}",
                block.id, block.state, next, block.normal, block.additional
            );
            block.state = next;
        }
    }
}

pub(crate) fn publish(
    world: Res<MapWorld>,
    groups: Res<super::Groups>,
    mut blocks: ResMut<visibility::VisibilityBlocks>,
    story: Res<super::story::MapStory>,
    mut authored: ResMut<visibility::AuthoredVisibility>,
) {
    if story.is_changed() {
        authored.maps = story
            .maps
            .iter()
            .map(|(id, map)| {
                (
                    id.clone(),
                    visibility::EventState {
                        by_name: map
                            .parts
                            .iter()
                            .map(|part| {
                                (
                                    part.name.clone(),
                                    map.enabled(part.entity_id, &part.name).unwrap_or(true),
                                )
                            })
                            .collect(),
                        by_entity: BTreeMap::new(),
                    },
                )
            })
            .collect();
    }
    for (id, block) in &world.blocks {
        let mut request = world.requests.get(id).cloned().unwrap_or_default();
        union(&mut request.normal, block.normal);
        union(&mut request.additional_backread, block.additional);
        blocks.0.insert(
            id.clone(),
            visibility::BlockVisibility {
                request,
                draw: block.normal,
                // A demanded block draws while it streams: each part appears once its own
                // scene has spawned, so the area around the player never waits for the rest.
                resident: matches!(block.state, RequestState::Ready | RequestState::Requested)
                    || groups.mode == GroupMode::Off,
                ready: true,
                groups_off: groups.mode == GroupMode::Off,
            },
        );
    }
}

/// Instantiate eligible canonical rows once; geometry stays warm while its request
/// state changes. Jobs C/E/F attach state through BlockPart without duplicate roots.
///
/// Interactive worlds queue each row's GLB nearest-first from `scene.focus` and stream them
/// ([`super::stream_assets`]); a MapPiece instantiates only the level of detail it shows at
/// the start, and [`update_detail`] adds another level the first time it is selected.
/// Fixed captures (`capture_views`) keep loading every level at once.
pub(crate) fn spawn(
    commands: &mut Commands,
    assets: &AssetServer,
    world: &mut MapWorld,
    scene: &super::MapScene,
    pending: &mut Pending,
) {
    if scene.no_pieces {
        return;
    }
    let mut spawned = 0;
    let mut capture_roots = BTreeMap::new();
    let args: Vec<_> = std::env::args().collect();
    let only_part = crate::arg_value(&args, "--world-part");
    // Nearest first across every block, so the stream loads outwards from the start.
    let mut order = Vec::new();
    for (id, block) in &world.blocks {
        for (index, row) in block.json["parts"]
            .as_array()
            .into_iter()
            .flatten()
            .enumerate()
        {
            let Some(transform) = super::row_transform(row) else {
                continue;
            };
            let distance = match (scene.focus, row["model"].as_str()) {
                (Some(focus), Some(model)) => {
                    super::placed_distance(&block.json["models"][model], focus, &transform)
                }
                _ => 0.0,
            };
            order.push((distance, id.clone(), index, transform));
        }
    }
    order.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut seen = BTreeMap::<String, BTreeSet<String>>::new();
    let mut skipped = BTreeMap::<String, BTreeMap<String, usize>>::new();
    for (_, id, index, transform) in order {
        let block = world.blocks.get_mut(&id).expect("listed block");
        let row = &block.json["parts"][index];
        if only_part
            .as_ref()
            .is_some_and(|name| row["name"].as_str() != Some(name))
        {
            continue;
        }
        if row["kind"] != "MapPiece" && row["kind"] != "Object" {
            continue;
        }
        let Some(model) = row["model"].as_str() else {
            continue;
        };
        let reason = row["render_status"]["reason"].as_str().unwrap_or("legacy");
        if !matches!(reason, "exported" | "legacy_zero_draw_groups" | "legacy") {
            *skipped
                .entry(block.id.clone())
                .or_default()
                .entry(reason.to_string())
                .or_default() += 1;
            continue;
        }
        if scene
            .hide
            .iter()
            .any(|h| model.starts_with(h) || row["name"].as_str() == Some(h))
        {
            continue;
        }
        let Some(glb) = block.json["models"][model]["glb"].as_str() else {
            continue;
        };
        if let (Some(coverage), Some(min), Some(max)) = (
            world.capture_views.as_ref(),
            super::vec3(&block.json["models"][model]["min"]),
            super::vec3(&block.json["models"][model]["max"]),
        ) && !coverage.includes(min, max, &transform)
        {
            continue;
        }
        let (draw, backread) = effective_masks(row);
        if !enabled(&block.id)
            && !draw
                .into_iter()
                .zip(block.warm_mask)
                .any(|(a, b)| a & b != 0)
            && !backread
                .into_iter()
                .zip(block.warm_mask)
                .any(|(a, b)| a & b != 0)
        {
            continue;
        }
        let glb = format!("{}{glb}", block.prefix);
        let capture_handle: Option<Handle<Gltf>> = world
            .capture_views
            .as_ref()
            .map(|_| super::load_map_asset(assets, glb.clone(), true));
        if capture_handle.is_some() && !capture_roots.contains_key(&glb) {
            match super::capture::scene_roots(&sekiro_sim::cache::root().join(&glb)) {
                Ok(roots) => {
                    capture_roots.insert(glb.clone(), roots);
                }
                Err(error) => {
                    error!("capture {glb}: {error:#}");
                    continue;
                }
            }
        }
        let first_sight = seen
            .entry(block.id.clone())
            .or_default()
            .insert(glb.clone());
        if let Some(handle) = &capture_handle
            && first_sight
        {
            // A sweep's legacy asset root is the primary map folder. Its bounded,
            // inventory-validated sibling paths are explicitly allowed to load.
            pending.handles.push(handle.clone().untyped());
            block.handles.push(handle.clone());
        }
        let source = match visibility::SourceMasks::from_row(row) {
            Ok(source) => source,
            Err(error) => {
                error!("{} {}: {error:#}", block.id, row["name"]);
                continue;
            }
        };
        let part_visibility = visibility::VisibilityPart::from_row(&block.id, row)
            .expect("validated canonical row")
            .unwrap_or_else(|| visibility::VisibilityPart {
                map: block.id.clone(),
                name: row["name"].as_str().unwrap_or_default().to_string(),
                entity_id: row["entity_id"].as_i64().unwrap_or(0) as i32,
                entity_groups: groups8(&row["entity_group_ids"]).map(|v| v as i32),
                effective: source.effective(row["kind"].as_str().unwrap_or_default()),
                source,
            });
        let root = commands
            .spawn((
                part_visibility,
                BlockPart {
                    block: block.id.clone(),
                    part_index: row["part_index"].as_u64().unwrap_or(0) as usize,
                },
                super::DrawGroups(draw),
                transform,
                Visibility::Hidden,
            ))
            .id();
        if enabled(&block.id)
            && row["kind"] == "MapPiece"
            && let (Some(min), Some(max), Some(&available)) = (
                super::vec3(&block.json["models"][model]["min"]),
                super::vec3(&block.json["models"][model]["max"]),
                block.detail.get(model),
            )
        {
            let rule = world
                .lod
                .map_piece(row["lod_param_id"].as_u64().unwrap_or(255) as u8)
                .expect("validated LOD row");
            let scales = super::lod::LodScales {
                platform: world.lod.platform_scale,
                ..default()
            };
            // The level the first update selects; capture views and the file-order fallback
            // keep the first available level.
            let initial = match scene.focus {
                Some(focus) if capture_handle.is_none() => {
                    let distance = super::lod::bounds_distance(focus, &transform, min, max);
                    let weights = super::lod::LodState::default()
                        .update(rule, scales, distance, 0.0, available);
                    dominant(weights)
                }
                _ => available.iter().position(|&v| v).unwrap_or(0),
            };
            let mut slots = [None; 3];
            let mut stream = None;
            for (level, &exists) in available.iter().enumerate() {
                if !exists {
                    continue;
                }
                let mut child = commands.spawn((
                    Transform::IDENTITY,
                    if level == initial {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    },
                    DetailWeight(f32::from(level == initial)),
                    ChildOf(root),
                ));
                if let Some(handle) = &capture_handle {
                    child.insert(super::capture::CaptureScene {
                        gltf: handle.clone(),
                        level,
                        roots: capture_roots[&glb][level].clone(),
                    });
                    pending.add_instance();
                } else if level == initial {
                    stream = Some(pending.queue(&glb, Some(&block.id), true, (child.id(), level)));
                }
                slots[level] = Some(child.id());
            }
            commands.entity(root).insert(DetailPart {
                state: super::lod::LodState::default(),
                rule,
                scales,
                min,
                max,
                available,
                slots,
                stream,
                shown: initial,
                spawned: std::array::from_fn(|level| {
                    capture_handle.is_some() && available[level] || level == initial
                }),
            });
        } else if let Some(handle) = &capture_handle {
            commands.entity(root).insert(super::capture::CaptureScene {
                gltf: handle.clone(),
                level: 0,
                roots: capture_roots[&glb][0].clone(),
            });
            pending.add_instance();
        } else {
            pending.queue(&glb, Some(&block.id), true, (root, 0));
        }
        spawned += 1;
    }
    for block in world.blocks.values_mut() {
        block.glb_total = seen.get(&block.id).map_or(0, BTreeSet::len);
        info!(
            "{} warm models {}, skipped render reasons {:?}",
            block.id,
            block.glb_total,
            skipped.get(&block.id).cloned().unwrap_or_default()
        );
    }
    info!(
        "spawned {spawned} connected map piece and ordinary Object instances, {} GLBs queued nearest-first",
        pending.streaming()
    );
}

/// The level with the largest weight, as [`update_detail`] draws it.
fn dominant(weights: [f32; 3]) -> usize {
    weights
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(i, _)| i)
        .unwrap_or(0)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn update_detail(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut pending: ResMut<Pending>,
    instances: Res<WorldInstanceSpawner>,
    time: Res<Time>,
    groups: Res<super::Groups>,
    cameras: Query<&Transform, With<Camera3d>>,
    mut parts: Query<(&Transform, &BlockPart, &mut DetailPart), Without<Camera3d>>,
    mut slots: Query<(
        &mut Visibility,
        &mut DetailWeight,
        Option<&WorldAssetRoot>,
        Option<&WorldInstance>,
    )>,
    mut debug: Local<Option<bool>>,
) {
    let _timer = crate::perf_log::SystemTimer::start("update_detail");
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    let debug = *debug.get_or_insert_with(|| std::env::args().any(|a| a == "--lod-debug"));
    for (transform, identity, mut part) in &mut parts {
        let distance =
            super::lod::bounds_distance(camera.translation, transform, part.min, part.max);
        let rule = part.rule;
        let scales = part.scales;
        let available = part.available;
        let weights = if groups.mode == GroupMode::Off {
            let mut weights = [0.; 3];
            if let Some(first) = available.iter().position(|&v| v) {
                weights[first] = 1.;
            }
            weights
        } else {
            part.state
                .update(rule, scales, distance, time.delta_secs(), available)
        };
        // The retail coverage shader is not traced. Until that renderer interface
        // exists, keep the dominant representation, preserving exact weights as data.
        let selected = dominant(weights);
        // A level selected for the first time gets its scene now. Every level lives in the same
        // GLB, so once the stream has requested it the scene comes from that load; before
        // then the current level stays, keeping the stream's order and in-flight limit.
        if !part.spawned[selected]
            && let Some(entity) = part.slots[selected]
            && let Some(scene) = part
                .stream
                .and_then(|index| pending.requested(index))
                .map(|(glb, unapproved)| super::scene_handle(&assets, glb, selected, unapproved))
        {
            commands.entity(entity).insert(WorldAssetRoot(scene));
            part.spawned[selected] = true;
            pending.add_instance();
        }
        // Keep drawing the current level until the selected one has spawned its meshes, so a
        // switch never leaves a gap. Capture scenes are spawned synchronously.
        if part.shown != selected
            && (part.stream.is_none()
                || part.slots[selected]
                    .and_then(|entity| slots.get(entity).ok())
                    .is_some_and(|(_, _, root, instance)| {
                        root.is_some() && instance.is_some_and(|i| instances.instance_is_ready(**i))
                    }))
        {
            part.shown = selected;
        }
        let shown = part.shown;
        for (level, entity) in part.slots.iter().enumerate() {
            if let Some(entity) = entity
                && let Ok((mut visibility, mut weight, _, _)) = slots.get_mut(*entity)
            {
                let desired = if level == shown {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                if *visibility != desired {
                    *visibility = desired;
                    if debug {
                        info!(
                            "{} part {} detail {}, weights {:?}",
                            identity.block, identity.part_index, shown, weights
                        );
                    }
                }
                weight.0 = weights[level];
            }
        }
    }
}

/// One-shot hierarchy audit after loading, for diagnosing scene handoffs at fixed poses.
pub(crate) fn debug_hierarchy(world: &mut World) {
    if !std::env::args().any(|a| a == "--world-debug")
        || !world.resource::<Pending>().loaded()
        || world.contains_resource::<HierarchyReported>()
    {
        return;
    }
    let roots: Vec<_> = world
        .query::<(Entity, &BlockPart, &DetailPart)>()
        .iter(world)
        .filter(|(_, id, _)| id.block == "m11_01_00_00")
        .take(3)
        .map(|(entity, id, detail)| (entity, id.part_index, detail.slots))
        .collect();
    for (root, index, slots) in roots {
        info!(
            "world hierarchy root part {}: visibility {:?}, inherited {:?}, transform {:?}",
            index,
            world.get::<Visibility>(root),
            world.get::<InheritedVisibility>(root),
            world.get::<GlobalTransform>(root)
        );
        for (level, slot) in slots.iter().enumerate() {
            let Some(slot) = slot else {
                continue;
            };
            info!(
                "world hierarchy level {}: visibility {:?}, inherited {:?}, parent {:?}, transform {:?}",
                level,
                world.get::<Visibility>(*slot),
                world.get::<InheritedVisibility>(*slot),
                world.get::<ChildOf>(*slot),
                world.get::<GlobalTransform>(*slot)
            );
            let mut queue = vec![*slot];
            let mut meshes = 0;
            while let Some(entity) = queue.pop() {
                if world.get::<Mesh3d>(entity).is_some() {
                    if meshes < 2 {
                        info!(
                            "world hierarchy mesh: visibility {:?}, inherited {:?}, transform {:?}",
                            world.get::<Visibility>(entity),
                            world.get::<InheritedVisibility>(entity),
                            world.get::<GlobalTransform>(entity)
                        );
                    }
                    meshes += 1;
                }
                if let Some(children) = world.get::<Children>(entity) {
                    queue.extend(children.iter());
                }
            }
            info!("world hierarchy level {level}: {meshes} meshes");
        }
    }
    world.insert_resource(HierarchyReported);
}

#[derive(Resource)]
struct HierarchyReported;

#[cfg(test)]
mod tests {
    #[test]
    fn material_layers_resolve_from_the_owning_model_and_preserve_asset_source() {
        for owner in ["m11_00_00_00", "m11_01_00_00", "m13_00_00_00"] {
            for source in ["", "cache://"] {
                let model = format!("{source}maps/{owner}/models/ground.glb#Material0");
                let path = bevy::asset::AssetPath::parse(&model);
                assert_eq!(
                    super::relative_material_texture(&path, "tex/ground.dds")
                        .map(|p| p.replace('\\', "/")),
                    Some(format!("{source}maps/{owner}/models/tex/ground.dds"))
                );
                assert!(super::relative_material_texture(&path, "tex/../other.dds").is_none());
            }
        }
    }

    #[test]
    fn fixed_capture_keeps_offscreen_shadow_casters_and_crossing_bounds() {
        use bevy::camera::CameraProjection;
        let coverage = super::CaptureViews {
            views: vec![(
                PerspectiveProjection::default().compute_frustum(&GlobalTransform::IDENTITY),
                Vec3::ZERO,
            )],
            caster_distance: 5.,
        };
        assert!(coverage.includes(
            Vec3::new(-1., -1., 2.),
            Vec3::new(1., 1., 3.),
            &Transform::IDENTITY
        ));
        assert!(!coverage.includes(
            Vec3::new(-1., -1., 20.),
            Vec3::new(1., 1., 21.),
            &Transform::IDENTITY
        ));
        assert!(coverage.includes(
            Vec3::new(-1000., -1., -10.),
            Vec3::new(1000., 1., -9.),
            &Transform::IDENTITY
        ));
    }
    use super::*;

    #[test]
    fn retail_wildcards_and_connection_mask_union() {
        let inventory = [
            "m11_00_00_00",
            "m11_01_00_00",
            "m13_00_00_00",
            "m13_01_00_00",
            "m13_00_00_99",
        ]
        .map(str::to_string);
        assert_eq!(
            targets([13, 255, 255, 255], &inventory),
            vec!["m13_00_00_00", "m13_00_00_99"]
        );
        assert!(targets([255; 4], &inventory).is_empty());
        let mut mask = display_mask(
            &json!({"display_group":{"condition":65,"disp_groups":[1,0,0,0,0,0,0,0]}}),
        );
        union(
            &mut mask,
            display_mask(
                &json!({"display_group":{"condition":66,"disp_groups":[8,0,0,0,0,0,0,0]}}),
            ),
        );
        assert_eq!(mask, [9, 0, 6, 0, 0, 0, 0, 0]);
        assert_eq!(
            display_mask(&json!({"display_group":{"condition":-1,"disp_groups":[16]}})),
            [16, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn installed_neighbourhood_keeps_blocks_and_baked_transforms() {
        let cache = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cache");
        if !cache.join("maps/m11_01_00_00/layout.json").is_file() {
            return;
        }
        let mut world = MapWorld::load(&cache, ASHINA[0], "maps/m11_00_00_00/").unwrap();
        assert_eq!(
            world.blocks.keys().map(String::as_str).collect::<Vec<_>>(),
            vec![
                "m11_00_00_00",
                "m11_01_00_00",
                "m11_02_00_00",
                "m13_00_00_00",
                "m17_00_00_00",
                "m20_00_00_00"
            ]
        );
        let original: Value = serde_json::from_slice(
            &std::fs::read(cache.join("maps/m11_00_00_00/layout.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            world.blocks[ASHINA[0]].json["parts"][0]["translation"],
            original["parts"][0]["translation"]
        );
        let inventory: Vec<_> = world.blocks.keys().cloned().collect();
        for (source, target) in [
            (ASHINA[0], ASHINA[1]),
            (ASHINA[1], ASHINA[0]),
            (ASHINA[1], "m11_02_00_00"),
            (ASHINA[1], "m13_00_00_00"),
            (ASHINA[1], "m17_00_00_00"),
        ] {
            let block = &world.blocks[source];
            let mut candidates = Vec::new();
            for row in block.json["connections"].as_array().unwrap() {
                if !pattern(row)
                    .is_some_and(|p| targets(p, &inventory).iter().any(|id| id == target))
                {
                    continue;
                }
                let Some(name) = row["connection"]["source_collision"]["name"].as_str() else {
                    continue;
                };
                let Some(surface) = block.collisions.iter().find(|s| s.name == name) else {
                    continue;
                };
                for tri in block.collision.indices
                    [surface.first * 3..(surface.first + surface.count) * 3]
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .take(32)
                {
                    let [a, b, c] = tri.map(|i| Vec3::from(block.collision.vertices[i as usize]));
                    if (b - a).cross(c - a).normalize_or_zero().y.abs() < 50f32.to_radians().cos() {
                        continue;
                    }
                    candidates.push(((a + b + c) / 3. + Vec3::Y * 1.65, name.to_string()));
                }
            }
            let found = candidates.into_iter().find(|(pos, _)| {
                world.demand(&[*pos], None);
                world.selected.as_ref().is_some_and(|(id, _)| id == source)
                    && world.blocks[target].normal != [0; 8]
            });
            let (pose, collision) =
                found.unwrap_or_else(|| panic!("no occupied {source} -> {target} connection"));
            println!(
                "native connection {source} -> {target}: camera {pose:?}, source {collision}, target mask {:08x?}",
                world.blocks[target].normal
            );
        }
        // Exercise a grounded seam with multiple attached rows and target blocks.
        // Use the loaded world so the asset and block inventory contract is identical.
        world.visibility_maps.clear();
        world.blocks.retain(|id, _| {
            matches!(
                id.as_str(),
                "m11_00_00_00" | "m11_01_00_00" | "m11_02_00_00"
            )
        });
        for (id, lo, hi, bit) in [
            (ASHINA[0], -10., 0., 1),
            (ASHINA[1], 0., 10., 2),
            ("m11_02_00_00", 20., 30., 4),
        ] {
            let block = world.blocks.get_mut(id).unwrap();
            block.collision = CollisionMesh {
                vertices: vec![[lo, 0., -10.], [hi, 0., -10.], [hi, 0., 10.], [lo, 0., 10.]],
                indices: vec![0, 1, 2, 0, 2, 3],
                materials: vec![1, 1],
            };
            block.collisions = vec![Surface {
                name: "support".into(),
                first: 0,
                count: 2,
                min: Vec2::new(lo, -10.),
                max: Vec2::new(hi, 10.),
                demand: [bit, 0, 0, 0, 0, 0, 0, 0],
                backread: [32, 0, 0, 0, 0, 0, 0, 0],
            }];
            block.json = json!({"connections":[]});
        }
        world.blocks.get_mut(ASHINA[0]).unwrap().json["connections"] = json!([
            {"connection":{"source_collision":{"name":"support"},"map_id":[11,1,255,255]},"display_group":{"condition":65,"disp_groups":[8,0,0,0,0,0,0,0]}},
            {"connection":{"source_collision":{"name":"support"},"map_id":[11,1,255,255]},"display_group":{"condition":66,"disp_groups":[0,0,0,0,0,0,0,0]}},
            {"connection":{"source_collision":{"name":"support"},"map_id":[11,2,255,255]},"display_group":{"condition":32,"disp_groups":[0,0,0,0,0,0,0,0]}},
            {"connection":{"source_collision":null,"map_id":[11,1,255,255]},"display_group":{"condition":90,"disp_groups":[0,0,0,0,0,0,0,0]}}
        ]);
        world.demand(&[Vec3::new(-1., 1., 0.)], None);
        assert_eq!(world.blocks[ASHINA[1]].normal, [8, 0, 6, 0, 0, 0, 0, 0]);
        assert_eq!(
            world.blocks["m11_02_00_00"].normal,
            [0, 1, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            world.blocks[ASHINA[0]].additional,
            [32, 0, 0, 0, 0, 0, 0, 0]
        );
        world.demand(&[Vec3::new(0., 1., 0.)], None);
        assert_eq!(world.blocks[ASHINA[1]].normal, [10, 0, 6, 0, 0, 0, 0, 0]);
        world.demand(&[Vec3::new(1., 1., 0.)], None);
        assert_eq!(world.blocks[ASHINA[1]].normal, [2, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(world.blocks[ASHINA[0]].normal, [0; 8]);
    }
}

//! `--map <id>`: walks a map area exported by `sekiro-extract map <id>` with a free-fly camera.
//!
//! Map piece GLBs are spawned at their MSB instance transforms (already in Bevy space in
//! `layout.json`). Controls: hold the right mouse button and move the mouse to look, WASD to
//! fly, Space/E up, Q/Ctrl down, Shift for speed, mouse wheel to change the base speed, C to
//! toggle the hit collision wireframe, P to print the camera pose.
//!
//! `--screenshot <png> [--at <seconds>]` waits until every piece and texture has loaded, then
//! `--at` more seconds, saves a frame and exits. `--camera x,y,z,yaw,pitch` (degrees) places
//! the camera; otherwise it starts at the first player start point.

//!
//! [`add_map`] puts the same map (pieces, draw-group culling around a [`MapFocus`] entity, sun,
//! fog, collision view) into another mode; `--play --map` uses it.

#[cfg(test)]
#[path = "map_gate_tests.rs"]
mod gate;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use bevy::app::AppExit;
use bevy::asset::RenderAssetUsages;
use bevy::asset::{AssetPlugin, LoadState, UntypedAssetId};
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::light::GlobalAmbientLight;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::pbr::wireframe::{Wireframe, WireframeColor, WireframePlugin};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::world_serialization::WorldInstanceReady;
use bevy::world_serialization::{WorldInstance, WorldInstanceSpawner};
use serde_json::Value;

#[path = "map_story.rs"]
pub(crate) mod story;

#[path = "map_visibility.rs"]
pub(crate) mod visibility;

#[path = "map_lod.rs"]
pub(crate) mod lod;
#[path = "map_material.rs"]
mod map_material;
pub(crate) use map_material::{ATTR_MAP_BLEND2, MaterialStats};
#[path = "sky.rs"]
pub(crate) mod sky;
#[path = "map_sweep.rs"]
pub(crate) mod sweep;
#[path = "map_world.rs"]
pub(crate) mod world;

#[path = "map_lights.rs"]
mod local_lights;
use local_lights::{LocalLights, update_local_lights};

#[path = "map_capture.rs"]
mod capture;
#[path = "capture_ready.rs"]
pub(crate) mod ready;

use crate::lighting::{
    MapLighting, MapProbes, ProbeRegion, ShadowSettings, apply_lighting, apply_probes,
    nearest_variant, spawn_lights,
};

pub struct Options {
    pub cache: PathBuf,
    pub id: String,
    pub screenshot: Option<PathBuf>,
    /// Seconds to wait after everything has loaded before the screenshot.
    pub at: f32,
    /// x, y, z, yaw, pitch (degrees).
    pub camera: Option<[f32; 5]>,
    /// Start with the collision wireframe visible.
    pub collision: bool,
    /// Hide the map pieces (collision only).
    pub no_pieces: bool,
    /// Player start point to begin at when `camera` is not given.
    pub start: usize,
    /// Skip map piece models whose name starts with any of these (e.g. `m9` for far models).
    pub hide: Vec<String>,
    /// Disable vsync, to measure frame rate.
    pub uncapped: bool,
    /// Draw-group culling: which collision group block selects the visible pieces.
    pub groups: GroupMode,
    /// Also spawn MSB objects on legacy maps. Ashina Explore includes ordinary Objects.
    pub objects: bool,
    /// Time of day (hours) and draw param override, see [`MapScene`].
    pub hour: f32,
    pub gparam: Option<String>,
}

/// How map pieces are culled by draw groups. In the game, the hit collision the player stands
/// on selects which pieces draw, which swaps distant stand-ins for detailed geometry; `Display`
/// uses the collision's display groups (word block 0 of the MSB mask struct), `Draw` its draw
/// groups (block 1), matched against each piece's draw groups (block 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupMode {
    Off,
    Display,
    Draw,
}

impl GroupMode {
    pub fn parse(s: Option<&str>) -> Result<Self> {
        Ok(match s.unwrap_or("draw") {
            "off" => Self::Off,
            "display" => Self::Display,
            "draw" => Self::Draw,
            other => anyhow::bail!("--groups {other}: want off, display or draw"),
        })
    }

    fn next(self) -> Self {
        match self {
            Self::Display => Self::Draw,
            Self::Draw => Self::Off,
            Self::Off => Self::Display,
        }
    }
}

/// A piece's draw groups.
#[derive(Component)]
struct DrawGroups([u32; 8]);

/// Distant render-only scenery is independent of the local collision's draw groups.
#[derive(Component)]
struct FarScenery;

type LegacyPieceFilter = (Without<FarScenery>, Without<world::BlockPart>);

struct CollisionPart {
    name: String,
    first: usize,
    count: usize,
    min: Vec2,
    max: Vec2,
    display: [u32; 8],
    draw: [u32; 8],
    /// Light set id of the collision's gparam config (-1 when none).
    light_set: i32,
    fog_set: i32,
    scattering_set: i32,
    env_map: i32,
}

#[derive(Resource)]
pub(crate) struct Groups {
    mode: GroupMode,
    parts: Vec<CollisionPart>,
    /// Collision part under the camera and the groups in force.
    current: Option<usize>,
    active: Option<[u32; 8]>,
    checked_at: Option<Vec3>,
    dirty: bool,
}

fn groups8(v: &Value) -> [u32; 8] {
    let mut out = [0u32; 8];
    for (o, x) in out.iter_mut().zip(v.as_array().into_iter().flatten()) {
        *o = x
            .as_u64()
            .map(|v| v as u32)
            .or_else(|| x.as_i64().map(|v| v as u32))
            .unwrap_or(0);
    }
    out
}

#[derive(Resource)]
struct Fly {
    yaw: f32,
    pitch: f32,
    speed: f32,
}

#[derive(Component)]
struct CollisionView;

/// Enemy and player start markers.
#[derive(Component)]
struct Marker;

#[derive(Component)]
struct Hud;

/// Every GLB handle the map needs, to know when loading is done.
#[derive(Resource)]
pub struct Pending {
    handles: Vec<UntypedHandle>,
    /// Map piece scene instances spawned, and how many have finished spawning.
    instances: usize,
    ready: usize,
    ready_roots: std::collections::HashSet<Entity>,
    /// When [`add_map`] was called, for the load time in the log.
    added_at: Instant,
    loaded_at: Option<Instant>,
}

impl Default for Pending {
    fn default() -> Self {
        Self {
            handles: Vec::new(),
            instances: 0,
            ready: 0,
            ready_roots: std::collections::HashSet::new(),
            added_at: Instant::now(),
            loaded_at: None,
        }
    }
}

impl Pending {
    /// Every map piece and texture has loaded.
    pub fn loaded(&self) -> bool {
        self.loaded_at.is_some()
    }

    /// Seconds since every map piece and texture loaded (0 before).
    pub fn loaded_for(&self) -> f32 {
        self.loaded_at.map_or(0.0, |t| t.elapsed().as_secs_f32())
    }
}

/// The world-space horizontal bounding box of a model row placed at `p` (from the `min` and
/// `max` its `models` entry records).
fn placed_box(model: &Value, p: &Pose) -> Option<(Vec2, Vec2)> {
    let (min, max) = (vec3(&model["min"])?, vec3(&model["max"])?);
    let mut lo = Vec2::splat(f32::MAX);
    let mut hi = Vec2::splat(f32::MIN);
    for i in 0..8 {
        let corner = Vec3::new(
            if i & 1 == 0 { min.x } else { max.x },
            if i & 2 == 0 { min.y } else { max.y },
            if i & 4 == 0 { min.z } else { max.z },
        );
        let w = p.translation + p.rotation * (p.scale * corner);
        lo = lo.min(Vec2::new(w.x, w.z));
        hi = hi.max(Vec2::new(w.x, w.z));
    }
    Some((lo, hi))
}

#[derive(Resource)]
struct Capture {
    path: PathBuf,
    at: f32,
    started: Instant,
    requested: Option<Instant>,
    ready_frames: u32,
}

struct Pose {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
}

fn vec3(v: &Value) -> Option<Vec3> {
    Some(Vec3::new(
        v.get(0)?.as_f64()? as f32,
        v.get(1)?.as_f64()? as f32,
        v.get(2)?.as_f64()? as f32,
    ))
}

fn pose(row: &Value) -> Option<Pose> {
    let r = &row["rotation"];
    Some(Pose {
        translation: vec3(&row["translation"])?,
        rotation: Quat::from_xyzw(
            r.get(0)?.as_f64()? as f32,
            r.get(1)?.as_f64()? as f32,
            r.get(2)?.as_f64()? as f32,
            r.get(3)?.as_f64()? as f32,
        ),
        scale: vec3(&row["scale"]).unwrap_or(Vec3::ONE),
    })
}

#[derive(Resource)]
struct Layout {
    json: Value,
    collision: Option<sekiro_formats::hknp::CollisionMesh>,
    scene: MapScene,
}

/// How a map is shown.
#[derive(Clone, Debug)]
pub struct MapScene {
    /// Asset path of the map folder below the asset root, with a trailing slash ("" when the
    /// asset root is the map folder).
    pub prefix: String,
    /// Start with the collision wireframe visible.
    pub collision: bool,
    /// Hide the map pieces (collision only).
    pub no_pieces: bool,
    /// Skip models whose name starts with any of these.
    pub hide: Vec<String>,
    /// Also spawn MSB objects.
    pub objects: bool,
    pub groups: GroupMode,
    /// Enemy and player start markers.
    pub markers: bool,
    /// Time of day in hours for the draw params (light sets are keyed by time).
    pub hour: f32,
    /// Draw param file stem (`param/drawparam/<stem>.gparam`); default `<mXX>_<YY>_0000`.
    pub gparam: Option<String>,
    /// Arena collision and object radius. Map pieces use ARENA_VISUAL_RADIUS unless
    /// --no-far-scenery is set; `None` loads the whole map.
    pub within: Option<(Vec3, f32)>,
}

/// Default draw param of a map: `m11_00_00_00` -> `m11_00_0000`.
#[cfg(test)]
fn default_gparam(id: &str) -> String {
    format!("{}_0000", id.get(..6).unwrap_or(id))
}

/// Cinematic layouts use the full map suffix in an `s` draw parameter stem.
fn cinematic_gparam(id: &str) -> Option<String> {
    let fields: Vec<_> = id.strip_prefix('m')?.split('_').collect();
    if fields.len() != 4
        || fields
            .iter()
            .any(|f| f.len() != 2 || !f.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    Some(format!(
        "s{}_{}_{}{}",
        fields[0], fields[1], fields[2], fields[3]
    ))
}

/// The entity whose position picks the draw groups (the free camera, or the player).
#[derive(Component)]
pub struct MapFocus;

/// `cache/maps/<id>`, or an error naming the export command.
pub fn map_dir(cache: &Path, id: &str) -> Result<PathBuf> {
    cache
        .join("maps")
        .join(id)
        .canonicalize()
        .with_context(|| format!("cache/maps/{id} not found: run `sekiro-extract map {id}`"))
}

/// Bevy's IO threads overflow small stacks while loading a map's GLBs and DDS textures.
/// The bounded Ashina neighbourhood also overflows 16 MiB; 64 MiB is verified with its warm
/// asset set. Threads read this when spawned, so call
/// it before the `App` is built.
pub fn raise_io_stack() {
    if std::env::var_os("RUST_MIN_STACK").is_none() {
        // SAFETY: no other thread exists yet, so nothing reads the environment concurrently.
        unsafe { std::env::set_var("RUST_MIN_STACK", (64usize << 20).to_string()) };
    }
}

/// The player start `index` from `layout.json`: position and Bevy-space yaw in degrees.
pub fn player_start(dir: &Path, index: usize) -> Result<(Vec3, f32)> {
    let json: Value = serde_json::from_slice(
        &std::fs::read(dir.join("layout.json")).context("reading layout.json")?,
    )?;
    let p = json["players"]
        .get(index)
        .with_context(|| format!("no player start {index}"))?;
    let pos = vec3(&p["translation"]).context("player start without translation")?;
    Ok((pos, p["yaw_degrees"].as_f64().unwrap_or(0.0) as f32))
}

/// Adds a map to an app built with `DefaultPlugins`: its pieces, draw-group culling around the
/// [`MapFocus`] entity, a sun, ambient light, sky colour, fog and far plane on every 3D camera,
/// and the collision view (C toggles it, G cycles group modes, M toggles markers).
pub fn add_map(app: &mut App, dir: &Path, scene: MapScene) -> Result<()> {
    let json: Value = serde_json::from_slice(
        &std::fs::read(dir.join("layout.json")).context("reading layout.json")?,
    )?;
    let collision = match std::fs::read(dir.join("collision.bin")) {
        Ok(bytes) => Some(sekiro_formats::hknp::CollisionMesh::from_bytes(&bytes)?),
        Err(_) => None,
    };
    let mut group_parts = Vec::new();
    if let Some(c) = &collision {
        for row in json["collisions"].as_array().into_iter().flatten() {
            let Some([first, count]) = row["triangles"]
                .as_array()
                .and_then(|r| Some([r.first()?.as_u64()? as usize, r.get(1)?.as_u64()? as usize]))
            else {
                continue;
            };
            let mut min = Vec2::splat(f32::MAX);
            let mut max = Vec2::splat(f32::MIN);
            for &i in &c.indices[first * 3..(first + count) * 3] {
                let v = c.vertices[i as usize];
                min = min.min(Vec2::new(v[0], v[2]));
                max = max.max(Vec2::new(v[0], v[2]));
            }
            group_parts.push(CollisionPart {
                name: row["name"].as_str().unwrap_or_default().to_string(),
                first,
                count,
                min,
                max,
                display: groups8(&row["display_groups"]),
                draw: groups8(&row["draw_groups"]),
                light_set: row["light_set"].as_i64().unwrap_or(-1) as i32,
                fog_set: row["fog_set"].as_i64().unwrap_or(-1) as i32,
                scattering_set: row["scattering_set"].as_i64().unwrap_or(-1) as i32,
                env_map: row["env_map"].as_i64().unwrap_or(-1) as i32,
            });
        }
    }
    let mode = scene.groups;
    let id = dir
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or_default()
        .to_string();
    // `dir` is canonical (verbatim on Windows), so walk up instead of joining "..".
    let cache = dir
        .parent()
        .and_then(Path::parent)
        .context("map folder has no cache above it")?;
    let connected = world::enabled(&id)
        && scene.within.is_none()
        && !app.world().contains_resource::<world::ArenaMap>()
        && !std::env::args().any(|a| a == "--arena" || a == "--no-connected-blocks");
    if connected {
        let world = world::MapWorld::load(cache, &id, &scene.prefix)?;
        let mut story_maps = story::MapStory::default();
        for (block, layout) in &world.blocks {
            if let Some(state) = story::load(cache, block, &layout.json)? {
                info!(
                    "{block}: offline story, {} parts, {} unsupported predicates",
                    state.parts.len(),
                    state.state.unsupported.len()
                );
                story_maps.maps.insert(block.clone(), state);
            }
        }
        app.insert_resource(story_maps);
        app.add_plugins(visibility::MapVisibilityPlugin)
            .insert_resource(world)
            .add_systems(
                Update,
                (world::update, world::update_detail)
                    .chain()
                    .after(track_loading)
                    .after(apply_groups),
            )
            .add_systems(
                PostUpdate,
                world::publish.in_set(visibility::VisibilityInputs),
            )
            .add_systems(
                PostUpdate,
                world::debug_hierarchy
                    .after(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate)
                    .after(bevy::transform::TransformSystems::Propagate),
            );
    }
    // Older extractor output lacks FogParamID; read the installed-data copy without rewriting it.
    if group_parts
        .iter()
        .any(|p| p.fog_set < 0 || p.scattering_set < 0 || p.env_map < 0)
        && let Ok(bytes) = std::fs::read(cache.join(format!("raw/map/mapstudio/{id}.msb")))
        && let Ok(msb) = sekiro_formats::msb::parse(&bytes)
    {
        for part in &mut group_parts {
            if let Some(g) = msb
                .parts
                .iter()
                .find(|p| p.name == part.name)
                .and_then(|p| p.gparam)
            {
                part.light_set = g[0];
                part.fog_set = g[1];
                part.scattering_set = g[2];
                part.env_map = g[3];
            }
        }
    }
    app.init_resource::<crate::lighting::fog::FogSelection>();
    crate::map_fx::install(app, dir)?;
    app.init_resource::<crate::lighting_selection::RegionSelection>();
    let args: Vec<_> = std::env::args().collect();
    let arena_name = crate::arg_value(&args, "--arena");
    let arena = arena_name
        .as_deref()
        .and_then(|name| sekiro_sim::arena::ArenaSpec::by_name(name).ok());
    let story = story::selection(&args);
    let mut selection = crate::lighting_selection::select(cache, &id, story.as_deref(), arena)?;
    if scene.hour.is_finite() {
        let hour_changed = scene.hour != selection.hour;
        selection.hour = scene.hour;
        if !selection.fixed_environment {
            selection.environment = nearest_variant(scene.hour);
            // An explicit clock override chooses its matching ordinary sky.
            if hour_changed || args.iter().any(|a| a == "--hour") {
                selection.sky = None;
            }
        }
    }
    let hour = selection.hour;
    info!(
        "map story lighting: {id}, hour={hour}, environment={}, sub-ID={}, source={}, flags={:?}, sky={:?}",
        selection.environment, selection.sub_id, selection.source, selection.flags, selection.sky
    );
    let selected_source = selection.source.clone();
    let stem = scene.gparam.clone().unwrap_or_else(|| {
        let standard = selected_source;
        if cache
            .join(format!("raw/param/drawparam/{standard}.gparam"))
            .is_file()
        {
            standard
        } else {
            cinematic_gparam(&id)
                .filter(|s| {
                    cache
                        .join(format!("raw/param/drawparam/{s}.gparam"))
                        .is_file()
                })
                .unwrap_or(standard)
        }
    });
    let gparam_path = cache.join(format!("raw/param/drawparam/{stem}.gparam"));
    let gparam = sekiro_formats::gparam::parse(
        &std::fs::read(&gparam_path)
            .with_context(|| format!("reading {}", gparam_path.display()))?,
    )?;
    let selection = crate::lighting_selection::LightingSelection::load(
        cache,
        &id,
        selection,
        scene.gparam.is_some(),
    )?;
    let selected_environment = selection.initial.environment;
    app.insert_resource(selection);
    crate::lighting_selection::MapStoryEvents::install(app, cache, &id, story.as_deref())?;
    app.add_systems(PostUpdate, crate::lighting_selection::sync_events);
    crate::lighting::install_post_effects(app, dir)?;
    sky::install(app, cache, &id, hour)?;
    let mut probes = MapProbes {
        prefix: scene.prefix.clone(),
        variant: selected_environment,
        ..default()
    };
    // This flag is shared by viewer and play without changing the play-mode options.
    if !std::env::args().any(|a| a == "--no-env-probe")
        && let Ok(bytes) = std::fs::read(dir.join("env.json"))
    {
        let env: Value = serde_json::from_slice(&bytes).context("reading env.json")?;
        for r in env["regions"].as_array().into_iter().flatten() {
            let probe = r["probe"].as_u64().context("probe index")? as u32;
            let vector = |key: &str| -> Result<Vec3> {
                let values: [f32; 3] = serde_json::from_value(r[key].clone())?;
                Ok(Vec3::from_array(values))
            };
            let rotation: [f32; 4] = serde_json::from_value(r["rotation"].clone())?;
            probes.regions.push(ProbeRegion {
                probe,
                position: vector("position")?,
                rotation: Quat::from_array(rotation),
                size: if r["size"].is_null() {
                    None
                } else {
                    Some(vector("size")?)
                },
            });
        }
        let exists = |probe: u32, variant: u32| {
            dir.join(format!("env/gilm{probe}_{variant:02}.dds"))
                .exists()
                && dir
                    .join(format!("env/gilm{probe}_{variant:02}_diffuse.dds"))
                    .exists()
        };
        if !probes
            .regions
            .iter()
            .any(|r| exists(r.probe, probes.variant))
        {
            warn!(
                "{id}: authored environment variant {} has no complete exported probes; no nearest-time substitution",
                probes.variant
            );
        }
        for r in &probes.regions {
            let probe = r.probe;
            if dir
                .join(format!("env/gilm{probe}_{:02}.dds", probes.variant))
                .exists()
                && dir
                    .join(format!("env/gilm{probe}_{:02}_diffuse.dds", probes.variant))
                    .exists()
            {
                probes.available.insert(probe);
            }
        }
    }
    crate::lighting::install_irradiance(app, dir, &probes)?;
    app.insert_resource(probes);
    let args: Vec<_> = std::env::args().collect();
    if args
        .iter()
        .any(|a| a == "--visual-stats" || a == "--profile-systems")
    {
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        }
        if !app.is_plugin_added::<bevy::render::diagnostic::RenderDiagnosticsPlugin>() {
            app.add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin);
        }
        app.add_systems(Update, visual_stats);
    }
    if args.iter().any(|a| a == "--profile-systems") {
        app.add_plugins((
            bevy::diagnostic::EntityCountDiagnosticsPlugin::default(),
            bevy::diagnostic::SystemInformationDiagnosticsPlugin,
        ));
        crate::perf_log::install_render_probe(app);
    }
    map_material::install(app, &scene.prefix, dir, &args);
    crate::map_fog_surface::install(app, &scene.prefix);
    crate::map_grass::install(app, dir, &scene.prefix)?;
    app.insert_resource(ShadowSettings::from_args(&gparam, hour, &args)?);
    LocalLights::install(app, dir, &args)?;
    app.add_plugins(WireframePlugin::default())
        .insert_resource(ClearColor(SKY))
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.80, 0.85, 1.0),
            brightness: crate::lighting::ENV_PER_UNIT,
            ..default()
        })
        .insert_resource(Layout {
            json,
            collision,
            scene,
        })
        .insert_resource(Groups {
            mode,
            parts: group_parts,
            current: None,
            active: None,
            checked_at: None,
            dirty: true,
        })
        .insert_resource(MapLighting {
            gparam,
            hour,
            light_set: 0,
            applied: None,
            source: stem,
        })
        .init_resource::<Pending>()
        .add_observer(map_scene_ready)
        .add_systems(Startup, spawn_map)
        .add_systems(
            Update,
            (
                camera_environment,
                toggle_collision,
                track_loading,
                budget_map_shadows,
                update_local_lights.after(world::update),
                (pick_groups, apply_groups, apply_lighting, apply_probes).chain(),
            ),
        );
    Ok(())
}

/// A far plane for every new 3D camera; lighting installs shader fog separately.
fn camera_environment(mut commands: Commands, cameras: Query<Entity, Added<Camera3d>>) {
    for e in &cameras {
        commands
            .entity(e)
            .insert(Projection::Perspective(PerspectiveProjection {
                fov: 60f32.to_radians(),
                near: 0.1,
                far: 20_000.0,
                ..default()
            }));
    }
}

/// Keep a player's initial inspection camera on the near side of surrounding collision.
fn viewer_eye(pos: Vec3, eye: Vec3, collision: &sekiro_formats::hknp::CollisionMesh) -> Vec3 {
    let focus = pos + Vec3::Y * 1.4;
    let delta = eye - focus;
    let distance = delta.length();
    if distance <= f32::EPSILON {
        return eye;
    }
    let direction = delta / distance;
    let nearby = sekiro_sim::arena::crop(collision, focus, distance + 1.0);
    let world = sekiro_sim::collision::CollisionWorld::from_mesh(&nearby);
    match world.raycast(focus, direction, distance) {
        Some(hit) => focus + direction * (hit - 0.2).max(0.0),
        None => eye,
    }
}

pub fn run(options: Options) -> Result<()> {
    let dir = map_dir(&options.cache, &options.id)?;

    // Start pose: --camera, else over the shoulder of a player start point.
    let (mut start, yaw, pitch) = match options.camera {
        Some([x, y, z, yaw, pitch]) => (Vec3::new(x, y, z), yaw, pitch),
        None => {
            let (pos, yaw) = player_start(&dir, options.start).unwrap_or((Vec3::ZERO, 0.0));
            // 3.5 m behind and 2.2 m above the start point, so its marker shows. Forward at
            // yaw 0 is -Z.
            let back = Quat::from_rotation_y(yaw.to_radians()) * Vec3::Z * 3.5;
            (pos + back + Vec3::new(0.0, 2.2, 0.0), yaw, -12.0)
        }
    };

    raise_io_stack();
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(layered_gltf_plugin())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: format!("sekiro-rs map - {}", options.id),
                    resolution: (1600, 900).into(),
                    present_mode: crate::presentation::mode(
                        options.uncapped,
                        bevy::window::PresentMode::AutoVsync,
                    ),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: if world::enabled(&options.id) {
                    options.cache.to_string_lossy().into_owned()
                } else {
                    dir.to_string_lossy().into_owned()
                },
                ..default()
            }),
        FrameTimeDiagnosticsPlugin::default(),
        bevy::render::diagnostic::RenderDiagnosticsPlugin,
    ))
    // Keep rendering at full rate when the window is not focused, so screenshots and
    // `--uncapped` measurements from a terminal are not throttled to 60 Hz.
    .insert_resource(bevy::winit::WinitSettings::continuous());
    add_map(
        &mut app,
        &dir,
        MapScene {
            prefix: if world::enabled(&options.id) {
                format!("maps/{}/", options.id)
            } else {
                String::new()
            },
            collision: options.collision,
            no_pieces: options.no_pieces,
            hide: options.hide,
            objects: options.objects || world::enabled(&options.id),
            groups: options.groups,
            markers: true,
            hour: options.hour,
            gparam: options.gparam,
            within: None,
        },
    )?;
    if options.camera.is_none()
        && let Ok((pos, _)) = player_start(&dir, options.start)
        && let Some(collision) = &app.world().resource::<Layout>().collision
    {
        start = viewer_eye(pos, start, collision);
    }
    app.insert_resource(Fly {
        yaw: yaw.to_radians(),
        pitch: pitch.to_radians(),
        speed: 12.0,
    })
    .add_systems(Startup, move |commands: Commands| {
        setup_view(commands, start)
    })
    .add_systems(Update, (fly, apply_fly, hud).chain());
    if let Some(path) = options.screenshot {
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        app.insert_resource(Capture {
            path,
            at: options.at,
            started: Instant::now(),
            requested: None,
            ready_frames: 0,
        })
        .add_systems(Last, capture);
    }
    ready::install(&mut app);
    app.add_plugins(crate::presentation::PresentationPlugin);
    app.run();
    Ok(())
}

const SKY: Color = Color::srgb(0.62, 0.68, 0.74);

pub(crate) fn layered_gltf_plugin() -> bevy::gltf::GltfPlugin {
    bevy::gltf::GltfPlugin::default()
        // gltf-json strips the leading underscore before Bevy looks up custom semantics.
        .add_custom_vertex_attribute("MAP_BLEND2", ATTR_MAP_BLEND2)
        .add_custom_vertex_attribute(
            "MAP_COLOR1",
            bevy::mesh::MeshVertexAttribute::new(
                "MapColor1",
                0x5ec1_0a03,
                bevy::render::render_resource::VertexFormat::Float32x4,
            ),
        )
}

fn setup_view(mut commands: Commands, start: Vec3) {
    commands.spawn((
        MapFocus,
        Camera3d::default(),
        bevy::core_pipeline::prepass::DepthPrepass,
        bevy::core_pipeline::prepass::MotionVectorPrepass,
        Transform::from_translation(start),
    ));
    commands.spawn((
        Hud,
        Text::new(""),
        TextFont {
            font_size: bevy::text::FontSize::Px(15.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: px(8),
            top: px(6),
            ..default()
        },
    ));
}

#[allow(clippy::too_many_arguments)]
fn spawn_map(
    arena_parts: Option<Res<crate::arena_b::Parts>>,
    mut commands: Commands,
    assets: Res<AssetServer>,
    layout: Res<Layout>,
    mut pending: ResMut<Pending>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    shadow: Res<ShadowSettings>,
    connected: Option<ResMut<world::MapWorld>>,
) {
    spawn_lights(&mut commands, &shadow);

    let scene = &layout.scene;
    let far_enabled = !std::env::args().any(|a| a == "--no-far-scenery");
    if let Some(mut connected) = connected {
        world::spawn(&mut commands, &assets, &mut connected, scene, &mut pending);
    } else if !scene.no_pieces {
        let models = &layout.json["models"];
        let mut spawned = 0;
        let mut seen = std::collections::HashSet::new();
        let kinds: &[&str] = if scene.objects {
            &["pieces", "objects"]
        } else {
            &["pieces"]
        };
        let rows = kinds
            .iter()
            .flat_map(|k| layout.json[*k].as_array().into_iter().flatten());
        for row in rows {
            let Some(model) = row["model"].as_str() else {
                continue;
            };
            if scene.hide.iter().any(|h| model.starts_with(h.as_str())) {
                continue;
            }
            if scene
                .hide
                .iter()
                .any(|h| row["name"].as_str() == Some(h.as_str()))
            {
                continue;
            }
            let Some(glb) = models[model]["glb"].as_str() else {
                continue;
            };
            let glb = format!("{}{glb}", scene.prefix);
            let Some(p) = pose(row) else { continue };
            if let Some((center, collision_radius)) = scene.within {
                let radius = if model.starts_with('m') && far_enabled {
                    sekiro_sim::arena::ARENA_VISUAL_RADIUS
                } else {
                    collision_radius
                };
                let inside = placed_box(&models[model], &p).is_some_and(|(lo, hi)| {
                    sekiro_sim::arena::box_within(lo, hi, center.xz(), radius)
                });
                if !inside {
                    continue;
                }
            }
            if seen.insert(glb.clone()) {
                pending
                    .handles
                    .push(assets.load::<Gltf>(glb.clone()).untyped());
            }
            let far_scenery = scene.within.is_some_and(|(center, radius)| {
                placed_box(&models[model], &p).is_some_and(|(lo, hi)| {
                    !sekiro_sim::arena::box_within(lo, hi, center.xz(), radius)
                })
            });
            let mut entity = commands.spawn((
                DrawGroups(groups8(&row["draw_groups"])),
                WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(glb))),
                Transform {
                    translation: p.translation,
                    rotation: p.rotation,
                    scale: p.scale,
                },
            ));
            if far_scenery {
                entity.insert(FarScenery);
            }
            if let Some(part) = arena_parts
                .as_ref()
                .and_then(|p| p.0.get(row["name"].as_str().unwrap_or_default()))
            {
                entity.insert(part.clone());
            }
            spawned += 1;
        }
        pending.instances = spawned;
        info!("spawned {spawned} map piece and object instances");
    }

    // Enemy (red, dummy enemies orange) and player start (blue) markers, 1.8 m tall, facing
    // their MSB direction (the nose points along the character's -Z forward).
    let body = meshes.add(Capsule3d::new(0.3, 1.2));
    let nose = meshes.add(Cuboid::new(0.12, 0.12, 0.5));
    let mut marker_material = |color: Color| {
        materials.add(StandardMaterial {
            base_color: color,
            emissive: color.to_linear() * 0.3,
            ..default()
        })
    };
    let enemy = marker_material(Color::srgb(0.85, 0.12, 0.10));
    let dummy = marker_material(Color::srgb(0.95, 0.55, 0.10));
    let player = marker_material(Color::srgb(0.15, 0.35, 0.95));
    let marker_sets: &[(&str, &Handle<StandardMaterial>)] = if scene.markers {
        &[("enemies", &enemy), ("players", &player)]
    } else {
        &[]
    };
    for &(key, default_material) in marker_sets {
        for row in layout.json[key].as_array().into_iter().flatten() {
            let Some(p) = pose(row) else { continue };
            let material = if row["dummy"].as_bool() == Some(true) {
                dummy.clone()
            } else {
                default_material.clone()
            };
            commands
                .spawn((
                    Marker,
                    Transform::from_translation(p.translation).with_rotation(p.rotation),
                    Visibility::Visible,
                ))
                .with_children(|c| {
                    c.spawn((
                        Mesh3d(body.clone()),
                        MeshMaterial3d(material.clone()),
                        Transform::from_xyz(0.0, 0.9, 0.0),
                    ));
                    c.spawn((
                        Mesh3d(nose.clone()),
                        MeshMaterial3d(material),
                        Transform::from_xyz(0.0, 1.5, -0.35),
                    ));
                });
        }
    }

    let cropped = match (&layout.collision, scene.within) {
        (Some(c), Some((center, radius))) => Some(sekiro_sim::arena::crop(c, center, radius)),
        _ => None,
    };
    if let Some(c) = cropped.as_ref().or(layout.collision.as_ref()) {
        let visibility = if scene.collision {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, c.vertices.clone());
        mesh.insert_indices(Indices::U32(c.indices.clone()));
        commands.spawn((
            CollisionView,
            Mesh3d(meshes.add(mesh)),
            Wireframe,
            WireframeColor {
                color: Color::srgb(1.0, 0.25, 0.1),
            },
            NoFrustumCulling,
            visibility,
        ));
        if scene.no_pieces {
            // Without map pieces, also draw the collision solid, flat shaded and coloured by
            // hit material, so its shape reads clearly.
            commands.spawn((
                CollisionView,
                Mesh3d(meshes.add(solid_collision(c))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    perceptual_roughness: 0.9,
                    ..default()
                })),
                NoFrustumCulling,
                visibility,
            ));
        }
        info!("collision: {} triangles", c.indices.len() / 3);
    }
}

/// Opt-in reproducible render measurements, including arena sessions.
fn visual_stats(
    pending: Res<Pending>,
    diagnostics: Res<DiagnosticsStore>,
    probe: Option<Res<crate::perf_log::RenderProbe>>,
    mut last: Local<u32>,
) {
    let seconds = pending.loaded_for() as u32;
    if seconds < 2 || seconds == *last {
        return;
    }
    *last = seconds;
    let frame = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.);
    info!("visual stats: {seconds} s after loading, frame {frame:.3} ms");
    if let Some(ref probe) = probe {
        let (max, sum, frames, ready, waiting, sets) = probe.take();
        info!(
            "profile render: mean {:.3} ms, max {max:.3} ms, pipelines {ready} ready / {waiting} waiting, sets {sets}",
            sum / frames.max(1) as f64
        );
        crate::perf_log::report_systems();
    }
    for d in diagnostics.iter().filter(|d| {
        let path = d.path().as_str();
        if probe.is_some() {
            path.ends_with("elapsed_gpu")
                || path.ends_with("elapsed_cpu")
                || path.contains("cpu_usage")
                || path == "entity_count"
        } else {
            (path.contains("game_fog") || path.contains("native_post"))
                && path.ends_with("elapsed_gpu")
        }
    }) {
        if let Some(ms) = d.smoothed() {
            info!("visual stats: {} {ms:.3} {}", d.path(), d.suffix);
        }
    }
}

/// Flat-shaded triangle soup with one colour per hit material id.
fn solid_collision(c: &sekiro_formats::hknp::CollisionMesh) -> Mesh {
    let tris = c.indices.len() / 3;
    let mut positions = Vec::with_capacity(tris * 3);
    let mut normals = Vec::with_capacity(tris * 3);
    let mut colors = Vec::with_capacity(tris * 3);
    for (t, tri) in c.indices.as_chunks::<3>().0.iter().enumerate() {
        let p = tri.map(|i| Vec3::from(c.vertices[i as usize]));
        let n = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or(Vec3::Y);
        let m = c.materials.get(t).copied().unwrap_or(u32::MAX);
        // A stable pastel colour per material id.
        let h = m.wrapping_mul(2_654_435_761);
        let color = Color::hsl((h % 360) as f32, 0.45, 0.6)
            .to_linear()
            .to_f32_array();
        for v in p {
            positions.push(v.to_array());
            normals.push(n.to_array());
            colors.push(color);
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
}

fn fly(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    time: Res<Time>,
    mut fly: ResMut<Fly>,
    mut camera: Query<&mut Transform, With<Camera3d>>,
) {
    if buttons.pressed(MouseButton::Right) {
        fly.yaw -= motion.delta.x * 0.003;
        fly.pitch = (fly.pitch - motion.delta.y * 0.003).clamp(-1.55, 1.55);
    }
    if scroll.delta.y != 0.0 {
        fly.speed = (fly.speed * (1.0 + scroll.delta.y * 0.15)).clamp(0.5, 2000.0);
    }
    let Ok(mut t) = camera.single_mut() else {
        return;
    };
    let rotation = Quat::from_euler(EulerRot::YXZ, fly.yaw, fly.pitch, 0.0);
    let forward = rotation * Vec3::NEG_Z;
    let right = rotation * Vec3::X;
    let mut dir = Vec3::ZERO;
    for (key, d) in [
        (KeyCode::KeyW, forward),
        (KeyCode::KeyS, -forward),
        (KeyCode::KeyD, right),
        (KeyCode::KeyA, -right),
        (KeyCode::Space, Vec3::Y),
        (KeyCode::KeyE, Vec3::Y),
        (KeyCode::KeyQ, -Vec3::Y),
        (KeyCode::ControlLeft, -Vec3::Y),
    ] {
        if keys.pressed(key) {
            dir += d;
        }
    }
    let boost = if keys.pressed(KeyCode::ShiftLeft) {
        6.0
    } else {
        1.0
    };
    t.translation += dir.normalize_or_zero() * fly.speed * boost * time.delta_secs();
    if keys.just_pressed(KeyCode::KeyP) {
        println!(
            "--camera {:.2},{:.2},{:.2},{:.1},{:.1}",
            t.translation.x,
            t.translation.y,
            t.translation.z,
            fly.yaw.to_degrees(),
            fly.pitch.to_degrees()
        );
    }
}

fn apply_fly(fly: Res<Fly>, mut camera: Query<&mut Transform, With<Camera3d>>) {
    for mut t in &mut camera {
        t.rotation = Quat::from_euler(EulerRot::YXZ, fly.yaw, fly.pitch, 0.0);
    }
}

/// Finds the collision part under the camera (the nearest surface below, else the nearest
/// above) and takes its groups and lighting configuration whenever the focus moves.
#[allow(clippy::too_many_arguments)]
fn pick_groups(
    keys: Res<ButtonInput<KeyCode>>,
    layout: Res<Layout>,
    mut groups: ResMut<Groups>,
    mut lighting: ResMut<MapLighting>,
    mut fog: ResMut<crate::lighting::fog::FogSelection>,
    mut region: ResMut<crate::lighting_selection::RegionSelection>,
    focus: Query<&Transform, With<MapFocus>>,
    connected: Option<Res<world::MapWorld>>,
) {
    let _timer = crate::perf_log::SystemTimer::start("pick_groups");
    if keys.just_pressed(KeyCode::KeyG) {
        groups.mode = groups.mode.next();
        groups.dirty = true;
        groups.checked_at = None;
    }
    if let Some(world) = connected {
        if let Some((block, collision)) = &world.selected
            && let Some(row) = world.blocks[block].json["parts"]
                .as_array()
                .and_then(|parts| parts.iter().find(|p| p["name"].as_str() == Some(collision)))
            && let Some(config) = row["gparam"].as_array()
        {
            let config =
                std::array::from_fn(|i| config.get(i).and_then(Value::as_i64).unwrap_or(-1) as i32);
            region.enter(config);
            if config[0] >= 0 {
                lighting.light_set = config[0];
            }
            if config[1] >= 0 {
                fog.0 = config[1];
            }
            groups.current = (block == &world.primary)
                .then(|| groups.parts.iter().position(|p| &p.name == collision))
                .flatten();
        }
        return;
    }
    let (Some(t), Some(c)) = (focus.iter().next(), &layout.collision) else {
        return;
    };
    let pos = t.translation;
    if groups.checked_at == Some(pos) {
        return;
    }
    groups.checked_at = Some(pos);
    let p2 = Vec2::new(pos.x, pos.z);
    let mut best: Option<(f32, usize)> = None;
    for (pi, part) in groups.parts.iter().enumerate() {
        if p2.cmplt(part.min).any() || p2.cmpgt(part.max).any() {
            continue;
        }
        let tris = &c.indices[part.first * 3..(part.first + part.count) * 3];
        for tri in tris.as_chunks::<3>().0 {
            let [a, b, cc] = tri.map(|i| Vec3::from(c.vertices[i as usize]));
            if let Some(h) = vertical_hit(pos, a, b, cc) {
                // Prefer the nearest surface below; surfaces above count only if none is below.
                let score = if h >= -0.5 { h } else { 1000.0 - h };
                if best.is_none_or(|(s, _)| score < s) {
                    best = Some((score, pi));
                }
            }
        }
    }
    let Some((_, pi)) = best else { return };
    let part = &groups.parts[pi];
    let g = match groups.mode {
        GroupMode::Display => part.display,
        _ => part.draw,
    };
    let light_set = part.light_set;
    region.enter([light_set, part.fog_set, part.scattering_set, part.env_map]);
    if part.fog_set >= 0 {
        fog.0 = part.fog_set;
    }
    groups.current = Some(pi);
    if light_set >= 0 && lighting.light_set != light_set {
        lighting.light_set = light_set;
    }
    // A collision with no groups leaves the previous selection in force.
    if g.iter().any(|&w| w != 0) && groups.active != Some(g) {
        groups.active = Some(g);
        groups.dirty = true;
    }
}

/// Height of `p` above the triangle (`p.y - surface y`) where the vertical line through `p`
/// crosses it.
fn vertical_hit(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let (x, z) = (p.x, p.z);
    let d = (b.z - c.z) * (a.x - c.x) + (c.x - b.x) * (a.z - c.z);
    if d.abs() < 1e-9 {
        return None;
    }
    let l1 = ((b.z - c.z) * (x - c.x) + (c.x - b.x) * (z - c.z)) / d;
    let l2 = ((c.z - a.z) * (x - c.x) + (a.x - c.x) * (z - c.z)) / d;
    let l3 = 1.0 - l1 - l2;
    if l1 < 0.0 || l2 < 0.0 || l3 < 0.0 {
        return None;
    }
    Some(p.y - (l1 * a.y + l2 * b.y + l3 * c.y))
}

fn apply_groups(
    mut groups: ResMut<Groups>,
    mut pieces: Query<(&DrawGroups, &mut Visibility), LegacyPieceFilter>,
    added: Query<(), Added<DrawGroups>>,
) {
    let _timer = crate::perf_log::SystemTimer::start("apply_groups");
    if !groups.dirty && added.is_empty() {
        return;
    }
    groups.dirty = false;
    let active = match groups.mode {
        GroupMode::Off => None,
        _ => groups.active,
    };
    for (g, mut v) in &mut pieces {
        let show = active.is_none_or(|a| a.iter().zip(g.0).any(|(x, y)| x & y != 0));
        let want = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *v != want {
            *v = want;
        }
    }
}

/// Only map descendants are budgeted; character meshes always retain their shadows.
/// Use the world-space bounds, not the instance origin, to retain large nearby walls.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn budget_map_shadows(
    mut commands: Commands,
    time: Res<Time>,
    settings: Res<ShadowSettings>,
    focus: Query<&GlobalTransform, With<MapFocus>>,
    meshes: Query<(Entity, &GlobalTransform, &Aabb, Option<&NotShadowCaster>), With<Mesh3d>>,
    parents: Query<&ChildOf>,
    roots: Query<(), With<DrawGroups>>,
    mut last: Local<f32>,
    mut counts: Local<Option<(usize, usize)>>,
) {
    let _timer = crate::perf_log::SystemTimer::start("budget_map_shadows");
    if time.elapsed_secs() - *last < 0.5 {
        return;
    }
    *last = time.elapsed_secs();
    let Some(focus) = focus.iter().next() else {
        return;
    };
    let position = focus.translation();
    let mut total = 0;
    let mut casting = 0;
    for (entity, transform, bounds, excluded) in &meshes {
        let mut ancestor = entity;
        while !roots.contains(ancestor) {
            let Ok(parent) = parents.get(ancestor) else {
                break;
            };
            ancestor = parent.parent();
        }
        if !roots.contains(ancestor) {
            continue;
        }
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = Vec3::splat(f32::NEG_INFINITY);
        for x in [-1., 1.] {
            for y in [-1., 1.] {
                for z in [-1., 1.] {
                    let corner = Vec3::from(bounds.center)
                        + Vec3::from(bounds.half_extents) * Vec3::new(x, y, z);
                    let corner = transform.transform_point(corner);
                    lo = lo.min(corner);
                    hi = hi.max(corner);
                }
            }
        }
        let cast = settings.enabled
            && position.distance_squared(position.clamp(lo, hi))
                <= settings.caster_distance.powi(2);
        total += 1;
        casting += usize::from(cast);
        if cast && excluded.is_some() {
            commands.entity(entity).remove::<NotShadowCaster>();
        }
        if !cast && excluded.is_none() {
            commands.entity(entity).insert(NotShadowCaster);
        }
    }
    if *counts != Some((casting, total)) {
        info!(
            "map shadow casters: {casting}/{total} meshes within {} m of focus",
            settings.caster_distance
        );
        *counts = Some((casting, total));
    }
}

fn toggle_collision(
    keys: Res<ButtonInput<KeyCode>>,
    mut collision: Query<&mut Visibility, (With<CollisionView>, Without<Marker>)>,
    mut markers: Query<&mut Visibility, (With<Marker>, Without<CollisionView>)>,
) {
    let flip = |v: &mut Visibility| {
        *v = match *v {
            Visibility::Hidden => Visibility::Visible,
            _ => Visibility::Hidden,
        };
    };
    if keys.just_pressed(KeyCode::KeyC) {
        collision.iter_mut().for_each(|mut v| flip(&mut v));
    }
    if keys.just_pressed(KeyCode::KeyM) {
        markers.iter_mut().for_each(|mut v| flip(&mut v));
    }
}

fn hud(
    diagnostics: Res<DiagnosticsStore>,
    groups: Res<Groups>,
    fly: Res<Fly>,
    pending: Res<Pending>,
    camera: Query<&Transform, With<Camera3d>>,
    meshes: Query<(), With<Mesh3d>>,
    mut text: Query<&mut Text, With<Hud>>,
) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    let Ok(t) = camera.single() else { return };
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    text.0 = format!(
        "{fps:.0} fps | {} mesh entities | pos {:.1} {:.1} {:.1} yaw {:.0} pitch {:.0} | speed {:.0}{}\n\
         groups {:?} from {} | RMB look, WASD fly, Space/Q up/down, Shift fast, wheel speed, \
         C collision, G groups, M markers, P print pose",
        meshes.iter().count(),
        t.translation.x,
        t.translation.y,
        t.translation.z,
        fly.yaw.to_degrees(),
        fly.pitch.to_degrees(),
        fly.speed,
        if pending.loaded_at.is_none() {
            " | loading"
        } else {
            ""
        },
        groups.mode,
        groups
            .current
            .map(|i| groups.parts[i].name.as_str())
            .unwrap_or("-"),
    );
}

#[allow(clippy::type_complexity)]
fn map_scene_ready(
    event: On<WorldInstanceReady>,
    roots: Query<(), Or<(With<DrawGroups>, With<world::DetailWeight>)>>,
    mut pending: ResMut<Pending>,
) {
    if roots.contains(event.entity) && pending.ready_roots.insert(event.entity) {
        pending.ready += 1;
    }
}

#[allow(clippy::type_complexity)]
fn track_loading(
    assets: Res<AssetServer>,
    mut pending: ResMut<Pending>,
    instances: Res<WorldInstanceSpawner>,
    worlds: Query<(Entity, Option<&WorldInstance>), With<WorldAssetRoot>>,
    parents: Query<&ChildOf>,
    roots: Query<(), With<DrawGroups>>,
) {
    let _timer = crate::perf_log::SystemTimer::start("track_loading");
    if pending.loaded_at.is_some() {
        return;
    }
    if pending.ready < pending.instances {
        return;
    }
    // A GLTF scene contains nested node worlds. The outer scene's Ready event
    // does not mean those node instances have finished creating their meshes.
    for (entity, instance) in &worlds {
        let mut ancestor = entity;
        while !roots.contains(ancestor) {
            let Ok(parent) = parents.get(ancestor) else {
                break;
            };
            ancestor = parent.parent();
        }
        if roots.contains(ancestor) && instance.is_none_or(|i| !instances.instance_is_ready(**i)) {
            return;
        }
    }
    let done = pending.handles.iter().all(|h| {
        let id: UntypedAssetId = h.id();
        assets.is_loaded_with_dependencies(id)
            || matches!(assets.get_load_state(id), Some(LoadState::Failed(_)))
    });
    if done {
        info!(
            "all map assets loaded ({} instances) {:.1} s after the map was added",
            pending.instances,
            pending.added_at.elapsed().as_secs_f32()
        );
        pending.loaded_at = Some(Instant::now());
    }
}

/// What the screenshot log reports.
#[derive(bevy::ecs::system::SystemParam)]
struct CaptureStats<'w, 's> {
    diagnostics: Res<'w, DiagnosticsStore>,
    meshes: Query<'w, 's, (), With<Mesh3d>>,
    pieces: Query<'w, 's, &'static Visibility, With<DrawGroups>>,
    groups: Res<'w, Groups>,
}

fn capture(
    mut commands: Commands,
    pending: Res<Pending>,
    mut capture: ResMut<Capture>,
    stats: CaptureStats,
    mut readiness: crate::map::ready::Ready,
    mut exit: MessageWriter<AppExit>,
) {
    let CaptureStats {
        diagnostics,
        meshes,
        pieces,
        groups,
    } = stats;
    if capture.started.elapsed() > Duration::from_secs(300) {
        error!("timed out waiting for the map to load");
        exit.write(AppExit::error());
        return;
    }
    let Some(loaded) = pending.loaded_at else {
        return;
    };
    capture.ready_frames += 1;
    match capture.requested {
        None if loaded.elapsed().as_secs_f32() >= capture.at => {
            // Residency becomes drawable after the prior render's probe was sampled.
            // Let the visibility change reach specialization before trusting a zero queue.
            if capture.ready_frames < 12 || !readiness.allow() {
                return;
            }
            let fps = diagnostics
                .get(&FrameTimeDiagnosticsPlugin::FPS)
                .and_then(|d| d.smoothed())
                .unwrap_or(0.0);
            let shown = pieces.iter().filter(|v| **v != Visibility::Hidden).count();
            println!(
                "{fps:.1} fps, {} mesh entities, {shown}/{} pieces shown (groups {:?} from {}), loaded in {:.1} s",
                meshes.iter().count(),
                pieces.iter().count(),
                groups.mode,
                groups
                    .current
                    .map(|i| groups.parts[i].name.as_str())
                    .unwrap_or("-"),
                (loaded - capture.started).as_secs_f32()
            );
            // GPU time of the heaviest render passes (timestamp queries), when available.
            let mut passes: Vec<(String, f64)> = diagnostics
                .iter()
                .filter(|d| d.path().as_str().ends_with("elapsed_gpu"))
                .filter_map(|d| Some((d.path().as_str().to_string(), d.smoothed()?)))
                .collect();
            passes.sort_by(|a, b| b.1.total_cmp(&a.1));
            for (path, ms) in passes.iter().take(6) {
                println!("  gpu {ms:.2} ms  {path}");
            }
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(capture.path.clone()));
            capture.requested = Some(Instant::now());
        }
        Some(at) if capture.path.is_file() && at.elapsed() > Duration::from_millis(500) => {
            println!("screenshot saved to {}", capture.path.display());
            exit.write(AppExit::Success);
        }
        _ => {}
    }
}

/// Parses `--camera x,y,z,yaw,pitch`.
pub fn parse_camera(arg: Option<&str>) -> Result<Option<[f32; 5]>> {
    let Some(arg) = arg else { return Ok(None) };
    let v: Vec<f32> = arg
        .split(',')
        .map(|s| s.trim().parse::<f32>())
        .collect::<std::result::Result<_, _>>()
        .with_context(|| format!("--camera {arg}"))?;
    match v[..] {
        [x, y, z, yaw, pitch] => Ok(Some([x, y, z, yaw, pitch])),
        [x, y, z] => Ok(Some([x, y, z, 0.0, 0.0])),
        _ => anyhow::bail!("--camera wants x,y,z,yaw,pitch"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readiness_ignores_nested_worlds_and_repeated_root_events() {
        let mut app = App::new();
        app.init_resource::<Pending>().add_observer(map_scene_ready);
        let root = app.world_mut().spawn(DrawGroups([1; 8])).id();
        let nested = app.world_mut().spawn(ChildOf(root)).id();
        let unrelated = app.world_mut().spawn_empty().id();
        let detail = app.world_mut().spawn(world::DetailWeight(1.)).id();
        let instance_id = WorldInstanceSpawner::default()
            .spawn(Handle::<bevy::world_serialization::WorldAsset>::default());
        for entity in [nested, unrelated, root, root, detail] {
            app.world_mut().trigger(WorldInstanceReady {
                entity,
                instance_id,
            });
        }
        assert_eq!(app.world().resource::<Pending>().ready, 2);
    }

    #[test]
    fn gltf_loader_preserves_native_material_blend_streams() {
        let target = std::env::var_os("CARGO_TARGET_DIR").unwrap_or_else(|| "target".into());
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(target)
            .join(format!("tests/map-stream-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let tint_wet = [0.2f32, 0.3, 0.4, 0.5];
        let mut bin = Vec::new();
        for value in [0.0f32, 0., 0., 1., 0., 0., 0., 1., 0.] {
            bin.extend_from_slice(&value.to_le_bytes());
        }
        for _ in 0..3 {
            for value in tint_wet {
                bin.extend_from_slice(&value.to_le_bytes());
            }
        }
        let doc = serde_json::json!({
            "asset": {"version": "2.0"}, "buffers": [{"byteLength": bin.len()}],
            "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": 36},
                {"buffer": 0, "byteOffset": 36, "byteLength": 48}],
            "accessors": [{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
                "min": [0,0,0], "max": [1,1,0]},
                {"bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC4"}],
            "meshes": [{"primitives": [{"attributes": {"POSITION": 0, "_MAP_BLEND2": 1}}]}],
            "nodes": [{"mesh": 0}], "scenes": [{"nodes": [0]}], "scene": 0
        });
        let mut json = serde_json::to_vec(&doc).unwrap();
        while !json.len().is_multiple_of(4) {
            json.push(b' ');
        }
        let mut glb = b"glTF".to_vec();
        glb.extend_from_slice(&2u32.to_le_bytes());
        glb.extend_from_slice(&((28 + json.len() + bin.len()) as u32).to_le_bytes());
        glb.extend_from_slice(&(json.len() as u32).to_le_bytes());
        glb.extend_from_slice(b"JSON");
        glb.extend_from_slice(&json);
        glb.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        glb.extend_from_slice(b"BIN\0");
        glb.extend_from_slice(&bin);
        std::fs::write(dir.join("stream.glb"), glb).unwrap();
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin {
                file_path: dir.to_string_lossy().into_owned(),
                ..default()
            },
            bevy::mesh::MeshPlugin,
            bevy::image::ImagePlugin::default(),
            bevy::scene::ScenePlugin,
            bevy::transform::TransformPlugin,
            layered_gltf_plugin(),
        ))
        .init_asset::<StandardMaterial>()
        .init_asset::<bevy::world_serialization::WorldAsset>();
        app.finish();
        app.cleanup();
        let handle = app
            .world()
            .resource::<AssetServer>()
            .load::<bevy::gltf::Gltf>("stream.glb");
        let started = Instant::now();
        loop {
            app.update();
            if let Some(gltf) = app
                .world()
                .resource::<Assets<bevy::gltf::Gltf>>()
                .get(&handle)
            {
                let meshes = app.world().resource::<Assets<bevy::gltf::GltfMesh>>();
                let primitive = &meshes.get(&gltf.meshes[0]).unwrap().primitives[0];
                let mesh = app
                    .world()
                    .resource::<Assets<Mesh>>()
                    .get(&primitive.mesh)
                    .unwrap();
                let Some(bevy::mesh::VertexAttributeValues::Float32x4(values)) =
                    mesh.attribute(ATTR_MAP_BLEND2)
                else {
                    panic!("native tint/wet stream lost");
                };
                assert_eq!(values, &[tint_wet; 3]);
                break;
            }
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "GLB load timed out"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn collision_regions_select_all_four_ids_even_with_culling_off() {
        let collision = sekiro_formats::hknp::CollisionMesh {
            vertices: vec![
                [0., 0., 0.],
                [10., 0., 0.],
                [0., 0., 10.],
                [20., 0., 0.],
                [30., 0., 0.],
                [20., 0., 10.],
            ],
            indices: vec![0, 1, 2, 3, 4, 5],
            materials: vec![0, 0],
        };
        let parts = [([110, 7, 9, 12], 0.), ([0; 4], 20.)]
            .into_iter()
            .enumerate()
            .map(|(first, (config, x))| CollisionPart {
                name: format!("h{first}"),
                first,
                count: 1,
                min: Vec2::new(x, 0.),
                max: Vec2::new(x + 10., 10.),
                display: [0; 8],
                draw: [0; 8],
                light_set: config[0],
                fog_set: config[1],
                scattering_set: config[2],
                env_map: config[3],
            })
            .collect();
        let mut app = App::new();
        app.insert_resource(ButtonInput::<KeyCode>::default())
            .insert_resource(Layout {
                json: serde_json::json!({}),
                collision: Some(collision),
                scene: MapScene {
                    prefix: String::new(),
                    collision: false,
                    no_pieces: false,
                    hide: vec![],
                    objects: false,
                    groups: GroupMode::Off,
                    markers: false,
                    hour: 6.,
                    gparam: None,
                    within: None,
                },
            })
            .insert_resource(Groups {
                mode: GroupMode::Off,
                parts,
                current: None,
                active: None,
                checked_at: None,
                dirty: false,
            })
            .insert_resource(MapLighting {
                gparam: Default::default(),
                hour: 6.,
                light_set: 0,
                applied: None,
                source: String::new(),
            })
            .init_resource::<crate::lighting::fog::FogSelection>()
            .init_resource::<crate::lighting_selection::RegionSelection>()
            .add_systems(Update, pick_groups);
        let focus = app
            .world_mut()
            .spawn((MapFocus, Transform::from_xyz(2., 1., 2.)))
            .id();
        for (x, ids) in [(2., [110, 7, 9, 12]), (22., [0; 4])] {
            app.world_mut()
                .get_mut::<Transform>(focus)
                .unwrap()
                .translation
                .x = x;
            app.update();
            assert_eq!(
                app.world()
                    .resource::<crate::lighting_selection::RegionSelection>()
                    .0,
                ids
            );
            assert_eq!(app.world().resource::<MapLighting>().light_set, ids[0]);
            assert_eq!(
                app.world()
                    .resource::<crate::lighting::fog::FogSelection>()
                    .0,
                ids[1]
            );
        }
    }

    #[test]
    fn draw_param_stems_preserve_cinematic_suffix() {
        assert_eq!(default_gparam("m11_02_00_00"), "m11_02_0000");
        assert_eq!(
            cinematic_gparam("m89_00_00_99").as_deref(),
            Some("s89_00_0099")
        );
        assert_eq!(cinematic_gparam("invalid"), None);
        assert_eq!(cinematic_gparam("m89_00_0x_99"), None);
    }

    #[test]
    fn initial_viewer_camera_stays_in_front_of_wall() {
        let wall = sekiro_formats::hknp::CollisionMesh {
            vertices: vec![[-10.0, -10.0, 1.0], [10.0, -10.0, 1.0], [0.0, 10.0, 1.0]],
            indices: vec![0, 1, 2],
            materials: vec![0],
        };
        let eye = viewer_eye(Vec3::ZERO, Vec3::new(0.0, 2.2, 3.5), &wall);
        assert!(eye.z > 0.0 && eye.z < 1.0);
        assert!(eye.y >= 1.4);
        let clear = sekiro_formats::hknp::CollisionMesh::default();
        let requested = Vec3::new(0.0, 2.2, 3.5);
        assert_eq!(viewer_eye(Vec3::ZERO, requested, &clear), requested);
    }

    #[test]
    fn shadow_budget_preserves_characters_and_large_walls_and_follows_focus() {
        let mut app = App::new();
        let settings =
            ShadowSettings::from_args(&sekiro_formats::gparam::Gparam { groups: vec![] }, 12., &[])
                .unwrap();
        app.insert_resource(settings)
            .insert_resource(Time::<()>::default())
            .add_systems(Update, budget_map_shadows);
        let root = app.world_mut().spawn(DrawGroups([0; 8])).id();
        let focus = app
            .world_mut()
            .spawn((MapFocus, GlobalTransform::default()))
            .id();
        let bounds = Aabb {
            center: Vec3::ZERO.into(),
            half_extents: Vec3::ONE.into(),
        };
        let character = app
            .world_mut()
            .spawn((
                Mesh3d::default(),
                GlobalTransform::from_translation(Vec3::X * 100.),
                bounds,
            ))
            .id();
        let far = app
            .world_mut()
            .spawn((
                Mesh3d::default(),
                ChildOf(root),
                GlobalTransform::from_translation(Vec3::X * 100.),
                bounds,
            ))
            .id();
        let wall = app
            .world_mut()
            .spawn((
                Mesh3d::default(),
                ChildOf(root),
                GlobalTransform::from_translation(Vec3::X * 100.),
                Aabb {
                    center: Vec3::ZERO.into(),
                    half_extents: Vec3::splat(80.).into(),
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs(1));
        app.update();
        assert!(app.world().get::<NotShadowCaster>(character).is_none());
        assert!(app.world().get::<NotShadowCaster>(far).is_some());
        assert!(app.world().get::<NotShadowCaster>(wall).is_none());
        *app.world_mut().get_mut::<GlobalTransform>(focus).unwrap() =
            GlobalTransform::from_translation(Vec3::X * 100.);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs(1));
        app.update();
        assert!(app.world().get::<NotShadowCaster>(far).is_none());
    }
}

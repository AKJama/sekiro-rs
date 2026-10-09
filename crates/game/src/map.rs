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

use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use bevy::app::AppExit;
use bevy::asset::RenderAssetUsages;
use bevy::asset::{AssetPlugin, LoadState, UntypedAssetId};
use bevy::camera::visibility::NoFrustumCulling;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::light::{CascadeShadowConfigBuilder, GlobalAmbientLight};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::pbr::wireframe::{Wireframe, WireframeColor, WireframePlugin};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::world_serialization::WorldInstanceReady;
use serde_json::Value;

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
    /// Also spawn MSB objects. Off by default: many are event-state props (siege barricades,
    /// story-gated doors) that the game enables from its event scripts, which we do not run.
    pub objects: bool,
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

struct CollisionPart {
    name: String,
    first: usize,
    count: usize,
    min: Vec2,
    max: Vec2,
    display: [u32; 8],
    draw: [u32; 8],
}

#[derive(Resource)]
struct Groups {
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
        *o = x.as_u64().unwrap_or(0) as u32;
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
#[derive(Resource, Default)]
struct Pending {
    handles: Vec<UntypedHandle>,
    /// Map piece scene instances spawned, and how many have finished spawning.
    instances: usize,
    ready: usize,
    loaded_at: Option<Instant>,
}

#[derive(Resource)]
struct Capture {
    path: PathBuf,
    at: f32,
    started: Instant,
    requested: Option<Instant>,
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
    options_collision: bool,
    no_pieces: bool,
    hide: Vec<String>,
    objects: bool,
}

pub fn run(options: Options) -> Result<()> {
    let dir = options
        .cache
        .join("maps")
        .join(&options.id)
        .canonicalize()
        .with_context(|| {
            format!(
                "cache/maps/{} not found: run `sekiro-extract map {}`",
                options.id, options.id
            )
        })?;
    let json: Value = serde_json::from_slice(
        &std::fs::read(dir.join("layout.json")).context("reading layout.json")?,
    )?;
    let collision = match std::fs::read(dir.join("collision.bin")) {
        Ok(bytes) => Some(sekiro_formats::hknp::CollisionMesh::from_bytes(&bytes)?),
        Err(_) => None,
    };

    // Start pose: --camera, else the first player start, at eye height.
    let (start, yaw, pitch) = match options.camera {
        Some([x, y, z, yaw, pitch]) => (Vec3::new(x, y, z), yaw, pitch),
        None => {
            let player = json["players"].get(options.start);
            let pos = player
                .and_then(|p| vec3(&p["translation"]))
                .unwrap_or(Vec3::ZERO);
            let yaw = player
                .and_then(|p| p["yaw_degrees"].as_f64())
                .unwrap_or(0.0) as f32;
            // Over the shoulder: 3.5 m behind and 2.2 m above the start point, so its marker
            // shows. Forward at yaw 0 is -Z.
            let back = Quat::from_rotation_y(yaw.to_radians()) * Vec3::Z * 3.5;
            (pos + back + Vec3::new(0.0, 2.2, 0.0), yaw, -12.0)
        }
    };

    // Bevy's IO threads overflow the default 2 MiB stack while loading this many GLBs and DDS
    // textures at once (still at 4 MiB; 8 MiB is enough). Threads read this when spawned.
    if std::env::var_os("RUST_MIN_STACK").is_none() {
        // SAFETY: no other thread exists yet, so nothing reads the environment concurrently.
        unsafe { std::env::set_var("RUST_MIN_STACK", (16usize << 20).to_string()) };
    }

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
            });
        }
    }

    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: format!("sekiro-rs map - {}", options.id),
                    resolution: (1600, 900).into(),
                    present_mode: if options.uncapped {
                        bevy::window::PresentMode::AutoNoVsync
                    } else {
                        bevy::window::PresentMode::AutoVsync
                    },
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: dir.to_string_lossy().into_owned(),
                ..default()
            }),
        WireframePlugin::default(),
        FrameTimeDiagnosticsPlugin::default(),
        bevy::render::diagnostic::RenderDiagnosticsPlugin,
    ))
    // Keep rendering at full rate when the window is not focused, so screenshots and
    // `--uncapped` measurements from a terminal are not throttled to 60 Hz.
    .insert_resource(bevy::winit::WinitSettings::continuous())
    .insert_resource(ClearColor(SKY))
    .insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.80, 0.85, 1.0),
        brightness: 900.0,
        ..default()
    })
    .insert_resource(Layout {
        json,
        collision,
        options_collision: options.collision,
        no_pieces: options.no_pieces,
        hide: options.hide,
        objects: options.objects,
    })
    .insert_resource(Fly {
        yaw: yaw.to_radians(),
        pitch: pitch.to_radians(),
        speed: 12.0,
    })
    .insert_resource(Groups {
        mode: options.groups,
        parts: group_parts,
        current: None,
        active: None,
        checked_at: None,
        dirty: true,
    })
    .init_resource::<Pending>()
    .add_observer(|_: On<WorldInstanceReady>, mut pending: ResMut<Pending>| {
        pending.ready += 1;
    })
    .add_systems(
        Startup,
        move |commands: Commands,
              assets: Res<AssetServer>,
              layout: Res<Layout>,
              pending: ResMut<Pending>,
              meshes: ResMut<Assets<Mesh>>,
              materials: ResMut<Assets<StandardMaterial>>| {
            setup(commands, assets, layout, pending, meshes, materials, start)
        },
    )
    .add_systems(Update, (fly, apply_fly).chain())
    .add_systems(Update, (toggle_collision, hud, track_loading))
    .add_systems(Update, (pick_groups, apply_groups).chain().after(apply_fly));
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
        })
        .add_systems(Update, capture.after(track_loading));
    }
    app.run();
    Ok(())
}

const SKY: Color = Color::srgb(0.62, 0.68, 0.74);

fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    layout: Res<Layout>,
    mut pending: ResMut<Pending>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    start: Vec3,
) {
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 60f32.to_radians(),
            near: 0.1,
            far: 20_000.0,
            ..default()
        }),
        Transform::from_translation(start),
        DistanceFog {
            color: SKY,
            directional_light_color: Color::srgba(1.0, 0.92, 0.75, 0.4),
            directional_light_exponent: 24.0,
            falloff: FogFalloff::from_visibility_colors(
                2_500.0,
                Color::srgb(0.55, 0.58, 0.62),
                Color::srgb(0.75, 0.78, 0.82),
            ),
        },
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 14_000.0,
            color: Color::srgb(1.0, 0.95, 0.86),
            shadow_maps_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            num_cascades: 4,
            maximum_distance: 250.0,
            first_cascade_far_bound: 15.0,
            ..default()
        }
        .build(),
        Transform::from_xyz(0.0, 0.0, 0.0).looking_to(Vec3::new(0.45, -0.75, 0.35), Vec3::Y),
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

    if !layout.no_pieces {
        let models = &layout.json["models"];
        let mut spawned = 0;
        let mut seen = std::collections::HashSet::new();
        let kinds: &[&str] = if layout.objects {
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
            if layout.hide.iter().any(|h| model.starts_with(h.as_str())) {
                continue;
            }
            let Some(glb) = models[model]["glb"].as_str() else {
                continue;
            };
            let Some(p) = pose(row) else { continue };
            if seen.insert(glb.to_string()) {
                pending
                    .handles
                    .push(assets.load::<Gltf>(glb.to_string()).untyped());
            }
            commands.spawn((
                DrawGroups(groups8(&row["draw_groups"])),
                WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(glb.to_string()))),
                Transform {
                    translation: p.translation,
                    rotation: p.rotation,
                    scale: p.scale,
                },
            ));
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
    for (key, default_material) in [("enemies", &enemy), ("players", &player)] {
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

    if let Some(c) = &layout.collision {
        let visibility = if layout.options_collision {
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
        if layout.no_pieces {
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
/// above) and takes its groups. Runs when the camera has moved a couple of metres.
fn pick_groups(
    keys: Res<ButtonInput<KeyCode>>,
    layout: Res<Layout>,
    mut groups: ResMut<Groups>,
    camera: Query<&Transform, With<Camera3d>>,
) {
    if keys.just_pressed(KeyCode::KeyG) {
        groups.mode = groups.mode.next();
        groups.dirty = true;
        groups.checked_at = None;
    }
    let (Ok(t), Some(c)) = (camera.single(), &layout.collision) else {
        return;
    };
    if groups.mode == GroupMode::Off {
        return;
    }
    let pos = t.translation;
    if groups.checked_at.is_some_and(|p| p.distance(pos) < 2.0) {
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
    groups.current = Some(pi);
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
    mut pieces: Query<(&DrawGroups, &mut Visibility)>,
    added: Query<(), Added<DrawGroups>>,
) {
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

fn track_loading(assets: Res<AssetServer>, mut pending: ResMut<Pending>) {
    if pending.loaded_at.is_some() {
        return;
    }
    if pending.ready < pending.instances {
        return;
    }
    let done = pending.handles.iter().all(|h| {
        let id: UntypedAssetId = h.id();
        assets.is_loaded_with_dependencies(id)
            || matches!(assets.get_load_state(id), Some(LoadState::Failed(_)))
    });
    if done {
        info!("all map assets loaded");
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
    match capture.requested {
        None if loaded.elapsed().as_secs_f32() >= capture.at => {
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

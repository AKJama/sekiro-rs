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
use bevy::asset::RenderAssetUsages;
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
}

#[derive(Resource)]
struct Fly {
    yaw: f32,
    pitch: f32,
    speed: f32,
}

#[derive(Component)]
struct CollisionView;

#[derive(Component)]
struct Hud;

/// Every GLB handle the map needs, to know when loading is done.
#[derive(Resource, Default)]
struct Pending {
    handles: Vec<UntypedHandle>,
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
            let player = json["players"].get(0);
            let pos = player
                .and_then(|p| vec3(&p["translation"]))
                .unwrap_or(Vec3::ZERO);
            let yaw = player
                .and_then(|p| p["yaw_degrees"].as_f64())
                .unwrap_or(0.0) as f32;
            (pos + Vec3::new(0.0, 1.7, 0.0), yaw, -5.0)
        }
    };

    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: format!("sekiro-rs map - {}", options.id),
                    resolution: (1600, 900).into(),
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
    ))
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
    })
    .insert_resource(Fly {
        yaw: yaw.to_radians(),
        pitch: pitch.to_radians(),
        speed: 12.0,
    })
    .init_resource::<Pending>()
    .add_systems(Startup, move |commands: Commands,
                                 assets: Res<AssetServer>,
                                 layout: Res<Layout>,
                                 pending: ResMut<Pending>,
                                 meshes: ResMut<Assets<Mesh>>| {
        setup(commands, assets, layout, pending, meshes, start)
    })
    .add_systems(Update, (fly, apply_fly).chain())
    .add_systems(Update, (toggle_collision, hud, track_loading));
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
        let limit: usize = std::env::var("MAP_LIMIT").ok().and_then(|v| v.parse().ok()).unwrap_or(usize::MAX);
        let skip: usize = std::env::var("MAP_SKIP").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
        for row in layout.json["pieces"].as_array().into_iter().flatten().skip(skip).take(limit) {
            let Some(model) = row["model"].as_str() else {
                continue;
            };
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
                WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(glb.to_string()))),
                Transform {
                    translation: p.translation,
                    rotation: p.rotation,
                    scale: p.scale,
                },
            ));
            spawned += 1;
        }
        info!("spawned {spawned} map piece instances");
    }

    if let Some(c) = &layout.collision {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
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
            if layout.options_collision {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
        ));
        info!(
            "collision: {} triangles",
            c.indices.len() / 3
        );
    }
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

fn toggle_collision(
    keys: Res<ButtonInput<KeyCode>>,
    mut view: Query<&mut Visibility, With<CollisionView>>,
) {
    if keys.just_pressed(KeyCode::KeyC) {
        for mut v in &mut view {
            *v = match *v {
                Visibility::Hidden => Visibility::Visible,
                _ => Visibility::Hidden,
            };
        }
    }
}

fn hud(
    diagnostics: Res<DiagnosticsStore>,
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
         RMB look, WASD fly, Space/Q up/down, Shift fast, wheel speed, C collision, P print pose",
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
        }
    );
}

fn track_loading(assets: Res<AssetServer>, mut pending: ResMut<Pending>) {
    if pending.loaded_at.is_some() {
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

fn capture(
    mut commands: Commands,
    pending: Res<Pending>,
    mut capture: ResMut<Capture>,
    diagnostics: Res<DiagnosticsStore>,
    meshes: Query<(), With<Mesh3d>>,
    mut exit: MessageWriter<AppExit>,
) {
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
            println!(
                "{fps:.1} fps, {} mesh entities, loaded in {:.1} s",
                meshes.iter().count(),
                (loaded - capture.started).as_secs_f32()
            );
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

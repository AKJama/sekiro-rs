//! `--play`: Wolf against an enemy (`--enemy`, default the Ashina soldier c1010; `c5060` is the
//! Great Shinobi Owl) on a floor, both driven by the game's own scripts, behaviour graphs and TAE
//! (`sekiro_sim::duel::Duel`).
//!
//! The simulation runs at a fixed 60 steps per second. Each step it receives an [`InputFrame`]
//! built from keyboard, mouse and gamepad (or from an input script) and reports, per character,
//! the full-body clip, its time, whether it just started and its TAE blend-in, plus the body's
//! position and facing. This module renders that: an [`Animator`] per character plays the
//! reported clip at the reported time with root motion disabled, crossfading over the blend-in,
//! and the entity follows the simulated body.
//!
//! Controls follow Sekiro's PC defaults where possible: WASD move, mouse look, LMB attack, RMB
//! guard, Space jump, Shift step (hold to sprint), C crouch, F grapple, Ctrl or MMB prosthetic,
//! R item, Q lock-on. Gamepad: left stick move, right stick camera, R1 attack, L1 guard, A jump,
//! B step/sprint, L3 crouch, L2 grapple, R2 prosthetic, X item, R3 lock-on. Esc frees the mouse.
//!
//! With `--map <id> [--start <n>]` the fight happens on an exported map instead of the floor:
//! Wolf stands at the player start, the enemy 5 m in front of him, both bodies walk on the
//! map's hit collision (`sekiro_sim::collision`), the map renders as in `--map` (see
//! [`crate::map::add_map`]) and the camera is pulled in when a wall is between it and Wolf.
//!
//! With `--arena <name>` (`sekiro_sim::arena`) the fight happens where the game places the
//! boss: his MSB part's transform, Wolf at the arena's player start facing him, and only the
//! map pieces and collision within the arena radius (`--full-map` loads all of it). A
//! character that falls far below its start is put back there.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use bevy::app::AppExit;
use bevy::asset::AssetPlugin;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::light::GlobalAmbientLight;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::{CursorGrabMode, CursorOptions};
use sekiro_sim::duel::{Duel, DuelReport};
use sekiro_sim::enemy::EnemySpec;
use sekiro_sim::{Buttons, InputFrame, StepReport, demo};

use crate::character::{AnimLibrary, Animator, Rig, animate, bind_rigs};
use crate::combat_hud;
use crate::draw_mask::{DrawMask, MaskSource, apply_draw_masks};
use crate::map::{self, MapFocus, MapScene};
use sekiro_sim::arena::{Arena, ArenaSpec};
use sekiro_sim::collision::CollisionWorld;

pub struct Options {
    pub cache: PathBuf,
    /// Input script to play instead of live input (`builtin` for the demo).
    pub script: Option<String>,
    /// Raw keyboard and mouse input to replay through the live input path
    /// ([`crate::input_replay`]).
    pub input: Option<String>,
    /// Screenshots to take at given simulation steps of the script: `(step, path)`.
    pub screenshots: Vec<(usize, PathBuf)>,
    /// Exit when the script ends and every screenshot is saved.
    pub exit_after_script: bool,
    /// Let the enemy fight (it always stands there; without this it stays passive).
    pub enemy: bool,
    /// Which enemy (`--enemy`): its model, animations, scripts, params and AI.
    pub opponent: EnemySpec,
    /// Start locked on to the enemy.
    pub lock_on: bool,
    /// Drive the enemy with the stand-in brain instead of its own AI scripts.
    pub simple_ai: bool,
    /// Who chooses a boss's acts (`--brain lua|scripted`) and whether its decisions are logged
    /// to `cache/brain_logs/` (`--brain-log`); see `docs/BRAIN.md`.
    pub brain: sekiro_sim::brain::BrainKind,
    pub brain_log: bool,
    /// Play on this exported map (`cache/maps/<id>`) instead of the flat floor.
    pub map: Option<String>,
    /// Player start point on the map.
    pub start: usize,
    /// Place Wolf at `x, y, z` (on the floor at or below `y + 1`) facing `yaw` degrees,
    /// instead of at a start point.
    pub spawn: Option<[f32; 4]>,
    /// Time of day (hours) and draw param override for the map lighting.
    pub hour: f32,
    pub gparam: Option<String>,
    /// A boss arena (`--arena owl`): the boss's map, his MSB placement and Wolf's start;
    /// replaces `map`, `start` and `spawn`.
    pub arena: Option<&'static ArenaSpec>,
    /// With `arena`, load the whole map instead of the pieces and collision within its radius.
    pub full_map: bool,
    /// On a map, skip map piece models whose name starts with any of these (as `--map --hide`).
    pub hide: Vec<String>,
    /// On a map, also spawn MSB objects (off by default, as in `--map`: many are event-state
    /// props).
    pub objects: bool,
}

/// The map's collision, for the camera; absent on the flat floor.
#[derive(Resource)]
struct MapCollision(Arc<CollisionWorld>);

/// Where model GLBs live below the asset root.
#[derive(Resource)]
struct ModelPath(&'static str);

const STEP_HZ: f64 = 60.0;
/// Radians of camera turn per mouse count (0.0035 was reported as far too sensitive).
const MOUSE_SENSITIVITY: f32 = 0.0012;
const WOLF: usize = 0;
const ENEMY: usize = 1;

/// Which simulated character an entity shows (0 Wolf, 1 the enemy).
#[derive(Component)]
struct Actor(usize);

#[derive(Component)]
struct Hud;

/// One of a small pool of markers that show bullets in flight (shuriken), by index.
#[derive(Component)]
struct BulletMarker(usize);

const BULLET_MARKERS: usize = 12;

#[derive(Component)]
struct FollowCamera;

/// Clip libraries per actor.
#[derive(Resource)]
struct Libraries([AnimLibrary; 2]);

/// The simulation and its last report. Not `Send`: the HKS VM uses `Rc`.
struct Sim {
    duel: Duel,
    last: DuelReport,
    /// Clip starts not yet shown, per actor: (clip, blend-in seconds).
    started: [Option<(String, f32)>; 2],
    /// The last hits, for the HUD.
    hit_log: Vec<String>,
    script: Option<Vec<InputFrame>>,
    step: usize,
    lock_requested: bool,
    /// Each actor's position and drawn yaw before the last step, to draw between steps.
    previous: [([f32; 3], f32); 2],
}

/// Live input gathered every rendered frame, consumed by the fixed steps.
#[derive(Resource, Default)]
struct LiveInput {
    frame: InputFrame,
}

#[derive(Resource)]
struct Orbit {
    focus: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
}

#[derive(Resource)]
struct Captures {
    pending: Vec<(usize, PathBuf)>,
    in_flight: Vec<PathBuf>,
    exit_after_script: bool,
}

pub fn run(options: Options) -> Result<()> {
    let cache = options.cache.canonicalize().context("cache directory")?;
    let libraries = [
        AnimLibrary::load(&cache, "c0000")?,
        AnimLibrary::load(&cache, &options.opponent.chr)?,
    ];
    let mut duel = Duel::load_with(&cache, 6.0, options.opponent.clone())
        .map_err(|e| anyhow::anyhow!("loading the simulation: {e}"))?;
    duel.enemy_active = options.enemy;
    if options.simple_ai {
        duel.use_simple_ai();
    } else if options.brain != sekiro_sim::brain::BrainKind::Lua || options.brain_log {
        let log = duel
            .set_brain(options.brain.build(), options.brain_log)
            .map_err(|e| anyhow::anyhow!("--brain: {e}"))?;
        eprintln!(
            "boss brain {:?}{}",
            options.brain,
            log.map(|p| format!(", log {}", p.display()))
                .unwrap_or_default()
        );
    }
    let mut on_map = None;
    let mut start_yaw = 0.0;
    let mut within = None;
    if let Some(spec) = options.arena {
        let t = std::time::Instant::now();
        let arena = Arena::load(&cache, spec, options.full_map).map_err(|e| {
            anyhow::anyhow!(
                "arena {}: {e} (run `sekiro-extract map {}`)",
                spec.name,
                spec.map
            )
        })?;
        arena.place(&mut duel);
        start_yaw = arena.layout.wolf.yaw;
        if !options.full_map {
            within = Some((arena.layout.center(), spec.radius));
        }
        eprintln!(
            "arena {} on {}: {} at {:?}, Wolf ({}) at {:?}; {} collision triangles{} in {:.2} s",
            spec.name,
            spec.map,
            spec.boss_part,
            duel.enemy.body.position,
            arena.layout.wolf_source,
            duel.player.body.position,
            arena.world.triangle_count(),
            if options.full_map {
                " (full map)".to_string()
            } else {
                format!(" within {} m", spec.radius)
            },
            t.elapsed().as_secs_f32()
        );
        on_map = Some((
            map::map_dir(&cache, spec.map)?,
            spec.map.to_string(),
            arena.world.clone(),
        ));
        map::raise_io_stack();
    } else if let Some(id) = &options.map {
        let dir = map::map_dir(&cache, id)?;
        let world =
            Arc::new(CollisionWorld::load(&dir.join("collision.bin")).map_err(anyhow::Error::msg)?);
        let (start, yaw_degrees) = match options.spawn {
            Some([x, y, z, yaw]) => {
                let y = world.floor(x, z, y + 1.0).unwrap_or(y);
                (Vec3::new(x, y, z), yaw)
            }
            None => map::player_start(&dir, options.start)?,
        };
        let yaw = yaw_degrees.to_radians();
        start_yaw = yaw;
        let floor_at = |x: f32, z: f32, near: f32| world.floor(x, z, near + 1.0).unwrap_or(near);
        let wolf = &mut duel.player.body;
        wolf.position = [start.x, floor_at(start.x, start.z, start.y), start.z];
        wolf.yaw = yaw;
        // The enemy 5 m in front of Wolf, facing him.
        let ahead = Quat::from_rotation_y(yaw) * Vec3::NEG_Z * 5.0;
        let (ex, ez) = (start.x + ahead.x, start.z + ahead.z);
        let enemy = &mut duel.enemy.body;
        enemy.position = [ex, floor_at(ex, ez, start.y), ez];
        enemy.yaw = yaw + std::f32::consts::PI;
        duel.player.ground = Box::new(world.clone());
        duel.enemy.ground = Box::new(world.clone());
        eprintln!(
            "{id}: start {} at {:?}, {} collision triangles",
            options.start,
            duel.player.body.position,
            world.triangle_count()
        );
        on_map = Some((dir, id.clone(), world));
        map::raise_io_stack();
    }
    let script = match options.script.as_deref() {
        None => None,
        Some("builtin") => Some(demo::parse(demo::DEMO).map_err(anyhow::Error::msg)?),
        Some(path) => Some(
            demo::parse(&std::fs::read_to_string(path).with_context(|| path.to_string())?)
                .map_err(anyhow::Error::msg)?,
        ),
    };
    let scripted = script.is_some();
    let replay = match options.input.as_deref() {
        None => None,
        Some(path) => Some(crate::input_replay::InputReplay::new(
            crate::input_replay::parse(
                &std::fs::read_to_string(path).with_context(|| path.to_string())?,
            )
            .map_err(anyhow::Error::msg)?,
        )),
    };
    for (_, path) in &options.screenshots {
        if path.exists() {
            std::fs::remove_file(path)?;
        }
    }

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "sekiro-rs".into(),
                    resolution: (1600, 900).into(),
                    ..default()
                }),
                // Scripted and replayed runs leave the cursor alone (a replay clicks to take it).
                primary_cursor_options: Some(CursorOptions {
                    visible: scripted || replay.is_some(),
                    grab_mode: if scripted || replay.is_some() {
                        CursorGrabMode::None
                    } else {
                        CursorGrabMode::Locked
                    },
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                // On a map the asset root is the cache, so models and map pieces both load.
                file_path: if on_map.is_some() {
                    cache.to_string_lossy().into_owned()
                } else {
                    cache.join("models").to_string_lossy().into_owned()
                },
                ..default()
            }),
    )
    .insert_resource(ModelPath(if on_map.is_some() { "models/" } else { "" }))
    .insert_resource(ClearColor(Color::srgb(0.52, 0.60, 0.70)))
    .insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.95, 0.95, 1.0),
        brightness: 450.0,
        ..default()
    })
    .insert_resource(bevy::winit::WinitSettings::continuous())
    .insert_resource(Time::<Fixed>::from_hz(STEP_HZ))
    .insert_resource(Libraries(libraries))
    .insert_resource(LiveInput::default())
    .insert_resource(Orbit {
        focus: Vec3::from_array(duel.player.body.position) + Vec3::Y * duel.camera.chr_org_offset_y,
        // Behind Wolf, so scripted "forward" is the way he faces.
        yaw: start_yaw,
        pitch: 0.25,
        distance: duel.camera.cam_dist,
    })
    .insert_resource(Captures {
        pending: options.screenshots,
        in_flight: Vec::new(),
        exit_after_script: options.exit_after_script,
    })
    .insert_non_send(Sim {
        last: DuelReport::default(),
        duel,
        started: [None, None],
        hit_log: Vec::new(),
        script,
        step: 0,
        lock_requested: options.lock_on,
        previous: [([0.0; 3], 0.0); 2],
    })
    .init_resource::<combat_hud::Gauges>()
    .add_plugins(crate::audio::GameAudioPlugin {
        cache: cache.clone(),
    })
    .add_systems(Startup, setup)
    .add_systems(FixedUpdate, simulate)
    .add_systems(
        Update,
        (
            // The camera turns first, so the stick read this frame is relative to the camera
            // that is drawn.
            orbit_camera,
            gather_input,
            bind_rigs,
            apply_draw_masks,
            sync_actors,
            sync_bullets,
            animate,
            follow_camera,
            hud,
            combat_hud::update,
            capture,
            log_camera,
        )
            .chain(),
    );
    if let Some(replay) = replay {
        app.insert_resource(replay).add_systems(
            PreUpdate,
            crate::input_replay::inject.before(bevy::input::InputSystems),
        );
    }
    if let Some((dir, id, world)) = on_map {
        // The map's sun, ambient light and sky replace the floor setup's.
        map::add_map(
            &mut app,
            &dir,
            MapScene {
                prefix: format!("maps/{id}/"),
                collision: false,
                no_pieces: false,
                hide: options
                    .hide
                    .iter()
                    .cloned()
                    .chain(
                        options
                            .arena
                            .into_iter()
                            .flat_map(|a| a.hidden_models.iter().map(|m| m.to_string())),
                    )
                    .collect(),
                objects: options.objects,
                groups: map::GroupMode::Draw,
                markers: false,
                hour: options.hour,
                gparam: options.gparam.clone(),
                within,
            },
        )?;
        app.insert_resource(MapCollision(world));
    }
    app.run();
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut libraries: ResMut<Libraries>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    model_path: Res<ModelPath>,
    on_map: Option<Res<MapCollision>>,
    sim: NonSend<Sim>,
) {
    commands.spawn((
        FollowCamera,
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: sim.duel.camera.cam_fov_y.to_radians(),
            ..default()
        }),
        Transform::from_xyz(0.0, 2.0, 4.0).looking_at(Vec3::new(0.0, 1.2, 0.0), Vec3::Y),
    ));
    combat_hud::spawn(&mut commands);
    // Bullets have no model yet (their look is an effect, FXR, not extracted): small spheres.
    let marker_mesh = meshes.add(Sphere::new(0.06).mesh().ico(2).unwrap());
    let marker_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.75, 0.78, 0.82),
        emissive: LinearRgba::rgb(0.6, 0.6, 0.65),
        metallic: 0.8,
        ..default()
    });
    for i in 0..BULLET_MARKERS {
        commands.spawn((
            BulletMarker(i),
            Mesh3d(marker_mesh.clone()),
            MeshMaterial3d(marker_material.clone()),
            Transform::IDENTITY,
            Visibility::Hidden,
        ));
    }
    let on_map = on_map.is_some();
    if !on_map {
        spawn_floor_stage(&mut commands, &mut meshes, &mut materials);
    }
    let models = [
        format!("{}c0000.glb", model_path.0),
        format!("{}{}.glb", model_path.0, sim.duel.spec.chr),
    ];
    for actor in [WOLF, ENEMY] {
        let lib = &mut libraries.0[actor];
        let mut idle = Animator::new(
            lib.clip("a000_000000").expect("idle clip is extracted"),
            false,
        );
        idle.apply_root_motion = false;
        idle.speed = 0.0;
        let mut e = commands.spawn((
            Actor(actor),
            WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(models[actor].clone()))),
            Transform::IDENTITY,
            Rig::new(Arc::new(lib.skeleton.clone())),
            idle,
        ));
        if actor == ENEMY {
            e.insert(DrawMask::new(MaskSource::Combat));
        }
        if actor == WOLF && on_map {
            // Draw groups follow Wolf.
            e.insert(MapFocus);
        }
    }
    commands.spawn((
        Hud,
        Text::new(""),
        TextFont {
            font_size: bevy::text::FontSize::Px(15.0),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

/// Two lights and a checkered floor, so movement and speed are visible off the map.
fn spawn_floor_stage(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            color: Color::srgb(1.0, 0.96, 0.90),
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-3.0, 6.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 2_500.0,
            color: Color::srgb(0.85, 0.9, 1.0),
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(3.0, 2.0, -4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // A checkered floor so movement and speed are visible.
    let tile = meshes.add(Plane3d::default().mesh().size(2.0, 2.0));
    let dark = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.19, 0.17),
        perceptual_roughness: 0.95,
        ..default()
    });
    let light = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.28, 0.25),
        perceptual_roughness: 0.95,
        ..default()
    });
    for i in -30..30 {
        for j in -30..30 {
            let mat = if (i + j) % 2 == 0 { &dark } else { &light };
            commands.spawn((
                Mesh3d(tile.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(i as f32 * 2.0 + 1.0, 0.0, j as f32 * 2.0 + 1.0),
            ));
        }
    }
}

fn gather_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    gamepads: Query<&Gamepad>,
    orbit: Res<Orbit>,
    mut live: ResMut<LiveInput>,
    mut cursor: Query<&mut CursorOptions>,
    mut grab_click: Local<bool>,
) {
    if keys.just_pressed(KeyCode::Escape)
        && let Ok(mut c) = cursor.single_mut()
    {
        let locked = c.grab_mode == CursorGrabMode::Locked;
        c.grab_mode = if locked {
            CursorGrabMode::None
        } else {
            CursorGrabMode::Locked
        };
        c.visible = locked;
    }
    // A click while the cursor is free (after Esc) takes the mouse back for the camera; that
    // click is not an attack.
    if !mouse.pressed(MouseButton::Left) {
        *grab_click = false;
    }
    if mouse.just_pressed(MouseButton::Left)
        && let Ok(mut c) = cursor.single_mut()
        && c.grab_mode != CursorGrabMode::Locked
    {
        c.grab_mode = CursorGrabMode::Locked;
        c.visible = false;
        *grab_click = true;
    }
    let key = |k| keys.pressed(k);
    let mut stick = Vec2::new(
        (key(KeyCode::KeyD) as i32 - key(KeyCode::KeyA) as i32) as f32,
        (key(KeyCode::KeyW) as i32 - key(KeyCode::KeyS) as i32) as f32,
    );
    let mut b = Buttons {
        attack: mouse.pressed(MouseButton::Left) && !*grab_click,
        guard: mouse.pressed(MouseButton::Right),
        jump: key(KeyCode::Space),
        dodge: key(KeyCode::ShiftLeft) || key(KeyCode::ShiftRight),
        crouch: key(KeyCode::KeyC),
        grapple: key(KeyCode::KeyF),
        prosthetic: key(KeyCode::ControlLeft) || mouse.pressed(MouseButton::Middle),
        item: key(KeyCode::KeyR),
        combat_art: false,
        lock_on: key(KeyCode::KeyQ),
    };
    for pad in &gamepads {
        let s = pad.left_stick();
        if s.length() > 0.2 {
            stick = s;
        }
        b.attack |= pad.pressed(GamepadButton::RightTrigger);
        b.guard |= pad.pressed(GamepadButton::LeftTrigger);
        b.jump |= pad.pressed(GamepadButton::South);
        b.dodge |= pad.pressed(GamepadButton::East);
        b.crouch |= pad.pressed(GamepadButton::LeftThumb);
        b.grapple |= pad.pressed(GamepadButton::LeftTrigger2);
        b.prosthetic |= pad.pressed(GamepadButton::RightTrigger2);
        b.item |= pad.pressed(GamepadButton::West);
        b.lock_on |= pad.pressed(GamepadButton::RightThumb);
    }
    if stick.length() > 1.0 {
        stick = stick.normalize();
    }
    live.frame = InputFrame {
        move_stick: [stick.x, stick.y],
        camera_yaw: orbit.yaw,
        buttons: b,
    };
}

fn orbit_camera(
    time: Res<Time>,
    motion: Res<AccumulatedMouseMotion>,
    cursor: Query<&CursorOptions>,
    gamepads: Query<&Gamepad>,
    sim: NonSend<Sim>,
    mut orbit: ResMut<Orbit>,
) {
    // Locked on, the camera swings behind Wolf toward the target.
    let params = &sim.duel.camera;
    let dt = time.delta_secs();
    if let Some(t) = sim.duel.player.lock_target {
        let target = Vec3::from_array(t) + Vec3::Y * params.chr_org_offset_y;
        let want = sekiro_sim::camera::lock_target_yaw(orbit.focus, target);
        let d = sekiro_sim::input::angle_diff(want, orbit.yaw);
        orbit.yaw += d * sekiro_sim::camera::chase(params.lock_rot_chase_rate_y, dt);
        let pitch = params.lock_target_pitch(orbit.focus, target);
        orbit.pitch +=
            (pitch - orbit.pitch) * sekiro_sim::camera::chase(params.lock_rot_chase_rate_x, dt);
        let (min, max) = params.pitch_limits(true, target.y - orbit.focus.y);
        orbit.pitch = orbit.pitch.clamp(min, max);
        return;
    }
    let locked = cursor
        .single()
        .is_ok_and(|c| c.grab_mode == CursorGrabMode::Locked);
    // Mouse motion while the map loads (the window appearing, the cursor being captured) would
    // spin the camera away from the start view before the fight begins.
    if locked && sim.step > 0 {
        orbit.yaw -= motion.delta.x * MOUSE_SENSITIVITY;
        orbit.pitch += motion.delta.y * MOUSE_SENSITIVITY;
    }
    for pad in &gamepads {
        let r = pad.right_stick();
        if r.length() > 0.2 {
            orbit.yaw -= r.x * params.rot_speed_max_y * dt;
            orbit.pitch -= r.y * params.rot_speed_max_x * dt;
        }
    }
    let (min, max) = params.pitch_limits(false, 0.0);
    orbit.pitch = orbit.pitch.clamp(min, max);
}

#[allow(clippy::too_many_arguments)]
fn simulate(
    mut sim: NonSendMut<Sim>,
    live: Res<LiveInput>,
    orbit: Res<Orbit>,
    rigs: Query<&Rig, With<Actor>>,
    sounds: Res<crate::audio::SoundLibrary>,
    mut cues: MessageWriter<crate::audio::SoundCue>,
    map_pending: Option<Res<map::Pending>>,
    replay: Option<ResMut<crate::input_replay::InputReplay>>,
) {
    // Hold the simulation until the models (and the map) are on screen, so scripted runs line
    // up with frames.
    if rigs.is_empty() || !rigs.iter().all(|r| r.bound) || map_pending.is_some_and(|p| !p.loaded())
    {
        return;
    }
    if let Some(mut r) = replay {
        r.running = true;
    }
    let dt = 1.0 / STEP_HZ as f32;
    let mut frame = match &sim.script {
        Some(frames) => {
            // Scripted sticks are camera-relative to the current orbit, like a player's.
            let mut f = frames.get(sim.step).copied().unwrap_or_default();
            f.camera_yaw += orbit.yaw;
            f
        }
        // The stick is relative to the camera on screen now (the orbit of the last frame).
        None => InputFrame {
            camera_yaw: orbit.yaw,
            ..live.frame
        },
    };
    if sim.lock_requested && sim.step == 0 {
        frame.buttons.lock_on = true;
    }
    sim.previous = [
        (sim.duel.player.body.position, sim.duel.player.visual_yaw()),
        (sim.duel.enemy.body.position, sim.duel.enemy.visual_yaw()),
    ];
    let before = sim.duel.player.body.position;
    let report = sim.duel.step(&frame, dt);
    if log_enabled() {
        log_move(sim.step, &frame, before, &sim.duel, &report.player);
    }
    crate::audio::emit(&sim.duel, &report, &sounds, &mut cues);
    for (i, r) in [&report.player, &report.enemy].into_iter().enumerate() {
        if !r.behavior.script_errors.is_empty() {
            warn!("script errors ({i}): {:?}", r.behavior.script_errors);
        }
        if r.clip_started
            && let Some(name) = &r.animation
        {
            sim.started[i] = Some((name.clone(), r.blend_in));
        }
    }
    let step = sim.step;
    for (hit, res) in &report.hits {
        let text = format!(
            "step {step}: {} hit {} ({}), AtkParam {}: HP -{}, posture -{} / attacker -{}",
            ["Wolf", "enemy"][hit.attacker],
            ["Wolf", "enemy"][hit.defender],
            if res.deflected {
                "deflected"
            } else if res.guarded {
                "blocked"
            } else {
                "hit"
            },
            hit.attack.atk_id,
            res.hp_damage,
            res.posture_damage,
            res.attacker_posture_damage
        );
        info!("{text}");
        sim.hit_log.push(text);
        if sim.hit_log.len() > 4 {
            sim.hit_log.remove(0);
        }
    }
    if sim.script.is_some()
        && (step.is_multiple_of(30) || std::env::var_os("PLAY_LOG_ALL").is_some())
    {
        info!(
            "step {step} wolf {} {} at {:.2?}{} | enemy {} {}",
            report.player.behavior.state_path.join("/"),
            report.player.animation.as_deref().unwrap_or("-"),
            report.player.position,
            if report.player.grounded { "" } else { " AIR" },
            report.enemy.behavior.state_path.join("/"),
            report.enemy.animation.as_deref().unwrap_or("-"),
        );
    }
    sim.last = report;
    sim.step += 1;
}

fn sync_actors(
    mut sim: NonSendMut<Sim>,
    fixed: Res<Time<Fixed>>,
    mut libraries: ResMut<Libraries>,
    mut q: Query<(&Actor, &mut Animator, &mut Transform)>,
) {
    for (actor, mut anim, mut tf) in &mut q {
        let i = actor.0;
        let (position, yaw) = shown(&sim, i, fixed.overstep_fraction());
        tf.translation = position;
        tf.rotation = Quat::from_rotation_y(yaw);
        let report: &StepReport = if i == WOLF {
            &sim.last.player
        } else {
            &sim.last.enemy
        };
        let Some(name) = report.animation.clone() else {
            continue;
        };
        let time = report.anim_time;
        let just_started = report.clip_started;
        let started = sim.started[i].take();
        let restart = started.as_ref().is_some_and(|(n, _)| *n == name);
        if anim.clip_name() != name || restart {
            let blend = started.map_or(0.0, |(_, b)| b);
            match libraries.0[i].clip(&name) {
                Ok(clip) => {
                    anim.play_named(&name, clip, false, blend);
                    anim.apply_root_motion = false;
                    anim.speed = 0.0;
                }
                // Not extracted: keep showing the previous clip.
                Err(_) => continue,
            }
        }
        // The clip time between steps too, matching the drawn position.
        let behind = (1.0 - fixed.overstep_fraction()) / STEP_HZ as f32;
        anim.time = if just_started {
            time
        } else {
            (time - behind).max(0.0)
        };
    }
}

/// Where actor `i` is drawn: between its position and yaw before and after the last step, by
/// how far the render time is into the next step, so motion is smooth at any frame rate
/// (drawing the last step alone stalls and jumps whenever a frame runs no step or two).
fn shown(sim: &Sim, i: usize, alpha: f32) -> (Vec3, f32) {
    let chr = if i == WOLF {
        &sim.duel.player
    } else {
        &sim.duel.enemy
    };
    let (now, yaw) = (Vec3::from_array(chr.body.position), chr.visual_yaw());
    let (before, yaw_before) = sim.previous[i];
    let before = Vec3::from_array(before);
    // Teleports (respawn, arena reset) are not drawn as a slide.
    if sim.step == 0 || before.distance(now) > 2.0 {
        return (now, yaw);
    }
    let turn = sekiro_sim::input::angle_diff(yaw, yaw_before);
    (before.lerp(now, alpha), yaw_before + turn * alpha)
}

/// Puts a marker on each bullet in flight and hides the rest.
fn sync_bullets(sim: NonSend<Sim>, mut q: Query<(&BulletMarker, &mut Transform, &mut Visibility)>) {
    let live = &sim.duel.bullets.live;
    for (m, mut tf, mut vis) in &mut q {
        match live.get(m.0) {
            Some(b) => {
                tf.translation = b.position;
                *vis = Visibility::Visible;
            }
            None => *vis = Visibility::Hidden,
        }
    }
}

fn follow_camera(
    sim: NonSend<Sim>,
    time: Res<Time>,
    fixed: Res<Time<Fixed>>,
    mut orbit: ResMut<Orbit>,
    collision: Option<Res<MapCollision>>,
    mut cam: Query<&mut Transform, With<FollowCamera>>,
) {
    let Ok(mut tf) = cam.single_mut() else {
        return;
    };
    let wolf = shown(&sim, WOLF, fixed.overstep_fraction()).0;
    let params = &sim.duel.camera;
    let wanted = wolf + Vec3::Y * params.chr_org_offset_y;
    let xz = sekiro_sim::camera::chase(params.chr_trans_chase_rate_xz, time.delta_secs());
    let y = sekiro_sim::camera::chase(params.chr_trans_chase_rate_y, time.delta_secs());
    orbit.focus.x += (wanted.x - orbit.focus.x) * xz;
    orbit.focus.z += (wanted.z - orbit.focus.z) * xz;
    orbit.focus.y += (wanted.y - orbit.focus.y) * y;
    let target = orbit.focus;
    let rot = Quat::from_euler(EulerRot::YXZ, orbit.yaw, -orbit.pitch, 0.0);
    let back = rot * Vec3::Z;
    let mut distance = orbit.distance;
    // Pull the camera in front of walls between it and Wolf.
    if let Some(c) = collision
        && let Some(hit) = c.0.raycast(target, back, orbit.distance + 0.3)
    {
        distance = (hit - 0.3).clamp(0.4, orbit.distance);
    }
    let eye = target + back * distance;
    *tf = Transform::from_translation(eye).looking_at(target, Vec3::Y);
}

fn hud(
    sim: NonSend<Sim>,
    mut text: Query<&mut Text, With<Hud>>,
    mut gauges: ResMut<combat_hud::Gauges>,
) {
    let f = &sim.duel.rules.fighters;
    *gauges = combat_hud::Gauges {
        hp: [0, 1].map(|i| (f[i].vitality.hp, f[i].vitality.max_hp)),
        posture: [0, 1].map(|i| (f[i].posture.remaining, f[i].posture.max)),
        // Just above the enemy's body capsule (NpcParam hitHeight: 1.8 m soldier, 2.5 m Owl).
        enemy_head: Some(
            Vec3::from_array(sim.duel.enemy.body.position)
                + Vec3::Y * (sim.duel.fighters[1].height + 0.2),
        ),
        enemy_alive: !f[1].dead(),
        deathblow: sim.duel.deathblow_available(),
        enemy_name: sim.duel.spec.display_name(),
        enemy_marks: sekiro_sim::deathblow::deathblow_marks(f[1].vitality.deathblows_left),
    };
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    let r = &sim.last;
    let mut flags = r.player.action_flags.clone();
    flags.retain(|f| [3, 7, 11, 26, 115, 117, 119].contains(f));
    let mut s = format!(
        "Wolf  {}  [{} t={:.2}]  HP {}{}\nrefs {:?}  windows {:?}\nEnemy  {}  [{} t={:.2}]  HP {}\n",
        r.player.behavior.state_path.join(" / "),
        r.player.animation.as_deref().unwrap_or("-"),
        r.player.anim_time,
        sim.duel.rules.fighters[0].vitality.hp,
        if r.locked_on { "  LOCKED ON" } else { "" },
        r.player.behavior_refs,
        flags,
        r.enemy.behavior.state_path.join(" / "),
        r.enemy.animation.as_deref().unwrap_or("-"),
        r.enemy.anim_time,
        sim.duel.rules.fighters[1].vitality.hp,
    );
    for line in &sim.hit_log {
        s += line;
        s.push('\n');
    }
    s += "WASD move, mouse look, LMB attack, RMB guard, Space jump, Shift step/sprint, C crouch, Q lock-on, Esc mouse";
    text.0 = s;
}

fn capture(
    mut commands: Commands,
    sim: NonSend<Sim>,
    mut cap: ResMut<Captures>,
    rigs: Query<&Rig>,
    mut exit: MessageWriter<AppExit>,
    replay: Option<Res<crate::input_replay::InputReplay>>,
) {
    if !rigs.iter().all(|r| r.bound) {
        return;
    }
    let step = sim.step;
    let due: Vec<PathBuf> = cap
        .pending
        .iter()
        .filter(|(s, _)| *s <= step)
        .map(|(_, p)| p.clone())
        .collect();
    cap.pending.retain(|(s, _)| *s > step);
    for path in due {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.clone()));
        cap.in_flight.push(path);
    }
    cap.in_flight.retain(|p| !p.is_file());
    let script_done =
        sim.script.as_ref().is_some_and(|s| step >= s.len()) || replay.is_some_and(|r| r.done());
    if cap.exit_after_script && script_done && cap.pending.is_empty() && cap.in_flight.is_empty() {
        std::thread::sleep(Duration::from_millis(200));
        exit.write(AppExit::Success);
    }
}

/// Per-frame camera and per-step movement logs, on for `--input` replays or with
/// `PLAY_LOG_CAMERA` set.
fn log_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| {
        std::env::var_os("PLAY_LOG_CAMERA").is_some() || std::env::args().any(|a| a == "--input")
    })
}

/// One `move` line per simulation step: the camera yaw the step used, the stick and its world
/// direction, Wolf's facing and direction of travel, and the travel error against the stick.
fn log_move(step: usize, frame: &InputFrame, before: [f32; 3], duel: &Duel, r: &StepReport) {
    let wolf = &duel.player;
    let (dx, dz) = (r.position[0] - before[0], r.position[2] - before[2]);
    let speed = (dx * dx + dz * dz).sqrt() * STEP_HZ as f32;
    let travel = (-dx).atan2(-dz);
    let stick = wolf.input.stick_world_yaw();
    let err = match stick {
        Some(s) if speed > 0.05 => sekiro_sim::input::angle_diff(travel, s).to_degrees(),
        _ => f32::NAN,
    };
    let var = |n: &str| wolf.behavior.runtime.variable(n).unwrap_or(f32::NAN);
    eprintln!(
        "move {step} cam {:.1} stick {:.2},{:.2} stick_yaw {:.1} face {:.1} travel {:.1} speed {:.2} err {:.1} MoveAngle {:.1} lock {} {} {}",
        frame.camera_yaw.to_degrees(),
        frame.move_stick[0],
        frame.move_stick[1],
        stick.map_or(f32::NAN, f32::to_degrees),
        r.yaw.to_degrees(),
        travel.to_degrees(),
        speed,
        err,
        var("MoveAngle"),
        wolf.lock_target.is_some(),
        r.behavior.state_path.last().map_or("-", String::as_str),
        r.animation.as_deref().unwrap_or("-"),
    );
}

/// One `cam` line per rendered frame: cursor grab, the frame's mouse motion, lock-on, the orbit
/// and the rendered camera's forward direction as a yaw.
#[allow(clippy::too_many_arguments)]
fn log_camera(
    time: Res<Time>,
    motion: Res<AccumulatedMouseMotion>,
    cursor: Query<&CursorOptions>,
    orbit: Res<Orbit>,
    live: Res<LiveInput>,
    sim: NonSend<Sim>,
    cam: Query<&Transform, With<FollowCamera>>,
    replay: Option<Res<crate::input_replay::InputReplay>>,
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    fixed: Res<Time<Fixed>>,
) {
    if !log_enabled() {
        return;
    }
    let grab = cursor
        .single()
        .map_or("none".to_string(), |c| format!("{:?}", c.grab_mode));
    let fwd = cam.single().map_or(Vec3::NAN, |t| t.forward().as_vec3());
    let cam_yaw = (-fwd.x).atan2(-fwd.z);
    let focus = orbit.focus;
    let wolf = Vec3::from_array(sim.duel.player.body.position);
    let drawn = shown(&sim, WOLF, fixed.overstep_fraction()).0;
    eprintln!(
        "cam {} t {:.3} dt {:.4} focused {} grab {grab} mouse {:.1},{:.1} lock {} orbit_yaw {:.1} pitch {:.1} cam_yaw {:.1} stick {:.2},{:.2} live_cam {:.1} step {} focus_lag {:.3} drawn {:.3},{:.3} sim {:.3},{:.3}",
        replay.as_ref().map_or(0, |r| r.frame),
        time.elapsed_secs(),
        time.delta_secs(),
        window.single().is_ok_and(|w| w.focused),
        motion.delta.x,
        motion.delta.y,
        sim.duel.player.lock_target.is_some(),
        orbit.yaw.to_degrees(),
        orbit.pitch.to_degrees(),
        cam_yaw.to_degrees(),
        live.frame.move_stick[0],
        live.frame.move_stick[1],
        live.frame.camera_yaw.to_degrees(),
        sim.step,
        Vec2::new(focus.x - wolf.x, focus.z - wolf.z).length(),
        drawn.x,
        drawn.z,
        wolf.x,
        wolf.z,
    );
}

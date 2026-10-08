//! `--play`: Wolf on a floor, driven by the game's own scripts, behaviour graph and TAE.
//!
//! The simulation (`sekiro_sim::PlayerCharacter`) runs at a fixed 60 steps per second. Each
//! step it receives an [`InputFrame`] built from keyboard, mouse and gamepad (or from an input
//! script), and reports the full-body clip, its time and the body's position and facing. This
//! module only renders that: the [`Animator`] plays the reported clip at the reported time with
//! root motion disabled, and the character entity follows the simulated body.
//!
//! Controls follow Sekiro's PC defaults where possible: WASD move, mouse look, LMB attack, RMB
//! guard, Space jump, Shift step (hold to sprint), C crouch, F grapple, Ctrl or MMB prosthetic,
//! R item, Q lock-on. Gamepad: left stick move, right stick camera, R1 attack, L1 guard, A jump,
//! B step/sprint, L3 crouch, L2 grapple, R2 prosthetic, X item, R3 lock-on. Esc frees the mouse.

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
use sekiro_sim::{Buttons, InputFrame, PlayerCharacter, StepReport, demo};

use crate::character::{AnimLibrary, Animator, Rig, animate, bind_rigs};

pub struct Options {
    pub cache: PathBuf,
    /// Input script to play instead of live input (`builtin` for the demo).
    pub script: Option<String>,
    /// Screenshots to take at given simulation steps of the script: `(step, path)`.
    pub screenshots: Vec<(usize, PathBuf)>,
    /// Exit when the script ends and every screenshot is saved.
    pub exit_after_script: bool,
}

const STEP_HZ: f64 = 60.0;
const CAMERA_HEIGHT: f32 = 1.4;
const MOUSE_SENSITIVITY: f32 = 0.0035;
const STICK_CAMERA_SPEED: f32 = 2.6;

#[derive(Component)]
struct Wolf;

#[derive(Component)]
struct Hud;

#[derive(Component)]
struct FollowCamera;

#[derive(Resource)]
struct Library(AnimLibrary);

/// The simulation and its last report. Not `Send`: the HKS VM uses `Rc`.
struct Sim {
    wolf: PlayerCharacter,
    last: StepReport,
    script: Option<Vec<InputFrame>>,
    step: usize,
}

/// Live input gathered every rendered frame, consumed by the fixed steps.
#[derive(Resource, Default)]
struct LiveInput {
    frame: InputFrame,
}

#[derive(Resource)]
struct Orbit {
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
    let library = AnimLibrary::load(&cache, "c0000")?;
    let mut wolf = PlayerCharacter::load_from_cache(&cache)
        .map_err(|e| anyhow::anyhow!("loading the player simulation: {e}"))?;
    wolf.settle(1.0 / STEP_HZ as f32, 400);
    let script = match options.script.as_deref() {
        None => None,
        Some("builtin") => Some(demo::parse(demo::DEMO).map_err(anyhow::Error::msg)?),
        Some(path) => Some(
            demo::parse(&std::fs::read_to_string(path).with_context(|| path.to_string())?)
                .map_err(anyhow::Error::msg)?,
        ),
    };
    let scripted = script.is_some();
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
                primary_cursor_options: Some(CursorOptions {
                    visible: scripted,
                    grab_mode: if scripted {
                        CursorGrabMode::None
                    } else {
                        CursorGrabMode::Locked
                    },
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: cache.join("models").to_string_lossy().into_owned(),
                ..default()
            }),
    )
    .insert_resource(ClearColor(Color::srgb(0.52, 0.60, 0.70)))
    .insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.95, 0.95, 1.0),
        brightness: 450.0,
        ..default()
    })
    .insert_resource(Time::<Fixed>::from_hz(STEP_HZ))
    .insert_resource(Library(library))
    .insert_resource(LiveInput::default())
    .insert_resource(Orbit {
        yaw: 0.0,
        pitch: 0.25,
        distance: 4.2,
    })
    .insert_resource(Captures {
        pending: options.screenshots,
        in_flight: Vec::new(),
        exit_after_script: options.exit_after_script,
    })
    .insert_non_send(Sim {
        last: StepReport::default(),
        wolf,
        script,
        step: 0,
    })
    .add_systems(Startup, setup)
    .add_systems(FixedUpdate, simulate)
    .add_systems(
        Update,
        (
            (gather_input, orbit_camera),
            bind_rigs,
            sync_wolf,
            animate,
            follow_camera,
            hud,
            capture,
        )
            .chain(),
    );
    app.run();
    Ok(())
}

fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut library: ResMut<Library>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        FollowCamera,
        Camera3d::default(),
        Transform::from_xyz(0.0, 2.0, 4.0).looking_at(Vec3::new(0.0, 1.2, 0.0), Vec3::Y),
    ));
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
    let mut idle = Animator::new(
        library
            .0
            .clip("a000_000000")
            .expect("idle clip is extracted"),
        false,
    );
    idle.apply_root_motion = false;
    idle.speed = 0.0;
    commands.spawn((
        Wolf,
        WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset("c0000.glb"))),
        Transform::IDENTITY,
        Rig::new(Arc::new(library.0.skeleton.clone())),
        idle,
    ));
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

fn gather_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    gamepads: Query<&Gamepad>,
    orbit: Res<Orbit>,
    mut live: ResMut<LiveInput>,
    mut cursor: Query<&mut CursorOptions>,
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
    let key = |k| keys.pressed(k);
    let mut stick = Vec2::new(
        (key(KeyCode::KeyD) as i32 - key(KeyCode::KeyA) as i32) as f32,
        (key(KeyCode::KeyW) as i32 - key(KeyCode::KeyS) as i32) as f32,
    );
    let mut b = Buttons {
        attack: mouse.pressed(MouseButton::Left),
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
    mut orbit: ResMut<Orbit>,
) {
    let locked = cursor
        .single()
        .is_ok_and(|c| c.grab_mode == CursorGrabMode::Locked);
    if locked {
        orbit.yaw -= motion.delta.x * MOUSE_SENSITIVITY;
        orbit.pitch += motion.delta.y * MOUSE_SENSITIVITY;
    }
    for pad in &gamepads {
        let r = pad.right_stick();
        if r.length() > 0.2 {
            orbit.yaw -= r.x * STICK_CAMERA_SPEED * time.delta_secs();
            orbit.pitch -= r.y * STICK_CAMERA_SPEED * time.delta_secs();
        }
    }
    orbit.pitch = orbit.pitch.clamp(-0.6, 1.2);
}

fn simulate(
    mut sim: NonSendMut<Sim>,
    live: Res<LiveInput>,
    orbit: Res<Orbit>,
    rigs: Query<&Rig, With<Wolf>>,
) {
    // Hold the simulation until the model is on screen, so scripted runs line up with frames.
    if !rigs.single().is_ok_and(|r| r.bound) {
        return;
    }
    let dt = 1.0 / STEP_HZ as f32;
    let frame = match &sim.script {
        Some(frames) => match frames.get(sim.step) {
            Some(f) => {
                // Scripted sticks are camera-relative to the current orbit, like a player's.
                let mut f = *f;
                f.camera_yaw += orbit.yaw;
                f
            }
            None => InputFrame {
                camera_yaw: orbit.yaw,
                ..InputFrame::default()
            },
        },
        None => live.frame,
    };
    let report = sim.wolf.tick(&frame, dt);
    if !report.behavior.script_errors.is_empty() {
        warn!("script errors: {:?}", report.behavior.script_errors);
    }
    if sim.script.is_some() && sim.step.is_multiple_of(30) {
        info!(
            "step {} {} {} t={:.2} pos={:?}",
            sim.step,
            report.behavior.state_path.join("/"),
            report.animation.as_deref().unwrap_or("-"),
            report.anim_time,
            report.position
        );
    }
    sim.last = report;
    sim.step += 1;
}

fn sync_wolf(
    sim: NonSend<Sim>,
    mut library: ResMut<Library>,
    mut q: Query<(&mut Animator, &mut Transform), With<Wolf>>,
) {
    let Ok((mut anim, mut tf)) = q.single_mut() else {
        return;
    };
    let body = &sim.wolf.body;
    tf.translation = Vec3::from_array(body.position);
    tf.rotation = Quat::from_rotation_y(body.yaw);
    let Some(name) = sim.last.animation.as_deref() else {
        return;
    };
    if anim.clip_name() != name {
        match library.0.clip(name) {
            Ok(clip) => {
                anim.play(clip, false);
                anim.apply_root_motion = false;
                anim.speed = 0.0;
            }
            Err(_) => {
                // Not extracted: keep showing the previous clip.
                return;
            }
        }
    }
    anim.time = sim.last.anim_time;
}

fn follow_camera(
    sim: NonSend<Sim>,
    orbit: Res<Orbit>,
    mut cam: Query<&mut Transform, With<FollowCamera>>,
) {
    let Ok(mut tf) = cam.single_mut() else {
        return;
    };
    let target = Vec3::from_array(sim.wolf.body.position) + Vec3::Y * CAMERA_HEIGHT;
    let rot = Quat::from_euler(EulerRot::YXZ, orbit.yaw, -orbit.pitch, 0.0);
    let eye = target + rot * Vec3::new(0.0, 0.0, orbit.distance);
    *tf = Transform::from_translation(eye).looking_at(target, Vec3::Y);
}

fn hud(sim: NonSend<Sim>, mut text: Query<&mut Text, With<Hud>>) {
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    let r = &sim.last;
    let mut flags = r.action_flags.clone();
    flags.retain(|f| [7, 11, 26, 115, 117, 119].contains(f));
    text.0 = format!(
        "{}\nclip {} t={:.2}\npos ({:.2}, {:.2}, {:.2}) yaw {:.0}{}\nbehaviour refs {:?}\nSpEffects {:?}\ncancel windows {:?}\n\
         WASD move, mouse look, LMB attack, RMB guard, Space jump, Shift step/sprint, C crouch, Esc mouse",
        r.behavior.state_path.join(" / "),
        r.animation.as_deref().unwrap_or("-"),
        r.anim_time,
        r.position[0],
        r.position[1],
        r.position[2],
        r.yaw.to_degrees(),
        if r.grounded { "" } else { "  (airborne)" },
        r.behavior_refs,
        r.sp_effects,
        flags,
    );
}

fn capture(
    mut commands: Commands,
    sim: NonSend<Sim>,
    mut cap: ResMut<Captures>,
    rigs: Query<&Rig>,
    mut exit: MessageWriter<AppExit>,
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
    let script_done = sim.script.as_ref().is_some_and(|s| step >= s.len());
    if cap.exit_after_script && script_done && cap.pending.is_empty() && cap.in_flight.is_empty() {
        std::thread::sleep(Duration::from_millis(200));
        exit.write(AppExit::Success);
    }
}

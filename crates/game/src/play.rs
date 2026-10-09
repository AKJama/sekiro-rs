//! `--play`: Wolf against an Ashina soldier on a floor, both driven by the game's own scripts,
//! behaviour graphs and TAE (`sekiro_sim::duel::Duel`).
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
//! Wolf stands at the player start, the soldier 5 m in front of him, both bodies walk on the
//! map's hit collision (`sekiro_sim::collision`), the map renders as in `--map` (see
//! [`crate::map::add_map`]) and the camera is pulled in when a wall is between it and Wolf.

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
use sekiro_sim::{Buttons, InputFrame, StepReport, demo};

use crate::character::{AnimLibrary, Animator, Rig, animate, bind_rigs};
use crate::combat_hud;
use crate::draw_mask::{DrawMask, MaskSource, apply_draw_masks};
use crate::map::{self, MapFocus, MapScene};
use sekiro_sim::collision::CollisionWorld;

pub struct Options {
    pub cache: PathBuf,
    /// Input script to play instead of live input (`builtin` for the demo).
    pub script: Option<String>,
    /// Screenshots to take at given simulation steps of the script: `(step, path)`.
    pub screenshots: Vec<(usize, PathBuf)>,
    /// Exit when the script ends and every screenshot is saved.
    pub exit_after_script: bool,
    /// Let the soldier fight (it always stands there; without this it stays passive).
    pub enemy: bool,
    /// Start locked on to the soldier.
    pub lock_on: bool,
    /// Drive the soldier with the stand-in brain instead of its own AI scripts.
    pub simple_ai: bool,
    /// Play on this exported map (`cache/maps/<id>`) instead of the flat floor.
    pub map: Option<String>,
    /// Player start point on the map.
    pub start: usize,
    /// Place Wolf at `x, y, z` (on the floor at or below `y + 1`) facing `yaw` degrees,
    /// instead of at a start point.
    pub spawn: Option<[f32; 4]>,
}

/// The map's collision, for the camera; absent on the flat floor.
#[derive(Resource)]
struct MapCollision(Arc<CollisionWorld>);

/// Where model GLBs live below the asset root.
#[derive(Resource)]
struct ModelPath(&'static str);

const STEP_HZ: f64 = 60.0;
const CAMERA_HEIGHT: f32 = 1.4;
const MOUSE_SENSITIVITY: f32 = 0.0035;
const STICK_CAMERA_SPEED: f32 = 2.6;
const WOLF: usize = 0;
const SOLDIER: usize = 1;

/// Which simulated character an entity shows (0 Wolf, 1 the soldier).
#[derive(Component)]
struct Actor(usize);

#[derive(Component)]
struct Hud;

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
    let libraries = [
        AnimLibrary::load(&cache, "c0000")?,
        AnimLibrary::load(&cache, "c1010")?,
    ];
    let mut duel =
        Duel::load(&cache, 6.0).map_err(|e| anyhow::anyhow!("loading the simulation: {e}"))?;
    duel.enemy_active = options.enemy;
    if options.simple_ai {
        duel.use_simple_ai();
    }
    let mut on_map = None;
    let mut start_yaw = 0.0;
    if let Some(id) = &options.map {
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
        // The soldier 5 m in front of Wolf, facing him.
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
        // Behind Wolf, so scripted "forward" is the way he faces.
        yaw: start_yaw,
        pitch: 0.25,
        distance: 4.2,
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
    })
    .init_resource::<combat_hud::Gauges>()
    .add_systems(Startup, setup)
    .add_systems(FixedUpdate, simulate)
    .add_systems(
        Update,
        (
            (gather_input, orbit_camera),
            bind_rigs,
            apply_draw_masks,
            sync_actors,
            animate,
            follow_camera,
            hud,
            combat_hud::update,
            capture,
        )
            .chain(),
    );
    if let Some((dir, id, world)) = on_map {
        // The map's sun, ambient light and sky replace the floor setup's.
        map::add_map(
            &mut app,
            &dir,
            MapScene {
                prefix: format!("maps/{id}/"),
                collision: false,
                no_pieces: false,
                hide: Vec::new(),
                objects: false,
                groups: map::GroupMode::Draw,
                markers: false,
            },
        )?;
        app.insert_resource(MapCollision(world));
    }
    app.run();
    Ok(())
}

fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut libraries: ResMut<Libraries>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    model_path: Res<ModelPath>,
    on_map: Option<Res<MapCollision>>,
) {
    commands.spawn((
        FollowCamera,
        Camera3d::default(),
        Transform::from_xyz(0.0, 2.0, 4.0).looking_at(Vec3::new(0.0, 1.2, 0.0), Vec3::Y),
    ));
    combat_hud::spawn(&mut commands);
    let on_map = on_map.is_some();
    if !on_map {
        spawn_floor_stage(&mut commands, &mut meshes, &mut materials);
    }
    let models = [
        format!("{}c0000.glb", model_path.0),
        format!("{}c1010.glb", model_path.0),
    ];
    for actor in [WOLF, SOLDIER] {
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
        if actor == SOLDIER {
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
    sim: NonSend<Sim>,
    mut orbit: ResMut<Orbit>,
) {
    // Locked on, the camera swings behind Wolf toward the target.
    if let Some(t) = sim.duel.player.lock_target {
        let want = sim.duel.player.yaw_to(t);
        let d = sekiro_sim::input::angle_diff(want, orbit.yaw);
        orbit.yaw += d * (6.0 * time.delta_secs()).min(1.0);
        orbit.pitch += (0.18 - orbit.pitch) * (4.0 * time.delta_secs()).min(1.0);
        return;
    }
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
    rigs: Query<&Rig, With<Actor>>,
) {
    // Hold the simulation until the models are on screen, so scripted runs line up with frames.
    if rigs.is_empty() || !rigs.iter().all(|r| r.bound) {
        return;
    }
    let dt = 1.0 / STEP_HZ as f32;
    let mut frame = match &sim.script {
        Some(frames) => {
            // Scripted sticks are camera-relative to the current orbit, like a player's.
            let mut f = frames.get(sim.step).copied().unwrap_or_default();
            f.camera_yaw += orbit.yaw;
            f
        }
        None => live.frame,
    };
    if sim.lock_requested && sim.step == 0 {
        frame.buttons.lock_on = true;
    }
    let report = sim.duel.step(&frame, dt);
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
            ["Wolf", "soldier"][hit.attacker],
            ["Wolf", "soldier"][hit.defender],
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
            "step {step} wolf {} {} at {:.2?}{} | soldier {} {}",
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
    mut libraries: ResMut<Libraries>,
    mut q: Query<(&Actor, &mut Animator, &mut Transform)>,
) {
    for (actor, mut anim, mut tf) in &mut q {
        let i = actor.0;
        let body = if i == WOLF {
            &sim.duel.player.body
        } else {
            &sim.duel.enemy.body
        };
        tf.translation = Vec3::from_array(body.position);
        tf.rotation = Quat::from_rotation_y(body.yaw);
        let report: &StepReport = if i == WOLF {
            &sim.last.player
        } else {
            &sim.last.enemy
        };
        let Some(name) = report.animation.clone() else {
            continue;
        };
        let time = report.anim_time;
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
        anim.time = time;
    }
}

fn follow_camera(
    sim: NonSend<Sim>,
    orbit: Res<Orbit>,
    collision: Option<Res<MapCollision>>,
    mut cam: Query<&mut Transform, With<FollowCamera>>,
) {
    let Ok(mut tf) = cam.single_mut() else {
        return;
    };
    let wolf = Vec3::from_array(sim.duel.player.body.position);
    let mut target = wolf + Vec3::Y * CAMERA_HEIGHT;
    // Locked on, aim between Wolf and the target, biased to Wolf.
    if let Some(t) = sim.duel.player.lock_target {
        target = target.lerp(Vec3::from_array(t) + Vec3::Y * 1.2, 0.3);
    }
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
        enemy_head: Some(Vec3::from_array(sim.duel.enemy.body.position) + Vec3::Y * 2.0),
        enemy_alive: !f[1].dead(),
        deathblow: sim.duel.deathblow_available(),
    };
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    let r = &sim.last;
    let mut flags = r.player.action_flags.clone();
    flags.retain(|f| [3, 7, 11, 26, 115, 117, 119].contains(f));
    let mut s = format!(
        "Wolf  {}  [{} t={:.2}]  HP {}{}\nrefs {:?}  windows {:?}\nSoldier  {}  [{} t={:.2}]  HP {}\n",
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

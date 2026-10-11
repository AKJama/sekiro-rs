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
//! B step/sprint, L3 crouch, L2 grapple, R2 prosthetic, X item, R3 lock-on. Esc opens the pause menu;
//! minus and equals (or `[` and `]`) change the volume (`crate::audio`).
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

use std::path::{Path, PathBuf};
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
#[path = "cinematic_fade.rs"]
mod cinematic_fade;
#[path = "rope.rs"]
mod rope;

pub struct Options {
    pub brain_overlay: bool,
    pub brain_label: String,
    pub window_size: (u32, u32),
    pub attack_power: sekiro_sim::progression::AttackPower,
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
    /// Testing aid (`--god`): Wolf takes no HP or posture damage; hits, reactions and hit stop
    /// are otherwise unchanged.
    pub god: bool,
    pub immortal: bool,
    /// Testing aid (`--force-act <id>`): the enemy requests this action (an animation id such
    /// as 3040, the Owl's grab) whenever it is free, on top of its AI (or alone with `--passive`).
    pub force_act: Option<i32>,
    /// Testing aid (`--break-posture`): the enemy's posture is held at 1 point outside
    /// deathblows, so Wolf's next landed hit breaks it and opens a deathblow.
    pub break_posture: bool,
    /// Who chooses a boss's acts (`--brain`) and whether its decisions are logged
    /// to `cache/brain_logs/` (`--brain-log`); see `docs/BRAIN.md`.
    pub brain: crate::BrainOptions,
    pub brain_log: bool,
    /// The boss brain's memory file across fights and sessions (`--brain-memory`; by default
    /// `logs/memory/<chr>.json` for hosted brains, none for the others).
    pub brain_memory: Option<PathBuf>,
    pub player: String,
    pub coach_interval: f32,
    /// Play on this exported map (`cache/maps/<id>`) instead of the flat floor.
    pub map: Option<String>,
    /// Player start point on the map.
    pub start: usize,
    /// Place Wolf at `x, y, z` (on the floor at or below `y + 1`) facing `yaw` degrees,
    /// instead of at a start point, with the enemy `enemy_distance` in front of him, facing
    /// him. Also works in an arena (a testing aid: put the pair next to a railing).
    pub spawn: Option<[f32; 4]>,
    /// Metres between Wolf and the enemy at a `spawn` or map start (`--enemy-distance`).
    pub enemy_distance: f32,
    /// Time of day (hours) and draw param override for the map lighting.
    pub hour: f32,
    pub gparam: Option<String>,
    /// A boss arena (`--arena owl`): the boss's map, his MSB placement and Wolf's start;
    /// replaces `map` and `start`.
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

#[derive(Resource)]
struct EnemyDrawGroups(Option<[bool; 32]>);

#[derive(Resource)]
struct EnemyScene {
    cache: PathBuf,
    spec: EnemySpec,
}

const STEP_HZ: f64 = 60.0;
/// Seconds the fight waits after the map has loaded. About 1.9 s after the map's textures and
/// meshes are uploaded, presentation stalls once for 50 to 70 ms: the render world waits that
/// long for the next swapchain image (measured on an RTX 5070 Ti with Vulkan, 2026-10-10, in
/// every run with map pieces drawn, in none without, and at the same time when the start was
/// held longer; no CPU work, pipeline compile, sound or clip load lines up with it; STATUS.md,
/// "Frame pacing"). Starting after it keeps that stall out of the fight.
const MAP_SETTLE_SECS: f32 = 2.5;
/// Radians of camera turn per mouse count (0.0035 was reported as far too sensitive).
const MOUSE_SENSITIVITY: f32 = 0.0012;
const WOLF: usize = 0;
const ENEMY: usize = 1;

/// Which simulated character an entity shows (0 Wolf, 1 the enemy).
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
    break_posture: bool,
    passive: bool,
    /// Each actor's position and drawn yaw before the last step, to draw between steps.
    previous: [([f32; 3], f32); 2],
    /// Each actor's own clock after the last step: the sum of the time it advanced, which stops
    /// while a hit stop holds it.
    clock: [f32; 2],
    /// Whether each actor is drawn hidden: faded out by its clip's TAE 193 (the Owl's warps).
    faded: [bool; 2],
    /// Seconds since a perilous-attack warning last reached Wolf, for the kanji.
    perilous_age: Option<f32>,
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
    /// The deathblow camera's look limits as they chase TAE 151.
    look: sekiro_sim::camera::LookLimits,
}

#[derive(Resource)]
struct Captures {
    pending: Vec<(usize, PathBuf)>,
    in_flight: Vec<PathBuf>,
    exit_after_script: bool,
}

pub fn run(options: Options) -> Result<()> {
    let cache = options.cache.canonicalize().context("cache directory")?;
    let roster = sekiro_sim::registry::Registry::load(&cache).map_err(anyhow::Error::msg)?;
    let anim_chr = roster.anim_chr(&options.opponent.chr, options.opponent.npc_param);
    let enemy_draw_groups = roster.draw_mask(&cache, &options.opponent);
    let libraries = [
        AnimLibrary::load(&cache, "c0000")?,
        AnimLibrary::load(&cache, &anim_chr)?,
    ];
    let mut duel = Duel::load_with(&cache, 6.0, options.opponent.clone())
        .map_err(|e| anyhow::anyhow!("loading the simulation: {e}"))?;
    duel.rules
        .set_attack_power(options.attack_power)
        .map_err(anyhow::Error::msg)?;
    duel.enemy_active = options.enemy;
    duel.god = options.god;
    duel.immortal = options.immortal;
    duel.force_act = options.force_act;
    if options.simple_ai {
        duel.use_simple_ai();
    } else {
        let brain = options.brain.build()?;
        let brain_name = brain.name();
        let log = duel
            .set_brain(brain, options.brain_log)
            .map_err(|e| anyhow::anyhow!("--brain: {e}"))?;
        eprintln!(
            "boss brain {}{}",
            brain_name,
            log.map(|p| format!(", log {}", p.display()))
                .unwrap_or_default()
        );
        if let Some(driver) = duel.boss_brain.as_mut() {
            driver.enable_session_report(Path::new("logs"));
        }
        if let Some(driver) = duel.boss_brain.as_mut() {
            driver.player.clone_from(&options.player);
            driver.coach_interval = options.coach_interval;
        }
        if let (Some(path), Some(driver)) = (options.brain_memory, duel.boss_brain.as_mut()) {
            driver.use_player_memory(path, &options.player);
        }
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
        if let Some([x, y, z, yaw_degrees]) = options.spawn {
            start_yaw = yaw_degrees.to_radians();
            place_pair(
                &mut duel,
                Vec3::new(x, y, z),
                start_yaw,
                options.enemy_distance,
                |x, z, near| arena.world.floor(x, z, near + 1.0).unwrap_or(near),
            );
        }
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
        place_pair(
            &mut duel,
            start,
            yaw,
            options.enemy_distance,
            |x, z, near| world.floor(x, z, near + 1.0).unwrap_or(near),
        );
        duel.player.ground = Box::new(world.clone());
        duel.enemy.ground = Box::new(world.clone());
        // Placeholder until the map's authored void return volumes are decoded.
        duel.fall_respawn_depth = Some(40.0);
        eprintln!(
            "{id}: start {} at {:?}, {} collision triangles",
            options.start,
            duel.player.body.position,
            world.triangle_count()
        );
        on_map = Some((dir, id.clone(), world));
        map::raise_io_stack();
    }
    if let Some((dir, _, _)) = &on_map
        && dir.join("edges.json").exists()
    {
        duel.player.docking = sekiro_sim::docking::Docking::load(&dir.join("edges.json"))
            .map_err(anyhow::Error::msg)?;
    }
    if let Some((dir, _, _)) = &on_map
        && dir.join("grapple.json").exists()
    {
        duel.player.wire = sekiro_sim::wire::Wire::load(&cache, dir).map_err(anyhow::Error::msg)?;
    }
    let script = match options.script.as_deref() {
        None => None,
        Some("builtin") => Some(demo::parse(demo::DEMO).map_err(anyhow::Error::msg)?),
        Some(path) => Some(
            sekiro_sim::input::parse_action_script(
                &std::fs::read_to_string(path).with_context(|| path.to_string())?,
            )
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
                    resolution: options.window_size.into(),
                    // Make the first input frame's request a false -> true transition.
                    // Bevy's winit backend only calls focus_window on that transition.
                    focused: false,
                    ..default()
                }),
                // The input system requests focus and cursor lock on the first frame;
                // initial scripted/replay settings remain useful before that system runs.
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
    .insert_resource(EnemyScene {
        cache: cache.clone(),
        spec: duel.spec.clone(),
    })
    .insert_resource(EnemyDrawGroups(enemy_draw_groups))
    .insert_resource(LiveInput::default())
    .insert_resource(Orbit {
        focus: Vec3::from_array(duel.player.body.position) + Vec3::Y * duel.camera.chr_org_offset_y,
        // Behind Wolf, so scripted "forward" is the way he faces.
        yaw: start_yaw,
        pitch: 0.25,
        distance: duel.camera.cam_dist,
        look: Default::default(),
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
        break_posture: options.break_posture,
        passive: !options.enemy,
        previous: [([0.0; 3], 0.0); 2],
        clock: [0.0; 2],
        faded: [false; 2],
        perilous_age: None,
    })
    .init_resource::<combat_hud::Gauges>()
    .insert_resource(crate::brain_overlay::RecordingBrain {
        enabled: options.brain_overlay,
        mode: options.brain_label,
        player: options.player,
        view: Default::default(),
        unicode_font: false,
    })
    .init_resource::<crate::bullet_fx::BulletFx>()
    .add_plugins(crate::fxr::plugin(cache.clone(), |app| {
        app.add_systems(
            FixedUpdate,
            (|sim: NonSend<Sim>, mut fx: ResMut<crate::fxr::FxrPlayer>| fx.observe(&sim.duel))
                .after(simulate)
                .run_if(crate::menu::active),
        );
    }))
    .add_plugins(cinematic_fade::CinematicFadePlugin)
    .add_plugins(crate::audio::GameAudioPlugin {
        cache: cache.clone(),
        map: on_map.as_ref().map(|(_, id, _)| id.clone()),
        enemy: Some((options.opponent.chr.clone(), options.opponent.npc_param)),
    })
    .add_systems(Startup, setup)
    .insert_resource(rope::RopeCache(cache.clone()))
    .add_systems(Startup, crate::brain_overlay::spawn)
    .add_systems(FixedUpdate, simulate.run_if(crate::menu::active))
    .add_systems(
        Update,
        (
            // The camera turns first, so the stick read this frame is relative to the camera
            // that is drawn.
            orbit_camera.run_if(crate::menu::active),
            gather_input.run_if(crate::menu::active),
            brain_report,
            recording_brain,
            reload_enemy,
            bind_rigs,
            apply_draw_masks,
            sync_actors,
            sync_bullets,
            sync_lightning_tint,
            (rope::pose, animate, rope::draw, sheath_sword).chain(),
            follow_camera,
            crate::brain_overlay::update,
            hud,
            grapple_marker,
            combat_hud::update,
            combat_hud::update_status,
            combat_hud::update_perilous,
            capture,
            log_camera,
        )
            .chain(),
    );
    crate::menu::install(&mut app);
    crate::perf_log::register!(app);
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
fn reload_enemy(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut scene: ResMut<EnemyScene>,
    mut libraries: ResMut<Libraries>,
    model_path: Res<ModelPath>,
    sim: NonSend<Sim>,
    actors: Query<(Entity, &Actor)>,
) {
    if scene.spec == sim.duel.spec {
        return;
    }
    let Ok(roster) = sekiro_sim::registry::Registry::load(&scene.cache) else {
        return;
    };
    let donor = roster.anim_chr(&sim.duel.spec.chr, sim.duel.spec.npc_param);
    let Ok(mut lib) = AnimLibrary::load(&scene.cache, &donor) else {
        return;
    };
    let Ok(clip) = lib.clip("a000_000000") else {
        return;
    };
    let mut idle = Animator::new(clip, false);
    idle.apply_root_motion = false;
    idle.speed = 0.0;
    for (entity, actor) in &actors {
        if actor.0 == ENEMY {
            commands.entity(entity).despawn();
        }
    }
    let mask = roster
        .draw_mask(&scene.cache, &sim.duel.spec)
        .map_or_else(|| DrawMask::new(MaskSource::Combat), DrawMask::with_groups);
    commands.spawn((
        Actor(ENEMY),
        WorldAssetRoot(
            assets.load(
                GltfAssetLabel::Scene(0)
                    .from_asset(format!("{}{}.glb", model_path.0, sim.duel.spec.chr)),
            ),
        ),
        Transform::IDENTITY,
        Visibility::Inherited,
        Rig::new(Arc::new(lib.skeleton.clone())),
        idle,
        mask,
    ));
    libraries.0[ENEMY] = lib;
    scene.spec = sim.duel.spec.clone();
}

#[allow(clippy::too_many_arguments)]
fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut libraries: ResMut<Libraries>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    model_path: Res<ModelPath>,
    enemy_draw_groups: Res<EnemyDrawGroups>,
    on_map: Option<Res<MapCollision>>,
    sim: NonSend<Sim>,
    rope_cache: Res<rope::RopeCache>,
) {
    rope::spawn(&mut commands, &assets, &rope_cache.0, model_path.0);
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
    commands.spawn((
        GrappleMarker,
        Node {
            position_type: PositionType::Absolute,
            width: px(18),
            height: px(18),
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::all(px(9)),
            display: Display::None,
            ..default()
        },
        BorderColor::all(Color::srgb(0.2, 1.0, 0.4)),
        GlobalZIndex(20),
    ));
    // Bullets have no model yet (their look is an effect, FXR, not extracted): simple shapes.
    let fx = crate::bullet_fx::spawn(&mut commands, &mut meshes, &mut materials);
    commands.insert_resource(fx);
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
            Visibility::Inherited,
            Rig::new(Arc::new(lib.skeleton.clone())),
            idle,
        ));
        if actor == ENEMY {
            let groups = enemy_draw_groups.0;
            e.insert(
                groups.map_or_else(|| DrawMask::new(MaskSource::Combat), DrawMask::with_groups),
            );
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
/// Puts Wolf at `start` facing `yaw` and the enemy `distance` metres in front of him, facing
/// him, both on the floor `floor_at(x, z, near_y)` finds.
fn place_pair(
    duel: &mut Duel,
    start: Vec3,
    yaw: f32,
    distance: f32,
    floor_at: impl Fn(f32, f32, f32) -> f32,
) {
    let wolf = &mut duel.player.body;
    wolf.position = [start.x, floor_at(start.x, start.z, start.y), start.z];
    wolf.yaw = yaw;
    wolf.fall_top = wolf.position[1];
    let ahead = Quat::from_rotation_y(yaw) * Vec3::NEG_Z * distance;
    let (ex, ez) = (start.x + ahead.x, start.z + ahead.z);
    let enemy = &mut duel.enemy.body;
    enemy.position = [ex, floor_at(ex, ez, start.y), ez];
    enemy.yaw = yaw + std::f32::consts::PI;
    enemy.fall_top = enemy.position[1];
}

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

#[derive(Default)]
struct MouseCapture {
    focus_requested: bool,
    first_click_grabbed: bool,
    grab_click: bool,
}

#[allow(clippy::too_many_arguments)]
fn gather_input(
    fade: Res<cinematic_fade::Fade>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    gamepads: Query<&Gamepad>,
    orbit: Res<Orbit>,
    mut live: ResMut<LiveInput>,
    mut windows: Query<(&mut Window, &mut CursorOptions)>,
    mut capture: Local<MouseCapture>,
) {
    if fade.alpha > 0.0 {
        live.frame = InputFrame::default();
        return;
    }
    if !capture.focus_requested
        && let Ok((mut window, mut c)) = windows.single_mut()
    {
        window.focused = true;
        c.grab_mode = CursorGrabMode::Locked;
        c.visible = false;
        capture.focus_requested = true;
    }
    // A click while the cursor is free (after Esc) takes the mouse back for the camera; that
    // click is not an attack.
    if !mouse.pressed(MouseButton::Left) {
        capture.grab_click = false;
    }
    if mouse.just_pressed(MouseButton::Left)
        && let Ok((window, mut c)) = windows.single_mut()
        && window.focused
        && window.cursor_position().is_some()
        && (!capture.first_click_grabbed || c.grab_mode != CursorGrabMode::Locked)
    {
        c.grab_mode = CursorGrabMode::Locked;
        c.visible = false;
        capture.first_click_grabbed = true;
        capture.grab_click = true;
    }
    let key = |k| keys.pressed(k);
    let mut stick = Vec2::new(
        (key(KeyCode::KeyD) as i32 - key(KeyCode::KeyA) as i32) as f32,
        (key(KeyCode::KeyW) as i32 - key(KeyCode::KeyS) as i32) as f32,
    );
    let mut b = Buttons {
        attack: mouse.pressed(MouseButton::Left) && !capture.grab_click,
        guard: mouse.pressed(MouseButton::Right),
        jump: key(KeyCode::Space),
        dodge: key(KeyCode::ShiftLeft) || key(KeyCode::ShiftRight),
        crouch: key(KeyCode::KeyC),
        grapple: key(KeyCode::KeyF),
        action: key(KeyCode::KeyE),
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
        b.action |= pad.pressed(GamepadButton::North);
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
    fade: Res<cinematic_fade::Fade>,
    time: Res<Time>,
    motion: Res<AccumulatedMouseMotion>,
    cursor: Query<&CursorOptions>,
    gamepads: Query<&Gamepad>,
    sim: NonSend<Sim>,
    mut orbit: ResMut<Orbit>,
) {
    if fade.alpha > 0.0 {
        return;
    }
    // Locked on, the camera swings behind Wolf toward the target. The rates are the camera
    // slot's (TAE 155: CameraParam 4000 in deathblows).
    let director = &sim.duel.camera_director;
    let params = &sekiro_sim::camera::CameraParams {
        // The lock-on pitch frames the target from where the camera is (TAE 153 brings it close
        // in deathblows), as FwLo's port of the exe's formula does.
        cam_dist: director.distance,
        ..director.params()
    };
    let dt = time.delta_secs();
    clamp_to_look_at(&sim, &mut orbit, dt);
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
    mut fade: ResMut<cinematic_fade::Fade>,
    live: Res<LiveInput>,
    mut orbit: ResMut<Orbit>,
    rigs: Query<&Rig, With<Actor>>,
    mut sounds: ResMut<crate::audio::SoundLibrary>,
    mut cues: MessageWriter<crate::audio::SoundCue>,
    map_pending: Option<Res<map::Pending>>,
    replay: Option<ResMut<crate::input_replay::InputReplay>>,
) {
    // Hold the simulation until the models (and the map) are on screen, so scripted runs line
    // up with frames.
    // The start also waits until the map has been drawn for MAP_SETTLE_SECS, past the one-off
    // presentation stall that follows the map's upload.
    if rigs.is_empty()
        || !rigs.iter().all(|r| r.bound)
        || map_pending
            .as_ref()
            .is_some_and(|p| !p.loaded() || (sim.step == 0 && p.loaded_for() < MAP_SETTLE_SECS))
    {
        return;
    }
    if sim
        .duel
        .cinematic
        .as_ref()
        .is_some_and(|c| !c.applied && c.elapsed >= sekiro_sim::events::Cinematic::FADE)
        && fade.black_frames < 2
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
    if sim.break_posture && sim.duel.deathblow.is_none() && sim.duel.cinematic.is_none() {
        let posture = &mut sim.duel.rules.fighters[1].posture;
        posture.remaining = posture.remaining.min(1);
    }
    let before = sim.duel.player.body.position;
    if sim.passive {
        sim.duel.enemy_active = false;
    }
    sim.duel.player.wire.view_pitch = orbit.pitch;
    let cinematic_applied = sim.duel.cinematic.as_ref().is_some_and(|c| c.applied);
    let report = sim.duel.step(&frame, dt);
    fade.alpha = sim.duel.cinematic.as_ref().map_or(0.0, |c| c.alpha());
    if !cinematic_applied && sim.duel.cinematic.as_ref().is_some_and(|c| c.applied) {
        orbit.focus = Vec3::from_array(sim.duel.player.body.position)
            + Vec3::Y * sim.duel.camera_director.focus_height;
        orbit.yaw = sim.duel.player.body.yaw;
        orbit.pitch = 0.25;
        orbit.look = Default::default();
        sim.previous = [
            (sim.duel.player.body.position, sim.duel.player.visual_yaw()),
            (sim.duel.enemy.body.position, sim.duel.enemy.visual_yaw()),
        ];
    }
    if std::env::var_os("PLAY_LOG_COMBO").is_some()
        && sim.step.is_multiple_of(15)
        && let Some(c) = &sim.duel.cinematic
    {
        eprintln!(
            "cinematic step={} elapsed={:.3} alpha={:.3} applied={} boss={}",
            sim.step,
            c.elapsed,
            c.alpha(),
            c.applied,
            sim.duel.spec.chr
        );
    }
    if std::env::var_os("PLAY_LOG_WIRE").is_some() {
        let wolf = &sim.duel.player;
        let target = wolf.wire.active.as_ref().or(wolf.wire.candidate.as_ref());
        eprintln!(
            "WIRE step={} env3000={} pos={:?} target={:?} arrival={:?} distance={:?} anim={:?} events={:?} precompute={:?} flight={:?} grounded={}",
            sim.step,
            wolf.behavior.env.wire_candidate,
            wolf.body.position,
            target.map(|t| &wolf.wire.points[t.index].source_part),
            target.map(|t| t.arrival.to_array()),
            target.map(|t| Vec3::from_array(wolf.body.position).distance(t.arrival)),
            report.player.animation,
            report.player.behavior.events,
            wolf.behavior.env.wire_precompute.get(),
            wolf.wire.flight_time(),
            wolf.body.grounded,
        );
        if report.player.clip_started
            && let Some(anim) = report
                .player
                .animation
                .as_ref()
                .and_then(|a| wolf.tae.anim(a))
        {
            for e in anim.events.iter() {
                if matches!(e.kind, sekiro_sim::tae::TaeKind::Wire { .. }) {
                    eprintln!("WIRE TAE {e:?}");
                }
            }
        }
    }
    if std::env::var_os("PLAY_LOG_DOCKING").is_some() {
        let wolf = &sim.duel.player;
        let target = wolf
            .docking
            .active
            .as_ref()
            .or(wolf.docking.candidate.as_ref());
        eprintln!(
            "DOCK step={} env3044={} env3050={} env3043={} end={} blocked={:?} pos={:?} edge={:?} point={:?} anim={:?} events={:?}",
            sim.step,
            wolf.behavior.env.hang_candidate && wolf.behavior.env.docking_searches.get() & 2 != 0,
            wolf.behavior.env.hang_can_climb,
            wolf.behavior.env.cover_candidate && wolf.behavior.env.docking_searches.get() & 1 != 0,
            wolf.behavior.env.docking_end,
            wolf.behavior.env.docking_blocked,
            wolf.body.position,
            target.map(|t| (
                &wolf.docking.edges[t.edge].source_part,
                wolf.docking.edges[t.edge].chain
            )),
            target.map(|t| t.point.to_array()),
            report.player.animation,
            report.player.behavior.events
        );
    }
    if report.respawned {
        // A respawn teleports the camera with Wolf instead of chasing up from the ravine.
        orbit.focus = Vec3::from_array(sim.duel.player.body.position)
            + Vec3::Y * sim.duel.camera_director.focus_height;
        orbit.yaw = sim.duel.player.body.yaw;
        orbit.pitch = 0.25;
        orbit.look = Default::default();
    }
    sim.clock[0] += report.player.advanced;
    sim.clock[1] += report.enemy.advanced;
    if log_enabled() {
        log_move(sim.step, &frame, before, &sim.duel, &report.player);
    }
    if std::env::var_os("PLAY_LOG_COMBO").is_some() {
        log_combo(sim.step, &frame, &sim.duel, &report);
    }
    crate::audio::emit(&sim.duel, &report, &mut sounds, &mut cues);
    for (i, r) in [&report.player, &report.enemy].into_iter().enumerate() {
        if !r.behavior.script_errors.is_empty() {
            warn!("script errors ({i}): {:?}", r.behavior.script_errors);
        }
        if (r.clip_started || report.respawned)
            && let Some(name) = &r.animation
        {
            // Teleports start a fresh pose, without blending from the falling death clip.
            let blend = if report.respawned { 0.0 } else { r.blend_in };
            sim.started[i] = Some((name.clone(), blend));
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
            "step {step} wolf {} {} at {:.2?}{} | enemy {} {} | fall_cm {:.1} vy_cm_s {:.1} respawn {}",
            report.player.behavior.state_path.join("/"),
            report.player.animation.as_deref().unwrap_or("-"),
            report.player.position,
            if report.player.grounded { "" } else { " AIR" },
            report.enemy.behavior.state_path.join("/"),
            report.enemy.animation.as_deref().unwrap_or("-"),
            sim.duel.player.behavior.env.fall_height,
            sim.duel.player.behavior.env.vertical_velocity,
            report.respawned,
        );
    }
    sim.perilous_age = match (&report.perilous, sim.perilous_age) {
        (Some(w), _) => {
            info!("perilous warning: judge {} {:?}", w.judge, w.kind);
            Some(0.0)
        }
        (None, age) => age.map(|a| a + dt),
    };
    if report.jump_counter {
        info!("jump counter: the kick landed on a perilous sweep");
    }
    sim.last = report;
    sim.step += 1;
}

fn sync_actors(
    mut sim: NonSendMut<Sim>,
    fixed: Res<Time<Fixed>>,
    mut libraries: ResMut<Libraries>,
    mut fx: ResMut<crate::bullet_fx::BulletFx>,
    mut q: Query<(&Actor, &mut Animator, &mut Transform, &mut Visibility)>,
) {
    for (actor, mut anim, mut tf, mut vis) in &mut q {
        let i = actor.0;
        let (position, yaw) = shown(&sim, i, fixed.overstep_fraction());
        tf.translation = position;
        tf.rotation = Quat::from_rotation_y(yaw);
        // TAE 193 fades the model (and the weapon in it) out and back in; drawn hidden past
        // half, with a puff where it goes and where it comes back (stand-ins for the game's
        // warp effects, FFX 650680 / 650682).
        let fade = if i == WOLF {
            sim.last.player.fade
        } else {
            sim.last.enemy.fade
        };
        let faded = fade >= 0.5;
        if faded != sim.faded[i] {
            sim.faded[i] = faded;
            fx.puff(position + Vec3::Y * 1.0);
        }
        *vis = if faded {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        anim.walk_twist = if i == WOLF {
            sim.duel.player.walk_twist
        } else {
            sim.duel.enemy.walk_twist
        };
        let report: &StepReport = if i == WOLF {
            &sim.last.player
        } else {
            &sim.last.enemy
        };
        let Some(name) = report.animation.clone() else {
            continue;
        };
        if sim.step > 0 {
            // The fight runs: decode the rest of the clips in the background.
            libraries.0[i].prefetch_all();
        }
        let alpha = fixed.overstep_fraction();
        anim.clock = Some(sim.clock[i] - behind(report, alpha));
        let time = drawn_clip_time(report, alpha);
        let blend_in = report.blend_in;
        let started = sim.started[i].take();
        let restart = started.as_ref().is_some_and(|(n, _)| *n == name);
        if anim.clip_name() != name || restart {
            // A previously active layered clip can become the main clip without restarting.
            // Use its own TAE, including when several fixed ticks preceded this draw.
            let blend = started
                .filter(|(n, _)| *n == name)
                .map_or(blend_in, |(_, b)| b);
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

/// How far a frame drawn `alpha` of the way into the next step trails the last step on the
/// actor's own clock: the part of the step's advance not yet reached. A frozen actor advanced
/// nothing, so it holds exactly still; while a hit stop eases out the advance is short, and so
/// is this.
fn behind(report: &StepReport, alpha: f32) -> f32 {
    (1.0 - alpha) * report.advanced
}

/// The full-body clip time drawn `alpha` of the way into the next step, matching the drawn
/// position. A clip that started in the last step is drawn from its start.
fn drawn_clip_time(report: &StepReport, alpha: f32) -> f32 {
    (report.anim_time - behind(report, alpha)).max(0.0)
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

/// Draws the bullets in flight and their areas (see [`crate::bullet_fx`]).
fn sync_lightning_tint(
    sim: NonSend<Sim>,
    roots: Query<&Actor>,
    parents: Query<&ChildOf>,
    parts: Query<(Entity, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut original: Local<std::collections::HashMap<AssetId<StandardMaterial>, (Color, LinearRgba)>>,
) {
    for (entity, material) in &parts {
        let mut current = entity;
        let mut actor = None;
        for _ in 0..32 {
            if let Ok(root) = roots.get(current) {
                actor = Some(root.0);
                break;
            }
            let Ok(parent) = parents.get(current) else {
                break;
            };
            current = parent.parent();
        }
        let Some(i) = actor else {
            continue;
        };
        let Some(mut mat) = materials.get_mut(&material.0) else {
            continue;
        };
        let saved = original
            .entry(material.0.id())
            .or_insert((mat.base_color, mat.emissive));
        let status = &sim.duel.rules.fighters[i].status;
        if status.has(9445) || status.has(9492) {
            mat.base_color = Color::srgb(0.25, 0.65, 1.0);
            mat.emissive = LinearRgba::rgb(0.15, 0.4, 0.9);
        } else {
            (mat.base_color, mat.emissive) = *saved;
        }
    }
}

/// Draws the bullets in flight and their areas (see [`crate::bullet_fx`]).
fn sync_bullets(
    sim: NonSend<Sim>,
    time: Res<Time>,
    mut fx: ResMut<crate::bullet_fx::BulletFx>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(&crate::bullet_fx::Marker, &mut Transform, &mut Visibility)>,
    mut gizmos: Gizmos,
    mut lightning: Local<crate::lightning_fx::LightningFx>,
) {
    crate::lightning_fx::draw(
        &sim.duel,
        time.elapsed_secs(),
        time.delta_secs(),
        &mut lightning,
        &mut gizmos,
    );
    crate::bullet_fx::sync(
        &sim.duel.bullets.live,
        time.delta_secs(),
        &mut fx,
        &mut materials,
        &mut q,
    );
}

/// TAE 151 (deathblows): the camera's pitch and yaw stay within the event's limits around the
/// direction from Wolf to his camera dummy ([`sekiro_sim::camera::CameraDirector::clamp_look`]).
fn clamp_to_look_at(sim: &Sim, orbit: &mut Orbit, dt: f32) {
    let director = &sim.duel.camera_director;
    let Some(look) = director.look_at else {
        orbit.look = Default::default();
        return;
    };
    let Some(dummy) = i16::try_from(look.dummy)
        .ok()
        .and_then(|d| sim.duel.dummy_position(WOLF, d))
    else {
        return;
    };
    // From Wolf's body centre, as FwLo's RustSekiro measures it (his capsule centre, 0.9 m above
    // the feet); from the feet the dummy (about 1.1 m up and 0.4 m ahead in the Owl's deathblow)
    // would centre the limits far below and turn the camera up from under the pair.
    let centre = Vec3::from_array(sim.duel.player.body.position) + Vec3::Y * 0.9;
    let dir = Vec3::from_array(dummy) - centre;
    let rate = director.params().lock_cam_param_lerp_rate;
    let (yaw, pitch) = orbit
        .look
        .clamp(Some(look), rate, orbit.yaw, orbit.pitch, dir, dt);
    orbit.yaw = yaw;
    orbit.pitch = pitch;
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
    // Scripted inspection angle: `--extra hang-view`, parsed here to avoid main.rs.
    if std::env::args().any(|a| a == "hang-view")
        && let Some(target) = &sim.duel.player.docking.active
    {
        orbit.yaw = target.yaw() - 30.0f32.to_radians();
        orbit.pitch = 0.1;
    }
    let director = &sim.duel.camera_director;
    let params = director.params();
    let mut wanted = wolf + Vec3::Y * director.focus_height;
    let mut xz = sekiro_sim::camera::chase(params.chr_trans_chase_rate_xz, time.delta_secs());
    let mut y = sekiro_sim::camera::chase(params.chr_trans_chase_rate_y, time.delta_secs());
    // TAE 151 (Wolf's deathblow clips): the focus chases his camera dummy, at the event's rate
    // on every axis when it gives one (FwLo's trace, `docs/kb/camera.md`).
    if let Some(look) = director.look_at
        && let Some(dummy) = i16::try_from(look.dummy)
            .ok()
            .and_then(|d| sim.duel.dummy_position(WOLF, d))
    {
        wanted = Vec3::from_array(dummy);
        if look.rate > 0.0 {
            xz = sekiro_sim::camera::chase(look.rate.min(1.0), time.delta_secs());
            y = xz;
        }
    }
    // TAE 153: the distance the animation asks for.
    orbit.distance = director.distance;
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

/// Names of the status effects the Owl's tools leave on Wolf, for the HUD.
const STATUS_NAMES: [(i32, &str); 5] = [
    (9445, "Electrified"),
    (9492, "Lightning Reversal ready"),
    (9050, "Poisoned"),
    (3506090, "Recovery sealed"),
    (113000, "Posture recovery slowed"),
];

/// Wolf's status line: each named effect with its seconds left, and blinding while it plays.
fn status_line(status: &sekiro_sim::status::Status, states: &[String]) -> String {
    let mut parts: Vec<String> = STATUS_NAMES
        .iter()
        .filter_map(|(id, name)| status.remaining(*id).map(|s| format!("{name} {s:.0} s")))
        .collect();
    if states.iter().any(|s| s.contains("Blinding")) {
        parts.push("Blinded".into());
    }
    parts.join("   ")
}

fn hud(
    sim: NonSend<Sim>,
    recording: Res<crate::brain_overlay::RecordingBrain>,
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
        // Hidden with him while a warp fades him out.
        enemy_alive: !f[1].dead() && sim.duel.enemy_bar_shown() && !sim.faded[1],
        boss_bar: sim
            .duel
            .events
            .as_ref()
            .is_some_and(|events| events.boss_bar.is_some()),
        deathblow: sim.duel.deathblow_available(),
        enemy_name: sim.duel.spec.display_name(),
        enemy_marks: sekiro_sim::deathblow::deathblow_marks(f[1].vitality.deathblows_left),
        poison: f[0].status.poison.fraction(),
        status: status_line(&f[0].status, &sim.last.player.behavior.state_path),
        // Lifted from the chest dummy to above Wolf's head (ours; the effect's own offset is in
        // its undecoded FXR).
        perilous_at: sim
            .duel
            .dummy_position(WOLF, sekiro_sim::perilous::KANJI_DUMMY)
            .map(|p| Vec3::from_array(p) + Vec3::Y * 0.6),
        perilous_age: sim.perilous_age,
    };
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    if recording.enabled {
        text.0.clear();
        return;
    }
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
    match sim.duel.player.behavior.env.action_guide.get() {
        Some((10, 12)) => s += "E / Y: Hug wall\n",
        Some((11, 12)) => s += "E / Y: Leave wall\n",
        Some((20, 12)) => s += "E / Y: Hang\n",
        Some((21, 12) | (22, 5)) => {
            if sim.duel.player.behavior.env.hang_can_climb {
                s += "E / Y: Climb up   ";
            }
            s += "Shift / B: Drop\n";
        }
        _ => {}
    }
    if sim.duel.player.behavior.env.in_water {
        s += "C / L3: Dive or descend   Space / A: Rise   Shift / B: Swim kick\n";
    }
    s += "WASD move, mouse look, LMB attack, RMB guard, Space jump, Shift step/sprint, C crouch, Q lock-on, Esc pause, -/= volume";
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
        "move {step} cam {:.1} stick {:.2},{:.2} stick_yaw {:.1} face {:.1} travel {:.1} speed {:.2} err {:.1} MoveAngle {:.1} walk_twist {:.1} lock {} {} {}",
        frame.camera_yaw.to_degrees(),
        frame.move_stick[0],
        frame.move_stick[1],
        stick.map_or(f32::NAN, f32::to_degrees),
        r.yaw.to_degrees(),
        travel.to_degrees(),
        speed,
        err,
        var("MoveAngle"),
        wolf.walk_twist,
        wolf.lock_target.is_some(),
        r.behavior.state_path.last().map_or("-", String::as_str),
        r.animation.as_deref().unwrap_or("-"),
    );
}

/// One `combo` line per simulation step with `PLAY_LOG_COMBO` set: the attack button, Wolf's
/// buffered presses, state, clip and clip time, both hit stops, this step's hits and the
/// enemy's state and clip.
fn log_combo(step: usize, frame: &InputFrame, duel: &Duel, r: &DuelReport) {
    let mut pending: Vec<i32> = duel.player.input.pending_actions().collect();
    pending.sort_unstable();
    let hits: Vec<String> = r
        .hits
        .iter()
        .map(|(h, res)| {
            let kind = if res.deflected {
                "deflect"
            } else if res.guarded {
                "guard"
            } else {
                "hit"
            };
            format!("{}>{}:{}:{kind}", h.attacker, h.defender, h.attack.atk_id)
        })
        .collect();
    let last = |s: &StepReport| s.behavior.state_path.last().cloned().unwrap_or_default();
    let (a, b) = (r.player.position, r.enemy.position);
    eprintln!(
        "combo {step} atk {} pend {pending:?} wolf {} {} {:.3} stop {:.3},{:.3} hits [{}] owl {} {} {:.3} dist {:.2}",
        frame.buttons.attack as u8,
        last(&r.player),
        r.player.animation.as_deref().unwrap_or("-"),
        r.player.anim_time,
        duel.hit_stop[0].remaining(),
        duel.hit_stop[1].remaining(),
        hits.join(" "),
        last(&r.enemy),
        r.enemy.animation.as_deref().unwrap_or("-"),
        r.enemy.anim_time,
        ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt(),
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

fn brain_report(keys: Res<ButtonInput<KeyCode>>, sim: NonSend<Sim>) {
    if keys.just_pressed(KeyCode::F9)
        && let Some(driver) = sim.duel.boss_brain.as_ref()
        && let Err(e) = driver.print_session_report()
    {
        eprintln!("brain session report: {e}");
    }
}

fn recording_brain(sim: NonSend<Sim>, mut brain: ResMut<crate::brain_overlay::RecordingBrain>) {
    if let Some(driver) = sim.duel.boss_brain.as_ref() {
        brain.view = driver.overlay();
    }
}

#[derive(Component)]
struct GrappleMarker;

/// TAE 720's right-hand sheath window frees both hands for docking.
fn sheath_sword(
    sim: NonSend<Sim>,
    actors: Query<(Entity, &Actor)>,
    children: Query<&Children>,
    names: Query<&Name>,
    globals: Query<&GlobalTransform>,
    mut attachments: Query<(&ChildOf, &mut Transform)>,
    mut original: Local<std::collections::HashMap<Entity, Transform>>,
) {
    let Some((root, _)) = actors.iter().find(|(_, a)| a.0 == WOLF) else {
        return;
    };
    let descendants: Vec<_> = children.iter_descendants(root).collect();
    let find = |name: &str| {
        descendants
            .iter()
            .copied()
            .find(|e| names.get(*e).is_ok_and(|n| n.as_str() == name))
    };
    let Some(blade) = find("dmyblade:1") else {
        return;
    };
    let Some(sheath) = find("dmy:147") else {
        return;
    };
    let Ok((parent, mut local)) = attachments.get_mut(blade) else {
        return;
    };
    let saved = *original.entry(blade).or_insert(*local);
    let report = &sim.last.player;
    // Cover's TAE has no 720 command: docking retains the sheath across these clips.
    let sheathed = sim
        .duel
        .player
        .docking
        .active
        .as_ref()
        .is_some_and(|t| sim.duel.player.docking.is_cover(t))
        || report
            .animation
            .as_ref()
            .and_then(|n| sim.duel.player.tae.anim(n))
            .is_some_and(|a| {
                a.events.iter().any(|e| {
                    e.start <= report.anim_time
                        && report.anim_time < e.end
                        && matches!(&e.kind, sekiro_sim::tae::TaeKind::Wire { kind: 720, args }
                if args.get(..2) == Some(&[0, 1]))
                })
            });
    if sheathed {
        if let (Ok(parent), Ok(sheath)) = (globals.get(parent.parent()), globals.get(sheath)) {
            *local =
                Transform::from_matrix(Mat4::from(parent.affine().inverse() * sheath.affine()));
        }
    } else {
        *local = saved;
    }
}

fn grapple_marker(
    sim: NonSend<Sim>,
    cameras: Query<(&Camera, &GlobalTransform), With<FollowCamera>>,
    mut markers: Query<(&mut Node, &mut BorderColor), With<GrappleMarker>>,
) {
    let Ok((mut node, mut color)) = markers.single_mut() else {
        return;
    };
    node.display = Display::None;
    let Some(target) = sim.duel.player.wire.candidate.as_ref() else {
        return;
    };
    let Ok((camera, transform)) = cameras.single() else {
        return;
    };
    let Ok(p) = camera.world_to_viewport(transform, Vec3::from_array(target.point.to_array()))
    else {
        return;
    };
    node.display = Display::Flex;
    node.left = px(p.x - 9.0);
    node.top = px(p.y - 9.0);
    *color = BorderColor::all(if target.rideable {
        Color::srgb(0.2, 1.0, 0.4)
    } else {
        Color::srgb(1.0, 0.7, 0.2)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use sekiro_sim::fight::FighterKind;

    /// Drawn frames between steps never run a clip backwards, hold exactly still while a hit stop
    /// holds the character, and resume from where they stopped. The soldier's row is set to the
    /// full hit stop types (his own are 0, as the Owl's), so Wolf's slash freezes both.
    #[test]
    fn drawn_clip_time_holds_through_hit_stop() {
        let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../cache");
        if !cache.join("raw/chr/c1010.behbnd.d/c9997.hkx").exists() {
            eprintln!("skipping: cache not extracted");
            return;
        }
        let mut duel = Duel::load(&cache, 4.0).unwrap();
        duel.use_simple_ai();
        duel.enemy_active = false;
        let FighterKind::Npc(npc) = &mut duel.rules.fighters[1].kind else {
            panic!("the enemy is an NPC")
        };
        npc.hit_stop_type = 2;
        npc.hit_stop_type_defender = 1;
        let dt = 1.0 / STEP_HZ as f32;
        let input = |i: usize| match i {
            0..=90 => InputFrame {
                move_stick: [0.0, 1.0],
                ..InputFrame::default()
            },
            120 => InputFrame {
                buttons: Buttons {
                    attack: true,
                    ..Buttons::default()
                },
                ..InputFrame::default()
            },
            _ => InputFrame::default(),
        };
        let mut last: [Option<(String, f32)>; 2] = [None, None];
        let mut held_frames = [0; 2];
        let mut hit = false;
        for i in 0..260 {
            let report = duel.step(&input(i), dt);
            hit |= report.hits.iter().any(|(h, _)| h.attacker == 0);
            for (side, r) in [&report.player, &report.enemy].into_iter().enumerate() {
                let Some(name) = r.animation.clone() else {
                    continue;
                };
                let held = duel.hit_stop[side].active() && r.advanced == 0.0;
                for alpha in [0.0, 0.25, 0.5, 0.75, 0.999] {
                    let t = drawn_clip_time(r, alpha);
                    if let Some((last_name, last_t)) = &last[side]
                        && *last_name == name
                        && !r.clip_started
                    {
                        // Float rounding of `time - advanced` aside.
                        assert!(
                            t >= *last_t - 1e-6,
                            "side {side} step {i}: {t} after {last_t}"
                        );
                    }
                    if held {
                        assert_eq!(t, r.anim_time, "side {side} moved while held at step {i}");
                    }
                    last[side] = Some((name.clone(), t));
                }
                held_frames[side] += usize::from(held);
            }
        }
        assert!(hit, "Wolf's slash never connected");
        assert!(
            held_frames[0] >= 3 && held_frames[1] >= 3,
            "{held_frames:?}"
        );
    }
}

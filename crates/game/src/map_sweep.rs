//! Offscreen, deterministic visual coverage measurements using the shared map renderer.
use super::*;
use bevy::app::ScheduleRunnerPlugin;
use bevy::asset::{embedded_asset, embedded_path};
use bevy::camera::{RenderTarget, visibility::RenderLayers};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::render::render_resource::AsBindGroup;
use bevy::render::view::screenshot::ScreenshotCaptured;
use bevy::shader::ShaderRef;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Resource, Clone, Default)]
struct Pipelines(Arc<(AtomicUsize, AtomicUsize)>);

#[derive(Clone, Asset, TypePath, AsBindGroup)]
struct Diagnostic {
    #[uniform(100)]
    bad: Vec4,
}
impl MaterialExtension for Diagnostic {
    fn fragment_shader() -> ShaderRef {
        bevy::asset::AssetPath::from_path_buf(embedded_path!("map_sweep.wgsl"))
            .with_source("embedded")
            .into()
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
}
type DiagnosticMaterial = ExtendedMaterial<StandardMaterial, Diagnostic>;

#[derive(Clone, Debug)]
struct View {
    eye: Vec3,
    yaw: f32,
    pitch: f32,
    edge: bool,
}
#[derive(Clone)]
struct Options {
    map: String,
    spacing: f32,
    yaws: usize,
    pitches: Vec<f32>,
    width: u32,
    height: u32,
    settle: usize,
    worst: usize,
    objects: bool,
    out: PathBuf,
}
impl Options {
    fn parse(args: &[String]) -> Result<Self> {
        let map = crate::arg_value(args, "--map").context("--sweep requires --map <id>")?;
        anyhow::ensure!(
            map.len() == 12
                && map.starts_with('m')
                && map
                    .chars()
                    .all(|c| c.is_ascii_digit() || c == '_' || c == 'm'),
            "invalid map id"
        );
        let spacing = crate::arg_f32(args, "--spacing", 80.)?;
        let yaws = crate::arg_f32(args, "--views", 4.)?;
        let width = crate::arg_f32(args, "--sweep-width", 256.)?;
        let height = crate::arg_f32(args, "--sweep-height", 144.)?;
        let settle = crate::arg_f32(args, "--settle-frames", 4.)?;
        let worst = crate::arg_f32(args, "--worst", 20.)?;
        for (name, n, min, max) in [
            ("spacing", spacing, 1., 1000.),
            ("views", yaws, 1., 64.),
            ("width", width, 16., 4096.),
            ("height", height, 16., 4096.),
            ("settle frames", settle, 4., 1000.),
            ("worst", worst, 0., 100.),
        ] {
            anyhow::ensure!(n.is_finite() && n >= min && n <= max, "invalid {name}");
            if name != "spacing" {
                anyhow::ensure!(n.fract() == 0., "{name} must be an integer");
            }
        }
        let pitches = crate::arg_value(args, "--edge-pitches")
            .unwrap_or_else(|| "-25,-50".into())
            .split(',')
            .map(str::parse)
            .collect::<std::result::Result<Vec<f32>, _>>()?;
        anyhow::ensure!(
            pitches
                .iter()
                .all(|p| p.is_finite() && *p >= -85. && *p < 0.),
            "edge pitches must be in [-85,0)"
        );
        let objects = args.iter().any(|a| a == "--objects");
        let out = PathBuf::from("target/alt-hs/sweep")
            .join(&map)
            .join(if objects { "objects" } else { "pieces" });
        Ok(Self {
            map,
            spacing,
            yaws: yaws as usize,
            pitches,
            width: width as u32,
            height: height as u32,
            settle: settle as usize,
            worst: worst as usize,
            objects,
            out,
        })
    }
}

/// Sample every horizontal grid cell covered by walkable collision, retaining floors on
/// separate vertical layers. A ceiling/headroom check rejects undersides and enclosed points.
fn sample(mesh: &sekiro_formats::hknp::CollisionMesh, options: &Options) -> Vec<View> {
    let collision = sekiro_sim::collision::CollisionWorld::from_mesh(mesh);
    let mut cells = BTreeMap::new();
    let spacing = options.spacing;
    for tri in mesh.indices.as_chunks::<3>().0 {
        let t = tri.map(|i| Vec3::from(mesh.vertices[i as usize]));
        let normal = (t[1] - t[0]).cross(t[2] - t[0]).normalize_or_zero();
        if normal.y.abs() < 50f32.to_radians().cos() {
            continue;
        }
        let lo = t[0].min(t[1]).min(t[2]);
        let hi = t[0].max(t[1]).max(t[2]);
        for x in (lo.x / spacing - 0.5).ceil() as i32..=(hi.x / spacing - 0.5).floor() as i32 {
            for z in (lo.z / spacing - 0.5).ceil() as i32..=(hi.z / spacing - 0.5).floor() as i32 {
                let p = Vec3::new((x as f32 + 0.5) * spacing, 0., (z as f32 + 0.5) * spacing);
                if let Some(h) = vertical_hit(p, t[0], t[1], t[2]) {
                    let y = -h;
                    cells
                        .entry((x, (y * 4.).round() as i32, z))
                        .or_insert(Vec3::new(p.x, y, p.z));
                }
            }
        }
    }
    let mut views = Vec::new();
    for feet in cells.into_values() {
        if collision
            .floor(feet.x, feet.z, feet.y + 0.1)
            .is_none_or(|y| (y - feet.y).abs() > 0.15)
            || collision
                .raycast(feet + Vec3::Y * 0.1, Vec3::Y, 1.65)
                .is_some()
        {
            continue;
        }
        let edge = [Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y].iter().any(|d| {
            let p = feet.xz() + *d * 2.;
            collision
                .floor(p.x, p.y, feet.y + 0.5)
                .is_none_or(|h| feet.y - h > 3.)
        });
        for yaw in 0..options.yaws {
            for pitch in std::iter::once(&0.).chain(options.pitches.iter().filter(|_| edge)) {
                views.push(View {
                    eye: feet + Vec3::Y * 1.65,
                    yaw: yaw as f32 * 360. / options.yaws as f32,
                    pitch: *pitch,
                    edge,
                });
            }
        }
    }
    views
}

#[derive(Component)]
struct Duplicate {
    source: Entity,
}
#[derive(Component)]
struct SweepCamera;
#[derive(Component)]
struct DepthCamera(usize);
#[derive(Resource)]
struct Received(Option<Image>);
#[derive(Resource)]
struct Sweep {
    options: Options,
    views: Vec<View>,
    index: usize,
    phase: usize,
    countdown: usize,
    waiting: bool,
    initialized: bool,
    frames: Vec<Image>,
    reports: Vec<Value>,
    worst: Vec<(u64, usize, Vec<Image>)>,
    target: Handle<Image>,
    started: Instant,
    phase_started: Instant,
    roots: Vec<(Entity, Visibility)>,
    unculled: Vec<Entity>,
    total_views: usize,
    started_unix_seconds: u64,
    configure_pending: bool,
    collision_oracle: sekiro_sim::collision::CollisionWorld,
    depth_target: Handle<Image>,
    collision_target: Handle<Image>,
    next_loading_log: Instant,
}

pub(crate) fn run(args: &[String]) -> Result<()> {
    let mut options = Options::parse(args)?;
    let cache = sekiro_sim::cache::root();
    let dir = map_dir(&cache, &options.map)?;
    let mesh = sekiro_formats::hknp::CollisionMesh::from_bytes(&std::fs::read(
        dir.join("collision.bin"),
    )?)?;
    let mut views = sample(&mesh, &options);
    let total_views = views.len();
    if let Some(limit) = crate::arg_value(args, "--limit-views") {
        let limit: usize = limit.parse()?;
        anyhow::ensure!(limit > 0, "view limit must be positive");
        views.truncate(limit);
        options.out = options.out.join(format!("smoke-{limit}"));
    }
    anyhow::ensure!(!views.is_empty(), "no camera points at this spacing");
    println!(
        "sweep {}: {} views, spacing {} m, {}x{}, objects {}",
        options.map,
        views.len(),
        options.spacing,
        options.width,
        options.height,
        options.objects
    );
    if args.iter().any(|a| a == "--sample-only") {
        return Ok(());
    }
    std::fs::create_dir_all(&options.out)?;
    let mut app = App::new();
    raise_io_stack();
    app.add_plugins(
        DefaultPlugins
            .set(layered_gltf_plugin())
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: bevy::window::ExitCondition::DontExit,
                ..default()
            })
            .set(AssetPlugin {
                file_path: cache.to_string_lossy().into_owned(),
                ..default()
            })
            .disable::<bevy::winit::WinitPlugin>()
            .disable::<bevy::audio::AudioPlugin>(),
    )
    .add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_millis(1)))
    .add_plugins(MaterialPlugin::<DiagnosticMaterial>::default());
    // Keep scrolling water/clouds identical in the normal/all comparison.
    // Advance once per pose to allow the shared 0.5 s local-light selection to run.
    let mut clock = app.world_mut().resource_mut::<Time<Virtual>>();
    clock.advance_by(Duration::from_secs(1));
    clock.pause();
    embedded_asset!(&mut app, "map_sweep.wgsl");
    add_map(
        &mut app,
        &dir,
        MapScene {
            prefix: format!("maps/{}/", options.map),
            collision: false,
            no_pieces: false,
            hide: Vec::new(),
            objects: options.objects,
            groups: GroupMode::Draw,
            markers: false,
            hour: f32::NAN,
            gparam: None,
            within: None,
        },
    )?;
    let pipelines = Pipelines::default();
    pipelines.0.0.store(1, Ordering::Relaxed);
    app.insert_resource(pipelines.clone());
    app.sub_app_mut(bevy::render::RenderApp)
        .insert_resource(pipelines)
        .add_systems(
            bevy::render::Render,
            (|status: Res<Pipelines>, cache: Res<bevy::render::render_resource::PipelineCache>| {
                status
                    .0
                    .0
                    .store(cache.waiting_pipelines().count(), Ordering::Relaxed);
                status.0.1.store(
                    cache
                        .pipelines()
                        .filter(|p| {
                            let failed = matches!(
                                p.state,
                                bevy::render::render_resource::CachedPipelineState::Err(
                                    bevy::shader::ShaderCacheError::ProcessShaderError(_)
                                        | bevy::shader::ShaderCacheError::CreateShaderModule(_)
                                )
                            );
                            if failed {
                                error!("sweep GPU pipeline failed: {:?}", p.state);
                            }
                            failed
                        })
                        .count(),
                    Ordering::Relaxed,
                );
            })
            .in_set(bevy::render::RenderSystems::PostCleanup),
        );
    let image = Image::new_target_texture(
        options.width,
        options.height,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        None,
    );
    let target = app.world_mut().resource_mut::<Assets<Image>>().add(image);
    let depth_target =
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::new_target_texture(
                options.width,
                options.height,
                bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                None,
            ));
    let collision_target =
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::new_target_texture(
                options.width,
                options.height,
                bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                None,
            ));
    app.world_mut().spawn((
        Camera3d::default(),
        Camera::default(),
        Msaa::Off,
        RenderTarget::Image(target.clone().into()),
        Transform::default(),
        MapFocus,
        SweepCamera,
        bevy::camera::ShadowLodOrigin,
        bevy::core_pipeline::prepass::DepthPrepass,
        bevy::core_pipeline::prepass::MotionVectorPrepass,
    ));
    for (layer, target) in [(1, depth_target.clone()), (2, collision_target.clone())] {
        app.world_mut().spawn((
            Camera3d::default(),
            Camera {
                is_active: false,
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            Msaa::Off,
            RenderTarget::Image(target.into()),
            Transform::default(),
            DepthCamera(if layer == 1 { 2 } else { 3 }),
            RenderLayers::layer(layer),
        ));
    }
    // Collision is rendered by the same projection/rasterizer as visual geometry.
    let mut collision_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    collision_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, mesh.vertices.clone());
    collision_mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vec![[0., 1., 0.]; mesh.vertices.len()],
    );
    collision_mesh.insert_indices(Indices::U32(
        mesh.indices
            .as_chunks::<3>()
            .0
            .iter()
            .enumerate()
            .filter(|(i, _)| !matches!(mesh.materials.get(*i), Some(22 | 36 | 38)))
            .flat_map(|(_, t)| *t)
            .collect(),
    ));
    let mesh_handle = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(collision_mesh);
    let material = app
        .world_mut()
        .resource_mut::<Assets<DiagnosticMaterial>>()
        .add(DiagnosticMaterial {
            base: StandardMaterial {
                cull_mode: None,
                ..default()
            },
            extension: Diagnostic { bad: Vec4::ZERO },
        });
    app.world_mut().spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(material),
        Transform::default(),
        RenderLayers::layer(2),
    ));
    app.insert_resource(Received(None))
        .insert_resource(Sweep {
            options,
            views,
            index: 0,
            phase: 0,
            countdown: 0,
            waiting: false,
            initialized: false,
            frames: Vec::new(),
            reports: Vec::new(),
            worst: Vec::new(),
            target,
            started: Instant::now(),
            phase_started: Instant::now(),
            roots: Vec::new(),
            unculled: Vec::new(),
            total_views,
            started_unix_seconds: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
            configure_pending: false,
            collision_oracle: sekiro_sim::collision::CollisionWorld::from_mesh(&mesh),
            depth_target,
            collision_target,
            next_loading_log: Instant::now() + Duration::from_secs(15),
        })
        .add_systems(First, apply_pending_view.after(bevy::time::TimeSystems))
        .add_systems(
            PostUpdate,
            all_visible
                .after(super::visibility::VisibilityWrite)
                .before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
        )
        .add_systems(
            PostUpdate,
            tick.after(bevy::transform::TransformSystems::Propagate),
        );
    let exit = app.run();
    anyhow::ensure!(exit == AppExit::Success, "sweep failed; see diagnostics");
    Ok(())
}

fn map_root(world: &World, mut e: Entity) -> bool {
    loop {
        if world.get::<DrawGroups>(e).is_some() {
            return true;
        }
        let Some(parent) = world.get::<ChildOf>(e) else {
            return false;
        };
        e = parent.parent();
    }
}
fn all_visible(sweep: Res<Sweep>, mut roots: Query<&mut Visibility, With<DrawGroups>>) {
    if sweep.phase == 1 && !sweep.configure_pending {
        for mut visible in &mut roots {
            *visible = Visibility::Visible;
        }
    }
}

fn initialize(world: &mut World, objects: bool) {
    // Connected exploration normally includes ordinary objects unconditionally.
    // Keep the sweep's explicit pieces/objects controls distinct using authored part kinds.
    let mut object_names = std::collections::HashSet::new();
    if !objects && let Some(connected) = world.get_resource::<super::world::MapWorld>() {
        for (id, block) in &connected.blocks {
            for row in block.json["parts"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|r| r["kind"] == "Object")
            {
                if let Some(name) = row["name"].as_str() {
                    object_names.insert((id.clone(), name.to_string()));
                }
            }
        }
    }
    let excluded: std::collections::HashSet<_> = world
        .query::<(Entity, &super::visibility::VisibilityPart)>()
        .iter(world)
        .filter(|(_, p)| object_names.contains(&(p.map.clone(), p.name.clone())))
        .map(|(e, _)| e)
        .collect();
    let sources: Vec<_> = world
        .query::<(Entity, &Mesh3d, &GlobalTransform)>()
        .iter(world)
        .filter(|(e, _, _)| map_root(world, *e))
        .map(|(e, m, t)| (e, m.clone(), t.compute_transform()))
        .collect();
    let mut cache = std::collections::HashMap::new();
    for (entity, mesh, transform) in sources {
        let mut ancestor = entity;
        let mut omit = false;
        loop {
            if excluded.contains(&ancestor) {
                omit = true;
                break;
            }
            let Some(parent) = world.get::<ChildOf>(ancestor) else {
                break;
            };
            ancestor = parent.parent();
        }
        if omit {
            world.entity_mut(entity).insert(RenderLayers::layer(3));
            continue;
        }
        let Some(map_material::SweepMaterial {
            key,
            mut base,
            missing,
            ..
        }) = map_material::sweep_base(world, entity)
        else {
            continue;
        };
        // Byte-encoded depth must not be added/multiplied/blended with a second surface.
        // Retain authored cutouts; approximate translucent coverage with a small alpha cutoff.
        if !matches!(base.alpha_mode, AlphaMode::Opaque | AlphaMode::Mask(_)) {
            base.alpha_mode = AlphaMode::Mask(0.05);
        }
        let material = cache
            .entry(key)
            .or_insert_with(|| {
                world
                    .resource_mut::<Assets<DiagnosticMaterial>>()
                    .add(DiagnosticMaterial {
                        base,
                        extension: Diagnostic {
                            bad: Vec4::splat(f32::from(missing)),
                        },
                    })
            })
            .clone();
        world.spawn((
            mesh,
            MeshMaterial3d(material),
            transform,
            Duplicate { source: entity },
            RenderLayers::layer(1),
        ));
    }
}

fn configure(_world: &mut World, sweep: &mut Sweep) {
    // Camera activation must precede Bevy's camera and shadow-cascade preparation.
    sweep.configure_pending = true;
    sweep.countdown = sweep.options.settle;
    sweep.phase_started = Instant::now();
}

fn apply_pending_view(world: &mut World) {
    let mut sweep = world.remove_resource::<Sweep>().unwrap();
    if sweep.configure_pending {
        sweep.configure_pending = false;
        apply_view(world, &mut sweep);
    }
    world.insert_resource(sweep);
}

fn apply_view(world: &mut World, sweep: &mut Sweep) {
    if sweep.phase == 0 {
        let time = {
            let mut clock = world.resource_mut::<Time<Virtual>>();
            clock.advance_by(Duration::from_millis(600));
            clock.as_generic()
        };
        // Give the shared LOD crossfade its pose-settling step, then hold time for comparison.
        world.insert_resource(time);
    }
    let view = &sweep.views[sweep.index];
    let transform = Transform::from_translation(view.eye).with_rotation(Quat::from_euler(
        EulerRot::YXZ,
        view.yaw.to_radians(),
        view.pitch.to_radians(),
        0.,
    ));
    for mut t in world
        .query_filtered::<&mut Transform, With<Camera3d>>()
        .iter_mut(world)
    {
        *t = transform;
    }
    for (mut camera, depth) in world
        .query::<(&mut Camera, Option<&DepthCamera>)>()
        .iter_mut(world)
    {
        camera.is_active = depth.map_or(sweep.phase < 2, |d| sweep.phase == d.0);
    }
    // Reset the native group selector for each view and comparison, retaining its floor choice.
    let mut groups = world.resource_mut::<Groups>();
    groups.mode = if sweep.phase == 1 {
        GroupMode::Off
    } else {
        GroupMode::Draw
    };
    groups.checked_at = None;
    groups.dirty = true;
    if sweep.phase == 1 {
        sweep.roots = world
            .query_filtered::<(Entity, &Visibility), With<DrawGroups>>()
            .iter(world)
            .map(|(e, v)| (e, *v))
            .collect();
        sweep.unculled = world
            .query_filtered::<Entity, (With<Mesh3d>, Without<NoFrustumCulling>)>()
            .iter(world)
            .collect();
        for e in &sweep.unculled {
            world.entity_mut(*e).insert(NoFrustumCulling);
        }
    } else if sweep.phase == 2 {
        for (e, v) in sweep.roots.drain(..) {
            world.entity_mut(e).insert(v);
        }
        for e in sweep.unculled.drain(..) {
            world.entity_mut(e).remove::<NoFrustumCulling>();
        }
    }
}

fn tick(world: &mut World) {
    let mut sweep = world.remove_resource::<Sweep>().unwrap();
    // Keep native post operators off diagnostic targets even during asset warm-up.
    let cameras: Vec<_> = world
        .query_filtered::<Entity, With<DepthCamera>>()
        .iter(world)
        .collect();
    for e in cameras {
        world.entity_mut(e).remove::<(
            crate::lighting::tonemap::GamePost,
            crate::lighting::fog::GameFog,
            crate::lighting::grading::GradingLut,
            bevy::light::Skybox,
        )>();
    }
    if world.resource::<Pipelines>().0.1.load(Ordering::Relaxed) > 0 {
        error!("sweep aborted: a render pipeline failed");
        world.write_message(AppExit::error());
        world.insert_resource(sweep);
        return;
    }
    // Fail rather than silently report a partially loaded or stalled render as a hole.
    if (!sweep.initialized && sweep.started.elapsed() > Duration::from_secs(600))
        || (sweep.initialized && sweep.phase_started.elapsed() > Duration::from_secs(180))
    {
        error!(
            "sweep timed out at view {} phase {}",
            sweep.index, sweep.phase
        );
        world.write_message(AppExit::error());
        world.insert_resource(sweep);
        return;
    }
    if !world.resource::<Pending>().loaded() || world.resource::<Pending>().loaded_for() < 3. {
        if Instant::now() >= sweep.next_loading_log {
            let pending = world.resource::<Pending>();
            println!(
                "sweep loading: {} / {} scenes ready",
                pending.ready, pending.instances
            );
            sweep.next_loading_log = Instant::now() + Duration::from_secs(15);
        }
        world.insert_resource(sweep);
        return;
    }
    if !sweep.initialized {
        let sources: Vec<_> = world
            .query_filtered::<Entity, With<Mesh3d>>()
            .iter(world)
            .filter(|e| map_root(world, *e))
            .collect();
        if sources
            .iter()
            .any(|e| map_material::sweep_base(world, *e).is_some_and(|m| m.pending))
        {
            if Instant::now() >= sweep.next_loading_log {
                println!("sweep loading: material textures pending");
                sweep.next_loading_log = Instant::now() + Duration::from_secs(15);
            }
            world.insert_resource(sweep);
            return;
        }
        initialize(world, sweep.options.objects);
        sweep.initialized = true;
        configure(world, &mut sweep);
        sweep.countdown = sweep.countdown.max(120);
    }
    let visibility: Vec<_> = world
        .query::<(Entity, &Duplicate)>()
        .iter(world)
        .map(|(e, d)| {
            (
                e,
                world
                    .get::<InheritedVisibility>(d.source)
                    .is_some_and(|v| v.get()),
            )
        })
        .collect();
    // Hold native exposure constant across each normal/all comparison.
    for (mut post, _) in world
        .query::<(&mut crate::lighting::tonemap::GamePost, &SweepCamera)>()
        .iter_mut(world)
    {
        post.exposure.w = 0.;
    }
    for (e, shown) in visibility {
        world.entity_mut(e).insert(if shown {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
    }
    if let Some(image) = world.resource_mut::<Received>().0.take() {
        sweep.waiting = false;
        sweep.frames.push(image);
        if sweep.phase < 3 {
            sweep.phase += 1;
            configure(world, &mut sweep);
        } else {
            if let Err(e) = validate_depth(
                &sweep.frames[3],
                &sweep.views[sweep.index],
                &sweep.collision_oracle,
            ) {
                error!("sweep collision depth validation: {e:#}");
                world.write_message(AppExit::error());
                world.insert_resource(sweep);
                return;
            }
            match measure(
                &sweep.frames,
                &sweep.views[sweep.index],
                &sweep.options,
                sweep.index,
            ) {
                Ok(report) => {
                    let score = report["score"].as_u64().unwrap();
                    sweep.reports.push(report);
                    // Retain progress so interrupted long runs still have useful measurements.
                    if sweep.index.is_multiple_of(20)
                        && let Err(e) = std::fs::write(
                            sweep.options.out.join("partial.json"),
                            serde_json::to_vec(&sweep.reports).unwrap(),
                        )
                    {
                        error!("sweep progress output: {e}");
                        world.write_message(AppExit::error());
                        world.insert_resource(sweep);
                        return;
                    }
                    let index = sweep.index;
                    let frames = std::mem::take(&mut sweep.frames);
                    if sweep.options.worst > 0
                        && (sweep.worst.len() < sweep.options.worst
                            || sweep.worst.last().is_some_and(|w| score > w.0))
                    {
                        sweep.worst.push((score, index, frames));
                        sweep
                            .worst
                            .sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
                        sweep.worst.truncate(sweep.options.worst);
                    }
                    sweep.index += 1;
                    if sweep.index.is_multiple_of(20) {
                        println!(
                            "sweep progress {}/{} ({:.1} s)",
                            sweep.index,
                            sweep.views.len(),
                            sweep.started.elapsed().as_secs_f32()
                        );
                    }
                    if sweep.index == sweep.views.len() {
                        if let Err(e) = finish(&sweep) {
                            error!("sweep output: {e:#}");
                            world.write_message(AppExit::error());
                        } else {
                            world.write_message(AppExit::Success);
                        }
                        world.insert_resource(sweep);
                        return;
                    }
                    sweep.phase = 0;
                    configure(world, &mut sweep);
                }
                Err(e) => {
                    error!("sweep measurement: {e:#}");
                    world.write_message(AppExit::error());
                    world.insert_resource(sweep);
                    return;
                }
            }
        }
    }
    if !sweep.waiting {
        if sweep.countdown > 0 {
            sweep.countdown -= 1;
        } else if world.resource::<Pipelines>().0.0.load(Ordering::Relaxed) == 0 {
            world
                .spawn(Screenshot::image(match sweep.phase {
                    2 => sweep.depth_target.clone(),
                    3 => sweep.collision_target.clone(),
                    _ => sweep.target.clone(),
                }))
                .observe(
                    |event: On<ScreenshotCaptured>, mut received: ResMut<Received>| {
                        received.0 = Some(event.image.clone());
                    },
                );
            sweep.waiting = true;
        }
    }
    world.insert_resource(sweep);
}

fn depth(pixel: &[u8]) -> Option<f32> {
    let code = u16::from(pixel[0]) * 256 + u16::from(pixel[1]);
    (code > 0).then(|| 2f32.powf((f32::from(code) - 1.) / 65534. * 16.) - 1.)
}

/// Independently check the GPU byte encoding and camera projection against real collision.
/// A few rays per view catch post-processing/colour conversion and stale-camera mistakes.
fn validate_depth(
    image: &Image,
    view: &View,
    collision: &sekiro_sim::collision::CollisionWorld,
) -> Result<()> {
    let rgba = image.clone().try_into_dynamic()?.to_rgba8();
    let (width, height) = rgba.dimensions();
    let rotation = Quat::from_euler(
        EulerRot::YXZ,
        view.yaw.to_radians(),
        view.pitch.to_radians(),
        0.,
    );
    for (x, y) in [
        (width / 2, height / 2),
        (width / 2, height * 3 / 4),
        (width / 4, height * 3 / 4),
        (width * 3 / 4, height * 3 / 4),
    ] {
        let tan = 30f32.to_radians().tan();
        let ray = (rotation
            * Vec3::new(
                (2. * (x as f32 + 0.5) / width as f32 - 1.) * tan * width as f32 / height as f32,
                (1. - 2. * (y as f32 + 0.5) / height as f32) * tan,
                -1.,
            ))
        .normalize();
        let expected = collision.raycast(view.eye, ray, 100.);
        let measured = depth(&rgba.get_pixel(x, y).0);
        if let Some(expected) = expected.filter(|d| *d >= 0.2) {
            anyhow::ensure!(
                measured.is_some_and(|d| (d - expected).abs() <= 0.5 + expected * 0.05),
                "eye {:?}, yaw {}, pitch {}, pixel {x},{y}: CPU {expected:.3} m, GPU {measured:?} m",
                view.eye,
                view.yaw,
                view.pitch
            );
        } else if expected.is_none() {
            anyhow::ensure!(
                measured.is_none_or(|d| d > 100.),
                "pixel {x},{y}: CPU clear to 100 m, GPU {measured:?} m"
            );
        }
    }
    Ok(())
}
fn counts(
    normal: &[u8],
    full: &[u8],
    diagnostic: &[u8],
    collision: &[u8],
    view: &View,
    width: u32,
    height: u32,
) -> [u64; 5] {
    let mut result = [0; 5];
    let rotation = Quat::from_euler(
        EulerRot::YXZ,
        view.yaw.to_radians(),
        view.pitch.to_radians(),
        0.,
    );
    let tan = 30f32.to_radians().tan();
    for i in 0..(width * height) as usize {
        let offset = i * 4;
        let observed = depth(&diagnostic[offset..offset + 4]);
        let expected = depth(&collision[offset..offset + 4]);
        let y = i as u32 / width;
        let x = i as u32 % width;
        let ray = rotation
            * Vec3::new(
                (2. * (x as f32 + 0.5) / width as f32 - 1.) * tan * width as f32 / height as f32,
                (1. - 2. * (y as f32 + 0.5) / height as f32) * tan,
                -1.,
            );
        if ray.y < 0. && observed.is_none_or(|d| d > 1000.) {
            result[0] += 1;
        }
        if let Some(d) = expected.filter(|d| *d <= 100.)
            && observed.is_none_or(|render| render > d + 2f32.max(d * 0.1))
        {
            result[1] += 1;
        }
        if observed.is_some() && diagnostic[offset + 2] > 127 {
            result[2] += 1;
        }
        let rgb = &normal[offset..offset + 3];
        let lo = *rgb.iter().min().unwrap();
        let hi = *rgb.iter().max().unwrap();
        if observed.is_some() && hi - lo <= 8 && (70..=210).contains(&lo) {
            result[4] += 1;
        }
        if (0..3).any(|c| normal[offset + c].abs_diff(full[offset + c]) > 12) {
            result[3] += 1;
        }
    }
    result
}
fn measure(images: &[Image], view: &View, options: &Options, id: usize) -> Result<Value> {
    let rgba = images
        .iter()
        .map(|image| {
            image
                .clone()
                .try_into_dynamic()
                .map(|d| d.to_rgba8().into_raw())
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let counts = counts(
        &rgba[0],
        &rgba[1],
        &rgba[2],
        &rgba[3],
        view,
        options.width,
        options.height,
    );
    Ok(
        json!({"id":id,"position":view.eye.to_array(),"yaw":view.yaw,"pitch":view.pitch,"edge":view.edge,"void":counts[0],"depth_gap":counts[1],"untextured":counts[2],"pop_in":counts[3],"grey":counts[4],"score":counts.iter().sum::<u64>()}),
    )
}
fn finish(sweep: &Sweep) -> Result<()> {
    let mut ranked = sweep.reports.clone();
    ranked.sort_by(|a, b| {
        b["score"]
            .as_u64()
            .cmp(&a["score"].as_u64())
            .then(a["id"].as_u64().cmp(&b["id"].as_u64()))
    });
    let totals: BTreeMap<_, _> = ["void", "depth_gap", "untextured", "grey", "pop_in", "score"]
        .into_iter()
        .map(|k| {
            (
                k,
                sweep
                    .reports
                    .iter()
                    .map(|r| r[k].as_u64().unwrap())
                    .sum::<u64>(),
            )
        })
        .collect();
    let report = json!({"version":1,"map":sweep.options.map,"objects":sweep.options.objects,"spacing":sweep.options.spacing,"yaws":sweep.options.yaws,"edge_pitches":sweep.options.pitches,"eye_height":1.65,"width":sweep.options.width,"height":sweep.options.height,"settle_frames":sweep.options.settle,"seconds":sweep.started.elapsed().as_secs_f64(),"started_unix_seconds":sweep.started_unix_seconds,"views":sweep.reports.len(),"total_sample_views":sweep.total_views,"complete_sample":sweep.reports.len()==sweep.total_views,"totals":totals,"worst20":ranked.iter().take(20).collect::<Vec<_>>(),"points":sweep.reports});
    std::fs::write(
        sweep.options.out.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    // Remove only this tool's previous screenshot names, keeping arbitrary user files.
    for entry in std::fs::read_dir(&sweep.options.out)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if let Some((id, label)) = name.split_once('-')
            && id.len() == 6
            && id.chars().all(|c| c.is_ascii_digit())
            && matches!(
                label,
                "normal.png" | "all.png" | "depth.png" | "collision.png"
            )
            && entry.file_type()?.is_file()
        {
            std::fs::remove_file(entry.path())?;
        }
    }
    for (_, id, images) in &sweep.worst {
        for (image, label) in images.iter().zip(["normal", "all", "depth", "collision"]) {
            image
                .clone()
                .try_into_dynamic()?
                .save(sweep.options.out.join(format!("{id:06}-{label}.png")))?;
        }
    }
    println!("worst 20: id position yaw pitch void depth_gap untextured grey pop_in score");
    for row in ranked.iter().take(20) {
        println!(
            "{} {} {} {} {} {} {} {} {} {}",
            row["id"],
            row["position"],
            row["yaw"],
            row["pitch"],
            row["void"],
            row["depth_gap"],
            row["untextured"],
            row["grey"],
            row["pop_in"],
            row["score"]
        );
    }
    println!(
        "{} views in {:.1}s; totals {}; report {}",
        sweep.reports.len(),
        sweep.started.elapsed().as_secs_f32(),
        json!(totals),
        sweep.options.out.join("report.json").display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pixels_distinguish_horizon_collision_texture_and_visibility() {
        let view = View {
            eye: Vec3::ZERO,
            yaw: 0.,
            pitch: 0.,
            edge: false,
        };
        // Four pixels, two above the horizon and two below it; no visual surface.
        let empty = [0u8; 16];
        let collision = [0, 0, 0, 255, 0, 0, 0, 255, 32, 0, 0, 255, 0, 0, 0, 255];
        let mut full = empty;
        full[0] = 13;
        assert_eq!(
            counts(&empty, &full, &empty, &collision, &view, 2, 2),
            [2, 1, 0, 1, 0]
        );
        let mut render = empty;
        render[8] = 32;
        render[11] = 255;
        render[10] = 255;
        assert_eq!(
            counts(&empty, &empty, &render, &collision, &view, 2, 2),
            [1, 0, 1, 0, 0]
        );
        let mut grey = empty;
        grey[8..11].fill(100);
        assert_eq!(
            counts(&grey, &grey, &render, &collision, &view, 2, 2),
            [1, 0, 1, 0, 1]
        );
        let down = View {
            pitch: -50.,
            ..view
        };
        assert_eq!(counts(&empty, &empty, &empty, &empty, &down, 2, 2)[0], 4);
    }
    #[test]
    fn samples_grid_interiors_and_retains_stacked_floors() {
        let mesh = sekiro_formats::hknp::CollisionMesh {
            vertices: vec![
                [0., 0., 0.],
                [0., 0., 4.],
                [4., 0., 0.],
                [0., 4., 0.],
                [0., 4., 4.],
                [4., 4., 0.],
            ],
            indices: vec![0, 1, 2, 3, 4, 5],
            materials: vec![0, 0],
        };
        let options = Options::parse(&[
            "--map".into(),
            "m11_01_00_00".into(),
            "--spacing".into(),
            "2".into(),
            "--views".into(),
            "1".into(),
        ])
        .unwrap();
        let views = sample(&mesh, &options);
        assert!(views.iter().any(|v| v.eye == Vec3::new(1., 1.65, 1.)));
        assert!(views.iter().any(|v| v.eye == Vec3::new(1., 5.65, 1.)));
        assert!(views.iter().any(|v| v.pitch < 0.));
    }
}

//! Authored FXR billboard playback. Unsupported emitters never suppress a stand-in.
//! Layout facts: EvenTorset (Unlicense); orientation/batching facts: FwLo (MIT).
use std::{collections::HashMap, path::PathBuf, sync::Arc};

use bevy::{
    asset::RenderAssetUsages,
    asset::{embedded_asset, embedded_path},
    mesh::MeshVertexBufferLayoutRef,
    mesh::{Indices, PrimitiveTopology},
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::render_resource::{
        AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};
use sekiro_formats::fxr::{Action, Blend, Fxr, NativeProperty};
use sekiro_sim::duel::Duel;

const PARTICLE_CAP: usize = 4096;
const EMITTER_CAP: usize = 256;
const EFFECT_SECONDS_CAP: f32 = 30.0;

/// A plugin-supplied bridge keeps the simulation private to the play module.
pub fn plugin(cache: PathBuf, bridge: impl Fn(&mut App) + Send + Sync + 'static) -> impl Plugin {
    struct PlayerPlugin<S> {
        cache: PathBuf,
        bridge: S,
    }
    impl<S: Fn(&mut App) + Send + Sync + 'static> Plugin for PlayerPlugin<S> {
        fn build(&self, app: &mut App) {
            embedded_asset!(app, "fxr.wgsl");
            app.add_plugins(MaterialPlugin::<FxrMaterial>::default());
            app.insert_resource(FxrPlayer::new(self.cache.clone()))
                .add_systems(Update, render);
            (self.bridge)(app);
        }
    }
    PlayerPlugin { cache, bridge }
}

#[derive(Clone)]
struct Channel {
    value: NativeProperty,
}
impl Channel {
    fn new(action: &Action, effect: &Fxr, name: &str, default_value: [f32; 4]) -> Option<Self> {
        let value = match action.property(effect, name) {
            Some(p) => p.compile_native_unmodified().ok()?,
            None => sekiro_formats::fxr::Property {
                tag: 0x23,
                evaluator: 0,
                values: default_value.map(f32::to_bits).to_vec(),
                modifiers: 0..0,
            }
            .compile_native_unmodified()
            .ok()?,
        };
        Some(Self { value })
    }
    fn at(&self, time: f32) -> [f32; 4] {
        self.value.sample(time).unwrap_or([0.0; 4])
    }
    fn scalar(&self, time: f32) -> f32 {
        self.at(time)[0]
    }
}

#[derive(Clone)]
struct Billboard {
    texture: PathBuf,
    blend: Blend,
    width: Channel,
    height: Channel,
    color: [Channel; 3],
    brightness: Channel,
    alpha: Channel,
    rotation: Channel,
    angular: Channel,
    angular_multiplier: Channel,
    frame: Channel,
    frame_offset: Channel,
    columns: u32,
    frames: u32,
    uniform: bool,
    orientation: u32,
    offset: [Channel; 3],
    fade: Channel,
    threshold: Channel,
}

struct Template {
    appearance: Billboard,
    delay: f32,
    duration: f32,
    lifetime: Channel,
    interval: Channel,
    count: Channel,
    total: Channel,
    concurrent: Channel,
    periodic: bool,
    speed: Channel,
    gravity: Channel,
    acceleration: Channel,
    transform: Transform,
    attached: bool,
    scale: [Channel; 2],
    color: Channel,
    direction: Vec3,
    movement_speed: Channel,
    movement_multiplier: Channel,
    acceleration_multiplier: Channel,
}

struct Emitter {
    template: Arc<Template>,
    key: (usize, i32, i32),
    transform: Transform,
    age: f32,
    next: f32,
    emissions: usize,
    stopped: bool,
    generation: usize,
}
struct Particle {
    template: Arc<Template>,
    position: Vec3,
    velocity: Vec3,
    rotation: Quat,
    age: f32,
    lifetime: f32,
    emitted: f32,
    key: (usize, i32, i32),
    generation: usize,
    spin: f32,
}

#[derive(Resource)]
pub struct FxrPlayer {
    enabled: bool,
    files: HashMap<i32, PathBuf>,
    templates: HashMap<i32, Vec<Arc<Template>>>,
    emitters: Vec<Emitter>,
    particles: Vec<Particle>,
    batches: HashMap<(PathBuf, bool), (Handle<Mesh>, Entity)>,
    bullets: HashMap<u64, (i32, Vec3)>,
    bullet_sfx: HashMap<i32, i32>,
    last_cues: Vec<(usize, i32, i32, u32)>,
    pub spawned: usize,
    pub rejected: usize,
    preview: Option<i32>,
    steps: usize,
    next_emitter: usize,
}

impl FxrPlayer {
    fn new(cache: PathBuf) -> Self {
        let enabled = !std::env::args().any(|a| a == "--no-fxr");
        let mut files = HashMap::new();
        if enabled && let Ok(dirs) = std::fs::read_dir(cache.join("raw/sfx")) {
            let mut dirs: Vec<_> = dirs
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect();
            dirs.sort();
            for dir in dirs {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().is_some_and(|e| e == "fxr")
                            && let Some(id) = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .and_then(|s| s.strip_prefix('f'))
                                .and_then(|s| s.parse().ok())
                        {
                            files.entry(id).or_insert(path);
                        }
                    }
                }
            }
        }
        let mut bullet_sfx = HashMap::new();
        if enabled
            && let Ok(params) = sekiro_formats::param::ParamSet::load(
                &cache.join("raw/param/gameparam/gameparam.parambnd.d"),
                &cache.join("refs/paramdex/Defs"),
            )
            && let Some(table) = params.get("Bullet")
        {
            for row in table.rows() {
                if let Ok(sfx) = row.int("sfxId_Bullet") {
                    bullet_sfx.insert(row.id(), sfx as i32);
                }
            }
        }
        info!(
            "FXR: enabled={enabled}, indexed {} installed effects",
            files.len()
        );
        Self {
            enabled,
            files,
            templates: HashMap::new(),
            emitters: Vec::new(),
            particles: Vec::new(),
            batches: HashMap::new(),
            bullets: HashMap::new(),
            bullet_sfx,
            last_cues: Vec::new(),
            spawned: 0,
            rejected: 0,
            preview: std::env::args()
                .collect::<Vec<_>>()
                .windows(2)
                .find(|a| a[0] == "--fxr-preview")
                .and_then(|a| a[1].parse().ok()),
            steps: 0,
            next_emitter: 0,
        }
    }

    fn load(&mut self, id: i32) {
        if self.templates.contains_key(&id) {
            return;
        }
        let templates = self
            .files
            .get(&id)
            .and_then(|p| {
                std::fs::read(p)
                    .ok()
                    .and_then(|b| sekiro_formats::fxr::parse(&b).ok())
                    .map(|f| (p.clone(), f))
            })
            .map(|(path, effect)| {
                let mut out = Vec::new();
                if let Some(bundle) = path.parent() {
                    collect_templates(&effect, 0, Transform::IDENTITY, 0, bundle, &mut out);
                }
                out
            })
            .unwrap_or_default();
        info!(
            "FXR {id}: {} supported billboard emitters (others retain stand-ins)",
            templates.len()
        );
        self.templates.insert(id, templates);
    }

    fn request(&mut self, id: i32, key: (usize, i32, i32), transform: Transform, start: bool) {
        if !self.enabled || id <= 0 {
            return;
        }
        if !start {
            for e in &mut self.emitters {
                if e.key == key && !e.stopped {
                    let next = transform * e.template.transform;
                    if e.template.attached {
                        let rotation = next.rotation * e.transform.rotation.inverse();
                        for p in &mut self.particles {
                            if p.key == key && p.generation == e.generation {
                                p.position = next.translation
                                    + rotation * (p.position - e.transform.translation);
                                p.velocity = rotation * p.velocity;
                                p.rotation = rotation * p.rotation;
                            }
                        }
                    }
                    e.transform = next;
                }
            }
            return;
        }
        self.load(id);
        let templates = &self.templates[&id];
        if templates.is_empty() {
            self.rejected += 1;
        }
        for template in templates {
            if self.emitters.len() >= EMITTER_CAP {
                break;
            }
            self.next_emitter += 1;
            self.emitters.push(Emitter {
                template: template.clone(),
                key,
                transform: transform * template.transform,
                age: 0.0,
                next: template.delay,
                emissions: 0,
                stopped: false,
                generation: self.next_emitter,
            });
        }
    }

    /// Called after each simulation step by the registration bridge.
    /// Positions use the simulation's posed dummy polys, including body fallback.
    pub fn observe(&mut self, duel: &Duel) {
        if !self.enabled {
            return;
        }
        let mut cues = Vec::new();
        let mut keys = Vec::new();
        self.steps += 1;
        if let Some(id) = self.preview {
            let key = (99, id, 0);
            keys.push(key);
            let position = (Vec3::from_array(duel.player.body.position)
                + Vec3::from_array(duel.enemy.body.position))
                * 0.5
                + Vec3::Y;
            if self.steps >= 60 {
                self.request(
                    id,
                    key,
                    Transform::from_translation(position),
                    (self.steps - 60).is_multiple_of(120),
                );
            }
            if self.steps.is_multiple_of(60) {
                info!(
                    "FXR preview step {}: particles={}, emitters={}, spawned={}, rejected={}",
                    self.steps,
                    self.particles.len(),
                    self.emitters.len(),
                    self.spawned,
                    self.rejected
                );
            }
        }
        for (actor, character) in [&duel.player, &duel.enemy].into_iter().enumerate() {
            for cue in &character.tae_frame().effects {
                let e = &cue.effect;
                // Blade effects and conditional/repeating variants need separate runtimes.
                if e.blade_tip.is_some()
                    || e.extra_condition != 0
                    || e.repeat
                    || e.dummy_source != 0
                {
                    continue;
                }
                let identity = (actor, e.id, e.dummy, cue.start.to_bits());
                let start = !self.last_cues.contains(&identity);
                cues.push(identity);
                let position = i16::try_from(e.dummy)
                    .ok()
                    .and_then(|d| duel.dummy_position(actor, d));
                if e.restrict_to_dummy && position.is_none() {
                    continue;
                }
                let position = position.unwrap_or(character.body.position);
                let transform = Transform::from_translation(Vec3::from_array(position))
                    .with_rotation(if e.ignore_dummy_angle {
                        Quat::IDENTITY
                    } else {
                        Quat::from_rotation_y(character.body.yaw)
                    });
                let key = (actor, e.id, e.dummy);
                keys.push(key);
                if start || e.follow_dummy {
                    self.request(e.id, key, transform, start);
                }
            }
        }
        self.last_cues = cues;
        for bullet in &duel.bullets.live {
            let id = self.bullet_sfx.get(&bullet.def.id).copied().unwrap_or(-1);
            let position = Vec3::from_array(bullet.position.to_array());
            let start = !self.bullets.contains_key(&bullet.serial);
            let key = (2, id, bullet.serial as i32);
            keys.push(key);
            self.request(
                id,
                key,
                Transform::from_translation(position).with_rotation(Quat::from_rotation_arc(
                    Vec3::Z,
                    Vec3::from_array(bullet.velocity.to_array()).normalize_or(Vec3::Z),
                )),
                start,
            );
            self.bullets.insert(bullet.serial, (id, position));
        }
        self.bullets
            .retain(|serial, _| duel.bullets.live.iter().any(|b| b.serial == *serial));
        for e in &mut self.emitters {
            if !keys.contains(&e.key) && e.template.periodic {
                e.stopped = true;
            }
        }
        self.advance(1.0 / 60.0);
    }

    fn advance(&mut self, dt: f32) {
        for particle in &mut self.particles {
            particle.age += dt;
            let t = &particle.template;
            let speed =
                t.movement_speed.scalar(particle.age) * t.movement_multiplier.scalar(particle.age);
            let previous_speed = t.movement_speed.scalar((particle.age - dt).max(0.0))
                * t.movement_multiplier.scalar((particle.age - dt).max(0.0));
            particle.velocity +=
                particle.velocity.normalize_or(t.direction) * (speed - previous_speed);
            particle.spin += t.appearance.angular.scalar(particle.age)
                * t.appearance.angular_multiplier.scalar(particle.age)
                * dt;
            particle.velocity += (particle.velocity.normalize_or(Vec3::Z)
                * t.acceleration.scalar(particle.age)
                * t.acceleration_multiplier.scalar(particle.age)
                - Vec3::Y * t.gravity.scalar(particle.age))
                * dt;
            particle.position += particle.velocity * dt;
        }
        self.particles
            .retain(|p| p.age < p.lifetime && p.position.is_finite());
        for emitter in &mut self.emitters {
            emitter.age += dt;
            let template = &emitter.template;
            let active = emitter.age - template.delay;
            if emitter.stopped || active >= template.duration || emitter.age >= EFFECT_SECONDS_CAP {
                continue;
            }
            let total = template.total.scalar(active);
            let max = template.concurrent.scalar(active);
            let mut iterations = 0;
            while emitter.next <= emitter.age
                && iterations < 8
                && (total < 0.0 || (emitter.emissions as f32) < total)
            {
                if !template.periodic && emitter.emissions != 0 {
                    break;
                }
                iterations += 1;
                let count = template.count.scalar(active).round().clamp(0.0, 256.0) as usize;
                for _ in 0..count {
                    if self.particles.len() >= PARTICLE_CAP {
                        break;
                    }
                    if max >= 0.0
                        && self
                            .particles
                            .iter()
                            .filter(|p| p.generation == emitter.generation)
                            .count()
                            >= max as usize
                    {
                        break;
                    }
                    let lifetime = template.lifetime.scalar(0.0);
                    let lifetime = if lifetime < 0.0 {
                        template.duration - active
                    } else {
                        lifetime
                    };
                    if lifetime <= 0.0 {
                        continue;
                    }
                    self.particles.push(Particle {
                        template: template.clone(),
                        position: emitter.transform.translation,
                        velocity: emitter.transform.rotation
                            * template.direction
                            * (template.speed.scalar(active)
                                + template.movement_speed.scalar(0.0)
                                    * template.movement_multiplier.scalar(0.0)),
                        rotation: emitter.transform.rotation,
                        age: 0.0,
                        lifetime: lifetime.min(EFFECT_SECONDS_CAP),
                        emitted: active,
                        key: emitter.key,
                        generation: emitter.generation,
                        spin: 0.0,
                    });
                    self.spawned += 1;
                }
                emitter.emissions += 1;
                emitter.next += template.interval.scalar(active).max(0.000001);
            }
        }
        self.emitters.retain(|e| {
            !e.stopped
                && e.age < e.template.delay + e.template.duration
                && e.age < EFFECT_SECONDS_CAP
        });
    }
}

fn collect_templates(
    effect: &Fxr,
    index: usize,
    parent: Transform,
    depth: usize,
    bundle: &std::path::Path,
    out: &mut Vec<Arc<Template>>,
) {
    if depth >= 64 || out.len() >= EMITTER_CAP {
        return;
    }
    let Some(node) = effect.nodes.get(index) else {
        return;
    };
    let mut parent = parent;
    if let Some(config) = effect.initial_configuration(node) {
        if config.tag as u16 == 1005 {
            return;
        }
        let actions = &effect.actions[config.actions.clone()];
        // Unsupported parent motion must not flatten its descendants into a false placement.
        if actions
            .iter()
            .any(|a| matches!(a.id(), 1 | 15 | 34 | 36 | 83 | 106 | 120..=123))
        {
            return;
        }
        if let Some((p, r)) = actions
            .iter()
            .find(|a| a.id() == 35)
            .and_then(Action::transform)
        {
            parent = parent
                * Transform::from_translation(Vec3::new(-p[0], p[1], p[2])).with_rotation(
                    Quat::from_euler(
                        EulerRot::YXZ,
                        -r[1].to_radians(),
                        r[0].to_radians(),
                        -r[2].to_radians(),
                    ),
                );
        }
        if config.tag as u16 == 1004
            && let Some(mut template) = template(effect, actions, bundle.to_path_buf())
        {
            template.transform = parent;
            out.push(Arc::new(template));
        }
    }
    for child in node.children.clone() {
        collect_templates(effect, child, parent, depth + 1, bundle, out);
    }
}

fn template(effect: &Fxr, actions: &[Action], bundle: PathBuf) -> Option<Template> {
    let find = |id| actions.iter().find(|a| a.id() == id);
    let appearance = find(603)?;
    // This milestone draws single-texture quads only, with explicit unsupported fallbacks.
    let channel = |a: &Action, name, value| Channel::new(a, effect, name, [value; 4]);
    let texture = channel(appearance, "texture", 1.0)?.scalar(0.0) as i32;
    let texture = bundle.join(format!("s{texture:05}.tpf"));
    if !texture.is_file() {
        return None;
    }
    let blend = Blend::from_raw(channel(appearance, "blend", 2.0)?.scalar(0.0) as u32)?;
    if !matches!(blend, Blend::Additive | Blend::Alpha) {
        return None;
    }
    let attrs = find(128)?;
    let duration = channel(attrs, "duration", -1.0)?.scalar(0.0);
    let duration = if duration < 0.0 {
        EFFECT_SECONDS_CAP
    } else {
        duration
    };
    let particle_attrs = find(129)?;
    let emitter = find(300).or_else(|| find(399))?;
    let modifier = find(131)?;
    let movement = find(55).or_else(|| find(60));
    let zero = channel(modifier, "unused", 0.0)?;
    let acceleration = movement
        .filter(|a| a.id() == 55)
        .map(|a| channel(a, "acceleration", 0.0))
        .unwrap_or_else(|| Some(zero.clone()))?;
    let gravity = movement
        .map(|a| channel(a, "gravity", 0.0))
        .unwrap_or_else(|| Some(zero.clone()))?;
    let speed = channel(modifier, "speed", 0.0)?;
    // Moving shapes, spread, node movement and attachment modes need separate integration.
    if actions.iter().any(|a| matches!(a.id(), 1 | 15 | 34 | 36 | 83 | 105 | 106 | 120..=123 | 301 | 401..=405 | 501..=503 | 1005)) { return None; }
    let transform = find(35)
        .and_then(Action::transform)
        .map(|(p, r)| {
            Transform::from_translation(Vec3::new(-p[0], p[1], p[2])).with_rotation(
                Quat::from_euler(
                    EulerRot::YXZ,
                    -r[1].to_radians(),
                    r[0].to_radians(),
                    -r[2].to_radians(),
                ),
            )
        })
        .unwrap_or_default();
    let delay = attrs.float_field("delay")?;
    let color = [
        channel(appearance, "color", 1.0)?,
        channel(appearance, "emission_color", 1.0)?,
        channel(appearance, "hdr_color", 1.0)?,
    ];
    Some(Template {
        appearance: Billboard {
            texture,
            blend,
            width: channel(appearance, "width", 1.0)?,
            height: channel(appearance, "height", 1.0)?,
            color,
            brightness: channel(appearance, "brightness", 1.0)?,
            alpha: channel(appearance, "alpha", 1.0)?,
            rotation: channel(appearance, "rotation_z", 0.0)?,
            angular: channel(appearance, "angular_z", 0.0)?,
            angular_multiplier: channel(appearance, "angular_multiplier_z", 1.0)?,
            frame: channel(appearance, "frame", 0.0)?,
            frame_offset: channel(appearance, "frame_offset", 0.0)?,
            columns: appearance.field("columns")?.max(1),
            frames: appearance.field("frames")?.max(1),
            uniform: appearance.field("uniform")? != 0,
            orientation: appearance.field("orientation")?,
            offset: [
                channel(appearance, "offset_x", 0.0)?,
                channel(appearance, "offset_y", 0.0)?,
                channel(appearance, "offset_z", 0.0)?,
            ],
            fade: channel(appearance, "alpha_fade", 0.0)?,
            threshold: channel(appearance, "alpha_threshold", 0.0)?,
        },
        delay,
        duration,
        lifetime: channel(particle_attrs, "duration", -1.0)?,
        interval: channel(emitter, "interval", 1.0)?,
        count: channel(emitter, "count", 1.0)?,
        total: channel(emitter, "total", -1.0)?,
        concurrent: channel(emitter, "concurrent", -1.0)?,
        periodic: emitter.id() == 300,
        speed,
        gravity,
        acceleration,
        transform,
        attached: particle_attrs.field("attachment")? != 0,
        scale: [
            channel(modifier, "scale_x", 1.0)?,
            channel(
                modifier,
                if modifier.field("uniform")? != 0 {
                    "scale_x"
                } else {
                    "scale_y"
                },
                1.0,
            )?,
        ],
        color: channel(modifier, "color", 1.0)?,
        direction: match find(400).and_then(|a| a.field("direction")).unwrap_or(0) {
            1 | 4 => Vec3::Y,
            2 | 5 => Vec3::NEG_Y,
            _ => Vec3::Z,
        },
        movement_speed: movement
            .filter(|a| a.id() == 60)
            .map(|a| channel(a, "speed", 0.0))
            .unwrap_or_else(|| Some(zero.clone()))?,
        movement_multiplier: movement
            .filter(|a| a.id() == 60)
            .map(|a| channel(a, "speed_multiplier", 1.0))
            .unwrap_or_else(|| channel(modifier, "unused", 1.0))?,
        acceleration_multiplier: movement
            .filter(|a| a.id() == 55)
            .map(|a| channel(a, "acceleration_multiplier", 1.0))
            .unwrap_or_else(|| channel(modifier, "unused", 1.0))?,
    })
}

#[derive(Default)]
struct QuadBatch {
    positions: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    uv: Vec<[f32; 2]>,
    indices: Vec<u32>,
    thresholds: Vec<[f32; 4]>,
}
impl QuadBatch {
    fn mesh(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uv)
        .with_inserted_attribute(Mesh::ATTRIBUTE_TANGENT, self.thresholds)
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            vec![[0.0, 0.0, 1.0]; self.indices.len() / 6 * 4],
        )
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct FxrMaterial {
    #[texture(0)]
    #[sampler(1)]
    texture: Handle<Image>,
    additive: bool,
}
impl Material for FxrMaterial {
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn alpha_mode(&self) -> AlphaMode {
        if self.additive {
            AlphaMode::Add
        } else {
            AlphaMode::Blend
        }
    }
    fn vertex_shader() -> ShaderRef {
        bevy::asset::AssetPath::from_path_buf(embedded_path!("fxr.wgsl"))
            .with_source("embedded")
            .into()
    }
    fn fragment_shader() -> ShaderRef {
        Self::vertex_shader()
    }
    fn specialize(
        _: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _: &MeshVertexBufferLayoutRef,
        _: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        if let Some(depth) = &mut descriptor.depth_stencil {
            depth.depth_write_enabled = Some(false);
        }
        Ok(())
    }
}

fn render(
    mut commands: Commands,
    mut player: ResMut<FxrPlayer>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<FxrMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(camera) = camera.iter().next() else {
        return;
    };
    let _timer = crate::perf_log::SystemTimer::start("fxr");
    let mut batches: HashMap<(PathBuf, bool), QuadBatch> = HashMap::new();
    for p in &player.particles {
        let a = &p.template.appearance;
        let key = (a.texture.clone(), a.blend == Blend::Additive);
        let width = a.width.scalar(p.age) * p.template.scale[0].scalar(p.emitted);
        let height = if a.uniform {
            width
        } else {
            a.height.scalar(p.age) * p.template.scale[1].scalar(p.emitted)
        };
        let angle = -(a.rotation.scalar(0.0) + p.spin).to_radians();
        let (sin, cos) = angle.sin_cos();
        let (right, up) = match a.orientation {
            0 | 3 => (Vec3::X, Vec3::Y),
            2 => (p.rotation * Vec3::X, p.rotation * Vec3::Y),
            4 | 7 => {
                let up = if a.orientation == 7 {
                    p.rotation * Vec3::Y
                } else {
                    Vec3::Y
                };
                (
                    up.cross(camera.translation() - p.position)
                        .normalize_or(Vec3::X),
                    up,
                )
            }
            _ => (camera.right().as_vec3(), camera.up().as_vec3()),
        };
        let horizontal = (right * cos + up * sin) * width * 0.5;
        let vertical = (up * cos - right * sin) * height * 0.5;
        let c = [
            a.color[0].at(p.age),
            a.color[1].at(p.emitted),
            a.color[2].at(p.age),
        ];
        let mut color: [f32; 4] = std::array::from_fn(|i| {
            c[0][i].clamp(0.0, 1.0)
                * c[1][i].clamp(0.0, 1.0)
                * c[2][i]
                * p.template.color.at(p.emitted)[i]
        });
        for lane in &mut color[..3] {
            *lane *= a.brightness.scalar(p.age + p.emitted) * crate::lighting::ENV_PER_UNIT;
        }
        color[3] *= a.alpha.scalar(p.age + p.emitted);
        if color.iter().any(|v| !v.is_finite()) || !width.is_finite() || !height.is_finite() {
            continue;
        }
        let frame =
            (a.frame.scalar(p.age) + a.frame_offset.scalar(p.age)).max(0.0) as u32 % a.frames;
        let rows = a.frames.div_ceil(a.columns);
        let batch = batches.entry(key).or_default();
        let base = batch.positions.len() as u32;
        for (corner, uv) in [
            (-horizontal - vertical, [0.0, 1.0]),
            (horizontal - vertical, [1.0, 1.0]),
            (horizontal + vertical, [1.0, 0.0]),
            (-horizontal + vertical, [0.0, 0.0]),
        ] {
            let offset = p.rotation
                * Vec3::new(
                    -a.offset[0].scalar(p.age) * width,
                    a.offset[1].scalar(p.age) * height,
                    a.offset[2].scalar(p.age),
                );
            batch
                .positions
                .push((p.position + offset + corner).to_array());
            batch.colors.push(color);
            batch.thresholds.push([
                a.fade.scalar(p.age) / 255.0,
                a.threshold.scalar(p.age + p.emitted) / 255.0,
                0.0,
                0.0,
            ]);
            batch.uv.push([
                (frame % a.columns) as f32 / a.columns as f32 + uv[0] / a.columns as f32,
                (frame / a.columns) as f32 / rows as f32 + uv[1] / rows as f32,
            ]);
        }
        batch
            .indices
            .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    for (key, batch) in batches {
        let mesh = batch.mesh();
        if let Some((handle, entity)) = player.batches.get(&key) {
            commands.entity(*entity).insert(Visibility::Visible);
            if let Some(mut existing) = meshes.get_mut(handle) {
                *existing = mesh;
            }
        } else {
            let texture = std::fs::read(&key.0).ok().and_then(|data| {
                sekiro_formats::tpf::parse(&data).ok().and_then(|t| {
                    t.first().and_then(|t| {
                        t.header().ok().and_then(|_| {
                            Image::from_buffer(
                                t.dds,
                                bevy::image::ImageType::Extension("dds"),
                                bevy::image::CompressedImageFormats::all(),
                                false,
                                bevy::image::ImageSampler::default(),
                                RenderAssetUsages::default(),
                            )
                            .ok()
                        })
                    })
                })
            });
            let Some(texture) = texture else {
                warn!("FXR texture cannot be decoded: {}", key.0.display());
                continue;
            };
            let material = materials.add(FxrMaterial {
                texture: images.add(texture),
                additive: key.1,
            });
            let handle = meshes.add(mesh);
            let entity = commands
                .spawn((
                    Mesh3d(handle.clone()),
                    MeshMaterial3d(material),
                    Transform::IDENTITY,
                    bevy::camera::visibility::NoFrustumCulling,
                    bevy::light::NotShadowCaster,
                    bevy::light::NotShadowReceiver,
                ))
                .id();
            player.batches.insert(key, (handle, entity));
        }
    }
    // Keep the last allocation and hide inactive batches. Zero-byte mesh uploads are not
    // supported by Bevy's slab allocator and repeatedly copying them reports freed keys.
    let live: Vec<_> = player
        .particles
        .iter()
        .map(|p| {
            (
                p.template.appearance.texture.clone(),
                p.template.appearance.blend == Blend::Additive,
            )
        })
        .collect();
    for (key, (_, entity)) in &player.batches {
        if !live.contains(key) {
            commands.entity(*entity).insert(Visibility::Hidden);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn constant(value: f32) -> Channel {
        Channel {
            value: sekiro_formats::fxr::Property {
                tag: 0x23,
                evaluator: 0,
                values: [value.to_bits(); 4].to_vec(),
                modifiers: 0..0,
            }
            .compile_native_unmodified()
            .unwrap(),
        }
    }

    fn test_template() -> Arc<Template> {
        Arc::new(Template {
            appearance: Billboard {
                texture: PathBuf::from("test.tpf"),
                blend: Blend::Alpha,
                width: constant(1.0),
                height: constant(1.0),
                color: std::array::from_fn(|_| constant(1.0)),
                brightness: constant(1.0),
                alpha: constant(1.0),
                rotation: constant(0.0),
                angular: constant(90.0),
                angular_multiplier: constant(1.0),
                frame: constant(0.0),
                frame_offset: constant(0.0),
                columns: 1,
                frames: 1,
                uniform: false,
                orientation: 1,
                offset: std::array::from_fn(|_| constant(0.0)),
                fade: constant(0.0),
                threshold: constant(0.0),
            },
            delay: 0.05,
            duration: 0.3,
            lifetime: constant(0.1),
            interval: constant(0.02),
            count: constant(3.0),
            total: constant(2.0),
            concurrent: constant(2.0),
            periodic: true,
            speed: constant(1.0),
            gravity: constant(10.0),
            acceleration: constant(0.0),
            transform: Transform::IDENTITY,
            attached: true,
            scale: [constant(1.0), constant(1.0)],
            color: constant(1.0),
            direction: Vec3::Z,
            movement_speed: constant(0.0),
            movement_multiplier: constant(1.0),
            acceleration_multiplier: constant(1.0),
        })
    }

    #[test]
    fn emission_delay_concurrency_attachment_and_expiry_are_bounded() {
        let mut player = FxrPlayer::new(PathBuf::from("missing-fxr-test-cache"));
        player.templates.insert(1, vec![test_template()]);
        let key = (0, 1, 0);
        player.request(1, key, Transform::IDENTITY, true);
        player.advance(0.02);
        assert!(player.particles.is_empty());
        player.advance(0.031);
        assert_eq!(player.particles.len(), 2);
        player.request(1, key, Transform::from_translation(Vec3::X), false);
        assert!(player.particles.iter().all(|p| p.position.x == 1.0));
        player.advance(0.02);
        assert_eq!(player.particles.len(), 2);
        assert!(
            player
                .particles
                .iter()
                .all(|p| p.position.y < 0.0 && p.spin > 0.0)
        );
        player.advance(0.2);
        assert!(player.particles.is_empty());
        player.advance(0.2);
        assert!(player.emitters.is_empty());
        for _ in 0..EMITTER_CAP + 10 {
            player.request(1, key, Transform::IDENTITY, true);
        }
        assert_eq!(player.emitters.len(), EMITTER_CAP);
        for emitter in &mut player.emitters {
            emitter.template = Arc::new(Template {
                concurrent: constant(-1.0),
                count: constant(256.0),
                ..Arc::try_unwrap(test_template()).ok().unwrap()
            });
        }
        player.advance(0.05);
        assert_eq!(player.particles.len(), PARTICLE_CAP);
    }

    #[test]
    fn installed_supported_emitters_are_reported_without_proprietary_fixtures() {
        let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../cache");
        if !cache.join("raw/sfx").is_dir() {
            return;
        }
        let mut player = FxrPlayer::new(cache);
        let ids: Vec<_> = player.files.keys().copied().collect();
        let mut count = 0;
        for id in ids {
            player.load(id);
            let supported = player.templates[&id].len();
            if supported != 0 {
                count += supported;
                println!("FXR {id}: {supported} supported emitters");
            }
        }
        println!("supported emitter total: {count}");
        assert!(count > 0, "no authored emitters can play");
    }

    #[test]
    fn quad_batches_have_one_triangle_pair_per_quad() {
        let batch = QuadBatch {
            positions: vec![[0.0; 3]; 8],
            colors: vec![[1.0; 4]; 8],
            uv: vec![[0.0; 2]; 8],
            indices: vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7],
            thresholds: vec![[0.0; 4]; 8],
        };
        let mesh = batch.mesh();
        assert_eq!(mesh.count_vertices(), 8);
        assert_eq!(mesh.indices().unwrap().len(), 12);
        assert_eq!(QuadBatch::default().mesh().count_vertices(), 0);
    }
}

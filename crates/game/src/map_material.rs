//! Layered-map renderer using exported sampler metadata and packed vertex weights.
use std::collections::HashMap;

use bevy::asset::{embedded_asset, embedded_path};
use bevy::gltf::GltfMaterialExtras;

/// Uploaded images may have retired their MAIN_WORLD assets and load-state entries.
#[derive(Resource, Clone, Default)]
pub(super) struct SweepImages(
    pub std::sync::Arc<std::sync::Mutex<std::collections::HashSet<AssetId<Image>>>>,
);
use bevy::image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use serde_json::Value;

#[path = "camera_fade.rs"]
mod camera_fade;

type LayeredMaterial = ExtendedMaterial<StandardMaterial, MapLayers>;

/// Retain the native base texture and alpha test in the sweep diagnostic pass.
pub(super) struct SweepMaterial {
    pub key: bevy::asset::UntypedAssetId,
    pub base: StandardMaterial,
    pub missing: bool,
    pub pending: bool,
}

pub(super) fn sweep_base(world: &World, entity: Entity) -> Option<SweepMaterial> {
    let mut textures = Vec::new();
    let (key, base) = if let Some(handle) = world.get::<MeshMaterial3d<LayeredMaterial>>(entity) {
        let material = world.resource::<Assets<LayeredMaterial>>().get(&handle.0)?;
        let layer = &material.extension;
        textures.extend(
            [
                &layer.overlay,
                &layer.snow,
                &layer.c,
                &layer.overlay_normal,
                &layer.snow_normal,
                &layer.c_normal,
                &layer.mask,
                &layer.base_normal,
                &layer.detail,
                &layer.mask_l,
                &layer.wet,
            ]
            .into_iter()
            .filter_map(|h| h.as_ref()),
        );
        (handle.0.id().untyped(), &material.base)
    } else {
        let handle = world.get::<MeshMaterial3d<StandardMaterial>>(entity)?;
        (
            handle.0.id().untyped(),
            world
                .resource::<Assets<StandardMaterial>>()
                .get(&handle.0)?,
        )
    };
    textures.extend(
        [
            &base.base_color_texture,
            &base.normal_map_texture,
            &base.emissive_texture,
            &base.metallic_roughness_texture,
            &base.occlusion_texture,
        ]
        .into_iter()
        .filter_map(|h| h.as_ref()),
    );
    let images = world.resource::<Assets<Image>>();
    let assets = world.resource::<AssetServer>();
    let uploaded = world
        .get_resource::<SweepImages>()
        .map(|images| images.0.lock().unwrap());
    let on_gpu = |h: &Handle<Image>| uploaded.as_ref().is_some_and(|ids| ids.contains(&h.id()));
    let pending = textures.iter().any(|h| {
        images.get(*h).is_none()
            && !on_gpu(h)
            && matches!(
                assets.get_load_state(h.id()),
                None | Some(bevy::asset::LoadState::Loading | bevy::asset::LoadState::NotLoaded)
            )
    });
    // A successfully uploaded RENDER_WORLD-only image may retire its CPU asset.
    let missing = textures
        .iter()
        .any(|h| match assets.get_load_state(h.id()) {
            Some(bevy::asset::LoadState::Failed(_)) => true,
            None => images.get(*h).is_none() && !on_gpu(h),
            _ => false,
        });
    Some(SweepMaterial {
        key,
        base: base.clone(),
        missing: base.base_color_texture.is_none() || missing,
        pending,
    })
}

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct MaterialStats<'w> {
    materials: Res<'w, Assets<LayeredMaterial>>,
}

impl MaterialStats<'_> {
    pub(crate) fn count(&self) -> usize {
        self.materials.len()
    }
}

fn release_materials(
    mut events: MessageReader<AssetEvent<StandardMaterial>>,
    mut removed_actors: RemovedComponents<ActorMaterialSource>,
    active: Query<&MeshMaterial3d<LayeredMaterial>>,
    mut settings: ResMut<LayerSettings>,
) {
    for event in events.read() {
        if let AssetEvent::Removed { id } = event {
            settings.materials.remove(id);
        }
    }
    // Scene assets can retain their source materials after the rig is gone.
    // Drop unused wrappers on actor teardown, without scanning meshes on ordinary frames.
    if removed_actors.read().next().is_some() {
        removed_actors.clear();
        let live: std::collections::HashSet<_> = active.iter().map(|h| h.0.id()).collect();
        settings.materials.retain(|_, h| live.contains(&h.id()));
    }
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[bind_group_data(MapPipelineKey)]
#[data(50, MapLayersUniform, binding_array(101))]
#[bindless(index_table(range(50..80), binding(100)))]
struct MapLayers {
    direct: [Vec4; 13],
    // xyz: enabled albedo layers; w: mask present.
    enabled: Vec4,
    albedo_enabled: Vec4,
    albedo_uv: Vec4,
    normal_uv: Vec4,
    normal_enabled: Vec4,
    hemi_axis: Vec4,
    hemi_delta: Vec4,
    camera_fade: Vec4,
    game: MapGame,
    #[texture(51)]
    #[sampler(52)]
    overlay: Option<Handle<Image>>,
    #[texture(53)]
    #[sampler(54)]
    snow: Option<Handle<Image>>,
    #[texture(55)]
    #[sampler(56)]
    c: Option<Handle<Image>>,
    #[texture(57)]
    #[sampler(58)]
    overlay_normal: Option<Handle<Image>>,
    #[texture(59)]
    #[sampler(60)]
    snow_normal: Option<Handle<Image>>,
    #[texture(61)]
    #[sampler(62)]
    c_normal: Option<Handle<Image>>,
    #[texture(63)]
    #[sampler(64)]
    mask: Option<Handle<Image>>,
    #[storage(65, read_only, binding_array(102))]
    irradiance: Handle<bevy::render::storage::ShaderBuffer>,
    #[texture(66, dimension = "cube_array")]
    #[sampler(67)]
    specular: Option<Handle<Image>>,
    #[texture(68, dimension = "cube_array")]
    #[sampler(69)]
    diffuse: Option<Handle<Image>>,
    #[texture(70, dimension = "3d")]
    #[sampler(71)]
    voxels: Option<Handle<Image>>,
    #[texture(72)]
    #[sampler(73)]
    base_normal: Option<Handle<Image>>,
    #[texture(74)]
    #[sampler(75)]
    detail: Option<Handle<Image>>,
    #[texture(76)]
    #[sampler(77)]
    mask_l: Option<Handle<Image>>,
    #[texture(78)]
    #[sampler(79)]
    wet: Option<Handle<Image>>,
}

#[derive(ShaderType, Reflect, Debug, Clone, Default)]
struct MapGame {
    flags: UVec4,
    p: [Vec4; 16],
}

pub(crate) const ATTR_MAP_BLEND2: bevy::mesh::MeshVertexAttribute =
    bevy::mesh::MeshVertexAttribute::new(
        "MapBlend2",
        0x5ec1_0a01,
        bevy::render::render_resource::VertexFormat::Float32x4,
    );

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct MapPipelineKey {
    camera_fade: bool,
    game: bool,
    native_direct: bool,
}

impl From<&MapLayers> for MapPipelineKey {
    fn from(layers: &MapLayers) -> Self {
        static LEGACY: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        Self {
            game: layers.game.flags.y != 0,
            native_direct: layers.direct[12].x > 0.,
            camera_fade: layers.camera_fade.x > 0.
                || *LEGACY.get_or_init(|| std::env::args().any(|a| a == "--legacy-camera-fade")),
        }
    }
}

// Packed once per material; bindless slabs preserve Bevy's multi-draw batching.
#[derive(Clone, ShaderType)]
struct MapLayersUniform {
    direct: [Vec4; 13],
    enabled: Vec4,
    albedo_enabled: Vec4,
    albedo_uv: Vec4,
    normal_uv: Vec4,
    normal_enabled: Vec4,
    hemi_axis: Vec4,
    hemi_delta: Vec4,
    camera_fade: Vec4,
    game: MapGame,
}

impl From<&MapLayers> for MapLayersUniform {
    fn from(layers: &MapLayers) -> Self {
        Self {
            direct: layers.direct,
            enabled: layers.enabled,
            albedo_enabled: layers.albedo_enabled,
            albedo_uv: layers.albedo_uv,
            normal_uv: layers.normal_uv,
            normal_enabled: layers.normal_enabled,
            hemi_axis: layers.hemi_axis,
            hemi_delta: layers.hemi_delta,
            camera_fade: layers.camera_fade,
            game: layers.game.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_only_images_are_not_reported_as_missing() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<StandardMaterial>()
            .init_asset::<LayeredMaterial>()
            .init_resource::<SweepImages>();
        let image = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .remove(image.id());
        let material = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color_texture: Some(image.clone()),
                ..default()
            });
        let entity = app.world_mut().spawn(MeshMaterial3d(material)).id();
        assert!(sweep_base(app.world(), entity).unwrap().pending);
        app.world()
            .resource::<SweepImages>()
            .0
            .lock()
            .unwrap()
            .insert(image.id());
        let status = sweep_base(app.world(), entity).unwrap();
        assert!(!status.missing);
        assert!(!status.pending);
    }

    #[test]
    fn map_extension_preserves_standard_material_bindless_batching() {
        use bevy::render::render_resource::AsBindGroup;
        assert!(<LayeredMaterial as AsBindGroup>::bindless_slot_count().is_some());
    }

    #[test]
    fn hemisphere_wraps_unlayered_map_surfaces_but_preserves_actor_materials() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<StandardMaterial>()
            .init_asset::<LayeredMaterial>()
            .insert_resource(crate::lighting::MapLighting {
                gparam: Default::default(),
                hour: 12.,
                light_set: 0,
                applied: None,
                source: String::new(),
            })
            .insert_resource(bevy::light::GlobalAmbientLight::default())
            .insert_resource(LayerSettings {
                prefix: String::new(),
                enabled: true,
                debug: false,
                materials: HashMap::new(),
                camera_fade: camera_fade::CameraFade::load(std::path::Path::new(""), false),
            })
            .add_systems(Update, (release_materials, replace_materials).chain())
            .add_systems(PostUpdate, sync_actor_materials);
        let original = StandardMaterial {
            unlit: true,
            base_color: Color::srgb(0.2, 0.3, 0.4),
            ..default()
        };
        let handle = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(original.clone());
        let root = app.world_mut().spawn(super::super::DrawGroups([0; 8])).id();
        let map = app
            .world_mut()
            .spawn((MeshMaterial3d(handle.clone()), ChildOf(root)))
            .id();
        let actor = app.world_mut().spawn(MeshMaterial3d(handle.clone())).id();
        let rig = app
            .world_mut()
            .spawn(crate::character::Rig::new(std::sync::Arc::new(
                sekiro_formats::anim::Skeleton {
                    name: String::new(),
                    bones: vec![],
                },
            )))
            .id();
        let wolf = app
            .world_mut()
            .spawn((MeshMaterial3d(handle.clone()), ChildOf(rig)))
            .id();
        app.update();
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(actor)
                .unwrap()
                .0,
            handle
        );
        assert!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(map)
                .is_none()
        );
        let wrapped = app
            .world()
            .get::<MeshMaterial3d<LayeredMaterial>>(map)
            .unwrap();
        let material = app
            .world()
            .resource::<Assets<LayeredMaterial>>()
            .get(&wrapped.0)
            .unwrap();
        assert!(material.base.unlit);
        assert_eq!(material.base.base_color, original.base_color);
        assert_eq!(material.extension.albedo_enabled, Vec4::ZERO);
        assert_eq!(material.extension.hemi_axis.xyz(), Vec3::Y);
        assert!(!MapPipelineKey::from(&material.extension).camera_fade);
        let mut blind = material.extension.clone();
        blind.camera_fade = Vec4::new(1., 1., 1. / 0.3, 0.);
        assert!(MapPipelineKey::from(&blind).camera_fade);
        assert!(app.world().get::<ActorMaterialSource>(wolf).is_some());
        app.world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .get_mut(&handle)
            .unwrap()
            .emissive = LinearRgba::rgb(0.15, 0.4, 0.9);
        app.update();
        let wrapped = &app
            .world()
            .get::<MeshMaterial3d<LayeredMaterial>>(wolf)
            .unwrap()
            .0;
        assert_eq!(
            app.world()
                .resource::<Assets<LayeredMaterial>>()
                .get(wrapped)
                .unwrap()
                .base
                .emissive,
            LinearRgba::rgb(0.15, 0.4, 0.9)
        );
        let mut proxies = app
            .world_mut()
            .query_filtered::<Entity, With<MaterialTintProxy>>();
        assert_eq!(proxies.iter(app.world()).count(), 1);
        app.update();
        assert_eq!(proxies.iter(app.world()).count(), 1);
        for fight in 0..20 {
            let source = app
                .world_mut()
                .resource_mut::<Assets<StandardMaterial>>()
                .add(original.clone());
            let root = app
                .world_mut()
                .spawn(crate::character::Rig::new(std::sync::Arc::new(
                    sekiro_formats::anim::Skeleton {
                        name: String::new(),
                        bones: vec![],
                    },
                )))
                .id();
            app.world_mut()
                .spawn((MeshMaterial3d(source.clone()), ChildOf(root)));
            app.update();
            assert_eq!(app.world().resource::<LayerSettings>().materials.len(), 2);
            app.world_mut().despawn(root);
            app.world_mut()
                .resource_mut::<Assets<StandardMaterial>>()
                .remove(source.id());
            for _ in 0..3 {
                app.update();
            }
            assert_eq!(
                app.world().resource::<LayerSettings>().materials.len(),
                1,
                "fight {fight}: wrapper survived source teardown"
            );
        }
    }
}

impl MaterialExtension for MapLayers {
    fn specialize(
        pipeline: &bevy::pbr::MaterialExtensionPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        key: bevy::pbr::MaterialExtensionKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        use bevy::shader::ShaderDefVal;
        let prepass = descriptor.vertex.shader_defs.iter().any(|d| {
            matches!(d,
            ShaderDefVal::Bool(n, true) if n == "PREPASS_PIPELINE")
        });
        if !key.bind_group_data.game && !prepass {
            descriptor.vertex.shader = pipeline.mesh_pipeline.shader.clone();
        }
        if key.bind_group_data.native_direct
            && !prepass
            && let Some(fragment) = descriptor.fragment.as_mut()
        {
            fragment.shader_defs.push("NATIVE_DIRECT".into());
        }
        if key.bind_group_data.game {
            if let Some(fragment) = descriptor.fragment.as_mut() {
                fragment.shader_defs.push("GAME_MATERIAL".into());
            }
            if !prepass {
                descriptor.vertex.shader_defs.push("GAME_LAYERS".into());
                if let Some(fragment) = descriptor.fragment.as_mut() {
                    fragment.shader_defs.push("GAME_LAYERS".into());
                }
                let mut attrs = vec![
                    Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
                    Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
                ];
                for (a, at) in [
                    (Mesh::ATTRIBUTE_UV_0, 2),
                    (Mesh::ATTRIBUTE_UV_1, 3),
                    (Mesh::ATTRIBUTE_TANGENT, 4),
                    (Mesh::ATTRIBUTE_COLOR, 5),
                ] {
                    if layout.0.contains(a) {
                        attrs.push(a.at_shader_location(at));
                    }
                }
                attrs.push(ATTR_MAP_BLEND2.at_shader_location(8));
                descriptor.vertex.buffers = vec![layout.0.get_layout(&attrs)?];
            }
        }
        if key.bind_group_data.camera_fade
            && (!prepass
                || key.mesh_key.intersects(
                    bevy::pbr::MeshPipelineKey::MOTION_VECTOR_PREPASS
                        | bevy::pbr::MeshPipelineKey::NORMAL_PREPASS,
                ))
            && let Some(fragment) = descriptor.fragment.as_mut()
        {
            fragment.shader_defs.push("CAMERA_FADE".into());
        }
        Ok(())
    }

    fn vertex_shader() -> ShaderRef {
        shader_path("map_material.wgsl")
    }

    fn fragment_shader() -> ShaderRef {
        shader_path("map_material.wgsl")
    }

    fn prepass_fragment_shader() -> ShaderRef {
        shader_path("map_prepass.wgsl")
    }
}

fn shader_path(name: &str) -> ShaderRef {
    let path = embedded_path!("map_material.wgsl").with_file_name(name);
    ShaderRef::Path(
        bevy::asset::AssetPath::parse(&format!("embedded://{}", path.to_string_lossy()))
            .into_owned(),
    )
}

#[derive(Resource)]
struct LayerSettings {
    prefix: String,
    enabled: bool,
    debug: bool,
    materials: HashMap<AssetId<StandardMaterial>, Handle<LayeredMaterial>>,
    camera_fade: camera_fade::CameraFade,
}

#[derive(bevy::ecs::system::SystemParam)]
struct MapEnvironment<'w, 's> {
    lighting: Res<'w, crate::lighting::MapLighting>,
    ambient: Res<'w, bevy::light::GlobalAmbientLight>,
    parents: Query<'w, 's, &'static ChildOf>,
    roots: Query<'w, 's, (), With<super::DrawGroups>>,
    actors: Query<'w, 's, (), With<crate::character::Rig>>,
    irradiance: Option<Res<'w, crate::lighting::MapIrradiance>>,
}

/// Retain actor material sources for gameplay tint, while drawing the native ambient wrapper.
#[derive(Component)]
struct ActorMaterialSource(Handle<StandardMaterial>);

#[derive(Component)]
struct MaterialTintProxy;

fn sync_actor_materials(
    sources: Res<Assets<StandardMaterial>>,
    mut materials: ResMut<Assets<LayeredMaterial>>,
    actors: Query<(&ActorMaterialSource, &MeshMaterial3d<LayeredMaterial>)>,
) {
    for (source, target) in &actors {
        if let Some(base) = sources.get(&source.0)
            && let Some(mut target) = materials.get_mut(&target.0)
            && (target.base.base_color != base.base_color || target.base.emissive != base.emissive)
        {
            target.base.base_color = base.base_color;
            target.base.emissive = base.emissive;
        }
    }
}

pub(super) fn install(app: &mut App, prefix: &str, dir: &std::path::Path, args: &[String]) {
    embedded_asset!(app, "map_material.wgsl");
    embedded_asset!(app, "map_prepass.wgsl");
    bevy::shader::load_shader_library!(app, "map_bindings.wgsl");
    bevy::shader::load_shader_library!(app, "map_coverage.wgsl");
    app.add_plugins(MaterialPlugin::<LayeredMaterial>::default())
        .insert_resource(LayerSettings {
            prefix: prefix.to_owned(),
            enabled: !args.iter().any(|s| s == "--no-layers"),
            debug: args.iter().any(|s| s == "--show-blend"),
            materials: HashMap::new(),
            camera_fade: camera_fade::CameraFade::load(
                dir,
                !args.iter().any(|s| s == "--no-camera-fade"),
            ),
        })
        .add_systems(
            Update,
            (release_materials, replace_materials, update_hemisphere)
                .chain()
                .after(crate::lighting::apply_lighting)
                .after(crate::lighting::apply_probes),
        )
        .add_systems(PostUpdate, sync_actor_materials);
}

fn update_hemisphere(
    lighting: Res<crate::lighting::MapLighting>,
    ambient: Res<bevy::light::GlobalAmbientLight>,
    mut materials: ResMut<Assets<LayeredMaterial>>,
    mut last: Local<Option<(i32, f32, bool, String)>>,
) {
    let _timer = crate::perf_log::SystemTimer::start("update_hemisphere");
    let key = (
        lighting.light_set,
        lighting.hour,
        ambient.brightness > 0.,
        lighting.source.clone(),
    );
    if last.as_ref() == Some(&key) {
        return;
    }
    *last = Some(key);
    let set =
        crate::lighting::LightSet::from_gparam(&lighting.gparam, lighting.light_set, lighting.hour);
    let axis = set.hemi_axis.extend(f32::from(ambient.brightness > 0.));
    let delta =
        (set.hemi_delta * crate::lighting::ENV_PER_UNIT).extend(crate::lighting::ENV_PER_UNIT);
    for (_, material) in materials.iter_mut() {
        material.extension.direct = set.direct;
        if material.extension.hemi_axis != axis || material.extension.hemi_delta != delta {
            material.extension.hemi_axis = axis;
            material.extension.hemi_delta = delta;
        }
    }
}

#[allow(clippy::type_complexity)]
fn replace_materials(
    mut commands: Commands,
    assets: Res<AssetServer>,
    materials: Res<Assets<StandardMaterial>>,
    mut layered: ResMut<Assets<LayeredMaterial>>,
    meshes: Query<
        (
            Entity,
            &MeshMaterial3d<StandardMaterial>,
            Option<&GltfMaterialExtras>,
            Option<&bevy::gltf::GltfMaterialName>,
        ),
        Without<MaterialTintProxy>,
    >,
    mut settings: ResMut<LayerSettings>,
    environment: MapEnvironment,
) {
    let _timer = crate::perf_log::SystemTimer::start("replace_materials");
    // Experimental ablation only. The render gate rejects this override.
    let route_materials = std::env::args().any(|a| a == "--test-material-routing");
    let MapEnvironment {
        lighting,
        ambient,
        parents,
        roots,
        actors,
        irradiance,
    } = environment;
    let set =
        crate::lighting::LightSet::from_gparam(&lighting.gparam, lighting.light_set, lighting.hour);
    for (entity, original, extras, name) in &meshes {
        // Map and rig descendants share spatial lighting. Actor tint proxies retain
        // the StandardMaterial handles used by gameplay; unrelated FX stay untouched.
        let mut ancestor = entity;
        let mut map_surface = roots.contains(ancestor);
        let mut actor_surface = actors.contains(ancestor);
        while !map_surface && !actor_surface {
            let Ok(parent) = parents.get(ancestor) else {
                break;
            };
            ancestor = parent.parent();
            map_surface = roots.contains(ancestor);
            actor_surface = actors.contains(ancestor);
        }
        if !map_surface && !actor_surface {
            continue;
        }
        // All map surfaces need the hemisphere, even without texture layers.
        let Some(base) = materials.get(&original.0) else {
            continue;
        };
        let handle = if let Some(handle) = settings.materials.get(&original.0.id()) {
            handle.clone()
        } else {
            let extras = extras
                .and_then(|e| serde_json::from_str::<Value>(&e.value).ok())
                .unwrap_or(Value::Null);
            let data = &extras["sekiro_layers"];
            let camera_fade = if map_surface {
                settings
                    .camera_fade
                    .material(name.map_or("", |name| name.0.as_str()))
            } else {
                Vec4::ZERO
            };
            let load = |key: &str, srgb: bool| -> Option<Handle<Image>> {
                if !settings.enabled {
                    return None;
                }
                let uri = data[key]["uri"].as_str()?;
                // Exported paths are relative to models/*.glb, rooted in this map only.
                if !uri.starts_with("tex/") || uri.contains("..") {
                    return None;
                }
                let loader = assets.load_builder();
                let loader = if route_materials {
                    loader.override_unapproved()
                } else {
                    loader
                };
                Some(
                    loader
                        .with_settings(move |s: &mut ImageLoaderSettings| {
                            s.is_srgb = srgb;
                            s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                                address_mode_u: ImageAddressMode::Repeat,
                                address_mode_v: ImageAddressMode::Repeat,
                                anisotropy_clamp: 8,
                                ..ImageSamplerDescriptor::linear()
                            });
                        })
                        .load(
                            route_materials
                                .then(|| {
                                    super::world::material_texture_path(&assets, &original.0, uri)
                                })
                                .flatten()
                                .unwrap_or_else(|| format!("{}models/{uri}", settings.prefix)),
                        ),
                )
            };
            let uv = |key: &str| data[key]["uv"].as_u64().unwrap_or(0) as f32;
            let present = |key: &str| f32::from(data[key]["uri"].is_string() && settings.enabled);
            let mut game = MapGame::default();
            if settings.enabled && data["version"].as_u64() == Some(2) {
                game.flags.y = data["kind"].as_u64().unwrap_or(0) as u32;
                game.flags.z = u32::from(settings.debug);
                for (key, bit) in [
                    ("base_normal", 1),
                    ("overlay", 2),
                    ("overlay_normal", 4),
                    ("snow", 8),
                    ("snow_normal", 16),
                    ("c", 32),
                    ("c_normal", 64),
                    ("mask", 128),
                    ("detail", 256),
                    ("mask_l", 512),
                    ("wet", 1024),
                ] {
                    if present(key) > 0. {
                        game.flags.x |= bit;
                    }
                }
                if base.base_color_texture.is_some() {
                    game.flags.x |= 2048;
                }
                for (i, p) in game.p.iter_mut().enumerate() {
                    *p = Vec4::from_array(std::array::from_fn(|c| {
                        data["params"][i][c].as_f64().unwrap_or(0.) as f32
                    }));
                }
            }
            let extension = MapLayers {
                game,
                enabled: Vec4::new(
                    present("overlay").max(present("overlay_normal")),
                    present("snow").max(present("snow_normal")),
                    present("c").max(present("c_normal")),
                    present("mask"),
                ),
                albedo_enabled: Vec4::new(
                    present("overlay"),
                    present("snow"),
                    present("c"),
                    f32::from(data.is_object()),
                ),
                albedo_uv: Vec4::new(uv("overlay"), uv("snow"), uv("c"), uv("mask")),
                normal_uv: Vec4::new(uv("overlay_normal"), uv("snow_normal"), uv("c_normal"), 0.),
                normal_enabled: Vec4::new(
                    present("overlay_normal"),
                    present("snow_normal"),
                    present("c_normal"),
                    f32::from(settings.debug),
                ),
                direct: set.direct,
                hemi_axis: set.hemi_axis.extend(f32::from(ambient.brightness > 0.)),
                hemi_delta: (set.hemi_delta * crate::lighting::ENV_PER_UNIT)
                    .extend(crate::lighting::ENV_PER_UNIT),
                camera_fade,
                overlay: load("overlay", true),
                snow: load("snow", true),
                c: load("c", true),
                overlay_normal: load("overlay_normal", false),
                snow_normal: load("snow_normal", false),
                c_normal: load("c_normal", false),
                mask: load("mask", false),
                base_normal: load("base_normal", false),
                detail: load("detail", false),
                mask_l: load("mask_l", false),
                wet: load("wet", false),
                irradiance: irradiance
                    .as_ref()
                    .map(|i| i.buffer.clone())
                    .unwrap_or_default(),
                specular: irradiance.as_ref().map(|i| i.specular.clone()),
                diffuse: irradiance.as_ref().map(|i| i.diffuse.clone()),
                voxels: irradiance.as_ref().map(|i| i.voxels.clone()),
            };
            let handle = layered.add(ExtendedMaterial {
                base: base.clone(),
                extension,
            });
            settings.materials.insert(original.0.id(), handle.clone());
            handle
        };
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert(MeshMaterial3d(handle));
        if actor_surface {
            commands
                .entity(entity)
                .insert(ActorMaterialSource(original.0.clone()));
            commands.spawn((
                MaterialTintProxy,
                MeshMaterial3d(original.0.clone()),
                ChildOf(ancestor),
            ));
        }
    }
}

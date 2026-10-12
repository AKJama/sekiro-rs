//! Original layered-map renderer using exported sampler metadata and packed vertex weights.
use std::collections::HashMap;

use bevy::asset::{embedded_asset, embedded_path};
use bevy::gltf::GltfMaterialExtras;
use bevy::image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use serde_json::Value;

#[path = "camera_fade.rs"]
mod camera_fade;

type LayeredMaterial = ExtendedMaterial<StandardMaterial, MapLayers>;

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[data(50, MapLayersUniform, binding_array(101))]
#[bindless(index_table(range(50..72), binding(100)))]
struct MapLayers {
    // xyz: enabled albedo layers; w: mask present.
    enabled: Vec4,
    albedo_enabled: Vec4,
    albedo_uv: Vec4,
    normal_uv: Vec4,
    normal_enabled: Vec4,
    hemi_axis: Vec4,
    hemi_delta: Vec4,
    camera_fade: Vec4,
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
}

// Packed once per material; bindless slabs preserve Bevy's multi-draw batching.
#[derive(Clone, ShaderType)]
struct MapLayersUniform {
    enabled: Vec4,
    albedo_enabled: Vec4,
    albedo_uv: Vec4,
    normal_uv: Vec4,
    normal_enabled: Vec4,
    hemi_axis: Vec4,
    hemi_delta: Vec4,
    camera_fade: Vec4,
}

impl From<&MapLayers> for MapLayersUniform {
    fn from(layers: &MapLayers) -> Self {
        Self {
            enabled: layers.enabled,
            albedo_enabled: layers.albedo_enabled,
            albedo_uv: layers.albedo_uv,
            normal_uv: layers.normal_uv,
            normal_enabled: layers.normal_enabled,
            hemi_axis: layers.hemi_axis,
            hemi_delta: layers.hemi_delta,
            camera_fade: layers.camera_fade,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            .add_systems(Update, replace_materials)
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
    }
}

impl MaterialExtension for MapLayers {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            bevy::asset::AssetPath::parse(&format!(
                "embedded://{}",
                embedded_path!("map_material.wgsl").to_string_lossy()
            ))
            .into_owned(),
        )
    }
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
            (replace_materials, update_hemisphere)
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
    mut last: Local<Option<(i32, f32, bool)>>,
) {
    let _timer = crate::perf_log::SystemTimer::start("update_hemisphere");
    let key = (lighting.light_set, lighting.hour, ambient.brightness > 0.);
    if *last == Some(key) {
        return;
    }
    *last = Some(key);
    let set =
        crate::lighting::LightSet::from_gparam(&lighting.gparam, lighting.light_set, lighting.hour);
    let axis = set.hemi_axis.extend(f32::from(ambient.brightness > 0.));
    let delta =
        (set.hemi_delta * crate::lighting::ENV_PER_UNIT).extend(crate::lighting::ENV_PER_UNIT);
    for (_, material) in materials.iter_mut() {
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
        let extras = extras
            .and_then(|e| serde_json::from_str::<Value>(&e.value).ok())
            .unwrap_or(Value::Null);
        let data = &extras["sekiro_layers"];
        let Some(base) = materials.get(&original.0) else {
            continue;
        };
        let handle = if let Some(handle) = settings.materials.get(&original.0.id()) {
            handle.clone()
        } else {
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
                Some(
                    assets
                        .load_builder()
                        .with_settings(move |s: &mut ImageLoaderSettings| {
                            s.is_srgb = srgb;
                            s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                                address_mode_u: ImageAddressMode::Repeat,
                                address_mode_v: ImageAddressMode::Repeat,
                                anisotropy_clamp: 8,
                                ..ImageSamplerDescriptor::linear()
                            });
                        })
                        .load(format!("{}models/{uri}", settings.prefix)),
                )
            };
            let uv = |key: &str| data[key]["uv"].as_u64().unwrap_or(0) as f32;
            let present = |key: &str| f32::from(data[key]["uri"].is_string() && settings.enabled);
            let extension = MapLayers {
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

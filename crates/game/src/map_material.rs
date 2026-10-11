//! Original layered-map renderer using exported sampler metadata and packed vertex weights.
use std::collections::HashMap;

use bevy::asset::{embedded_asset, embedded_path};
use bevy::gltf::GltfMaterialExtras;
use bevy::image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use serde_json::Value;

type LayeredMaterial = ExtendedMaterial<StandardMaterial, MapLayers>;

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
struct MapLayers {
    // xyz: enabled albedo layers; w: mask present.
    #[uniform(100)]
    enabled: Vec4,
    #[uniform(100)]
    albedo_enabled: Vec4,
    #[uniform(100)]
    albedo_uv: Vec4,
    #[uniform(100)]
    normal_uv: Vec4,
    #[uniform(100)]
    normal_enabled: Vec4,
    #[uniform(100)]
    hemi_axis: Vec4,
    #[uniform(100)]
    hemi_delta: Vec4,
    #[texture(101)]
    #[sampler(102)]
    overlay: Option<Handle<Image>>,
    #[texture(103)]
    #[sampler(104)]
    snow: Option<Handle<Image>>,
    #[texture(105)]
    #[sampler(106)]
    c: Option<Handle<Image>>,
    #[texture(107)]
    #[sampler(108)]
    overlay_normal: Option<Handle<Image>>,
    #[texture(109)]
    #[sampler(110)]
    snow_normal: Option<Handle<Image>>,
    #[texture(111)]
    #[sampler(112)]
    c_normal: Option<Handle<Image>>,
    #[texture(113)]
    #[sampler(114)]
    mask: Option<Handle<Image>>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
            })
            .add_systems(Update, replace_materials);
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
}

#[derive(bevy::ecs::system::SystemParam)]
struct MapEnvironment<'w, 's> {
    lighting: Res<'w, crate::lighting::MapLighting>,
    ambient: Res<'w, bevy::light::GlobalAmbientLight>,
    parents: Query<'w, 's, &'static ChildOf>,
    roots: Query<'w, 's, (), With<super::DrawGroups>>,
}

pub(super) fn install(app: &mut App, prefix: &str, args: &[String]) {
    embedded_asset!(app, "map_material.wgsl");
    app.add_plugins(MaterialPlugin::<LayeredMaterial>::default())
        .insert_resource(LayerSettings {
            prefix: prefix.to_owned(),
            enabled: !args.iter().any(|s| s == "--no-layers"),
            debug: args.iter().any(|s| s == "--show-blend"),
            materials: HashMap::new(),
        })
        .add_systems(
            Update,
            (replace_materials, update_hemisphere)
                .chain()
                .after(crate::lighting::apply_lighting)
                .after(crate::lighting::apply_probes),
        );
}

fn update_hemisphere(
    lighting: Res<crate::lighting::MapLighting>,
    ambient: Res<bevy::light::GlobalAmbientLight>,
    mut materials: ResMut<Assets<LayeredMaterial>>,
    mut last: Local<Option<(i32, f32, bool)>>,
) {
    let key = (lighting.light_set, lighting.hour, ambient.brightness > 0.);
    if *last == Some(key) {
        return;
    }
    *last = Some(key);
    let set =
        crate::lighting::LightSet::from_gparam(&lighting.gparam, lighting.light_set, lighting.hour);
    let axis = set.hemi_axis.extend(f32::from(ambient.brightness > 0.));
    let delta = (set.hemi_delta * crate::lighting::ENV_PER_UNIT).extend(0.);
    for (_, material) in materials.iter_mut() {
        if material.extension.hemi_axis != axis || material.extension.hemi_delta != delta {
            material.extension.hemi_axis = axis;
            material.extension.hemi_delta = delta;
        }
    }
}

fn replace_materials(
    mut commands: Commands,
    assets: Res<AssetServer>,
    materials: Res<Assets<StandardMaterial>>,
    mut layered: ResMut<Assets<LayeredMaterial>>,
    meshes: Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        Option<&GltfMaterialExtras>,
    )>,
    mut settings: ResMut<LayerSettings>,
    environment: MapEnvironment,
) {
    let MapEnvironment {
        lighting,
        ambient,
        parents,
        roots,
    } = environment;
    let set =
        crate::lighting::LightSet::from_gparam(&lighting.gparam, lighting.light_set, lighting.hour);
    for (entity, original, extras) in &meshes {
        // Limit replacement to map descendants. Actor/FX material systems retain
        // their StandardMaterial handles for animation and gameplay-driven tint.
        let mut ancestor = entity;
        let mut map_surface = roots.contains(ancestor);
        while !map_surface {
            let Ok(parent) = parents.get(ancestor) else {
                break;
            };
            ancestor = parent.parent();
            map_surface = roots.contains(ancestor);
        }
        if !map_surface {
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
                hemi_delta: (set.hemi_delta * crate::lighting::ENV_PER_UNIT).extend(0.),
                overlay: load("overlay", true),
                snow: load("snow", true),
                c: load("c", true),
                overlay_normal: load("overlay_normal", false),
                snow_normal: load("snow_normal", false),
                c_normal: load("c_normal", false),
                mask: load("mask", false),
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
    }
}

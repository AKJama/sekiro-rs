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

pub(super) fn install(app: &mut App, prefix: &str, args: &[String]) {
    embedded_asset!(app, "map_material.wgsl");
    app.add_plugins(MaterialPlugin::<LayeredMaterial>::default())
        .insert_resource(LayerSettings {
            prefix: prefix.to_owned(),
            enabled: !args.iter().any(|s| s == "--no-layers"),
            debug: args.iter().any(|s| s == "--show-blend"),
            materials: HashMap::new(),
        })
        .add_systems(Update, replace_materials);
}

fn replace_materials(
    mut commands: Commands,
    assets: Res<AssetServer>,
    materials: Res<Assets<StandardMaterial>>,
    mut layered: ResMut<Assets<LayeredMaterial>>,
    meshes: Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        &GltfMaterialExtras,
    )>,
    mut settings: ResMut<LayerSettings>,
) {
    for (entity, original, extras) in &meshes {
        let Ok(extras) = serde_json::from_str::<Value>(&extras.value) else {
            continue;
        };
        let Some(data) = extras.get("sekiro_layers") else {
            continue;
        };
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
                albedo_enabled: Vec4::new(present("overlay"), present("snow"), present("c"), 0.),
                albedo_uv: Vec4::new(uv("overlay"), uv("snow"), uv("c"), uv("mask")),
                normal_uv: Vec4::new(uv("overlay_normal"), uv("snow_normal"), uv("c_normal"), 0.),
                normal_enabled: Vec4::new(
                    present("overlay_normal"),
                    present("snow_normal"),
                    present("c_normal"),
                    f32::from(settings.debug),
                ),
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

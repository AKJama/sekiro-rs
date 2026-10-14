//! Authored landscape fog cards, independent of the volumetric fog pass.
use std::collections::HashMap;

use bevy::{
    asset::{embedded_asset, embedded_path},
    gltf::GltfMaterialExtras,
    image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    light::{NotShadowCaster, NotShadowReceiver},
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::render_resource::{
        AsBindGroup, Face, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};
use serde_json::Value;

#[derive(Clone, Debug, ShaderType)]
struct Inputs {
    colors: [Vec4; 4],
    floats: [Vec4; 14],
    vectors: [Vec4; 18],
    uv: [Vec4; 4],
    control: Vec4,
    sun: Vec4,
    direct: Vec4,
    ambient: Vec4,
    hemi_axis: Vec4,
    hemi_delta: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
#[bind_group_data(FogKey)]
struct FogMaterial {
    #[uniform(0)]
    inputs: Inputs,
    #[texture(1)]
    #[sampler(2)]
    mask: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    flow: Handle<Image>,
    #[texture(5)]
    #[sampler(6)]
    ground: Handle<Image>,
    #[texture(7)]
    #[sampler(8)]
    height: Handle<Image>,
    #[storage(9, read_only)]
    irradiance: Handle<bevy::render::storage::ShaderBuffer>,
    #[texture(10, dimension = "3d")]
    #[sampler(11)]
    voxels: Option<Handle<Image>>,
    #[texture(12, dimension = "cube_array")]
    #[sampler(13)]
    diffuse: Option<Handle<Image>>,
    double_sided: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct FogKey {
    double_sided: bool,
}
impl From<&FogMaterial> for FogKey {
    fn from(m: &FogMaterial) -> Self {
        Self {
            double_sided: m.double_sided,
        }
    }
}

impl Material for FogMaterial {
    fn fragment_shader() -> ShaderRef {
        bevy::asset::AssetPath::from_path_buf(embedded_path!("map_fog_surface.wgsl"))
            .with_source("embedded")
            .into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn specialize(
        _: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = if key.bind_group_data.double_sided {
            None
        } else {
            Some(Face::Back)
        };
        Ok(())
    }
}

#[derive(Resource)]
struct Settings {
    prefix: String,
    enabled: bool,
    empty_gi: Handle<bevy::render::storage::ShaderBuffer>,
    materials: HashMap<AssetId<StandardMaterial>, Handle<FogMaterial>>,
}

pub(crate) fn install(app: &mut App, prefix: &str) {
    embedded_asset!(app, "map_fog_surface.wgsl");
    let empty_gi = app
        .world_mut()
        .resource_mut::<Assets<bevy::render::storage::ShaderBuffer>>()
        .add(bevy::render::storage::ShaderBuffer::from(vec![0u32; 4]));
    app.add_plugins(MaterialPlugin::<FogMaterial>::default())
        .insert_resource(Settings {
            prefix: prefix.to_owned(),
            enabled: !std::env::args().any(|a| a == "--no-fog-surfaces"),
            empty_gi,
            materials: HashMap::new(),
        })
        // Replace before the ordinary map material wrapper consumes these entities.
        .add_systems(PreUpdate, replace)
        .add_systems(
            PostUpdate,
            update_lighting.after(crate::lighting::apply_lighting),
        );
}

fn inputs(data: &Value) -> Inputs {
    let family = data["family"].as_str().unwrap_or("CloudFog");
    let params = &data["params"];
    let prefix = format!("M_LandScape__{family}_snp_0_");
    let read = |name: &str| -> Vec4 {
        let v = params[name].as_array();
        Vec4::from_array(std::array::from_fn(|i| {
            v.and_then(|v| v.get(i))
                .and_then(Value::as_f64)
                .unwrap_or(0.) as f32
        }))
    };
    let mut result = Inputs {
        colors: std::array::from_fn(|i| {
            let name = format!("{prefix}color_{i}");
            let c = read(&name);
            let intensity = params[format!("{name}-Intensity")][0]
                .as_f64()
                .unwrap_or(1.) as f32;
            // Same CPU colour convention as the authored sky and LightSet.
            Vec4::new(c.x * c.x, c.y * c.y, c.z * c.z, c.w) * intensity
        }),
        floats: std::array::from_fn(|i| read(&format!("{prefix}float_{i}"))),
        vectors: [Vec4::ZERO; 18],
        uv: [Vec4::new(1., 1., 0., 0.); 4],
        control: Vec4::new(
            f32::from(family == "GroundFog"),
            params["GXFT_SATStaticSwitch"][0].as_f64().unwrap_or(0.) as f32,
            0.,
            0.,
        ),
        sun: Vec4::Y,
        direct: Vec4::ZERO,
        ambient: Vec4::ZERO,
        hemi_axis: Vec4::ZERO,
        hemi_delta: Vec4::ZERO,
    };
    // float2 and float3 slots overlap numerically, but are separate native registers.
    for i in 0..6 {
        result.vectors[i] = read(&format!("{prefix}float3_{i}"));
    }
    for i in 0..12 {
        result.vectors[i + 6] = read(&format!("{prefix}float2_{i}"));
    }
    result.colors[1] = read("g_DiffuseMapColor");
    result.colors[1] = Vec4::new(
        result.colors[1].x.powi(2),
        result.colors[1].y.powi(2),
        result.colors[1].z.powi(2),
        1.,
    );
    result
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn replace(
    mut commands: Commands,
    assets: Res<AssetServer>,
    standard: Res<Assets<StandardMaterial>>,
    mut fog: ResMut<Assets<FogMaterial>>,
    mut settings: ResMut<Settings>,
    meshes: Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        &GltfMaterialExtras,
    )>,
    irradiance: Option<Res<crate::lighting::MapIrradiance>>,
) {
    for (entity, original, extras) in &meshes {
        let Ok(extras) = serde_json::from_str::<Value>(&extras.value) else {
            continue;
        };
        let data = &extras["sekiro_fog_surface"];
        if data["version"].as_u64() != Some(1) || !standard.contains(&original.0) {
            continue;
        }
        if !settings.enabled {
            commands
                .entity(entity)
                .remove::<MeshMaterial3d<StandardMaterial>>()
                .insert(Visibility::Hidden);
            continue;
        }
        let prefix = settings.prefix.clone();
        let empty_gi = settings.empty_gi.clone();
        let handle = settings
            .materials
            .entry(original.0.id())
            .or_insert_with(|| {
                let mut inputs = inputs(data);
                let textures = data["textures"].as_array();
                let slots = if inputs.control.x > 0. {
                    [0, 2, 3, 4]
                } else {
                    [1, 7, 1, 7]
                };
                info!(
                    "map fog surface material: family={}, slots={slots:?}",
                    data["family"].as_str().unwrap_or_default()
                );
                let images: [Handle<Image>; 4] = std::array::from_fn(|i| {
                    let row = textures
                        .and_then(|t| t.iter().find(|t| t["slot"].as_u64() == Some(slots[i])));
                    let Some(row) = row else {
                        return Handle::default();
                    };
                    let Some(uri) = row["uri"].as_str() else {
                        return Handle::default();
                    };
                    inputs.uv[i] = Vec4::new(
                        row["scale"][0].as_f64().unwrap_or(1.) as f32,
                        row["scale"][1].as_f64().unwrap_or(1.) as f32,
                        row["uv"].as_f64().unwrap_or(0.) as f32,
                        0.,
                    );
                    // Every bank is relative to this part's source block, including neighbours.
                    let path = crate::map::world::material_texture_path(&assets, &original.0, uri)
                        .unwrap_or_else(|| format!("{prefix}models/{uri}"));
                    let srgb =
                        inputs.control.x > 0. && (i == 1 || (i == 2 && inputs.control.y < 0.5));
                    assets
                        .load_builder()
                        .with_settings(move |s: &mut ImageLoaderSettings| {
                            s.is_srgb = srgb;
                            s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                                address_mode_u: ImageAddressMode::Repeat,
                                address_mode_v: ImageAddressMode::Repeat,
                                ..ImageSamplerDescriptor::linear()
                            });
                        })
                        .load(path)
                });
                fog.add(FogMaterial {
                    inputs,
                    mask: images[0].clone(),
                    flow: images[1].clone(),
                    ground: images[2].clone(),
                    height: images[3].clone(),
                    irradiance: irradiance.as_ref().map_or(empty_gi, |i| i.buffer.clone()),
                    voxels: irradiance.as_ref().map(|i| i.voxels.clone()),
                    diffuse: irradiance.as_ref().map(|i| i.diffuse.clone()),
                    double_sided: standard.get(&original.0).is_some_and(|m| m.double_sided),
                })
            })
            .clone();
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert((MeshMaterial3d(handle), NotShadowCaster, NotShadowReceiver));
    }
}

fn update_lighting(
    lighting: Res<crate::lighting::MapLighting>,
    ambient_light: Res<bevy::light::GlobalAmbientLight>,
    mut materials: ResMut<Assets<FogMaterial>>,
) {
    let set =
        crate::lighting::LightSet::from_gparam(&lighting.gparam, lighting.light_set, lighting.hour);
    let rgb = |color: Color| {
        let c = color.to_linear();
        Vec3::new(c.red, c.green, c.blue)
    };
    let direct = rgb(set.sun_color) * set.sun_lux / std::f32::consts::PI;
    let ambient = rgb(set.ambient) * ambient_light.brightness;
    for (_, material) in materials.iter_mut() {
        material.inputs.sun = set.sun_toward.extend(0.);
        material.inputs.control.w = crate::lighting::ENV_PER_UNIT;
        material.inputs.direct = direct.extend(0.);
        material.inputs.ambient = ambient.extend(0.);
        material.inputs.hemi_axis = set.hemi_axis.extend(0.);
        material.inputs.hemi_delta = (set.hemi_delta * ambient_light.brightness).extend(0.);
    }
}

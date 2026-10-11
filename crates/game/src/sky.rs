//! Camera-centred authored m89 skies, with independent material radiance.
use std::{collections::BTreeMap, path::Path};

use anyhow::{Context, Result};
use bevy::{
    asset::{RenderAssetUsages, embedded_asset, embedded_path},
    camera::visibility::NoFrustumCulling,
    image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
    light::{NotShadowCaster, NotShadowReceiver},
    mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology},
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::render_resource::{
        AsBindGroup, CompareFunction, RenderPipelineDescriptor, ShaderType,
        SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};
use serde_json::Value;

// Calibration against the native-light Bevy scene; SAT colour CPU packing is unresolved.
const SKY_RADIANCE_SCALE: f32 = 8.0;
const ORIGINAL_SKY_RADIANCE_SCALE: f32 = 1.0;

#[derive(Resource)]
pub(crate) struct SkyEnabled;

#[derive(Clone, ShaderType, Debug)]
struct SkyInputs {
    colors: [Vec4; 11],
    floats: [Vec4; 48],
    vectors: [Vec4; 10],
    uv: [Vec4; 10],
    sun: Vec4,
    control: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct SkyMaterial {
    #[uniform(0)]
    inputs: SkyInputs,
    #[texture(1)]
    #[sampler(2)]
    high: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    low: Handle<Image>,
}

impl Material for SkyMaterial {
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn alpha_mode(&self) -> AlphaMode {
        if self.inputs.control.x > 1.5 {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        }
    }
    fn vertex_shader() -> ShaderRef {
        bevy::asset::AssetPath::from_path_buf(embedded_path!("sky.wgsl"))
            .with_source("embedded")
            .into()
    }
    fn fragment_shader() -> ShaderRef {
        Self::vertex_shader()
    }
    fn specialize(
        _: &MaterialPipeline,
        d: &mut RenderPipelineDescriptor,
        _: &MeshVertexBufferLayoutRef,
        _: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        d.primitive.cull_mode = None;
        if let Some(depth) = &mut d.depth_stencil {
            depth.depth_write_enabled = Some(false);
            depth.depth_compare = Some(CompareFunction::GreaterEqual);
        }
        Ok(())
    }
}

#[derive(Resource)]
struct Library {
    id: String,
    meshes: BTreeMap<String, Handle<Mesh>>,
    materials: BTreeMap<String, Handle<SkyMaterial>>,
    current: String,
}

#[derive(Component)]
struct Dome;
#[derive(Component)]
struct CloudDome;

// common.emevd event 100 activates these m89 groups with 2012[1].
// Story time flags 9800..9803 map to the matching authored light/probe hour keys.
fn selection(id: &str, hour: f32) -> &'static str {
    if id.starts_with("m25") {
        return "m900070";
    }
    if id.starts_with("m10") {
        return "m900060";
    }
    let keys = [
        (0., "m900040"),
        (2., "m900050"),
        (6., "m900000"),
        (12., "m900010"),
        (18., "m900020"),
        (22., "m900030"),
    ];
    let hour = hour.rem_euclid(24.);
    keys.into_iter()
        .min_by(|a, b| {
            let distance = |h: f32| {
                let d = (h - hour).abs();
                d.min(24. - d)
            };
            distance(a.0).total_cmp(&distance(b.0))
        })
        .unwrap()
        .1
}

fn inputs(row: &Value) -> SkyInputs {
    let shader = row["shader"].as_str().unwrap_or_default();
    let prefix = if shader.contains("Ver2") {
        "M_Sky__SkyDome_Ver2_snp_0_"
    } else if shader.contains("M[Sky]_Cloud.") {
        "M_Sky__Cloud_snp_0_"
    } else {
        "M_Sky__SkyDome_snp_0_"
    };
    let values = |name: &str| -> Vec<f32> {
        row["params"][name]
            .as_array()
            .into_iter()
            .flatten()
            .map(|v| v.as_f64().unwrap_or(0.) as f32)
            .collect()
    };
    let vector = |name: &str| {
        let v = values(name);
        Vec4::new(
            *v.first().unwrap_or(&0.),
            *v.get(1).unwrap_or(&0.),
            *v.get(2).unwrap_or(&0.),
            *v.get(3).unwrap_or(&0.),
        )
    };
    SkyInputs {
        colors: std::array::from_fn(|i| {
            let name = format!("{prefix}color_{i}");
            let c = vector(&name);
            // Colour packing is calibrated as gamma 2, following native LightSet uploads.
            let intensity = values(&format!("{name}-Intensity"))
                .first()
                .copied()
                .unwrap_or(1.);
            (c.xyz() * c.xyz() * intensity).extend(1.)
        }),
        floats: std::array::from_fn(|i| vector(&format!("{prefix}float_{i}"))),
        vectors: std::array::from_fn(|i| vector(&format!("{prefix}float3_{i}"))),
        uv: std::array::from_fn(|i| vector(&format!("{prefix}float2_{i}"))),
        sun: Vec4::Y,
        control: Vec4::new(
            if prefix.contains("Ver2") {
                1.
            } else if prefix.contains("Cloud_snp") {
                2.
            } else {
                0.
            },
            if prefix == "M_Sky__SkyDome_snp_0_" {
                ORIGINAL_SKY_RADIANCE_SCALE
            } else {
                SKY_RADIANCE_SCALE
            },
            0.,
            values("GXFT_SATStaticSwitch")
                .first()
                .copied()
                .unwrap_or(0.),
        ),
    }
}

/// Read the existing GLB mesh without spawning its ordinary, lit scene materials.
fn mesh(path: &Path, material_index: usize) -> Result<Mesh> {
    let bytes = std::fs::read(path)?;
    anyhow::ensure!(bytes.get(..4) == Some(b"glTF"), "invalid sky GLB");
    let word = |offset| -> Result<usize> {
        Ok(u32::from_le_bytes(
            bytes
                .get(offset..offset + 4)
                .context("truncated sky GLB")?
                .try_into()?,
        ) as usize)
    };
    let len = word(12)?;
    let json: Value = serde_json::from_slice(bytes.get(20..20 + len).context("GLB JSON")?)?;
    let bin = bytes.get(28 + len..).context("GLB binary")?;
    let p = json["meshes"][0]["primitives"]
        .as_array()
        .context("sky primitives")?
        .iter()
        .find(|p| p["material"].as_u64() == Some(material_index as u64))
        .context("sky dome primitive")?;
    let accessor = |index: &Value, components: usize| -> Result<Vec<Vec<f32>>> {
        let a = &json["accessors"][index.as_u64().context("accessor index")? as usize];
        let v = &json["bufferViews"][a["bufferView"].as_u64().context("buffer view")? as usize];
        anyhow::ensure!(a["componentType"] == 5126, "sky attribute must be float");
        let offset = v["byteOffset"].as_u64().unwrap_or(0) as usize
            + a["byteOffset"].as_u64().unwrap_or(0) as usize;
        let stride = v["byteStride"]
            .as_u64()
            .map_or(components * 4, |v| v as usize);
        (0..a["count"].as_u64().context("attribute count")? as usize)
            .map(|i| {
                (0..components)
                    .map(|c| {
                        let o = offset + i * stride + c * 4;
                        Ok(f32::from_le_bytes(
                            bin.get(o..o + 4).context("attribute bytes")?.try_into()?,
                        ))
                    })
                    .collect()
            })
            .collect()
    };
    let positions: Vec<[f32; 3]> = accessor(&p["attributes"]["POSITION"], 3)?
        .into_iter()
        .map(|v| [v[0], v[1], v[2]])
        .collect();
    let uv: Vec<[f32; 2]> = accessor(&p["attributes"]["TEXCOORD_0"], 2)?
        .into_iter()
        .map(|v| [v[0], v[1]])
        .collect();
    let uv_b = if p["attributes"]["TEXCOORD_1"].is_null() {
        uv.clone()
    } else {
        accessor(&p["attributes"]["TEXCOORD_1"], 2)?
            .into_iter()
            .map(|v| [v[0], v[1]])
            .collect()
    };
    let a = &json["accessors"][p["indices"].as_u64().context("indices")? as usize];
    let v = &json["bufferViews"][a["bufferView"].as_u64().context("index view")? as usize];
    let offset = v["byteOffset"].as_u64().unwrap_or(0) as usize
        + a["byteOffset"].as_u64().unwrap_or(0) as usize;
    let size = match a["componentType"].as_u64() {
        Some(5123) => 2,
        Some(5125) => 4,
        _ => anyhow::bail!("sky indices type"),
    };
    let indices: Vec<u32> = (0..a["count"].as_u64().context("index count")? as usize)
        .map(|i| {
            let o = offset + i * size;
            let b = bin.get(o..o + size).context("index bytes")?;
            Ok(if size == 2 {
                u16::from_le_bytes(b.try_into()?) as u32
            } else {
                u32::from_le_bytes(b.try_into()?)
            })
        })
        .collect::<Result<_>>()?;
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, uv_b);
    mesh.insert_indices(Indices::U32(indices));
    Ok(mesh)
}

fn texture_slots(
    app: &mut App,
    dir: &Path,
    row: &Value,
    images: &mut BTreeMap<String, Handle<Image>>,
) -> Result<Vec<Handle<Image>>> {
    let mut slots = Vec::new();
    for path in row["textures"]
        .as_object()
        .context("sky textures")?
        .values()
    {
        let path = path.as_str().context("sky texture path")?;
        let image = if let Some(image) = images.get(path) {
            image.clone()
        } else {
            let mut image = Image::from_buffer(
                &std::fs::read(dir.join(path))?,
                bevy::image::ImageType::Extension("dds"),
                bevy::image::CompressedImageFormats::all(),
                false,
                ImageSampler::default(),
                RenderAssetUsages::default(),
            )?;
            image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                ..default()
            });
            let image = app.world_mut().resource_mut::<Assets<Image>>().add(image);
            images.insert(path.into(), image.clone());
            image
        };
        slots.push(image);
    }
    anyhow::ensure!(!slots.is_empty(), "authored sky has no exported textures");
    Ok(slots)
}

pub(super) fn install(app: &mut App, cache: &Path, id: &str, hour: f32) -> Result<()> {
    if std::env::args().any(|a| a == "--no-sky-dome" || a == "--probe-sky") {
        return Ok(());
    }
    let dir = cache.join("maps/m89_00_00_99");
    let Ok(bytes) = std::fs::read(dir.join("sky.json")) else {
        warn!("sky.json missing; run SEKIRO_MAP_SKY_ONLY=1 sekiro-extract map m89_00_00_99");
        return Ok(());
    };
    let json: Value = serde_json::from_slice(&bytes)?;
    embedded_asset!(app, "sky.wgsl");
    app.add_plugins(MaterialPlugin::<SkyMaterial>::default());
    let mut library = Library {
        id: id.into(),
        meshes: BTreeMap::new(),
        materials: BTreeMap::new(),
        current: selection(id, hour).into(),
    };
    let mut images = BTreeMap::<String, Handle<Image>>::new();
    for name in [
        "m900000", "m900010", "m900020", "m900030", "m900040", "m900050", "m900060", "m900070",
    ] {
        let row = &json["models"][name];
        anyhow::ensure!(!row.is_null(), "missing sky material {name}");
        let slots = texture_slots(app, &dir, row, &mut images)?;
        let value = inputs(row);
        let material = app
            .world_mut()
            .resource_mut::<Assets<SkyMaterial>>()
            .add(SkyMaterial {
                inputs: value.clone(),
                high: slots[0].clone(),
                low: slots.last().unwrap().clone(),
            });
        library.materials.insert(name.into(), material);
        library.meshes.insert(
            name.into(),
            app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh(
                &dir.join(format!("models/{name}.glb")),
                row["material_index"].as_u64().unwrap_or(0) as usize,
            )?),
        );
    }
    app.world_mut().spawn((
        Dome,
        Mesh3d(library.meshes[&library.current].clone()),
        MeshMaterial3d(library.materials[&library.current].clone()),
        Transform::default(),
        NoFrustumCulling,
        NotShadowCaster,
        NotShadowReceiver,
    ));
    let cloud = &json["models"][&library.current]["cloud"];
    if !cloud.is_null() {
        let slots = texture_slots(app, &dir, cloud, &mut images)?;
        if let Some(high) = slots.first() {
            let mut cloud_inputs = inputs(cloud);
            cloud_inputs.control.y = inputs(&json["models"][&library.current]).control.y;
            let material = app
                .world_mut()
                .resource_mut::<Assets<SkyMaterial>>()
                .add(SkyMaterial {
                    inputs: cloud_inputs,
                    high: high.clone(),
                    low: slots.last().unwrap().clone(),
                });
            let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh(
                &dir.join(format!("models/{}.glb", library.current)),
                cloud["material_index"]
                    .as_u64()
                    .context("cloud material index")? as usize,
            )?);
            app.world_mut().spawn((
                CloudDome,
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::default(),
                NoFrustumCulling,
                NotShadowCaster,
                NotShadowReceiver,
            ));
        }
    }
    info!("authored sky {} for {id} at {hour}h", library.current);
    app.insert_resource(library)
        .insert_resource(SkyEnabled)
        .add_systems(Update, update);
    Ok(())
}

fn update(
    lighting: Res<crate::lighting::MapLighting>,
    time: Res<Time>,
    mut library: ResMut<Library>,
    mut materials: ResMut<Assets<SkyMaterial>>,
    mut domes: Query<(&mut Mesh3d, &mut MeshMaterial3d<SkyMaterial>), With<Dome>>,
    clouds: Query<&MeshMaterial3d<SkyMaterial>, (With<CloudDome>, Without<Dome>)>,
) {
    let selected = selection(&library.id, lighting.hour);
    if library.current != selected {
        for (mut mesh, mut material) in &mut domes {
            mesh.0 = library.meshes[selected].clone();
            material.0 = library.materials[selected].clone();
        }
        library.current = selected.into();
    }
    let set =
        crate::lighting::LightSet::from_gparam(&lighting.gparam, lighting.light_set, lighting.hour);
    if let Some(mut material) = materials.get_mut(&library.materials[selected]) {
        material.inputs.sun = set.sun_toward.extend(0.);
        material.inputs.control.z = time.elapsed_secs();
    }
    for handle in &clouds {
        if let Some(mut material) = materials.get_mut(&handle.0) {
            material.inputs.sun = set.sun_toward.extend(0.);
            material.inputs.control.z = time.elapsed_secs();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_common_event_and_msb_agree_on_selected_groups() {
        use sekiro_formats::{emevd::Emevd, msb};
        let raw = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cache/raw");
        let Ok(bytes) = std::fs::read(raw.join("event/common.emevd")) else {
            return;
        };
        let events = Emevd::parse(&bytes).unwrap();
        let event = events.event(100).unwrap();
        let map = msb::parse(&std::fs::read(raw.join("map/mapstudio/m89_00_00_99.msb")).unwrap())
            .unwrap();
        for (model, group) in [
            ("m900000", 8907000),
            ("m900010", 8907010),
            ("m900020", 8907020),
            ("m900030", 8907030),
            ("m900040", 8907040),
            ("m900050", 8907050),
            ("m900060", 8907060),
            ("m900070", 8907070),
        ] {
            assert!(
                map.parts
                    .iter()
                    .any(|p| map.model(p).is_some_and(|m| m.name == model)
                        && p.entity_group_ids.contains(&group))
            );
            for state in [0, 1] {
                assert!(event.instructions.iter().any(|i| i.code() == (2012, 1)
                    && i.i32_at(0) == Some(group)
                    && i.u8_at(4) == Some(state)));
            }
        }
        assert!(!SkyMaterial::enable_prepass());
        assert!(!SkyMaterial::enable_shadows());
    }
    #[test]
    fn hours_wrap_and_special_maps_keep_their_authored_environment() {
        for (h, n) in [
            (6., "m900000"),
            (12., "m900010"),
            (18., "m900020"),
            (0., "m900040"),
            (24., "m900040"),
            (-6., "m900020"),
        ] {
            assert_eq!(selection("m11_00_00_00", h), n);
            assert_eq!(selection("m11_01_00_00", h), n);
        }
        assert_eq!(selection("m25_00_00_00", 12.), "m900070");
        assert_eq!(selection("m10_00_00_00", 12.), "m900060");
    }
    #[test]
    fn library_meshes_and_material_inputs_are_valid() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cache/maps/m89_00_00_99");
        let Ok(bytes) = std::fs::read(dir.join("sky.json")) else {
            return;
        };
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        for name in [
            "m900000", "m900010", "m900020", "m900040", "m900060", "m900070",
        ] {
            let mesh = mesh(
                &dir.join(format!("models/{name}.glb")),
                json["models"][name]["material_index"].as_u64().unwrap_or(0) as usize,
            )
            .unwrap();
            assert!(mesh.count_vertices() > 0);
            assert!(!mesh.indices().unwrap().is_empty());
            assert!(
                inputs(&json["models"][name])
                    .colors
                    .iter()
                    .all(|v| v.is_finite())
            );
        }
    }
}

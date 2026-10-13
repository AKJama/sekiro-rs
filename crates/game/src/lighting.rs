//! Map lighting from the game's own draw params (`param/drawparam/<area>_<block>_0000.gparam`).
//!
//! A light set gives, per light set id (0 outdoors; 100, 110, ... for interiors, chosen by the
//! hit collision under the player through its MSB gparam config) and per time of day:
//! a sun (`Directional Light Angle0`, `DiffColor0`), a second directional fill light
//! (`Angle1`, `DiffColor1`), hemispherical ambient colours (`Hemi Color Up/Down`) and, in the
//! volumetric fog group, the sky colour, a fixed fog density and the colour of light scattered
//! toward the sun (`DirectColor`). Colours are RGB with the intensity in W.
//!
//! Verified colour uploads use squared RGB times W, with one Bevy unit calibration.
//! Map materials retain the directional hemisphere term;
//! fog uses the recovered global shader profile in an HDR post pass. Angles are (pitch, yaw) in radians in the
//! game's space: the vector `Ry(yaw) * Rx(pitch) * +Z` points toward the light, and is mirrored
//! on X into Bevy space like everything else.

use anyhow::Context;
use bevy::light::{EnvironmentMapLight, GlobalAmbientLight, Skybox};
#[path = "fog.rs"]
pub(crate) mod fog;
use bevy::prelude::*;
use sekiro_formats::gparam::Gparam;

#[path = "grading.rs"]
mod grading;
use bevy::camera::Hdr;
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::post_process::{auto_exposure::AutoExposure, bloom::Bloom};
use bevy::render::view::ColorGrading;

#[path = "tonemap.rs"]
pub(crate) mod tonemap;

#[derive(Resource, Default)]
struct MapPostEffects {
    luts: std::collections::BTreeMap<u32, Handle<Image>>,
    no_grading: bool,
    no_bloom: bool,
    no_auto: bool,
    tone_override: Option<u32>,
}

struct PostEffectsPlugin;
impl Plugin for PostEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            grading::GradingPlugin,
            fog::FogPlugin,
            tonemap::NativePostPlugin,
        ));
    }
}

/// Load authored strips as unorm data, even though PNG loaders normally assume sRGB.
pub fn install_post_effects(app: &mut App, dir: &std::path::Path) -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let mut effects = MapPostEffects {
        no_grading: args.iter().any(|a| a == "--no-grading"),
        no_bloom: args.iter().any(|a| a == "--no-bloom"),
        no_auto: args.iter().any(|a| a == "--no-auto-exposure"),
        tone_override: if args.iter().any(|a| a == "--native-tone-reinhard") {
            Some(3)
        } else if args.iter().any(|a| a == "--native-tone-filmic") {
            Some(0)
        } else {
            None
        },
        ..default()
    };
    if let Ok(entries) = std::fs::read_dir(dir.join("grading")) {
        for entry in entries {
            let path = entry?.path();
            let Some(id) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.strip_prefix("lut"))
                .and_then(|s| s.parse::<u32>().ok())
            else {
                continue;
            };
            if path.extension().is_none_or(|s| s != "png") {
                continue;
            }
            let image = Image::from_buffer(
                &std::fs::read(&path)?,
                bevy::image::ImageType::Extension("png"),
                bevy::image::CompressedImageFormats::NONE,
                false,
                bevy::image::ImageSampler::default(),
                bevy::asset::RenderAssetUsages::default(),
            )?;
            anyhow::ensure!(
                image.width() == 16 && image.height() == 256,
                "invalid LUT strip"
            );
            effects.luts.insert(
                id,
                app.world_mut().resource_mut::<Assets<Image>>().add(image),
            );
        }
    }
    app.insert_resource(effects)
        .add_plugins(PostEffectsPlugin)
        .add_systems(Update, apply_post_effects.after(apply_lighting));
    Ok(())
}

#[derive(Debug, Clone)]
struct PostSettings {
    exposure: f32,
    middle: f32,
    min: f32,
    max: f32,
    auto: bool,
    lut: Option<u32>,
    alpha: f32,
    threshold: f32,
    radius: f32,
    glare: f32,
    saturation: f32,
    center: f32,
    freeze: bool,
    mode: u32,
    light_adapt: f32,
    dark_adapt: f32,
    filmic: f32,
    reinhard: f32,
    white_balance: f32,
    bloom_normalizer: f32,
}

impl PostSettings {
    fn from_gparam(g: &Gparam, id: i32, hour: f32) -> Self {
        let get = |group, name, fallback| {
            g.param(group, name)
                .and_then(|p| p.at(id, hour))
                .and_then(|v| v.first().copied())
                .filter(|v| v.is_finite())
                .unwrap_or(fallback)
        };
        let lut = (get("ColorGrading", "Enable", 0.) > 0.5).then(|| {
            g.param("ColorGrading", "LutSourceId")
                .and_then(|p| p.nearest(id, hour))
                .and_then(|v| v.first().copied())
                .unwrap_or(0.)
                .max(0.) as u32
        });
        let min = get("Tone Map", "Adaption Min", -8.);
        let max = get("Tone Map", "Adaption Max", 8.);
        let mode = g
            .param("Tone Map", "ToneMapMode")
            .or_else(|| g.param("Tone Map", "ToneMap-Mode"))
            .and_then(|p| p.nearest(id, hour))
            .and_then(|v| v.first().copied())
            .unwrap_or(7.) as u32;
        let mode = match mode {
            0 | 3 | 7 | 8 => mode,
            _ => {
                warn!("unimplemented native tone mode {mode}; using sensitometric 7");
                7
            }
        };
        Self {
            exposure: get("Tone Map", "ToneMap-Exposure", 0.),
            middle: get("Tone Map", "Middle Gray", -2.),
            min,
            max,
            auto: get("Tone Map", "Enable AutoExposure", 0.) > 0.5,
            lut,
            alpha: if get("Bloom", "Enable", 1.) > 0.5 {
                get("Bloom", "GlareBloom Alpha", 0.).max(0.)
            } else {
                0.
            },
            threshold: get("Bloom", "Glare Threshold", 0.).max(0.),
            radius: get("Bloom", "GaussianRadiusScale", 1.).max(0.),
            glare: get("Bloom", "Glare Luminance", 1.).max(0.),
            saturation: get("Bloom", "Sturate Ctrl", 0.).clamp(0., 1.),
            center: get("Tone Map", "Center Weight", 4.).max(0.),
            freeze: get("Tone Map", "FreezeAdaptionFade", 0.) > 0.5,
            mode,
            light_adapt: get("Tone Map", "LightAdapt", 1.).max(0.),
            dark_adapt: get("Tone Map", "DarkAdapt", 0.5).max(0.),
            filmic: get("Tone Map", "FilmicContrast", 0.5),
            reinhard: get("Tone Map", "ReinhardContrast", 1.6),
            white_balance: get("Tone Map", "ReinhardWhiteBalance", 1.7),
            bloom_normalizer: (0..(get("Bloom", "GlareNumLevels", 7.) as i32).clamp(1, 16))
                .map(|i| get("Bloom", "GlareBloom Smooth", 0.8).powi(i))
                .sum(),
        }
    }
    fn upload(&self, effects: &MapPostEffects) -> tonemap::GamePost {
        use tonemap::*;
        GamePost {
            exposure: Vec4::new(
                self.exposure.exp2(),
                1.,
                self.middle,
                f32::from(self.auto && !effects.no_auto),
            ),
            adaptation: Vec4::new(self.min, self.max, 0., 0.),
            bloom: Vec4::new(
                self.threshold,
                if effects.no_bloom {
                    0.
                } else {
                    self.alpha * self.glare * BLOOM_QUALITY_SCALE / self.bloom_normalizer
                },
                self.radius,
                1.,
            ),
            transfer: Vec4::new(
                1.,
                1. / OUTPUT_GAMMA,
                effects.tone_override.unwrap_or(self.mode) as f32,
                self.center,
            ),
            curve: Vec4::new(
                self.light_adapt,
                self.dark_adapt,
                self.filmic,
                self.reinhard,
            ),
            reinhard: Vec4::new(self.white_balance, 0., 0., 0.),
            controls: Vec4::new(
                self.saturation,
                f32::from(self.freeze),
                f32::from(
                    !effects.no_grading
                        && self.lut.is_some_and(|id| effects.luts.contains_key(&id)),
                ),
                0.,
            ),
        }
    }
}

#[derive(Component)]
struct AppliedPost(i32, f32, String);

fn apply_post_effects(
    mut commands: Commands,
    lighting: Res<MapLighting>,
    effects: Res<MapPostEffects>,
    cameras: Query<(Entity, Option<&AppliedPost>), With<Camera3d>>,
) {
    let _timer = crate::perf_log::SystemTimer::start("apply_post_effects");
    let pending: Vec<_> = cameras
        .iter()
        .filter(|(_, p)| {
            p.is_none_or(|p| {
                p.0 != lighting.light_set || p.1 != lighting.hour || p.2 != lighting.source
            })
        })
        .map(|(e, _)| e)
        .collect();
    if pending.is_empty() {
        return;
    }
    let set = PostSettings::from_gparam(&lighting.gparam, lighting.light_set, lighting.hour);
    info!(
        "map native post: {set:?}, upload={:?}; CPU mappings from SHADER-INPUTS.md",
        set.upload(&effects)
    );
    for entity in pending {
        let mut camera = commands.entity(entity);
        camera.insert((
            Hdr,
            Tonemapping::None,
            DebandDither::Disabled,
            AppliedPost(lighting.light_set, lighting.hour, lighting.source.clone()),
        ));
        camera.insert((ColorGrading::default(), set.upload(&effects)));
        camera.remove::<(AutoExposure, Bloom)>();
        if !effects.no_grading
            && let Some(lut) = set.lut.and_then(|id| effects.luts.get(&id))
        {
            camera.insert(grading::GradingLut(lut.clone()));
        } else {
            camera.remove::<grading::GradingLut>();
        }
    }
}

/// Inferred engine bridge, not a GPARAM transform: Bevy lux per native scene unit.
/// 4500 folds the former 9000-lux and 0.5 post calibration into one light scale.
/// With Bevy's default EV 9.7, diffuse radiance is scale * exposure / pi.
pub const SCENE_TO_BEVY: f32 = 4_500.0;
/// Inferred BTL candela mapping shares the bridge; BTL native units remain open.
pub const LOCAL_LUMENS_PER_UNIT: f32 = SCENE_TO_BEVY * 4.0 * std::f32::consts::PI;

#[derive(Resource, Clone, Debug)]
pub struct ShadowSettings {
    pub enabled: bool,
    pub cascades: usize,
    pub distance: f32,
    pub caster_distance: f32,
    pub first_split: f32,
    pub depth_bias: f32,
    pub normal_bias: f32,
}

impl ShadowSettings {
    pub fn from_args(g: &Gparam, hour: f32, args: &[String]) -> anyhow::Result<Self> {
        let mut settings = Self {
            enabled: !args.iter().any(|a| a == "--no-shadow"),
            cascades: 2,
            distance: 60.,
            caster_distance: 45.,
            depth_bias: 0.02,
            normal_bias: 0.6,
            first_split: g
                .param("ShadowParam", "Cascade Dist 0->1")
                .and_then(|p| p.at(0, hour))
                .and_then(|v| v.first().copied())
                .filter(|v| v.is_finite() && *v > 0.)
                .unwrap_or(15.),
        };
        for (i, arg) in args.iter().enumerate() {
            if arg == "--shadow" {
                let value = args
                    .get(i + 1)
                    .context("--shadow needs cascades,distance")?;
                let (count, distance) = value
                    .split_once(',')
                    .context("--shadow needs cascades,distance")?;
                settings.cascades = count.parse()?;
                settings.distance = distance.parse()?;
            }
            if arg == "--shadow-casters" {
                settings.caster_distance = args
                    .get(i + 1)
                    .context("--shadow-casters needs metres")?
                    .parse()?;
            }
            if arg == "--shadow-bias" {
                let value = args
                    .get(i + 1)
                    .context("--shadow-bias needs depth,normal")?;
                let (depth, normal) = value
                    .split_once(',')
                    .context("--shadow-bias needs depth,normal")?;
                settings.depth_bias = depth.parse()?;
                settings.normal_bias = normal.parse()?;
            }
        }
        anyhow::ensure!(
            (1..=4).contains(&settings.cascades),
            "shadow cascades must be 1..4"
        );
        anyhow::ensure!(
            settings.distance.is_finite() && settings.distance >= 1.,
            "shadow distance must be finite and >= 1 m"
        );
        anyhow::ensure!(
            settings.caster_distance.is_finite() && settings.caster_distance > 0.,
            "caster distance must be finite and positive"
        );
        settings.first_split = settings.first_split.min(settings.distance * 0.5);
        anyhow::ensure!(
            settings.depth_bias.is_finite()
                && settings.depth_bias >= 0.
                && settings.normal_bias.is_finite()
                && settings.normal_bias >= 0.,
            "shadow bias must be finite and nonnegative"
        );
        Ok(settings)
    }
}
/// Irradiance-to-radiance bridge shared by hemisphere and environment lighting.
pub const ENV_PER_UNIT: f32 = SCENE_TO_BEVY / std::f32::consts::PI;

/// Native RGBA8 coefficient voxels and spatial records, shared by every map/actor material.
#[derive(Resource)]
pub struct MapIrradiance {
    pub buffer: Handle<bevy::render::storage::ShaderBuffer>,
    pub specular: Handle<Image>,
    pub diffuse: Handle<Image>,
    pub voxels: Handle<Image>,
}

/// Pack four coefficient planes side-by-side in one hardware-filtered 3D atlas.
fn irradiance_atlas(records: &mut [[u32; 52]], source: &[u32]) -> anyhow::Result<Image> {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let width = records
        .iter()
        .map(|r| r[16])
        .max()
        .unwrap_or(1)
        .max(128)
        .next_power_of_two();
    let height = records
        .iter()
        .map(|r| r[17])
        .max()
        .unwrap_or(1)
        .max(128)
        .next_power_of_two();
    let (mut x, mut y, mut z, mut row_height, mut layer_depth) = (0, 0, 0, 0, 0);
    let mut locations = vec![[0; 3]; records.len()];
    // Deepest volumes first prevents a shallow box from wasting a deep shelf.
    let mut order: Vec<_> = (0..records.len()).collect();
    order.sort_by_key(|i| std::cmp::Reverse(records[*i][18]));
    for i in order {
        let r = &records[i];
        if x + r[16] > width {
            x = 0;
            y += row_height;
            row_height = 0;
        }
        if y + r[17] > height {
            x = 0;
            y = 0;
            z += layer_depth;
            layer_depth = 0;
            row_height = 0;
        }
        locations[i] = [x, y, z];
        x += r[16];
        row_height = row_height.max(r[17]);
        layer_depth = layer_depth.max(r[18]);
    }
    let depth = (z + layer_depth).max(1);
    anyhow::ensure!(
        width * 4 <= 2048 && height <= 2048 && depth <= 2048,
        "GIIV atlas exceeds portable 3D texture limits"
    );
    let mut data = vec![0u8; (width * 4) as usize * height as usize * depth as usize * 4];
    for (r, location) in records.iter_mut().zip(locations) {
        let [ox, oy, oz] = location;
        for axis in 0..4 {
            let dims = &r[40 + axis * 3..43 + axis * 3];
            for slice in 0..dims[2] {
                for row in 0..dims[1] {
                    let src = (r[20 + axis] + (slice * dims[1] + row) * dims[0]) as usize;
                    let dst = (((oz + slice) * height + oy + row) * (width * 4)
                        + ox
                        + axis as u32 * width) as usize
                        * 4;
                    for (i, pixel) in source[src..src + dims[0] as usize].iter().enumerate() {
                        data[dst + i * 4..dst + i * 4 + 4].copy_from_slice(&pixel.to_le_bytes());
                    }
                }
            }
        }
        r[20..24].copy_from_slice(&[ox, oy, oz, width]);
    }
    let mut image = Image::new_uninit(
        Extent3d {
            width: width * 4,
            height,
            depth_or_array_layers: depth,
        },
        TextureDimension::D3,
        TextureFormat::Rgba8Unorm,
        default(),
    );
    image.data = Some(data);
    image.sampler = bevy::image::ImageSampler::linear();
    info!(
        "GIIV atlas: {}x{height}x{depth}, {:.1} MiB",
        width * 4,
        image.data.as_ref().unwrap().len() as f32 / (1024. * 1024.)
    );
    Ok(image)
}

fn probe_array(dir: &std::path::Path, probes: &MapProbes, diffuse: bool) -> anyhow::Result<Image> {
    use anyhow::ensure;
    use bevy::render::render_resource::{
        Extent3d, TextureDataOrder, TextureDimension, TextureFormat, TextureViewDescriptor,
        TextureViewDimension,
    };
    let size: u32 = if diffuse { 16 } else { 128 };
    let levels = if diffuse { 1 } else { 8 };
    let suffix = if diffuse { "_diffuse" } else { "" };
    let count = probes.available.len().max(1) as u32;
    let mut data = Vec::new();
    let face_bytes: usize = (0..levels).map(|m| (size >> m).pow(2) as usize * 8).sum();
    for probe in &probes.available {
        let bytes =
            std::fs::read(dir.join(format!("env/gilm{probe}_{:02}{suffix}.dds", probes.variant)))?;
        ensure!(
            bytes.len() == 128 + face_bytes * 6 && bytes.starts_with(b"DDS "),
            "invalid exported GILM cube"
        );
        data.extend_from_slice(&bytes[128..]);
    }
    if data.is_empty() {
        data.resize(face_bytes * 6, 0);
    }
    let mut image = Image::new_uninit(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: count * 6,
        },
        TextureDimension::D2,
        TextureFormat::Rgba16Float,
        default(),
    );
    image.texture_descriptor.mip_level_count = levels;
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::CubeArray),
        ..default()
    });
    image.data_order = TextureDataOrder::LayerMajor;
    image.data = Some(data);
    image.sampler = bevy::image::ImageSampler::linear();
    Ok(image)
}

pub fn install_irradiance(
    app: &mut App,
    dir: &std::path::Path,
    probes: &MapProbes,
) -> anyhow::Result<()> {
    use anyhow::{Context, ensure};
    let mut regions: Vec<_> = probes.regions.iter().filter(|r| r.size.is_some()).collect();
    regions.sort_by(|a, b| {
        b.size
            .unwrap()
            .element_product()
            .total_cmp(&a.size.unwrap().element_product())
    });
    let enabled = !std::env::args().any(|a| a == "--no-giiv");
    let indices: std::collections::BTreeMap<_, _> = probes
        .available
        .iter()
        .enumerate()
        .map(|(i, p)| (*p, i as u32))
        .collect();
    let mut records = Vec::<[u32; 52]>::new();
    let mut voxels = Vec::new();
    {
        for region in regions {
            let Some(cube) = indices.get(&region.probe) else {
                continue;
            };
            let mut dimensions = [0u32; 3];
            let mut axis_dimensions = [[0u32; 3]; 4];
            let mut offsets = [0; 4];
            let mut coefficients = Vec::new();
            for (index, (axis, offset)) in ['x', 'y', 'z', 'w']
                .into_iter()
                .zip(&mut offsets)
                .enumerate()
            {
                let path = dir.join(format!(
                    "env/giiv{}_{:02}_{axis}.rgba8",
                    region.probe, probes.variant
                ));
                if !path.exists() {
                    break;
                }
                let bytes =
                    std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
                ensure!(bytes.len() >= 12, "truncated GIIV header");
                let dims: [u32; 3] = std::array::from_fn(|i| {
                    u32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap())
                });
                ensure!(
                    dims.iter().all(|d| *d > 0 && *d <= 2048),
                    "invalid GIIV dimensions"
                );
                ensure!(
                    bytes.len() == 12 + dims.iter().map(|d| *d as usize).product::<usize>() * 4,
                    "truncated GIIV voxels"
                );
                axis_dimensions[index] = dims;
                dimensions = std::array::from_fn(|i| dimensions[i].max(dims[i]));
                *offset = coefficients.len() as u32;
                coefficients.extend(
                    bytes[12..]
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .map(|b| u32::from_le_bytes(*b)),
                );
            }
            if axis_dimensions.iter().any(|dims| dims.contains(&0)) {
                continue;
            }
            let size = region.size.unwrap();
            ensure!(
                size.is_finite() && size.min_element() > 0.,
                "invalid GIIV extent"
            );
            // UV is native left-handed box space, Y bottom-centred like MSB.
            let world =
                Mat4::from_scale_rotation_translation(size, region.rotation, region.position);
            let uv = Mat4::from_translation(Vec3::new(0.5, 0., 0.5))
                * Mat4::from_scale(Vec3::new(-1., 1., 1.))
                * world.inverse();
            let mut record = [0; 52];
            record[..16].copy_from_slice(&uv.to_cols_array().map(f32::to_bits));
            record[16..19].copy_from_slice(&dimensions);
            for (axis, dims) in axis_dimensions.iter().enumerate() {
                record[40 + axis * 3..43 + axis * 3].copy_from_slice(dims);
            }
            record[19] = *cube;
            record[20..24].copy_from_slice(&offsets.map(|o| o + voxels.len() as u32));
            let center = region.position + region.rotation * Vec3::Y * size.y * 0.5;
            let extent = Mat3::from_quat(region.rotation).abs() * size * 0.5;
            record[24..27].copy_from_slice(&(center - extent).to_array().map(f32::to_bits));
            record[28..31].copy_from_slice(&(center + extent).to_array().map(f32::to_bits));
            record[32..35].copy_from_slice(&size.to_array().map(f32::to_bits));
            let origin = probes
                .regions
                .iter()
                .find(|r| r.probe == region.probe && r.size.is_none())
                .map_or(center, |r| r.position);
            record[36..39].copy_from_slice(&origin.to_array().map(f32::to_bits));
            records.push(record);
            voxels.extend(coefficients);
        }
    }
    let count = records.len();
    let atlas = irradiance_atlas(&mut records, &voxels)?;
    let mut data = vec![count as u32, u32::from(enabled), 0, 0];
    for record in records {
        data.extend(record);
    }
    let buffer = app
        .world_mut()
        .resource_mut::<Assets<bevy::render::storage::ShaderBuffer>>()
        .add(bevy::render::storage::ShaderBuffer::from(data));
    let specular = probe_array(dir, probes, false)?;
    let diffuse = probe_array(dir, probes, true)?;
    let mut images = app.world_mut().resource_mut::<Assets<Image>>();
    let specular = images.add(specular);
    let diffuse = images.add(diffuse);
    let voxels = images.add(atlas);
    info!(
        "GIIV: {count} complete native coefficient volumes, enabled {enabled}, variant {}",
        probes.variant
    );
    app.insert_resource(MapIrradiance {
        buffer,
        specular,
        diffuse,
        voxels,
    });
    Ok(())
}

#[derive(Debug)]
pub struct ProbeRegion {
    pub probe: u32,
    pub position: Vec3,
    pub rotation: Quat,
    pub size: Option<Vec3>,
}

impl ProbeRegion {
    fn contains(&self, point: Vec3) -> bool {
        self.size.is_some_and(|size| {
            let local = self.rotation.inverse() * (point - self.position);
            local.x.abs() <= size.x * 0.5
                && local.z.abs() <= size.z * 0.5
                && local.y >= 0.
                && local.y <= size.y
        })
    }
}

pub fn nearest_variant(hour: f32) -> u32 {
    nearest_available_variant(hour, 0..6).unwrap_or(2)
}

pub fn nearest_available_variant(hour: f32, variants: impl Iterator<Item = u32>) -> Option<u32> {
    const HOURS: [f32; 6] = [0., 6., 12., 18., 22., 2.];
    let distance = |h: f32| {
        let d = (hour.rem_euclid(24.) - h).abs();
        d.min(24. - d)
    };
    variants.min_by(|a, b| distance(HOURS[*a as usize]).total_cmp(&distance(HOURS[*b as usize])))
}

#[derive(Resource, Default)]
pub struct MapProbes {
    pub regions: Vec<ProbeRegion>,
    /// Only complete extracted diffuse/specular pairs are candidates.
    pub available: std::collections::BTreeSet<u32>,
    pub prefix: String,
    pub variant: u32,
    pub active: Option<u32>,
    pub(crate) handles: std::collections::HashMap<u32, (Handle<Image>, Handle<Image>)>,
}

impl MapProbes {
    fn choose(&self, point: Vec3) -> Option<u32> {
        let candidates = self
            .regions
            .iter()
            .filter(|r| self.available.contains(&r.probe));
        candidates
            .clone()
            .filter(|r| r.contains(point))
            .min_by(|a, b| {
                a.size
                    .unwrap()
                    .element_product()
                    .total_cmp(&b.size.unwrap().element_product())
            })
            .or_else(|| {
                candidates.filter(|r| r.size.is_none()).min_by(|a, b| {
                    a.position
                        .distance_squared(point)
                        .total_cmp(&b.position.distance_squared(point))
                })
            })
            .map(|r| r.probe)
    }
}

/// Spatial reflection probes fade per receiver, so crossing boxes never cuts the whole view.
#[allow(clippy::type_complexity, clippy::too_many_arguments)] // Independent Bevy system parameters.
pub fn apply_probes(
    mut commands: Commands,
    mut probes: ResMut<MapProbes>,
    assets: Res<AssetServer>,
    mut ambient: ResMut<GlobalAmbientLight>,
    focus: Query<&Transform, With<crate::map::MapFocus>>,
    cameras: Query<(Entity, Option<&EnvironmentMapLight>, Option<&Skybox>), With<Camera3d>>,
    mut probe_sky: Local<Option<bool>>,
    selection: Res<crate::lighting_selection::RegionSelection>,
    mut last_env_bank: Local<Option<i32>>,
) {
    let _timer = crate::perf_log::SystemTimer::start("apply_probes");
    // The probes are captures from points inside the level, so as a sky they show nearby
    // scenery blown up behind the world; the real sky is the map's dome meshes. Lighting only,
    // unless `--probe-sky` asks to see the capture for debugging.
    let show_sky = *probe_sky.get_or_insert_with(|| std::env::args().any(|a| a == "--probe-sky"));
    // Stable enclosing fallback rather than a camera-driven region hard cut.
    if *last_env_bank != Some(selection.0[3]) {
        *last_env_bank = Some(selection.0[3]);
        if selection.0[3] > 0 {
            warn!(
                "unverified nonzero MSB environment bank {}; preserving the bank ID without inventing a GILM probe mapping",
                selection.0[3]
            );
        }
    }
    let selected = probes
        .regions
        .iter()
        .filter(|r| probes.available.contains(&r.probe) && r.size.is_some())
        .max_by(|a, b| {
            a.size
                .unwrap()
                .element_product()
                .total_cmp(&b.size.unwrap().element_product())
        })
        .map(|r| r.probe)
        .or_else(|| {
            focus
                .iter()
                .next()
                .and_then(|t| probes.choose(t.translation))
        });
    let maps = selected.map(|id| {
        if !probes.handles.contains_key(&id) {
            let specular = assets.load(format!(
                "{}env/gilm{id}_{:02}.dds",
                probes.prefix, probes.variant
            ));
            let diffuse = assets.load(format!(
                "{}env/gilm{id}_{:02}_diffuse.dds",
                probes.prefix, probes.variant
            ));
            probes.handles.insert(id, (specular, diffuse));
        }
        probes.handles[&id].clone()
    });
    let ready = maps.filter(|(s, d)| {
        assets.is_loaded_with_dependencies(s.id()) && assets.is_loaded_with_dependencies(d.id())
    });
    let active = if ready.is_some() { selected } else { None };
    if probes.active != active {
        info!(
            "environment probe {active:?}, variant {} (intensity {ENV_PER_UNIT:.1} cd/m2)",
            probes.variant
        );
        probes.active = active;
    }
    let brightness = if ready.is_some() { 0. } else { ENV_PER_UNIT };
    if ambient.brightness != brightness {
        ambient.brightness = brightness;
    }
    for (entity, current, current_sky) in &cameras {
        if let Some((specular, diffuse)) = &ready {
            if current.is_none_or(|c| c.specular_map != *specular) {
                commands.entity(entity).insert(EnvironmentMapLight {
                    specular_map: specular.clone(),
                    diffuse_map: diffuse.clone(),
                    intensity: ENV_PER_UNIT,
                    ..default()
                });
            }
            if show_sky && current_sky.is_none_or(|s| s.image.as_ref() != Some(specular)) {
                commands.entity(entity).insert(Skybox {
                    image: Some(specular.clone()),
                    brightness: ENV_PER_UNIT,
                    ..default()
                });
            }
        } else if current.is_some() {
            commands
                .entity(entity)
                .remove::<(Skybox, EnvironmentMapLight)>();
        }
    }
}

/// The sun, driven by the gparam light set.
#[derive(Component)]
pub struct Sun;

/// The second directional light of the light set.
#[derive(Component)]
pub struct FillLight;

#[derive(Resource)]
pub struct MapLighting {
    pub gparam: Gparam,
    pub hour: f32,
    /// Light set id in force (from the collision under the focus), and the one last applied.
    pub light_set: i32,
    pub applied: Option<i32>,
    pub source: String,
}

/// One light set at one time of day, in Bevy terms.
#[derive(Debug, Clone)]
pub struct LightSet {
    pub sun_toward: Vec3,
    pub sun_color: Color,
    pub sun_lux: f32,
    pub fill_toward: Vec3,
    pub fill_color: Color,
    pub fill_lux: f32,
    pub ambient: Color,
    pub ambient_brightness: f32,
    pub hemi_axis: Vec3,
    pub hemi_delta: Vec3,
    pub sky: Color,
}

fn colour(v: &[f32]) -> (Color, f32) {
    let get = |i: usize| v.get(i).copied().unwrap_or(0.0);
    (
        // Verified 310630: gamma 2.0; W remains a separate linear multiplier.
        Color::linear_rgb(get(0).powi(2), get(1).powi(2), get(2).powi(2)),
        v.get(3).copied().unwrap_or(1.0),
    )
}

/// Verified fog copy quantizes clamped RGB to bytes before gamma 2.2 times W.
pub(crate) fn fog_colour(rgb: Vec3, w: f32) -> Vec3 {
    // Inferred nearest-byte rounding; the trace verifies quantization, not its tie rule.
    Vec3::from_array(
        rgb.clamp(Vec3::ZERO, Vec3::ONE)
            .to_array()
            .map(|c| ((c * 255.).round() / 255.).powf(2.2)),
    ) * w
}

/// Direction toward a light from a gparam (pitch, yaw) angle, in Bevy space.
fn toward(angle: &[f32]) -> Vec3 {
    let (pitch, yaw) = (
        angle.first().copied().unwrap_or(-0.8),
        angle.get(1).copied().unwrap_or(0.0),
    );
    let v = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch) * Vec3::Z;
    // Mirror into Bevy space, and make sure the light comes from above.
    let v = Vec3::new(-v.x, v.y, v.z);
    if v.y < 0.0 { -v } else { v }
}

impl LightSet {
    pub fn from_gparam(g: &Gparam, id: i32, hour: f32) -> Self {
        let get = |name: &str| g.param("LightSet", name).and_then(|p| p.at(id, hour));
        let fog = |name: &str| g.param("VolumetricFog", name).and_then(|p| p.at(id, hour));
        let (sun_color, sun_w) = colour(&get("DiffColor0").unwrap_or(vec![1.0, 0.95, 0.85, 1.5]));
        let (fill_color, fill_w) = colour(&get("DiffColor1").unwrap_or(vec![0.5, 0.6, 0.7, 0.2]));
        let (up, up_w) = colour(&get("Hemi Color Up").unwrap_or(vec![0.6, 0.7, 0.8, 0.5]));
        let (down, down_w) = colour(&get("Hemi Color Down").unwrap_or(vec![0.4, 0.4, 0.4, 0.2]));
        let sky_raw = fog("SkyColor").unwrap_or(vec![0.5, 0.55, 0.65, 1.0]);
        let sky_rgb = fog_colour(
            Vec3::new(sky_raw[0], sky_raw[1], sky_raw[2]),
            sky_raw.get(3).copied().unwrap_or(1.),
        );
        let sky = Color::linear_rgb(sky_rgb.x, sky_rgb.y, sky_rgb.z);
        let up_l = up.to_linear();
        let down_l = down.to_linear();
        let ambient = Color::linear_rgb(
            (up_l.red * up_w + down_l.red * down_w) * 0.5,
            (up_l.green * up_w + down_l.green * down_w) * 0.5,
            (up_l.blue * up_w + down_l.blue * down_w) * 0.5,
        );
        Self {
            sun_toward: toward(&get("Angle0").unwrap_or_default()),
            sun_color,
            sun_lux: sun_w * SCENE_TO_BEVY,
            fill_toward: toward(&get("Angle1").unwrap_or_default()),
            fill_color,
            fill_lux: fill_w * SCENE_TO_BEVY,
            ambient,
            ambient_brightness: ENV_PER_UNIT,
            // Verified coefficient packing; default axis is up. Axis field resolution inferred.
            hemi_axis: get("Hemi Direction")
                .filter(|v| v.len() >= 3)
                .map_or(Vec3::Y, |v| Vec3::new(-v[0], v[1], v[2])),
            hemi_delta: Vec3::new(
                up_l.red * up_w - down_l.red * down_w,
                up_l.green * up_w - down_l.green * down_w,
                up_l.blue * up_w - down_l.blue * down_w,
            ) * 0.5,
            sky,
        }
    }
}

/// Spawns the sun and fill lights; [`apply_lighting`] sets them from the light set.
pub fn spawn_lights(commands: &mut Commands, settings: &ShadowSettings) {
    info!("map shadow budget: {settings:?}; normal bias in shadow texels");
    commands.spawn((
        Sun,
        DirectionalLight {
            shadow_maps_enabled: settings.enabled,
            shadow_depth_bias: settings.depth_bias,
            shadow_normal_bias: settings.normal_bias,
            ..default()
        },
        bevy::light::CascadeShadowConfigBuilder {
            num_cascades: settings.cascades,
            maximum_distance: settings.distance,
            first_cascade_far_bound: settings.first_split,
            ..default()
        }
        .build(),
        Transform::default(),
    ));
    commands.spawn((
        FillLight,
        DirectionalLight {
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::default(),
    ));
}

/// Applies the light set in force to the lights, ambient light, sky and fog when it changes
/// (and to cameras spawned since).
#[allow(clippy::type_complexity)]
pub fn apply_lighting(
    mut lighting: ResMut<MapLighting>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut clear: ResMut<ClearColor>,
    mut sun: Query<(&mut DirectionalLight, &mut Transform), (With<Sun>, Without<FillLight>)>,
    mut fill: Query<(&mut DirectionalLight, &mut Transform), (With<FillLight>, Without<Sun>)>,
) {
    let _timer = crate::perf_log::SystemTimer::start("apply_lighting");
    if lighting.applied == Some(lighting.light_set) {
        return;
    }
    let set = LightSet::from_gparam(&lighting.gparam, lighting.light_set, lighting.hour);
    if lighting.applied != Some(lighting.light_set) {
        info!(
            "light set {} at {:.1} h from {}: {set:?}",
            lighting.light_set, lighting.hour, lighting.source
        );
    }
    lighting.applied = Some(lighting.light_set);
    for (mut l, mut t) in &mut sun {
        l.color = set.sun_color;
        l.illuminance = set.sun_lux;
        *t = Transform::IDENTITY.looking_to(-set.sun_toward, Vec3::Y);
    }
    for (mut l, mut t) in &mut fill {
        l.color = set.fill_color;
        l.illuminance = set.fill_lux;
        *t = Transform::IDENTITY.looking_to(-set.fill_toward, Vec3::Y);
    }
    ambient.color = set.ambient;
    ambient.brightness = set.ambient_brightness;
    clear.0 = set.sky;
}

#[cfg(test)]
mod probe_tests {
    use super::*;
    #[test]
    fn atlas_preserves_independent_coefficient_resolutions() {
        let mut record = [0u32; 52];
        record[16..19].copy_from_slice(&[4, 3, 2]);
        let dimensions = [[2, 1, 1], [4, 3, 2], [1, 2, 1], [3, 1, 2]];
        let mut source = Vec::new();
        for (axis, dims) in dimensions.iter().enumerate() {
            record[20 + axis] = source.len() as u32;
            record[40 + axis * 3..43 + axis * 3].copy_from_slice(dims);
            source.extend((0..dims.iter().product::<u32>()).map(|i| i + axis as u32 * 100));
        }
        let atlas = irradiance_atlas(std::slice::from_mut(&mut record), &source).unwrap();
        let data = atlas.data.unwrap();
        let size = atlas.texture_descriptor.size;
        for (axis, [w, h, d]) in dimensions.into_iter().enumerate() {
            for z in 0..d {
                for y in 0..h {
                    for x in 0..w {
                        let index = ((z * size.height + y) * size.width
                            + axis as u32 * record[23]
                            + x) as usize
                            * 4;
                        let actual = u32::from_le_bytes(data[index..index + 4].try_into().unwrap());
                        assert_eq!(actual, (z * h + y) * w + x + axis as u32 * 100);
                    }
                }
            }
        }
    }

    #[test]
    fn cached_volume_upload_matches_native_axes_and_cube_array_layout() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cache/maps/m11_01_00_00");
        if !dir.join("env/giiv0_02_w.rgba8").exists() {
            return;
        }
        let env: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join("env.json")).unwrap()).unwrap();
        let r = env["regions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["probe"] == 0 && !r["size"].is_null())
            .unwrap();
        let mut probes = MapProbes {
            variant: 2,
            ..default()
        };
        probes.available.insert(0);
        let position = Vec3::from_array(serde_json::from_value(r["position"].clone()).unwrap());
        let rotation = Quat::from_array(serde_json::from_value(r["rotation"].clone()).unwrap());
        let size = Vec3::from_array(serde_json::from_value(r["size"].clone()).unwrap());
        probes.regions.push(ProbeRegion {
            probe: 0,
            position,
            rotation,
            size: Some(size),
        });
        let mut app = App::new();
        app.insert_resource(Assets::<bevy::render::storage::ShaderBuffer>::default())
            .insert_resource(Assets::<Image>::default());
        install_irradiance(&mut app, &dir, &probes).unwrap();
        let gi = app.world().resource::<MapIrradiance>();
        let buffer = app
            .world()
            .resource::<Assets<bevy::render::storage::ShaderBuffer>>()
            .get(&gi.buffer)
            .unwrap();
        let words: Vec<u32> = buffer
            .data
            .as_ref()
            .unwrap()
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| u32::from_le_bytes(*b))
            .collect();
        assert_eq!(words[0], 1);
        assert_eq!(words[23], 0); // First cube array layer, independent of sparse probe IDs.
        let matrix = Mat4::from_cols_array(&std::array::from_fn(|i| f32::from_bits(words[4 + i])));
        assert!((matrix.transform_point3(position) - Vec3::new(0.5, 0., 0.5)).length() < 0.0001);
        assert!(
            (matrix
                .transform_point3(position + rotation * Vec3::X * size.x * 0.5)
                .x)
                .abs()
                < 0.0001
        );
        assert_eq!(&words[24..27], &[0, 0, 0]);
        assert!(words[27] >= words[20]);
        let voxels = app
            .world()
            .resource::<Assets<Image>>()
            .get(&gi.voxels)
            .unwrap();
        assert_eq!(
            voxels.texture_descriptor.dimension,
            bevy::render::render_resource::TextureDimension::D3
        );
        let raw = std::fs::read(dir.join("env/giiv0_02_x.rgba8")).unwrap();
        assert_eq!(&voxels.data.as_ref().unwrap()[..4], &raw[12..16]);
        for (axis, name) in ['x', 'y', 'z', 'w'].into_iter().enumerate() {
            let raw = std::fs::read(dir.join(format!("env/giiv0_02_{name}.rgba8"))).unwrap();
            let (w, h, d) = (words[20], words[21], words[22]);
            let image_width = voxels.texture_descriptor.size.width;
            let image_height = voxels.texture_descriptor.size.height;
            for (native, atlas) in [
                (0, axis as u32 * words[27]),
                (
                    w * h * d - 1,
                    ((d - 1) * image_height + h - 1) * image_width + axis as u32 * words[27] + w
                        - 1,
                ),
            ] {
                assert_eq!(
                    &voxels.data.as_ref().unwrap()[atlas as usize * 4..atlas as usize * 4 + 4],
                    &raw[12 + native as usize * 4..16 + native as usize * 4]
                );
            }
        }
        for handle in [&gi.specular, &gi.diffuse] {
            let image = app.world().resource::<Assets<Image>>().get(handle).unwrap();
            assert_eq!(image.texture_descriptor.size.depth_or_array_layers, 6);
            assert_eq!(
                image.texture_view_descriptor.as_ref().unwrap().dimension,
                Some(bevy::render::render_resource::TextureViewDimension::CubeArray)
            );
        }
    }
    #[test]
    fn squared_light_colours_and_hemisphere_endpoints() {
        use sekiro_formats::gparam::{Entry, Group, Param, Value};
        let g = Gparam {
            groups: vec![Group {
                name: "LightSet".into(),
                display_name: String::new(),
                params: [
                    ("DiffColor0", [0.5, 0.25, 1., 2.]),
                    ("Hemi Color Up", [0.5, 1., 0.25, 2.]),
                    ("Hemi Color Down", [1., 0.5, 0.5, 0.5]),
                ]
                .into_iter()
                .map(|(name, value)| Param {
                    name: name.into(),
                    display_name: String::new(),
                    entries: vec![Entry {
                        id: 0,
                        time: 12.,
                        value: Value::Vec4(value),
                    }],
                })
                .collect(),
            }],
        };
        let set = LightSet::from_gparam(&g, 0, 12.);
        let c = set.sun_color.to_linear();
        assert_eq!(
            Vec3::new(c.red, c.green, c.blue) * set.sun_lux / SCENE_TO_BEVY,
            Vec3::new(0.5, 0.125, 2.)
        );
        let mean = set.ambient.to_linear();
        let mean = Vec3::new(mean.red, mean.green, mean.blue);
        assert_eq!(mean + set.hemi_delta, Vec3::new(0.5, 2., 0.125));
        assert_eq!(mean - set.hemi_delta, Vec3::new(0.5, 0.125, 0.125));
        let fog = fog_colour(Vec3::new(0.5, 1., -1.), 2.);
        assert!((fog.x - 2. * (128_f32 / 255.).powf(2.2)).abs() < 0.000001);
        assert_eq!(fog.yz(), Vec2::new(2., 0.));
    }
    #[test]
    fn shadow_options_reject_unbounded_or_unsupported_configurations() {
        let g = Gparam { groups: vec![] };
        let args = |value: &str| vec!["--shadow".into(), value.into()];
        assert!(ShadowSettings::from_args(&g, 12., &args("0,60")).is_err());
        assert!(ShadowSettings::from_args(&g, 12., &args("5,60")).is_err());
        assert!(ShadowSettings::from_args(&g, 12., &args("2,NaN")).is_err());
        let s = ShadowSettings::from_args(&g, 12., &args("4,250")).unwrap();
        assert_eq!(s.cascades, 4);
        assert_eq!(s.distance, 250.);
    }
    #[test]
    fn post_effects_only_mark_3d_cameras_and_disabled_lut_is_removed() {
        use sekiro_formats::gparam::{Entry, Group, Param, Value};
        let mut app = App::new();
        let lut = Handle::<Image>::default();
        let gparam = Gparam {
            groups: vec![Group {
                name: "ColorGrading[Yebis]".into(),
                display_name: String::new(),
                params: ["Enable", "LutSourceId"]
                    .into_iter()
                    .map(|name| Param {
                        name: name.into(),
                        display_name: String::new(),
                        entries: vec![Entry {
                            id: 0,
                            time: 12.,
                            value: Value::Int(if name == "Enable" { 1 } else { 30 }),
                        }],
                    })
                    .collect(),
            }],
        };
        app.insert_resource(MapLighting {
            gparam,
            hour: 12.,
            light_set: 0,
            applied: None,
            source: "fixture".into(),
        })
        .insert_resource(MapPostEffects {
            luts: [(30, lut)].into(),
            ..default()
        })
        .add_systems(Update, apply_post_effects);
        let scene = app.world_mut().spawn(Camera3d::default()).id();
        let hud = app.world_mut().spawn(Camera2d).id();
        app.update();
        assert!(app.world().get::<Hdr>(scene).is_some());
        assert_eq!(
            *app.world().get::<Tonemapping>(scene).unwrap(),
            Tonemapping::None
        );
        assert!(app.world().get::<tonemap::GamePost>(scene).is_some());
        assert!(app.world().get::<Bloom>(scene).is_none());
        assert!(app.world().get::<AutoExposure>(scene).is_none());
        assert!(app.world().get::<grading::GradingLut>(scene).is_some());
        assert!(app.world().get::<Hdr>(hud).is_none());
        assert!(app.world().get::<tonemap::GamePost>(hud).is_none());
        assert!(app.world().get::<Bloom>(hud).is_none());
        assert!(app.world().get::<AutoExposure>(hud).is_none());
        assert!(app.world().get::<grading::GradingLut>(hud).is_none());
        app.world_mut().resource_mut::<MapPostEffects>().no_grading = true;
        app.world_mut().resource_mut::<MapLighting>().hour = 18.;
        app.update();
        assert!(app.world().get::<grading::GradingLut>(scene).is_none());
    }
    #[test]
    fn recovered_post_preserves_authored_bounds_and_disables_stages() {
        let set = PostSettings {
            exposure: 0.4,
            middle: -4.2,
            min: -0.01,
            max: 8.,
            auto: true,
            lut: None,
            alpha: 0.5,
            threshold: 1.5,
            radius: 3.,
            glare: 1.,
            saturation: 0.,
            center: 1.,
            freeze: false,
            mode: 7,
            light_adapt: 1.,
            dark_adapt: 0.5,
            filmic: 0.5,
            reinhard: 1.6,
            white_balance: 1.7,
            bloom_normalizer: 3.951424,
        };
        let upload = set.upload(&MapPostEffects::default());
        assert_eq!(upload.adaptation.y, 8.);
        assert_eq!(upload.bloom.x, 1.5);
        assert_eq!(upload.bloom.z, 3.);
        assert_eq!(upload.bloom.y, 0.5 / 3.951424);
        let disabled = set.upload(&MapPostEffects {
            no_auto: true,
            no_bloom: true,
            ..default()
        });
        assert_eq!(disabled.exposure.w, 0.);
        assert_eq!(disabled.bloom.y, 0.);
    }
    #[test]
    fn native_constructor_defaults_and_bloom_normalization() {
        let set = PostSettings::from_gparam(&Gparam::default(), 0, 12.);
        assert_eq!((set.mode, set.middle, set.min, set.max), (7, -2., -8., 8.));
        assert_eq!((set.light_adapt, set.dark_adapt, set.center), (1., 0.5, 4.));
        assert!((set.bloom_normalizer - 3.951424).abs() < 0.000001);
        let upload = set.upload(&MapPostEffects::default());
        assert_eq!(upload.exposure.x, 1.);
        assert_eq!(upload.exposure.z, -2.);
        assert_eq!(upload.transfer.x, 1.);
        assert_eq!(upload.transfer.y, 1. / 2.2);
    }
    #[test]
    fn extracted_hdr_cube_loads_as_six_layers() {
        use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../cache/maps/m11_00_00_00/env");
        for (file, size, levels) in [("gilm0_02.dds", 128, 8), ("gilm0_02_diffuse.dds", 16, 1)] {
            let path = dir.join(file);
            if !path.exists() {
                return;
            }
            let image = Image::from_buffer(
                &std::fs::read(path).unwrap(),
                ImageType::Extension("dds"),
                CompressedImageFormats::NONE,
                false,
                ImageSampler::default(),
                bevy::asset::RenderAssetUsages::default(),
            )
            .unwrap();
            assert_eq!(image.texture_descriptor.size.width, size);
            assert_eq!(image.texture_descriptor.size.depth_or_array_layers, 6);
            assert_eq!(image.texture_descriptor.mip_level_count, levels);
            assert_eq!(
                image.texture_descriptor.format,
                bevy::render::render_resource::TextureFormat::Rgba16Float
            );
            assert_eq!(
                image.texture_view_descriptor.unwrap().dimension,
                Some(bevy::render::render_resource::TextureViewDimension::Cube)
            );
        }
    }
    #[test]
    fn hour_wrap_and_mapping() {
        for (hour, variant) in [
            (0., 0),
            (6., 1),
            (12., 2),
            (18., 3),
            (22., 4),
            (2., 5),
            (25., 0),
            (-2., 4),
        ] {
            assert_eq!(nearest_variant(hour), variant);
        }
        assert_eq!(nearest_available_variant(12., [4].into_iter()), Some(4));
        assert_eq!(
            nearest_available_variant(0., [1, 2, 3, 4, 5].into_iter()),
            Some(4)
        );
    }
    #[test]
    fn rotated_box_before_nearest_point() {
        let mut probes = MapProbes::default();
        probes.available.extend([1, 2]);
        probes.regions = vec![
            ProbeRegion {
                probe: 1,
                position: Vec3::ZERO,
                rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
                size: Some(Vec3::new(2., 4., 8.)),
            },
            ProbeRegion {
                probe: 2,
                position: Vec3::X * 3.,
                rotation: Quat::IDENTITY,
                size: None,
            },
        ];
        assert_eq!(probes.choose(Vec3::X * 3.), Some(1));
        assert_eq!(probes.choose(Vec3::X * 5.), Some(2));
        probes.available.remove(&1);
        assert_eq!(probes.choose(Vec3::ZERO), Some(2));
    }
}

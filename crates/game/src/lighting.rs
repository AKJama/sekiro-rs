//! Map lighting from the game's own draw params (`param/drawparam/<area>_<block>_0000.gparam`).
//!
//! A light set gives, per light set id (0 outdoors; 100, 110, ... for interiors, chosen by the
//! hit collision under the player through its MSB gparam config) and per time of day:
//! a sun (`Directional Light Angle0`, `DiffColor0`), a second directional fill light
//! (`Angle1`, `DiffColor1`), hemispherical ambient colours (`Hemi Color Up/Down`) and, in the
//! volumetric fog group, the sky colour, a fixed fog density and the colour of light scattered
//! toward the sun (`DirectColor`). Colours are RGB with the intensity in W.
//!
//! Light-unit uploads remain approximated; tone arithmetic comes from recovered shaders, so
//! intensities scale by [`LUX_PER_UNIT`]; ambient light is the average of the hemisphere colours;
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
        let max = get("Tone Map", "Adaption Max", 8.).max(min);
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
                .map(|i| get("Bloom", "GlareBloom Smooth", 0.8).clamp(0., 1.).powi(i))
                .sum(),
        }
    }
    fn upload(&self, effects: &MapPostEffects) -> tonemap::GamePost {
        use tonemap::*;
        GamePost {
            exposure: Vec4::new(
                self.exposure.exp2() * BEVY_HDR_TO_NATIVE_RADIANCE,
                BEVY_HDR_TO_NATIVE_RADIANCE,
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
struct AppliedPost(i32, f32);

fn apply_post_effects(
    mut commands: Commands,
    lighting: Res<MapLighting>,
    effects: Res<MapPostEffects>,
    cameras: Query<(Entity, Option<&AppliedPost>), With<Camera3d>>,
) {
    let pending: Vec<_> = cameras
        .iter()
        .filter(|(_, p)| p.is_none_or(|p| p.0 != lighting.light_set || p.1 != lighting.hour))
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
            AppliedPost(lighting.light_set, lighting.hour),
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

/// Bevy illuminance (lux) per unit of gparam light intensity. Tuned by eye so the noon sun of
/// m11 lands near Bevy's default outdoor exposure.
const LUX_PER_UNIT: f32 = 9_000.0;
/// Local-light candela starts from the sun scale, with a provisional 0.1 calibration.
/// The unscaled mapping flooded Castle's tower cyan; 900 cd/unit retains local detail.
/// Bevy takes lumens, so an isotropic source emits 4*pi times the candela.
pub const LOCAL_LUMENS_PER_UNIT: f32 = LUX_PER_UNIT * 0.1 * 4.0 * std::f32::consts::PI;

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
/// Ambient brightness per unit of hemisphere intensity.
const AMBIENT_PER_UNIT: f32 = 1_800.0;
/// FwLo's ENV_PER_UNIT = LUX_PER_UNIT / pi, in cd/m2.
pub const ENV_PER_UNIT: f32 = LUX_PER_UNIT / std::f32::consts::PI;

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

/// Cut between authored probes; use hemisphere ambient until both images load successfully.
#[allow(clippy::type_complexity)]
pub fn apply_probes(
    mut commands: Commands,
    mut probes: ResMut<MapProbes>,
    assets: Res<AssetServer>,
    mut ambient: ResMut<GlobalAmbientLight>,
    focus: Query<&Transform, With<crate::map::MapFocus>>,
    cameras: Query<(Entity, Option<&EnvironmentMapLight>, Option<&Skybox>), With<Camera3d>>,
    mut probe_sky: Local<Option<bool>>,
) {
    // The probes are captures from points inside the level, so as a sky they show nearby
    // scenery blown up behind the world; the real sky is the map's dome meshes. Lighting only,
    // unless `--probe-sky` asks to see the capture for debugging.
    let show_sky = *probe_sky.get_or_insert_with(|| std::env::args().any(|a| a == "--probe-sky"));
    let selected = focus
        .iter()
        .next()
        .and_then(|t| probes.choose(t.translation));
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
    let brightness = if ready.is_some() {
        0.
    } else {
        AMBIENT_PER_UNIT
    };
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
    pub sky: Color,
}

fn colour(v: &[f32]) -> (Color, f32) {
    let get = |i: usize| v.get(i).copied().unwrap_or(0.0).max(0.0);
    (
        Color::linear_rgb(get(0), get(1), get(2)),
        v.get(3).copied().unwrap_or(1.0),
    )
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
        let (sky, _) = colour(&fog("SkyColor").unwrap_or(vec![0.5, 0.55, 0.65, 1.0]));
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
            sun_lux: sun_w * LUX_PER_UNIT,
            fill_toward: toward(&get("Angle1").unwrap_or_default()),
            fill_color,
            fill_lux: fill_w * LUX_PER_UNIT,
            ambient,
            ambient_brightness: AMBIENT_PER_UNIT,
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
        assert_eq!(upload.exposure.x, tonemap::BEVY_HDR_TO_NATIVE_RADIANCE);
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

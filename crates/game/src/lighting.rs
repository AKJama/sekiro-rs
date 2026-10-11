//! Map lighting from the game's own draw params (`param/drawparam/<area>_<block>_0000.gparam`).
//!
//! A light set gives, per light set id (0 outdoors; 100, 110, ... for interiors, chosen by the
//! hit collision under the player through its MSB gparam config) and per time of day:
//! a sun (`Directional Light Angle0`, `DiffColor0`), a second directional fill light
//! (`Angle1`, `DiffColor1`), hemispherical ambient colours (`Hemi Color Up/Down`) and, in the
//! volumetric fog group, the sky colour, a fixed fog density and the colour of light scattered
//! toward the sun (`DirectColor`). Colours are RGB with the intensity in W.
//!
//! What maps onto Bevy is approximated: the engine's units and tone mapping are not known, so
//! intensities scale by [`LUX_PER_UNIT`]; ambient light is the average of the hemisphere colours;
//! volumetric fog becomes exponential distance fog. Angles are (pitch, yaw) in radians in the
//! game's space: the vector `Ry(yaw) * Rx(pitch) * +Z` points toward the light, and is mirrored
//! on X into Bevy space like everything else.

use bevy::light::{EnvironmentMapLight, GlobalAmbientLight, Skybox};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use sekiro_formats::gparam::Gparam;

/// Bevy illuminance (lux) per unit of gparam light intensity. Tuned by eye so the noon sun of
/// m11 lands near Bevy's default outdoor exposure.
const LUX_PER_UNIT: f32 = 9_000.0;
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
    pub fog_density: f32,
    pub fog_sun_color: Color,
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
        let (fog_sun, fog_sun_w) = colour(&fog("DirectColor").unwrap_or(vec![1.0, 0.8, 0.6, 1.0]));
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
            fog_density: fog("FixedDensity")
                .and_then(|v| v.first().copied())
                .unwrap_or(0.002)
                .max(0.0003),
            fog_sun_color: fog_sun.with_alpha((fog_sun_w * 0.4).clamp(0.0, 1.0)),
        }
    }
}

/// Spawns the sun and fill lights; [`apply_lighting`] sets them from the light set.
pub fn spawn_lights(commands: &mut Commands) {
    commands.spawn((
        Sun,
        DirectionalLight {
            shadow_maps_enabled: true,
            ..default()
        },
        bevy::light::CascadeShadowConfigBuilder {
            num_cascades: 4,
            maximum_distance: 250.0,
            first_cascade_far_bound: 15.0,
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
    mut fogs: Query<&mut DistanceFog>,
) {
    // Reading the change ticks through `iter_mut` does not mark the fog changed.
    let new_fog = fogs.iter_mut().any(|f| f.is_added());
    if lighting.applied == Some(lighting.light_set) && !new_fog {
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
    for mut f in &mut fogs {
        f.color = set.sky;
        f.directional_light_color = set.fog_sun_color;
        f.directional_light_exponent = 16.0;
        f.falloff = FogFalloff::Exponential {
            density: set.fog_density,
        };
    }
}

#[cfg(test)]
mod probe_tests {
    use super::*;
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

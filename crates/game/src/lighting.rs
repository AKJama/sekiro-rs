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

use bevy::light::GlobalAmbientLight;
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use sekiro_formats::gparam::Gparam;

/// Bevy illuminance (lux) per unit of gparam light intensity. Tuned by eye so the noon sun of
/// m11 lands near Bevy's default outdoor exposure.
const LUX_PER_UNIT: f32 = 9_000.0;
/// Ambient brightness per unit of hemisphere intensity.
const AMBIENT_PER_UNIT: f32 = 1_800.0;

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
    (Color::linear_rgb(get(0), get(1), get(2)), v.get(3).copied().unwrap_or(1.0))
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
        let (sun_color, sun_w) =
            colour(&get("DiffColor0").unwrap_or(vec![1.0, 0.95, 0.85, 1.5]));
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
    new_fogs: Query<(), Added<DistanceFog>>,
) {
    if lighting.applied == Some(lighting.light_set) && new_fogs.is_empty() {
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

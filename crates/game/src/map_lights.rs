//! Authored BTL local lights, shared by map capture and play.
use super::*;

#[derive(Clone)]
struct LocalLight {
    block: String,
    position: Vec3,
    color: Color,
    intensity: f32,
    range: f32,
    /// BTL view distance (light object +0xbc); 0 takes the scene's auto distance.
    view_distance: f32,
    spot: Option<(Quat, f32)>,
    shadows: bool,
}

#[derive(Resource, Default)]
pub(super) struct LocalLights {
    lights: Vec<LocalLight>,
    /// Spawned lights with the view fade their intensity carries.
    active: std::collections::BTreeMap<usize, (Entity, f32)>,
}

/// `localLightDistFactor` (GConfig +0x4c, read by `142260`) at the High lighting quality.
const LOCAL_LIGHT_DIST_FACTOR: f32 = 1.;

/// The scene's `LightCollect` auto view distance (`Setting Info` group, blended per light set
/// and hour like the record `142260` reads at +0x68..+0x74; fields bound in `28e6d0`).
#[derive(Clone, Copy, Debug, PartialEq)]
struct LightCollect {
    auto: bool,
    base: f32,
    scaler: f32,
    max: f32,
}

impl LightCollect {
    /// `mapbase.gparam`, which every map stacks over `default.gparam` and no Ashina file
    /// overrides.
    const MAPBASE: Self = Self {
        auto: true,
        base: 10.,
        scaler: 10.,
        max: 130.,
    };

    fn from_lighting(lighting: Option<&crate::lighting::MapLighting>) -> Self {
        // Diagnostic `SEKIRO_ABLATE=nolightcollect` turns the auto distance off, as before #339.
        if crate::lighting::ablate("nolightcollect") {
            return Self {
                auto: false,
                ..Self::MAPBASE
            };
        }
        let Some(l) = lighting else {
            return Self::MAPBASE;
        };
        let get = |name: &str, fallback: f32| {
            l.gparam
                .param("Setting Info", name)
                .and_then(|p| p.at(l.light_set, l.hour))
                .and_then(|v| v.first().copied())
                .unwrap_or(fallback)
        };
        let m = Self::MAPBASE;
        Self {
            auto: get("LightCollect- Auto Compute Enable", m.auto as i32 as f32) >= 0.5,
            base: get("LightCollect- BaseDist", m.base),
            scaler: get("LightCollect-DistScaler", m.scaler),
            max: get("LightCollect-DistMax", m.max),
        }
    }

    /// The light's view fade, `20d9c0` as the point (`210010`) and spot (`211610`) builders call
    /// it: the radius is twice the BTL range, the distance the light's own view distance or
    /// `min(base + radius * scaler, max)`, both times [`LOCAL_LIGHT_DIST_FACTOR`]. Without
    /// either, the light never culls. A light at 0 is not built, so it draws no shadow map.
    fn fade(&self, light: &LocalLight, depth: f32) -> f32 {
        if !self.auto && light.view_distance == 0. {
            return 1.;
        }
        let radius = light.range * 2.;
        let distance = if light.view_distance != 0. {
            light.view_distance
        } else {
            (self.base + radius * self.scaler).min(self.max)
        } * LOCAL_LIGHT_DIST_FACTOR;
        crate::fxr::light_view_fade(distance, depth, radius)
    }
}

impl LocalLights {
    pub(super) fn install(app: &mut App, dir: &Path, args: &[String]) -> Result<()> {
        let lights = match app.world().get_resource::<world::MapWorld>() {
            Some(world) => Self::load_world(dir, args, world)?,
            None => Self::load(dir, args)?,
        };
        app.insert_resource(lights);
        Ok(())
    }

    /// The lights of `dir` and of the connected `world`'s other blocks.
    pub(super) fn load_world(dir: &Path, args: &[String], world: &world::MapWorld) -> Result<Self> {
        let mut lights = Self::load(dir, args)?;
        if !crate::lighting::legacy_map_lights() {
            for id in world.blocks.keys().filter(|id| *id != &world.primary) {
                let neighbour = dir.parent().context("map parent")?.join(id);
                lights.lights.extend(Self::load(&neighbour, args)?.lights);
            }
        }
        Ok(lights)
    }

    pub(super) fn load(dir: &Path, args: &[String]) -> Result<Self> {
        let mut result = Self::default();
        if args.iter().any(|a| a == "--no-local-lights") {
            return Ok(result);
        }
        let scale = args
            .iter()
            .position(|a| a == "--local-light-scale")
            .map(|i| {
                args.get(i + 1)
                    .context("--local-light-scale needs a multiplier")?
                    .parse::<f32>()
                    .context("invalid local light scale")
            })
            .transpose()?
            .unwrap_or(1.);
        anyhow::ensure!(
            scale.is_finite() && scale >= 0.,
            "local light scale must be finite and nonnegative"
        );
        let path = dir.join("lights.json");
        if !path.exists() {
            error!(
                "missing BTL light bank: {} (re-extract map)",
                path.display()
            );
            return Ok(result);
        }
        let value: Value = serde_json::from_slice(&std::fs::read(path)?)?;
        let layout: Value = serde_json::from_slice(&std::fs::read(dir.join("layout.json"))?)?;
        let mut map_rotation = Quat::IDENTITY;
        let mut map_translation = Vec3::ZERO;
        if !crate::lighting::legacy_map_lights()
            && let Some(offset) = layout["map_offsets"].as_array().and_then(|a| a.first())
        {
            map_translation = Vec3::from_array(serde_json::from_value(offset["position"].clone())?);
            let yaw = offset["rotation_y"]
                .as_f64()
                .context("MapOffset rotation")? as f32;
            map_rotation = Quat::from_rotation_y(yaw.to_radians());
        }
        anyhow::ensure!(value["version"] == 1, "unsupported lights.json version");
        let banks = value["banks"].as_array().context("BTL banks array")?;
        let bank = args
            .iter()
            .position(|a| a == "--light-bank")
            .map(|i| {
                args.get(i + 1)
                    .context("--light-bank needs a bank number")?
                    .parse::<u32>()
                    .context("invalid BTL bank")
            })
            .transpose()?;
        let requested = bank;
        let bank = requested.unwrap_or(0);
        let selected = banks.iter().find(|b| {
            b["source"]
                .as_str()
                .is_some_and(|s| s.ends_with(&format!("_{bank:04}.btl")))
        });
        let Some(selected) = selected else {
            error!("missing BTL bank {bank:04} in {}", dir.display());
            anyhow::ensure!(requested.is_none(), "BTL bank {bank:04} is absent");
            return Ok(result);
        };
        for row in selected["lights"].as_array().context("BTL lights array")? {
            let raw: Vec<u8> = serde_json::from_value(row["raw"].clone())?;
            let native = sekiro_formats::btl::parse_record(
                &raw,
                row["name"].as_str().unwrap_or_default().into(),
            )?;
            anyhow::ensure!(
                native.kind <= 1,
                "unsupported BTL light type {}",
                native.kind
            );
            let vector = |key: &str| -> Result<[f32; 3]> {
                let a = row[key].as_array().context("BTL vector")?;
                anyhow::ensure!(a.len() == 3, "BTL vector length");
                let values = [0, 1, 2].map(|i| a[i].as_f64().map(|v| v as f32));
                let [Some(x), Some(y), Some(z)] = values else {
                    anyhow::bail!("BTL vector values")
                };
                anyhow::ensure!([x, y, z].iter().all(|v| v.is_finite()), "BTL finite vector");
                Ok([x, y, z])
            };
            let color = vector("color")?;
            let intensity = row["intensity"].as_f64().context("BTL intensity")? as f32;
            let range = row["range"].as_f64().context("BTL range")? as f32;
            anyhow::ensure!(
                intensity.is_finite() && intensity >= 0. && range.is_finite() && range >= 0.,
                "BTL range/intensity"
            );
            result.lights.push(LocalLight {
                block: dir
                    .file_name()
                    .context("map ID")?
                    .to_string_lossy()
                    .into_owned(),
                position: {
                    // MSB parts are already placed; only ancillary BTL coordinates use MapOffset.
                    let p = map_rotation * Vec3::from_array(native.position) + map_translation;
                    Vec3::new(-p.x, p.y, p.z)
                },
                // 3892e0 squares the local-light diffuse RGB before multiplying power.
                // Keep the old upload only for the matched runtime control.
                color: {
                    let rgb = Vec3::from_array(color) / 255.;
                    let rgb = if crate::lighting::legacy_map_lights() {
                        rgb
                    } else {
                        rgb * rgb
                    };
                    Color::linear_rgb(rgb.x, rgb.y, rgb.z)
                },
                intensity: intensity * scale,
                range,
                view_distance: native.view_distance,
                spot: if native.kind == 1 && !crate::lighting::legacy_map_lights() {
                    let [x, y, z] = native.rotation;
                    let rotation = Quat::from_rotation_y(-map_rotation.to_euler(EulerRot::YXZ).0)
                        * Quat::from_euler(EulerRot::YXZ, -y, x, -z)
                        * Quat::from_rotation_y(std::f32::consts::PI);
                    // BTL angles are degrees; Bevy takes a half angle and emits along -Z.
                    Some((rotation, native.cone_angle.to_radians() * 0.5))
                } else {
                    None
                },
                shadows: native.cast_shadows && !crate::lighting::legacy_map_lights(),
            });
        }
        info!(
            "BTL bank {}: {} lights, scale {scale}; authored range, clustered GPU culling",
            selected["source"],
            result.lights.len()
        );
        Ok(result)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn update_local_lights(
    mut commands: Commands,
    time: Res<Time>,
    mut lights: ResMut<LocalLights>,
    focus: Query<&GlobalTransform, With<MapFocus>>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    world: Option<Res<world::MapWorld>>,
    lighting: Option<Res<crate::lighting::MapLighting>>,
    mut points: Query<&mut PointLight>,
    mut spots: Query<&mut SpotLight>,
    mut last: Local<f32>,
) {
    let _timer = crate::perf_log::SystemTimer::start("update_local_lights");
    let legacy = crate::lighting::legacy_map_lights();
    if legacy && time.elapsed_secs() - *last < 0.5 {
        return;
    }
    *last = time.elapsed_secs();
    let Some(focus) = focus.iter().next() else {
        return;
    };
    let lights = &mut *lights;
    let position = focus.translation();
    // 20d9c0 measures depth along the view axis; without a camera, the distance to the focus.
    let view = camera
        .iter()
        .next()
        .map(|c| (c.translation(), c.forward().as_vec3()));
    let collect = LightCollect::from_lighting(lighting.as_deref());
    let mut candidates: Vec<_> = lights
        .lights
        .iter()
        .enumerate()
        .filter_map(|(i, l)| {
            if !legacy
                && world.as_ref().is_some_and(|world| {
                    world
                        .blocks
                        .get(&l.block)
                        .is_some_and(|b| b.state != world::RequestState::Ready)
                })
            {
                return None;
            }
            let distance = l.position.distance_squared(position);
            if l.intensity <= 0. || l.range <= 0. {
                return None;
            }
            let fade = if legacy {
                if distance > (60. + l.range).powi(2) {
                    return None;
                }
                1.
            } else {
                let depth = match view {
                    Some((eye, forward)) => (l.position - eye).dot(forward),
                    None => distance.sqrt(),
                };
                collect.fade(l, depth)
            };
            (fade > 0.).then_some((i, distance, fade))
        })
        .collect();
    candidates.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    let wanted: std::collections::BTreeMap<_, _> = candidates
        .into_iter()
        .take(if legacy { 64 } else { usize::MAX })
        .map(|(i, _, fade)| (i, fade))
        .collect();
    let old_count = lights.active.len();
    let removed: Vec<_> = lights
        .active
        .keys()
        .copied()
        .filter(|i| !wanted.contains_key(i))
        .collect();
    for i in removed {
        commands
            .entity(lights.active.remove(&i).unwrap().0)
            .despawn();
    }
    for (i, fade) in wanted {
        let light = &lights.lights[i];
        let intensity = light.intensity * fade * crate::lighting::LOCAL_LUMENS_PER_UNIT;
        if let Some((entity, shown)) = lights.active.get_mut(&i) {
            if *shown != fade {
                *shown = fade;
                if let Ok(mut point) = points.get_mut(*entity) {
                    point.intensity = intensity;
                } else if let Ok(mut spot) = spots.get_mut(*entity) {
                    spot.intensity = intensity;
                }
            }
            continue;
        }
        let mut entity = commands.spawn(Transform::from_translation(light.position));
        if let Some((rotation, cone)) = light.spot {
            entity.insert((
                SpotLight {
                    color: light.color,
                    intensity,
                    range: light.range,
                    radius: 0.,
                    inner_angle: 0.,
                    outer_angle: cone,
                    shadow_maps_enabled: light.shadows,
                    ..default()
                },
                Transform::from_translation(light.position).with_rotation(rotation),
            ));
        } else {
            entity.insert(PointLight {
                color: light.color,
                intensity,
                range: light.range,
                radius: 0.,
                shadow_maps_enabled: light.shadows,
                ..default()
            });
        }
        lights.active.insert(i, (entity.id(), fade));
    }
    if lights.active.len() != old_count {
        let shadowed = lights
            .active
            .keys()
            .filter(|&&i| lights.lights[i].shadows)
            .count();
        info!(
            "BTL active lights: {} ({shadowed} with shadow maps)",
            lights.active.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installed_banks_keep_spot_types_and_apply_only_ancillary_offsets() {
        let root = sekiro_sim::cache::root().join("maps");
        if !root.join("m11_00_00_00/lights.json").exists() {
            return;
        }
        let outskirts = LocalLights::load(&root.join("m11_00_00_00"), &[]).unwrap();
        let castle = LocalLights::load(&root.join("m11_01_00_00"), &[]).unwrap();
        assert_eq!(outskirts.lights.len(), 16);
        assert_eq!(castle.lights.len(), 34);
        assert_eq!(castle.lights.iter().filter(|l| l.spot.is_some()).count(), 2);
        assert!(
            outskirts
                .lights
                .iter()
                .chain(&castle.lights)
                .all(|l| !l.shadows)
        );
        let before = Vec3::new(139.609, -144.677, 1.99318);
        assert!(outskirts.lights[0].position.distance(before) > 90.);
        assert!((outskirts.lights[0].position.y - (-141.677)).abs() < 0.001);
        assert!(
            castle.lights[0]
                .position
                .distance(Vec3::new(344.742, -31.3559, 258.093))
                < 0.001
        );
        assert!(
            LocalLights::load(
                &root.join("m11_01_00_00"),
                &["--light-bank".into(), "9999".into()]
            )
            .is_err()
        );
    }
    fn light(x: f32, range: f32, shadows: bool) -> LocalLight {
        LocalLight {
            block: String::new(),
            position: Vec3::X * x,
            color: Color::WHITE,
            intensity: 1.,
            range,
            view_distance: 0.,
            spot: None,
            shadows,
        }
    }
    #[test]
    fn view_distance_follows_20d9c0() {
        let c = LightCollect::MAPBASE;
        // Range 6: radius 12, distance min(10 + 12 * 10, 130) = 130, band min(9.1, 10).
        let l = light(0., 6., true);
        assert_eq!(c.fade(&l, 141.), 1.);
        assert!((c.fade(&l, 142. + 4.55) - 0.5).abs() < 1e-4);
        assert_eq!(c.fade(&l, 151.2), 0.);
        // Range 1: radius 2, distance 10 + 2 * 10 = 30.
        assert_eq!(c.fade(&light(0., 1., false), 31.), 1.);
        assert_eq!(c.fade(&light(0., 1., false), 34.2), 0.);
        // A BTL view distance replaces the auto distance and holds without auto compute.
        let own = LocalLight {
            view_distance: 20.,
            ..light(0., 6., false)
        };
        assert_eq!(c.fade(&own, 31.), 1.);
        assert_eq!(c.fade(&own, 33.5), 0.);
        let off = LightCollect { auto: false, ..c };
        assert_eq!(off.fade(&own, 33.5), 0.);
        assert_eq!(off.fade(&light(0., 6., true), 1e4), 1.);
    }
    #[test]
    fn far_lights_cull_and_draw_no_shadow_map() {
        let mut app = App::new();
        let mut lights: Vec<_> = (0..100).map(|i| light(i as f32 * 0.1, 4., false)).collect();
        // Like the m17 lanterns seen from the castle forest: shadowed, range 6, 380 m away.
        lights.push(light(380., 6., true));
        lights.push(light(10., 6., true));
        lights.push(light(142. + 4.55, 6., false));
        app.insert_resource(Time::<()>::default())
            .insert_resource(LocalLights {
                lights,
                active: default(),
            })
            .add_systems(Update, update_local_lights);
        let focus = app
            .world_mut()
            .spawn((MapFocus, GlobalTransform::default()))
            .id();
        let camera = app
            .world_mut()
            .spawn((
                Camera3d::default(),
                GlobalTransform::from(Transform::default().looking_at(Vec3::X, Vec3::Y)),
            ))
            .id();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs(1));
        app.update();
        let lights = app.world().resource::<LocalLights>();
        let mut kept: Vec<_> = (0..100).collect();
        kept.extend([101, 102]);
        assert_eq!(lights.active.keys().copied().collect::<Vec<_>>(), kept);
        let point = |i: usize| app.world().get::<PointLight>(lights.active[&i].0).unwrap();
        assert!(point(101).shadow_maps_enabled);
        assert!((0..100).all(|i| !point(i).shadow_maps_enabled));
        let full = crate::lighting::LOCAL_LUMENS_PER_UNIT;
        assert_eq!(point(0).intensity, full);
        assert!((point(102).intensity / full - 0.5).abs() < 1e-4);
        assert_eq!(
            app.world_mut()
                .query::<&PointLight>()
                .iter(app.world())
                .filter(|l| l.shadow_maps_enabled)
                .count(),
            1
        );
        // Turning away keeps lights behind the view plane; walking off culls them all.
        *app.world_mut().get_mut::<GlobalTransform>(camera).unwrap() = GlobalTransform::from(
            Transform::from_xyz(1000., 0., 0.).looking_at(Vec3::X * 2000., Vec3::Y),
        );
        *app.world_mut().get_mut::<GlobalTransform>(focus).unwrap() =
            GlobalTransform::from_translation(Vec3::X * 1000.);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs(1));
        app.update();
        assert_eq!(app.world().resource::<LocalLights>().active.len(), 103);
        *app.world_mut().get_mut::<GlobalTransform>(camera).unwrap() = GlobalTransform::from(
            Transform::from_xyz(1000., 0., 0.).looking_at(Vec3::ZERO, Vec3::Y),
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs(1));
        app.update();
        assert!(app.world().resource::<LocalLights>().active.is_empty());
        assert_eq!(
            app.world_mut()
                .query::<&PointLight>()
                .iter(app.world())
                .count(),
            0
        );
    }
}

//! Authored BTL local lights, shared by map capture and play.
use super::*;
use crate::lighting::LightCollect;

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
    active: std::collections::BTreeMap<usize, Entity>,
}

/// What [`LightCollect`] was last read from: `MapLighting` revision, light set, hour, source.
type CollectKey = (u32, i32, f32, String);

/// A light's `20d9c0` view fade as the point (`210010`) and spot (`211610`) builders call it, with
/// the radius twice the BTL range. `None` once its near edge is half its distance again past
/// the fade band, where the entity is despawned; a light at 0 is only hidden, so the
/// cull edge does not churn entities, and a hidden light draws no shadow map.
fn view_fade(collect: &LightCollect, light: &LocalLight, depth: f32) -> Option<f32> {
    let radius = light.range * 2.;
    let Some(distance) = collect.distance(light.view_distance, radius) else {
        return Some(1.);
    };
    let far = distance * 1.5 + crate::lighting::light_fade_band(distance);
    (depth - radius <= far).then(|| crate::lighting::light_view_fade(distance, depth, radius))
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

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn update_local_lights(
    mut commands: Commands,
    time: Res<Time>,
    mut lights: ResMut<LocalLights>,
    focus: Query<&GlobalTransform, With<MapFocus>>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    world: Option<Res<world::MapWorld>>,
    lighting: Option<Res<crate::lighting::MapLighting>>,
    mut shown: Query<(AnyOf<(&mut PointLight, &mut SpotLight)>, &mut Visibility)>,
    mut collect: Local<Option<(Option<CollectKey>, LightCollect)>>,
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
    let key = lighting
        .as_ref()
        .map(|l| (l.revision, l.light_set, l.hour, l.source.clone()));
    if collect.as_ref().is_none_or(|(k, _)| *k != key) {
        let value = lighting.as_ref().map_or_else(LightCollect::fallback, |l| {
            LightCollect::from_gparam(&l.gparam, l.light_set, l.hour)
        });
        *collect = Some((key, value));
    }
    let collect = collect.as_ref().unwrap().1;
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
            if l.intensity <= 0. || l.range <= 0. {
                return None;
            }
            let distance = l.position.distance_squared(position);
            if legacy {
                return (distance <= (60. + l.range).powi(2)).then_some((i, distance, 1.));
            }
            let depth = match view {
                Some((eye, forward)) => (l.position - eye).dot(forward),
                None => distance.sqrt(),
            };
            let fade = view_fade(&collect, l, depth)?;
            // A light reaching 0 is hidden; only one already shown keeps its entity.
            (fade > 0. || lights.active.contains_key(&i)).then_some((i, distance, fade))
        })
        .collect();
    if legacy {
        candidates.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        candidates.truncate(64);
    }
    let wanted: std::collections::BTreeMap<_, _> = candidates
        .into_iter()
        .map(|(i, _, fade)| (i, fade))
        .collect();
    let old_count = lights.active.len();
    lights.active.retain(|i, entity| {
        let keep = wanted.contains_key(i);
        if !keep {
            commands.entity(*entity).despawn();
        }
        keep
    });
    for (i, fade) in wanted {
        let light = &lights.lights[i];
        let intensity = light.intensity * fade * crate::lighting::LOCAL_LUMENS_PER_UNIT;
        let visibility = if fade > 0. {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if let Some(&entity) = lights.active.get(&i) {
            if let Ok(((point, spot), mut shown)) = shown.get_mut(entity) {
                shown.set_if_neq(visibility);
                if fade > 0. {
                    if let Some(mut point) = point {
                        if point.intensity != intensity {
                            point.intensity = intensity;
                        }
                    } else if let Some(mut spot) = spot
                        && spot.intensity != intensity
                    {
                        spot.intensity = intensity;
                    }
                }
            }
            continue;
        }
        let mut entity = commands.spawn((Transform::from_translation(light.position), visibility));
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
        lights.active.insert(i, entity.id());
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
        // Auto: min(10 + radius * 10, 130), radius twice the range.
        assert_eq!(c.distance(0., 12.), Some(130.));
        assert_eq!(c.distance(0., 2.), Some(30.));
        // A BTL view distance replaces it, and holds without auto compute.
        assert_eq!(c.distance(20., 12.), Some(20.));
        let off = LightCollect { auto: false, ..c };
        assert_eq!(off.distance(20., 12.), Some(20.));
        assert_eq!(off.distance(0., 12.), None);
        assert_eq!(view_fade(&off, &light(0., 6., true), 1e4), Some(1.));
        // Hidden past the band (151.1 m for range 6), despawned past 1.5 times the distance.
        assert_eq!(view_fade(&c, &light(0., 6., true), 160.), Some(0.));
        assert_eq!(view_fade(&c, &light(0., 6., true), 220.), None);
    }
    #[test]
    fn far_lights_hide_then_despawn_and_draw_no_shadow_map() {
        let mut app = App::new();
        let mut lights: Vec<_> = (0..100).map(|i| light(i as f32 * 0.1, 4., false)).collect();
        // 100: like the m17 lanterns seen from the castle gate, shadowed, range 6, 380 m away.
        // 101: a near shadowed light. 102: one in the hide band (151 to 204 m of depth).
        lights.extend([
            light(380., 6., true),
            light(10., 6., true),
            light(180., 6., true),
        ]);
        app.insert_resource(Time::<()>::default())
            .insert_resource(LocalLights {
                lights,
                active: default(),
            })
            .add_systems(Update, update_local_lights);
        app.world_mut()
            .spawn((MapFocus, GlobalTransform::default()));
        let camera = app
            .world_mut()
            .spawn((Camera3d::default(), GlobalTransform::default()))
            .id();
        let look = |app: &mut App, from: f32, to: f32| {
            *app.world_mut().get_mut::<GlobalTransform>(camera).unwrap() = GlobalTransform::from(
                Transform::from_xyz(from, 0., 0.).looking_at(Vec3::X * to, Vec3::Y),
            );
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(Duration::from_secs(1));
            app.update();
            app.world().resource::<LocalLights>().active.clone()
        };
        let active = look(&mut app, 0., 1.);
        let mut kept: Vec<_> = (0..100).collect();
        kept.push(101);
        assert_eq!(active.keys().copied().collect::<Vec<_>>(), kept);
        let point =
            |app: &App, i: usize| app.world().get::<PointLight>(active[&i]).copied().unwrap();
        assert!(point(&app, 101).shadow_maps_enabled);
        assert!((0..100).all(|i| !point(&app, i).shadow_maps_enabled));
        assert_eq!(
            point(&app, 0).intensity,
            crate::lighting::LOCAL_LUMENS_PER_UNIT
        );
        // Behind the view plane every light is kept, as in 20d9c0.
        let behind = look(&mut app, 0., -1.);
        assert_eq!(behind.len(), 103);
        let visibility = |app: &App, e: Entity| *app.world().get::<Visibility>(e).unwrap();
        assert_eq!(visibility(&app, behind[&102]), Visibility::Inherited);
        // Turning back hides the band light on the same entity, so it draws no shadow map,
        // and despawns the one far past it.
        let ahead = look(&mut app, 0., 1.);
        assert_eq!(ahead.get(&102), Some(&behind[&102]));
        assert_eq!(visibility(&app, ahead[&102]), Visibility::Hidden);
        assert!(!ahead.contains_key(&100));
        // Walking away despawns it too.
        let away = look(&mut app, -100., 1.);
        assert!(!away.contains_key(&102));
        assert!(app.world().get_entity(behind[&102]).is_err());
    }
}

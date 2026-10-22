//! Native MSB SFX and parameter-resolved first-visit object effects.
use crate::{
    fxr::FxrPlayer,
    map::{
        story::MapStory,
        world::{MapWorld, RequestState},
    },
};
// Alias avoids Bevy 0.20 prelude ContextExt ambiguity.
use anyhow::{Context as ContextExt, Result};
use bevy::prelude::*;
use sekiro_formats::msb::MapRegion;
use sekiro_sim::collision::CollisionWorld;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::Path,
    sync::Arc,
};

struct Placement {
    block: String,
    effect: i32,
    transform: Transform,
    enabled: bool,
    weather: bool,
    camera_relative: bool,
    activation: Option<crate::map::visibility::VisibilityPart>,
    /// A Sculptor's Idol's idle effect, by the idol's bonfire entity: it shows only while the
    /// idol's lit state matches (#154).
    idol: Option<(i32, sekiro_sim::map_effects::IdolIdle)>,
}

/// The bonfire entities of the play world's idols Wolf has lit (knelt at) so far: their unlit
/// idle effect gives way to the lit one, the flame above the idol's hands.
#[derive(Resource, Default)]
pub(crate) struct FoundIdols(pub std::collections::BTreeSet<i32>);

#[derive(Resource)]
struct MapEffects {
    placements: Vec<Placement>,
    regions: BTreeMap<String, Vec<MapRegion>>,
}

impl MapEffects {
    /// The falling-weather effect ids, each once (several placements can share one).
    fn weather_effects(&self) -> BTreeSet<i32> {
        self.placements
            .iter()
            .filter(|p| p.weather)
            .map(|p| p.effect)
            .collect()
    }
}

pub(crate) fn install(app: &mut App, dir: &Path) -> Result<()> {
    let primary = dir.file_name().and_then(|s| s.to_str()).context("map ID")?;
    if !sekiro_sim::world_events::SUPPORTED_MAPS.contains(&primary) {
        return Ok(());
    }
    let cache = dir
        .parent()
        .and_then(Path::parent)
        .context("cache directory")?;
    let mut fx = MapEffects {
        placements: Vec::new(),
        regions: BTreeMap::new(),
    };
    if !app.world().contains_resource::<FxrPlayer>() {
        app.add_plugins(crate::fxr::plugin(cache.to_owned(), |_| {}));
    }
    for block in sekiro_sim::world_events::SUPPORTED_MAPS {
        if block != primary && app.world().get_resource::<MapWorld>().is_none() {
            continue;
        }
        let state = app
            .world()
            .get_resource::<MapStory>()
            .and_then(|s| s.maps.get(block))
            .map(|s| s.state.clone())
            .or_else(|| {
                sekiro_sim::world_events::first_visit(cache, block)
                    .ok()
                    .flatten()
            })
            .context("map FX story state")?;
        let catalog =
            sekiro_sim::map_effects::load(cache, block, &state).map_err(anyhow::Error::msg)?;
        let fallback;
        let layout = if let Some(world) = app.world().get_resource::<MapWorld>() {
            &world.blocks.get(block).context("FX map block")?.json
        } else {
            fallback = serde_json::from_slice::<serde_json::Value>(&std::fs::read(
                cache.join(format!("maps/{block}/layout.json")),
            )?)?;
            &fallback
        };
        let mut parents = BTreeMap::new();
        for row in layout["parts"].as_array().context("FX parts")? {
            if !matches!(row["kind"].as_str(), Some("MapPiece" | "Object")) {
                continue;
            }
            if let Some(part) = crate::map::visibility::VisibilityPart::from_row(block, row)? {
                parents.insert(part.name.clone(), part);
            }
        }
        let events: BTreeMap<_, _> = state
            .parts(layout)
            .into_iter()
            .map(|p| (p.name, p.enabled))
            .collect();
        info!(
            "map FX {block}: {} placements, {} enabled",
            catalog.placements.len(),
            catalog.placements.iter().filter(|p| p.enabled).count()
        );
        for p in catalog.placements {
            let (camera_relative, weather) = app
                .world_mut()
                .resource_mut::<FxrPlayer>()
                .map_actions(p.effect);
            fx.placements.push(Placement {
                block: block.into(),
                effect: p.effect,
                transform: Transform::from_translation(Vec3::from_array(p.position))
                    .with_rotation(Quat::from_array(p.rotation)),
                enabled: p.enabled
                    && p.activation
                        .as_ref()
                        .is_none_or(|name| events.get(name).copied().unwrap_or(true)),
                weather: weather && camera_relative,
                camera_relative,
                activation: p.activation.and_then(|name| parents.get(&name).cloned()),
                idol: p.idol.map(|idle| (p.entity, idle)),
            });
        }
        fx.regions.insert(block.into(), catalog.regions);
    }
    if !app.world().contains_resource::<FxrPlayer>() {
        app.add_plugins(crate::fxr::plugin(cache.to_owned(), |_| {}));
    }
    app.insert_resource(fx).add_systems(
        PostUpdate,
        update.after(bevy::transform::TransformSystems::Propagate),
    );
    Ok(())
}

fn contains(regions: &[MapRegion], index: usize, point: Vec3, depth: usize) -> bool {
    if depth > 8 {
        return false;
    }
    let Some(r) = regions.get(index) else {
        return false;
    };
    if r.shape == 6 {
        return r
            .children
            .iter()
            .any(|&i| i >= 0 && contains(regions, i as usize, point, depth + 1));
    }
    let rot = crate::fxr_tree::game_rot([r.rotation[0], r.rotation[1], r.rotation[2], 0.]);
    let p = rot.inverse() * (point - Vec3::new(-r.position[0], r.position[1], r.position[2]));
    match (r.shape, r.dimensions.as_slice()) {
        (5, [w, d, h]) => p.x.abs() <= w / 2. && p.z.abs() <= d / 2. && p.y >= 0. && p.y <= *h,
        (2, [radius]) => p.length_squared() <= radius * radius,
        _ => false,
    }
}

/// The event regions inside which the map's camera-front weather is deleted: Outskirts event
/// 11105107 (region 1102107, a temple building west of the Sculptor's hall) and Castle event
/// 11115170 (composites 1112150/1112151, the keep's atria and attic). These are the game's only
/// data rule for indoor snow; elsewhere a roof hides it ([`WeatherCover`], #79).
const SNOW_OFF_REGIONS: [i32; 3] = [1102107, 1112150, 1112151];

/// Whether Wolf at `point` stands in one of the block's [`SNOW_OFF_REGIONS`].
fn snow_off(regions: &[MapRegion], point: Vec3) -> bool {
    regions
        .iter()
        .enumerate()
        .any(|(i, r)| SNOW_OFF_REGIONS.contains(&r.entity_id) && contains(regions, i, point, 0))
}

/// Top-down cover for falling weather (#79): a particle under any play collision surface (a
/// roof, an eave, a bridge) is removed, so snow stops under the Sculptor's hall roof while it
/// keeps falling in the open yard. The game hides it there too (seen in the real game), but the
/// engine rule behind that is not traced: no MSB region or event covers the hall, and the FXR
/// schemas name a field `unkHideIndoors` (set on the snow 839500) without a known source for
/// "indoors". This is the observed behaviour, not the exe's mechanism (docs/MAP-FX.md).
#[derive(Default)]
struct WeatherCover {
    world: Option<Arc<CollisionWorld>>,
    /// The highest collision surface over each [`Self::CELL`] square, filled as particles reach it.
    tops: HashMap<(i32, i32), f32>,
}

impl WeatherCover {
    /// The cover's horizontal resolution in metres.
    const CELL: f32 = 0.5;

    fn covered(&mut self, world: &Arc<CollisionWorld>, p: Vec3) -> bool {
        if !self.world.as_ref().is_some_and(|w| Arc::ptr_eq(w, world)) || self.tops.len() > 1 << 16
        {
            self.world = Some(world.clone());
            self.tops.clear();
        }
        let cell = (
            (p.x / Self::CELL).floor() as i32,
            (p.z / Self::CELL).floor() as i32,
        );
        let top = *self.tops.entry(cell).or_insert_with(|| {
            let (x, z) = (
                (cell.0 as f32 + 0.5) * Self::CELL,
                (cell.1 as f32 + 0.5) * Self::CELL,
            );
            world
                .surfaces_at(x, z)
                .first()
                .map_or(f32::NEG_INFINITY, |s| s.0)
        });
        p.y < top
    }

    /// Removes the covered particles of the weather effects `effects`; returns how many.
    fn cull(
        &mut self,
        player: &mut FxrPlayer,
        world: &Arc<CollisionWorld>,
        effects: impl IntoIterator<Item = i32>,
    ) -> usize {
        effects
            .into_iter()
            .map(|id| player.cull_map_weather(id, &mut |p| self.covered(world, p)))
            .sum()
    }
}

#[allow(clippy::too_many_arguments)]
fn update(
    fx: Res<MapEffects>,
    mut player: ResMut<FxrPlayer>,
    cameras: Query<
        (
            &GlobalTransform,
            &Camera,
            Option<&bevy::camera::visibility::RenderLayers>,
        ),
        With<Camera3d>,
    >,
    focus: Query<&GlobalTransform, With<crate::map::MapFocus>>,
    world: Option<Res<MapWorld>>,
    time: Res<Time>,
    clock: Res<Time<Virtual>>,
    real: Res<Time<Real>>,
    blocks: Option<Res<crate::map::visibility::VisibilityBlocks>>,
    found: Option<Res<FoundIdols>>,
    mut next_log: Local<f32>,
    mut weather_log: Local<Option<(bool, bool)>>,
    collision: Option<Res<crate::play::MapCollision>>,
    mut cover: Local<WeatherCover>,
) {
    if clock.is_paused()
        && blocks
            .as_ref()
            .is_some_and(|b| b.0.values().any(|b| b.groups_off))
    {
        return;
    }
    let Some((camera, _, _)) = cameras.iter().find(|(_, c, layers)| {
        c.is_active
            && c.order == 0
            && layers
                .is_none_or(|l| l.intersects(&bevy::camera::visibility::RenderLayers::layer(0)))
    }) else {
        return;
    };
    let point = focus
        .iter()
        .next()
        .map_or(camera.translation(), |f| f.translation());
    let disabled = std::env::var_os("SEKIRO_NO_MAP_FX").is_some();
    let (mut weather_on, mut weather_off_indoors) = (false, false);
    for (i, p) in fx.placements.iter().enumerate() {
        let ready = world.as_ref().is_none_or(|w| {
            w.blocks
                .get(&p.block)
                .is_some_and(|b| b.state == RequestState::Ready)
        });
        let activation = p.activation.as_ref().is_none_or(|part| {
            blocks.as_ref().is_none_or(|blocks| {
                blocks.0.get(&p.block).is_some_and(|block| {
                    crate::map::visibility::decide(part, block, p.enabled).residency
                        != crate::map::visibility::ResidencyRequest::None
                })
            })
        });
        let indoors = p.weather
            && fx
                .regions
                .get(&p.block)
                .is_some_and(|regions| snow_off(regions, point));
        let idle = p.idol.is_none_or(|(entity, idle)| {
            idle.shows(found.as_ref().is_some_and(|f| f.0.contains(&entity)))
        });
        let on = !disabled
            && p.enabled
            && idle
            && ready
            && activation
            && !indoors
            && (!p.camera_relative
                || world.as_ref().is_none_or(|w| {
                    w.selected
                        .as_ref()
                        .map_or(w.primary == p.block, |(block, _)| *block == p.block)
                }))
            && (p.camera_relative
                || p.transform
                    .translation
                    .distance_squared(camera.translation())
                    < 120f32.powi(2));
        weather_on |= p.weather && on;
        weather_off_indoors |= indoors;
        let at = if p.camera_relative {
            Transform::from_translation(camera.translation())
        } else {
            p.transform
        };
        player.map_effect(
            p.effect,
            i as i32,
            at,
            on,
            clock.is_paused() && std::env::var_os("SEKIRO_MAP_FX_REALTIME").is_none(),
        );
    }
    if *weather_log != Some((weather_on, weather_off_indoors)) {
        *weather_log = Some((weather_on, weather_off_indoors));
        info!(
            "map FX weather: falling {weather_on}, in a snow-off region {weather_off_indoors}, Wolf at {:.1}",
            point
        );
    }
    let dt = if std::env::var_os("SEKIRO_MAP_FX_REALTIME").is_some() {
        real.delta_secs()
    } else if clock.is_paused() {
        0.
    } else {
        time.delta_secs()
    };
    player.advance_map(dt.min(1. / 15.));
    let culled = collision.as_ref().map_or(0, |collision| {
        let _timer = crate::perf_log::SystemTimer::start("weather cover");
        cover.cull(&mut player, &collision.0, fx.weather_effects())
    });
    // Weather density around the camera twice a second, to check it falls steadily (#84).
    if std::env::var_os("SEKIRO_FXR_LOG").is_some() && time.elapsed_secs() >= *next_log {
        *next_log = time.elapsed_secs() + 0.5;
        // Counts sum every placement of an effect, so log each effect once.
        for effect in fx.weather_effects() {
            info!(
                "weather {} near camera {} at {:.1?}, {culled} covered this frame",
                effect,
                player.map_particles_near(effect, camera.translation(), 5.),
                camera.translation()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regions(map: &str) -> Vec<MapRegion> {
        let path = sekiro_sim::cache::root().join(format!("raw/map/mapstudio/{map}.msb"));
        sekiro_formats::msb::parse(&std::fs::read(path).unwrap())
            .unwrap()
            .regions
    }

    /// Points are play-space feet positions (MSB x negated) on the collision floor.
    #[test]
    fn outskirts_snow_stops_only_inside_the_authored_temple_region() {
        let r = regions("m11_00_00_00");
        // Inside the temple building region 1102107 covers (collision h023200, LightSet 110).
        assert!(snow_off(&r, Vec3::new(-94.4, -51.37, -154.1)));
        assert!(snow_off(&r, Vec3::new(-94.4, -46.0, -154.1)));
        // Above its 8 m box, and in the yard by the Dilapidated Temple idol.
        assert!(!snow_off(&r, Vec3::new(-94.4, -43.0, -154.1)));
        assert!(!snow_off(&r, Vec3::new(-139.2, -50.7, -162.1)));
        // The Sculptor's hall (start 0, collision h022400, LightSet 110) has no region: the
        // event keeps the snow on, and only its roof hides it there (the test below).
        assert!(!snow_off(&r, Vec3::new(-125.8, -47.23, -193.08)));
    }

    #[test]
    fn castle_snow_stops_inside_the_keep_composite() {
        let r = regions("m11_01_00_00");
        // Child box "atrium 4F B" of composite 1112150, 38.4 x 48.6 x 10.19 m.
        assert!(snow_off(&r, Vec3::new(112.761, 30.0, 236.39)));
        // The rooftop fighting floor above the keep is outside every box.
        assert!(!snow_off(&r, Vec3::new(112.761, 54.04, 236.39)));
    }

    /// #79: in the real game no snow falls inside the Sculptor's hall (start 0), which no
    /// snow-off region covers; the camera snow 839500 now loses every flake under the hall's
    /// roof (collision 6 to 8.7 m over the floor) and keeps falling in the open yard.
    #[test]
    fn snow_stops_under_the_sculptors_hall_roof_and_falls_in_the_yard() {
        const SNOW: i32 = 839500;
        let cache = sekiro_sim::cache::root();
        let world = Arc::new(CollisionWorld::load_map(&cache.join("maps/m11_00_00_00")).unwrap());
        // Flakes within 5 m of the eye, from 3 m below it up to `ceiling`, after 3 s of snow.
        let falling = |eye: Vec3, ceiling: f32, covered: bool| {
            let mut player = FxrPlayer::new(cache.clone());
            let mut cover = WeatherCover::default();
            let at = Transform::from_translation(eye);
            for _ in 0..90 {
                player.map_effect(SNOW, 0, at, true, false);
                player.advance_map(1. / 30.);
                if covered {
                    cover.cull(&mut player, &world, [SNOW]);
                }
            }
            player.map_particles_where(SNOW, |p| {
                (p - eye).xz().length() < 5. && p.y > eye.y - 3. && p.y < ceiling
            })
        };
        // Wolf's eye at start 0 in the hall (floor -47.23). The roof slopes from -38.5 over the
        // middle to -41.2 at the eaves; flakes above that lie over the roof, outside.
        let hall = Vec3::new(-125.8, -45.6, -193.08);
        let before = falling(hall, -41.5, false);
        assert!(
            before > 20,
            "the uncovered snow falls in the hall: {before}"
        );
        assert_eq!(falling(hall, -41.5, true), 0, "snow under the hall roof");
        // The yard by the Dilapidated Temple idol is open sky (floor -50.76).
        let yard = Vec3::new(-139.2, -49.1, -162.1);
        let open = falling(yard, yard.y + 8., true);
        let uncovered = falling(yard, yard.y + 8., false);
        assert!(
            open > 20 && open * 10 >= uncovered * 9,
            "snow in the yard: {open} of {uncovered}"
        );
    }
}

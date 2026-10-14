//! Event-controlled map parts, objects and placed effects for arena staging.

use std::collections::HashMap;
use std::path::Path;

use crate::map::visibility::{ArenaVisibility, EventState, VisibilityInputs, VisibilityPart};
use bevy::prelude::*;
use sekiro_sim::arena::scene::SceneState;
use sekiro_sim::arena_b::MapEffect;

#[derive(Component, Clone)]
pub struct Part {
    pub entity: i32,
    pub groups: [i32; 8],
}

#[derive(Resource, Default)]
pub struct Parts(pub HashMap<String, Part>);

#[derive(Resource, Default)]
pub struct Scene(pub SceneState);

#[derive(Resource)]
struct Effects(Vec<MapEffect>);

#[derive(Resource)]
struct StoryHour {
    initial: f32,
    explicit: bool,
}

#[derive(Resource)]
struct VisibilityMap(String);

pub fn install(app: &mut App, cache: &Path, map: &str) -> anyhow::Result<()> {
    let initial = app.world().resource::<crate::lighting::MapLighting>().hour;
    let bytes = std::fs::read(cache.join(format!("raw/map/MapStudio/{map}.msb")))?;
    let msb = sekiro_formats::msb::parse(&bytes)?;
    let parts = msb
        .parts
        .into_iter()
        .map(|p| {
            (
                p.name,
                Part {
                    entity: p.entity_id,
                    groups: p.entity_group_ids,
                },
            )
        })
        .collect();
    app.insert_resource(Parts(parts))
        .insert_resource(VisibilityMap(map.to_string()))
        .init_resource::<ArenaVisibility>()
        .insert_resource(StoryHour {
            initial,
            explicit: std::env::args().any(|a| a == "--hour"),
        })
        .init_resource::<Scene>()
        .insert_resource(Effects(
            sekiro_sim::arena_b::map_effects(cache, map).map_err(anyhow::Error::msg)?,
        ))
        .add_systems(PostUpdate, visibility.in_set(VisibilityInputs))
        .add_systems(PostUpdate, (effects, lighting));
    Ok(())
}

fn visibility(
    scene: Res<Scene>,
    map: Res<VisibilityMap>,
    mut arena: ResMut<ArenaVisibility>,
    mut parts: Query<(&Part, &mut Visibility), Without<VisibilityPart>>,
) {
    // Managed roots receive one combined decision in Job E's writer.
    // Replace the snapshot even when empty, so restarts release the previous override.
    arena.maps.insert(
        map.0.clone(),
        EventState {
            by_name: Default::default(),
            by_entity: scene.0.enabled.iter().map(|(&id, &on)| (id, on)).collect(),
        },
    );
    for (part, mut visibility) in &mut parts {
        let on = scene
            .0
            .enabled
            .get(&part.entity)
            .or_else(|| part.groups.iter().find_map(|g| scene.0.enabled.get(g)));
        if let Some(&on) = on {
            *visibility = if on {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}

fn effects(scene: Res<Scene>, effects: Res<Effects>, mut fx: ResMut<crate::fxr::FxrPlayer>) {
    static DISABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let disabled = *DISABLED.get_or_init(|| std::env::args().any(|a| a == "--no-arena-effects"));
    for effect in &effects.0 {
        // An absent entry after event restart releases the previous scene's hold.
        let on = scene
            .0
            .effects
            .get(&effect.entity)
            .copied()
            .unwrap_or(false);
        let [rx, ry, rz] = effect.rotation.map(f32::to_radians);
        let at = Transform::from_translation(effect.position.into()).with_rotation(
            Quat::from_rotation_y(ry) * Quat::from_rotation_z(rz) * Quat::from_rotation_x(rx),
        );
        fx.hold(effect.effect, effect.entity, at, on && !disabled);
    }
}

fn lighting(
    scene: Res<Scene>,
    story: Res<StoryHour>,
    mut light: ResMut<crate::lighting::MapLighting>,
    mut probes: ResMut<crate::lighting::MapProbes>,
    selection: Res<crate::lighting_selection::LightingSelection>,
) {
    if story.explicit {
        return;
    }
    let hour = scene
        .0
        .lighting
        .and_then(sekiro_sim::story::StoryTime::from_variant)
        .map_or(story.initial, sekiro_sim::story::StoryTime::hour);
    if light.hour != hour {
        light.hour = hour;
        light.applied = None;
        probes.variant = selection.environment_for_hour(hour);
        probes.active = None;
        probes.handles.clear();
    }
    if let Some(variant) = scene.0.environment
        && probes.variant != u32::from(variant)
    {
        probes.variant = u32::from(variant);
        probes.active = None;
        probes.handles.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_roots_combine_groups_events_and_residency_across_restarts() {
        use crate::map::visibility::{
            AuthoredVisibility, BlockRequest, BlockVisibility, MapVisibilityPlugin, SourceMasks,
            VisibilityBlocks, VisibilityDecision,
        };
        let map = "m11_01_00_00";
        let mut app = App::new();
        app.add_plugins(MapVisibilityPlugin)
            .insert_resource(Scene::default())
            .insert_resource(VisibilityMap(map.into()))
            .add_systems(PostUpdate, visibility.in_set(VisibilityInputs));
        let mut draw = [0; 8];
        draw[0] = 1;
        let source = SourceMasks {
            primary_draw: draw,
            alternative_draw: [0; 8],
            draw_selector: [0; 8],
            primary_backread: draw,
            alternative_backread: [0; 8],
            backread_selector: [0; 8],
            condition1: 0,
            condition2: 0,
        };
        let root = app
            .world_mut()
            .spawn((
                VisibilityPart {
                    map: map.into(),
                    name: "door".into(),
                    entity_id: 1101000,
                    entity_groups: [-1; 8],
                    effective: source.effective("Object"),
                    source,
                },
                Part {
                    entity: 1101000,
                    groups: [-1; 8],
                },
                Visibility::Inherited,
            ))
            .id();
        let block = BlockVisibility {
            resident: true,
            ready: true,
            draw,
            request: BlockRequest {
                normal: draw,
                ..default()
            },
            ..default()
        };
        app.world_mut()
            .resource_mut::<VisibilityBlocks>()
            .0
            .insert(map.into(), block);
        app.world_mut()
            .resource_mut::<Scene>()
            .0
            .enabled
            .insert(1101000, true);
        app.world_mut()
            .resource_mut::<VisibilityBlocks>()
            .0
            .get_mut(map)
            .unwrap()
            .draw = [0; 8];
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(root).unwrap(),
            Visibility::Hidden
        );
        let changed = app
            .world()
            .entity(root)
            .get_ref::<Visibility>()
            .unwrap()
            .last_changed();
        app.update();
        assert_eq!(
            app.world()
                .entity(root)
                .get_ref::<Visibility>()
                .unwrap()
                .last_changed(),
            changed,
            "arena staging must not write a managed root even when the final state is hidden"
        );
        app.world_mut()
            .resource_mut::<VisibilityBlocks>()
            .0
            .get_mut(map)
            .unwrap()
            .draw = draw;
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(root).unwrap(),
            Visibility::Inherited
        );
        app.world_mut()
            .resource_mut::<Scene>()
            .0
            .enabled
            .insert(1101000, false);
        app.update();
        assert!(!app.world().get::<VisibilityDecision>(root).unwrap().visible);
        app.world_mut().resource_mut::<Scene>().0 = SceneState::default();
        app.world_mut()
            .resource_mut::<VisibilityBlocks>()
            .0
            .get_mut(map)
            .unwrap()
            .ready = false;
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(root).unwrap(),
            Visibility::Hidden
        );
        app.world_mut()
            .resource_mut::<VisibilityBlocks>()
            .0
            .get_mut(map)
            .unwrap()
            .ready = true;
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(root).unwrap(),
            Visibility::Inherited
        );
        app.world_mut()
            .resource_mut::<AuthoredVisibility>()
            .maps
            .insert(
                map.into(),
                EventState {
                    by_name: std::collections::BTreeMap::from([("door".into(), false)]),
                    ..default()
                },
            );
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(root).unwrap(),
            Visibility::Hidden
        );
    }

    #[test]
    fn restart_releases_held_map_effects_for_twenty_fights() {
        let cache = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cache");
        let placed = sekiro_sim::arena_b::map_effects(&cache, "m11_01_00_00").unwrap();
        let mut app = App::new();
        app.insert_resource(Scene::default())
            .insert_resource(Effects(placed))
            .insert_resource(crate::fxr::FxrPlayer::new(cache))
            .add_systems(Update, effects);
        let entities: Vec<_> = app
            .world()
            .resource::<Effects>()
            .0
            .iter()
            .map(|e| e.entity)
            .collect();
        for fight in 0..20 {
            for &entity in &entities {
                app.world_mut()
                    .resource_mut::<Scene>()
                    .0
                    .effects
                    .insert(entity, true);
            }
            app.update();
            let active = app
                .world()
                .resource::<crate::fxr::FxrPlayer>()
                .live_counts()
                .1;
            assert!(active > 0, "installed effects must be exercised");
            // Restart replaces SceneState with a fresh event scene, without off entries.
            app.world_mut().resource_mut::<Scene>().0 = SceneState::default();
            app.update();
            assert_eq!(
                app.world()
                    .resource::<crate::fxr::FxrPlayer>()
                    .live_counts()
                    .1,
                0,
                "fight {fight}"
            );
            app.world_mut()
                .resource_mut::<crate::fxr::FxrPlayer>()
                .advance(31.);
            assert_eq!(
                app.world()
                    .resource::<crate::fxr::FxrPlayer>()
                    .live_counts(),
                (0, 0, 0)
            );
        }
    }
}

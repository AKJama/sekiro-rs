//! Event-controlled map parts, objects and placed effects for arena staging.

use std::collections::HashMap;
use std::path::Path;

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
        .insert_resource(StoryHour {
            initial,
            explicit: std::env::args().any(|a| a == "--hour"),
        })
        .init_resource::<Scene>()
        .insert_resource(Effects(
            sekiro_sim::arena_b::map_effects(cache, map).map_err(anyhow::Error::msg)?,
        ))
        .add_systems(PostUpdate, (visibility, effects, lighting));
    Ok(())
}

fn visibility(scene: Res<Scene>, mut parts: Query<(&Part, &mut Visibility)>) {
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

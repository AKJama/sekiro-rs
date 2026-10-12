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
    for effect in &effects.0 {
        let Some(&on) = scene.0.effects.get(&effect.entity) else {
            continue;
        };
        let [rx, ry, rz] = effect.rotation.map(f32::to_radians);
        let at = Transform::from_translation(effect.position.into()).with_rotation(
            Quat::from_rotation_y(ry) * Quat::from_rotation_z(rz) * Quat::from_rotation_x(rx),
        );
        fx.hold(effect.effect, effect.entity, at, on);
    }
}

fn lighting(
    scene: Res<Scene>,
    story: Res<StoryHour>,
    mut light: ResMut<crate::lighting::MapLighting>,
    mut probes: ResMut<crate::lighting::MapProbes>,
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
        probes.variant = crate::lighting::nearest_variant(hour);
        probes.active = None;
    }
}

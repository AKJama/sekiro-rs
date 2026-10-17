//! GLTF StandardMaterials are source descriptions for map wrappers, rather than extra draws.
//! Prepare only descriptors referenced by actual StandardMaterial mesh entities.
use bevy::prelude::*;
use bevy::render::erased_render_asset::ExtractedAssets;
use bevy::render::{Extract, ExtractSchedule, Render, RenderApp, RenderSystems};
use std::collections::HashSet;

#[derive(Resource, Default)]
struct UsedStandardMaterials {
    used: HashSet<AssetId<StandardMaterial>>,
    previous: HashSet<AssetId<StandardMaterial>>,
    newly_used: Vec<(AssetId<StandardMaterial>, StandardMaterial)>,
}

pub(crate) fn install(app: &mut App) {
    if std::env::var_os("SEKIRO_ALL_SOURCE_MATERIALS").is_some() {
        return;
    }
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        render
            .init_resource::<UsedStandardMaterials>()
            .add_systems(ExtractSchedule, extract_used)
            .add_systems(
                Render,
                filter_unused
                    .after(RenderSystems::ExtractCommands)
                    .before(RenderSystems::PrepareAssets),
            );
    }
}

fn extract_used(
    meshes: Extract<Query<&MeshMaterial3d<StandardMaterial>, With<Mesh3d>>>,
    assets: Extract<Res<Assets<StandardMaterial>>>,
    mut used: ResMut<UsedStandardMaterials>,
) {
    let current: HashSet<_> = meshes.iter().map(|m| m.0.id()).collect();
    used.newly_used = current
        .difference(&used.used)
        .filter_map(|id| assets.get(*id).map(|asset| (*id, asset.clone())))
        .collect();
    used.previous = std::mem::replace(&mut used.used, current);
}

fn filter_unused(
    mut used: ResMut<UsedStandardMaterials>,
    mut extracted: ResMut<ExtractedAssets<MeshMaterial3d<StandardMaterial>>>,
) {
    let _timer = crate::perf_log::SystemTimer::start("source material residency");
    extracted.extracted.retain(|(id, _)| used.used.contains(id));
    extracted.added.retain(|id| used.used.contains(id));
    extracted.modified.retain(|id| used.used.contains(id));
    // Retire former draws even when a GLTF scene continues holding its source handle.
    extracted
        .removed
        .extend(used.previous.difference(&used.used).copied());
    let present: HashSet<_> = extracted.extracted.iter().map(|(id, _)| *id).collect();
    for (id, material) in used.newly_used.drain(..) {
        // A newly spawned mesh can reference an old asset, without an Added/Modified event.
        if !present.contains(&id) {
            extracted.extracted.push((id, material));
            extracted.added.insert(id);
        }
    }
}

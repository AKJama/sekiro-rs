//! Upload reusable source meshes only while an entity references them.
//! CPU sources remain available for streamed instances and static-batch fallback.
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{MeshMetadataFallbackMesh, RenderMesh};
use bevy::render::render_asset::{AssetExtractionSystems, ExtractedAssets};
use bevy::render::{Extract, ExtractSchedule, RenderApp};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

#[derive(Resource, Default)]
struct SharedMeshes {
    buckets: HashMap<u64, Vec<AssetId<Mesh>>>,
    aliases: HashMap<AssetId<Mesh>, Handle<Mesh>>,
    seen: HashSet<AssetId<Mesh>>,
}

#[derive(Resource, Default)]
struct Revision(u64);

#[derive(Resource, Default)]
struct Resident {
    revision: Option<u64>,
    referenced: HashSet<AssetId<Mesh>>,
    suspended: HashSet<AssetId<Mesh>>,
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<Revision>()
        .init_resource::<SharedMeshes>()
        .add_systems(Update, share_sources)
        .add_systems(Last, track);
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        render
            .init_resource::<Resident>()
            .add_systems(ExtractSchedule, extract.after(AssetExtractionSystems));
    }
}

fn share_sources(
    assets: Res<Assets<Mesh>>,
    server: Res<AssetServer>,
    mut state: ResMut<SharedMeshes>,
    mut changed: Query<&mut Mesh3d, Changed<Mesh3d>>,
    mut events: MessageReader<bevy::asset::AssetEvent<Mesh>>,
) {
    for event in events.read() {
        if let bevy::asset::AssetEvent::Unused { id } = event {
            state.aliases.remove(id);
            state.seen.remove(id);
        }
    }
    for mut handle in &mut changed {
        let id = handle.id();
        if let Some(canonical) = state.aliases.get(&id) {
            handle.0 = canonical.clone();
            continue;
        }
        if state.seen.contains(&id)
            || !server.get_path(id).is_some_and(|p| {
                p.path()
                    .components()
                    .next()
                    .is_some_and(|c| c.as_os_str() == "maps")
            })
        {
            continue;
        }
        let Some(mesh) = assets.get(id) else {
            continue;
        };
        if !mesh.asset_usage.contains(RenderAssetUsages::MAIN_WORLD)
            || mesh.try_attributes().is_err()
            || mesh.contains_attribute(Mesh::ATTRIBUTE_JOINT_INDEX)
            || mesh.morph_targets().is_some()
        {
            continue;
        }
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        mesh.primitive_topology().hash(&mut hash);
        for (attribute, values) in mesh.attributes() {
            attribute.id.hash(&mut hash);
            attribute.format.hash(&mut hash);
            values.get_bytes().hash(&mut hash);
        }
        mesh.get_index_buffer_bytes().hash(&mut hash);
        let key = hash.finish();
        let bucket = state.buckets.entry(key).or_default();
        bucket.retain(|candidate| assets.contains(*candidate));
        let canonical = bucket
            .iter()
            .copied()
            .find(|candidate| assets.get(*candidate) == Some(mesh));
        if let Some(canonical) = canonical.and_then(|id| server.get_id_handle(id)) {
            state.aliases.insert(id, canonical.clone());
            handle.0 = canonical;
        } else {
            bucket.push(id);
            state.seen.insert(id);
        }
    }
}

fn track(
    changed: Query<(), Changed<Mesh3d>>,
    mut removed: RemovedComponents<Mesh3d>,
    mut revision: ResMut<Revision>,
) {
    if removed.read().count() > 0 || !changed.is_empty() {
        revision.0 += 1;
    }
}

fn extract(
    meshes: Extract<Query<&Mesh3d>>,
    assets: Extract<Res<Assets<Mesh>>>,
    revision: Extract<Res<Revision>>,
    fallback: Res<MeshMetadataFallbackMesh>,
    mut resident: ResMut<Resident>,
    mut extracted: ResMut<ExtractedAssets<RenderMesh>>,
) {
    if resident.revision != Some(revision.0) {
        resident.revision = Some(revision.0);
        let mut referenced: HashSet<_> = meshes.iter().map(|m| m.0.id()).collect();
        referenced.insert(fallback.0.id());
        let retired: Vec<_> = resident
            .referenced
            .difference(&referenced)
            .copied()
            .collect();
        for id in retired {
            if assets
                .get(id)
                .is_some_and(|m| m.asset_usage.contains(RenderAssetUsages::MAIN_WORLD))
            {
                extracted.removed.insert(id);
                resident.suspended.insert(id);
            }
        }
        resident.referenced = referenced;
    }
    reconcile(&assets, &mut resident, &mut extracted);
}

fn reconcile(
    assets: &Assets<Mesh>,
    resident: &mut Resident,
    extracted: &mut ExtractedAssets<RenderMesh>,
) {
    // Retire GLTF primitives that have not been instantiated yet. Their main-world data
    // is still owned by the GLTF, so it can be extracted when a later LOD needs it.
    extracted.extracted.retain(|(id, mesh)| {
        let keep = !mesh.asset_usage.contains(RenderAssetUsages::MAIN_WORLD)
            || resident.referenced.contains(id);
        if !keep {
            resident.suspended.insert(*id);
            extracted.added.remove(id);
            extracted.modified.remove(id);
            extracted.removed.insert(*id);
        }
        keep
    });
    let resume: Vec<_> = resident
        .suspended
        .intersection(&resident.referenced)
        .copied()
        .collect();
    for id in resume {
        resident.suspended.remove(&id);
        if !extracted.added.contains(&id)
            && let Some(mesh) = assets.get(id)
            && mesh.try_attributes().is_ok()
        {
            extracted.extracted.push((id, mesh.clone()));
            extracted.added.insert(id);
        }
        extracted.removed.remove(&id);
    }
    resident.suspended.retain(|id| assets.contains(*id));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merged_sources_release_gpu_buffers_and_can_be_restored() {
        let mut assets = Assets::<Mesh>::default();
        let handle = assets.add(Mesh::from(Triangle3d::new(Vec3::ZERO, Vec3::X, Vec3::Y)));
        let id = handle.id();
        let mut resident = Resident::default();
        let mut extracted = ExtractedAssets::<RenderMesh>::default();
        extracted
            .extracted
            .push((id, assets.get(id).unwrap().clone()));
        extracted.added.insert(id);
        reconcile(&assets, &mut resident, &mut extracted);
        assert!(extracted.extracted.is_empty());
        assert!(extracted.removed.contains(&id));
        assert!(!extracted.added.contains(&id));
        resident.referenced.insert(id);
        reconcile(&assets, &mut resident, &mut extracted);
        assert!(!extracted.removed.contains(&id));
        assert_eq!(extracted.extracted[0].1.count_vertices(), 3);
        assert!(extracted.added.contains(&id));
    }
}

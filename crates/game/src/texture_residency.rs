//! Share identical immutable DDS uploads across connected blocks, preserving sampler identity.
use bevy::prelude::*;
use bevy::render::render_asset::{AssetExtractionSystems, ExtractedAssets, RenderAssets};
use bevy::render::texture::GpuImage;
use bevy::render::{ExtractSchedule, MainWorld, Render, RenderApp, RenderSystems};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Resource, Default)]
struct Textures {
    buckets: HashMap<u64, Vec<AssetId<Image>>>,
    aliases: HashMap<AssetId<Image>, Handle<Image>>,
}

pub(crate) fn install(app: &mut App) {
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        render
            .init_resource::<Textures>()
            .add_systems(ExtractSchedule, intern.after(AssetExtractionSystems))
            .add_systems(
                Render,
                share
                    .in_set(RenderSystems::PrepareAssets)
                    .after(bevy::render::render_asset::prepare_assets::<GpuImage>)
                    .before(
                        bevy::render::erased_render_asset::prepare_erased_assets::<
                            MeshMaterial3d<StandardMaterial>,
                        >,
                    )
                    .before(
                        bevy::render::erased_render_asset::prepare_erased_assets::<
                            MeshMaterial3d<crate::map::LayeredMaterial>,
                        >,
                    ),
            );
    }
}

fn fingerprint(image: &Image) -> u64 {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    format!(
        "{:?}/{:?}/{:?}/{:?}/{:?}/{}",
        image.texture_descriptor,
        image.sampler,
        image.texture_view_descriptor,
        image.data_order,
        image.asset_usage,
        image.copy_on_resize
    )
    .hash(&mut hash);
    image.data.hash(&mut hash);
    hash.finish()
}

fn intern(
    mut main: ResMut<MainWorld>,
    mut state: ResMut<Textures>,
    mut extracted: ResMut<ExtractedAssets<GpuImage>>,
) {
    main.resource_scope(|world, mut images: Mut<Assets<Image>>| {
        let assets = world.resource::<AssetServer>();
        let extracted = &mut *extracted;
        for id in &extracted.removed {
            state.aliases.remove(id);
        }
        extracted.extracted.retain(|(id, image)| {
            if state.aliases.contains_key(id) {
                extracted.added.remove(id);
                extracted.modified.remove(id);
                return false;
            }
            if image.data.is_none()
                || !assets.get_path(*id).is_some_and(|p| {
                    p.path()
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("dds"))
                })
            {
                return true;
            }
            let key = fingerprint(image);
            let bucket = state.buckets.entry(key).or_default();
            bucket.retain(|candidate| images.contains(*candidate));
            // Always compare the complete image after hashing, including format, mips and sampler.
            let canonical = bucket
                .iter()
                .copied()
                .find(|candidate| *candidate != *id && images.get(*candidate) == Some(image));
            if let Some(canonical) = canonical.and_then(|id| images.get_strong_handle(id)) {
                state.aliases.insert(*id, canonical);
                extracted.added.remove(id);
                extracted.modified.remove(id);
                extracted.removed.insert(*id);
                // The canonical CPU copy supports exact comparisons of future blocks.
                // Duplicate copies have no CPU consumers and need no pixel buffer.
                if let Some(mut image) = images.get_mut(*id) {
                    image.data = None;
                }
                false
            } else {
                if !bucket.contains(id) {
                    bucket.push(*id);
                }
                true
            }
        });
    });
}

fn share(state: Res<Textures>, mut images: ResMut<RenderAssets<GpuImage>>) {
    for (alias, canonical) in &state.aliases {
        if let Some(image) = images.get(canonical.id())
            && images
                .get(*alias)
                .is_none_or(|alias| alias.texture.id() != image.texture.id())
        {
            let image = image.clone();
            images.insert(*alias, image);
        }
    }
}

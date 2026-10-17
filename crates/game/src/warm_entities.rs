//! Populate per-entity color and cascade caches for resident map placements during loading.
//! Bevy retains these entries while entities are hidden and invalidates actual mesh/material edits.
use bevy::material::key::{ErasedMaterialPipelineKey, ErasedMeshPipelineKey};
use bevy::pbr::{
    LightKeyCache, MeshPipelineKey, PreparedMaterial, ShadowsDepthOnlyDrawFunction,
    ShadowsDrawFunction, SpecializedMaterialPipelineCache, SpecializedShadowMaterialPipelineCache,
    ViewKeyCache,
};
use bevy::prelude::*;
use bevy::render::erased_render_asset::ErasedRenderAssets;
use bevy::render::mesh::RenderMesh;
use bevy::render::render_asset::RenderAssets;
use bevy::render::renderer::RenderDevice;
use bevy::render::sync_world::MainEntity;
use bevy::render::{Extract, ExtractSchedule};
use std::{any::TypeId, collections::VecDeque};

#[derive(Clone, Copy)]
struct Resident {
    entity: MainEntity,
    mesh: AssetId<Mesh>,
    material: bevy::asset::UntypedAssetId,
}

#[derive(Resource, Default)]
struct Residents(Vec<Resident>);

#[derive(Resource, Default)]
struct Queue {
    work: Option<VecDeque<Resident>>,
    completed: usize,
}

pub(super) fn install<M: Material>(render: &mut SubApp) {
    render
        .init_resource::<Residents>()
        .init_resource::<Queue>()
        .add_systems(ExtractSchedule, extract::<M>);
}

fn extract<M: Material>(
    meshes: Extract<Query<(Entity, &Mesh3d, &MeshMaterial3d<M>)>>,
    mut residents: ResMut<Residents>,
    warmup: Res<super::PipelineWarmup>,
) {
    residents.0.clear();
    if warmup.ready() {
        return;
    }
    residents
        .0
        .extend(meshes.iter().map(|(entity, mesh, material)| Resident {
            entity: entity.into(),
            mesh: mesh.0.id(),
            material: material.0.id().untyped(),
        }));
}

pub(super) fn warm<M: Material>(world: &mut World) -> bool {
    let _timer = crate::perf_log::SystemTimer::start("entity pipeline warmup");
    let mut queue = world.remove_resource::<Queue>().unwrap();
    if queue.work.is_none() {
        queue.work = Some(world.resource::<Residents>().0.iter().copied().collect());
    }
    let colors: Vec<_> = world
        .resource::<ViewKeyCache>()
        .iter()
        .map(|(&view, &key)| (view, key))
        .collect();
    let shadows: Vec<_> = world
        .resource::<LightKeyCache>()
        .iter()
        .map(|(&view, &key)| (view, key))
        .collect();
    let depth_clip = world
        .resource::<RenderDevice>()
        .features()
        .contains(wgpu::Features::DEPTH_CLIP_CONTROL);
    for _ in 0..128 {
        let Some(resident) = queue.work.as_mut().unwrap().pop_front() else {
            break;
        };
        let Some(mesh) = world
            .resource::<RenderAssets<RenderMesh>>()
            .get(resident.mesh)
        else {
            continue;
        };
        if mesh.layout.0.contains(Mesh::ATTRIBUTE_JOINT_INDEX) {
            continue;
        }
        let layout = mesh.layout.clone();
        let bits = MeshPipelineKey::from_bits_retain(mesh.key_bits.bits());
        let Some(material) = world
            .resource::<ErasedRenderAssets<PreparedMaterial>>()
            .get(resident.material)
        else {
            continue;
        };
        let properties = material.properties.clone();
        for (shadow, views) in [(false, &colors), (true, &shadows)] {
            if shadow && !properties.shadows_enabled {
                continue;
            }
            let specialize = if shadow {
                properties.prepass_specialize
            } else {
                properties.base_specialize
            };
            let Some(specialize) = specialize else {
                continue;
            };
            for &(view, view_key) in views {
                let mut key = view_key | bits;
                if shadow {
                    if !matches!(
                        properties.alpha_mode,
                        AlphaMode::Opaque | AlphaMode::Multiply
                    ) {
                        key |= MeshPipelineKey::MAY_DISCARD;
                    }
                    if properties
                        .get_shader(bevy::pbr::PrepassVertexShader)
                        .is_some()
                        || properties
                            .get_shader(bevy::pbr::PrepassFragmentShader)
                            .is_some()
                    {
                        key |= MeshPipelineKey::PREPASS_READS_MATERIAL;
                    }
                } else {
                    key |= properties
                        .mesh_pipeline_key_bits
                        .downcast::<MeshPipelineKey>();
                    key |= bevy::pbr::alpha_mode_pipeline_key(
                        properties.alpha_mode,
                        &Msaa::from_samples(view_key.msaa_samples()),
                    );
                }
                let draw = if shadow {
                    let depth_only = !key.intersects(
                        MeshPipelineKey::MAY_DISCARD | MeshPipelineKey::PREPASS_READS_MATERIAL,
                    ) && (depth_clip
                        || !key.contains(MeshPipelineKey::UNCLIPPED_DEPTH_ORTHO));
                    let draw = if depth_only {
                        properties.get_draw_function(ShadowsDepthOnlyDrawFunction)
                    } else {
                        properties.get_draw_function(ShadowsDrawFunction)
                    };
                    let Some(draw) = draw else {
                        continue;
                    };
                    Some(draw)
                } else {
                    None
                };
                let erased = ErasedMaterialPipelineKey {
                    type_id: TypeId::of::<M>(),
                    mesh_key: ErasedMeshPipelineKey::new(key),
                    material_key: properties.material_key.clone(),
                };
                if let Ok(pipeline) = specialize(world, erased, &layout, &properties) {
                    if let Some(draw) = draw {
                        world
                            .resource_mut::<SpecializedShadowMaterialPipelineCache>()
                            .entry(view)
                            .or_default()
                            .insert(resident.entity, (pipeline, draw));
                    } else {
                        world
                            .resource_mut::<SpecializedMaterialPipelineCache>()
                            .entry(view)
                            .or_default()
                            .insert(resident.entity, pipeline);
                    }
                }
            }
        }
        queue.completed += 1;
    }
    let ready = queue.work.as_ref().unwrap().is_empty();
    if ready {
        info!(
            "map entity warmup complete: {} resident mesh placements",
            queue.completed
        );
    }
    world.insert_resource(queue);
    ready
}

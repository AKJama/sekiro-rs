//! Specialize resident map layouts during loading, including layouts only used by other LODs.
use std::any::TypeId;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use bevy::material::{
    MaterialProperties,
    key::{ErasedMaterialPipelineKey, ErasedMeshPipelineKey},
};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{
    LightKeyCache, MeshPipelineKey, PreparedMaterial, RenderMaterialInstances, RenderMeshInstances,
    ViewKeyCache, ViewKeyPrepassCache,
};
use bevy::prelude::*;
use bevy::render::erased_render_asset::ErasedRenderAssets;
use bevy::render::mesh::RenderMesh;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::PipelineCache;
use bevy::render::{Render, RenderApp, RenderSystems};

#[path = "warm_entities.rs"]
mod entities;

#[derive(Resource, Clone, Default)]
pub(crate) struct PipelineWarmup {
    loaded: Arc<AtomicBool>,
    ready: Arc<AtomicBool>,
}

impl PipelineWarmup {
    pub(crate) fn ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }
}

pub(crate) fn install<M: Material>(app: &mut App, args: &[String]) {
    // Fixed captures wait for each pose's exact pipelines themselves, including
    // diagnostic passes that never occur in traversal. Avoid compiling unused variants.
    if args
        .iter()
        .any(|a| matches!(a.as_str(), "--sweep" | "--render-gate" | "--reference-gate"))
    {
        return;
    }
    let warmup = PipelineWarmup::default();
    app.insert_resource(warmup.clone()).add_systems(
        First,
        |pending: Res<crate::map::Pending>,
         batches: Res<crate::map::StaticBatches>,
         warmup: Res<PipelineWarmup>| {
            warmup
                .loaded
                .store(pending.loaded() && batches.ready(), Ordering::Release);
        },
    );
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        entities::install::<M>(render);
        render.insert_resource(warmup).add_systems(
            Render,
            warm_pipelines::<M>
                .after(RenderSystems::Specialize)
                .before(RenderSystems::PrepareViews),
        );
    }
}

struct Work {
    key: ErasedMaterialPipelineKey,
    layout: MeshVertexBufferLayoutRef,
    properties: Arc<MaterialProperties>,
    prepass: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pass {
    Color,
    Prepass,
    Shadow,
}

#[derive(Default)]
struct WarmCache {
    started: bool,
    seen: HashSet<(bool, ErasedMaterialPipelineKey, MeshVertexBufferLayoutRef)>,
    work: VecDeque<Work>,
    queued: usize,
    frames: u32,
}

impl WarmCache {
    fn push(&mut self, work: Work) {
        if self
            .seen
            .insert((work.prepass, work.key.clone(), work.layout.clone()))
        {
            self.work.push_back(work);
        }
    }
}

/// Maximum loading frames spent specializing and compiling the fixed resident set.
const SETTLE_DEADLINE_FRAMES: u32 = 300;

fn warm_pipelines<M: Material>(world: &mut World, mut state: Local<WarmCache>) {
    let warmup = world.resource::<PipelineWarmup>().clone();
    if warmup.ready() || (!state.started && !warmup.loaded.load(Ordering::Acquire)) {
        return;
    }
    let _timer = crate::perf_log::SystemTimer::start("pipeline warmup");
    state.frames += 1;
    if !state.started {
        state.started = true;
        let layouts: HashSet<_> = world
            .resource::<RenderAssets<RenderMesh>>()
            .iter()
            .filter(|(_, mesh)| {
                mesh.layout.0.contains(Mesh::ATTRIBUTE_NORMAL)
                    && mesh.layout.0.contains(Mesh::ATTRIBUTE_UV_0)
            })
            .filter(|(_, mesh)| !mesh.layout.0.contains(Mesh::ATTRIBUTE_JOINT_INDEX))
            .map(|(_, mesh)| (mesh.layout.clone(), mesh.key_bits.bits()))
            .collect();
        let mut by_type: HashMap<TypeId, HashSet<_>> =
            HashMap::from([(TypeId::of::<M>(), layouts)]);
        for (entity, material) in &world.resource::<RenderMaterialInstances>().instances {
            let Some(mesh) = world
                .resource::<RenderMeshInstances>()
                .mesh_asset_id(*entity)
                .and_then(|id| world.resource::<RenderAssets<RenderMesh>>().get(id))
            else {
                continue;
            };
            by_type
                .entry(material.asset_id.type_id())
                .or_default()
                .insert((mesh.layout.clone(), mesh.key_bits.bits()));
        }
        let mut keys = HashSet::new();
        let materials: Vec<_> = world
            .resource::<ErasedRenderAssets<PreparedMaterial>>()
            .iter()
            .filter_map(|(id, material)| {
                let p = &material.properties;
                keys.insert((
                    id.type_id(),
                    p.material_key.clone(),
                    p.mesh_pipeline_key_bits,
                    alpha_key(p.alpha_mode),
                ))
                .then(|| (id.type_id(), p.clone()))
            })
            .collect();
        // Cameras can start without EnvironmentMapLight until their first probe is ready.
        // Probe selection then retains the component at zero intensity between volumes.
        // Warm both initial camera and resident-probe view keys.
        let color: Vec<_> = world
            .resource::<ViewKeyCache>()
            .values()
            .copied()
            .flat_map(|key| {
                [
                    key | MeshPipelineKey::ENVIRONMENT_MAP,
                    key & !MeshPipelineKey::ENVIRONMENT_MAP,
                ]
            })
            .collect();
        let prepass: Vec<_> = world
            .resource::<ViewKeyPrepassCache>()
            .values()
            .copied()
            .collect();
        let shadow: Vec<_> = world
            .resource::<LightKeyCache>()
            .values()
            .copied()
            .collect();
        for (type_id, properties) in materials {
            let Some(layouts) = by_type.get(&type_id) else {
                continue;
            };
            let opaque = matches!(
                properties.render_phase_type,
                bevy::material::RenderPhaseType::Opaque
                    | bevy::material::RenderPhaseType::AlphaMask
            );
            for (layout, bits) in layouts {
                let mesh = MeshPipelineKey::from_bits_retain(*bits);
                for (pass, views) in [
                    (Pass::Color, &color),
                    (Pass::Prepass, &prepass),
                    (Pass::Shadow, &shadow),
                ] {
                    if pass == Pass::Prepass && !opaque {
                        continue;
                    }
                    if pass == Pass::Prepass && !properties.prepass_enabled
                        || pass == Pass::Shadow && !properties.shadows_enabled
                    {
                        continue;
                    }
                    for &view in views {
                        let key = pass_key(pass, view, mesh, layout, &properties);
                        state.push(Work {
                            key: ErasedMaterialPipelineKey {
                                type_id,
                                mesh_key: ErasedMeshPipelineKey::new(key),
                                material_key: properties.material_key.clone(),
                            },
                            layout: layout.clone(),
                            properties: properties.clone(),
                            prepass: pass != Pass::Color,
                        });
                    }
                }
            }
        }
    }
    // Keep compilation requests bounded even on loading frames.
    for _ in 0..16 {
        let Some(work) = state.work.pop_front() else {
            break;
        };
        let specialize = if work.prepass {
            work.properties.prepass_specialize
        } else {
            work.properties.base_specialize
        };
        if let Some(specialize) = specialize {
            // Incompatible layout/material combinations are not renderable and need no pipeline.
            if specialize(world, work.key, &work.layout, &work.properties).is_ok() {
                state.queued += 1;
            }
        }
    }
    // Do not hold loading forever if a pipeline cannot finish compiling.
    if state.frames >= SETTLE_DEADLINE_FRAMES {
        warn!("map pipeline warmup deadline reached");
        state.work.clear();
        state.seen.clear();
        warmup.ready.store(true, Ordering::Release);
        return;
    }
    if state.work.is_empty()
        && world
            .resource::<PipelineCache>()
            .waiting_pipelines()
            .count()
            == 0
    {
        if !entities::warm::<M>(world)
            || world
                .resource::<PipelineCache>()
                .waiting_pipelines()
                .count()
                > 0
        {
            return;
        }
        info!(
            "map pipeline warmup complete: {} compatible layout/pass variants",
            state.queued
        );
        warmup.ready.store(true, Ordering::Release);
        state.seen.clear();
    }
}

/// Preserve mask thresholds without allocating formatted material keys.
fn alpha_key(alpha: AlphaMode) -> (std::mem::Discriminant<AlphaMode>, u32) {
    (
        std::mem::discriminant(&alpha),
        match alpha {
            AlphaMode::Mask(value) => value.to_bits(),
            _ => 0,
        },
    )
}

/// Match Bevy's color, prepass and shadow key rules for both warmup paths.
fn pass_key(
    pass: Pass,
    view: MeshPipelineKey,
    mesh: MeshPipelineKey,
    layout: &MeshVertexBufferLayoutRef,
    properties: &MaterialProperties,
) -> MeshPipelineKey {
    let mut key = view | mesh;
    if pass == Pass::Color {
        key |= properties
            .mesh_pipeline_key_bits
            .downcast::<MeshPipelineKey>();
    }
    if pass != Pass::Shadow {
        key |= bevy::pbr::alpha_mode_pipeline_key(
            properties.alpha_mode,
            &Msaa::from_samples(view.msaa_samples()),
        );
    } else if !matches!(
        properties.alpha_mode,
        AlphaMode::Opaque | AlphaMode::Multiply
    ) {
        key |= MeshPipelineKey::MAY_DISCARD;
    }
    if pass != Pass::Color
        && (properties
            .get_shader(bevy::pbr::PrepassVertexShader)
            .is_some()
            || properties
                .get_shader(bevy::pbr::PrepassFragmentShader)
                .is_some())
    {
        key |= MeshPipelineKey::PREPASS_READS_MATERIAL;
    }
    if pass != Pass::Shadow
        && view.contains(MeshPipelineKey::MOTION_VECTOR_PREPASS)
        && layout.0.contains(Mesh::ATTRIBUTE_JOINT_INDEX)
    {
        key |= MeshPipelineKey::HAS_PREVIOUS_SKIN;
    }
    key
}

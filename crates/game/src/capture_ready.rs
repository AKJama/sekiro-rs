//! Shared asset and completed-render checks for window and offscreen captures.
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use bevy::render::render_resource::{CachedPipelineState, PipelineCache};
use bevy::render::{Render, RenderApp, RenderSystems};

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PipelineStatus {
    pub frame: u64,
    pub ready: usize,
    pub pending: usize,
    pub failed: usize,
}

#[derive(Clone, Resource)]
pub(crate) struct Pipelines(Arc<Mutex<PipelineStatus>>);

impl Default for Pipelines {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(PipelineStatus {
            pending: 1,
            ..default()
        })))
    }
}

impl Pipelines {
    pub fn status(&self) -> PipelineStatus {
        *self.0.lock().unwrap()
    }
}

/// Mesh entities extracted before their mesh reached the GPU.
///
/// Bevy 0.19's material specialization (`specialize_material_meshes` in bevy_pbr) skips an
/// entity whose mesh asset is not prepared yet without queueing a retry, unlike a missing
/// material, and specializes it again only when the entity changes. With the per-frame upload
/// budget (`crate::map::upload_budget`) a new entity can be extracted before its mesh is
/// prepared and then stays undrawn. The render world reports meshes that were not prepared
/// when first seen and later are, and [`watch_meshes`] marks their entities changed so Bevy
/// specializes them again. This runs for the whole session, so meshes streamed or levels of
/// detail spawned during play are covered too.
#[derive(Clone, Default, Resource)]
pub(crate) struct LateMeshes(Arc<Mutex<LateState>>);

#[derive(Default)]
struct LateState {
    /// Main to render: meshes of entities that appeared since the last check.
    new: Vec<AssetId<Mesh>>,
    /// Render: meshes not prepared when first checked.
    waiting: std::collections::HashSet<AssetId<Mesh>>,
    /// Render to main: checked meshes that were prepared in time, and waiting ones now prepared.
    on_time: Vec<AssetId<Mesh>>,
    ready: Vec<AssetId<Mesh>>,
}

impl LateMeshes {
    /// A few of the meshes still waiting, for the loading log.
    pub fn sample(&self) -> Vec<AssetId<Mesh>> {
        self.0
            .lock()
            .unwrap()
            .waiting
            .iter()
            .take(4)
            .copied()
            .collect()
    }

    /// Meshes of spawned entities not yet known to be prepared on the GPU.
    pub fn waiting(&self) -> usize {
        let state = self.0.lock().unwrap();
        state.new.len() + state.waiting.len()
    }
}

/// Main world: report new mesh entities and mark entities changed once their late mesh is
/// prepared. Entities are kept only until the render world has checked their mesh; a mesh
/// handle swapped on an existing entity counts as new too.
#[allow(clippy::type_complexity)]
fn watch_meshes(
    late: Res<LateMeshes>,
    assets: Res<Assets<Mesh>>,
    mut meshes: ParamSet<(
        Query<(Entity, &Mesh3d), Changed<Mesh3d>>,
        Query<&mut Mesh3d>,
    )>,
    mut unchecked: Local<std::collections::HashMap<AssetId<Mesh>, Vec<Entity>>>,
) {
    let (on_time, ready) = {
        let mut state = late.0.lock().unwrap();
        for (entity, mesh) in &meshes.p0() {
            // Placeholder handles (`Mesh3d::default()`) never load; a mesh that is not in the
            // main world has nothing to upload from it.
            if !assets.contains(mesh.id()) {
                continue;
            }
            state.new.push(mesh.id());
            unchecked.entry(mesh.id()).or_default().push(entity);
        }
        (
            std::mem::take(&mut state.on_time),
            std::mem::take(&mut state.ready),
        )
    };
    for id in on_time {
        unchecked.remove(&id);
    }
    for id in ready {
        for entity in unchecked.remove(&id).unwrap_or_default() {
            if let Ok(mut mesh) = meshes.p1().get_mut(entity) {
                mesh.set_changed();
            }
        }
    }
}

/// Render world, after specialization: sort newly reported meshes into prepared and waiting,
/// and report waiting meshes that have been prepared since.
fn check_late_meshes(
    late: Res<LateMeshes>,
    meshes: Res<bevy::render::render_asset::RenderAssets<bevy::render::mesh::RenderMesh>>,
) {
    let mut state = late.0.lock().unwrap();
    let state = &mut *state;
    for id in std::mem::take(&mut state.new) {
        if meshes.get(id).is_some() {
            state.on_time.push(id);
        } else {
            state.waiting.insert(id);
        }
    }
    let prepared: Vec<_> = state
        .waiting
        .iter()
        .copied()
        .filter(|id| meshes.get(*id).is_some())
        .collect();
    for id in prepared {
        state.waiting.remove(&id);
        state.ready.push(id);
    }
}

#[derive(Default, Resource)]
pub(crate) struct CaptureWait {
    requested: bool,
    stable: usize,
    last_frame: u64,
    started: Option<std::time::Instant>,
}

impl CaptureWait {
    pub fn allow(&mut self, pipelines: &Pipelines) -> bool {
        let status = pipelines.status();
        if status.pending > 0 || status.failed > 0 || status.ready == 0 {
            self.stable = 0;
        }
        if self.stable >= 3 {
            self.requested = false;
            self.stable = 0;
            self.started = None;
            info!("capture ready: {status:?}; material textures ready");
            return true;
        }
        if !self.requested {
            info!(
                "capture deferred: waiting for material textures and completed pipelines; {:?}",
                pipelines.status()
            );
        }
        self.requested = true;
        self.started.get_or_insert_with(std::time::Instant::now);
        false
    }

    fn observe(&mut self, status: PipelineStatus, textures_pending: bool) {
        if textures_pending || status.pending > 0 || status.failed > 0 || status.ready == 0 {
            self.stable = 0;
        } else if status.frame != self.last_frame {
            self.stable += 1;
        }
        self.last_frame = status.frame;
    }
}

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct Ready<'w> {
    pipelines: Res<'w, Pipelines>,
    wait: ResMut<'w, CaptureWait>,
    pending: Option<Res<'w, super::Pending>>,
}

impl Ready<'_> {
    pub fn allow(&mut self) -> bool {
        if self.pending.as_ref().is_some_and(|p| !p.loaded()) {
            return false;
        }
        self.wait.allow(&self.pipelines)
    }
}

pub(crate) fn textures_pending(world: &World, entities: &[Entity]) -> bool {
    let mut checked = std::collections::HashSet::new();
    entities.iter().any(|e| {
        if super::map_material::sweep_key(world, *e).is_some_and(|key| !checked.insert(key)) {
            return false;
        }
        super::map_material::sweep_base(world, *e).is_some_and(|m| m.pending)
            || crate::map_fog_surface::sweep_pending(world, *e)
            || world
                .get::<MeshMaterial3d<StandardMaterial>>(*e)
                .is_some_and(|h| {
                    world
                        .resource::<Assets<StandardMaterial>>()
                        .get(&h.0)
                        .is_none()
                })
    })
}

/// Render uploads still outstanding: images loaded in the main world for the render world that
/// it has not prepared yet (queued behind the per-frame upload budget), and meshes of spawned
/// entities not yet known to be prepared ([`LateMeshes`]). Main-world-only images never
/// upload, so they do not count.
pub(crate) fn pending_uploads(world: &World) -> (usize, usize) {
    let images = {
        let uploaded = world
            .resource::<super::map_material::SweepImages>()
            .0
            .lock()
            .unwrap();
        world
            .resource::<Assets<Image>>()
            .iter()
            .filter(|(id, image)| {
                image
                    .asset_usage
                    .contains(bevy::asset::RenderAssetUsages::RENDER_WORLD)
                    && !uploaded.contains(id)
            })
            .count()
    };
    (images, world.resource::<LateMeshes>().waiting())
}

pub(crate) fn install(app: &mut App) {
    if app.world().contains_resource::<Pipelines>() {
        return;
    }
    let pipelines = Pipelines::default();
    let images = super::map_material::SweepImages::default();
    let late = LateMeshes::default();
    app.insert_resource(pipelines.clone())
        .insert_resource(images.clone())
        .insert_resource(late.clone())
        .init_resource::<CaptureWait>()
        .add_systems(PostUpdate, (prepare, watch_meshes));
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        render
            .insert_resource(pipelines)
            .insert_resource(images)
            .insert_resource(late)
            .add_systems(
                Render,
                (record, check_late_meshes).in_set(RenderSystems::PostCleanup),
            );
    }
}

fn record(
    status: Res<Pipelines>,
    cache: Res<PipelineCache>,
    uploaded: Res<super::map_material::SweepImages>,
    images: Res<bevy::render::render_asset::RenderAssets<bevy::render::texture::GpuImage>>,
) {
    *uploaded.0.lock().unwrap() = images.iter().map(|(id, _)| id).collect();
    let mut next = PipelineStatus {
        pending: cache.waiting_pipelines().count(),
        ..default()
    };
    for pipeline in cache.pipelines() {
        match &pipeline.state {
            CachedPipelineState::Ok(_) => next.ready += 1,
            CachedPipelineState::Err(
                bevy::shader::ShaderCacheError::ProcessShaderError(_)
                | bevy::shader::ShaderCacheError::CreateShaderModule(_),
            ) => {
                error!("capture GPU pipeline failed: {:?}", pipeline.state);
                next.failed += 1;
            }
            _ => {}
        }
    }
    let mut state = status.0.lock().unwrap();
    next.frame = state.frame + 1;
    *state = next;
}

fn prepare(world: &mut World) {
    if !world.resource::<CaptureWait>().requested {
        return;
    }
    if world
        .resource::<CaptureWait>()
        .started
        .is_some_and(|t| t.elapsed().as_secs() >= 300)
    {
        error!("capture timed out waiting for material textures and render pipelines");
        world.write_message(AppExit::error());
        return;
    }
    let status = world.resource::<Pipelines>().status();
    let meshes: Vec<_> = world
        .query_filtered::<Entity, With<Mesh3d>>()
        .iter(world)
        .collect();
    let pending = textures_pending(world, &meshes);
    world.resource_mut::<CaptureWait>().observe(status, pending);
    if status.failed > 0 {
        error!("capture aborted: a render pipeline failed");
        world.write_message(AppExit::error());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_waits_for_textures_and_distinct_completed_renders() {
        let mut wait = CaptureWait::default();
        let pipelines = Pipelines::default();
        assert!(!wait.allow(&pipelines));
        let started = wait.started;
        assert!(!wait.allow(&pipelines));
        assert_eq!(
            wait.started, started,
            "pending pipelines must not restart the timeout"
        );
        let mut status = PipelineStatus {
            frame: 1,
            ready: 10,
            ..default()
        };
        wait.observe(status, false);
        wait.observe(status, false);
        assert_eq!(
            wait.stable, 1,
            "repeated main frames cannot settle one render"
        );
        for (pending, failed, textures) in [(1, 0, false), (0, 1, false), (0, 0, true)] {
            status.frame += 1;
            status.pending = pending;
            status.failed = failed;
            wait.observe(status, textures);
            assert_eq!(wait.stable, 0);
        }
        status.pending = 0;
        status.failed = 0;
        for _ in 0..3 {
            status.frame += 1;
            wait.observe(status, false);
        }
        *pipelines.0.lock().unwrap() = status;
        assert!(wait.allow(&pipelines));
        assert!(!wait.allow(&pipelines), "a later shot must settle again");
    }
}

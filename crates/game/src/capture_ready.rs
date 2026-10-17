//! Shared asset and completed-render checks for window and offscreen captures.
use std::collections::HashSet;
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
///
/// It also counts the meshes the world is still loading, for the play gate
/// ([`LateMeshes::loading`]). Bevy marks `Mesh3d` changed every frame for an entity whose mesh
/// or material asset is rewritten (effect batches, cloth, the sky's animated materials), and a
/// rewritten mesh leaves the render world until it is prepared again, so the gate counts only
/// meshes never seen prepared before and skips [`RuntimeMesh`] entities.
#[derive(Clone, Default, Resource)]
pub(crate) struct LateMeshes(Arc<Mutex<LateState>>);

/// A mesh entity that keeps appearing during play (an effect batch), so the play gate does not
/// wait for its mesh. It is still specialized again when its mesh arrives late.
#[derive(Clone, Copy, Component, Default)]
pub(crate) struct RuntimeMesh;

#[derive(Default)]
struct LateState {
    /// Main to render: meshes of entities that appeared since the last check.
    new: Vec<AssetId<Mesh>>,
    /// Render: meshes not prepared when first checked.
    waiting: HashSet<AssetId<Mesh>>,
    /// Render to main: checked meshes that were prepared in time, and waiting ones now prepared.
    on_time: Vec<AssetId<Mesh>>,
    ready: Vec<AssetId<Mesh>>,
    /// Reported meshes of the world's own entities not yet seen prepared: what the play gate
    /// waits for.
    loading: HashSet<AssetId<Mesh>>,
    /// Meshes of the world's own entities seen prepared at least once.
    loaded: HashSet<AssetId<Mesh>>,
}

impl LateState {
    /// Main world: an entity's mesh appeared or changed. `runtime` entities never hold the gate.
    fn report(&mut self, id: AssetId<Mesh>, runtime: bool) {
        self.new.push(id);
        if !runtime && !self.loaded.contains(&id) {
            self.loading.insert(id);
        }
    }

    /// Render world: sort newly reported meshes into prepared and waiting, and report waiting
    /// meshes that have been prepared since.
    fn check(&mut self, prepared: impl Fn(AssetId<Mesh>) -> bool) {
        if self.new.is_empty() && self.waiting.is_empty() {
            return;
        }
        for id in std::mem::take(&mut self.new) {
            if prepared(id) {
                self.on_time.push(id);
                self.mark_loaded(id);
            } else {
                self.waiting.insert(id);
            }
        }
        let mut ready = Vec::new();
        self.waiting.retain(|id| {
            let done = prepared(*id);
            if done {
                ready.push(*id);
            }
            !done
        });
        for id in ready {
            self.mark_loaded(id);
            self.ready.push(id);
        }
    }

    fn mark_loaded(&mut self, id: AssetId<Mesh>) {
        if self.loading.remove(&id) {
            self.loaded.insert(id);
        }
    }

    /// Main world: forget meshes whose asset was removed, so one removed before it was
    /// prepared does not wait forever.
    fn forget_removed(&mut self, exists: impl Fn(AssetId<Mesh>) -> bool) {
        self.waiting.retain(|id| exists(*id));
        self.loading.retain(|id| exists(*id));
        self.loaded.retain(|id| exists(*id));
    }
}

impl LateMeshes {
    /// Meshes of the world's own entities (not [`RuntimeMesh`] ones) not yet seen prepared on
    /// the GPU. A mesh seen prepared once does not count again when its asset or material is
    /// rewritten.
    pub fn loading(&self) -> usize {
        self.0.lock().unwrap().loading.len()
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
        Query<(Entity, &Mesh3d, Has<RuntimeMesh>), Changed<Mesh3d>>,
        Query<&mut Mesh3d>,
    )>,
    mut unchecked: Local<std::collections::HashMap<AssetId<Mesh>, Vec<Entity>>>,
) {
    if unchecked.is_empty() && meshes.p0().is_empty() {
        return;
    }
    let (on_time, ready) = {
        let mut state = late.0.lock().unwrap();
        for (entity, mesh, runtime) in &meshes.p0() {
            // Placeholder handles (`Mesh3d::default()`) never load; a mesh that is not in the
            // main world has nothing to upload from it.
            if !assets.contains(mesh.id()) {
                continue;
            }
            state.report(mesh.id(), runtime);
            unchecked.entry(mesh.id()).or_default().push(entity);
        }
        state.forget_removed(|id| assets.contains(id));
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
    // A mesh asset removed before the render world prepared it never reports back.
    unchecked.retain(|id, _| assets.contains(*id));
}

/// Render world, after specialization: sort newly reported meshes into prepared and waiting,
/// and report waiting meshes that have been prepared since.
fn check_late_meshes(
    late: Res<LateMeshes>,
    meshes: Res<bevy::render::render_asset::RenderAssets<bevy::render::mesh::RenderMesh>>,
) {
    late.0.lock().unwrap().check(|id| meshes.get(id).is_some());
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
    warmup: Option<Res<'w, crate::pipeline_warmup::PipelineWarmup>>,
    batches: Option<Res<'w, super::StaticBatches>>,
}

impl Ready<'_> {
    pub fn allow(&mut self) -> bool {
        if self.pending.as_ref().is_some_and(|p| !p.loaded()) {
            return false;
        }
        if self.warmup.as_ref().is_some_and(|w| !w.ready()) {
            return false;
        }
        if self.batches.as_ref().is_some_and(|b| !b.ready()) {
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
/// it has not prepared yet (queued behind the per-frame upload budget), and meshes the world is
/// still loading ([`LateMeshes::loading`]). Main-world-only images never upload, so they do not
/// count.
pub(crate) fn pending_uploads(world: &World) -> (usize, usize) {
    let images: Vec<AssetId<Image>> = world
        .resource::<Assets<Image>>()
        .iter()
        .filter(|(_, image)| {
            image
                .asset_usage
                .contains(bevy::asset::RenderAssetUsages::RENDER_WORLD)
        })
        .map(|(id, _)| id)
        .collect();
    let images = {
        let uploaded = world
            .resource::<super::map_material::SweepImages>()
            .0
            .lock()
            .unwrap();
        images.iter().filter(|id| !uploaded.contains(id)).count()
    };
    (images, world.resource::<LateMeshes>().loading())
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
    mut reported_failures: Local<HashSet<usize>>,
) {
    *uploaded.0.lock().unwrap() = images.iter().map(|(id, _)| id).collect();
    let mut next = PipelineStatus {
        pending: cache.waiting_pipelines().count(),
        ..default()
    };
    for (index, pipeline) in cache.pipelines().enumerate() {
        match &pipeline.state {
            CachedPipelineState::Ok(_) => next.ready += 1,
            CachedPipelineState::Err(
                bevy::shader::ShaderCacheError::ProcessShaderError(_)
                | bevy::shader::ShaderCacheError::CreateShaderModule(_),
            ) => {
                if reported_failures.insert(index) {
                    // PipelineCache already logs the shader diagnostic once. Reprinting its
                    // complete embedded shader every frame can produce gigabytes of logs.
                    error!(
                        "capture GPU pipeline failed (cache entry {index}); see shader compilation error"
                    );
                }
                next.failed += 1;
            }
            _ => {}
        }
    }
    if next.failed == 0 {
        reported_failures.clear();
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
    fn gate_waits_for_new_world_meshes_not_rewritten_or_runtime_ones() {
        let id = |i: u128| AssetId::<Mesh>::Uuid {
            uuid: bevy::asset::uuid::Uuid::from_u128(i),
        };
        let (map, effect, removed) = (id(1), id(2), id(3));
        let mut state = LateState::default();
        let mut on_gpu = HashSet::new();
        state.report(map, false);
        state.report(effect, true);
        state.report(removed, false);
        assert_eq!(
            state.loading.len(),
            2,
            "only the world's own meshes hold the gate"
        );
        state.check(|id| on_gpu.contains(&id));
        assert_eq!(state.loading.len(), 2);
        on_gpu.insert(map);
        state.check(|id| on_gpu.contains(&id));
        assert_eq!(state.ready, vec![map], "a late mesh is still reported");
        assert_eq!(state.loading, HashSet::from([removed]));
        state.forget_removed(|id| id != removed);
        assert!(state.loading.is_empty() && !state.waiting.contains(&removed));
        // Bevy marks the entity changed again whenever its mesh or material asset is
        // rewritten, and a rewritten mesh leaves the render world until it is prepared again:
        // every frame for effect batches, cloth and the animated sky. That must not hold the
        // gate, but the entity is still specialized again once its mesh is back.
        on_gpu.remove(&map);
        state.report(map, false);
        state.check(|id| on_gpu.contains(&id));
        assert_eq!(state.loading.len(), 0);
        assert!(state.waiting.contains(&map));
    }

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

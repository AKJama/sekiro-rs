//! Merge static object surfaces across placements, retaining the authored visibility roots.
use super::{LayeredMaterial, MaterialOrigin};
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use bevy::tasks::{
    AsyncComputeTaskPool, Task,
    futures_lite::future::{block_on, poll_once},
};
use std::collections::HashMap;

struct Source {
    entity: Entity,
    mesh: Handle<Mesh>,
    root: Entity,
    root_pose: Mat4,
    pose: Mat4,
}

struct Group {
    sources: Vec<Source>,
    material: Handle<LayeredMaterial>,
    origin: Option<bevy::asset::AssetPath<'static>>,
    no_shadow: bool,
    no_receiver: bool,
    authored_no_shadow: bool,
}

#[derive(Hash, Eq, PartialEq)]
struct GroupKey {
    material: AssetId<LayeredMaterial>,
    visibility: String,
    layout: String,
    no_shadow: bool,
    no_receiver: bool,
    authored_no_shadow: bool,
    region: [i32; 2],
    scale: u32,
}

struct Batch {
    entity: Entity,
    sources: Vec<Source>,
    merged: bool,
}
struct Job {
    group: Group,
    task: Task<Option<Mesh>>,
}

#[derive(Component)]
struct BatchMember(usize);

#[derive(Resource, Default)]
pub(crate) struct StaticBatches {
    started: bool,
    jobs: Vec<Job>,
    batches: Vec<Batch>,
    dirty: Vec<usize>,
    source_to_batch: HashMap<Entity, usize>,
}

impl StaticBatches {
    pub(crate) fn ready(&self) -> bool {
        self.started && self.jobs.is_empty()
    }
}

pub(super) fn install(app: &mut App) {
    if std::env::var_os("SEKIRO_NO_STATIC_BATCHES").is_some() {
        return;
    }
    app.init_resource::<StaticBatches>().add_systems(
        PostUpdate,
        (changed_batches, update)
            .chain()
            .after(bevy::transform::TransformSystems::Propagate)
            .after(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate)
            .before(bevy::camera::visibility::VisibilitySystems::CheckVisibility),
    );
}

#[allow(clippy::type_complexity)] // Independent transform and visibility change filters.
fn changed_batches(
    changed: Query<&BatchMember, Or<(Changed<GlobalTransform>, Changed<InheritedVisibility>)>>,
    mut removed: RemovedComponents<BatchMember>,
    mut state: ResMut<StaticBatches>,
) {
    state.dirty.extend(changed.iter().map(|m| m.0));
    for entity in removed.read() {
        if let Some(index) = state.source_to_batch.remove(&entity) {
            state.dirty.push(index);
        }
    }
    state.dirty.sort_unstable();
    state.dirty.dedup();
}

fn update(world: &mut World) {
    if !world.resource::<crate::map::Pending>().loaded() {
        return;
    }
    let _timer = crate::perf_log::SystemTimer::start("static map batches");
    let mut state = world.remove_resource::<StaticBatches>().unwrap();
    if !state.started {
        state.started = true;
        state.jobs = plan(world);
        info!(
            "static map merge: {} groups, {} source meshes",
            state.jobs.len(),
            state
                .jobs
                .iter()
                .map(|j| j.group.sources.len())
                .sum::<usize>()
        );
    }
    let mut completed = 0;
    let mut i = 0;
    while i < state.jobs.len() && completed < 4 {
        let Some(mesh) = block_on(poll_once(&mut state.jobs[i].task)) else {
            i += 1;
            continue;
        };
        let job = state.jobs.swap_remove(i);
        completed += 1;
        let Some(mesh) = mesh else {
            continue;
        };
        if job
            .group
            .sources
            .iter()
            .any(|s| world.get_entity(s.entity).is_err())
        {
            continue;
        }
        let mesh = world.resource_mut::<Assets<Mesh>>().add(mesh);
        let parent = job.group.sources[0].root;
        let mut entity = world.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(job.group.material),
            Transform::IDENTITY,
            Visibility::Hidden,
            ChildOf(parent),
        ));
        if job.group.no_shadow {
            entity.insert(NotShadowCaster);
        }
        if job.group.authored_no_shadow {
            entity.insert(crate::map::world::AuthoredNoShadow);
        }
        if job.group.no_receiver {
            entity.insert(NotShadowReceiver);
        }
        if let Some(origin) = job.group.origin {
            entity.insert(MaterialOrigin(origin));
        }
        let entity = entity.id();
        let index = state.batches.len();
        for source in &job.group.sources {
            world.entity_mut(source.entity).insert(BatchMember(index));
            state.source_to_batch.insert(source.entity, index);
        }
        state.batches.push(Batch {
            entity,
            sources: job.group.sources,
            merged: false,
        });
        state.dirty.push(index);
    }
    for index in std::mem::take(&mut state.dirty) {
        guard(world, &mut state.batches[index]);
    }
    world.insert_resource(state);
}

fn guard(world: &mut World, batch: &mut Batch) {
    let first = batch
        .sources
        .first()
        .and_then(|s| world.get::<InheritedVisibility>(s.entity))
        .map(|v| v.get());
    let coherent = first.is_some()
        && batch.sources.iter().all(|s| {
            world
                .get::<InheritedVisibility>(s.entity)
                .is_some_and(|v| Some(v.get()) == first)
                && world
                    .get::<GlobalTransform>(s.root)
                    .is_some_and(|p| p.to_matrix() == s.root_pose)
                && world
                    .get::<GlobalTransform>(s.entity)
                    .is_some_and(|p| p.to_matrix() == s.pose)
        });
    if coherent != batch.merged {
        for source in &batch.sources {
            if let Ok(mut entity) = world.get_entity_mut(source.entity) {
                if coherent {
                    entity.remove::<Mesh3d>();
                } else {
                    entity.insert(Mesh3d(source.mesh.clone()));
                }
            }
        }
        batch.merged = coherent;
    }
    if let Some(mut visibility) = world.get_mut::<Visibility>(batch.entity) {
        let desired = if coherent && first == Some(true) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != desired {
            *visibility = desired;
        }
    }
    // This exclusive system runs after hierarchy propagation and before culling. Publish
    // the same-frame gate as well, so activation never removes sources a frame too early.
    if let Some(mut visibility) = world.get_mut::<InheritedVisibility>(batch.entity) {
        let desired = if coherent && first == Some(true) {
            InheritedVisibility::VISIBLE
        } else {
            InheritedVisibility::HIDDEN
        };
        if *visibility != desired {
            *visibility = desired;
        }
    }
}

fn plan(world: &mut World) -> Vec<Job> {
    let entities: Vec<_> = world
        .query_filtered::<Entity, (With<Mesh3d>, With<MeshMaterial3d<LayeredMaterial>>)>()
        .iter(world)
        .collect();
    let mut groups: HashMap<GroupKey, Group> = HashMap::new();
    let mut root_keys = HashMap::new();
    for entity in entities {
        if world.get::<Children>(entity).is_some_and(|c| !c.is_empty())
            || world.get::<bevy::mesh::MeshTag>(entity).is_some()
        {
            continue;
        }
        let mut root = entity;
        while world.get::<crate::map::world::BlockPart>(root).is_none() {
            let Some(parent) = world.get::<ChildOf>(root) else {
                break;
            };
            root = parent.parent();
        }
        // MapPiece levels keep independent distance crossfades. Grass is already merged per surface.
        if world.get::<crate::map::world::BlockPart>(root).is_none()
            || world.get::<crate::map::world::DetailPart>(root).is_some()
        {
            continue;
        }
        let Some(part) = world.get::<crate::map::visibility::VisibilityPart>(root) else {
            continue;
        };
        let key = root_keys
            .entry(root)
            .or_insert_with(|| {
                format!(
                    "{}/{:?}/{:?}/{}/{:?}",
                    part.map,
                    part.effective.draw,
                    part.effective.backread,
                    part.entity_id,
                    part.entity_groups
                )
            })
            .clone();
        let material = world
            .get::<MeshMaterial3d<LayeredMaterial>>(entity)
            .unwrap()
            .0
            .clone();
        if !world
            .resource::<Assets<LayeredMaterial>>()
            .get(&material)
            .is_some_and(|m| {
                matches!(
                    m.base.alpha_mode,
                    AlphaMode::Opaque | AlphaMode::Mask(_) | AlphaMode::AlphaToCoverage
                )
            })
        {
            continue;
        }
        let mesh = world.get::<Mesh3d>(entity).unwrap().0.clone();
        let Some(asset) = world.resource::<Assets<Mesh>>().get(&mesh) else {
            continue;
        };
        if asset.contains_attribute(Mesh::ATTRIBUTE_JOINT_INDEX) || asset.morph_targets().is_some()
        {
            continue;
        }
        let Some(pose) = world.get::<GlobalTransform>(entity).map(|p| p.to_matrix()) else {
            continue;
        };
        let root_pose = world.get::<GlobalTransform>(root).unwrap().to_matrix();
        let scale = pose.x_axis.truncate().length();
        if scale.to_bits() != root_pose.x_axis.truncate().length().to_bits() {
            continue;
        }
        let region = (pose.w_axis.truncate().xz() / 128.)
            .floor()
            .as_ivec2()
            .to_array();
        let layout = format!(
            "{:?}/{:?}",
            asset.primitive_topology(),
            asset
                .attributes()
                .map(|(a, _)| (a.id, a.format))
                .collect::<Vec<_>>()
        );
        let no_shadow = world.get::<NotShadowCaster>(entity).is_some();
        let no_receiver = world.get::<NotShadowReceiver>(entity).is_some();
        let authored_no_shadow = world
            .get::<crate::map::world::AuthoredNoShadow>(entity)
            .is_some();
        let origin = world.get::<MaterialOrigin>(entity).map(|o| o.0.clone());
        let group = groups
            .entry(GroupKey {
                material: material.id(),
                visibility: key,
                layout,
                no_shadow,
                no_receiver,
                authored_no_shadow,
                region,
                scale: scale.to_bits(),
            })
            .or_insert_with(|| Group {
                sources: Vec::new(),
                material,
                origin,
                no_shadow,
                no_receiver,
                authored_no_shadow,
            });
        group.sources.push(Source {
            entity,
            mesh,
            root,
            root_pose,
            pose,
        });
    }
    let mut jobs = Vec::new();
    for group in groups.into_values().filter(|g| g.sources.len() > 1) {
        let inverse = group.sources[0].root_pose.inverse();
        let mut inputs = Vec::new();
        for source in &group.sources {
            let relative = inverse * source.pose;
            let transform = Transform::from_matrix(relative);
            if !transform.to_matrix().abs_diff_eq(relative, 0.00001) {
                inputs.clear();
                break;
            }
            let Some(mesh) = world.resource::<Assets<Mesh>>().get(&source.mesh) else {
                inputs.clear();
                break;
            };
            inputs.push((mesh.clone(), transform));
        }
        if inputs.is_empty() {
            continue;
        }
        let task = AsyncComputeTaskPool::get().spawn(async move {
            let _timer = crate::perf_log::SystemTimer::start("static map merge worker");
            let mut combined: Option<Mesh> = None;
            for (mut mesh, transform) in inputs {
                let indices = mesh.indices().map_or_else(
                    || (0..mesh.count_vertices() as u32).collect(),
                    |i| i.iter().map(|i| i as u32).collect(),
                );
                mesh.insert_indices(bevy::mesh::Indices::U32(indices));
                mesh.transform_by(transform);
                if let Some(combined) = &mut combined {
                    if combined.merge(&mesh).is_err() {
                        return None;
                    }
                } else {
                    combined = Some(mesh);
                }
            }
            combined
        });
        jobs.push(Job { group, task });
    }
    jobs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merged_parts_fall_back_when_an_event_hides_one_or_a_placement_moves() {
        let mut world = World::new();
        let root = world.spawn(GlobalTransform::IDENTITY).id();
        let sources: Vec<_> = (0..2)
            .map(|_| {
                let entity = world
                    .spawn((
                        GlobalTransform::IDENTITY,
                        InheritedVisibility::VISIBLE,
                        Mesh3d(Handle::default()),
                    ))
                    .id();
                Source {
                    entity,
                    root,
                    mesh: Handle::default(),
                    root_pose: Mat4::IDENTITY,
                    pose: Mat4::IDENTITY,
                }
            })
            .collect();
        let entity = world.spawn(Visibility::Hidden).id();
        let mut batch = Batch {
            entity,
            sources,
            merged: false,
        };
        guard(&mut world, &mut batch);
        assert!(batch.merged);
        assert!(
            batch
                .sources
                .iter()
                .all(|s| world.get::<Mesh3d>(s.entity).is_none())
        );
        world
            .entity_mut(batch.sources[1].entity)
            .insert(InheritedVisibility::HIDDEN);
        guard(&mut world, &mut batch);
        assert!(!batch.merged);
        assert_eq!(
            *world.get::<Visibility>(entity).unwrap(),
            Visibility::Hidden
        );
        assert!(
            batch
                .sources
                .iter()
                .all(|s| world.get::<Mesh3d>(s.entity).is_some())
        );
        world
            .entity_mut(batch.sources[1].entity)
            .insert(InheritedVisibility::VISIBLE);
        guard(&mut world, &mut batch);
        assert!(batch.merged);
        world
            .entity_mut(root)
            .insert(GlobalTransform::from_translation(Vec3::X));
        guard(&mut world, &mut batch);
        assert!(!batch.merged);
    }
}

//! Record independent sun cascades with separate encoders, including parallel finalization.
//! The draw-function lock still protects Bevy's cached render commands.
use bevy::core_pipeline::schedule::Core3d;
use bevy::ecs::schedule::ScheduleCleanupPolicy;
use bevy::ecs::system::{Deferred, SystemBuffer, SystemMeta};
use bevy::ecs::world::DeferredWorld;
use bevy::pbr::{Shadow, ShadowView, ViewLightEntities, per_view_shadow_pass};
use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::render_phase::{TrackedRenderPass, ViewBinnedRenderPhases};
use bevy::render::render_resource::{CommandEncoderDescriptor, RenderPassDescriptor, StoreOp};
use bevy::render::renderer::{PendingCommandBuffers, RenderDevice, ViewQuery};
use bevy::render::view::ExtractedView;
use bevy::tasks::{ComputeTaskPool, Task, futures_lite::future::block_on};

#[derive(Default)]
struct ShadowCommands(Vec<Task<wgpu::CommandBuffer>>);

impl SystemBuffer for ShadowCommands {
    fn queue(&mut self, _meta: &SystemMeta, mut world: DeferredWorld) {
        // Deferred buffers flush in schedule order: indirect-parameter writes, shadows,
        // then main passes. Finish encoders in the meantime, without delaying those passes.
        let _timer = crate::perf_log::SystemTimer::start("shadow finish wait");
        let buffers: Vec<_> = self.0.drain(..).map(block_on).collect();
        world.resource_mut::<PendingCommandBuffers>().push(buffers);
    }
}

pub(crate) struct ParallelShadowsPlugin;

impl Plugin for ParallelShadowsPlugin {
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        if std::env::var_os("SEKIRO_SERIAL_SHADOWS").is_some() {
            return;
        }
        let Some(render) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        let world = render.world_mut();
        let mut schedule = world
            .resource_mut::<Schedules>()
            .remove(Core3d)
            .expect("3D schedule");
        let removed = schedule
            .remove_systems_in_set(
                per_view_shadow_pass::<false>,
                world,
                ScheduleCleanupPolicy::RemoveSystemsOnly,
            )
            .expect("replace sun shadow pass");
        assert_eq!(removed, 1, "sun shadow pass must be replaced exactly once");
        // Preserve all existing prepass, occlusion and main-pass ordering edges.
        schedule.add_systems(parallel_shadow_pass
            .after(bevy::pbr::early_prepass_build_indirect_parameters)
            .before(bevy::core_pipeline::mip_generation::experimental::depth::early_downsample_depth)
            .before(per_view_shadow_pass::<true>)
            .before(bevy::core_pipeline::schedule::Core3dSystems::MainPass));
        world.resource_mut::<Schedules>().insert(schedule);
        info!("parallel sun shadow encoders installed");
    }
}

fn parallel_shadow_pass(
    world: &World,
    view: ViewQuery<&ViewLightEntities>,
    views: Query<(&ShadowView, &ExtractedView)>,
    phases: Res<ViewBinnedRenderPhases<Shadow>>,
    device: Res<RenderDevice>,
    mut commands: Deferred<ShadowCommands>,
) {
    let _timer = crate::perf_log::SystemTimer::start("shadow tasks");
    let view = view.into_inner();
    for &entity in &view.lights {
        let Ok((shadow, extracted)) = views.get(entity) else {
            continue;
        };
        let Some(phase) = phases.get(&extracted.retained_view_entity) else {
            continue;
        };
        let device = &*device;
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some(&shadow.pass_name),
        });
        {
            let _timer = crate::perf_log::SystemTimer::start("shadow record");
            let pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some(&shadow.pass_name),
                color_attachments: &[],
                depth_stencil_attachment: Some(
                    shadow.depth_attachment.get_attachment(StoreOp::Store),
                ),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let mut pass = TrackedRenderPass::new(device, pass);
            if let Err(error) = phase.render(&mut pass, world, entity) {
                error!("shadow phase: {error:?}");
            }
        }
        commands.0.push(ComputeTaskPool::get().spawn(async move {
            let _timer = crate::perf_log::SystemTimer::start("shadow finish");
            encoder.finish()
        }));
    }
}

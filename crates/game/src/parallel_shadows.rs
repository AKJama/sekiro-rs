//! Record each sun cascade through Bevy, preserving its native indirect preparation.
//! Separate encoders let Bevy 0.20 finish the cascades in parallel before submission.
use bevy::core_pipeline::schedule::Core3d;
use bevy::ecs::schedule::{ScheduleCleanupPolicy, ScheduleConfigs};
use bevy::ecs::system::ScheduleSystem;
use bevy::pbr::{ViewLightEntities, per_view_shadow_pass};
use bevy::prelude::*;
use bevy::render::{RenderApp, renderer::CurrentView};

pub(crate) struct ParallelShadowsPlugin;

impl Plugin for ParallelShadowsPlugin {
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
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
        schedule.add_systems(ordered_cascades(parallel_shadow_pass.into_configs())
            .after(bevy::pbr::early_prepass_build_indirect_parameters)
            .before(bevy::core_pipeline::mip_generation::experimental::depth::early_downsample_depth)
            .before(per_view_shadow_pass::<true>)
            .before(bevy::core_pipeline::schedule::Core3dSystems::MainPass));
        world.resource_mut::<Schedules>().insert(schedule);
    }
}

fn ordered_cascades(pass: ScheduleConfigs<ScheduleSystem>) -> ScheduleConfigs<ScheduleSystem> {
    // Core3d disables automatic deferred barriers. The cached native shadow system
    // queues immediately, so preceding GPU preprocessing must be queued first.
    (ApplyDeferred, pass).chain()
}

fn parallel_shadow_pass(world: &mut World, mut lights: Local<Vec<Entity>>) {
    let _timer = crate::perf_log::SystemTimer::start("shadow tasks");
    let view = world.resource::<CurrentView>().0;
    let Some(mut view_lights) = world.get_mut::<ViewLightEntities>(view) else {
        return;
    };
    std::mem::swap(&mut *lights, &mut view_lights.lights);
    // Each invocation flushes its own encoder into Bevy's ordered pending list.
    // Calling the native system also keeps its per-shadow-view indirect writes.
    let result = lights.iter().try_for_each(|&light| {
        world
            .get_mut::<ViewLightEntities>(view)
            .unwrap()
            .lights
            .push(light);
        let result = world.run_system_cached(per_view_shadow_pass::<false>);
        world
            .get_mut::<ViewLightEntities>(view)
            .unwrap()
            .lights
            .clear();
        result
    });
    std::mem::swap(
        &mut *lights,
        &mut world.get_mut::<ViewLightEntities>(view).unwrap().lights,
    );
    result.expect("native sun shadow pass");
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::{
        system::{Deferred, SystemBuffer, SystemMeta},
        world::DeferredWorld,
    };

    #[derive(Resource, Default)]
    struct Submitted(Vec<&'static str>);

    #[derive(Default)]
    struct Encoder(Vec<&'static str>);

    impl SystemBuffer for Encoder {
        fn queue(&mut self, _: &SystemMeta, mut world: DeferredWorld) {
            world.resource_mut::<Submitted>().0.append(&mut self.0);
        }
    }

    #[test]
    fn cascades_submit_after_preprocessing_before_color_on_every_frame() {
        fn preprocess(mut encoder: Deferred<Encoder>) {
            encoder.0.push("preprocess");
        }
        fn cascade(mut encoder: Deferred<Encoder>) {
            encoder.0.push("cascade");
        }
        fn split(world: &mut World) {
            for _ in 0..4 {
                world.run_system_cached(cascade).unwrap();
            }
        }
        fn color(mut encoder: Deferred<Encoder>) {
            encoder.0.push("color");
        }
        let mut world = World::new();
        world.init_resource::<Submitted>();
        let mut schedule = Core3d::base_schedule();
        schedule.add_systems((preprocess, ordered_cascades(split.into_configs()), color).chain());
        for _ in 0..3 {
            schedule.run(&mut world);
            assert_eq!(
                std::mem::take(&mut world.resource_mut::<Submitted>().0),
                [
                    "preprocess",
                    "cascade",
                    "cascade",
                    "cascade",
                    "cascade",
                    "color"
                ]
            );
        }
    }
}

//! Finish independent game-pass encoders on workers, retaining Bevy's submission order.
use bevy::ecs::system::{Deferred, SystemBuffer, SystemMeta, SystemParam};
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use bevy::render::diagnostic::DiagnosticsRecorder;
use bevy::render::render_phase::TrackedRenderPass;
use bevy::render::render_resource::{
    CommandEncoder, CommandEncoderDescriptor, RenderPassDescriptor,
};
use bevy::render::renderer::{PendingCommandBuffers, RenderDevice};
use bevy::tasks::{ComputeTaskPool, Task, futures_lite::future::block_on};

#[derive(Default)]
pub(crate) struct EncoderState {
    encoder: Option<CommandEncoder>,
    task: Option<Task<wgpu::CommandBuffer>>,
}

impl SystemBuffer for EncoderState {
    fn queue(&mut self, _meta: &SystemMeta, mut world: DeferredWorld) {
        let _timer = crate::perf_log::SystemTimer::start("game encoder wait");
        let buffer = self
            .task
            .take()
            .map(block_on)
            .or_else(|| self.encoder.take().map(|e| e.finish()));
        if let Some(buffer) = buffer {
            world.resource_mut::<PendingCommandBuffers>().push([buffer]);
        }
    }
}

#[derive(SystemParam)]
pub(crate) struct RenderContext<'w, 's> {
    state: Deferred<'s, EncoderState>,
    device: Res<'w, RenderDevice>,
    diagnostics: Option<Res<'w, DiagnosticsRecorder>>,
}

impl<'w, 's> RenderContext<'w, 's> {
    pub(crate) fn render_device(&self) -> &RenderDevice {
        &self.device
    }

    pub(crate) fn diagnostic_recorder(&self) -> Option<Res<'w, DiagnosticsRecorder>> {
        self.diagnostics.as_ref().map(Res::clone)
    }

    pub(crate) fn command_encoder(&mut self) -> &mut CommandEncoder {
        self.state.encoder.get_or_insert_with(|| {
            self.device
                .create_command_encoder(&CommandEncoderDescriptor::default())
        })
    }

    pub(crate) fn begin_tracked_render_pass<'a>(
        &'a mut self,
        descriptor: RenderPassDescriptor<'_>,
    ) -> TrackedRenderPass<'a> {
        let encoder = self.state.encoder.get_or_insert_with(|| {
            self.device
                .create_command_encoder(&CommandEncoderDescriptor::default())
        });
        TrackedRenderPass::new(&self.device, encoder.begin_render_pass(&descriptor))
    }
}

impl Drop for RenderContext<'_, '_> {
    fn drop(&mut self) {
        static SERIAL: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if *SERIAL.get_or_init(|| std::env::var_os("SEKIRO_SERIAL_GAME_PASSES").is_some()) {
            return;
        }
        if let Some(encoder) = self.state.encoder.take() {
            self.state.task = Some(ComputeTaskPool::get().spawn(async move {
                let _timer = crate::perf_log::SystemTimer::start("game encoder finish");
                encoder.finish()
            }));
        }
    }
}

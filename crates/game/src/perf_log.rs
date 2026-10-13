//! Opt-in (`PLAY_LOG_PERF=1`), buffered per-render-frame measurements in `cache/perf/<ms>.csv`
//! (override the directory with `PLAY_PERF_DIR`):
//! wall time, fixed steps, drawn and simulated clip times and positions, hit stops, hits and
//! sounds, plus where a slow frame went: `x_main_ms` (the main schedule, `First` to `Last`),
//! `x_fixed_ms` (the fixed steps), `x_clip_loads`/`x_clip_ms` (clips read from disk on the frame
//! they were first shown), `x_render_max_ms` and `x_render_sets` (the slowest render-world frame
//! since the last row and its time per render set: start to PrepareAssets, to Specialize, to
//! Queue (this one includes waiting for the swapchain image, about 13 ms at 60 Hz with vsync), to
//! PhaseSort, to Prepare, to Render, to Cleanup, to the end) and the pipeline cache's ready and
//! waiting counts. `tools/perf/summary.py` summarises the files.
use bevy::prelude::*;
use std::io::{BufWriter, Write};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

type SystemSamples = std::collections::BTreeMap<&'static str, (usize, f64, f64)>;
static SYSTEM_SAMPLES: std::sync::Mutex<SystemSamples> =
    std::sync::Mutex::new(std::collections::BTreeMap::new());

pub struct SystemTimer(Option<(&'static str, Instant)>);

impl SystemTimer {
    pub fn start(name: &'static str) -> Self {
        static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        Self(
            ENABLED
                .get_or_init(|| std::env::args().any(|a| a == "--profile-systems"))
                .then(|| (name, Instant::now())),
        )
    }
}

impl Drop for SystemTimer {
    fn drop(&mut self) {
        if let Some((name, start)) = self.0
            && let Ok(mut samples) = SYSTEM_SAMPLES.lock()
        {
            let ms = start.elapsed().as_secs_f64() * 1000.;
            let sample = samples.entry(name).or_default();
            sample.0 += 1;
            sample.1 += ms;
            sample.2 = sample.2.max(ms);
        }
    }
}

pub fn report_systems() {
    if let Ok(mut samples) = SYSTEM_SAMPLES.lock() {
        for (name, (count, sum, max)) in samples.iter() {
            info!(
                "profile system {name}: mean {:.3} ms, max {max:.3} ms",
                sum / *count as f64
            );
        }
        samples.clear();
    }
}

#[derive(Resource)]
pub struct PerfLog {
    output: BufWriter<std::fs::File>,
    last: Instant,
    frame: usize,
    pub previous_step: usize,
    pub steps: usize,
    pub hits: usize,
    pub clip_starts: [usize; 2],
    pub stop_max: [f32; 2],
    sounds: usize,
    /// When this frame's main schedule began (`First`).
    pub frame_start: Instant,
    /// When the running fixed step began, and the fixed steps' total time this frame.
    pub fixed_start: Instant,
    pub fixed_ms: f64,
}

impl PerfLog {
    pub fn open() -> std::io::Result<Self> {
        let dir = std::env::var_os("PLAY_PERF_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| "cache/perf".into());
        std::fs::create_dir_all(&dir)?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let path = dir.join(format!("{stamp}.csv"));
        let mut output = BufWriter::new(
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?,
        );
        writeln!(
            output,
            "frame,wall_ms,steps,alpha,sim_step,replay_frame,wolf_state,wolf_clip,wolf_time,wolf_sim_time,draw_x,draw_y,draw_z,sim_x,sim_y,sim_z,owl_state,owl_clip,owl_time,wolf_stop,owl_stop,x_main_ms,x_fixed_ms,x_clip_loads,x_clip_ms,x_render_max_ms,x_render_frames,x_pipelines_ok,x_pipelines_waiting,x_render_sets,hits,sounds,wolf_starts,owl_starts,wolf_stop_max,owl_stop_max"
        )?;
        info!("perf log: {}", path.display());
        Ok(Self {
            output,
            last: Instant::now(),
            frame: 0,
            previous_step: 0,
            steps: 0,
            hits: 0,
            clip_starts: [0; 2],
            stop_max: [0.0; 2],
            sounds: 0,
            frame_start: Instant::now(),
            fixed_start: Instant::now(),
            fixed_ms: 0.0,
        })
    }

    pub fn row(&mut self, fields: &str, sounds: usize) {
        let now = Instant::now();
        let ms = now.duration_since(self.last).as_secs_f64() * 1000.0;
        self.last = now;
        let started = sounds.saturating_sub(self.sounds);
        self.sounds = sounds;
        if let Err(error) = writeln!(
            self.output,
            "{},{ms:.6},{},{fields},{},{started},{},{},{},{}",
            self.frame,
            self.steps,
            self.hits,
            self.clip_starts[0],
            self.clip_starts[1],
            self.stop_max[0],
            self.stop_max[1]
        ) {
            error!("perf log write: {error}");
        }
        self.frame += 1;
        self.steps = 0;
        self.hits = 0;
        self.clip_starts = [0; 2];
        self.stop_max = [0.0; 2];
        self.fixed_ms = 0.0;
    }

    /// Milliseconds since `start`.
    pub fn since(start: Instant) -> f64 {
        start.elapsed().as_secs_f64() * 1000.0
    }
}

/// Render-world timings shared with the main world's log: the render schedule's CPU time per
/// render frame, and the pipeline cache's counts (a pipeline that finishes compiling shows as a
/// step in `pipelines_ok`).
#[derive(Default)]
pub struct RenderProbeData {
    start: Option<Instant>,
    /// Stamps at the start of each probed render set, in order.
    stamps: Vec<Instant>,
    /// Per probed set, its time in the slowest render frame since the last read.
    pub slowest_sets: Vec<f64>,
    /// Longest and total render schedule time since the main log last read it.
    pub max_ms: f64,
    pub sum_ms: f64,
    pub frames: usize,
    pub pipelines_ok: usize,
    pub pipelines_waiting: usize,
    /// Queued draw batches in the main opaque, alpha-mask and transparent passes.
    pub draw_batches: usize,
}

#[derive(Resource, Clone, Default)]
pub struct RenderProbe(pub std::sync::Arc<std::sync::Mutex<RenderProbeData>>);

impl RenderProbe {
    /// (max ms, total ms, frames, pipelines ready, pipelines waiting, per-set ms of the slowest
    /// frame joined by `/`) since the last call.
    pub fn take(&self) -> (f64, f64, usize, usize, usize, String) {
        let Ok(mut d) = self.0.lock() else {
            return (0.0, 0.0, 0, 0, 0, String::new());
        };
        let sets = d
            .slowest_sets
            .iter()
            .map(|ms| format!("{ms:.1}"))
            .collect::<Vec<_>>()
            .join("/");
        let out = (
            d.max_ms,
            d.sum_ms,
            d.frames,
            d.pipelines_ok,
            d.pipelines_waiting,
            sets,
        );
        d.slowest_sets.clear();
        d.max_ms = 0.0;
        d.sum_ms = 0.0;
        d.frames = 0;
        out
    }
}

/// Adds the render-world half of [`RenderProbe`].
pub fn add_render_probe(app: &mut App, probe: RenderProbe) {
    use bevy::render::render_resource::{CachedPipelineState, PipelineCache};
    use bevy::render::{Render, RenderApp, RenderSystems};
    let Some(render) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    render.insert_resource(probe);
    if std::env::var_os("PF_RESPAWN_PROBE").is_some() {
        render.add_systems(Render, count_draw_batches.in_set(RenderSystems::Render));
    }
    render.add_systems(
        Render,
        (|p: Res<RenderProbe>| {
            if let Ok(mut d) = p.0.lock() {
                d.start = Some(Instant::now());
                d.stamps.clear();
            }
        })
        .in_set(RenderSystems::ExtractCommands),
    );
    let stamp = |p: Res<RenderProbe>| {
        if let Ok(mut d) = p.0.lock() {
            d.stamps.push(Instant::now());
        }
    };
    for set in [
        RenderSystems::PrepareAssets,
        RenderSystems::Specialize,
        RenderSystems::Queue,
        RenderSystems::PhaseSort,
        RenderSystems::Prepare,
        RenderSystems::Render,
        RenderSystems::Cleanup,
    ] {
        render.add_systems(Render, stamp.in_set(set));
    }
    render.add_systems(
        Render,
        (|p: Res<RenderProbe>, cache: Res<PipelineCache>| {
            let ok = cache
                .pipelines()
                .filter(|c| matches!(c.state, CachedPipelineState::Ok(_)))
                .count();
            let waiting = cache.waiting_pipelines().count();
            if let Ok(mut d) = p.0.lock() {
                if let Some(start) = d.start.take() {
                    let ms = PerfLog::since(start);
                    if ms > d.max_ms {
                        let mut prev = start;
                        let mut sets = Vec::new();
                        for &t in d.stamps.iter().chain(std::iter::once(&Instant::now())) {
                            sets.push(t.duration_since(prev).as_secs_f64() * 1000.0);
                            prev = t;
                        }
                        d.slowest_sets = sets;
                    }
                    d.max_ms = d.max_ms.max(ms);
                    d.sum_ms += ms;
                    d.frames += 1;
                }
                d.pipelines_ok = ok;
                d.pipelines_waiting = waiting;
            }
        })
        .in_set(RenderSystems::PostCleanup),
    );
}

fn count_draw_batches(
    probe: Res<RenderProbe>,
    opaque: Res<
        bevy::render::render_phase::ViewBinnedRenderPhases<bevy::core_pipeline::core_3d::Opaque3d>,
    >,
    masked: Res<
        bevy::render::render_phase::ViewBinnedRenderPhases<
            bevy::core_pipeline::core_3d::AlphaMask3d,
        >,
    >,
    transparent: Res<
        bevy::render::render_phase::ViewSortedRenderPhases<
            bevy::core_pipeline::core_3d::Transparent3d,
        >,
    >,
) {
    fn binned<P: bevy::render::render_phase::BinnedPhaseItem>(
        phases: &bevy::render::render_phase::ViewBinnedRenderPhases<P>,
    ) -> usize {
        phases
            .0
            .values()
            .map(|p| {
                p.multidrawable_meshes.len()
                    + p.batchable_meshes.len()
                    + p.unbatchable_meshes
                        .values()
                        .map(|b| b.entities.len())
                        .sum::<usize>()
                    + p.non_mesh_items
                        .values()
                        .map(|b| b.entities.len())
                        .sum::<usize>()
            })
            .sum()
    }
    if let Ok(mut data) = probe.0.lock() {
        data.draw_batches = binned(&opaque)
            + binned(&masked)
            + transparent
                .0
                .values()
                .map(|p| {
                    p.items
                        .iter()
                        .filter(|(_, i)| !i.batch_range.is_empty())
                        .count()
                })
                .sum::<usize>();
    }
}

pub fn enabled() -> bool {
    std::env::var("PLAY_LOG_PERF").is_ok_and(|v| v == "1")
}

macro_rules! register {
    ($app:ident) => {
        if crate::perf_log::enabled() {
            let probe = crate::perf_log::RenderProbe::default();
            crate::perf_log::add_render_probe(&mut $app, probe.clone());
            $app.insert_resource(crate::perf_log::PerfLog::open()?)
                .insert_resource(probe)
                .add_systems(First, |mut log: ResMut<crate::perf_log::PerfLog>| {
                    log.frame_start = std::time::Instant::now();
                })
                .add_systems(FixedFirst, |mut log: ResMut<crate::perf_log::PerfLog>| {
                    log.fixed_start = std::time::Instant::now();
                })
                .add_systems(FixedLast, |mut log: ResMut<crate::perf_log::PerfLog>| {
                    let ms = crate::perf_log::PerfLog::since(log.fixed_start);
                    log.fixed_ms += ms;
                })
                .add_systems(
                    FixedUpdate,
                    (|sim: NonSend<Sim>, mut log: ResMut<crate::perf_log::PerfLog>| {
                        if sim.step == log.previous_step {
                            return;
                        }
                        log.steps += sim.step - log.previous_step;
                        log.previous_step = sim.step;
                        log.hits += sim.last.hits.len();
                        for (i, report) in
                            [&sim.last.player, &sim.last.enemy].into_iter().enumerate()
                        {
                            log.clip_starts[i] += usize::from(report.clip_started);
                            log.stop_max[i] = log.stop_max[i].max(sim.duel.hit_stop[i].remaining());
                        }
                    })
                    .after(simulate),
                )
                .add_systems(
                    Last,
                    |sim: NonSend<Sim>,
                     fixed: Res<Time<Fixed>>,
                     actors: Query<(&Actor, &Animator, &Transform)>,
                     sounds: Res<crate::audio::SoundLibrary>,
                     replay: Option<Res<crate::input_replay::InputReplay>>,
                     mut libraries: ResMut<Libraries>,
                     probe: Res<crate::perf_log::RenderProbe>,
                     mut log: ResMut<crate::perf_log::PerfLog>| {
                        let (render_max, _, render_frames, pipelines_ok, pipelines_waiting, sets) =
                            probe.take();
                        let (mut clip_loads, mut clip_ms) = (0, 0.0);
                        for library in &mut libraries.0 {
                            let (n, ms) = library.take_load_stats();
                            clip_loads += n;
                            clip_ms += ms;
                        }
                        let mut names = ["-"; 2];
                        let mut times = [0.0; 2];
                        let mut drawn = Vec3::ZERO;
                        for (actor, anim, tf) in &actors {
                            names[actor.0] = anim.clip_name();
                            times[actor.0] = anim.time;
                            if actor.0 == WOLF {
                                drawn = tf.translation;
                            }
                        }
                        let p = sim.duel.player.body.position;
                        let fields = format!(
                            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.3},{:.3},{},{:.3},{:.3},{},{},{},{}",
                            fixed.overstep_fraction(),
                            sim.step,
                            replay.as_ref().map_or(0, |r| r.frame),
                            sim.last.player.behavior.state_path.join("/"),
                            names[0],
                            times[0],
                            sim.last.player.anim_time,
                            drawn.x,
                            drawn.y,
                            drawn.z,
                            p[0],
                            p[1],
                            p[2],
                            sim.last.enemy.behavior.state_path.join("/"),
                            names[1],
                            times[1],
                            sim.duel.hit_stop[0].remaining(),
                            sim.duel.hit_stop[1].remaining(),
                            crate::perf_log::PerfLog::since(log.frame_start),
                            log.fixed_ms,
                            clip_loads,
                            clip_ms,
                            render_max,
                            render_frames,
                            pipelines_ok,
                            pipelines_waiting,
                            sets
                        );
                        log.row(&fields, sounds.played.len());
                    },
                );
        }
    };
}
pub(crate) use register;

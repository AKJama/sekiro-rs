//! Opt-in, buffered per-render-frame combat measurements.
use bevy::prelude::*;
use std::io::{BufWriter, Write};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

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
}

impl PerfLog {
    pub fn open() -> std::io::Result<Self> {
        std::fs::create_dir_all("cache/perf")?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let path = format!("cache/perf/{stamp}.csv");
        let mut output = BufWriter::new(
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?,
        );
        writeln!(
            output,
            "frame,wall_ms,steps,alpha,sim_step,replay_frame,wolf_state,wolf_clip,wolf_time,wolf_sim_time,draw_x,draw_y,draw_z,sim_x,sim_y,sim_z,owl_state,owl_clip,owl_time,wolf_stop,owl_stop,hits,sounds,wolf_starts,owl_starts,wolf_stop_max,owl_stop_max"
        )?;
        info!("perf log: {path}");
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
    }
}

pub fn enabled() -> bool {
    std::env::var("PLAY_LOG_PERF").is_ok_and(|v| v == "1")
}

macro_rules! register {
    ($app:ident) => {
        if crate::perf_log::enabled() {
            $app.insert_resource(crate::perf_log::PerfLog::open()?)
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
                            log.stop_max[i] = log.stop_max[i].max(sim.duel.hit_stop[i].remaining);
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
                     mut log: ResMut<crate::perf_log::PerfLog>| {
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
                            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
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
                            sim.duel.hit_stop[0].remaining,
                            sim.duel.hit_stop[1].remaining
                        );
                        log.row(&fields, sounds.played.len());
                    },
                );
        }
    };
}
pub(crate) use register;

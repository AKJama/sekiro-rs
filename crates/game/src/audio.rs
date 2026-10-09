//! Plays the game's own sounds: decoded banks in `cache/sound/<bank>/` (from
//! `sekiro-extract sound`) looked up by sound event name.
//!
//! The simulation names the sounds ([`sekiro_sim::sound`]: TAE sound events and hit, block and
//! deflect sounds); [`emit`] turns one duel step into [`SoundCue`] messages and [`play_cues`]
//! plays each as a positional one-shot at the character, choosing a variation by the bank's
//! weights. A cue whose event is not in the decoded banks is logged once and skipped.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::audio::{AudioPlayer, AudioSource, PlaybackSettings, SpatialListener, Volume};
use bevy::prelude::*;
use sekiro_sim::duel::{Duel, DuelReport};
use sekiro_sim::sound::{HitSounds, tae_cues};

/// One sound to play at a position.
#[derive(Message, Debug, Clone)]
pub struct SoundCue {
    pub name: String,
    pub position: Vec3,
}

/// Event name to decoded files, loaded lazily into audio assets.
#[derive(Resource, Default)]
pub struct SoundLibrary {
    events: HashMap<String, Vec<(PathBuf, u32)>>,
    loaded: HashMap<PathBuf, Handle<AudioSource>>,
    missing: HashSet<String>,
    pub hit_sounds: Option<HitSounds>,
    rng: u64,
    /// Every cue played, in order (for logs and tests).
    pub played: Vec<String>,
}

impl SoundLibrary {
    /// Reads every `cache/sound/*/events.json`.
    pub fn load(cache: &Path) -> Self {
        let mut lib = SoundLibrary {
            hit_sounds: HitSounds::load_from_cache(cache),
            rng: 0x9e37_79b9_7f4a_7c15,
            ..default()
        };
        let Ok(dirs) = std::fs::read_dir(cache.join("sound")) else {
            warn!("no cache/sound: run `sekiro-extract sound` for audio");
            return lib;
        };
        for dir in dirs.flatten().map(|d| d.path()) {
            let Ok(text) = std::fs::read_to_string(dir.join("events.json")) else {
                continue;
            };
            let Ok(map) = serde_json::from_str::<HashMap<String, Vec<serde_json::Value>>>(&text)
            else {
                continue;
            };
            for (event, variations) in map {
                let list = lib.events.entry(event).or_default();
                for v in variations {
                    if let Some(file) = v.get("file").and_then(|f| f.as_str()) {
                        let weight = v.get("weight").and_then(|w| w.as_u64()).unwrap_or(100);
                        list.push((dir.join(file), weight.max(1) as u32));
                    }
                }
            }
        }
        info!("sound: {} events from cache/sound", lib.events.len());
        lib
    }

    fn pick(&mut self, event: &str) -> Option<PathBuf> {
        let list = self.events.get(event)?;
        let total: u32 = list.iter().map(|(_, w)| w).sum();
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        let mut r = (self.rng % total.max(1) as u64) as u32;
        for (path, w) in list {
            if r < *w {
                return Some(path.clone());
            }
            r -= w;
        }
        list.first().map(|(p, _)| p.clone())
    }
}

pub struct GameAudioPlugin {
    pub cache: PathBuf,
}

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SoundLibrary::load(&self.cache))
            .add_message::<SoundCue>()
            .add_systems(Update, (attach_listener, play_cues));
    }
}

/// Puts the spatial listener on the 3D camera.
fn attach_listener(
    mut commands: Commands,
    cameras: Query<Entity, (With<Camera3d>, Without<SpatialListener>)>,
) {
    for e in &cameras {
        commands.entity(e).insert(SpatialListener::new(0.3));
    }
}

/// Plays queued cues as positional one-shots.
pub fn play_cues(
    mut commands: Commands,
    mut cues: MessageReader<SoundCue>,
    mut lib: ResMut<SoundLibrary>,
    mut sources: ResMut<Assets<AudioSource>>,
) {
    for cue in cues.read() {
        let Some(path) = lib.pick(&cue.name) else {
            if lib.missing.insert(cue.name.clone()) {
                info!("sound {}: not in the decoded banks", cue.name);
            }
            continue;
        };
        let handle = match lib.loaded.get(&path) {
            Some(h) => h.clone(),
            None => match std::fs::read(&path) {
                Ok(bytes) => {
                    let h = sources.add(AudioSource {
                        bytes: Arc::from(bytes),
                    });
                    lib.loaded.insert(path.clone(), h.clone());
                    h
                }
                Err(e) => {
                    warn!("sound {}: {e}", path.display());
                    continue;
                }
            },
        };
        if !lib.played.contains(&cue.name) {
            info!("sound {}: playing {}", cue.name, path.display());
        }
        lib.played.push(cue.name.clone());
        commands.spawn((
            AudioPlayer(handle),
            PlaybackSettings::DESPAWN
                .with_spatial(true)
                .with_volume(Volume::Linear(0.8)),
            Transform::from_translation(cue.position + Vec3::Y),
        ));
    }
}

/// The cues of one duel step: TAE sound events of both characters and the sounds of the hits.
pub fn emit(
    duel: &Duel,
    report: &DuelReport,
    lib: &SoundLibrary,
    out: &mut MessageWriter<SoundCue>,
) {
    let mut cues = tae_cues(0, &duel.player);
    cues.extend(tae_cues(1, &duel.enemy));
    if let Some(hs) = &lib.hit_sounds {
        for (hit, res) in &report.hits {
            cues.extend(hs.cue(hit, res));
        }
    }
    for c in cues {
        out.write(SoundCue {
            name: c.name,
            position: Vec3::from_array(c.position),
        });
    }
}

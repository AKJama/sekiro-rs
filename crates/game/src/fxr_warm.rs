//! Effect warm-up (#313): the effects a fight's timelines can spawn are parsed, compiled and their
//! textures decoded on a background thread, so an effect's first spawn (a parse costs 5-6 ms, a texture decode
//! 6-43 ms) reads no files in the frame.
//! An effect spawned before its warm-up arrives is parsed in the frame as before.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use bevy::prelude::Image;
use serde_json::Value;

use crate::fxr_tree::{FxrLib, NodeDef};

pub(super) enum Warmed {
    /// An effect's tree and compiled root (see [`FxrLib::adopt`]).
    Effect(i64, Option<Arc<Value>>, Option<Arc<NodeDef>>),
    /// A decoded effect texture (one that is not installed or does not decode is left out).
    Texture(i64, Box<Image>),
}

/// Warms `ids` from a copy of `index` on a new thread. The thread stops early once the receiver
/// is dropped.
pub(super) fn start(index: FxrLib, ids: Vec<i64>) -> Receiver<Warmed> {
    let (tx, rx) = channel();
    let spawned = std::thread::Builder::new()
        .name("fxr warm-up".into())
        .spawn(move || warm(index, ids, &tx));
    if let Err(e) = spawned {
        bevy::log::warn!("FXR warm-up thread did not start: {e}");
    }
    rx
}

fn warm(mut lib: FxrLib, ids: Vec<i64>, tx: &Sender<Warmed>) {
    let started = std::time::Instant::now();
    // Parse every effect first (compiling the effects each references), then send them in one
    // pass, then decode the textures, the slow part.
    let mut textures = Vec::new();
    for id in &ids {
        for texture in lib.texture_ids(*id) {
            if !textures.contains(&texture) {
                textures.push(texture);
            }
        }
    }
    let mut effects = 0;
    for (effect, tree, node) in lib.parsed() {
        effects += 1;
        if tx.send(Warmed::Effect(effect, tree, node)).is_err() {
            return;
        }
    }
    for &texture in &textures {
        if let Some(image) = super::load_texture(&lib, texture, false)
            && tx.send(Warmed::Texture(texture, Box::new(image))).is_err()
        {
            return;
        }
    }
    bevy::log::info!(
        "FXR warm-up: {} effects ({effects} with references), {} textures in {:.2} s",
        ids.len(),
        textures.len(),
        started.elapsed().as_secs_f32()
    );
}

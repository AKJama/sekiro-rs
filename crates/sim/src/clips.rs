//! Decoded animation clips for the simulation: lengths and root motion.
//!
//! Clips come from the extracted cache (`cache/anim/<chr>/<name>.bin`). Some animations have no
//! HKX of their own and play another's, as recorded in the TAE headers; [`ClipLibrary::alias`]
//! resolves those. The library is a cheap shared handle so the behaviour runtime (lengths) and
//! the locomotion (root motion) read the same cache.

use crate::behavior::ClipDurations;
use sekiro_formats::anim::AnimClip;
use sekiro_formats::tae;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

#[derive(Default)]
struct Inner {
    dir: PathBuf,
    clips: HashMap<String, Option<Arc<AnimClip>>>,
    /// Full animation id to the id whose HKX it plays.
    hkx_alias: HashMap<i64, i64>,
}

/// A shared, lazily filled clip cache.
#[derive(Clone, Default)]
pub struct ClipLibrary(Rc<RefCell<Inner>>);

impl ClipLibrary {
    /// Opens `dir` (`cache/anim/<chr>`), reading the extractor's `aliases.json` (animation name
    /// to the decoded clip it plays, chains already resolved) when present.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        let aliases: std::collections::BTreeMap<String, String> =
            std::fs::read_to_string(dir.join("aliases.json"))
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
        let lib = Self(Rc::new(RefCell::new(Inner {
            dir,
            ..Inner::default()
        })));
        for (name, src) in aliases {
            if let (Some(a), Some(b)) = (tae::anim_id(&name), tae::anim_id(&src)) {
                lib.add_alias(a, b);
            }
        }
        lib
    }

    /// Records that animation `id` plays the HKX of `source` (full ids). The first alias recorded
    /// for an id wins, so the resolved `aliases.json` takes precedence over single TAE hops.
    pub fn add_alias(&self, id: i64, source: i64) {
        if id != source {
            self.0.borrow_mut().hkx_alias.entry(id).or_insert(source);
        }
    }

    /// Reads HKX aliases from every `*.tae` in `dir` (an extracted `anibnd`).
    pub fn with_tae_dir(self, dir: &Path) -> Self {
        for (id, anim) in crate::tae::read_tae_dir(dir) {
            self.add_alias(id, anim.hkx_source);
        }
        self
    }

    /// The animation name whose HKX `name` plays.
    pub fn alias(&self, name: &str) -> String {
        tae::anim_id(name)
            .and_then(|id| self.0.borrow().hkx_alias.get(&id).copied())
            .map(tae::anim_name)
            .unwrap_or_else(|| name.to_owned())
    }

    /// The decoded clip (source space), following HKX aliases. `None` if not extracted.
    pub fn clip(&self, name: &str) -> Option<Arc<AnimClip>> {
        if let Some(c) = self.0.borrow().clips.get(name) {
            return c.clone();
        }
        let load = |n: &str| -> Option<Arc<AnimClip>> {
            let bytes = std::fs::read(self.0.borrow().dir.join(format!("{n}.bin"))).ok()?;
            AnimClip::from_bytes(&bytes).ok().map(Arc::new)
        };
        let clip = load(name).or_else(|| {
            let src = self.alias(name);
            (src != name).then(|| load(&src)).flatten()
        });
        self.0
            .borrow_mut()
            .clips
            .insert(name.to_owned(), clip.clone());
        clip
    }
}

impl ClipDurations for ClipLibrary {
    fn duration(&mut self, animation: &str) -> Option<f32> {
        // An animation the game data does not contain (the graph references a few, such as
        // a000_206100) counts as zero length, so its state ends at once instead of hanging.
        Some(self.clip(animation).map_or(0.0, |c| c.duration))
    }
}

//! Job C handoff: first-visit story input for the single Job E visibility decision.
//!
//! `load(cache, map, layout)` returns None outside Outskirts/Castle, preserving old maps.
//! `enabled(entity_id, name)` supplies the story gate for each real part.
//! Job E combines this gate with masks/residency; this module never writes Visibility.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use bevy::prelude::Resource;
use sekiro_sim::world_events::{self, PartState, WorldState};
use serde_json::Value;

#[derive(Debug, Resource, Default)]
pub struct MapStory {
    pub maps: HashMap<String, StoryMap>,
}

#[derive(Debug)]
pub struct StoryMap {
    pub state: WorldState,
    pub parts: Vec<PartState>,
    by_name: HashMap<String, usize>,
}

impl StoryMap {
    pub fn enabled(&self, entity_id: i32, name: &str) -> Option<bool> {
        let part = self.part(name)?;
        (part.entity_id == entity_id).then_some(part.enabled)
    }

    pub fn part(&self, name: &str) -> Option<&PartState> {
        self.by_name.get(name).map(|&i| &self.parts[i])
    }
}

pub fn load(cache: &Path, map: &str, layout: &Value) -> Result<Option<StoryMap>> {
    // Diagnostic control for paired hole-sweep runs with identical loaded geometry.
    if std::env::args().any(|arg| arg == "--no-map-story") {
        return Ok(None);
    }
    let Some(state) = world_events::first_visit(cache, map).map_err(anyhow::Error::msg)? else {
        return Ok(None);
    };
    anyhow::ensure!(
        layout["layout_version"].as_u64() == Some(2),
        "story state requires lossless layout v2"
    );
    layout["parts"]
        .as_array()
        .context("story state requires canonical parts")?;
    let parts = state.parts(layout);
    let mut by_name = HashMap::new();
    for (i, part) in parts.iter().enumerate() {
        anyhow::ensure!(
            by_name.insert(part.name.clone(), i).is_none(),
            "duplicate story part name {}",
            part.name
        );
    }
    Ok(Some(StoryMap {
        state,
        parts,
        by_name,
    }))
}

//! Offline story input for the single map visibility decision.
//!
//! `load(cache, map, layout)` returns None outside Outskirts/Castle, preserving old maps.
//! `enabled(entity_id, name)` supplies the story gate for each real part.
//! Visibility combines this gate with masks/residency; this module never writes Visibility.
//! Named reference progression adds known boss defeats to the first-visit fixture.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use bevy::prelude::Resource;
use sekiro_sim::world_events::{self, PartState, WorldState};
use serde_json::Value;

pub const REFERENCE_STORY: &str = "genichiro-defeated";

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

/// `--story`, else the story a render/reference gate's fixture declares (the canonical
/// reference story when it declares none), else None for the map's own default.
pub fn selection(args: &[String]) -> Option<String> {
    crate::arg_value(args, "--story").or_else(|| {
        let story = std::fs::read(gate_fixture(args)?)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            .and_then(|gate| gate["story"].as_str().map(String::from));
        Some(story.unwrap_or_else(|| REFERENCE_STORY.into()))
    })
}

/// The checked-in fixture a `--render-gate` or `--reference-gate` run compares against:
/// `tests/data/<map>[.<--gate-fixture>].<render|reference>.json`.
pub(crate) fn gate_fixture(args: &[String]) -> Option<PathBuf> {
    let kind = if args.iter().any(|a| a == "--reference-gate") {
        "reference"
    } else if args.iter().any(|a| a == "--render-gate") {
        "render"
    } else {
        return None;
    };
    let map = crate::arg_value(args, "--map")?;
    let variant = gate_variant(args).map_or(String::new(), |v| format!("{v}."));
    Some(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("tests/data/{map}.{variant}{kind}.json")),
    )
}

/// `--gate-fixture <name>`: a named variant of a map's gate fixture, such as `opening`.
pub(crate) fn gate_variant(args: &[String]) -> Option<String> {
    crate::arg_value(args, "--gate-fixture")
        .filter(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
}

impl StoryMap {
    pub fn enabled(&self, entity_id: i32, name: &str) -> Option<bool> {
        let part = self.part(name)?;
        (part.entity_id == entity_id).then_some(part.enabled)
    }

    pub fn part(&self, name: &str) -> Option<&PartState> {
        self.by_name.get(name).map(|&i| &self.parts[i])
    }

    /// The story has switched this part off (unknown parts stay on).
    pub fn switched_off(&self, name: &str) -> bool {
        self.part(name).is_some_and(|p| !p.enabled)
    }
}

pub(crate) fn initial_state(
    cache: &Path,
    map: &str,
    story: Option<&str>,
) -> Result<Option<WorldState>> {
    let Some(mut fixture) = world_events::FirstVisit::for_map(map) else {
        return Ok(None);
    };
    if let Some(name) = story {
        match name {
            // Persistent writes of 11105800, then 11115800 through its final cutscene guard.
            "gyoubu-defeated" => fixture.flags.extend([9301, 11100800, 6801, 6078]),
            "genichiro-defeated" => fixture
                .flags
                .extend([9301, 11100800, 6801, 6078, 9303, 11110800, 6803, 9403]),
            _ => {}
        }
    }
    world_events::evaluate(cache, map, fixture)
        .map(Some)
        .map_err(anyhow::Error::msg)
}

/// [`load`] for each of a world's blocks, by block id.
pub fn load_blocks(cache: &Path, layouts: &BTreeMap<String, Value>) -> Result<MapStory> {
    let mut out = MapStory::default();
    for (block, layout) in layouts {
        if let Some(state) = load(cache, block, layout)? {
            out.maps.insert(block.clone(), state);
        }
    }
    Ok(out)
}

pub fn load(cache: &Path, map: &str, layout: &Value) -> Result<Option<StoryMap>> {
    // Diagnostic control for paired hole-sweep runs with identical loaded geometry.
    if std::env::args().any(|arg| arg == "--no-map-story") {
        return Ok(None);
    }
    let args: Vec<_> = std::env::args().collect();
    let Some(state) = initial_state(cache, map, selection(&args).as_deref())? else {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_gate_runs_the_story_its_fixture_declares() {
        let args = |s: &str| s.split(' ').map(String::from).collect::<Vec<_>>();
        for (command, story, sky) in [
            (
                "--render-gate --map m11_00_00_00",
                Some("genichiro-defeated"),
                Some("m900010"),
            ),
            (
                "--render-gate --map m11_01_00_00",
                Some("genichiro-defeated"),
                Some("m900010"),
            ),
            // The opening (before Gyoubu, morning sky), the state a new play session starts in.
            (
                "--render-gate --map m11_00_00_00 --gate-fixture opening",
                Some("morning"),
                Some("m900000"),
            ),
            (
                "--reference-gate --map m11_00_00_00",
                Some(REFERENCE_STORY),
                None,
            ),
            (
                "--render-gate --map m11_00_00_00 --story night",
                Some("night"),
                None,
            ),
            ("--play --map m11_00_00_00", None, None),
        ] {
            let args = args(command);
            assert_eq!(selection(&args).as_deref(), story, "{command}");
            if let Some(sky) = sky {
                assert!(gate_fixture(&args).unwrap().is_file(), "{command}");
                let lit = crate::lighting_selection::select(
                    &sekiro_sim::cache::root(),
                    &crate::arg_value(&args, "--map").unwrap(),
                    story,
                    None,
                )
                .unwrap();
                assert_eq!(lit.sky.as_deref(), Some(sky), "{command}");
            }
        }
    }
}

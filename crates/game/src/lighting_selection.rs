//! Selection only: event-authored story environments and collision GPARAM keys.
use std::{collections::BTreeMap, path::Path};

use anyhow::{Context, Result};
use bevy::prelude::*;
use sekiro_formats::{
    emedf,
    emevd::{Emevd, Instruction},
    gparam::{self, Gparam},
};
use sekiro_sim::{
    arena::{ArenaSpec, scene::SceneState},
    events::{EventEntry, EventHost, EventScripts},
    story::StoryTime,
};
use serde_json::Value;

#[path = "lighting/maps/m10.rs"]
mod m10;
#[path = "lighting/maps/m11_00.rs"]
mod m11_00;
#[path = "lighting/maps/m11_01.rs"]
mod m11_01;
#[path = "lighting/maps/m11_02.rs"]
mod m11_02;
#[path = "lighting/maps/m13.rs"]
mod m13;
#[path = "lighting/maps/m15.rs"]
mod m15;
#[path = "lighting/maps/m17.rs"]
mod m17;
#[path = "lighting/maps/m20.rs"]
mod m20;
#[path = "lighting/maps/m25.rs"]
mod m25;
#[path = "lighting/maps/m89.rs"]
mod m89;

const MAPS: [&str; 10] = [
    m10::PROFILE,
    m11_00::PROFILE,
    m11_01::PROFILE,
    m11_02::PROFILE,
    m13::PROFILE,
    m15::PROFILE,
    m17::PROFILE,
    m20::PROFILE,
    m25::PROFILE,
    m89::PROFILE,
];

/// All four independent fields, even when the installed map only uses zero for three.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegionSelection(pub [i32; 4]);

impl RegionSelection {
    pub fn enter(&mut self, config: [i32; 4]) {
        for (current, next) in self.0.iter_mut().zip(config) {
            if next >= 0 {
                *current = next;
            }
        }
    }
}

#[derive(Debug)]
pub struct Selection {
    pub fixed_environment: bool,
    pub hour: f32,
    pub environment: u32,
    pub sub_id: u8,
    pub source: String,
    pub sky: Option<String>,
    pub flags: Vec<i32>,
}

fn profile(id: &str) -> Result<Value> {
    MAPS.iter()
        .map(|s| serde_json::from_str::<Value>(s).expect("lighting profile JSON"))
        .find(|p| id.get(..6) == p["map"].as_str())
        .with_context(|| format!("no lighting profile for {id}"))
}

struct MapHost {
    area: u8,
    block: u8,
}
impl EventHost for MapHost {
    fn encounter_condition(&self, i: &Instruction) -> Option<bool> {
        (i.code() == (3, 8)).then(|| {
            (i.u8_at(2) == Some(self.area) && i.u8_at(3) == Some(self.block))
                == (i.u8_at(1) == Some(1))
        })
    }
    fn has_sp_effect(&self, _: i32, _: i32) -> Option<bool> {
        None
    }
    fn has_event_message(&self, _: i32, _: i32) -> Option<bool> {
        None
    }
    fn is_dead(&self, _: i32) -> Option<bool> {
        None
    }
    fn hp(&self, _: i32) -> Option<i32> {
        None
    }
}

fn script(cache: &Path, map: &str) -> Result<Emevd> {
    let path = cache.join(format!("raw/event/{map}.emevd"));
    Ok(Emevd::parse(
        &std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?,
    )?)
}

/// Resolve a sub-ID against the installed catalog, never substitute a nearest number.
/// Families without a 00 file must be unique or explicitly documented in the map profile.
pub fn sub_file(cache: &Path, id: &str, sub: u8) -> Result<String> {
    let p = profile(id)?;
    if let Some(stem) = p["sub_files"][sub.to_string()].as_str() {
        anyhow::ensure!(
            cache
                .join(format!("raw/param/drawparam/{stem}.gparam"))
                .is_file(),
            "missing {stem}"
        );
        return Ok(stem.into());
    }
    let base = format!("{}_00{sub:02}", &id[..6]);
    let dir = cache.join("raw/param/drawparam");
    if dir.join(format!("{base}.gparam")).is_file() {
        return Ok(base);
    }
    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        if name.starts_with(&format!("{}_", &id[..6]))
            && name.ends_with(&format!("{sub:02}.gparam"))
        {
            candidates.push(name.trim_end_matches(".gparam").to_string());
        }
    }
    anyhow::ensure!(
        candidates.len() == 1,
        "{id} sub-ID {sub}: ambiguous/missing GPARAM families {candidates:?}"
    );
    Ok(candidates.remove(0))
}

pub fn select(
    cache: &Path,
    id: &str,
    story: Option<&str>,
    arena: Option<&ArenaSpec>,
) -> Result<Selection> {
    let p = profile(id)?;
    let name = story.unwrap_or(p["default"].as_str().context("map default")?);
    let states = p["states"].as_array().context("map states")?;
    let state = states
        .iter()
        .find(|s| s["name"].as_str() == Some(name))
        .with_context(|| {
            format!(
                "--story {name} for {id}: choose {}",
                states
                    .iter()
                    .filter_map(|s| s["name"].as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
    let mut flags: Vec<i32> = state["flags"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_i64)
        .map(|f| f as i32)
        .collect();
    if story.is_none()
        && let Some(arena) = arena
    {
        flags = arena.beaten.to_vec();
        if let Some(boss) = sekiro_sim::arena::boss_events(&arena.enemy_spec()) {
            flags.extend(boss.flags);
        }
    }
    for rule in p["seed_rules"].as_array().into_iter().flatten() {
        if rule["when"].as_array().is_some_and(|required| {
            required
                .iter()
                .all(|f| f.as_i64().is_some_and(|f| flags.contains(&(f as i32))))
        }) {
            flags.extend(
                rule["set"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_i64)
                    .map(|f| f as i32),
            );
        }
    }
    let area = id[1..3].parse()?;
    let state = if story.is_none() && arena.is_some() {
        states
            .iter()
            .find(|s| {
                s["when_flags"].as_array().is_some_and(|required| {
                    !required.is_empty()
                        && required
                            .iter()
                            .all(|f| f.as_i64().is_some_and(|f| flags.contains(&(f as i32))))
                })
            })
            .unwrap_or(state)
    } else {
        state
    };
    let apply_preset = story.is_some() || arena.is_none() || !state["when_flags"].is_null();
    let block = id[4..6].parse()?;
    let common = script(cache, "common")?;
    let mut events = EventScripts::new(
        &common,
        &[EventEntry {
            id: 100,
            from_label: None,
        }],
    )
    .map_err(anyhow::Error::msg)?;
    // Event 820 raises the cumulative clock before event 100 selects the environment.
    let time = StoryTime::from_flags(|f| flags.contains(&f));
    let variant = super::lighting::nearest_variant(time.hour()) as usize;
    let count = match variant {
        1 => 0,
        2 => 1,
        3 => 2,
        4 => 3,
        5 => 4,
        _ => 0,
    };
    flags.extend(sekiro_sim::story::TIME_FLAGS[..count].iter());
    events.flags.extend(flags.iter().copied());
    events.step(0., &MapHost { area, block });
    let mut scene = events.cast_scene;
    // Presets reference original instruction locations, not copied parameter values.
    if apply_preset {
        for command in state["commands"].as_array().into_iter().flatten() {
            let event = command[0].as_i64().context("lighting event reference")?;
            let index = command[1]
                .as_u64()
                .context("lighting instruction reference")? as usize;
            let source_map = state["event_map"].as_str().unwrap_or(id);
            let file = script(cache, source_map)?;
            let ins = file
                .event(event)
                .and_then(|e| e.instructions.get(index))
                .context("lighting instruction missing")?;
            anyhow::ensure!(
                matches!(ins.code(), (2003, 80 | 81 | 88) | (2012, 1)),
                "lighting reference changed: {event}:{index}"
            );
            let args: Vec<_> = emedf::decode(ins)
                .context("lighting instruction layout")?
                .1
                .into_iter()
                .map(|(_, value)| value)
                .collect();
            scene.instruction(ins, &args);
        }
    }
    let environment = scene
        .environment
        .context("event 100 did not select an environment")? as u32;
    let mut hour = StoryTime::from_variant(environment as i32)
        .context("unknown authored environment")?
        .hour();
    if let Some(lighting) = scene.lighting.and_then(StoryTime::from_variant) {
        hour = lighting.hour();
    }
    if story.is_none()
        && let Some(arena) = arena
    {
        hour = arena.story_time(&arena.enemy_spec()).hour();
    }
    let sub_id = scene
        .gparam_sub_ids
        .get(&(area as i16, block as i16))
        .map_or(0, |s| s.0);
    let source = if sub_id == 0
        && let Some(source) = p["base_file"].as_str()
    {
        source.into()
    } else if !state["commands"].is_null()
        && apply_preset
        && let Some(source) = p["event_sub_files"][sub_id.to_string()].as_str()
    {
        source.into()
    } else {
        sub_file(cache, id, sub_id)?
    };
    let sky = sky_model(cache, &scene)?;
    Ok(Selection {
        fixed_environment: p["fixed_environment"].as_bool().unwrap_or(false),
        hour,
        environment,
        sub_id,
        source,
        sky,
        flags,
    })
}

fn sky_model(cache: &Path, scene: &SceneState) -> Result<Option<String>> {
    let file = cache.join("raw/map/mapstudio/m89_00_00_99.msb");
    let msb = sekiro_formats::msb::parse(&std::fs::read(file)?)?;
    Ok(scene
        .enabled_order
        .iter()
        .rev()
        .filter(|group| scene.enabled.get(group) == Some(&true))
        .find_map(|group| {
            msb.parts
                .iter()
                .find(|p| p.entity_group_ids.contains(group))
                .and_then(|p| msb.model(p))
                .map(|m| m.name.clone())
        }))
}

/// Loaded once per map. Runtime event overrides use the same resolver as initial selection.
#[derive(Resource)]
pub struct LightingSelection {
    pub initial: Selection,
    pub area: (i16, i16),
    files: BTreeMap<u8, (String, Gparam)>,
    base: Gparam,
    current: Option<u8>,
    pinned: bool,
    fixed_environment: bool,
}

impl LightingSelection {
    pub fn load(cache: &Path, id: &str, initial: Selection, pinned: bool) -> Result<Self> {
        let area = (id[1..3].parse()?, id[4..6].parse()?);
        let mut subs = std::collections::BTreeSet::from([initial.sub_id]);
        for map in ["common", id] {
            if !cache.join(format!("raw/event/{map}.emevd")).is_file() {
                continue;
            }
            for event in script(cache, map)?.events {
                for i in event.instructions {
                    if i.code() == (2003, 88)
                        && i.i32_at(0) == Some(i32::from(area.0))
                        && i.i32_at(4) == Some(i32::from(area.1))
                        && let Some(sub) = i.u8_at(8)
                    {
                        subs.insert(sub);
                    }
                }
            }
        }
        let mut files = BTreeMap::new();
        let profile = profile(id)?;
        for sub in subs {
            let source = if let Some(source) = profile["event_sub_files"][sub.to_string()].as_str()
            {
                source.into()
            } else if sub == initial.sub_id {
                initial.source.clone()
            } else {
                sub_file(cache, id, sub)?
            };
            let g = gparam::parse(&std::fs::read(
                cache.join(format!("raw/param/drawparam/{source}.gparam")),
            )?)?;
            files.insert(sub, (source, g));
        }
        let base = gparam::parse(&std::fs::read(
            cache.join(format!("raw/param/drawparam/{}.gparam", initial.source)),
        )?)?;
        Ok(Self {
            initial,
            area,
            files,
            base,
            current: None,
            pinned,
            fixed_environment: profile["fixed_environment"].as_bool().unwrap_or(false),
        })
    }

    pub fn environment_for_hour(&self, hour: f32) -> u32 {
        if self.fixed_environment {
            self.initial.environment
        } else {
            crate::lighting::nearest_variant(hour)
        }
    }

    pub fn apply_sub(&mut self, requested: Option<u8>, light: &mut crate::lighting::MapLighting) {
        if self.pinned || requested == self.current {
            return;
        }
        let Some(sub) = requested else {
            light.gparam.clone_from(&self.base);
            light.source.clone_from(&self.initial.source);
            light.applied = None;
            self.current = None;
            return;
        };
        if let Some((source, g)) = self.files.get(&sub) {
            light.gparam.clone_from(g);
            light.source.clone_from(source);
            light.applied = None;
            self.current = Some(sub);
            info!("event lighting: {source}, sub-ID {sub}");
        } else {
            warn!("event lighting: unavailable sub-ID {sub}");
        }
    }
}

pub fn sync_events(
    scene: Option<Res<crate::arena_b::Scene>>,
    map_scene: Res<MapEventScene>,
    mut selection: ResMut<LightingSelection>,
    mut light: ResMut<crate::lighting::MapLighting>,
) {
    let scene = scene.as_deref().map_or(&map_scene.0, |s| &s.0);
    let sub = scene.gparam_sub_ids.get(&selection.area).map(|s| s.0);
    selection.apply_sub(sub, &mut light);
}

#[derive(Resource, Default)]
pub struct MapEventScene(pub SceneState);

#[derive(Resource)]
pub struct MapStoryEvents {
    cache: std::path::PathBuf,
    map: String,
    scripts: EventScripts,
}

impl MapStoryEvents {
    pub fn install(app: &mut App, cache: &Path, id: &str, story: Option<&str>) -> Result<()> {
        app.init_resource::<MapEventScene>();
        let p = profile(id)?;
        let name = story.unwrap_or(p["default"].as_str().context("map default")?);
        let state = p["states"]
            .as_array()
            .context("map states")?
            .iter()
            .find(|s| s["name"].as_str() == Some(name))
            .context("map story state")?;
        let entries: Vec<_> = state["events"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_i64)
            .map(|id| EventEntry {
                id,
                from_label: None,
            })
            .collect();
        if entries.is_empty() {
            return Ok(());
        }
        let mut scripts =
            EventScripts::new(&script(cache, id)?, &entries).map_err(anyhow::Error::msg)?;
        scripts.flags.extend(
            app.world()
                .resource::<LightingSelection>()
                .initial
                .flags
                .iter()
                .copied(),
        );
        app.insert_resource(Self {
            cache: cache.into(),
            map: id.into(),
            scripts,
        })
        .add_systems(Update, tick_map_events);
        Ok(())
    }
}

struct PositionHost<'a> {
    cache: &'a Path,
    map: &'a str,
    position: [f32; 3],
}
impl EventHost for PositionHost<'_> {
    fn has_sp_effect(&self, _: i32, _: i32) -> Option<bool> {
        None
    }
    fn has_event_message(&self, _: i32, _: i32) -> Option<bool> {
        None
    }
    fn is_dead(&self, _: i32) -> Option<bool> {
        None
    }
    fn hp(&self, _: i32) -> Option<i32> {
        None
    }
    fn inside_area(&self, entity: i32, area: i32) -> Option<bool> {
        (entity == sekiro_sim::events::PLAYER_ENTITY)
            .then(|| {
                sekiro_sim::arena_c::regions::contains(self.cache, self.map, area, self.position)
            })
            .flatten()
    }
}

fn tick_map_events(
    mut events: ResMut<MapStoryEvents>,
    mut scene: ResMut<MapEventScene>,
    time: Res<Time>,
    focus: Query<&Transform, With<crate::map::MapFocus>>,
) {
    let Some(focus) = focus.iter().next() else {
        return;
    };
    let MapStoryEvents {
        cache,
        map,
        scripts,
    } = &mut *events;
    scripts.step(
        time.delta_secs(),
        &PositionHost {
            cache,
            map,
            position: focus.translation.to_array(),
        },
    );
    scene.0.clone_from(&scripts.cast_scene);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cache() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cache")
    }
    #[test]
    fn defaults_follow_installed_common_event() {
        for (map, hour, environment) in [
            ("m10_00", 0., 0),
            ("m11_00", 6., 1),
            ("m11_01", 6., 1),
            ("m11_02", 0., 0),
            ("m13_00", 6., 1),
            ("m15_00", 6., 1),
            ("m17_00", 6., 1),
            ("m20_00", 6., 1),
            ("m25_00", 22., 4),
        ] {
            let selection = select(&cache(), &format!("{map}_00_00"), None, None).unwrap();
            assert_eq!(
                (selection.hour, selection.environment),
                (hour, environment),
                "{map}"
            );
            assert_eq!(selection.source, format!("{map}_0000"));
        }
    }
    #[test]
    fn invasion_and_storm_use_authored_sub_ids() {
        for map in ["m11_00", "m11_01", "m11_02"] {
            let s = select(&cache(), &format!("{map}_00_00"), Some("invasion"), None).unwrap();
            assert_eq!((s.environment, s.sub_id), (4, 20));
            assert_eq!(s.source, format!("{map}_0020"));
        }
        let s = select(&cache(), "m11_02_00_00", Some("sword-saint-storm"), None).unwrap();
        assert_eq!(s.source, "m11_02_0930");
        let clear = select(&cache(), "m11_02_00_00", Some("sword-saint-defeated"), None).unwrap();
        assert_eq!(clear.source, "m11_02_0920");
        for (map, sub, source) in [("m11_00", 30, "m11_00_0030"), ("m11_01", 10, "m11_01_0010")] {
            let s = select(&cache(), &format!("{map}_00_00"), Some("tomoe"), None).unwrap();
            assert_eq!((s.hour, s.sub_id, s.source.as_str()), (18., sub, source));
        }
    }
    #[test]
    fn boss_arenas_keep_their_story_state() {
        for arena in sekiro_sim::arena::ARENAS {
            let s = select(&cache(), arena.map, None, Some(arena)).unwrap();
            assert_eq!(
                s.hour,
                arena.story_time(&arena.enemy_spec()).hour(),
                "{}",
                arena.name
            );
            if arena.map.starts_with("m10") {
                assert_eq!(s.environment, 0);
            }
        }
        let morning = select(
            &cache(),
            "m11_01_00_00",
            None,
            Some(ArenaSpec::by_name("genichiro").unwrap()),
        )
        .unwrap();
        let evening = select(
            &cache(),
            "m11_01_00_00",
            None,
            Some(ArenaSpec::by_name("owl").unwrap()),
        )
        .unwrap();
        assert_eq!((morning.hour, evening.hour), (6., 18.));
    }
    #[test]
    fn region_switch_preserves_all_independent_collision_ids() {
        let mut r = RegionSelection::default();
        r.enter([110, 7, 9, 12]);
        assert_eq!(r.0, [110, 7, 9, 12]);
        r.enter([0, 0, 0, 0]);
        assert_eq!(r.0, [0; 4]);
        r.enter([-1; 4]);
        assert_eq!(r.0, [0; 4]);
    }

    #[test]
    fn map_sky_override_wins_over_the_common_sky() {
        let s = select(&cache(), "m20_00_00_00", Some("illusion"), None).unwrap();
        let msb = sekiro_formats::msb::parse(
            &std::fs::read(cache().join("raw/map/mapstudio/m89_00_00_99.msb")).unwrap(),
        )
        .unwrap();
        let part = msb
            .parts
            .iter()
            .find(|p| p.entity_group_ids.contains(&8_907_085))
            .unwrap();
        assert_eq!(
            s.sky.as_deref(),
            Some(msb.model(part).unwrap().name.as_str())
        );
    }
    #[test]
    #[ignore = "slow: audit every installed map story preset"]
    fn all_presets_resolve() {
        for json in MAPS.iter().take(9) {
            let p: Value = serde_json::from_str(json).unwrap();
            let map = format!("{}_00_00", p["map"].as_str().unwrap());
            for state in p["states"].as_array().unwrap() {
                select(&cache(), &map, state["name"].as_str(), None).unwrap();
            }
        }
    }

    #[test]
    fn event_sub_id_reset_restores_the_initial_story_file() {
        let s = select(&cache(), "m11_02_00_00", None, None).unwrap();
        let mut library = LightingSelection::load(&cache(), "m11_02_00_00", s, false).unwrap();
        let (source, gparam) = library.files[&0].clone();
        let mut light = crate::lighting::MapLighting {
            source,
            gparam,
            hour: 0.,
            light_set: 0,
            applied: Some(0),
        };
        library.apply_sub(Some(30), &mut light);
        assert_eq!(light.source, "m11_02_0930");
        library.apply_sub(None, &mut light);
        assert_eq!(light.source, "m11_02_0000");
        assert_eq!(light.applied, None);
    }
}

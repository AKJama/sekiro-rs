//! Memory across fights and sessions: the outcome tally and Wolf's habits summed over every
//! fight against one enemy, with the fight count and who won, kept in
//! `logs/memory/<chr>.json` (gitignored) and sent with each request. Hosted models remember
//! nothing, so this is all they know of earlier fights.
//!
//! The file is versioned ([`MEMORY_VERSION`]); a missing file starts fresh, and an unreadable
//! one (bad JSON, another version or another enemy) starts fresh with a warning and is kept
//! beside the new one as `<name>.json.corrupt`.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::habits::{HabitSummary, Habits};
use super::tally::{CountMap, Counts, Side, Tally, TopMap, merge_counts, ranked_counts};

/// The memory file format version.
pub const MEMORY_VERSION: u32 = 1;

/// Where memory lives by default.
pub const MEMORY_DIR: &str = "logs/memory";

/// How a fight ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FightEnd {
    /// Wolf died.
    BossWon,
    /// The boss died.
    WolfWon,
    /// The game closed (or the file was saved) mid-fight.
    Unfinished,
}

/// The memory file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Memory {
    pub version: u32,
    /// The enemy (`c1010`).
    pub chr: String,
    /// Fights recorded, finished or not.
    #[serde(default)]
    pub fights: u32,
    #[serde(default)]
    pub boss_wins: u32,
    #[serde(default)]
    pub wolf_wins: u32,
    /// The tally of every fight, per boss action id and per Wolf attack.
    #[serde(default)]
    pub boss: CountMap,
    #[serde(default)]
    pub wolf: CountMap,
    #[serde(default)]
    pub habits: Habits,
}

/// How a load went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Loaded {
    /// Read from the file.
    Read,
    /// No file yet.
    Missing,
    /// The file could not be used (the reason); it was moved aside.
    Corrupt(String),
}

/// [`Memory`] as a controller sees it.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct MemorySummary {
    /// Earlier fights against this enemy, and who won them (the rest were left unfinished).
    pub fights: u32,
    pub boss_wins: u32,
    pub wolf_wins: u32,
    /// The tally over every fight, this one included, most used first.
    pub boss: TopMap<Counts>,
    pub wolf: TopMap<Counts>,
    pub habits: HabitSummary,
}

impl MemorySummary {
    /// Drops one item to shrink a request (see `BrainRequest::fit`); true if something went.
    pub fn trim(&mut self, keep: usize) -> bool {
        self.boss.pop(keep) || self.wolf.pop(keep) || self.habits.trim(keep)
    }
}

impl Memory {
    pub fn new(chr: &str) -> Self {
        Self {
            version: MEMORY_VERSION,
            chr: chr.to_string(),
            fights: 0,
            boss_wins: 0,
            wolf_wins: 0,
            boss: CountMap::new(),
            wolf: CountMap::new(),
            habits: Habits::default(),
        }
    }

    /// Reads the memory for `chr` from `path`; starts fresh when it is missing or unusable,
    /// moving an unusable file aside so the next save does not destroy it.
    pub fn load(path: &Path, chr: &str) -> (Self, Loaded) {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return (Self::new(chr), Loaded::Missing);
            }
            Err(e) => return (Self::new(chr), Loaded::Corrupt(e.to_string())),
        };
        let problem = match serde_json::from_str::<Memory>(&text) {
            Ok(m) if m.version != MEMORY_VERSION => format!(
                "format version {} (this build reads {MEMORY_VERSION})",
                m.version
            ),
            Ok(m) if m.chr != chr => format!("memory of {} (wanted {chr})", m.chr),
            Ok(m) => return (m, Loaded::Read),
            Err(e) => format!("bad JSON: {e}"),
        };
        let mut aside = path.as_os_str().to_owned();
        aside.push(".corrupt");
        let _ = std::fs::rename(path, PathBuf::from(aside));
        (Self::new(chr), Loaded::Corrupt(problem))
    }

    /// Writes the file (through a temporary file, so a crash never leaves half a file).
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir)?;
        }
        let mut tmp = path.as_os_str().to_owned();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        {
            let mut f = std::fs::File::create(&tmp)?;
            serde_json::to_writer_pretty(&mut f, self)?;
            f.write_all(b"\n")?;
            f.sync_all()?;
        }
        std::fs::rename(&tmp, path)
    }

    /// Adds one fight: its tally (already flushed) and Wolf's habits in it.
    pub fn add_fight(&mut self, tally: &Tally, habits: &Habits, end: FightEnd) {
        self.fights += 1;
        match end {
            FightEnd::BossWon => self.boss_wins += 1,
            FightEnd::WolfWon => self.wolf_wins += 1,
            FightEnd::Unfinished => {}
        }
        self.add_data(tally, habits);
    }

    /// Adds a fight's tally and habits without counting it as a fight.
    pub fn add_data(&mut self, tally: &Tally, habits: &Habits) {
        merge_counts(&mut self.boss, tally.all(Side::Boss));
        merge_counts(&mut self.wolf, tally.all(Side::Wolf));
        self.habits.merge(habits);
    }

    pub fn summary(&self) -> MemorySummary {
        MemorySummary {
            fights: self.fights,
            boss_wins: self.boss_wins,
            wolf_wins: self.wolf_wins,
            boss: ranked_counts(&self.boss),
            wolf: ranked_counts(&self.wolf),
            habits: self.habits.summary(),
        }
    }
}

/// The default memory file for an enemy: `logs/memory/<chr>.json`.
pub fn default_path(chr: &str) -> PathBuf {
    Path::new(MEMORY_DIR).join(format!("{chr}.json"))
}

/// The memory file from `--brain-memory`: `off` none, `on` the default, a path ending in
/// `.json` that file, any other path a folder holding `<chr>.json`. Without the flag, hosted
/// brains remember (`hosted`) and the others do not.
pub fn resolve(arg: Option<&str>, chr: &str, hosted: bool) -> Option<PathBuf> {
    match arg {
        None => hosted.then(|| default_path(chr)),
        Some(a) if a.eq_ignore_ascii_case("off") => None,
        Some(a) if a.eq_ignore_ascii_case("on") => Some(default_path(chr)),
        Some(a) if a.to_ascii_lowercase().ends_with(".json") => Some(PathBuf::from(a)),
        Some(a) => Some(Path::new(a).join(format!("{chr}.json"))),
    }
}

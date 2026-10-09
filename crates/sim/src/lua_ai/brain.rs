//! [`AiBrain`]: the game's own AI scripts driving an [`NpcControl`].

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::lua50::{LuaError, Value, Vm, load_chunk};
use super::runtime::{AiEvent, AiState, tick};
use super::world::{AiWorld, ThinkParams};
use crate::character::NpcControl;

/// Behaviour reference the NPC script reads as "in battle" (`SP_EFFECT_REF_AI_BATTLE`).
pub const REF_AI_BATTLE: i32 = 1_000_003;

#[derive(Debug, thiserror::Error)]
pub enum AiLoadError {
    #[error("{0}: {1}")]
    Io(PathBuf, std::io::Error),
    #[error("{0}: {1}")]
    Lua(String, LuaError),
}

/// Files that define constants and the table-goal helpers; loaded before the rest.
const FIRST: [&str; 5] = [
    "ai_define.lua",
    "goal_list.lua",
    "logic_list.lua",
    "event_list.lua",
    "table_ai_common.lua",
];

/// Engine functions the scripts call by global name.
const NATIVE_GLOBALS: [&str; 9] = [
    "REGISTER_GOAL",
    "REGISTER_LOGIC_FUNC",
    "REGISTER_GOAL_NO_SUB_GOAL",
    "REGISTER_GOAL_NO_UPDATE",
    "REGISTER_GOAL_NO_INTERUPT",
    "REGISTER_GOAL_UPDATE_TIME",
    "REGISTER_GOAL_USE_AVOID_CHR",
    "REGISTER_DBG_GOAL_PARAM",
    "ENABLE_COMBO_ATK_CANCEL",
];

/// Reads a `.luainfo` table: goal or logic id to the name its script callbacks use.
pub fn read_luainfo(data: &[u8]) -> HashMap<i32, String> {
    let mut out = HashMap::new();
    if data.len() < 16 || &data[..4] != b"LUAI" {
        return out;
    }
    let count = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
    let utf16 = |off: usize| -> String {
        let mut units = Vec::new();
        let mut p = off;
        while p + 1 < data.len() {
            let u = u16::from_le_bytes([data[p], data[p + 1]]);
            if u == 0 {
                break;
            }
            units.push(u);
            p += 2;
        }
        String::from_utf16_lossy(&units)
    };
    for i in 0..count {
        let e = 16 + i * 24;
        if e + 24 > data.len() {
            break;
        }
        let id = i32::from_le_bytes(data[e..e + 4].try_into().unwrap());
        let off = u64::from_le_bytes(data[e + 8..e + 16].try_into().unwrap()) as usize;
        out.insert(id, utf16(off));
    }
    out
}

/// The game's AI for one character.
pub struct AiBrain {
    vm: Vm,
    pub state: AiState,
    /// How many log lines [`AiBrain::drain_log`] has returned.
    log_read: usize,
}

impl AiBrain {
    /// Loads `aicommon.luabnd` and the given per-enemy scripts from an extracted cache, for the
    /// think params `think` (the soldier: NpcThinkParam 10100000, logic and battle goal 101000).
    ///
    /// `map_dir` is the map's unpacked luabnd (such as `script/m11_00_00_00.luabnd.d`); only
    /// files named `<logic_id>_*.lua` and `<battle_goal_id>_*.lua` are loaded from it.
    pub fn load(
        cache: &Path,
        map_luabnd: &str,
        think: ThinkParams,
        seed: u64,
    ) -> Result<Self, AiLoadError> {
        let common = cache.join("raw/script/aicommon.luabnd.d");
        let map = cache.join("raw/script").join(map_luabnd);
        let read = |p: &Path| std::fs::read(p).map_err(|e| AiLoadError::Io(p.to_owned(), e));
        let mut names = read_luainfo(&read(&common.join("aiCommon.luainfo"))?);
        if let Ok(entries) = std::fs::read_dir(&map) {
            for e in entries.flatten() {
                if e.path().extension().is_some_and(|x| x == "luainfo") {
                    names.extend(read_luainfo(&read(&e.path())?));
                }
            }
        }
        let mut files: Vec<PathBuf> = FIRST.iter().map(|f| common.join(f)).collect();
        let mut rest: Vec<PathBuf> = std::fs::read_dir(&common)
            .map_err(|e| AiLoadError::Io(common.clone(), e))?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "lua"))
            .filter(|p| !FIRST.iter().any(|f| p.ends_with(f)))
            .collect();
        rest.sort();
        files.extend(rest);
        for id in [think.logic_id, think.battle_goal_id] {
            for kind in ["logic", "battle"] {
                let p = map.join(format!("{id:06}_{kind}.lua"));
                if p.is_file() && !files.contains(&p) {
                    files.push(p);
                }
            }
        }
        let mut vm = Vm::new();
        for f in NATIVE_GLOBALS {
            vm.set_global(f, Value::Host(f.into()));
        }
        let mut state = AiState::new(think, names, seed);
        for f in &files {
            let data = read(f)?;
            let name = f.file_name().unwrap().to_string_lossy().into_owned();
            let proto = load_chunk(&data).map_err(|e| AiLoadError::Lua(name.clone(), e))?;
            vm.exec_chunk(&mut state, proto)
                .map_err(|e| AiLoadError::Lua(name.clone(), e))?;
        }
        Ok(Self {
            vm,
            state,
            log_read: 0,
        })
    }

    /// Runs one AI tick and returns the control for the character's next step.
    pub fn think(&mut self, world: AiWorld) -> NpcControl {
        tick(&mut self.vm, &mut self.state, world);
        let r = &self.state.request;
        let mut c = NpcControl {
            action: r.action,
            move_level: r.move_level,
            move_yaw: r.move_yaw,
            face_yaw: r.face_yaw,
            ..NpcControl::default()
        };
        if self.state.battle {
            c.refs.insert(REF_AI_BATTLE);
        }
        c
    }

    /// Log lines added since the last call.
    pub fn drain_log(&mut self) -> Vec<AiEvent> {
        let out = self.state.log[self.log_read..].to_vec();
        self.log_read = self.state.log.len();
        out
    }

    /// The active goal chain from the top, as names.
    pub fn goal_stack(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut id = self.state.top;
        while let Some(&next) = self.state.goal(id).subgoals.front() {
            out.push(self.state.name(self.state.goal(next).kind));
            id = next;
        }
        out
    }
}

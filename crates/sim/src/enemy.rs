//! Which enemy a duel loads: its character model, NpcParam and NpcThinkParam rows and the map
//! script bundle its Lua AI lives in.
//!
//! The rows are the ones the map layouts place: the soldier as in `m11_00_00_00`, the Great
//! Shinobi Owl as part `c5060_0000` of `m11_01_00_00` (the Ashina Castle roof). See
//! `docs/OWL.md`.

/// One enemy as the duel loads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnemySpec {
    /// Character id, such as `c1010`: model, animations, HKS script and behaviour graph.
    pub chr: String,
    /// NpcParam row: HP, posture, deathblow marks, hit radius, behaviour variation.
    pub npc_param: i32,
    /// NpcThinkParam row: the logic and battle goal of its Lua AI.
    pub think: i32,
    /// The unpacked map script bundle that holds its `<id>_logic.lua` and `<id>_battle.lua`.
    pub ai_bundle: String,
    /// The AI starts in the battle target state instead of noticing Wolf first. Boss fights
    /// begin after a cutscene with the boss already fighting; NpcParam 50600000 also carries
    /// the find/battle state SpEffect 200004 as a resident effect (that this is why is
    /// inferred, see `docs/OWL.md`).
    pub starts_in_battle: bool,
}

impl EnemySpec {
    /// The Ashina soldier with a one-handed sword (NpcParam and NpcThinkParam 10100000).
    pub fn soldier() -> Self {
        Self {
            chr: "c1010".into(),
            npc_param: 10_100_000,
            think: 10_100_000,
            ai_bundle: "m11_00_00_00.luabnd.d".into(),
            starts_in_battle: false,
        }
    }

    /// The Great Shinobi Owl on the Ashina Castle roof (忍軍の長, "chief of the shinobi army"):
    /// NpcParam 50600000, NpcThinkParam 50601000 (logic 506000, battle goal 506001), as part
    /// `c5060_0000` of `m11_01_00_00` places him.
    pub fn owl() -> Self {
        Self {
            chr: "c5060".into(),
            npc_param: 50_600_000,
            think: 50_601_000,
            ai_bundle: "m11_01_00_00.luabnd.d".into(),
            starts_in_battle: true,
        }
    }

    /// The enemy for a command-line name: a character id (`c1010`, `c5060`) or `soldier` /
    /// `owl`. `c5070` is the Owl's pet bird, not the Owl, and is refused with a hint.
    pub fn by_name(name: &str) -> Result<Self, String> {
        match name.to_ascii_lowercase().as_str() {
            "c1010" | "soldier" => Ok(Self::soldier()),
            "c5060" | "owl" => Ok(Self::owl()),
            "c5070" => Err(
                "c5070 is the Owl's bird (長の梟, NpcParam 50700000, placed only in \
                 the Hirata Estate fight); the Great Shinobi Owl is c5060"
                    .into(),
            ),
            other => Err(format!(
                "unknown enemy {other}: known are c1010 (soldier), c5060 (owl)"
            )),
        }
    }

    /// The enemy's name for the HUD.
    pub fn display_name(&self) -> &'static str {
        match self.chr.as_str() {
            "c1010" => "Ashina Soldier",
            "c5060" => "Great Shinobi Owl",
            _ => "Enemy",
        }
    }

    /// The model number ThrowParam's `defChrId` uses (`c1010` -> 1010).
    pub fn throw_chr(&self) -> i32 {
        self.chr.trim_start_matches('c').parse().unwrap_or(0)
    }
}

impl Default for EnemySpec {
    fn default() -> Self {
        Self::soldier()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_resolve() {
        assert_eq!(EnemySpec::by_name("owl").unwrap().chr, "c5060");
        assert_eq!(EnemySpec::by_name("C1010").unwrap(), EnemySpec::soldier());
        assert!(EnemySpec::by_name("c5070").unwrap_err().contains("c5060"));
        assert_eq!(EnemySpec::owl().throw_chr(), 5060);
    }
}

//! The actions a plan may name, per boss, and how a plan becomes an act on his battle goal.
//!
//! Each entry is one of the boss's own action ids (the animation ids his Lua acts request),
//! with the approach distance and the AI-number change of the act that opens with it in his
//! battle script (`506001_battle.lua` for the Owl). A plan runs exactly like one of his acts:
//! an approach (`Approach_Act_Flex`), then the attacks as `AttackTunableSpin` (a single one) or
//! `ComboAttackTunableSpin` / `ComboRepeat` / `ComboFinal` (a chain), so animations, TAE
//! cancels, combo windows and reactions go through the same goals as his own choices.

use crate::lua_ai::{ActChoice, ActGoal, NumberOnActivate};

/// What kind of action an id is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    Attack,
    /// A step (`SpinStep`); never approaches first.
    Step,
}

/// One action a plan may name.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BossAction {
    pub id: i32,
    pub label: &'static str,
    pub kind: ActionKind,
    /// Approach until within this many metres (centre to centre, before the scripts subtract
    /// the hit radius) when the action opens a plan; 0 means no approach.
    pub approach: f32,
    /// AI number 5 (the pressure score `Kengeki_Activate` reads) on activation, as his act
    /// sets it.
    pub pressure: NumberOnActivate,
}

/// A boss's action list and the battle-goal acts that stay reactions.
#[derive(Debug, Clone, Copy)]
pub struct BossActions {
    /// Character id (`c5060`).
    pub chr: &'static str,
    pub actions: &'static [BossAction],
    /// Act indices of his battle goal that are reactions: when his script weighs any of them,
    /// its own pick stands and the plan waits (see `lua_ai::ActHook`).
    pub reflex_acts: &'static [i32],
    /// The most actions one plan may hold.
    pub max_plan: usize,
}

const fn attack(id: i32, label: &'static str, approach: f32, pressure: f64) -> BossAction {
    BossAction {
        id,
        label,
        kind: ActionKind::Attack,
        approach,
        pressure: NumberOnActivate::Add(5, pressure),
    }
}

const fn step(id: i32, label: &'static str) -> BossAction {
    BossAction {
        id,
        label,
        kind: ActionKind::Step,
        approach: 0.0,
        pressure: NumberOnActivate::Set(5, 0.0),
    }
}

/// The Great Shinobi Owl (c5060) in phase 1. Approach distances and pressure from the act that
/// uses each id in `506001_battle.lua` (Act01 to Act15, Act30); descriptions from his TAE and
/// AtkParam rows (`docs/OWL.md`). The tool acts (3010 firecracker, 3017 explosive slash, 3030
/// poison balls, 3069 smoke screen) were added with their bullets; their approach distances are
/// ours, read from where their bullets land (the reach of each in `docs/OWL.md`, "Tools"), and
/// their pressure is 0 until the acts that use them are read. Left out: warps, grabs, phase-2
/// acts other than the poison and the deflect counters, which stay reactions.
pub const OWL: BossActions = BossActions {
    chr: "c5060",
    actions: &[
        attack(3000, "kesa slash, combo opener", 4.8, 2.0),
        attack(3001, "second slash", 5.8, 7.0),
        attack(3002, "third slash, combo finisher", 4.8, 10.0),
        attack(3003, "jumping forward roll, slash down", 10.7, 9.0),
        attack(3005, "front kick (posture only)", 2.8, 1.0),
        attack(3009, "shuriken", 9.7, 1.0),
        attack(
            3010,
            "firecrackers scattered in front (unblockable bangs)",
            4.0,
            0.0,
        ),
        attack(3011, "rushing slash", 11.0, 1.0),
        attack(3014, "second shuriken", 9.7, 2.0),
        attack(3015, "rushing slash curving left", 9.0, 5.0),
        attack(3017, "explosive slash (fire)", 5.0, 0.0),
        attack(3018, "shuriken then dash slash", 11.0, 9.0),
        attack(
            3030,
            "poison balls, three rows of poison floor (phase 2)",
            5.0,
            0.0,
        ),
        attack(3031, "recovery-sealing throw", 5.5, 6.0),
        attack(3032, "recovery-sealing throw follow-up", 9.0, 8.0),
        attack(3063, "rushing follow-up, two hits", 4.8, 10.0),
        attack(3069, "smoke screen (blinds, breaks lock-on)", 5.5, 0.0),
        step(5201, "back step"),
        step(5202, "side step (his act picks it with Wolf on his right)"),
        step(5203, "side step (his act picks it with Wolf on his left)"),
    ],
    // Act21 turns to punish a heal or face a Wolf behind him; Act40 answers SpEffect 110125.
    reflex_acts: &[21, 40],
    max_plan: 3,
};

/// Why a plan was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    Empty,
    TooLong(usize),
    Unknown(i32),
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanError::Empty => write!(f, "empty plan"),
            PlanError::TooLong(n) => write!(f, "plan of {n} actions is too long"),
            PlanError::Unknown(id) => write!(f, "unknown action {id}"),
        }
    }
}

impl BossActions {
    pub fn get(&self, id: i32) -> Option<&BossAction> {
        self.actions.iter().find(|a| a.id == id)
    }

    pub fn ids(&self) -> Vec<i32> {
        self.actions.iter().map(|a| a.id).collect()
    }

    /// The boss's actions for a character id, if a table exists.
    pub fn for_chr(chr: &str) -> Option<&'static BossActions> {
        (chr == OWL.chr).then_some(&OWL)
    }

    /// Turns a plan into an act on the battle goal. `hit_radius` is the boss's NpcParam
    /// `hitRadius`, which his acts subtract from every distance.
    pub fn act_for(&self, plan: &[i32], hit_radius: f32) -> Result<ActChoice, PlanError> {
        if plan.is_empty() {
            return Err(PlanError::Empty);
        }
        if plan.len() > self.max_plan {
            return Err(PlanError::TooLong(plan.len()));
        }
        let actions = plan
            .iter()
            .map(|&id| self.get(id).copied().ok_or(PlanError::Unknown(id)))
            .collect::<Result<Vec<_>, _>>()?;
        let first = actions[0];
        let approach = (first.kind == ActionKind::Attack && first.approach > 0.0)
            .then(|| f64::from(first.approach - hit_radius).max(0.5));
        let last = actions.len() - 1;
        let goals = actions
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let id = f64::from(a.id);
                // TARGET_ENE_0 = 0, AI_DIR_TYPE_B = 2 (ai_define.lua); 9999 is the scripts'
                // "any distance" success range.
                let (goal, params): (&'static str, Vec<f64>) = match a.kind {
                    ActionKind::Step => ("GOAL_COMMON_SpinStep", vec![id, 0.0, 0.0, 2.0, 0.0]),
                    ActionKind::Attack if last == 0 => (
                        "GOAL_COMMON_AttackTunableSpin",
                        vec![id, 0.0, 9999.0, 0.0, 0.0, 0.0, 0.0],
                    ),
                    ActionKind::Attack if i == 0 => (
                        "GOAL_COMMON_ComboAttackTunableSpin",
                        vec![id, 0.0, 9999.0, 0.0, 0.0, 0.0, 0.0],
                    ),
                    ActionKind::Attack if i == last => {
                        ("GOAL_COMMON_ComboFinal", vec![id, 0.0, 9999.0, 0.0, 0.0])
                    }
                    ActionKind::Attack => ("GOAL_COMMON_ComboRepeat", vec![id, 0.0, 9999.0, 0.0]),
                };
                ActGoal {
                    goal,
                    life: if a.kind == ActionKind::Step {
                        3.0
                    } else {
                        10.0
                    },
                    params,
                    number: Some(a.pressure),
                }
            })
            .collect();
        Ok(ActChoice { approach, goals })
    }
}

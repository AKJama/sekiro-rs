//! The actions a plan may name, per boss, and how a plan becomes an act on his battle goal.
//!
//! Each entry is one of the boss's own action ids (the animation ids his Lua acts request),
//! with the approach distance and the AI-number change of the act that opens with it in his
//! battle script (`506001_battle.lua` for the Owl, `101000_battle.lua` for the Ashina soldier).
//! A plan runs exactly like one of his acts: an approach (`Approach_Act_Flex`), then the
//! attacks as `AttackTunableSpin` (a single one) or `ComboAttackTunableSpin` / `ComboRepeat` /
//! `ComboFinal` (a chain), so animations, TAE cancels, combo windows and reactions go through
//! the same goals as his own choices.

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
    /// The AI number his act sets when it runs: for the Owl, number 5, the pressure score
    /// `Kengeki_Activate` reads; for the soldier, number 5 marks his rushes as used. `None`
    /// when the act sets none.
    pub pressure: Option<NumberOnActivate>,
}

/// A named chain of actions offered to hosted controllers as one choice.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Combo {
    pub ids: &'static [i32],
    pub label: &'static str,
}

impl Combo {
    /// The choice value, such as `3000+3001+3002`.
    pub fn value(&self) -> String {
        self.ids
            .iter()
            .map(i32::to_string)
            .collect::<Vec<_>>()
            .join("+")
    }
}

/// A boss's action list and the battle-goal acts that stay reactions.
#[derive(Debug, Clone, Copy)]
pub struct BossActions {
    /// Character id (`c5060`).
    pub chr: &'static str,
    /// Who he is, for the hosted instructions ("the Great Shinobi Owl").
    pub name: &'static str,
    pub actions: &'static [BossAction],
    /// Act indices of his battle goal that are reactions: when his script weighs any of them,
    /// its own pick stands and the plan waits (see `lua_ai::ActHook`).
    pub reflex_acts: &'static [i32],
    /// The most actions one plan may hold.
    pub max_plan: usize,
    /// Chains offered to hosted controllers as one choice, each of at most `max_plan` ids.
    pub combos: &'static [Combo],
    /// His back steps (the habit "what Wolf does after a back step" starts at these).
    pub back_steps: &'static [i32],
    /// The scripted test controller's opener, and the plan it switches to once the opener's
    /// first action has been deflected twice (`ScriptedBrain::for_actions`).
    pub scripted: [&'static [i32]; 2],
}

const fn attack(id: i32, label: &'static str, approach: f32, pressure: f64) -> BossAction {
    BossAction {
        id,
        label,
        kind: ActionKind::Attack,
        approach,
        pressure: Some(NumberOnActivate::Add(5, pressure)),
    }
}

/// An attack whose act sets no AI number.
const fn plain(id: i32, label: &'static str, approach: f32) -> BossAction {
    BossAction {
        id,
        label,
        kind: ActionKind::Attack,
        approach,
        pressure: None,
    }
}

/// An attack whose act sets AI number `slot` to `value`.
const fn setting(id: i32, label: &'static str, approach: f32, slot: i32, value: f64) -> BossAction {
    BossAction {
        pressure: Some(NumberOnActivate::Set(slot, value)),
        ..plain(id, label, approach)
    }
}

const fn step(id: i32, label: &'static str) -> BossAction {
    BossAction {
        id,
        label,
        kind: ActionKind::Step,
        approach: 0.0,
        pressure: Some(NumberOnActivate::Set(5, 0.0)),
    }
}

/// A step whose act sets no AI number.
const fn plain_step(id: i32, label: &'static str) -> BossAction {
    BossAction {
        pressure: None,
        ..step(id, label)
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
    name: "the Great Shinobi Owl",
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
    combos: &[Combo {
        ids: &[3000, 3001, 3002],
        label: "three-slash combo",
    }],
    back_steps: &[5201],
    // Kick (3005); after two deflected kicks, the recovery-sealing throw (3031).
    scripted: [&[3005], &[3031]],
};

/// The Ashina soldier with a one-handed sword (c1010, NpcThinkParam 10100000, battle goal
/// 101000). Approach distances, combos and AI numbers from the act that uses each id in
/// `101000_battle.lua` (Act01 to Act09, Act11, Act14, Act15, Act24); descriptions from his
/// BehaviorParam names (the judge ids in his TAE) and AtkParam rows (10100xxx: the thrust 3013
/// cannot be guarded, the low sweep 3014 can be neither guarded nor deflected, the kick and the
/// pounces deal 1 HP).
///
/// - 3001 and 3007 only follow 3000 and 3005 in his acts; as openers they use their act's
///   distance (ours).
/// - The stone throw's act (Act07) never approaches, so neither does a plan opening with it.
/// - His acts set AI number 5 to 1 when he rushes (3010, 3011): his script only weighs the
///   rushes from afar while it is 0 under SpEffect 200050.
/// - Left out: 3004 and 3006 (Act04) and the side steps 5202 and 5203 (Act22), which his
///   script names but his TAE (`c1010.tae`) has no clips for; 3013 toward an event point
///   (Act10) and 3030 (Act12, a tutorial dodge); the deflect counters (3050 to 3076,
///   `Kengeki*`), which stay reactions.
pub const SOLDIER: BossActions = BossActions {
    chr: "c1010",
    name: "an Ashina soldier with a one-handed sword",
    actions: &[
        plain(3000, "sweeping slash, combo opener", 2.5),
        plain(3001, "second sweeping slash (follows 3000)", 2.5),
        plain(3002, "running sweeping slash", 7.0),
        plain(3003, "leaping slash", 7.1),
        plain(3005, "kick (posture only)", 3.7),
        plain(3007, "rising slash (follows the kick)", 3.7),
        plain(3008, "flurry of three slashes", 4.7),
        plain(3009, "stone throw", 0.0),
        setting(3010, "pouncing leap (posture only)", 5.0, 5, 1.0),
        setting(
            3011,
            "running lariat, then a pounce (posture only)",
            7.6,
            5,
            1.0,
        ),
        plain(3012, "left sweep, then an overhead slam", 3.5),
        plain(
            3013,
            "perilous thrust (cannot be guarded; deflect or counter it)",
            5.0,
        ),
        plain(
            3014,
            "perilous low sweep (cannot be guarded or deflected; jump)",
            3.5,
        ),
        plain_step(5201, "back step"),
        plain_step(5211, "short back step (his act uses it within 4 m)"),
    ],
    // Act23 strafes (only with Wolf down, or in the tutorial), Act25 backs away (SpEffect
    // 3170200 on Wolf), Act27 waits and strafes (no path to Wolf, Wolf hanging, or SpEffect
    // 3101540): reactions his script weighs before any distance band.
    reflex_acts: &[23, 25, 27],
    max_plan: 3,
    combos: &[
        Combo {
            ids: &[3000, 3001],
            label: "both sweeping slashes",
        },
        Combo {
            ids: &[3005, 3007],
            label: "kick, then rising slash",
        },
    ],
    back_steps: &[5201, 5211],
    // Both sweeping slashes; after two deflected openers, the kick and rising slash.
    scripted: [&[3000, 3001], &[3005, 3007]],
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
        [&OWL, &SOLDIER].into_iter().find(|a| a.chr == chr)
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
                    number: a.pressure,
                }
            })
            .collect();
        Ok(ActChoice { approach, goals })
    }
}

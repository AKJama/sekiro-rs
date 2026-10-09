//! The goal machinery around the Lua scripts: goal objects, the AI object's methods, and the
//! engine-side ("native") leaf goals.
//!
//! The scripts drive a tree of goals. Each goal has a numeric kind (`GOAL_COMMON_Attack` = 2100,
//! the soldier's battle goal 101000, ...), a lifetime, numbered parameters and a queue of
//! subgoals that run front first. A goal kind is implemented in one of three ways:
//!
//! - a *table goal*: registered by `RegisterTableGoal` (a Lua table with `Activate`, `Update`,
//!   ... called through the script helpers `ActivateTableGoal`, `UpdateTableGoal`, ...);
//! - a *script goal*: named in the `.luainfo` table (2100 is `Attack`), whose callbacks are the
//!   globals `Attack_Activate`, `Attack_Update`, `Attack_Terminate`;
//! - a *native goal*: implemented by the engine (waiting, moving, guarding, attacking). These
//!   are re-implemented here in [`NativeGoal`].
//!
//! See `docs/AI.md` for what is engine behaviour read from the executable and what is our
//! reconstruction.

use std::collections::{HashMap, HashSet, VecDeque};
use std::f32::consts::PI;
use std::rc::Rc;

use super::lua50::{Host, LuaError, LuaResult, Obj, Value, Vm};
use super::world::{AiActor, AiWorld, ThinkParams};

pub const RESULT_FAILED: i32 = -1;
pub const RESULT_CONTINUE: i32 = 0;
pub const RESULT_SUCCESS: i32 = 1;

const OBJ_AI: u8 = 0;
const OBJ_GOAL: u8 = 1;

/// The engine's leaf goals, by goal id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeGoal {
    /// 2000 Wait, 2 Stay, 5000 WaitCancelTiming: idle until the lifetime ends.
    Wait,
    /// 2019 MoveToSomewhere and its variants: move until within a distance of a target.
    MoveTo,
    /// 2101 Guard: hold the guard action for the lifetime.
    Guard,
    /// 2017 SidewayMove and variants: strafe around the target.
    Sideway,
    /// 2018 KeepDist: move until the distance is inside a range.
    KeepDist,
    /// 2016 LeaveTarget: back away until a distance.
    Leave,
    /// 2200 CommonAttack, 2020 SpinStep, 2113 Parry, 2107 ApproachStep: request an action and
    /// wait for its animation.
    Action,
    /// Anything else the engine implements: succeeds at once (logged).
    Unsupported,
}

impl NativeGoal {
    pub fn for_id(id: i32) -> Option<NativeGoal> {
        Some(match id {
            2000 | 2 | 5000 => NativeGoal::Wait,
            2019 | 2013 | 2025 | 2026 | 2027 | 2023 | 2024 | 2028 | 4 => NativeGoal::MoveTo,
            2101 => NativeGoal::Guard,
            2017 | 2030 | 2036 => NativeGoal::Sideway,
            2018 => NativeGoal::KeepDist,
            2016 => NativeGoal::Leave,
            2200 | 2020 | 2113 | 2107 => NativeGoal::Action,
            2031..=2037 | 2500..=2504 | 5100 => NativeGoal::Unsupported,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalImpl {
    Table,
    Script,
    Native(NativeGoal),
    /// No implementation found: succeeds when its subgoals are done.
    Empty,
}

/// A goal instance.
#[derive(Debug, Clone)]
pub struct Goal {
    pub kind: i32,
    pub imp: GoalImpl,
    /// Remaining lifetime in seconds; negative means unlimited.
    pub life: f32,
    pub initial_life: f32,
    pub params: Vec<Value>,
    pub subgoals: VecDeque<u32>,
    pub activated: bool,
    pub numbers: HashMap<i32, f64>,
    pub timers: HashMap<i32, f32>,
    pub parent: Option<u32>,
    /// Native goal state.
    pub phase: u8,
    pub elapsed: f32,
    pub started_anim: bool,
}

impl Goal {
    fn param(&self, i: usize) -> f64 {
        self.params.get(i).and_then(Value::num).unwrap_or(0.0)
    }
    fn param_bool(&self, i: usize) -> bool {
        self.params.get(i).is_some_and(Value::truthy)
    }
}

/// One line of the AI log.
#[derive(Debug, Clone, PartialEq)]
pub enum AiEvent {
    Logic(i32),
    GoalStart {
        depth: usize,
        kind: i32,
        name: String,
    },
    GoalEnd {
        kind: i32,
        name: String,
        result: i32,
    },
    Action(i32),
    Interrupt(i32),
    Unknown(String),
    Error(String),
}

/// What the native goals asked for this tick.
#[derive(Debug, Clone, Default)]
pub struct Request {
    pub action: i32,
    pub move_level: f32,
    pub move_yaw: Option<f32>,
    pub face_yaw: Option<f32>,
}

/// Registered goal and logic information plus all per-character AI state. Implements the
/// scripts' [`Host`].
pub struct AiState {
    pub think: ThinkParams,
    pub names: HashMap<i32, String>,
    pub table_goals: HashSet<i32>,
    pub combo_cancel: HashSet<i32>,
    pub goals: Vec<Option<Goal>>,
    pub top: u32,
    pub replan: bool,
    pub world: AiWorld,
    pub request: Request,
    pub numbers: HashMap<i32, f64>,
    pub string_numbers: HashMap<String, f64>,
    pub timers: HashMap<i32, f32>,
    /// Seconds since each action id was last requested.
    pub action_age: HashMap<i32, f32>,
    pub time: f32,
    pub rng: u64,
    pub log: Vec<AiEvent>,
    pub unknown: HashSet<String>,
    pub battle: bool,
    /// AI target state (AI_TARGET_STATE: 0 none, 1 caution, 2 find, 3 battle) and its value at
    /// the previous logic run; the engine keeps both (state object +0x158 and +0x15C).
    pub target_state: i32,
    pub prev_target_state: i32,
    /// The interrupt being dispatched (INTERUPT_* id), answered by `IsInterupt`.
    pub interrupt: Option<i32>,
    /// Edge detection for the world's interrupt sources.
    last_parry_timing: bool,
}

impl AiState {
    pub fn new(think: ThinkParams, names: HashMap<i32, String>, seed: u64) -> Self {
        let mut s = Self {
            think,
            names,
            table_goals: HashSet::new(),
            combo_cancel: HashSet::new(),
            goals: Vec::new(),
            top: 0,
            replan: false,
            world: AiWorld::default(),
            request: Request::default(),
            numbers: HashMap::new(),
            string_numbers: HashMap::new(),
            timers: HashMap::new(),
            action_age: HashMap::new(),
            time: 0.0,
            rng: seed | 1,
            log: Vec::new(),
            unknown: HashSet::new(),
            battle: false,
            target_state: 0,
            prev_target_state: 0,
            interrupt: None,
            last_parry_timing: false,
        };
        s.top = s.new_goal(0, -1.0, Vec::new(), None);
        s
    }

    pub fn name(&self, kind: i32) -> String {
        self.names
            .get(&kind)
            .cloned()
            .unwrap_or_else(|| kind.to_string())
    }

    pub fn goal(&self, id: u32) -> &Goal {
        self.goals[id as usize].as_ref().expect("live goal")
    }

    pub fn goal_mut(&mut self, id: u32) -> &mut Goal {
        self.goals[id as usize].as_mut().expect("live goal")
    }

    fn new_goal(&mut self, kind: i32, life: f32, params: Vec<Value>, parent: Option<u32>) -> u32 {
        let imp = if let Some(n) = NativeGoal::for_id(kind) {
            GoalImpl::Native(n)
        } else if self.table_goals.contains(&kind) {
            GoalImpl::Table
        } else if self.names.contains_key(&kind) {
            GoalImpl::Script
        } else {
            GoalImpl::Empty
        };
        let g = Goal {
            kind,
            imp,
            life,
            initial_life: life,
            params,
            subgoals: VecDeque::new(),
            activated: false,
            numbers: HashMap::new(),
            timers: HashMap::new(),
            parent,
            phase: 0,
            elapsed: 0.0,
            started_anim: false,
        };
        if let Some(i) = self.goals.iter().position(Option::is_none) {
            self.goals[i] = Some(g);
            i as u32
        } else {
            self.goals.push(Some(g));
            (self.goals.len() - 1) as u32
        }
    }

    fn drop_goal(&mut self, id: u32) {
        let subs: Vec<u32> = self.goal(id).subgoals.iter().copied().collect();
        for s in subs {
            self.drop_goal(s);
        }
        self.goals[id as usize] = None;
    }

    fn random(&mut self) -> f64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 11) as f64 / (1u64 << 53) as f64
    }

    fn actor(&self, target: i32) -> Option<AiActor> {
        match target {
            -1 => Some(self.world.me.clone()),
            0 | 21 => self.world.target.clone(),
            _ => None,
        }
    }

    /// Position of a target or point id.
    fn position(&self, target: i32) -> Option<[f32; 3]> {
        match target {
            -1 => Some(self.world.me.position),
            0 | 21 => self.world.target.as_ref().map(|t| t.position),
            100 | 111 | 121 | 125 => Some(self.world.home),
            _ => None,
        }
    }

    fn dist(&self, target: i32) -> f32 {
        match self.position(target) {
            Some(p) => {
                let m = self.world.me.position;
                ((p[0] - m[0]).powi(2) + (p[2] - m[2]).powi(2)).sqrt()
            }
            None => 9999.0,
        }
    }

    fn yaw_to(&self, target: i32) -> Option<f32> {
        let p = self.position(target)?;
        let m = self.world.me.position;
        let (dx, dz) = (p[0] - m[0], p[2] - m[2]);
        if dx == 0.0 && dz == 0.0 {
            return None;
        }
        Some((-dx).atan2(-dz))
    }

    /// Signed angle in degrees from my facing to the target, positive to the left.
    fn angle_to(&self, target: i32) -> f32 {
        match self.yaw_to(target) {
            Some(y) => wrap(y - self.world.me.yaw).to_degrees(),
            None => 0.0,
        }
    }

    /// Whether `target` lies inside a cone of `angle` degrees centred on direction `dir`
    /// (AI_DIR_TYPE: 1 front, 2 back, 3 left, 4 right; 0 counts as front).
    fn inside(&self, target: i32, dir: i32, angle: f32) -> bool {
        let centre = match dir {
            2 | 6 => 180.0,
            3 | 7 => 90.0,
            4 | 8 => -90.0,
            _ => 0.0,
        };
        wrap((self.angle_to(target) - centre).to_radians())
            .to_degrees()
            .abs()
            <= angle * 0.5
    }

    fn has_effect(&self, target: i32, id: i32) -> bool {
        // The engine marks the AI state with SpEffects: 200001 non-combat caution, 200002
        // combat caution, 200004 find/battle (SpEffectParam names).
        if target == -1 && id == 200004 && self.target_state >= 2 {
            return true;
        }
        self.actor(target)
            .is_some_and(|a| a.sp_effects.contains(&id))
    }

    fn excel(&self, idx: i32) -> f64 {
        const NAMES: [&str; 31] = [
            "",
            "maxBackhomeDist",
            "backhomeDist",
            "backhomeBattleDist",
            "nonBattleActLife",
            "BattleStartDist",
            "bMoveOnHearSound",
            "idAttackCannotMove",
            "battleGoalID",
            "BackHome_LookTargetTime",
            "BackHome_LookTargetDist",
            "BackHomeLife_OnHitEneWal",
            "callHelp_IsCall",
            "callHelp_IsReply",
            "callHelp_MyPeerId",
            "callHelp_CallPeerId",
            "callHelp_DelayTime",
            "callHelp_CallActionId",
            "callHelp_ReplyBehaviorType",
            "callHelp_ForgetTimeByArrival",
            "callHelp_MinWaitTime",
            "callHelp_MaxWaitTime",
            "callHelp_ReplyActionId",
            "thinkAttr_doAdmirer",
            "goalAction_ToDisappear",
            "goalAction_ToCaution",
            "goalAction_ToCautionImportant",
            "goalAction_ToFind",
            "changeStateAction_ToNormal",
            "goalAction_ToCautionIndicationTarget",
            "goalAction_ToCautionCorpseTarget",
        ];
        let name = NAMES.get(idx as usize).copied().unwrap_or("");
        self.think.fields.get(name).copied().unwrap_or(0.0)
    }

    fn log(&mut self, e: AiEvent) {
        self.log.push(e);
    }
}

fn wrap(a: f32) -> f32 {
    let mut a = a % (2.0 * PI);
    if a > PI {
        a -= 2.0 * PI;
    } else if a < -PI {
        a += 2.0 * PI;
    }
    a
}

fn num(v: f64) -> Vec<Value> {
    vec![Value::Num(v)]
}

fn boolean(b: bool) -> Vec<Value> {
    vec![Value::Bool(b)]
}

fn goal_obj(id: u32) -> Value {
    Value::Obj(Obj { kind: OBJ_GOAL, id })
}

pub fn ai_obj() -> Value {
    Value::Obj(Obj {
        kind: OBJ_AI,
        id: 0,
    })
}

impl Host for AiState {
    fn call_host(&mut self, _vm: &mut Vm, name: &str, args: &[Value]) -> LuaResult<Vec<Value>> {
        let n = |i: usize| args.get(i).and_then(Value::num).unwrap_or(0.0);
        match name {
            "REGISTER_GOAL" => {
                self.table_goals.insert(n(0) as i32);
                if let Some(s) = args.get(1).and_then(Value::as_str) {
                    self.names
                        .entry(n(0) as i32)
                        .or_insert_with(|| s.to_owned());
                }
            }
            "ENABLE_COMBO_ATK_CANCEL" => {
                self.combo_cancel.insert(n(0) as i32);
            }
            "REGISTER_LOGIC_FUNC"
            | "REGISTER_GOAL_NO_SUB_GOAL"
            | "REGISTER_GOAL_NO_UPDATE"
            | "REGISTER_GOAL_NO_INTERUPT"
            | "REGISTER_GOAL_UPDATE_TIME"
            | "REGISTER_GOAL_USE_AVOID_CHR"
            | "REGISTER_DBG_GOAL_PARAM"
            | "print" => {}
            "loadstring" => {
                // Only `GOAL_COMMON_If` builds code at run time, always the same shape:
                // "return function (arg) OnIf_<id>(arg.ai, arg.goal, arg.codeNo) end".
                let src = args.first().and_then(Value::as_str).unwrap_or("");
                let Some(start) = src.find("OnIf_") else {
                    return Ok(vec![Value::Nil, Value::str("loadstring unsupported")]);
                };
                let end = src[start..].find('(').map_or(src.len(), |e| start + e);
                return Ok(vec![Value::Host(Rc::from(format!(
                    "ifchunk:{}",
                    &src[start..end]
                )))]);
            }
            _ if name.starts_with("ifchunk:") => {
                return Ok(vec![Value::Host(Rc::from(format!(
                    "ifcall:{}",
                    &name[8..]
                )))]);
            }
            _ if name.starts_with("ifcall:") => {
                let Some(Value::Table(t)) = args.first() else {
                    return Ok(vec![]);
                };
                let (ai, goal, code) = {
                    let t = t.borrow();
                    (t.get_str("ai"), t.get_str("goal"), t.get_str("codeNo"))
                };
                let f = _vm.get_global(&name[7..]);
                if !f.is_nil() {
                    return _vm.call(self, &f, &[ai, goal, code]);
                }
            }
            _ => {
                if self.unknown.insert(name.to_owned()) {
                    self.log(AiEvent::Unknown(name.to_owned()));
                }
            }
        }
        Ok(vec![])
    }

    fn call_method(
        &mut self,
        _vm: &mut Vm,
        obj: Obj,
        name: &str,
        args: &[Value],
    ) -> LuaResult<Vec<Value>> {
        // args[0] is the object itself (method-call syntax).
        let a = &args[1.min(args.len())..];
        if obj.kind == OBJ_GOAL {
            return self.goal_method(obj.id, name, a);
        }
        self.ai_method(name, a)
    }
}

impl AiState {
    fn add_goal(&mut self, parent: u32, front: bool, a: &[Value]) -> Vec<Value> {
        let n = |i: usize| a.get(i).and_then(Value::num).unwrap_or(0.0);
        let kind = n(0) as i32;
        let life = n(1) as f32;
        let params: Vec<Value> = a.iter().skip(2).cloned().collect();
        let id = self.new_goal(kind, life, params, Some(parent));
        let p = self.goal_mut(parent);
        if front {
            p.subgoals.push_front(id);
        } else {
            p.subgoals.push_back(id);
        }
        vec![goal_obj(id)]
    }

    fn goal_method(&mut self, id: u32, name: &str, a: &[Value]) -> LuaResult<Vec<Value>> {
        let n = |i: usize| a.get(i).and_then(Value::num).unwrap_or(0.0);
        if self.goals.get(id as usize).is_none_or(Option::is_none) {
            return Ok(vec![]);
        }
        Ok(match name {
            "AddSubGoal" => self.add_goal(id, false, a),
            "AddSubGoal_Front" => self.add_goal(id, true, a),
            "ClearSubGoal" => {
                let subs: Vec<u32> = self.goal_mut(id).subgoals.drain(..).collect();
                for s in subs {
                    self.drop_goal(s);
                }
                vec![]
            }
            // Unset parameters read as 0, like the engine's zero-filled parameter array.
            "GetParam" => vec![match self.goal(id).params.get(n(0) as usize) {
                None | Some(Value::Nil) => Value::Num(0.0),
                Some(v) => v.clone(),
            }],
            "GetLife" => num(self.goal(id).life as f64),
            "GetSubGoalNum" => num(self.goal(id).subgoals.len() as f64),
            "GetNumber" => num(*self.goal(id).numbers.get(&(n(0) as i32)).unwrap_or(&0.0)),
            "SetNumber" => {
                self.goal_mut(id).numbers.insert(n(0) as i32, n(1));
                vec![]
            }
            "SetTimer" => {
                self.goal_mut(id).timers.insert(n(0) as i32, n(1) as f32);
                vec![]
            }
            "GetTimer" => num(*self.goal(id).timers.get(&(n(0) as i32)).unwrap_or(&0.0) as f64),
            "IsFinishTimer" => {
                boolean(*self.goal(id).timers.get(&(n(0) as i32)).unwrap_or(&0.0) <= 0.0)
            }
            "GetBattleGoalId" => num(self.think.battle_goal_id as f64),
            "GetId" => num(self.goal(id).kind as f64),
            "IsInterruptSubGoalChanged" => boolean(false),
            "SetLifeEndSuccess" | "SetTargetRange" | "SetFailedEndOption" | "SetNoUpdate"
            | "SetManagementGoal" | "SetTableGoal" | "SetNormalGoal" => vec![],
            _ => self.default_method(&format!("goal:{name}")),
        })
    }

    fn default_method(&mut self, name: &str) -> Vec<Value> {
        if self.unknown.insert(name.to_owned()) {
            self.log(AiEvent::Unknown(name.to_owned()));
        }
        let m = name.rsplit(':').next().unwrap_or(name);
        if m.starts_with("Is") || m.starts_with("Has") || m.starts_with("Check") {
            boolean(false)
        } else if m.starts_with("Get") {
            num(0.0)
        } else {
            vec![]
        }
    }

    fn ai_method(&mut self, name: &str, a: &[Value]) -> LuaResult<Vec<Value>> {
        let n = |i: usize| a.get(i).and_then(Value::num).unwrap_or(0.0);
        let t = |i: usize| a.get(i).and_then(Value::num).unwrap_or(0.0) as i32;
        Ok(match name {
            "AddTopGoal" => {
                let top = self.top;
                self.add_goal(top, false, a)
            }
            "GetTopGoal" => vec![goal_obj(self.top)],
            "HasGoal" => {
                let k = t(0);
                boolean(self.goals.iter().flatten().any(|g| g.kind == k))
            }
            "Replanning" => {
                self.replan = true;
                vec![]
            }
            "GetDist" => num(self.dist(t(0)) as f64),
            "GetDist_Point" => num(self.dist(t(0)) as f64),
            "GetDistYSigned" | "GetDistY" => num(0.0),
            "GetRandam_Int" => {
                let (lo, hi) = (n(0).floor(), n(1).floor());
                let r = self.random();
                num((lo + (r * (hi - lo + 1.0)).floor()).min(hi))
            }
            "GetRandam_Float" => {
                let r = self.random();
                num(n(0) + r * (n(1) - n(0)))
            }
            "GetHpRate" => num(self.actor(t(0)).map_or(1.0, |a| {
                if a.max_hp > 0.0 {
                    (a.hp / a.max_hp) as f64
                } else {
                    1.0
                }
            })),
            "GetHp" => num(self.actor(t(0)).map_or(0.0, |a| a.hp as f64)),
            "GetSp" => num(self.actor(t(0)).map_or(0.0, |a| a.posture as f64)),
            "GetSpRate" => num(self.actor(t(0)).map_or(1.0, |a| {
                if a.max_posture > 0.0 {
                    (a.posture / a.max_posture) as f64
                } else {
                    1.0
                }
            })),
            "HasSpecialEffectId" => boolean(self.has_effect(t(0), t(1))),
            "IsTargetGuard" => boolean(self.actor(t(0)).is_some_and(|a| a.guarding)),
            "IsInsideTarget" => boolean(self.inside(t(0), t(1), n(2) as f32)),
            "IsInsideTargetEx" => {
                boolean(self.inside(t(0), t(2), n(3) as f32) && self.dist(t(0)) <= n(4) as f32)
            }
            "IsLookToTarget" => {
                let ang = if a.len() > 1 { n(1) as f32 } else { 30.0 };
                boolean(self.inside(t(0), 1, ang))
            }
            "GetToTargetAngle" => num(self.angle_to(t(0)) as f64),
            "GetMapHitRadius" => num(self.world.hit_radius as f64),
            "GetExcelParam" => num(self.excel(t(0))),
            "GetNpcThinkParamID" => num(self.think.id as f64),
            "GetNumber" => num(*self.numbers.get(&t(0)).unwrap_or(&0.0)),
            "SetNumber" => {
                self.numbers.insert(t(0), n(1));
                vec![]
            }
            "GetStringIndexedNumber" => {
                let k = a.first().and_then(Value::as_str).unwrap_or("").to_owned();
                num(*self.string_numbers.get(&k).unwrap_or(&0.0))
            }
            "SetStringIndexedNumber" => {
                let k = a.first().and_then(Value::as_str).unwrap_or("").to_owned();
                self.string_numbers.insert(k, n(1));
                vec![]
            }
            "SetTimer" | "StartIdTimer" | "TimingSetTimer" => {
                self.timers.insert(t(0), n(1) as f32);
                vec![]
            }
            "GetTimer" | "GetIdTimer" => num(*self.timers.get(&t(0)).unwrap_or(&0.0) as f64),
            "IsFinishTimer" => boolean(*self.timers.get(&t(0)).unwrap_or(&0.0) <= 0.0),
            "GetAttackPassedTime" => num(*self.action_age.get(&t(0)).unwrap_or(&9999.0) as f64),
            "StartAttackPassedTimer" => {
                self.action_age.insert(t(0), 0.0);
                vec![]
            }
            "IsBattleState" => boolean(self.target_state == 3),
            "IsFindState" => boolean(self.target_state == 2),
            "IsCautionState" => boolean(self.target_state == 1),
            "IsChangeState" => boolean(self.target_state != self.prev_target_state),
            "GetPrevTargetState" => num(self.prev_target_state as f64),
            "IsSearchTarget" | "IsVisibleTarget" | "IsVisibleCurrTarget" => {
                boolean(self.world.target.is_some())
            }
            "IsInterupt" => boolean(self.interrupt == Some(t(0))),
            "IsLadderAct"
            | "IsForceBattleGoal"
            | "TeamHelp_IsValidReply"
            | "IsTouchBreakableObject"
            | "IsLockOnTarget" => boolean(false),
            "CheckDoesExistPath" | "IsExistMeshOnLine" | "IsFinishAttackCoolTime" => boolean(true),
            "GetEventRequest" => num(-1.0),
            "GetTeamOrder"
            | "GetChangeBattleStateCount"
            | "GetCurrTargetType"
            | "GetMovePointEffectRange"
            | "GetLatestSoundTargetID"
            | "GetSpecialEffectActivateInterruptType"
            | "GetSpecialEffectInactivateInterruptType"
            | "GetOddsParamIdOffset" => num(0.0),
            "TurnTo" => {
                self.request.face_yaw = self.yaw_to(t(0));
                vec![]
            }
            "DoEzAction" => {
                if n(1) > 0.0 {
                    self.request.action = t(1);
                }
                vec![]
            }
            "AddObserveSpecialEffectAttribute"
            | "AddObserveRegion"
            | "AddObserveChrDmyArea"
            | "DeleteObserve"
            | "SetEventFlag"
            | "PrintText"
            | "SetEventMoveTarget"
            | "ReqPlatoonState"
            | "ClearForceBattleGoal"
            | "RequestEmergencyQuickTurn"
            | "DbgSetLastActIdx"
            | "DbgSetLastKengekiActIdx"
            | "SetAIPredictionMoveTargetSpecifyTargetDir"
            | "ClearEnemyTarget"
            | "RemoveTriggerRegionObserver"
            | "SetTableLogic"
            | "SetNormalLogic"
            | "ClearSoundTarget"
            | "ClearIndicationPosTarget"
            | "ClearLastMemoryTargetPos"
            | "SetEnableEndureCancel_forGoal"
            | "ClearEnableEndureCancel_forGoal" => vec![],
            "DbgGetForceActIdx" | "DbgGetForceKengekiActIdx" => num(0.0),
            // Free floor in every direction: the whole probed distance is walkable. The engine
            // probes its navmesh here; our arenas are open (see docs/AI.md).
            "GetExistMeshOnLineDistSpecifyAngleEx" | "GetExistMeshOnLineDistSpecifyAngle" => {
                num(n(2))
            }
            _ => self.default_method(&format!("ai:{name}")),
        })
    }
}

/// Runs one tick of the goal tree.
pub fn tick(vm: &mut Vm, st: &mut AiState, world: AiWorld) {
    st.world = world;
    let dt = st.world.dt;
    st.time += dt;
    st.request = Request::default();
    for v in st.timers.values_mut() {
        *v -= dt;
    }
    for v in st.action_age.values_mut() {
        *v += dt;
    }
    if st.world.target.is_some() {
        st.battle = true;
    }
    let mut fired = Vec::new();
    if st.world.parry_timing && !st.last_parry_timing {
        // INTERUPT_FindAttack and INTERUPT_ParryTiming.
        fired.extend([1, 24]);
    }
    st.last_parry_timing = st.world.parry_timing;
    if st.world.damaged {
        fired.push(2);
    }
    for kind in fired {
        dispatch_interrupt(vm, st, kind);
    }
    let top = st.top;
    if st.replan || st.goal(top).subgoals.is_empty() {
        st.replan = false;
        let subs: Vec<u32> = st.goal_mut(top).subgoals.drain(..).collect();
        for s in subs {
            terminate(vm, st, s);
        }
        // A seen target moves the AI to the find state, and the next logic run to battle.
        if st.world.target.is_some() {
            st.target_state = if st.target_state < 2 { 2 } else { 3 };
        }
        let logic = st.think.logic_id;
        st.log(AiEvent::Logic(logic));
        let r = vm.call_global(st, "ExecTableLogic", &[ai_obj(), Value::Num(logic as f64)]);
        if let Err(e) = r {
            st.log(AiEvent::Error(e.to_string()));
        }
        st.prev_target_state = st.target_state;
    }
    let subs: Vec<u32> = st.goal(top).subgoals.iter().copied().collect();
    if let Some(&front) = subs.first() {
        let r = update(vm, st, front, 1);
        if r != RESULT_CONTINUE {
            st.goal_mut(top).subgoals.retain(|&g| g != front);
            terminate(vm, st, front);
        }
    }
    if st.request.face_yaw.is_none() && st.battle {
        st.request.face_yaw = st.yaw_to(0);
    }
}

/// Offers interrupt `kind` to the logic, then to the active goals from the deepest up, until
/// one handles it. Our reconstruction of the dispatch order; see docs/AI.md.
fn dispatch_interrupt(vm: &mut Vm, st: &mut AiState, kind: i32) {
    st.interrupt = Some(kind);
    st.log(AiEvent::Interrupt(kind));
    let logic = st.think.logic_id as f64;
    let top = st.top;
    let handled = vm
        .call_global(
            st,
            "InterruptTableLogic_Common",
            &[ai_obj(), goal_obj(top), Value::Num(logic)],
        )
        .ok()
        .flatten()
        .and_then(|v| v.into_iter().next())
        .is_some_and(|v| v.truthy());
    if !handled {
        let mut chain = Vec::new();
        let mut id = st.top;
        while let Some(&next) = st.goal(id).subgoals.front() {
            chain.push(next);
            id = next;
        }
        for &g in chain.iter().rev() {
            if st.goals.get(g as usize).is_none_or(Option::is_none) {
                continue;
            }
            let r = if st.goal(g).imp == GoalImpl::Table {
                let k = st.goal(g).kind as f64;
                vm.call_global(
                    st,
                    "InterruptTableGoal_Common",
                    &[ai_obj(), goal_obj(g), Value::Num(k)],
                )
                .ok()
                .flatten()
                .and_then(|v| v.into_iter().next())
            } else {
                call_cb(vm, st, g, "Interrupt")
            };
            if r.is_some_and(|v| v.truthy()) {
                break;
            }
        }
    }
    st.interrupt = None;
}

fn call_cb(vm: &mut Vm, st: &mut AiState, id: u32, which: &str) -> Option<Value> {
    let g = st.goal(id);
    let (kind, imp) = (g.kind, g.imp);
    let r: LuaResult<Option<Vec<Value>>> = match imp {
        GoalImpl::Table => {
            let f = format!("{which}TableGoal");
            vm.call_global(st, &f, &[ai_obj(), goal_obj(id), Value::Num(kind as f64)])
        }
        GoalImpl::Script => {
            let n = st.name(kind);
            let suffix = if which == "Interrupt" {
                "Interupt"
            } else {
                which
            };
            vm.call_global(st, &format!("{n}_{suffix}"), &[ai_obj(), goal_obj(id)])
        }
        _ => Ok(None),
    };
    match r {
        Ok(v) => v.and_then(|v| v.into_iter().next()),
        Err(e) => {
            let msg = match e {
                LuaError::Runtime(m) | LuaError::Chunk(m) => m,
            };
            st.log(AiEvent::Error(format!("{} {which}: {msg}", st.name(kind))));
            None
        }
    }
}

fn terminate(vm: &mut Vm, st: &mut AiState, id: u32) {
    if st.goals.get(id as usize).is_none_or(Option::is_none) {
        return;
    }
    let subs: Vec<u32> = st.goal_mut(id).subgoals.drain(..).collect();
    for s in subs {
        terminate(vm, st, s);
    }
    if st.goal(id).activated {
        call_cb(vm, st, id, "Terminate");
    }
    st.goals[id as usize] = None;
}

/// Updates goal `id` (activating it first) and returns its result.
fn update(vm: &mut Vm, st: &mut AiState, id: u32, depth: usize) -> i32 {
    if depth > 24 {
        return RESULT_FAILED;
    }
    let dt = st.world.dt;
    if !st.goal(id).activated {
        st.goal_mut(id).activated = true;
        let kind = st.goal(id).kind;
        let name = st.name(kind);
        st.log(AiEvent::GoalStart { depth, kind, name });
        call_cb(vm, st, id, "Activate");
        if st.goals.get(id as usize).is_none_or(Option::is_none) {
            return RESULT_FAILED;
        }
    }
    {
        let g = st.goal_mut(id);
        g.elapsed += dt;
        for v in g.timers.values_mut() {
            *v -= dt;
        }
    }
    // Subgoals first, front to back; a finished one is removed and the next starts next tick.
    let mut emptied = false;
    if let Some(&front) = st.goal(id).subgoals.front() {
        let r = update(vm, st, front, depth + 1);
        if r != RESULT_CONTINUE {
            st.goal_mut(id).subgoals.pop_front();
            let kind = st.goals[front as usize].as_ref().map_or(0, |g| g.kind);
            let name = st.name(kind);
            st.log(AiEvent::GoalEnd {
                kind,
                name,
                result: r,
            });
            terminate(vm, st, front);
            if r == RESULT_FAILED {
                let subs: Vec<u32> = st.goal_mut(id).subgoals.drain(..).collect();
                for s in subs {
                    terminate(vm, st, s);
                }
                return RESULT_FAILED;
            }
            emptied = st.goal(id).subgoals.is_empty();
        }
    }
    let imp = st.goal(id).imp;
    let result = match imp {
        GoalImpl::Native(n) => native_update(st, id, n),
        GoalImpl::Table | GoalImpl::Script => {
            match call_cb(vm, st, id, "Update").and_then(|v| v.num()) {
                Some(r) => r as i32,
                None if st.goals[id as usize].is_none() => RESULT_FAILED,
                None => {
                    if st.goal(id).subgoals.is_empty() {
                        RESULT_SUCCESS
                    } else {
                        RESULT_CONTINUE
                    }
                }
            }
        }
        GoalImpl::Empty => {
            if st.goal(id).subgoals.is_empty() {
                RESULT_SUCCESS
            } else {
                RESULT_CONTINUE
            }
        }
    };
    if st.goals.get(id as usize).is_none_or(Option::is_none) {
        return RESULT_FAILED;
    }
    // A goal whose last subgoal just succeeded succeeds too, unless its Update decided
    // otherwise or added new subgoals (the script goals' Update functions return Continue and
    // rely on this).
    if emptied && result == RESULT_CONTINUE && st.goal(id).subgoals.is_empty() {
        return RESULT_SUCCESS;
    }
    let g = st.goal_mut(id);
    if result == RESULT_CONTINUE && g.life >= 0.0 {
        g.life -= dt;
        if g.life < 0.0 {
            // Lifetime over: waiting-style goals succeed, the rest fail (our reading of how
            // the scripts use lifetimes, see docs/AI.md).
            let ok = matches!(
                g.imp,
                GoalImpl::Native(NativeGoal::Wait | NativeGoal::Guard | NativeGoal::Sideway)
            );
            return if ok { RESULT_SUCCESS } else { RESULT_FAILED };
        }
    }
    result
}

fn native_update(st: &mut AiState, id: u32, n: NativeGoal) -> i32 {
    let g = st.goal(id).clone();
    match n {
        NativeGoal::Wait => {
            let target = g.param(0) as i32;
            if target != -1 && target != -2 {
                st.request.face_yaw = st.yaw_to(target);
            }
            RESULT_CONTINUE
        }
        NativeGoal::MoveTo => {
            // MoveToSomewhere(target, dirType, dist, turnTarget, walk, ...).
            let target = g.param(0) as i32;
            let dist = g.param(2) as f32;
            let walk = g.param_bool(4);
            if st.dist(target) <= dist.max(0.0) {
                return RESULT_SUCCESS;
            }
            st.request.move_yaw = st.yaw_to(target);
            st.request.move_level = if walk { 0.5 } else { 1.0 };
            st.request.face_yaw = st.yaw_to(target);
            RESULT_CONTINUE
        }
        NativeGoal::Guard => {
            // Guard(actionId, target, ...).
            let action = g.param(0) as i32;
            if action > 0 {
                st.request.action = action;
            }
            st.request.face_yaw = st.yaw_to(g.param(1) as i32);
            RESULT_CONTINUE
        }
        NativeGoal::Sideway => {
            // SidewayMove(target, right(0)/left(1), angle, turnTarget, walk, ...).
            let target = g.param(0) as i32;
            let left = g.param(1) as i32 == 1;
            if let Some(y) = st.yaw_to(target) {
                let side = if left { PI / 2.0 } else { -PI / 2.0 };
                st.request.move_yaw = Some(y + side);
                st.request.face_yaw = Some(y);
                st.request.move_level = 0.5;
            }
            RESULT_CONTINUE
        }
        NativeGoal::KeepDist => {
            // KeepDist(target, minDist, maxDist, turnTarget, walk, ...).
            let target = g.param(0) as i32;
            let (lo, hi) = (g.param(1) as f32, g.param(2) as f32);
            let d = st.dist(target);
            let Some(y) = st.yaw_to(target) else {
                return RESULT_SUCCESS;
            };
            st.request.face_yaw = Some(y);
            st.request.move_level = if g.param_bool(4) { 0.5 } else { 1.0 };
            if d < lo {
                st.request.move_yaw = Some(y + PI);
            } else if d > hi {
                st.request.move_yaw = Some(y);
            } else {
                return RESULT_SUCCESS;
            }
            RESULT_CONTINUE
        }
        NativeGoal::Leave => {
            // LeaveTarget(target, dist, turnTarget, walk, ...).
            let target = g.param(0) as i32;
            if st.dist(target) >= g.param(1) as f32 {
                return RESULT_SUCCESS;
            }
            if let Some(y) = st.yaw_to(target) {
                st.request.move_yaw = Some(y + PI);
                st.request.face_yaw = Some(y);
                st.request.move_level = if g.param_bool(3) { 0.5 } else { 1.0 };
            }
            RESULT_CONTINUE
        }
        NativeGoal::Action => action_update(st, id, &g),
        NativeGoal::Unsupported => RESULT_SUCCESS,
    }
}

/// CommonAttack(ezStateId, target, successDist, turnAngle, turnTime, frontAngle, isCombo, ...)
/// and the other action goals: turn toward the target, request the action until its animation
/// plays, then wait for it to end (or, for combo steps, for the combo window).
fn action_update(st: &mut AiState, id: u32, g: &Goal) -> i32 {
    let action = g.param(0) as i32;
    let target = g.param(1) as i32;
    let combo = g.kind == 2200 && g.param_bool(6);
    if target >= 0 {
        st.request.face_yaw = st.yaw_to(target);
    }
    let playing = st.world.current_anim == Some(action);
    if !g.started_anim {
        if playing {
            st.goal_mut(id).started_anim = true;
            return RESULT_CONTINUE;
        }
        if g.elapsed > 2.0 {
            return RESULT_FAILED;
        }
        if g.phase == 0 {
            st.log(AiEvent::Action(action));
            st.action_age.insert(action, 0.0);
            st.goal_mut(id).phase = 1;
        }
        st.request.action = action;
        return RESULT_CONTINUE;
    }
    if !playing {
        return RESULT_SUCCESS;
    }
    if combo && st.world.combo_window {
        return RESULT_SUCCESS;
    }
    RESULT_CONTINUE
}

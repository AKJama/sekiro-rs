//! A behaviour-graph runtime: which states are active, how events move them, which clip plays.
//!
//! This is a deliberately small model of Havok Behavior as FromSoftware uses it:
//!
//! - Every state machine remembers its current state. The active set is everything reachable
//!   from the root through current states, all layers, all blend children, the selected child
//!   of each manual selector and the offset-matched clip of each CMSG.
//! - Events are matched against the active machines first (the current state's own
//!   transitions, then the machine's wildcards), then against the global wildcards of inactive
//!   machines. Taking a global wildcard also switches every enclosing machine to the state that
//!   contains it, which is how one `W_*` event reaches a deeply nested state.
//! - Blends are not modelled: a transition switches instantly. Transition effects are kept on
//!   the [`Transition`] for the animation layer to use.
//! - State hooks are named after the state (`<StateName>_onActivate` and so on), matching the
//!   player script's callback names.

use sekiro_formats::hkb::{Graph, NodeId, NodeKind, Transition, parse_anim_name, transition_flags};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

/// Supplies clip lengths in seconds; `None` means unknown (treated as never ending).
pub trait ClipDurations {
    fn duration(&mut self, animation: &str) -> Option<f32>;
}

impl<F: FnMut(&str) -> Option<f32>> ClipDurations for F {
    fn duration(&mut self, animation: &str) -> Option<f32> {
        self(animation)
    }
}

/// Which part of a state's lifetime a hook call is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookKind {
    Activate,
    Update,
    Deactivate,
}

impl HookKind {
    pub fn suffix(self) -> &'static str {
        match self {
            HookKind::Activate => "_onActivate",
            HookKind::Update => "_onUpdate",
            HookKind::Deactivate => "_onDeactivate",
        }
    }
}

/// A state hook the script should run, e.g. `StandToDeflectGuard` + `_onActivate`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookCall {
    pub state: String,
    pub kind: HookKind,
}

impl HookCall {
    pub fn function_name(&self) -> String {
        format!("{}{}", self.state, self.kind.suffix())
    }
}

/// A playing clip.
#[derive(Debug, Clone, PartialEq)]
pub struct ClipState {
    pub node: NodeId,
    pub animation: String,
    pub time: f32,
    pub duration: Option<f32>,
    pub looping: bool,
    pub speed: f32,
}

impl ClipState {
    /// True once a non-looping clip has played to its end.
    pub fn at_end(&self) -> bool {
        !self.looping && self.duration.is_some_and(|d| self.time >= d)
    }
}

/// The animation offset (`aXXX_` prefix) the engine derives for each CMSG offset type.
///
/// Observed types in the player graph: 0 and 11 (fixed, mostly `a000`), 13 (right-hand weapon
/// motion category, `a050` for the katana), 14 (left-hand prosthetic, `a070`..`a079`) and 17
/// (throw-specific `a2xx`).
#[derive(Debug, Clone, Default)]
pub struct AnimOffsets {
    pub by_type: HashMap<i64, u32>,
}

impl AnimOffsets {
    /// Defaults for the player with the katana drawn and no prosthetic selected.
    pub fn player_default() -> Self {
        let mut by_type = HashMap::new();
        by_type.insert(13, 50);
        by_type.insert(14, 70);
        Self { by_type }
    }

    pub fn offset(&self, offset_type: i64) -> u32 {
        self.by_type.get(&offset_type).copied().unwrap_or(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct StateRef {
    sm: NodeId,
    index: usize,
}

pub struct BehaviorRuntime {
    graph: Arc<Graph>,
    /// For every node, the state that encloses it (first found from the root).
    enclosing: Vec<Option<StateRef>>,
    /// Current state index per state machine node.
    current: HashMap<NodeId, usize>,
    active: Vec<bool>,
    /// Active states in activation order (parents before children).
    active_states: Vec<StateRef>,
    clips: HashMap<NodeId, ClipState>,
    variables: Vec<f32>,
    pub offsets: AnimOffsets,
    queue: VecDeque<usize>,
    /// States re-entered by a self transition since the last rebuild.
    restarted: HashSet<StateRef>,
    /// Machines whose state was set explicitly since the last rebuild.
    pinned: HashSet<NodeId>,
    /// Layer on/off overrides set by layer events, keyed by (layer generator, layer index).
    layer_on: HashMap<(NodeId, usize), bool>,
    /// For every clip, the CMSG that owns it (if any).
    clip_owner: Vec<Option<NodeId>>,
    /// Clips whose end has already been signalled since they started.
    end_signalled: HashSet<NodeId>,
    /// Set when a variable changed, since bound selectors and layer weights may now differ.
    dirty: bool,
    /// The machine whose active clip answers "animation ended" for slot 0.
    main_machine: NodeId,
    /// Events that matched no transition, most recent last (bounded).
    pub unhandled_events: VecDeque<String>,
}

impl BehaviorRuntime {
    pub fn new(graph: Arc<Graph>, offsets: AnimOffsets) -> Self {
        let n = graph.nodes.len();
        let mut enclosing = vec![None; n];
        let mut seen = vec![false; n];
        let mut stack = vec![(graph.root, None)];
        while let Some((id, enc)) = stack.pop() {
            if seen[id] {
                continue;
            }
            seen[id] = true;
            enclosing[id] = enc;
            match &graph.nodes[id].kind {
                NodeKind::StateMachine(sm) => {
                    for (i, s) in sm.states.iter().enumerate().rev() {
                        if let Some(g) = s.generator {
                            stack.push((g, Some(StateRef { sm: id, index: i })));
                        }
                    }
                }
                _ => {
                    for c in graph.nodes[id].children().into_iter().rev() {
                        stack.push((c, enc));
                    }
                }
            }
        }
        let mut clip_owner = vec![None; n];
        for (id, node) in graph.nodes.iter().enumerate() {
            if let NodeKind::Cmsg(c) = &node.kind {
                for g in c.generators.iter().flatten() {
                    clip_owner[*g] = Some(id);
                }
            }
        }
        let variables = graph.variables.iter().map(|v| v.initial.as_f32()).collect();
        let main_machine = graph.node_by_name("Master_SM").unwrap_or(graph.root);
        let mut rt = Self {
            graph,
            enclosing,
            current: HashMap::new(),
            active: vec![false; n],
            active_states: Vec::new(),
            clips: HashMap::new(),
            variables,
            offsets,
            queue: VecDeque::new(),
            restarted: HashSet::new(),
            pinned: HashSet::new(),
            layer_on: HashMap::new(),
            clip_owner,
            end_signalled: HashSet::new(),
            dirty: false,
            main_machine,
            unhandled_events: VecDeque::new(),
        };
        rt.rebuild(&mut |_: &str| None);
        rt
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// Hooks for the states active after construction (call once after loading the scripts).
    pub fn initial_hooks(&self) -> Vec<HookCall> {
        self.active_states
            .iter()
            .map(|s| HookCall {
                state: self.state_name(*s).to_owned(),
                kind: HookKind::Activate,
            })
            .collect()
    }

    fn state_name(&self, s: StateRef) -> &str {
        match &self.graph.nodes[s.sm].kind {
            NodeKind::StateMachine(sm) => &sm.states[s.index].name,
            _ => "",
        }
    }

    fn current_index(&self, sm_id: NodeId) -> usize {
        let NodeKind::StateMachine(sm) = &self.graph.nodes[sm_id].kind else {
            return 0;
        };
        self.current
            .get(&sm_id)
            .copied()
            .or_else(|| sm.state_index(sm.start_state_id))
            .unwrap_or(0)
    }

    /// Queues an event by name. Returns false if the graph has no such event.
    pub fn fire_event(&mut self, name: &str) -> bool {
        match self.graph.event_index(name) {
            Some(e) => {
                self.queue.push_back(e);
                true
            }
            None => {
                self.note_unhandled(name.to_owned());
                false
            }
        }
    }

    fn note_unhandled(&mut self, name: String) {
        if self.unhandled_events.len() >= 64 {
            self.unhandled_events.pop_front();
        }
        self.unhandled_events.push_back(name);
    }

    pub fn has_pending_events(&self) -> bool {
        !self.queue.is_empty()
    }

    /// Applies all queued events and returns the hooks to run, deactivations first.
    pub fn process_events(&mut self, durations: &mut dyn ClipDurations) -> Vec<HookCall> {
        let before: Vec<StateRef> = self.active_states.clone();
        let mut changed = std::mem::take(&mut self.dirty);
        while let Some(e) = self.queue.pop_front() {
            if self.apply_event(e) {
                changed = true;
            } else {
                let name = self.graph.events[e].clone();
                self.note_unhandled(name);
            }
        }
        if !changed {
            return Vec::new();
        }
        let restarted = self.restarted.clone();
        self.rebuild(durations);
        let after: HashSet<StateRef> = self.active_states.iter().copied().collect();
        let before_set: HashSet<StateRef> = before.iter().copied().collect();
        let mut hooks = Vec::new();
        for s in before.iter().rev() {
            if !after.contains(s) || restarted.contains(s) {
                hooks.push(HookCall {
                    state: self.state_name(*s).to_owned(),
                    kind: HookKind::Deactivate,
                });
            }
        }
        for s in &self.active_states {
            if !before_set.contains(s) || restarted.contains(s) {
                hooks.push(HookCall {
                    state: self.state_name(*s).to_owned(),
                    kind: HookKind::Activate,
                });
            }
        }
        hooks
    }

    /// Update hooks for every active state, in activation order.
    pub fn update_hooks(&self) -> Vec<HookCall> {
        self.active_states
            .iter()
            .map(|s| HookCall {
                state: self.state_name(*s).to_owned(),
                kind: HookKind::Update,
            })
            .collect()
    }

    fn find_transition(transitions: &[Transition], event: i32) -> Option<&Transition> {
        transitions
            .iter()
            .filter(|t| t.event == event && t.flags & transition_flags::DISABLED == 0)
            .min_by_key(|t| t.priority)
    }

    fn apply_event(&mut self, event: usize) -> bool {
        let graph = self.graph.clone();
        let ev = event as i32;
        let mut handled = false;
        // Layer on/off events of active layer generators.
        for (id, node) in graph.nodes.iter().enumerate() {
            if !self.active[id] {
                continue;
            }
            if let NodeKind::Layers(layers) = &node.kind {
                for (i, l) in layers.iter().enumerate() {
                    if l.on_event == ev {
                        self.layer_on.insert((id, i), true);
                        handled = true;
                    } else if l.off_event == ev {
                        self.layer_on.insert((id, i), false);
                        handled = true;
                    }
                }
            }
        }
        // Active machines, outermost first: the current state's transitions, then wildcards.
        let mut machines: Vec<NodeId> = Vec::new();
        for s in &self.active_states {
            if !machines.contains(&s.sm) {
                machines.push(s.sm);
            }
        }
        for &sm_id in &machines {
            let NodeKind::StateMachine(sm) = &graph.nodes[sm_id].kind else {
                continue;
            };
            let cur = self.current_index(sm_id);
            let hit = Self::find_transition(&sm.states[cur].transitions, ev)
                .or_else(|| Self::find_transition(&sm.wildcard_transitions, ev));
            if let Some(t) = hit {
                return self.take(sm_id, t) || handled;
            }
        }
        // Global wildcards of inactive machines.
        for (sm_id, sm) in graph.state_machines() {
            if self.active[sm_id] {
                continue;
            }
            if let Some(t) = Self::find_transition(&sm.wildcard_transitions, ev)
                && t.flags & transition_flags::IS_GLOBAL_WILDCARD != 0
            {
                return self.take(sm_id, t) || handled;
            }
        }
        handled
    }

    fn take(&mut self, sm_id: NodeId, t: &Transition) -> bool {
        let graph = self.graph.clone();
        let NodeKind::StateMachine(sm) = &graph.nodes[sm_id].kind else {
            return false;
        };
        let Some(index) = sm.state_index(t.to_state) else {
            return false;
        };
        let target = StateRef { sm: sm_id, index };
        if self.active[sm_id] && self.current_index(sm_id) == index {
            self.restarted.insert(target);
        }
        self.current.insert(sm_id, index);
        self.pinned.insert(sm_id);
        // Nested target state inside the destination state's own machine.
        if t.flags & transition_flags::TO_NESTED_STATE_ID_IS_VALID != 0
            && let Some(g) = sm.states[index].generator
            && let NodeKind::StateMachine(inner) = &graph.nodes[g].kind
            && let Some(i) = inner.state_index(t.to_nested_state)
        {
            self.current.insert(g, i);
            self.pinned.insert(g);
        }
        // Switch every enclosing machine to the state containing this one.
        let mut up = self.enclosing[sm_id];
        while let Some(s) = up {
            self.current.insert(s.sm, s.index);
            self.pinned.insert(s.sm);
            up = self.enclosing[s.sm];
        }
        true
    }

    fn rebuild(&mut self, durations: &mut dyn ClipDurations) {
        let graph = self.graph.clone();
        let was_active = std::mem::replace(&mut self.active, vec![false; graph.nodes.len()]);
        let restarted: HashSet<StateRef> = self.restarted.iter().copied().collect();
        let mut states = Vec::new();
        let mut clips = Vec::new();
        let mut stack = vec![graph.root];
        while let Some(id) = stack.pop() {
            if self.active[id] {
                continue;
            }
            self.active[id] = true;
            let node = &graph.nodes[id];
            match &node.kind {
                NodeKind::StateMachine(sm) => {
                    if !was_active[id] && !self.pinned.contains(&id) {
                        self.current.remove(&id);
                    }
                    if sm.states.is_empty() {
                        continue;
                    }
                    let index = self.current_index(id);
                    states.push(StateRef { sm: id, index });
                    if let Some(g) = sm.states[index].generator {
                        stack.push(g);
                    }
                }
                NodeKind::ManualSelector(m) => {
                    let idx = self.bound_index(node, "selectedGeneratorIndex", m.selected_index);
                    let pick = m
                        .generators
                        .get(idx)
                        .or(m.generators.first())
                        .copied()
                        .flatten();
                    stack.extend(pick);
                }
                NodeKind::Cmsg(c) => {
                    if let Some(g) = self.pick_cmsg_clip(c) {
                        stack.push(g);
                    }
                }
                NodeKind::Clip(_) => clips.push(id),
                NodeKind::Layers(layers) => {
                    for (i, l) in layers.iter().enumerate().rev() {
                        if self.layer_enabled(id, i, l)
                            && let Some(g) = l.generator
                        {
                            stack.push(g);
                        }
                    }
                }
                _ => {
                    for ch in node.children().into_iter().rev() {
                        stack.push(ch);
                    }
                }
            }
        }
        self.pinned.clear();
        // Clips: keep time for clips that stayed active, restart the rest.
        let restarted_clips: HashSet<NodeId> = clips
            .iter()
            .copied()
            .filter(|c| {
                self.enclosing_state(*c)
                    .is_some_and(|s| restarted.contains(&s))
            })
            .collect();
        let mut new_clips = HashMap::new();
        for id in clips {
            let NodeKind::Clip(c) = &graph.nodes[id].kind else {
                continue;
            };
            let keep = was_active[id] && !restarted_clips.contains(&id);
            if !keep {
                self.end_signalled.remove(&id);
            }
            let state = match (keep, self.clips.remove(&id)) {
                (true, Some(s)) => s,
                _ => ClipState {
                    node: id,
                    animation: c.animation.clone(),
                    time: c.start_time,
                    duration: durations.duration(&c.animation),
                    looping: c.mode == 1,
                    speed: c.playback_speed,
                },
            };
            new_clips.insert(id, state);
        }
        self.end_signalled.retain(|c| new_clips.contains_key(c));
        self.clips = new_clips;
        self.active_states = states;
        self.restarted.clear();
    }

    /// The nearest state machine state enclosing `node`, following the active path.
    fn enclosing_state(&self, node: NodeId) -> Option<StateRef> {
        self.enclosing[node]
    }

    /// A layer contributes when it is switched on (by default or by its on/off events) and its
    /// weight, usually bound to a `*Blend` variable the script sets, is above zero.
    fn layer_enabled(&self, layers: NodeId, index: usize, l: &sekiro_formats::hkb::Layer) -> bool {
        let on = self
            .layer_on
            .get(&(layers, index))
            .copied()
            .unwrap_or(l.on_by_default);
        let weight = l
            .bindings
            .iter()
            .find(|b| b.member_path.ends_with("weight") && b.binding_type == 0)
            .and_then(|b| self.variables.get(b.variable as usize).copied())
            .unwrap_or(l.weight);
        on && weight > 0.0
    }

    /// The clip directly owned by the active state named `state` (not through nested machines).
    pub fn state_clip(&self, state: &str) -> Option<&ClipState> {
        let s = *self
            .active_states
            .iter()
            .find(|s| self.state_name(**s) == state)?;
        self.clips
            .values()
            .filter(|c| self.enclosing[c.node] == Some(s))
            .min_by_key(|c| c.node)
    }

    fn bound_index(&self, node: &sekiro_formats::hkb::Node, member: &str, default: i64) -> usize {
        node.bindings
            .iter()
            .find(|b| b.member_path == member && b.binding_type == 0)
            .and_then(|b| self.variables.get(b.variable as usize))
            .map(|v| v.max(0.0) as usize)
            .unwrap_or(default.max(0) as usize)
    }

    fn pick_cmsg_clip(&self, c: &sekiro_formats::hkb::Cmsg) -> Option<NodeId> {
        let want = self.offsets.offset(c.offset_type);
        let offset_of = |g: NodeId| match &self.graph.nodes[g].kind {
            NodeKind::Clip(clip) => parse_anim_name(&clip.animation).map(|(o, _)| o),
            _ => None,
        };
        let gens: Vec<NodeId> = c.generators.iter().flatten().copied().collect();
        gens.iter()
            .copied()
            .find(|&g| offset_of(g) == Some(want))
            .or_else(|| gens.iter().copied().find(|&g| offset_of(g) == Some(0)))
            .or_else(|| gens.first().copied())
    }

    /// Queues the events that ending clips send, following each owning CMSG's
    /// `animeEndEventType`, as inferred from the shipped graph:
    ///
    /// - 0: take the owning state's own (first) transition, e.g. stand-to-guard into guard idle;
    /// - 2: send the CMSG's end event (event 0, `Idle_wild`, returns to idle);
    /// - 1 and 3: nothing (hold the last frame, or loop).
    ///
    /// Returns the number of events queued.
    pub fn queue_clip_end_events(&mut self) -> usize {
        let graph = self.graph.clone();
        let mut ended: Vec<NodeId> = self
            .clips
            .values()
            .filter(|c| c.at_end() && !self.end_signalled.contains(&c.node))
            .map(|c| c.node)
            .collect();
        ended.sort_unstable();
        let mut queued = 0;
        for clip in ended {
            self.end_signalled.insert(clip);
            let Some(owner) = self.clip_owner[clip] else {
                continue;
            };
            let NodeKind::Cmsg(c) = &graph.nodes[owner].kind else {
                continue;
            };
            let event = match c.anime_end_event_type {
                0 => self.enclosing[clip].and_then(|s| match &graph.nodes[s.sm].kind {
                    NodeKind::StateMachine(sm) => {
                        sm.states[s.index].transitions.first().map(|t| t.event)
                    }
                    _ => None,
                }),
                2 => Some(c.end_event),
                _ => None,
            };
            if let Some(e) = event.and_then(|e| usize::try_from(e).ok()) {
                self.queue.push_back(e);
                queued += 1;
            }
        }
        queued
    }

    /// Advances every active clip.
    pub fn advance(&mut self, dt: f32) {
        for c in self.clips.values_mut() {
            c.time += dt * c.speed;
            if c.looping
                && let Some(d) = c.duration
                && d > 0.0
            {
                c.time %= d;
            }
        }
    }

    /// The clip playing under the main state machine (the full-body slot 0).
    pub fn main_clip(&self) -> Option<&ClipState> {
        let mut sm = self.main_machine;
        // Follow current states down through nested machines to the deepest clip.
        let mut best: Option<&ClipState> = None;
        for _ in 0..64 {
            if !self.active[sm] {
                break;
            }
            let idx = self.current_index(sm);
            let next = self
                .clips
                .values()
                .filter(|c| self.enclosing[c.node] == Some(StateRef { sm, index: idx }))
                .min_by_key(|c| c.node);
            if next.is_some() {
                best = next;
            }
            let child_sm = self
                .active_states
                .iter()
                .find(|s| s.sm != sm && self.enclosing[s.sm] == Some(StateRef { sm, index: idx }))
                .map(|s| s.sm);
            match child_sm {
                Some(c) => sm = c,
                None => break,
            }
        }
        best
    }

    /// Names of the active states under the main machine, outermost first.
    pub fn main_state_path(&self) -> Vec<String> {
        let mut path = Vec::new();
        let mut sm = self.main_machine;
        for _ in 0..64 {
            if !self.active[sm] {
                break;
            }
            let idx = self.current_index(sm);
            let here = StateRef { sm, index: idx };
            path.push(self.state_name(here).to_owned());
            match self
                .active_states
                .iter()
                .find(|s| s.sm != sm && self.enclosing[s.sm] == Some(here))
            {
                Some(s) => sm = s.sm,
                None => break,
            }
        }
        path
    }

    /// All active state names, in activation order.
    pub fn active_state_names(&self) -> Vec<String> {
        self.active_states
            .iter()
            .map(|s| self.state_name(*s).to_owned())
            .collect()
    }

    /// `env(339, slot)`: whether an animation has ended. When a state hook is running, the
    /// question is about that state's own clip (additive-layer states ask with slot 0 too);
    /// otherwise about the full-body clip. Only slot 0 is modelled.
    pub fn anim_ended(&self, slot: i32, hook_state: Option<&str>) -> bool {
        if slot != 0 {
            return false;
        }
        hook_state
            .and_then(|s| self.state_clip(s))
            .or_else(|| self.main_clip())
            .is_some_and(ClipState::at_end)
    }

    pub fn is_node_active(&self, name: &str) -> bool {
        self.graph
            .nodes
            .iter()
            .enumerate()
            .any(|(i, n)| self.active[i] && n.name == name)
    }

    pub fn variable(&self, name: &str) -> Option<f32> {
        self.graph
            .variable_index(name)
            .and_then(|i| self.variables.get(i).copied())
    }

    pub fn set_variable(&mut self, name: &str, value: f32) -> bool {
        match self.graph.variable_index(name) {
            Some(i) => {
                if self.variables[i] != value {
                    self.variables[i] = value;
                    self.dirty = true;
                }
                true
            }
            None => false,
        }
    }

    pub fn clips(&self) -> impl Iterator<Item = &ClipState> {
        self.clips.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sekiro_formats::hkb::{Clip, Node, State, StateMachine};

    fn clip(name: &str, looping: bool) -> Node {
        Node {
            type_name: "hkbClipGenerator".into(),
            name: name.into(),
            user_data: 0,
            bindings: Vec::new(),
            kind: NodeKind::Clip(Clip {
                animation: name.into(),
                mode: i64::from(looping),
                playback_speed: 1.0,
                start_time: 0.0,
                crop_start: 0.0,
                crop_end: 0.0,
                enforced_duration: 0.0,
                user_controlled_time_fraction: 0.0,
                flags: 0,
            }),
        }
    }

    fn state(name: &str, id: i32, generator: NodeId) -> State {
        State {
            name: name.into(),
            id,
            generator: Some(generator),
            transitions: Vec::new(),
            probability: 1.0,
            enable: true,
        }
    }

    fn machine(name: &str, start: i32, states: Vec<State>, wildcards: Vec<Transition>) -> Node {
        Node {
            type_name: "hkbStateMachine".into(),
            name: name.into(),
            user_data: 0,
            bindings: Vec::new(),
            kind: NodeKind::StateMachine(StateMachine {
                start_state_id: start,
                states,
                wildcard_transitions: wildcards,
                return_to_previous_state_event: -1,
                self_transition_mode: 0,
                start_state_mode: 0,
            }),
        }
    }

    fn wildcard(event: i32, to: i32) -> Transition {
        Transition {
            event,
            to_state: to,
            from_nested_state: 0,
            to_nested_state: 0,
            priority: 0,
            flags: transition_flags::IS_GLOBAL_WILDCARD | transition_flags::IS_LOCAL_WILDCARD,
            effect: None,
            has_condition: false,
        }
    }

    /// Root: A (clip a) or B (inner machine: X or Y). `W_Y` is a global wildcard of the inner
    /// machine, `W_A` of the root.
    fn graph() -> Graph {
        let nodes = vec![
            machine(
                "Master_SM",
                0,
                vec![state("A", 0, 1), state("B", 1, 2)],
                vec![wildcard(1, 0)],
            ),
            clip("a000_000000", true),
            machine(
                "Inner_SM",
                0,
                vec![state("X", 0, 3), state("Y", 1, 4)],
                vec![wildcard(0, 1)],
            ),
            clip("a000_000001", true),
            clip("a050_000002", false),
        ];
        Graph {
            name: "test".into(),
            events: vec!["W_Y".into(), "W_A".into()],
            event_flags: vec![0, 0],
            variables: Vec::new(),
            animation_names: Vec::new(),
            nodes,
            effects: Vec::new(),
            root: 0,
        }
    }

    #[test]
    fn global_wildcard_switches_enclosing_machines() {
        let mut rt = BehaviorRuntime::new(Arc::new(graph()), AnimOffsets::default());
        let mut durations = |name: &str| (name == "a050_000002").then_some(0.5);
        assert_eq!(rt.main_state_path(), ["A"]);
        assert!(rt.fire_event("W_Y"));
        let hooks = rt.process_events(&mut durations);
        let names: Vec<String> = hooks.iter().map(HookCall::function_name).collect();
        assert_eq!(names, ["A_onDeactivate", "B_onActivate", "Y_onActivate"]);
        assert_eq!(rt.main_state_path(), ["B", "Y"]);
        assert_eq!(rt.main_clip().unwrap().animation, "a050_000002");

        rt.advance(0.6);
        assert!(rt.anim_ended(0, None));
        assert!(rt.is_node_active("Inner_SM"));

        // Self transition restarts the state and its clip.
        rt.fire_event("W_Y");
        let names: Vec<String> = rt
            .process_events(&mut durations)
            .iter()
            .map(HookCall::function_name)
            .collect();
        assert_eq!(names, ["Y_onDeactivate", "Y_onActivate"]);
        assert_eq!(rt.main_clip().unwrap().time, 0.0);

        // Leaving and re-entering B resets the inner machine to its start state.
        rt.fire_event("W_A");
        rt.process_events(&mut durations);
        assert_eq!(rt.main_state_path(), ["A"]);
        assert!(!rt.is_node_active("Inner_SM"));
        assert!(!rt.fire_event("NoSuchEvent"));
    }
}

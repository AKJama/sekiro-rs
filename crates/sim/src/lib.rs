//! Bevy-free gameplay simulation.
//!
//! [`behavior`] runs a Havok behaviour graph (states, events, clips), [`player`] drives it with
//! the game's own HKS player scripts through the `sekiro-hks` VM, and [`scenario`] holds scripted
//! input sequences used by tests and the `player-sim` CLI.

pub mod behavior;
pub mod player;
pub mod scenario;

pub use behavior::{AnimOffsets, BehaviorRuntime, ClipDurations, ClipState, HookCall, HookKind};
pub use player::{PlayerBehavior, PlayerEnv, TickReport};

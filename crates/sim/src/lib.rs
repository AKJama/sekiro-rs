//! Bevy-free gameplay simulation.
//!
//! [`behavior`] runs a Havok behaviour graph (states, events, clips), [`player`] drives it with
//! the game's own HKS player scripts through the `sekiro-hks` VM, [`tae`] turns the playing
//! clips' TimeAct events into cancel windows and SpEffects, [`input`] and [`body`] translate
//! controls and move the character, and [`character`] puts them together into a playable Wolf.
//! [`scenario`] and [`demo`] hold scripted input for tests and CLIs.

pub mod behavior;
pub mod body;
pub mod character;
pub mod clips;
pub mod demo;
pub mod input;
pub mod player;
pub mod scenario;
pub mod tae;

pub use behavior::{AnimOffsets, BehaviorRuntime, ClipDurations, ClipState, HookCall, HookKind};
pub use character::{PlayerCharacter, StepReport};
pub use input::{Buttons, InputFrame};
pub use player::{PlayerBehavior, PlayerEnv, TickReport};

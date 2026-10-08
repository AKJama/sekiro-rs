//! Readers for the data formats shipped with Sekiro: Shadows Die Twice (PC, v1.6.0.0).
//!
//! Everything here reads; nothing writes back to the game. Format knowledge comes from public
//! community documentation (SoulsFormats, Paramdex, DSAnimStudio, HavokLib), reimplemented.

pub mod bhd;
pub mod bnd4;
pub mod dcx;
pub mod dvd;
pub mod reader;

pub use reader::{Error, Result};

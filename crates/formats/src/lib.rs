//! Readers for the data formats shipped with Sekiro: Shadows Die Twice (PC, v1.6.0.0).
//!
//! Everything here reads; nothing writes back to the game. Format knowledge comes from public
//! community documentation (SoulsFormats, Paramdex, DSAnimStudio, HavokLib), reimplemented.

pub mod anim;
pub mod bhd;
pub mod bnd4;
pub mod dcx;
pub mod dvd;
pub mod flver;
pub mod hkb;
pub mod hks;
pub mod hkx;
pub mod mtd;
pub mod param;
pub mod paramdef;
pub mod reader;
pub mod tae;
pub mod tpf;

pub use reader::{Error, Result};

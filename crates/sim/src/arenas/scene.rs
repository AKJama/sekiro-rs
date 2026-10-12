//! Presentation state from arena EMEVD instructions, independent of combat AI.

use std::collections::HashMap;

use sekiro_formats::{emedf::Arg, emevd::Instruction};

#[derive(Debug, Clone, Default)]
pub struct SceneState {
    pub enabled: HashMap<i32, bool>,
    pub animations: HashMap<i32, (i32, bool)>,
    pub animation_enabled: HashMap<i32, bool>,
    pub masks: HashMap<i32, HashMap<u8, bool>>,
    pub effects: HashMap<i32, bool>,
    pub lighting: Option<i32>,
    /// Region warps for supporting actors; bone-relative attachments need the actor rig.
    pub warps: HashMap<i32, i32>,
    pub backread_enabled: HashMap<i32, bool>,
}

impl SceneState {
    /// Returns true only for instructions fully consumed by the presentation state.
    /// Character enables are also retained here, but still reach the combat interpreter.
    pub fn instruction(&mut self, ins: &Instruction, args: &[Arg]) -> bool {
        let i = |n: usize| args.get(n).map_or(0, |v| v.as_i64());
        match ins.code() {
            (2004, 29) => {
                self.backread_enabled.insert(i(0) as i32, i(1) == 0);
                true
            }
            (2004, 41) if i(1) == 1 => {
                self.warps.insert(i(0) as i32, i(2) as i32);
                true
            }
            (2004, 5) => {
                self.enabled.insert(i(0) as i32, i(1) != 0);
                false
            }
            (2005, 3) | (2012, 1) => {
                self.enabled.insert(i(0) as i32, i(1) != 0);
                true
            }
            (2003, 18) => {
                self.animations
                    .insert(i(0) as i32, (i(1) as i32, i(2) != 0));
                true
            }
            (2004, 39) => {
                self.animation_enabled.insert(i(0) as i32, i(1) != 0);
                true
            }
            (2004, 26) => {
                self.masks
                    .entry(i(0) as i32)
                    .or_default()
                    .insert(i(1) as u8, i(2) != 0);
                true
            }
            (2006, 1) => {
                self.effects.insert(i(0) as i32, false);
                true
            }
            (2006, 2) => {
                self.effects.insert(i(0) as i32, true);
                true
            }
            (2003, 81) => {
                self.lighting = Some(i(0) as i32);
                true
            }
            _ => false,
        }
    }
}

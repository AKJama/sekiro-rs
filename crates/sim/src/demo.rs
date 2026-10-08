//! Scripted input: a tiny text format for repeatable play sessions, and a built-in demo.
//!
//! One line per segment: `<frames> <stick_x> <stick_y> <buttons> [camera_yaw_degrees]`.
//! Buttons are a comma list of `attack, guard, jump, dodge, crouch, grapple, prosthetic, item,
//! combat_art, lock_on`, or `-` for none. `#` starts a comment.

use crate::input::{Buttons, InputFrame};

/// Run, sprint, jump, a three-hit combo, a held guard, a step and a crouch, at 60 steps/s.
pub const DEMO: &str = "\
# idle
30 0 0 -
# run forward, then turn right while running
60 0 1 -
40 1 0 -
# sprint: hold dodge while running
1 0 1 dodge
50 0 1 dodge
20 0 0 -
# jump forward and land
1 0 1 jump
80 0 0 -
# three attacks: press, release, press again in the combo window
1 0 0 attack
25 0 0 -
1 0 0 attack
25 0 0 -
1 0 0 attack
70 0 0 -
# hold guard, then release
1 0 0 guard
60 0 0 guard
50 0 0 -
# step dodge to the left (holding the direction, as a player would)
1 -1 0 dodge
25 -1 0 -
40 0 0 -
# crouch toggle
1 0 0 crouch
40 0 0 -
1 0 0 crouch
40 0 0 -
";

fn parse_buttons(s: &str) -> Result<Buttons, String> {
    let mut b = Buttons::default();
    if s == "-" {
        return Ok(b);
    }
    for name in s.split(',') {
        match name.trim() {
            "attack" => b.attack = true,
            "guard" => b.guard = true,
            "jump" => b.jump = true,
            "dodge" => b.dodge = true,
            "crouch" => b.crouch = true,
            "grapple" => b.grapple = true,
            "prosthetic" => b.prosthetic = true,
            "item" => b.item = true,
            "combat_art" => b.combat_art = true,
            "lock_on" => b.lock_on = true,
            other => return Err(format!("unknown button {other:?}")),
        }
    }
    Ok(b)
}

/// Expands a script into one [`InputFrame`] per step.
pub fn parse(text: &str) -> Result<Vec<InputFrame>, String> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            return Err(format!("line {}: expected frames x y buttons", n + 1));
        }
        let num = |s: &str| s.parse::<f32>().map_err(|e| format!("line {}: {e}", n + 1));
        let frames = parts[0]
            .parse::<usize>()
            .map_err(|e| format!("line {}: {e}", n + 1))?;
        let frame = InputFrame {
            move_stick: [num(parts[1])?, num(parts[2])?],
            camera_yaw: parts
                .get(4)
                .map(|s| num(s))
                .transpose()?
                .unwrap_or(0.0)
                .to_radians(),
            buttons: parse_buttons(parts[3]).map_err(|e| format!("line {}: {e}", n + 1))?,
        };
        out.extend(std::iter::repeat_n(frame, frames));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    #[test]
    fn demo_parses() {
        let frames = super::parse(super::DEMO).unwrap();
        assert!(frames.len() > 600);
        assert!(frames[30].move_stick[1] == 1.0);
        assert!(super::parse("3 0 0 kick").is_err());
    }
}

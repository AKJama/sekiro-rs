//! `--input FILE`: replays raw keyboard and mouse input into Bevy's input messages, so a run
//! exercises the same path as a player at the keyboard (key state, accumulated mouse motion,
//! cursor grab, camera orbit, stick building) instead of the simulation-level `--script`.
//!
//! One line per segment of rendered frames: `<frames> <keys> [<mouse_dx> <mouse_dy>]`, where
//! keys is a comma list of `W A S D Q E Z X C V Space Shift Esc Enter Up Down Left Right F3 F8
//! Click MMB` (`Click` is the left mouse button, `MMB` the middle one, the game's lock-on; a key
//! named in consecutive segments stays held) or
//! `-`, and the mouse moves by the given counts every frame. `#` starts a comment. Frames count
//! from the first frame the simulation runs. A line `shot <path>` saves a screenshot there when
//! the replay gets to it, so a run can capture screens that pause the simulation (the in-game
//! menu), which `--shots` (counted in simulation steps) cannot.
//!
//! The replay owns the input: real keyboard, mouse button and mouse motion messages are dropped,
//! so someone using the machine meanwhile does not disturb the run. The run starts with the
//! cursor free, as after Esc; a `Click` takes it, as a player clicking into the window does.
//!
//! Every frame is logged to stderr as a `cam` line (frame, focus, grab mode, mouse delta,
//! lock-on, camera yaw and pitch, stick) and every simulation step as a `move` line (see
//! `play.rs`).

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::{MouseButtonInput, MouseMotion};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// One segment of replayed input.
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub frames: usize,
    /// A screenshot saved where the segment starts (a `shot` line, which has no frames).
    pub shot: Option<std::path::PathBuf>,
    pub keys: Vec<KeyCode>,
    /// Mouse buttons held (`Click` left, `MMB` middle).
    pub buttons: Vec<MouseButton>,
    pub mouse: Vec2,
}

fn button(name: &str) -> Option<MouseButton> {
    match name {
        "Click" => Some(MouseButton::Left),
        "MMB" => Some(MouseButton::Middle),
        _ => None,
    }
}

fn key(name: &str) -> Result<KeyCode, String> {
    Ok(match name {
        "W" => KeyCode::KeyW,
        "A" => KeyCode::KeyA,
        "S" => KeyCode::KeyS,
        "D" => KeyCode::KeyD,
        "Q" => KeyCode::KeyQ,
        "C" => KeyCode::KeyC,
        "Z" => KeyCode::KeyZ,
        "X" => KeyCode::KeyX,
        "V" => KeyCode::KeyV,
        "Space" => KeyCode::Space,
        "Shift" => KeyCode::ShiftLeft,
        "E" => KeyCode::KeyE,
        "Esc" => KeyCode::Escape,
        "Enter" => KeyCode::Enter,
        "Up" => KeyCode::ArrowUp,
        "Down" => KeyCode::ArrowDown,
        "Left" => KeyCode::ArrowLeft,
        "Right" => KeyCode::ArrowRight,
        "F8" => KeyCode::F8,
        "F3" => KeyCode::F3,
        other => return Err(format!("unknown key {other:?}")),
    })
}

pub fn parse(text: &str) -> Result<Vec<Segment>, String> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let err = |e: String| format!("line {}: {e}", n + 1);
        if let Some(path) = line.strip_prefix("shot ") {
            out.push(Segment {
                frames: 0,
                shot: Some(path.trim().into()),
                keys: Vec::new(),
                buttons: Vec::new(),
                mouse: Vec2::ZERO,
            });
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 2 && parts.len() != 4 {
            return Err(err("expected frames keys [dx dy]".into()));
        }
        let frames = parts[0].parse().map_err(|e| err(format!("{e}")))?;
        let names: Vec<&str> = match parts[1] {
            "-" => Vec::new(),
            list => list.split(',').collect(),
        };
        let buttons = names.iter().filter_map(|n| button(n)).collect();
        let keys = names
            .into_iter()
            .filter(|n| button(n).is_none())
            .map(key)
            .collect::<Result<_, _>>()
            .map_err(err)?;
        let num = |s: &str| s.parse::<f32>().map_err(|e| err(format!("{e}")));
        let mouse = match parts.get(2..4) {
            Some([x, y]) => Vec2::new(num(x)?, num(y)?),
            _ => Vec2::ZERO,
        };
        out.push(Segment {
            frames,
            shot: None,
            keys,
            buttons,
            mouse,
        });
    }
    Ok(out)
}

/// The replay and how far it has got.
#[derive(Resource)]
pub struct InputReplay {
    segments: Vec<Segment>,
    /// Frames replayed so far.
    pub frame: usize,
    held: Vec<KeyCode>,
    held_buttons: Vec<MouseButton>,
    /// Screenshots due and not yet taken: the window takes one a frame.
    shots: Vec<std::path::PathBuf>,
    /// Set by the game once the simulation runs; frames count from there.
    pub running: bool,
}

impl InputReplay {
    pub fn new(segments: Vec<Segment>) -> Self {
        Self {
            segments,
            frame: 0,
            held: Vec::new(),
            held_buttons: Vec::new(),
            shots: Vec::new(),
            running: false,
        }
    }

    pub fn total_frames(&self) -> usize {
        self.segments.iter().map(|s| s.frames).sum()
    }

    pub fn done(&self) -> bool {
        self.frame >= self.total_frames()
    }

    fn segment_at(&self, frame: usize) -> Option<&Segment> {
        let mut start = 0;
        for s in &self.segments {
            if frame < start + s.frames {
                return Some(s);
            }
            start += s.frames;
        }
        None
    }

    /// The screenshots due at `frame`.
    fn shots_at(&self, frame: usize) -> impl Iterator<Item = &std::path::PathBuf> {
        self.segments
            .iter()
            .scan(0, |start, s| {
                let at = *start;
                *start += s.frames;
                Some((at, s))
            })
            .filter(move |(at, _)| *at == frame)
            .filter_map(|(_, s)| s.shot.as_ref())
    }
}

/// Drops the real input of this frame and writes the replayed key and button changes and mouse
/// motion instead. Runs before Bevy's input systems, so `ButtonInput` and
/// `AccumulatedMouseMotion` see them this frame.
pub fn inject(
    mut commands: Commands,
    mut replay: ResMut<InputReplay>,
    window: Query<Entity, With<PrimaryWindow>>,
    mut keys: ResMut<Messages<KeyboardInput>>,
    mut buttons: ResMut<Messages<MouseButtonInput>>,
    mut motion: ResMut<Messages<MouseMotion>>,
) {
    keys.clear();
    buttons.clear();
    motion.clear();
    if !replay.running {
        return;
    }
    let Ok(window) = window.single() else {
        return;
    };
    let frame = replay.frame;
    let due: Vec<_> = replay.shots_at(frame).cloned().collect();
    replay.shots.extend(due);
    if !replay.shots.is_empty() {
        let path = replay.shots.remove(0);
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(path));
    }
    let (want, want_buttons, mouse) = match replay.segment_at(frame) {
        Some(s) => (s.keys.clone(), s.buttons.clone(), s.mouse),
        None => (Vec::new(), Vec::new(), Vec2::ZERO),
    };
    let message = |key_code: KeyCode, state: ButtonState| KeyboardInput {
        key_code,
        logical_key: Key::Unidentified(NativeKey::Unidentified),
        state,
        text: None,
        repeat: false,
        window,
    };
    for k in replay.held.iter().filter(|k| !want.contains(k)) {
        keys.write(message(*k, ButtonState::Released));
    }
    for k in want.iter().filter(|k| !replay.held.contains(k)) {
        keys.write(message(*k, ButtonState::Pressed));
    }
    replay.held = want;
    let button = |button: MouseButton, state: ButtonState| MouseButtonInput {
        button,
        state,
        window,
    };
    for b in replay
        .held_buttons
        .iter()
        .filter(|b| !want_buttons.contains(b))
    {
        buttons.write(button(*b, ButtonState::Released));
    }
    for b in want_buttons
        .iter()
        .filter(|b| !replay.held_buttons.contains(b))
    {
        buttons.write(button(*b, ButtonState::Pressed));
    }
    replay.held_buttons = want_buttons;
    if mouse != Vec2::ZERO {
        motion.write(MouseMotion { delta: mouse });
    }
    replay.frame += 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_keys_clicks_and_mouse() {
        let s = parse("# x\n10 W,D,Click,MMB 3 -1\n5 -\n").unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].keys, vec![KeyCode::KeyW, KeyCode::KeyD]);
        assert_eq!(s[0].buttons, vec![MouseButton::Left, MouseButton::Middle]);
        assert!(s[1].buttons.is_empty());
        assert_eq!(s[0].mouse, Vec2::new(3.0, -1.0));
        assert_eq!(s[1].mouse, Vec2::ZERO);
        assert_eq!(parse("3 X").unwrap()[0].keys, vec![KeyCode::KeyX]);
        assert!(parse("3 Unknown").is_err());
        let s = parse(
            "4 -
shot a.png
2 X
",
        )
        .unwrap();
        let replay = InputReplay::new(s);
        assert_eq!(replay.total_frames(), 6);
        assert_eq!(replay.shots_at(4).count(), 1);
        assert_eq!(replay.shots_at(3).count(), 0);
    }
}

//! Start and pause panels. Relaunch after App drops keeps shutdown identical to Quit.
use bevy::app::AppExit;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::{CursorGrabMode, CursorOptions};
use std::path::PathBuf;
use std::sync::Mutex;

static NEXT: Mutex<Option<Vec<String>>> = Mutex::new(None);
pub fn relaunch() -> anyhow::Result<()> {
    if let Some(args) = NEXT.lock().unwrap().take() {
        std::process::Command::new(std::env::current_exe()?)
            .args(args)
            .spawn()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn equivalent(choices: Choices, flags: &str) {
        let mut menu = vec!["sekiro-rs".into()];
        menu.extend(choices.args());
        let mut cli: Vec<String> = std::iter::once("sekiro-rs")
            .chain(flags.split_whitespace())
            .map(String::from)
            .collect();
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        for args in [&mut menu, &mut cli] {
            args.extend([
                "--cache".into(),
                root.join("cache").to_string_lossy().into_owned(),
            ]);
        }
        let a = crate::play_options_with(&menu, &root.join("logs/datasets")).unwrap();
        let b = crate::play_options_with(&cli, &root.join("logs/datasets")).unwrap();
        assert_eq!(a.opponent, b.opponent);
        assert_eq!(a.arena.map(|v| v.name), b.arena.map(|v| v.name));
        assert_eq!((a.map, a.start, a.hour), (b.map, b.start, b.hour));
        assert_eq!(
            (a.enemy, a.god, a.immortal, a.attack_power),
            (b.enemy, b.god, b.immortal, b.attack_power)
        );
        assert_eq!(
            (a.brain.name, a.brain_label, a.brain_overlay, a.player),
            (b.brain.name, b.brain_label, b.brain_overlay, b.player)
        );
        assert_eq!(
            (a.window_size, a.coach_interval),
            (b.window_size, b.coach_interval)
        );
        assert_eq!(
            (
                a.cache,
                a.script,
                a.input,
                a.screenshots,
                a.exit_after_script
            ),
            (
                b.cache,
                b.script,
                b.input,
                b.screenshots,
                b.exit_after_script
            )
        );
        assert_eq!(
            (
                a.full_map,
                a.hide,
                a.objects,
                a.lock_on,
                a.simple_ai,
                a.force_act,
                a.break_posture,
                a.brain_log
            ),
            (
                b.full_map,
                b.hide,
                b.objects,
                b.lock_on,
                b.simple_ai,
                b.force_act,
                b.break_posture,
                b.brain_log
            )
        );
        assert_eq!(
            (a.spawn, a.enemy_distance, a.gparam),
            (b.spawn, b.enemy_distance, b.gparam)
        );
        assert_eq!((a.brain.model, a.brain.chr), (b.brain.model, b.brain.chr));
        assert_eq!(
            (
                a.brain.coach.backend,
                a.brain.coach.model,
                a.brain.coach.effort,
                a.brain.coach.budget_usd,
                a.brain.coach.token_budget
            ),
            (
                b.brain.coach.backend,
                b.brain.coach.model,
                b.brain.coach.effort,
                b.brain.coach.budget_usd,
                b.brain.coach.token_budget
            )
        );
        match (a.brain_memory, b.brain_memory) {
            (Some(a), Some(b)) => assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&std::fs::read(a).unwrap()).unwrap(),
                serde_json::from_slice::<serde_json::Value>(&std::fs::read(b).unwrap()).unwrap()
            ),
            (None, None) => {}
            _ => panic!("memory selection differs"),
        }
    }
    #[test]
    fn soldier_flat_matches_cli() {
        equivalent(
            Choices::default(),
            "--play --enemy soldier --progression castle-roof",
        );
    }
    #[test]
    fn owl_coach_alive_overlay_matches_cli() {
        let mut c = Choices::default();
        for (i, v) in [
            (0, "Great Shinobi Owl"),
            (2, "Arena"),
            (5, "Coach brain (codex)"),
            (6, "alive"),
            (7, "on"),
        ] {
            c.values[i] = v.into();
        }
        equivalent(
            c,
            "--arena owl --brain coach --brain-stage alive --brain-overlay --progression castle-roof",
        );
    }
    #[test]
    fn map_start_hour_no_enemy_matches_cli() {
        let mut c = Choices::default();
        for (i, v) in [
            (0, "No enemy"),
            (2, "Map + start"),
            (3, "m11_01"),
            (4, "2"),
            (9, "18"),
        ] {
            c.values[i] = v.into();
        }
        equivalent(
            c,
            "--play --map m11_01 --start 2 --hour 18 --no-enemy --progression castle-roof",
        );
    }
    #[test]
    fn genichiro_uses_arena_registry_enemy() {
        let mut c = Choices::default();
        c.values[0] = "Genichiro".into();
        c.values[2] = "Arena".into();
        equivalent(c, "--arena genichiro --progression castle-roof");
    }
}

const MAPS: [&str; 10] = [
    "m10_00",
    "m11_00",
    "m11_01",
    "m11_02",
    "m13_00",
    "m15_00",
    "m17_00",
    "m20_00",
    "m25_00",
    "m89_00_00_99",
];
const LABELS: [&str; 14] = [
    "Fight",
    "Enemy id",
    "Place",
    "Map",
    "Start number",
    "AI",
    "Brain stage (Owl)",
    "Overlay",
    "Progression / AP",
    "Hour",
    "Player",
    "Player name",
    "",
    "Start",
];

#[derive(Clone, Debug, PartialEq)]
pub struct Choices {
    values: Vec<String>,
}
impl Default for Choices {
    fn default() -> Self {
        Self {
            values: [
                "Ashina soldier",
                "c1010",
                "Flat floor",
                "m11_01",
                "0",
                "Lua",
                "none",
                "off",
                "castle-roof",
                "12",
                "normal",
                "default",
                "",
                "",
            ]
            .map(String::from)
            .to_vec(),
        }
    }
}
impl Choices {
    fn enabled(&self, row: usize) -> bool {
        match row {
            1 => self.values[0] == "Any enemy id",
            3 | 4 => self.values[2] == "Map + start",
            6 => self.owl(),
            _ => true,
        }
    }
    fn owl(&self) -> bool {
        self.values[0] == "Great Shinobi Owl"
            || (self.values[0] == "Any enemy id"
                && self.values[1]
                    .split(':')
                    .next()
                    .is_some_and(|v| v.eq_ignore_ascii_case("c5060")))
    }
    pub fn args(&self) -> Vec<String> {
        let v = &self.values;
        let mut args = vec!["--play".into()];
        let mut pair = |flag: &str, value: &str| {
            args.extend([flag.into(), value.into()]);
        };
        let enemy = match v[0].as_str() {
            "Great Shinobi Owl" => "owl",
            "Genichiro" => {
                sekiro_sim::arena::ArenaSpec::by_name("genichiro")
                    .expect("supported arena")
                    .enemy
            }
            "Any enemy id" => &v[1],
            _ => "soldier",
        };
        pair("--enemy", enemy);
        let arena = if self.owl() {
            Some("owl")
        } else if enemy
            .split(':')
            .next()
            .is_some_and(|v| v.eq_ignore_ascii_case("c7100"))
        {
            Some("genichiro")
        } else {
            None
        };
        if v[2] == "Arena"
            && let Some(arena) = arena
        {
            pair("--arena", arena);
        }
        if v[2] == "Map + start" {
            pair("--map", &v[3]);
            pair("--start", &v[4]);
        }
        pair(
            "--brain",
            match v[5].as_str() {
                "Coach brain (codex)" => "coach",
                "Scripted test plans" => "scripted",
                _ => "lua",
            },
        );
        if self.owl() && v[6] != "none" {
            pair("--brain-stage", &v[6]);
        }
        pair(
            if v[8] == "castle-roof" {
                "--progression"
            } else {
                "--attack-power"
            },
            &v[8],
        );
        pair("--hour", &v[9]);
        pair("--player", &v[11]);
        if v[0] == "No enemy" {
            args.push("--no-enemy".into());
        }
        if v[7] == "on" {
            args.push("--brain-overlay".into());
        }
        if v[10] != "normal" {
            args.push(format!("--{}", v[10]));
        }
        args
    }
    fn change(&mut self, row: usize, delta: i32) {
        if delta == 0 || !self.enabled(row) {
            return;
        }
        let list: &[&str] = match row {
            0 => &[
                "Ashina soldier",
                "Great Shinobi Owl",
                "Genichiro",
                "Any enemy id",
                "No enemy",
            ],
            2 => &["Arena", "Flat floor", "Map + start"],
            3 => &MAPS,
            5 => &["Lua", "Coach brain (codex)", "Scripted test plans"],
            6 if self.owl() => &["none", "lua", "early", "alive", "outsmarted"],
            7 => &["off", "on"],
            10 => &["normal", "god", "immortal"],
            _ => &[],
        };
        if !list.is_empty() {
            let index = list
                .iter()
                .position(|v| *v == self.values[row])
                .unwrap_or(0) as i32;
            self.values[row] = list[(index + delta).rem_euclid(list.len() as i32) as usize].into();
        } else if row == 1 {
            let old = self.values[row]
                .trim_start_matches(['c', 'C'])
                .split(':')
                .next()
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(1010);
            self.values[row] = format!("c{:04}", (old + delta).clamp(0, 9999));
        } else if matches!(row, 4 | 8 | 9) {
            let old = self.values[row].parse::<i32>().unwrap_or(6);
            let (min, max) = match row {
                4 => (0, 99),
                9 => (0, 23),
                _ => (1, 99),
            };
            self.values[row] = if row == 8 && old == 1 && delta < 0 {
                "castle-roof".into()
            } else {
                (old + delta).clamp(min, max).to_string()
            };
        }
    }
    fn path() -> Option<PathBuf> {
        std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("XDG_CONFIG_HOME"))
            .map(|p| PathBuf::from(p).join("sekiro-rs/menu.json"))
    }
    fn load() -> Self {
        let mut result = Self::default();
        if let Some(v) = Self::path()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice::<Vec<String>>(&b).ok())
            && v.len() == LABELS.len()
        {
            result.values = v;
        }
        result
    }
    fn save(&self) {
        if let Some(path) = Self::path() {
            let _ = std::fs::create_dir_all(path.parent().unwrap());
            if let Ok(bytes) = serde_json::to_vec(&self.values) {
                let _ = std::fs::write(path, bytes);
            }
        }
    }
}

#[derive(Resource)]
pub struct PanelState {
    pub paused: bool,
    start: bool,
    choices: Choices,
    selected: usize,
    error: String,
    extras: Vec<String>,
    fight_script: Option<String>,
    return_script: Option<String>,
    frame: usize,
    script: Vec<(usize, String)>,
    stick: IVec2,
}
impl PanelState {
    fn new(start: bool, args: &[String]) -> Self {
        let script = crate::arg_value(args, "--menu-script")
            .and_then(|p| std::fs::read_to_string(p).ok())
            .unwrap_or_default()
            .lines()
            .filter_map(|l| {
                let (n, action) = l.split_once(' ')?;
                Some((n.parse().ok()?, action.to_string()))
            })
            .collect();
        let mut extras = Vec::new();
        for flag in ["--cache", "--window-size", "--script", "--shots", "--input"] {
            if let Some(v) = crate::arg_value(args, flag) {
                extras.extend([flag.to_string(), v]);
            }
        }
        for flag in ["--simple-ai", "--exit"] {
            if args.iter().any(|a| a == flag) {
                extras.push(flag.into());
            }
        }
        let choices = crate::arg_value(args, "--menu-choices")
            .and_then(|v| serde_json::from_str::<Vec<String>>(&v).ok())
            .filter(|v| v.len() == LABELS.len())
            .map(|values| Choices { values })
            .unwrap_or_else(Choices::load);
        Self {
            paused: start,
            start,
            choices,
            selected: 0,
            error: String::new(),
            extras,
            fight_script: crate::arg_value(args, "--menu-fight-script"),
            return_script: crate::arg_value(args, "--menu-return-script"),
            frame: 0,
            script,
            stick: IVec2::ZERO,
        }
    }
}
#[derive(Component)]
struct Panel;
#[derive(Component)]
struct Row(usize);
#[derive(Component)]
struct Help;

pub fn active(state: Res<PanelState>) -> bool {
    !state.paused
}
pub fn install(app: &mut App) {
    let args: Vec<_> = std::env::args().collect();
    app.insert_resource(PanelState::new(false, &args))
        .add_systems(Startup, spawn)
        .add_systems(PreUpdate, update.after(bevy::input::InputSystems));
}
pub fn run(args: &[String]) -> anyhow::Result<()> {
    let size = crate::arg_value(args, "--window-size").unwrap_or_else(|| "1600x900".into());
    let (w, h) = size
        .split_once('x')
        .ok_or_else(|| anyhow::anyhow!("--window-size wants WIDTHxHEIGHT"))?;
    let size: (u32, u32) = (w.parse()?, h.parse()?);
    anyhow::ensure!(
        size.0 >= 800 && size.1 >= 600,
        "--window-size minimum is 800x600"
    );
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Sekiro-rs".into(),
            resolution: size.into(),
            ..default()
        }),
        ..default()
    }))
    .insert_resource(ClearColor(Color::srgb(0.035, 0.042, 0.055)))
    .insert_resource(PanelState::new(true, args))
    .add_systems(
        Startup,
        (
            |mut commands: Commands| {
                commands.spawn(Camera2d);
            },
            spawn,
        ),
    )
    .add_systems(Update, update)
    .add_systems(Last, save_on_exit);
    app.run();
    Ok(())
}
fn save_on_exit(state: Res<PanelState>, mut exits: MessageReader<AppExit>) {
    if exits.read().next().is_some() {
        state.choices.save();
    }
}
fn spawn(mut commands: Commands, mut fonts: ResMut<Assets<Font>>, state: Res<PanelState>) {
    let font = std::fs::read("C:/Windows/Fonts/segoeui.ttf")
        .ok()
        .map(|b| bevy::text::FontSource::Handle(fonts.add(Font::from_bytes(b))))
        .unwrap_or_default();
    commands
        .spawn((
            Panel,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            GlobalZIndex(100),
            if state.paused {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: px(690),
                    max_width: percent(94),
                    padding: UiRect::all(px(24)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.025, 0.028, 0.035, 0.94)),
            ))
            .with_children(|p| {
                p.spawn((
                    Text::new("Sekiro-rs"),
                    TextFont {
                        font: font.clone(),
                        font_size: bevy::text::FontSize::Px(32.),
                        ..default()
                    },
                    TextColor(Color::srgb(0.91, 0.78, 0.54)),
                    Node {
                        margin: UiRect::bottom(px(12)),
                        ..default()
                    },
                ));
                for i in 0..if state.start { LABELS.len() } else { 3 } {
                    p.spawn((
                        Button,
                        Row(i),
                        Node {
                            width: percent(100),
                            min_height: px(30),
                            padding: UiRect::axes(px(10), px(3)),
                            ..default()
                        },
                    ))
                    .with_children(|r| {
                        r.spawn((
                            Text::new(""),
                            TextFont {
                                font: font.clone(),
                                font_size: bevy::text::FontSize::Px(18.),
                                ..default()
                            },
                        ));
                    });
                }
                p.spawn((
                    Help,
                    Text::new(""),
                    TextFont {
                        font,
                        font_size: bevy::text::FontSize::Px(14.),
                        ..default()
                    },
                    TextColor(Color::srgb(0.68, 0.71, 0.75)),
                    Node {
                        margin: UiRect::top(px(12)),
                        ..default()
                    },
                ));
            });
        });
}

#[allow(clippy::too_many_arguments)]
fn update(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    mut typing: MessageReader<KeyboardInput>,
    mut state: ResMut<PanelState>,
    mut panels: Query<&mut Visibility, With<Panel>>,
    mut rows: Query<(&Row, Ref<Interaction>, &mut BackgroundColor, &Children)>,
    mut texts: Query<(&mut Text, &mut TextColor)>,
    help: Query<Entity, With<Help>>,
    mut cursors: Query<&mut CursorOptions>,
    mut exit: MessageWriter<AppExit>,
    mut time: ResMut<Time<Virtual>>,
) {
    state.frame += 1;
    let actions: Vec<_> = state
        .script
        .iter()
        .filter(|(n, _)| *n == state.frame)
        .map(|(_, a)| a.clone())
        .collect();
    let mut activate = keys.just_pressed(KeyCode::Enter);
    if !state.start
        && (keys.just_pressed(KeyCode::Escape)
            || actions.iter().any(|a| a == "pause")
            || pads.iter().any(|p| p.just_pressed(GamepadButton::Start)))
    {
        state.paused = !state.paused;
        state.selected = 0;
        if state.paused {
            time.pause();
        } else {
            time.unpause();
        }
        for mut c in &mut cursors {
            c.visible = state.paused;
            c.grab_mode = if state.paused {
                CursorGrabMode::None
            } else {
                CursorGrabMode::Locked
            };
        }
    }
    for mut p in &mut panels {
        *p = if state.paused {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for a in &actions {
        if let Some(path) = a.strip_prefix("shot:") {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(PathBuf::from(path)));
        }
        if a == "quit" {
            exit.write(AppExit::Success);
        }
    }
    if !state.paused {
        typing.clear();
        return;
    }
    let count = if state.start { LABELS.len() } else { 3 };
    let mut movement = IVec2::new(
        i32::from(keys.just_pressed(KeyCode::ArrowRight))
            - i32::from(keys.just_pressed(KeyCode::ArrowLeft)),
        i32::from(keys.just_pressed(KeyCode::ArrowDown))
            - i32::from(keys.just_pressed(KeyCode::ArrowUp)),
    );
    let mut stick = IVec2::ZERO;
    for p in &pads {
        activate |= p.just_pressed(GamepadButton::South);
        movement.x += i32::from(p.just_pressed(GamepadButton::DPadRight))
            - i32::from(p.just_pressed(GamepadButton::DPadLeft));
        movement.y += i32::from(p.just_pressed(GamepadButton::DPadDown))
            - i32::from(p.just_pressed(GamepadButton::DPadUp));
        let s = p.left_stick();
        stick = IVec2::new(
            if s.x.abs() > 0.6 {
                s.x.signum() as i32
            } else {
                0
            },
            if s.y.abs() > 0.6 {
                -s.y.signum() as i32
            } else {
                0
            },
        );
    }
    if stick != state.stick {
        movement += stick;
    }
    state.stick = stick;
    for a in &actions {
        if let Some(setting) = a.strip_prefix("set:")
            && let Some((row, value)) = setting.split_once(':')
            && let Ok(row) = row.parse::<usize>()
            && row < 12
        {
            state.choices.values[row] = value.into();
        }
        match a.as_str() {
            "down" => movement.y += 1,
            "up" => movement.y -= 1,
            "right" => movement.x += 1,
            "left" => movement.x -= 1,
            "start" => {
                state.selected = count - 1;
                activate = true;
            }
            "back" => {
                state.selected = 1;
                activate = true;
            }
            _ => {}
        }
    }
    state.selected = (state.selected as i32 + movement.y).rem_euclid(count as i32) as usize;
    if state.start && state.selected == 12 {
        state.selected = if movement.y < 0 { 11 } else { 13 };
    }
    let row = state.selected;
    if state.start {
        state.choices.change(row, movement.x);
        if matches!(row, 1 | 11) && state.choices.enabled(row) {
            for input in typing.read().filter(|i| i.state.is_pressed() && !i.repeat) {
                if input.key_code == KeyCode::Backspace {
                    state.choices.values[row].pop();
                }
                if let Key::Character(s) = &input.logical_key {
                    for c in s
                        .chars()
                        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | ':'))
                    {
                        if state.choices.values[row].len() < 40 {
                            state.choices.values[row].push(c);
                        }
                    }
                }
            }
        } else {
            typing.clear();
        }
    }
    for (r, interaction, mut bg, children) in &mut rows {
        if interaction.is_changed() && *interaction == Interaction::Pressed {
            state.selected = r.0;
            if state.start && r.0 < count - 1 {
                state.choices.change(r.0, 1);
            } else {
                activate = true;
            }
        }
        let disabled = state.start && r.0 == 6 && !state.choices.owl();
        let muted = state.start && !state.choices.enabled(r.0);
        *bg = BackgroundColor(if r.0 == state.selected {
            Color::srgba(0.55, 0.41, 0.20, 0.40)
        } else {
            Color::NONE
        });
        for child in children.iter() {
            if let Ok((mut t, mut color)) = texts.get_mut(child) {
                color.0 = if muted {
                    Color::srgb(0.42, 0.44, 0.48)
                } else {
                    Color::srgb(0.92, 0.90, 0.85)
                };
                t.0 = if state.start {
                    if r.0 == count - 1 {
                        "Start".into()
                    } else if r.0 == 12 {
                        state.error.clone()
                    } else {
                        format!(
                            "{}:  {}",
                            LABELS[r.0],
                            if disabled {
                                "unavailable"
                            } else {
                                &state.choices.values[r.0]
                            }
                        )
                    }
                } else {
                    ["Resume", "Back to start menu", "Quit"][r.0].into()
                };
            }
        }
    }
    for e in &help {
        if let Ok((mut t, _)) = texts.get_mut(e) {
            t.0 = if state.start {
                "Up/Down: select   Left/Right or click: change   Enter/A: start   Type: id/name"
            } else {
                "Up/Down: select   Enter/A or click: confirm   Esc/Start: resume"
            }
            .into();
        }
    }
    if activate {
        if state.start {
            let mut args = state.choices.args();
            args.extend(state.extras.clone());
            args.extend([
                "--menu-choices".into(),
                serde_json::to_string(&state.choices.values).unwrap(),
            ]);
            if let Some(script) = &state.fight_script {
                args.extend(["--menu-script".into(), script.clone()]);
            }
            if let Some(script) = &state.return_script {
                args.extend(["--menu-return-script".into(), script.clone()]);
            }
            let mut cli = vec!["sekiro-rs".into()];
            cli.extend(args.clone());
            let validated = crate::play_options(&cli).and_then(|options| {
                if let Some(id) = options.map.as_deref().filter(|_| options.arena.is_none()) {
                    let dir = crate::map::map_dir(&options.cache, id)?;
                    crate::map::player_start(&dir, options.start)?;
                }
                Ok(options)
            });
            match validated {
                Ok(_) => {
                    state.choices.save();
                    *NEXT.lock().unwrap() = Some(args);
                    exit.write(AppExit::Success);
                }
                Err(e) => state.error = e.to_string(),
            }
        } else if !state.start {
            match state.selected {
                0 => {
                    state.paused = false;
                    time.unpause();
                    for mut c in &mut cursors {
                        c.visible = false;
                        c.grab_mode = CursorGrabMode::Locked;
                    }
                }
                1 => {
                    let mut args = vec![
                        "--menu".into(),
                        "--menu-choices".into(),
                        serde_json::to_string(&state.choices.values).unwrap(),
                    ];
                    for flag in ["--window-size", "--cache"] {
                        if let Some(value) = crate::arg_value(&state.extras, flag) {
                            args.extend([flag.into(), value]);
                        }
                    }
                    if let Some(script) = &state.return_script {
                        args.extend(["--menu-script".into(), script.clone()]);
                    }
                    *NEXT.lock().unwrap() = Some(args);
                    exit.write(AppExit::Success);
                }
                _ => {
                    exit.write(AppExit::Success);
                }
            }
        }
    }
}

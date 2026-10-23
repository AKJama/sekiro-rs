//! The start menu the game opens on (#8): the title art and New Game, Chapter and Exit, in the
//! play window itself. Picking an area hands its start to the play session
//! ([`crate::play::TitlePick`]), which loads behind the loading screen in the same window.
//!
//! The art is the game's own title atlas (`menu/hi/01_common.tpf`, `SB_Title`: the 隻狼 logo,
//! "Shadows" and "Die Twice" and the FromSoftware mark). The real title screen draws them over
//! a scene rendered in the engine; no title movie ships (`movie/` holds only the opening and
//! staff list), so the backdrop here is black.
//!
//! `--title-script FILE` drives it for tests: lines of `<frame> <action>`, with the actions
//! `up`, `down`, `ok`, `back` and `shot:<path>`.
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::ui_widgets::{Activate, ActivateOnPress, Button};

use crate::game_ui::{GameFonts, MenuArt};

/// The title screen's rows.
const MAIN_ROWS: [&str; 3] = ["New Game", "Chapter", "Exit"];
/// The text colours of a row: chosen and not.
const CHOSEN: Color = Color::srgb(0.93, 0.84, 0.62);
const UNCHOSEN: Color = Color::srgb(0.58, 0.56, 0.52);

/// Which list the menu shows and its cursor.
#[derive(Resource, Debug, Clone, PartialEq)]
pub(crate) struct Title {
    /// The Chapter list is open.
    chapter: bool,
    selected: usize,
    /// The map's player starts, as the Chapter list names them.
    starts: Vec<String>,
    frame: usize,
    script: Vec<(usize, String)>,
    /// Rows clicked this frame.
    clicked: Vec<usize>,
    picked: bool,
}

/// What a menu action does.
#[derive(Debug, PartialEq)]
enum Outcome {
    Stay,
    Pick(usize),
    Exit,
}

impl Title {
    fn new(starts: Vec<String>, script: Vec<(usize, String)>) -> Self {
        Self {
            chapter: false,
            selected: 0,
            starts,
            frame: 0,
            script,
            clicked: Vec::new(),
            picked: false,
        }
    }

    /// The rows of the list that is open.
    fn rows(&self) -> Vec<String> {
        if self.chapter {
            self.starts.clone()
        } else {
            MAIN_ROWS.map(String::from).to_vec()
        }
    }

    /// Moves the cursor by `delta` rows, wrapping.
    fn step(&mut self, delta: i32) {
        let count = self.rows().len().max(1) as i32;
        self.selected = (self.selected as i32 + delta).rem_euclid(count) as usize;
    }

    /// OK on the chosen row: New Game starts at the map's first start, Chapter opens its list,
    /// a chapter starts at its start.
    fn ok(&mut self) -> Outcome {
        if self.chapter {
            return Outcome::Pick(self.selected);
        }
        match self.selected {
            0 => Outcome::Pick(0),
            1 => {
                self.chapter = true;
                self.selected = 0;
                Outcome::Stay
            }
            _ => Outcome::Exit,
        }
    }

    /// Back closes the Chapter list onto its row.
    fn back(&mut self) {
        if self.chapter {
            self.chapter = false;
            self.selected = 1;
        }
    }
}

/// The map's player starts as Chapter rows: `Start <n>` and its MSB part.
fn chapter_rows(dir: &Path) -> Vec<String> {
    let players = std::fs::read(dir.join("layout.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|l| l["players"].as_array().cloned())
        .unwrap_or_default();
    let rows: Vec<_> = players
        .iter()
        .enumerate()
        .map(|(i, p)| match p["name"].as_str() {
            Some(name) => format!("Start {i}  ({name})"),
            None => format!("Start {i}"),
        })
        .collect();
    if rows.is_empty() {
        vec!["Start 0".into()]
    } else {
        rows
    }
}

fn read_script(path: Option<PathBuf>) -> Vec<(usize, String)> {
    path.and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let (n, action) = l.trim().split_once(' ')?;
            Some((n.parse().ok()?, action.to_string()))
        })
        .collect()
}

#[derive(Component)]
struct TitleCamera;
#[derive(Component)]
struct TitleRoot;
#[derive(Component)]
struct RowList;
#[derive(Component)]
struct TitleRow(usize);

/// Adds the start menu over the play app built by `crate::play::run_title`, with the
/// Chapter list of the map in `dir`.
pub(crate) fn install(app: &mut App, dir: &Path) {
    let args: Vec<String> = std::env::args().collect();
    let script = read_script(crate::arg_value(&args, "--title-script").map(PathBuf::from));
    GameFonts::insert(app);
    MenuArt::add(app, &["SB_Title"]);
    app.insert_resource(Title::new(chapter_rows(dir), script))
        .add_systems(Startup, spawn)
        .add_systems(
            PreUpdate,
            update
                .after(bevy::input::InputSystems)
                .after(bevy::picking::PickingSystems::Last),
        )
        .add_systems(Update, draw_rows);
}

/// Removes the menu's camera once the session's own takes over; the menu itself went when the
/// area was picked.
pub(crate) fn leave(world: &mut World) {
    let entities: Vec<Entity> = world
        .query_filtered::<Entity, Or<(With<TitleCamera>, With<TitleRoot>)>>()
        .iter(world)
        .collect();
    for e in entities {
        world.despawn(e);
    }
}

fn spawn(mut commands: Commands, fonts: Res<GameFonts>, art: Res<MenuArt>) {
    eprintln!(
        "title: first frame at {:.1} s after launch",
        crate::since_start()
    );
    commands.spawn((TitleCamera, Camera2d));
    let sprite = |name: &str, height: f32| {
        art.sprite("SB_Title", name).map(|(image, aspect)| {
            (
                image,
                Node {
                    height: percent(height),
                    aspect_ratio: Some(aspect),
                    ..default()
                },
            )
        })
    };
    commands
        .spawn((
            TitleRoot,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                padding: UiRect::top(percent(7)),
                ..default()
            },
            BackgroundColor(Color::BLACK),
            GlobalZIndex(300),
        ))
        .with_children(|root| {
            if let Some(logo) = sprite("MENU_Title_SEKIRO", 42.0) {
                root.spawn(logo);
            }
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: percent(2.5),
                height: percent(6),
                margin: UiRect::top(percent(2)),
                ..default()
            })
            .with_children(|line| {
                for name in ["MENU_Shadows", "MENU_DieTwice"] {
                    if let Some(words) = sprite(name, 100.0) {
                        line.spawn(words);
                    }
                }
            });
            root.spawn((
                RowList,
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: px(6),
                    margin: UiRect::top(percent(5)),
                    ..default()
                },
            ));
            if let Some(mark) = sprite("MENU_FROMSOFTWARE_Logo", 100.0) {
                root.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: percent(5),
                        height: percent(2.5),
                        ..default()
                    },
                    children![mark],
                ));
            }
            root.spawn((
                fonts.ui(14.0),
                Text::new("A fan project developed for Sekiro: Shadows Die Twice"),
                TextColor(Color::srgb(0.45, 0.44, 0.42)),
                Node {
                    position_type: PositionType::Absolute,
                    bottom: percent(1),
                    ..default()
                },
            ));
        });
}

fn click_row(event: On<Activate>, rows: Query<&TitleRow>, mut title: ResMut<Title>) {
    if let Ok(row) = rows.get(event.entity) {
        title.clicked.push(row.0);
    }
}

/// Rebuilds the rows when the list or the cursor changes.
fn draw_rows(
    mut commands: Commands,
    title: Res<Title>,
    fonts: Res<GameFonts>,
    lists: Query<(Entity, Option<&Children>), With<RowList>>,
    mut drawn: Local<Option<(bool, usize)>>,
) {
    if *drawn == Some((title.chapter, title.selected)) {
        return;
    }
    let Ok((list, children)) = lists.single() else {
        return;
    };
    *drawn = Some((title.chapter, title.selected));
    for child in children.into_iter().flatten() {
        commands.entity(*child).despawn();
    }
    let rows = title.rows();
    // The Chapter list shows a window of rows around the cursor, short enough to stay above
    // the FromSoftware mark.
    const SHOWN: usize = 5;
    let first = title
        .selected
        .saturating_sub(SHOWN / 2)
        .min(rows.len().saturating_sub(SHOWN));
    commands.entity(list).with_children(|list| {
        for (i, label) in rows.iter().enumerate().skip(first).take(SHOWN) {
            let chosen = i == title.selected;
            list.spawn((
                Button,
                ActivateOnPress,
                TitleRow(i),
                Node {
                    padding: UiRect::axes(px(18), px(2)),
                    ..default()
                },
            ))
            .observe(click_row)
            .with_child((
                Text::new(if chosen {
                    format!("-  {label}  -")
                } else {
                    label.clone()
                }),
                fonts.ui(if title.chapter { 22.0 } else { 26.0 }),
                TextColor(if chosen { CHOSEN } else { UNCHOSEN }),
            ));
        }
        if title.chapter {
            list.spawn((Text::new("Back: Esc"), fonts.ui(14.0), TextColor(UNCHOSEN)));
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn update(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    mut title: ResMut<Title>,
    roots: Query<Entity, With<TitleRoot>>,
    mut exit: MessageWriter<AppExit>,
) {
    title.frame += 1;
    let frame = title.frame;
    let actions: Vec<String> = title
        .script
        .iter()
        .filter(|(n, _)| *n == frame)
        .map(|(_, a)| a.clone())
        .collect();
    let pad = |b| pads.iter().any(|p| p.just_pressed(b));
    let mut delta = i32::from(
        keys.any_just_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) || pad(GamepadButton::DPadDown),
    ) - i32::from(
        keys.any_just_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) || pad(GamepadButton::DPadUp),
    );
    let mut ok = crate::menu::ok_pressed(&keys, &pads);
    let mut back = keys.just_pressed(KeyCode::Escape) || crate::menu::back_pressed(&keys, &pads);
    for a in &actions {
        match a.as_str() {
            "down" => delta += 1,
            "up" => delta -= 1,
            "ok" => ok = true,
            "back" => back = true,
            shot => {
                if let Some(path) = shot.strip_prefix("shot:") {
                    commands
                        .spawn(Screenshot::primary_window())
                        .observe(save_to_disk(PathBuf::from(path)));
                }
            }
        }
    }
    // After the pick only the script's screenshots still run (of the loading screen and play).
    if title.picked {
        return;
    }
    if let Some(row) = std::mem::take(&mut title.clicked).last() {
        title.selected = *row;
        ok = true;
    }
    if back {
        title.back();
    }
    if delta != 0 {
        title.step(delta);
    }
    if !ok {
        return;
    }
    match title.ok() {
        Outcome::Stay => {}
        Outcome::Exit => {
            eprintln!("title: exit");
            exit.write(AppExit::Success);
        }
        Outcome::Pick(start) => {
            title.picked = true;
            for root in &roots {
                commands.entity(root).despawn();
            }
            crate::play::spawn_loading_screen(commands.reborrow());
            commands.insert_resource(crate::play::TitlePick(start));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn title() -> Title {
        Title::new(
            vec!["Start 0".into(), "Start 1".into(), "Start 2".into()],
            vec![],
        )
    }

    #[test]
    fn new_game_starts_at_the_first_start() {
        assert_eq!(title().ok(), Outcome::Pick(0));
    }

    #[test]
    fn chapter_opens_the_starts_and_picks_one() {
        let mut t = title();
        t.step(1);
        assert_eq!(t.ok(), Outcome::Stay);
        assert!(t.chapter);
        assert_eq!(t.rows().len(), 3);
        t.step(2);
        assert_eq!(t.ok(), Outcome::Pick(2));
    }

    #[test]
    fn back_returns_to_the_chapter_row_and_exit_exits() {
        let mut t = title();
        t.step(1);
        t.ok();
        t.back();
        assert_eq!((t.chapter, t.selected), (false, 1));
        t.step(1);
        assert_eq!(t.ok(), Outcome::Exit);
        // The cursor wraps.
        t.step(1);
        assert_eq!(t.selected, 0);
    }
}

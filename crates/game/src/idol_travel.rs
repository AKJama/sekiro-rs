//! Travel between Sculptor's Idols (#148): the idol menu's Travel row opens the game's list of
//! the idols Wolf has found (`Kit::found_idols`), and picking one plays the warp
//! (`Duel::travel_to_idol`) under a fade to black.
//!
//! The list is BonfireWarpParam's: one page per `menuPageNo` (0 "Important Places", then the
//! areas, titled NTC_メニューテキスト 211100 + page, whose `<?placeName@n?>` is 地名 n), each
//! listing its rows' idols in row id order by name (`msgId`). Only pages with a found idol
//! show, and only found idols; an idol on two pages (the Dilapidated Temple on Important
//! Places and Ashina Outskirts) is listed on both. After FwLo's RustSekiro `src/idol.rs`
//! (`warp_rows`, `page_name`, `regions` and its Travel screen, laid out from captures of the
//! game): the regions in a column on the left, the idols of the picked one on a parchment on
//! the right.
//!
//! An idol of this play world is a teleport in the sim. One on another map ends this run and
//! starts the game again on that map (`crate::menu::relaunch_with`, the path the start menu's
//! map changes take), at the idol (`--at-idol`) with the kit carried over (`--kit`).
//! gap: the game streams the other map in the same run; the grey-out flags
//! (`grayoutEventflagId`), the page's area art (`menuImageId`) and Wolf's marker by his area
//! are not modelled, and the menu takes keys and the pad, not the mouse, as the idol menu does.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use bevy::app::AppExit;
use bevy::prelude::*;
use sekiro_sim::duel::Duel;
use sekiro_sim::duel::idol::{Destination, IdolPhase, WARP_CUT};

use super::Sim;
use super::idol::{CURSOR, DIM, GREYED, HELP, Idols, KEY_OK, TEXT, TRAVEL, WARP_POINT, int};
use crate::game_ui::{GameFonts, MenuArt, abs};

/// NTC_メニューテキスト: the screen's title ("Travel") and page 0's ("Important Places").
const TITLE: i32 = 211000;
const PAGES: i32 = 211100;
/// NTC_キーガイド: "Back".
const KEY_BACK: i32 = 110000;
/// Seconds the black takes to lift after an arrival (the death screen's lift).
const LIFT: f32 = 0.6;

/// One idol of the game's Travel list.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Dest {
    pub entity: i32,
    pub page: usize,
    pub name: String,
    /// Its map (`m11_00_00_00`), from the entity's area and block digits.
    pub map: String,
}

/// The game's Travel list: the page titles and every BonfireWarpParam row's idol in row order,
/// with the maps that are extracted.
#[derive(Debug, Default)]
pub(super) struct Atlas {
    pages: BTreeMap<usize, String>,
    dests: Vec<Dest>,
    maps: BTreeSet<String>,
}

impl Atlas {
    /// From BonfireWarpParam `rows` in id order and the menu texts `names`.
    pub fn load<'a>(
        cache: &Path,
        rows: impl IntoIterator<Item = &'a serde_json::Value>,
        names: &BTreeMap<i32, String>,
    ) -> Self {
        let places = crate::game_ui::texts(cache, "item", "地名");
        let mut atlas = Self::from_rows(rows, names, &places);
        atlas.maps = atlas
            .dests
            .iter()
            .map(|d| d.map.clone())
            .filter(|m| cache.join("maps").join(m).join("layout.json").is_file())
            .collect();
        atlas
    }

    fn from_rows<'a>(
        rows: impl IntoIterator<Item = &'a serde_json::Value>,
        names: &BTreeMap<i32, String>,
        places: &BTreeMap<i32, String>,
    ) -> Self {
        let mut atlas = Self::default();
        for r in rows {
            let (Some(entity), Some(page)) = (
                int(&r["bonfireEntityId"]).filter(|e| *e > 0),
                int(&r["menuPageNo"]).and_then(|p| usize::try_from(p).ok()),
            ) else {
                continue;
            };
            let name = int(&r["msgId"])
                .and_then(|m| names.get(&m).cloned())
                .unwrap_or_else(|| format!("Idol {entity}"));
            atlas.pages.entry(page).or_insert_with(|| {
                let title = names.get(&(PAGES + page as i32)).cloned();
                page_title(title.unwrap_or_default(), places)
            });
            atlas.dests.push(Dest {
                entity,
                page,
                name,
                map: map_of(entity),
            });
        }
        atlas
    }

    /// The pages with a `found` idol, in page order: the title and the indices of its found
    /// idols in `dests`.
    fn regions(&self, found: &BTreeSet<i32>) -> Vec<(String, Vec<usize>)> {
        self.pages
            .iter()
            .filter_map(|(page, title)| {
                let list: Vec<usize> = (self.dests.iter().enumerate())
                    .filter(|(_, d)| d.page == *page && found.contains(&d.entity))
                    .map(|(i, _)| i)
                    .collect();
                (!list.is_empty()).then(|| (title.clone(), list))
            })
            .collect()
    }

    /// Whether a Travel can reach `d`: in this play world, or on a map that is extracted.
    fn reachable(&self, d: &Dest, idols: &Idols) -> bool {
        idols.index(d.entity).is_some() || self.maps.contains(&d.map)
    }
}

/// A page title with its `<?placeName@n?>` tag resolved from `places` (地名).
fn page_title(title: String, places: &BTreeMap<i32, String>) -> String {
    title
        .strip_prefix("<?placeName@")
        .and_then(|r| r.strip_suffix("?>"))
        .and_then(|n| n.parse().ok())
        .and_then(|n| places.get(&n).cloned())
        .unwrap_or(title)
}

/// The map an idol's entity is in: Sekiro's entity ids start with the map's area and block
/// (1111950 is in m11_01_00_00), as FwLo's `sr_map.rs` `map_of` reads them. Every
/// BonfireWarpParam idol and its warp point are in that map's MSB.
fn map_of(entity: i32) -> String {
    format!(
        "m{:02}_{:02}_00_00",
        entity / 100_000,
        (entity / 10_000) % 10
    )
}

/// The Travel screen's state.
#[derive(Resource, Default)]
pub(super) struct TravelMenu {
    pub open: bool,
    /// The picked region, and the picked idol in it while the cursor is in the list.
    region: usize,
    index: usize,
    in_list: bool,
    /// Opened this frame: the OK that opened it is not a pick.
    just_opened: bool,
    /// The found idols' regions as the screen opened.
    regions: Vec<(String, Vec<usize>)>,
}

impl TravelMenu {
    /// Opens the screen on the region of the idol Wolf prays at.
    pub fn open_at(&mut self, idols: &Idols, duel: &Duel) {
        let regions = idols.atlas.regions(&duel.kit.found_idols);
        let here = duel.idol().map(|i| idols.spots[i].entity);
        let region = regions
            .iter()
            .position(|(_, list)| {
                list.iter()
                    .any(|&d| Some(idols.atlas.dests[d].entity) == here)
            })
            .unwrap_or(0);
        *self = Self {
            open: !regions.is_empty(),
            region,
            index: 0,
            in_list: false,
            just_opened: true,
            regions,
        };
    }
}

/// What the screen shows: its region, idol, column and the idol Wolf prays at.
type Shown = Option<(usize, usize, bool, Option<usize>)>;

#[derive(Component)]
struct TravelRoot;
#[derive(Component)]
struct Fade;

pub(super) fn install(app: &mut App, events: &BTreeMap<i32, String>) {
    let help = events.get(&(TRAVEL + HELP)).cloned().unwrap_or_default();
    app.init_resource::<TravelMenu>()
        .add_systems(Startup, spawn_fade)
        .add_systems(
            PreUpdate,
            travel_keys
                .after(super::idol::menu_keys)
                .run_if(crate::menu::active),
        )
        .add_systems(
            Update,
            (
                move |commands: Commands,
                      menu: Res<TravelMenu>,
                      idols: Res<Idols>,
                      sim: NonSend<Sim>,
                      fonts: Res<GameFonts>,
                      art: Res<MenuArt>,
                      roots: Query<Entity, With<TravelRoot>>,
                      shown: Local<Shown>| {
                    show(commands, menu, idols, sim, fonts, art, roots, shown, &help)
                },
                fade,
                depart,
            )
                .run_if(crate::menu::active),
        );
}

/// The Travel screen's keys: up and down move the cursor, OK (or right) goes into the region's
/// list and in the list travels, Esc or back (or left in the list) goes back a level.
fn travel_keys(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    mut menu: ResMut<TravelMenu>,
    idols: Res<Idols>,
    mut sim: NonSendMut<Sim>,
) {
    if !menu.open {
        return;
    }
    if std::mem::take(&mut menu.just_opened) {
        return;
    }
    if sim.duel.idol_phase() != Some(IdolPhase::Praying) {
        menu.open = false;
        return;
    }
    let pad = |b| pads.iter().any(|p| p.just_pressed(b));
    let key = |k: &[KeyCode]| keys.any_just_pressed(k.iter().copied());
    let len = if menu.in_list {
        menu.regions[menu.region].1.len()
    } else {
        menu.regions.len()
    };
    let at = if menu.in_list {
        menu.index
    } else {
        menu.region
    };
    let mut to = at;
    if key(&[KeyCode::ArrowDown, KeyCode::KeyS]) || pad(GamepadButton::DPadDown) {
        to = (at + 1) % len;
    }
    if key(&[KeyCode::ArrowUp, KeyCode::KeyW]) || pad(GamepadButton::DPadUp) {
        to = (at + len - 1) % len;
    }
    if menu.in_list {
        menu.index = to;
    } else if to != at {
        menu.region = to;
        menu.index = 0;
    }
    let ok = crate::menu::ok_pressed(&keys, &pads);
    let back =
        key(&[KeyCode::Escape, KeyCode::Backspace]) || crate::menu::back_pressed(&keys, &pads);
    if !menu.in_list {
        if ok || key(&[KeyCode::ArrowRight, KeyCode::KeyD]) || pad(GamepadButton::DPadRight) {
            menu.in_list = true;
        } else if back {
            menu.open = false;
        }
        return;
    }
    if back || key(&[KeyCode::ArrowLeft, KeyCode::KeyA]) || pad(GamepadButton::DPadLeft) {
        menu.in_list = false;
        return;
    }
    if !ok {
        return;
    }
    let d = &idols.atlas.dests[menu.regions[menu.region].1[menu.index]];
    let to = match idols.index(d.entity) {
        Some(i) => Destination::Here(i),
        None if idols.atlas.reachable(d, &idols) => Destination::Away(d.entity),
        None => return,
    };
    if sim.duel.travel_to_idol(to) {
        info!("idol travel: to {} ({}, {to:?})", d.name, d.entity);
        menu.open = false;
    }
}

/// Builds the screen again when it opens, closes or its cursor moves.
#[allow(clippy::too_many_arguments)]
fn show(
    mut commands: Commands,
    menu: Res<TravelMenu>,
    idols: Res<Idols>,
    sim: NonSend<Sim>,
    fonts: Res<GameFonts>,
    art: Res<MenuArt>,
    roots: Query<Entity, With<TravelRoot>>,
    mut shown: Local<Shown>,
    help: &str,
) {
    let now = menu
        .open
        .then_some((menu.region, menu.index, menu.in_list, sim.duel.idol()));
    if *shown == now {
        return;
    }
    *shown = now;
    for root in &roots {
        commands.entity(root).despawn();
    }
    if !menu.open {
        return;
    }
    let here = sim.duel.idol().map(|i| idols.spots[i].entity);
    let sprite = |name: &str| {
        art.sprite("SB_In_Game", name)
            .map(|(image, _)| ImageNode {
                image_mode: bevy::ui::widget::NodeImageMode::Stretch,
                ..image
            })
            .unwrap_or_default()
    };
    let text = |t: &str, px: f32, color: Color| fonts.shadowed(t, px, color);
    let cache = sekiro_sim::cache::root();
    let menu_texts = crate::game_ui::menu_texts(&cache, "NTC_メニューテキスト");
    let guides = crate::game_ui::menu_texts(&cache, "NTC_キーガイド");
    let row = Node {
        width: percent(100),
        height: percent(100),
        align_items: AlignItems::Center,
        ..default()
    };
    commands
        .spawn((
            TravelRoot,
            crate::game_ui::Covers,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
            GlobalZIndex(111),
        ))
        .with_children(|root| {
            root.spawn((abs(0.0, 0.0, 100.0, 100.0), sprite("MENU_Base_00")));
            root.spawn(abs(5.9, 7.6, 30.0, 5.0)).with_child(text(
                menu_texts.get(&TITLE).map_or("Travel", |t| t.as_str()),
                21.0,
                Color::WHITE,
            ));
            root.spawn((
                abs(0.0, 12.8, 32.0, 0.2),
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.25)),
            ));
            for (i, (name, _)) in menu.regions.iter().enumerate() {
                let t = 16.0 + i as f32 * 8.2;
                let sel = i == menu.region;
                let plate = if sel {
                    "MENU_Cursor_03"
                } else {
                    "MENU_ItemBase_02"
                };
                let tint = match (sel, menu.in_list) {
                    (true, true) => CURSOR.with_alpha(0.55),
                    (true, false) => CURSOR,
                    _ => Color::WHITE,
                };
                root.spawn((abs(6.8, t, 34.5, 6.4), sprite(plate).with_color(tint)))
                    .with_child((
                        Node {
                            padding: UiRect::left(percent(5.0)),
                            ..row.clone()
                        },
                        children![text(name, 17.0, if sel { Color::WHITE } else { TEXT })],
                    ));
            }
            // The region's idols on a parchment, and the picture panel under it.
            root.spawn((abs(43.2, 14.6, 50.0, 42.6), sprite("MENU_WP_Base")));
            root.spawn((
                abs(43.2, 58.4, 50.0, 30.6),
                sprite("MENU_WP_Base").with_color(Color::srgb(0.6, 0.6, 0.6)),
            ));
            if let Some((_, list)) = menu.regions.get(menu.region) {
                // Rows closer together when a region has many (the parchment is 40 high).
                let step = (40.0 / list.len().max(1) as f32).min(5.6);
                for (k, &d) in list.iter().enumerate() {
                    let d = &idols.atlas.dests[d];
                    let sel = menu.in_list && k == menu.index;
                    let can = Some(d.entity) != here && idols.atlas.reachable(d, &idols);
                    let plate = if sel {
                        "MENU_Cursor_03"
                    } else {
                        "MENU_ItemBase_07"
                    };
                    let colour = match (can, sel) {
                        (false, _) => GREYED,
                        (true, true) => Color::WHITE,
                        (true, false) => TEXT,
                    };
                    root.spawn((
                        abs(45.8, 16.2 + k as f32 * step, 42.6, step.min(4.6)),
                        sprite(plate).with_color(if sel {
                            CURSOR
                        } else {
                            Color::srgb(0.5, 0.5, 0.5)
                        }),
                    ))
                    .with_child((
                        Node {
                            justify_content: JustifyContent::Center,
                            ..row.clone()
                        },
                        children![text(&d.name, 16.5, colour)],
                    ));
                }
            }
            root.spawn(abs(5.9, 89.6, 30.0, 4.0))
                .with_child(text(help, 16.5, TEXT));
            root.spawn(Node {
                position_type: PositionType::Absolute,
                left: percent(5.3),
                top: percent(93.6),
                align_items: AlignItems::Center,
                column_gap: px(4),
                ..default()
            })
            .with_children(|r| {
                for (name, label) in [("KG_Key_Enter", KEY_OK), ("KG_Key_Esc", KEY_BACK)] {
                    if let Some((image, aspect)) = art.sprite("SB_KeyTop", name) {
                        r.spawn((
                            image,
                            Node {
                                height: crate::game_ui::stage(20.0),
                                aspect_ratio: Some(aspect),
                                ..default()
                            },
                        ));
                    }
                    let label = guides.get(&label).cloned().unwrap_or_default();
                    r.spawn(text(&format!(":{label}  "), 16.0, DIM));
                }
            });
        });
}

fn spawn_fade(mut commands: Commands) {
    commands.spawn((
        Fade,
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            ..default()
        },
        BackgroundColor(Color::NONE),
        GlobalZIndex(140),
        Pickable::IGNORE,
    ));
}

/// The loading fade: black rises over the warp clip up to the cut, holds while the next map
/// loads, and lifts over [`LIFT`] seconds once Wolf has arrived.
fn fade(
    time: Res<Time>,
    sim: NonSend<Sim>,
    mut lift: Local<Option<f32>>,
    mut fades: Query<&mut BackgroundColor, With<Fade>>,
) {
    let travel = sim.duel.idol_travel();
    let now = time.elapsed_secs();
    if travel.is_some() {
        *lift = Some(f32::INFINITY);
    } else if *lift == Some(f32::INFINITY) {
        *lift = Some(now);
    }
    let alpha = match (travel, *lift) {
        (Some(t), _) => (t / WARP_CUT).clamp(0.0, 1.0),
        // Left for another map: black until the run ends.
        (None, _) if sim.duel.idol_phase() == Some(IdolPhase::Traveling) => 1.0,
        (None, Some(since)) => (1.0 - (now - since) / LIFT).max(0.0),
        (None, None) => 0.0,
    };
    if alpha <= 0.0 {
        *lift = None;
    }
    for mut c in &mut fades {
        c.set_if_neq(BackgroundColor(Color::BLACK.with_alpha(alpha)));
    }
}

/// Travel to another map: this run ends and the game starts again on that map, at the idol,
/// with the kit.
fn depart(mut sim: NonSendMut<Sim>, mut exit: MessageWriter<AppExit>) {
    let Some(entity) = sim.duel.take_departure() else {
        return;
    };
    let kit = match serde_json::to_string(&sim.duel.kit) {
        Ok(kit) => kit,
        Err(e) => {
            error!("idol travel: the kit does not serialize: {e}");
            return;
        }
    };
    let args = travel_args(std::env::args().skip(1), &map_of(entity), entity, &kit);
    info!(
        "idol travel: relaunching on {} at idol {entity} (warp point {})",
        map_of(entity),
        entity - WARP_POINT
    );
    crate::menu::relaunch_with(args);
    exit.write(AppExit::Success);
}

/// This run's `args` for a run on `map` that starts at idol `entity` with `kit`: its map, start
/// and kit flags replaced, the rest kept.
fn travel_args(
    args: impl IntoIterator<Item = String>,
    map: &str,
    entity: i32,
    kit: &str,
) -> Vec<String> {
    const REPLACED: [&str; 5] = ["--map", "--start", "--spawn", "--at-idol", "--kit"];
    let mut out = Vec::new();
    let mut args = args.into_iter();
    while let Some(a) = args.next() {
        if REPLACED.contains(&a.as_str()) {
            args.next();
        } else {
            out.push(a);
        }
    }
    out.extend([
        "--map".into(),
        map.into(),
        "--at-idol".into(),
        entity.to_string(),
        "--kit".into(),
        kit.into(),
    ]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_travel_list_is_the_games_pages_of_found_idols_in_row_order() {
        let cache = sekiro_sim::cache::root();
        let Ok(rows) = sekiro_sim::registry::rows(&cache, "BonfireWarpParam") else {
            eprintln!("skipping: cache not extracted");
            return;
        };
        let names = crate::game_ui::menu_texts(&cache, "NTC_メニューテキスト");
        let atlas = Atlas::load(&cache, rows.values(), &names);
        // The temple (on Important Places and Ashina Outskirts), the Castle Gate and an idol of
        // Ashina Castle, found out of order.
        let found = BTreeSet::from([1111950, 1101951, 1101950]);
        let regions: Vec<_> = (atlas.regions(&found).into_iter())
            .map(|(title, list)| {
                let names: Vec<_> = list
                    .iter()
                    .map(|&d| atlas.dests[d].name.replace('\u{a0}', " "))
                    .collect();
                (title, names)
            })
            .collect();
        assert_eq!(regions.len(), 3, "{regions:?}");
        assert_eq!(regions[0].0, "Important Places");
        assert_eq!(regions[0].1, ["Dilapidated Temple"]);
        assert_eq!(regions[1].0, "Ashina Outskirts");
        // Row 1100 (the temple) before row 1102 (the gate path).
        assert_eq!(regions[1].1[0], "Dilapidated Temple");
        assert_eq!(regions[1].1.len(), 2);
        assert_eq!(regions[2].0, "Ashina Castle");
        assert_eq!(regions[2].1.len(), 1);
        // Every idol's map is its entity's, and extracted ones are reachable from anywhere.
        let castle = atlas.dests.iter().find(|d| d.entity == 1111950).unwrap();
        assert_eq!(castle.map, "m11_01_00_00");
    }

    #[test]
    fn travel_to_another_map_restarts_this_run_there_at_the_idol_with_the_kit() {
        let args = [
            "--play",
            "--map",
            "m11_00_00_00",
            "--spawn",
            "1,2,3,4,0",
            "--perf",
            "--kit",
            "{}",
            "--no-enemy",
        ]
        .map(String::from);
        assert_eq!(
            travel_args(args, "m20_00_00_00", 2001950, "{\"sen\":1}"),
            [
                "--play",
                "--perf",
                "--no-enemy",
                "--map",
                "m20_00_00_00",
                "--at-idol",
                "2001950",
                "--kit",
                "{\"sen\":1}"
            ]
        );
    }
}

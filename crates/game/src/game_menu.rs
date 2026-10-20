//! The in-game menu (#127): Esc pauses the world under the game's wooden board with the tabs
//! Equipment, Inventory, Skills and Options, drawn with its menu art, item icons and texts
//! (item.msgbnd) and showing Wolf's kit (`sekiro_sim::kit`). X / Z switch tabs, V / C the
//! category, the arrows move the cursor; Esc closes it.
//! After FwLo's RustSekiro `src/game_menu.rs` (the layout set from captures of the game).
//! gap: display only (no Equipped Items picker, no item use, no mouse); the tab and section
//! labels are English strings, not yet the game's FMG ids; attribute levels, Unseen Aid and
//! skill experience are not modelled; Options holds this build's Resume / Back to start menu /
//! Quit, not the game's settings; Ninjutsu lists nothing (Wolf starts with none, and the game
//! keeps their texts in the goods banks).
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use bevy::prelude::*;
use sekiro_sim::kit::Kit;

use super::Sim;
use crate::game_ui::{self, GameFonts, MenuArt, abs};

const TABS: [&str; 4] = ["Equipment", "Inventory", "Skills", "Options"];
const OPTION_ITEMS: [&str; 3] = ["Resume", "Back to start menu", "Quit"];
const TEXT: Color = Color::srgb(0.88, 0.85, 0.78);
const DIM: Color = Color::srgba(0.75, 0.72, 0.66, 0.85);
const ORANGE: Color = Color::srgb(0.93, 0.6, 0.25);
/// Rows shown at once in a list screen.
const ROWS: usize = 6;
/// gap: Prayer Necklace attribute levels are not modelled; FwLo's starting level.
const PLAYER_LEVEL: u32 = 1;
const ICONS: [&str; 5] = [
    "SB_Icon",
    "SB_Icon_02",
    "SB_Icon_03",
    "SB_Icon_04",
    "SB_Boss",
];
const STORIES: [&str; 2] = ["SB_Icon_Skill", "SB_Icon_Skill_02"];
const INV_CATS: [(&str, &str); 4] = [
    ("Quick Items", "MENU_TB_Item"),
    ("Key Items", "MENU_TB_valuable"),
    ("Upgrade Materials", "MENU_TB_Material"),
    ("Prosthetic Arm Tools", "MENU_TB_Arm"),
];
/// gap: MENU_TB_Attack stands in for the game's plain Combat Arts blade, as in FwLo's menu.
const SKILL_CATS: [(&str, &str); 4] = [
    ("Combat Arts", "MENU_TB_Attack"),
    ("Shinobi Martial Arts", "MENU_TB_Action"),
    ("Latent Skills", "MENU_TB_Sinobi"),
    ("Ninjutsu Techniques", "MENU_TB_Ninjyutu"),
];

/// The last tab and categories persist across pauses; the cursor starts at the first row.
#[derive(Resource, Default)]
struct GameMenu {
    tab: usize,
    index: usize,
    cat: [usize; 2],
    top: usize,
}

#[derive(Component)]
struct GameMenuRoot;

/// The screen and cursor whose drawing is currently built.
type Place = (usize, usize, usize, [usize; 2]);

/// The game's item names and descriptions, shared by slots, lists and skill categories.
#[derive(Default)]
struct ItemTexts {
    weapon_name: BTreeMap<i32, String>,
    weapon_info: BTreeMap<i32, String>,
    goods_name: BTreeMap<i32, String>,
    goods_info: BTreeMap<i32, String>,
}

impl ItemTexts {
    fn load(cache: &Path) -> Self {
        let read = |bank| game_ui::texts(cache, "item", bank);
        Self {
            weapon_name: read("武器名"),
            weapon_info: read("武器説明"),
            goods_name: read("アイテム名"),
            goods_info: read("アイテム説明"),
        }
    }

    /// The lowest id with this English name, as FwLo's key-item rows use.
    fn goods_id(&self, name: &str) -> Option<i32> {
        self.goods_name
            .iter()
            .find(|(_, n)| n.as_str() == name)
            .map(|(id, _)| *id)
    }

    fn weapon_id(&self, name: &str) -> Option<i32> {
        self.weapon_name
            .iter()
            .find(|(_, n)| n.as_str() == name)
            .map(|(id, _)| *id)
    }
}

/// One skill and the virtual weapon that supplies its text and art.
struct Skill {
    id: i32,
    weapon: i32,
}

/// The goods fields needed to order held goods and show their capacity.
struct Goods {
    kind: i64,
    sort: i64,
    max: u32,
}

#[derive(Default)]
struct Tables {
    skills: [Vec<Skill>; 4],
    weapon_icon: BTreeMap<i32, i64>,
    goods_icon: BTreeMap<i32, i64>,
    goods: BTreeMap<i32, Goods>,
}

impl Tables {
    /// FwLo's categories: skills in id order, deduplicated by name and classified by description.
    fn load(cache: &Path, texts: &ItemTexts) -> Self {
        let rows = |name| {
            sekiro_sim::registry::rows(cache, name)
                .inspect_err(|e| warn!("game menu: {e}"))
                .unwrap_or_default()
        };
        let mut tables = Self::default();
        let mut seen = HashSet::new();
        for (id, row) in rows("SkillParam") {
            if id >= 700 {
                continue;
            }
            let Some(weapon) = row["virtualWeaponId"].as_i64().map(|n| n as i32) else {
                continue;
            };
            let name = texts.weapon_name.get(&weapon).cloned().unwrap_or_default();
            let info = texts
                .weapon_info
                .get(&weapon)
                .map(|s| s.to_lowercase())
                .unwrap_or_default();
            let first = info.split('.').next().unwrap_or("");
            let cat = if first.contains("latent skill") {
                2
            } else if first.contains("ninjutsu") {
                3
            } else if first.contains("martial art") {
                1
            } else if first.contains("combat art") || info.contains("combat art") {
                0
            } else {
                continue;
            };
            if !name.is_empty() && seen.insert(name) {
                tables.skills[cat].push(Skill { id, weapon });
            }
        }
        for (id, row) in rows("EquipParamWeapon") {
            if let Some(icon) = row["iconId"].as_i64() {
                tables.weapon_icon.insert(id, icon);
            }
        }
        for (id, row) in rows("EquipParamGoods") {
            if let Some(icon) = row["iconId"].as_i64() {
                tables.goods_icon.insert(id, icon);
            }
            tables.goods.insert(
                id,
                Goods {
                    kind: row["goodsType"].as_i64().unwrap_or(-1),
                    sort: row["sortId"].as_i64().unwrap_or(0),
                    max: row["maxNum"].as_u64().unwrap_or(99) as u32,
                },
            );
        }
        tables
    }

    fn max(&self, id: i32) -> u32 {
        self.goods.get(&id).map_or(99, |g| g.max)
    }
}

/// Texts and params read once, plus this run's attack power and translated key guides.
#[derive(Resource)]
struct Book {
    texts: ItemTexts,
    tables: Tables,
    attack: u8,
    guides: BTreeMap<i32, String>,
}

impl Book {
    fn load(cache: &Path, attack: u8) -> Self {
        let texts = ItemTexts::load(cache);
        let tables = Tables::load(cache, &texts);
        Self {
            texts,
            tables,
            attack,
            guides: game_ui::menu_texts(cache, "NTC_キーガイド"),
        }
    }
}

/// One Equipment slot, in cursor order: three tools, the art, five quick items, ninjutsu.
enum Slot {
    Weapon(i32),
    Goods(i32, u32),
    Empty,
}

fn slots(kit: &Kit) -> Vec<Slot> {
    let mut slots = Vec::new();
    for i in 0..3 {
        slots.push(
            kit.prosthetics
                .get(i)
                .map_or(Slot::Empty, |id| Slot::Weapon(*id)),
        );
    }
    slots.push(Slot::Weapon(kit.combat_art));
    for id in kit.quick_items {
        slots.push(if id > 0 {
            Slot::Goods(id, kit.count(id))
        } else {
            Slot::Empty
        });
    }
    slots.push(Slot::Empty);
    slots
}

/// Held goods of one goodsType, sorted by the game's sortId then id.
fn held_goods(kit: &Kit, tables: &Tables, kind: i64) -> Vec<(i32, u32)> {
    let mut goods: Vec<_> = kit
        .goods
        .iter()
        .copied()
        .filter(|(id, n)| *n > 0 && tables.goods.get(id).is_some_and(|g| g.kind == kind))
        .collect();
    goods.sort_by_key(|(id, _)| (tables.goods[id].sort, *id));
    goods
}

/// Sprite references stay independent of Bevy assets so row data can be checked alone.
fn any_icon(id: i64) -> Option<String> {
    Some(format!("item/MENU_ItemIcon_{id:05}"))
}
fn sprite(file: &str) -> Option<String> {
    Some(file.to_string())
}

fn slot_icon(tables: &Tables, slot: &Slot) -> Option<String> {
    let id = match slot {
        Slot::Weapon(id) => tables.weapon_icon.get(id).copied(),
        Slot::Goods(3000, _) => Some(151),
        Slot::Goods(id, _) => tables.goods_icon.get(id).copied(),
        Slot::Empty => None,
    }?;
    any_icon(id)
}

/// Tool upgrade levels share their base's name and description, independently falling back.
fn slot_text(texts: &ItemTexts, slot: &Slot) -> (String, String) {
    let pick = |names: &BTreeMap<i32, String>, infos: &BTreeMap<i32, String>, id: i32| {
        let base = [id, id / 100 * 100, id / 1000 * 1000];
        (
            base.iter()
                .find_map(|id| names.get(id))
                .cloned()
                .unwrap_or_default(),
            base.iter()
                .find_map(|id| infos.get(id))
                .cloned()
                .unwrap_or_default(),
        )
    };
    match slot {
        Slot::Weapon(id) => pick(&texts.weapon_name, &texts.weapon_info, *id),
        Slot::Goods(id, _) => pick(&texts.goods_name, &texts.goods_info, *id),
        Slot::Empty => (String::new(), String::new()),
    }
}

/// The look shared by each drawing function, with no assets loaded while a screen is built.
struct Ui<'a> {
    art: &'a MenuArt,
    fonts: &'a GameFonts,
    aspect: f32,
}

impl Ui<'_> {
    fn text(&self, text: &str, px: f32, color: Color) -> (Text, TextFont, TextColor, TextShadow) {
        (
            Text::new(text),
            self.fonts.stage(px),
            TextColor(color),
            TextShadow {
                offset: Vec2::splat(1.0),
                color: Color::srgba(0.0, 0.0, 0.0, 0.8),
            },
        )
    }

    fn find(&self, file: &str) -> Option<(ImageNode, f32)> {
        let (atlas, name) = file.split_once('/')?;
        match atlas {
            "item" => self.art.find(&ICONS, name),
            "story" => self.art.find(&STORIES, name),
            "sb_in_game" => self.art.sprite("SB_In_Game", name),
            "sb_keytop" => self.art.sprite("SB_KeyTop", name),
            "sb_fe" => self.art.sprite("SB_FE", name),
            _ => None,
        }
    }

    /// Plates and icons stretch into FwLo's boxes; a missing sprite draws nothing.
    fn image(&self, file: Option<String>) -> ImageNode {
        let mut image = file
            .and_then(|f| self.find(&f))
            .map(|(i, _)| i)
            .unwrap_or_else(|| ImageNode {
                color: Color::NONE,
                ..default()
            });
        image.image_mode = bevy::ui::widget::NodeImageMode::Stretch;
        image
    }

    /// Story art fits its capture's box without distorting the sprite's aspect.
    fn story(&self, root: &mut ChildSpawnerCommands, file: &str) {
        if let Some((image, aspect)) = self.find(file) {
            let w = (16.5 * aspect / self.aspect).min(27.0);
            let h = w * self.aspect / aspect;
            root.spawn((
                abs(61.0 + (27.0 - w) / 2.0, 20.6 + (16.5 - h) / 2.0, w, h),
                image,
            ));
        }
    }
}

/// A full-size child laid out as a row, vertically centred.
fn fill_row() -> Node {
    Node {
        width: percent(100),
        height: percent(100),
        align_items: AlignItems::Center,
        ..default()
    }
}

/// A list row: icon, name, count, optional value, and equipped mark.
#[derive(Clone, Default)]
struct Row {
    icon: Option<String>,
    name: String,
    count: Option<String>,
    value: Option<String>,
    equipped: bool,
}

/// The selected item's parchment: name, icon or story art, stats, and description.
#[derive(Clone, Default)]
struct Detail {
    name: String,
    icon: Option<String>,
    art: Option<String>,
    stats: Vec<(String, String)>,
    info: String,
}

/// Installs the display over the existing pause state, with the kit owned by the sim.
pub(super) fn install(app: &mut App, attack: u8) {
    GameFonts::insert(app);
    MenuArt::insert(app);
    MenuArt::add(
        app,
        &[
            "SB_Icon",
            "SB_Icon_02",
            "SB_Icon_03",
            "SB_Icon_04",
            "SB_Boss",
            "SB_Icon_Skill",
            "SB_Icon_Skill_02",
            "SB_FE",
        ],
    );
    game_ui::install_cover(app);
    app.insert_resource(Book::load(&sekiro_sim::cache::root(), attack))
        .init_resource::<GameMenu>()
        .init_resource::<crate::menu::PauseView>()
        .add_systems(
            PreUpdate,
            menu_keys
                .after(bevy::input::InputSystems)
                .after(crate::menu::PauseSystems)
                .run_if(crate::menu::paused),
        )
        .add_systems(Update, show);
}

/// Tab and category switches, wrapped cursor movement, and the Options actions only.
fn menu_keys(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    mut menu: ResMut<GameMenu>,
    mut view: ResMut<crate::menu::PauseView>,
    book: Res<Book>,
    sim: NonSend<Sim>,
) {
    let key = |k: &[KeyCode]| keys.any_just_pressed(k.iter().copied());
    let pad = |b| pads.iter().any(|p| p.just_pressed(b));
    if key(&[KeyCode::KeyX, KeyCode::KeyE]) || pad(GamepadButton::RightTrigger) {
        menu.tab = (menu.tab + 1) % TABS.len();
        menu.index = 0;
        menu.top = 0;
    }
    if key(&[KeyCode::KeyZ, KeyCode::KeyQ]) || pad(GamepadButton::LeftTrigger) {
        menu.tab = (menu.tab + TABS.len() - 1) % TABS.len();
        menu.index = 0;
        menu.top = 0;
    }
    if menu.tab == 1 || menu.tab == 2 {
        let k = menu.tab - 1;
        if key(&[KeyCode::KeyV]) || pad(GamepadButton::RightTrigger2) {
            menu.cat[k] = (menu.cat[k] + 1) % 4;
            menu.index = 0;
            menu.top = 0;
        }
        if key(&[KeyCode::KeyC]) || pad(GamepadButton::LeftTrigger2) {
            menu.cat[k] = (menu.cat[k] + 3) % 4;
            menu.index = 0;
            menu.top = 0;
        }
    }
    let kit = &sim.duel.kit;
    let n = match menu.tab {
        0 => 10,
        1 => inventory(menu.cat[0], kit, &book.texts, &book.tables).len(),
        2 => skills(menu.cat[1], kit, &book.texts, &book.tables).len(),
        _ => OPTION_ITEMS.len(),
    }
    .max(1);
    if key(&[
        KeyCode::ArrowRight,
        KeyCode::KeyD,
        KeyCode::ArrowDown,
        KeyCode::KeyS,
    ]) || pad(GamepadButton::DPadRight)
        || pad(GamepadButton::DPadDown)
    {
        menu.index = (menu.index + 1) % n;
    }
    if key(&[
        KeyCode::ArrowLeft,
        KeyCode::KeyA,
        KeyCode::ArrowUp,
        KeyCode::KeyW,
    ]) || pad(GamepadButton::DPadLeft)
        || pad(GamepadButton::DPadUp)
    {
        menu.index = (menu.index + n - 1) % n;
    }
    menu.top = keep_visible(menu.top, menu.index, n);
    if menu.tab == 3 && (key(&[KeyCode::Enter, KeyCode::Space]) || pad(GamepadButton::South)) {
        view.request = Some(menu.index);
    }
}

/// Rebuilds only for a changed cursor or screen; closing keeps the tab and categories.
fn show(
    mut commands: Commands,
    state: Res<crate::menu::PanelState>,
    mut menu: ResMut<GameMenu>,
    (fonts, art, book): (Res<GameFonts>, Res<MenuArt>, Res<Book>),
    sim: NonSend<Sim>,
    (roots, windows): (Query<Entity, With<GameMenuRoot>>, Query<&Window>),
    mut built: Local<Option<Place>>,
) {
    if !state.paused {
        for root in &roots {
            commands.entity(root).despawn();
        }
        *built = None;
        menu.index = 0;
        menu.top = 0;
        return;
    }
    let now = (menu.tab, menu.index, menu.top, menu.cat);
    if *built == Some(now) {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    let hp = &sim.duel.rules.fighters[0].vitality;
    build(
        &mut commands,
        &Ui {
            art: &art,
            fonts: &fonts,
            aspect: windows
                .iter()
                .next()
                .map_or(16.0 / 9.0, |w| w.width() / w.height().max(1.0)),
        },
        &menu,
        &sim.duel.kit,
        &book,
        (hp.hp as f32, hp.max_hp as f32),
    );
    *built = Some(now);
}

/// The key guide at the bottom right, using the game's OK and Close labels.
fn key_guide(root: &mut ChildSpawnerCommands, ui: &Ui, guides: &BTreeMap<i32, String>) {
    root.spawn(Node {
        position_type: PositionType::Absolute,
        right: percent(5.3),
        top: percent(91.4),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(4),
        ..default()
    })
    .with_children(|r| {
        for (name, id, suffix) in [("KG_Key_Enter", 100000, "  "), ("KG_Key_Esc", 110001, "")] {
            r.spawn((
                Node {
                    height: px(20),
                    width: px(30),
                    ..default()
                },
                ui.image(sprite(&format!("sb_keytop/{name}"))),
            ));
            let label = guides.get(&id).map(String::as_str).unwrap_or("");
            r.spawn(ui.text(&format!(":{label}{suffix}"), 16.0, DIM));
        }
    });
}

/// The first row shown is kept while the cursor stays in its six-row window.
fn keep_visible(top: usize, index: usize, n: usize) -> usize {
    let top = if index < top {
        index
    } else if index >= top + ROWS {
        index + 1 - ROWS
    } else {
        top
    };
    top.min(n.saturating_sub(ROWS))
}

fn list_screen(
    root: &mut ChildSpawnerCommands,
    ui: &Ui,
    categories: (&[(&str, &str)], usize),
    rows: &[Row],
    window: (usize, usize),
    detail: Option<&Detail>,
) {
    let (cats, cat) = categories;
    let (index, top) = window;
    if let Some((name, _)) = cats.get(cat) {
        root.spawn(abs(9.7, 14.0, 30.0, 4.0)).with_children(|c| {
            c.spawn(ui.text(name, 17.0, TEXT));
        });
    }
    if !cats.is_empty() {
        root.spawn((
            abs(5.4, 19.2, 1.4, 4.4),
            ui.image(sprite("sb_in_game/MENU_Arrow")),
        ));
        root.spawn((
            abs(7.4, 19.8, 1.6, 3.2),
            ui.image(sprite("sb_keytop/KG_Key_C")),
        ));
        for (i, (_, icon)) in cats.iter().enumerate() {
            let l = 10.0 + i as f32 * 4.85;
            if i == cat {
                root.spawn((
                    abs(l - 0.35, 17.7, 3.9, 7.2),
                    ui.image(sprite("sb_in_game/MENU_Cursor_TB"))
                        .with_color(ORANGE),
                ));
            }
            root.spawn((
                abs(l, 18.3, 3.2, 6.0),
                ui.image(sprite(&format!("sb_in_game/{icon}"))),
            ));
        }
        let end = 10.0 + cats.len() as f32 * 4.85;
        root.spawn((
            abs(end, 19.8, 1.6, 3.2),
            ui.image(sprite("sb_keytop/KG_Key_V")),
        ));
        let mut right = ui.image(sprite("sb_in_game/MENU_Arrow"));
        right.flip_x = true;
        root.spawn((abs(end + 2.1, 19.2, 1.4, 4.4), right));
    }
    let start = keep_visible(top, index, rows.len());
    for (k, (i, row)) in rows.iter().enumerate().skip(start).take(ROWS).enumerate() {
        let sel = i == index;
        let t = 26.6 + k as f32 * 8.3;
        root.spawn((
            abs(7.4, t, 4.2, 7.4),
            ui.image(sprite("sb_in_game/MENU_IconBase_02")),
        ))
        .with_children(|b| {
            b.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(6.0),
                    top: Val::Percent(6.0),
                    width: Val::Percent(88.0),
                    height: Val::Percent(88.0),
                    ..default()
                },
                ui.image(row.icon.clone()),
            ));
            if row.equipped {
                b.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(-6.0),
                        top: Val::Percent(-6.0),
                        width: Val::Percent(34.0),
                        height: Val::Percent(34.0),
                        ..default()
                    },
                    ui.image(sprite("sb_in_game/MENU_Equipped")),
                ));
            }
            if let Some(n) = &row.count {
                b.spawn(Node {
                    position_type: PositionType::Absolute,
                    right: Val::Percent(4.0),
                    bottom: Val::Percent(0.0),
                    ..default()
                })
                .with_children(|c| {
                    c.spawn(ui.text(n, 13.0, Color::WHITE));
                });
            }
        });
        let plate = if sel {
            "sb_in_game/MENU_Cursor_03"
        } else {
            "sb_in_game/MENU_ItemBase_02"
        };
        root.spawn((abs(12.3, t + 0.6, 36.6, 6.2), ui.image(sprite(plate))))
            .with_children(|b| {
                b.spawn(Node {
                    padding: UiRect::axes(Val::Percent(4.5), Val::Px(0.0)),
                    justify_content: JustifyContent::SpaceBetween,
                    ..fill_row()
                })
                .with_children(|c| {
                    c.spawn(ui.text(&row.name, 17.0, if sel { Color::WHITE } else { TEXT }));
                    if let Some(v) = &row.value {
                        c.spawn(Node {
                            column_gap: Val::Px(14.0),
                            align_items: AlignItems::Center,
                            margin: UiRect::right(Val::Percent(4.0)),
                            ..default()
                        })
                        .with_children(|r| {
                            // A price has the coin; a word ("Owned") stands alone, dimmed.
                            let price = v.parse::<u32>().is_ok();
                            if price {
                                r.spawn((
                                    Node {
                                        height: Val::Px(18.0),
                                        width: Val::Px(18.0),
                                        ..default()
                                    },
                                    ui.image(sprite("sb_fe/MENU_Money")),
                                ));
                            }
                            r.spawn(ui.text(v, 16.0, if price { TEXT } else { DIM }));
                        });
                    }
                });
            });
    }
    // The scroll bar (its thumb the shown share of the list).
    if rows.len() > ROWS {
        root.spawn((
            abs(50.0, 27.5, 0.9, 47.5),
            ui.image(sprite("sb_in_game/MENU_Scroll_Base")),
        ));
        let share = ROWS as f32 / rows.len() as f32;
        root.spawn((
            abs(
                50.17,
                27.5 + 47.5 * start as f32 / rows.len() as f32,
                0.55,
                47.5 * share,
            ),
            ui.image(sprite("sb_in_game/MENU_Scroll_Bar")),
        ));
    }
    if let Some(d) = detail {
        parchment(root, ui, d);
    }
}

/// The parchment on the right: the name on its black bar, the big icon (items) or the story art
/// (skills), the stat lines on ink strokes, the text over faint rules.
fn parchment(root: &mut ChildSpawnerCommands, ui: &Ui, d: &Detail) {
    root.spawn((
        abs(55.5, 14.4, 38.0, 62.3),
        ui.image(sprite("sb_in_game/MENU_WP_Base")),
    ));
    root.spawn((
        abs(56.5, 15.6, 36.0, 4.4),
        ui.image(sprite("sb_in_game/MENU_ItemBase_08"))
            .with_color(Color::srgb(0.25, 0.25, 0.25)),
    ))
    .with_children(|b| {
        b.spawn(Node {
            justify_content: JustifyContent::Center,
            ..fill_row()
        })
        .with_children(|c| {
            c.spawn(ui.text(&d.name, 17.0, Color::WHITE));
        });
    });
    let mut y = 22.0;
    if let Some(art) = &d.art {
        ui.story(root, art);
        y = 38.0;
    } else if d.icon.is_some() {
        root.spawn((
            abs(81.3, 20.8, 9.4, 16.2),
            ui.image(sprite("sb_in_game/MENU_IconBase_01")),
        ))
        .with_children(|b| {
            b.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(8.0),
                    top: Val::Percent(8.0),
                    width: Val::Percent(84.0),
                    height: Val::Percent(84.0),
                    ..default()
                },
                ui.image(d.icon.clone()),
            ));
        });
        y = 30.0;
    }
    for (label, value) in &d.stats {
        root.spawn((
            abs(57.0, y, 22.5, 3.6),
            ui.image(sprite("sb_in_game/MENU_ItemBase_08"))
                .with_color(Color::srgba(0.5, 0.5, 0.5, 0.9)),
        ))
        .with_children(|b| {
            b.spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::axes(Val::Percent(4.0), Val::Px(0.0)),
                ..fill_row()
            })
            .with_children(|c| {
                c.spawn(ui.text(label, 15.5, TEXT));
                c.spawn((Node {
                    margin: UiRect::right(Val::Percent(12.0)),
                    ..default()
                },))
                    .with_children(|v| {
                        v.spawn(ui.text(value, 15.5, TEXT));
                    });
            });
        });
        y += 4.4;
    }
    y += 1.0;
    // Faint rules under the text lines (the game's ruled parchment).
    let mut r = y + 3.2;
    while r < 75.5 {
        root.spawn((
            abs(57.3, r, 34.5, 0.15),
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.18)),
        ));
        r += 3.2;
    }
    root.spawn(abs(57.6, y, 34.0, 75.0 - y)).with_children(|c| {
        c.spawn(ui.text(&d.info, 15.5, TEXT));
    });
}

/// The status bar along the bottom (all but Options): resurrection nodes over the Vitality bar,
/// Unseen Aid, Vitality / Attack Power, the skill level and its gauge, Spirit Emblems and Sen.
/// gap: attribute levels, Unseen Aid and skill experience are not modelled: Wolf's start values.
fn status_bar(
    root: &mut ChildSpawnerCommands,
    ui: &Ui,
    kit: &Kit,
    tables: &Tables,
    texts: &ItemTexts,
    wolf: (f32, f32),
    attack: u8,
) {
    let plate = Color::srgba(0.06, 0.06, 0.06, 0.78);
    let (t, h) = (79.2, 8.8);
    let (hp, hp_max) = wolf;
    // Resurrection and Vitality.
    root.spawn((abs(7.1, t, 19.1, h), BackgroundColor(plate)))
        .with_children(|b| {
            for k in 0..kit.resurrections.max(1) {
                let lit = k < kit.resurrections;
                b.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(14.0 + k as f32 * 22.0),
                        top: Val::Px(6.0),
                        width: Val::Px(18.0),
                        height: Val::Px(18.0),
                        ..default()
                    },
                    ui.image(sprite("sb_fe/MENU_Revive_Gauge"))
                        .with_color(if lit {
                            Color::WHITE
                        } else {
                            Color::srgba(0.4, 0.4, 0.4, 0.6)
                        }),
                ));
            }
            b.spawn((
                abs(5.0, 50.0, 90.0, 9.0),
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            ));
            b.spawn((
                abs(
                    5.0,
                    50.0,
                    90.0 * (hp / hp_max.max(1.0)).clamp(0.0, 1.0),
                    9.0,
                ),
                BackgroundColor(Color::srgb(0.62, 0.17, 0.12)),
            ));
        });
    // Unseen Aid (gap: no Dragonrot; the game's base 30 %).
    root.spawn((abs(27.1, t, 15.2, h), BackgroundColor(plate)))
        .with_children(|b| {
            b.spawn(Node {
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                padding: UiRect::left(Val::Percent(30.0)),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            })
            .with_children(|c| {
                c.spawn(ui.text("Unseen Aid", 16.5, TEXT));
                c.spawn(ui.text("30%", 16.5, TEXT));
            });
        });
    // Vitality / Attack Power.
    root.spawn((abs(43.4, t, 15.2, h), BackgroundColor(plate)))
        .with_children(|b| {
            b.spawn(Node {
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(2.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::axes(Val::Percent(4.0), Val::Px(0.0)),
                ..default()
            })
            .with_children(|c| {
                for (icon, label, value) in [
                    ("MENU_ST_Vitality", "Vitality", PLAYER_LEVEL.to_string()),
                    ("MENU_ST_Offensive", "Attack Power", attack.to_string()),
                ] {
                    c.spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(6.0),
                        ..default()
                    })
                    .with_children(|r| {
                        r.spawn((
                            Node {
                                width: Val::Px(20.0),
                                height: Val::Px(20.0),
                                ..default()
                            },
                            ui.image(sprite(&format!("sb_in_game/{icon}"))),
                        ));
                        r.spawn((Node {
                            flex_grow: 1.0,
                            ..default()
                        },))
                            .with_children(|n| {
                                n.spawn(ui.text(label, 16.0, TEXT));
                            });
                        r.spawn(ui.text(&value, 16.0, TEXT));
                    });
                }
            });
        });
    // The skill level and the gauge to the next point.
    root.spawn((abs(59.5, t, 20.0, h), BackgroundColor(plate)))
        .with_children(|b| {
            b.spawn((
                abs(2.0, 12.0, 11.0, 40.0),
                ui.image(sprite("sb_fe/MENU_Skill_Lv"))
                    .with_color(Color::srgb(0.15, 0.15, 0.15)),
            ))
            .with_children(|d| {
                d.spawn(Node {
                    justify_content: JustifyContent::Center,
                    ..fill_row()
                })
                .with_children(|c| {
                    c.spawn(ui.text(
                        &kit.skill_points.to_string(),
                        14.0,
                        Color::srgb(0.55, 0.85, 0.9),
                    ));
                });
            });
            b.spawn((
                abs(14.0, 25.0, 60.0, 12.0),
                ui.image(sprite("sb_in_game/MENU_GaugeFrame")),
            ));
            b.spawn((Node {
                position_type: PositionType::Absolute,
                right: Val::Percent(5.0),
                bottom: Val::Percent(14.0),
                ..default()
            },))
                .with_children(|c| {
                    c.spawn(ui.text("to next Skill Point  -", 14.0, Color::srgb(0.55, 0.85, 0.9)));
                });
        });
    // Spirit Emblems and Sen.
    let emblem = texts
        .goods_id("Spirit Emblem")
        .and_then(|id| tables.goods_icon.get(&id))
        .and_then(|i| any_icon(*i));
    root.spawn((abs(80.5, t, 12.8, h), BackgroundColor(plate)))
        .with_children(|b| {
            b.spawn(Node {
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(2.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::axes(Val::Percent(4.0), Val::Px(0.0)),
                ..default()
            })
            .with_children(|c| {
                let n = kit.spirit_emblems;
                for (icon, value) in [
                    (emblem, format!("{n}/15")),
                    (sprite("sb_fe/MENU_Money"), kit.sen.to_string()),
                ] {
                    c.spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|r| {
                        r.spawn((
                            Node {
                                width: Val::Px(22.0),
                                height: Val::Px(22.0),
                                ..default()
                            },
                            ui.image(icon),
                        ));
                        r.spawn((Node {
                            flex_grow: 1.0,
                            justify_content: JustifyContent::FlexEnd,
                            ..default()
                        },))
                            .with_children(|n| {
                                n.spawn(ui.text(&value, 16.0, TEXT));
                            });
                    });
                }
            });
        });
}

/// Inventory's rows and details for a category: Quick Items (the Healing Gourd, the Homeward
/// Idol), Key Items (Kusabimaru, the Shinobi Prosthetic), Upgrade Materials, Prosthetic
/// Arm Tools (those fitted). gap: no pickups, so only what Wolf starts with.
fn inventory(cat: usize, kit: &Kit, texts: &ItemTexts, tables: &Tables) -> Vec<(Row, Detail)> {
    let goods = |id: i32, count: Option<u32>, stats: Vec<(String, String)>| {
        let (name, info) = slot_text(texts, &Slot::Goods(id, 0));
        let icon = if id == 3000 {
            any_icon(151)
        } else {
            tables.goods_icon.get(&id).and_then(|i| any_icon(*i))
        };
        (
            Row {
                icon: icon.clone(),
                name: name.clone(),
                count: count.map(|n| n.to_string()),
                ..default()
            },
            Detail {
                name,
                icon,
                art: None,
                stats,
                info,
            },
        )
    };
    let weapon = |id: i32, stats: Vec<(String, String)>| {
        let (name, info) = slot_text(texts, &Slot::Weapon(id));
        let icon = tables
            .weapon_icon
            .get(&id)
            .or_else(|| tables.weapon_icon.get(&(id / 100 * 100)))
            .and_then(|i| any_icon(*i));
        (
            Row {
                icon: icon.clone(),
                name: name.clone(),
                ..default()
            },
            Detail {
                name,
                icon,
                art: None,
                stats,
                info,
            },
        )
    };
    let held = |n: u32, of: u32| ("Number Held".to_string(), format!("{n}/ {of}"));
    let storage = || ("In Storage".to_string(), "-".to_string());
    match cat {
        0 => {
            let g = kit.gourd_charges;
            let mut v = vec![goods(
                3000,
                Some(g),
                vec![("Uses".to_string(), format!("{g}/{g}")), storage()],
            )];
            for (id, n) in held_goods(kit, tables, 0) {
                let max = tables.max(id);
                v.push(goods(id, Some(n), vec![held(n, max), storage()]));
            }
            v
        }
        1 => {
            let mut v = Vec::new();
            if let Some(id) = texts.weapon_id("Kusabimaru") {
                v.push(weapon(id, vec![held(1, 1), storage()]));
            }
            if let Some(id) = texts.goods_id("Shinobi Prosthetic") {
                v.push(goods(id, None, vec![held(1, 1), storage()]));
            }
            for (id, n) in held_goods(kit, tables, 1) {
                v.push(goods(id, (n > 1).then_some(n), vec![]));
            }
            v
        }
        2 => held_goods(kit, tables, 2)
            .into_iter()
            .map(|(id, n)| {
                let max = tables.max(id);
                goods(id, Some(n), vec![held(n, max), storage()])
            })
            .collect(),
        _ => kit
            .prosthetics
            .iter()
            .map(|id| weapon(*id, vec![]))
            .collect(),
    }
}

/// Skills' rows and details for a category: the learned skills (Kit::skills; with
/// all_skills, all of them), each with its story art (sb_icon_skill
/// MENU_SkillStory_<iconId>).
fn skills(cat: usize, kit: &Kit, texts: &ItemTexts, tables: &Tables) -> Vec<(Row, Detail)> {
    tables.skills[cat]
        .iter()
        .filter(|s| kit.all_skills || kit.skills.contains(&s.id))
        .map(|s| {
            let name = texts
                .weapon_name
                .get(&s.weapon)
                .cloned()
                .unwrap_or_default();
            let info = texts
                .weapon_info
                .get(&s.weapon)
                .cloned()
                .unwrap_or_default();
            let icon_id = tables.weapon_icon.get(&s.weapon).copied();
            let icon = icon_id.and_then(any_icon);
            let art = icon_id.and_then(|i| sprite(&format!("story/MENU_SkillStory_{i:05}")));
            (
                Row {
                    icon: icon.clone(),
                    name: name.clone(),
                    ..default()
                },
                Detail {
                    name,
                    icon,
                    art,
                    stats: vec![],
                    info,
                },
            )
        })
        .collect()
}

fn equipment(
    root: &mut ChildSpawnerCommands,
    ui: &Ui,
    menu: &GameMenu,
    kit: &Kit,
    tables: &Tables,
    texts: &ItemTexts,
) {
    let all = slots(kit);
    // Section bars.
    for (label, l, t) in [
        ("Prosthetic Tools", 6.5, 16.5),
        ("Combat Arts", 38.5, 16.5),
        ("Quick Items", 6.5, 45.0),
        ("Ninjutsu", 38.5, 45.0),
    ] {
        root.spawn((
            abs(l, t, 17.0, 4.5),
            ui.image(sprite("sb_in_game/MENU_Caption_02")),
        ))
        .with_children(|b| {
            b.spawn(Node {
                padding: UiRect::left(Val::Percent(6.0)),
                ..fill_row()
            })
            .with_children(|c| {
                c.spawn(ui.text(label, 17.0, TEXT));
            });
        });
    }
    // The slots: (index, left %, top %, size % of height).
    let mut places: Vec<(usize, f32, f32, f32)> = Vec::new();
    for i in 0..3 {
        places.push((i, 7.0 + i as f32 * 10.2, 22.5, 15.0));
    }
    places.push((3, 39.5, 22.5, 15.0));
    for i in 0..5 {
        places.push((4 + i, 7.0 + i as f32 * 6.2, 51.0, 10.0));
    }
    places.push((9, 39.5, 51.0, 15.0));
    for (i, l, t, size) in places {
        let w = size * 9.0 / 16.0;
        let slot = &all[i];
        root.spawn((
            abs(l, t, w, size),
            ui.image(sprite("sb_in_game/MENU_IconBase_02")),
        ))
        .with_children(|s| {
            // The cursor glows behind the item, not over it.
            if i == menu.index {
                s.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(-8.0),
                        top: Val::Percent(-8.0),
                        width: Val::Percent(116.0),
                        height: Val::Percent(116.0),
                        ..default()
                    },
                    ui.image(sprite("sb_in_game/MENU_Cursor_TB"))
                        .with_color(ORANGE.with_alpha(0.55)),
                ));
            }
            if let Some(icon) = slot_icon(tables, slot) {
                s.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(4.0),
                        top: Val::Percent(4.0),
                        width: Val::Percent(92.0),
                        height: Val::Percent(92.0),
                        ..default()
                    },
                    ui.image(Some(icon)),
                ));
            }
            if let Slot::Goods(_, n) = slot {
                s.spawn((Node {
                    position_type: PositionType::Absolute,
                    right: Val::Percent(6.0),
                    bottom: Val::Percent(2.0),
                    ..default()
                },))
                    .with_children(|c| {
                        c.spawn(ui.text(&n.to_string(), 16.0, Color::WHITE));
                    });
            }
        });
    }
    // The item under the cursor on the parchment.
    let slot = &all[menu.index.min(all.len() - 1)];
    let (name, info) = slot_text(texts, slot);
    parchment(
        root,
        ui,
        &Detail {
            name,
            icon: slot_icon(tables, slot),
            art: None,
            stats: vec![],
            info,
        },
    );
}

fn build(
    commands: &mut Commands,
    ui: &Ui,
    menu: &GameMenu,
    kit: &Kit,
    book: &Book,
    wolf: (f32, f32),
) {
    let (texts, tables) = (&book.texts, &book.tables);
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
            GlobalZIndex(110),
            GameMenuRoot,
            game_ui::Covers,
        ))
        .with_children(|root| {
            root.spawn((
                abs(0.0, 0.0, 100.0, 100.0),
                ui.image(sprite("sb_in_game/MENU_Base_00")),
            ));
            // Tabs, with the switch keys and the brush arrows at either end.
            root.spawn((
                abs(5.3, 6.0, 1.3, 4.6),
                ui.image(sprite("sb_in_game/MENU_Arrow")),
            ));
            root.spawn((
                abs(7.3, 6.4, 1.8, 3.4),
                ui.image(sprite("sb_keytop/KG_Key_Z")),
            ));
            root.spawn((
                abs(90.0, 6.4, 1.8, 3.4),
                ui.image(sprite("sb_keytop/KG_Key_X")),
            ));
            let mut right = ui.image(sprite("sb_in_game/MENU_Arrow"));
            right.flip_x = true;
            root.spawn((abs(92.4, 6.0, 1.3, 4.6), right));
            for (i, name) in TABS.iter().enumerate() {
                let open = i == menu.tab;
                let brush = if open {
                    "sb_in_game/MENU_Cursor_01"
                } else {
                    "sb_in_game/MENU_Caption_01"
                };
                root.spawn((
                    abs(11.5 + i as f32 * 19.0, 4.2, 18.0, 9.0),
                    ui.image(sprite(brush)),
                ))
                .with_children(|t| {
                    t.spawn(Node {
                        justify_content: JustifyContent::Center,
                        ..fill_row()
                    })
                    .with_children(|c| {
                        c.spawn(ui.text(name, 18.0, if open { Color::WHITE } else { TEXT }));
                    });
                });
            }
            // The rule under the tabs.
            root.spawn((
                abs(7.0, 13.6, 86.0, 0.25),
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
            ));
            match menu.tab {
                0 => equipment(root, ui, menu, kit, tables, texts),
                1 | 2 => {
                    let k = menu.tab - 1;
                    let (cats, list) = if menu.tab == 1 {
                        (&INV_CATS, inventory(menu.cat[k], kit, texts, tables))
                    } else {
                        (&SKILL_CATS, skills(menu.cat[k], kit, texts, tables))
                    };
                    let rows: Vec<Row> = list.iter().map(|(r, _)| r.clone()).collect();
                    let detail = list.get(menu.index).map(|(_, d)| d);
                    list_screen(
                        root,
                        ui,
                        (cats, menu.cat[k]),
                        &rows,
                        (menu.index, menu.top),
                        detail,
                    );
                }
                _ => {
                    for (i, label) in OPTION_ITEMS.iter().enumerate() {
                        let sel = i == menu.index;
                        let plate = if sel {
                            "sb_in_game/MENU_Cursor_01"
                        } else {
                            "sb_in_game/MENU_Caption_01"
                        };
                        root.spawn((
                            abs(6.8, 18.0 + i as f32 * 7.55, 22.2, 6.4),
                            ui.image(sprite(plate)),
                        ))
                        .with_children(|b| {
                            b.spawn(Node {
                                justify_content: JustifyContent::Center,
                                ..fill_row()
                            })
                            .with_children(|c| {
                                c.spawn(ui.text(
                                    label,
                                    17.0,
                                    if sel { Color::WHITE } else { TEXT },
                                ));
                            });
                        });
                    }
                }
            }
            if menu.tab != 3 {
                status_bar(root, ui, kit, tables, texts, wolf, book.attack);
            }
            // The key guide: help on the left, the keys on the right.
            let help = match menu.tab {
                0 => "Check equipped items.",
                1 => "Check items in your possession.",
                2 => "View unlocked Skills",
                _ => "Resume, return to the start menu, or quit.",
            };
            root.spawn(abs(5.3, 91.6, 50.0, 4.0)).with_children(|c| {
                c.spawn(ui.text(help, 16.0, DIM));
            });
            key_guide(root, ui, &book.guides);
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book() -> Option<Book> {
        let cache = sekiro_sim::cache::root();
        if !cache.join("params/SkillParam.json").exists() {
            eprintln!("skipping: cache not extracted");
            return None;
        }
        Some(Book::load(&cache, 1))
    }

    #[test]
    fn the_book_names_items_and_classifies_skills() {
        let Some(book) = book() else {
            return;
        };
        for (slot, name) in [
            (Slot::Weapon(70000), "Loaded Shuriken"),
            (Slot::Weapon(5100), "Whirlwind Slash"),
            (Slot::Goods(3000, 0), "Healing Gourd"),
        ] {
            assert_eq!(slot_text(&book.texts, &slot).0, name);
        }
        // Ninjutsu are goods, not SkillParam rows: that category stays empty.
        for cat in &book.tables.skills[..3] {
            assert!(!cat.is_empty());
        }
        assert!(
            book.tables.skills[0]
                .iter()
                .any(|s| book.texts.weapon_name[&s.weapon] == "Whirlwind Slash")
        );
    }

    #[test]
    fn inventory_shows_the_gourd_and_held_materials() {
        let Some(book) = book() else {
            return;
        };
        let kit = Kit::default();
        let quick = inventory(0, &kit, &book.texts, &book.tables);
        assert_eq!(quick[0].0.name, "Healing Gourd");
        assert_eq!(quick[0].0.count.as_deref(), Some("3"));
        let materials = inventory(2, &kit, &book.texts, &book.tables);
        assert!(
            materials
                .iter()
                .any(|(r, _)| r.name == "Scrap Iron" && r.count.as_deref() == Some("20"))
        );
    }

    #[test]
    fn equipment_follows_the_ten_slots_of_the_kit() {
        let Some(book) = book() else {
            return;
        };
        let kit = Kit::default();
        let slots = slots(&kit);
        assert_eq!(slot_text(&book.texts, &slots[0]).0, "Loaded Shuriken");
        assert_eq!(slot_text(&book.texts, &slots[3]).0, "Whirlwind Slash");
        for (slot, id) in slots[4..9].iter().zip(kit.quick_items) {
            assert!(matches!(slot, Slot::Goods(i, n) if *i == id && *n == kit.count(id)));
            assert!(!slot_text(&book.texts, slot).0.is_empty());
        }
        assert!(matches!(slots[9], Slot::Empty));
        assert_eq!(
            slot_icon(&book.tables, &slots[4]).as_deref(),
            Some("item/MENU_ItemIcon_00151")
        );
    }

    #[test]
    fn the_list_window_holds_until_the_cursor_leaves_it() {
        let n = 12;
        let mut top = 0;
        for i in 0..=7 {
            top = keep_visible(top, i, n);
        }
        assert_eq!(top, 2, "the cursor on row 7 shows rows 2-7");
        top = keep_visible(top, 5, n);
        assert_eq!(top, 2, "back up to row 5: the window stays");
        top = keep_visible(top, 1, n);
        assert_eq!(top, 1);
        assert_eq!(keep_visible(9, 11, n), 6, "never past the end");
    }
}

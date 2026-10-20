//! Map items (#136): each item lying on the play blocks glints with the game's common item
//! effect (FFX 61, common_func 20006030) until it is taken; in reach, the "Pick Up Item" guide
//! (ActionButtonParam 4000 textId 10010103 in イベントテキスト) shows with its key, and the
//! action button (E, or Y on a pad) takes it (`Duel::action_target`). A short line then names
//! what was got and how many for 2.5 s.
//!
//! After FwLo's RustSekiro `src/items.rs` (`spawn_line`, the glint) and `src/action_guide.rs`
//! (Fireguy32's pickups); the guide is built like the idol's (`super::idol`).
//! gap: the first-time item dialog and the line's item icon.
use std::collections::BTreeMap;
use std::sync::Arc;

use bevy::prelude::*;
use sekiro_sim::items::{ItemData, MapPickup};

use super::Sim;
use crate::game_ui::{GameFonts, MenuArt, abs};

/// The item glint (`sfxbnd_commoneffects` f000000061).
const GLINT: i32 = 61;
/// イベントテキスト: "Pick Up Item".
const PICK_UP: i32 = 10010103;
/// How long the "got" line stays (s).
const LINE_SECS: f32 = 2.5;
const GUIDE_TEXT: Color = Color::srgb(0.9, 0.87, 0.8);
const COUNT_TEXT: Color = Color::srgb(0.88, 0.85, 0.78);

/// The items lying on the play `blocks`, with the params they read; empty when the params or
/// the blocks' MSBs are missing.
pub(super) fn load(
    cache: &std::path::Path,
    blocks: &BTreeMap<String, serde_json::Value>,
) -> (Vec<MapPickup>, Arc<ItemData>) {
    let data = match ItemData::load(cache) {
        Ok(data) => Arc::new(data),
        Err(e) => {
            warn!("map items: {e}");
            return (Vec::new(), Arc::default());
        }
    };
    let mut pickups = Vec::new();
    for block in blocks.keys() {
        match sekiro_sim::items::map_pickups(cache, block, &data) {
            Ok(found) => pickups.extend(found),
            Err(e) => warn!("map items: {e}"),
        }
    }
    eprintln!("map items: {} on {} blocks", pickups.len(), blocks.len());
    (pickups, data)
}

#[derive(Component)]
struct Guide;
#[derive(Component)]
struct Line;
#[derive(Component)]
struct LineName;
#[derive(Component)]
struct LineCount;

/// Item names (アイテム名) and the pickups already shown in the line.
#[derive(Resource, Default)]
struct Book {
    names: BTreeMap<i32, String>,
    shown: u32,
    since: Option<f32>,
}

/// The glints, the guide and the line; nothing when the play world has no items.
pub(super) fn install(app: &mut App, any: bool) {
    if !any {
        return;
    }
    GameFonts::insert(app);
    MenuArt::insert(app);
    let cache = sekiro_sim::cache::root();
    app.insert_resource(Book {
        names: crate::game_ui::texts(&cache, "item", "アイテム名"),
        ..default()
    })
    .add_systems(Startup, spawn)
    .add_systems(
        Update,
        (glint, show)
            .after(super::simulate)
            .run_if(crate::menu::active),
    );
}

fn spawn(mut commands: Commands, fonts: Res<GameFonts>, art: Res<MenuArt>) {
    let cache = sekiro_sim::cache::root();
    let events = crate::game_ui::menu_texts(&cache, "イベントテキスト");
    let text = |t: String, px: f32, color: Color| fonts.shadowed(t, px, color);
    // The guide, low in the middle like the idol's: "[E] : Pick Up Item".
    commands
        .spawn((
            Guide,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                bottom: percent(5),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: px(2),
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|r| {
            match art.sprite("SB_KeyTop", "KG_Key_E") {
                Some((image, aspect)) => r.spawn((
                    image,
                    Node {
                        height: vh(22.0 / 10.8),
                        aspect_ratio: Some(aspect),
                        ..default()
                    },
                )),
                None => r.spawn(text("E".into(), 16.5, GUIDE_TEXT)),
            };
            let label = events.get(&PICK_UP).cloned().unwrap_or_default();
            r.spawn(text(format!(" : {label}"), 16.5, GUIDE_TEXT));
        });
    // The "got" line above the guide: the item's name and count on the game's item plate.
    let plate = art
        .sprite("SB_In_Game", "MENU_ItemBase_07")
        .map(|(image, _)| ImageNode {
            image_mode: bevy::ui::widget::NodeImageMode::Stretch,
            ..image
        });
    let mut line = commands.spawn((
        Line,
        abs(36.0, 72.0, 28.0, 7.0),
        GlobalZIndex(90),
        Visibility::Hidden,
    ));
    match plate {
        Some(plate) => line.insert(plate),
        None => line.insert(BackgroundColor(Color::BLACK.with_alpha(0.6))),
    };
    line.with_children(|b| {
        b.spawn(Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            column_gap: px(14),
            padding: UiRect::left(percent(7.0)),
            ..default()
        })
        .with_children(|r| {
            r.spawn((LineName, text(String::new(), 18.0, Color::WHITE)));
            r.spawn((LineCount, text(String::new(), 17.0, COUNT_TEXT)));
        });
    });
}

/// One held glint per item still lying on the map, let go once it is taken.
fn glint(sim: NonSend<Sim>, mut fx: ResMut<crate::fxr::FxrPlayer>) {
    for (i, p) in sim.duel.pickups().iter().enumerate() {
        let at = Transform::from_translation(Vec3::from_array(p.position));
        // Negative serials keep clear of the map's entity ids, which other holds use.
        fx.hold(GLINT, -1 - i as i32, at, sim.duel.pickup_lies(i));
    }
}

/// The guide in reach of an item, and the line after a pickup.
#[allow(clippy::type_complexity)]
fn show(
    sim: NonSend<Sim>,
    time: Res<Time<Real>>,
    mut book: ResMut<Book>,
    mut parts: Query<(&mut Visibility, Has<Guide>), Or<(With<Guide>, With<Line>)>>,
    mut texts: Query<(&mut Text, Has<LineName>), Or<(With<LineName>, With<LineCount>)>>,
) {
    let now = time.elapsed_secs();
    let (taken, got) = sim.duel.last_pickup();
    if taken != book.shown {
        book.shown = taken;
        let (id, n) = got.and_then(|g| g.goods.first().copied()).unwrap_or((0, 0));
        if id != 0 {
            let name = book
                .names
                .get(&id)
                .cloned()
                .unwrap_or_else(|| id.to_string());
            for (mut t, is_name) in &mut texts {
                t.0 = if is_name {
                    name.clone()
                } else if n > 1 {
                    format!("x {n}")
                } else {
                    String::new()
                };
            }
            book.since = Some(now);
        }
    }
    let line_up = book.since.is_some_and(|s| now - s < LINE_SECS);
    let near = sim.duel.pickup_in_reach().is_some();
    for (mut v, guide) in &mut parts {
        let on = if guide { near } else { line_up };
        v.set_if_neq(if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

//! Sculptor's Idols (#128): near one, the game's "Rest" guide (イベントテキスト 15000120) shows
//! with its key; the action button (E, or Y on a pad) kneels Wolf there (`Duel::idol_input`,
//! which also makes it the checkpoint), and once he prays the idol menu opens: the idol's name
//! (BonfireWarpParam `msgId` in NTC_メニューテキスト), the game's rows with their help lines,
//! Rest working and the rest greyed, and the key guide. Esc (or B) stands him up.
//!
//! The idols are the play world's objects whose entity is a BonfireWarpParam `bonfireEntityId`
//! (o005000, o115040), facing their front (the object's -Z). Layout, sizes and colours from
//! FwLo's RustSekiro `src/idol.rs`, `src/action_guide.rs` and `src/game_menu.rs` (Fireguy32's
//! idols, set from captures of the game), in percent of the screen and pixels of its
//! 1920 x 1080 stage.
//! gap: the menu's stats plate, Travel, Purchase Spirit Emblems, skills and forms (#73, #96).
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use bevy::prelude::*;
use sekiro_sim::arena::yaw_toward;
use sekiro_sim::deathblow::Placement;
use sekiro_sim::duel::idol::{IdolPhase, IdolSpot};

use super::Sim;
use crate::game_ui::{GameFonts, MenuArt, abs};

/// How far in front of the idol Wolf kneels, and how much further out a respawn stands him (m).
const KNEEL: f32 = 0.9;
const RESPAWN: f32 = 0.4;
/// イベントテキスト: "Rest" (the guide and the first row), and the menu's rows in the game's
/// order (each one's help line is its id + 100000) with whether they work here.
const REST: i32 = 15000120;
const ROWS: [(i32, bool); 10] = [
    (REST, true),
    (15000140, false), // Purchase Spirit Emblems
    (15000150, false), // Travel
    (15000300, false), // Acquire Skills
    (15000170, false), // Enhance Physical Attributes
    (15000600, false), // Enhance Attack Power
    (15000250, false), // Dragonrot Restoration
    (15000902, false), // Change Form
    (15000900, false), // Reflection of Strength
    (15000901, false), // Gauntlet of Strength
];
const HELP: i32 = 100000;
/// NTC_キーガイド: "OK", "Close".
const KEY_OK: i32 = 100000;
const KEY_CLOSE: i32 = 110001;
const TEXT: Color = Color::srgb(0.88, 0.85, 0.78);
const DIM: Color = Color::srgba(0.75, 0.72, 0.66, 0.85);
const GREYED: Color = Color::srgba(0.7, 0.68, 0.64, 0.4);
const CURSOR: Color = Color::srgba(0.8, 0.55, 0.3, 0.95);

/// The play world's idols: where Wolf kneels at each (for the sim) and its name.
#[derive(Resource, Default)]
pub(super) struct Idols {
    pub spots: Vec<IdolSpot>,
    names: Vec<String>,
}

impl Idols {
    /// The idols in the play blocks' `layouts`, standing on `floor(x, z, near_y)`.
    pub fn load(
        cache: &Path,
        layouts: &BTreeMap<String, serde_json::Value>,
        floor: impl Fn(f32, f32, f32) -> Option<f32>,
    ) -> Self {
        let names = crate::game_ui::menu_texts(cache, "NTC_メニューテキスト");
        let warps: HashMap<i64, i64> = sekiro_sim::registry::rows(cache, "BonfireWarpParam")
            .inspect_err(|e| warn!("idols: {e}"))
            .unwrap_or_default()
            .values()
            .filter_map(|r| Some((r["bonfireEntityId"].as_i64()?, r["msgId"].as_i64()?)))
            .collect();
        let mut idols = Self::default();
        for layout in layouts.values() {
            for o in layout["objects"].as_array().into_iter().flatten() {
                let Some(msg) = o["entity_id"].as_i64().and_then(|e| warps.get(&e)) else {
                    continue;
                };
                let Some(pose) = crate::map::pose(o) else {
                    continue;
                };
                let base = pose.translation;
                let out = (pose.rotation.normalize() * Vec3::NEG_Z)
                    .with_y(0.0)
                    .normalize_or(Vec3::Z);
                let at = |d: f32| {
                    let p = base + out * d;
                    p.with_y(floor(p.x, p.z, base.y).unwrap_or(base.y))
                };
                let (kneel, checkpoint) = (at(KNEEL), at(KNEEL + RESPAWN));
                let yaw = yaw_toward(kneel, base);
                idols.spots.push(IdolSpot {
                    kneel: Placement {
                        position: kneel.to_array(),
                        yaw,
                    },
                    checkpoint: Placement {
                        position: checkpoint.to_array(),
                        yaw: yaw + std::f32::consts::PI,
                    },
                });
                idols
                    .names
                    .push(names.get(&(*msg as i32)).cloned().unwrap_or_default());
            }
        }
        idols
    }
}

/// The row picked in the idol menu.
#[derive(Resource, Default)]
struct Row(usize);

#[derive(Component)]
struct Guide;
#[derive(Component)]
struct Menu;
#[derive(Component)]
struct Place;
#[derive(Component)]
struct Cursor(usize);
#[derive(Component)]
struct Label(usize);
/// The help line, with each row's.
#[derive(Component)]
struct HelpLine(Vec<String>);

/// The idol guide and menu; nothing when the play world has no idols.
pub(super) fn install(app: &mut App, idols: Idols) {
    if idols.spots.is_empty() {
        return;
    }
    GameFonts::insert(app);
    MenuArt::insert(app);
    crate::game_ui::install_cover(app);
    app.insert_resource(idols)
        .init_resource::<Row>()
        .init_resource::<crate::menu::Modal>()
        .add_systems(Startup, spawn)
        .add_systems(
            PreUpdate,
            menu_keys
                .after(bevy::input::InputSystems)
                .run_if(crate::menu::active),
        )
        .add_systems(Update, (show, hold_pause).run_if(crate::menu::active));
}

fn spawn(mut commands: Commands, fonts: Res<GameFonts>, art: Res<MenuArt>) {
    let cache = sekiro_sim::cache::root();
    let key = |name: &str, height: f32| {
        art.sprite("SB_KeyTop", name).map(|(image, aspect)| {
            (
                image,
                Node {
                    height: vh(height / 10.8),
                    aspect_ratio: Some(aspect),
                    ..default()
                },
            )
        })
    };
    let events = crate::game_ui::menu_texts(&cache, "イベントテキスト");
    let guides = crate::game_ui::menu_texts(&cache, "NTC_キーガイド");
    let event = |id: i32| events.get(&id).cloned().unwrap_or_default();
    let text = |t: String, px: f32, color: Color| fonts.shadowed(t, px, color);

    // The guide, low in the middle: "[E] : Rest".
    crate::game_ui::spawn_action_guide(&mut commands, &fonts, &art, &event(REST), Guide);

    // The menu: a dark fade on the left with the idol's name, the rows, the help line and the
    // key guide.
    let cursor = art
        .sprite("SB_In_Game", "MENU_Cursor_03")
        .map(|(image, _)| ImageNode {
            image_mode: bevy::ui::widget::NodeImageMode::Stretch,
            ..image.with_color(CURSOR)
        })
        .unwrap_or_default();
    commands
        .spawn((
            Menu,
            crate::game_ui::Covers,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                ..default()
            },
            GlobalZIndex(110),
            Visibility::Hidden,
        ))
        .with_children(|root| {
            root.spawn((abs(0.0, 0.0, 33.0, 100.0), BackgroundColor(fade(0.93))));
            root.spawn((
                abs(33.0, 0.0, 10.0, 100.0),
                BackgroundGradient(vec![
                    LinearGradient::to_right(vec![fade(0.93).into(), fade(0.0).into()]).into(),
                ]),
            ));
            root.spawn(abs(5.9, 7.6, 30.0, 5.0))
                .with_child((Place, text(String::new(), 21.0, Color::WHITE)));
            root.spawn((
                abs(0.0, 12.8, 32.0, 0.2),
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.25)),
            ));
            for (i, (id, _)) in ROWS.iter().enumerate() {
                let t = 14.6 + i as f32 * 5.6;
                let row = abs(4.0, t, 30.0, 4.8);
                root.spawn((Cursor(i), row.clone(), cursor.clone()));
                root.spawn((
                    Node {
                        align_items: AlignItems::Center,
                        // 6.5 % of the row.
                        padding: UiRect::left(percent(6.5 * 0.3)),
                        ..row
                    },
                    children![(Label(i), text(event(*id), 17.0, TEXT))],
                ));
                root.spawn((
                    abs(5.9, t + 4.9, 25.5, 0.15),
                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.18)),
                ));
            }
            let lines = ROWS.iter().map(|(id, _)| event(id + HELP)).collect();
            root.spawn(abs(5.9, 89.6, 30.0, 4.0))
                .with_child((HelpLine(lines), text(String::new(), 16.5, TEXT)));
            root.spawn(Node {
                position_type: PositionType::Absolute,
                left: percent(5.3),
                top: percent(93.6),
                align_items: AlignItems::Center,
                column_gap: px(4),
                ..default()
            })
            .with_children(|r| {
                for (name, label) in [("KG_Key_Enter", KEY_OK), ("KG_Key_Esc", KEY_CLOSE)] {
                    if let Some(icon) = key(name, 20.0) {
                        r.spawn(icon);
                    }
                    let label = guides.get(&label).cloned().unwrap_or_default();
                    r.spawn(text(format!(":{label}  "), 16.0, DIM));
                }
            });
        });
}

fn fade(alpha: f32) -> Color {
    Color::BLACK.with_alpha(alpha)
}

/// The menu's keys while Wolf prays: up and down pick a row, OK (Enter, Space, E, A) takes it,
/// Esc or back (Q, B) stands him up.
fn menu_keys(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    mut row: ResMut<Row>,
    mut sim: NonSendMut<Sim>,
) {
    if sim.duel.idol_phase() != Some(IdolPhase::Praying) {
        return;
    }
    let pad = |b| pads.iter().any(|p| p.just_pressed(b));
    let key = |k: &[KeyCode]| keys.any_just_pressed(k.iter().copied());
    if key(&[KeyCode::ArrowDown, KeyCode::KeyS]) || pad(GamepadButton::DPadDown) {
        row.0 = (row.0 + 1) % ROWS.len();
    }
    if key(&[KeyCode::ArrowUp, KeyCode::KeyW]) || pad(GamepadButton::DPadUp) {
        row.0 = (row.0 + ROWS.len() - 1) % ROWS.len();
    }
    if crate::menu::ok_pressed(&keys, &pads) && ROWS[row.0].0 == REST {
        sim.duel.rest_at_idol();
    }
    if key(&[KeyCode::Escape, KeyCode::Backspace]) || crate::menu::back_pressed(&keys, &pads) {
        sim.duel.leave_idol();
    }
}

/// The idol menu is up while Wolf prays and through a rest.
fn menu_open(sim: &Sim) -> bool {
    matches!(
        sim.duel.idol_phase(),
        Some(IdolPhase::Praying | IdolPhase::Resting)
    )
}

/// The guide near an idol and the menu while Wolf prays.
#[allow(clippy::type_complexity)]
fn show(
    sim: NonSend<Sim>,
    row: Res<Row>,
    idols: Res<Idols>,
    mut shown_for: Local<Option<(usize, usize)>>,
    mut parts: Query<
        (&mut Visibility, Has<Guide>, Option<&Cursor>),
        Or<(With<Guide>, With<Menu>, With<Cursor>)>,
    >,
    mut labels: Query<(&Label, &mut TextColor)>,
    mut texts: Query<(&mut Text, Option<&HelpLine>), Or<(With<Place>, With<HelpLine>)>>,
) {
    let near = sim.duel.idol_in_reach().is_some();
    let open = menu_open(&sim);
    for (mut v, guide, cursor) in &mut parts {
        let on = match (guide, cursor) {
            (true, _) => near,
            (_, Some(c)) => c.0 == row.0,
            _ => open,
        };
        v.set_if_neq(if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
    // The texts change only with the idol and the row.
    let now = sim.duel.idol().filter(|_| open).map(|i| (i, row.0));
    if now.is_none() || *shown_for == now {
        return;
    }
    *shown_for = now;
    let (idol, row) = now.unwrap();
    for (l, mut color) in &mut labels {
        color.0 = match (ROWS[l.0].1, l.0 == row) {
            (false, _) => GREYED,
            (true, true) => Color::WHITE,
            (true, false) => TEXT,
        };
    }
    for (mut t, help) in &mut texts {
        t.0 = match help {
            Some(lines) => lines.0[row].clone(),
            None => idols.names.get(idol).cloned().unwrap_or_default(),
        };
    }
}

/// The pause menu stays shut while Wolf is at an idol, since Esc stands him up.
fn hold_pause(sim: NonSend<Sim>, mut modal: ResMut<crate::menu::Modal>) {
    let at_idol = sim.duel.idol_phase().is_some();
    if modal.0 != at_idol {
        modal.0 = at_idol;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_idols_of_a_block_are_found_named_and_faced() {
        let cache = sekiro_sim::cache::root();
        let Ok(layout) = std::fs::read(cache.join("maps/m11_00_00_00/layout.json")) else {
            eprintln!("skipping: cache not extracted");
            return;
        };
        let layouts = BTreeMap::from([(
            "m11_00_00_00".to_string(),
            serde_json::from_slice(&layout).unwrap(),
        )]);
        let idols = Idols::load(&cache, &layouts, |_, _, y| Some(y));
        assert_eq!(idols.spots.len(), 8, "{:?}", idols.names);
        assert!(idols.names.iter().any(|n| n == "Ashina Castle Gate"));
        assert!(
            idols.names.iter().all(|n| !n.is_empty()),
            "{:?}",
            idols.names
        );
        for s in &idols.spots {
            let kneel = Vec3::from_array(s.kneel.position);
            let out = Vec3::from_array(s.checkpoint.position) - kneel;
            assert!((out.length() - RESPAWN).abs() < 1e-3);
            // Kneeling he faces the idol; the respawn stands him further out, facing away.
            let facing = sekiro_sim::arena::forward(s.kneel.yaw);
            assert!(facing.dot(out.normalize()) < -0.999, "{s:?}");
            let away = sekiro_sim::arena::forward(s.checkpoint.yaw);
            assert!(away.dot(out.normalize()) > 0.999, "{s:?}");
        }
    }
}

//! Sculptor's Idols (#128): near one, the game's "Rest" guide (イベントテキスト 15000120) shows
//! with its key; E (or Y on a pad) kneels Wolf there (`Duel::kneel_at_idol`, which also makes it
//! the checkpoint), and once he prays the idol menu opens: the idol's name (BonfireWarpParam
//! `msgId` in NTC_メニューテキスト), the game's rows with their help lines, Rest working and the
//! rest greyed, and the key guide. Esc (or B) stands him up.
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
use sekiro_sim::deathblow::Placement;
use sekiro_sim::duel::idol::IdolPhase;

use super::{LiveInput, Sim};
use crate::game_ui::{GameFonts, MenuArt};

/// How far in front of the idol Wolf kneels, and how much further out a respawn stands him (m).
const KNEEL: f32 = 0.9;
const RESPAWN: f32 = 0.4;
/// How near the kneeling spot the guide shows (m across, m up or down).
const REACH: f32 = 1.3;
const REACH_Y: f32 = 0.6;
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
const GUIDE_TEXT: Color = Color::srgb(0.9, 0.87, 0.8);
const CURSOR: Color = Color::srgba(0.8, 0.55, 0.3, 0.95);

/// One idol of the play world.
#[derive(Debug, Clone)]
pub(super) struct Idol {
    pub name: String,
    /// Where Wolf kneels, facing it.
    pub kneel: Placement,
    /// Where a respawn stands him, facing away.
    pub checkpoint: Placement,
}

#[derive(Resource, Default)]
pub(super) struct Idols(pub Vec<Idol>);

impl Idols {
    /// The idols in the play blocks' `layouts`, standing on `floor(x, z, near_y)`.
    pub fn load(
        cache: &Path,
        layouts: &BTreeMap<String, serde_json::Value>,
        floor: impl Fn(f32, f32, f32) -> Option<f32>,
    ) -> Self {
        let names = crate::game_ui::menu_texts(cache, "NTC_メニューテキスト");
        let warps: HashMap<i64, i64> = std::fs::read(cache.join("params/BonfireWarpParam.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
            .and_then(|j| {
                let rows = j["rows"].as_array()?.iter();
                Some(
                    rows.filter_map(|r| {
                        Some((r["bonfireEntityId"].as_i64()?, r["msgId"].as_i64()?))
                    })
                    .collect(),
                )
            })
            .unwrap_or_default();
        let f = |v: &serde_json::Value, i: usize| v[i].as_f64().unwrap_or(0.0) as f32;
        let mut idols = Vec::new();
        for layout in layouts.values() {
            for o in layout["objects"].as_array().into_iter().flatten() {
                let Some(msg) = o["entity_id"].as_i64().and_then(|e| warps.get(&e)) else {
                    continue;
                };
                let (t, r) = (&o["translation"], &o["rotation"]);
                let base = Vec3::new(f(t, 0), f(t, 1), f(t, 2));
                let rot = Quat::from_xyzw(f(r, 0), f(r, 1), f(r, 2), f(r, 3)).normalize();
                let out = (rot * Vec3::NEG_Z).with_y(0.0).normalize_or(Vec3::Z);
                let at = |d: f32, yaw: f32| {
                    let p = base + out * d;
                    Placement {
                        position: [p.x, floor(p.x, p.z, base.y).unwrap_or(base.y), p.z],
                        yaw,
                    }
                };
                idols.push(Idol {
                    name: names.get(&(*msg as i32)).cloned().unwrap_or_default(),
                    kneel: at(KNEEL, f32::atan2(out.x, out.z)),
                    checkpoint: at(KNEEL + RESPAWN, f32::atan2(-out.x, -out.z)),
                });
            }
        }
        Self(idols)
    }

    /// The idol whose kneeling spot Wolf's feet are at.
    fn near(&self, feet: [f32; 3]) -> Option<usize> {
        self.0.iter().position(|i| {
            let [x, y, z] = i.kneel.position;
            (x - feet[0]).hypot(z - feet[2]) < REACH && (y - feet[1]).abs() < REACH_Y
        })
    }
}

/// The idol Wolf was sent to kneel at (from the press until he stands again) and the row picked.
#[derive(Resource, Default)]
struct Seat {
    idol: Option<usize>,
    /// The kneel has started (the phase lags the press by a step).
    knelt: bool,
    row: usize,
}

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

pub(super) fn install(app: &mut App, idols: Idols) {
    GameFonts::insert(app);
    MenuArt::insert(app);
    app.insert_resource(idols)
        .init_resource::<Seat>()
        .add_systems(Startup, spawn)
        .add_systems(
            PreUpdate,
            menu_keys
                .after(bevy::input::InputSystems)
                .before(crate::menu::MenuKeys)
                .run_if(crate::menu::active),
        )
        .add_systems(
            Update,
            (kneel, show)
                .chain()
                .after(super::gather_input)
                .run_if(crate::menu::active),
        );
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
    let text = |t: String, px: f32, color: Color| {
        (
            Text::new(t),
            fonts.stage(px),
            TextColor(color),
            TextShadow {
                offset: Vec2::splat(1.0),
                color: Color::srgba(0.0, 0.0, 0.0, 0.8),
            },
        )
    };

    // The guide, low in the middle: "[E] : Rest".
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
            match key("KG_Key_E", 22.0) {
                Some(icon) => r.spawn(icon),
                None => r.spawn(text("E".into(), 16.5, GUIDE_TEXT)),
            };
            r.spawn(text(format!(" : {}", event(REST)), 16.5, GUIDE_TEXT));
        });

    // The menu: a dark fade on the left with the idol's name, the rows, the help line and the
    // key guide.
    commands
        .spawn((
            Menu,
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
            for k in 0..10 {
                let a = 0.93 * (1.0 - (k as f32 + 1.0) / 11.0);
                root.spawn((
                    abs(33.0 + k as f32, 0.0, 1.0, 100.0),
                    BackgroundColor(fade(a)),
                ));
            }
            root.spawn(abs(5.9, 7.6, 30.0, 5.0))
                .with_child((Place, text(String::new(), 21.0, Color::WHITE)));
            root.spawn((
                abs(0.0, 12.8, 32.0, 0.2),
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.25)),
            ));
            for (i, (id, _)) in ROWS.iter().enumerate() {
                let t = 14.6 + i as f32 * 5.6;
                let cursor = art
                    .sprite("SB_In_Game", "MENU_Cursor_03")
                    .map(|(image, _)| image.with_color(CURSOR))
                    .unwrap_or_default();
                root.spawn((Cursor(i), abs(4.0, t, 30.0, 4.8), cursor));
                root.spawn((
                    Node {
                        align_items: AlignItems::Center,
                        // 6.5 % of the row.
                        padding: UiRect::left(percent(6.5 * 0.3)),
                        ..abs(4.0, t, 30.0, 4.8)
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

/// A box at (left, top) of (width, height), all in percent of the screen.
fn abs(left: f32, top: f32, width: f32, height: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: percent(left),
        top: percent(top),
        width: percent(width),
        height: percent(height),
        ..default()
    }
}

fn fade(alpha: f32) -> Color {
    Color::BLACK.with_alpha(alpha)
}

/// E (Y) at an idol kneels Wolf; while he is at one, his own input is held.
fn kneel(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    idols: Res<Idols>,
    mut seat: ResMut<Seat>,
    mut sim: NonSendMut<Sim>,
    mut live: ResMut<LiveInput>,
) {
    let phase = sim.duel.idol_phase();
    // Off the idol once he stood up, or something else took him (a hit, death).
    if seat.idol.is_some() {
        if phase.is_some() {
            seat.knelt = true;
        } else if seat.knelt {
            *seat = Seat::default();
        }
    }
    let pressed = keys.just_pressed(KeyCode::KeyE)
        || pads.iter().any(|p| p.just_pressed(GamepadButton::North));
    if seat.idol.is_none()
        && pressed
        && let Some(i) = idols.near(sim.duel.player.body.position)
        && sim
            .duel
            .kneel_at_idol(idols.0[i].kneel, idols.0[i].checkpoint)
    {
        *seat = Seat {
            idol: Some(i),
            ..default()
        };
    }
    if seat.idol.is_some() {
        live.frame = sekiro_sim::InputFrame {
            camera_yaw: live.frame.camera_yaw,
            ..default()
        };
    }
}

/// The menu's keys while Wolf prays: up and down pick a row, Enter (or E, A) takes it, Esc
/// (or B) stands him up. They run before the pause menu, which would take Esc.
fn menu_keys(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    mut seat: ResMut<Seat>,
    mut sim: NonSendMut<Sim>,
) {
    if seat.idol.is_none() || sim.duel.idol_phase() != Some(IdolPhase::Praying) {
        return;
    }
    let pad = |b| pads.iter().any(|p| p.just_pressed(b));
    let key = |k: &[KeyCode]| keys.any_just_pressed(k.iter().copied());
    if key(&[KeyCode::ArrowDown, KeyCode::KeyS]) || pad(GamepadButton::DPadDown) {
        seat.row = (seat.row + 1) % ROWS.len();
    }
    if key(&[KeyCode::ArrowUp, KeyCode::KeyW]) || pad(GamepadButton::DPadUp) {
        seat.row = (seat.row + ROWS.len() - 1) % ROWS.len();
    }
    if (key(&[KeyCode::Enter, KeyCode::KeyE]) || pad(GamepadButton::South))
        && ROWS[seat.row].0 == REST
    {
        sim.duel.rest_at_idol();
    }
    if key(&[KeyCode::Escape, KeyCode::Backspace]) || pad(GamepadButton::East) {
        sim.duel.leave_idol();
    }
    // Not for the pause menu or the next kneel.
    for k in [KeyCode::Escape, KeyCode::Enter, KeyCode::KeyE] {
        keys.clear_just_pressed(k);
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn show(
    sim: NonSend<Sim>,
    seat: Res<Seat>,
    idols: Res<Idols>,
    mut guide: Query<&mut Visibility, (With<Guide>, Without<Menu>)>,
    mut menu: Query<&mut Visibility, (With<Menu>, Without<Guide>, Without<Cursor>)>,
    mut cursors: Query<(&Cursor, &mut Visibility), (Without<Guide>, Without<Menu>)>,
    mut labels: Query<(&Label, &mut TextColor)>,
    mut place: Query<&mut Text, (With<Place>, Without<HelpLine>)>,
    mut help: Query<(&HelpLine, &mut Text), Without<Place>>,
) {
    let alive = !sim.duel.rules.fighters[0].dead();
    let near = seat.idol.is_none() && alive && idols.near(sim.duel.player.body.position).is_some();
    for mut v in &mut guide {
        v.set_if_neq(shown(near));
    }
    let open = seat.idol.is_some() && sim.duel.idol_phase() == Some(IdolPhase::Praying);
    for mut v in &mut menu {
        v.set_if_neq(shown(open));
    }
    if !open {
        return;
    }
    let idol = seat.idol.and_then(|i| idols.0.get(i));
    for mut t in &mut place {
        t.set_if_neq(Text::new(idol.map_or("", |i| &i.name)));
    }
    for (c, mut v) in &mut cursors {
        v.set_if_neq(if c.0 == seat.row {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
    for (l, mut color) in &mut labels {
        let c = match (ROWS[l.0].1, l.0 == seat.row) {
            (false, _) => GREYED,
            (true, true) => Color::WHITE,
            (true, false) => TEXT,
        };
        color.set_if_neq(TextColor(c));
    }
    for (lines, mut t) in &mut help {
        t.set_if_neq(Text::new(lines.0[seat.row].clone()));
    }
}

fn shown(on: bool) -> Visibility {
    if on {
        Visibility::Inherited
    } else {
        Visibility::Hidden
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
        let names: Vec<_> = idols.0.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(idols.0.len(), 8, "{names:?}");
        assert!(names.contains(&"Ashina Castle Gate"), "{names:?}");
        assert!(names.iter().all(|n| !n.is_empty()), "{names:?}");
        for i in &idols.0 {
            let [kx, _, kz] = i.kneel.position;
            let [cx, _, cz] = i.checkpoint.position;
            // Kneeling he faces the idol; the respawn stands him further out, facing away.
            let facing = Vec2::new(-i.kneel.yaw.sin(), -i.kneel.yaw.cos());
            let out = Vec2::new(cx - kx, cz - kz);
            assert!((out.length() - RESPAWN).abs() < 1e-3);
            assert!(facing.dot(out.normalize()) < -0.999, "{i:?}");
            assert!((i.checkpoint.yaw - i.kneel.yaw).abs() - std::f32::consts::PI < 1e-3);
            assert_eq!(
                idols.near(i.kneel.position).map(|n| &idols.0[n].name),
                Some(&i.name)
            );
        }
        assert_eq!(idols.near([0.0, 1000.0, 0.0]), None);
    }
}

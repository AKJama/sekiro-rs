//! HP and posture bars laid out like Sekiro's: Wolf's vitality at the bottom left and his
//! posture bar at the bottom centre; the enemy's vitality and posture float above its head.
//! Posture bars grow from the centre outward as posture damage builds, from yellow toward red,
//! and turn solid red when broken. A red mark shows when a deathblow is available. Above Wolf's
//! vitality, a purple bar shows poison build-up and a line names the status effects on him
//! (poisoned, recovery sealed, ...) with the seconds left. A red 危 marks a perilous attack's
//! warning on Wolf (see [`update_perilous`]).

use bevy::prelude::*;

/// What the bars show, filled by the simulation's owner every frame.
#[derive(Resource, Default, Clone)]
pub struct Gauges {
    /// `(current, max)` HP, Wolf then the enemy.
    pub hp: [(i32, i32); 2],
    /// `(remaining, max)` posture.
    pub posture: [(i32, i32); 2],
    /// Where the enemy's head is, for its floating bars.
    pub enemy_head: Option<Vec3>,
    /// The enemy's bars are shown: it is alive and, in a boss fight run by the map events,
    /// the events show its health bar (the Owl's is hidden during his heat-up).
    pub enemy_alive: bool,
    /// A map-script boss bar stays on screen rather than following the character's head.
    pub boss_bar: bool,
    pub deathblow: bool,
    /// The enemy's name over its bars.
    pub enemy_name: &'static str,
    /// Deathblow marks the enemy has left (NpcParam `ninsatuNum`, 0 counting as one).
    pub enemy_marks: i32,
    /// Wolf's poison build-up as a fraction of his resistance.
    pub poison: f32,
    /// Wolf's status effects, one line (empty for none).
    pub status: String,
    /// Where the perilous-attack kanji is drawn: Wolf's dummy poly 220 (`Spine2`, where
    /// SpEffectVfxParam 48100 plays the game's kanji effect), lifted to above his head.
    pub perilous_at: Option<Vec3>,
    /// Seconds since a perilous warning last reached Wolf; `None` before the first.
    pub perilous_age: Option<f32>,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum Fill {
    Hp(usize),
    Posture(usize),
    /// Wolf's poison build-up.
    Poison,
}

/// Wolf's status line.
#[derive(Component)]
pub struct StatusText;

/// The poison bar's frame, shown only while poison builds up.
#[derive(Component)]
pub struct PoisonRow;

#[derive(Component)]
pub struct Label(usize);

/// The enemy's floating group.
#[derive(Component)]
pub struct EnemyPanel;

#[derive(Component)]
pub struct DeathblowMark;

const WOLF_HP_WIDTH: f32 = 340.0;
const WOLF_POSTURE_WIDTH: f32 = 420.0;
const ENEMY_WIDTH: f32 = 150.0;
const POISON_WIDTH: f32 = 160.0;

fn frame_color() -> Color {
    Color::srgba(0.02, 0.02, 0.02, 0.75)
}

/// A bar of `width` x `height` with a dark frame; `centred` fills from the middle out.
fn bar(parent: &mut ChildSpawnerCommands, fill: Fill, width: f32, height: f32, centred: bool) {
    parent
        .spawn((
            Node {
                width: if matches!(fill, Fill::Hp(1) | Fill::Posture(1)) {
                    percent(100.0)
                } else {
                    px(width)
                },
                height: px(height),
                padding: UiRect::all(px(2)),
                justify_content: if centred {
                    JustifyContent::Center
                } else {
                    JustifyContent::FlexStart
                },
                ..default()
            },
            BackgroundColor(frame_color()),
        ))
        .with_children(|b| {
            b.spawn((
                fill,
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ));
        });
}

fn label(parent: &mut ChildSpawnerCommands, actor: usize, size: f32) {
    parent.spawn((
        Label(actor),
        Text::new(""),
        TextFont {
            font_size: bevy::text::FontSize::Px(size),
            ..default()
        },
        TextColor(Color::srgb(0.92, 0.9, 0.85)),
    ));
}

/// Spawns the bars (call once at startup).
pub fn spawn(commands: &mut Commands) {
    // Wolf: vitality bottom left.
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: px(28),
            bottom: px(26),
            flex_direction: FlexDirection::Column,
            row_gap: px(3),
            ..default()
        })
        .with_children(|p| {
            p.spawn((
                StatusText,
                Text::new(""),
                TextFont {
                    font_size: bevy::text::FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(0.78, 0.55, 0.95)),
            ));
            p.spawn((PoisonRow, Node::default(), Visibility::Hidden))
                .with_children(|r| bar(r, Fill::Poison, POISON_WIDTH, 6.0, false));
            label(p, 0, 13.0);
            bar(p, Fill::Hp(0), WOLF_HP_WIDTH, 12.0, false);
        });
    // Wolf: posture bottom centre.
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            bottom: px(64),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|p| bar(p, Fill::Posture(0), WOLF_POSTURE_WIDTH, 9.0, true));
    // The enemy: posture over vitality, above its head.
    commands
        .spawn((
            EnemyPanel,
            Node {
                position_type: PositionType::Absolute,
                width: px(ENEMY_WIDTH),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(3),
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|p| {
            // The deathblow mark: a red square with a dark rim (the default font has no
            // diamond glyph).
            p.spawn((
                DeathblowMark,
                Node {
                    width: px(14),
                    height: px(14),
                    border: UiRect::all(px(2)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.85, 0.05, 0.05)),
                BorderColor::all(Color::srgb(0.15, 0.0, 0.0)),
                Visibility::Hidden,
            ));
            bar(p, Fill::Posture(1), ENEMY_WIDTH, 7.0, true);
            bar(p, Fill::Hp(1), ENEMY_WIDTH, 6.0, false);
            label(p, 1, 11.0);
        });
}

/// Posture colour: yellow when light, orange, then red near breaking.
fn posture_color(fraction: f32) -> Color {
    let t = fraction.clamp(0.0, 1.0);
    Color::srgb(0.95, 0.78 - 0.62 * t, 0.18 - 0.12 * t)
}

/// Updates widths, colours and the floating panel.
#[allow(clippy::type_complexity)]
pub fn update(
    gauges: Res<Gauges>,
    (recording, overlay_size): (
        Option<Res<crate::brain_overlay::RecordingBrain>>,
        Query<&ComputedNode, With<crate::brain_overlay::Panel>>,
    ),
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut fills: Query<(&Fill, &mut Node, &mut BackgroundColor), Without<EnemyPanel>>,
    mut labels: Query<(&Label, &mut Text)>,
    mut panel: Query<(&mut Node, &mut Visibility), (With<EnemyPanel>, Without<Fill>)>,
    mut mark: Query<&mut Visibility, (With<DeathblowMark>, Without<EnemyPanel>)>,
) {
    for (fill, mut node, mut color) in &mut fills {
        let (fraction, c) = match *fill {
            Fill::Hp(i) => {
                let (hp, max) = gauges.hp[i];
                let f = hp.max(0) as f32 / max.max(1) as f32;
                (f, Color::srgb(0.72, 0.1, 0.08))
            }
            Fill::Posture(i) => {
                let (left, max) = gauges.posture[i];
                let f = ((max - left) as f32 / max.max(1) as f32).clamp(0.0, 1.0);
                let c = if left < 1 {
                    Color::srgb(0.9, 0.05, 0.04)
                } else {
                    posture_color(f)
                };
                (f, c)
            }
            Fill::Poison => (gauges.poison, Color::srgb(0.55, 0.2, 0.75)),
        };
        node.width = percent(fraction * 100.0);
        color.0 = c;
    }
    for (l, mut text) in &mut labels {
        let (hp, max) = gauges.hp[l.0];
        text.0 = if l.0 == 0 {
            format!("Wolf   {hp} / {max}")
        } else {
            let marks = if gauges.enemy_marks > 0 {
                format!(
                    "   {} deathblow{}",
                    gauges.enemy_marks,
                    if gauges.enemy_marks == 1 { "" } else { "s" }
                )
            } else {
                String::new()
            };
            format!("{}   {hp}{marks}", gauges.enemy_name)
        };
    }
    let Ok((mut node, mut vis)) = panel.single_mut() else {
        return;
    };
    node.width = px(if gauges.boss_bar { 350.0 } else { ENEMY_WIDTH });
    if gauges.boss_bar {
        node.left = px(28.0);
        node.top = px(190.0);
        *vis = if gauges.enemy_alive {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if let Ok(mut mark) = mark.single_mut() {
            *mark = if gauges.deathblow {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
        return;
    }
    let screen = gauges
        .enemy_head
        .filter(|_| gauges.enemy_alive)
        .and_then(|head| {
            let (cam, tf) = camera.single().ok()?;
            cam.world_to_viewport(tf, head).ok()
        });
    match screen {
        Some(p) => {
            let mut left = p.x - ENEMY_WIDTH * 0.5;
            if recording.as_ref().is_some_and(|r| r.enabled)
                && let Ok(size) = overlay_size.single()
                && let Ok((cam, _)) = camera.single()
                && let Some(viewport) = cam.logical_viewport_size()
            {
                let overlay_left = viewport.x - 24.0 - 344.0;
                let overlay_bottom = 24.0 + size.size().y * size.inverse_scale_factor;
                if left + ENEMY_WIDTH > overlay_left - 12.0
                    && p.y > 24.0
                    && p.y - 46.0 < overlay_bottom
                {
                    left = overlay_left - ENEMY_WIDTH - 12.0;
                }
            }
            node.left = px(left);
            node.top = px(p.y - 46.0);
            *vis = Visibility::Inherited;
        }
        None => *vis = Visibility::Hidden,
    }
    if let Ok(mut m) = mark.single_mut() {
        *m = if gauges.deathblow {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// Updates Wolf's status line and shows the poison bar while poison builds up.
pub fn update_status(
    gauges: Res<Gauges>,
    mut status: Query<(&mut Text, &mut TextColor), With<StatusText>>,
    mut poison_row: Query<&mut Visibility, With<PoisonRow>>,
) {
    if let Ok((mut t, mut color)) = status.single_mut() {
        if t.0 != gauges.status {
            t.0.clone_from(&gauges.status);
        }
        color.0 = if gauges.status.contains("Electrified") || gauges.status.contains("Lightning") {
            Color::srgb(0.4, 0.8, 1.0)
        } else {
            Color::WHITE
        };
    }
    if let Ok(mut v) = poison_row.single_mut() {
        *v = if gauges.poison > 0.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// The perilous-attack kanji.
#[derive(Component)]
pub struct PerilousMark;

/// How long the kanji shows, seconds: pops in, holds, fades. The game's effect (FXR 600035) is
/// not decoded; 0.8 s is FwLo's RustSekiro value (`src/hud.rs`), and close to what the game
/// shows.
const PERILOUS_SHOW: f32 = 0.8;

/// Windows fonts that have the kanji, tried in order; without one the mark falls back to "!!".
const KANJI_FONTS: [&str; 3] = ["YuGothB.ttc", "msgothic.ttc", "meiryo.ttc"];

/// Shows the red 危 over Wolf when a perilous warning reaches him (`docs/COMBAT-RULES.md`,
/// "Perilous attacks"). A stand-in for the game's effect: its texture lives in the install's
/// effect archive and its FXR is not decoded, so a Japanese system font draws the glyph (FwLo's
/// RustSekiro does the same, `src/hud.rs`). Spawns the mark on its first run.
pub fn update_perilous(
    mut commands: Commands,
    mut fonts: ResMut<Assets<Font>>,
    gauges: Res<Gauges>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut mark: Query<
        (&mut Node, &mut TextColor, &mut TextShadow, &mut Visibility),
        With<PerilousMark>,
    >,
) {
    let Ok((mut node, mut color, mut shadow, mut vis)) = mark.single_mut() else {
        let font = KANJI_FONTS
            .iter()
            .find_map(|f| std::fs::read(std::path::Path::new("C:/Windows/Fonts").join(f)).ok());
        let glyph = if font.is_some() { "危" } else { "!!" };
        let font = font
            .map(|bytes| bevy::text::FontSource::Handle(fonts.add(Font::from_bytes(bytes))))
            .unwrap_or_default();
        commands.spawn((
            PerilousMark,
            Text::new(glyph),
            TextFont {
                font,
                font_size: bevy::text::FontSize::Px(PERILOUS_SIZE),
                ..default()
            },
            TextColor(Color::NONE),
            TextShadow {
                offset: Vec2::splat(2.0),
                color: Color::srgba(0.1, 0.0, 0.0, 0.0),
            },
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            Visibility::Hidden,
        ));
        return;
    };
    let shown = gauges.perilous_age.filter(|&a| a < PERILOUS_SHOW);
    let screen = shown.zip(gauges.perilous_at).and_then(|(_, at)| {
        let (cam, tf) = camera.single().ok()?;
        cam.world_to_viewport(tf, at).ok()
    });
    match (shown, screen) {
        (Some(age), Some(p)) => {
            // Pops in over the first tenth of a second, holds, fades over the last third.
            let pop = (age / 0.1).min(1.0);
            let fade = ((PERILOUS_SHOW - age) / (PERILOUS_SHOW / 3.0)).min(1.0);
            let a = pop.min(fade);
            color.0 = Color::srgba(0.92, 0.05, 0.04, a);
            shadow.color = Color::srgba(0.1, 0.0, 0.0, 0.8 * a);
            node.left = px(p.x - PERILOUS_SIZE * 0.5);
            node.top = px(p.y - PERILOUS_SIZE * 0.5);
            *vis = Visibility::Inherited;
        }
        _ => *vis = Visibility::Hidden,
    }
}

/// The kanji's size on screen, pixels.
const PERILOUS_SIZE: f32 = 96.0;

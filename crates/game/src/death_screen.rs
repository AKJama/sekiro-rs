//! The death screen (#123): while Wolf lies dead, the game's 死 brush mark (`MENU_Death`, menu
//! atlas SB_TextEffect) fades in white with DEATH under it and slowly turns red while the screen
//! darkens; then everything fades to black just before the restart (`RESPAWN_DELAY` after the
//! death), and the black lifts off the restarted scene.
//! Without the menu art the kanji font draws 死, and without that DEATH alone.
//!
//! Timings from FwLo's RustSekiro `src/menu.rs` `death_overlay` (set by eye there from the
//! game's screen). gap: the game's own movie for it (01_210_dead.gfx: the ink bleed, its
//! timings) is not decoded, and there is no resurrection yet (#124), so every death is final.
use bevy::prelude::*;
use sekiro_sim::duel::RESPAWN_DELAY;

use crate::combat_hud::Gauges;
use crate::game_ui::{GameFonts, MenuAtlas};

/// Seconds: the mark's fade in, its turn from white to red, the darkening behind it (to
/// `SHADE`), the fade to black that ends at the restart, and the black lifting after it.
const MARK_IN: f32 = 0.8;
const MARK_RED: f32 = 4.0;
const DARKEN: f32 = 4.0;
const SHADE: f32 = 0.45;
const FINAL_FADE: f32 = 2.5;
const LIFT: f32 = 0.6;
const RED: Color = Color::srgb(0.75, 0.06, 0.05);

#[derive(Component)]
struct DeathScreen;
#[derive(Component)]
struct Mark;
#[derive(Component)]
struct Caption;

pub fn install(app: &mut App) {
    app.add_systems(Startup, spawn).add_systems(Update, update);
}

fn spawn(
    mut commands: Commands,
    mut fonts: ResMut<Assets<Font>>,
    mut images: ResMut<Assets<Image>>,
) {
    let cache = sekiro_sim::cache::root();
    let game_fonts = GameFonts::load(&cache, &mut fonts);
    let art = MenuAtlas::load(&cache, &["SB_TextEffect"], &mut images)
        .get("SB_TextEffect")
        .and_then(|atlas| atlas.sprite("MENU_Death"));
    commands
        .spawn((
            DeathScreen,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::NONE),
            GlobalZIndex(150),
            Visibility::Hidden,
        ))
        .with_children(|c| {
            match (art, &game_fonts.kanji) {
                (Some((image, aspect)), _) => {
                    c.spawn((
                        Mark,
                        image.with_color(Color::NONE),
                        Node {
                            height: vh(30),
                            aspect_ratio: Some(aspect),
                            ..default()
                        },
                    ));
                }
                (None, Some(_)) => {
                    c.spawn((
                        Mark,
                        Text::new("死"),
                        game_fonts.kanji(150.0),
                        TextColor(Color::NONE),
                    ));
                }
                (None, None) => {}
            }
            c.spawn((
                Caption,
                Text::new("D E A T H"),
                game_fonts.ui(22.0),
                TextColor(Color::NONE),
                Node {
                    margin: UiRect::top(vh(1)),
                    ..default()
                },
            ));
        });
    commands.insert_resource(game_fonts);
}

/// How the screen looks `t` seconds after the death: the mark's colour, the caption's colour
/// and the black behind them.
#[derive(Debug, PartialEq)]
struct Look {
    mark: Color,
    caption: Color,
    shade: f32,
}

fn lerp(a: Color, b: Color, k: f32, alpha: f32) -> Color {
    let (a, b) = (a.to_srgba(), b.to_srgba());
    let k = k.clamp(0.0, 1.0);
    let m = |x: f32, y: f32| x + (y - x) * k;
    Color::srgba(
        m(a.red, b.red),
        m(a.green, b.green),
        m(a.blue, b.blue),
        alpha,
    )
}

fn look(t: f32) -> Look {
    let alpha = (t / MARK_IN).min(1.0);
    let shade = SHADE * (t / DARKEN).min(1.0);
    let fin = ((t - (RESPAWN_DELAY - FINAL_FADE)) / FINAL_FADE).clamp(0.0, 1.0);
    Look {
        mark: lerp(Color::WHITE, RED, t / MARK_RED, alpha),
        caption: lerp(
            Color::WHITE,
            Color::srgb(0.85, 0.82, 0.76),
            t / MARK_RED,
            alpha * 0.75,
        ),
        shade: shade + (1.0 - shade) * fin,
    }
}

/// Where the screen is: since when Wolf has been dead, or since when he has been back.
#[derive(Default)]
enum Phase {
    #[default]
    Alive,
    Dead(f32),
    Back(f32),
}

#[allow(clippy::type_complexity)]
fn update(
    time: Res<Time>,
    gauges: Res<Gauges>,
    mut phase: Local<Phase>,
    mut root: Query<(&mut Visibility, &mut BackgroundColor), With<DeathScreen>>,
    mut marks: Query<(Option<&mut ImageNode>, Option<&mut TextColor>), With<Mark>>,
    mut caption: Query<&mut TextColor, (With<Caption>, Without<Mark>)>,
) {
    let Ok((mut visibility, mut background)) = root.single_mut() else {
        return;
    };
    let now = time.elapsed_secs();
    // The bars start empty (max 0) before the first simulation step.
    let dead = gauges.wolf_max_hp > 0 && gauges.hp[0] <= 0.0;
    *phase = match (&*phase, dead) {
        (Phase::Dead(at), true) => Phase::Dead(*at),
        (_, true) => Phase::Dead(now),
        (Phase::Dead(_), false) => Phase::Back(now),
        (Phase::Back(at), false) if now - at < LIFT => Phase::Back(*at),
        (_, false) => Phase::Alive,
    };
    let look = match *phase {
        Phase::Alive => {
            *visibility = Visibility::Hidden;
            return;
        }
        Phase::Dead(at) => look(now - at),
        Phase::Back(at) => Look {
            mark: Color::NONE,
            caption: Color::NONE,
            shade: 1.0 - (now - at) / LIFT,
        },
    };
    *visibility = Visibility::Inherited;
    background.0 = Color::srgba(0.0, 0.0, 0.0, look.shade);
    for (image, text) in &mut marks {
        if let Some(mut image) = image {
            image.color = look.mark;
        }
        if let Some(mut text) = text {
            text.0 = look.mark;
        }
    }
    for mut text in &mut caption {
        text.0 = look.caption;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mark_turns_red_and_the_screen_is_black_at_the_restart() {
        let start = look(0.0);
        assert_eq!(start.mark.alpha(), 0.0);
        assert_eq!(start.shade, 0.0);
        let shown = look(MARK_RED);
        let (mark, red) = (shown.mark.to_srgba(), RED.to_srgba());
        assert!(
            (mark.green - red.green).abs() < 1e-4 && mark.alpha == 1.0,
            "{mark:?}"
        );
        assert!(shown.shade > SHADE && shown.shade < 1.0, "{shown:?}");
        assert_eq!(look(RESPAWN_DELAY).shade, 1.0);

        // Dying shows the screen; coming back lifts the black, then hides it.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(Gauges {
                hp: [0.0, 1.0],
                wolf_max_hp: 100,
                ..default()
            })
            .add_systems(Update, update);
        let screen = app
            .world_mut()
            .spawn((
                DeathScreen,
                BackgroundColor(Color::NONE),
                Visibility::Hidden,
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<Visibility>(screen),
            Some(&Visibility::Inherited)
        );
        app.world_mut().resource_mut::<Gauges>().hp[0] = 1.0;
        app.update();
        assert_eq!(
            app.world().get::<Visibility>(screen),
            Some(&Visibility::Inherited)
        );
        // Virtual time steps at most 0.25 s a frame.
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(0.2),
        ));
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            app.world().get::<Visibility>(screen),
            Some(&Visibility::Hidden)
        );
    }
}

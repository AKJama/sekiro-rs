//! The "SCULPTOR'S IDOL FOUND" banner (#152): the first kneel at an idol lights it
//! (`Duel::take_idol_found`, its BonfireWarpParam `eventflagId`), and the game's text effect
//! plays once over the screen: the screen darkens, the brush mark 鬼仏見出 (`MENU_Onihotoke`,
//! menu atlas SB_TextEffect) and the caption (イベントテキスト 10010901) fade in gold, a soft
//! glow (`MENU_Onihotoke_02`) swells over the mark, and all of it fades out.
//! The idol menu waits for it to end (`Playing`).
//!
//! Everything here is the game's own movie: the `BonfireLit` clip of the text effects in
//! `menu/01_000_FE.gfx` (a Scaleform movie at 60 frames a second, played from its `Normal`
//! label). Its keyframes below are that clip's frame numbers and the alphas and scales of its
//! layers, read from its place-object tags (ours, from the uncompressed movie); its positions
//! are pixels of the 1920 x 1080 stage, with the hi-res atlas art drawn at half size.
//! gap: the movie's blend modes are not read (all layers blend normally here), and the
//! caption's font is the menu font, not the movie's MenuFont_03.
use bevy::prelude::*;
use bevy::text::LetterSpacing;
use bevy::window::PrimaryWindow;

use super::Sim;
use crate::game_ui::{Covers, GameFonts, MenuArt};

/// The movie's frame rate, and the frame its `Normal` label (where playback starts) is on.
const FPS: f32 = 60.0;
const START: f32 = 1.0;
/// (movie frame, value) keyframes, straight lines between them.
type Keys = &'static [(f32, f32)];
/// The black over the screen, the mark, the caption and the glow's alpha.
const SHADE: Keys = &[(69.0, 0.0), (101.0, 0.5), (174.0, 0.5), (196.0, 0.0)];
const MARK: Keys = &[
    (81.0, 0.0),
    (85.0, 0.36),
    (90.0, 0.7),
    (95.0, 0.91),
    (100.0, 1.0),
    (174.0, 1.0),
    (175.0, 0.91),
    (180.0, 0.53),
    (185.0, 0.25),
    (190.0, 0.074),
    (195.0, 0.0),
];
const CAPTION: Keys = &[(81.0, 0.0), (101.0, 1.0), (174.0, 1.0), (196.0, 0.0)];
const GLOW: Keys = &[
    (85.0, 0.0),
    (86.0, 0.04),
    (90.0, 0.18),
    (95.0, 0.3),
    (100.0, 0.375),
    (127.0, 0.45),
    (174.0, 0.3),
    (185.0, 0.227),
    (193.0, 0.078),
    (197.0, 0.0),
];
/// The glow's width and height over its first size.
const GLOW_WIDTH: Keys = &[
    (85.0, 1.0),
    (90.0, 1.136),
    (95.0, 1.218),
    (100.0, 1.292),
    (105.0, 1.312),
];
const GLOW_HEIGHT: Keys = &[(85.0, 1.0), (105.0, 1.075), (174.0, 1.047), (194.0, 1.068)];
/// The clip's last frame.
const END: f32 = 197.0;
/// Stage pixels: the mark's size and top, the glow's first size, the height both are centred
/// on, and the caption's top, size and letter spacing.
const MARK_SIZE: Vec2 = Vec2::new(117.0, 538.0);
const MARK_TOP: f32 = 147.0;
const GLOW_SIZE: Vec2 = Vec2::new(130.0, 552.0);
const CENTRE_Y: f32 = MARK_TOP + MARK_SIZE.y / 2.0;
const CAPTION_TOP: f32 = 746.0;
const CAPTION_PX: f32 = 33.0;
const CAPTION_SPACING: f32 = 14.0;
/// The mark's and the glow's tint, and the caption's colour.
const GOLD: Color = Color::srgb(220.0 / 255.0, 188.0 / 255.0, 131.0 / 255.0);
const CAPTION_COLOR: Color = Color::srgb(226.0 / 255.0, 201.0 / 255.0, 154.0 / 255.0);
/// イベントテキスト: "SCULPTOR'S IDOL FOUND".
const CAPTION_TEXT: i32 = 10010901;

/// When the banner started, while it plays; the idol menu waits for it.
#[derive(Resource, Default)]
pub(super) struct Playing(Option<f32>);

impl Playing {
    pub(super) fn on(&self) -> bool {
        self.0.is_some()
    }
}

#[derive(Component)]
struct Banner;
#[derive(Component)]
struct Mark;
#[derive(Component)]
struct Glow;
#[derive(Component)]
struct Caption;

pub(super) fn install(app: &mut App) {
    app.init_resource::<Playing>()
        .add_systems(Startup, spawn)
        .add_systems(Update, update.run_if(crate::menu::active));
}

/// `px` pixels of the 1920 x 1080 stage, scaled with the window's height.
fn stage(px: f32) -> Val {
    vh(px / 10.8)
}

fn spawn(mut commands: Commands, fonts: Res<GameFonts>, art: Res<MenuArt>) {
    let cache = sekiro_sim::cache::root();
    let caption = crate::game_ui::menu_texts(&cache, "イベントテキスト")
        .remove(&CAPTION_TEXT)
        .unwrap_or_else(|| "SCULPTOR'S IDOL FOUND".into());
    let image = |name: &str| {
        art.sprite("SB_TextEffect", name)
            .map(|(image, _)| image.with_color(Color::NONE))
    };
    // A covering screen of its own, so it hides the HUD as the idol menu does.
    commands
        .spawn((
            Banner,
            Covers,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                ..default()
            },
            BackgroundColor(Color::NONE),
            GlobalZIndex(120),
            Visibility::Hidden,
        ))
        .with_children(|c| {
            if let Some(image) = image("MENU_Onihotoke") {
                c.spawn((
                    Mark,
                    image,
                    Node {
                        position_type: PositionType::Absolute,
                        left: percent(50),
                        margin: UiRect::left(stage(-MARK_SIZE.x / 2.0)),
                        top: stage(MARK_TOP),
                        width: stage(MARK_SIZE.x),
                        height: stage(MARK_SIZE.y),
                        ..default()
                    },
                ));
            }
            if let Some(image) = image("MENU_Onihotoke_02") {
                c.spawn((
                    Glow,
                    image,
                    Node {
                        position_type: PositionType::Absolute,
                        left: percent(50),
                        ..default()
                    },
                ));
            }
            c.spawn(Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                top: stage(CAPTION_TOP),
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_child((
                Caption,
                Text::new(caption),
                fonts.stage(CAPTION_PX),
                TextColor(Color::NONE),
                TextLayout::justify(Justify::Center),
            ));
        });
}

/// How the banner looks: its layers' alphas and the glow's scale.
#[derive(Debug, PartialEq)]
struct Look {
    shade: f32,
    mark: f32,
    caption: f32,
    glow: f32,
    glow_scale: Vec2,
}

/// `keys` at `frame`, held before the first and after the last.
fn at(keys: Keys, frame: f32) -> f32 {
    let i = keys.partition_point(|(f, _)| *f <= frame);
    match (keys.get(i.wrapping_sub(1)), keys.get(i)) {
        (Some(&(f0, v0)), Some(&(f1, v1))) => v0 + (v1 - v0) * (frame - f0) / (f1 - f0),
        (Some(&(_, v)), None) | (None, Some(&(_, v))) => v,
        (None, None) => 0.0,
    }
}

/// The banner `t` seconds after the idol was lit, `None` once the clip is over.
fn look(t: f32) -> Option<Look> {
    let frame = START + t * FPS;
    (frame <= END).then(|| Look {
        shade: at(SHADE, frame),
        mark: at(MARK, frame),
        caption: at(CAPTION, frame),
        glow: at(GLOW, frame),
        glow_scale: Vec2::new(at(GLOW_WIDTH, frame), at(GLOW_HEIGHT, frame)),
    })
}

#[allow(clippy::type_complexity)]
fn update(
    time: Res<Time>,
    mut sim: NonSendMut<Sim>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut playing: ResMut<Playing>,
    mut root: Query<(&mut Visibility, &mut BackgroundColor), With<Banner>>,
    mut images: Query<(&mut ImageNode, &mut Node, Has<Glow>), Or<(With<Mark>, With<Glow>)>>,
    mut caption: Query<(&mut TextColor, &mut LetterSpacing), With<Caption>>,
) {
    let now = time.elapsed_secs();
    if sim.duel.take_idol_found().is_some() {
        playing.0 = Some(now);
    }
    let Ok((mut visibility, mut background)) = root.single_mut() else {
        return;
    };
    let Some(look) = playing.0.and_then(|s| look(now - s)) else {
        if playing.0.is_some() {
            playing.0 = None;
        }
        visibility.set_if_neq(Visibility::Hidden);
        return;
    };
    visibility.set_if_neq(Visibility::Inherited);
    background.set_if_neq(BackgroundColor(Color::BLACK.with_alpha(look.shade)));
    for (mut image, mut node, glow) in &mut images {
        let alpha = if glow { look.glow } else { look.mark };
        image.color = GOLD.with_alpha(alpha);
        if glow {
            // Centred on the mark as it swells.
            let size = GLOW_SIZE * look.glow_scale;
            node.width = stage(size.x);
            node.height = stage(size.y);
            node.margin = UiRect::left(stage(-size.x / 2.0));
            node.top = stage(CENTRE_Y - size.y / 2.0);
        }
    }
    // The movie's letter spacing is stage pixels; ours is logical pixels of the window.
    let scale = window.single().map_or(1.0, |w| w.height() / 1080.0);
    for (mut color, mut spacing) in &mut caption {
        color.set_if_neq(TextColor(CAPTION_COLOR.with_alpha(look.caption)));
        spacing.set_if_neq(LetterSpacing::Px(CAPTION_SPACING * scale));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_banner_plays_the_clip_once_and_ends() {
        // Nothing shows before the screen darkens, the mark is full at its hold, and the clip
        // ends at its last frame.
        let first = look(0.0).unwrap();
        assert_eq!(
            (first.shade, first.mark, first.caption, first.glow),
            (0.0, 0.0, 0.0, 0.0)
        );
        let held = look((140.0 - START) / FPS).unwrap();
        assert_eq!((held.shade, held.mark, held.caption), (0.5, 1.0, 1.0));
        assert!(held.glow > 0.3 && held.glow_scale.x > 1.3, "{held:?}");
        let last = look((END - START) / FPS).unwrap();
        assert_eq!(
            (last.shade, last.mark, last.caption, last.glow),
            (0.0, 0.0, 0.0, 0.0)
        );
        assert_eq!(look((END - START) / FPS + 0.1), None);
    }
}

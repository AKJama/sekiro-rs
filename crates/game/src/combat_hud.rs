//! HP and posture bars laid out like Sekiro's HUD. Wolf's vitality sits at the bottom left, its
//! length following his maximum vitality, with a light damage trail behind the fill; his posture
//! bar sits at the bottom centre and shows only while it has damage. A plain enemy's vitality
//! and posture float over its Head bone, with its deathblow dots over the bar's left end; a boss
//! bar (one the map events put up) has its vitality at the top left with the deathblow orbs over
//! it and the name under it, and its posture at the top centre. Posture bars grow from the centre
//! outward in the game's state colours, with an arrow at each end while they have damage. The
//! item cluster (quick item and prosthetic tool) sits at the bottom right. Above Wolf's
//! vitality, a purple bar shows poison build-up and a line names the status effects on him
//! (poisoned, recovery sealed, ...) with the seconds left. A red 危 marks a perilous attack's
//! warning on Wolf (see [`update_perilous`]).
//!
//! Layout, colours and timings adapted from Fireguy32's RustSekiro fork (MIT), commit d11ef65,
//! which measured them from captures of the game's HUD. The game's own HUD sprites (SB_FE,
//! SB_Boss) are not extracted, so plain framed nodes stand in for them.

use bevy::prelude::*;

/// What the bars show, filled by the simulation's owner every frame.
#[derive(Resource, Default, Clone)]
pub struct Gauges {
    /// `(current, max)` HP, Wolf then the enemy.
    pub hp: [(i32, i32); 2],
    /// `(remaining, max)` posture.
    pub posture: [(i32, i32); 2],
    /// Where the enemy's head is (its skeleton's Head bone), for its floating bars and the
    /// deathblow mark.
    pub enemy_head: Option<Vec3>,
    /// The enemy's bars are shown: it is alive and, in a boss fight run by the map events,
    /// the events show its health bar (the Owl's is hidden during his heat-up).
    pub enemy_alive: bool,
    /// A map-script boss bar stays on screen rather than following the character's head.
    pub boss_bar: bool,
    pub deathblow: bool,
    /// The enemy's name, under the boss bar.
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
    /// The item cluster's buttons held this frame: quick item, then prosthetic tool.
    pub held: [bool; 2],
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum Fill {
    Hp(usize),
    /// The light damage trail behind a vitality fill.
    Trail(usize),
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

/// Wolf's vitality frame, whose length follows his maximum vitality.
#[derive(Component)]
pub struct HpFrame;

/// A posture bar's row, shown while the posture has damage (and [`SP_MIN_DISP_TIME`] after).
#[derive(Component)]
pub struct PostureRow(usize);

/// A posture bar's end arrow, shown while its posture has damage.
#[derive(Component)]
pub struct PostureArrow(usize);

/// The enemy's floating group.
#[derive(Component)]
pub struct EnemyPanel;

/// A deathblow dot over the floating bars' left end (index).
#[derive(Component)]
pub struct EnemyDot(i32);

/// The boss bars' group.
#[derive(Component)]
pub struct BossPanel;

/// A deathblow orb over the boss's vitality (index).
#[derive(Component)]
pub struct BossOrb(i32);

/// The boss's name under its vitality.
#[derive(Component)]
pub struct BossName;

/// The deathblow mark on the enemy.
#[derive(Component)]
pub struct DeathblowMark;

/// An item cluster slot: 0 the quick item, 1 the prosthetic tool.
#[derive(Component)]
pub struct ItemSlot(usize);

/// Frame colours: dark lacquer with a thin gold edge, like the game's bars.
const FRAME: Color = Color::srgba(0.05, 0.04, 0.03, 0.75);
const EDGE: Color = Color::srgba(0.78, 0.66, 0.42, 0.55);
/// The vitality fill: the game's muted orange-red (about 180, 70, 48 in captures).
const VITALITY: Color = Color::srgb(180.0 / 255.0, 70.0 / 255.0, 48.0 / 255.0);
/// The damage trail's light, desaturated red (about 165, 130, 120 in captures).
const TRAIL: Color = Color::srgb(165.0 / 255.0, 130.0 / 255.0, 120.0 / 255.0);
const POISON_WIDTH: f32 = 160.0;

/// Wolf's vitality bar width (percent of the screen width) at [`HP_BAR_BASE`] maximum
/// vitality (level 1); it grows in proportion, as the game's does with each Vitality level.
const HP_BAR_VW: f32 = 9.5;
const HP_BAR_BASE: f32 = 320.0;
/// The widest the vitality bar gets (ours: keeps a modded maximum on screen).
const HP_BAR_MAX_VW: f32 = 60.0;
/// The over-head bars' width (percent of the screen width).
const OVERHEAD_W: f32 = 8.5;
/// How far over the Head bone the floating bars stand (m).
const OVERHEAD_ABOVE_HEAD: f32 = 0.35;
/// MenuParam SpMinimumDispTime: the least time a posture gauge stays up once its damage is gone.
const SP_MIN_DISP_TIME: f32 = 0.5;
/// The damage trail's hold from the first hit (later hits do not restart it) and its collapse
/// time.
const TRAIL_HOLD: f32 = 0.5;
const TRAIL_COLLAPSE: f32 = 0.05;
/// The posture fill per state (normal, about to break, breakable, broken), from the HUD movie's
/// colour transforms on the posture bar.
const POSTURE_FILL: [[u8; 3]; 4] = [[142, 123, 60], [154, 116, 0], [168, 107, 0], [182, 96, 0]];

/// Wolf's vitality bar width, percent of the screen width, for `max_hp`.
pub fn hp_bar_vw(max_hp: i32) -> f32 {
    (HP_BAR_VW * max_hp.max(1) as f32 / HP_BAR_BASE).min(HP_BAR_MAX_VW)
}

/// The posture gauge's state for damage fraction `f`: 0 normal, 1 about to break (at most 30 %
/// of the posture left), 2 breakable (full), 3 broken.
fn posture_state(f: f32, broken: bool) -> usize {
    if broken {
        3
    } else if f >= 1.0 {
        2
    } else if f >= 0.7 {
        1
    } else {
        0
    }
}

fn posture_color(f: f32, broken: bool) -> Color {
    let [r, g, b] = POSTURE_FILL[posture_state(f, broken)];
    Color::srgb_u8(r, g, b)
}

/// Steps a damage trail `(shown, held)` toward the bar's fill `f`: it holds [`TRAIL_HOLD`]
/// from the first hit, then collapses onto the fill over [`TRAIL_COLLAPSE`].
fn step_trail(trail: &mut (f32, f32), f: f32, dt: f32) {
    let (shown, hold) = trail;
    if f < *shown {
        *hold += dt;
        if *hold > TRAIL_HOLD {
            let left = (TRAIL_HOLD + TRAIL_COLLAPSE - *hold + dt).max(dt);
            *shown = (*shown - (*shown - f) * (dt / left).min(1.0)).max(f);
        }
    } else {
        *shown = f;
        *hold = 0.0;
    }
}

/// Steps a posture gauge's display hold: full while it has damage, then counting down.
fn step_hold(hold: &mut f32, damaged: bool, dt: f32) -> bool {
    *hold = if damaged {
        SP_MIN_DISP_TIME
    } else {
        (*hold - dt).max(0.0)
    };
    *hold > 0.0
}

/// A framed bar of `width` x `height`: the fill (and an optional damage trail behind it) inside
/// a thin gold frame; `posture` fills from the middle out and gets an arrow at each end.
fn bar(
    parent: &mut ChildSpawnerCommands,
    fill: Fill,
    trail: Option<Fill>,
    (width, height): (Val, Val),
    posture: Option<usize>,
    frame: impl Bundle,
) {
    parent
        .spawn((
            frame,
            Node {
                width,
                height,
                border: UiRect::all(px(1)),
                justify_content: if posture.is_some() {
                    JustifyContent::Center
                } else {
                    JustifyContent::FlexStart
                },
                ..default()
            },
            BackgroundColor(FRAME),
            BorderColor::all(EDGE),
        ))
        .with_children(|b| {
            if let Some(t) = trail {
                b.spawn((
                    t,
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        width: percent(100),
                        height: percent(100),
                        ..default()
                    },
                    BackgroundColor(TRAIL),
                ));
            }
            b.spawn((
                fill,
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ));
            // The game's "<" ">" arrows at the posture bar's outer ends; a gold diamond stands in
            // for the arrow sprite (MENU_Taikan_Arrow, not extracted).
            if let Some(i) = posture {
                for right in [false, true] {
                    b.spawn((
                        PostureArrow(i),
                        Node {
                            position_type: PositionType::Absolute,
                            left: if right { Val::Auto } else { percent(-2.5) },
                            right: if right { percent(-2.5) } else { Val::Auto },
                            top: percent(-10),
                            height: percent(120),
                            aspect_ratio: Some(1.0),
                            ..default()
                        },
                        UiTransform::from_rotation(Rot2::degrees(45.0)),
                        BackgroundColor(Color::srgb(0.85, 0.72, 0.45)),
                        Visibility::Hidden,
                    ));
                }
            }
        });
}

/// A round deathblow dot `size` across.
fn dot(size: Val) -> impl Bundle {
    (
        Node {
            width: size,
            height: size,
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::MAX,
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgb(0.85, 0.06, 0.05)),
        BorderColor::all(Color::srgb(0.25, 0.02, 0.0)),
    )
}

fn text(s: &str, size: f32) -> impl Bundle {
    (
        Text::new(s),
        TextFont {
            font_size: bevy::text::FontSize::Px(size),
            ..default()
        },
        TextColor(Color::srgba(0.92, 0.88, 0.8, 1.0)),
        TextShadow {
            offset: Vec2::new(1.5, 1.5),
            color: Color::BLACK,
        },
    )
}

/// Spawns the bars (call once at startup).
pub fn spawn(commands: &mut Commands) {
    // Wolf: vitality bottom left, status and poison above it.
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: percent(5.7),
            bottom: percent(7.8),
            flex_direction: FlexDirection::Column,
            row_gap: px(6),
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
                .with_children(|r| {
                    bar(r, Fill::Poison, None, (px(POISON_WIDTH), px(6)), None, ());
                });
            bar(
                p,
                Fill::Hp(0),
                Some(Fill::Trail(0)),
                (Val::Vw(HP_BAR_VW), Val::Vh(1.25)),
                None,
                HpFrame,
            );
        });
    // Wolf: posture bottom centre.
    commands
        .spawn((
            PostureRow(0),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                bottom: percent(12.6),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|p| {
            bar(
                p,
                Fill::Posture(0),
                None,
                (Val::Vw(27.0), Val::Vh(1.3)),
                Some(0),
                (),
            );
        });
    // A plain enemy: deathblow dots, vitality over posture, above its head.
    commands
        .spawn((
            EnemyPanel,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Vw(OVERHEAD_W),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Vh(0.3),
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|p| {
            p.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Vw(0.15),
                margin: UiRect {
                    left: Val::Vw(-0.2),
                    bottom: Val::Vh(0.1),
                    ..default()
                },
                ..default()
            })
            .with_children(|r| {
                for i in 0..3 {
                    r.spawn((EnemyDot(i), dot(Val::Vw(1.15))));
                }
            });
            bar(
                p,
                Fill::Hp(1),
                Some(Fill::Trail(1)),
                (percent(100), Val::Vh(0.75)),
                None,
                (),
            );
            p.spawn((
                PostureRow(1),
                Node {
                    width: percent(100),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Visibility::Hidden,
            ))
            .with_children(|r| {
                bar(
                    r,
                    Fill::Posture(1),
                    None,
                    (percent(100), Val::Vh(0.6)),
                    Some(1),
                    (),
                );
            });
        });
    // A boss: vitality top left with the deathblow orbs over it and the name under it, posture
    // top centre with its centre mark.
    commands
        .spawn((
            BossPanel,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|c| {
            c.spawn(Node {
                position_type: PositionType::Absolute,
                left: percent(5.5),
                top: percent(4.6),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Vw(0.3),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|r| {
                for i in 0..4 {
                    r.spawn((BossOrb(i), dot(Val::Vw(1.4))));
                }
            });
            c.spawn(Node {
                position_type: PositionType::Absolute,
                left: percent(5.7),
                top: percent(8.3),
                ..default()
            })
            .with_children(|r| {
                bar(
                    r,
                    Fill::Hp(1),
                    Some(Fill::Trail(1)),
                    (Val::Vw(21.6), Val::Vh(1.25)),
                    None,
                    (),
                );
            });
            c.spawn((
                BossName,
                text("", 15.0),
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(6.3),
                    top: percent(11.0),
                    ..default()
                },
            ));
            c.spawn((
                PostureRow(1),
                Node {
                    position_type: PositionType::Absolute,
                    top: percent(5.4),
                    width: percent(100),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Visibility::Hidden,
            ))
            .with_children(|r| {
                bar(
                    r,
                    Fill::Posture(1),
                    None,
                    (Val::Vw(38.4), Val::Vh(1.8)),
                    Some(1),
                    (),
                );
            });
            // The centre mark (MENU_Sp_Center, not extracted: a gold tick).
            c.spawn((
                PostureRow(1),
                Node {
                    position_type: PositionType::Absolute,
                    top: percent(5.0),
                    width: percent(100),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Visibility::Hidden,
            ))
            .with_children(|r| {
                r.spawn((
                    Node {
                        width: Val::Vh(0.5),
                        height: Val::Vh(2.6),
                        ..default()
                    },
                    BackgroundColor(EDGE),
                ));
            });
        });
    // The deathblow mark on the enemy: a red diamond with a dark rim (the game's MENU_ninsatu
    // sprite is not extracted).
    commands.spawn((
        DeathblowMark,
        Node {
            position_type: PositionType::Absolute,
            width: px(16),
            height: px(16),
            border: UiRect::all(px(2)),
            ..default()
        },
        UiTransform::from_rotation(Rot2::degrees(45.0)),
        BackgroundColor(Color::srgb(0.85, 0.05, 0.05)),
        BorderColor::all(Color::srgb(0.15, 0.0, 0.0)),
        Visibility::Hidden,
    ));
    // The item cluster, bottom right: the quick item's box, then the prosthetic tool's bigger
    // one (percent of the screen). No items are equipped in our game yet, so each box shows only
    // its button and lights its edge while it is held.
    for (slot, (left, top, w, h, key)) in
        [(79.1, 87.2, 4.2, 7.3, "R"), (84.2, 83.1, 6.3, 11.3, "Ctrl")]
            .into_iter()
            .enumerate()
    {
        commands
            .spawn((
                ItemSlot(slot),
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(left),
                    top: percent(top),
                    width: percent(w),
                    height: percent(h),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(4)),
                    justify_content: JustifyContent::FlexEnd,
                    align_items: AlignItems::FlexEnd,
                    padding: UiRect::all(px(4)),
                    ..default()
                },
                BackgroundColor(FRAME),
                BorderColor::all(EDGE),
            ))
            .with_children(|b| {
                b.spawn(text(key, 13.0));
            });
    }
}

/// Where world point `at` is on screen, `None` when it is behind the camera.
fn project(camera: &Query<(&Camera, &GlobalTransform), With<Camera3d>>, at: Vec3) -> Option<Vec2> {
    let (cam, tf) = camera.single().ok()?;
    cam.world_to_viewport(tf, at).ok()
}

/// Updates widths, colours, the floating panel and the boss bars.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update(
    gauges: Res<Gauges>,
    time: Res<Time>,
    (mut trails, mut holds): (Local<[(f32, f32); 2]>, Local<[f32; 2]>),
    (recording, overlay_size): (
        Option<Res<crate::brain_overlay::RecordingBrain>>,
        Query<&ComputedNode, With<crate::brain_overlay::Panel>>,
    ),
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut fills: Query<(&Fill, &mut Node, &mut BackgroundColor)>,
    mut frames: Query<&mut Node, (With<HpFrame>, Without<Fill>)>,
    mut panel: Query<
        (&mut Node, &mut Visibility),
        (With<EnemyPanel>, Without<Fill>, Without<HpFrame>),
    >,
    mut boss: Query<&mut Visibility, (With<BossPanel>, Without<EnemyPanel>)>,
    mut rows: Query<(&PostureRow, &mut Visibility), (Without<EnemyPanel>, Without<BossPanel>)>,
    mut arrows: Query<
        (&PostureArrow, &mut Visibility),
        (Without<EnemyPanel>, Without<BossPanel>, Without<PostureRow>),
    >,
    (mut dots, mut orbs): (
        Query<(&EnemyDot, &mut Node), (Without<Fill>, Without<EnemyPanel>, Without<HpFrame>)>,
        Query<
            (&BossOrb, &mut Node),
            (
                Without<Fill>,
                Without<EnemyPanel>,
                Without<HpFrame>,
                Without<EnemyDot>,
            ),
        >,
    ),
    mut name: Query<&mut Text, With<BossName>>,
    mut mark: Query<
        (&mut Node, &mut Visibility),
        (
            With<DeathblowMark>,
            Without<Fill>,
            Without<EnemyPanel>,
            Without<HpFrame>,
            Without<EnemyDot>,
            Without<BossOrb>,
            Without<BossPanel>,
            Without<PostureRow>,
            Without<PostureArrow>,
        ),
    >,
    mut slots: Query<(&ItemSlot, &mut BorderColor)>,
) {
    let dt = time.delta_secs();
    let hp_fraction = |i: usize| {
        let (hp, max) = gauges.hp[i];
        (hp.max(0) as f32 / max.max(1) as f32).clamp(0.0, 1.0)
    };
    let posture_fraction = |i: usize| {
        let (left, max) = gauges.posture[i];
        ((max - left) as f32 / max.max(1) as f32).clamp(0.0, 1.0)
    };
    for (i, trail) in trails.iter_mut().enumerate() {
        step_trail(trail, hp_fraction(i), dt);
    }
    let shown = [0, 1].map(|i| step_hold(&mut holds[i], posture_fraction(i) > 0.005, dt));
    for (fill, mut node, mut color) in &mut fills {
        let (fraction, c) = match *fill {
            Fill::Hp(i) => (hp_fraction(i), VITALITY),
            Fill::Trail(i) => (trails[i].0, TRAIL),
            Fill::Posture(i) => {
                let f = posture_fraction(i);
                (f, posture_color(f, gauges.posture[i].0 < 1))
            }
            Fill::Poison => (gauges.poison, Color::srgb(0.55, 0.2, 0.75)),
        };
        node.width = percent(fraction * 100.0);
        color.0 = c;
    }
    for mut node in &mut frames {
        let w = Val::Vw(hp_bar_vw(gauges.hp[0].1));
        if node.width != w {
            node.width = w;
        }
    }
    for (row, mut v) in &mut rows {
        *v = if shown[row.0] && (row.0 == 0 || gauges.enemy_alive) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (arrow, mut v) in &mut arrows {
        *v = if posture_fraction(arrow.0) > 0.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    let marks = gauges.enemy_marks;
    for (d, mut node) in &mut dots {
        node.display = if d.0 < marks {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (o, mut node) in &mut orbs {
        node.display = if o.0 < marks {
            Display::Flex
        } else {
            Display::None
        };
    }
    if let Ok(mut t) = name.single_mut()
        && t.0 != gauges.enemy_name
    {
        t.0 = gauges.enemy_name.to_string();
    }
    for (slot, mut border) in &mut slots {
        *border = BorderColor::all(if gauges.held[slot.0] {
            Color::srgb(1.0, 0.9, 0.6)
        } else {
            EDGE
        });
    }
    if let Ok(mut v) = boss.single_mut() {
        *v = if gauges.boss_bar && gauges.enemy_alive {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    let head = gauges.enemy_head.filter(|_| gauges.enemy_alive);
    if let Ok((mut node, mut vis)) = mark.single_mut() {
        match head
            .filter(|_| gauges.deathblow)
            .and_then(|h| project(&camera, h))
        {
            Some(p) => {
                node.left = px(p.x - 8.0);
                node.top = px(p.y - 8.0);
                *vis = Visibility::Inherited;
            }
            None => *vis = Visibility::Hidden,
        }
    }
    let Ok((mut node, mut vis)) = panel.single_mut() else {
        return;
    };
    let screen = head
        .filter(|_| !gauges.boss_bar)
        .and_then(|h| project(&camera, h + Vec3::Y * OVERHEAD_ABOVE_HEAD));
    let Some(p) = screen else {
        *vis = Visibility::Hidden;
        return;
    };
    let viewport = camera
        .single()
        .ok()
        .and_then(|(cam, _)| cam.logical_viewport_size())
        .unwrap_or(Vec2::new(1920.0, 1080.0));
    let width = viewport.x * OVERHEAD_W / 100.0;
    // The bars' bottom at the point over the head.
    let top = p.y - 0.035 * viewport.y;
    let mut left = p.x - width * 0.5;
    if recording.as_ref().is_some_and(|r| r.enabled)
        && let Ok(size) = overlay_size.single()
    {
        let overlay_left = viewport.x - 24.0 - 344.0;
        let overlay_bottom = 24.0 + size.size().y * size.inverse_scale_factor;
        if left + width > overlay_left - 12.0 && p.y > 24.0 && top < overlay_bottom {
            left = overlay_left - width - 12.0;
        }
    }
    node.left = px(left);
    node.top = px(top);
    *vis = Visibility::Inherited;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vitality_bar_length_follows_max_hp() {
        // Level 1's 320 is the measured 9.5 % of the screen; Vitality 20's 1120 is 3.5 times it.
        assert!((hp_bar_vw(320) - 9.5).abs() < 1e-4);
        assert!((hp_bar_vw(1120) - 33.25).abs() < 1e-3);
        assert!((hp_bar_vw(640) - 2.0 * hp_bar_vw(320)).abs() < 1e-4);
        // A missing maximum still draws a sliver, and a huge one stays on screen.
        assert!(hp_bar_vw(0) > 0.0);
        assert_eq!(hp_bar_vw(100_000), HP_BAR_MAX_VW);
    }

    #[test]
    fn posture_states_follow_the_hud_movie_thresholds() {
        assert_eq!(posture_state(0.0, false), 0);
        assert_eq!(posture_state(0.69, false), 0);
        assert_eq!(posture_state(0.7, false), 1);
        assert_eq!(posture_state(1.0, false), 2);
        assert_eq!(posture_state(1.0, true), 3);
        assert_eq!(posture_color(0.0, false), Color::srgb_u8(142, 123, 60));
    }

    #[test]
    fn damage_trail_holds_then_collapses_onto_the_fill() {
        let dt = 1.0 / 60.0;
        let mut trail = (1.0, 0.0);
        // Held for the first half second.
        for _ in 0..29 {
            step_trail(&mut trail, 0.6, dt);
        }
        assert_eq!(trail.0, 1.0);
        // On the fill a few frames later.
        for _ in 0..6 {
            step_trail(&mut trail, 0.6, dt);
        }
        assert!((trail.0 - 0.6).abs() < 1e-5);
        // Healing snaps the trail up with the fill and resets the hold.
        step_trail(&mut trail, 0.9, dt);
        assert_eq!(trail, (0.9, 0.0));
    }

    #[test]
    fn posture_gauge_stays_up_for_the_minimum_display_time() {
        let mut hold = 0.0;
        assert!(!step_hold(&mut hold, false, 0.1));
        assert!(step_hold(&mut hold, true, 0.1));
        assert!(step_hold(&mut hold, false, 0.3));
        assert!(!step_hold(&mut hold, false, 0.3));
    }
}

//! The loading screen and the point where play starts.
//!
//! Like the game's own loading screen, an opaque screen with a progress bar covers the view and
//! the simulation does not step (so the player cannot move) until loading has finished: Wolf's
//! model is bound, every map GLB and texture has loaded and every scene has spawned
//! ([`crate::map::Pending::loaded`]), no render pipeline is still compiling, the collision
//! floor under Wolf exists, every image and spawned mesh is prepared on the GPU and the
//! drawn meshes within 60 m have their textures ([`check_render`]), and then 60 consecutive
//! frames stay within 1/30 s, so uploads and first-use work that follow loading also finish
//! behind the screen. Scripted, replayed and interactive runs all wait for the whole map. The
//! time to that point is logged as `play ready`; a machine that never settles starts after
//! 60 s with a warning instead of waiting forever.
use super::{Actor, MapCollision, Sim};
use crate::character::Rig;
use bevy::prelude::*;
use std::time::Instant;

#[derive(Resource, Default)]
pub(super) struct PlayGate {
    /// When loading finished and play may start.
    pub(super) ready_at: Option<Instant>,
    /// When everything had loaded, and the consecutive frames within budget since.
    loaded_at: Option<Instant>,
    stable_frames: u32,
    /// The last GPU readiness check ([`check_render`]); `None` before the first.
    render: Option<RenderWait>,
    /// Drawn meshes near Wolf, listed once when the upload counts first reach zero (Wolf does
    /// not move before play).
    near: Option<Vec<Entity>>,
    last_log: Option<Instant>,
}

/// What the GPU still waits for before the world is drawable.
#[derive(Clone, Copy, Debug, Default)]
struct RenderWait {
    /// A drawn mesh near Wolf still waits for its material textures.
    textures: bool,
    /// Images queued behind the upload budget.
    images: usize,
    /// Spawned mesh entities whose mesh is not yet prepared.
    meshes: usize,
}

impl RenderWait {
    fn ready(&self) -> bool {
        !self.textures && self.images == 0 && self.meshes == 0
    }
}

/// Drawn meshes within this distance of Wolf must have their textures before play starts.
const RENDER_RADIUS: f32 = 60.0;

/// A frame within budget: 30 Hz, so a vsync-limited 60 Hz frame always counts.
const STABLE_FRAME_SECONDS: f32 = 1.0 / 30.0;
/// Consecutive frames within budget that mark the end of loading.
const STABLE_FRAMES: u32 = 60;
/// A machine that never reaches the budget still starts, with a warning.
const STABLE_TIMEOUT_SECONDS: f32 = 60.0;

impl PlayGate {
    /// Whether the simulation may step now. The 60 stable frames before readiness also cover
    /// the presentation stall that once followed the map upload; a further 2.5 s settle made
    /// no difference to the first frames of play (about 80 ms either way, measured 2026-10-16).
    pub(super) fn open(&self) -> bool {
        self.ready_at.is_some()
    }

    fn render_ready(&self) -> bool {
        self.render.is_some_and(|r| r.ready())
    }
}

#[derive(Component)]
pub(super) struct LoadingScreen;

#[derive(Component)]
pub(super) struct LoadingBar;

/// Run condition: the loading screen is still up. Once play starts it is despawned and the
/// gate's systems stop running.
pub(super) fn loading(screens: Query<(), With<LoadingScreen>>) -> bool {
    !screens.is_empty()
}

pub(super) fn spawn_screen(mut commands: Commands) {
    commands
        .spawn((
            LoadingScreen,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::FlexEnd,
                padding: UiRect::all(px(48)),
                row_gap: px(10),
                ..default()
            },
            BackgroundColor(Color::BLACK),
            GlobalZIndex(100),
        ))
        .with_children(|screen| {
            screen.spawn((
                Text::new("Loading"),
                TextFont {
                    font_size: bevy::text::FontSize::Px(22.0),
                    ..default()
                },
                TextColor(Color::srgb(0.85, 0.82, 0.75)),
            ));
            screen
                .spawn((
                    Node {
                        width: px(320),
                        height: px(4),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                ))
                .with_child((
                    LoadingBar,
                    Node {
                        width: percent(0),
                        height: percent(100),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.85, 0.82, 0.75)),
                ));
        });
}

#[allow(clippy::too_many_arguments)]
pub(super) fn update(
    mut commands: Commands,
    mut gate: ResMut<PlayGate>,
    sim: NonSend<Sim>,
    time: Res<Time<Real>>,
    rigs: Query<&Rig, With<Actor>>,
    pending: Option<Res<crate::map::Pending>>,
    pipelines: Option<Res<crate::map::ready::Pipelines>>,
    collision: Option<Res<MapCollision>>,
    screens: Query<Entity, With<LoadingScreen>>,
    mut bars: Query<&mut Node, With<LoadingBar>>,
) {
    if sim.step > 0 {
        for screen in &screens {
            commands.entity(screen).despawn();
        }
        return;
    }
    let progress = if gate.ready_at.is_none() {
        let rigs_bound = !rigs.is_empty() && rigs.iter().all(|r| r.bound);
        let map = pending.as_ref().is_none_or(|p| p.loaded());
        let compiled = pipelines.as_ref().is_none_or(|p| {
            let status = p.status();
            status.pending == 0 && status.ready > 0
        });
        let p = sim.duel.player.body.position;
        let floor = collision
            .as_ref()
            .is_none_or(|c| c.0.floor(p[0], p[2], p[1] + 1.0).is_some());
        // GPU uploads trail the CPU load; then uploads, scene spawning and shader compiles show
        // up as slow frames, so the area is ready once the world is drawable and frames stay
        // within budget.
        if rigs_bound && map && compiled && floor {
            let since = *gate.loaded_at.get_or_insert_with(Instant::now);
            if gate.render_ready() && time.delta_secs() <= STABLE_FRAME_SECONDS {
                gate.stable_frames += 1;
            } else {
                gate.stable_frames = 0;
            }
            let settled = gate.stable_frames >= STABLE_FRAMES;
            if settled || since.elapsed().as_secs_f32() >= STABLE_TIMEOUT_SECONDS {
                gate.ready_at = Some(Instant::now());
                let launch = crate::since_start();
                if settled {
                    info!(
                        "play ready: map loaded and {STABLE_FRAMES} frames within budget, {launch:.1} s after launch ({:.1} s settling)",
                        since.elapsed().as_secs_f32()
                    );
                } else {
                    warn!(
                        "play ready: not settled within {STABLE_TIMEOUT_SECONDS} s of loading ({:?}), starting anyway {launch:.1} s after launch",
                        gate.render
                    );
                }
            }
        }
        pending.as_ref().map_or(1.0, |p| p.progress()) * 0.9
            + 0.04 * f32::from(u8::from(rigs_bound))
            + 0.03 * f32::from(u8::from(compiled))
            + 0.03 * (gate.stable_frames as f32 / STABLE_FRAMES as f32).min(1.0)
    } else {
        1.0
    };
    let width = percent(progress * 100.0);
    for mut bar in &mut bars {
        if bar.width != width {
            bar.width = width;
        }
    }
}

/// Once everything has loaded, checks that the world is drawable: no image upload is still
/// queued behind the upload budget, every spawned mesh entity's mesh is prepared on the GPU
/// ([`crate::map::ready::LateMeshes`], which also gets late entities drawn), and the
/// materials of the drawn meshes within [`RENDER_RADIUS`] of Wolf have their textures (the
/// capture readiness check). The nearby list is made once, when the upload counts first reach
/// zero.
pub(super) fn check_render(world: &mut World) {
    if !world.contains_resource::<crate::map::ready::LateMeshes>() {
        world.resource_mut::<PlayGate>().render = Some(RenderWait::default());
        return;
    }
    let gate = world.resource::<PlayGate>();
    if gate.ready_at.is_some() || gate.loaded_at.is_none() {
        return;
    }
    let (images, meshes) = crate::map::ready::pending_uploads(world);
    let textures = images == 0 && meshes == 0 && {
        let near = match world.resource_mut::<PlayGate>().near.take() {
            Some(near) => near,
            None => near_meshes(world),
        };
        let pending = crate::map::ready::textures_pending(world, &near);
        world.resource_mut::<PlayGate>().near = Some(near);
        pending
    };
    let wait = RenderWait {
        textures,
        images,
        meshes,
    };
    let mut gate = world.resource_mut::<PlayGate>();
    gate.render = Some(wait);
    if gate
        .last_log
        .is_none_or(|t| t.elapsed().as_secs_f32() >= 1.0)
    {
        gate.last_log = Some(Instant::now());
        info!("loading: waiting for the GPU: {wait:?}");
    }
}

/// The drawn meshes whose bounds come within [`RENDER_RADIUS`] of Wolf.
fn near_meshes(world: &mut World) -> Vec<Entity> {
    let wolf = Vec3::from_array(world.non_send::<Sim>().duel.player.body.position);
    let mut query = world.query_filtered::<(
        Entity,
        &GlobalTransform,
        &InheritedVisibility,
        &bevy::camera::primitives::Aabb,
    ), With<Mesh3d>>();
    query
        .iter(world)
        .filter(|(_, transform, visible, aabb)| {
            let (lo, hi) = crate::map::world_bounds(transform, aabb);
            visible.get() && wolf.distance(wolf.clamp(lo, hi)) <= RENDER_RADIUS
        })
        .map(|(entity, ..)| entity)
        .collect()
}

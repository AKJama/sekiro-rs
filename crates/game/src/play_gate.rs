//! The loading screen and the point where play starts.
//!
//! Like the game's own loading screen, an opaque screen with a progress bar covers the view and
//! the simulation does not step (so the player cannot move) until loading has finished: Wolf's
//! model is bound, every map GLB and texture has loaded and every scene has spawned
//! ([`crate::map::Pending::loaded`]), no render pipeline is still compiling, the collision
//! floor under Wolf exists, the drawn map meshes within 60 m have their mesh buffers and
//! textures prepared on the GPU ([`check_render`]), and then 60 consecutive frames stay
//! within 1/30 s, so uploads and first-use work that follow loading also finish behind the
//! screen. The time to that point is logged as `play ready`; a machine that never settles starts after 60 s with
//! a warning instead of waiting forever.
use super::{Actor, MapCollision, Sim};
use crate::character::Rig;
use bevy::prelude::*;
use std::time::Instant;

#[derive(Resource, Default)]
pub(super) struct PlayGate {
    /// When the area became ready; play starts [`super::MAP_SETTLE_SECS`] later.
    pub(super) ready_at: Option<Instant>,
    /// When everything had loaded, and the consecutive frames within budget since.
    loaded_at: Option<Instant>,
    stable_frames: u32,
    progress: f32,
    /// Every drawn map mesh near Wolf has its mesh and material textures prepared on the GPU.
    render_ready: bool,
    render_missing: (usize, bool, usize, usize),
    last_log: Option<Instant>,
    /// Mesh entities were marked changed once every upload had finished.
    refreshed: bool,
}

/// Drawn map meshes within this distance of Wolf must be renderable before play starts.
const RENDER_RADIUS: f32 = 60.0;

/// A frame within budget: 30 Hz, so a vsync-limited 60 Hz frame always counts.
const STABLE_FRAME_SECONDS: f32 = 1.0 / 30.0;
/// Consecutive frames within budget that mark the end of loading.
const STABLE_FRAMES: u32 = 60;
/// A machine that never reaches the budget still starts, with a warning.
const STABLE_TIMEOUT_SECONDS: f32 = 60.0;

impl PlayGate {
    /// Whether the simulation may step now.
    pub(super) fn open(&self, first_step: bool) -> bool {
        self.ready_at
            .is_some_and(|t| !first_step || t.elapsed().as_secs_f32() >= super::MAP_SETTLE_SECS)
    }
}

#[derive(Component)]
pub(super) struct LoadingScreen;

#[derive(Component)]
pub(super) struct LoadingBar;

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
    if gate.ready_at.is_none() {
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
        let loaded = rigs_bound && map && compiled && floor;
        // GPU uploads trail the CPU load; then uploads, scene spawning and shader compiles show
        // up as slow frames, so the area is ready once nearby pieces are renderable and frames
        // stay within budget.
        if loaded {
            let since = *gate.loaded_at.get_or_insert_with(Instant::now);
            if gate.render_ready && time.delta_secs() <= STABLE_FRAME_SECONDS {
                gate.stable_frames += 1;
            } else {
                gate.stable_frames = 0;
            }
            let settled = gate.stable_frames >= STABLE_FRAMES;
            let timed_out = since.elapsed().as_secs_f32() >= STABLE_TIMEOUT_SECONDS;
            if settled || timed_out {
                gate.ready_at = Some(Instant::now());
                let launch = crate::process_start().elapsed().as_secs_f32();
                if settled {
                    info!(
                        "play ready: map loaded and {STABLE_FRAMES} frames within budget, {launch:.1} s after launch ({:.1} s settling)",
                        since.elapsed().as_secs_f32()
                    );
                } else {
                    warn!(
                        "play ready: not settled within {STABLE_TIMEOUT_SECONDS} s of loading (near meshes not on the GPU, near textures pending, images and meshes not uploaded: {:?}), starting anyway {launch:.1} s after launch",
                        gate.render_missing
                    );
                }
            }
        }
        let map_progress = pending.as_ref().map_or(1.0, |p| p.progress());
        gate.progress = map_progress * 0.9
            + 0.04 * f32::from(u8::from(rigs_bound))
            + 0.03 * f32::from(u8::from(compiled))
            + 0.03 * (gate.stable_frames as f32 / STABLE_FRAMES as f32).min(1.0);
    } else {
        gate.progress = 1.0;
    }
    for mut bar in &mut bars {
        bar.width = percent(gate.progress * 100.0);
    }
    if sim.step > 0 {
        for screen in &screens {
            commands.entity(screen).despawn();
        }
    }
}

/// Once everything has loaded, checks that the drawn map meshes near Wolf are renderable: the
/// render world has prepared their mesh buffers ([`crate::map::ready::UploadedMeshes`]) and
/// their materials' textures are on the GPU (the capture readiness check). GPU uploads are
/// spread over frames (`crate::map::upload_budget`), so this trails the CPU-side load.
pub(super) fn check_render(world: &mut World) {
    let Some(uploads) = world
        .get_resource::<crate::map::ready::UploadedMeshes>()
        .cloned()
    else {
        world.resource_mut::<PlayGate>().render_ready = true;
        return;
    };
    let gate = world.resource::<PlayGate>();
    let waiting = gate.ready_at.is_none() && gate.loaded_at.is_some();
    uploads
        .wanted
        .store(waiting, std::sync::atomic::Ordering::Relaxed);
    if !waiting {
        return;
    }
    let wolf = Vec3::from_array(world.non_send::<Sim>().duel.player.body.position);
    let mut query = world.query::<(
        Entity,
        &Mesh3d,
        &GlobalTransform,
        &InheritedVisibility,
        &bevy::camera::primitives::Aabb,
    )>();
    let mut near = Vec::new();
    for (entity, mesh, transform, visible, aabb) in query.iter(world) {
        if !visible.get() {
            continue;
        }
        let centre = transform.transform_point(Vec3::from(aabb.center));
        let extent = Vec3::from(transform.affine().matrix3.abs() * aabb.half_extents);
        if wolf.distance(wolf.clamp(centre - extent, centre + extent)) > RENDER_RADIUS {
            continue;
        }
        near.push((entity, mesh.id()));
    }
    // Map pieces only: Wolf and effects have their own readiness.
    near.retain(|&(entity, _)| {
        let mut ancestor = entity;
        for _ in 0..64 {
            if world.get::<crate::map::DrawGroups>(ancestor).is_some() {
                return true;
            }
            match world.get::<ChildOf>(ancestor) {
                Some(parent) => ancestor = parent.parent(),
                None => return false,
            }
        }
        false
    });
    let missing = {
        let ids = uploads.ids.lock().unwrap();
        near.iter().filter(|(_, id)| !ids.contains(id)).count()
    };
    let entities: Vec<Entity> = near.iter().map(|(e, _)| *e).collect();
    let textures = crate::map::ready::textures_pending(world, &entities);
    let (images, meshes) = crate::map::ready::pending_uploads(world);
    let near_count = near.len();
    let uploaded = missing == 0 && !textures && images == 0 && meshes == 0;
    // Bevy extracts a mesh entity when its components change. One first extracted while its
    // mesh was still queued behind the upload budget stays undrawn until something changes,
    // so once every upload has finished, every mesh entity is extracted again.
    if uploaded && !world.resource::<PlayGate>().refreshed {
        let mut query = world.query::<&mut Mesh3d>();
        for mut mesh in query.iter_mut(world) {
            mesh.set_changed();
        }
        world.resource_mut::<PlayGate>().refreshed = true;
        return;
    }
    let mut gate = world.resource_mut::<PlayGate>();
    gate.render_ready = uploaded;
    gate.render_missing = (missing, textures, images, meshes);
    if gate
        .last_log
        .is_none_or(|t| t.elapsed().as_secs_f32() >= 1.0)
    {
        gate.last_log = Some(Instant::now());
        info!(
            "loading: {near_count} map meshes near Wolf, {missing} not on the GPU, textures pending {textures}; {images} images and {meshes} meshes waiting for upload"
        );
    }
}

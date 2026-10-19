//! Rendering and overhead gauges for the optional MSB population.
use super::Sim;
use crate::character::{AnimLibrary, Animator, PoseDormant, PresentationLod, Rig};
use crate::draw_mask::{DrawMask, MaskSource};
use anyhow::Result;
use bevy::camera::primitives::{Frustum, Sphere};
use bevy::camera::visibility::{DynamicSkinnedMeshBounds, NoFrustumCulling};
use bevy::light::NotShadowCaster;
use bevy::mesh::skinning::SkinnedMesh;
use bevy::prelude::*;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

#[derive(Component)]
pub(super) struct FoeActor(usize, Vec<Entity>, f32, bool);
#[derive(Component)]
struct Panel(usize);
#[derive(Component)]
struct Fill(usize, bool);
#[derive(Component)]
struct Mark(usize);
#[derive(Resource)]
struct Libraries(HashMap<String, AnimLibrary>);
#[derive(Resource)]
struct Placements(Vec<(String, String, Option<[bool; 32]>)>);

pub(super) fn install(app: &mut App, cache: &Path) -> Result<()> {
    let roster = sekiro_sim::registry::Registry::load(cache).map_err(anyhow::Error::msg)?;
    let sim = app.world().non_send::<Sim>();
    let mut libraries = HashMap::new();
    let mut placements = Vec::new();
    let sound_foes = sim
        .duel
        .population
        .as_ref()
        .unwrap()
        .foes
        .iter()
        .map(|foe| {
            (
                foe.duel.spec.chr.clone(),
                foe.duel.spec.npc_param,
                foe.active,
            )
        })
        .collect::<Vec<_>>();
    for foe in &sim.duel.population.as_ref().unwrap().foes {
        let spec = &foe.duel.spec;
        let donor = roster.anim_chr(&spec.chr, spec.npc_param);
        if let std::collections::hash_map::Entry::Vacant(entry) = libraries.entry(donor.clone()) {
            let mut lib = AnimLibrary::load(cache, &donor)?;
            lib.clip("a000_000000")?;
            entry.insert(lib);
        }
        if foe.active {
            libraries.get_mut(&donor).unwrap().prefetch_all();
        }
        anyhow::ensure!(
            cache.join(format!("models/{}.glb", spec.chr)).is_file(),
            "map enemies: missing model {}",
            spec.chr
        );
        placements.push((spec.chr.clone(), donor, roster.draw_mask(cache, spec)));
    }
    crate::audio::prepare_population(app, &sound_foes);
    app.world_mut()
        .resource_scope(|world, mut fx: Mut<crate::fxr::FxrPlayer>| {
            fx.ensure_tables(&world.non_send::<Sim>().duel);
        });
    app.insert_resource(Libraries(libraries))
        .insert_resource(Placements(placements))
        .add_systems(Startup, spawn.before(crate::map::MapSpawn))
        .add_systems(Update, sound_banks.before(crate::audio::play_cues))
        .add_systems(
            Update,
            (
                sync.before(crate::character::animate)
                    .after(crate::character::bind_rigs),
                gauges.after(super::follow_camera),
            ),
        );
    Ok(())
}

fn sound_banks(sim: NonSend<Sim>, fmod: Option<NonSendMut<crate::audio::FmodAudio>>) {
    if let Some(mut fmod) = fmod {
        fmod.population_banks(
            sim.duel
                .population
                .as_ref()
                .unwrap()
                .foes
                .iter()
                .filter(|foe| foe.active)
                .map(|foe| foe.duel.spec.chr.as_str()),
        );
    }
}

fn spawn(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut libraries: ResMut<Libraries>,
    placements: Res<Placements>,
    sim: NonSend<Sim>,
) {
    let population = sim.duel.population.as_ref().unwrap();
    for (i, (model, donor, mask)) in placements.0.iter().enumerate() {
        let foe = &population.foes[i];
        let lib = libraries.0.get_mut(donor).unwrap();
        let mut idle = Animator::new(lib.clip("a000_000000").unwrap(), false);
        idle.apply_root_motion = false;
        idle.speed = 0.0;
        let mut e = commands.spawn((
            FoeActor(i, Vec::new(), f32::NAN, false),
            WorldAssetRoot(
                assets
                    .load_builder()
                    .with_settings(|settings: &mut bevy::gltf::GltfLoaderSettings| {
                        // Generate per-joint mesh bounds once on the shared mesh asset.
                        settings.skinned_mesh_bounds_policy =
                            Some(bevy::gltf::GltfSkinnedMeshBoundsPolicy::Dynamic);
                    })
                    .load(GltfAssetLabel::Scene(0).from_asset(format!("models/{model}.glb"))),
            ),
            Transform::from_translation(Vec3::from_array(foe.duel.enemy.body.position))
                .with_rotation(Quat::from_rotation_y(foe.duel.enemy.visual_yaw())),
            Visibility::Inherited,
            Rig::new(Arc::new(lib.skeleton.clone())),
            idle,
            mask.map_or_else(|| DrawMask::new(MaskSource::Combat), DrawMask::with_groups),
        ));
        if !foe.active {
            e.insert(PoseDormant);
        }
        commands
            .spawn((
                Panel(i),
                Node {
                    position_type: PositionType::Absolute,
                    width: px(130.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(3.0),
                    ..default()
                },
                Visibility::Hidden,
            ))
            .with_children(|p| {
                p.spawn((
                    Mark(i),
                    Text::new("●"),
                    TextFont {
                        font_size: bevy::text::FontSize::Px(20.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.95, 0.05, 0.04)),
                    Node {
                        position_type: PositionType::Absolute,
                        right: px(-20.0),
                        top: px(-8.0),
                        ..default()
                    },
                    Visibility::Hidden,
                ));
                for posture in [true, false] {
                    p.spawn((
                        Node {
                            width: percent(100.0),
                            height: px(if posture { 5.0 } else { 8.0 }),
                            padding: UiRect::all(px(1.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.02, 0.02, 0.02, 0.8)),
                    ))
                    .with_children(|bar| {
                        bar.spawn((
                            Fill(i, posture),
                            Node {
                                width: percent(100.0),
                                height: percent(100.0),
                                ..default()
                            },
                            BackgroundColor(Color::NONE),
                        ));
                    });
                }
            });
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn sync(
    mut commands: Commands,
    sim: NonSend<Sim>,
    fixed: Res<Time<Fixed>>,
    placements: Res<Placements>,
    mut libraries: ResMut<Libraries>,
    cameras: Query<(&Frustum, &GlobalTransform), With<Camera3d>>,
    children: Query<&Children>,
    rigs: Query<&Rig>,
    meshes: Query<
        (
            Option<&SkinnedMesh>,
            Has<DynamicSkinnedMeshBounds>,
            Has<NotShadowCaster>,
        ),
        With<Mesh3d>,
    >,
    mut actors: Query<(
        Entity,
        &mut FoeActor,
        &mut Animator,
        &mut Transform,
        Option<&PoseDormant>,
        &mut Visibility,
    )>,
) {
    let p = sim.duel.population.as_ref().unwrap();
    for (entity, mut actor, mut anim, mut tf, dormant, mut visibility) in &mut actors {
        let foe = &p.foes[actor.0];
        let position = Vec3::from_array(foe.duel.enemy.body.position);
        let radius = (foe.duel.fighters[1].height * 2.0).max(8.0);
        let near = position.distance(Vec3::from_array(sim.duel.player.body.position)) <= 20.0;
        let draw = near
            || cameras.iter().next().is_none_or(|(frustum, _)| {
                frustum.intersects_sphere(
                    &Sphere {
                        center: (position + Vec3::Y * (radius * 0.5)).into(),
                        radius,
                    },
                    true,
                )
            });
        let full = foe.lod == sekiro_sim::duel::population::Lod::Full;
        let pose = draw && ((foe.active && (full || actor.2 != foe.clock)) || !actor.3);
        if pose {
            actor.2 = foe.clock;
        }
        actor.3 = draw;
        let lod = PresentationLod {
            draw,
            pose,
            cloth: draw && foe.active && full,
        };
        commands.entity(entity).insert(lod);
        visibility.set_if_neq(if draw {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        let bound = rigs.get(entity).is_ok_and(|r| r.bound);
        let first = actor.1.is_empty() && bound;
        if first {
            actor.1 = children
                .iter_descendants(entity)
                .filter(|e| meshes.contains(*e))
                .collect();
        }
        // GLB mesh/material handles come from one AssetServer scene per model, shared by
        // every placement. Only CPU cloth's mutable display buffers are per character.
        let shadows = cameras
            .iter()
            .next()
            .is_none_or(|(_, cam)| position.distance(cam.translation()) <= 30.0);
        for &mesh in &actor.1 {
            let Ok((skin, dynamic, no_shadow)) = meshes.get(mesh) else {
                continue;
            };
            if first {
                commands.entity(mesh).remove::<NoFrustumCulling>();
            }
            let update_bounds = skin.is_some() && pose;
            if update_bounds && !dynamic {
                commands.entity(mesh).insert(DynamicSkinnedMeshBounds);
            }
            if !update_bounds && dynamic {
                commands.entity(mesh).remove::<DynamicSkinnedMeshBounds>();
            }
            if shadows && no_shadow {
                commands.entity(mesh).remove::<NotShadowCaster>();
            }
            if !shadows && !no_shadow {
                commands.entity(mesh).insert(NotShadowCaster);
            }
        }
        if !foe.active {
            if dormant.is_none() {
                commands.entity(entity).insert(PoseDormant);
            }
            continue;
        }
        if dormant.is_some() {
            commands.entity(entity).remove::<PoseDormant>();
        }
        let r = &foe.report.enemy;
        let alpha = fixed.overstep_fraction();
        tf.translation = super::interpolate_position(
            Vec3::from_array(foe.previous.0),
            Vec3::from_array(foe.duel.enemy.body.position),
            alpha,
        );
        let yaw = foe.duel.enemy.visual_yaw();
        let turn = (yaw - foe.previous.1 + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI;
        tf.rotation = Quat::from_rotation_y(foe.previous.1 + turn * alpha);
        anim.walk_twist = foe.duel.enemy.walk_twist;
        anim.clock = Some(foe.clock - super::behind(r, alpha));
        let Some(name) = &r.animation else {
            continue;
        };
        let lib = libraries.0.get_mut(&placements.0[actor.0].1).unwrap();
        if anim.clip_name() != name || (r.clip_started && anim.time > r.anim_time) {
            match lib.clip(name) {
                Ok(clip) => anim.play_named(name, clip, false, r.blend_in),
                Err(error) => {
                    warn!("map enemy {}: {error}", foe.part);
                    continue;
                }
            }
        }
        anim.time = super::drawn_clip_time(r, alpha);
    }
}

#[allow(clippy::type_complexity)]
fn gauges(
    sim: NonSend<Sim>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut panels: Query<(&Panel, &mut Node, &mut Visibility)>,
    mut fills: Query<(&Fill, &mut Node, &mut BackgroundColor), Without<Panel>>,
    mut marks: Query<(&Mark, &mut Visibility), Without<Panel>>,
) {
    let p = sim.duel.population.as_ref().unwrap();
    let Ok((camera, tf)) = camera.single() else {
        return;
    };
    for (panel, mut node, mut vis) in &mut panels {
        let foe = &p.foes[panel.0];
        let d = &foe.duel;
        let head =
            Vec3::from_array(d.enemy.body.position) + Vec3::Y * (d.fighters[1].height + 0.35);
        *vis = Visibility::Hidden;
        if !foe.active || d.rules.fighters[1].dead() {
            continue;
        }
        if let Ok(screen) = camera.world_to_viewport(tf, head) {
            node.left = px(screen.x - 65.0);
            node.top = px(screen.y - 16.0);
            *vis = Visibility::Inherited;
        }
    }
    for (fill, mut node, mut color) in &mut fills {
        let f = &p.foes[fill.0].duel.rules.fighters[1];
        let fraction = if fill.1 {
            f.posture_fraction()
        } else {
            f.hp_fraction()
        };
        node.width = percent(fraction.clamp(0.0, 1.0) * 100.0);
        color.0 = if fill.1 {
            Color::srgb(0.95, 0.65, 0.16)
        } else {
            Color::srgb(0.72, 0.1, 0.08)
        };
    }
    for (mark, mut vis) in &mut marks {
        *vis = if p.foes[mark.0].active && p.foes[mark.0].can_deathblow {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

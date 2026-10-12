//! Sunken Valley supporting actors and silvergrass storm presentation.
#[path = "ape_hud.rs"]
mod ape_hud;
use super::Sim;
use crate::{
    character::{AnimLibrary, Animator, Rig},
    draw_mask::DrawMask,
};
use anyhow::Result;
use bevy::prelude::*;
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Component)]
struct Support {
    entity: i32,
    library: AnimLibrary,
    requested: Option<(i32, bool)>,
    warped: Option<(i32, i16)>,
    base_groups: [bool; 32],
    tae_mask: Option<[u8; 32]>,
}
#[derive(Resource)]
struct Stage {
    cache: PathBuf,
    cast: Vec<(Value, AnimLibrary, [bool; 32])>,
    gparam: Option<u8>,
    base_gparam: Option<sekiro_formats::gparam::Gparam>,
}

pub(super) fn install(
    app: &mut App,
    cache: &Path,
    spec: Option<&sekiro_sim::arena::ArenaSpec>,
) -> Result<()> {
    let Some(spec) = spec.filter(|s| matches!(s.map, "m11_02_00_00" | "m17_00_00_00")) else {
        return Ok(());
    };
    let mut stage = Stage {
        cache: cache.into(),
        cast: Vec::new(),
        gparam: None,
        base_gparam: None,
    };
    if spec.map == "m17_00_00_00" {
        let layout: Value = serde_json::from_slice(&std::fs::read(
            cache.join(format!("maps/{}/layout.json", spec.map)),
        )?)?;
        let registry = sekiro_sim::registry::Registry::load(cache).map_err(anyhow::Error::msg)?;
        let ids: &[i32] = if spec.name == "apes-burrow" {
            &[1_700_851, 1_700_852]
        } else {
            &[1_700_801]
        };
        for &entity in ids {
            let row = layout["enemies"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["entity_id"].as_i64() == Some(i64::from(entity)))
                .ok_or_else(|| anyhow::anyhow!("m17 layout: missing supporting actor {entity}"))?;
            let enemy = sekiro_sim::enemy::EnemySpec {
                chr: row["model"].as_str().unwrap().into(),
                npc_param: row["npc_param"].as_i64().unwrap() as i32,
                think: row["think_param"].as_i64().unwrap() as i32,
                ai_bundle: format!("{}.luabnd.d", spec.map),
                starts_in_battle: false,
            };
            let library =
                AnimLibrary::load(cache, &registry.anim_chr(&enemy.chr, enemy.npc_param))?;
            let mask = registry.draw_mask(cache, &enemy).unwrap_or([true; 32]);
            stage.cast.push((row.clone(), library, mask));
        }
    }
    if spec.map == "m17_00_00_00" && !cache.join("models/c5001.glb").is_file() {
        warn!(
            "arena staging: missing cache/models/c5001.glb; finishing centipede remains invisible"
        );
    }
    app.insert_resource(stage)
        .add_systems(Startup, spawn)
        .add_systems(Startup, ape_hud::spawn)
        .add_systems(Update, ape_hud::update.after(super::sync_actors))
        .add_systems(
            Update,
            update
                .after(super::sync_actors)
                // Play's main chain applies masks before synchronizing actors.
                .after(crate::draw_mask::apply_draw_masks)
                .before(crate::character::animate),
        );
    Ok(())
}

fn spawn(mut commands: Commands, assets: Res<AssetServer>, mut stage: ResMut<Stage>) {
    for (row, mut library, masks) in stage.cast.drain(..) {
        let Ok(clip) = library.clip("a000_000000") else {
            continue;
        };
        let mut animator = Animator::new(clip, true);
        animator.apply_root_motion = false;
        let p = &row["translation"];
        let q = &row["rotation"];
        commands.spawn((
            WorldAssetRoot(
                assets.load(
                    GltfAssetLabel::Scene(0)
                        .from_asset(format!("models/{}.glb", row["model"].as_str().unwrap())),
                ),
            ),
            Transform::from_translation(Vec3::new(
                p[0].as_f64().unwrap() as f32,
                p[1].as_f64().unwrap() as f32,
                p[2].as_f64().unwrap() as f32,
            ))
            .with_rotation(Quat::from_xyzw(
                q[0].as_f64().unwrap() as f32,
                q[1].as_f64().unwrap() as f32,
                q[2].as_f64().unwrap() as f32,
                q[3].as_f64().unwrap() as f32,
            )),
            Visibility::Hidden,
            Rig::new(Arc::new(library.skeleton.clone())),
            animator,
            DrawMask::with_groups(masks),
            Support {
                entity: row["entity_id"].as_i64().unwrap() as i32,
                library,
                requested: None,
                warped: None,
                base_groups: masks,
                tae_mask: None,
            },
        ));
    }
}

fn update(
    sim: NonSend<Sim>,
    mut stage: ResMut<Stage>,
    mut actors: Query<(
        &mut Support,
        &mut Visibility,
        &mut Animator,
        &mut DrawMask,
        &mut Transform,
    )>,
    mut lighting: Option<ResMut<crate::lighting::MapLighting>>,
    mut sky: Option<ResMut<crate::map::sky::SkyOverride>>,
    mut main_masks: Query<(&super::Actor, &mut DrawMask), Without<Support>>,
) {
    let Some(events) = sim.duel.events.as_ref() else {
        return;
    };
    let scene = &events.scripts.cast_scene;
    if let Some(light) = lighting.as_mut() {
        if stage.base_gparam.is_none() {
            stage.base_gparam = Some(light.gparam.clone());
        }
        if events.scripts.arena_c_state.gparam.is_none()
            && stage.gparam.take().is_some()
            && let Some(base) = stage.base_gparam.as_ref()
        {
            light.gparam.clone_from(base);
            light.applied = None;
        }
    }
    if let Some(sky) = sky.as_mut() {
        sky.0 = match events.boss_entity {
            1_120_800 => Some("m900040"),
            1_120_830 | 1_120_860 => Some("m900075"),
            _ => None,
        };
    }
    for (mut support, mut visible, mut animator, mut mask, mut transform) in &mut actors {
        if support.entity == sekiro_sim::duel::headless_ape::BROWN
            && let Some(ape) = sim.duel.headless_ape.as_ref()
        {
            *visible = if ape.duel.enemy_active {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            *transform =
                Transform::from_translation(Vec3::from_array(ape.duel.enemy.body.position))
                    .with_rotation(Quat::from_rotation_y(ape.duel.enemy.body.yaw));
            let report = &ape.report.enemy;
            if support.tae_mask != ape.duel.enemy.tae_runtime.draw_mask {
                support.tae_mask = ape.duel.enemy.tae_runtime.draw_mask;
                if let Some(values) = &support.tae_mask {
                    mask.apply_event(values);
                } else {
                    *mask = DrawMask::with_groups(support.base_groups);
                }
            }
            if let Some(name) = &report.animation {
                if (animator.clip_name() != name || report.clip_started)
                    && let Ok(clip) = support.library.clip(name)
                {
                    animator.play_named(name, clip, false, report.blend_in);
                }
                animator.time = report.anim_time;
                animator.speed = 0.0;
                animator.apply_root_motion = false;
            }
            continue;
        }
        let warp = events
            .scripts
            .arena_c_state
            .bone_warps
            .get(&support.entity)
            .copied();
        if warp.is_none() {
            support.warped = None;
        }
        if !scene.animations.contains_key(&support.entity) {
            support.requested = None;
        }
        if warp != support.warped
            && let Some((entity, dummy)) = warp
            && entity == events.boss_entity
        {
            let model = sim.duel.enemy.bone_source_matrices();
            if let Some(&[position, forward, up]) = sim.duel.fighters[1]
                .rig
                .frames(dummy, &model, sim.duel.enemy.source_to_world())
                .first()
            {
                *transform = crate::fxr::frame_transform(
                    position.to_array(),
                    forward.to_array(),
                    up.to_array(),
                );
                support.warped = warp;
            }
        }
        *visible = if scene.enabled.get(&support.entity).copied().unwrap_or(false) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if let Some(&request) = scene.animations.get(&support.entity)
            && support.requested != Some(request)
        {
            let name = format!("a000_{:06}", request.0);
            if let Ok(clip) = support.library.clip(&name) {
                animator.play_named(&name, clip, request.1, 0.0);
                support.requested = Some(request);
            }
        }
        if let Some(masks) = scene.masks.get(&support.entity) {
            let mut values = [2; 32];
            for (&bit, &on) in masks {
                if let Some(v) = values.get_mut(bit as usize) {
                    *v = u8::from(on);
                }
            }
            mask.apply_event(&values);
        }
    }
    if let Some(values) = &sim.duel.enemy.tae_runtime.draw_mask {
        for (actor, mut mask) in &mut main_masks {
            if actor.0 == super::ENEMY {
                mask.apply_event(values);
            }
        }
    }
    if let Some(masks) = scene.masks.get(&events.boss_entity) {
        for (actor, mut mask) in &mut main_masks {
            if actor.0 != super::ENEMY {
                continue;
            }
            let mut values = [2; 32];
            for (&bit, &on) in masks {
                if let Some(v) = values.get_mut(bit as usize) {
                    *v = u8::from(on);
                }
            }
            mask.apply_event(&values);
        }
    }
    if let Some((11, 2, sub, _seconds)) = events.scripts.arena_c_state.gparam
        && stage.gparam != Some(sub)
        && let Some(light) = lighting.as_mut()
    {
        // 0930 is the sole authored storm-sub-ID file for this map; 0920 is its clear variant.
        // The transition snaps to the authored set; interpolation is still a renderer gap.
        let path = stage
            .cache
            .join(format!("raw/param/drawparam/m11_02_09{sub:02}.gparam"));
        match std::fs::read(&path)
            .ok()
            .and_then(|b| sekiro_formats::gparam::parse(&b).ok())
        {
            Some(gparam) => {
                light.gparam = gparam;
                light.applied = None;
            }
            None => warn!("arena staging: missing GPARAM {}", path.display()),
        }
        stage.gparam = Some(sub);
    }
}

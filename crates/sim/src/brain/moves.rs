//! Boss knowledge derived once from the same clips, hitboxes and posture resolver as combat.
use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::character::ai_cancel;
use crate::combat::hit::HitOutcome;
use crate::duel::Duel;
use crate::tae::TaeKind;

use super::BossActions;

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Move {
    /// [seconds, guard posture, deflect posture, behaviour judge].
    pub hits: Vec<[f32; 4]>,
    pub duration: f32,
    pub recovery: f32,
    /// [start, end, ChrActionFlag, stateInfo]; stateInfo 0 is unconditional.
    pub cancels: Vec<[f32; 4]>,
    pub reach: Option<f32>,
}

pub type MoveSheet = BTreeMap<String, Move>;

/// Pool only byte-identical decoded clips with identical resolved TAE event timelines.
pub fn shared_genichiro_actions(
    cache: &std::path::Path,
    old: &crate::Character,
    next: &crate::Character,
) -> BTreeSet<String> {
    let mut shared = BTreeSet::new();
    for id in (3000..3100).chain(5200..5300) {
        let name = format!("a000_{id:06}");
        let (Some(a), Some(b)) = (old.tae.anim(&name), next.tae.anim(&name)) else {
            continue;
        };
        if a.events != b.events {
            continue;
        }
        let first = std::fs::read(cache.join("anim/c7100").join(format!("{name}.bin")));
        let second = std::fs::read(cache.join("anim/c7110").join(format!("{name}.bin")));
        if let (Ok(a), Ok(b)) = (first, second)
            && a == b
        {
            shared.insert(id.to_string());
        }
    }
    shared
}

fn rounded(v: f32) -> f32 {
    (v * 1000.0).round() / 1000.0
}

fn root_at(clip: &sekiro_formats::anim::AnimClip, time: f32) -> glam::Mat4 {
    let Some(rm) = &clip.root_motion else {
        return glam::Mat4::IDENTITY;
    };
    let start = rm.sample(0.0);
    let end = rm.sample(time);
    glam::Mat4::from_translation(glam::Vec3::new(
        end[0] - start[0],
        end[1] - start[1],
        end[2] - start[2],
    )) * glam::Mat4::from_rotation_y(end[3] - start[3])
}

/// Nominal radial envelope without target-dependent TAE 760 scaling or steering.
fn clip_reach(duel: &Duel, id: i32, limit: f32, origin: glam::Mat4) -> Option<f32> {
    let npc = &duel.enemy;
    let fighter = &duel.fighters[1];
    let name = format!("a000_{id:06}");
    let clip = npc.clips.clip(&name)?;
    let tae = npc.tae.anim(&name)?;
    let mut reach: Option<f32> = None;
    for e in tae.events.iter() {
        let TaeKind::Attack { judge, .. } = e.kind else {
            continue;
        };
        if e.state_info != 0 {
            continue;
        }
        let Some(attack) = fighter.attacks.get(&judge) else {
            continue;
        };
        if crate::perilous::counter_dummy(attack) {
            continue;
        }
        let mut t = e.start;
        while t <= e.end.min(clip.duration).min(limit) {
            let model = npc.skeleton.model_space(&clip.sample(t, &npc.skeleton));
            let world = origin * root_at(&clip, t);
            for shape in &attack.shapes {
                for dummy in [shape.dummy_a, shape.dummy_b] {
                    for p in fighter.rig.positions(dummy, &model, world) {
                        let r = p.x.hypot(p.z) + shape.radius + duel.fighters[0].radius;
                        reach = Some(reach.unwrap_or(0.0).max(rounded(r)));
                    }
                }
            }
            t += 1.0 / 60.0;
        }
    }
    reach
}

impl Move {
    /// Time to the first unconditional attack cancel or natural end, from clip time.
    pub fn wait(&self, time: f32) -> f32 {
        self.cancels
            .iter()
            .filter(|w| {
                w[3] == 0.0
                    && matches!(w[2] as i32, ai_cancel::COMBO | ai_cancel::ATTACK)
                    && w[1] > time
            })
            .map(|w| (w[0] - time).max(0.0))
            .fold((self.duration - time).max(0.0), f32::min)
    }
}

pub fn build(duel: &Duel, actions: &BossActions) -> MoveSheet {
    let mut sheet = MoveSheet::new();
    let npc = &duel.enemy;
    let fighter = &duel.fighters[1];
    let effects_d = duel.rules.active_effects(0, &duel.player);
    // Wolf's normal sword guard behaviour, independent of his current idle clip.
    let guard = duel.fighters[0].attacks.get(&90);
    let ids: BTreeSet<_> = actions
        .actions
        .iter()
        .map(|a| a.id)
        .chain(actions.counter_ids())
        .collect();
    for id in ids {
        let name = format!("a000_{id:06}");
        let (Some(clip), Some(tae)) = (npc.clips.clip(&name), npc.tae.anim(&name)) else {
            continue;
        };
        let mut row = Move {
            duration: rounded(clip.duration),
            ..Move::default()
        };
        for e in tae.events.iter() {
            match e.kind {
                TaeKind::ActionFlag(flag)
                    if matches!(
                        flag,
                        ai_cancel::COMBO
                            | ai_cancel::ATTACK
                            | ai_cancel::STEP
                            | crate::tae::cancel::MOVE
                    ) =>
                {
                    row.cancels.push([
                        rounded(e.start),
                        rounded(e.end),
                        flag as f32,
                        e.state_info as f32,
                    ]);
                }
                TaeKind::Attack { judge, .. } if e.state_info == 0 => {
                    let Some(attack) = fighter.attacks.get(&judge) else {
                        continue;
                    };
                    let Some(profile) = &attack.profile else {
                        continue;
                    };
                    if crate::perilous::counter_dummy(attack) {
                        continue;
                    }
                    let mut runtime = crate::tae::TaeRuntime::default();
                    runtime.update(
                        &npc.tae,
                        &[crate::tae::PlayingClip {
                            animation: name.clone(),
                            prev_time: e.start,
                            time: e.start,
                        }],
                        0.0,
                    );
                    let effects_a = duel.rules.effects_for(1, &runtime.frame);
                    let damage = |outcome| {
                        duel.rules.posture_for_outcome(
                            1, 0, profile, outcome, guard, &effects_a, &effects_d,
                        ) as f32
                    };
                    let hit = [
                        rounded(e.start),
                        damage(HitOutcome::Block),
                        damage(HitOutcome::Deflect),
                        judge as f32,
                    ];
                    if !row.hits.iter().any(|h| h[0] == hit[0] && h[3] == hit[3]) {
                        row.hits.push(hit);
                    }
                }
                _ => {}
            }
        }
        row.reach = clip_reach(duel, id, clip.duration, glam::Mat4::IDENTITY);
        row.hits.sort_by(|a, b| a[0].total_cmp(&b[0]));
        row.cancels.sort_by(|a, b| a[0].total_cmp(&b[0]));
        row.recovery = row.hits.last().map_or(0.0, |h| rounded(row.wait(h[0])));
        sheet.insert(id.to_string(), row);
    }
    let mut combos: BTreeSet<Vec<i32>> = actions.combos.iter().map(|c| c.ids.to_vec()).collect();
    // Attack groups in mixed tactic plans are part of the offered vocabulary too.
    for chain in actions.chains {
        for group in chain.split('>') {
            if group.contains('+')
                && let Ok(ids) = group
                    .split('+')
                    .map(str::parse::<i32>)
                    .collect::<Result<Vec<_>, _>>()
            {
                combos.insert(ids);
            }
        }
    }
    for ids in combos {
        let mut row = Move::default();
        let mut complete = true;
        let mut origin = glam::Mat4::IDENTITY;
        for (i, id) in ids.iter().enumerate() {
            let Some(part) = sheet.get(&id.to_string()) else {
                complete = false;
                break;
            };
            let offset = row.duration;
            let length = if i + 1 < ids.len() {
                part.wait(0.0)
            } else {
                part.duration
            };
            row.hits.extend(
                part.hits
                    .iter()
                    .filter(|h| h[0] <= length)
                    .map(|h| [rounded(h[0] + offset), h[1], h[2], h[3]]),
            );
            row.cancels
                .extend(part.cancels.iter().filter(|w| w[0] <= length).map(|w| {
                    [
                        rounded(w[0] + offset),
                        rounded(w[1].min(length) + offset),
                        w[2],
                        w[3],
                    ]
                }));
            row.duration = rounded(offset + length);
            row.recovery = part.recovery;
            row.reach = row
                .reach
                .into_iter()
                .chain(clip_reach(duel, *id, length, origin))
                .max_by(f32::total_cmp);
            if let Some(clip) = npc.clips.clip(&format!("a000_{id:06}")) {
                origin *= root_at(&clip, length);
            }
        }
        if complete {
            sheet.insert(
                ids.iter().map(i32::to_string).collect::<Vec<_>>().join("+"),
                row,
            );
        }
    }
    sheet
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_wait_uses_only_live_attack_windows_or_end() {
        let row = Move {
            duration: 3.0,
            cancels: vec![
                [0.1, 0.5, 79.0, 0.0],
                [0.2, 0.5, 23.0, 99.0],
                [1.0, 1.5, 23.0, 0.0],
                [2.0, 2.5, 86.0, 0.0],
            ],
            ..Default::default()
        };
        assert_eq!(row.wait(0.0), 1.0);
        assert_eq!(row.wait(1.0), 0.0);
        assert_eq!(row.wait(1.5), 0.5);
        assert_eq!(row.wait(2.5), 0.5);
        assert_eq!(row.wait(3.0), 0.0);
    }
}

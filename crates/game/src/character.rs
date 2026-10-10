//! Drives an exported character GLB with decoded Havok clips.
//!
//! Clips and skeletons come from `cache/anim/<chr>/` and are converted to Bevy space once at load.
//! Each frame the active clip is sampled in Havok bone order, composed to model space, and each
//! glTF joint (matched by bone name) gets the transform relative to its glTF parent. Root motion
//! moves the character entity itself.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use bevy::prelude::*;
use sekiro_formats::anim::{AnimClip, Skeleton, to_bevy};

/// Loaded skeleton and clip cache for one character id (`c0000`, `c1010`).
pub struct AnimLibrary {
    dir: PathBuf,
    pub skeleton: Skeleton,
    clips: HashMap<String, Arc<AnimClip>>,
    /// Animations that play another's HKX (`aliases.json` from the extractor).
    aliases: HashMap<String, String>,
}

impl AnimLibrary {
    pub fn load(cache: &Path, chr: &str) -> Result<Self> {
        let dir = cache.join("anim").join(chr);
        let bytes = std::fs::read(dir.join("skeleton.bin"))
            .with_context(|| format!("{}: run `sekiro-extract anims {chr}`", dir.display()))?;
        let skeleton = to_bevy::skeleton(&Skeleton::from_bytes(&bytes)?);
        let aliases = std::fs::read_to_string(dir.join("aliases.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Ok(Self {
            dir,
            skeleton,
            clips: HashMap::new(),
            aliases,
        })
    }

    pub fn clip(&mut self, name: &str) -> Result<Arc<AnimClip>> {
        if let Some(c) = self.clips.get(name) {
            return Ok(c.clone());
        }
        let file = self.aliases.get(name).map_or(name, String::as_str);
        let path = self.dir.join(format!("{file}.bin"));
        let bytes = std::fs::read(&path).with_context(|| format!("{}", path.display()))?;
        let clip = Arc::new(to_bevy::clip(&AnimClip::from_bytes(&bytes)?));
        self.clips.insert(name.to_string(), clip.clone());
        Ok(clip)
    }
}

/// What a character is playing. Gameplay code sets `clip`; [`animate`] advances and applies it.
#[derive(Component)]
pub struct Animator {
    pub clip: Arc<AnimClip>,
    pub time: f32,
    pub looping: bool,
    pub speed: f32,
    pub apply_root_motion: bool,
    /// Root motion sample at the previous frame, for deltas.
    last_root: [f32; 4],
    /// The animation name that was asked for (an alias may play another clip's data).
    name: String,
    /// Crossfade from the previous pose: (clip, its time when replaced, duration, elapsed).
    blend: Option<(Arc<AnimClip>, f32, f32, f32)>,
    /// The locked-on walk twist in degrees, positive with the legs turned right
    /// (`sekiro_sim::Character::walk_twist`), applied to the pose by [`apply_walk_twist`].
    pub walk_twist: f32,
}

impl Animator {
    pub fn new(clip: Arc<AnimClip>, looping: bool) -> Self {
        Self {
            time: 0.0,
            looping,
            speed: 1.0,
            apply_root_motion: true,
            last_root: [0.0; 4],
            name: clip_name_of(&clip),
            blend: None,
            walk_twist: 0.0,
            clip,
        }
    }

    pub fn play(&mut self, clip: Arc<AnimClip>, looping: bool) {
        self.name = clip_name_of(&clip);
        self.blend = None;
        self.clip = clip;
        self.time = 0.0;
        self.looping = looping;
        self.last_root = [0.0; 4];
    }

    pub fn finished(&self) -> bool {
        !self.looping && self.time >= self.clip.duration
    }

    /// Plays `clip` under the requested animation `name`, crossfading from the current pose over
    /// `blend_secs` (0 cuts).
    pub fn play_named(&mut self, name: &str, clip: Arc<AnimClip>, looping: bool, blend_secs: f32) {
        let from = (self.clip.clone(), self.time);
        self.play(clip, looping);
        self.name = name.to_string();
        if blend_secs > 0.0 {
            self.blend = Some((from.0, from.1, blend_secs, 0.0));
        }
    }

    /// The requested animation name (which may be an alias of the clip data playing).
    pub fn clip_name(&self) -> &str {
        &self.name
    }
}

/// Havok skeleton plus the glTF joint entity for each bone (filled once the scene spawns).
#[derive(Component)]
pub struct Rig {
    pub skeleton: Arc<Skeleton>,
    /// Per Havok bone: the joint entity and the Havok index of its glTF parent, if animated.
    joints: Vec<Option<(Entity, Option<usize>)>>,
    pub bound: bool,
    /// Model-space bone matrices from the last applied pose (Bevy space, character local).
    pub model: Vec<Mat4>,
}

impl Rig {
    pub fn new(skeleton: Arc<Skeleton>) -> Self {
        Self {
            skeleton,
            joints: Vec::new(),
            bound: false,
            model: Vec::new(),
        }
    }
}

/// Binds Havok bones to glTF joint entities by name once the character's scene exists.
pub fn bind_rigs(
    mut rigs: Query<(Entity, &mut Rig)>,
    children: Query<&Children>,
    names: Query<&Name>,
    parents: Query<&ChildOf>,
) {
    for (root, mut rig) in &mut rigs {
        if rig.bound {
            continue;
        }
        let mut by_name: HashMap<String, Entity> = HashMap::new();
        for e in children.iter_descendants(root) {
            if let Ok(n) = names.get(e) {
                by_name.entry(n.as_str().to_string()).or_insert(e);
            }
        }
        if !by_name.contains_key("Master") {
            continue; // scene not spawned yet
        }
        let index: HashMap<Entity, usize> = rig
            .skeleton
            .bones
            .iter()
            .enumerate()
            .filter_map(|(i, b)| by_name.get(&b.name).map(|&e| (e, i)))
            .collect();
        let joints = rig
            .skeleton
            .bones
            .iter()
            .map(|b| {
                by_name.get(&b.name).map(|&e| {
                    let parent = parents
                        .get(e)
                        .ok()
                        .and_then(|p| index.get(&p.parent()).copied());
                    (e, parent)
                })
            })
            .collect();
        let bound = rig
            .skeleton
            .bones
            .iter()
            .filter(|b| by_name.contains_key(&b.name))
            .count();
        info!("rig bound {bound}/{} bones", rig.skeleton.bones.len());
        rig.joints = joints;
        rig.bound = true;
    }
}

pub fn animate(
    time: Res<Time>,
    mut q: Query<(&mut Animator, &mut Rig, &mut Transform)>,
    mut joints: Query<&mut Transform, Without<Rig>>,
) {
    let dt = time.delta_secs();
    for (mut anim, mut rig, mut body) in &mut q {
        if !rig.bound {
            continue;
        }
        let prev = anim.time;
        anim.time += dt * anim.speed;
        let clip = anim.clip.clone();
        let t = if anim.looping && clip.duration > 0.0 {
            anim.time.rem_euclid(clip.duration)
        } else {
            anim.time.min(clip.duration)
        };

        if anim.apply_root_motion
            && let Some(rm) = &clip.root_motion
        {
            let wrapped = anim.looping && clip.duration > 0.0 && t < prev.rem_euclid(clip.duration);
            let now = rm.sample(t);
            let delta = if wrapped {
                let end = rm.total();
                [
                    end[0] - anim.last_root[0] + now[0],
                    end[1] - anim.last_root[1] + now[1],
                    end[2] - anim.last_root[2] + now[2],
                    end[3] - anim.last_root[3] + now[3],
                ]
            } else {
                [
                    now[0] - anim.last_root[0],
                    now[1] - anim.last_root[1],
                    now[2] - anim.last_root[2],
                    now[3] - anim.last_root[3],
                ]
            };
            anim.last_root = now;
            let local = Vec3::new(delta[0], delta[1], delta[2]);
            let moved = body.rotation * local;
            body.translation += moved;
            body.rotate_y(delta[3]);
        }

        let mut pose = clip.sample(t, &rig.skeleton);
        // Crossfade from the previous clip's pose, frozen where it was replaced.
        if let Some((from, from_t, dur, elapsed)) = anim.blend.as_mut() {
            *elapsed += dt;
            let w = (1.0 - *elapsed / *dur).clamp(0.0, 1.0);
            if w > 0.0 {
                let from_t = from_t.min(from.duration);
                let old = from.sample(from_t, &rig.skeleton);
                for (p, o) in pose.iter_mut().zip(&old) {
                    *p = p.lerp(o, w);
                }
            }
        }
        if anim.blend.as_ref().is_some_and(|(_, _, d, e)| e >= d) {
            anim.blend = None;
        }
        let mut model = rig.skeleton.model_space(&pose);
        apply_walk_twist(&rig.skeleton, &mut model, anim.walk_twist);
        for (i, joint) in rig.joints.iter().enumerate() {
            let Some((entity, parent)) = joint else {
                continue;
            };
            let local = match parent {
                Some(p) => model[*p].inverse() * model[i],
                None => model[i],
            };
            if let Ok(mut tf) = joints.get_mut(*entity) {
                *tf = Transform::from_matrix(local);
            }
        }
        rig.model = model;
    }
}

/// The locked-on walk twist on a model-space pose, as Wolf's behaviour graph does it (its
/// `TwistMaster` and `TwistRootRotYCancel` twist modifiers, bound to `TwistMasterAngle`; facts
/// from FwLo's RustSekiro, `docs/kb/movement.md`): the Master bone, and so the whole body, turns
/// by `degrees` about the vertical (positive turns the legs right), then Spine, Spine1 and
/// Spine2 each turn back by a third about their own joints, so the legs step along the move
/// while the chest, arms and head keep facing the lock-on target.
fn apply_walk_twist(skeleton: &Skeleton, model: &mut [Mat4], degrees: f32) {
    if degrees.abs() < 0.01 {
        return;
    }
    let find = |name: &str| skeleton.bones.iter().position(|b| b.name == name);
    let Some(master) = find("Master") else {
        return;
    };
    // Bevy model space: forward is -Z and a positive turn about +Y goes left.
    let angle = degrees.to_radians();
    turn_subtree(skeleton, model, master, Mat4::from_rotation_y(-angle));
    let spine = ["Spine", "Spine1", "Spine2"].map(find);
    let share = angle / spine.len() as f32;
    for bone in spine.into_iter().flatten() {
        let pivot = model[bone].w_axis.truncate();
        let back = Mat4::from_translation(pivot)
            * Mat4::from_rotation_y(share)
            * Mat4::from_translation(-pivot);
        turn_subtree(skeleton, model, bone, back);
    }
}

/// Applies the model-space transform `by` to bone `root` and everything under it (parents
/// always precede their children in the skeleton).
fn turn_subtree(skeleton: &Skeleton, model: &mut [Mat4], root: usize, by: Mat4) {
    let mut inside = vec![false; skeleton.bones.len()];
    for (i, bone) in skeleton.bones.iter().enumerate().skip(root) {
        inside[i] = i == root || bone.parent.is_some_and(|p| inside[p as usize]);
        if inside[i]
            && let Some(m) = model.get_mut(i)
        {
            *m = by * *m;
        }
    }
}

fn clip_name_of(clip: &AnimClip) -> String {
    clip.name.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With the legs twisted 45 degrees right, the hips turn right with them while the
    /// shoulders still face where they did.
    #[test]
    fn walk_twist_turns_the_hips_and_keeps_the_shoulders() {
        let cache = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cache");
        let Ok(lib) = AnimLibrary::load(&cache, "c0000") else {
            eprintln!("skipping: c0000 animations are not extracted");
            return;
        };
        let skeleton = &lib.skeleton;
        let at = |model: &[Mat4], name: &str| {
            let i = skeleton.bones.iter().position(|b| b.name == name).unwrap();
            model[i].w_axis.truncate()
        };
        let across = |model: &[Mat4], left: &str, right: &str| {
            let v = at(model, right) - at(model, left);
            Vec3::new(v.x, 0.0, v.z).normalize()
        };
        let still = skeleton.model_space(&[]);
        let mut twisted = still.clone();
        apply_walk_twist(skeleton, &mut twisted, 45.0);
        // Facing -Z, the right side is +X; turning right by 45 degrees is -45 about +Y.
        let side = across(&still, "L_Thigh", "R_Thigh");
        assert!(
            side.x > 0.9,
            "the model does not face -Z: right thigh is {side:?} of the left"
        );
        let right = Quat::from_rotation_y(-45f32.to_radians());
        let hips = across(&twisted, "L_Thigh", "R_Thigh");
        let want = right * across(&still, "L_Thigh", "R_Thigh");
        assert!(hips.dot(want) > 0.99, "hips {hips:?}, want {want:?}");
        let shoulders = across(&twisted, "L_UpperArm", "R_UpperArm");
        let before = across(&still, "L_UpperArm", "R_UpperArm");
        assert!(
            shoulders.dot(before) > 0.99,
            "shoulders {shoulders:?}, were {before:?}"
        );
    }
}

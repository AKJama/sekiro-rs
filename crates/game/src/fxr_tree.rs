//! The FXR node tree at run time: nodes, emitters, particles and property sampling.
//!
//! Adapted from FwLo's RustSekiro `src/fxr.rs` (MIT, see `CREDITS.md`), which plays effects from
//! the named tree of EvenTorset's FXR library (Unlicense; the same tree
//! [`sekiro_formats::fxr_json`] builds from our own reader). Semantics follow that library's
//! action definitions: the node tree (2000 root, 2200 basic nodes, 2001 references, 2202 levels
//! of detail), configs (1004 particles, 1005 child nodes), emitters (399 once, 300 periodic, 301
//! by distance), emitter shapes (400-405), spread (500-503), particle movement
//! (55/60/84/105/64/65), node movement (1/15/34/83/106/120/122), and properties over
//! Constant0 / ParticleAge / EmissionTime / ActiveTime with Linear / Stepped / Hermite / Bezier
//! keyframes and RandomDelta / RandomFraction / RandomRange modifiers. Hermite keyframes blend as
//! the exe does (`game_hermite`, traced by FwLo).
//!
//! The tree is data only; drawing lives in `fxr.rs`.

use bevy::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

// ---------------------------------------------------------------- properties

/// Deterministic per-particle randomness: the k-th random number of a seed.
#[derive(Clone, Copy)]
pub struct Rnd {
    pub seed: u32,
    pub k: u32,
}

impl Rnd {
    pub fn next(&mut self) -> f32 {
        self.k = self.k.wrapping_add(1);
        let mut x = self.seed ^ self.k.wrapping_mul(0x9E37_79B9);
        x ^= x >> 16;
        x = x.wrapping_mul(0x7feb_352d);
        x ^= x >> 15;
        x = x.wrapping_mul(0x846c_a68b);
        x ^= x >> 16;
        (x % 1_000_000) as f32 / 1_000_000.0
    }
}

pub fn num(v: &Value) -> f32 {
    v.as_f64().unwrap_or(0.0) as f32
}

/// A value as up to 4 components.
fn vec4(v: &Value) -> [f32; 4] {
    match v {
        Value::Array(a) => {
            let mut o = [0.0; 4];
            for (i, x) in a.iter().take(4).enumerate() {
                o[i] = num(x);
            }
            if a.len() == 1 {
                o = [o[0]; 4];
            }
            o
        }
        Value::Bool(b) => [if *b { 1.0 } else { 0.0 }; 4],
        _ => [num(v); 4],
    }
}

fn lerp4(a: [f32; 4], b: [f32; 4], k: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * k[i])
}

/// The game's Hermite keyframe blend (FXR property function 6): `x` is the 0..1 position between
/// the two keyframes, `t1` / `t2` the earlier keyframe's two angles (radians). Two fixed Hermite
/// curves bent into circle arcs, an ease-in `lo` and an ease-out `hi`; each angle blends from one
/// of them (angle 0) through the straight line (pi/4) to the other (pi/2), and the two results are
/// blended over x.
fn game_hermite(t1: f32, t2: f32, x: f32) -> f32 {
    const Q: f32 = std::f32::consts::FRAC_PI_4;
    let h = |m0: f32, m1: f32| {
        let (x2, x3) = (x * x, x * x * x);
        (x3 - 2.0 * x2 + x) * m0 + (-2.0 * x3 + 3.0 * x2) + (x3 - x2) * m1
    };
    let lo = 1.0 - (1.0 - h(0.7, 1.5).powi(2)).max(0.0).sqrt();
    let hi = (1.0 - (h(1.5, 0.7) - 1.0).powi(2)).max(0.0).sqrt();
    let s = if t1 <= Q {
        lo + t1 / Q * (x - lo)
    } else {
        x + (t1 - Q) / Q * (hi - x)
    };
    let e = if t2 <= Q {
        hi + t2 / Q * (x - hi)
    } else {
        x + (t2 - Q) / Q * (lo - x)
    };
    s + x * (e - s)
}

/// A property at argument `t`, with its modifiers drawn from `rnd`.
pub fn prop(v: Option<&Value>, t: f32, rnd: &mut Rnd, default: [f32; 4]) -> [f32; 4] {
    let Some(v) = v else { return default };
    if v.is_null() {
        return default;
    }
    let base = match v.get("function").and_then(Value::as_str) {
        Some(func) => {
            let kf = v["keyframes"]
                .as_array()
                .map(|a| a.as_slice())
                .unwrap_or(&[]);
            if kf.is_empty() {
                default
            } else {
                let last = num(&kf[kf.len() - 1]["position"]);
                let t = if v["loop"].as_bool() == Some(true) && last > 0.0 {
                    t.rem_euclid(last)
                } else {
                    t
                };
                let i = kf.iter().rposition(|k| num(&k["position"]) <= t);
                match i {
                    None => vec4(&kf[0]["value"]),
                    Some(i) if i + 1 >= kf.len() => vec4(&kf[i]["value"]),
                    Some(i) => {
                        let (a, b) = (&kf[i], &kf[i + 1]);
                        let (pa, pb) = (num(&a["position"]), num(&b["position"]));
                        let x = if pb > pa { (t - pa) / (pb - pa) } else { 1.0 };
                        let (va, vb) = (vec4(&a["value"]), vec4(&b["value"]));
                        match func {
                            "Stepped" => va,
                            "Hermite" => {
                                let (t1, t2) = (vec4(&a["t1"]), vec4(&a["t2"]));
                                let k: [f32; 4] =
                                    std::array::from_fn(|c| game_hermite(t1[c], t2[c], x));
                                lerp4(va, vb, k)
                            }
                            "Bezier" => {
                                let (p1, p2) = (vec4(&a["p1"]), vec4(&b["p2"]));
                                let cubic = |v1: f32, c1: f32, c2: f32, v2: f32| {
                                    let u = 1.0 - x;
                                    u * u * u * v1
                                        + 3.0 * u * u * x * c1
                                        + 3.0 * u * x * x * c2
                                        + x * x * x * v2
                                };
                                std::array::from_fn(|c| {
                                    cubic(va[c], va[c] + p1[c] / 3.0, vb[c] - p2[c] / 3.0, vb[c])
                                })
                            }
                            _ => lerp4(va, vb, [x; 4]),
                        }
                    }
                }
            }
        }
        None => match v.get("value") {
            Some(inner) => vec4(inner),
            None => vec4(v),
        },
    };
    let mut out = base;
    if let Some(mods) = v.get("modifiers").and_then(Value::as_array) {
        for m in mods {
            let max = vec4(&m["max"]);
            match m["type"].as_str() {
                Some("RandomDelta") => {
                    for c in 0..4 {
                        out[c] += (rnd.next() * 2.0 - 1.0) * max[c];
                    }
                }
                Some("RandomFraction") => {
                    for c in 0..4 {
                        out[c] += out[c] * (rnd.next() * 2.0 - 1.0) * max[c];
                    }
                }
                Some("RandomRange") => {
                    let min = vec4(&m["min"]);
                    for c in 0..4 {
                        out[c] += min[c] + (max[c] - min[c]) * rnd.next();
                    }
                }
                _ => {}
            }
        }
    }
    out
}

pub fn scalar(a: &Value, key: &str, t: f32, rnd: &mut Rnd, default: f32) -> f32 {
    prop(a.get(key), t, rnd, [default; 4])[0]
}

pub fn int(a: &Value, key: &str, default: i64) -> i64 {
    match a.get(key) {
        Some(Value::Number(n)) => n
            .as_i64()
            .unwrap_or(n.as_f64().unwrap_or(default as f64) as i64),
        Some(Value::Bool(b)) => *b as i64,
        Some(v) => v
            .get("value")
            .and_then(Value::as_f64)
            .map_or(default, |f| f as i64),
        None => default,
    }
}

// ---------------------------------------------------------------- game space

/// Game (FromSoft) space -> ours: X mirrored; a rotation's Y and Z angles flip with it. FXR
/// rotations are degrees, applied Z -> X -> Y.
pub fn game_vec(v: [f32; 4]) -> Vec3 {
    Vec3::new(-v[0], v[1], v[2])
}

pub fn game_rot(r: [f32; 4]) -> Quat {
    let (x, y, z) = (r[0].to_radians(), -r[1].to_radians(), -r[2].to_radians());
    Quat::from_rotation_y(y) * Quat::from_rotation_x(x) * Quat::from_rotation_z(z)
}

// ---------------------------------------------------------------- library

/// Installed effects by id: parsed with our own reader and converted to the named tree on first
/// use, plus the texture files of every effect archive.
#[derive(Default)]
pub struct FxrLib {
    files: HashMap<i64, PathBuf>,
    textures: HashMap<i64, PathBuf>,
    cache: HashMap<i64, Option<Arc<Value>>>,
}

impl FxrLib {
    /// Indexes every `sfxbnd_*.ffxbnd.d` folder under `<cache>/raw/sfx` (the common effects first:
    /// an id in several areas comes from the first).
    pub fn scan(cache: &std::path::Path) -> Self {
        let mut lib = FxrLib::default();
        let Ok(dirs) = std::fs::read_dir(cache.join("raw/sfx")) else {
            return lib;
        };
        let mut dirs: Vec<_> = dirs
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();
        for dir in dirs {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                match path.extension().and_then(|e| e.to_str()) {
                    Some("fxr") => {
                        if let Some(id) = stem.strip_prefix('f').and_then(|s| s.parse::<i64>().ok())
                        {
                            lib.files.entry(id).or_insert(path);
                        }
                    }
                    Some("tpf") => {
                        if let Some(id) = stem.strip_prefix('s').and_then(|s| s.parse::<i64>().ok())
                        {
                            lib.textures.entry(id).or_insert(path);
                        }
                    }
                    _ => {}
                }
            }
        }
        lib
    }

    pub fn effect_count(&self) -> usize {
        self.files.len()
    }

    pub fn texture_path(&self, id: i64) -> Option<&PathBuf> {
        self.textures.get(&id)
    }

    /// The effect's tree; `None` when it is not installed or does not parse.
    pub fn get(&mut self, id: i64) -> Option<Arc<Value>> {
        if let Some(hit) = self.cache.get(&id) {
            return hit.clone();
        }
        let tree = self
            .files
            .get(&id)
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| sekiro_formats::fxr::parse(&b).ok())
            .map(|fxr| sekiro_formats::fxr_json::to_json(&fxr).0)
            .map(Arc::new);
        self.cache.insert(id, tree.clone());
        tree
    }

    pub fn has(&mut self, id: i64) -> bool {
        self.get(id).is_some()
    }

    pub fn uses_action(&mut self, id: i64, slot: &str, action: i64) -> bool {
        fn walk(lib: &mut FxrLib, node: &Value, slot: &str, action: i64, depth: usize) -> bool {
            if depth > 12 {
                return false;
            }
            if node["configs"]
                .as_array()
                .is_some_and(|cs| cs.iter().any(|c| c[slot]["type"].as_i64() == Some(action)))
            {
                return true;
            }
            if let Some(id) = node["sfx"].as_i64()
                && let Some(tree) = lib.get(id)
                && walk(lib, &tree["root"], slot, action, depth + 1)
            {
                return true;
            }
            node["nodes"]
                .as_array()
                .is_some_and(|cs| cs.iter().any(|n| walk(lib, n, slot, action, depth + 1)))
        }
        self.get(id)
            .is_some_and(|tree| walk(self, &tree["root"], slot, action, 0))
    }

    /// Whether the effect (or an effect it references) draws a model (appearance 605), which the
    /// player does not draw yet: a stand-in for the model's own subject should stay.
    pub fn has_model(&mut self, id: i64) -> bool {
        fn walk(lib: &mut FxrLib, node: &Value, depth: u32) -> bool {
            if depth > 8 {
                return false;
            }
            let own = node["configs"].as_array().is_some_and(|cs| {
                cs.iter()
                    .any(|c| c["appearance"]["type"].as_i64() == Some(605))
            });
            if own {
                return true;
            }
            if let Some(r) = node["sfx"].as_i64()
                && let Some(tree) = lib.get(r)
                && walk(lib, &tree["root"], depth + 1)
            {
                return true;
            }
            node["nodes"]
                .as_array()
                .is_some_and(|kids| kids.iter().any(|k| walk(lib, k, depth + 1)))
        }
        let Some(tree) = self.get(id) else {
            return false;
        };
        walk(self, &tree["root"], 0)
    }

    /// Installs a tree under `id` without a file (tests).
    #[cfg(test)]
    pub fn insert_for_test(&mut self, id: i64, tree: Value) {
        self.cache.insert(id, Some(Arc::new(tree)));
    }
}

// ---------------------------------------------------------------- runtime

pub struct Particle {
    /// World position (attachment None) or position in the node's frame.
    pub pos: Vec3,
    pub dir: Vec3,
    pub speed: f32,
    pub age: f32,
    pub life: f32,
    pub emit_t: f32,
    pub seed: u32,
    pub attached: bool,
    /// Tracer points (world position, source axis - zero = by travel - and age at creation).
    pub trail: Vec<(Vec3, Vec3, f32)>,
    pub next_seg: f32,
    pub dead: bool,
    /// A tracer particle whose life (or effect) ended: it no longer moves or adds points, its
    /// segments age out over segmentDuration, then it goes.
    pub ended: bool,
}

pub struct NodeInst {
    /// Active config (stateConfigMap[0]).
    pub cfg: Option<Arc<Value>>,
    pub local: Transform,
    pub world: Transform,
    pub time: f32,
    pub delay: f32,
    pub duration: f32,
    emit_next: f32,
    emissions: i64,
    last_emit_at: Option<Vec3>,
    pub particles: Vec<Particle>,
    pub children: Vec<NodeInst>,
    /// For node emitters (1005): templates to spawn per emission.
    templates: Vec<Arc<Value>>,
    seed: u32,
    pub finished: bool,
    /// NodeAccelerationPartialFollow / NodeSpeedPartialFollow followFactor fell to 0: the node
    /// stays where it was in the world.
    detached: Option<Transform>,
    /// NodeTranslation (15): the offset applied so far.
    offset: Vec3,
}

fn config_of(node: &Value) -> Option<Arc<Value>> {
    let i = node["stateConfigMap"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    node["configs"]
        .as_array()
        .and_then(|c| c.get(i))
        .cloned()
        .map(Arc::new)
}

pub fn make_node(lib: &mut FxrLib, node: &Value, seed: u32, depth: u32) -> Option<NodeInst> {
    if depth > 12 {
        return None;
    }
    let ty = node["type"].as_i64().unwrap_or(0);
    // ReferenceNode: another effect's root.
    if ty == 2001 {
        let r = lib.get(node["sfx"].as_i64().unwrap_or(-1))?;
        return make_node(lib, &r["root"], seed ^ 0x55, depth + 1);
    }
    let cfg = config_of(node);
    let mut rnd = Rnd { seed, k: 0 };
    let (mut local, mut delay, mut duration) = (Transform::IDENTITY, 0.0, -1.0);
    if let Some(c) = &cfg {
        let attrs = &c["nodeAttributes"];
        delay = scalar(attrs, "delay", 0.0, &mut rnd, 0.0);
        duration = scalar(attrs, "duration", 0.0, &mut rnd, -1.0);
        let tr = &c["nodeTransform"];
        let mut off = vec4(&tr["offset"]);
        let mut rot = vec4(&tr["rotation"]);
        if tr["type"].as_i64() == Some(36) {
            let (ov, rv) = (vec4(&tr["offsetVariance"]), vec4(&tr["rotationVariance"]));
            for i in 0..3 {
                off[i] += (rnd.next() * 2.0 - 1.0) * ov[i];
                rot[i] += (rnd.next() * 2.0 - 1.0) * rv[i];
            }
        }
        // The pitch sign of a node's authored rotation is the inverse of `game_rot`'s: designers
        // rotate +90 about X to stand a Z-axis emitter upright (64 of the rising disc/point
        // nodes in the common effects), and a +X rotation that sent Z downward would bury the
        // brazier sparks (+45) and the ground-fire smoke (+90) in the floor.
        rot[0] = -rot[0];
        local = Transform::from_translation(game_vec(off)).with_rotation(game_rot(rot));
    }
    let is_node_emitter = cfg
        .as_ref()
        .is_some_and(|c| c["type"].as_i64() == Some(1005));
    let kids: Vec<Value> = node["nodes"].as_array().cloned().unwrap_or_default();
    // Levels of detail: the closest (first) child only.
    let kids: Vec<Value> = if ty == 2202 {
        kids.into_iter().take(1).collect()
    } else {
        kids
    };
    let (children, templates) = if is_node_emitter {
        (Vec::new(), kids.into_iter().map(Arc::new).collect())
    } else {
        (
            kids.iter()
                .enumerate()
                .filter_map(|(i, k)| {
                    make_node(
                        lib,
                        k,
                        seed.wrapping_mul(31).wrapping_add(i as u32 + 1),
                        depth + 1,
                    )
                })
                .collect(),
            Vec::new(),
        )
    };
    Some(NodeInst {
        cfg,
        local,
        world: Transform::IDENTITY,
        time: 0.0,
        delay,
        duration,
        emit_next: 0.0,
        emissions: 0,
        last_emit_at: None,
        particles: Vec::new(),
        children,
        templates,
        seed,
        finished: false,
        detached: None,
        offset: Vec3::ZERO,
    })
}

/// A direction inside a cone of `angle` degrees around `n`.
fn spread_dir(n: Vec3, angle: f32, rnd: &mut Rnd) -> Vec3 {
    if angle <= 0.0 {
        return n;
    }
    let (t, b) = n.any_orthonormal_pair();
    let phi = rnd.next() * std::f32::consts::TAU;
    let cos_max = angle.to_radians().cos();
    let cos_t = 1.0 - rnd.next() * (1.0 - cos_max);
    let sin_t = (1.0 - cos_t * cos_t).max(0.0).sqrt();
    (n * cos_t + (t * phi.cos() + b * phi.sin()) * sin_t).normalize_or(n)
}

/// Emission point and direction in the node's frame, from its emitter shape and spread.
fn emit_point(c: &Value, t: f32, rnd: &mut Rnd, frame: Quat) -> (Vec3, Vec3) {
    let shape = &c["emitterShape"];
    let st = shape["type"].as_i64().unwrap_or(400);
    let dirmode = int(shape, "direction", 0);
    let s = |k: &str, d: f32, rnd: &mut Rnd| scalar(shape, k, t, rnd, d);
    let (pos, outward) = match st {
        401 => {
            let r = s("radius", 1.0, rnd) * rnd.next().sqrt();
            let a = rnd.next() * std::f32::consts::TAU;
            let p = Vec3::new(a.cos() * r, a.sin() * r, 0.0);
            (p, p.normalize_or(Vec3::Z))
        }
        403 => {
            let r = s("radius", 1.0, rnd);
            let d = Vec3::new(
                rnd.next() * 2.0 - 1.0,
                rnd.next() * 2.0 - 1.0,
                rnd.next() * 2.0 - 1.0,
            )
            .normalize_or(Vec3::Y);
            let inside = shape["emitInside"].as_bool().unwrap_or(true);
            (d * if inside { r * rnd.next().cbrt() } else { r }, d)
        }
        404 => {
            let (x, y, z) = (
                s("sizeX", 1.0, rnd),
                s("sizeY", 1.0, rnd),
                s("sizeZ", 1.0, rnd),
            );
            let p = Vec3::new(
                (rnd.next() - 0.5) * x,
                (rnd.next() - 0.5) * y,
                (rnd.next() - 0.5) * z,
            );
            (game_vec([p.x, p.y, p.z, 0.0]), Vec3::Z)
        }
        402 => {
            let (x, y) = (s("sizeX", 1.0, rnd), s("sizeY", 1.0, rnd));
            (
                Vec3::new((rnd.next() - 0.5) * x, (rnd.next() - 0.5) * y, 0.0),
                Vec3::Z,
            )
        }
        405 => {
            let (r, h) = (s("radius", 1.0, rnd), s("height", 1.0, rnd));
            let a = rnd.next() * std::f32::consts::TAU;
            let inside = shape["emitInside"].as_bool().unwrap_or(true);
            let rr = if inside { r * rnd.next().sqrt() } else { r };
            let y_axis = shape["yAxis"].as_bool().unwrap_or(true);
            let radial = Vec3::new(a.cos(), 0.0, a.sin());
            let along = (rnd.next() - 0.5) * h;
            if y_axis {
                (radial * rr + Vec3::Y * along, radial)
            } else {
                (
                    Vec3::new(a.cos() * rr, a.sin() * rr, along),
                    Vec3::new(a.cos(), a.sin(), 0.0),
                )
            }
        }
        _ => (Vec3::ZERO, Vec3::Z),
    };
    // InitialDirection: 0 by shape, 1 / 4 up, 2 / 5 down, 3 forward, 6 world up (the 6 was
    // read from the data: see docs/MAP-FX.md, flame height).
    // 6 is world up: the torch and brazier sparks (gravity -1, a 5.6 degree cone, no node
    // rotation) and flame cores of the common effects only rise if it is, wherever the node
    // is tilted.
    let base = match dirmode {
        1 | 4 => Vec3::Y,
        2 | 5 => Vec3::NEG_Y,
        6 => frame.inverse() * Vec3::Y,
        3 => Vec3::Z,
        _ => outward,
    };
    let sp = &c["directionSpread"];
    let dir = match sp["type"].as_i64().unwrap_or(500) {
        501 => spread_dir(base, scalar(sp, "angle", t, rnd, 30.0), rnd),
        502 | 503 => spread_dir(
            base,
            scalar(sp, "angleX", t, rnd, 30.0).max(scalar(sp, "angleY", t, rnd, 30.0)),
            rnd,
        ),
        _ => base,
    };
    (pos, dir)
}

/// Advances one node (and its children) by `dt`; `parent` is the parent's world transform.
pub fn step_node(n: &mut NodeInst, lib: &mut FxrLib, parent: Transform, dt: f32, stop: bool) {
    let previous = n.world.translation;
    n.world = match n.detached {
        Some(w) => w * n.local,
        None => parent * n.local,
    };
    if n.time < n.delay {
        n.time += dt;
        for c in &mut n.children {
            step_node(c, lib, n.world, dt, stop);
        }
        return;
    }
    let active_t = n.time - n.delay;
    let alive = !stop && (n.duration < 0.0 || active_t < n.duration);
    if let Some(c) = n.cfg.clone() {
        if c["appearance"]["type"].as_i64() == Some(10000) {
            weather(n, &c["appearance"], dt, alive, previous);
            n.time += dt;
            n.finished = !alive && n.particles.is_empty();
            return;
        }
        let mv = &c["nodeMovement"];
        let mut rnd = Rnd {
            seed: n.seed ^ 0xabc,
            k: 0,
        };
        match mv["type"].as_i64() {
            Some(1) | Some(106) | Some(122) | Some(83) | Some(120) => {
                let ty = mv["type"].as_i64();
                let v = if ty == Some(120) {
                    scalar(mv, "speedZ", 0.0, &mut rnd, 0.0)
                        * scalar(mv, "speedMultiplierZ", active_t, &mut rnd, 1.0)
                } else {
                    scalar(mv, "speedZ", 0.0, &mut rnd, 0.0)
                        + scalar(mv, "accelerationZ", active_t, &mut rnd, 0.0)
                            * scalar(mv, "accelerationMultiplierZ", active_t, &mut rnd, 1.0)
                            * active_t
                };
                // 83: a random turn of up to maxTurnAngle every turnInterval s.
                let every = scalar(mv, "turnInterval", active_t, &mut rnd, 0.0);
                if ty == Some(83)
                    && every > 0.0
                    && ((active_t / every).floor() > ((active_t - dt) / every).floor())
                {
                    let max = scalar(mv, "maxTurnAngle", active_t, &mut rnd, 0.0).to_radians();
                    let mut tr = Rnd {
                        seed: n.seed ^ ((active_t / every) as u32).wrapping_mul(0x9e37),
                        k: 0,
                    };
                    let (yaw, pitch) =
                        ((tr.next() * 2.0 - 1.0) * max, (tr.next() * 2.0 - 1.0) * max);
                    n.local.rotation *= Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
                }
                let fwd = n.local.rotation * Vec3::Z;
                n.local.translation += fwd * v * dt;
                n.local.translation.y -=
                    scalar(mv, "accelerationY", active_t, &mut rnd, 0.0) * active_t * dt;
            }
            Some(15) => {
                let o = game_vec(prop(mv.get("translation"), active_t, &mut rnd, [0.0; 4]));
                n.local.translation += o - n.offset;
                n.offset = o;
            }
            Some(34) => {
                let s =
                    |k: &str, rnd: &mut Rnd| scalar(mv, k, active_t, rnd, 0.0).to_radians() * dt;
                let (x, y, z) = (
                    s("angularSpeedX", &mut rnd),
                    -s("angularSpeedY", &mut rnd),
                    -s("angularSpeedZ", &mut rnd),
                );
                n.local.rotation *= Quat::from_euler(EulerRot::YXZ, y, x, z);
            }
            _ => {}
        }
        // followFactor (106 / 122): at 0 the node lets go of its parent and stays in the world.
        if matches!(mv["type"].as_i64(), Some(106) | Some(122))
            && n.detached.is_none()
            && mv.get("followFactor").is_some()
            && scalar(mv, "followFactor", active_t, &mut rnd, 0.0) <= 0.0
        {
            n.detached = Some(parent);
        }
        n.world = match n.detached {
            Some(w) => w * n.local,
            None => parent * n.local,
        };
        if alive {
            emit(n, &c, active_t, &mut rnd, lib);
        }
        move_particles(n, &c, dt, stop, alive);
    }
    n.time += dt;
    let world = n.world;
    for ch in &mut n.children {
        step_node(ch, lib, world, dt, stop || !alive);
    }
    n.children.retain(|c| !c.finished);
    let emitting = alive && n.cfg.is_some();
    n.finished = !emitting
        && n.particles.is_empty()
        && n.children.is_empty()
        && (stop || n.duration >= 0.0 && n.time - n.delay >= n.duration || n.cfg.is_none());
}

/// CPU fallback for GPUStandardParticle. Integer durations/intervals use the FXR 30 Hz clock.
fn weather(n: &mut NodeInst, a: &Value, dt: f32, alive: bool, previous: Vec3) {
    for p in &mut n.particles {
        let mut rnd = Rnd { seed: p.seed, k: 0 };
        let follow = scalar(a, "particleFollowFactor", p.age, &mut rnd, 0.);
        if n.time > 0. {
            p.pos += (n.world.translation - previous) * follow;
        }
        p.age += dt;
        p.pos += p.dir * p.speed * dt;
    }
    n.particles.retain(|p| p.age < p.life);
    let mut rnd = Rnd {
        seed: n.seed.wrapping_add(n.emissions as u32),
        k: 0,
    };
    let mut guard = 0;
    while alive && n.emit_next <= n.time && guard < 4 {
        let count = int(a, "emissionParticleCount", 0).max(0) as usize;
        let size = game_vec(prop(a.get("emitterSize"), 0., &mut rnd, [1.; 4]));
        let rotation = game_rot(prop(a.get("emitterRotation"), 0., &mut rnd, [0.; 4]));
        for i in 0..count.min(2048usize.saturating_sub(n.particles.len())) {
            let seed = n
                .seed
                .wrapping_add((n.emissions as u32).wrapping_mul(4001))
                .wrapping_add(i as u32);
            let mut r = Rnd { seed, k: 0 };
            let spread = Vec3::new(r.next() - 0.5, r.next() - 0.5, r.next() - 0.5) * size;
            let vmin = game_vec(prop(a.get("particleSpeedMin"), 0., &mut r, [0.; 4]));
            let vmax = game_vec(prop(a.get("particleSpeedMax"), 0., &mut r, [0.; 4]));
            let velocity = game_vec(prop(a.get("particleSpeed"), 0., &mut r, [0.; 4]))
                + vmin
                + (vmax - vmin) * Vec3::new(r.next(), r.next(), r.next());
            let life = int(a, "particleDuration", 30).max(1) as f32 / 30.
                * scalar(a, "particleDurationMultiplier", 0., &mut r, 1.);
            n.particles.push(Particle {
                pos: n.world.translation + rotation * spread,
                dir: velocity.normalize_or(Vec3::NEG_Y),
                speed: velocity.length(),
                age: 0.,
                life,
                emit_t: n.time,
                seed,
                attached: false,
                trail: Vec::new(),
                next_seg: 0.,
                dead: false,
                ended: false,
            });
        }
        n.emissions += 1;
        let interval = int(a, "emissionIntervalMin", 0).max(1) as f32 / 30.;
        n.emit_next += interval;
        guard += 1;
    }
}

fn emit(n: &mut NodeInst, c: &Arc<Value>, active_t: f32, rnd: &mut Rnd, lib: &mut FxrLib) {
    let em = &c["emitter"];
    let mut count = 0i64;
    let total = int(em, "totalEmissions", -1);
    match em["type"].as_i64().unwrap_or(399) {
        399 => {
            if n.emissions == 0 {
                count = 1;
            }
        }
        300 => {
            let interval = scalar(em, "interval", active_t, rnd, 1.0).max(0.0);
            let per = scalar(em, "perEmission", active_t, rnd, 1.0)
                .max(0.0)
                .round() as i64;
            let mut guard = 0;
            while n.emit_next <= active_t && guard < 8 && (total < 0 || n.emissions < total) {
                count += per;
                n.emissions += 1;
                n.emit_next += interval.max(1.0 / 60.0);
                guard += 1;
            }
        }
        301 => {
            let th = scalar(em, "threshold", active_t, rnd, 0.1).max(0.01);
            let here = n.world.translation;
            match n.last_emit_at {
                None => {
                    n.last_emit_at = Some(here);
                    count = 1;
                }
                Some(p) if p.distance(here) >= th => {
                    n.last_emit_at = Some(here);
                    count = 1;
                }
                _ => {}
            }
        }
        _ => {}
    }
    if em["type"].as_i64().unwrap_or(399) == 399 && count > 0 {
        n.emissions = 1;
    }
    let max_conc = int(em, "maxConcurrent", -1);
    for i in 0..count {
        if max_conc > 0 && (n.particles.len() + n.children.len()) as i64 >= max_conc {
            break;
        }
        // A bounded pool: runaway emitters never grow without limit.
        if n.particles.len() >= 2048 {
            break;
        }
        let pseed = n
            .seed
            .wrapping_mul(2_654_435_761)
            .wrapping_add((n.emissions * 64 + i) as u32);
        let mut prnd = Rnd { seed: pseed, k: 0 };
        let active = active_t;
        let (p, d) = emit_point(c, active, &mut prnd, n.world.rotation);
        if !n.templates.is_empty() {
            // NodeEmitter: child nodes at the emission point (200 all / 201 one at random).
            let pick: Vec<Arc<Value>> = if c["nodeSelector"]["type"].as_i64() == Some(201) {
                vec![
                    n.templates
                        [(prnd.next() * n.templates.len() as f32) as usize % n.templates.len()]
                    .clone(),
                ]
            } else {
                n.templates.clone()
            };
            for t in pick {
                if let Some(mut child) = make_node(lib, &t, pseed ^ 0x77, 1) {
                    let rot = Quat::from_rotation_arc(Vec3::Z, d.normalize_or(Vec3::Z));
                    child.local = Transform::from_translation(p).with_rotation(rot) * child.local;
                    n.children.push(child);
                }
            }
            continue;
        }
        let pa = &c["particleAttributes"];
        let life = scalar(pa, "duration", 0.0, &mut prnd, -1.0);
        let attached = int(pa, "attachment", 1) != 0;
        let world_p = n.world.transform_point(p);
        let world_d = (n.world.rotation * d).normalize_or(Vec3::Z);
        n.particles.push(Particle {
            pos: if attached { p } else { world_p },
            dir: if attached { d } else { world_d },
            speed: scalar(&c["particleModifier"], "speed", active_t, &mut prnd, 0.0),
            age: 0.0,
            life,
            emit_t: active_t,
            seed: pseed,
            attached,
            trail: Vec::new(),
            next_seg: 0.0,
            dead: false,
            ended: false,
        });
    }
}

fn move_particles(n: &mut NodeInst, c: &Arc<Value>, dt: f32, stop: bool, alive: bool) {
    let pm = &c["particleMovement"];
    let pmt = pm["type"].as_i64().unwrap_or(0);
    let world_rot_inv = n.world.rotation.inverse();
    for p in &mut n.particles {
        p.age += dt;
        if p.ended {
            p.dead = p.trail.is_empty();
            continue;
        }
        // A negative particle duration (-1, or the common -1/30) lives as long as its node.
        if p.life >= 0.0 && p.age > p.life || (stop || !alive) && p.life < 0.0 {
            p.ended = p.trail.len() > 1;
            p.dead = !p.ended;
            continue;
        }
        let mut r = Rnd {
            seed: p.seed ^ 0x5151,
            k: 0,
        };
        let gravity = scalar(pm, "gravity", p.age, &mut r, 0.0);
        let mut vel = p.dir * p.speed;
        match pmt {
            60 | 64 | 65 => {
                let s = scalar(pm, "speed", p.age, &mut r, 0.0)
                    * scalar(pm, "speedMultiplier", p.age, &mut r, 1.0);
                vel = p.dir * (p.speed + s);
            }
            55 | 84 | 105 => {
                let a = scalar(pm, "acceleration", p.age, &mut r, 0.0)
                    * scalar(pm, "accelerationMultiplier", p.age, &mut r, 1.0);
                p.speed += a * dt;
                vel = p.dir * p.speed;
            }
            _ => {}
        }
        // Gravity (positive = down) bends the direction.
        if gravity != 0.0 {
            let g = if p.attached {
                world_rot_inv * Vec3::NEG_Y
            } else {
                Vec3::NEG_Y
            };
            let v = vel + g * gravity * dt;
            let s = v.length();
            if s > 1e-5 {
                p.dir = v / s;
                p.speed = s - if matches!(pmt, 60 | 64 | 65) {
                    scalar(pm, "speed", p.age, &mut r, 0.0)
                } else {
                    0.0
                };
            }
            vel = v;
        }
        p.pos += vel * dt;
    }
    n.particles.retain(|p| !p.dead);
}

/// Tracer segments: a new point every segmentInterval, each kept segmentDuration.
pub fn grow_trails(n: &mut NodeInst) {
    if let Some(c) = n.cfg.clone() {
        let a = &c["appearance"];
        if matches!(a["type"].as_i64(), Some(606) | Some(10012)) {
            let interval = num(&a["segmentInterval"]).max(1.0 / 60.0);
            let seg_life = num(&a["segmentDuration"]).max(0.02);
            let max_segs = a["concurrentSegments"].as_i64().unwrap_or(100).max(2) as usize;
            // TracerOrientationMode: 0 Travel (by the path and the camera, at draw time), 1 the
            // node's local Z, 2 global vertical, 3 global X, 4 cross (vertical here, X added at
            // draw), 5 the global diagonal.
            let axis = match int(a, "orientation", 0) {
                1 => n.world.rotation * Vec3::Z,
                2 | 4 => Vec3::Y,
                3 => Vec3::X,
                5 => Vec3::new(-1.0, 1.0, 1.0).normalize(),
                _ => Vec3::ZERO,
            };
            for p in &mut n.particles {
                let world = if p.attached {
                    n.world.transform_point(p.pos)
                } else {
                    p.pos
                };
                if p.ended {
                    let age = p.age;
                    p.trail.retain(|(_, _, t)| age - t <= seg_life);
                    continue;
                }
                // Points are laid every segmentInterval and stay where they were laid; the head
                // follows the particle until the next one is laid.
                if p.trail.is_empty() {
                    p.trail.push((world, axis, p.age));
                    p.trail.push((world, axis, p.age));
                    p.next_seg = p.age + interval;
                } else if p.age >= p.next_seg {
                    p.trail.push((world, axis, p.age));
                    p.next_seg = p.age + interval;
                } else if let Some(last) = p.trail.last_mut() {
                    last.0 = world;
                    last.1 = axis;
                }
                let age = p.age;
                p.trail.retain(|(_, _, t)| age - t <= seg_life);
                if p.trail.len() > max_segs {
                    let k = p.trail.len() - max_segs;
                    p.trail.drain(0..k);
                }
            }
        }
    }
    for ch in &mut n.children {
        grow_trails(ch);
    }
}

// ---------------------------------------------------------------- appearance helpers

/// The flipbook frame's corners, the next frame's and the fraction between them, each corner's UV
/// inside the frame scaled by `scale`.
pub fn frame_uvs(
    a: &Value,
    age: f32,
    rnd: &mut Rnd,
    scale: [f32; 2],
) -> ([[f32; 2]; 4], [[f32; 2]; 4], f32) {
    let cols = int(a, "columns", 1).max(1) as f32;
    let total = int(a, "totalFrames", 1).max(1) as f32;
    let rows = (total / cols).ceil().max(1.0);
    let fi = scalar(a, "frameIndex", age, rnd, 0.0) + scalar(a, "frameIndexOffset", age, rnd, 0.0);
    let rect = |f: f32| {
        let f = f.rem_euclid(total);
        let (cx, cy) = (f % cols, (f / cols).floor());
        [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]
            .map(|[u, v]: [f32; 2]| [(cx + u * scale[0]) / cols, (cy + v * scale[1]) / rows])
    };
    (rect(fi.floor()), rect(fi.floor() + 1.0), fi - fi.floor())
}

/// Particle colour: color1 * color2 * color3 * the node's particleModifier colour, rgbMultiplier,
/// alphaMultiplier (HDR above 1 feeds the bloom). Lit particles (lighting -2 / 0) are drawn
/// unlit at 1/8 of their rgbMultiplier: where the game ships a lit config next to an unlit one
/// for the same node, the unlit one's rgbMultiplier is the lit one's x 0.125 in all 48 such pairs
/// (FwLo's measurement).
pub fn particle_color(
    a: &Value,
    c: &Value,
    p: &Particle,
    active_t: f32,
    rnd: &mut Rnd,
) -> [f32; 4] {
    let c1 = prop(a.get("color1"), p.age, rnd, [1.0; 4]).map(|v| v.clamp(0.0, 1.0));
    let c2 = prop(a.get("color2"), p.emit_t, rnd, [1.0; 4]).map(|v| v.clamp(0.0, 1.0));
    let c3 = prop(a.get("color3"), p.age, rnd, [1.0; 4]);
    let pm = prop(c["particleModifier"].get("color"), active_t, rnd, [1.0; 4]);
    let lit = matches!(a["lighting"].as_i64(), Some(-2 | 0));
    let rgbm = scalar(a, "rgbMultiplier", active_t, rnd, 1.0) * if lit { 0.125 } else { 1.0 };
    let am = scalar(a, "alphaMultiplier", active_t, rnd, 1.0);
    let k: [f32; 4] = std::array::from_fn(|i| c1[i] * c2[i] * c3[i] * pm[i]);
    let h = hue_squared([k[0], k[1], k[2]]);
    [
        h[0] * rgbm,
        h[1] * rgbm,
        h[2] * rgbm,
        (k[3] * am).clamp(0.0, 1.0),
    ]
}

/// The game's effect pixel shaders split the particle colour into its length and direction,
/// square the texture times the direction and scale by the length (FwLo's disassembly of
/// GXFfxtexture.ppo): the hue is gamma 2, the brightness linear. Here: c_i^2 / |c|, exactly the
/// shader's (texture x c/|c|)^2 x |c| for a texture of one. An earlier version multiplied by
/// sqrt 3 so a grey kept its value; the disassembly has no such factor (it made every effect
/// 1.73 times too bright, the map fires a floodlight).
fn hue_squared(c: [f32; 3]) -> [f32; 3] {
    let len = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt();
    if len < 1e-5 {
        return [0.0; 3];
    }
    c.map(|v| v * v / len)
}

/// The appearance's camera-distance fade: hidden nearer than minDistance, opacity rising linearly
/// to 1 at minFadeDistance; the same toward maxDistance from maxFadeDistance; hidden past the hard
/// cut-offs. -1 turns each off.
pub fn view_fade(a: &Value, d: f32) -> f32 {
    let f = |k: &str| a[k].as_f64().map(|v| v as f32).unwrap_or(-1.0);
    let (min_d, min_f, max_f, max_d) = (
        f("minDistance"),
        f("minFadeDistance"),
        f("maxFadeDistance"),
        f("maxDistance"),
    );
    let (min_t, max_t) = (f("minDistanceThreshold"), f("maxDistanceThreshold"));
    if (min_t >= 0.0 && d < min_t) || (max_t >= 0.0 && d > max_t) {
        return 0.0;
    }
    let mut k = 1.0;
    if min_d >= 0.0 && min_f >= 0.0 {
        k *= if min_f > min_d {
            ((d - min_d) / (min_f - min_d)).clamp(0.0, 1.0)
        } else if d < min_d {
            0.0
        } else {
            1.0
        };
    }
    if max_d >= 0.0 && max_f >= 0.0 {
        k *= if max_d > max_f {
            ((max_d - d) / (max_d - max_f)).clamp(0.0, 1.0)
        } else if d > max_d {
            0.0
        } else {
            1.0
        };
    }
    k
}

/// segmentSubdivision: each completed segment is split into `sub` + 1 pieces (Catmull-Rom through
/// the points), so a fast swing drawn from points a frame apart reads as an arc. Returns the points
/// with their place along the trail (0 the oldest, 1 the source).
pub fn subdivide(segs: &[(Vec3, Vec3, f32)], sub: usize) -> Vec<(Vec3, Vec3, f32)> {
    let n = segs.len();
    let total = (n - 1) as f32;
    let mut out = Vec::with_capacity(n * (sub + 1));
    for i in 0..n - 1 {
        out.push((segs[i].0, segs[i].1, i as f32 / total));
        if i + 2 >= n {
            continue;
        }
        let (p0, p1, p2, p3) = (
            segs[i.saturating_sub(1)].0,
            segs[i].0,
            segs[i + 1].0,
            segs[i + 2].0,
        );
        for k in 1..=sub {
            let t = k as f32 / (sub + 1) as f32;
            let (t2, t3) = (t * t, t * t * t);
            let p = 0.5
                * (2.0 * p1
                    + (p2 - p0) * t
                    + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
                    + (3.0 * p1 - p0 - 3.0 * p2 + p3) * t3);
            let axis = segs[i].1.lerp(segs[i + 1].1, t).normalize_or_zero();
            out.push((p, axis, (i as f32 + t) / total));
        }
    }
    out.push((segs[n - 1].0, segs[n - 1].1, 1.0));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn linear_and_hermite_keyframes_blend() {
        let mut rnd = Rnd { seed: 1, k: 0 };
        let linear = json!({"function": "Linear", "loop": false, "keyframes": [
            {"position": 0.0, "value": 0.0}, {"position": 2.0, "value": 4.0}]});
        assert!((prop(Some(&linear), 1.0, &mut rnd, [0.0; 4])[0] - 2.0).abs() < 1e-5);
        assert_eq!(prop(Some(&linear), 5.0, &mut rnd, [0.0; 4])[0], 4.0);
        // Both angles pi/4 is the straight line.
        let straight = std::f32::consts::FRAC_PI_4;
        let hermite = json!({"function": "Hermite", "loop": false, "keyframes": [
            {"position": 0.0, "value": 0.0, "t1": straight, "t2": straight},
            {"position": 1.0, "value": 1.0, "t1": straight, "t2": straight}]});
        assert!((prop(Some(&hermite), 0.25, &mut rnd, [0.0; 4])[0] - 0.25).abs() < 1e-4);
        assert_eq!(prop(None, 0.0, &mut rnd, [7.0; 4])[0], 7.0);
        assert_eq!(
            prop(Some(&json!([1.0, 2.0, 3.0])), 0.0, &mut rnd, [0.0; 4])[..3],
            [1.0, 2.0, 3.0]
        );
    }

    #[test]
    fn modifiers_stay_inside_their_bounds() {
        let v = json!({"value": 10.0, "modifiers": [{"type": "RandomDelta", "max": 2.0}]});
        for seed in 0..50 {
            let mut rnd = Rnd { seed, k: 0 };
            let x = prop(Some(&v), 0.0, &mut rnd, [0.0; 4])[0];
            assert!((8.0..=12.0).contains(&x), "{x}");
        }
    }

    #[test]
    fn a_periodic_emitter_spawns_and_expires() {
        // One node: a 0.1 s node emitting 3 particles of 0.2 s every 0.05 s.
        let tree = json!({"type": 2200, "stateConfigMap": [0], "nodes": [], "configs": [{
            "type": 1004,
            "nodeAttributes": {"type": 128, "delay": 0.0, "duration": 0.1},
            "emitter": {"type": 300, "interval": 0.05, "perEmission": 3.0, "totalEmissions": -1.0, "maxConcurrent": -1.0},
            "emitterShape": {"type": 400, "direction": 0},
            "directionSpread": {"type": 500},
            "particleAttributes": {"type": 129, "duration": 0.2, "attachment": 1},
            "particleModifier": {"type": 131, "speed": 1.0},
            "appearance": {"type": 603}}]});
        let mut lib = FxrLib::default();
        let mut node = make_node(&mut lib, &tree, 7, 0).unwrap();
        step_node(&mut node, &mut lib, Transform::IDENTITY, 1.0 / 60.0, false);
        assert_eq!(node.particles.len(), 3);
        for _ in 0..120 {
            step_node(&mut node, &mut lib, Transform::IDENTITY, 1.0 / 60.0, false);
        }
        assert!(node.particles.is_empty());
        assert!(node.finished);
    }

    #[test]
    fn weather_falls_in_world_space_follows_camera_and_expires() {
        let tree = json!({"type":2200,"configs":[{"nodeAttributes":{"duration":-1.},"appearance":{
            "type":10000,"emissionParticleCount":2,"emitterSize":[2.,2.,2.],"particleDuration":30,
            "particleSpeed":[0.,-1.,0.],"particleFollowFactor":0.5}}],"nodes":[]});
        let mut lib = FxrLib::default();
        let mut n = make_node(&mut lib, &tree, 7, 0).unwrap();
        step_node(&mut n, &mut lib, Transform::IDENTITY, 1. / 60., false);
        assert_eq!(n.particles.len(), 2);
        let first = n.particles[0].pos;
        step_node(
            &mut n,
            &mut lib,
            Transform::from_xyz(4., 0., 0.),
            1. / 60.,
            false,
        );
        assert!((n.particles[0].pos.x - first.x - 2.).abs() < 1e-5);
        assert!(n.particles[0].pos.y < first.y);
        for _ in 0..90 {
            step_node(
                &mut n,
                &mut lib,
                Transform::from_xyz(4., 0., 0.),
                1. / 60.,
                true,
            );
        }
        assert!(n.finished && n.particles.is_empty());
    }
}

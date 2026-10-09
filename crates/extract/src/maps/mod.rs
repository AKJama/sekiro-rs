//! `sekiro-extract map <id>`: one map area to `cache/maps/<id>/`:
//!
//! - `models/<model>.glb`: one static GLB per map piece and object model the MSB places
//!   (LOD0 only),
//! - `models/tex/<name>.dds`: the textures those GLBs reference, as the original DDS data
//!   (next to the GLBs so viewers that forbid `..` in asset paths load them),
//! - `layout.json`: map piece instances, objects, enemies, player starts, in Bevy space,
//! - `collision.bin`: the hit collision (`h*` binder) as one triangle mesh in Bevy space
//!   (`sekiro_formats::hknp::CollisionMesh` in postcard).
//!
//! Every position and transform is mirrored across X like the model export (docs/FORMATS.md).

mod pieces;
mod textures;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, Result};
use glam::{Mat4, Vec3};
use rayon::prelude::*;
use sekiro_formats::{bxf4, dcx::Oodle, flver, hknp, hkx::Compendium, msb, mtd};
use serde_json::{Value, json};

use pieces::Mtds;
use textures::MapTextures;

/// Mirror across X: FromSoftware left-handed space to Bevy's right-handed space.
fn mirror(m: Mat4) -> Mat4 {
    let s = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
    s * m * s
}

/// A part's transform in Bevy space as JSON (translation, rotation quaternion xyzw, scale).
fn transform_json(part: &msb::Part) -> Value {
    let (s, r, t) = mirror(part.matrix()).to_scale_rotation_translation();
    json!({
        "translation": t.to_array(),
        "rotation": r.normalize().to_array(),
        "scale": s.to_array(),
    })
}

/// Yaw in degrees about +Y in Bevy space (the mirror negates FromSoftware's Y rotation).
fn yaw_degrees(part: &msb::Part) -> f32 {
    -part.rotation[1]
}

pub fn summary(cache: &Path, id: &str) -> Result<()> {
    let m = read_msb(cache, id)?;
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    for p in &m.parts {
        *kinds.entry(format!("{:?}", p.kind)).or_default() += 1;
    }
    println!(
        "{} models, {} parts: {kinds:?}",
        m.models.len(),
        m.parts.len()
    );
    let mut chrs: BTreeMap<String, usize> = BTreeMap::new();
    for p in &m.parts {
        if p.kind == msb::PartKind::Enemy {
            *chrs
                .entry(m.model(p).map(|m| m.name.clone()).unwrap_or_default())
                .or_default() += 1;
        }
    }
    println!("enemies by model: {chrs:?}");
    let mut names: BTreeMap<i16, usize> = BTreeMap::new();
    for p in &m.parts {
        if let Some(c) = &p.collision {
            *names.entry(c.map_name_id).or_default() += 1;
        }
    }
    println!("collision parts by map name id: {names:?}");
    for p in m.parts.iter().filter(|p| p.kind == msb::PartKind::Player) {
        println!(
            "player start {} at {:?}, yaw {}",
            p.name, p.position, p.rotation[1]
        );
    }
    Ok(())
}

fn read_msb(cache: &Path, id: &str) -> Result<msb::Msb> {
    let path = cache.join(format!("raw/map/mapstudio/{id}.msb"));
    let data = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    msb::parse(&data).with_context(|| format!("parsing {}", path.display()))
}

fn load_mtds(raw: &Path) -> Result<Mtds> {
    let dir = raw.join("mtd/allmaterialbnd.mtdbnd.d");
    let mut out = HashMap::new();
    for e in std::fs::read_dir(&dir).with_context(|| format!("reading {}", dir.display()))? {
        let e = e?;
        let name = e.file_name().to_string_lossy().to_ascii_lowercase();
        if !name.ends_with(".mtd") {
            continue;
        }
        if let Ok(m) = mtd::parse(&std::fs::read(e.path())?) {
            out.insert(name, m);
        }
    }
    Ok(Mtds(out))
}

/// `m000003` in map `m11_00_00_00` -> `raw/map/m11_00_00_00/m11_00_00_00_000003.mapbnd.d/...flver`.
fn piece_flver(raw: &Path, id: &str, model: &str) -> PathBuf {
    let digits = model.trim_start_matches('m');
    raw.join(format!(
        "map/{id}/{id}_{digits}.mapbnd.d/{id}_{digits}.flver"
    ))
}

/// `o000100` -> `raw/obj/o000100.objbnd.d/o000100.flver`.
fn object_flver(raw: &Path, model: &str) -> PathBuf {
    raw.join(format!("obj/{model}.objbnd.d/{model}.flver"))
}

pub fn export(game: &Path, cache: &Path, id: &str) -> Result<()> {
    let raw = cache.join("raw");
    let out = cache.join("maps").join(id);
    std::fs::create_dir_all(out.join("models"))?;
    std::fs::create_dir_all(out.join("models/tex"))?;
    let oodle = Oodle::load(game).context("the map export needs the game's Oodle DLL")?;
    let msb = read_msb(cache, id)?;
    let mtds = load_mtds(&raw)?;

    // Map piece and object models actually placed, with their FLVER (and object TPF).
    let mut used: BTreeMap<String, PathBuf> = BTreeMap::new();
    let mut instances = [0usize; 2];
    for p in &msb.parts {
        let Some(m) = msb.model(p) else { continue };
        let path = match p.kind {
            msb::PartKind::MapPiece => {
                instances[0] += 1;
                piece_flver(&raw, id, &m.name)
            }
            msb::PartKind::Object => {
                instances[1] += 1;
                object_flver(&raw, &m.name)
            }
            _ => continue,
        };
        used.entry(m.name.clone()).or_insert(path);
    }
    eprintln!(
        "{id}: {} map piece and {} object instances of {} models",
        instances[0],
        instances[1],
        used.len()
    );

    // Pass 1: which textures do the pieces want?
    let wanted: BTreeSet<String> = used
        .par_iter()
        .filter_map(|(_, path)| {
            let data = std::fs::read(path).ok()?;
            let f = flver::parse(&data).ok()?;
            Some(pieces::wanted_textures(&f, &mtds))
        })
        .flatten()
        .collect::<Vec<_>>()
        .into_iter()
        .collect();

    // Pass 2: write the textures that exist.
    let mut store = MapTextures::open(&raw, oodle)?;
    // Objects carry their own textures next to their FLVER.
    for path in used.values() {
        let tpf = path.with_extension("tpf");
        if tpf.is_file() {
            store.add_tpf(&tpf)?;
        }
    }
    eprintln!(
        "  {} textures wanted, {} indexed",
        wanted.len(),
        store.len()
    );
    let written: BTreeSet<String> = wanted
        .par_iter()
        .filter_map(|name| {
            let path = out.join("models/tex").join(format!("{name}.dds"));
            if path.is_file() {
                return Some(name.clone());
            }
            match store.dds(name) {
                Ok(Some(dds)) => match std::fs::write(&path, dds) {
                    Ok(()) => Some(name.clone()),
                    Err(e) => {
                        eprintln!("  warning: writing {}: {e}", path.display());
                        None
                    }
                },
                Ok(None) => None,
                Err(e) => {
                    eprintln!("  warning: texture {name}: {e:#}");
                    None
                }
            }
        })
        .collect::<Vec<_>>()
        .into_iter()
        .collect();
    let missing: Vec<&String> = wanted.difference(&written).collect();
    eprintln!(
        "  wrote {} textures, {} missing{}",
        written.len(),
        missing.len(),
        if missing.is_empty() {
            String::new()
        } else {
            format!(" (e.g. {:?})", missing.iter().take(8).collect::<Vec<_>>())
        }
    );

    // Pass 3: one GLB per model.
    let failed = AtomicUsize::new(0);
    let uri = |name: &str| -> Option<String> {
        written.contains(name).then(|| format!("tex/{name}.dds"))
    };
    let models: Vec<(String, Value)> = used
        .par_iter()
        .filter_map(|(model, path)| {
            let result = (|| -> Result<_> {
                let data =
                    std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
                let f = flver::parse(&data)?;
                let built = pieces::build(&f, &mtds, &uri)?;
                if built.primitives == 0 {
                    return Ok(None);
                }
                std::fs::write(out.join(format!("models/{model}.glb")), &built.glb)?;
                Ok(Some(json!({
                    "glb": format!("models/{model}.glb"),
                    "triangles": built.triangles,
                    "primitives": built.primitives,
                    "min": built.bbox_min,
                    "max": built.bbox_max,
                })))
            })();
            match result {
                Ok(Some(v)) => Some((model.clone(), v)),
                Ok(None) => None,
                Err(e) => {
                    failed.fetch_add(1, Ordering::Relaxed);
                    eprintln!("  warning: {model}: {e:#}");
                    None
                }
            }
        })
        .collect();
    let models: BTreeMap<String, Value> = models.into_iter().collect();
    eprintln!(
        "  wrote {} GLBs ({} failed, {} with nothing visible)",
        models.len(),
        failed.load(Ordering::Relaxed),
        used.len() - models.len() - failed.load(Ordering::Relaxed)
    );

    // Layout.
    let model_name = |p: &msb::Part| msb.model(p).map(|m| m.name.clone()).unwrap_or_default();
    let mut piece_rows = Vec::new();
    let mut never_drawn = 0;
    let mut bmin = Vec3::splat(f32::MAX);
    let mut bmax = Vec3::splat(f32::MIN);
    for p in msb
        .parts
        .iter()
        .filter(|p| p.kind == msb::PartKind::MapPiece)
    {
        let model = model_name(p);
        let Some(m) = models.get(&model) else {
            continue;
        };
        if p.masks
            .get(8..16)
            .is_some_and(|g| g.iter().all(|&w| w == 0))
        {
            // No draw groups: the piece never draws.
            never_drawn += 1;
            continue;
        }
        let world = mirror(p.matrix());
        for corner in corners(&m["min"], &m["max"]) {
            let c = world.transform_point3(corner);
            bmin = bmin.min(c);
            bmax = bmax.max(c);
        }
        let mut row = transform_json(p);
        row["name"] = json!(p.name);
        row["model"] = json!(model);
        row["draw_groups"] = json!(groups(p, 1));
        piece_rows.push(row);
    }
    let rows_of = |kinds: &[msb::PartKind]| -> Vec<Value> {
        msb.parts
            .iter()
            .filter(|p| kinds.contains(&p.kind))
            .map(|p| {
                let mut row = transform_json(p);
                row["name"] = json!(p.name);
                row["model"] = json!(model_name(p));
                row["yaw_degrees"] = json!(yaw_degrees(p));
                row["entity_id"] = json!(p.entity_id);
                if p.masks.len() >= 16 && p.kind != msb::PartKind::Collision {
                    row["draw_groups"] = json!(groups(p, 1));
                }
                if let Some(e) = &p.enemy {
                    row["npc_param"] = json!(e.npc_param_id);
                    row["think_param"] = json!(e.think_param_id);
                    row["chara_init"] = json!(e.chara_init_id);
                    row["platoon"] = json!(e.platoon_id);
                    row["dummy"] = json!(p.kind == msb::PartKind::DummyEnemy);
                }
                if let Some(c) = &p.collision {
                    row["hit_filter_id"] = json!(c.hit_filter_id);
                    row["map_name_id"] = json!(c.map_name_id);
                    row["display_groups"] = json!(groups(p, 0));
                    row["draw_groups"] = json!(groups(p, 1));
                }
                row
            })
            .collect()
    };

    // Collision.
    let (collision, ranges, collision_note) = export_collision(&raw, id, &msb, oodle)?;
    std::fs::write(out.join("collision.bin"), collision.to_bytes()?)?;
    eprintln!(
        "  collision: {} triangles, {} vertices ({collision_note})",
        collision.triangle_count(),
        collision.vertices.len()
    );

    let layout = json!({
        "id": id,
        "source": "sekiro-extract map",
        "convention": "Bevy space: FromSoftware x mirrored (x -> -x); rotations are quaternions xyzw",
        "bounds": { "min": bmin.to_array(), "max": bmax.to_array() },
        "models": models,
        "pieces": piece_rows,
        "objects": rows_of(&[msb::PartKind::Object]),
        "enemies": rows_of(&[msb::PartKind::Enemy, msb::PartKind::DummyEnemy]),
        "players": rows_of(&[msb::PartKind::Player]),
        "collisions": rows_of(&[msb::PartKind::Collision])
            .into_iter()
            .map(|mut row| {
                if let Some(r) = row["name"].as_str().and_then(|n| ranges.get(n)) {
                    row["triangles"] = json!(r);
                }
                row
            })
            .collect::<Vec<_>>(),
        "collision": {
            "file": "collision.bin",
            "triangles": collision.triangle_count(),
            "note": collision_note,
        },
    });
    std::fs::write(out.join("layout.json"), serde_json::to_vec_pretty(&layout)?)?;
    eprintln!(
        "  layout: {} pieces ({never_drawn} without draw groups left out), bounds {:?}..{:?}; wrote {}",
        piece_rows.len(),
        bmin.to_array(),
        bmax.to_array(),
        out.display()
    );
    Ok(())
}

/// One 8-word group block of a part's mask struct: 0 display groups, 1 draw groups
/// (SoulsFormats' MSBS `UnkStruct1` layout: display[8], draw[8], collision mask[32]).
fn groups(p: &msb::Part, block: usize) -> Vec<u32> {
    p.masks
        .get(block * 8..block * 8 + 8)
        .map(|g| g.to_vec())
        .unwrap_or_default()
}

fn corners(min: &Value, max: &Value) -> Vec<Vec3> {
    let v = |x: &Value, i: usize| x[i].as_f64().unwrap_or(0.0) as f32;
    let (a, b) = (
        Vec3::new(v(min, 0), v(min, 1), v(min, 2)),
        Vec3::new(v(max, 0), v(max, 1), v(max, 2)),
    );
    (0..8)
        .map(|i| {
            Vec3::new(
                if i & 1 == 0 { a.x } else { b.x },
                if i & 2 == 0 { a.y } else { b.y },
                if i & 4 == 0 { a.z } else { b.z },
            )
        })
        .collect()
}

/// Decodes the high-resolution hit collision of every collision part in the MSB.
///
/// HKX vertices are in the collision model's own space (the hknp bodies carry identity
/// transforms); the MSB collision part transform places them, like any other part. Checked
/// against the player start points, which sit within a metre of collision only with it.
fn export_collision(
    raw: &Path,
    id: &str,
    msb: &msb::Msb,
    oodle: &'static Oodle,
) -> Result<(hknp::CollisionMesh, HashMap<String, [usize; 2]>, String)> {
    let tail = &id[1..];
    let dir = raw.join(format!("map/{id}"));
    let bhd = std::fs::read(dir.join(format!("h{tail}.hkxbhd")))?;
    let bdt = std::fs::read(dir.join(format!("h{tail}.hkxbdt")))?;
    let files = bxf4::parse(&bhd, &bdt)?;
    let compendium = match files.iter().find(|f| f.file_name().contains("compendium")) {
        Some(f) => Some(Compendium::parse(&f.contents(Some(oodle))?)?),
        None => None,
    };
    let by_name: HashMap<String, &bxf4::BxfFile> = files
        .iter()
        .map(|f| (f.file_name().to_ascii_lowercase(), f))
        .collect();
    let parts: Vec<&msb::Part> = msb
        .parts
        .iter()
        .filter(|p| p.kind == msb::PartKind::Collision)
        .collect();
    let wanted: BTreeSet<String> = parts
        .iter()
        .filter_map(|p| msb.model(p).map(|m| m.name.clone()))
        .collect();
    let decoded: Vec<(String, Result<(hknp::CollisionMesh, hknp::DecodeStats)>)> = wanted
        .par_iter()
        .filter_map(|model| {
            let digits = model.trim_start_matches('h');
            let f = by_name.get(&format!("h{tail}_{digits}.hkx.dcx"))?;
            let r = (|| {
                let data = f.contents(Some(oodle))?;
                hknp::decode(&data, compendium.as_ref())
                    .with_context(|| format!("decoding {}", f.name))
            })();
            Some((model.clone(), r))
        })
        .collect();
    let mut by_model = HashMap::new();
    let mut total = hknp::DecodeStats::default();
    let mut errors = 0;
    for (model, r) in decoded {
        match r {
            Ok((m, s)) => {
                total.bodies += s.bodies;
                total.sections += s.sections;
                total.primitives += s.primitives;
                total.quads += s.quads;
                total.convex_skipped += s.convex_skipped;
                total.unmatched_material += s.unmatched_material;
                by_model.insert(model, m);
            }
            Err(e) => {
                errors += 1;
                eprintln!("  warning: collision: {e:#}");
            }
        }
    }
    let mut mesh = hknp::CollisionMesh::default();
    let mut ranges = HashMap::new();
    for p in &parts {
        let Some(model) = msb.model(p).and_then(|m| by_model.get(&m.name)) else {
            continue;
        };
        let world = mirror(p.matrix());
        let mut placed = model.clone();
        for v in &mut placed.vertices {
            // Mirror first (FromSoftware space -> Bevy space), then the mirrored part matrix.
            *v = world
                .transform_point3(Vec3::new(-v[0], v[1], v[2]))
                .to_array();
        }
        ranges.insert(
            p.name.clone(),
            [mesh.triangle_count(), placed.triangle_count()],
        );
        mesh.extend(&placed);
    }

    // Validation: every player start should stand on collision.
    let verts: Vec<Vec3> = mesh.vertices.iter().map(|v| Vec3::from(*v)).collect();
    let start_gap = msb
        .parts
        .iter()
        .filter(|p| p.kind == msb::PartKind::Player)
        .map(|p| {
            let s = Vec3::new(-p.position[0], p.position[1], p.position[2]);
            verts
                .par_iter()
                .map(|v| v.distance(s))
                .reduce(|| f32::MAX, f32::min)
        })
        .fold(0.0f32, f32::max);
    let materials: BTreeSet<u32> = mesh.materials.iter().copied().collect();
    let note = format!(
        "{} collision parts, {} models, {} failed, {} bodies, {} sections, {} primitives ({} quads, {} convex skipped), {} triangles without material, {} distinct materials, furthest player start {:.2} m from a collision vertex",
        parts.len(),
        wanted.len(),
        errors,
        total.bodies,
        total.sections,
        total.primitives,
        total.quads,
        total.convex_skipped,
        total.unmatched_material,
        materials.len(),
        start_gap,
    );
    Ok((mesh, ranges, note))
}

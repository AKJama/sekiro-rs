//! Unweighted character and parts vertices are stored in the frame of the node their normal W
//! names (issue #11, the same rule as map objects). The FLVER header bounds are the game's
//! model-space bounds, so the exported glTF, at rest, must fit them.

use super::*;
use rayon::prelude::*;
use sekiro_formats::flver::{LayoutSemantic, LayoutType};
use std::path::PathBuf;

/// Header bounds are stored as f32 from the authoring tool; 1 cm covers their rounding.
const TOLERANCE: f32 = 0.01;

/// Every unpacked character and parts FLVER.
fn chr_and_parts_flvers() -> Vec<PathBuf> {
    let raw = sekiro_formats::cache::root().join("raw");
    let mut out = Vec::new();
    for dir in ["chr", "parts"] {
        let Ok(entries) = std::fs::read_dir(raw.join(dir)) else {
            continue;
        };
        for bnd in entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
            for f in std::fs::read_dir(&bnd).unwrap().flatten() {
                let f = f.path();
                if f.extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("flver"))
                {
                    out.push(f);
                }
            }
        }
    }
    out.sort();
    out
}

/// Model-space bounds of every mesh, unweighted vertices placed by their normal-W node.
fn model_space_bounds(f: &flver::Flver) -> Option<(Vec3, Vec3)> {
    let world = flver::world_matrices(&f.nodes);
    let (mut lo, mut hi) = (Vec3::MAX, Vec3::MIN);
    for mesh in &f.meshes {
        for p in &mesh.model_space_vertices(&world).positions {
            lo = lo.min(Vec3::from(*p));
            hi = hi.max(Vec3::from(*p));
        }
    }
    (lo.x <= hi.x).then_some((lo, hi))
}

/// Exports `f` as its own skeleton and skins every vertex at rest, as a glTF viewer does
/// (joint world matrix times inverse bind matrix, weighted). Returns the bounds in
/// FromSoftware space (x mirrored back).
fn exported_rest_bounds(raw: &Path, f: &flver::Flver) -> (Vec3, Vec3) {
    let model = Model {
        skeleton: f.clone(),
        parts: vec![Part {
            label: "part".into(),
            flver: f.clone(),
            tpfs: Vec::new(),
            visible: vec![true; f.meshes.len()],
            attach: None,
        }],
        notes: Vec::new(),
        draw_masks: None,
    };
    let glb = build(raw, &model, false).unwrap();
    let json_len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
    let doc: Value = serde_json::from_slice(&glb[20..20 + json_len]).unwrap();
    let bin = &glb[20 + json_len + 8..];
    let read = |accessor: usize, width: usize| -> Vec<Vec<f32>> {
        let a = &doc["accessors"][accessor];
        let view = &doc["bufferViews"][a["bufferView"].as_u64().unwrap() as usize];
        let offset = view["byteOffset"].as_u64().unwrap() as usize;
        let count = a["count"].as_u64().unwrap() as usize;
        let short = a["componentType"].as_u64() == Some(5123);
        let size = if short { 2 } else { 4 };
        (0..count)
            .map(|i| {
                (0..width)
                    .map(|c| {
                        let at = offset + (i * width + c) * size;
                        if short {
                            u16::from_le_bytes(bin[at..at + 2].try_into().unwrap()) as f32
                        } else {
                            f32::from_le_bytes(bin[at..at + 4].try_into().unwrap())
                        }
                    })
                    .collect()
            })
            .collect()
    };
    // World matrices of the glTF nodes.
    let nodes = doc["nodes"].as_array().unwrap();
    let mut world = vec![Mat4::IDENTITY; nodes.len()];
    let mut stack: Vec<(usize, Mat4)> = doc["scenes"][0]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| (n.as_u64().unwrap() as usize, Mat4::IDENTITY))
        .collect();
    while let Some((n, parent)) = stack.pop() {
        let v = |k: &str| -> Vec<f32> {
            nodes[n][k]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap() as f32)
                .collect()
        };
        let (t, r, s) = (v("translation"), v("rotation"), v("scale"));
        let local = Mat4::from_scale_rotation_translation(
            Vec3::new(s[0], s[1], s[2]),
            glam::Quat::from_xyzw(r[0], r[1], r[2], r[3]),
            Vec3::new(t[0], t[1], t[2]),
        );
        world[n] = parent * local;
        for c in nodes[n]["children"].as_array().into_iter().flatten() {
            stack.push((c.as_u64().unwrap() as usize, world[n]));
        }
    }
    let (mut lo, mut hi) = (Vec3::MAX, Vec3::MIN);
    for node in nodes.iter().filter(|n| n.get("mesh").is_some()) {
        let skin = &doc["skins"][node["skin"].as_u64().unwrap() as usize];
        let ibms = read(skin["inverseBindMatrices"].as_u64().unwrap() as usize, 16);
        let joints: Vec<Mat4> = skin["joints"]
            .as_array()
            .unwrap()
            .iter()
            .zip(&ibms)
            .map(|(j, ibm)| world[j.as_u64().unwrap() as usize] * Mat4::from_cols_slice(ibm))
            .collect();
        let mesh = &doc["meshes"][node["mesh"].as_u64().unwrap() as usize];
        for primitive in mesh["primitives"].as_array().unwrap() {
            let attr = |k: &str| primitive["attributes"][k].as_u64().unwrap() as usize;
            let positions = read(attr("POSITION"), 3);
            let joint_ids = read(attr("JOINTS_0"), 4);
            let weights = read(attr("WEIGHTS_0"), 4);
            for ((p, j), w) in positions.iter().zip(&joint_ids).zip(&weights) {
                let p = Vec3::new(p[0], p[1], p[2]);
                let skinned: Vec3 = (0..4)
                    .filter(|&c| w[c] > 0.0)
                    .map(|c| joints[j[c] as usize].transform_point3(p) * w[c])
                    .sum();
                let skinned = Vec3::from(mirror_point(skinned.to_array()));
                lo = lo.min(skinned);
                hi = hi.max(skinned);
            }
        }
    }
    (lo, hi)
}

fn bounds_error((lo, hi): (Vec3, Vec3), f: &flver::Flver) -> f32 {
    let (a, b) = (Vec3::from(f.bbox_min), Vec3::from(f.bbox_max));
    (lo - a)
        .abs()
        .max_element()
        .max((hi - b).abs().max_element())
}

#[test]
fn chr_and_parts_fit_their_header_bounds_in_model_space() {
    let files = chr_and_parts_flvers();
    if files.is_empty() {
        eprintln!("installed chr/parts fixture unavailable");
        return;
    }
    let rows: Vec<_> = files
        .par_iter()
        .map(|path| {
            let f = flver::parse(&std::fs::read(path).unwrap()).unwrap();
            let world = flver::world_matrices(&f.nodes);
            let moved = f
                .meshes
                .iter()
                .any(|m| matches!(m.model_space_vertices(&world), std::borrow::Cow::Owned(_)));
            // A Float3 normal has no W, which would bind the mesh to node 0.
            let float3_rigid = f.meshes.iter().any(|m| {
                !m.dynamic
                    && m.buffers.iter().any(|b| {
                        f.layouts[b.layout].iter().any(|l| {
                            l.semantic == LayoutSemantic::Normal && l.kind == LayoutType::Float3
                        })
                    })
            });
            let name = path
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_ascii_lowercase();
            let error = model_space_bounds(&f).map(|b| bounds_error(b, &f));
            (name, moved, float3_rigid, error)
        })
        .collect();
    let mut moved = Vec::new();
    for (name, m, float3_rigid, error) in &rows {
        assert!(
            !float3_rigid,
            "{name}: unweighted mesh with a Float3 normal"
        );
        if let Some(error) = error {
            assert!(
                *error <= TOLERANCE,
                "{name}: model-space vertices miss the header bounds by {error} m"
            );
        }
        if *m {
            moved.push(name.as_str());
        }
    }
    // The known node-bound cases; raw positions miss c7102's bounds by 1 m.
    for name in ["c7102", "wp_a_0720", "wp_a_0760_1", "wp_a_0763_1_l"] {
        assert!(moved.contains(&name), "{name} has no node-bound vertices");
    }
    assert!(rows.len() > 300, "only {} FLVERs checked", rows.len());
}

/// The exported glTF, skinned at rest, must fit the header bounds for every character and
/// parts FLVER with node-bound vertices (c7102 was 1 m off, WP_A_0720 16 cm, before #11).
#[test]
fn node_bound_chr_and_parts_export_inside_their_header_bounds() {
    let raw = sekiro_formats::cache::root().join("raw");
    let files = chr_and_parts_flvers();
    if files.is_empty() || !raw.join("mtd").is_dir() {
        eprintln!("installed chr/parts fixture unavailable");
        return;
    }
    let node_bound: Vec<_> = files
        .par_iter()
        .filter_map(|path| {
            let f = flver::parse(&std::fs::read(path).unwrap()).unwrap();
            let world = flver::world_matrices(&f.nodes);
            f.meshes
                .iter()
                .any(|m| matches!(m.model_space_vertices(&world), std::borrow::Cow::Owned(_)))
                .then(|| (path.file_stem().unwrap().to_string_lossy().to_string(), f))
        })
        .collect();
    assert!(
        node_bound.len() > 30,
        "only {} node-bound FLVERs",
        node_bound.len()
    );
    let errors: Vec<_> = node_bound
        .par_iter()
        .map(|(name, f)| {
            let error = bounds_error(exported_rest_bounds(&raw, f), f);
            (name, error)
        })
        .collect();
    for (name, error) in errors {
        assert!(
            error <= TOLERANCE,
            "{name}: exported rest pose misses the header bounds by {error} m"
        );
    }
}

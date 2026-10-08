//! Map piece FLVERs to static GLBs. LOD0 face sets only, mirrored on X like every export (see
//! docs/FORMATS.md). Textures are referenced as external DDS files shared by the whole map.

use std::collections::HashMap;

use anyhow::{Result, bail};
use glam::Vec3;
use sekiro_formats::{flver, mtd};
use serde_json::{Value, json};

use crate::models::glb::Glb;
use crate::models::textures::texture_stem;

/// A texture choice: game path (its stem is the texture name) and UV channel.
#[derive(Clone, Debug)]
pub struct TexRef {
    pub path: String,
    pub uv: usize,
}

/// What one FLVER material renders with.
#[derive(Clone, Debug, Default)]
pub struct MaterialChoice {
    pub albedo: Option<TexRef>,
    pub normal: Option<TexRef>,
    pub alpha_mode: Option<(&'static str, f32)>,
    pub mtd: String,
}

/// MTDs by lower-case file name.
pub struct Mtds(pub HashMap<String, mtd::Mtd>);

impl Mtds {
    pub fn get(&self, mtd_path: &str) -> Option<&mtd::Mtd> {
        self.0
            .get(&format!("{}.mtd", texture_stem(mtd_path).to_ascii_lowercase()))
    }
}

/// Picks the base colour (and its normal map) for a map material.
///
/// Map materials stack many layers (`M_Multiple`, `MultiBlend3`, ...) and leave the FLVER
/// texture paths empty, so paths come from the MTD. The base layer is the first albedo slot
/// with a path, in MTD order (for `M_Multiple` that is slot 8, the main surface; later slots
/// are moss, snow and other overlays). A slot named after the material (`<name>_a`) wins.
pub fn choose_material(material: &flver::Material, mtd: Option<&mtd::Mtd>) -> MaterialChoice {
    let mut albedo = Vec::new();
    let mut normal = Vec::new();
    for t in &material.textures {
        let from_mtd = mtd.and_then(|m| m.textures.iter().find(|mt| mt.kind == t.param));
        let path = if t.path.is_empty() {
            from_mtd.map(|m| m.path.clone()).unwrap_or_default()
        } else {
            t.path.clone()
        };
        if path.is_empty() {
            continue;
        }
        let uv = from_mtd
            .map(|m| (m.uv_number - 1).clamp(0, 1) as usize)
            .unwrap_or(0);
        let param = t.param.to_ascii_lowercase();
        let r = TexRef { path, uv };
        if param.contains("albedomap") || param.contains("diffuse") {
            albedo.push(r);
        } else if param.contains("normalmap") || param.contains("bumpmap") {
            normal.push(r);
        }
    }
    let own = format!("{}_a", material.name.to_ascii_lowercase());
    let a = albedo
        .iter()
        .position(|r| texture_stem(&r.path).eq_ignore_ascii_case(&own))
        .or(if albedo.is_empty() { None } else { Some(0) });
    let n = a.and_then(|a| {
        let stem = texture_stem(&albedo[a].path).to_ascii_lowercase();
        let want = format!("{}_n", stem.strip_suffix("_a")?);
        normal
            .iter()
            .position(|r| texture_stem(&r.path).eq_ignore_ascii_case(&want))
    });
    let mut choice = MaterialChoice {
        albedo: a.map(|i| albedo[i].clone()),
        normal: n.map(|i| normal[i].clone()),
        alpha_mode: None,
        mtd: texture_stem(&material.mtd).to_string(),
    };
    if let Some(m) = mtd {
        let int = |name: &str| match m.param(name) {
            Some(mtd::ParamValue::Ints(v)) => v.first().copied().unwrap_or(0),
            _ => 0,
        };
        let (alpha_ref, blend) = (int("g_AlphaRef"), int("g_BlendMode"));
        if blend == 2 || blend >= 4 {
            choice.alpha_mode = Some(("BLEND", 0.0));
        } else if alpha_ref > 0 || blend == 1 {
            choice.alpha_mode = Some(("MASK", (alpha_ref.max(1) as f32 / 255.0).max(0.3)));
        }
    }
    choice
}

/// Materials whose meshes are not drawn as visible surfaces in our renderer.
pub fn skip_material(mtd_name: &str) -> bool {
    let m = mtd_name.to_ascii_lowercase();
    // Shadow casters, invisible blockers and light-probe helpers.
    m.contains("shadow") || m.contains("[dummy]") || m.contains("_dummy") || m.contains("invisible")
}

pub struct BuiltPiece {
    pub glb: Vec<u8>,
    pub triangles: usize,
    pub primitives: usize,
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
}

/// Builds a static GLB for one map piece. `texture_uri` maps a texture name to the URI to
/// reference, or None when the texture is unavailable (the material then has no texture).
pub fn build(
    flver: &flver::Flver,
    mtds: &Mtds,
    texture_uri: &dyn Fn(&str) -> Option<String>,
) -> Result<BuiltPiece> {
    let mut g = Glb::default();
    let mut primitives = Vec::new();
    let mut texture_ids: HashMap<String, usize> = HashMap::new();
    let mut triangles = 0;
    let mut bmin = Vec3::splat(f32::MAX);
    let mut bmax = Vec3::splat(f32::MIN);
    for (mi, mesh) in flver.meshes.iter().enumerate() {
        let Some(lod0) = mesh.lod0() else { continue };
        let v = &mesh.vertices;
        let count = v.len();
        if count == 0 {
            continue;
        }
        let material = &flver.materials[mesh.material];
        let mtd = mtds.get(&material.mtd);
        let choice = choose_material(material, mtd);
        if skip_material(&choice.mtd) {
            continue;
        }
        let indices = lod0.triangles(count);
        if indices.is_empty() {
            continue;
        }
        if indices.iter().any(|&i| i as usize >= count) {
            bail!("mesh {mi} index out of range");
        }
        let positions: Vec<[f32; 3]> = v.positions.iter().map(|p| [-p[0], p[1], p[2]]).collect();
        for p in &positions {
            bmin = bmin.min(Vec3::from(*p));
            bmax = bmax.max(Vec3::from(*p));
        }
        let mut attributes = json!({ "POSITION": g.vec3(&positions, true) });
        if v.normals.len() == count {
            let normals: Vec<[f32; 3]> = v
                .normals
                .iter()
                .map(|n| {
                    Vec3::new(-n[0], n[1], n[2])
                        .try_normalize()
                        .unwrap_or(Vec3::Y)
                        .into()
                })
                .collect();
            attributes["NORMAL"] = json!(g.vec3(&normals, false));
        }
        for (c, uv) in v.uvs.iter().take(2).enumerate() {
            if uv.len() == count {
                attributes[format!("TEXCOORD_{c}")] = json!(g.vec2(uv));
            }
        }
        let uv_ok = |uv: usize| v.uvs.get(uv).is_some_and(|u| u.len() == count) && uv < 2;

        let mut pbr = json!({ "metallicFactor": 0.0, "roughnessFactor": 0.85 });
        let mut texture = |r: &TexRef, g: &mut Glb| -> Option<(usize, usize)> {
            let name = texture_stem(&r.path).to_ascii_lowercase();
            let id = match texture_ids.get(&name) {
                Some(&t) => t,
                None => {
                    let uri = texture_uri(&name)?;
                    let t = g.texture_uri(&uri, &name);
                    texture_ids.insert(name, t);
                    t
                }
            };
            Some((id, if uv_ok(r.uv) { r.uv } else { 0 }))
        };
        if let Some(r) = &choice.albedo
            && let Some((t, uv)) = texture(r, &mut g)
        {
            pbr["baseColorTexture"] = json!({ "index": t, "texCoord": uv });
        } else {
            // No texture: a neutral stone grey keeps the shape readable.
            pbr["baseColorFactor"] = json!([0.45, 0.43, 0.40, 1.0]);
        }
        let mut mat = json!({
            "name": format!("m{mi} {} | {}", material.name, choice.mtd),
            "pbrMetallicRoughness": pbr,
            "doubleSided": !lod0.cull_backfaces,
        });
        if let Some((mode, cutoff)) = choice.alpha_mode {
            mat["alphaMode"] = json!(mode);
            if mode == "MASK" {
                mat["alphaCutoff"] = json!(cutoff);
            }
        }
        g.materials.push(mat);
        triangles += indices.len() / 3;
        primitives.push(json!({
            "attributes": attributes,
            "indices": g.indices(&indices),
            "material": g.materials.len() - 1,
        }));
    }
    let count = primitives.len();
    let mut roots = Vec::new();
    if !primitives.is_empty() {
        g.meshes.push(json!({ "primitives": primitives }));
        g.nodes.push(json!({ "mesh": 0 }));
        roots.push(0);
    }
    let extras: Value = json!({
        "source": "sekiro-extract map",
        "convention": "FromSoftware x mirrored (x -> -x); see docs/FORMATS.md",
    });
    Ok(BuiltPiece {
        glb: g.finish(&roots, extras),
        triangles,
        primitives: count,
        bbox_min: bmin.to_array(),
        bbox_max: bmax.to_array(),
    })
}

/// Texture names (lower case) a FLVER's visible materials would use.
pub fn wanted_textures(flver: &flver::Flver, mtds: &Mtds) -> Vec<String> {
    let mut out = Vec::new();
    for mesh in &flver.meshes {
        let material = &flver.materials[mesh.material];
        let choice = choose_material(material, mtds.get(&material.mtd));
        if skip_material(&choice.mtd) {
            continue;
        }
        if let Some(r) = choice.albedo {
            out.push(texture_stem(&r.path).to_ascii_lowercase());
        }
    }
    out
}

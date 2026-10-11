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
    /// Tangent-space normal map (exported converted to BC5, see `textures::normal_to_bc5`).
    pub normal: Option<TexRef>,
    pub alpha_mode: Option<(&'static str, f32)>,
    pub mtd: String,
    pub layers: Option<Layers>,
    pub double_sided: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Layers {
    pub overlay: Option<TexRef>,
    pub overlay_normal: Option<TexRef>,
    pub snow: Option<TexRef>,
    pub snow_normal: Option<TexRef>,
    pub c: Option<TexRef>,
    pub c_normal: Option<TexRef>,
    pub mask: Option<TexRef>,
}

fn slot_number(kind: &str) -> Option<u32> {
    kind.split("Texture2D_")
        .nth(1)?
        .split('_')
        .next()?
        .parse()
        .ok()
}

/// MTDs by lower-case file name.
pub struct Mtds(pub HashMap<String, mtd::Mtd>);

impl Mtds {
    pub fn get(&self, mtd_path: &str) -> Option<&mtd::Mtd> {
        self.0.get(&format!(
            "{}.mtd",
            texture_stem(mtd_path).to_ascii_lowercase()
        ))
    }
}

/// Picks the base colour and normal map for a map material.
///
/// Map materials stack many layers (`M_Multiple`, `MultiBlend3`, ...) and leave the FLVER
/// texture paths empty, so paths come from the MTD. The base layer is the first albedo slot
/// with a path, in MTD order (for `M_Multiple` that is slot 8, the main surface; later slots
/// are moss, snow and other overlays). A slot named after the material (`<name>_a`) wins.
/// The normal map is the one named like the albedo (`x_a` -> `x_n`), else the one named after
/// the material (`<name>_n`, often the mesh's own baked normal), else the first with a path.
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
        } else if param.contains("normalmap") {
            normal.push(r);
        }
    }
    // Wind and Gradient variants can use different sampler names for the same foliage.
    // Only fall back when no albedo resolved, and prefer the primary (_0) MTD set.
    if albedo.is_empty()
        && let Some(m) = mtd
    {
        let primary = |needle: &str| {
            m.textures
                .iter()
                .filter(|t| !t.path.is_empty() && t.kind.to_ascii_lowercase().contains(needle))
                .min_by_key(|t| {
                    (
                        !t.kind.ends_with("_0"),
                        slot_number(&t.kind).unwrap_or(u32::MAX),
                    )
                })
                .map(|t| TexRef {
                    path: t.path.clone(),
                    uv: (t.uv_number - 1).clamp(0, 1) as usize,
                })
        };
        albedo.extend(primary("albedomap"));
        normal.extend(primary("normalmap"));
    }
    let own = format!("{}_a", material.name.to_ascii_lowercase());
    let a = albedo
        .iter()
        .position(|r| texture_stem(&r.path).eq_ignore_ascii_case(&own))
        .or(if albedo.is_empty() { None } else { Some(0) });
    let named = |want: &str| {
        normal
            .iter()
            .position(|r| texture_stem(&r.path).eq_ignore_ascii_case(want))
    };
    let n = a
        .and_then(|a| {
            let stem = texture_stem(&albedo[a].path).to_ascii_lowercase();
            named(&format!("{}_n", stem.strip_suffix("_a")?))
        })
        .or_else(|| named(&format!("{}_n", material.name.to_ascii_lowercase())))
        .or(if normal.is_empty() { None } else { Some(0) });
    let mut choice = MaterialChoice {
        albedo: a.map(|i| albedo[i].clone()),
        normal: n.map(|i| normal[i].clone()),
        alpha_mode: None,
        mtd: texture_stem(&material.mtd).to_string(),
        layers: None,
        double_sided: false,
    };
    if let Some(m) = mtd {
        let int = |name: &str| match m.param(name) {
            Some(mtd::ParamValue::Ints(v)) => v.first().copied().unwrap_or(0),
            _ => 0,
        };
        let (alpha_ref, blend) = (int("g_AlphaRef"), int("g_BlendMode"));
        choice.double_sided = int("g_DoubleSided") != 0
            || matches!(m.param("g_DoubleSided"), Some(mtd::ParamValue::Bool(true)));
        if blend == 2 || blend >= 4 {
            choice.alpha_mode = Some(("BLEND", 0.0));
        } else if alpha_ref > 0 || blend == 1 {
            choice.alpha_mode = Some(("MASK", (alpha_ref.max(1) as f32 / 255.0).max(0.3)));
            choice.double_sided = true;
        }
        if m.shader.contains("M[Multiple]") || m.shader.contains("M[MultipleGround]") {
            let slot = |number, semantic: &str| {
                m.textures
                    .iter()
                    .find(|t| slot_number(&t.kind) == Some(number) && t.kind.contains(semantic))
                    .and_then(|t| {
                        let path = material
                            .textures
                            .iter()
                            .find(|f| f.param == t.kind && !f.path.is_empty())
                            .map(|f| &f.path)
                            .unwrap_or(&t.path);
                        // Some authored MTD defaults put a mask in an albedo slot or an
                        // albedo in a normal slot. Native branch switches are untraced;
                        // do not use those as colour/normal layers in our approximation.
                        let stem = texture_stem(path).to_ascii_lowercase();
                        if (semantic == "AlbedoMap"
                            && (stem.ends_with("_3m") || stem.ends_with("_n")))
                            || (semantic == "NormalMap"
                                && (stem.ends_with("_3m") || stem.ends_with("_a")))
                        {
                            return None;
                        }
                        (!path.is_empty()).then(|| TexRef {
                            path: path.clone(),
                            uv: (t.uv_number - 1).clamp(0, 1) as usize,
                        })
                    })
            };
            let ground = m.shader.contains("M[MultipleGround]");
            choice.albedo = slot(8, "AlbedoMap").or(choice.albedo);
            choice.normal = slot(3, "NormalMap").or(choice.normal);
            choice.layers = Some(Layers {
                overlay: slot(5, "AlbedoMap"),
                overlay_normal: slot(4, "NormalMap"),
                snow: slot(if ground { 11 } else { 12 }, "AlbedoMap"),
                snow_normal: slot(if ground { 7 } else { 10 }, "NormalMap"),
                c: if ground { None } else { slot(7, "AlbedoMap") },
                c_normal: if ground { None } else { slot(0, "NormalMap") },
                mask: slot(9, "Mask"),
            });
            if choice.layers.as_ref().is_some_and(|l| {
                l.overlay.is_none()
                    && l.overlay_normal.is_none()
                    && l.snow.is_none()
                    && l.snow_normal.is_none()
                    && l.c.is_none()
                    && l.c_normal.is_none()
            }) {
                choice.layers = None;
            }
        }
    }
    choice
}

/// Materials whose meshes are not drawn as visible surfaces in our renderer.
pub fn skip_material(mtd_name: &str) -> bool {
    let m = mtd_name.to_ascii_lowercase();
    // Shadow casters, invisible blockers and light-probe helpers.
    m.contains("shadow")
        || m.contains("[dummy]")
        || m.contains("_dummy")
        || m.contains("invisible")
        // Ground fog and cloud cards: drawn by fog shaders we do not have.
        || m.contains("fog")
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
        if choice.layers.is_some() {
            let weights: Vec<[f32; 4]> = (0..count)
                .map(|i| {
                    let byte = |stream: usize| {
                        v.blends
                            .get(stream)
                            .and_then(|b| b.get(i))
                            .map_or(0., |b| b[0] as f32 / 255.)
                    };
                    [byte(0), byte(4), byte(2), 1.]
                })
                .collect();
            attributes["COLOR_0"] = json!(g.vec4(&weights));
        } else if choice.alpha_mode.is_some()
            && let Some(colors) = v.colors.first()
            && colors.len() == count
        {
            // Mesh decals feather coverage in vertex alpha even when their DDS is opaque.
            // RGB here is shader data (often constant red), not a StandardMaterial tint.
            let opacity: Vec<_> = colors.iter().map(|color| [1., 1., 1., color[3]]).collect();
            attributes["COLOR_0"] = json!(g.vec4(&opacity));
        }
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
            // Sekiro's semantic-6 vector is the UV bitangent, not glTF's U tangent.
            // Mirror it with the normal, then reconstruct U using the mirrored handedness.
            // Exporting the stored vector directly rotates normal-map detail by 90 degrees.
            if (choice.normal.is_some() || choice.layers.is_some())
                && let Some(t) = v.tangents.first()
                && t.len() == count
            {
                let tangents: Vec<[f32; 4]> = t
                    .iter()
                    .zip(&normals)
                    .map(|(t, n)| {
                        let n = Vec3::from(*n);
                        let bitangent = Vec3::new(-t[0], t[1], t[2]);
                        let w = if t[3] < 0.0 { 1.0 } else { -1.0 };
                        let t3 = (n.cross(bitangent) * w)
                            .try_normalize()
                            .unwrap_or_else(|| n.any_orthonormal_vector());
                        [t3.x, t3.y, t3.z, w]
                    })
                    .collect();
                attributes["TANGENT"] = json!(g.vec4(&tangents));
            }
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
            "doubleSided": !lod0.cull_backfaces || choice.double_sided,
        });
        if let Some(layers) = &choice.layers {
            let mut extras = json!({"version": 1});
            for (key, r) in [
                ("overlay", &layers.overlay),
                ("overlay_normal", &layers.overlay_normal),
                ("snow", &layers.snow),
                ("snow_normal", &layers.snow_normal),
                ("c", &layers.c),
                ("c_normal", &layers.c_normal),
                ("mask", &layers.mask),
            ] {
                if let Some(r) = r
                    && let Some(uri) = texture_uri(&texture_stem(&r.path).to_ascii_lowercase())
                {
                    extras[key] = json!({"uri": uri, "uv": if uv_ok(r.uv) {r.uv} else {0}});
                }
            }
            mat["extras"] = json!({"sekiro_layers": extras});
        }
        if let Some(r) = &choice.normal
            && let Some((t, uv)) = texture(r, &mut g)
        {
            mat["normalTexture"] = json!({ "index": t, "texCoord": uv });
        }
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

/// Texture names (lower case) a FLVER's visible materials would use, and whether each is a
/// normal map.
pub fn wanted_textures(flver: &flver::Flver, mtds: &Mtds) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    for mesh in &flver.meshes {
        let material = &flver.materials[mesh.material];
        let choice = choose_material(material, mtds.get(&material.mtd));
        if skip_material(&choice.mtd) {
            continue;
        }
        if let Some(r) = choice.albedo {
            out.push((texture_stem(&r.path).to_ascii_lowercase(), false));
        }
        if let Some(r) = choice.normal {
            out.push((texture_stem(&r.path).to_ascii_lowercase(), true));
        }
        if let Some(l) = choice.layers {
            for (r, normal) in [
                (l.overlay, false),
                (l.overlay_normal, true),
                (l.snow, false),
                (l.snow_normal, true),
                (l.c, false),
                (l.c_normal, true),
                (l.mask, false),
            ] {
                if let Some(r) = r {
                    out.push((texture_stem(&r.path).to_ascii_lowercase(), normal));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn mismatched_foliage_sampler_names_use_primary_mtd_albedo_and_cutout() {
        let mat = flver::Material {
            name: "leaves".into(),
            mtd: "leaves.mtd".into(),
            index: 0,
            textures: vec![flver::Texture {
                param: "Character_Wind_Texture2D_2_AlbedoMap".into(),
                path: String::new(),
                scale: [1., 1.],
            }],
        };
        let mtd = mtd::Mtd {
            shader: "Character[Gradient]".into(),
            description: String::new(),
            params: vec![mtd::Param {
                name: "g_AlphaRef".into(),
                kind: "int".into(),
                value: mtd::ParamValue::Ints(vec![128]),
            }],
            textures: vec![mtd::Texture {
                kind: "Character_Gradient_Texture2D_1_AlbedoMap_0".into(),
                uv_number: 1,
                shader_data_index: 0,
                path: "leaves_a.tif".into(),
            }],
        };
        let choice = choose_material(&mat, Some(&mtd));
        assert_eq!(choice.albedo.unwrap().path, "leaves_a.tif");
        assert_eq!(choice.alpha_mode.unwrap().0, "MASK");
        assert!(choice.double_sided);
    }

    #[test]
    fn multiple_materials_choose_base_slot_before_snow() {
        let mat = flver::Material {
            name: "snow_a".into(),
            mtd: "multiple.mtd".into(),
            index: 0,
            textures: vec![],
        };
        let mtd = mtd::Mtd {
            shader: "M[Multiple].spx".into(),
            description: String::new(),
            params: vec![],
            textures: [(12, "snow_a.tif"), (8, "stone_a.tif"), (5, "moss_a.tif")]
                .into_iter()
                .map(|(slot, path)| mtd::Texture {
                    kind: format!("M_Multiple__snp_Texture2D_{slot}_AlbedoMap_0"),
                    uv_number: 1,
                    shader_data_index: 0,
                    path: path.into(),
                })
                .collect(),
        };
        let choice = choose_material(&mat, Some(&mtd));
        assert_eq!(choice.albedo.unwrap().path, "stone_a.tif");
        let layers = choice.layers.unwrap();
        assert_eq!(layers.snow.unwrap().path, "snow_a.tif");
        assert_eq!(layers.overlay.unwrap().path, "moss_a.tif");
    }

    #[test]
    fn cached_valley_alpha_materials_resolve_real_textures() {
        let raw = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cache/raw");
        if !raw.is_dir() {
            return;
        }
        let mtds = super::super::load_mtds(&raw).unwrap();
        for name in ["M[m11_00]_Log1.mtd", "M[m11_00]_Rock4.mtd"] {
            if let Some(m) = mtds.0.get(&name.to_ascii_lowercase()) {
                println!("LAYER VARIANT {name} {}", m.shader);
                for t in &m.textures {
                    println!("VARIANT SLOT {} {}", t.kind, t.path);
                }
            }
        }
        for model in ["409110", "000064", "101100"] {
            let path = raw.join(format!(
                "map/m17_00_00_00/m17_00_00_00_{model}.mapbnd.d/m17_00_00_00_{model}.flver"
            ));
            if !path.is_file() {
                continue;
            }
            let f = flver::parse(&std::fs::read(path).unwrap()).unwrap();
            if model == "101100" {
                let built = build(&f, &mtds, &|_| None).unwrap();
                let len = u32::from_le_bytes(built.glb[12..16].try_into().unwrap()) as usize;
                let doc: Value = serde_json::from_slice(&built.glb[20..20 + len]).unwrap();
                let bin = &built.glb[28 + len..];
                let mut feathered = false;
                for primitive in doc["meshes"][0]["primitives"].as_array().unwrap() {
                    let material =
                        &doc["materials"][primitive["material"].as_u64().unwrap() as usize];
                    if material["alphaMode"] != "BLEND" {
                        continue;
                    }
                    let accessor = &doc["accessors"]
                        [primitive["attributes"]["COLOR_0"].as_u64().unwrap() as usize];
                    let view =
                        &doc["bufferViews"][accessor["bufferView"].as_u64().unwrap() as usize];
                    let offset = view["byteOffset"].as_u64().unwrap() as usize;
                    let count = accessor["count"].as_u64().unwrap() as usize;
                    for vertex in bin[offset..offset + count * 16].as_chunks::<16>().0 {
                        let rgba: Vec<_> = vertex
                            .as_chunks::<4>()
                            .0
                            .iter()
                            .map(|f| f32::from_le_bytes(*f))
                            .collect();
                        assert_eq!(&rgba[..3], &[1., 1., 1.]);
                        feathered |= rgba[3] < 1.;
                    }
                }
                assert!(feathered, "snow/ground decals must retain vertex opacity");
            }
            for mat in &f.materials {
                let mtd = mtds.get(&mat.mtd);
                let choice = choose_material(mat, mtd);
                if choice.alpha_mode.is_some() || choice.mtd.contains("Snow") {
                    assert!(choice.albedo.is_some());
                    if choice.alpha_mode.is_some_and(|a| a.0 == "MASK") {
                        assert!(choice.double_sided);
                    }
                    println!(
                        "VALLEY {} {} albedo={:?} alpha={:?} shader={:?}",
                        model,
                        choice.mtd,
                        choice.albedo,
                        choice.alpha_mode,
                        mtd.map(|m| &m.shader)
                    );
                    for t in &mtd.unwrap().textures {
                        println!("SLOT {} {}", t.kind, t.path);
                    }
                    for mesh in f
                        .meshes
                        .iter()
                        .filter(|mesh| f.materials[mesh.material].name == mat.name)
                    {
                        for colors in &mesh.vertices.colors {
                            println!(
                                "VERTEX COLOR {:?}..{:?}",
                                (0..4)
                                    .map(|c| colors
                                        .iter()
                                        .map(|v| v[c])
                                        .fold(f32::INFINITY, f32::min))
                                    .collect::<Vec<_>>(),
                                (0..4)
                                    .map(|c| colors
                                        .iter()
                                        .map(|v| v[c])
                                        .fold(f32::NEG_INFINITY, f32::max))
                                    .collect::<Vec<_>>()
                            );
                        }
                    }
                    if let Some(a) = choice.albedo {
                        let path = raw.parent().unwrap().join(format!(
                            "maps/m17_00_00_00/models/tex/{}.dds",
                            texture_stem(&a.path).to_ascii_lowercase()
                        ));
                        if path.is_file() {
                            let dds = std::fs::read(path).unwrap();
                            let h = sekiro_formats::tpf::Dds::parse(&dds).unwrap();
                            let rgba = crate::models::textures::decode_rgba(&h, &dds).unwrap();
                            println!(
                                "ALPHA {:?}: transparent={}/{}",
                                h.format,
                                rgba.as_chunks::<4>()
                                    .0
                                    .iter()
                                    .filter(|p| p[3] < 128)
                                    .count(),
                                rgba.len() / 4
                            );
                        }
                    }
                }
            }
        }
    }
}

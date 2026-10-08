//! `sekiro-extract models`: FLVER + TPF + MTD to skinned, textured GLB files in cache/models.
//!
//! Coordinate convention (see docs/FORMATS.md): FromSoftware data is left-handed with +Y up.
//! Every position, direction and matrix is mirrored across X (`x -> -x`, `M -> S M S` with
//! `S = diag(-1, 1, 1)`), which gives right-handed +Y-up data for glTF and Bevy. Index order is
//! kept: FromSoftware front faces are clockwise in their left-handed space, which the mirror
//! turns into the counter-clockwise front faces glTF expects (verified on single-sided meshes).

mod glb;
mod info;
mod textures;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use glam::{Mat4, Vec3};
use sekiro_formats::{flver, mtd};
use serde_json::{Value, json};

pub use info::info;

use crate::paramlite::{Def, Param};
use glb::Glb;
use textures::{TextureStore, Usage};

/// Models exported when no ids are given.
const DEFAULT_IDS: &[&str] = &["c1010", "c0000"];

pub fn export(cache: &Path, ids: &[String], verbose: bool) -> Result<()> {
    let raw = cache.join("raw");
    let out = cache.join("models");
    std::fs::create_dir_all(&out)?;
    let ids: Vec<String> = if ids.is_empty() {
        DEFAULT_IDS.iter().map(|s| s.to_string()).collect()
    } else {
        ids.to_vec()
    };
    let params = Params::open(cache)?;
    for id in &ids {
        eprintln!("{id}:");
        let model = if id == "c0000" {
            wolf(&raw, &params)?
        } else {
            npc(&raw, &params, id)?
        };
        for note in &model.notes {
            eprintln!("  {note}");
        }
        let bytes = build(&raw, &model, verbose)?;
        let path = out.join(format!("{id}.glb"));
        std::fs::write(&path, &bytes)?;
        eprintln!("  wrote {} ({} KiB)", path.display(), bytes.len() / 1024);
    }
    Ok(())
}

/// A model to export: one skeleton plus the parts skinned to it.
struct Model {
    skeleton: flver::Flver,
    parts: Vec<Part>,
    /// Human-readable record of the choices made (variant masks, equipment).
    notes: Vec<String>,
}

struct Part {
    label: String,
    flver: flver::Flver,
    tpfs: Vec<PathBuf>,
    /// Per-mesh visibility.
    visible: Vec<bool>,
    /// Skeleton node that this part's parentless bones hang from (weapons).
    attach: Option<String>,
}

/// The `#NN#` display-mask group a material belongs to, if any.
fn mask_group(material_name: &str) -> Option<usize> {
    let rest = material_name.strip_prefix('#')?;
    let end = rest.find('#')?;
    rest[..end].parse().ok()
}

fn read_flver(path: &Path) -> Result<flver::Flver> {
    let data = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    flver::parse(&data).with_context(|| format!("parsing {}", path.display()))
}

/// Finds a file in `dir` by case-insensitive name.
fn find_ci(dir: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir).ok()?.find_map(|e| {
        let e = e.ok()?;
        e.file_name()
            .to_str()?
            .eq_ignore_ascii_case(name)
            .then(|| e.path())
    })
}

struct Params {
    root: PathBuf,
    defs: PathBuf,
}

impl Params {
    fn open(cache: &Path) -> Result<Self> {
        Ok(Self {
            root: cache.join("raw/param/gameparam/gameparam.parambnd.d"),
            defs: cache.join("refs/paramdex/Defs"),
        })
    }

    fn load(&self, name: &str) -> Result<(Param, Def)> {
        Ok((
            Param::load(&self.root.join(format!("{name}.param")))?,
            Def::load(&self.defs.join(format!("{name}.xml")))?,
        ))
    }
}

/// An enemy/NPC character: its chrbnd FLVER, textures from its texbnd, and the variant meshes
/// enabled by the NpcParam row `<model number> * 10000` (the first row for that model).
fn npc(raw: &Path, params: &Params, id: &str) -> Result<Model> {
    let chrbnd = raw.join(format!("chr/{id}.chrbnd.d"));
    let flver = read_flver(&chrbnd.join(format!("{id}.flver")))?;
    let mut notes = Vec::new();
    let number: i32 = id.trim_start_matches('c').parse()?;
    let (npc_param, npc_def) = params.load("NpcParam")?;
    let row_id = number * 10000;
    let masks: Option<Vec<bool>> = match npc_param.row(&npc_def, row_id) {
        Some(row) => Some(
            (0..32)
                .map(|i| row.int(&format!("modelDispMask{i}")).map(|v| v != 0))
                .collect::<Result<_>>()?,
        ),
        None => None,
    };
    match &masks {
        Some(m) => notes.push(format!(
            "NpcParam {row_id} display masks on: {:?}",
            m.iter()
                .enumerate()
                .filter(|(_, on)| **on)
                .map(|(i, _)| i)
                .collect::<Vec<_>>()
        )),
        None => notes.push(format!(
            "NpcParam {row_id} missing; showing only unmasked meshes"
        )),
    }
    let visible = flver
        .meshes
        .iter()
        .map(|m| {
            let name = &flver.materials[m.material].name;
            match mask_group(name) {
                Some(g) => masks
                    .as_ref()
                    .is_some_and(|m| m.get(g).copied().unwrap_or(false)),
                None => true,
            }
        })
        .collect();
    let mut tpfs = vec![raw.join(format!("chr/{id}.texbnd.d/{id}.tpf"))];
    tpfs.push(chrbnd.join(format!("{id}.tpf")));
    Ok(Model {
        skeleton: flver.clone(),
        parts: vec![Part {
            label: id.to_string(),
            flver,
            tpfs,
            visible,
            attach: None,
        }],
        notes,
    })
}

/// The ordinary Wolf: c0000's skeleton, the protector parts from the new-game CharaInitParam
/// row, the face part, and the starting katana with its scabbard.
fn wolf(raw: &Path, params: &Params) -> Result<Model> {
    const CHARA_INIT_ROW: i32 = 10010; // "Castle": ordinary gear, katana, prosthetic
    let skeleton = read_flver(&raw.join("chr/c0000.chrbnd.d/c0000.flver"))?;
    let mut notes = Vec::new();
    let (init, init_def) = params.load("CharaInitParam")?;
    let (prot, prot_def) = params.load("EquipParamProtector")?;
    let (wep, wep_def) = params.load("EquipParamWeapon")?;
    let row = init
        .row(&init_def, CHARA_INIT_ROW)
        .context("CharaInitParam new-game row missing")?;

    let mut part_files: Vec<(String, Option<String>)> = Vec::new();
    // Display-mask groups hidden by the equipped protectors.
    let mut hidden = [false; 96];
    for slot in ["equip_Helm", "equip_Armer", "equip_Gaunt", "equip_Leg"] {
        let id = row.int(slot)? as i32;
        let Some(p) = prot.row(&prot_def, id) else {
            notes.push(format!("{slot} = {id}: no EquipParamProtector row"));
            continue;
        };
        let model = p.int("equipModelId")?;
        let prefix = match (
            p.int("headEquip")?,
            p.int("bodyEquip")?,
            p.int("armEquip")?,
            p.int("legEquip")?,
        ) {
            (_, 1, _, _) => "BD",
            (_, _, 1, _) => "AM",
            (_, _, _, 1) => "LG",
            _ => "HD",
        };
        for (g, h) in hidden.iter_mut().enumerate() {
            if p.int(&format!("invisibleFlag{g:02}")).unwrap_or(0) != 0 {
                *h = true;
            }
        }
        let mut name = format!("{prefix}_M_{model:04}");
        // Sekiro head protectors name face models (FC_M_*), which carry the head and hair.
        if prefix == "HD"
            && !raw
                .join(format!("parts/{}.partsbnd.d", name.to_ascii_lowercase()))
                .exists()
        {
            name = format!("FC_M_{model:04}");
        }
        notes.push(format!("{slot} = protector {id} -> model {name}"));
        part_files.push((name, None));
    }

    let wep_id = row.int("equip_Wep_Right")? as i32;
    match wep.row(&wep_def, wep_id) {
        Some(w) => {
            let model = w.int("equipModelId")?;
            let name = format!("WP_A_{model:04}");
            notes.push(format!(
                "equip_Wep_Right = weapon {wep_id} -> {name} in R_Weapon, scabbard {name}_1 on Sheath"
            ));
            part_files.push((name.clone(), Some("R_Weapon".to_string())));
            part_files.push((format!("{name}_1"), Some("Sheath".to_string())));
        }
        None => notes.push(format!(
            "equip_Wep_Right = {wep_id}: no EquipParamWeapon row"
        )),
    }
    let hidden_groups: Vec<usize> = (0..96).filter(|&g| hidden[g]).collect();
    notes.push(format!(
        "display-mask groups hidden by protectors: {hidden_groups:?}"
    ));

    let mut parts = Vec::new();
    for (name, attach) in part_files {
        let base = name.split('_').take(3).collect::<Vec<_>>().join("_");
        let dir = raw.join(format!("parts/{}.partsbnd.d", base.to_ascii_lowercase()));
        let Some(path) = find_ci(&dir, &format!("{name}.flver")) else {
            notes.push(format!("{name}: no FLVER in {}, skipped", dir.display()));
            continue;
        };
        let flver = read_flver(&path)?;
        let visible = flver
            .meshes
            .iter()
            .map(|m| match mask_group(&flver.materials[m.material].name) {
                Some(g) => !hidden.get(g).copied().unwrap_or(false),
                None => true,
            })
            .collect();
        let tpfs = std::fs::read_dir(&dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("tpf")))
            .collect();
        parts.push(Part {
            label: name,
            flver,
            tpfs,
            visible,
            attach,
        });
    }
    Ok(Model {
        skeleton,
        parts,
        notes,
    })
}

/// Mirror across X: converts FromSoftware's left-handed space to right-handed.
fn mirror_point(p: [f32; 3]) -> [f32; 3] {
    [-p[0], p[1], p[2]]
}

fn mirror_matrix(m: Mat4) -> Mat4 {
    let s = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
    s * m * s
}

/// Bind-pose world matrices of a FLVER node table, in FromSoftware space.
fn world_matrices(nodes: &[flver::Node]) -> Vec<Mat4> {
    let mut world: Vec<Option<Mat4>> = vec![None; nodes.len()];
    fn resolve(i: usize, nodes: &[flver::Node], world: &mut [Option<Mat4>], depth: usize) -> Mat4 {
        if let Some(m) = world[i] {
            return m;
        }
        let local = nodes[i].local_matrix();
        let p = nodes[i].parent;
        let m = if p >= 0 && depth < nodes.len() {
            resolve(p as usize, nodes, world, depth + 1) * local
        } else {
            local
        };
        world[i] = Some(m);
        m
    }
    (0..nodes.len())
        .map(|i| resolve(i, nodes, &mut world, 0))
        .collect()
}

struct Node {
    name: String,
    local: Mat4,
    children: Vec<usize>,
    mesh: Option<usize>,
    skin: Option<usize>,
}

struct Scene {
    nodes: Vec<Node>,
    roots: Vec<usize>,
    by_name: HashMap<String, usize>,
}

impl Scene {
    fn add(&mut self, name: &str, local: Mat4, parent: Option<usize>) -> usize {
        let i = self.nodes.len();
        self.nodes.push(Node {
            name: name.to_string(),
            local,
            children: Vec::new(),
            mesh: None,
            skin: None,
        });
        match parent {
            Some(p) => self.nodes[p].children.push(i),
            None => self.roots.push(i),
        }
        i
    }
}

/// Maps a part's node indices onto scene nodes, adding nodes the skeleton lacks.
struct PartNodes<'a> {
    part: &'a Part,
    map: HashMap<usize, usize>,
}

impl PartNodes<'_> {
    fn node(&mut self, scene: &mut Scene, i: usize) -> usize {
        if let Some(&n) = self.map.get(&i) {
            return n;
        }
        let src = &self.part.flver.nodes[i];
        let n = match scene.by_name.get(&src.name) {
            Some(&n) => n,
            None => {
                let parent = if src.parent >= 0 {
                    Some(self.node(scene, src.parent as usize))
                } else {
                    self.part
                        .attach
                        .as_ref()
                        .and_then(|a| scene.by_name.get(a).copied())
                };
                scene.add(&src.name, mirror_matrix(src.local_matrix()), parent)
            }
        };
        self.map.insert(i, n);
        n
    }
}

/// A game texture path and the UV channel it is sampled with.
type TextureRef = (String, usize);

/// Picks the base colour and normal map paths (with UV channel) for a material.
///
/// Sekiro materials list many layers. The main albedo is the slot whose name ends in
/// `AlbedoMap_0` when there is one, otherwise the first albedo that is not an overlay (shared
/// `FC_A_0000_*` detail maps, damage, expression or skin-detail layers). The normal map is the
/// one named like the chosen albedo (`x_a` -> `x_n`), falling back to the same rules.
fn material_textures(
    material: &flver::Material,
    mtd: Option<&mtd::Mtd>,
) -> (Option<TextureRef>, Option<TextureRef>) {
    struct Slot {
        param: String,
        path: String,
        uv: usize,
    }
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
            .map(|m| (m.uv_number - 1).max(0) as usize)
            .unwrap_or(0);
        let param = t.param.to_ascii_lowercase();
        let slot = Slot { param, path, uv };
        if slot.param.contains("albedomap") || slot.param.contains("diffuse") {
            albedo.push(slot);
        } else if slot.param.contains("normalmap") || slot.param.contains("bumpmap") {
            normal.push(slot);
        }
    }
    let overlay = |s: &Slot| {
        let stem = textures::texture_stem(&s.path).to_ascii_lowercase();
        stem.starts_with("fc_a_0000")
            || ["damage", "expression", "_skin_"]
                .iter()
                .any(|w| stem.contains(w))
    };
    let main = |list: &[Slot], suffix: &str| -> Option<usize> {
        list.iter()
            .position(|s| s.param.ends_with(suffix) && !overlay(s))
            .or_else(|| list.iter().position(|s| !overlay(s)))
            .or(if list.is_empty() { None } else { Some(0) })
    };
    let a = main(&albedo, "albedomap_0");
    let n = a
        .and_then(|a| {
            let stem = textures::texture_stem(&albedo[a].path).to_ascii_lowercase();
            let want = format!("{}_n", stem.strip_suffix("_a")?);
            normal
                .iter()
                .position(|s| textures::texture_stem(&s.path).eq_ignore_ascii_case(&want))
        })
        .or_else(|| main(&normal, "normalmap_4"));
    let out = |list: &[Slot], i: Option<usize>| i.map(|i| (list[i].path.clone(), list[i].uv));
    (out(&albedo, a), out(&normal, n))
}

fn build(raw: &Path, model: &Model, verbose: bool) -> Result<Vec<u8>> {
    let mut g = Glb::default();
    let mut store = TextureStore::new(raw);
    let mtd_dir = raw.join("mtd/allmaterialbnd.mtdbnd.d");
    let mtd_index: HashMap<String, PathBuf> = std::fs::read_dir(&mtd_dir)
        .with_context(|| format!("reading {}", mtd_dir.display()))?
        .filter_map(|e| {
            let e = e.ok()?;
            Some((e.file_name().to_str()?.to_ascii_lowercase(), e.path()))
        })
        .collect();

    // Skeleton nodes, in file order so node i of the skeleton FLVER is scene node i.
    let mut scene = Scene {
        nodes: Vec::new(),
        roots: Vec::new(),
        by_name: HashMap::new(),
    };
    for n in &model.skeleton.nodes {
        scene.add(&n.name, mirror_matrix(n.local_matrix()), None);
    }
    scene.roots.clear();
    for (i, n) in model.skeleton.nodes.iter().enumerate() {
        if n.parent >= 0 {
            scene.nodes[n.parent as usize].children.push(i);
        } else {
            scene.roots.push(i);
        }
        scene.by_name.entry(n.name.clone()).or_insert(i);
    }

    let mut texture_ids: HashMap<(String, Usage), Option<usize>> = HashMap::new();
    let mut stats = Vec::new();
    for part in &model.parts {
        for t in &part.tpfs {
            store.load_tpf(t)?;
        }
        let world = world_matrices(&part.flver.nodes);
        let mut nodes = PartNodes {
            part,
            map: HashMap::new(),
        };
        let mut joint_slots: HashMap<usize, u16> = HashMap::new();
        let mut joints: Vec<usize> = Vec::new();
        let mut ibms: Vec<Mat4> = Vec::new();
        let mut primitives = Vec::new();
        let mut shown = 0;
        for (mi, mesh) in part.flver.meshes.iter().enumerate() {
            if !part.visible[mi] {
                continue;
            }
            let Some(lod0) = mesh.lod0() else { continue };
            let v = &mesh.vertices;
            let count = v.len();
            if count == 0 {
                continue;
            }
            shown += 1;
            let mut slot_of = |src: usize, scene: &mut Scene| -> u16 {
                if let Some(&s) = joint_slots.get(&src) {
                    return s;
                }
                let node = nodes.node(scene, src);
                joints.push(node);
                ibms.push(mirror_matrix(world[src]).inverse());
                let s = (joints.len() - 1) as u16;
                joint_slots.insert(src, s);
                s
            };
            let to_node = |b: u16| -> usize {
                if mesh.bone_indices.is_empty() {
                    b as usize
                } else {
                    mesh.bone_indices
                        .get(b as usize)
                        .copied()
                        .unwrap_or(0)
                        .max(0) as usize
                }
            };
            let default_bone = if mesh.node >= 0 {
                mesh.node as usize
            } else {
                0
            };
            let mut joint_data = Vec::with_capacity(count);
            let mut weight_data = Vec::with_capacity(count);
            for k in 0..count {
                let (idx, w) = match (v.bone_indices.get(k), v.bone_weights.get(k)) {
                    (Some(i), Some(w)) if mesh.dynamic => (*i, *w),
                    _ => ([0; 4], [0.0; 4]),
                };
                let total: f32 = w.iter().sum();
                if total <= 1e-6 {
                    let s = slot_of(default_bone, &mut scene);
                    joint_data.push([s, s, s, s]);
                    weight_data.push([1.0, 0.0, 0.0, 0.0]);
                    continue;
                }
                let mut j = [0u16; 4];
                let mut ww = [0f32; 4];
                let first = (0..4).find(|&c| w[c] > 0.0).unwrap_or(0);
                let fallback = slot_of(to_node(idx[first]), &mut scene);
                for c in 0..4 {
                    if w[c] > 0.0 {
                        j[c] = slot_of(to_node(idx[c]), &mut scene);
                        ww[c] = w[c] / total;
                    } else {
                        j[c] = fallback;
                    }
                }
                joint_data.push(j);
                weight_data.push(ww);
            }
            let positions: Vec<[f32; 3]> = v.positions.iter().map(|&p| mirror_point(p)).collect();
            let normals: Vec<[f32; 3]> = v
                .normals
                .iter()
                .map(|&n| {
                    let n = Vec3::from(mirror_point(n));
                    n.try_normalize().unwrap_or(Vec3::Y).into()
                })
                .collect();
            let indices = lod0.triangles(count);
            if indices.iter().any(|&i| i as usize >= count) {
                bail!("{}: mesh {mi} index out of range", part.label);
            }
            let mut attributes = json!({
                "POSITION": g.vec3(&positions, true),
                "JOINTS_0": g.joints(&joint_data),
                "WEIGHTS_0": g.vec4(&weight_data),
            });
            if normals.len() == count {
                attributes["NORMAL"] = json!(g.vec3(&normals, false));
            }
            for (c, uv) in v.uvs.iter().take(2).enumerate() {
                if uv.len() == count {
                    attributes[format!("TEXCOORD_{c}")] = json!(g.vec2(uv));
                }
            }

            let mtd_name = textures::texture_stem(&part.flver.materials[mesh.material].mtd)
                .to_ascii_lowercase();
            let mtd = mtd_index
                .get(&format!("{mtd_name}.mtd"))
                .map(|p| -> Result<mtd::Mtd> { Ok(mtd::parse(&std::fs::read(p)?)?) })
                .transpose()?;
            let mut ctx = MaterialContext {
                g: &mut g,
                store: &mut store,
                texture_ids: &mut texture_ids,
            };
            let mat = material_json(part, mi, lod0, mtd.as_ref(), &mtd_name, &mut ctx)?;
            if verbose {
                eprintln!(
                    "    {}.{}: {}",
                    part.label,
                    primitives.len(),
                    mat["name"].as_str().unwrap_or_default()
                );
            }
            g.materials.push(mat);
            let material = g.materials.len() - 1;
            primitives.push(json!({
                "attributes": attributes,
                "indices": g.indices(&indices),
                "material": material,
            }));
        }
        if verbose {
            // How far each part's bind pose is from the skeleton's rest pose, per shared bone.
            let skel_world = world_matrices(&model.skeleton.nodes);
            for (&src, &slot) in &joint_slots {
                let name = &part.flver.nodes[src].name;
                let Some(&s) = scene.by_name.get(name) else {
                    continue;
                };
                if s >= skel_world.len() {
                    continue;
                }
                let diff = world[src] - skel_world[s];
                let d = diff
                    .to_cols_array()
                    .iter()
                    .fold(0.0f32, |m, v| m.max(v.abs()));
                if d > 0.002 {
                    eprintln!(
                        "    {}: bone {name} (joint {slot}) bind differs from skeleton (max matrix delta {d:.3})",
                        part.label
                    );
                }
            }
        }
        stats.push(format!(
            "{}: {shown}/{} meshes, {} joints",
            part.label,
            part.flver.meshes.len(),
            joints.len()
        ));
        if primitives.is_empty() {
            continue;
        }
        let ibm = g.mat4s(&ibms);
        g.skins.push(json!({
            "name": part.label,
            "joints": joints,
            "inverseBindMatrices": ibm,
        }));
        g.meshes
            .push(json!({ "name": part.label, "primitives": primitives }));
        let n = scene.add(&part.label, Mat4::IDENTITY, None);
        scene.nodes[n].mesh = Some(g.meshes.len() - 1);
        scene.nodes[n].skin = Some(g.skins.len() - 1);
    }
    for s in &stats {
        eprintln!("  {s}");
    }

    for n in &scene.nodes {
        let (s, r, t) = n.local.to_scale_rotation_translation();
        let mut node = json!({
            "name": n.name,
            "translation": t.to_array(),
            "rotation": r.normalize().to_array(),
            "scale": s.to_array(),
        });
        if !n.children.is_empty() {
            node["children"] = json!(n.children);
        }
        if let Some(m) = n.mesh {
            node["mesh"] = json!(m);
        }
        if let Some(s) = n.skin {
            node["skin"] = json!(s);
        }
        g.nodes.push(node);
    }

    let dummies: Vec<Value> = model
        .skeleton
        .dummies
        .iter()
        .map(|d| {
            let name = |i: i16| {
                (i >= 0)
                    .then(|| model.skeleton.nodes.get(i as usize).map(|n| n.name.clone()))
                    .flatten()
            };
            json!({
                "id": d.reference_id,
                "attach": name(d.attach_bone),
                "parent": name(d.parent_bone),
                "position": mirror_point(d.position),
                "forward": mirror_point(d.forward),
                "upward": mirror_point(d.upward),
                "useUpward": d.use_upward,
            })
        })
        .collect();
    let extras = json!({
        "source": "sekiro-extract models",
        "convention": "FromSoftware x mirrored (x -> -x); see docs/FORMATS.md",
        "notes": model.notes,
        "dummies": dummies,
    });
    Ok(g.finish(&scene.roots, extras))
}

struct MaterialContext<'a> {
    g: &'a mut Glb,
    store: &'a mut TextureStore,
    texture_ids: &'a mut HashMap<(String, Usage), Option<usize>>,
}

impl MaterialContext<'_> {
    fn texture(&mut self, path: &str, usage: Usage) -> Result<Option<usize>> {
        let key = (textures::texture_stem(path).to_ascii_lowercase(), usage);
        if let Some(&t) = self.texture_ids.get(&key) {
            return Ok(t);
        }
        let t = self
            .store
            .png(path, usage)?
            .map(|png| self.g.texture(&png, &key.0));
        self.texture_ids.insert(key, t);
        Ok(t)
    }
}

/// A glTF material for one FLVER mesh. Named `m<mesh> <material> | <owner node> | <mtd>` so
/// variants can be identified (and filtered in the viewer).
fn material_json(
    part: &Part,
    mesh_index: usize,
    lod0: &flver::FaceSet,
    mtd: Option<&mtd::Mtd>,
    mtd_name: &str,
    ctx: &mut MaterialContext,
) -> Result<Value> {
    let mesh = &part.flver.meshes[mesh_index];
    let fm = &part.flver.materials[mesh.material];
    let owner = usize::try_from(mesh.node)
        .ok()
        .and_then(|n| part.flver.nodes.get(n))
        .map(|n| n.name.as_str())
        .unwrap_or("-");
    let (albedo, normal) = material_textures(fm, mtd);
    let mut pbr = json!({ "metallicFactor": 0.0, "roughnessFactor": 0.75 });
    if let Some((path, uv)) = &albedo
        && let Some(t) = ctx.texture(path, Usage::Color)?
    {
        pbr["baseColorTexture"] = json!({ "index": t, "texCoord": uv });
    }
    let mut mat = json!({
        "name": format!("m{mesh_index} {} | {owner} | {mtd_name}", fm.name),
        "pbrMetallicRoughness": pbr,
        "doubleSided": !lod0.cull_backfaces,
    });
    if let Some((path, uv)) = &normal
        && let Some(t) = ctx.texture(path, Usage::Normal)?
    {
        mat["normalTexture"] = json!({ "index": t, "texCoord": uv });
    }
    if let Some(m) = mtd {
        let int = |name: &str| match m.param(name) {
            Some(mtd::ParamValue::Ints(v)) => v.first().copied().unwrap_or(0),
            _ => 0,
        };
        let (alpha_ref, blend) = (int("g_AlphaRef"), int("g_BlendMode"));
        if blend == 2 || blend >= 4 {
            mat["alphaMode"] = json!("BLEND");
        } else if alpha_ref > 0 || blend == 1 {
            mat["alphaMode"] = json!("MASK");
            mat["alphaCutoff"] = json!((alpha_ref.max(1) as f32 / 255.0).max(0.3));
        }
    }
    Ok(mat)
}

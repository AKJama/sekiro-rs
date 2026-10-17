//! Grass clumps share mesh/material handles for Bevy's automatic GPU instancing.
//! Instances are children of their authored terrain part, including its visibility gates.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result, ensure};
use bevy::asset::UntypedAssetId;
use bevy::gltf::{
    GltfExtras, GltfMaterialExtras, GltfMaterialName, GltfMesh, GltfMeshExtras, GltfNode,
};
use bevy::prelude::*;
use serde_json::Value;

use crate::map::world::BlockPart;

#[derive(Component)]
struct GrassNoShadow;

#[derive(Resource, Default)]
struct Grass {
    blocks: BTreeMap<String, Block>,
    attached: HashSet<Entity>,
    /// Parentless roots without `BlockPart`, matched once against the primary block's bindings.
    checked: HashMap<Entity, Checked>,
    primary: String,
}

/// What a parentless root was matched with: its scene and pose then, and the bound part.
struct Checked {
    scene: UntypedAssetId,
    pose: Transform,
    part: Option<usize>,
}

struct Block {
    manifest: Value,
    prototypes: BTreeMap<String, Prototype>,
    /// Bindings by parent model: parent transform and part index, in manifest order.
    parents: HashMap<String, Vec<(Mat4, usize)>>,
}

struct Prototype {
    gltf: Handle<Gltf>,
    path: String,
    roots: Vec<usize>,
    primitives: Option<Vec<Primitive>>,
    barrier: Option<Entity>,
}

#[derive(Clone)]
struct Primitive {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    transform: Transform,
    extras: Option<GltfExtras>,
    mesh_extras: Option<GltfExtras>,
    material_extras: Option<GltfExtras>,
    material_name: Option<String>,
}

pub(crate) fn install(app: &mut App, dir: &Path, prefix: &str) -> Result<()> {
    if std::env::var_os("SEKIRO_NO_GRASS").is_some() {
        return Ok(());
    }
    let maps = dir.parent().context("map directory")?;
    let primary = dir.file_name().and_then(|s| s.to_str()).context("map ID")?;
    if !matches!(primary, "m11_00_00_00" | "m11_01_00_00") {
        return Ok(());
    }
    let mut grass = Grass {
        primary: primary.to_owned(),
        ..Default::default()
    };
    for id in ["m11_00_00_00", "m11_01_00_00"] {
        let path = maps.join(id).join("grass/grass.json");
        if !path.is_file() {
            continue;
        }
        let manifest: Value = serde_json::from_slice(&std::fs::read(path)?)?;
        ensure!(
            manifest["version"] == 1 && manifest["map"] == id,
            "unsupported grass manifest"
        );
        let block_prefix = if id == primary {
            prefix.to_owned()
        } else if prefix.is_empty() {
            format!("../{id}/")
        } else {
            format!("maps/{id}/")
        };
        let assets = app.world().resource::<AssetServer>();
        let mut prototypes = BTreeMap::new();
        for model in manifest["models"].as_array().context("grass models")? {
            let model = model.as_str().context("grass model name")?;
            let path = format!("{block_prefix}grass/models/{model}.glb");
            let bytes = std::fs::read(maps.join(id).join(format!("grass/models/{model}.glb")))?;
            ensure!(
                bytes.len() >= 20 && &bytes[..4] == b"glTF",
                "invalid grass GLB"
            );
            let length = u32::from_le_bytes(bytes[12..16].try_into()?) as usize;
            let doc: Value =
                serde_json::from_slice(bytes.get(20..20 + length).context("grass GLB JSON")?)?;
            let roots = doc["scenes"][0]["nodes"]
                .as_array()
                .context("grass scene roots")?
                .iter()
                .map(|n| n.as_u64().map(|n| n as usize).context("grass node index"))
                .collect::<Result<_>>()?;
            let gltf = assets
                .load_builder()
                .override_unapproved()
                .load(path.clone());
            prototypes.insert(
                model.to_owned(),
                Prototype {
                    gltf,
                    path,
                    roots,
                    primitives: None,
                    barrier: None,
                },
            );
        }
        grass.blocks.insert(
            id.to_owned(),
            Block {
                parents: index_parents(&manifest)?,
                manifest,
                prototypes,
            },
        );
    }
    app.insert_resource(grass)
        .add_systems(PreUpdate, instantiate)
        .add_systems(PostUpdate, enforce_shadow_flags);
    Ok(())
}

fn transform(row: &Value) -> Result<Transform> {
    Ok(Transform {
        translation: Vec3::from_array(serde_json::from_value(row["translation"].clone())?),
        rotation: Quat::from_array(serde_json::from_value(row["rotation"].clone())?),
        scale: Vec3::from_array(serde_json::from_value(row["scale"].clone())?),
    })
}

/// Index the bindings that name a parent model, for loaders without `BlockPart`.
fn index_parents(manifest: &Value) -> Result<HashMap<String, Vec<(Mat4, usize)>>> {
    let mut parents: HashMap<String, Vec<(Mat4, usize)>> = HashMap::new();
    for binding in manifest["bindings"].as_array().into_iter().flatten() {
        let Some(model) = binding["parent_model"].as_str() else {
            continue;
        };
        let Ok(pose) = transform(&binding["parent_transform"]) else {
            continue;
        };
        ensure!(!model.contains('/'), "grass parent model {model:?}");
        let part = binding["part_index"].as_u64().context("grass part index")? as usize;
        parents
            .entry(model.to_owned())
            .or_default()
            .push((pose.to_matrix(), part));
    }
    Ok(parents)
}

/// The part bound to a root scene at `path` (`.../models/<model>.glb#...`) placed at `pose`.
fn bound_part(
    parents: &HashMap<String, Vec<(Mat4, usize)>>,
    path: &str,
    pose: &Transform,
) -> Option<usize> {
    let pose = pose.to_matrix();
    path.match_indices(".glb#").find_map(|(end, _)| {
        let head = &path[..end];
        let start = head.rfind('/')? + 1;
        if !head[..start].ends_with("models/") {
            return None;
        }
        parents
            .get(&head[start..])?
            .iter()
            .find(|(expected, _)| expected.abs_diff_eq(pose, 1e-5))
            .map(|&(_, part)| part)
    })
}

/// Match parentless roots to bound parts. Each root is matched once, and again only if its scene
/// or pose changes; a root whose scene path is not known yet is tried again next frame.
fn match_roots(
    parents: &HashMap<String, Vec<(Mat4, usize)>>,
    checked: &mut HashMap<Entity, Checked>,
    roots: impl IntoIterator<Item = (Entity, UntypedAssetId, Transform)>,
    mut path: impl FnMut(UntypedAssetId) -> Option<String>,
) -> Vec<(Entity, usize)> {
    let mut matched = Vec::new();
    for (entity, scene, pose) in roots {
        let known = checked
            .get(&entity)
            .filter(|c| c.scene == scene && c.pose == pose);
        let part = match known {
            Some(c) => c.part,
            None => {
                let Some(path) = path(scene) else {
                    continue;
                };
                let part = bound_part(parents, &path, &pose);
                checked.insert(entity, Checked { scene, pose, part });
                part
            }
        };
        if let Some(part) = part {
            matched.push((entity, part));
        }
    }
    matched
}

fn plan(world: &World, prototype: &Prototype) -> Option<Vec<Primitive>> {
    let assets = world.resource::<AssetServer>();
    if !assets.is_loaded_with_dependencies(prototype.gltf.id()) {
        return None;
    }
    let gltf = world.resource::<Assets<Gltf>>().get(&prototype.gltf)?;
    let nodes = world.resource::<Assets<GltfNode>>();
    let meshes = world.resource::<Assets<GltfMesh>>();
    let mut output = Vec::new();
    #[allow(clippy::too_many_arguments)]
    fn visit(
        world: &World,
        gltf: &Gltf,
        nodes: &Assets<GltfNode>,
        meshes: &Assets<GltfMesh>,
        handle: &Handle<GltfNode>,
        parent: Mat4,
        path: &str,
        out: &mut Vec<Primitive>,
    ) -> Option<()> {
        let node = nodes.get(handle)?;
        let matrix = parent * node.transform.to_matrix();
        if let Some(handle) = &node.mesh {
            let mesh = meshes.get(handle)?;
            for p in &mesh.primitives {
                let index = gltf
                    .materials
                    .iter()
                    .position(|m| Some(m) == p.material.as_ref())?;
                let material: Handle<StandardMaterial> = world
                    .resource::<AssetServer>()
                    .load_builder()
                    .override_unapproved()
                    .load(format!("{path}#Material{index}/std"));
                if !world
                    .resource::<Assets<StandardMaterial>>()
                    .contains(&material)
                {
                    return None;
                }
                out.push(Primitive {
                    mesh: p.mesh.clone(),
                    material,
                    transform: Transform::from_matrix(matrix),
                    extras: p.extras.clone(),
                    mesh_extras: mesh.extras.clone(),
                    material_extras: p.material_extras.clone(),
                    material_name: p.material.as_ref().and_then(|m| {
                        gltf.named_materials
                            .iter()
                            .find(|(_, h)| h.id() == m.id())
                            .map(|(n, _)| n.to_string())
                    }),
                });
            }
        }
        for child in &node.children {
            visit(world, gltf, nodes, meshes, child, matrix, path, out)?;
        }
        Some(())
    }
    // Exported clumps use the first authored detail scene, just like LOD0 map geometry.
    for &root in &prototype.roots {
        visit(
            world,
            gltf,
            nodes,
            meshes,
            gltf.nodes.get(root)?,
            Mat4::IDENTITY,
            &prototype.path,
            &mut output,
        )?;
    }
    (!output.is_empty()).then_some(output)
}

fn instantiate(world: &mut World) {
    let _timer = crate::perf_log::SystemTimer::start("grass instantiate");
    let Some(mut grass) = world.remove_resource::<Grass>() else {
        return;
    };
    let mut parents: Vec<_> = world
        .query::<(Entity, &BlockPart)>()
        .iter(world)
        .filter(|(entity, _)| !grass.attached.contains(entity))
        .map(|(entity, part)| (entity, part.block.clone(), part.part_index))
        .collect();
    // Arena and explicitly disconnected map loaders predate BlockPart. Match
    // their outer scene's authored model and transform, without creating new visibility roots.
    if let Some(block) = grass.blocks.get(&grass.primary) {
        let roots: Vec<_> = world
            .query_filtered::<(Entity, &WorldAssetRoot, &Transform), (Without<ChildOf>, Without<BlockPart>)>()
            .iter(world)
            .filter(|(entity, ..)| !grass.attached.contains(entity))
            .map(|(entity, root, pose)| (entity, root.0.id().untyped(), *pose))
            .collect();
        let assets = world.resource::<AssetServer>();
        let matched = match_roots(&block.parents, &mut grass.checked, roots, |scene| {
            assets.get_path(scene).map(|path| path.to_string())
        });
        parents.extend(
            matched
                .into_iter()
                .map(|(entity, part)| (entity, grass.primary.clone(), part)),
        );
    }
    for (parent, id, part_index) in parents {
        let Some(block) = grass.blocks.get_mut(&id) else {
            grass.attached.insert(parent);
            continue;
        };
        let bindings: Vec<_> = block.manifest["bindings"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|b| b["part_index"].as_u64() == Some(part_index as u64))
            .cloned()
            .collect();
        let mut ready = true;
        for binding in bindings {
            let surface = &block.manifest["surfaces"][binding["surface"].as_str().unwrap()];
            let model = surface["model"].as_str().unwrap();
            let prototype = block.prototypes.get_mut(model).unwrap();
            if prototype.primitives.is_none() {
                prototype.primitives = plan(world, prototype);
                if prototype.primitives.is_none() {
                    // The ordinary map readiness walk waits for this nested scene as well.
                    // It is removed before diagnostic geometry is enumerated.
                    if prototype.barrier.is_none() {
                        let handle = world
                            .resource::<AssetServer>()
                            .load_builder()
                            .override_unapproved()
                            .load(GltfAssetLabel::Scene(0).from_asset(prototype.path.clone()));
                        prototype.barrier = Some(
                            world
                                .spawn((
                                    WorldAssetRoot(handle),
                                    Transform::IDENTITY,
                                    Visibility::Hidden,
                                    ChildOf(parent),
                                ))
                                .id(),
                        );
                    }
                    ready = false;
                    break;
                }
                if let Some(barrier) = prototype.barrier.take() {
                    world.despawn(barrier);
                }
            }
        }
        if !ready {
            continue;
        }
        for binding in block.manifest["bindings"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|b| b["part_index"].as_u64() == Some(part_index as u64))
        {
            let surface = &block.manifest["surfaces"][binding["surface"].as_str().unwrap()];
            let prototype = &block.prototypes[surface["model"].as_str().unwrap()];
            let cast_shadow = surface["params"]["castShadow"].as_u64().unwrap_or(0) != 0;
            for placement in surface["placements"].as_array().into_iter().flatten() {
                let placement = transform(placement).expect("validated grass placement");
                for primitive in prototype.primitives.as_ref().unwrap() {
                    let mut entity = world.spawn((
                        Mesh3d(primitive.mesh.clone()),
                        MeshMaterial3d(primitive.material.clone()),
                        placement.mul_transform(primitive.transform),
                        Visibility::Inherited,
                        ChildOf(parent),
                    ));
                    if !cast_shadow {
                        entity.insert((GrassNoShadow, bevy::light::NotShadowCaster));
                    }
                    if let Some(extras) = &primitive.extras {
                        entity.insert(extras.clone());
                    }
                    if let Some(extras) = &primitive.mesh_extras {
                        entity.insert(GltfMeshExtras {
                            value: extras.value.clone(),
                        });
                    }
                    if let Some(extras) = &primitive.material_extras {
                        entity.insert(GltfMaterialExtras {
                            value: extras.value.clone(),
                        });
                    }
                    if let Some(name) = &primitive.material_name {
                        entity.insert(GltfMaterialName(name.clone()));
                    }
                }
            }
        }
        grass.attached.insert(parent);
    }
    world.insert_resource(grass);
}

fn enforce_shadow_flags(
    mut commands: Commands,
    meshes: Query<Entity, (With<GrassNoShadow>, Without<bevy::light::NotShadowCaster>)>,
) {
    // The `--shadow-casters` budget can restore map casters. Native type flags still win.
    for entity in &meshes {
        commands.entity(entity).insert(bevy::light::NotShadowCaster);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn clumps_share_gpu_assets_and_inherit_each_source_part_once() {
        let mut world = World::new();
        let roots: Vec<_> = (0..2)
            .map(|_| {
                world
                    .spawn((
                        BlockPart {
                            block: "test".into(),
                            part_index: 4,
                        },
                        Transform::default(),
                        Visibility::Hidden,
                    ))
                    .id()
            })
            .collect();
        let prototype = Prototype {
            gltf: Handle::default(),
            path: String::new(),
            roots: vec![],
            barrier: None,
            primitives: Some(vec![Primitive {
                mesh: Handle::default(),
                material: Handle::default(),
                transform: Transform::IDENTITY,
                extras: None,
                mesh_extras: None,
                material_extras: None,
                material_name: None,
            }]),
        };
        let manifest = json!({"bindings":[{"part_index":4,"surface":"ground"}],"surfaces":{"ground":{"model":"tuft","params":{"castShadow":0},"placements":[{"translation":[1.,2.,3.],"rotation":[0.,0.,0.,1.],"scale":[1.,1.,1.]}]}}});
        world.insert_resource(Grass {
            primary: String::new(),
            blocks: BTreeMap::from([(
                "test".into(),
                Block {
                    manifest,
                    prototypes: BTreeMap::from([("tuft".into(), prototype)]),
                    parents: HashMap::new(),
                },
            )]),
            attached: HashSet::new(),
            checked: HashMap::new(),
        });
        instantiate(&mut world);
        instantiate(&mut world);
        let children: Vec<_> = world
            .query::<(
                &ChildOf,
                &Mesh3d,
                &MeshMaterial3d<StandardMaterial>,
                &Visibility,
                &Transform,
            )>()
            .iter(&world)
            .collect();
        assert_eq!(children.len(), 2);
        assert_ne!(children[0].0.parent(), children[1].0.parent());
        assert!(children.iter().all(|c| roots.contains(&c.0.parent())
            && *c.3 == Visibility::Inherited
            && c.4.translation == Vec3::new(1., 2., 3.)));
        assert_eq!(children[0].1.0.id(), children[1].1.0.id());
        assert_eq!(children[0].2.0.id(), children[1].2.0.id());
        world.despawn(roots[0]);
        assert_eq!(world.query::<&Mesh3d>().iter(&world).count(), 1);
    }

    fn pose(x: f32, yaw: f32) -> Value {
        let q = Quat::from_rotation_y(yaw);
        json!({"translation":[x,0.,-x],"rotation":[q.x,q.y,q.z,q.w],"scale":[1.,1.,1.]})
    }

    #[test]
    fn parent_index_matches_the_linear_binding_scan() {
        let manifest = json!({"bindings":[
            {"part_index":1,"parent_model":"m000100","parent_transform":pose(1.,0.)},
            {"part_index":2,"parent_model":"m000100","parent_transform":pose(2.,0.5)},
            {"part_index":3,"parent_model":"m000100","parent_transform":pose(2.,0.5)},
            {"part_index":4,"parent_model":"m000200","parent_transform":pose(1.,0.)},
            {"part_index":5,"parent_model":"m000300","parent_transform":{"translation":[0.]}},
            {"part_index":6},
        ]});
        // The lookup this index replaced: a scan of every binding per root.
        let linear = |path: &str, pose: &Transform| {
            manifest["bindings"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|b| {
                    let Some(model) = b["parent_model"].as_str() else {
                        return false;
                    };
                    let Ok(expected) = transform(&b["parent_transform"]) else {
                        return false;
                    };
                    path.contains(&format!("models/{model}.glb#"))
                        && expected.to_matrix().abs_diff_eq(pose.to_matrix(), 1e-5)
                })
                .map(|b| b["part_index"].as_u64().unwrap() as usize)
        };
        let parents = index_parents(&manifest).unwrap();
        let paths = [
            "m11_00_00_00/models/m000100.glb#Scene0",
            "maps/m11_00_00_00/models/m000200.glb#Scene1",
            "models/m000300.glb#Scene0",
            "m11_00_00_00/othermodels/m000100.glb#Scene0",
            "m11_00_00_00/models/m000100.glb",
            "m11_00_00_00/models/xm000100.glb#Scene0",
            "m11_00_00_00/models/m000100.glbx#Scene0",
        ];
        let mut found = 0;
        for path in paths {
            for (x, yaw) in [(1., 0.), (2., 0.5), (2., 0.), (1. + 1e-7, 0.)] {
                let pose = transform(&pose(x, yaw)).unwrap();
                let expected = linear(path, &pose);
                assert_eq!(
                    bound_part(&parents, path, &pose),
                    expected,
                    "{path} {x} {yaw}"
                );
                found += usize::from(expected.is_some());
            }
        }
        assert_eq!(found, 8);
    }

    #[test]
    fn parentless_roots_are_matched_once_not_every_frame() {
        let manifest = json!({"bindings":[
            {"part_index":7,"parent_model":"m000100","parent_transform":pose(1.,0.)},
        ]});
        let parents = index_parents(&manifest).unwrap();
        let scene = |n: u128| {
            AssetId::<Mesh>::Uuid {
                uuid: bevy::asset::uuid::Uuid::from_u128(n),
            }
            .untyped()
        };
        let bound = Entity::from_raw_u32(1).unwrap();
        let unbound = Entity::from_raw_u32(2).unwrap();
        let late = Entity::from_raw_u32(3).unwrap();
        let at = transform(&pose(1., 0.)).unwrap();
        let mut checked = HashMap::new();
        let mut lookups = 0;
        let mut path_known = false;
        let frame = |checked: &mut HashMap<_, _>,
                     path_known: bool,
                     lookups: &mut usize,
                     unbound_at: Transform| {
            let roots = [
                (bound, scene(1), at),
                (unbound, scene(2), unbound_at),
                (late, scene(3), at),
            ];
            match_roots(&parents, checked, roots, |id| {
                *lookups += 1;
                (id != scene(3) || path_known).then(|| {
                    if id == scene(2) {
                        "a/models/m000200.glb#Scene0"
                    } else {
                        "a/models/m000100.glb#Scene0"
                    }
                    .to_owned()
                })
            })
        };
        // A bound root that is not attached yet stays matched on later frames without a rescan.
        assert_eq!(
            frame(&mut checked, path_known, &mut lookups, at),
            [(bound, 7)]
        );
        assert_eq!(lookups, 3);
        assert_eq!(
            frame(&mut checked, path_known, &mut lookups, at),
            [(bound, 7)]
        );
        assert_eq!(
            lookups, 4,
            "only the root without a known path is looked up again"
        );
        path_known = true;
        assert_eq!(
            frame(&mut checked, path_known, &mut lookups, at),
            [(bound, 7), (late, 7)]
        );
        assert_eq!(lookups, 5);
        for _ in 0..10 {
            frame(&mut checked, path_known, &mut lookups, at);
        }
        assert_eq!(
            lookups, 5,
            "unmatched roots must not be rescanned every frame"
        );
        // A root that moves is matched again.
        frame(
            &mut checked,
            path_known,
            &mut lookups,
            Transform::from_xyz(9., 0., 0.),
        );
        assert_eq!(lookups, 6);
    }
}

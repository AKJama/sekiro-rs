//! Map hit collision: Havok 2016 `hknpPhysicsSceneData` tagfiles whose bodies use FromSoftware's
//! `fsnpCustomParamCompressedMeshShape` (an `hknpCompressedMeshShape` plus per-triangle
//! material data), decoded to plain triangle lists.
//!
//! Compressed mesh layout, as documented by the public readers (DSMapStudio's
//! `HavokCollisionResource` / HKX2 `hknpCompressedMeshShapeData`, soulstruct-havok type dumps):
//!
//! - The mesh tree holds sections. Each section owns a run of primitives
//!   (`primitives.data`: start index `>> 8`, count `& 0xFF`) and a run of local "packed"
//!   vertices starting at `firstPackedVertex`; `sharedVertices.data` gives the number of packed
//!   vertices (`& 0xFF`) and the start of the section's entries in `sharedVerticesIndex`
//!   (`>> 8`).
//! - A primitive is four byte indices. An index below the packed count is a packed vertex
//!   (32 bits: x 11, y 11, z 10, dequantised with the section's `codecParms`: offset xyz then
//!   scale xyz). Otherwise it selects a shared vertex through `sharedVerticesIndex`
//!   (64 bits: x 21, y 21, z 22, dequantised over the mesh tree's domain AABB).
//! - Indices `(a, b, c, d)` with `c != d` are a quad, split as `(a, b, c)` and `(a, c, d)`.
//!   `DE AD DE AD` marks a convex-shape primitive, which map collision does not use.
//!
//! The FromSoftware wrapper adds `triangleIndexToShapeKey` (authored triangle index to shape
//! key, the key left-aligned in `numShapeKeyBits`) and `pParam`, whose per-triangle
//! `primitiveDataIndex` selects a `PrimitiveData` with the hit material id
//! (`materialNameData`). The shape key of a triangle is
//! `(section << 8) | (primitive_in_section << 1) | second_triangle_of_quad`, so sections hold
//! at most 128 primitives and `numShapeKeyBits` is the section index bits plus 8. On
//! m11_00_00_00 every decoded triangle finds its key this way.
//!
//! Object binders' physics files also use convex polytopes (boxes, capsules and hulls), which
//! [`decode_bodies`] can turn into triangles. Output is in the file's own (FromSoftware,
//! left-handed) space with the body transform applied.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::hkx::{Compendium, TagFile, Value};
use crate::reader::{Result, bail};

/// A triangle soup with one material id per triangle.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CollisionMesh {
    pub vertices: Vec<[f32; 3]>,
    /// Three per triangle.
    pub indices: Vec<u32>,
    /// One per triangle: the FromSoftware hit material id, or `u32::MAX` when unknown.
    pub materials: Vec<u32>,
}

impl CollisionMesh {
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Appends another mesh.
    pub fn extend(&mut self, other: &CollisionMesh) {
        let base = self.vertices.len() as u32;
        self.vertices.extend_from_slice(&other.vertices);
        self.indices.extend(other.indices.iter().map(|i| i + base));
        self.materials.extend_from_slice(&other.materials);
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        postcard::to_stdvec(self).map_err(|e| crate::Error::Format(e.to_string()))
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        postcard::from_bytes(data).map_err(|e| crate::Error::Format(e.to_string()))
    }
}

/// Counters from decoding, for logs and sanity checks.
#[derive(Debug, Clone, Default)]
pub struct DecodeStats {
    pub bodies: usize,
    pub sections: usize,
    pub primitives: usize,
    pub quads: usize,
    pub convex_skipped: usize,
    /// Triangles whose shape key had no material entry.
    pub unmatched_material: usize,
    /// Convex polytope bodies decoded ([`decode_bodies`] with `polytopes`).
    pub polytopes: usize,
    /// Bodies skipped for their shape type.
    pub other_shapes: usize,
}

/// The collision of a map's placed objects (`cache/maps/<id>/object_collision.bin`, postcard):
/// the static and keyframed bodies of each placed object's physics binder (`<model>.hkx` in its
/// objbnd), placed with the object's MSB transform, in Bevy space like `collision.bin`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ObjectCollision {
    pub parts: Vec<ObjectCollisionPart>,
    pub mesh: CollisionMesh,
}

/// One placed object's triangles in [`ObjectCollision::mesh`].
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ObjectCollisionPart {
    /// MSB part name, as in `layout.json`.
    pub name: String,
    pub model: String,
    pub entity_id: i32,
    /// `ObjectParam.collisionType` of the model's row (0 without a row, as the exe reads it),
    /// which selects the bodies' Havok collision group.
    pub collision_type: u8,
    pub first: u32,
    pub count: u32,
}

impl ObjectCollision {
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        postcard::to_stdvec(self).map_err(|e| crate::Error::Format(e.to_string()))
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        postcard::from_bytes(data).map_err(|e| crate::Error::Format(e.to_string()))
    }

    /// One part's triangles as a mesh of their own.
    pub fn part_mesh(&self, part: &ObjectCollisionPart) -> CollisionMesh {
        let mut out = CollisionMesh::default();
        let range = part.first as usize..(part.first + part.count) as usize;
        for t in range {
            let base = out.vertices.len() as u32;
            out.vertices.extend(
                self.mesh.indices[t * 3..t * 3 + 3]
                    .iter()
                    .map(|&i| self.mesh.vertices[i as usize]),
            );
            out.indices.extend([base, base + 1, base + 2]);
            out.materials
                .push(self.mesh.materials.get(t).copied().unwrap_or(u32::MAX));
        }
        out
    }
}

/// Decodes every compressed-mesh body in a collision tagfile.
pub fn decode(
    data: &[u8],
    compendium: Option<&Compendium>,
) -> Result<(CollisionMesh, DecodeStats)> {
    let (bodies, stats) = decode_bodies(data, compendium, false)?;
    let mut mesh = CollisionMesh::default();
    for body in &bodies {
        mesh.extend(&body.mesh);
    }
    Ok((mesh, stats))
}

/// How a body moves, the serialised `motionType` of its `bodyCinfo`. Object binders use all
/// three: static props (0), bone-driven doors and tree trunks with no mass (1), and the massive
/// fragments of a broken object's `_1.hkx` (2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MotionType {
    Static,
    Keyframed,
    Dynamic,
}

/// One rigid body of a physics system, in the file's own space with its body transform applied.
#[derive(Debug, Clone)]
pub struct Body {
    pub name: String,
    pub motion: MotionType,
    pub mesh: CollisionMesh,
}

/// Decodes the bodies of a physics tagfile in order. Compressed meshes always; with
/// `polytopes` also the convex polytopes objects use (`hknpConvexPolytopeShape`, and the box and
/// capsule shapes derived from it), as their hull pushed out by the convex radius. Bodies of
/// other shapes are skipped and counted.
pub fn decode_bodies(
    data: &[u8],
    compendium: Option<&Compendium>,
    polytopes: bool,
) -> Result<(Vec<Body>, DecodeStats)> {
    let file = TagFile::parse(data, compendium)?;
    let mut bodies = Vec::new();
    let mut stats = DecodeStats::default();
    for system in file.objects_of("hknpPhysicsSystemData") {
        for body in system.get("bodyCinfos")?.elements()? {
            let Some(shape) = body.get("shape")?.deref()? else {
                continue;
            };
            let part = if shape.is_a("hknpCompressedMeshShape") {
                decode_shape(shape, &mut stats)?
            } else if polytopes && shape.is_a("hknpConvexPolytopeShape") {
                stats.polytopes += 1;
                decode_polytope(shape)?
            } else {
                stats.other_shapes += 1;
                continue;
            };
            let position = body.get("position")?.floats()?;
            let orientation = body.get("orientation")?.floats()?;
            let transform = glam::Affine3A::from_rotation_translation(
                glam::Quat::from_xyzw(
                    orientation[0],
                    orientation[1],
                    orientation[2],
                    orientation[3],
                )
                .normalize(),
                glam::Vec3::new(position[0], position[1], position[2]),
            );
            let mut mesh = part;
            for v in &mut mesh.vertices {
                *v = transform.transform_point3((*v).into()).to_array();
            }
            let motion = match body.get("motionType")?.int()? {
                0 => MotionType::Static,
                1 => MotionType::Keyframed,
                2 => MotionType::Dynamic,
                other => return bail(format!("unknown body motion type {other}")),
            };
            bodies.push(Body {
                name: body.get("name")?.string().unwrap_or_default(),
                motion,
                mesh,
            });
            stats.bodies += 1;
        }
    }
    Ok((bodies, stats))
}

/// A convex polytope as triangles: each face fanned from its first vertex and moved out along
/// its plane by the convex radius, since the vertices are the hull shrunk by that radius (an
/// Outskirts chest, `o114740`, has its box top at 1.006 m, radius 0.05, and its FLVER top at
/// 1.056 m).
fn decode_polytope(shape: Value<'_>) -> Result<CollisionMesh> {
    let radius = shape.get("convexRadius")?.f32()?;
    let vertices: Vec<glam::Vec3> = shape
        .get("vertices")?
        .elements()?
        .iter()
        .map(|v| {
            let f = v.floats()?;
            Ok(glam::Vec3::new(f[0], f[1], f[2]))
        })
        .collect::<Result<_>>()?;
    let planes = shape.get("planes")?.elements()?;
    let indices = shape.get("indices")?.ints()?;
    let mut mesh = CollisionMesh::default();
    for (face, plane) in shape.get("faces")?.elements()?.iter().zip(&planes) {
        let first = face.get("firstIndex")?.int()? as usize;
        let count = face.get("numIndices")?.int()? as usize;
        let normal = plane.floats()?;
        let offset = glam::Vec3::new(normal[0], normal[1], normal[2]) * radius;
        let Some(corners) = indices.get(first..first + count) else {
            return bail("polytope face indices out of range");
        };
        let corners: Vec<glam::Vec3> = corners
            .iter()
            .map(|&i| {
                vertices
                    .get(i as usize)
                    .map(|v| *v + offset)
                    .ok_or_else(|| crate::Error::Format("polytope vertex out of range".into()))
            })
            .collect::<Result<_>>()?;
        let base = mesh.vertices.len() as u32;
        mesh.vertices.extend(corners.iter().map(|c| c.to_array()));
        for k in 1..corners.len().saturating_sub(1) as u32 {
            mesh.indices.extend([base, base + k, base + k + 1]);
            mesh.materials.push(u32::MAX);
        }
    }
    Ok(mesh)
}

fn u32s(bytes: &[u8]) -> Vec<u32> {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b))
        .collect()
}

/// Shape key -> hit material id, from the FromSoftware per-triangle parameters.
fn material_by_key(shape: Value<'_>) -> Result<HashMap<u32, u32>> {
    let mut out = HashMap::new();
    if !shape.has("triangleIndexToShapeKey") || !shape.has("pParam") {
        return Ok(out);
    }
    let bits = shape.get("numShapeKeyBits")?.int()? as u32;
    if bits == 0 || bits > 32 {
        return Ok(out);
    }
    let Some(param) = shape.get("pParam")?.deref()? else {
        return Ok(out);
    };
    let keys = u32s(shape.get("triangleIndexToShapeKey")?.array_bytes()?);
    let primitive_materials: Vec<u32> = param
        .get("primitiveDataArray")?
        .elements()?
        .iter()
        .map(|p| Ok(p.get("materialNameData")?.int()? as u32))
        .collect::<Result<_>>()?;
    let triangles = param.get("triangleDataArray")?.elements()?;
    for (tri, raw) in triangles.iter().zip(keys) {
        let key = raw >> (32 - bits);
        let pd = tri.get("primitiveDataIndex")?.int()? as usize;
        if let Some(&m) = primitive_materials.get(pd) {
            out.entry(key).or_insert(m);
        }
    }
    Ok(out)
}

fn decode_shape(shape: Value<'_>, stats: &mut DecodeStats) -> Result<CollisionMesh> {
    let Some(data) = shape.get("data")?.deref()? else {
        return Ok(CollisionMesh::default());
    };
    let materials = material_by_key(shape)?;
    let tree = data.get("meshTree")?;
    let domain = tree.get("domain")?;
    let dmin = domain.get("min")?.floats()?;
    let dmax = domain.get("max")?.floats()?;
    let shared_scale = [
        (dmax[0] - dmin[0]) / ((1u32 << 21) - 1) as f32,
        (dmax[1] - dmin[1]) / ((1u32 << 21) - 1) as f32,
        (dmax[2] - dmin[2]) / ((1u32 << 22) - 1) as f32,
    ];
    let primitives = tree.get("primitives")?.array_bytes()?;
    let packed = u32s(tree.get("packedVertices")?.array_bytes()?);
    let shared: Vec<u64> = tree
        .get("sharedVertices")?
        .array_bytes()?
        .as_chunks::<8>()
        .0
        .iter()
        .map(|b| u64::from_le_bytes(*b))
        .collect();
    let shared_index: Vec<u16> = tree
        .get("sharedVerticesIndex")?
        .array_bytes()?
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_le_bytes(*b))
        .collect();

    let mut mesh = CollisionMesh::default();
    for (si, section) in tree.get("sections")?.elements()?.iter().enumerate() {
        stats.sections += 1;
        let codec = section.get("codecParms")?.floats()?;
        if codec.len() < 6 {
            return bail("section codecParms has fewer than 6 floats");
        }
        let first_packed = section.get("firstPackedVertex")?.int()? as usize;
        let sv = section.get("sharedVertices")?.get("data")?.int()? as u32;
        let packed_count = (sv & 0xFF) as usize;
        let shared_start = (sv >> 8) as usize;
        let pr = section.get("primitives")?.get("data")?.int()? as u32;
        let prim_start = (pr >> 8) as usize;
        let prim_count = (pr & 0xFF) as usize;

        let vertex = |i: u8| -> Result<[f32; 3]> {
            let i = i as usize;
            if i < packed_count {
                let Some(&v) = packed.get(first_packed + i) else {
                    return bail("packed vertex index out of range");
                };
                Ok([
                    (v & 0x7FF) as f32 * codec[3] + codec[0],
                    ((v >> 11) & 0x7FF) as f32 * codec[4] + codec[1],
                    ((v >> 22) & 0x3FF) as f32 * codec[5] + codec[2],
                ])
            } else {
                let Some(&s) = shared_index.get(shared_start + i - packed_count) else {
                    return bail("shared vertex index out of range");
                };
                let Some(&v) = shared.get(s as usize) else {
                    return bail("shared vertex out of range");
                };
                Ok([
                    (v & 0x1F_FFFF) as f32 * shared_scale[0] + dmin[0],
                    ((v >> 21) & 0x1F_FFFF) as f32 * shared_scale[1] + dmin[1],
                    ((v >> 42) & 0x3F_FFFF) as f32 * shared_scale[2] + dmin[2],
                ])
            }
        };

        // Each section indexes at most 256 vertices; weld them once per section.
        let mut welded = [u32::MAX; 256];
        let mut index_of = |i: u8, mesh: &mut CollisionMesh| -> Result<u32> {
            if welded[i as usize] == u32::MAX {
                welded[i as usize] = mesh.vertices.len() as u32;
                mesh.vertices.push(vertex(i)?);
            }
            Ok(welded[i as usize])
        };
        for p in 0..prim_count {
            let o = (prim_start + p) * 4;
            let Some(ix) = primitives.get(o..o + 4) else {
                return bail("primitive index out of range");
            };
            if ix == [0xDE, 0xAD, 0xDE, 0xAD] {
                stats.convex_skipped += 1;
                continue;
            }
            stats.primitives += 1;
            let key = ((si as u32) << 8) | ((p as u32) << 1);
            let mut material_of = |key: u32| match materials.get(&key) {
                Some(&m) => m,
                None => {
                    if !materials.is_empty() {
                        stats.unmatched_material += 1;
                    }
                    u32::MAX
                }
            };
            let first_material = material_of(key);
            let [a, b, c, d] = [ix[0], ix[1], ix[2], ix[3]];
            let (a, b, c) = (
                index_of(a, &mut mesh)?,
                index_of(b, &mut mesh)?,
                index_of(c, &mut mesh)?,
            );
            mesh.indices.extend([a, b, c]);
            mesh.materials.push(first_material);
            if ix[2] != ix[3] {
                stats.quads += 1;
                let d = index_of(d, &mut mesh)?;
                mesh.indices.extend([a, c, d]);
                let second = material_of(key | 1);
                mesh.materials.push(second);
            }
        }
    }
    Ok(mesh)
}

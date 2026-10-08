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
//! `(section << 8) | (primitive_in_section << 1) | second_triangle_of_quad`: sections hold at
//! most 128 primitives, and `numShapeKeyBits` is the section bits plus 8 (checked on every
//! m11_00_00_00 collision file: the key count equals the triangle count).
//!
//! Output is in the file's own (FromSoftware, left-handed) space with the body transform
//! applied.

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
}

/// Decodes every compressed-mesh body in a collision tagfile.
pub fn decode(
    data: &[u8],
    compendium: Option<&Compendium>,
) -> Result<(CollisionMesh, DecodeStats)> {
    let file = TagFile::parse(data, compendium)?;
    let mut mesh = CollisionMesh::default();
    let mut stats = DecodeStats::default();
    for system in file.objects_of("hknpPhysicsSystemData") {
        for body in system.get("bodyCinfos")?.elements()? {
            let Some(shape) = body.get("shape")?.deref()? else {
                continue;
            };
            if !shape.is_a("hknpCompressedMeshShape") {
                continue;
            }
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
            let part = decode_shape(shape, &mut stats)?;
            let base = mesh.vertices.len() as u32;
            mesh.vertices.extend(
                part.vertices
                    .iter()
                    .map(|v| transform.transform_point3((*v).into()).to_array()),
            );
            mesh.indices.extend(part.indices.iter().map(|i| i + base));
            mesh.materials.extend_from_slice(&part.materials);
            stats.bodies += 1;
        }
    }
    Ok((mesh, stats))
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

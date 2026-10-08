//! FLVER2 model reader (Sekiro uses version 0x2001A).
//!
//! Layout knowledge comes from SoulsFormatsNEXT (`FLVER/FLVER2/*.cs`, `FLVER/*.cs`),
//! reimplemented here. Only little-endian PC files are supported, and EdgeGeom compressed
//! buffers (a PS3 feature) are rejected.
//!
//! Everything is kept in FromSoftware's own coordinate system (left-handed, +Y up, +Z forward).
//! Conversion to glTF/Bevy happens in the exporter; see `docs/FORMATS.md`.

use crate::reader::{Reader, Result, bail, sjisz, utf16z};

/// Face set flag bits.
pub mod face_set_flags {
    pub const LOD1: u32 = 0x0100_0000;
    pub const LOD2: u32 = 0x0200_0000;
    pub const LOD_EX: u32 = 0x0400_0000;
    pub const EDGE_COMPRESSED: u32 = 0x4000_0000;
    pub const MOTION_BLUR: u32 = 0x8000_0000;
}

#[derive(Debug, Clone)]
pub struct Flver {
    pub version: u32,
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
    pub dummies: Vec<Dummy>,
    pub materials: Vec<Material>,
    pub nodes: Vec<Node>,
    pub meshes: Vec<Mesh>,
    pub layouts: Vec<Vec<LayoutMember>>,
}

/// A dummy polygon: a named attachment point (hitbox ends, effect spawns, weapon sockets).
#[derive(Debug, Clone)]
pub struct Dummy {
    pub position: [f32; 3],
    /// ARGB as stored.
    pub color: [u8; 4],
    pub forward: [f32; 3],
    pub reference_id: i16,
    /// Node the position is expressed relative to (usually a root-level node).
    pub parent_bone: i16,
    pub upward: [f32; 3],
    /// Node the dummy follows when animated.
    pub attach_bone: i16,
    pub flag1: bool,
    pub use_upward: bool,
}

#[derive(Debug, Clone)]
pub struct Material {
    pub name: String,
    pub mtd: String,
    pub textures: Vec<Texture>,
    pub index: i32,
}

#[derive(Debug, Clone)]
pub struct Texture {
    /// Shader parameter name, e.g. `g_DiffuseTexture`.
    pub param: String,
    /// Texture path; often empty in Sekiro (resolved via the MTD instead).
    pub path: String,
    pub scale: [f32; 2],
}

/// A transform node: bones, dummy owners and mesh owners share this table.
#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub translation: [f32; 3],
    /// Euler angles in radians, applied X then Z then Y (see [`Node::local_matrix`]).
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    pub parent: i16,
    pub first_child: i16,
    pub next_sibling: i16,
    pub previous_sibling: i16,
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
    pub flags: i32,
}

impl Node {
    /// Local transform as a column-vector matrix (`world = parent * local`), in the file's own
    /// coordinate system. SoulsFormats composes S * Rx * Rz * Ry * T in row-vector convention,
    /// which in column-vector form is T * Ry * Rz * Rx * S.
    pub fn local_matrix(&self) -> glam::Mat4 {
        let [rx, ry, rz] = self.rotation;
        let rotation = glam::Quat::from_rotation_y(ry)
            * glam::Quat::from_rotation_z(rz)
            * glam::Quat::from_rotation_x(rx);
        glam::Mat4::from_scale_rotation_translation(
            self.scale.into(),
            rotation,
            self.translation.into(),
        )
    }

    pub fn local_rotation(&self) -> glam::Quat {
        let [rx, ry, rz] = self.rotation;
        glam::Quat::from_rotation_y(ry)
            * glam::Quat::from_rotation_z(rz)
            * glam::Quat::from_rotation_x(rx)
    }
}

#[derive(Debug, Clone)]
pub struct Mesh {
    /// True when vertices carry bone weights.
    pub dynamic: bool,
    pub material: usize,
    /// Node that owns the mesh (its default bone).
    pub node: i32,
    /// Mesh-local bone palette. Empty in Sekiro: vertex bone indices then index the node table.
    pub bone_indices: Vec<i32>,
    pub face_sets: Vec<FaceSet>,
    pub buffers: Vec<VertexBufferInfo>,
    pub vertices: Vertices,
}

impl Mesh {
    /// The LOD0, non-motion-blur face set, if any.
    pub fn lod0(&self) -> Option<&FaceSet> {
        self.face_sets.iter().find(|f| f.flags == 0)
    }
}

#[derive(Debug, Clone)]
pub struct FaceSet {
    pub flags: u32,
    pub triangle_strip: bool,
    pub cull_backfaces: bool,
    pub indices: Vec<u32>,
}

impl FaceSet {
    /// Triangle list indices. Strips are unrolled with alternating winding, dropping
    /// degenerate triangles and honouring 0xFFFF primitive restarts.
    pub fn triangles(&self, vertex_count: usize) -> Vec<u32> {
        if !self.triangle_strip {
            return self.indices.clone();
        }
        let restarts = vertex_count < 0xFFFF;
        let mut out = Vec::with_capacity(self.indices.len() * 3);
        let mut flip = false;
        for w in self.indices.windows(3) {
            let (a, b, c) = (w[0], w[1], w[2]);
            if restarts && (a == 0xFFFF || b == 0xFFFF || c == 0xFFFF) {
                flip = false;
                continue;
            }
            if a != b && b != c && a != c {
                if flip {
                    out.extend([c, b, a]);
                } else {
                    out.extend([a, b, c]);
                }
            }
            flip = !flip;
        }
        out
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VertexBufferInfo {
    pub layout: usize,
    pub vertex_size: usize,
    pub vertex_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutMember {
    pub kind: LayoutType,
    pub semantic: LayoutSemantic,
    pub index: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutType {
    Float1,
    Float2,
    Float3,
    Float4,
    Color,
    UByte4,
    Byte4,
    UByte4Norm,
    Byte4Norm,
    Short2,
    Short4,
    UShort2,
    UShort4,
    Short4Norm,
    Half2,
    Half4,
    Byte4E,
}

impl LayoutType {
    fn from_u32(v: u32) -> Result<Self> {
        use LayoutType::*;
        Ok(match v {
            0 => Float1,
            1 => Float2,
            2 => Float3,
            3 => Float4,
            16 => Color,
            17 => UByte4,
            18 => Byte4,
            19 => UByte4Norm,
            20 => Byte4Norm,
            21 => Short2,
            22 => Short4,
            23 => UShort2,
            24 => UShort4,
            26 => Short4Norm,
            45 => Half2,
            46 => Half4,
            47 => Byte4E,
            240 => return bail("EdgeGeom compressed vertex layouts are not supported"),
            _ => return bail(format!("unknown FLVER layout type {v}")),
        })
    }

    pub fn size(self) -> usize {
        use LayoutType::*;
        match self {
            Float1 | Color | UByte4 | Byte4 | UByte4Norm | Byte4Norm | Short2 | UShort2
            | Byte4E | Half2 => 4,
            Float2 | Short4 | UShort4 | Short4Norm | Half4 => 8,
            Float3 => 12,
            Float4 => 16,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutSemantic {
    Position,
    BoneWeights,
    BoneIndices,
    Normal,
    Uv,
    Tangent,
    Bitangent,
    VertexColor,
}

impl LayoutSemantic {
    fn from_u32(v: u32) -> Result<Self> {
        use LayoutSemantic::*;
        Ok(match v {
            0 => Position,
            1 => BoneWeights,
            2 => BoneIndices,
            3 => Normal,
            5 => Uv,
            6 => Tangent,
            7 => Bitangent,
            10 => VertexColor,
            _ => return bail(format!("unknown FLVER layout semantic {v}")),
        })
    }
}

/// Decoded vertex attributes, one entry per vertex in each populated array.
#[derive(Debug, Clone, Default)]
pub struct Vertices {
    pub positions: Vec<[f32; 3]>,
    /// Further position streams (some meshes carry a second one, e.g. for cloth).
    pub extra_positions: Vec<Vec<[f32; 3]>>,
    pub normals: Vec<[f32; 3]>,
    pub extra_normals: Vec<Vec<[f32; 3]>>,
    /// The normal's W component; for unweighted meshes this can carry a bone index.
    pub normal_w: Vec<i32>,
    /// One array per tangent channel; W is the bitangent sign.
    pub tangents: Vec<Vec<[f32; 4]>>,
    pub bitangents: Vec<[f32; 4]>,
    /// One array per UV channel.
    pub uvs: Vec<Vec<[f32; 2]>>,
    /// One array per colour channel, RGBA 0..1.
    pub colors: Vec<Vec<[f32; 4]>>,
    pub bone_weights: Vec<[f32; 4]>,
    pub bone_indices: Vec<[u16; 4]>,
}

impl Vertices {
    pub fn len(&self) -> usize {
        self.positions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }
}

pub fn is_flver(data: &[u8]) -> bool {
    data.starts_with(b"FLVER\0")
}

const HEADER_SIZE: usize = 0x80;
const DUMMY_SIZE: usize = 0x40;
const MATERIAL_SIZE: usize = 0x20;
const NODE_SIZE: usize = 0x80;
const MESH_SIZE: usize = 0x30;
const FACE_SET_SIZE: usize = 0x20;
const VERTEX_BUFFER_SIZE: usize = 0x20;
const LAYOUT_SIZE: usize = 0x10;
const TEXTURE_SIZE: usize = 0x20;

pub fn parse(data: &[u8]) -> Result<Flver> {
    let mut r = Reader::new(data);
    r.magic(b"FLVER\0")?;
    if r.bytes(2)? != b"L\0" {
        return bail("big-endian FLVER is not supported");
    }
    let version = r.u32()?;
    if version < 0x20010 {
        return bail(format!(
            "FLVER version {version:#x} is older than supported"
        ));
    }
    let data_offset = r.u32()? as usize;
    let _data_length = r.u32()?;
    let dummy_count = r.u32()? as usize;
    let material_count = r.u32()? as usize;
    let node_count = r.u32()? as usize;
    let mesh_count = r.u32()? as usize;
    let vertex_buffer_count = r.u32()? as usize;
    let bbox_min = r.vec3()?;
    let bbox_max = r.vec3()?;
    r.seek(0x48);
    let header_index_size = r.u8()? as u32;
    let unicode = r.u8()? != 0;
    r.seek(0x50);
    let face_set_count = r.u32()? as usize;
    let layout_count = r.u32()? as usize;
    let texture_count = r.u32()? as usize;

    let string = |offset: usize| -> Result<String> {
        if unicode {
            utf16z(data, offset)
        } else {
            sjisz(data, offset)
        }
    };

    let mut pos = HEADER_SIZE;
    let mut table = |count: usize, size: usize| {
        let start = pos;
        pos += count * size;
        start
    };
    let dummies_at = table(dummy_count, DUMMY_SIZE);
    let materials_at = table(material_count, MATERIAL_SIZE);
    let nodes_at = table(node_count, NODE_SIZE);
    let meshes_at = table(mesh_count, MESH_SIZE);
    let face_sets_at = table(face_set_count, FACE_SET_SIZE);
    let buffers_at = table(vertex_buffer_count, VERTEX_BUFFER_SIZE);
    let layouts_at = table(layout_count, LAYOUT_SIZE);
    let textures_at = table(texture_count, TEXTURE_SIZE);
    crate::reader::slice(data, 0, pos)?;

    let mut dummies = Vec::with_capacity(dummy_count);
    for i in 0..dummy_count {
        let mut r = Reader::at(data, dummies_at + i * DUMMY_SIZE);
        let position = r.vec3()?;
        let color: [u8; 4] = r.bytes(4)?.try_into().unwrap();
        let forward = r.vec3()?;
        let reference_id = r.i16()?;
        let parent_bone = r.i16()?;
        let upward = r.vec3()?;
        let attach_bone = r.i16()?;
        let flag1 = r.u8()? != 0;
        let use_upward = r.u8()? != 0;
        dummies.push(Dummy {
            position,
            color,
            forward,
            reference_id,
            parent_bone,
            upward,
            attach_bone,
            flag1,
            use_upward,
        });
    }

    let mut textures = Vec::with_capacity(texture_count);
    for i in 0..texture_count {
        let mut r = Reader::at(data, textures_at + i * TEXTURE_SIZE);
        let path_offset = r.u32()? as usize;
        let param_offset = r.u32()? as usize;
        let scale = [r.f32()?, r.f32()?];
        textures.push(Texture {
            param: string(param_offset)?,
            path: string(path_offset)?,
            scale,
        });
    }

    let mut materials = Vec::with_capacity(material_count);
    for i in 0..material_count {
        let mut r = Reader::at(data, materials_at + i * MATERIAL_SIZE);
        let name_offset = r.u32()? as usize;
        let mtd_offset = r.u32()? as usize;
        let tex_count = r.u32()? as usize;
        let tex_index = r.u32()? as usize;
        let _string_bytes = r.u32()?;
        let _gx_offset = r.u32()?;
        let index = r.i32()?;
        let Some(tex) = textures.get(tex_index..tex_index + tex_count) else {
            return bail(format!("material {i} texture range out of bounds"));
        };
        materials.push(Material {
            name: string(name_offset)?,
            mtd: string(mtd_offset)?,
            textures: tex.to_vec(),
            index,
        });
    }

    let mut nodes = Vec::with_capacity(node_count);
    for i in 0..node_count {
        let mut r = Reader::at(data, nodes_at + i * NODE_SIZE);
        let translation = r.vec3()?;
        let name_offset = r.u32()? as usize;
        let rotation = r.vec3()?;
        let parent = r.i16()?;
        let first_child = r.i16()?;
        let scale = r.vec3()?;
        let next_sibling = r.i16()?;
        let previous_sibling = r.i16()?;
        let bbox_min = r.vec3()?;
        let flags = r.i32()?;
        let bbox_max = r.vec3()?;
        if parent >= node_count as i16 {
            return bail(format!("node {i} parent {parent} out of range"));
        }
        nodes.push(Node {
            name: string(name_offset)?,
            translation,
            rotation,
            scale,
            parent,
            first_child,
            next_sibling,
            previous_sibling,
            bbox_min,
            flags,
            bbox_max,
        });
    }

    let mut layouts = Vec::with_capacity(layout_count);
    for i in 0..layout_count {
        let mut r = Reader::at(data, layouts_at + i * LAYOUT_SIZE);
        let member_count = r.u32()? as usize;
        r.skip(8);
        let member_offset = r.u32()? as usize;
        let mut members = Vec::with_capacity(member_count);
        let mut m = Reader::at(data, member_offset);
        let mut struct_offset = 0;
        for _ in 0..member_count {
            let _stream = m.i32()?;
            let offset = m.u32()? as usize;
            if offset != struct_offset {
                return bail(format!(
                    "layout {i} member offset {offset} != {struct_offset}"
                ));
            }
            let kind = LayoutType::from_u32(m.u32()?)?;
            let semantic = LayoutSemantic::from_u32(m.u32()?)?;
            let index = m.i32()?;
            struct_offset += kind.size();
            members.push(LayoutMember {
                kind,
                semantic,
                index,
            });
        }
        layouts.push(members);
    }

    let read_face_set = |index: usize| -> Result<FaceSet> {
        let mut r = Reader::at(data, face_sets_at + index * FACE_SET_SIZE);
        let flags = r.u32()?;
        let triangle_strip = r.u8()? != 0;
        let cull_backfaces = r.u8()? != 0;
        let _unk06 = r.i16()?;
        let index_count = r.u32()? as usize;
        let indices_offset = r.u32()? as usize;
        let _length = r.u32()?;
        r.skip(4);
        let mut index_size = r.u32()?;
        if index_size == 0 {
            index_size = header_index_size;
        }
        if flags & face_set_flags::EDGE_COMPRESSED != 0 {
            return bail("EdgeGeom compressed face sets are not supported");
        }
        let start = data_offset + indices_offset;
        let indices = match index_size {
            16 => crate::reader::slice(data, start, index_count * 2)?
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes(*c) as u32)
                .collect(),
            32 => crate::reader::slice(data, start, index_count * 4)?
                .as_chunks::<4>()
                .0
                .iter()
                .map(|c| u32::from_le_bytes(*c))
                .collect(),
            other => return bail(format!("unsupported index size {other}")),
        };
        Ok(FaceSet {
            flags,
            triangle_strip,
            cull_backfaces,
            indices,
        })
    };

    let uv_factor = if version >= 0x2000E { 2048.0 } else { 1024.0 };

    let mut meshes = Vec::with_capacity(mesh_count);
    for i in 0..mesh_count {
        let mut r = Reader::at(data, meshes_at + i * MESH_SIZE);
        let dynamic = r.u8()? != 0;
        r.skip(3);
        let material = r.u32()? as usize;
        r.skip(8);
        let node = r.i32()?;
        let bone_count = r.u32()? as usize;
        let _bbox_offset = r.u32()?;
        let bone_offset = r.u32()? as usize;
        let mesh_face_sets = r.u32()? as usize;
        let face_set_offset = r.u32()? as usize;
        let buffer_count = r.u32()? as usize;
        let buffer_offset = r.u32()? as usize;
        if material >= materials.len() {
            return bail(format!("mesh {i} material {material} out of range"));
        }
        let ints = |offset: usize, count: usize| -> Result<Vec<i32>> {
            Ok(crate::reader::slice(data, offset, count * 4)?
                .as_chunks::<4>()
                .0
                .iter()
                .map(|c| i32::from_le_bytes(*c))
                .collect())
        };
        let bone_indices = ints(bone_offset, bone_count)?;
        let mut face_sets = Vec::with_capacity(mesh_face_sets);
        for fs in ints(face_set_offset, mesh_face_sets)? {
            if fs < 0 || fs as usize >= face_set_count {
                return bail(format!("mesh {i} face set {fs} out of range"));
            }
            face_sets.push(read_face_set(fs as usize)?);
        }
        let mut buffers = Vec::with_capacity(buffer_count);
        let mut vertices = Vertices::default();
        let buffer_indices = ints(buffer_offset, buffer_count)?;
        let vertex_count = match buffer_indices.first() {
            Some(&b) => {
                Reader::at(data, buffers_at + b as usize * VERTEX_BUFFER_SIZE + 12).u32()? as usize
            }
            None => 0,
        };
        for b in buffer_indices {
            if b < 0 || b as usize >= vertex_buffer_count {
                return bail(format!("mesh {i} vertex buffer {b} out of range"));
            }
            let mut r = Reader::at(data, buffers_at + b as usize * VERTEX_BUFFER_SIZE);
            let buffer_index = r.u32()?;
            if buffer_index & 0x6000_0000 != 0 {
                return bail("EdgeGeom compressed vertex buffers are not supported");
            }
            let layout = r.u32()? as usize;
            let vertex_size = r.u32()? as usize;
            let count = r.u32()? as usize;
            r.skip(8);
            let _length = r.u32()?;
            let offset = r.u32()? as usize;
            let Some(members) = layouts.get(layout) else {
                return bail(format!("vertex buffer {b} layout {layout} out of range"));
            };
            let layout_size: usize = members.iter().map(|m| m.kind.size()).sum();
            if layout_size != vertex_size {
                return bail(format!(
                    "vertex buffer {b}: stride {vertex_size} != layout size {layout_size}"
                ));
            }
            if count != vertex_count {
                return bail(format!(
                    "vertex buffer {b}: {count} vertices, mesh expects {vertex_count}"
                ));
            }
            let bytes =
                crate::reader::slice(data, data_offset + offset, vertex_size * vertex_count)?;
            read_buffer(bytes, members, vertex_size, uv_factor, &mut vertices)?;
            buffers.push(VertexBufferInfo {
                layout,
                vertex_size,
                vertex_count,
            });
        }
        if vertices.positions.len() != vertex_count {
            return bail(format!(
                "mesh {i}: {} positions for {vertex_count} vertices (layouts {:?})",
                vertices.positions.len(),
                buffers.iter().map(|b| b.layout).collect::<Vec<_>>()
            ));
        }
        meshes.push(Mesh {
            dynamic,
            material,
            node,
            bone_indices,
            face_sets,
            buffers,
            vertices,
        });
    }

    Ok(Flver {
        version,
        bbox_min,
        bbox_max,
        dummies,
        materials,
        nodes,
        meshes,
        layouts,
    })
}

/// Decodes one vertex buffer, appending its streams to `out`.
fn read_buffer(
    bytes: &[u8],
    members: &[LayoutMember],
    stride: usize,
    uv_factor: f32,
    out: &mut Vertices,
) -> Result<()> {
    use LayoutSemantic as S;
    use LayoutType as T;

    // Assign channel slots for repeatable semantics before decoding.
    let mut slots = Vec::with_capacity(members.len());
    let mut position_claimed = !out.positions.is_empty();
    let mut normal_claimed = !out.normals.is_empty();
    for m in members {
        let slot = match m.semantic {
            S::Position if position_claimed => {
                out.extra_positions
                    .push(Vec::with_capacity(bytes.len() / stride));
                out.extra_positions.len()
            }
            S::Position => {
                position_claimed = true;
                0
            }
            S::Normal if normal_claimed => {
                out.extra_normals
                    .push(Vec::with_capacity(bytes.len() / stride));
                out.extra_normals.len()
            }
            S::Normal => {
                normal_claimed = true;
                0
            }
            S::Uv => {
                let first = out.uvs.len();
                let n = match m.kind {
                    T::Float4 | T::UByte4Norm | T::Short4 | T::Half4 => 2,
                    _ => 1,
                };
                for _ in 0..n {
                    out.uvs.push(Vec::with_capacity(bytes.len() / stride));
                }
                first
            }
            S::Tangent => {
                out.tangents.push(Vec::with_capacity(bytes.len() / stride));
                out.tangents.len() - 1
            }
            S::VertexColor => {
                out.colors.push(Vec::with_capacity(bytes.len() / stride));
                out.colors.len() - 1
            }
            _ => 0,
        };
        slots.push(slot);
    }

    let unsupported = |m: &LayoutMember| {
        bail(format!(
            "unsupported vertex member {:?} {:?}",
            m.kind, m.semantic
        ))
    };
    let ubn = |b: u8| (b as f32 - 127.0) / 127.0;
    let sbn = |b: u8| (b as i8) as f32 / 127.0;
    let shn = |c: &[u8], i: usize| i16::from_le_bytes([c[i * 2], c[i * 2 + 1]]) as f32 / 32767.0;
    let i16_at = |c: &[u8], i: usize| i16::from_le_bytes([c[i * 2], c[i * 2 + 1]]);
    let u16_at = |c: &[u8], i: usize| u16::from_le_bytes([c[i * 2], c[i * 2 + 1]]);
    let f32_at = |c: &[u8], i: usize| f32::from_le_bytes(c[i * 4..i * 4 + 4].try_into().unwrap());

    for vertex in bytes.chunks_exact(stride) {
        let mut at = 0;
        for (m, &slot) in members.iter().zip(&slots) {
            let c = &vertex[at..at + m.kind.size()];
            at += m.kind.size();
            match m.semantic {
                S::Position => match m.kind {
                    T::Float3 | T::Float4 => {
                        let p = [f32_at(c, 0), f32_at(c, 1), f32_at(c, 2)];
                        match slot {
                            0 => out.positions.push(p),
                            k => out.extra_positions[k - 1].push(p),
                        }
                    }
                    _ => return unsupported(m),
                },
                S::BoneWeights => {
                    let w = match m.kind {
                        T::Color => c.map_n(|b| (b as i8) as f32 / 127.0),
                        T::UByte4Norm => c.map_n(|b| b as f32 / 255.0),
                        T::Short4 => {
                            let mut w = [0.0; 4];
                            for (k, v) in w.iter_mut().enumerate() {
                                let raw = u16_at(c, k) as i32;
                                let raw = if raw >= 0x8000 {
                                    raw - 0x8000
                                } else {
                                    raw + 0x8000
                                };
                                *v = raw as f32 / 65535.0;
                            }
                            w
                        }
                        T::Short4Norm => [shn(c, 0), shn(c, 1), shn(c, 2), shn(c, 3)],
                        _ => return unsupported(m),
                    };
                    out.bone_weights.push(w);
                }
                S::BoneIndices => {
                    let idx = match m.kind {
                        T::UByte4 | T::Byte4 | T::Byte4E => {
                            [c[0] as u16, c[1] as u16, c[2] as u16, c[3] as u16]
                        }
                        T::UShort2 => [u16_at(c, 0), u16_at(c, 1), 0, 0],
                        T::UShort4 => [u16_at(c, 0), u16_at(c, 1), u16_at(c, 2), u16_at(c, 3)],
                        _ => return unsupported(m),
                    };
                    out.bone_indices.push(idx);
                }
                S::Normal => {
                    let (n, w) = match m.kind {
                        T::Float3 => ([f32_at(c, 0), f32_at(c, 1), f32_at(c, 2)], 0),
                        T::Float4 => (
                            [f32_at(c, 0), f32_at(c, 1), f32_at(c, 2)],
                            f32_at(c, 3) as i32,
                        ),
                        T::Color | T::UByte4 | T::UByte4Norm | T::Byte4E => {
                            ([ubn(c[0]), ubn(c[1]), ubn(c[2])], c[3] as i32)
                        }
                        T::Byte4 => ([sbn(c[3]), sbn(c[2]), sbn(c[1])], c[0] as i32),
                        T::Short4Norm => ([shn(c, 0), shn(c, 1), shn(c, 2)], i16_at(c, 3) as i32),
                        T::Half4 => {
                            let u = |i| (u16_at(c, i) as f32 - 32767.0) / 32767.0;
                            ([u(0), u(1), u(2)], i16_at(c, 3) as i32)
                        }
                        _ => return unsupported(m),
                    };
                    if slot == 0 {
                        out.normals.push(n);
                        out.normal_w.push(w);
                    } else {
                        out.extra_normals[slot - 1].push(n);
                    }
                }
                S::Uv => {
                    let s = |i| i16_at(c, i) as f32 / uv_factor;
                    match m.kind {
                        T::Float2 => out.uvs[slot].push([f32_at(c, 0), f32_at(c, 1)]),
                        T::Float3 => out.uvs[slot].push([f32_at(c, 0), f32_at(c, 1)]),
                        T::Float4 => {
                            out.uvs[slot].push([f32_at(c, 0), f32_at(c, 1)]);
                            out.uvs[slot + 1].push([f32_at(c, 2), f32_at(c, 3)]);
                        }
                        T::Color | T::UByte4 | T::Byte4 | T::Short2 | T::Half2 => {
                            out.uvs[slot].push([s(0), s(1)])
                        }
                        T::UByte4Norm => {
                            out.uvs[slot].push([c[0] as f32 / 255.0, c[1] as f32 / 255.0]);
                            out.uvs[slot + 1].push([c[2] as f32 / 255.0, c[3] as f32 / 255.0]);
                        }
                        T::Short4 | T::Half4 => {
                            out.uvs[slot].push([s(0), s(1)]);
                            out.uvs[slot + 1].push([s(2), s(3)]);
                        }
                        _ => return unsupported(m),
                    }
                }
                S::Tangent => {
                    let t = match m.kind {
                        T::Float4 => [f32_at(c, 0), f32_at(c, 1), f32_at(c, 2), f32_at(c, 3)],
                        T::Color | T::UByte4 | T::UByte4Norm | T::Byte4E => c.map_n(ubn),
                        T::Byte4Norm => [sbn(c[3]), sbn(c[2]), sbn(c[1]), sbn(c[0])],
                        T::Short4Norm => [shn(c, 0), shn(c, 1), shn(c, 2), shn(c, 3)],
                        _ => return unsupported(m),
                    };
                    out.tangents[slot].push(t);
                }
                S::Bitangent => match m.kind {
                    T::Color | T::UByte4 | T::UByte4Norm | T::Byte4E => {
                        out.bitangents.push(c.map_n(ubn))
                    }
                    _ => return unsupported(m),
                },
                S::VertexColor => {
                    let col = match m.kind {
                        T::Float4 => [f32_at(c, 0), f32_at(c, 1), f32_at(c, 2), f32_at(c, 3)],
                        T::Color | T::UByte4Norm => c.map_n(|b| b as f32 / 255.0),
                        _ => return unsupported(m),
                    };
                    out.colors[slot].push(col);
                }
            }
        }
    }
    Ok(())
}

trait MapN {
    fn map_n(&self, f: impl Fn(u8) -> f32) -> [f32; 4];
}

impl MapN for [u8] {
    fn map_n(&self, f: impl Fn(u8) -> f32) -> [f32; 4] {
        [f(self[0]), f(self[1]), f(self[2]), f(self[3])]
    }
}

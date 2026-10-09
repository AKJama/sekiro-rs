//! Sekiro map layouts (`MSB `, the "MSBS" variant): the models a map uses and the parts that
//! place them (map pieces, objects, enemies, player start points, collision).
//!
//! Layout knowledge from SoulsFormatsNEXT `MSBS` (ModelParam, PartsParam), reimplemented. Only
//! what the map loader needs is decoded; events, regions and routes are skipped over.
//!
//! Part transforms are in FromSoftware's left-handed space: position in metres, rotation in
//! degrees about X, Y and Z, composed like FLVER nodes (`T * Ry * Rz * Rx * S` in
//! column-vector form, see [`Part::matrix`]).

use crate::reader::{Reader, Result, bail, utf16z};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelKind {
    MapPiece,
    Object,
    Enemy,
    Player,
    Collision,
    Other(u32),
}

impl ModelKind {
    fn from_u32(v: u32) -> Self {
        match v {
            0 => Self::MapPiece,
            1 => Self::Object,
            2 => Self::Enemy,
            4 => Self::Player,
            5 => Self::Collision,
            other => Self::Other(other),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Model {
    pub name: String,
    pub kind: ModelKind,
    pub sib_path: String,
    pub instance_count: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartKind {
    MapPiece,
    Object,
    Enemy,
    Player,
    Collision,
    DummyObject,
    DummyEnemy,
    ConnectCollision,
    Other(u32),
}

impl PartKind {
    fn from_u32(v: u32) -> Self {
        match v {
            0 => Self::MapPiece,
            1 => Self::Object,
            2 => Self::Enemy,
            4 => Self::Player,
            5 => Self::Collision,
            9 => Self::DummyObject,
            10 => Self::DummyEnemy,
            11 => Self::ConnectCollision,
            other => Self::Other(other),
        }
    }
}

/// Enemy and dummy-enemy settings.
#[derive(Debug, Clone, Default)]
pub struct EnemyData {
    pub think_param_id: i32,
    pub npc_param_id: i32,
    pub talk_id: i32,
    pub platoon_id: i16,
    pub chara_init_id: i32,
    /// Index of the collision part the enemy stands on, or -1.
    pub collision_part: i32,
    pub backup_event_anim_id: i32,
    pub event_flag_id: i32,
}

/// Collision (hit) part settings.
#[derive(Debug, Clone, Default)]
pub struct CollisionData {
    pub hit_filter_id: u8,
    pub sound_space_type: u8,
    pub map_name_id: i16,
    pub disable_start: bool,
    pub play_region_id: i32,
    pub lock_cam_param_id: i16,
}

#[derive(Debug, Clone)]
pub struct Part {
    pub name: String,
    pub kind: PartKind,
    /// Index into [`Msb::models`], or -1.
    pub model_index: i32,
    pub sib_path: String,
    pub position: [f32; 3],
    /// Euler angles in degrees.
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    pub entity_id: i32,
    pub entity_group_ids: [i32; 8],
    /// The 48-word mask block most parts carry (SoulsFormats calls it `CollisionMask`; the first
    /// words are the draw groups). Empty when the part type has none.
    pub masks: Vec<u32>,
    /// Display groups of collision parts, empty otherwise.
    pub disp_groups: Vec<u32>,
    pub lod_param_id: u8,
    pub is_shadow_only: bool,
    pub enemy: Option<EnemyData>,
    pub collision: Option<CollisionData>,
    /// For connect collisions: (collision part index, target map id bytes).
    pub connect: Option<(i32, [u8; 4])>,
    /// Draw-param selection: light set, fog, light scattering and environment map ids.
    pub gparam: Option<[i32; 4]>,
}

impl Part {
    /// Local-to-map transform in FromSoftware space (column-vector convention).
    pub fn matrix(&self) -> glam::Mat4 {
        let [rx, ry, rz] = self.rotation.map(f32::to_radians);
        let rotation = glam::Quat::from_rotation_y(ry)
            * glam::Quat::from_rotation_z(rz)
            * glam::Quat::from_rotation_x(rx);
        glam::Mat4::from_scale_rotation_translation(
            self.scale.into(),
            rotation,
            self.position.into(),
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct Msb {
    pub models: Vec<Model>,
    pub parts: Vec<Part>,
}

impl Msb {
    pub fn model(&self, part: &Part) -> Option<&Model> {
        usize::try_from(part.model_index)
            .ok()
            .and_then(|i| self.models.get(i))
    }
}

pub fn is_msb(data: &[u8]) -> bool {
    data.starts_with(b"MSB ")
}

pub fn parse(data: &[u8]) -> Result<Msb> {
    let mut r = Reader::new(data);
    r.magic(b"MSB ")?;
    if r.i32()? != 1 || r.i32()? != 0x10 {
        return bail("unexpected MSB header");
    }
    let mut msb = Msb::default();
    let mut pos = 0x10;
    let mut guard = 0;
    while pos != 0 {
        guard += 1;
        if guard > 32 {
            return bail("too many MSB params");
        }
        let mut p = Reader::at(data, pos);
        let _version = p.i32()?;
        let offset_count = p.i32()? as usize;
        let name_offset = p.offset64()?;
        let mut entries = Vec::with_capacity(offset_count.saturating_sub(1));
        for _ in 1..offset_count {
            entries.push(p.offset64()?);
        }
        let next = p.offset64()?;
        let name = utf16z(data, name_offset)?;
        match name.as_str() {
            "MODEL_PARAM_ST" => {
                for &e in &entries {
                    msb.models.push(read_model(data, e)?);
                }
            }
            "PARTS_PARAM_ST" => {
                for &e in &entries {
                    msb.parts.push(read_part(data, e)?);
                }
            }
            _ => {}
        }
        pos = next;
    }
    Ok(msb)
}

fn read_model(data: &[u8], start: usize) -> Result<Model> {
    let mut r = Reader::at(data, start);
    let name_offset = r.offset64()?;
    let kind = ModelKind::from_u32(r.u32()?);
    let _id = r.i32()?;
    let sib_offset = r.offset64()?;
    let instance_count = r.i32()?;
    Ok(Model {
        name: utf16z(data, start + name_offset)?,
        kind,
        sib_path: utf16z(data, start + sib_offset)?,
        instance_count,
    })
}

fn read_part(data: &[u8], start: usize) -> Result<Part> {
    let mut r = Reader::at(data, start);
    let name_offset = r.offset64()?;
    let kind = PartKind::from_u32(r.u32()?);
    let _id = r.i32()?;
    let model_index = r.i32()?;
    r.skip(4);
    let sib_offset = r.offset64()?;
    let position = r.vec3()?;
    let rotation = r.vec3()?;
    let scale = r.vec3()?;
    r.skip(12);
    let unk1 = r.i64()?;
    let unk2 = r.i64()?;
    let entity = r.offset64()?;
    let type_data = r.offset64()?;
    let gparam = r.i64()?;

    let mut part = Part {
        name: utf16z(data, start + name_offset)?,
        kind,
        model_index,
        sib_path: utf16z(data, start + sib_offset)?,
        position,
        rotation,
        scale,
        entity_id: -1,
        entity_group_ids: [-1; 8],
        masks: Vec::new(),
        disp_groups: Vec::new(),
        lod_param_id: 0,
        is_shadow_only: false,
        enemy: None,
        collision: None,
        connect: None,
        gparam: None,
    };
    if gparam > 0 {
        let mut g = Reader::at(data, start + gparam as usize);
        part.gparam = Some([g.i32()?, g.i32()?, g.i32()?, g.i32()?]);
    }

    if unk1 > 0 {
        let mut u = Reader::at(data, start + unk1 as usize);
        part.masks = (0..48).map(|_| u.u32()).collect::<Result<_>>()?;
    }
    if unk2 > 0 {
        let mut u = Reader::at(data, start + unk2 as usize);
        let _condition = u.i32()?;
        part.disp_groups = (0..8).map(|_| u.u32()).collect::<Result<_>>()?;
    }

    let mut e = Reader::at(data, start + entity);
    part.entity_id = e.i32()?;
    e.skip(4); // E04..E06, lantern id
    part.lod_param_id = e.u8()?;
    e.skip(9); // E09..E11
    part.is_shadow_only = e.u8()? != 0; // E12
    e.skip(9); // E13..E17, E18 (i32)
    for g in &mut part.entity_group_ids {
        *g = e.i32()?;
    }

    let mut t = Reader::at(data, start + type_data);
    match kind {
        PartKind::Enemy | PartKind::DummyEnemy => {
            t.skip(8);
            let think_param_id = t.i32()?;
            let npc_param_id = t.i32()?;
            let talk_id = t.i32()?;
            t.skip(2);
            let platoon_id = t.i16()?;
            let chara_init_id = t.i32()?;
            let collision_part = t.i32()?;
            t.skip(8 + 16);
            let backup_event_anim_id = t.i32()?;
            t.skip(4);
            let event_flag_id = t.i32()?;
            part.enemy = Some(EnemyData {
                think_param_id,
                npc_param_id,
                talk_id,
                platoon_id,
                chara_init_id,
                collision_part,
                backup_event_anim_id,
                event_flag_id,
            });
        }
        PartKind::Collision => {
            let hit_filter_id = t.u8()?;
            let sound_space_type = t.u8()?;
            t.skip(2 + 4 + 12);
            let map_name_id = t.i16()?;
            let disable_start = t.u8()? != 0;
            t.skip(1 + 4 + 8 + 4);
            let play_region_id = t.i32()?;
            let lock_cam_param_id = t.i16()?;
            part.collision = Some(CollisionData {
                hit_filter_id,
                sound_space_type,
                map_name_id,
                disable_start,
                play_region_id,
                lock_cam_param_id,
            });
        }
        PartKind::ConnectCollision => {
            let index = t.i32()?;
            let map: [u8; 4] = t.bytes(4)?.try_into().unwrap();
            part.connect = Some((index, map));
        }
        _ => {}
    }
    Ok(part)
}

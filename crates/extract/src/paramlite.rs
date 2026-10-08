//! A tiny read-only PARAM accessor, just enough for the model exporter to look up
//! NpcParam display masks and the player's starting equipment.
//!
//! Field offsets are computed from Paramdex XML definitions (cache/refs/paramdex/Defs).
//! The full PARAM reader lives in `sekiro_formats` once it lands; this module only exists so the
//! model exporter does not depend on it.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};

/// Byte offset and optional bit position of each field in a row.
pub struct Def {
    fields: HashMap<String, Field>,
}

#[derive(Clone, Copy)]
struct Field {
    offset: usize,
    kind: Kind,
    bit: Option<(u32, u32)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    F32,
    Other(usize),
}

impl Kind {
    fn parse(t: &str) -> Kind {
        match t {
            "s8" => Kind::I8,
            "u8" | "dummy8" => Kind::U8,
            "s16" => Kind::I16,
            "u16" => Kind::U16,
            "s32" => Kind::I32,
            "u32" => Kind::U32,
            "f32" | "angle32" => Kind::F32,
            "fixstr" => Kind::Other(1),
            "fixstrW" => Kind::Other(2),
            "f64" => Kind::Other(8),
            _ => Kind::Other(4),
        }
    }

    fn size(self) -> usize {
        match self {
            Kind::I8 | Kind::U8 => 1,
            Kind::I16 | Kind::U16 => 2,
            Kind::I32 | Kind::U32 | Kind::F32 => 4,
            Kind::Other(n) => n,
        }
    }
}

impl Def {
    pub fn load(path: &Path) -> Result<Self> {
        let xml =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let mut fields = HashMap::new();
        let mut offset = 0usize;
        // Open bitfield unit: (kind, bits used).
        let mut unit: Option<(Kind, u32)> = None;
        for part in xml.split("<Field Def=\"").skip(1) {
            let def = &part[..part.find('"').context("unterminated Def")?];
            let def = def.split('=').next().unwrap().trim();
            let (ty, rest) = def.split_once(' ').context("bad field def")?;
            let kind = Kind::parse(ty);
            let rest = rest.trim();
            if let Some((name, bits)) = rest.split_once(':') {
                let bits: u32 = bits.trim().parse()?;
                let cap = kind.size() as u32 * 8;
                let start = match unit {
                    Some((k, used)) if k.size() == kind.size() && used + bits <= cap => {
                        offset -= kind.size();
                        used
                    }
                    _ => 0,
                };
                fields.insert(
                    name.trim().to_string(),
                    Field {
                        offset,
                        kind,
                        bit: Some((start, bits)),
                    },
                );
                unit = Some((kind, start + bits));
                offset += kind.size();
            } else {
                unit = None;
                let (name, count) = match rest.split_once('[') {
                    Some((n, c)) => (n, c.trim_end_matches(']').parse::<usize>()?),
                    None => (rest, 1),
                };
                fields.insert(
                    name.trim().to_string(),
                    Field {
                        offset,
                        kind,
                        bit: None,
                    },
                );
                offset += kind.size() * count;
            }
        }
        Ok(Self { fields })
    }
}

pub struct Param {
    data: Vec<u8>,
    rows: HashMap<i32, usize>,
}

impl Param {
    pub fn load(path: &Path) -> Result<Self> {
        let data = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        if data.len() < 0x40 {
            bail!("{} too short", path.display());
        }
        let count = u16::from_le_bytes([data[0xA], data[0xB]]) as usize;
        let format = data[0x2D];
        if format & 0x04 == 0 {
            bail!(
                "{}: only 64-bit offset PARAMs are supported",
                path.display()
            );
        }
        let mut rows = HashMap::with_capacity(count);
        for i in 0..count {
            let at = 0x40 + i * 24;
            let id = i32::from_le_bytes(data[at..at + 4].try_into()?);
            let offset = u64::from_le_bytes(data[at + 8..at + 16].try_into()?) as usize;
            rows.insert(id, offset);
        }
        Ok(Self { data, rows })
    }

    pub fn row<'a>(&'a self, def: &'a Def, id: i32) -> Option<Row<'a>> {
        let offset = *self.rows.get(&id)?;
        Some(Row {
            data: self.data.get(offset..)?,
            def,
        })
    }
}

pub struct Row<'a> {
    data: &'a [u8],
    def: &'a Def,
}

impl Row<'_> {
    /// Reads any integer field (bitfields included) as i64.
    pub fn int(&self, name: &str) -> Result<i64> {
        let f = self
            .def
            .fields
            .get(name)
            .with_context(|| format!("no field {name}"))?;
        let b = &self.data[f.offset..f.offset + f.kind.size()];
        let raw: i64 = match f.kind {
            Kind::I8 => b[0] as i8 as i64,
            Kind::U8 => b[0] as i64,
            Kind::I16 => i16::from_le_bytes([b[0], b[1]]) as i64,
            Kind::U16 => u16::from_le_bytes([b[0], b[1]]) as i64,
            Kind::I32 => i32::from_le_bytes(b.try_into()?) as i64,
            Kind::U32 => u32::from_le_bytes(b.try_into()?) as i64,
            Kind::F32 | Kind::Other(_) => bail!("field {name} is not an integer"),
        };
        Ok(match f.bit {
            Some((start, bits)) => (raw as u64 >> start & ((1u64 << bits) - 1)) as i64,
            None => raw,
        })
    }
}

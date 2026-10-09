//! Draw parameters (`param/drawparam/*.gparam`, "filt" files): light sets, fog, tone mapping and
//! other renderer settings, grouped by editor page.
//!
//! Layout from SoulsFormatsNEXT `GPARAM` (Sekiro variant), reimplemented. Each param holds one or
//! more values, each tagged with a value id (0 is the base; other ids are per-area variants
//! selected elsewhere) and, in Sekiro, a time of day in hours. [`Param::at`] interpolates the
//! values of one id over the time of day.

use crate::reader::{Reader, Result, bail, utf16z};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i32),
    Float(f32),
    Bool(bool),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Bytes([u8; 4]),
}

impl Value {
    /// The value as floats (booleans and integers as 0/1 and their value).
    pub fn floats(&self) -> Vec<f32> {
        match self {
            Value::Int(i) => vec![*i as f32],
            Value::Float(f) => vec![*f],
            Value::Bool(b) => vec![*b as i32 as f32],
            Value::Vec2(v) => v.to_vec(),
            Value::Vec3(v) => v.to_vec(),
            Value::Vec4(v) => v.to_vec(),
            Value::Bytes(b) => b.iter().map(|&x| x as f32).collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: i32,
    /// Hours, 0 to 24.
    pub time: f32,
    pub value: Value,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub display_name: String,
    pub entries: Vec<Entry>,
}

impl Param {
    /// The value for `id` at `hour`, interpolated linearly between the nearest keyed times
    /// (wrapping around midnight). Falls back to id 0 when `id` has no entries.
    pub fn at(&self, id: i32, hour: f32) -> Option<Vec<f32>> {
        let mut keys: Vec<&Entry> = self.entries.iter().filter(|e| e.id == id).collect();
        if keys.is_empty() && id != 0 {
            return self.at(0, hour);
        }
        if keys.is_empty() {
            return None;
        }
        keys.sort_by(|a, b| a.time.total_cmp(&b.time));
        let hour = hour.rem_euclid(24.0);
        let after = keys.iter().position(|e| e.time > hour);
        let (a, b) = match after {
            Some(0) => (keys[keys.len() - 1], keys[0]),
            Some(i) => (keys[i - 1], keys[i]),
            None => (keys[keys.len() - 1], keys[0]),
        };
        let span = (b.time - a.time).rem_euclid(24.0);
        let t = if span > 0.0 {
            (hour - a.time).rem_euclid(24.0) / span
        } else {
            0.0
        };
        let (va, vb) = (a.value.floats(), b.value.floats());
        Some(va.iter().zip(&vb).map(|(x, y)| x + (y - x) * t).collect())
    }
}

#[derive(Debug, Clone)]
pub struct Group {
    pub name: String,
    pub display_name: String,
    pub params: Vec<Param>,
}

#[derive(Debug, Clone, Default)]
pub struct Gparam {
    pub groups: Vec<Group>,
}

impl Gparam {
    /// The param whose name contains `name`, in the group whose name contains `group`.
    pub fn param(&self, group: &str, name: &str) -> Option<&Param> {
        self.groups
            .iter()
            .filter(|g| g.name.contains(group))
            .flat_map(|g| &g.params)
            .find(|p| p.name.contains(name))
    }
}

pub fn parse(data: &[u8]) -> Result<Gparam> {
    let mut r = Reader::new(data);
    if data.starts_with(b"f\0i\0l\0t\0") {
        r.seek(8);
    } else if data.starts_with(b"filt") {
        r.seek(4);
    } else {
        return bail("not a GPARAM file");
    }
    let game = r.u32()?;
    if game != 5 {
        return bail(format!("GPARAM for game {game}, expected Sekiro (5)"));
    }
    r.skip(4);
    let group_count = r.i32()? as usize;
    let _unk14 = r.i32()?;
    let header_size = r.i32()? as usize;
    let group_headers = r.i32()? as usize;
    let param_header_offsets = r.i32()? as usize;
    let param_headers = r.i32()? as usize;
    let values = r.i32()? as usize;
    let value_ids = r.i32()? as usize;

    let mut out = Gparam::default();
    for g in 0..group_count {
        let mut gr = Reader::at(data, header_size + g * 4);
        let go = group_headers + gr.i32()? as usize;
        let mut h = Reader::at(data, go);
        let param_count = h.i32()? as usize;
        let pho = param_header_offsets + h.i32()? as usize;
        let name = utf16z(data, go + 8)?;
        let display_name = utf16z(data, go + 8 + (name.encode_utf16().count() + 1) * 2)?;
        let mut params = Vec::with_capacity(param_count);
        for p in 0..param_count {
            let ph = param_headers + Reader::at(data, pho + p * 4).i32()? as usize;
            let mut x = Reader::at(data, ph);
            let vo = values + x.i32()? as usize;
            let io = value_ids + x.i32()? as usize;
            let ty = x.u8()?;
            let count = x.u8()? as usize;
            let name = utf16z(data, ph + 12)?;
            let display_name = utf16z(data, ph + 12 + (name.encode_utf16().count() + 1) * 2)?;
            let size = match ty {
                1 | 5 | 11 => 1,
                2 => 2,
                3 | 7 | 9 | 15 => 4,
                12..=14 => 16,
                other => return bail(format!("GPARAM param {name}: unknown type {other}")),
            };
            let mut entries = Vec::with_capacity(count);
            for i in 0..count {
                let mut v = Reader::at(data, vo + i * size);
                let value = match ty {
                    1 => Value::Int(v.u8()? as i32),
                    2 => Value::Int(v.i16()? as i32),
                    3 | 7 => Value::Int(v.i32()?),
                    5 | 11 => Value::Bool(v.u8()? != 0),
                    9 => Value::Float(v.f32()?),
                    12 => Value::Vec2([v.f32()?, v.f32()?]),
                    13 => Value::Vec3(v.vec3()?),
                    14 => Value::Vec4(v.vec4()?),
                    _ => Value::Bytes(v.bytes(4)?.try_into().unwrap()),
                };
                let mut id = Reader::at(data, io + i * 8);
                entries.push(Entry {
                    id: id.i32()?,
                    time: id.f32()?,
                    value,
                });
            }
            params.push(Param {
                name,
                display_name,
                entries,
            });
        }
        out.groups.push(Group {
            name,
            display_name,
            params,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn param(times: &[(f32, f32)]) -> Param {
        Param {
            name: "p".into(),
            display_name: "p".into(),
            entries: times
                .iter()
                .map(|&(time, v)| Entry {
                    id: 0,
                    time,
                    value: Value::Float(v),
                })
                .collect(),
        }
    }

    #[test]
    fn interpolates_over_the_day() {
        let p = param(&[(6.0, 0.0), (12.0, 6.0), (18.0, 0.0)]);
        assert_eq!(p.at(0, 9.0).unwrap(), vec![3.0]);
        assert_eq!(p.at(0, 12.0).unwrap(), vec![6.0]);
        // Wraps from 18:00 to 06:00 through midnight.
        assert_eq!(p.at(0, 0.0).unwrap(), vec![0.0]);
        // Unknown ids fall back to the base values.
        assert_eq!(p.at(100, 15.0).unwrap(), vec![3.0]);
    }
}

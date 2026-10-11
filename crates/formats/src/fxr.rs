//! Bounds-checked reader for the little-endian FXR version 5 tables used by Sekiro.
//!
//! The high bytes of record tags are retained: the low 16 bits identify a node, action or
//! property evaluator. Unknown semantics remain raw rather than being guessed as particles.

use crate::reader::{Reader, Result, bail, slice};
use std::ops::Range;

const STRIDES: [usize; 14] = [16, 16, 96, 48, 32, 64, 40, 32, 24, 16, 4, 4, 4, 4];

#[derive(Debug, Clone)]
pub struct Table {
    pub offset: usize,
    pub count: usize,
    pub stride: usize,
}

#[derive(Debug, Clone)]
pub struct State {
    pub tag: u32,
    pub transitions: Range<usize>,
}

#[derive(Debug, Clone)]
pub struct Transition {
    pub tag: u32,
    pub conditions: Range<usize>,
}

#[derive(Debug, Clone)]
pub struct Condition {
    pub words: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub tag: u32,
    pub configurations: Range<usize>,
    pub actions: Range<usize>,
    pub children: Range<usize>,
}

#[derive(Debug, Clone)]
pub struct Configuration {
    pub tag: u32,
    pub fields: Vec<u32>,
    pub actions: Range<usize>,
}

#[derive(Debug, Clone)]
pub struct Action {
    pub tag: u32,
    pub variant: u32,
    /// The two fixed-field groups, without the property payloads interleaved after them.
    pub fields: [Vec<u32>; 2],
    pub properties: [Range<usize>; 2],
    /// Sequence descriptors in table 10, distinct from property modifiers in table 8.
    pub sequences: Range<usize>,
}

impl Action {
    pub fn id(&self) -> u16 {
        self.tag as u16
    }
}

#[derive(Debug, Clone)]
pub struct Property {
    pub tag: u32,
    pub evaluator: u32,
    /// Raw little-endian words, preserving integers and floating-point bits alike.
    pub values: Vec<u32>,
    pub modifiers: Range<usize>,
}

/// Structure of a sequence payload, retaining cubic controls without guessing their evaluator.
#[derive(Debug, Clone, PartialEq)]
pub struct Curve {
    pub function: u16,
    pub looping: bool,
    pub argument: u32,
    pub bounds: [Vec<f32>; 2],
    pub times: Vec<f32>,
    pub values: Vec<Vec<f32>>,
    pub controls: Vec<f32>,
}

impl Property {
    /// Decodes the structure shared by stepped, linear and two cubic evaluator families.
    /// Bounds and cubic control words are data, not an assertion of native interpolation rules.
    pub fn curve(&self) -> Option<Curve> {
        let dimensions = match self.tag & 15 {
            0 => 1,
            1 => 2,
            2 => 3,
            3 => 4,
            _ => return None,
        };
        let function = ((self.tag >> 4) & 255) as u16;
        let controls_per_key = match function {
            3 | 4 => 0,
            5 | 6 => 2 * dimensions,
            _ => return None,
        };
        let header = 1 + 2 * dimensions;
        let payload = self.values.len().checked_sub(header)?;
        let stride = 1 + dimensions + controls_per_key;
        if payload == 0 || payload % stride != 0 {
            return None;
        }
        let count = payload / stride;
        let floats: Vec<_> = self.values.iter().map(|&v| f32::from_bits(v)).collect();
        let times = floats[header..header + count].to_vec();
        let values_end = header + count + count * dimensions;
        let values = floats[header + count..values_end]
            .chunks_exact(dimensions)
            .map(<[f32]>::to_vec)
            .collect();
        Some(Curve {
            function,
            looping: self.tag & 0x1000 != 0,
            argument: self.values[0],
            bounds: [
                floats[1..1 + dimensions].to_vec(),
                floats[1 + dimensions..header].to_vec(),
            ],
            times,
            values,
            controls: floats[values_end..].to_vec(),
        })
    }

    /// Constant evaluators observed in the version 5 files. Curves are deliberately distinct.
    pub fn constant(&self) -> Option<Vec<f32>> {
        let dimensions = match self.tag & 15 {
            0 => 1,
            1 => 2,
            2 => 3,
            3 => 4,
            _ => return None,
        };
        match self.tag & 0xfff0 {
            0 => Some(vec![0.0; dimensions]),
            0x10 => Some(vec![1.0; dimensions]),
            0x20 if self.values.len() == dimensions => {
                Some(self.values.iter().map(|&v| f32::from_bits(v)).collect())
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Modifier {
    pub tag: u32,
    pub evaluator: u32,
    pub values: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct Sequence {
    pub values: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct Fxr {
    pub id: u32,
    pub tables: Vec<Table>,
    pub states: Vec<State>,
    pub transitions: Vec<Transition>,
    pub conditions: Vec<Condition>,
    pub nodes: Vec<Node>,
    pub configurations: Vec<Configuration>,
    pub actions: Vec<Action>,
    pub properties: Vec<Property>,
    pub modifiers: Vec<Modifier>,
    pub sequences: Vec<Sequence>,
    /// Whole input retained for state/transition/condition records and unknown fields.
    pub bytes: Vec<u8>,
}

fn word(data: &[u8], at: usize) -> Result<u32> {
    Reader::at(data, at).u32()
}

fn offset(data: &[u8], at: usize) -> Result<usize> {
    Reader::at(data, at).offset64()
}

fn words(data: &[u8], offset: usize, count: usize) -> Result<Vec<u32>> {
    let length = count
        .checked_mul(4)
        .ok_or_else(|| crate::Error::Format("FXR word count overflow".into()))?;
    Ok(slice(data, offset, length)?
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&w| u32::from_le_bytes(w))
        .collect())
}

fn reference(table: &Table, offset: usize, count: usize) -> Result<Range<usize>> {
    if count == 0 {
        return Ok(0..0);
    }
    let Some(relative) = offset.checked_sub(table.offset) else {
        return bail("FXR reference precedes its table");
    };
    if relative % table.stride != 0 {
        return bail("unaligned FXR table reference");
    }
    let start = relative / table.stride;
    let Some(end) = start.checked_add(count).filter(|&end| end <= table.count) else {
        return bail("FXR reference exceeds its table");
    };
    Ok(start..end)
}

/// Reads the table graph without interpreting undocumented action field positions.
pub fn parse(data: &[u8]) -> Result<Fxr> {
    slice(data, 0, 144)?;
    let mut r = Reader::new(data);
    r.magic(b"FXR\0")?;
    if r.u32()? != 0x0005_0000 || r.u32()? != 1 {
        return bail("expected little-endian Sekiro FXR version 5");
    }
    let id = r.u32()?;
    let mut tables = Vec::new();
    for &stride in &STRIDES[..11] {
        let offset = r.u32()? as usize;
        let count = r.u32()? as usize;
        slice(
            data,
            offset,
            count
                .checked_mul(stride)
                .ok_or_else(|| crate::Error::Format("FXR table length overflow".into()))?,
        )?;
        tables.push(Table {
            offset,
            count,
            stride,
        });
    }
    // Version 5 inserts two header flags before its three additional tables.
    r.u32()?;
    r.u32()?;
    for &stride in &STRIDES[11..] {
        let offset = r.u32()? as usize;
        let count = r.u32()? as usize;
        slice(
            data,
            offset,
            count
                .checked_mul(stride)
                .ok_or_else(|| crate::Error::Format("FXR table length overflow".into()))?,
        )?;
        tables.push(Table {
            offset,
            count,
            stride,
        });
    }
    let range =
        |table: usize, offset: usize, count: u32| reference(&tables[table], offset, count as usize);
    let values = |offset: usize, count: usize| {
        reference(&tables[10], offset, count)?;
        words(data, offset, count)
    };
    let mut states = Vec::new();
    let table = &tables[0];
    for i in 0..table.count {
        let at = table.offset + i * table.stride;
        states.push(State {
            tag: word(data, at)?,
            transitions: range(1, offset(data, at + 8)?, word(data, at + 4)?)?,
        });
    }
    let mut transitions = Vec::new();
    let table = &tables[1];
    for i in 0..table.count {
        let at = table.offset + i * table.stride;
        transitions.push(Transition {
            tag: word(data, at)?,
            conditions: range(2, offset(data, at + 8)?, word(data, at + 4)?)?,
        });
    }
    let mut conditions = Vec::new();
    let table = &tables[2];
    for i in 0..table.count {
        conditions.push(Condition {
            words: words(data, table.offset + i * table.stride, 24)?,
        });
    }
    let mut nodes = Vec::new();
    let table = &tables[3];
    for i in 0..table.count {
        let at = table.offset + i * table.stride;
        nodes.push(Node {
            tag: word(data, at)?,
            configurations: range(4, offset(data, at + 24)?, word(data, at + 8)?)?,
            actions: range(5, offset(data, at + 32)?, word(data, at + 12)?)?,
            children: range(3, offset(data, at + 40)?, word(data, at + 16)?)?,
        });
    }
    let mut configurations = Vec::new();
    let table = &tables[4];
    for i in 0..table.count {
        let at = table.offset + i * table.stride;
        configurations.push(Configuration {
            tag: word(data, at)?,
            fields: values(offset(data, at + 16)?, word(data, at + 8)? as usize)?,
            actions: range(5, offset(data, at + 24)?, word(data, at + 12)?)?,
        });
    }
    let mut actions = Vec::new();
    let table = &tables[5];
    for i in 0..table.count {
        let at = table.offset + i * table.stride;
        let first_fields = word(data, at + 8)? as usize;
        let second_fields = word(data, at + 20)? as usize;
        let first_properties = word(data, at + 16)?;
        let second_properties = word(data, at + 28)?;
        let fields_offset = offset(data, at + 32)?;
        let property_offset = offset(data, at + 48)?;
        actions.push(Action {
            tag: word(data, at)?,
            variant: word(data, at + 4)?,
            fields: [
                values(fields_offset, first_fields)?,
                values(fields_offset + first_fields * 4, second_fields)?,
            ],
            properties: [
                range(6, property_offset, first_properties)?,
                range(
                    6,
                    property_offset
                        .checked_add(first_properties as usize * 40)
                        .ok_or_else(|| {
                            crate::Error::Format("FXR property offset overflow".into())
                        })?,
                    second_properties,
                )?,
            ],
            sequences: range(9, offset(data, at + 40)?, word(data, at + 12)?)?,
        });
    }
    let mut properties = Vec::new();
    let table = &tables[6];
    for i in 0..table.count {
        let at = table.offset + i * table.stride;
        properties.push(Property {
            tag: word(data, at)?,
            evaluator: word(data, at + 4)?,
            values: values(offset(data, at + 16)?, word(data, at + 8)? as usize)?,
            modifiers: range(7, offset(data, at + 24)?, word(data, at + 32)?)?,
        });
    }
    let mut modifiers = Vec::new();
    let table = &tables[7];
    for i in 0..table.count {
        let at = table.offset + i * table.stride;
        modifiers.push(Modifier {
            tag: word(data, at)?,
            evaluator: word(data, at + 4)?,
            values: values(offset(data, at + 16)?, word(data, at + 8)? as usize)?,
        });
    }
    let mut sequences = Vec::new();
    let table = &tables[9];
    for i in 0..table.count {
        let at = table.offset + i * table.stride;
        sequences.push(Sequence {
            values: values(offset(data, at)?, word(data, at + 8)? as usize)?,
        });
    }
    Ok(Fxr {
        id,
        tables,
        states,
        transitions,
        conditions,
        nodes,
        configurations,
        actions,
        properties,
        modifiers,
        sequences,
        bytes: data.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_truncated_headers_and_unsupported_versions() {
        for length in 0..144 {
            let mut data = vec![0; length];
            if length >= 12 {
                data[..4].copy_from_slice(b"FXR\0");
                data[4..8].copy_from_slice(&0x0005_0000u32.to_le_bytes());
                data[8..12].copy_from_slice(&1u32.to_le_bytes());
            }
            assert!(parse(&data).is_err(), "length {length}");
        }
        assert!(parse(&[0; 144]).is_err());
    }

    #[test]
    fn table_references_reject_misalignment_and_overrun() {
        let table = Table {
            offset: 144,
            count: 3,
            stride: 48,
        };
        assert_eq!(reference(&table, 192, 2).unwrap(), 1..3);
        assert!(reference(&table, 143, 1).is_err());
        assert!(reference(&table, 145, 1).is_err());
        assert!(reference(&table, 240, 2).is_err());
    }

    #[test]
    fn rejects_links_outside_the_value_pool_and_full_width_bad_offsets() {
        let mut bytes = vec![0; 260];
        bytes[..4].copy_from_slice(b"FXR\0");
        for (at, value) in [
            (4, 0x0005_0000u32),
            (8, 1),
            (12, 42),
            (40, 144),
            (44, 1), // node table
            (56, 192),
            (60, 1), // action table
            (96, 256),
            (100, 1), // raw-value pool
            (144, 0x0100_07d0),
            (156, 1),
            (176, 192),
            (192, 0x0100_0023),
            (200, 1),
            (224, 256),
            (256, 0x3f80_0000),
        ] {
            bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        let effect = parse(&bytes).unwrap();
        assert_eq!(effect.nodes[0].actions, 0..1);
        assert_eq!(effect.actions[0].fields[0], [0x3f80_0000]);
        let mut bad = bytes.clone();
        bad[224..232].copy_from_slice(&0u64.to_le_bytes());
        assert!(parse(&bad).is_err());
        let mut bad = bytes.clone();
        bad[224..232].copy_from_slice(&(256u64 + (1 << 32)).to_le_bytes());
        assert!(parse(&bad).is_err());
        let mut bad = bytes;
        bad[200..204].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(parse(&bad).is_err());
    }

    #[test]
    fn decodes_curve_arrays_without_discarding_cubic_controls() {
        let property = |tag, values: &[f32]| Property {
            tag,
            evaluator: 0,
            values: values.iter().map(|v| v.to_bits()).collect(),
            modifiers: 0..0,
        };
        let linear = property(
            0x0100_1040,
            &[0.0, -4.0, 12.0, 0.0, 2.0, 7.0, 12.0, 3.0, -4.0],
        );
        let curve = linear.curve().unwrap();
        assert_eq!(curve.times, [0.0, 2.0, 7.0]);
        assert_eq!(curve.values, [vec![12.0], vec![3.0], vec![-4.0]]);
        assert!(curve.looping);
        assert_eq!(curve.function, 4);
        let cubic = property(
            0x0100_0060,
            &[0.0, 2.0, 20.0, 0.0, 3.0, 2.0, 20.0, 0.2, 0.4, 0.6, 0.8],
        );
        let curve = cubic.curve().unwrap();
        assert_eq!(curve.values, [vec![2.0], vec![20.0]]);
        assert_eq!(curve.controls, [0.2, 0.4, 0.6, 0.8]);
        assert!(property(0x0100_0040, &[0.0; 4]).curve().is_none());
        assert_eq!(property(0x0100_0013, &[]).constant(), Some(vec![1.0; 4]));
    }
}

//! Sekiro version-2 baked local lights, decoded from the installed BTL bytes.
//! Header: version, header marker, count, UTF-16 byte length, reserved, record size;
//! strings begin at 0x3c, followed by count records of 0xe8 bytes.
//! Unknown fields remain available verbatim rather than acquiring invented semantics.
use crate::reader::{Reader, Result, bail, slice, utf16z};

#[derive(Debug, Clone)]
pub struct Light {
    pub name: String,
    /// Observed value 1 for every local light in the nine installed gameplay maps.
    pub kind: u8,
    pub color: [u8; 3],
    pub intensity: f32,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub range: f32,
    pub raw: Vec<u8>,
}

pub fn parse(data: &[u8]) -> Result<Vec<Light>> {
    let mut r = Reader::new(data);
    if r.u32()? != 2 || r.u32()? != 16 {
        return bail("unsupported BTL header");
    }
    let count = r.u32()? as usize;
    let string_size = r.u32()? as usize;
    if r.u32()? != 0 || r.u32()? != 232 {
        return bail("unsupported BTL record size");
    }
    let strings = slice(data, 60, string_size)?;
    let start = 60usize
        .checked_add(string_size)
        .ok_or_else(|| crate::Error::Format("BTL size overflow".into()))?;
    let size = count
        .checked_mul(232)
        .ok_or_else(|| crate::Error::Format("BTL count overflow".into()))?;
    let records = slice(data, start, size)?;
    if start + size != data.len() {
        return bail("unexpected BTL trailing bytes");
    }
    let mut lights = Vec::with_capacity(count);
    for raw in records.as_chunks::<232>().0 {
        let mut r = Reader::at(raw, 16);
        let name_offset = r.u32()? as usize;
        if !name_offset.is_multiple_of(2) || name_offset >= strings.len() {
            return bail("invalid BTL name offset");
        }
        let name = utf16z(strings, name_offset)?;
        r.seek(28);
        let kind = r.u8()?;
        let color = [r.u8()?, r.u8()?, r.u8()?];
        let intensity = r.f32()?;
        r.seek(56);
        let position = [r.f32()?, r.f32()?, r.f32()?];
        let rotation = [r.f32()?, r.f32()?, r.f32()?];
        r.seek(88);
        let range = r.f32()?;
        if !intensity.is_finite()
            || intensity < 0.
            || !range.is_finite()
            || range < 0.
            || position
                .iter()
                .chain(rotation.iter())
                .any(|v| !v.is_finite())
        {
            return bail("invalid BTL light values");
        }
        lights.push(Light {
            name,
            kind,
            color,
            intensity,
            position,
            rotation,
            range,
            raw: raw.to_vec(),
        });
    }
    Ok(lights)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installed_castle_and_truncation() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../cache/raw/map/m11_01_00_00");
        let Ok(data) = std::fs::read(root.join("m11_01_00_00_0000.btl")) else {
            return;
        };
        let lights = parse(&data).unwrap();
        assert_eq!(lights.len(), 34);
        assert_eq!(lights[0].color, [255, 121, 12]);
        assert_eq!(lights[0].intensity, 0.3);
        assert_eq!(lights[0].range, 1.);
        assert!((lights[0].position[0] + 344.742).abs() < 0.001);
        assert!(!lights[0].name.is_empty());
        for length in [0, 23, 59, 1580, data.len() - 1] {
            assert!(parse(&data[..length]).is_err());
        }
        let mut invalid = data.clone();
        invalid[1580 + 16..1580 + 20].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(parse(&invalid).is_err());
    }
    #[test]
    fn installed_maps_census() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cache/raw/map");
        let Ok(maps) = std::fs::read_dir(root) else {
            return;
        };
        for map in maps.flatten().filter(|e| e.path().is_dir()) {
            for file in std::fs::read_dir(map.path())
                .unwrap()
                .flatten()
                .filter(|e| e.path().extension().is_some_and(|s| s == "btl"))
            {
                let lights = parse(&std::fs::read(file.path()).unwrap()).unwrap();
                assert!(lights.iter().all(|l| l.kind == 1 && l.range > 0.));
            }
        }
    }
}

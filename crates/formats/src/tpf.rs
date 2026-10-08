//! TPF texture containers and the DDS headers inside them.
//!
//! Layout knowledge comes from SoulsFormatsNEXT (`TPF/TPF.cs`), reimplemented. Only the PC
//! platform is supported: its payloads are plain DDS files.

use crate::reader::{Reader, Result, bail, sjisz, utf16z};

#[derive(Debug, Clone)]
pub struct TpfTexture<'a> {
    pub name: String,
    /// FromSoftware's own format byte (not DXGI).
    pub format: u8,
    /// 0 texture, 1 cubemap, 2 volume.
    pub kind: u8,
    pub mipmaps: u8,
    /// The DDS file, header included.
    pub dds: &'a [u8],
}

impl TpfTexture<'_> {
    pub fn header(&self) -> Result<Dds> {
        Dds::parse(self.dds)
    }
}

pub fn is_tpf(data: &[u8]) -> bool {
    data.starts_with(b"TPF\0")
}

/// Parses a PC TPF. Texture payloads borrow from `data`.
pub fn parse(data: &[u8]) -> Result<Vec<TpfTexture<'_>>> {
    let mut r = Reader::new(data);
    r.magic(b"TPF\0")?;
    let _data_length = r.u32()?;
    let count = r.u32()? as usize;
    let platform = r.u8()?;
    let _flag2 = r.u8()?;
    let encoding = r.u8()?;
    r.u8()?;
    if platform != 0 {
        return bail(format!("TPF platform {platform} is not PC"));
    }
    let mut textures = Vec::with_capacity(count);
    for _ in 0..count {
        let offset = r.u32()? as usize;
        let size = r.u32()? as usize;
        let format = r.u8()?;
        let kind = r.u8()?;
        let mipmaps = r.u8()?;
        let flags1 = r.u8()?;
        let name_offset = r.u32()? as usize;
        let has_float_struct = r.u32()? == 1;
        if has_float_struct {
            let _unk = r.i32()?;
            let len = r.u32()? as usize;
            r.skip(len);
        }
        if flags1 == 2 || flags1 == 3 {
            return bail("DCP_EDGE compressed TPF payloads are not supported");
        }
        let name = if encoding == 1 {
            utf16z(data, name_offset)?
        } else {
            sjisz(data, name_offset)?
        };
        textures.push(TpfTexture {
            name,
            format,
            kind,
            mipmaps,
            dds: crate::reader::slice(data, offset, size)?,
        });
    }
    Ok(textures)
}

/// Pixel formats found in Sekiro's PC textures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DdsFormat {
    Bc1,
    Bc2,
    Bc3,
    Bc4,
    Bc5,
    Bc6hUf16,
    Bc6hSf16,
    Bc7,
    Bgra8,
    Rgba8,
    Unknown(u32),
}

impl DdsFormat {
    /// Bytes per 4x4 block for block-compressed formats, or bytes per pixel otherwise.
    pub fn block_bytes(self) -> Option<(usize, bool)> {
        use DdsFormat::*;
        match self {
            Bc1 | Bc4 => Some((8, true)),
            Bc2 | Bc3 | Bc5 | Bc6hUf16 | Bc6hSf16 | Bc7 => Some((16, true)),
            Bgra8 | Rgba8 => Some((4, false)),
            Unknown(_) => None,
        }
    }

    fn from_dxgi(v: u32) -> (Self, bool) {
        use DdsFormat::*;
        match v {
            28 => (Rgba8, false),
            29 => (Rgba8, true),
            70..=71 => (Bc1, false),
            72 => (Bc1, true),
            73..=74 => (Bc2, false),
            75 => (Bc2, true),
            76..=77 => (Bc3, false),
            78 => (Bc3, true),
            79..=81 => (Bc4, false),
            82..=84 => (Bc5, false),
            87 => (Bgra8, false),
            91 => (Bgra8, true),
            94..=95 => (Bc6hUf16, false),
            96 => (Bc6hSf16, false),
            97..=98 => (Bc7, false),
            99 => (Bc7, true),
            other => (Unknown(other), false),
        }
    }
}

/// The parts of a DDS header needed to decode its first image.
#[derive(Debug, Clone, Copy)]
pub struct Dds {
    pub width: u32,
    pub height: u32,
    pub mip_count: u32,
    pub format: DdsFormat,
    pub srgb: bool,
    pub cubemap: bool,
    /// Byte offset of the pixel data (after the DX10 extension when present).
    pub data_offset: usize,
}

impl Dds {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut r = Reader::new(data);
        r.magic(b"DDS ")?;
        let _size = r.u32()?;
        let _flags = r.u32()?;
        let height = r.u32()?;
        let width = r.u32()?;
        let _pitch = r.u32()?;
        let _depth = r.u32()?;
        let mip_count = r.u32()?.max(1);
        r.seek(0x50);
        let pf_flags = r.u32()?;
        let fourcc = r.bytes(4)?;
        let bit_count = r.u32()?;
        let r_mask = r.u32()?;
        r.seek(0x70);
        let caps2 = r.u32()?;
        let mut data_offset = 0x80;
        let (format, srgb) = if pf_flags & 0x4 != 0 {
            match fourcc {
                b"DX10" => {
                    let mut x = Reader::at(data, 0x80);
                    let dxgi = x.u32()?;
                    data_offset += 20;
                    DdsFormat::from_dxgi(dxgi)
                }
                b"DXT1" => (DdsFormat::Bc1, false),
                b"DXT2" | b"DXT3" => (DdsFormat::Bc2, false),
                b"DXT4" | b"DXT5" => (DdsFormat::Bc3, false),
                b"ATI1" | b"BC4U" => (DdsFormat::Bc4, false),
                b"ATI2" | b"BC5U" => (DdsFormat::Bc5, false),
                other => (
                    DdsFormat::Unknown(u32::from_le_bytes(other.try_into().unwrap())),
                    false,
                ),
            }
        } else if bit_count == 32 {
            if r_mask == 0x00ff_0000 {
                (DdsFormat::Bgra8, false)
            } else {
                (DdsFormat::Rgba8, false)
            }
        } else {
            (DdsFormat::Unknown(0), false)
        };
        Ok(Self {
            width,
            height,
            mip_count,
            format,
            srgb,
            cubemap: caps2 & 0x200 != 0,
            data_offset,
        })
    }

    /// Byte length of mip level 0 of the first image.
    pub fn top_mip_len(&self) -> Option<usize> {
        let (bytes, blocks) = self.format.block_bytes()?;
        let (w, h) = (self.width as usize, self.height as usize);
        Some(if blocks {
            w.div_ceil(4) * h.div_ceil(4) * bytes
        } else {
            w * h * bytes
        })
    }

    /// The pixel data of mip level 0 of the first image (first face for cubemaps).
    pub fn top_mip<'a>(&self, data: &'a [u8]) -> Result<&'a [u8]> {
        let Some(len) = self.top_mip_len() else {
            return bail(format!("unsupported DDS format {:?}", self.format));
        };
        crate::reader::slice(data, self.data_offset, len)
    }
}

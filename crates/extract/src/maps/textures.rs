//! Map textures: every `map/mXX/*.tpfbhd` split binder, `other/maptex.tpf` and the TPFs of
//! placed objects, indexed by texture name. Textures are written out as the DDS files the TPFs already contain, so the GPU
//! gets the original block-compressed data with its mip chain.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use memmap2::Mmap;
use sekiro_formats::{bxf4, dcx::Oodle, tpf};

enum Source {
    /// Entry `entry` of split binder `binder`.
    Bxf { binder: usize, entry: usize },
    /// Texture `index` of loose TPF `file`.
    Tpf { file: usize, index: usize },
}

struct Binder {
    bhd: Vec<u8>,
    bdt: Mmap,
}

pub struct MapTextures {
    oodle: &'static Oodle,
    binders: Vec<Binder>,
    tpfs: Vec<Vec<u8>>,
    by_name: HashMap<String, Source>,
}

impl MapTextures {
    /// Indexes every map texture binder under `raw/map` and the shared `other/maptex.tpf`.
    pub fn open(raw: &Path, oodle: &'static Oodle) -> Result<Self> {
        let mut store = Self {
            oodle,
            binders: Vec::new(),
            tpfs: Vec::new(),
            by_name: HashMap::new(),
        };
        let mut bhds: Vec<PathBuf> = Vec::new();
        for area in std::fs::read_dir(raw.join("map"))? {
            let area = area?.path();
            if !area.is_dir() {
                continue;
            }
            for f in std::fs::read_dir(&area)? {
                let f = f?.path();
                if f.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("tpfbhd"))
                {
                    bhds.push(f);
                }
            }
        }
        bhds.sort();
        for bhd_path in bhds {
            let bdt_path = bhd_path.with_extension("tpfbdt");
            let bhd = std::fs::read(&bhd_path)?;
            let file = std::fs::File::open(&bdt_path)
                .with_context(|| format!("opening {}", bdt_path.display()))?;
            // SAFETY: read-only mapping of a cache file nothing else writes while we run.
            let bdt = unsafe { Mmap::map(&file)? };
            let binder = store.binders.len();
            for (entry, f) in bxf4::parse(&bhd, &bdt)?.iter().enumerate() {
                let stem = tpf_stem(f.file_name());
                store
                    .by_name
                    .entry(stem)
                    .or_insert(Source::Bxf { binder, entry });
            }
            store.binders.push(Binder { bhd, bdt });
        }
        let maptex = raw.join("other/maptex.tpf");
        if maptex.is_file() {
            store.add_tpf(&maptex)?;
        }
        Ok(store)
    }

    /// Makes the textures of a loose TPF findable. Earlier sources win on name clashes.
    pub fn add_tpf(&mut self, path: &Path) -> Result<()> {
        let data = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        let file = self.tpfs.len();
        for (index, t) in tpf::parse(&data)?.iter().enumerate() {
            self.by_name
                .entry(t.name.to_ascii_lowercase())
                .or_insert(Source::Tpf { file, index });
        }
        self.tpfs.push(data);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    /// The DDS file for a texture name, or None when it is unknown.
    pub fn dds(&self, name: &str) -> Result<Option<Vec<u8>>> {
        let name = name.to_ascii_lowercase();
        match self.by_name.get(&name) {
            None => Ok(None),
            Some(Source::Tpf { file, index }) => {
                Ok(Some(tpf::parse(&self.tpfs[*file])?[*index].dds.to_vec()))
            }
            Some(Source::Bxf { binder, entry }) => {
                let b = &self.binders[*binder];
                let files = bxf4::parse(&b.bhd, &b.bdt)?;
                let data = files[*entry].contents(Some(self.oodle))?;
                let textures = tpf::parse(&data)?;
                let t = textures
                    .iter()
                    .find(|t| t.name.eq_ignore_ascii_case(&name))
                    .or(textures.first())
                    .context("empty TPF")?;
                Ok(Some(t.dds.to_vec()))
            }
        }
    }
}

/// Converts a Sekiro normal map (X in red, Y in green, DirectX-style Y down, other channels
/// unrelated) to a BC5 DDS with a full mip chain and glTF's Y-up convention. Bevy treats BC5
/// normal maps as two-channel and rebuilds Z, which is exactly what the source data needs.
pub fn normal_to_bc5(dds: &[u8]) -> anyhow::Result<Vec<u8>> {
    let header = tpf::Dds::parse(dds)?;
    let rgba = crate::models::textures::decode_rgba(&header, dds)?;
    let (mut w, mut h) = (header.width as usize, header.height as usize);
    // Two 8-bit channels: X, and Y flipped to point up the texture.
    let mut rg: Vec<[u8; 2]> = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| [p[0], 255 - p[1]])
        .collect();
    let mut body = Vec::new();
    let mut mips = 0u32;
    loop {
        encode_bc5(&rg, w, h, &mut body);
        mips += 1;
        if w == 1 && h == 1 {
            break;
        }
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let mut next = vec![[0u8; 2]; nw * nh];
        for y in 0..nh {
            for x in 0..nw {
                let mut sum = [0u32; 2];
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let (sx, sy) = ((2 * x + dx).min(w - 1), (2 * y + dy).min(h - 1));
                    let p = rg[sy * w + sx];
                    sum[0] += p[0] as u32;
                    sum[1] += p[1] as u32;
                }
                next[y * nw + x] = [(sum[0] / 4) as u8, (sum[1] / 4) as u8];
            }
        }
        rg = next;
        (w, h) = (nw, nh);
    }
    let mut out = Vec::with_capacity(148 + body.len());
    let u32le = |out: &mut Vec<u8>, v: u32| out.extend_from_slice(&v.to_le_bytes());
    out.extend_from_slice(b"DDS ");
    u32le(&mut out, 124);
    // CAPS | HEIGHT | WIDTH | PIXELFORMAT | MIPMAPCOUNT | LINEARSIZE
    u32le(&mut out, 0x1 | 0x2 | 0x4 | 0x1000 | 0x20000 | 0x80000);
    u32le(&mut out, header.height);
    u32le(&mut out, header.width);
    u32le(
        &mut out,
        (header.width.div_ceil(4) * header.height.div_ceil(4) * 16).max(16),
    );
    u32le(&mut out, 0);
    u32le(&mut out, mips);
    out.extend_from_slice(&[0u8; 44]);
    // Pixel format: FOURCC "DX10".
    u32le(&mut out, 32);
    u32le(&mut out, 0x4);
    out.extend_from_slice(b"DX10");
    out.extend_from_slice(&[0u8; 20]);
    u32le(&mut out, 0x1000 | 0x8 | 0x400000); // TEXTURE | COMPLEX | MIPMAP
    out.extend_from_slice(&[0u8; 16]);
    // DX10 header: DXGI_FORMAT_BC5_UNORM, 2D texture, one array element.
    u32le(&mut out, 83);
    u32le(&mut out, 3);
    u32le(&mut out, 0);
    u32le(&mut out, 1);
    u32le(&mut out, 0);
    out.extend_from_slice(&body);
    Ok(out)
}

/// Appends one BC5 mip level (two BC4 blocks per 4x4 tile).
fn encode_bc5(rg: &[[u8; 2]], w: usize, h: usize, out: &mut Vec<u8>) {
    for by in 0..h.div_ceil(4) {
        for bx in 0..w.div_ceil(4) {
            for c in [0usize, 1] {
                let mut v = [0u8; 16];
                for (i, t) in v.iter_mut().enumerate() {
                    let (x, y) = ((bx * 4 + i % 4).min(w - 1), (by * 4 + i / 4).min(h - 1));
                    *t = rg[y * w + x][c];
                }
                out.extend_from_slice(&encode_bc4(&v));
            }
        }
    }
}

/// A BC4 block in its eight-value mode: endpoints are the block's max and min, indices pick
/// the nearest of the eight interpolated values.
fn encode_bc4(v: &[u8; 16]) -> [u8; 8] {
    let hi = *v.iter().max().unwrap();
    let lo = *v.iter().min().unwrap();
    let mut out = [0u8; 8];
    out[0] = hi;
    out[1] = lo;
    if hi == lo {
        return out;
    }
    // Palette in code order: 0 = hi, 1 = lo, 2..7 = (6 hi + lo) / 7 .. (hi + 6 lo) / 7.
    let mut palette = [0f32; 8];
    palette[0] = hi as f32;
    palette[1] = lo as f32;
    for k in 1..7 {
        palette[k + 1] = ((7 - k) as f32 * hi as f32 + k as f32 * lo as f32) / 7.0;
    }
    let mut bits: u64 = 0;
    for (i, &x) in v.iter().enumerate() {
        let code = (0..8)
            .min_by(|&a, &b| {
                (palette[a] - x as f32)
                    .abs()
                    .total_cmp(&(palette[b] - x as f32).abs())
            })
            .unwrap() as u64;
        bits |= code << (3 * i);
    }
    out[2..8].copy_from_slice(&bits.to_le_bytes()[..6]);
    out
}

/// `m11\tex\m11_rock_04_a.tpf.dcx` -> `m11_rock_04_a`.
fn tpf_stem(file_name: &str) -> String {
    let lower = file_name.to_ascii_lowercase();
    let lower = lower.strip_suffix(".dcx").unwrap_or(&lower);
    lower.strip_suffix(".tpf").unwrap_or(lower).to_string()
}

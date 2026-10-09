//! Map textures: every `map/mXX/*.tpfbhd` split binder plus `other/maptex.tpf`, indexed by
//! texture name. Textures are written out as the DDS files the TPFs already contain, so the GPU
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
        for loose in ["other/maptex.tpf"] {
            let path = raw.join(loose);
            if let Ok(data) = std::fs::read(&path) {
                let file = store.tpfs.len();
                for (index, t) in tpf::parse(&data)?.iter().enumerate() {
                    store
                        .by_name
                        .entry(t.name.to_ascii_lowercase())
                        .or_insert(Source::Tpf { file, index });
                }
                store.tpfs.push(data);
            }
        }
        Ok(store)
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

/// `m11\tex\m11_rock_04_a.tpf.dcx` -> `m11_rock_04_a`.
fn tpf_stem(file_name: &str) -> String {
    let lower = file_name.to_ascii_lowercase();
    let lower = lower.strip_suffix(".dcx").unwrap_or(&lower);
    lower.strip_suffix(".tpf").unwrap_or(lower).to_string()
}

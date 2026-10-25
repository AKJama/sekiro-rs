//! Native foliage translucency for `M[Translucency]*` materials (grass, ferns, blinds).
//!
//! Every `M[Translucency]*` G-buffer shader writes `max(normal.a * float_0, 0) *
//! FC_TranslucencyScale * 0.1` to G-buffer 1 `.y`, where `normal.a` is the normal map's alpha
//! (a translucency mask) and `float_0` is the MTD's `*_snp_0_float_0`. The deferred sun and fill
//! (`GXLightAcc_DDirLightShadowed1`) then read `k` (that value times 10) and light the pixel with
//! `max(N.L + k, 0) / (1 + 2k)` plus a back-light lobe, so thin blades stay lit on both sides.
//!
//! The exported normal maps are BC5 and drop the alpha mask, so each material takes the mean
//! mask over its opaque texels, read from the game's own texture. `FC_TranslucencyScale` is a
//! CPU constant that is not traced; 1 is assumed.
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use sekiro_formats::mtd::{self, Mtd, ParamValue};
use sekiro_formats::tpf::{self, Dds, DdsFormat};

/// Assumed `FC_TranslucencyScale` (cbMatDynParam); its CPU producer is not traced.
const TRANSLUCENCY_SCALE: f32 = 1.;

/// G-buffer 1 is UNORM and stores `k * 0.1`, so `k` saturates at 10.
const MAX_TRANSLUCENCY: f32 = 10.;

/// Native alpha test (`g_AlphaRef` 128 on every grass MTD).
const OPAQUE_ALPHA: u32 = 128;

pub(super) struct Translucency {
    mtd_dir: PathBuf,
    obj_dir: PathBuf,
    materials: HashMap<String, f32>,
    enabled: bool,
}

impl Translucency {
    pub(super) fn load(map_dir: &Path, enabled: bool) -> Self {
        let cache = map_dir.parent().and_then(Path::parent).unwrap_or(map_dir);
        Self {
            mtd_dir: cache.join("raw/mtd/allmaterialbnd.mtdbnd.d"),
            obj_dir: cache.join("raw/obj"),
            materials: HashMap::new(),
            enabled,
        }
    }

    /// The native translucency `k` for an exported material name (`"m0 label | MTD"`), or 0.
    pub(super) fn material(&mut self, name: &str) -> f32 {
        if !self.enabled {
            return 0.;
        }
        let Some((_, name)) = name.rsplit_once(" | ") else {
            return 0.;
        };
        if name.contains(['/', '\\']) || name.contains("..") {
            return 0.;
        }
        if let Some(&k) = self.materials.get(name) {
            return k;
        }
        let k = std::fs::read(self.mtd_dir.join(format!("{name}.mtd")))
            .ok()
            .and_then(|bytes| mtd::parse(&bytes).ok())
            .and_then(|m| strength(&m, |path| mean_mask(&self.obj_dir, path)))
            .unwrap_or(0.);
        self.materials.insert(name.to_owned(), k);
        k
    }
}

/// `k = max(mask * float_0, 0) * FC_TranslucencyScale` for an `M[Translucency]*` MTD, with the
/// mask from `mask(normal, albedo)`. Other shaders, and masks that cannot be read, give `None`.
fn strength(m: &Mtd, mask: impl FnOnce(&MaskSource) -> Option<f32>) -> Option<f32> {
    let shader = m.shader.rsplit(['\\', '/']).next()?;
    if !shader.starts_with("M[Translucency]") {
        return None;
    }
    let float_0 = m.params.iter().find_map(|p| match &p.value {
        ParamValue::Floats(v) if p.name.ends_with("_snp_0_float_0") => v.first().copied(),
        _ => None,
    })?;
    let texture = |slot: &str| {
        m.textures
            .iter()
            .find(|t| t.kind.contains(slot) && !t.path.is_empty())
            .map(|t| t.path.as_str())
    };
    let source = MaskSource {
        normal: texture("NormalMap")?,
        albedo: texture("AlbedoMap"),
    };
    let k = (mask(&source)? * float_0).max(0.) * TRANSLUCENCY_SCALE;
    Some(k.min(MAX_TRANSLUCENCY))
}

/// The MTD's default texture paths, e.g. `N:\...\obj\o00\o007600\tex\o007600_Grass_01_n.tif`.
struct MaskSource<'a> {
    normal: &'a str,
    albedo: Option<&'a str>,
}

/// Mean normal-map alpha over texels the albedo alpha test keeps, from the object's own TPF.
fn mean_mask(obj_dir: &Path, source: &MaskSource) -> Option<f32> {
    let (object, normal) = object_texture(source.normal)?;
    let dir = obj_dir.join(format!("{object}.objbnd.d"));
    let mut tpfs = std::fs::read_dir(&dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("tpf")))
        .collect::<Vec<_>>();
    tpfs.sort();
    let files = tpfs
        .iter()
        .filter_map(|p| std::fs::read(p).ok())
        .collect::<Vec<_>>();
    let find = |name: &str| {
        files.iter().find_map(|bytes| {
            tpf::parse(bytes)
                .ok()?
                .into_iter()
                .find(|t| t.name.eq_ignore_ascii_case(name))
                .and_then(|t| decode(t.dds))
        })
    };
    let normal = find(normal)?;
    let albedo = source
        .albedo
        .and_then(object_texture)
        .filter(|(o, _)| o.eq_ignore_ascii_case(object))
        .and_then(|(_, name)| find(name));
    masked_mean(&normal, albedo.as_ref())
}

/// `(object, texture)` from an object texture path; other paths give `None`.
fn object_texture(path: &str) -> Option<(&str, &str)> {
    let parts: Vec<&str> = path.split(['\\', '/']).collect();
    let tex = parts.iter().rposition(|p| p.eq_ignore_ascii_case("tex"))?;
    let object = *parts.get(tex.checked_sub(1)?)?;
    if !parts[..tex].iter().any(|p| p.eq_ignore_ascii_case("obj")) {
        return None;
    }
    let file = parts.last()?;
    let stem = file.rsplit_once('.').map_or(*file, |(s, _)| s);
    Some((object, stem))
}

/// An RGBA image as `width`, `height` and texels (alpha in the top byte).
struct Texels(usize, usize, Vec<u32>);

/// Decodes the smallest mip at least 64 texels on each side (or mip 0 if smaller).
fn decode(dds: &[u8]) -> Option<Texels> {
    let header = Dds::parse(dds).ok()?;
    let (block, _) = header.format.block_bytes()?;
    let size = |w: usize, h: usize| w.div_ceil(4) * h.div_ceil(4) * block;
    let (mut w, mut h) = (header.width as usize, header.height as usize);
    let mut offset = header.data_offset;
    for _ in 1..header.mip_count {
        if w / 2 < 64 || h / 2 < 64 {
            break;
        }
        offset += size(w, h);
        (w, h) = (w / 2, h / 2);
    }
    let data = dds.get(offset..offset + size(w, h))?;
    let mut texels = vec![0u32; w * h];
    match header.format {
        DdsFormat::Bc1 => texture2ddecoder::decode_bc1a(data, w, h, &mut texels),
        DdsFormat::Bc3 => texture2ddecoder::decode_bc3(data, w, h, &mut texels),
        DdsFormat::Bc7 => texture2ddecoder::decode_bc7(data, w, h, &mut texels),
        _ => return None,
    }
    .ok()?;
    Some(Texels(w, h, texels))
}

fn alpha(texel: u32) -> u32 {
    texel >> 24
}

/// Mean `normal` alpha in 0..1 over texels whose `albedo` alpha passes the native alpha test.
fn masked_mean(normal: &Texels, albedo: Option<&Texels>) -> Option<f32> {
    let Texels(w, h, texels) = normal;
    let (mut sum, mut count) = (0u64, 0u64);
    for y in 0..*h {
        for x in 0..*w {
            let kept = albedo.is_none_or(|Texels(aw, ah, a)| {
                alpha(a[(y * ah / h) * aw + x * aw / w]) >= OPAQUE_ALPHA
            });
            if kept {
                sum += u64::from(alpha(texels[y * w + x]));
                count += 1;
            }
        }
    }
    (count > 0).then(|| sum as f32 / count as f32 / 255.)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texels(w: usize, h: usize, alphas: &[u32]) -> Texels {
        Texels(w, h, alphas.iter().map(|a| a << 24).collect())
    }

    #[test]
    fn mask_mean_counts_only_texels_the_alpha_test_keeps() {
        let normal = texels(2, 2, &[255, 0, 51, 102]);
        let albedo = texels(1, 2, &[0, 255]);
        let mean = masked_mean(&normal, Some(&albedo)).unwrap();
        assert!((mean - 153. / 2. / 255.).abs() < 1e-6);
        assert!((masked_mean(&normal, None).unwrap() - 408. / 4. / 255.).abs() < 1e-6);
        assert_eq!(masked_mean(&normal, Some(&texels(1, 1, &[0]))), None);
    }

    #[test]
    fn object_texture_paths_name_the_object_and_texture() {
        assert_eq!(
            object_texture(r"N:\NTC\data\Model\obj\o00\o007600\tex\o007600_Grass_01_n.tif"),
            Some(("o007600", "o007600_Grass_01_n"))
        );
        assert_eq!(
            object_texture(r"N:\NTC\data\Model\map\m11\tex\m11_wood_n.tif"),
            None
        );
    }

    /// Ashina grass is `M[Translucency]_e`: its MTD float_0 and its normal maps' alpha make a
    /// translucency the deferred sun reads, so blades lit from behind are not black (#367).
    #[test]
    fn ashina_grass_takes_its_native_translucency() {
        let cache = sekiro_sim::cache::root();
        if !cache.join("raw/mtd/allmaterialbnd.mtdbnd.d").is_dir() {
            eprintln!("skipped: no raw MTDs");
            return;
        }
        let mut translucency = Translucency::load(&cache.join("maps/m11_00_00_00"), true);
        // The gatehouse-yard dead grass: float_0 3 and a mask of about 0.6 on the blades.
        let dead_grass = translucency.material("m0 dead grass | M[Grass_o007600]_tr");
        assert!((1.2..3.).contains(&dead_grass), "{dead_grass}");
        let small = translucency.material("m0 dead grass | M[Grass_o007610]_tr");
        assert!(small > 0.5, "{small}");
        // Ordinary map materials and switched-off lookups are untouched.
        assert_eq!(
            translucency.material("m8 timber | M[m11_01]_Wood2_multi_NoDF"),
            0.
        );
        assert_eq!(translucency.material("m0 | ../../untrusted"), 0.);
        assert_eq!(
            Translucency::load(&cache.join("maps/m11_00_00_00"), false)
                .material("m0 dead grass | M[Grass_o007600]_tr"),
            0.
        );
    }
}

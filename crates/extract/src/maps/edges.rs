//! Export action edges with exactly the collision mesh's mirror and part placement.
use anyhow::{Context, Result};
use glam::Vec3;
use sekiro_formats::{bxf4, dcx::Oodle, edge, msb};
use serde_json::json;
use std::path::Path;

pub(super) fn export(cache: &Path, id: &str, msb: &msb::Msb, oodle: &Oodle) -> Result<bool> {
    let base = cache.join(format!("raw/map/{id}/{id}"));
    let bhd = std::fs::read(base.with_extension("edgebhd"))?;
    let bdt = std::fs::read(base.with_extension("edgebdt"))?;
    let mut rows = Vec::new();
    let mut counts = [0usize; 4];
    for file in bxf4::parse(&bhd, &bdt)? {
        let data = file.contents(Some(oodle))?;
        let edges = edge::parse(&data).with_context(|| file.name.clone())?;
        let model = format!("h{:06}", edges.owner);
        for part in msb.parts.iter().filter(|p| {
            p.kind == msb::PartKind::Collision && msb.model(p).is_some_and(|m| m.name == model)
        }) {
            let matrix = super::mirror(part.matrix());
            let transform = |p: [f32; 3]| {
                matrix
                    .transform_point3(Vec3::new(-p[0], p[1], p[2]))
                    .to_array()
            };
            for e in &edges.edges {
                if (e.kind() as usize) < counts.len() {
                    counts[e.kind() as usize] += 1;
                }
                rows.push(json!({ "kind": e.kind(), "variation": e.variation(), "start": transform(e.start), "end": transform(e.end), "normal": null, "source_part": part.name, "chain": e.chain, "flags": e.flags }));
            }
        }
    }
    let out = cache.join("maps").join(id);
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("edges.json"), serde_json::to_vec_pretty(&rows)?)?;
    eprintln!(
        "{id}: {} placed edges (grapple {}, hang {}, cover {})",
        rows.len(),
        counts[1],
        counts[2],
        counts[3]
    );
    Ok(std::env::var_os("SEKIRO_EDGES_ONLY").is_some())
}

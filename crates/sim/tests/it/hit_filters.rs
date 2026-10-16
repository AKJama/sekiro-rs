//! Collision parts by MSB hit filter over the whole Ashina Outskirts and Ashina Castle blocks
//! (`cache/maps/m11_00_00_00`, `m11_01_00_00`), and which of them the player collides with
//! (`sekiro_sim::hit_filter`, traced in `docs/HIT-FILTERS.md`). Skips when the maps have not
//! been exported.

use std::collections::BTreeMap;
use std::path::PathBuf;

use glam::Vec3;
use sekiro_formats::hknp::CollisionMesh;
use sekiro_sim::body::Ground;
use sekiro_sim::collision::CollisionWorld;
use sekiro_sim::hit_filter;

fn load(id: &str) -> Option<(serde_json::Value, CollisionMesh)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../cache/maps")
        .join(id);
    if !dir.join("collision.bin").is_file() {
        eprintln!("skipping: {} not exported", dir.display());
        return None;
    }
    let layout = serde_json::from_slice(&std::fs::read(dir.join("layout.json")).unwrap()).unwrap();
    let mesh =
        CollisionMesh::from_bytes(&std::fs::read(dir.join("collision.bin")).unwrap()).unwrap();
    Some((layout, mesh))
}

/// Hit filter -> (parts, triangles) from the layout's collision rows.
fn census(layout: &serde_json::Value) -> BTreeMap<u8, (usize, usize)> {
    let mut out = BTreeMap::new();
    for row in layout["collisions"].as_array().unwrap() {
        let filter = row["hit_filter_id"].as_u64().unwrap() as u8;
        let triangles = row["triangles"][1].as_u64().unwrap_or(0) as usize;
        let entry = out.entry(filter).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += triangles;
    }
    out
}

/// (hit filter, parts, triangles, the player collides) for each map.
const OUTSKIRTS: [(u8, usize, usize, bool); 9] = [
    (8, 155, 2_323_885, true),
    (9, 1, 655, false),
    (11, 1, 0, false),
    (13, 1, 61, false),
    (15, 1, 60, false),
    (19, 2, 114, false),
    (22, 1, 16, true),
    (25, 2, 136, false),
    (28, 2, 1_428, false),
];
const CASTLE: [(u8, usize, usize, bool); 11] = [
    (8, 192, 1_652_576, true),
    (9, 1, 32, false),
    (11, 3, 66, false),
    (13, 2, 127, false),
    (15, 2, 127, false),
    (16, 1, 7, false),
    (19, 2, 14, false),
    (22, 3, 58, true),
    (26, 1, 228, false),
    (27, 12, 1_609, true),
    (28, 2, 1_540, false),
];

fn check(
    id: &str,
    expected: &[(u8, usize, usize, bool)],
) -> Option<(CollisionWorld, CollisionWorld)> {
    let (layout, mut mesh) = load(id)?;
    let census = census(&layout);
    let got: Vec<_> = census
        .iter()
        .map(|(&f, &(parts, tris))| (f, parts, tris, hit_filter::player_collides(f)))
        .collect();
    assert_eq!(got, expected, "{id}: collision parts by hit filter");
    let unstripped = CollisionWorld::from_mesh(&mesh);
    let triangles = mesh.triangle_count();
    let helpers = hit_filter::strip_helpers(&mut mesh, &layout);
    assert_eq!(
        mesh.triangle_count(),
        triangles,
        "{id}: triangle indices kept"
    );
    let removed: BTreeMap<u8, usize> = expected
        .iter()
        .filter(|e| !e.3 && e.2 > 0)
        .map(|e| (e.0, e.2))
        .collect();
    assert_eq!(
        helpers.removed, removed,
        "{id}: triangles kept off the player"
    );
    // Enemy and camera helpers stay, held by those that collide with them.
    let mut kept: BTreeMap<u8, usize> = BTreeMap::new();
    for e in expected
        .iter()
        .filter(|e| !e.3 && hit_filter::mask(e.0) != 0)
    {
        *kept.entry(hit_filter::mask(e.0)).or_default() += e.2;
    }
    let got: BTreeMap<u8, usize> = helpers
        .parts
        .iter()
        .map(|(&m, mesh)| (m, mesh.triangle_count()))
        .collect();
    assert_eq!(got, kept, "{id}: enemy and camera helpers");
    let walls: usize = helpers
        .triggers
        .iter()
        .map(|t| t.mesh.triangle_count())
        .sum();
    let event: usize = expected
        .iter()
        .filter(|e| hit_filter::is_event_wall(e.0))
        .map(|e| e.2)
        .sum();
    assert_eq!(walls, event, "{id}: event wall triangles");
    let mut world = CollisionWorld::from_mesh(&mesh);
    world.add_helpers(&helpers);
    // No player start stood on a helper: the floor under each is unchanged.
    for p in layout["players"].as_array().unwrap() {
        let v: Vec<f32> = p["translation"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap() as f32)
            .collect();
        let floor = Ground::floor(&world, v[0], v[2], v[1] + 0.5);
        assert!(floor.is_some(), "{id} {}: no floor under {v:?}", p["name"]);
        assert_eq!(
            floor,
            Ground::floor(&unstripped, v[0], v[2], v[1] + 0.5),
            "{id} {}",
            p["name"]
        );
    }
    Some((unstripped, world))
}

#[test]
fn outskirts_helpers_are_not_floors() {
    let Some((unstripped, world)) = check("m11_00_00_00", &OUTSKIRTS) else {
        return;
    };
    // The ravine north of start 0: the only "floor" under it was the filter-15 helper
    // h900000, an event wall the player falls through (the fall return).
    let (x, z) = (-137.6, -124.5);
    let helper = Ground::floor(&unstripped, x, z, -44.3).unwrap();
    assert!((helper + 74.18).abs() < 0.05, "{helper}");
    assert_eq!(Ground::floor(&world, x, z, -44.3), None);
    let contact = world
        .trigger_contact(Vec3::new(x, -70.0, z), Vec3::new(x, -78.0, z))
        .expect("the drop meets h900000");
    assert_eq!((contact.hit_filter, contact.part.as_str()), (15, "h900000"));
    assert!((contact.point.y + 74.18).abs() < 0.05, "{contact:?}");
}

#[test]
fn castle_helpers_are_not_floors() {
    check("m11_01_00_00", &CASTLE);
}

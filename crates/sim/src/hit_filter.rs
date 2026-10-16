//! What a map collision part's MSB hit filter (`hit_filter_id`) means for the player, traced in
//! the exe (`docs/HIT-FILTERS.md` has the addresses and the full tables).
//!
//! The game turns the hit filter into a Havok collision group (`FUN_1409b58b0`): filter 8 and
//! every unlisted value is ordinary map collision (group 38 for the high-resolution `h` binder,
//! 39 or 40 for the low one), and the helper filters get groups of their own. Whether two groups
//! collide is a 128 x 128 bit matrix built once (`FUN_140933390`). The player's capsule is group
//! 1 and the camera's ray is group 48, so the matrix says which parts the player stands on or
//! bumps into and which only the camera sees. The `h9xxxxx` helpers are mostly not for the
//! player: enemy-only walls, camera blockers, water surfaces, wall-hug and wall-jump volumes,
//! fall-prevention assists, and "map action event walls" (group 44) that nothing solid touches
//! but whose contact runs a reaction (`FUN_1409edf20`).
//!
//! The exported `collision.bin` holds every part's triangles; [`strip_helpers`] takes the ones
//! the player does not collide with out of it (in place, so the triangle ranges recorded in
//! `layout.json` stay valid) and returns the enemy and camera parts and the contact triggers
//! for [`crate::collision::CollisionWorld::add_helpers`]. Enemies and bosses use
//! [`crate::collision::EnemyGround`]: they also stand on water surfaces and stop at
//! enemy-only walls.

use std::collections::BTreeMap;

use sekiro_formats::hknp::CollisionMesh;
use serde_json::Value;

/// Havok collision group of a collision part (`FUN_1409b58b0`). `low` is the low-resolution
/// (`l` binder) resource and `middle` its middle variant; the exported collision is the
/// high-resolution binder, which ordinary parts give group 38.
pub fn collision_group(hit_filter: u8, low: bool, middle: bool) -> u8 {
    match hit_filter {
        9 => 41,
        11 | 19 => 42,
        12..=15 | 20 | 21 | 23 => 44,
        16 => 46,
        17 => 109,
        18 => 51,
        22 => 43,
        24 => 68,
        25 => 85,
        26 => 88,
        27 => 91,
        28 => 92,
        29 => 79,
        _ if low => 39 + u8::from(middle),
        _ => 38,
    }
}

/// Groups the player's capsule (group 1) collides with, among the groups map parts get, from
/// the matrix `FUN_140933390` builds: low and middle map (39, 40), the capsule-only wall (43)
/// and the swim hit (91). The high-resolution map (38) is not among them; the exe walks the
/// player on the low-resolution binder, which this simulation approximates with the high one.
const PLAYER_GROUPS: [u8; 4] = [39, 40, 43, 91];
/// Groups an enemy's or a boss's capsule (groups 2 and 5, and their map-only variants 7 and
/// 10) collides with: the player's plus the enemy-only wall (42) and the water surface (46).
const ENEMY_GROUPS: [u8; 6] = [39, 40, 42, 43, 46, 91];
/// Groups the camera's ray (group 48) collides with: low and middle map, the map camera hit
/// (41) and the water surface (46).
const CAMERA_GROUPS: [u8; 4] = [39, 40, 41, 46];

/// Who collides with a part: bits of [`mask`].
pub const PLAYER: u8 = 1;
pub const ENEMY: u8 = 2;
pub const CAMERA: u8 = 4;

/// Whether the player stands on and bumps into parts with this hit filter.
pub fn player_collides(hit_filter: u8) -> bool {
    // Ordinary parts stand in for their low-resolution twins (group 39).
    PLAYER_GROUPS.contains(&collision_group(hit_filter, true, false))
}

/// Whether enemies and bosses stand on and bump into parts with this hit filter.
pub fn enemy_collides(hit_filter: u8) -> bool {
    ENEMY_GROUPS.contains(&collision_group(hit_filter, true, false))
}

/// Whether the camera ray stops at parts with this hit filter.
pub fn camera_collides(hit_filter: u8) -> bool {
    CAMERA_GROUPS.contains(&collision_group(hit_filter, true, false))
}

/// [`PLAYER`], [`ENEMY`] and [`CAMERA`] bits for the parts with this hit filter.
pub fn mask(hit_filter: u8) -> u8 {
    [
        (player_collides(hit_filter), PLAYER),
        (enemy_collides(hit_filter), ENEMY),
        (camera_collides(hit_filter), CAMERA),
    ]
    .into_iter()
    .filter(|(on, _)| *on)
    .fold(0, |m, (_, bit)| m | bit)
}

/// Hit filters whose parts are map action event walls (group 44): untouchable, but a
/// character's contact with one runs that filter's reaction in `FUN_1409edf20`.
pub fn is_event_wall(hit_filter: u8) -> bool {
    collision_group(hit_filter, false, false) == 44
}

/// The event wall reaction this simulation applies: contact with a filter-15 wall starts the
/// fall return. The exe fades the character's model from 1 to 0 at -2 per second
/// (`chr+0x1a48`, `chr+0x1a4c`), and when it reaches 0 sets the physics module's `0x92b`,
/// which makes `FUN_140bcc420` set `0x92a` and hold the body still.
pub const FALL_RETURN_FILTER: u8 = 15;
/// Seconds of the fall return's model fade: 1.0 at -2.0 per second.
pub const FALL_RETURN_FADE: f32 = 0.5;

/// The helper triangles taken out of the player's collision.
#[derive(Debug, Default, Clone)]
pub struct HelperCollision {
    /// Parts only enemies or the camera collide with, by [`mask`] (hit filters 9, 11, 16 and 19).
    pub parts: BTreeMap<u8, CollisionMesh>,
    /// Map action event walls by hit filter, each with its part names.
    pub triggers: Vec<Trigger>,
    /// Triangles removed per hit filter, for logs and tests.
    pub removed: BTreeMap<u8, usize>,
}

#[derive(Debug, Default, Clone)]
pub struct Trigger {
    pub hit_filter: u8,
    pub part: String,
    pub mesh: CollisionMesh,
}

impl HelperCollision {
    /// Every helper mesh, for cropping them with the rest of an arena's collision.
    pub fn meshes_mut(&mut self) -> impl Iterator<Item = &mut CollisionMesh> {
        self.parts
            .values_mut()
            .chain(self.triggers.iter_mut().map(|t| &mut t.mesh))
    }

    /// Appends another block's helpers.
    pub fn extend(&mut self, other: HelperCollision) {
        for (mask, mesh) in other.parts {
            self.parts.entry(mask).or_default().extend(&mesh);
        }
        self.triggers.extend(other.triggers);
        for (filter, count) in other.removed {
            *self.removed.entry(filter).or_default() += count;
        }
    }
}

/// The hit filter of one of a layout's `collisions` rows.
pub fn row_filter(row: &Value) -> Option<u8> {
    row["collision"]["hit_filter_id"]
        .as_u64()
        .or_else(|| row["hit_filter_id"].as_u64())
        .and_then(|f| u8::try_from(f).ok())
}

/// Takes the triangles of every collision part the player does not collide with out of
/// `mesh`, a map's exported `collision.bin` read with its `layout`. Removed triangles collapse
/// to a point, which [`crate::collision::CollisionWorld`] skips, so every triangle index in the
/// layout still addresses the same triangle. Triangles outside every part stay, and ones an
/// earlier step already collapsed (hidden or disabled parts) are not taken.
pub fn strip_helpers(mesh: &mut CollisionMesh, layout: &Value) -> HelperCollision {
    let mut out = HelperCollision::default();
    let triangles = mesh.triangle_count();
    for row in layout["collisions"].as_array().into_iter().flatten() {
        let Some(filter) = row_filter(row) else {
            continue;
        };
        if player_collides(filter) {
            continue;
        }
        let (Some(first), Some(count)) = (
            row["triangles"][0].as_u64().map(|v| v as usize),
            row["triangles"][1].as_u64().map(|v| v as usize),
        ) else {
            continue;
        };
        let range = first.min(triangles)..(first + count).min(triangles);
        let mut part = CollisionMesh::default();
        for t in range {
            let tri = &mut mesh.indices[t * 3..t * 3 + 3];
            if tri[0] == tri[1] && tri[1] == tri[2] {
                continue;
            }
            let base = part.vertices.len() as u32;
            part.vertices
                .extend(tri.iter().map(|&i| mesh.vertices[i as usize]));
            part.indices.extend([base, base + 1, base + 2]);
            part.materials
                .push(mesh.materials.get(t).copied().unwrap_or(u32::MAX));
            let keep = tri[0];
            tri.fill(keep);
        }
        *out.removed.entry(filter).or_default() += part.triangle_count();
        if part.indices.is_empty() {
            continue;
        }
        let who = mask(filter);
        if who != 0 {
            out.parts.entry(who).or_default().extend(&part);
        } else if is_event_wall(filter) {
            out.triggers.push(Trigger {
                hit_filter: filter,
                part: row["name"].as_str().unwrap_or_default().to_string(),
                mesh: part,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_follow_the_exe_switch() {
        assert_eq!(collision_group(8, false, false), 38);
        assert_eq!(collision_group(8, true, false), 39);
        assert_eq!(collision_group(8, true, true), 40);
        assert_eq!(collision_group(15, true, false), 44);
        assert_eq!(collision_group(27, false, false), 91);
    }

    #[test]
    fn player_and_camera_sets() {
        let player: Vec<u8> = (0..32).filter(|&f| player_collides(f)).collect();
        // Every unlisted filter is ordinary map collision.
        assert!(player.contains(&8) && player.contains(&22) && player.contains(&27));
        for f in [
            9, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 23, 24, 25, 26, 28, 29,
        ] {
            assert!(!player_collides(f), "{f}");
        }
        let camera: Vec<u8> = (8..30).filter(|&f| camera_collides(f)).collect();
        assert_eq!(camera, [8, 9, 10, 16]);
        let enemy: Vec<u8> = (8..30).filter(|&f| enemy_collides(f)).collect();
        assert_eq!(enemy, [8, 10, 11, 16, 19, 22, 27]);
        assert_eq!(mask(8), PLAYER | ENEMY | CAMERA);
        assert_eq!(mask(16), ENEMY | CAMERA);
        assert_eq!(mask(15), 0);
    }

    #[test]
    fn strip_keeps_triangle_indices() {
        let mut mesh = CollisionMesh {
            vertices: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, -5.0, 0.0],
                [1.0, -5.0, 0.0],
                [0.0, -5.0, 1.0],
            ],
            indices: vec![0, 1, 2, 3, 4, 5],
            materials: vec![1, 2],
        };
        let layout = serde_json::json!({"collisions": [
            {"name": "h000100", "hit_filter_id": 8, "triangles": [0, 1]},
            {"name": "h900000", "hit_filter_id": 15, "triangles": [1, 1]},
        ]});
        let helpers = strip_helpers(&mut mesh, &layout);
        assert_eq!(mesh.triangle_count(), 2);
        assert_eq!(&mesh.indices[..3], &[0, 1, 2]);
        assert_eq!(&mesh.indices[3..], &[3, 3, 3]);
        assert_eq!(helpers.triggers.len(), 1);
        assert_eq!(helpers.triggers[0].part, "h900000");
        assert_eq!(helpers.triggers[0].mesh.vertices[0], [0.0, -5.0, 0.0]);
        assert_eq!(helpers.removed.get(&15), Some(&1));
    }
}

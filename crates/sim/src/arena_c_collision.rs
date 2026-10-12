//! Independently switched event collision, sharing the arena's query world.
use super::*;

impl CollisionWorld {
    pub fn append_event_part(&mut self, entity: i32, mesh: &CollisionMesh, on: bool) {
        let part = Self::from_mesh(mesh);
        let vertex_offset = self.vertices.len() as u32;
        let first = self.triangles.len() as u32;
        let last = first + part.triangles.len() as u32;
        self.vertices.extend(part.vertices);
        self.triangles.extend(
            part.triangles
                .into_iter()
                .map(|tri| tri.map(|i| i + vertex_offset)),
        );
        self.normals.extend(part.normals);
        self.materials.extend(part.materials);
        for (cell, triangles) in part.cells {
            self.cells
                .entry(cell)
                .or_default()
                .extend(triangles.into_iter().map(|t| t + first));
        }
        self.event_parts
            .push((entity, first..last, AtomicBool::new(on)));
    }

    pub fn set_event_part(&self, entity: i32, on: bool) {
        for (id, _, state) in &self.event_parts {
            if *id == entity {
                state.store(on, Ordering::Relaxed);
            }
        }
    }
}

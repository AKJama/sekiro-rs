//! Independently switched event collision, sharing the arena's query world.
use super::*;

impl CollisionWorld {
    pub fn append_event_part(&mut self, entity: i32, mesh: &CollisionMesh, on: bool) {
        let range = self.append(mesh);
        self.event_parts.push((entity, range, AtomicBool::new(on)));
    }

    pub fn set_event_part(&self, entity: i32, on: bool) {
        for (id, _, state) in &self.event_parts {
            if *id == entity {
                state.store(on, Ordering::Relaxed);
            }
        }
    }
}

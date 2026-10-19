//! Immutable extracted assets, keyed by type and source path for the process lifetime.
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

type Asset = Arc<dyn Any + Send + Sync>;
type Cell = Arc<OnceLock<Option<Asset>>>;
type Assets = HashMap<(TypeId, PathBuf), Cell>;

pub(crate) fn load<T: Any + Send + Sync>(
    path: PathBuf,
    read: impl FnOnce() -> Option<T>,
) -> Option<Arc<T>> {
    static ASSETS: OnceLock<Mutex<Assets>> = OnceLock::new();
    let path = path.canonicalize().unwrap_or(path);
    let cell = ASSETS
        .get_or_init(Default::default)
        .lock()
        .unwrap()
        .entry((TypeId::of::<T>(), path))
        .or_default()
        .clone();
    cell.get_or_init(|| read().map(|v| Arc::new(v) as Asset))
        .as_ref()?
        .clone()
        .downcast()
        .ok()
}

/// A combat kernel borrows the encounter's Wolf while stepping; otherwise its slot is empty.
pub struct BehaviorSlot(Option<crate::player::PlayerBehavior>);

impl BehaviorSlot {
    pub fn empty() -> Self {
        Self(None)
    }
    pub fn is_present(&self) -> bool {
        self.0.is_some()
    }
}
impl From<crate::player::PlayerBehavior> for BehaviorSlot {
    fn from(value: crate::player::PlayerBehavior) -> Self {
        Self(Some(value))
    }
}
impl std::ops::Deref for BehaviorSlot {
    type Target = crate::player::PlayerBehavior;
    fn deref(&self) -> &Self::Target {
        self.0
            .as_ref()
            .expect("combat kernel does not currently own Wolf")
    }
}
impl std::ops::DerefMut for BehaviorSlot {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
            .as_mut()
            .expect("combat kernel does not currently own Wolf")
    }
}

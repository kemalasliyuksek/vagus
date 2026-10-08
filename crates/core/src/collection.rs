use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde::Serialize;

/// The latest contents of an entity collection.
///
/// A view is serialized only when a client asks for it, so publishing costs nothing
/// while nobody is reading.
pub trait CollectionView: Send + Sync {
    /// Serializes the contents as JSON.
    fn to_json(&self) -> serde_json::Result<serde_json::Value>;
}

impl<T: Serialize + Send + Sync> CollectionView for T {
    fn to_json(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }
}

/// The latest published view of every collection.
///
/// The sampling thread writes it and client threads read it. Both hold the lock only
/// long enough to replace or clone an `Arc`.
#[derive(Default)]
pub struct CollectionStore {
    views: Mutex<HashMap<&'static str, Arc<dyn CollectionView>>>,
}

impl CollectionStore {
    /// Replaces the view of collection `id`.
    pub fn publish(&self, id: &'static str, view: Arc<dyn CollectionView>) {
        let replaced = self.lock().insert(id, view);
        // Dropped after the lock is released, in case it was the last reference.
        drop(replaced);
    }

    /// Returns the current view of collection `id`, if one has been published.
    #[must_use]
    pub fn latest(&self, id: &str) -> Option<Arc<dyn CollectionView>> {
        self.lock().get(id).cloned()
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<&'static str, Arc<dyn CollectionView>>> {
        // The map only holds `Arc`s that are replaced whole, so a panic on another
        // thread cannot leave it inconsistent and a poisoned lock is still safe to use.
        self.views.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Two buffers that let a producer build the next immutable snapshot while readers on
/// other threads still hold the previous one, without allocating in steady state.
///
/// The producer fills [`next_mut`](Self::next_mut) and publishes the `Arc` that
/// [`swap`](Self::swap) returns. The buffer that was current becomes the spare and is
/// reused once no reader holds it.
pub struct DoubleBuffer<T> {
    current: Arc<T>,
    spare: Arc<T>,
}

impl<T: Default> DoubleBuffer<T> {
    /// Creates two empty buffers.
    #[must_use]
    pub fn new() -> Self {
        Self {
            current: Arc::default(),
            spare: Arc::default(),
        }
    }

    /// Returns the buffer for the next snapshot. It still holds an older snapshot's
    /// contents, which the caller overwrites. If a reader still holds that buffer, the
    /// reader keeps it and a new, empty buffer is returned instead.
    pub fn next_mut(&mut self) -> &mut T {
        if Arc::get_mut(&mut self.spare).is_none() {
            self.spare = Arc::default();
        }
        Arc::get_mut(&mut self.spare).expect("a newly created Arc has no other owners")
    }

    /// Makes the buffer filled through [`next_mut`](Self::next_mut) current and
    /// returns it for publishing.
    pub fn swap(&mut self) -> Arc<T> {
        std::mem::swap(&mut self.current, &mut self.spare);
        Arc::clone(&self.current)
    }
}

impl<T: Default> Default for DoubleBuffer<T> {
    fn default() -> Self {
        Self::new()
    }
}

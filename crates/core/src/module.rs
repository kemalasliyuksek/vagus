use std::error::Error;
use std::sync::Arc;
use std::time::Instant;

use schemars::{JsonSchema, Schema};
use serde::Serialize;

use crate::collection::{CollectionStore, CollectionView};

/// A unit of functionality, such as performance monitoring, compiled into the daemon
/// (ADR 0017) and described by a manifest (ADR 0007).
///
/// Only the daemon's sampling thread touches a module. Clients see its data through the
/// collections it publishes, so a client request never waits for a sample and a sample
/// never waits for a client.
pub trait Module: Send {
    /// Describes what the module provides.
    fn manifest(&self) -> ModuleManifest;

    /// Called on every sampling tick. The module decides which of its sources are due.
    fn sample(&mut self, ctx: &SampleContext<'_>) -> Result<(), ModuleError>;
}

/// Static description of a module.
#[derive(Debug, Clone, Serialize)]
pub struct ModuleManifest {
    /// Stable identifier, such as `perf`.
    pub id: &'static str,
    /// Entity collections the module publishes.
    pub collections: Vec<CollectionDescriptor>,
}

/// An entity collection, such as the process list, with the schema of its items.
#[derive(Debug, Clone, Serialize)]
pub struct CollectionDescriptor {
    /// Stable identifier, such as `perf.processes`. Clients ask for data by this id.
    pub id: &'static str,
    /// JSON Schema of one item, generated from its Rust type.
    pub item_schema: Schema,
}

impl CollectionDescriptor {
    /// Describes collection `id`, whose items are of type `T`.
    #[must_use]
    pub fn new<T: JsonSchema>(id: &'static str) -> Self {
        Self {
            id,
            item_schema: schemars::schema_for!(T),
        }
    }
}

/// What a module receives on each sampling tick.
pub struct SampleContext<'a> {
    tick: u64,
    now: Instant,
    store: &'a CollectionStore,
}

impl<'a> SampleContext<'a> {
    /// Creates the context for tick number `tick`, which started at `now`.
    #[must_use]
    pub fn new(tick: u64, now: Instant, store: &'a CollectionStore) -> Self {
        Self { tick, now, store }
    }

    /// Number of this tick, counting from 0. A source that runs slower than the tick
    /// rate samples on every n-th tick.
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// When this tick started. Rates come from the difference between two ticks.
    #[must_use]
    pub fn now(&self) -> Instant {
        self.now
    }

    /// Makes `view` the current contents of collection `id`.
    pub fn publish(&self, id: &'static str, view: Arc<dyn CollectionView>) {
        self.store.publish(id, view);
    }
}

/// A failed sample. The module's previously published data stays in place.
#[derive(Debug, thiserror::Error)]
pub enum ModuleError {
    /// A data source failed.
    #[error("data source failed")]
    Source(#[source] Box<dyn Error + Send + Sync>),
}

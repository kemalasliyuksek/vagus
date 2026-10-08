//! The performance module: what is using the machine's resources right now.
//!
//! It starts with the process list; system metrics follow in the next roadmap slice.

#![forbid(unsafe_code)]

mod processes;

use vagus_core::{
    CollectionDescriptor, Module, ModuleError, ModuleManifest, ProcessSource, SampleContext,
};

pub use processes::ProcessItem;
use processes::ProcessTracker;

/// Module id.
pub const MODULE_ID: &str = "perf";

/// Id of the process collection.
pub const PROCESSES: &str = "perf.processes";

/// The process snapshot costs about 6 ms of CPU on the reference machine, so it runs on
/// every second 1 Hz tick (ADR 0019).
const PROCESS_SNAPSHOT_EVERY_TICKS: u64 = 2;

/// The performance module.
pub struct PerfModule<S> {
    processes: ProcessTracker<S>,
}

impl<S: ProcessSource> PerfModule<S> {
    /// Creates the module on top of a platform's process source.
    #[must_use]
    pub fn new(processes: S) -> Self {
        Self {
            processes: ProcessTracker::new(processes),
        }
    }
}

impl<S: ProcessSource> Module for PerfModule<S> {
    fn manifest(&self) -> ModuleManifest {
        ModuleManifest {
            id: MODULE_ID,
            collections: vec![CollectionDescriptor::new::<ProcessItem>(PROCESSES)],
        }
    }

    fn sample(&mut self, ctx: &SampleContext<'_>) -> Result<(), ModuleError> {
        if ctx.tick() % PROCESS_SNAPSHOT_EVERY_TICKS == 0 {
            let items = self
                .processes
                .sample(ctx.now())
                .map_err(|error| ModuleError::Source(Box::new(error)))?;
            ctx.publish(PROCESSES, items);
        }
        Ok(())
    }
}

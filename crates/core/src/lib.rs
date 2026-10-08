//! Core building blocks shared by the Vagus daemon and its modules.
//!
//! See `docs/design/vagus-architecture.md` for how the pieces fit together.

#![forbid(unsafe_code)]

mod collection;
mod module;
mod process;
mod ring_buffer;

pub use collection::{CollectionStore, CollectionView, DoubleBuffer};
pub use module::{CollectionDescriptor, Module, ModuleError, ModuleManifest, SampleContext};
pub use process::{ProcessCounters, ProcessEntry, ProcessName, ProcessSource};
pub use ring_buffer::RingBuffer;

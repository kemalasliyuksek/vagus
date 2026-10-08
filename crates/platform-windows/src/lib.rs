//! Safe wrappers over the Windows APIs that Vagus collects data from (ADR 0019).
//!
//! This crate family is the only place where `unsafe` is allowed. Kernel buffers are
//! parsed in safe code, so the parsers can be tested with synthetic input.

mod process_snapshot;

pub use process_snapshot::{ImageName, ProcessRecord, ProcessSnapshot, Processes, SnapshotError};

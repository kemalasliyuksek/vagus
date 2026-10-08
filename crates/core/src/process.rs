//! A platform-neutral view of per-process counters. A platform crate implements
//! [`ProcessSource`]; modules depend only on the trait, so they can be tested with
//! fakes.

use std::error::Error;

/// Cumulative counters of one process, as a platform reports them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProcessCounters {
    /// Process id. Ids are reused, so a process is identified by the id together with
    /// [`create_time`](Self::create_time).
    pub pid: u32,
    /// Creation time, in the platform's units. Only compared for equality.
    pub create_time: u64,
    /// CPU cycles charged to the process.
    pub cycle_time: u64,
    /// Working set in bytes.
    pub working_set: u64,
    /// Private part of the working set in bytes.
    pub private_working_set: u64,
    /// Private committed memory in bytes.
    pub private_bytes: u64,
    /// Bytes read by read operations, from files and devices alike.
    pub read_bytes: u64,
    /// Bytes written by write operations, to files and devices alike.
    pub write_bytes: u64,
    /// Bytes transferred by operations that are neither reads nor writes.
    pub other_bytes: u64,
}

/// A process name in the platform's own encoding.
///
/// Decoding allocates, so it happens only when a process is seen for the first time.
pub trait ProcessName {
    /// Decodes the name, replacing anything that cannot be decoded.
    fn decode(&self) -> String;
}

/// One process in a snapshot.
pub struct ProcessEntry<'a> {
    /// The process's counters.
    pub counters: ProcessCounters,
    /// The process's image name, such as `explorer.exe`.
    pub name: &'a dyn ProcessName,
}

/// A source of process snapshots, implemented by a platform crate.
pub trait ProcessSource: Send {
    /// The platform's error type.
    type Error: Error + Send + Sync + 'static;

    /// Takes a snapshot and passes every process in it to `visit`.
    ///
    /// On error, `visit` may already have seen part of the list, and the caller must
    /// discard it.
    fn snapshot(&mut self, visit: &mut dyn FnMut(ProcessEntry<'_>)) -> Result<(), Self::Error>;
}

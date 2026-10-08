//! A steady-state sample must not allocate (AGENTS.md, hot paths). `allocation-counter`
//! replaces the global allocator of this test binary and counts only the allocations
//! made by the measuring thread.

use std::time::{Duration, Instant};

use vagus_core::{
    CollectionStore, Module, ProcessCounters, ProcessEntry, ProcessName, ProcessSource,
    SampleContext,
};
use vagus_perf::PerfModule;

struct Name(&'static str);

impl ProcessName for Name {
    fn decode(&self) -> String {
        self.0.to_owned()
    }
}

/// Lists the same processes on every call, with growing counters.
#[derive(Default)]
struct SteadySource {
    calls: u64,
}

impl ProcessSource for SteadySource {
    type Error = std::io::Error;

    fn snapshot(&mut self, visit: &mut dyn FnMut(ProcessEntry<'_>)) -> std::io::Result<()> {
        self.calls += 1;
        for (pid, name) in [(0, ""), (4, "System"), (1234, "explorer.exe")] {
            visit(ProcessEntry {
                counters: ProcessCounters {
                    pid,
                    create_time: 1,
                    cycle_time: self.calls * 1000 * (u64::from(pid) + 1),
                    read_bytes: self.calls * 4096,
                    ..ProcessCounters::default()
                },
                name: &Name(name),
            });
        }
        Ok(())
    }
}

#[test]
fn a_steady_state_sample_does_not_allocate() {
    let mut module = PerfModule::new(SteadySource::default());
    let store = CollectionStore::default();
    let start = Instant::now();
    let ctx = |tick: u64| SampleContext::new(tick, start + Duration::from_secs(tick), &store);

    // The first sample sizes the buffers and decodes the names; counting it also shows
    // that the counting allocator is active in this binary.
    let first = allocation_counter::measure(|| module.sample(&ctx(0)).unwrap());
    assert!(first.count_total > 0);
    // Both halves of the double buffer are sized by the third snapshot.
    module.sample(&ctx(2)).unwrap();
    module.sample(&ctx(4)).unwrap();

    let steady = allocation_counter::measure(|| module.sample(&ctx(6)).unwrap());

    assert_eq!(steady.count_total, 0, "{steady:?}");
}

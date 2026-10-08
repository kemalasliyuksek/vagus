use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use serde_json::Value;
use vagus_core::{
    CollectionStore, Module, ProcessCounters, ProcessEntry, ProcessName, ProcessSource,
    SampleContext,
};
use vagus_perf::{PROCESSES, PerfModule};
use vagus_platform_windows::ProcessSnapshot;

/// A source that always lists one process and counts how often it is asked.
struct CountingSource {
    calls: Arc<AtomicUsize>,
}

struct Name(&'static str);

impl ProcessName for Name {
    fn decode(&self) -> String {
        self.0.to_owned()
    }
}

impl ProcessSource for CountingSource {
    type Error = std::io::Error;

    fn snapshot(&mut self, visit: &mut dyn FnMut(ProcessEntry<'_>)) -> std::io::Result<()> {
        let calls = self.calls.fetch_add(1, Ordering::Relaxed) as u64;
        visit(ProcessEntry {
            counters: ProcessCounters {
                pid: 42,
                create_time: 1,
                cycle_time: calls * 1000,
                ..ProcessCounters::default()
            },
            name: &Name("answer.exe"),
        });
        Ok(())
    }
}

fn processes(store: &CollectionStore) -> Vec<Value> {
    let json = store
        .latest(PROCESSES)
        .expect("the process list is published")
        .to_json()
        .expect("the process list serializes");
    serde_json::from_value(json).expect("the process list is a JSON array")
}

#[test]
fn manifest_describes_the_process_collection() {
    let module = PerfModule::new(CountingSource {
        calls: Arc::default(),
    });
    let manifest = module.manifest();

    assert_eq!(manifest.id, "perf");
    let collection = &manifest.collections[0];
    assert_eq!(collection.id, PROCESSES);
    let schema = serde_json::to_value(&collection.item_schema).unwrap();
    assert!(schema["properties"]["cpu_percent"].is_object());
}

#[test]
fn snapshots_run_every_second_tick_and_are_published() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut module = PerfModule::new(CountingSource {
        calls: Arc::clone(&calls),
    });
    let store = CollectionStore::default();
    let start = Instant::now();

    for tick in 0..4 {
        let now = start + Duration::from_secs(tick);
        module
            .sample(&SampleContext::new(tick, now, &store))
            .unwrap();
    }

    assert_eq!(calls.load(Ordering::Relaxed), 2);
    let items = processes(&store);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["pid"], 42);
    assert_eq!(items[0]["name"], "answer.exe");
    // The only process holds all the cycles of the interval.
    assert_eq!(items[0]["cpu_percent"], 100.0);
}

/// Runs the module on the live process list. Needs no special hardware or rights.
#[test]
fn live_snapshot_shows_this_process_using_cpu() {
    let mut module = PerfModule::new(ProcessSnapshot::new());
    let store = CollectionStore::default();
    let start = Instant::now();

    module
        .sample(&SampleContext::new(0, start, &store))
        .unwrap();
    let busy_until = Instant::now() + Duration::from_millis(200);
    let mut counter = 0u64;
    while Instant::now() < busy_until {
        counter = std::hint::black_box(counter.wrapping_add(1));
    }
    module
        .sample(&SampleContext::new(2, Instant::now(), &store))
        .unwrap();

    let items = processes(&store);
    let own = items
        .iter()
        .find(|p| p["pid"] == std::process::id())
        .expect("this process is listed");
    let cpu = own["cpu_percent"]
        .as_f64()
        .expect("a CPU share on the second sample");
    assert!(cpu > 0.0 && cpu <= 100.0, "cpu_percent {cpu}");
    let total: f64 = items.iter().filter_map(|p| p["cpu_percent"].as_f64()).sum();
    assert!((total - 100.0).abs() < 1e-6, "shares add up to {total}");
}

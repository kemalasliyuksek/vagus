use std::sync::Arc;
use std::time::Instant;

use schemars::JsonSchema;
use serde::Serialize;
use vagus_core::{DoubleBuffer, ProcessCounters, ProcessSource};

/// One process in the `perf.processes` collection.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ProcessItem {
    /// Process id.
    pub pid: u32,
    /// Image name, such as `explorer.exe`. Empty for the idle process.
    pub name: Arc<str>,
    /// Share of all CPU time over the last interval, from 0 to 100, measured in CPU
    /// cycles. Absent the first time a process is seen.
    pub cpu_percent: Option<f64>,
    /// Private committed memory in bytes.
    pub private_bytes: u64,
    /// Working set in bytes.
    pub working_set: u64,
    /// Private part of the working set in bytes.
    pub private_working_set: u64,
    /// Bytes read per second by read operations, from files and devices alike. Absent
    /// the first time a process is seen.
    pub read_bytes_per_sec: Option<f64>,
    /// Bytes written per second by write operations, to files and devices alike. Absent
    /// the first time a process is seen.
    pub write_bytes_per_sec: Option<f64>,
    /// Bytes per second transferred by other I/O operations. Absent the first time a
    /// process is seen.
    pub other_bytes_per_sec: Option<f64>,
}

/// A process from one snapshot, with its decoded name.
struct Tracked {
    counters: ProcessCounters,
    name: Arc<str>,
}

/// Turns cumulative process counters into per-interval values.
///
/// Every buffer is reused between samples, and a name is decoded only the first time
/// its process is seen, so steady-state samples do not allocate.
pub(crate) struct ProcessTracker<S> {
    source: S,
    /// The last successful snapshot, sorted by pid.
    previous: Vec<Tracked>,
    previous_at: Option<Instant>,
    /// Scratch space for the snapshot being taken.
    current: Vec<Tracked>,
    items: DoubleBuffer<Vec<ProcessItem>>,
}

impl<S: ProcessSource> ProcessTracker<S> {
    pub(crate) fn new(source: S) -> Self {
        Self {
            source,
            previous: Vec::new(),
            previous_at: None,
            current: Vec::new(),
            items: DoubleBuffer::new(),
        }
    }

    /// Takes a snapshot at `now` and returns the process list to publish.
    ///
    /// A failed snapshot leaves the previous one in place, so the next successful one
    /// computes its rates over the longer interval.
    pub(crate) fn sample(&mut self, now: Instant) -> Result<Arc<Vec<ProcessItem>>, S::Error> {
        let previous = &self.previous;
        let current = &mut self.current;
        current.clear();
        self.source.snapshot(&mut |entry| {
            let name = match find(previous, &entry.counters) {
                Some(known) => Arc::clone(&known.name),
                None => Arc::from(entry.name.decode()),
            };
            current.push(Tracked {
                counters: entry.counters,
                name,
            });
        })?;
        current.sort_unstable_by_key(|p| p.counters.pid);

        // CPU shares are relative to the cycles of every process seen in both snapshots,
        // the idle process included. That total is the machine's whole CPU capacity over
        // the interval, so no clock frequency is needed.
        let total_cycles: u64 = current
            .iter()
            .filter_map(|p| find(previous, &p.counters).map(|before| cycles_since(before, p)))
            .sum();
        let elapsed_secs = self
            .previous_at
            .map(|at| now.saturating_duration_since(at).as_secs_f64())
            .filter(|secs| *secs > 0.0);

        let items = self.items.next_mut();
        items.clear();
        for process in current.iter() {
            let before = find(previous, &process.counters);
            let rate = |count: fn(&ProcessCounters) -> u64| {
                let before = before?;
                let secs = elapsed_secs?;
                let delta = count(&process.counters).saturating_sub(count(&before.counters));
                Some(delta as f64 / secs)
            };
            let counters = &process.counters;
            items.push(ProcessItem {
                pid: counters.pid,
                name: Arc::clone(&process.name),
                cpu_percent: before.filter(|_| total_cycles > 0).map(|before| {
                    cycles_since(before, process) as f64 * 100.0 / total_cycles as f64
                }),
                private_bytes: counters.private_bytes,
                working_set: counters.working_set,
                private_working_set: counters.private_working_set,
                read_bytes_per_sec: rate(|c| c.read_bytes),
                write_bytes_per_sec: rate(|c| c.write_bytes),
                other_bytes_per_sec: rate(|c| c.other_bytes),
            });
        }

        std::mem::swap(&mut self.previous, &mut self.current);
        self.previous_at = Some(now);
        Ok(self.items.swap())
    }
}

/// Finds the same process (same pid and creation time) in a list sorted by pid.
fn find<'a>(sorted: &'a [Tracked], counters: &ProcessCounters) -> Option<&'a Tracked> {
    let index = sorted
        .binary_search_by_key(&counters.pid, |p| p.counters.pid)
        .ok()?;
    let found = &sorted[index];
    (found.counters.create_time == counters.create_time).then_some(found)
}

fn cycles_since(before: &Tracked, now: &Tracked) -> u64 {
    now.counters
        .cycle_time
        .saturating_sub(before.counters.cycle_time)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::collections::VecDeque;
    use std::io;
    use std::time::Duration;

    use vagus_core::{ProcessEntry, ProcessName};

    use super::*;

    /// A scripted process source. Each snapshot is a list of processes, or an error
    /// returned after visiting the given processes.
    #[derive(Default)]
    struct FakeSource {
        snapshots: VecDeque<(Vec<(ProcessCounters, &'static str)>, bool)>,
        decodes: Cell<usize>,
    }

    impl FakeSource {
        fn push(&mut self, processes: Vec<(ProcessCounters, &'static str)>) {
            self.snapshots.push_back((processes, false));
        }

        fn push_failure_after(&mut self, processes: Vec<(ProcessCounters, &'static str)>) {
            self.snapshots.push_back((processes, true));
        }
    }

    struct FakeName<'a> {
        name: &'static str,
        decodes: &'a Cell<usize>,
    }

    impl ProcessName for FakeName<'_> {
        fn decode(&self) -> String {
            self.decodes.set(self.decodes.get() + 1);
            self.name.to_owned()
        }
    }

    impl ProcessSource for FakeSource {
        type Error = io::Error;

        fn snapshot(&mut self, visit: &mut dyn FnMut(ProcessEntry<'_>)) -> io::Result<()> {
            let (processes, fail) = self.snapshots.pop_front().expect("a scripted snapshot");
            for (counters, name) in processes {
                let name = FakeName {
                    name,
                    decodes: &self.decodes,
                };
                visit(ProcessEntry {
                    counters,
                    name: &name,
                });
            }
            if fail {
                return Err(io::Error::other("scripted failure"));
            }
            Ok(())
        }
    }

    fn process(pid: u32, create_time: u64, cycle_time: u64) -> ProcessCounters {
        ProcessCounters {
            pid,
            create_time,
            cycle_time,
            ..ProcessCounters::default()
        }
    }

    fn cpu(items: &[ProcessItem], pid: u32) -> Option<f64> {
        items.iter().find(|p| p.pid == pid).unwrap().cpu_percent
    }

    fn assert_near(actual: Option<f64>, expected: f64) {
        let actual = actual.expect("a value");
        assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
    }

    #[test]
    fn first_snapshot_has_no_rates() {
        let mut source = FakeSource::default();
        let mut explorer = process(1234, 1, 500);
        explorer.private_bytes = 4096;
        source.push(vec![(process(0, 0, 100), ""), (explorer, "explorer.exe")]);
        let mut tracker = ProcessTracker::new(source);

        let items = tracker.sample(Instant::now()).unwrap();

        assert_eq!(items.len(), 2);
        let explorer = items.iter().find(|p| p.pid == 1234).unwrap();
        assert_eq!(&*explorer.name, "explorer.exe");
        assert_eq!(explorer.private_bytes, 4096);
        assert_eq!(explorer.cpu_percent, None);
        assert_eq!(explorer.read_bytes_per_sec, None);
    }

    #[test]
    fn cpu_is_each_process_share_of_all_cycles() {
        let mut source = FakeSource::default();
        source.push(vec![
            (process(0, 0, 1000), ""),
            (process(10, 1, 1000), "a.exe"),
            (process(20, 1, 1000), "b.exe"),
        ]);
        // The kernel does not list processes in pid order.
        source.push(vec![
            (process(20, 1, 1100), "b.exe"),
            (process(0, 0, 1600), ""),
            (process(10, 1, 1300), "a.exe"),
        ]);
        let mut tracker = ProcessTracker::new(source);
        let start = Instant::now();

        tracker.sample(start).unwrap();
        let items = tracker.sample(start + Duration::from_secs(2)).unwrap();

        assert_near(cpu(&items, 0), 60.0);
        assert_near(cpu(&items, 10), 30.0);
        assert_near(cpu(&items, 20), 10.0);
    }

    #[test]
    fn io_rates_are_per_second() {
        let mut source = FakeSource::default();
        let mut before = process(10, 1, 0);
        before.read_bytes = 1000;
        before.write_bytes = 2000;
        before.other_bytes = 3000;
        let mut after = before;
        after.read_bytes += 8192;
        after.write_bytes += 4096;
        after.other_bytes += 512;
        source.push(vec![(before, "a.exe")]);
        source.push(vec![(after, "a.exe")]);
        let mut tracker = ProcessTracker::new(source);
        let start = Instant::now();

        tracker.sample(start).unwrap();
        let items = tracker.sample(start + Duration::from_secs(2)).unwrap();

        assert_near(items[0].read_bytes_per_sec, 4096.0);
        assert_near(items[0].write_bytes_per_sec, 2048.0);
        assert_near(items[0].other_bytes_per_sec, 256.0);
    }

    #[test]
    fn a_reused_pid_is_a_new_process() {
        let mut source = FakeSource::default();
        source.push(vec![
            (process(0, 0, 1000), ""),
            (process(10, 1, 5000), "old.exe"),
        ]);
        source.push(vec![
            (process(0, 0, 1500), ""),
            (process(10, 2, 100), "new.exe"),
        ]);
        let mut tracker = ProcessTracker::new(source);
        let start = Instant::now();

        tracker.sample(start).unwrap();
        let items = tracker.sample(start + Duration::from_secs(2)).unwrap();

        let reused = items.iter().find(|p| p.pid == 10).unwrap();
        assert_eq!(&*reused.name, "new.exe");
        assert_eq!(reused.cpu_percent, None);
        // The new process's cycles are not part of the total, so idle holds all of it.
        assert_near(cpu(&items, 0), 100.0);
    }

    #[test]
    fn names_are_decoded_once_per_process() {
        let mut source = FakeSource::default();
        for _ in 0..3 {
            source.push(vec![
                (process(10, 1, 0), "a.exe"),
                (process(20, 1, 0), "b.exe"),
            ]);
        }
        let mut tracker = ProcessTracker::new(source);
        let start = Instant::now();

        for second in 0..3 {
            tracker.sample(start + Duration::from_secs(second)).unwrap();
        }

        assert_eq!(tracker.source.decodes.get(), 2);
    }

    #[test]
    fn a_failed_snapshot_keeps_the_previous_one() {
        let mut source = FakeSource::default();
        let mut before = process(10, 1, 0);
        before.read_bytes = 0;
        let mut after = before;
        after.read_bytes = 4000;
        source.push(vec![(before, "a.exe")]);
        source.push_failure_after(vec![(after, "a.exe")]);
        source.push(vec![(after, "a.exe")]);
        let mut tracker = ProcessTracker::new(source);
        let start = Instant::now();

        tracker.sample(start).unwrap();
        assert!(tracker.sample(start + Duration::from_secs(2)).is_err());
        let items = tracker.sample(start + Duration::from_secs(4)).unwrap();

        // The rate spans the four seconds since the last successful snapshot.
        assert_eq!(items.len(), 1);
        assert_near(items[0].read_bytes_per_sec, 1000.0);
    }

    #[test]
    fn an_exited_process_is_dropped() {
        let mut source = FakeSource::default();
        source.push(vec![
            (process(10, 1, 0), "a.exe"),
            (process(20, 1, 0), "b.exe"),
        ]);
        source.push(vec![(process(20, 1, 0), "b.exe")]);
        let mut tracker = ProcessTracker::new(source);
        let start = Instant::now();

        tracker.sample(start).unwrap();
        let items = tracker.sample(start + Duration::from_secs(2)).unwrap();

        let pids: Vec<u32> = items.iter().map(|p| p.pid).collect();
        assert_eq!(pids, [20]);
    }
}

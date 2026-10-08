use std::error::Error;
use std::io;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use tracing::{info, warn};
use vagus_core::{CollectionStore, Module, SampleContext};
use vagus_platform_windows::current_process::{
    process_cycles, process_private_bytes, thread_cycles,
};

/// How often the sampler logs what the daemon costs.
const COST_REPORT_INTERVAL: Duration = Duration::from_secs(60);

/// Calls every module on a dedicated thread at a fixed rate, so that I/O and client
/// load do not jitter the samples (ADR 0019).
pub struct Sampler {
    slots: Vec<Slot>,
    store: Arc<CollectionStore>,
    period: Duration,
    cycles_per_second: f64,
}

struct Slot {
    id: &'static str,
    module: Box<dyn Module>,
    /// Cycles spent in this module's samples since the last cost report.
    cycles: u64,
}

impl Sampler {
    /// Creates a sampler that calls every module once per `period` and publishes into
    /// `store`.
    ///
    /// `cycles_per_second` turns cycle counts into CPU time for the cost report (see
    /// `current_process::estimate_cycle_rate`).
    #[must_use]
    pub fn new(
        modules: Vec<Box<dyn Module>>,
        store: Arc<CollectionStore>,
        period: Duration,
        cycles_per_second: f64,
    ) -> Self {
        let slots = modules
            .into_iter()
            .map(|module| Slot {
                id: module.manifest().id,
                module,
                cycles: 0,
            })
            .collect();
        Self {
            slots,
            store,
            period,
            cycles_per_second,
        }
    }

    /// Starts sampling on a thread of its own. The first tick runs immediately.
    pub fn spawn(self) -> io::Result<SamplerHandle> {
        let (stop, stop_requested) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("vagus-sampler".into())
            .spawn(move || self.run(&stop_requested))?;
        Ok(SamplerHandle { stop, thread })
    }

    fn run(mut self, stop_requested: &Receiver<()>) {
        let mut tick = 0;
        let mut due = Instant::now();
        let mut window = CostWindow::start(due);
        while wait_until(due, stop_requested) {
            let now = Instant::now();
            self.sample_all(tick, now);
            if now.duration_since(window.started) >= COST_REPORT_INTERVAL {
                self.report_cost(&window, now);
                window = CostWindow::start(now);
            }

            tick += 1;
            due += self.period;
            // Ticks missed while a slow sample ran are skipped, not run back to back.
            // Tick numbers keep counting periods, so modules keep their cadence.
            let after = Instant::now();
            let mut skipped = 0;
            while due < after {
                due += self.period;
                tick += 1;
                skipped += 1;
            }
            if skipped > 0 {
                warn!(skipped, "sampling overran its period");
            }
        }
    }

    fn sample_all(&mut self, tick: u64, now: Instant) {
        let ctx = SampleContext::new(tick, now, &self.store);
        for slot in &mut self.slots {
            let before = thread_cycles();
            let result = slot.module.sample(&ctx);
            if let (Ok(before), Ok(after)) = (before, thread_cycles()) {
                slot.cycles += after.saturating_sub(before);
            }
            if let Err(error) = result {
                warn!(
                    module = slot.id,
                    error = &error as &(dyn Error + 'static),
                    "sampling failed"
                );
            }
        }
    }

    fn report_cost(&mut self, window: &CostWindow, now: Instant) {
        let elapsed = now.duration_since(window.started);
        let rate = self.cycles_per_second;
        let (Some(start_cycles), Ok(end_cycles), Ok(private_bytes)) = (
            window.process_cycles,
            process_cycles(),
            process_private_bytes(),
        ) else {
            warn!("could not read the daemon's own counters; skipping the cost report");
            return;
        };

        let sampling: u64 = self.slots.iter().map(|slot| slot.cycles).sum();
        info!(
            daemon_cpu_ms_per_s = cpu_ms_per_second(end_cycles - start_cycles, rate, elapsed),
            sampling_cpu_ms_per_s = cpu_ms_per_second(sampling, rate, elapsed),
            private_mib = private_bytes as f64 / (1024.0 * 1024.0),
            "daemon cost over the last {} s",
            elapsed.as_secs()
        );
        for slot in &mut self.slots {
            info!(
                module = slot.id,
                cpu_ms_per_s = cpu_ms_per_second(slot.cycles, rate, elapsed),
                "module sampling cost"
            );
            slot.cycles = 0;
        }
    }
}

/// A running sampler.
pub struct SamplerHandle {
    stop: Sender<()>,
    thread: JoinHandle<()>,
}

impl SamplerHandle {
    /// Stops the sampler and waits for its thread. A sample in progress finishes
    /// first; the next tick is not awaited.
    pub fn stop(self) -> thread::Result<()> {
        drop(self.stop);
        self.thread.join()
    }

    /// Waits for the sampler thread, which only ends early if it panics.
    pub fn join(self) -> thread::Result<()> {
        let Self { stop, thread } = self;
        let result = thread.join();
        drop(stop);
        result
    }
}

/// Process-wide counters at the start of a cost report window.
struct CostWindow {
    started: Instant,
    process_cycles: Option<u64>,
}

impl CostWindow {
    fn start(now: Instant) -> Self {
        Self {
            started: now,
            process_cycles: process_cycles().ok(),
        }
    }
}

/// Sleeps until `due`. Returns false if a stop was requested, by a message or by the
/// handle being dropped.
fn wait_until(due: Instant, stop_requested: &Receiver<()>) -> bool {
    let timeout = due.saturating_duration_since(Instant::now());
    matches!(
        stop_requested.recv_timeout(timeout),
        Err(RecvTimeoutError::Timeout)
    )
}

/// Converts cycles spent over `elapsed` into milliseconds of CPU per second, rounded to
/// hundredths for the log.
fn cpu_ms_per_second(cycles: u64, cycles_per_second: f64, elapsed: Duration) -> f64 {
    let secs = elapsed.as_secs_f64();
    if secs <= 0.0 || cycles_per_second <= 0.0 {
        return 0.0;
    }
    let ms = cycles as f64 / cycles_per_second * 1000.0 / secs;
    (ms * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_cycles_to_cpu_milliseconds_per_second() {
        // 3 GHz counter, 90 million cycles over 10 s: 30 ms of CPU, 3 ms per second.
        let ms = cpu_ms_per_second(90_000_000, 3e9, Duration::from_secs(10));
        assert!((ms - 3.0).abs() < 1e-9, "{ms}");
    }

    #[test]
    fn rounds_to_hundredths() {
        let ms = cpu_ms_per_second(1_234_567, 1e9, Duration::from_secs(1));
        assert!((ms - 1.23).abs() < 1e-9, "{ms}");
    }

    #[test]
    fn an_empty_window_costs_nothing() {
        assert_eq!(cpu_ms_per_second(1000, 3e9, Duration::ZERO), 0.0);
    }
}

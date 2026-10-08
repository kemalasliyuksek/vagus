use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use tracing::info;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;
use vagus_core::{CollectionStore, Module};
use vagus_daemon::Sampler;
use vagus_perf::PerfModule;
use vagus_platform_windows::ProcessSnapshot;
use vagus_platform_windows::current_process::estimate_cycle_rate;

/// Metrics are sampled at 1 Hz (ADR 0019).
const SAMPLE_PERIOD: Duration = Duration::from_secs(1);

fn main() -> Result<()> {
    // `VAGUS_LOG` sets the filter, for example `VAGUS_LOG=debug` (ADR 0013).
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .with_env_var("VAGUS_LOG")
        .from_env()
        .context("VAGUS_LOG is not a valid log filter")?;
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    let cycles_per_second = estimate_cycle_rate().context("estimating the cycle counter rate")?;
    let modules: Vec<Box<dyn Module>> = vec![Box::new(PerfModule::new(ProcessSnapshot::new()))];
    let store = Arc::new(CollectionStore::default());
    let sampler = Sampler::new(modules, store, SAMPLE_PERIOD, cycles_per_second)
        .spawn()
        .context("starting the sampler thread")?;
    info!(cycles_per_second, "vagus-daemon started");

    sampler
        .join()
        .map_err(|_| anyhow!("the sampler thread panicked"))
}

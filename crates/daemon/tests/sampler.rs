use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::json;
use vagus_core::{CollectionStore, Module, ModuleError, ModuleManifest, SampleContext};
use vagus_daemon::Sampler;

/// Records every tick it is called on and publishes the latest one.
struct Recorder {
    ticks: Arc<Mutex<Vec<u64>>>,
}

impl Module for Recorder {
    fn manifest(&self) -> ModuleManifest {
        ModuleManifest {
            id: "recorder",
            collections: Vec::new(),
        }
    }

    fn sample(&mut self, ctx: &SampleContext<'_>) -> Result<(), ModuleError> {
        self.ticks
            .lock()
            .expect("no test thread panics while holding the lock")
            .push(ctx.tick());
        ctx.publish("recorder.last_tick", Arc::new(ctx.tick()));
        Ok(())
    }
}

/// Fails on every tick.
struct Failing;

impl Module for Failing {
    fn manifest(&self) -> ModuleManifest {
        ModuleManifest {
            id: "failing",
            collections: Vec::new(),
        }
    }

    fn sample(&mut self, _ctx: &SampleContext<'_>) -> Result<(), ModuleError> {
        Err(ModuleError::Source(Box::new(std::io::Error::other(
            "scripted",
        ))))
    }
}

fn recorder() -> (Box<dyn Module>, Arc<Mutex<Vec<u64>>>) {
    let ticks = Arc::new(Mutex::new(Vec::new()));
    let module = Recorder {
        ticks: Arc::clone(&ticks),
    };
    (Box::new(module), ticks)
}

/// Waits until `ticks` holds at least `count` entries, for up to five seconds.
fn wait_for_ticks(ticks: &Mutex<Vec<u64>>, count: usize) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while ticks
        .lock()
        .expect("no test thread panics while holding the lock")
        .len()
        < count
    {
        assert!(
            Instant::now() < deadline,
            "the sampler did not reach {count} ticks"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn modules_run_on_increasing_ticks_and_publish() {
    let (module, ticks) = recorder();
    let store = Arc::new(CollectionStore::default());
    let sampler = Sampler::new(
        vec![module],
        Arc::clone(&store),
        Duration::from_millis(10),
        3e9,
    )
    .spawn()
    .unwrap();

    wait_for_ticks(&ticks, 5);
    sampler.stop().unwrap();

    let ticks = ticks.lock().unwrap().clone();
    assert_eq!(ticks[0], 0);
    // A busy machine may skip ticks, but never repeats or reorders them.
    assert!(ticks.windows(2).all(|pair| pair[0] < pair[1]), "{ticks:?}");
    let last = store
        .latest("recorder.last_tick")
        .unwrap()
        .to_json()
        .unwrap();
    assert_eq!(last, json!(ticks.last().unwrap()));
}

#[test]
fn a_failing_module_does_not_stop_the_others() {
    let (module, ticks) = recorder();
    let sampler = Sampler::new(
        vec![Box::new(Failing), module],
        Arc::default(),
        Duration::from_millis(10),
        3e9,
    )
    .spawn()
    .unwrap();

    wait_for_ticks(&ticks, 3);
    sampler.stop().unwrap();
}

#[test]
fn stop_does_not_wait_for_the_next_tick() {
    let (module, ticks) = recorder();
    let sampler = Sampler::new(vec![module], Arc::default(), Duration::from_secs(60), 3e9)
        .spawn()
        .unwrap();
    wait_for_ticks(&ticks, 1);

    let stopping = Instant::now();
    sampler.stop().unwrap();

    assert!(stopping.elapsed() < Duration::from_secs(1));
    assert_eq!(*ticks.lock().unwrap(), [0]);
}

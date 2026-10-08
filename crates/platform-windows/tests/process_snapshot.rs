//! Checks the snapshot against the live process list. It needs no special hardware or
//! rights, so it runs on every Windows machine, CI included.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use vagus_platform_windows::{ProcessRecord, ProcessSnapshot};

/// The Unix epoch as a `FILETIME`, in 100 ns units since 1601-01-01.
const UNIX_EPOCH_AS_FILETIME: u64 = 116_444_736_000_000_000;
const FILETIME_UNITS_PER_SECOND: u64 = 10_000_000;

fn now_as_filetime() -> u64 {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the system clock is after 1970");
    UNIX_EPOCH_AS_FILETIME + since_epoch.as_secs() * FILETIME_UNITS_PER_SECOND
}

/// Busy work long enough to span several timer ticks (15.6 ms by default).
fn burn_cpu(duration: Duration) {
    let start = Instant::now();
    let mut counter = 0u64;
    while start.elapsed() < duration {
        counter = std::hint::black_box(counter.wrapping_add(1));
    }
}

fn find(snapshot: &ProcessSnapshot, pid: u32) -> Option<ProcessRecord<'_>> {
    snapshot
        .processes()
        .map(|p| p.expect("the live snapshot parses"))
        .find(|p| p.pid == pid)
}

#[test]
fn finds_this_process_with_plausible_counters() {
    burn_cpu(Duration::from_millis(100));
    let mut snapshot = ProcessSnapshot::new();
    snapshot.refresh().unwrap();
    let own = find(&snapshot, std::process::id()).expect("this process is listed");

    let exe = std::env::current_exe().unwrap();
    let expected_name = exe.file_name().unwrap().to_string_lossy();
    assert_eq!(own.image_name.to_string_lossy(), expected_name);

    // The test binary started moments ago; an hour leaves room for a slow CI runner.
    // The bound includes one extra second because `now` is truncated to seconds.
    let now = now_as_filetime() + FILETIME_UNITS_PER_SECOND;
    let an_hour_ago = now - 3600 * FILETIME_UNITS_PER_SECOND;
    assert!(
        (an_hour_ago..=now).contains(&own.create_time),
        "create time {} is not within the last hour (now {now})",
        own.create_time
    );

    assert!(own.cycle_time > 0);
    assert!(own.user_time + own.kernel_time > 0);
    assert!(own.private_working_set > 0);
    assert!(own.private_working_set <= own.working_set);
    assert!(own.private_bytes > 0);
}

/// The I/O counters sit in the part of the entry that winternl.h leaves unnamed, so this
/// checks their offsets against real transfers.
#[test]
fn counts_this_process_file_io() {
    const SIZE: usize = 64 * 1024;
    let path = std::env::temp_dir().join(format!("vagus-snapshot-test-{}.bin", std::process::id()));
    std::fs::write(&path, vec![0x5a; SIZE]).unwrap();
    let read_back = std::fs::read(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(read_back.len(), SIZE);

    let mut snapshot = ProcessSnapshot::new();
    snapshot.refresh().unwrap();
    let own = find(&snapshot, std::process::id()).unwrap();
    assert!(
        own.write_bytes >= SIZE as u64,
        "write bytes {}",
        own.write_bytes
    );
    assert!(
        own.read_bytes >= SIZE as u64,
        "read bytes {}",
        own.read_bytes
    );
}

#[test]
fn lists_the_idle_and_system_processes() {
    let mut snapshot = ProcessSnapshot::new();
    snapshot.refresh().unwrap();

    let idle = find(&snapshot, 0).expect("the idle process is listed");
    assert!(idle.image_name.is_empty());
    let system = find(&snapshot, 4).expect("the System process is listed");
    assert_eq!(system.image_name.to_string_lossy(), "System");
}

#[test]
fn a_refresh_replaces_the_previous_snapshot() {
    let pid = std::process::id();
    let mut snapshot = ProcessSnapshot::new();

    snapshot.refresh().unwrap();
    let before = find(&snapshot, pid).unwrap().cycle_time;
    burn_cpu(Duration::from_millis(50));
    snapshot.refresh().unwrap();
    let after = find(&snapshot, pid).unwrap().cycle_time;

    assert!(
        after > before,
        "cycle time did not grow: {before} -> {after}"
    );
}

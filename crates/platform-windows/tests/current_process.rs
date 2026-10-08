//! Checks the self-measurement calls against work this test does itself.

use std::time::{Duration, Instant};

use vagus_platform_windows::current_process::{
    estimate_cycle_rate, process_cycles, process_private_bytes, thread_cycles,
};

fn spin(duration: Duration) {
    let start = Instant::now();
    while start.elapsed() < duration {
        std::hint::spin_loop();
    }
}

#[test]
fn thread_and_process_cycles_grow_with_work() {
    let thread_before = thread_cycles().unwrap();
    let process_before = process_cycles().unwrap();
    spin(Duration::from_millis(20));
    let thread_spent = thread_cycles().unwrap() - thread_before;
    let process_spent = process_cycles().unwrap() - process_before;

    assert!(thread_spent > 0);
    // The process total includes this thread.
    assert!(process_spent >= thread_spent);
}

#[test]
fn cycle_rate_is_plausible_and_repeatable() {
    let first = estimate_cycle_rate().unwrap();
    let second = estimate_cycle_rate().unwrap();

    for rate in [first, second] {
        assert!((0.5e9..10e9).contains(&rate), "{rate} cycles per second");
    }
    let spread = (first - second).abs() / first.max(second);
    assert!(
        spread < 0.1,
        "estimates {first} and {second} differ by {spread}"
    );
}

#[test]
fn spinning_for_a_known_time_converts_back_to_that_time() {
    let rate = estimate_cycle_rate().unwrap();
    // A preempted spin uses less CPU than its wall time, never more, so the best of a
    // few spins is the one to compare.
    let cpu_ms = (0..3)
        .map(|_| {
            let before = thread_cycles().unwrap();
            spin(Duration::from_millis(50));
            (thread_cycles().unwrap() - before) as f64 / rate * 1000.0
        })
        .fold(0.0, f64::max);

    assert!((40.0..=55.0).contains(&cpu_ms), "{cpu_ms} ms of CPU");
}

#[test]
fn private_bytes_follow_a_large_allocation() {
    const SIZE: usize = 64 * 1024 * 1024;
    let before = process_private_bytes().unwrap();
    let block = vec![1u8; SIZE];
    let during = process_private_bytes().unwrap();
    drop(std::hint::black_box(block));

    assert!(
        during >= before + (SIZE as u64) * 9 / 10,
        "private bytes went from {before} to {during}"
    );
}

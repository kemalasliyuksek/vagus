//! Cost of the calling process and thread, for the daemon's self-measurement.

use std::io;
use std::mem::size_of;
use std::time::{Duration, Instant};

use windows_sys::Win32::System::ProcessStatus::{
    GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetCurrentThread};
use windows_sys::Win32::System::WindowsProgramming::{QueryProcessCycleTime, QueryThreadCycleTime};

/// CPU cycles charged so far to the calling thread, user and kernel mode together.
///
/// Unlike `GetThreadTimes`, which advances in timer ticks (15.6 ms by default), cycle
/// counts do not overstate short periodic work.
pub fn thread_cycles() -> io::Result<u64> {
    let mut cycles = 0;
    // SAFETY: GetCurrentThread returns a pseudo handle that is always valid for the
    // calling thread, and `cycles` is a valid out pointer.
    let ok = unsafe { QueryThreadCycleTime(GetCurrentThread(), &mut cycles) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(cycles)
}

/// CPU cycles charged so far to all threads of this process.
pub fn process_cycles() -> io::Result<u64> {
    let mut cycles = 0;
    // SAFETY: GetCurrentProcess returns a pseudo handle that is always valid for this
    // process, and `cycles` is a valid out pointer.
    let ok = unsafe { QueryProcessCycleTime(GetCurrentProcess(), &mut cycles) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(cycles)
}

/// Private committed memory of this process in bytes (Task Manager's "Commit size").
pub fn process_private_bytes() -> io::Result<u64> {
    let mut counters = PROCESS_MEMORY_COUNTERS_EX {
        cb: size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
        ..Default::default()
    };
    // SAFETY: GetCurrentProcess returns a pseudo handle that is always valid for this
    // process. The extended structure starts with PROCESS_MEMORY_COUNTERS, and `cb`
    // tells the API how large the buffer really is.
    let ok = unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            (&raw mut counters).cast::<PROCESS_MEMORY_COUNTERS>(),
            counters.cb,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(counters.PrivateUsage as u64)
}

/// Estimates how many units the cycle counter advances per second, to turn cycle
/// counts into CPU time.
///
/// Windows does not report this rate. Microsoft's documentation warns that on some
/// processors the counter follows the core clock; on processors with an invariant
/// time-stamp counter it runs at a fixed rate, which this estimate relies on, as the
/// Phase 0 measurements did. The calling thread spins for many short rounds and the
/// highest rate wins: a round in which the thread was preempted counts fewer cycles,
/// never more, and on a busy machine many short rounds are less likely to all be
/// preempted than a few long ones.
pub fn estimate_cycle_rate() -> io::Result<f64> {
    const ROUNDS: u32 = 20;
    const SPIN: Duration = Duration::from_millis(2);
    let mut best = 0.0_f64;
    for _ in 0..ROUNDS {
        let start_cycles = thread_cycles()?;
        let start = Instant::now();
        while start.elapsed() < SPIN {
            std::hint::spin_loop();
        }
        let elapsed = start.elapsed();
        let cycles = thread_cycles()?.saturating_sub(start_cycles);
        best = best.max(cycles as f64 / elapsed.as_secs_f64());
    }
    Ok(best)
}

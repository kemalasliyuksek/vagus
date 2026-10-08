# Process snapshot cost: NtQuerySystemInformation

- Date: 2026-10-06
- Phase 0 item: processes ([roadmap](../roadmap.md))
- Machine: the [reference machine](2026-10-06-reference-machine-baseline.md), Windows
  build 26300.9457, on AC, standard (non-elevated) user. The Windows power scheme was
  named "Silent"; the Armoury Crate operating mode was not recorded, and the scheme
  name does not reflect it (see the
  [thermal and power modes note](2026-10-08-thermal-and-power-modes.md)).
- Scope labels follow [ADR 0018](../adr/0018-generic-windows-baseline-vendor-integrations-optional.md):
  **generic** means it follows from how Windows works and should hold on any PC;
  **this machine** means it may depend on the hardware or power settings.

## Question

What does one `NtQuerySystemInformation(SystemProcessInformation)` snapshot cost at 1 Hz,
how does the cost scale, and what can a standard user read about each process?

Decision rule, written before measuring: accept if the snapshot plus parsing costs at
most 2 ms at p95 when called once per second; otherwise consider a slower cadence for
the process list.

## Method

Throwaway spike, kept outside the repository, built in release mode against
`windows-sys` 0.61.2.

- One reused buffer, grown with headroom on `STATUS_INFO_LENGTH_MISMATCH`.
- "Parse" turns each entry into a fixed-size row (PID, create time, CPU time, working
  set, private bytes, I/O bytes, thread count) in a reused vector, sorts by PID and
  merge-joins with the previous tick to compute CPU deltas, as the perf module will.
- Wall time from `Instant` (QPC). CPU from `QueryThreadCycleTime` per call and
  `QueryProcessCycleTime` per run.
- Modes:
  - **back to back:** 50 warm-up calls, then 1000 calls;
  - **1 Hz:** 300 calls on a fixed one-second schedule;
  - **thread scaling:** six rounds of 200 calls each, alternating without and with
    5000 extra idle threads inside the spike process, so slow drifts affect both sides
    equally;
  - **memory probe:** a dependent-load chase over 256 MB, a buffer much larger than L3,
    run before each call, back to back and at 1 Hz;
  - **coverage:** one pass over every process, counting successes only. Command lines
    were read only to test the call, then discarded; none were printed or stored.

## Results

### Cost at 1 Hz

377 to 395 processes, about 8000 threads, 300 calls:

| | p50 | p95 | p99 | max |
|---|---|---|---|---|
| Snapshot + parse | 6.06 ms | 7.22 ms | 8.43 ms | 8.81 ms |

The cost was flat across the five minutes. A second, 60-second run gave p50 6.16 ms and
p95 7.22 ms.

CPU over that 60-second run was 400 ms by `QueryProcessCycleTime`. That is 0.67 % of one
core, or 0.021 % of the 32 logical processors (**this machine**).

### Cost depends on how busy the memory subsystem is (this machine)

When calls run back to back, the cost falls the longer they run. In one run of 3000
calls:

- the first 300 calls averaged 8.3 ms, and the last 300 averaged 3.0 ms;
- the process count stayed at 379 to 383;
- a fixed ALU loop timed next to each call sped up by only 8 %, from 248 µs to 229 µs.

So the CPU clock does not explain the change.

The memory probe separates the possible causes:

| Run | DRAM latency per load | Snapshot |
|---|---|---|
| Back to back, chase before each call | 119 to 130 ns | 2.0 to 2.8 ms |
| 1 Hz, chase before each call | 178 to 200 ns | 5.8 to 6.5 ms |

The chase evicts L3 before every call, yet the call is fast back to back, so cache
contents are not the cause. Between one-second ticks the platform's memory subsystem
returns to a slower, low-power state, and every tick pays for it.

The 1 Hz figure is the one that matters, because the daemon will call it once per
second. Later runs showed that the penalty persists in the Performance mode and grows
on battery ([thermal and power modes](2026-10-08-thermal-and-power-modes.md)).
Whether it is specific to this AMD platform is still open.

### Cost scales at least linearly with thread count (generic)

Interleaved rounds, back to back:

| Threads | Snapshot p50 |
|---|---|
| ~7900 (machine as is) | 2.93 to 3.09 ms |
| ~12900 (+5000 idle threads) | 5.55 to 5.88 ms |

Each of the six rounds gave a ratio of 1.82 to 1.97. A 63 % increase in threads nearly
doubled the cost.

The buffer holds an 80-byte record per thread. With ~7900 threads those records are
about 70 % of the 860 KB snapshot. Machines with many threads, such as development
machines running WSL, Docker and several browsers, will pay proportionally more.

### Memory and handles

- The buffer reached 1.1 MB capacity after one grow at start-up and was not
  reallocated afterwards.
- In the back-to-back and 1 Hz runs, where they were recorded, the spike's private
  bytes stayed at about 2.2 MB and its handle count did not change.

### Tick-sampled CPU time overstates short periodic work (generic)

Over the same 60-second run:

| Source | CPU charged to the spike |
|---|---|
| `GetProcessTimes` | 953 ms |
| `QueryProcessCycleTime` | 400 ms |

The cycle count matches the per-call measurements. The `GetProcessTimes` figure exceeds
the total time spent in the calls, so it cannot be right. Kernel and user times are
sampled at clock ticks, which misattributes short, timer-driven bursts; here the
overstatement was 2.4×.

The cycle counters advanced at about 2.4 GHz of wall time in every mode. That is the
CPU's nominal frequency, so on this machine they count at a constant rate and work as a
precise CPU-time clock.

### What a standard user can read (generic)

Over 378 processes (Idle excluded):

| Data | Readable |
|---|---|
| Image path by PID (`SystemProcessIdInformation`, no handle needed) | 378 (100 %) |
| `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` | 219 (58 %) |
| Full image path through the handle | 219 |
| Command line through the handle | 219 |

159 processes could not be opened: 150 in session 0, and 9 in the user session.

- **Session 0:** Windows services, security software, vendor services and
  virtualization services.
- **User session:** for example `csrss.exe`, `winlogon.exe`, `dwm.exe` and
  `fontdrvhost.exe`.

## Verdict

**The decision rule fails.** The p95 at 1 Hz is 7.2 ms against a limit of 2 ms, and a
tick that includes a snapshot cannot meet the design document's 5 ms tick budget on its
own.

`SystemProcessInformation` remains the right source. One call covers every process,
including the 42 % that cannot be opened without admin. No cheaper documented
alternative is known yet.

## Consequences for the design

To be settled with the prototype tick (block 4 of the Phase 0 plan), once every
source's cost is known:

- **Separate cadences.** System-wide metrics at 1 Hz from cheap sources; the process
  snapshot less often, for example every 2 seconds. Per-process CPU, I/O and fault
  counters are cumulative, so a slower snapshot loses no attribution, only time
  resolution. Processes that start and exit between two snapshots are missed.
- **Budget definition.** "Below 5 ms wall time per tick" may be the wrong measure
  for a single kernel call of ~6 ms that blocks nothing else. A budget in CPU time per
  second, on its own thread, may describe the real cost better.
- **CPU accounting.** The daemon's self-measurement must use cycle counters
  (`QueryProcessCycleTime`). Per-process CPU should be checked against the snapshot's
  per-process `CycleTime` field rather than the tick-sampled user and kernel times.
  To be verified against Task Manager in Phase 1.
- **Process details.** Image paths come from `SystemProcessIdInformation` for every
  process. Command lines are available for about 58 % of processes without admin; the
  UI must show the rest as unavailable rather than empty.
- **GPU attribution.** D3DKMT per-process statistics need a process handle, so without
  admin they would miss the same 42 %, including `dwm.exe`, one of the heaviest GPU
  users. This feeds the PDH versus D3DKMT comparison.

## Open

- Look for a documented information class that omits per-thread records.
- Repeat on a machine with a different platform (Intel, desktop) to learn whether the
  low-power penalty is specific to this one.

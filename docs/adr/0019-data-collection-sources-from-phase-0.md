# 0019. Data collection sources and cadences, from Phase 0 measurements

- Status: accepted (supersedes [ADR 0010](0010-data-collection-strategy.md); completes the
  baseline source validation of
  [ADR 0018](0018-generic-windows-baseline-vendor-integrations-optional.md))
- Date: 2026-10-08
- Deciders: Kemal Aslıyüksek

## Context and problem

[ADR 0010](0010-data-collection-strategy.md) settled the strategy: metrics are polled,
inventory is event-driven, and WMI and process spawning stay out of periodic paths. Its
list of preferred sources was left "to be confirmed by Phase 0 measurements". ADR 0018
did the same for the baseline thermal signals.

Phase 0 measured every candidate on the reference machine
([summary](../research/2026-10-08-phase-0-summary.md)). Several assumptions did not
hold:

- D3DKMT is not the cheaper GPU source.
- `GetIfTable2` is too costly to call every tick.
- The PDH queue length counter costs as much as a full process scan.
- `% Performance Limit` did not report throttling.
- A per-tick wall-time budget punished sources that wait without using CPU.

## Considered options

- Accept ADR 0010's list as written.
- Keep ADR 0010 and record the corrections only in research notes.
- Replace ADR 0010 with a decision based on the measurements.

## Decision

**1. The strategy stays.**
- Metrics are polled.
- Inventory is event-driven, with a slow poll only as a fallback.
- WMI queries, spawning processes and `Win32_Product` are banned from periodic code
  paths.
- Every module reports its own sampling cost.

**2. Cadence and threads.**
- **Metrics:** 1 Hz, on a dedicated sampling thread.
- **Process snapshot:** every 2 seconds. It costs about 6 ms of CPU, and per-process
  counters are cumulative, so the slower cadence costs time resolution, not
  attribution.
- **Sources that wait on hardware:** sources such as the Energy Meter run on their own
  thread and publish their latest value. They never block the sampling thread.
- **CPU measurement:** CPU time is measured with cycle counters, not tick-sampled
  process times.

**3. Accepted sources.**

| Data | Source | Cadence | Privilege |
|---|---|---|---|
| Per-process CPU, memory, I/O, thread states | `NtQuerySystemInformation(SystemProcessInformation)` | 2 s | user |
| Run-queue length | Ready threads in the same snapshot | 2 s | user |
| Process image path | `NtQuerySystemInformation(SystemProcessIdInformation)` | new process | user |
| Command line | `ProcessCommandLineInformation`, redacted | on demand | user; about 58 % of processes |
| CPU total and per core, memory, disk, thermal zone | PDH with English counter names | 1 Hz | user |
| GPU utilization per process, engine and adapter | PDH `GPU Engine` | 1 Hz | user |
| Dedicated GPU memory per process | PDH `GPU Process Memory` | 1 Hz | user |
| Adapter names | D3DKMT driver description, by LUID | adapter change | user |
| Display topology | `EnumDisplayDevicesW` | display change | user |
| Package and core power | PDH Energy Meter, where present | 1 Hz, own thread | user |
| Network totals | `GetIfEntry2` on hardware interfaces that are up | 1 Hz | user |
| Power source | `GetSystemPowerStatus` | 1 Hz | user |
| Power mode | effective power mode notifications | event | user |
| Services and the processes hosting them | `EnumServicesStatusExW`, on service change notifications | event | user |
| Per-process network bytes | kernel ETW, `Microsoft-Windows-Kernel-Network` | streaming, in `vagus-sensor` | admin ([ADR 0009](0009-privilege-model.md)) |

The interface list is rebuilt with `GetIfTable2` only when interfaces change. Inventory
sources that Phase 0 did not measure keep ADR 0010's choices:

- installed programs: the uninstall registry keys with `RegNotifyChangeKeyValue`;
- git repositories: `ReadDirectoryChangesW` and `gix`;
- Docker: the Engine API events stream;
- service changes: `NotifyServiceStatusChangeW`.

**4. Rejected or demoted.**
- **D3DKMT engine totals:** 8 ms per tick.
- **D3DKMT per-process statistics:** they need `PROCESS_QUERY_INFORMATION` and cannot
  see `dwm.exe` or `System`.
- **PDH `\System\Processor Queue Length`:** 4.3 ms.
- **`GetIfTable2` every tick:** 1.8 ms, allocates on every call, and counts virtual
  interfaces.
- **The active power scheme name:** unchanged by vendor modes.
- **The `Processor Frequency` counter:** not the effective frequency.
- **`Kernel-File` events as disk I/O:** they include sockets and cached I/O.

**5. Baseline thermal signals (ADR 0018, point 3).**

| Signal | Phase 0 result |
|---|---|
| Thermal zone temperature | Kept, labeled low confidence. |
| Passive limit and throttle reasons | Kept. They did not move in any run. |
| `% Performance Limit` | A secondary signal only: it stayed at 100 under power caps and thermal regulation. |
| Effective frequency | Replaces the frequency counter: nominal × `% Processor Performance` / 100. Throttling is detected from this falling while the CPU is busy. |
| Effective power mode | Kept as the power mode signal. |
| Package and core power | Added to the baseline where the Energy Meter exists. Power held flat at high utilization indicates a power cap. |
| GPU temperature | Comes from D3DKMT adapter perf data once it is shown not to wake a sleeping discrete GPU. |

**6. Budgets.** The design document's CPU budgets are CPU time per second: sampling
below 10 ms and the daemon below 15 ms on the reference machine.

## Consequences

- Positive: the measured prototype of this loop used 7.3 ms of CPU per second with
  flat memory.
- Positive: every periodic source except per-process network works without admin.
- Negative: per-process attribution has 2-second resolution. A process that starts and
  exits between snapshots is missed.
- Negative: the sampler uses two threads, so the lifecycle of the waiting sources needs
  care.
- Negative: the costs come from one AMD laptop. The snapshot scales with thread count,
  and one effect (a low-power penalty between ticks) may be specific to this platform.
  Costs are re-measured in Phase 1 on CI and on other hardware when available.
- Open:
  - whether GPU queries wake a sleeping discrete GPU;
  - per-process disk attribution;
  - per-process CPU from cycle counts, checked against Task Manager.

## Why the other options were rejected

- **Accept ADR 0010 as written:** three of its candidates were measured and lost to
  alternatives that cost less and cover more.
- **Corrections only in research notes:** agents and contributors read the ADRs first,
  and a proposed ADR with known-wrong sources would mislead them.

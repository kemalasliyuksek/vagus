# Phase 0 summary: data sources on the reference machine

- Date: 2026-10-08
- Machine: the [reference machine](2026-10-06-reference-machine-baseline.md), Windows
  build 26300.9457, standard user unless noted.
- Decisions taken from these results: [ADR 0019](../adr/0019-data-collection-sources-from-phase-0.md).

Phase 0 measured each candidate data source before the daemon is written. This note
collects the results; the linked notes hold the method, raw figures and caveats.
Costs are at 1 Hz with cold calls, on AC, unless stated. "CPU" is thread CPU time from
cycle counters.

## Sources

### Processes and system

| Source | Gives | Cost (p50 / p95) | Admin | Verdict | Note |
|---|---|---|---|---|---|
| `NtQuerySystemInformation(SystemProcessInformation)` plus parsing | per-process CPU, memory, I/O, thread states | 6.1 / 7.2 ms; on battery 9.6 / 17.0 ms | no | accepted, every 2 s | [snapshot](2026-10-06-process-snapshot-cost.md) |
| Ready threads in the same snapshot | run-queue length | none extra | no | accepted; replaces the PDH counter | [PDH](2026-10-07-pdh-system-counters.md) |
| `SystemProcessIdInformation` | image path by PID | one call per new process | no | accepted, 100 % coverage | [snapshot](2026-10-06-process-snapshot-cost.md) |
| `ProcessCommandLineInformation` through `OpenProcess` | command line | on demand | no | accepted, 58 % coverage, redacted | [snapshot](2026-10-06-process-snapshot-cost.md) |
| `SystemProcessorPerformanceInformation` | per-core times | 0.03 / 0.15 ms | no | alternative to PDH per-core | [PDH](2026-10-07-pdh-system-counters.md) |
| PDH, reduced set: CPU, per-core utility, memory, disk, thermal zone | system counters | 0.63 / 1.27 ms | no | accepted, 1 Hz | [PDH](2026-10-07-pdh-system-counters.md) |
| PDH `\System\Processor Queue Length` | run-queue length | 4.3 / 5.0 ms | no | rejected | [PDH](2026-10-07-pdh-system-counters.md) |
| `GetIfEntry2` on hardware interfaces that are up | network totals | 0.15 / 0.20 ms | no | accepted, 1 Hz | [prototype](2026-10-08-baseline-sources-and-prototype-tick.md) |
| `GetIfTable2` | all 53 interfaces | 1.80 / 2.23 ms, allocates | no | only to rebuild the interface list | [prototype](2026-10-08-baseline-sources-and-prototype-tick.md) |
| `GetSystemPowerStatus` | power source, battery | 0.008 ms | no | accepted, 1 Hz | [prototype](2026-10-08-baseline-sources-and-prototype-tick.md) |
| Effective power mode notifications | Windows power mode | event | no | accepted | [power modes](2026-10-08-thermal-and-power-modes.md) |
| Active power scheme name | scheme name | not measured | no | rejected: unchanged by vendor modes | [power modes](2026-10-08-thermal-and-power-modes.md) |
| `EnumServicesStatusExW` | services with their processes | 0.93 ms, 0.18 ms CPU | no | accepted, on change; 96 of 96 svchost processes mapped | [prototype](2026-10-08-baseline-sources-and-prototype-tick.md) |

### Thermal and power

| Source | Gives | Cost (p50 / p95) | Admin | Verdict | Note |
|---|---|---|---|---|---|
| PDH thermal zone temperature | temperature | in the reduced set | no | accepted, labeled low confidence; matched the vendor CPU reading within 1 to 3 °C on this machine | [power modes](2026-10-08-thermal-and-power-modes.md) |
| PDH thermal zone passive limit, throttle reasons | ACPI throttling | in the reduced set | no | accepted; did not move in any run | [PDH](2026-10-07-pdh-system-counters.md) |
| PDH `% Processor Performance` | effective frequency, throttling | in the reduced set | no | accepted; the signal that moved | [power modes](2026-10-08-thermal-and-power-modes.md) |
| PDH `% Performance Limit` | firmware limit | in the reduced set | no | secondary only: stayed at 100 under power caps and thermal regulation | [power modes](2026-10-08-thermal-and-power-modes.md) |
| PDH `Processor Frequency` | frequency | in the reduced set | no | rejected: not the effective frequency | [PDH](2026-10-07-pdh-system-counters.md) |
| PDH Energy Meter (RAPL) | package and core power | 1.5 / 4.1 ms wall, 0.08 ms CPU, up to 160 ms waits | no | accepted where present, on its own thread | [prototype](2026-10-08-baseline-sources-and-prototype-tick.md) |

### GPU

| Source | Gives | Cost (p50 / p95) | Admin | Verdict | Note |
|---|---|---|---|---|---|
| PDH `GPU Engine` | utilization per process, engine and adapter | 1.05 / 1.48 ms (746 instances) | no | accepted, 1 Hz | [GPU](2026-10-08-gpu-sources.md) |
| PDH `GPU Process Memory` | dedicated memory per process | not measured | no | accepted; cost to be measured in Phase 1 | [power modes](2026-10-08-thermal-and-power-modes.md) |
| D3DKMT engine totals | utilization per engine | 8.0 / 10.4 ms | no | rejected | [GPU](2026-10-08-gpu-sources.md) |
| D3DKMT per-process statistics | utilization per process | 0.57 / 0.83 ms | no, but needs `PROCESS_QUERY_INFORMATION` | rejected: cannot see `dwm.exe` or `System` | [GPU](2026-10-08-gpu-sources.md) |
| D3DKMT adapter perf data | GPU temperature, memory clock | 0.60 / 1.13 ms for 3 adapters | no | candidate; whether it wakes a sleeping discrete GPU is open | [GPU](2026-10-08-gpu-sources.md) |
| D3DKMT driver description | adapter name per LUID | once per adapter | no | accepted | [GPU](2026-10-08-gpu-sources.md) |
| `EnumDisplayDevicesW` | which adapter drives the desktop | once per display change | no | accepted | [GPU](2026-10-08-gpu-sources.md) |

### Privileged and UI

| Source | Gives | Cost | Admin | Verdict | Note |
|---|---|---|---|---|---|
| Kernel ETW, `Microsoft-Windows-Kernel-Network` | network bytes per process | consumer 0.42 ms CPU per second in normal use, 6.6 ms at 29,000 events per second; no lost events | yes | accepted for the Phase 2 sensor service | [ETW](2026-10-08-kernel-etw.md) |
| Kernel ETW, `Microsoft-Windows-Kernel-File` | file-object I/O per process | same session | yes | not a disk view: includes sockets and cached I/O | [ETW](2026-10-08-kernel-etw.md) |
| WebView2 through Tauri's `wry` | the desktop window | first frame 326 to 422 ms; 170 MiB open; 115 ms reopen when kept warm | no | budgets met | [WebView2](2026-10-08-webview2-open-time-and-memory.md) |

## The whole sampling loop

A 15-minute prototype ran every accepted periodic source on its cadence: the reduced
PDH set with `GPU Engine`, network, power status and the Energy Meter every second, and
the process snapshot every 2 seconds.

| Measure | Result |
|---|---|
| CPU | 7.3 ms per second (0.73 % of one core) |
| Memory | flat |
| Ticks over 5 ms of wall time | 53 % |

The slow ticks are caused by the Energy Meter's waits and by the snapshot being one
6 ms call. This led to the CPU-per-second budgets now in the design document
([prototype note](2026-10-08-baseline-sources-and-prototype-tick.md)).

## Findings that apply beyond this machine

- **Tick-sampled times overstate short periodic work.** CPU times from
  `GetProcessTimes` overstated the spike's own cost by 2.4×, so CPU must be measured
  with cycle counters.
- **The process snapshot scales with threads.** Its cost grows at least linearly with
  the thread count.
- **Interfaces must be filtered.** Summing every network interface counts traffic more
  than once; only hardware interfaces that are up should be summed.
- **Some vendor sensors wait on hardware.** A source that waits must not sit on the
  sampling thread.
- **Generic counters can explain a hybrid-GPU laptop.** They show which applications
  keep the discrete GPU awake, matching the vendor tool's own list.

## Findings that may be specific to this machine

- **Low-power penalty.** The snapshot costs two to three times more at 1 Hz than back to
  back, because the memory subsystem drops into a low-power state between ticks, and
  more again on battery.
- **Thermal zone.** It reports the CPU temperature here, but may measure something else
  on other machines.
- **`% Performance Limit`.** It never left 100, even under clear power caps.

## Still open

- **Discrete GPU wakes.** Do GPU queries (PDH `GPU Engine`, D3DKMT perf data) wake a
  sleeping discrete GPU? This needs the discrete GPU to be allowed to sleep, which
  first means moving the Claude desktop app and Snipping Tool to the integrated GPU.
- **Per-process disk I/O.** Attribution at the disk level.
- **Per-process CPU.** Per-process CPU from cycle counts, checked against Task Manager.
- **Other hardware.** Every cost on an Intel machine and on a desktop.

# Remaining baseline sources and a prototype sampling loop

- Date: 2026-10-08
- Phase 0 item: other baseline sources and the prototype tick ([roadmap](../roadmap.md))
- Machine: the [reference machine](2026-10-06-reference-machine-baseline.md), Windows
  build 26300.9457, on AC, effective power mode "max performance", standard
  (non-elevated) user.
- Scope labels as in the [process snapshot note](2026-10-06-process-snapshot-cost.md).

## Questions

1. What do the remaining baseline sources cost: network counters, power status and the
   service list?
2. How should network totals and the svchost-to-service mapping be computed?
3. Does a loop running every chosen source on its cadence meet the performance budgets
   in the design document, and does its memory stay flat?

## Method

Throwaway spike outside the repository, `windows-sys` 0.61.2.

- **Source costs:** a 1 Hz rotation, 60 cold calls per source.
  - `GetIfTable2`, which returns every interface and allocates the table on each call.
  - `GetIfEntry2` on a reused row for each hardware interface that is up.
  - `GetSystemPowerStatus`.
  - `EnumServicesStatusExW`, with process information.
- **Interfaces:** counted by IANA type and by the `HardwareInterface` flag. Names and
  addresses were not recorded.
- **Services:** processes of running services matched against `svchost.exe` processes
  from the process snapshot.
- **Prototype:** 900 one-second ticks, about 15 minutes, on one thread:
  - **every second:** one PDH query with the reduced set from the
    [PDH note](2026-10-07-pdh-system-counters.md) plus `GPU Engine`; `GetIfEntry2` for
    the hardware interfaces; `GetSystemPowerStatus`; the Energy Meter package counter,
    in its own query;
  - **every second tick:** the process snapshot, with per-process CPU deltas and the
    Ready-thread count that replaces the queue length counter.

  Measured: wall time and thread cycles per part and per tick; process CPU from
  `QueryProcessCycleTime`; private bytes and handle count at the start, middle and end.

## Results

### Source costs at 1 Hz

| Source | Wall p50 | Wall p95 | CPU p50 |
|---|---|---|---|
| `GetIfTable2`, 53 interfaces | 1.80 ms | 2.23 ms | 1.29 ms |
| `GetIfEntry2`, one hardware interface | 0.15 ms | 0.20 ms | 0.11 ms |
| `GetSystemPowerStatus` | 0.008 ms | 0.010 ms | 0.009 ms |
| `EnumServicesStatusExW` | 0.93 ms | 1.22 ms | 0.18 ms |

The service enumeration spends most of its time waiting on the service control manager.
It is needed only when a service changes, according to
[ADR 0010](../adr/0010-data-collection-strategy.md).

### Network totals need the hardware flag (generic)

Of 53 interfaces:

- 2 are hardware interfaces: one Ethernet and one Wi-Fi;
- 29 are up;
- 1 is both hardware and up.

| IANA type | Interfaces | Hardware |
|---|---|---|
| 6, Ethernet | 28 | 1 |
| 71, Wi-Fi | 16 | 1 |
| 131, tunnel | 7 | 0 |
| 24, loopback | 1 | 0 |
| 23, PPP | 1 | 0 |

The rest are virtual switches and adapters (Hyper-V, WSL, Docker), filter interfaces
and tunnels. Several of them carry the same traffic again. Summing every interface
would count bytes more than once.

Network totals should sum the hardware interfaces that are up. Traffic through a VPN
tunnel is then counted once, encrypted, on the physical interface. The interface set
changes when the machine moves between networks or a VPN connects. The list should be
refreshed on interface change notifications; that part was not measured.

### Every svchost process maps to a service (generic)

- **Services:** 318 Win32 services; 144 are running in a process; there are 134
  distinct service processes.
- **svchost:** 96 `svchost.exe` processes, all 96 matched to at least one service.
- **Sharing:** at most 5 services share one process.

`EnumServicesStatusExW` alone is enough to name what every svchost process hosts.

### Prototype loop

Over 900 ticks:

| Part | Wall p50 | Wall p95 | Wall p99 | Wall max | CPU p50 |
|---|---|---|---|---|---|
| Fast part: PDH with GPU Engine, network, power | 1.46 ms | 2.37 ms | 4.72 ms | 10.5 ms | 1.26 ms |
| Energy Meter, package counter only | 1.44 ms | 4.41 ms | 21.4 ms | 160 ms | 0.03 ms |
| Process snapshot, every 2 s | 6.27 ms | 8.38 ms | 13.1 ms | 23.5 ms | 6.16 ms |
| Whole tick | 8.22 ms | 12.29 ms | 32.2 ms | 167 ms | 5.83 ms |

- **CPU:** 7.30 ms per second on average, 0.73 % of one core, or 0.023 % of the 32
  logical processors, measured with cycle counters.
- **Memory:** private bytes were 8.6, 7.8 and 8.2 MB at the start, middle and end.
  Handles were 236, 235 and 235. Neither grew.
- **Ticks over 5 ms:** 480 of the 900 ticks took longer than 5 ms of wall time.

**The Energy Meter waits (this machine).** It used 0.03 ms of CPU per read. Its wall
time reached 21 ms at p99 and 160 ms once. Almost all of the slowest whole ticks come
from it.

## Verdict against the design budgets

**"One sampling tick below 5 ms wall time": fails,** for 53 % of ticks. This budget
measures the wrong thing:

- **Waiting counted as work.** The Energy Meter's waits cost no CPU but break the
  budget.
- **An indivisible call.** The process snapshot is one kernel call of about 6 ms that
  cannot be split.

**"Daemon CPU below 0.5 % of total CPU": passes by a factor of 20, and says little.**
On 32 logical processors 0.5 % of total CPU allows 160 ms of CPU per second, while the
whole sampling loop uses 7.3 ms. The same percentage would allow 20 ms on a 4-thread
machine. A share of total CPU depends on core count. It is a poor target for a program
whose work does not grow with core count.

Before measuring, the plan set per-source budgets: 2 ms for the process list, 1 ms for
PDH and 1 ms for GPU. Each was missed: the snapshot by a wide margin, PDH (1.27 ms) and
GPU (1.48 ms) narrowly. The decision below replaces that framework. It is made because
the old measure mixed waiting with work, not to excuse the misses. The per-source
numbers stay in the research notes.

## Decision

These changes are recorded in the design document:

- **Sampling cost:** all Phase 1 modules together stay below 10 ms of CPU time per
  second on the reference machine, on AC, measured with cycle counters. Measured:
  7.3 ms.
- **Daemon CPU:** below 15 ms of CPU time per second, 1.5 % of one core, as a 1-minute
  average. This leaves room for the recorder, rules and API on top of sampling.
- **Sampling thread:** no source blocks it. Sources that wait on hardware, such as the
  Energy Meter, are read on their own thread and publish their latest value.
- **Cadence:** system metrics at 1 Hz, the process snapshot every 2 seconds.
- **Network:** `GetIfEntry2` on the hardware interfaces that are up, not `GetIfTable2`
  every tick.

## Open

- **Battery:** repeat the prototype on battery, where the snapshot costs more (p50
  9.6 ms in the [thermal and power modes note](2026-10-08-thermal-and-power-modes.md)).
- **Interface changes:** the cost and reliability of interface change notifications.
- **Other machines:** whether the budget holds there, especially with more threads,
  since the snapshot cost scales with thread count.

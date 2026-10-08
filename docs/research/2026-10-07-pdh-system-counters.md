# PDH system counters: availability, cost and throttling signals

- Date: 2026-10-07
- Phase 0 item: PDH and thermal ([roadmap](../roadmap.md))
- Machine: the [reference machine](2026-10-06-reference-machine-baseline.md), Windows
  build 26300.9457, on AC, standard (non-elevated) user. The Windows power scheme was
  named "Silent"; the Armoury Crate operating mode was not recorded. The package power
  under load (up to 97 W) matches the Performance mode measured on 2026-10-08, not
  Silent, which held 65 W ([thermal and power modes](2026-10-08-thermal-and-power-modes.md)).
- Scope labels as in the [process snapshot note](2026-10-06-process-snapshot-cost.md):
  **generic** or **this machine**.

## Question

Which of the planned PDH counters exist? What does collecting them cost at 1 Hz? Does
any of that cost show up in another process? Do the throttling counters show
throttling?

Decision rules, written before measuring:

- **Cost:** the system set is accepted if it costs at most 1 ms at p95 at 1 Hz, with no
  meaningful cost in other processes.
- **Throttling counter:** if frequency falls while `% Performance Limit` stays at 100,
  the counter is not reliable here, and the diagnosis must use `% Processor
  Performance` instead.

## Method

Throwaway spike outside the repository, `windows-sys` 0.61.2. All counters were added
with `PdhAddEnglishCounterW`.

- **Cost:** each counter group is its own query. At 1 Hz, one group is collected per
  tick in rotation, so every measurement starts from an idle system. Measured work:
  `PdhCollectQueryData`, plus reading every value with `PdhGetFormattedCounterArrayW`
  into a reused buffer, without converting names to strings. 40 ticks per group; the
  reduced set got its own 120-tick run.
- **Hidden cost:** a 60-second idle control period, then 60 seconds of collecting the
  full set at 10 Hz. Per-process cycle counts of every other process were compared
  between the two periods.
- **Throttling:** 1 Hz log of the throttling, thermal and power counters: 60 s idle,
  300 s with all 32 logical processors busy, 120 s cool-down.
- **Queue length:** `\System\Processor Queue Length` compared with the number of
  threads in the Ready state in the process snapshot, idle and with 64 busy threads.

## Results

### Availability

All 20 planned counters exist on this machine. Of note:

- `Thermal Zone Information`: a single zone, `\_TZ.TZ01`. `High Precision
  Temperature` moved in whole-kelvin steps.
- `Energy Meter(*)\Power`: RAPL power in milliwatts, readable without admin. It has one
  instance per core (16), one for the package, and `_Total`. The platform's energy
  meter interface provides it, so its presence varies between machines.
- `Power Meter(*)\Power`: one instance, which read 0 on AC.

### Cost per group at 1 Hz

| Group | Counters | Wall p50 | Wall p95 | CPU p50 |
|---|---|---|---|---|
| CPU total | utility, performance, limit, frequency | 0.14 ms | 0.62 ms | 0.14 ms |
| CPU per core | `% Processor Utility`, 34 instances | 0.14 ms | 0.43 ms | 0.13 ms |
| Memory | 4 counters | 0.14 ms | 0.18 ms | 0.14 ms |
| Disk | 3 counters × 2 instances | 0.21 ms | 0.26 ms | 0.21 ms |
| Thermal zone | temperature, passive limit, throttle reasons | 0.14 ms | 0.19 ms | 0.07 ms |
| Energy Meter | 18 instances | 1.52 ms | 4.08 ms | 0.08 ms |
| `\System\Processor Queue Length` | 1 counter | 4.32 ms | 5.01 ms | 4.31 ms |
| **Full set**, one query | all of the above | 6.30 ms | 11.67 ms | 4.67 ms |
| **Reduced set**, one query | without queue length and Energy Meter | 0.63 ms | 1.27 ms | 0.39 ms |
| `NtQuerySystemInformation` per-processor times, for comparison | | 0.03 ms | 0.15 ms | 0.02 ms |

CPU time is derived from thread cycle counts, which count at about 2.4 GHz on this
machine.

Back to back, the full set took 4.2 ms at p50, 11.4 ms at p95, and 80 ms at worst.

**The queue length counter is expensive (generic).** It costs 4.3 ms of CPU, all of it
in the spike's own process. PDH computes the whole `System` object to return any of its
counters. That object includes process and thread counts, and its cost is close to
that of the process snapshot. This explanation is an inference, not a measurement.

**The Energy Meter waits rather than works (this machine).** It used 0.08 ms of CPU but
1.5 ms of wall time, with a 23 ms worst case. The time goes into reading the hardware.

### Queue length from the process snapshot (generic)

The process snapshot already includes every thread's scheduler state. The number of
threads in the Ready state matched the PDH counter:

- **Idle:** 0 to 1, in both sources.
- **With 64 busy threads:** pairs such as 92 and 93, 100 and 100, 88 and 87.

The remaining differences come from the two readings being a few milliseconds apart.

### Hidden cost

Neither period showed a consistent increase in WMI or counter-provider host processes.
The measured cost stays in the calling process, as the cycle counts above indicate.

The machine's background activity was high, so small effects were invisible. Other
processes used about 115 s of CPU per minute in both periods, mostly the System
process, Defender and a browser. Single processes varied by several seconds between
periods, so an effect below about 1 % of one core could not be resolved.

### Throttling signals under sustained load (this machine)

| Phase | `% Processor Utility` | `% Processor Performance` | `% Performance Limit` | Thermal zone | Package power |
|---|---|---|---|---|---|
| Idle, 60 s | 2 to 24 | 150 to 193 | 100 | 77 to 89 °C | 28 to 59 W |
| All-core load, 300 s | 158 to 194 | 158 to 194 | 100 | 93 to 95 °C | 74 to 97 W |
| Cool-down, 120 s | 3 to 21 | 150 to 187 | 100 | 72 to 94 °C | 32 to 52 W |

- **No limit was reported.** `% Performance Limit` stayed at 100 throughout. The thermal
  zone held a flat 94 to 95 °C under load, which is consistent with the processor
  holding a firmware temperature target. It manages that target itself, and this
  counter does not report it as a limit.
- **No ACPI throttling.** `% Passive Limit` stayed at 100 and `Throttle Reasons` at 0,
  so ACPI passive cooling never engaged.
- **Frequency counter:** `Processor Frequency` read 2401 MHz, the nominal frequency,
  throughout this run. On 2026-10-08 it read 1992 to 2137 MHz at idle in the Silent
  mode on AC and 2401 MHz in every other phase, while the effective frequency ranged
  from about 0.7 to 4.9 GHz. It does not
  track the effective frequency, which is nominal × `% Processor Performance` / 100.
- **Thermal zone response:** the zone followed load within about 10 seconds in both
  directions. A later comparison with Armoury Crate confirmed that on this machine it
  reports the CPU temperature ([thermal and power modes](2026-10-08-thermal-and-power-modes.md)).
- **Idle power:** the package drew 28 to 59 W during the "idle" phase. Utilization was
  2 to 24 %, with the background activity described above. That is enough to explain
  a high idle temperature.

## Verdict

**Cost rule: fails for the planned set.** The full set costs 11.7 ms at p95, almost all
of it in two counters. The reduced set also misses narrowly: 1.27 ms at p95 against
1 ms, with a p50 of 0.63 ms.

**Throttling rule: `% Performance Limit` is not reliable here.** It stayed at 100 while
the processor was regulating itself at its temperature target. A value below 100 is
still meaningful where a platform reports it. A value of 100 proves nothing.

## Consequences for the design

- **Drop `\System\Processor Queue Length`.** Count Ready threads in the process snapshot
  instead. It costs nothing extra, but it is only as frequent as the snapshot.
- **Treat the Energy Meter as an optional baseline sensor.** Detect it, since not every
  platform has it, and read it off the sampling thread or at a lower rate, because of
  its wait time and worst case. Package power explains heat and battery drain without
  any vendor integration. This adds a signal to
  [ADR 0018](../adr/0018-generic-windows-baseline-vendor-integrations-optional.md)'s
  baseline list.
- **Diagnose throttling from what actually moves.** Use `% Processor Performance` and
  effective frequency falling under load, the thermal zone near its plateau, and
  package power. Use `% Performance Limit` and the ACPI counters only when they leave
  their idle values.
- **Report effective frequency, not the frequency counter.**
- **Close the remaining gap in block 4.** One option is per-core CPU from
  `NtQuerySystemInformation` processor times (0.03 ms) instead of PDH per-core utility.
  That gives up frequency scaling per core, but per-core times are cumulative and
  cheap. Decide once the cost of every source is known.

## Open

- On other platforms, check whether `% Performance Limit` moves under power or thermal
  limits (Intel in particular).

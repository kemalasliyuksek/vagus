# GPU sources: PDH GPU Engine versus D3DKMT

- Date: 2026-10-08
- Phase 0 item: GPU ([roadmap](../roadmap.md)). The dGPU power-state part (block 3a) is
  still open: it needs a discrete GPU that is allowed to sleep, which this machine's
  current setup does not allow (see the
  [thermal and power modes note](2026-10-08-thermal-and-power-modes.md)).
- Machine: the [reference machine](2026-10-06-reference-machine-baseline.md), Windows
  build 26300.9457, on AC, effective power mode "max performance", standard
  (non-elevated) user. GPU mode Standard (MSHybrid).
- Adapters:
  - NVIDIA GeForce RTX 5060 Laptop GPU, 14 engines;
  - AMD Radeon 610M, 11 engines;
  - Microsoft Basic Render Driver, 33 engines.
- Scope labels as in the [process snapshot note](2026-10-06-process-snapshot-cost.md).

## Question

Which source should give per-process GPU use, per-adapter utilization, adapter names and
GPU temperature, at what cost and with what coverage?

Decision rule, written before measuring: D3DKMT becomes the primary utilization source
only if all three hold:

- its chosen strategy costs at most 1 ms at p95 at 1 Hz;
- it agrees with PDH within 5 points;
- its process coverage is acceptable.

Otherwise PDH `GPU Engine` is primary.

## Method

Throwaway spike outside the repository, `windows-sys` 0.61.2.

- **Adapters:** `D3DKMTEnumAdapters2`, then `D3DKMTQueryAdapterInfo` for the driver
  description and `D3DKMTQueryStatistics(ADAPTER)` for the engine count. DXGI is not
  used.
- **Cost:** a 1 Hz rotation, so every call is cold, with 60 ticks per source. The
  sources:
  1. **PDH:** collect `\GPU Engine(*)\Utilization Percentage` and read every value into
     a reused buffer.
  2. **D3DKMT engine totals:** `D3DKMTQueryStatistics(NODE)` for every engine of every
     adapter, and separately for the hardware adapters only.
  3. **D3DKMT per process:** `D3DKMTQueryStatistics(PROCESS_NODE)` for every engine of
     every process and adapter pair where the process has a device. Handles are opened
     once and reused, as the daemon would.
  4. **D3DKMT perf data:** `D3DKMTQueryAdapterInfo(ADAPTERPERFDATA)` for every adapter.
- **Agreement:** once per second for 20 seconds, the busiest engine per adapter from
  PDH, with each engine's utilization summed over processes, against D3DKMT engine
  running-time deltas.
- **Coverage:** processes that have `GPU Engine` instances, compared with those whose
  handles D3DKMT accepts.

## Results

### Cost at 1 Hz

| Source | Calls per tick | Wall p50 | Wall p95 | CPU p50 |
|---|---|---|---|---|
| PDH `GPU Engine` | 1 collect, 746 instances | 1.05 ms | 1.48 ms | 1.04 ms |
| D3DKMT engine totals, all adapters | 58 | 8.03 ms | 10.35 ms | 7.8 ms |
| D3DKMT engine totals, hardware adapters | 25 | 7.83 ms | 10.12 ms | 7.7 ms |
| D3DKMT per process | 641 (37 process/adapter pairs) | 0.57 ms | 0.83 ms | 0.56 ms |
| D3DKMT adapter perf data | 3 | 0.60 ms | 1.13 ms | 0.59 ms |

**Engine totals are expensive (this machine).** Each engine query on the two hardware
adapters costs about 0.3 ms of CPU, while the 33 software-adapter engines add almost
nothing.

A NODE query also returns the engine's perf data (frequency and voltage), which
suggests the call reaches the display driver. That is an inference. If it is true,
these queries are also candidates for waking a sleeping discrete GPU (block 3a).

**Per-process queries are cheap,** at under 1 µs per call. 125 of the 641 calls per tick
failed, for engines a process had not used.

### Agreement (generic)

The two sources agreed within 0.5 points on every sample. Examples of PDH against
D3DKMT on the integrated GPU: 53.4 % and 53.4 %, 27.6 % and 27.5 %, 41.5 % and 41.6 %.
The NVIDIA adapter was idle during this window, and both sources read 0.0 % for it.
PDH's `GPU Engine` counters appear to be computed from the same kernel statistics.

### Coverage (generic)

D3DKMT per-process statistics need a handle opened with `PROCESS_QUERY_INFORMATION`.

- **Handle rights:** handles opened with `PROCESS_QUERY_LIMITED_INFORMATION` were always
  rejected with `STATUS_INVALID_PARAMETER`.
- **Adapters without a device:** the same status comes back for an adapter on which the
  process has no device.

| | Processes |
|---|---|
| With `GPU Engine` instances | 27 |
| Openable with the limited right | 22 |
| Openable with the query right, which D3DKMT needs | 19 |
| Active in a 2-second window | 3 |
| Active and queryable by D3DKMT | 1 |

The active processes that D3DKMT could not see were `System` and `dwm.exe`. DWM is one
of the heaviest GPU users on this machine (see the thermal note). PDH reports both, by
PID, without opening any process.

### Adapter names, display topology and dedicated memory (generic)

- **Names:** `D3DKMTQueryAdapterInfo(DRIVER_DESCRIPTION)` gives the adapter name for
  each LUID, so PDH instance names can be mapped to adapters without DXGI. LUIDs change
  at every boot and must be mapped at runtime.
- **Display topology:** `EnumDisplayDevicesW` shows which adapter drives each display
  that is attached to the desktop.
- **Dedicated memory:** `\GPU Process Memory(*)\Dedicated Usage` shows which processes
  hold memory on each adapter. The thermal note used this to find what keeps the
  discrete GPU awake.

### GPU temperature from D3DKMT perf data

`D3DKMT_ADAPTER_PERFDATA` returned:

- **NVIDIA:** 494, read as 49.4 °C. The field is in tenths of a degree. The memory
  clock read 405 MHz, which matches Armoury Crate's "Memory Frequency 405 MHz" at idle.
  The power and fan fields read 0, so this driver does not fill them.
- **AMD 610M:** 570 to 620, read as 57 to 62 °C.
- **Basic Render Driver:** not available.

Armoury Crate showed the NVIDIA GPU at 50 to 61 °C during the day's other captures, but
never at the same moment as these readings, so this is not yet a comparison (**this
machine**).

## Verdict

**PDH `GPU Engine` is the primary source for GPU utilization, per process and per
adapter.** D3DKMT engine totals cost 8 ms. D3DKMT per-process statistics are cheap but
cannot see `dwm.exe` or `System` and need a stronger access right. This reverses the
candidate order in [ADR 0010](../adr/0010-data-collection-strategy.md), which is updated
at the end of Phase 0.

PDH costs 1.48 ms at p95, slightly above the 1 ms set for GPU in the tick budget. This
is settled with the tick budget in block 4.

**D3DKMT stays in a supporting role:**

- adapter names (LUID to name);
- GPU temperature and memory clock from adapter perf data, which is a candidate for the
  baseline GPU temperature in
  [ADR 0018](../adr/0018-generic-windows-baseline-vendor-integrations-optional.md),
  pending the wake test.

## Open

- **Block 3a.** Detect the discrete GPU's power state without waking it, then check
  whether PDH `GPU Engine`, D3DKMT engine queries or D3DKMT perf data wake it. This
  needs a sleeping discrete GPU: the Claude desktop app and Snipping Tool must be moved
  to the integrated GPU first.
- **GPU temperature check.** Compare D3DKMT GPU temperature with Armoury Crate at the
  same moment.
- **Instance churn.** Measure how the cost of PDH `GPU Engine` grows with the instance
  count, for example with many browser tabs open.

# Thermal zone accuracy and power modes on the reference machine

- Date: 2026-10-08
- Phase 0 items: thermal, and the open items of the
  [process snapshot](2026-10-06-process-snapshot-cost.md) and
  [PDH](2026-10-07-pdh-system-counters.md) notes
- Machine: the [reference machine](2026-10-06-reference-machine-baseline.md), Windows
  build 26300.9457, standard (non-elevated) user. Armoury Crate GPU mode: Standard
  (MSHybrid).
- Background load: one Docker container in all runs, and a second Claude Code session
  during runs B and C.
- Scope labels as in the [process snapshot note](2026-10-06-process-snapshot-cost.md);
  most results here are **this machine**.

## Questions

1. Does the ACPI thermal zone report the CPU temperature here? The question has been
   open since the baseline, which read 83 to 89 °C at idle.
2. How do the vendor power modes and battery power change power limits, throttling
   and the cost of the process snapshot?
3. What can Vagus learn about the active mode through Windows alone?

## Method

Three runs, each a 1 Hz log of these counters, through an idle phase, an all-core load
phase and a cool-down:

- `% Processor Utility`, `% Processor Performance`, `% Performance Limit` and
  `Processor Frequency`;
- thermal zone temperature, `% Passive Limit` and `Throttle Reasons`;
- RAPL package power from the Energy Meter;
- the busiest GPU engine per adapter, from `\GPU Engine(*)\Utilization Percentage`.

| Run | Power source | Armoury Crate mode | Phases (idle / load / cool) |
|---|---|---|---|
| A | AC | Silent | 120 / 180 / 120 s |
| B | AC | Performance | 60 / 180 / 60 s, after a 60 s process snapshot run at 1 Hz |
| C | battery | Silent, chosen by Armoury Crate on unplugging | 60 / 180 / 60 s, after a 60 s process snapshot run at 1 Hz |

- **Reference values:** the owner captured Armoury Crate's Home screen (CPU
  temperature, fan speeds) at requested moments, 12 captures in all.
- **Windows' view of the mode:** the effective power mode, read with
  `PowerRegisterForEffectivePowerModeNotifications`, and the active power scheme's name.
- **Graphics topology:** which adapter drives the desktop, from `EnumDisplayDevicesW`.
  Adapter names come from D3DKMT. Dedicated GPU memory per process comes from
  `\GPU Process Memory(*)\Dedicated Usage`.

## Results

### The thermal zone matches Armoury Crate's CPU temperature (this machine)

| Moment | Armoury Crate CPU temperature | Thermal zone, same moment or phase |
|---|---|---|
| A, idle, 10:32 to 10:33 | 80 °C | 77 to 80 °C |
| A, load, 10:34 to 10:35 | 86 °C | 85 to 87 °C |
| A, end of load, about 10:36:05 | 88 °C | 88 °C |
| A, cool-down, 10:37 to 10:38 | 63 °C | 62 °C |
| A, idle, 10:38 to 10:39 | 77 °C | 75 to 77 °C |
| B, end of load | 96 °C | 96 °C |
| C, end of load | 66 °C | 67 °C |

The two agree within 1 to 3 °C at idle, under load and while cooling.

At idle the temperature moved by up to 10 °C within 10 seconds, for example from 62 to
72 °C. A reading 35 seconds apart from the reference showed 72 °C against 85 °C, so
comparisons must be made at the same moment.

The baseline's 83 to 89 °C idle readings were real CPU temperatures. They are explained
by 28 to 59 W of package power at low utilization (see the PDH note), not by the zone
measuring something else.

### Power modes and battery (this machine)

Under all-core load:

| | A: Silent, AC | B: Performance, AC | C: battery |
|---|---|---|---|
| Package power | 65 W, flat | 75 to 90 W | 42 W, flat |
| `% Processor Performance` | 162 to 180, then a drop to 29 (below) | 154 to 195 | 102 to 113 |
| Thermal zone | 83 to 88 °C | 93 to 97 °C | 65 to 69 °C |
| `% Performance Limit` | 100 | 100 | 100 |
| Fans: CPU / GPU / system, from Armoury Crate | 3100 / 3900 / 4000 RPM | 3900 / 4600 / 4200 RPM | 3200 / 3900 / 4000 RPM |

- **Power caps:** Silent and battery hold the package power at a flat cap; the samples
  stay within 1 W. Performance has no visible cap. There, the processor reached its
  temperature target after about a minute and settled from about 190 % to 163 %
  performance.
- **Fans:** in Silent, the fans read the same at idle and under load. In Performance
  they read 2500 / 3100 / 2700 RPM at idle and rose under load.
- **Mode memory:** after unplugging, Armoury Crate switched itself to Silent. After
  plugging back in, it returned to Performance, so it keeps a separate mode per power
  source.

### What Windows reports about the mode

| Run | Windows power scheme name | Windows effective power mode |
|---|---|---|
| A | Silent | not recorded (the owner's setting: Best performance) |
| B | Silent | max performance (Best performance) |
| C | Silent | balanced |

- **The scheme name is useless here.** Armoury Crate created a scheme named "Silent",
  but the scheme stays the same when the Armoury Crate mode changes.
- **The effective power mode is useful.** It followed the power source, because Windows
  keeps a separate mode for AC and battery. It does not follow the Armoury Crate mode.
- **The cap is visible.** The clearest generic evidence of a vendor mode is its effect:
  package power pinned flat under load.

### The process snapshot gets more expensive on battery (this machine)

| Run | Snapshot + parse at 1 Hz | CPU |
|---|---|---|
| 2026-10-06, AC, mode not recorded | p50 6.06 ms, p95 7.22 ms | 0.67 % of one core |
| B, Performance, AC | p50 7.40 ms, p95 8.38 ms | 0.82 % of one core |
| C, battery | p50 9.60 ms, p95 16.95 ms, p99 65 ms | 1.39 % of one core |

The low-power penalty does not disappear in the Performance mode, and it grows on
battery.

### A throttling event that the counters missed (this machine)

In run A at 10:36:15, all cores were still busy, yet:

- `% Processor Performance` fell from 162 to 59, then to 29, roughly 0.7 GHz;
- package power fell from 65 to 31 W;
- the low state outlasted the load by about 70 seconds and recovered at 10:37:35.

`% Performance Limit`, `% Passive Limit` and `Throttle Reasons` did not change.

The cause is unknown. The capture taken at that moment showed the NVIDIA GPU at 30 %
utilization, while the other captures showed 0 to 3 %. However, runs B and C kept the
NVIDIA GPU at 15 to 44 % during load without a similar drop, which argues against a
shared power budget as the explanation.

### The discrete GPU never slept, and generic counters show why (generic method)

- **The display:** the only display output on the desktop belongs to the integrated
  AMD GPU. All four NVIDIA outputs were inactive.
- **The NVIDIA adapter was busy anyway:** its busiest engine reported 15 to 44 % in most
  samples of runs B and C.
- **The holders:** processes with dedicated memory on the NVIDIA adapter were
  `dwm.exe` (600 MiB), the Claude desktop app (110 MiB), Snipping Tool (54 MiB),
  `explorer.exe` and smaller shell components.
- **DWM's share:** in the same 5-second window `dwm.exe` used about 47 % of one CPU
  core. It ran at 39 to 43 % on the integrated GPU's 3D engine and at 20 to 25 % on the
  NVIDIA one.

The likely mechanism: applications that render on the discrete GPU keep it awake. DWM
then copies their output to the display, which hangs off the integrated GPU.

Armoury Crate's own list of "applications using the GPU" named the same applications:
two `claude.exe` processes, `snippingtool.exe`, `shellhost.exe` and
`textinputhost.exe`. The generic counters reproduce the vendor tool's answer without
any vendor interface. This is the first concrete support for
[ADR 0018](../adr/0018-generic-windows-baseline-vendor-integrations-optional.md).

## Consequences for the design

- **Thermal zone label:** keep the low-confidence label in general. Vagus cannot tell
  what a zone measures on an unknown machine, even though on this one it matches the
  vendor reading.
- **Power-limit finding:** package power that stays flat while utilization is high is a
  generic sign of a power cap, wherever the Energy Meter exists.
- **Power mode:** report the effective power mode, which is event-driven and fits
  [ADR 0010](../adr/0010-data-collection-strategy.md). Do not use the scheme name.
- **Throttling:** look for `% Processor Performance` falling while the CPU is busy.
  Run A shows a deep drop that no throttling counter reported.
- **Discrete GPU finding:** "the discrete GPU is kept awake by these applications" can
  be produced from generic counters and display topology. It is a candidate Phase 1
  finding.
- **Process snapshot:** a p95 of 17 ms on battery supports a slower process cadence,
  off the 1 Hz tick. Decide in block 4.

## Open

- The cause of the drop in run A.
- Fan speed has no generic source and needs a vendor integration.
- Whether the Optimized GPU mode, or moving the Claude app to the integrated GPU, lowers
  idle power and temperature. This is the owner's choice; measure it if they change it.

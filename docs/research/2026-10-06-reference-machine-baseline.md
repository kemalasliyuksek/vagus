# Reference machine baseline

- Date: 2026-10-06
- Method: one-off PowerShell queries as a standard (non-elevated) user, before any
  Vagus code existed.

These readings are a starting point for Phase 0, not benchmarks.

## Hardware and software

| Item | Value |
|---|---|
| Model | ASUS ROG Strix G18 (G814PM) |
| CPU | AMD Ryzen 9 8940HX, 16 cores / 32 threads |
| Memory | 32 GB |
| GPUs | AMD Radeon 610M (integrated) + NVIDIA GeForce RTX 5060 Laptop GPU (hybrid graphics) |
| OS | Windows 11 Pro, build 26300 |
| Vendor stack | Armoury Crate installed; "ASUS System Control Interface v3" and "Armoury Crate Control Interface" devices present; 12 ASUS services running |
| Toolchain | Rust 1.99.0 (stable, MSVC), Visual Studio Build Tools 2022, WebView2 154, Node.js 24, pnpm 12, gitleaks 8.30 |

## Data sources tried without admin rights

| Source | Result |
|---|---|
| PDH `\GPU Engine(*)\Utilization Percentage` | Readable. Instances encode PID, adapter LUID, engine index and engine type (for example `pid_<pid>_luid_0x..._phys_0_eng_0_engtype_3D`), so per-process GPU use and the adapter can be attributed. |
| PDH `\Thermal Zone Information(*)\Temperature` | Readable. One zone (`\_TZ.TZ01`). Values in Kelvin; converted readings were 88.9, 84.9 and 82.9 °C across three samples while total CPU load was 3.6 %. |
| WMI `root/wmi:MSAcpi_ThermalZoneTemperature` | Access denied. |
| PDH `\Processor Information(_Total)\% Processor Performance` | 161.9 (running at about 162 % of base frequency, so boosting). |
| PDH `\Processor Information(_Total)\% Performance Limit` | 100 (no firmware limit at the time). |
| PDH `\Processor Information(_Total)\Processor Frequency` | 2401 MHz (nominal base). |
| PDH `\System\Processor Queue Length` | 0. |
| PDH `\Memory\Available MBytes` | 18380. |
| PDH `\Memory\Pages Input/sec` | 15.9 (low). |
| Power | On AC, battery 100 %, active power scheme "Silent" (created by Armoury Crate). |

## Observations

- The system was not under pressure: empty run queue, plenty of free memory, few hard
  faults, CPU boosting and not limited.
- The thermal zone reported 83 to 89 °C while the CPU was nearly idle. With the Silent
  scheme, the fans may simply be running slowly. Alternatively, the zone does not
  measure the CPU die. This cannot be told apart without fan speed and a die
  temperature. That is exactly why [ADR 0012](../adr/0012-hardware-sensor-sources.md)
  labels the thermal zone as low confidence and plans ATKACPI as the primary source.
- `% Performance Limit` and the power scheme are cheap, admin-free signals that explain
  "slow or hot for no visible reason". They belong in the first version of the
  diagnosis engine.
- PDH counter names are localized by display language. This machine uses an English
  UI, but Vagus must use `PdhAddEnglishCounterW` regardless.

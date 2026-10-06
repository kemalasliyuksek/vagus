# 0012. Hardware sensor sources without vulnerable drivers

- Status: proposed (ATKACPI access and NVML gating are validated in Phase 0)
- Date: 2026-10-06
- Deciders: Kemal Aslıyüksek (owner), proposed by Claude Code

## Context and problem

Windows has no public user-mode API for CPU temperature or fan speed:

- The ACPI thermal zone counter is readable without admin, but it is not the CPU die
  temperature. On the reference machine it read 83 to 89 °C at 3.6 % CPU load, with
  the "Silent" power plan active (see `docs/research/`).
- On AMD Ryzen, the real CPU temperature is read from the SMU through PCI
  configuration space, which needs a kernel driver.
- Laptop fans are controlled by the embedded controller (EC). Generic tools that look
  for desktop Super I/O chips often cannot see them.

Generic sensor libraries have long relied on WinRing0. That driver gives any user-mode
process access to MSRs, I/O ports and physical memory, has a known vulnerability
(CVE-2020-14979), and has been flagged by Microsoft Defender since 2025. It is a
standard "bring your own vulnerable driver" tool.

On hybrid-GPU laptops, polling the discrete GPU through NVML wakes it from its
low-power state and keeps it awake. That drains the battery and adds heat, so the
monitor creates the very problem it reports.

## Considered options

- LibreHardwareMonitor (a .NET library that relies on a kernel driver).
- Reading HWiNFO's shared memory.
- Bundling WinRing0 or a similar generic driver.
- Vendor-specific sources behind a common trait.

## Decision

A `SensorSource` trait with vendor-specific implementations:

- **ASUS ATKACPI** (the "ASUS System Control Interface" device, which Armoury Crate also
  uses) for CPU and GPU temperatures and fan speeds. **Only read methods are called,
  never write or set methods.** Whether this needs admin is measured in Phase 0. The
  identifiers are documented independently (see
  [ADR 0003](0003-personal-ownership-and-dual-license.md) on not copying GPL code).
- **NVIDIA NVML**, queried **only while the discrete GPU is already awake**. How to
  detect that without waking the GPU is a Phase 0 question.
- **ACPI thermal zone** as a fallback, always labeled as low confidence in the UI.

No WinRing0, and no generic port, MSR or physical-memory driver. Any third-party
kernel driver requires a new ADR first.

## Consequences

- Positive: no vulnerable drivers and no extra installation, and the sensors are
  accurate on supported hardware.
- Negative: coverage starts with ASUS and NVIDIA, so other machines show fewer sensors
  until more sources are written.
- Negative: ATKACPI is undocumented and may change with BIOS updates. The source must
  fail soft, reporting "unavailable" instead of wrong values.

## Why the other options were rejected

- LibreHardwareMonitor: .NET (not our stack), and its CPU sensors depend on a kernel
  driver.
- HWiNFO shared memory: closed source, the user must install and configure HWiNFO, and
  its license terms apply.
- WinRing0 or similar: a known vulnerable driver; shipping it would put users at risk.

# 0018. Generic Windows baseline, vendor integrations as optional modules

- Status: accepted (supersedes [ADR 0012](0012-hardware-sensor-sources.md); the baseline
  sources are validated in Phase 0)
- Date: 2026-10-06
- Deciders: Kemal Aslıyüksek

## Context and problem

Vagus is built for any Windows 11 PC. The reference machine, an ASUS ROG laptop, is
where things are measured, but [ADR 0012](0012-hardware-sensor-sources.md) and the first
Phase 0 plan were shaped around it: ASUS ATKACPI was the primary sensor source.

Windows PCs differ: desktop or laptop, Intel or AMD, one GPU or two, with or without a
battery, with or without vendor software. Some data is available on every Windows 11 PC
through the operating system:

- CPU, memory, disk, network and per-process GPU use;
- power source, power plan and power mode;
- ACPI thermal zones and their throttling counters, where the firmware provides them;
- GPU temperature as reported by the display driver (to be verified in Phase 0).

Other data has no generic Windows API, CPU die temperature and fan speeds in
particular. Laptop vendors expose them through their own, mostly undocumented, ACPI or
WMI interfaces; GPU vendors through their own libraries, such as NVIDIA NVML or AMD
ADLX.

What ADR 0012 established still holds: generic hardware-access drivers such as WinRing0
are a security risk, NVML wakes a sleeping discrete GPU, and an ACPI thermal zone is not
guaranteed to measure the CPU die.

## Considered options

- Vendor sources first, generic sources as a fallback (ADR 0012 as written).
- Generic sources only, no vendor integrations.
- A generic baseline that works everywhere, with vendor integrations as optional
  modules.

## Decision

1. **Baseline.** Every core module works on any Windows 11 PC using only what the
   operating system provides, with no vendor software or driver. When the hardware lacks
   something (no battery, no thermal zone, no discrete GPU), the capability reports "not
   available" and the module keeps working.
2. **Diagnosis needs only the baseline.** Every finding can be produced from baseline
   signals. Vendor data adds detail and confidence, for example a fan speed that explains
   a temperature, and is never a precondition.
3. **Baseline thermal signals**, to be validated in Phase 0: thermal zone temperature
   (always labeled low confidence), thermal zone passive limit and throttle reasons,
   `% Performance Limit`, processor frequency and the power mode. GPU temperature from
   the display driver's adapter performance data, if Phase 0 shows that reading it does
   not wake a sleeping discrete GPU.
4. **Vendor integrations are optional modules**, compiled in
   ([ADR 0017](0017-compiled-in-modules-before-plugins.md)). Each one:
   - activates only when a cheap, side-effect-free probe detects its hardware;
   - calls read methods only, never a write or set method;
   - fails soft, reporting "unavailable" rather than a wrong value, and disables itself
     after repeated failures;
   - learns identifiers from facts, never from copied GPL code
     ([ADR 0003](0003-personal-ownership-and-dual-license.md));
   - moves its reads to `vagus-sensor` if they need admin
     ([ADR 0009](0009-privilege-model.md)).

   GPU vendor libraries such as NVML are queried only while the discrete GPU is already
   awake.
5. **No WinRing0** and no generic port, MSR or physical-memory driver. Any third-party
   kernel driver requires a new ADR first.
6. **Order.** The baseline ships first, in Phase 1. Vendor integrations follow one at a
   time, chosen by how many machines they help and how useful their data is. ASUS
   ATKACPI is the first candidate, because the reference machine can test it.
7. **Machine-specific findings are labeled.** Research notes say whether a result holds
   for Windows in general or only for the reference machine.

## Consequences

- Positive: Vagus is useful on any Windows PC from the first release, and vendor work
  cannot block the core.
- Positive: vendor-specific code is isolated in its own modules and can be removed or
  replaced without touching the core.
- Negative: without a matching vendor integration, CPU temperature is low confidence or
  missing and fan speeds are missing. The diagnosis must say so instead of guessing.
- Negative: only one machine is available for testing. Until more hardware is, generic
  behavior rests on documented API behavior, tests with fakes, and the Windows CI runner,
  a virtual machine without sensors or a discrete GPU that exercises the "not available"
  paths.
- Negative: some vendors expose sensors only through WMI, which
  [ADR 0010](0010-data-collection-strategy.md) bans from periodic code paths. Such an
  integration needs its own decision.
- Phase 0 no longer touches ATKACPI. What the thermal zone measures on the reference
  machine is checked against the temperature Armoury Crate displays, without Vagus
  talking to a vendor driver.

## Why the other options were rejected

- Vendor sources first: the core would be shaped by one laptop, and every other machine
  would get a degraded experience by design.
- Generic sources only: CPU temperature and fan speed are among the most useful signals
  on laptops. Refusing vendor integrations entirely would give that up.

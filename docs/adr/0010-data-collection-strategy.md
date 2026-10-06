# 0010. Data collection strategy: poll metrics, subscribe to inventory

- Status: proposed (source costs are measured in Phase 0)
- Date: 2026-10-06
- Deciders: Kemal Aslıyüksek

## Context and problem

A "brain that sees everything" must stay cheap, or it becomes the thing slowing the
machine down. Windows offers many APIs for the same data, with very different costs.
Vendor utilities that feel slow often poll expensive sources such as WMI or spawn
helper processes on a timer.

## Considered options

- WMI-based collection (simple, uniform API).
- Spawning existing tools (`nvidia-smi`, `git`, PowerShell) and parsing their output.
- Native APIs, polling everything on a timer.
- Native APIs, with polling for metrics and events for inventory.

## Decision

There are two kinds of data, and each is collected in its own way:

- **Metrics** (CPU, memory, disk, GPU, network, power, thermals) change constantly and
  are polled, by default once per second, configurable per module.
- **Inventory** (services, installed programs, git repositories, containers, startup
  items) changes rarely. It is **event-driven**, with a slow poll only as a fallback.

Preferred sources, to be confirmed by Phase 0 measurements:

| Data | Source |
|---|---|
| Per-process CPU, memory, I/O counters | `NtQuerySystemInformation(SystemProcessInformation)`, one call for all processes |
| System counters (performance %, performance limit %, queue length, memory, disk latency) | PDH, added with `PdhAddEnglishCounterW` |
| Network totals | `GetIfTable2` |
| Per-process GPU usage and adapter | D3DKMT statistics APIs (candidate) or PDH `GPU Engine` counters (fallback) |
| Process-to-service mapping | `EnumServicesStatusExW` (gives each service's PID) |
| Power source and plan | `GetSystemPowerStatus`, `PowerGetActiveScheme` |
| Service changes | `NotifyServiceStatusChangeW` |
| Installed programs | Uninstall registry keys, watched with `RegNotifyChangeKeyValue` |
| Git repositories | `ReadDirectoryChangesW` on configured roots, read with `gix` |
| Docker | Engine API over `//./pipe/docker_engine` with `bollard`, using the events stream |
| Per-process network and disk (Phase 2, privileged) | Kernel ETW providers |

**Banned in periodic code paths:** WMI queries, spawning processes, and
`Win32_Product`. Querying `Win32_Product` makes Windows Installer run a consistency
check on every MSI package; it is slow and can trigger repairs.

Every module reports its own sampling cost, and the daemon reports its own CPU and
memory use in the UI.

## Consequences

- Positive: the per-tick cost stays bounded and measurable, and inventory costs nothing
  while nothing changes.
- Negative: native APIs mean more FFI code and more platform-specific work than WMI.
- Negative: event subscriptions need careful lifecycle handling (re-subscribe after
  errors, the service restarting, Docker restarting).

## Why the other options were rejected

- WMI: slow, and queries burn CPU in `WmiPrvSE.exe`, so the monitor would show up as a
  culprit itself. Some providers also need elevation.
- Spawning tools: process creation cost on every tick, fragile output parsing, and
  localized output.
- Polling inventory: wasted work that grows with the number of repositories and
  containers.

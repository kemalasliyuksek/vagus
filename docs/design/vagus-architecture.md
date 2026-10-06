# Vagus architecture

- Status: draft
- Owner: Kemal Aslıyüksek
- Date: 2026-10-06

This is the living description of the system. The reasoning behind each choice lives
in the [ADRs](../adr/README.md). Anything marked **open** has not been decided yet.

## Problem

What a Windows machine is doing is scattered across many tools: Task Manager for
processes, the Services console, Settings for installed programs, vendor utilities for
sensors, Docker Desktop for containers, a terminal for git. Each shows its own slice as
raw numbers. None of them explains anything, and none can be queried by other tools.

The most urgent case is slowness. When a Windows machine slows down or behaves
strangely, finding the cause takes too long:

- **Task Manager** shows numbers, not causes.
- **Resource Monitor and System Informer** are powerful but dense. They answer "what
  is every process doing", not "why is my machine slow right now".
- **Vendor utilities such as Armoury Crate** are slow and opaque. On the reference
  machine, a dozen ASUS background services run all the time.
- **Laptop sensors** that often explain slowness (CPU temperature, fan speed,
  throttling, power mode, which GPU is active) are scattered across tools or hidden
  behind undocumented interfaces.
- **Spikes vanish:** by the time any tool opens, the spike that caused the slowdown is
  often over.

Vagus is a fast, local-first control center for the whole machine:

- One daemon watches everything at almost no cost.
- The desktop app, the CLI, MCP and later a widget all see the same picture through
  one API.
- It grows module by module: services, installed programs, git repositories,
  containers and more.

The first module tackles slowness. It names the cause in one sentence and remembers
the last 15 minutes.

## Goals and non-goals

### Goals

1. **One place for the whole machine.** Processes, hardware, and later services,
   installed programs, repositories and containers, in one app and one API instead of a
   dozen tools.
2. **Explain, do not just display.** When something is wrong, the first screen names
   the bottleneck and the likely cause in one sentence, for example: "CPU is idle but
   at 83 °C: power mode is Silent and the fans are slow".
3. **Remember the recent past.** The last 15 minutes are kept in memory, so a spike that
   already ended is still visible ([ADR 0011](../adr/0011-in-memory-flight-recorder.md)).
4. **Cost almost nothing.** The daemon stays within the
   [performance budgets](#performance-budgets) and shows its own resource use.
5. **Open by design.** Every capability is available to the UI, MCP and the CLI through
   one schema-described API
   ([ADR 0007](../adr/0007-api-first-capability-registry.md),
   [ADR 0008](../adr/0008-mcp-as-a-first-class-interface.md)).
6. **Grow by modules.** New areas (services, apps, repositories, Docker, ...) are added
   without changing the core or the clients
   ([ADR 0017](../adr/0017-compiled-in-modules-before-plugins.md)).
7. **Pleasant to use.** The UI is simple, clear, elegant and customizable, with light
   and dark themes, and launches in English and Turkish
   ([ADR 0014](../adr/0014-localization.md),
   [ADR 0015](../adr/0015-theming-and-customization.md)).
8. **Safe.** Least privilege, gated actions, an audit log and no telemetry
   ([ADR 0009](../adr/0009-privilege-model.md),
   [ADR 0013](../adr/0013-logging-and-no-telemetry.md)).

### Non-goals (for now)

- **Platforms other than Windows 11.** The platform layer is abstracted so Linux can
  follow, but nothing is promised.
- **Changing hardware settings.** Fan control, overclocking, undervolting and power
  limits are out of scope; sensors are read-only.
- **Fleet monitoring.** No remote monitoring of other machines.
- **A dynamic plugin ABI** ([ADR 0017](../adr/0017-compiled-in-modules-before-plugins.md)).
- **Long-term on-disk history** in Phase 1.
- **Deep debugging.** Handles, memory maps and kernel debugging remain System
  Informer's job.
- **The desktop widget** comes in Phase 3, after the windowed app.
- **A bundled AI model or provider.** AI connects through MCP.

## Proposed design

### Overview

```
                    +------------------------------------------+
   Win32 / NT APIs  |               vagus-daemon               |
   PDH, ETW (via    |  scheduler -> modules -> capability      |
   sensor service), |  registry -> flight recorder -> rules    |
   ATKACPI, NVML,   |  -> diagnosis -> notifications           |
   Docker, gix  --> |  config store, audit log, API server     |
                    +--------------------+---------------------+
                                         | JSON-RPC 2.0 over a per-user named pipe
            +----------------------------+----------------------------+
            |                            |                            |
   desktop UI (Tauri)         vagus mcp (stdio bridge)          vagus CLI
   window, tray, widget                 |
                              MCP clients: Claude Code,
                              Claude Desktop, local models
```

From Phase 2 on, two more components appear:

- `vagus-sensor`: a privileged, read-only Windows service that feeds kernel ETW data
  and privileged sensors to the daemon.
- An on-demand elevated helper that performs one admin action per UAC prompt.

### Processes

| Process | Runs as | Lifetime | Responsibility |
|---|---|---|---|
| `vagus-daemon` | user | starts at login, always on | Collection, registry, recorder, diagnosis, notifications, config, API |
| desktop UI | user | while the window is open (optional keep-warm) | Presentation only; a client of the API |
| `vagus` CLI / `vagus mcp` | user | per invocation / per MCP session | API client; MCP bridge |
| `vagus-sensor` (Phase 2) | LocalSystem service | always on | Fixed read-only privileged queries (kernel ETW, privileged sensors) |
| elevated helper (Phase 2) | admin via UAC | one action, then exits | Executes exactly one validated admin action |

The daemon is a user process rather than a service, so that it can show toast
notifications in the user's session and never holds admin rights.

### Capability registry and modules

A **module** is a unit of functionality (perf, sensors, services, ...). It declares its
capabilities in a manifest and is driven by the scheduler. There are four kinds of
capability:

- **Metric**: a numeric time series with an id (`cpu.performance_limit_pct`), a unit and
  labels. Metrics are sampled on the module's interval and stored in the recorder.
- **Entity collection**: inventory items with a schema (processes, services,
  containers). They are updated by events or by sampling.
- **Action**: a named operation with an input schema, a privilege tier
  (`read` / `user` / `admin`) and a flag saying whether it needs confirmation.
- **Event**: something that happened, such as a service stopping, a container exiting
  or a rule firing.

Each schema is generated from Rust types with `schemars`. From the registry, Vagus
generates:

- the API methods;
- the MCP tools, resources and prompts;
- the CLI commands;
- the TypeScript types for the UI.

The trait shape below is illustrative. The real one is designed in Phase 1:

```rust
pub trait Module: Send {
    /// Static description: id, metrics, entity collections, actions, events.
    fn manifest(&self) -> ModuleManifest;

    /// Called by the scheduler on the module's interval. Writes into the context,
    /// never allocates per call in steady state.
    fn sample(&mut self, ctx: &mut SampleContext<'_>) -> Result<(), ModuleError>;

    /// Executes one of the module's declared actions after tier checks.
    fn invoke(&mut self, request: ActionRequest) -> Result<ActionOutcome, ModuleError>;
}
```

### Daemon API

- **Transport:** JSON-RPC 2.0 over a per-user named pipe, `\\.\pipe\vagus-<user SID>`.
  The pipe sets `PIPE_REJECT_REMOTE_CLIENTS` and has an explicit DACL.
- **Handshake:** the client sends `hello`, carrying `protocol_version`, the client kind
  (`ui`, `mcp`, `cli`, `widget`) and the client version.
- **Method families** (names indicative):
  - `capabilities.list`
  - `metrics.query`, `metrics.subscribe`
  - `entities.list`, `entities.subscribe`
  - `actions.invoke`
  - `diagnosis.current`, `diagnosis.subscribe`
  - `config.get`, `config.set`, `config.schema`
  - `logs.tail`, `logs.set_filter`
- **Subscriptions** push batched updates at most once per sampling tick.
- **Encoding:** JSON. MessagePack (or similar) only if measurements show JSON
  serialization is a meaningful share of the daemon's cost (**open**).

The protocol is deliberately the same message model as MCP (JSON-RPC 2.0), which keeps
the MCP bridge thin.

### MCP surface

Details in [ADR 0008](../adr/0008-mcp-as-a-first-class-interface.md). In short:

- **Server:** `vagus mcp`, MCP over stdio, built on `rmcp`.
- **Read tools are on by default**, for example `get_diagnosis`, `list_top_processes`,
  `query_metric`, `list_services` and `read_recent_logs`.
- **Action tools are off by default** and enabled one by one. Admin actions always pass
  through a local UAC prompt.
- **Resources:** for example `vagus://diagnosis/current`, `vagus://logs/recent` and
  `vagus://config/schema`.
- **Prompts:** for example `diagnose_slowness`.
- **Data hygiene:** third-party text is returned as data in delimited fields, never in
  tool descriptions. Secrets are redacted before leaving the daemon.

### Privilege model

Details in [ADR 0009](../adr/0009-privilege-model.md):

- The daemon is never elevated.
- The privileged service only reads.
- Admin actions use a per-action elevated helper behind a UAC prompt.
- Every action is audit-logged.

### Data collection

Details in [ADR 0010](../adr/0010-data-collection-strategy.md):

- **Metrics are polled**, at 1 Hz by default.
- **Inventory is event-driven.**
- **Preferred sources:** native APIs such as `NtQuerySystemInformation`, PDH with
  English counter names, `GetIfTable2`, D3DKMT, and the service, registry and directory
  change notifications.
- **Never in periodic paths:** WMI, spawning processes, or `Win32_Product`.

The sampling loop runs on a dedicated thread so I/O load does not jitter it (initial
plan, confirmed in Phase 0).

### Hardware sensors

Details in [ADR 0012](../adr/0012-hardware-sensor-sources.md). Sensors sit behind a
`SensorSource` trait:

- **ASUS ATKACPI**, called with read methods only.
- **NVIDIA NVML**, queried only while the discrete GPU is already awake.
- **ACPI thermal zone**, as a low-confidence fallback.

No WinRing0 or other generic hardware-access drivers. Every sensor fails soft and
reports "unavailable" rather than a wrong value.

### Flight recorder

Details in [ADR 0011](../adr/0011-in-memory-flight-recorder.md):

- 15 minutes of history in `RingBuffer`s: every system metric each tick, and the top N
  processes per resource.
- Recorder memory of 10 MB or less.
- Evicted samples are recycled.
- No disk persistence in Phase 1.

### Diagnosis and notifications

The diagnosis engine turns raw metrics into findings. For every resource it checks
**utilization, saturation and errors** (Brendan Gregg's USE method), plus the context
signals that commonly explain slowness on laptops.

| Resource | Utilization | Saturation | Context / errors |
|---|---|---|---|
| CPU | total and per-core busy % | processor queue length; runnable time per process | `% Performance Limit` below 100 (throttling), `% Processor Performance`, power plan, AC or battery |
| Memory | committed vs limit, available | hard faults/sec (`Pages Input/sec`) | commit failures |
| Disk | active time | queue length | latency (`Avg. Disk sec/Transfer`), errors (later) |
| GPU | per-engine utilization per adapter | dedicated memory pressure | which adapter each process uses; dGPU power state |
| Network | throughput vs link speed | | errors and discards |
| Thermal | temperatures | fan speed vs temperature | sensor confidence |

A **finding** is an internal model, not a public format yet. It contains:

- `resource` and `severity`;
- a localized summary, as a message key plus parameters;
- **evidence**: references to metrics over a time window;
- **culprits**: references to entities, such as processes or services;
- **suggested actions**: references to registered actions.

The overview screen leads with the top finding as a single sentence, or with "Everything
looks normal" when there are none.

**Rules** are declarative configuration with a duration and hysteresis, so short spikes
do not fire. The format is decided in Phase 1; this example is illustrative only:

```json
{
  "$schema": "./schemas/rule.schema.json",
  "version": 1,
  "id": "memory-pressure",
  "when": { "metric": "memory.hard_faults_per_sec", "above": 500, "for": "30s" },
  "clear": { "below": 200, "for": "60s" },
  "severity": "warning",
  "cooldown": "10m"
}
```

**Notifications** are Windows toasts sent by the daemon:

- Each one carries the finding's sentence and action buttons such as "Show" or "End
  process".
- Each finding has a cooldown, related findings are grouped, and quiet hours are
  supported.
- Text is localized from the shared catalog.

Baseline-based anomaly detection ("unusual for this machine at this hour") comes after
the rule engine has proven itself.

### Logging

Details in [ADR 0013](../adr/0013-logging-and-no-telemetry.md):

- `tracing`, writing JSON Lines under `%LOCALAPPDATA%\Vagus\logs\`, with daily rotation
  and 14-day retention.
- A separate audit log.
- Redaction before anything is written or sent.
- UI errors and panics land in the same log.
- The filter can be changed at runtime.
- Logs are visible in the UI and over MCP.
- No telemetry.

### Configuration

Details in [ADR 0016](../adr/0016-configuration-as-validated-data.md):

- JSON files with `$schema` under `%APPDATA%\Vagus\`: settings, themes, layouts, rules
  and module settings.
- Schemas are generated from Rust types.
- Files are hot-reloaded; if a file is invalid, the last valid version stays active.
- Writes are atomic and audited.
- Each file has a `version` field for migrations.

### User interface

Stack in [ADR 0006](../adr/0006-tauri-and-svelte-for-the-desktop-ui.md): Tauri 2,
Svelte 5 (runes), strict TypeScript, Tailwind 4 on design tokens, Bits UI, uPlot,
Lucide and Paraglide.

Screens in Phase 1:

- **Overview**: the diagnosis sentence, key gauges (CPU, memory, disk, GPU, network,
  thermals, power) and a 15-minute timeline you can scrub back through.
- **Processes**: top consumers, sortable by resource and grouped by application.
  svchost instances are mapped to the services they host, and each process shows the
  GPU adapter it uses.
- **Details**: one view per resource, with history.
- **Logs**: a filterable log viewer.
- **Settings**: theme, language, density, notifications, MCP tool permissions.

Later screens: services, installed programs, startup items, git repositories,
containers, and an AI view.

Principles:

- **Progressive disclosure:** a one-sentence verdict first, numbers on demand.
- **Calm by default:** no blinking and no red unless something actually needs attention.
- **Keyboard first:** a global hotkey opens the window; every view is reachable from
  the keyboard.
- **Window lifecycle:** opened from the hotkey or the tray. Closing the window frees the
  WebView unless "keep warm" is enabled.
- **Theming** ([ADR 0015](../adr/0015-theming-and-customization.md)):
  - tokens as CSS variables, with themes as JSON token sets;
  - system, light or dark mode, plus accent color and density;
  - Mica backdrop on Windows 11;
  - layouts stored as configuration.
- **Accessibility:** WCAG 2.1 AA contrast for built-in themes, respect for
  `prefers-reduced-motion`, and visible focus styles.

### Localization

Details in [ADR 0014](../adr/0014-localization.md):

- English source and Turkish translation in a shared `locales/<lang>.json` catalog,
  used by the UI (Paraglide) and the daemon (notifications).
- No hard-coded user-facing strings.
- Formatting goes through `Intl`; case conversion always passes a locale; sorting uses
  `Intl.Collator`.
- Tests cover the Turkish I.

### Performance budgets

Targets, validated in Phase 0 and 1. Once met, a regression counts as a bug.

| Item | Target |
|---|---|
| Daemon CPU, steady state | below 0.5 % of total CPU, 1-minute average, 1 Hz sampling |
| Daemon memory | below 30 MB private bytes, recorder included |
| Flight recorder | 10 MB or less |
| One sampling tick, all Phase 1 modules | below 5 ms wall time on the reference machine |
| UI window, warm open | below 300 ms to first meaningful paint |
| UI window, cold open | below 1.5 s |
| UI while open and nothing changes | no continuous CPU use |
| `vagus mcp` bridge | below 15 MB, near-zero idle CPU |

The daemon measures itself and shows the numbers in the UI. A soak test (one hour on
the reference machine) checks memory stays flat.

### Platform abstraction

Windows-specific FFI lives in a `platform-windows` crate family, the only place
`unsafe` is allowed. Modules depend on traits in `vagus-core`, which keeps a later
Linux implementation possible without rewriting modules.

### Repository layout

Current and planned (planned entries are marked):

```
vagus/
  crates/
    core/                 vagus-core: shared types, ring buffer; later registry and module trait
    daemon/               (planned) vagus-daemon: scheduler, API, recorder, rules, notifications
    cli/                  (planned) vagus: CLI and the `vagus mcp` bridge
    platform-windows/     (planned) Win32 / NT wrappers, the only crate with unsafe code
    sensor-service/       (planned, Phase 2) vagus-sensor privileged read-only service
    elevate/              (planned, Phase 2) one-shot elevated action helper
    modules/              (planned) perf, sensors, services, ... one crate each
  apps/
    desktop/              (planned) Tauri 2 + Svelte 5 UI
  locales/                (planned) en.json, tr.json
  docs/
    design/               this document
    adr/                  architecture decision records
    research/             measurements and findings
    roadmap.md
  .githooks/              pre-commit secret scan (gitleaks)
```

When `crates/modules/*` appears, the workspace `members` list in `Cargo.toml` needs
that glob added as well.

## Alternatives considered

Each ADR has the full comparison. The main forks were:

- **Build a Rainmeter skin on HWiNFO.** Fast to get something on screen. Rejected: no
  causal diagnosis, no history, no MCP, and it depends on a closed-source sensor tool.
- **Extend System Informer with a plugin.** It has a mature plugin SDK and excellent NT
  internals. Rejected: the goal is an opinionated, calm UX with MCP parity, not another
  panel in a dense expert tool. Its MIT-licensed source remains a valuable reference
  for NT APIs.
- **C# / .NET with LibreHardwareMonitor and WPF.** Fastest path to sensors. Rejected
  for always-on overhead, no Native AOT for WPF, and a driver dependency
  ([ADR 0005](../adr/0005-rust-for-the-core-daemon.md)).
- **egui, Slint, iced or WinUI for the UI.** Lighter or more native. Rejected for
  styling and ecosystem reasons, or for splitting the stack
  ([ADR 0006](../adr/0006-tauri-and-svelte-for-the-desktop-ui.md)).
- **A single elevated process.** Simplest. Rejected as an escalation risk
  ([ADR 0009](../adr/0009-privilege-model.md)).
- **WMI-based collection.** Simplest API. Rejected for cost
  ([ADR 0010](../adr/0010-data-collection-strategy.md)).

## Risks and open questions

Risks:

- **ATKACPI is undocumented.** Does it need admin? Which identifiers apply to this
  model? Is it stable across BIOS updates? Phase 0 measures this. Only read methods are
  ever called.
- **NVML wakes the dGPU.** We need a way to detect the discrete GPU's power state
  without waking it. Phase 0.
- **Thermal zone reading on the reference machine.** It reads 83 to 89 °C at 3.6 %
  CPU load with the Silent plan. It cannot be interpreted until fan speed is known.
  Phase 0.
- **WebView2 under load.** Real cold-open times and memory need measuring on the
  reference machine.
- **Toasts from an unpackaged daemon** need a registered AppUserModelID (normally via a
  Start menu shortcut created by the installer). This affects installer design.
- **Code signing.** Unsigned binaries trigger SmartScreen and "Unknown publisher" in
  UAC prompts. Options for open source signing must be evaluated before the first
  release.
- **Antivirus heuristics.** Tools that enumerate processes, use ETW and talk to
  drivers can trigger false positives. Releases should be submitted to vendors early.
- **Scope creep.** "Monitor everything" invites building the platform before the first
  useful feature. Mitigation: the roadmap ships one module end to end first.

Open questions:

- PDH `GPU Engine` counters versus D3DKMT: cost and adapter mapping (LUID to name).
- IPC encoding: JSON versus a binary format.
- TypeScript binding generator: `specta`, `ts-rs` or `typeshare`.
- How the daemon consumes the Paraglide catalogs, and how plurals are handled.
- Default top-N per resource in the recorder.
- Startup at login: `HKCU\...\Run`, Task Scheduler, or the Startup folder.
- Installer: Tauri bundler, NSIS or MSI, and how the daemon and the Phase 2 service are
  installed.
- Persistent downsampled history: whether to add it, and in what store (SQLite).

## Test and verification plan

- **Unit tests** in `vagus-core`: ring buffer, rule evaluation, redaction, config
  validation and migration.
- **Module contract tests:** platform calls sit behind traits, so modules are tested
  with fakes and recorded fixtures. Hardware-specific sources have smoke tests that skip
  cleanly when the hardware is absent (CI has no ATKACPI).
- **Windows CI** (GitHub Actions, `windows-latest`): fmt, clippy with `-D warnings`,
  tests, `svelte-check`, frontend tests, gitleaks, and dependency license and advisory
  checks.
- **Performance:**
  - micro-benchmarks for the sampling tick;
  - a one-hour soak test of daemon CPU and memory on the reference machine;
  - Vagus's own self-measurement shown in the UI.
- **Security tests:**
  - another user cannot connect to the pipe;
  - every action is rejected unless its tier is satisfied;
  - MCP action tools stay disabled by default;
  - redaction catches the fixture secrets.
- **Localization:** a missing key fails the build, and Turkish casing and sorting tests
  pass.
- **MCP:** contract tests through an `rmcp` client and manual checks with MCP Inspector.

## Implementation and rollout

Phases, deliverables and exit criteria are in the [roadmap](../roadmap.md). In short:

- **Phase 0** measures the risky data sources.
- **Phase 1** ships the daemon, the perf module, diagnosis, notifications, the desktop
  UI, the CLI and read-only MCP.
- **Phase 2** adds privileged data, sensors and gated actions.
- **Phase 3** adds the desktop widget.

Modules for services, programs, repositories and containers follow, one at a time.

## Prior art

| Tool | What we learn from it |
|---|---|
| Task Manager / Resource Monitor | Which counters Windows itself trusts; Resource Monitor's per-process network and disk come from kernel ETW |
| System Informer (MIT) | NT internals, GPU statistics, service mapping, overall data model |
| Rainmeter | Desktop-pinned widgets ("on desktop" layer), skinning model |
| HWiNFO, LibreHardwareMonitor | Sensor coverage, and why kernel drivers are a liability |
| G-Helper (GPL-3.0) | That the ATKACPI interface exposes fans and temperatures; facts only, no code |
| Armoury Crate | What to avoid: many always-on services, slow UI |

# Roadmap

Status on 2026-10-06: Phase 0 is in progress; measurements so far are in
[`docs/research/`](research/). The repository contains the design document, the ADRs
and a minimal `vagus-core` crate that keeps the quality gate honest.

The rule for every phase: ship one thing end to end before widening the scope.

## Phase 0: measure the risky sources

Goal: replace the assumptions in [ADR 0010](adr/0010-data-collection-strategy.md) and
the baseline sources of
[ADR 0018](adr/0018-generic-windows-baseline-vendor-integrations-optional.md) with
numbers before the daemon is written. Phase 0 uses only what Windows provides; vendor
interfaces are validated when their integration is built.

Spikes are throwaway code outside the repository; findings are recorded in
`docs/research/`. What to measure:

- **Processes:** cost of `NtQuerySystemInformation(SystemProcessInformation)` per call
  at the reference machine's process count.
- **PDH:** cost of a collect with the planned counter set (English names), and how
  `% Performance Limit` behaves under load and heat.
- **GPU:** cost of PDH `GPU Engine` counters versus D3DKMT statistics, mapping adapter
  LUIDs to names, and GPU temperature from the display driver's adapter performance
  data.
- **dGPU:** detecting the discrete GPU's power state without waking it, and checking
  that the generic GPU sources do not wake it either.
- **Thermal:** thermal zone temperature, passive limit and throttle reasons. On the
  reference machine, compare the zone with the temperature Armoury Crate displays to
  learn what it measures.
- **Other baseline sources:** `GetIfTable2`, power status and power mode, and
  `EnumServicesStatusExW`, followed by a prototype tick that runs every chosen source
  together.
- **ETW:** overhead of a kernel ETW session for per-process network and disk (admin).
- **WebView2:** cold and warm window open time, and memory, on the reference machine.

Exit criteria:

- A table of every source with its cost, privilege requirement and reliability.
- ADR 0010 accepted or superseded, and the baseline sources in ADR 0018 confirmed or
  replaced.

## Phase 1: core and the first module, end to end

- **`vagus-core`:** capability registry, `Module` trait, finding model, ring buffer
  (exists).
- **`platform-windows`:** safe wrappers for the sources chosen in Phase 0.
- **`vagus-daemon`:**
  - scheduler and flight recorder;
  - API server (named pipe, JSON-RPC 2.0);
  - config store (schemas, hot reload, audit);
  - logging (`tracing`, redaction);
  - rules and diagnosis v1;
  - toast notifications.
- **perf module:**
  - processes;
  - CPU, including throttling and power plan;
  - baseline thermal signals: thermal zone, passive limit, throttle reasons, and GPU
    temperature if Phase 0 confirms it;
  - memory and disk;
  - GPU per process with its adapter;
  - network totals;
  - mapping svchost instances to the services they host.
- **CLI:** `vagus status`, `vagus top`, `vagus diagnose`, `vagus logs`.
- **`vagus mcp`:** read-only tools, resources and prompts.
- **Desktop UI:**
  - views: overview, processes, details, logs, settings;
  - themes: system, light or dark, with accent and density;
  - languages: English and Turkish;
  - global hotkey and tray icon.
- **Tooling:**
  - Windows CI;
  - `cargo-deny` and a JavaScript license check;
  - frontend lint, typecheck and tests;
  - root scripts that run the whole gate.

Exit criteria:

- The performance budgets in the design document are met.
- A week of daily use on the reference machine.
- Diagnosis correctly explains at least four situations:
  - CPU saturation caused by a process;
  - memory pressure (paging);
  - disk saturation;
  - heat caused by throttling or the power mode.

## Phase 2: privileged data, sensors and actions

- **`vagus-sensor` service:** per-process network and disk from kernel ETW, plus the
  privileged sensors.
- **First vendor integrations** (optional modules, ADR 0018): ASUS ATKACPI for
  temperatures and fans, and NVML gated on the dGPU being awake.
- **Actions:**
  - end process (`user` tier);
  - stop and start a service (`admin` tier, through the elevated helper);
  - an audit log view.
- **MCP:** action tools, opt-in one by one.
- **Installer:** daemon autostart, service installation, and the AppUserModelID needed
  for toasts.

Exit criteria:

- A written threat model for the privileged surface.
- Pipe ACL and privilege tier tests pass.

## Phase 3: desktop widget

A second client of the same API: a compact panel with two modes, toggled by a hotkey.

- **On desktop:** pinned under normal windows and unaffected by Win+D.
- **Overlay:** always on top and click-through.

Themes are shared with the main window.

Exit criteria:

- The widget's idle cost is within budget.
- It behaves correctly across Win+D, display changes and DPI changes.

## Later: one module at a time

The order is flexible.

- Services manager: inventory, startup type, dependencies.
- Installed programs and startup items.
- Git repositories: status across configured roots.
- Docker: containers, images, volumes and their resource use.
- Scheduled tasks, drivers, and Event Log insights.
- More vendor integrations: other laptop makers and GPU vendors, one at a time.
- Persistent, downsampled history.
- Baseline anomaly detection.
- An in-app AI view: opt-in, with a local-model option, built as an MCP client of
  Vagus.
- A Linux platform layer.
- A plugin system ([ADR 0017](adr/0017-compiled-in-modules-before-plugins.md)
  revisited).

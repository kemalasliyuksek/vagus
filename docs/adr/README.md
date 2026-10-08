# Architecture Decision Records

Why Vagus is built the way it is. See [ADR 0001](0001-record-architecture-decisions.md)
for how ADRs work in this project. The current system is described in
[the design document](../design/vagus-architecture.md).

| # | Decision | Status |
|---|---|---|
| [0001](0001-record-architecture-decisions.md) | Record architecture decisions | accepted |
| [0002](0002-project-name-vagus.md) | Project name: Vagus | accepted |
| [0003](0003-personal-ownership-and-dual-license.md) | Personal ownership and dual MIT / Apache-2.0 license | accepted |
| [0004](0004-english-only-repository.md) | English-only repository, localized user interface | accepted |
| [0005](0005-rust-for-the-core-daemon.md) | Rust for the core daemon | accepted |
| [0006](0006-tauri-and-svelte-for-the-desktop-ui.md) | Tauri 2 and Svelte 5 for the desktop UI | accepted |
| [0007](0007-api-first-capability-registry.md) | API-first: one daemon, one capability registry, many clients | accepted |
| [0008](0008-mcp-as-a-first-class-interface.md) | MCP as a first-class interface | accepted |
| [0009](0009-privilege-model.md) | Privilege model: privileged reads only, per-action elevation | accepted |
| [0010](0010-data-collection-strategy.md) | Data collection strategy: poll metrics, subscribe to inventory | superseded by 0019 |
| [0011](0011-in-memory-flight-recorder.md) | In-memory flight recorder | accepted |
| [0012](0012-hardware-sensor-sources.md) | Hardware sensor sources without vulnerable drivers | superseded by 0018 |
| [0013](0013-logging-and-no-telemetry.md) | Structured local logging, audit log, no telemetry | accepted |
| [0014](0014-localization.md) | Localization: English and Turkish, one catalog, typed messages | accepted |
| [0015](0015-theming-and-customization.md) | Theming and customization through design tokens | accepted |
| [0016](0016-configuration-as-validated-data.md) | Configuration as schema-validated data | accepted |
| [0017](0017-compiled-in-modules-before-plugins.md) | Compiled-in modules before a plugin system | accepted |
| [0018](0018-generic-windows-baseline-vendor-integrations-optional.md) | Generic Windows baseline, vendor integrations as optional modules | accepted |
| [0019](0019-data-collection-sources-from-phase-0.md) | Data collection sources and cadences, from Phase 0 measurements | accepted |

New ADR: copy the structure of an existing one, take the next number, and add it to
this table. To change a decision, write a new ADR that supersedes the old one, and
update the old one's status.

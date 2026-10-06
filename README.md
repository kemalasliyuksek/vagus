# Vagus

A fast, local-first control center for your Windows machine. Vagus watches everything
that runs on it, explains what is going on, and lets you act on it, from one app or
from AI tools over MCP.

> **Status: pre-alpha, design phase.** There is nothing to install yet. See the
> [roadmap](docs/roadmap.md).

## Why

What a computer is doing is scattered across many tools:

- Task Manager for processes;
- the Services console for services;
- Settings for installed programs;
- vendor utilities for temperatures and fans;
- Docker Desktop for containers;
- a terminal for git repositories.

They show numbers, none of them explains anything, the vendor utilities are slow, and
none of them can be asked a question.

Vagus brings all of this into one place. A small daemon watches the whole machine at
almost no cost and keeps the recent past in memory. Every client sees the same picture
through one API: the desktop app, a CLI, an MCP server for AI tools, and later a
desktop widget.

## Where it starts

None of this exists yet; the [roadmap](docs/roadmap.md) tracks progress.

The first module answers the most urgent question: **why is the machine slow right
now, or a minute ago?**

- A one-sentence diagnosis, for example "CPU is idle but at 83 °C: power mode is Silent
  and the fans are slow", with the evidence one click away.
- A 15-minute in-memory flight recorder, so spikes that already ended are still visible.
- Per-process and per-service use of CPU, memory, disk, GPU and network, including
  which GPU each process runs on.
- Laptop sensors (temperatures, fans, throttling, power mode) without vulnerable kernel
  drivers.

It ships with a calm, customizable desktop app (light and dark themes, English and
Turkish) and an MCP server, so AI tools can inspect the machine with the same
capabilities as the app.

## Where it goes

Modules are added one at a time. Each one appears in the app, the CLI and MCP at once.

- Services: state, startup type, dependencies, start and stop.
- Installed programs and startup items.
- Git repositories across your projects.
- Docker containers, images and volumes.
- Scheduled tasks, drivers and Event Log insights.
- A desktop widget.
- An optional in-app AI view that works through Vagus's own MCP server.

## Principles

- **Performance first:** the daemon targets under 0.5 % CPU and under 30 MB of memory,
  and shows its own cost.
- **Local-first:** no telemetry, and no network connections unless you configure them.
- **Observe first, act on request:** Vagus watches continuously but changes nothing on
  its own. Every action is explicit and logged.
- **Least privilege:** the daemon never runs elevated, and every admin action asks for
  your consent through UAC, including actions requested by AI tools.
- **Causes, not numbers:** details are always available, but the verdict comes first.
- **One API for everything:** whatever the app can do, the CLI and MCP can do too.

## How it works

```
OS, sensors, services, Docker, git --> vagus-daemon (watch, record, explain, act)
                                            |  JSON-RPC over a per-user named pipe
                     desktop app   |   vagus CLI   |   vagus mcp --> AI tools
```

Read the [architecture](docs/design/vagus-architecture.md) and the
[architecture decision records](docs/adr/README.md) for the details and the reasoning.

## How Vagus is built

Vagus is designed and maintained by Kemal Aslıyüksek. Most of the code is written with
Claude Code, an AI coding agent, working from the design and the decisions in
[`docs/`](docs/). Kemal sets the direction, makes the decisions and reviews every
change. Changes must pass the quality gate, and behavior changes come with tests.

The [ADRs](docs/adr/README.md) explain why things are the way they are.
[AGENTS.md](AGENTS.md) holds the rules that agents and human contributors follow.

## Development

Prerequisites:

- Windows 11.
- Rust stable through rustup. The toolchain is pinned in `rust-toolchain.toml`.
- Visual Studio Build Tools with the C++ workload.
- [gitleaks](https://github.com/gitleaks/gitleaks), used by the pre-commit hook.
- From Phase 1: Node.js 24+ and pnpm. WebView2 ships with Windows 11.

Set up a clone:

```
git clone https://github.com/kemalasliyuksek/vagus.git
cd vagus
git config core.hooksPath .githooks
```

Run the quality gate:

```
cargo fmt --all --check
cargo lint
cargo typecheck
cargo test --workspace
```

Contributors, human or AI, should read [AGENTS.md](AGENTS.md) first.

## The name

The vagus nerve connects the body's organs to the brain. Roughly four fifths of its
fibers are sensory: it mostly *reports*, and rarely *commands*. Vagus does the same
for your machine: it brings every part of it into one place, observes continuously,
and acts only when you ask.

## Security

See [SECURITY.md](SECURITY.md) for how to report vulnerabilities.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE)), or
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for
inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual
licensed as above, without any additional terms or conditions.

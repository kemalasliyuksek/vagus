# Vagus

A fast, local-first system monitor for Windows that tells you *why* your computer is
slow.

> **Status: pre-alpha, design phase.** There is nothing to install yet. See the
> [roadmap](docs/roadmap.md).

## Why

When a PC slows down, today's tools show you numbers: hundreds of processes, dozens of
graphs, and a vendor utility that is itself slow. By the time you find the right
window, the spike is often gone.

Vagus is built to answer one question quickly: **what is wrong right now, or what was
wrong a minute ago, and what caused it?**

## Planned

None of this exists yet; the [roadmap](docs/roadmap.md) tracks progress.

- A one-sentence diagnosis of what is slowing the machine down, for example "CPU is
  idle but at 83 °C: power mode is Silent and the fans are slow", with the evidence one
  click away.
- A 15-minute in-memory flight recorder, so spikes that already ended are still visible.
- Per-process and per-service use of CPU, memory, disk, GPU and network, including
  which GPU each process runs on.
- Laptop sensors (temperatures, fans, throttling, power mode) without vulnerable kernel
  drivers.
- A calm, customizable UI with light and dark themes, in English and Turkish.
- An MCP server, so AI tools can inspect the machine with the same capabilities as the
  UI.

## Principles

- **Performance first:** the daemon targets under 0.5 % CPU and under 30 MB of memory,
  and shows its own cost.
- **Local-first:** no telemetry, and no network connections unless you configure them.
- **Least privilege:** the daemon never runs elevated, and every admin action asks for
  your consent through UAC.
- **Causes, not numbers:** details are always available, but the verdict comes first.
- **One API for everything:** whatever the UI can do, the CLI and MCP can do too.

## How it works

```
OS APIs and sensors --> vagus-daemon (collect, record, diagnose, notify)
                             |  JSON-RPC over a per-user named pipe
            desktop UI   |   vagus CLI   |   vagus mcp --> AI tools
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

The vagus nerve carries signals from the body's organs to the brain. Roughly four
fifths of its fibers are sensory: it mostly *reports*, and rarely *commands*. Vagus
works the same way. It observes continuously and acts only when you ask.

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

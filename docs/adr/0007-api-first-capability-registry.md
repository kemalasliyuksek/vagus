# 0007. API-first: one daemon, one capability registry, many clients

- Status: accepted
- Date: 2026-10-06
- Deciders: Kemal Aslıyüksek (owner), proposed by Claude Code

## Context and problem

Vagus will have several clients:

- the desktop window;
- the MCP bridge, so AI tools can observe and change things;
- a CLI;
- later, a desktop widget.

New capabilities (services, installed programs, git repositories, Docker, and more)
must appear in all of them without per-client work. The UI should not touch the
operating system directly, and an AI client should see exactly what the user sees.

## Considered options

- A monolith: the UI process collects data and acts on the OS itself.
- A daemon with hand-written APIs per client.
- A daemon with a single schema-described capability registry and one generic API.

## Decision

The **daemon owns all collection, state and actions**. Each module registers its
capabilities in a central registry, and every capability carries a JSON Schema
generated from its Rust type with `schemars`. There are four kinds of capability:

- **Metric**: a numeric time series with an id, a unit and labels.
- **Entity collection**: inventory items such as processes, services or containers.
- **Action**: a named operation with an input schema and a privilege tier
  ([ADR 0009](0009-privilege-model.md)).
- **Event**: something that happened, such as a service stopping or a threshold being
  crossed.

The daemon exposes **one API: JSON-RPC 2.0 over a per-user Windows named pipe**. The
desktop UI, `vagus mcp`, the CLI and the future widget are all clients of this API.
TypeScript types for the UI are generated from the Rust types; the generator is an
open question in the design doc.

The rule: **anything the UI can do, the API can do.**

Process model:

- `vagus-daemon`: runs as the user and starts at login.
- The desktop UI process.
- `vagus` CLI, which also provides the `vagus mcp` bridge.
- From Phase 2: a privileged read-only sensor service and an on-demand elevated helper
  ([ADR 0009](0009-privilege-model.md)).

## Consequences

- Positive: a new module shows up in the UI, MCP and CLI without per-client wiring.
- Positive: the UI is replaceable (the widget is just another client), and behavior is
  testable through the API.
- Negative: the registry and protocol need careful design up front, and the protocol
  needs versioning (a handshake with `protocol_version`).
- Negative: IPC adds serialization cost. JSON is the starting point; a binary encoding
  is introduced only if measurements show JSON is a bottleneck.

## Why the other options were rejected

- Monolith: the UI would have to run all the time (and partly elevated). AI clients
  would not get parity with the UI.
- Per-client APIs: they drift apart, and every feature costs three implementations.

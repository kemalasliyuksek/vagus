# 0017. Compiled-in modules before a plugin system

- Status: accepted
- Date: 2026-10-06
- Deciders: Kemal Aslıyüksek (owner), proposed by Claude Code

## Context and problem

Vagus is meant to grow module by module: performance, sensors, services, installed
programs, git repositories, Docker and more. A plugin system sounds like the natural
way to allow that. But designing a plugin interface before several real modules exist
means guessing its shape, and guesses get frozen into a public contract.

## Considered options

- Native dynamic libraries (DLL plugins).
- WebAssembly components from day one.
- Out-of-process modules that speak the daemon protocol.
- Modules as workspace crates compiled into the daemon.

## Decision

Modules are **Rust crates in the workspace, compiled into the daemon**. Each module
implements the core `Module` trait and registers its capabilities
([ADR 0007](0007-api-first-capability-registry.md)). Optional modules may sit behind
Cargo features.

Dynamic loading is reconsidered only after **at least three real modules** exist and
there is a concrete need for third-party modules. At that point the candidates are:

- WebAssembly components (for example with `wasmtime`), for sandboxing;
- out-of-process modules speaking the same JSON-RPC protocol, so they can be written in
  any language.

## Consequences

- Positive: the module trait evolves freely while the project learns what modules need.
  Modules get the full Rust toolchain, with no ABI or sandbox overhead.
- Negative: third parties cannot ship modules without a fork or a pull request, for
  now.

## Why the other options were rejected

- DLL plugins: Rust has no stable ABI, a crashing plugin takes down the daemon, and
  loading native code is a security risk.
- WebAssembly from day one: designing the host interface before knowing what modules
  need, and adding a runtime before it pays for itself.
- Out-of-process modules from day one: extra processes and protocol work for modules
  that are all written by the project anyway.

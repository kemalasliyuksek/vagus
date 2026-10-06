# 0005. Rust for the core daemon

- Status: accepted
- Date: 2026-10-06
- Deciders: Kemal Aslıyüksek (owner), proposed by Claude Code

## Context and problem

The daemon runs all the time, so its own cost is part of what the user experiences. It
needs deep Win32 access (NT native APIs, PDH, ETW, DeviceIoControl, service control),
predictable latency without collection pauses, instant start, and a small footprint.
Later it gains a privileged component, where memory safety matters. The code is
written mostly by AI agents, so compile-time checking is especially valuable. The
owner's existing knowledge is explicitly not a selection criterion.

## Considered options

- C# / .NET (with LibreHardwareMonitorLib)
- Rust
- C++
- Go

## Decision

**Rust**, stable channel, edition 2024, organized as a Cargo workspace.

- The `windows` crate for Win32 and WinRT APIs.
- `tokio` for I/O (IPC, file and service watchers, Docker events).
- Sampling runs on a dedicated thread, so sampling intervals do not jitter with I/O
  load. This is the initial plan; Phase 0 measurements confirm or revise it.
- `unsafe` code is confined to platform crates, and every `unsafe` block carries a
  `// SAFETY:` comment (enforced by `clippy::undocumented_unsafe_blocks`).

## Consequences

- Positive: a single native binary, small memory use, no GC or JIT, and memory safety
  for the privileged parts.
- Positive: strict compiler and Clippy checks catch many agent mistakes before runtime.
- Negative: longer compile times than Go or C#.
- Negative: no ready-made hardware sensor library comparable to LibreHardwareMonitor;
  sensor access is written per vendor ([ADR 0012](0012-hardware-sensor-sources.md)).
- Negative: the FFI boundary needs discipline and review.

## Why the other options were rejected

- C# / .NET: LibreHardwareMonitorLib is attractive, but WPF cannot be compiled with
  Native AOT, and the runtime and JIT add memory and startup cost to an always-on
  process. Its sensor coverage also depends on a kernel driver we choose not to ship.
- Go: well suited to daemons, but its Win32, COM and ETW bindings are less complete,
  and its garbage-collected runtime is overhead this process does not need.
- C++: full control, but no memory safety for a privileged component, and slower,
  riskier iteration for agent-written code.

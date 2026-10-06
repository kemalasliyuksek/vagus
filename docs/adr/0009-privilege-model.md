# 0009. Privilege model: privileged reads only, per-action elevation

- Status: accepted
- Date: 2026-10-06
- Deciders: Kemal Aslıyüksek

## Context and problem

Some data needs administrator rights, for example kernel ETW sessions for per-process
network and disk I/O, and possibly the ASUS ATKACPI interface. Some actions need them
too, such as stopping a service. A privileged process that accepts commands from less
privileged processes is a classic local privilege escalation pattern, and it has
appeared again and again in hardware vendors' utilities. Any process running as the
user could reach such a service and borrow its rights.

## Considered options

- Run everything elevated.
- A privileged service that collects data and also executes commands.
- A split design: a minimal privileged service for reads only, plus per-action
  elevation.

## Decision

1. **`vagus-daemon` runs as the user, never elevated.** It owns the API, the registry,
   the recorder and notifications.
2. **From Phase 2, `vagus-sensor` (a Windows service) provides privileged data
   only.** It offers a fixed set of read-only queries and has no actions. It accepts no
   arbitrary paths or arguments and is kept as small as possible.
3. **Actions are tiered.** Each action declares its tier in the registry:
   - `read`: no side effects.
   - `user`: performed with the user's own rights, for example ending one of the
     user's processes or stopping a Docker container.
   - `admin`: performed by a short-lived elevated helper that is started with a UAC
     prompt for each invocation, executes exactly one validated action, and exits.
4. **Every action invocation is audit-logged**, whether it succeeds or fails, with the
   client that asked (UI, MCP or CLI), the redacted parameters and the result.
5. **Pipes are hardened:**
   - pipe names are per user;
   - `PIPE_REJECT_REMOTE_CLIENTS` is set;
   - each pipe has an explicit DACL: the user's SID for the daemon pipe, and SYSTEM,
     Administrators and the daemon's user for the sensor service.
6. **No vulnerable or generic hardware-access drivers**
   ([ADR 0012](0012-hardware-sensor-sources.md)).

## Consequences

- Positive: compromising the user-level daemon gives no admin rights. Admin actions are
  always visible to the user as a UAC prompt, including actions requested over MCP.
- Negative: admin actions cost a UAC prompt each time. This friction is intentional.
- Negative: more processes to build, install and test.
- Negative: an unsigned helper shows "Unknown publisher" in the UAC prompt. Code signing
  options for open source projects have to be evaluated before the first release.

## Why the other options were rejected

- Everything elevated: the UI, parsers and network-facing code would run as admin, and
  starting an elevated app at login needs Task Scheduler workarounds.
- A command-executing privileged service: this is exactly the escalation pattern
  described above.

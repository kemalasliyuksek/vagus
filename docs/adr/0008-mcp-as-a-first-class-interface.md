# 0008. MCP as a first-class interface

- Status: accepted
- Date: 2026-10-06
- Deciders: Kemal Aslıyüksek (owner), proposed by Claude Code

## Context and problem

The owner wants AI tools to be able to observe Vagus and, within limits, change it.
The Model Context Protocol (MCP) is the common way for AI clients such as Claude Code
and Claude Desktop to reach local tools. Two risks come with this. First, an AI client
that can act is a new path to the machine. Second, much of the collected data contains
third-party text (log lines, command lines, container names, repository files) that
could carry prompt-injection attempts.

## Considered options

- A built-in AI assistant that calls a cloud model directly.
- A custom plugin API for AI integrations.
- An MCP server generated from the capability registry.

## Decision

`vagus mcp` serves MCP over stdio using `rmcp` (the official Rust MCP SDK) and bridges
to the daemon's API ([ADR 0007](0007-api-first-capability-registry.md)). Adding it to
Claude Code is one command: `claude mcp add vagus -- vagus mcp`.

- **Generated surface:** MCP tools, resources and prompts are generated from the
  registry, so they stay in sync with the UI automatically.
- **Read tools are on by default.** Action tools are **off by default** and enabled one
  by one in the configuration.
- **Admin actions always need local consent:** they go through the per-action UAC
  elevation of [ADR 0009](0009-privilege-model.md), whoever asks for them.
- **Secrets never leave the daemon:** they are redacted before data reaches any client
  ([ADR 0013](0013-logging-and-no-telemetry.md)).
- **Third-party text is returned as data**, in clearly delimited fields, and never
  placed into tool names or descriptions.
- **No network listener by default.** An optional Streamable HTTP transport may come
  later; it would be off by default, bound to localhost only, and require a token.

## Consequences

- Positive: any MCP client (Claude Code, Claude Desktop, local models) can diagnose the
  machine, without an AI provider built into the core. By default Vagus makes no
  network connections.
- Negative: tool names and descriptions become a public contract that agents depend
  on, so changing them requires care and versioning.
- Negative: prompt injection through collected data is a standing risk. It is mitigated
  by delimiting data, by having action tools off by default, and by requiring consent
  for admin actions, but it is never fully eliminated.

## Why the other options were rejected

- Built-in cloud assistant: it would send machine data to a provider by default, add a
  network dependency, and lock the project to one vendor. An in-app assistant may come
  later, opt-in and with a local-model option, as an MCP client of Vagus itself.
- Custom AI plugin API: it would reinvent MCP, and no existing client would speak it.

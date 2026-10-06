# 0013. Structured local logging, audit log, no telemetry

- Status: accepted
- Date: 2026-10-06
- Deciders: Kemal Aslıyüksek

## Context and problem

Errors and important events must be traceable by the owner and by AI agents debugging
the system. At the same time, the collected data is sensitive. Process command lines
can contain tokens and passwords, and paths and repository names reveal what the user
works on. A monitoring tool that phones home would contradict the whole project.

## Considered options

- Plain text log files.
- Windows Event Log as the primary log.
- Structured logging to local files.
- A hosted error and telemetry service.

## Decision

- **`tracing`** everywhere in Rust. Files are JSON Lines under
  `%LOCALAPPDATA%\Vagus\logs\`, rotate daily, and are kept for 14 days by default
  (configurable).
- **A separate audit log** records every action invocation and every configuration
  change ([ADR 0009](0009-privilege-model.md),
  [ADR 0016](0016-configuration-as-validated-data.md)).
- **A redaction layer** runs before anything is written or sent to a client. It masks
  known secret patterns, credential-like command-line arguments (`--password`,
  `--token`, `KEY=value` pairs with secret-like keys) and tokens embedded in URLs.
  Redaction is best-effort and covered by fixture tests.
- **UI errors** (`window.onerror`, unhandled promise rejections) are forwarded to the
  daemon and land in the same log. A panic hook writes panics to the log before the
  process exits.
- **The log filter can change at runtime** through the API. `VAGUS_LOG` sets the
  startup default.
- **Logs are visible** in a UI log viewer and as an MCP resource, both redacted, so an
  agent can read recent errors and temporarily raise verbosity while debugging.
- **No telemetry**: no crash upload and no usage statistics. Update checks are a
  separate, future, opt-in decision.

## Consequences

- Positive: humans and agents can debug from the same records, and nothing leaves the
  machine.
- Negative: redaction can miss novel secret formats, so it needs fixture tests that
  grow over time.
- Negative: without telemetry the project learns about crashes only from user reports.

## Why the other options were rejected

- Plain text: hard to filter and parse for tools and agents.
- Windows Event Log as primary: awkward for high volume and structured fields. It may
  still be used for a few critical service events later.
- Hosted error and telemetry service: sends sensitive data off the machine, which
  contradicts the local-first principle.

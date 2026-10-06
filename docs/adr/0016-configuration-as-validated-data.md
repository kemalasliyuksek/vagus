# 0016. Configuration as schema-validated data

- Status: accepted
- Date: 2026-10-06
- Deciders: Kemal Aslıyüksek (owner), proposed by Claude Code

## Context and problem

The owner wants the program to be dynamic: its behavior (settings, themes, layouts,
notification rules, module options) should be editable by the user in the UI, by hand,
and by AI agents, without recompiling. Changes from any of these sources must be
validated the same way, and must be traceable.

## Considered options

- Windows registry.
- SQLite database for settings.
- TOML files.
- JSON files with JSON Schema.

## Decision

- **JSON files with a `$schema` reference** under `%APPDATA%\Vagus\`:
  - `settings.json`;
  - `themes/*.json`;
  - `layouts/*.json`;
  - `rules/*.json`;
  - per-module settings.
- **Schemas are generated from Rust types** with `schemars` and shipped with the app, so
  editors and agents get validation and completion.
- **Hot reload:** a file watcher reloads changed files. If a file is invalid, the last
  valid version stays active, and the error is logged and shown in the UI.
- **Writes through the API** (from the UI, the CLI or MCP) are validated, written
  atomically (temporary file, then rename), and recorded in the audit log
  ([ADR 0013](0013-logging-and-no-telemetry.md)).
- **Every file carries a `version` field**, and the daemon migrates old versions forward.

## Consequences

- Positive: everything is scriptable, diffable and portable, and agents edit the same
  validated data as the UI.
- Negative: schema evolution needs migrations and tests.
- Negative: hand-edited JSON cannot hold comments. The schema descriptions carry the
  documentation instead.

## Why the other options were rejected

- Registry: opaque, awkward for humans and agents, and not portable.
- SQLite: not hand-editable or diffable.
- TOML: nicer to edit by hand, but JSON Schema tooling for JSON is stronger, and
  comments would be lost on the first UI write anyway.

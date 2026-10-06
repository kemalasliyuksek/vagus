# Security policy

Vagus is pre-alpha and has no releases yet. Because it inspects the whole machine and
will include a privileged component, security reports are welcome from day one.

## Reporting a vulnerability

Report vulnerabilities privately through GitHub's private vulnerability reporting
("Report a vulnerability" in the repository's Security tab). Please do not open public
issues for security problems.

## Design commitments

These are the guarantees the design is built around. A violation of any of them is a
security bug.

- **No telemetry.** Vagus makes no network connections unless the user configures one.
- **No elevated daemon.** The always-on daemon runs with the user's rights.
- **Read-only privileged service.** The privileged component (planned for Phase 2)
  answers a fixed set of read-only queries and executes no commands.
- **Consent for admin actions.** Every admin-level action requires a UAC prompt,
  including actions requested by AI tools over MCP.
- **No vulnerable drivers.** No generic hardware-access drivers (such as WinRing0) are
  shipped or loaded.
- **Secrets stay put.** Secrets found in collected data are redacted before they are
  logged or returned to any client.
- **Local-only pipes.** Named pipes reject remote clients and are restricted to their
  intended users.

See [ADR 0009](docs/adr/0009-privilege-model.md) and
[ADR 0013](docs/adr/0013-logging-and-no-telemetry.md).

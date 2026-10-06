@AGENTS.md

# Claude Code notes

These notes add to `AGENTS.md` (imported above) and to the owner's global instructions.

## Overrides of the owner's global defaults

The owner's global instructions default to Turkish code comments and Turkish commit
messages (`/commit-tr`). This project explicitly overrides both
([ADR 0004](docs/adr/0004-english-only-repository.md)):

- Code, identifiers, comments, docs, ADRs, commit messages and PR text are in English.
- Commit with `/commit-en`.
- Conversation with the owner stays in Turkish.

## Project identity

- This is a personal open source project of Kemal Aslıyüksek.
- Do not add company or brand names, logos or alternative git identities. The global
  personal git identity is correct here.
- Planned remote: `github.com/kemalasliyuksek/vagus`. It has not been created yet;
  creating it and pushing need the owner's approval.
- License copyright line: `Copyright (c) 2026 Kemal Aslıyüksek`
  ([ADR 0003](docs/adr/0003-personal-ownership-and-dual-license.md)).

## Working conventions

- **Session continuity:** `.claude/session-state.md` (git-ignored), maintained through
  `/session-start` and `/session-end`.
- **Phase 0 spikes** are throwaway code in the session scratchpad. Only their findings
  go into `docs/research/`.
- **Reference machine:** `docs/research/2026-10-06-reference-machine-baseline.md`.
  Sensor work targets this hardware first.
- **Verification:** run the quality gate from `AGENTS.md` before claiming anything
  works. `/check` runs the same steps.

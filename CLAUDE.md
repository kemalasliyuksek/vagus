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
- Remote: `github.com/kemalasliyuksek/vagus`. Pushing needs the owner's approval.
- License copyright line: `Copyright (c) 2026 Kemal Aslıyüksek`
  ([ADR 0003](docs/adr/0003-personal-ownership-and-dual-license.md)).

## Working conventions

- **Session continuity:** `.claude/session-state.md` (git-ignored), maintained through
  `/session-start` and `/session-end`.
- **Phase 0 spikes** are throwaway code in `C:\Projects\vagus-spikes\`, outside the
  repository, so they survive across sessions. Only their findings go into
  `docs/research/`.
- **Reference machine:** `docs/research/2026-10-06-reference-machine-baseline.md`.
  Sensor work targets this hardware first.
- **Verification:** run the quality gate from `AGENTS.md` before claiming anything
  works. `/check` runs the same steps.
- **Review before commit:** after a change, summarize what changed, why and how it was
  verified, so Kemal can review the diff. Commit only when he asks. The bar is the
  "Quality bar" section of `AGENTS.md`.

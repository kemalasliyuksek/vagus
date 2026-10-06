# 0004. English-only repository, localized user interface

- Status: accepted
- Date: 2026-10-06
- Deciders: Kemal Aslıyüksek

## Context and problem

Vagus is open source and aims at a global audience. The owner's personal defaults for
other projects use Turkish code comments and Turkish commit messages. A mixed-language
repository is harder for contributors and AI agents to work with.

## Considered options

- Turkish comments and commits, English identifiers (the owner's global default).
- Everything in the repository in English; the user interface localized.

## Decision

Everything in the repository is written in English: identifiers, code comments,
documentation, ADRs, commit messages (Conventional Commits), issue and PR text.

The user interface is localized separately ([ADR 0014](0014-localization.md)). At
launch it supports English and Turkish. Turkish appears in the repository only in the
Turkish message catalog and in tests that cover Turkish-specific behavior.

The owner may talk to agents in any language. That does not change the language of
what is committed.

## Consequences

- Positive: one language for contributors, reviewers and agents.
- Negative: the project's `CLAUDE.md` has to override the owner's global defaults
  explicitly, otherwise agents fall back to Turkish comments and commit messages.

## Why the other options were rejected

- Turkish comments and commits would shut out most potential contributors of an open
  source project.

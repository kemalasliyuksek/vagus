# 0001. Record architecture decisions

- Status: accepted
- Date: 2026-10-06
- Deciders: Kemal Aslıyüksek

## Context and problem

Vagus is open source, and most of its code is written with AI coding agents (Claude
Code) under the maintainer's direction and review. Agents and contributors start every
session without memory of earlier discussions. Without a written record, settled decisions get
re-argued or silently reversed.

## Considered options

- No formal record: rely on commit messages and chat history.
- A single, long design document.
- Architecture Decision Records (ADRs) next to a living design document.

## Decision

Record significant decisions as ADRs in `docs/adr/`, named `NNNN-kebab-case-title.md`.
The living design document in `docs/design/` describes the current system and links to
the ADRs for the reasoning.

An accepted ADR is not edited in substance. When a decision changes, a new ADR
supersedes it and the old ADR's status points to the new one. ADRs marked `proposed`
are waiting for evidence (usually Phase 0 measurements) and are accepted or replaced
once that evidence exists.

## Consequences

- Positive: agents can load the reasoning behind a constraint before changing it, and
  reviewers can check a change against recorded intent.
- Negative: every significant change costs the time to write an ADR.

## Why the other options were rejected

- Commit messages are hard to discover, and chat history is not part of the repository.
- A single document mixes "what is" with "why", and loses the history of reversed
  decisions.

# 0002. Project name: Vagus

- Status: accepted
- Date: 2026-10-06
- Deciders: Kemal Aslıyüksek (owner)

## Context and problem

The project needs a name that is short enough to type as a CLI command, distinctive
enough to search for, and that carries the idea of a system sensing its own internal
state.

## Considered options

- **Vagus**: the vagus nerve carries signals from the internal organs to the brain.
- **Homeostat**: W. Ross Ashby's 1948 cybernetic machine that keeps its own internal
  balance; Ashby later wrote *Design for a Brain* (1952). Strong story, long command.
- **Lares**: the Roman household guardian spirits (the household one was the *Lar
  Familiaris*). Short and warm, but the meaning is not obvious.
- Rejected early because they are overused or taken in the infrastructure space:
  Sentinel, Argus, Heimdall, Watchtower (a known Docker tool), Cortex (a CNCF metrics
  project), Synapse (the Matrix server), Nerve (Airbnb's service discovery tool),
  Finch (AWS's open source container tool), Maxwell (a Zendesk tool).

## Decision

**Vagus.** Roughly four fifths of the vagus nerve's fibers are sensory (afferent): it
mostly reports the body's state rather than issuing commands. That is the project's
core principle (observe first, act rarely and only with consent; see
[ADR 0009](0009-privilege-model.md)). The term is also familiar in Turkish
("vagus siniri"), the owner's language.

Availability check on 2026-10-06:

- GitHub `kemalasliyuksek/vagus`: free.
- Other GitHub repositories named "vagus": a handful of small, unrelated projects
  (at most 7 stars), including an Android remote access tool and an unrelated MCP
  project.
- crates.io `vagus`: taken by an unrelated CLI (43 downloads).
- npm `vagus`: free.

## Consequences

- Positive: short command (`vagus`), meaningful story, no dominant namesake.
- Negative: crates cannot be published under the bare name. Workspace crates use the
  `vagus-` prefix and `publish = false`. If a crate ever needs publishing, it needs a
  free name first.
- Negative: search results may show the unrelated small namesakes, including one
  remote access tool. Antivirus detection is content-based, so the name alone is not
  expected to cause false positives.

## Why the other options were rejected

- Homeostat: too long for a command people type often.
- Lares: needs an explanation before it means anything to most users.

# 0003. Personal ownership and dual MIT / Apache-2.0 license

- Status: accepted
- Date: 2026-10-06
- Deciders: Kemal Aslıyüksek (owner)

## Context and problem

Vagus is a personal open source project of Kemal Aslıyüksek, hosted at
`github.com/kemalasliyuksek/vagus`. It is not a product of any company or brand. The
license should encourage adoption and contributions and fit the Rust ecosystem the
daemon depends on.

## Considered options

- MIT
- Apache-2.0
- MIT OR Apache-2.0 (dual license, user's choice)
- GPL-3.0
- MPL-2.0

## Decision

**MIT OR Apache-2.0**, with the copyright line `Copyright (c) 2026 Kemal Aslıyüksek`.
The full texts live in `LICENSE-MIT` and `LICENSE-APACHE`.

Contributions are accepted under the same terms (inbound equals outbound), as stated
in the README: unless a contributor says otherwise, a contribution intentionally
submitted for inclusion is dual licensed as above, without additional terms.

## Consequences

- Positive: this is the Rust ecosystem convention, so it is compatible with nearly all
  dependencies. Apache-2.0 adds an explicit patent grant, MIT keeps compatibility with
  GPLv2-only projects.
- Negative: anyone may ship a closed-source fork.
- Negative: GPL code cannot be copied into Vagus. In particular G-Helper (to our
  knowledge GPL-3.0) may be read to learn facts about the ASUS ATKACPI interface, such
  as method and device identifiers, but its code must not be copied or translated.
- Follow-up: dependency licenses are checked automatically (`cargo-deny` and a
  JavaScript equivalent) once dependencies arrive in Phase 1.

## Why the other options were rejected

- GPL-3.0 would keep forks open, but limits embedding and adoption and does not match
  the ecosystem the project builds on.
- MIT alone has no patent grant.
- Apache-2.0 alone is incompatible with GPLv2-only software.
- MPL-2.0 adds file-level copyleft friction without a clear benefit for this project.

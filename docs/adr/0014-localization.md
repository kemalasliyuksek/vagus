# 0014. Localization: English and Turkish, one catalog, typed messages

- Status: accepted
- Date: 2026-10-06
- Deciders: Kemal Aslıyüksek

## Context and problem

The UI launches in English and Turkish, and adding a language later must be easy. Some
user-facing text comes from the daemon, not the UI: toast notifications are sent while
the window may be closed. Turkish has its own casing rules: dotted and dotless i ("I"
lowercases to "ı", and "i" uppercases to "İ"). Locale-insensitive case conversion breaks
them, both in JavaScript and in Rust's `to_lowercase()`.

## Considered options

- i18next (runtime lookup, string keys).
- Project Fluent (`fluent-rs` and `@fluent/bundle`).
- Paraglide JS with a catalog shared with the daemon.
- Hard-coded strings, translated later.

## Decision

- **Paraglide JS** in the UI. Messages compile to typed functions, so a missing or
  misspelled key fails the build instead of showing an empty string.
- **One catalog per language** in `locales/<lang>.json` (`en.json` is the source,
  `tr.json` the first translation). The UI and the daemon read the same catalogs. How
  the daemon consumes them (build-time embedding, placeholder and plural handling) is
  an open question for Phase 1.
- **Adding a language** means adding a catalog file and registering the locale, with no
  code changes.
- **No hard-coded user-facing strings** in UI or daemon code.
- **Formatting:**
  - numbers, units, dates and durations use `Intl` with the active locale;
  - case conversion of user-visible text always passes a locale
    (`toLocaleUpperCase(locale)`);
  - sorting uses `Intl.Collator`;
  - tests cover I, ı, İ and i.

## Consequences

- Positive: compile-time safety for translations and one source of truth for the UI
  and the daemon.
- Negative: Paraglide's message format and the daemon's reader must stay compatible.
  This needs a small amount of custom code and tests.

## Why the other options were rejected

- i18next: untyped string keys mean missing translations only show up at runtime.
- Fluent: excellent plural and grammar support and good Rust support, but weaker typed
  integration with Svelte tooling. It remains the fallback if plural needs outgrow the
  chosen format.
- Hard-coded strings: retrofitting localization later is expensive and error-prone.

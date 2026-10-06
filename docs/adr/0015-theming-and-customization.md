# 0015. Theming and customization through design tokens

- Status: accepted
- Date: 2026-10-06
- Deciders: Kemal Aslıyüksek

## Context and problem

The UI should feel soft, calm and elegant, offer light and dark modes, and let users
customize colors, density and dashboard layout. Customizations should be editable from
the UI, by hand, and by AI agents through MCP, without rebuilding the app.

## Considered options

- CSS-in-JS theming.
- An off-the-shelf component library with a fixed visual language.
- Tailwind utility classes only.
- Design tokens as CSS custom properties, with themes as data.

## Decision

- **Design tokens** (color, radius, spacing, typography, elevation, motion) are CSS
  custom properties. Tailwind 4 is configured to use them, so components never contain
  raw color values.
- **Themes are JSON token sets.** Built-in light and dark themes ship with the app; user
  themes live in the configuration directory and are validated by a schema
  ([ADR 0016](0016-configuration-as-validated-data.md)).
- **Mode** is `system`, `light` or `dark`, and defaults to following Windows.
- **User-adjustable:** accent color and density (`comfortable`, `compact`).
- **Window backdrop:** Mica on Windows 11 where available, with a solid fallback.
- **Dashboard layouts** (panels, order, size) are configuration, editable by
  drag-and-drop in the UI, by hand, or through MCP.
- **Accessibility:** built-in themes meet WCAG 2.1 AA contrast (checked in tests),
  `prefers-reduced-motion` is respected, and everything is keyboard navigable.

## Consequences

- Positive: themes and layouts change at runtime without a rebuild, and agents can
  create or adjust them through the same validated path as users.
- Negative: user themes can be ugly or low-contrast. Contrast is checked and warned
  about, but the user is not blocked.

## Why the other options were rejected

- CSS-in-JS: runtime cost and less natural with Svelte.
- A fixed-look component library: makes the product look like everyone else's and
  fights customization.
- Tailwind only: utility classes are resolved at build time, so runtime user themes
  would be impossible without tokens.

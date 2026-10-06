# 0006. Tauri 2 and Svelte 5 for the desktop UI

- Status: accepted
- Date: 2026-10-06
- Deciders: Kemal Aslıyüksek (owner), proposed by Claude Code

## Context and problem

The UI must be simple, clear and elegant. It needs light and dark themes,
user-defined themes and layouts, localization, dense live charts, and later richer
surfaces such as an AI conversation view. It also must not burden the system. Users
look at it only occasionally; the daemon does the continuous work.

## Considered options

- Native C# UI (WPF or WinUI 3)
- Immediate-mode Rust UI (egui)
- Retained-mode native Rust UI (Slint, iced)
- Electron
- Tauri 2 with React, Solid or Svelte

## Decision

**Tauri 2 + Svelte 5 + TypeScript (strict) + Vite**, with:

- Tailwind CSS 4 on top of CSS custom-property design tokens
  ([ADR 0015](0015-theming-and-customization.md));
- Bits UI as the headless, accessible component layer;
- uPlot for time-series charts;
- Lucide icons;
- Paraglide JS for localization ([ADR 0014](0014-localization.md)).

Why:

- Web technology has the strongest ecosystem for styling, theming, motion and
  localization.
- Tauri uses the system WebView2 instead of bundling Chromium. Its Rust backend shares
  types with the daemon.
- Svelte compiles to fine-grained DOM updates. When hundreds of values change every
  second, only the changed nodes are touched, with no memoization discipline needed.
- uPlot is purpose-built for dense time series and stays cheap at this data rate.

## Consequences

- Positive: fast iteration on look and feel; themes and localization are
  straightforward.
- Negative: WebView2 costs several processes and tens to over a hundred megabytes
  while the window is open. Mitigations:
  - the UI is a client of the daemon, and closing the window frees that memory;
  - nothing continuous runs in the UI;
  - a "keep warm" setting is optional.
- Negative: cold start of the window is slower than a native window, especially under
  load. Mitigation: the flight recorder ([ADR 0011](0011-in-memory-flight-recorder.md))
  means the window shows what happened even if it opens a second late.
- Negative: agents sometimes write Svelte 4 syntax. Svelte 5 runes are mandatory, and
  `svelte-check` runs in the quality gate.

## Why the other options were rejected

- WPF / WinUI 3: splits the stack across C# and Rust. WPF cannot use Native AOT, and
  WinUI 3 packaging adds complexity.
- egui: very fast, but the look is utilitarian, and theming and animation are limited.
- Slint / iced: native and light, but smaller ecosystems for charts, localization and
  rich text, and custom styling takes more work.
- Electron: bundles its own Chromium and Node, which is heavier than Tauri for no gain.
- React: the largest ecosystem, but its re-render model needs constant memoization at
  this update rate, which is easy for agents to get wrong.
- Solid: performance similar to Svelte, with a smaller ecosystem of headless
  components.

# ADR-006: The UI is organized around switchable writing modes

**Status:** Accepted (direction; per-mode quality gate below)  
**Date:** 2026-07-11

## Context

Structural UI mockups (three-pane workshop / chromeless page / continuous
manuscript) were presented 2026-07-11. Owner ruling:

> none of them are special. none of them are good. all of them should be
> options in the app once improved because they serve different writing
> modes.

## Decision

The app's UI is organized as **switchable writing modes**, not one fixed
layout:

- **Draft** — chromeless page: only the text, everything else summoned on
  demand (seed: today's focus mode).
- **Organize** — binder + inspector workshop (seed: today's default
  three-pane layout).
- **Revise** — continuous manuscript scroll with scene landmarks and margin
  annotations (seed: today's flow mode).

Mode names provisional. "Once improved" is the gate: a mode ships only when
the owner, using a real build, accepts it — static mockups are retired as an
approval vehicle for UI work.

## Consequences

- UI work is planned and executed per-mode; the existing layout remains the
  Organize baseline until replaced.
- Modes are views over the same engine state; no format or engine changes
  are implied by this ADR.
- Differentiation should come from surfacing the app's unique organs
  (embedded history, drafts, plain files) inside each mode, not from layout
  novelty.

## Owner corrections (2026-07-11)

- **No visible scratch-outs in the editor.** Cuts vanish normally while
  writing; struck-through text lingering on the page "will encourage saving
  just to have a usable editor." History stays behind the scenes.
- **Edge case noted, not planned:** some writers need cut text truly GONE
  (a prof deleted 100 pages to foreclose salvage). Recorded as a boundary
  the lifecycle design should not foreclose, **not** a feature to build —
  the format is genre-agnostic and most genres would not want a burn bag
  (same reasoning that kept domain-specific labels out of the format).
  Today's answer for that writer: export keepers, delete the project.

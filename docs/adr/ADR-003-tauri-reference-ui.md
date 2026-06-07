# ADR-003: Tauri + React is the reference GUI

**Status:** Accepted  
**Date:** 2026-06-07

## Context

The repo contains Tauri, WinUI, SwiftUI, and Qt frontends at vastly different maturity levels. Chasing parity across all four multiplied work and duplicated the engine (ADR-001).

The owner needs one coherent app to describe in plain English while agents handle implementation.

## Decision

1. **Shipping desktop GUI:** `src-tauri/` + `ui/` (Tauri 2 + React + TipTap).

2. **Target platforms for this GUI:** macOS and Linux for v1; Windows via **Tauri** when added — not via WinUI parity.

3. **New user-facing features** (inspector fields, corkboard, compile dialog, revisions UX, AI menu, etc.) are implemented only in Tauri unless owner issues `AMEND ADR`.

4. `src-tauri` remains a **thin command layer** over `chickenscratch-core` — no format logic duplication.

## Consequences

### Positive

- Single React codebase for writer UX
- Same UI on macOS and Linux (native window chrome only differs)
- Aligns with engine-in-Rust architecture

### Negative

- Does not use OS-native control libraries (WinUI, SwiftUI, Qt)
- Webview-based editor — acceptable; TipTap already chosen

## Compliance

- Feature requests → implement in `ui/` + `src-tauri/src/commands/`
- Do not add parallel XAML/Swift/QML implementations of the same feature
- README platform table should list Tauri as primary; mark WinUI/Swift/Qt as deprecated/experimental
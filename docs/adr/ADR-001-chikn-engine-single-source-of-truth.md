# ADR-001: chikn-engine (Rust) is the single source of truth for `.chikn` I/O

**Status:** Accepted  
**Date:** 2026-06-07  
**Authority:** Owner directive — structure repo for coherence and drift resistance.

## Context

The repository grew multiple **independent implementations** of `.chikn` read/write/git:

- `crates/core` (Rust) — used by Tauri, TUI, converter, Linux bridge
- `macos/Sources/ChiknKit` (Swift)
- `windows/ChickenScratch.Core` (C#)

Each security fix, schema change, or writer bug had to be ported three times. `cross_frontend/run.sh` and extensive review findings document ongoing **drift** and data-loss risk.

The owner’s intent: **one engine, any UI is a customer.**

## Decision

1. **`chickenscratch-core`** (`crates/core/`) is the only place that implements:
   - project/document read and write
   - embedded git operations
   - Scrivener conversion
   - compile orchestration
   - path safety and atomic persistence

2. All new format or git behavior is implemented in `crates/core` first, with tests.

3. UIs call the engine via:
   - **In-process** (Tauri, TUI, CLI) — preferred
   - **FFI / sidecar** — if a future native shell is revived; never a second language rewrite

4. Swift `ChiknKit` and C# `ChickenScratch.Core` are **deprecated** for new work (see ADR-004).

## Consequences

### Positive

- One test suite defines correct behavior
- Owner can request features without multi-language parity projects
- Format can be published independently of any UI

### Negative

- WinUI / SwiftUI “fully native” apps are deprioritized
- Existing deprecated code may bit-rot until removed

## Compliance

Agents must reject tasks that say “add X to WinUI writer” or “implement Y in ChiknKit” unless the owner explicitly says `AMEND ADR` and overrides ADR-001.

Correct pattern: implement in `crates/core`, expose via Tauri command if the app needs it.
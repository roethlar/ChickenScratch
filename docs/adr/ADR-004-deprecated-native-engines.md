# ADR-004: Deprecated native frontends and duplicate engines

**Status:** Accepted  
**Date:** 2026-06-07

## Context

`macos/`, `windows/`, and `linux/` were experiments in per-platform native UI. They introduced **duplicate format implementations** (Swift, C#) and “sync five UIs” phases that conflict with ADR-001 and ADR-003.

## Decision

| Path | Verdict | Agent policy |
|------|---------|--------------|
| `macos/` (SwiftUI + ChiknKit) | **Deprecated** | No new features. No format fixes in Swift — fix Rust engine. |
| `windows/` (WinUI + ChickenScratch.Core) | **Deprecated** | No new features. Not the Windows shipping plan. |
| `linux/` (Qt + cxx-qt) | **Deprecated** | Tauri is the Linux GUI. |

Allowed maintenance on deprecated paths **only** when:

- Owner explicitly requests archival/removal work, or
- A change removes the path from CI/release gates to reduce noise

**Not allowed:** Bringing deprecated UIs to Tauri feature parity.

Older docs referencing “five frontends” or “sync all UIs” are **historical**. [`CURRENT_PHASE.md`](../CURRENT_PHASE.md) and ADR-001 supersede them.

## Consequences

- `cross_frontend/run.sh` may be narrowed or removed in a later phase — not a goal for new work
- Large directories remain until owner approves deletion/archive

## Compliance

Before editing files under `macos/`, `windows/`, or `linux/`, agents must state why the task cannot be done in `crates/core` or Tauri. If it can be done there, do it there instead.
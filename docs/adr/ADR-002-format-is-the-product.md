# ADR-002: The `.chikn` format is the product; apps are reference implementations

**Status:** Accepted  
**Date:** 2026-06-07

## Context

Marketing and architecture discussions revealed confusion: five frontends, unclear shipping target, format virtues buried under UI parity work.

The durable technical differentiator is **Folder-First Documents** as `.chikn` — plain Markdown, YAML sidecars, embedded git — not any particular widget toolkit.

## Decision

1. **Primary deliverable:** documented, stable `.chikn` format + Rust engine that reads/writes it.

2. **ChickenScratch (Tauri app)** is the **reference GUI** for using the format, not the definition of the format.

3. **`chikn-converter`** and **`chikn` TUI** are first-class proof that the format works without the GUI.

4. Spec changes go through `docs/CHIKN_FORMAT_SPEC.md` and engine tests. UI-only concerns do not change on-disk layout without a spec version bump.

5. Genre-specific concepts (novelist POV, etc.) live in **UI convention docs**, not in the format schema (Invariant I4).

## Consequences

### Positive

- Clear story for non-developers: “your files are yours”
- Engine and converter can be shared with other tools later
- Reduces scope creep in the GUI

### Negative

- App must be good enough to be a credible reference, but format can outshine app maturity

## Compliance

- Do not add typed domain fields to `Document` in core — use `fields` map
- Do not ship marketing copy that promises five-platform native parity
- Format docs (`CHIKN_FORMAT_SPEC`, `FOLDER_FIRST_DOCUMENTS`) take precedence over README platform tables when they conflict
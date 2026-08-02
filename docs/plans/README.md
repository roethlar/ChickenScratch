# Plans

Design documents for sizeable future work. Each plan covers motivation, format-spec impact, UX, backend+frontend touchpoints, explicit scope limits, and open questions — enough to start implementing from.

These sit alongside [ROADMAP.md](../ROADMAP.md) (forward-looking feature catalogue) and [DEVLOG.md](../../DEVLOG.md) (history of what shipped). When a plan ships, move its summary into ROADMAP "What's Built" and fold the plan doc into DEVLOG.

## Current phase

**Authoritative:** [`CURRENT_PHASE.md`](../CURRENT_PHASE.md) · [`INVARIANTS.md`](../INVARIANTS.md)

**Engine hardening** is active. The authoritative scope and work order live in
[`CURRENT_PHASE.md`](../CURRENT_PHASE.md); no code slice is approved merely by
advancing the phase.

**Completed predecessor:**
[PHASE_FORMAT_FINALIZATION.md](PHASE_FORMAT_FINALIZATION.md) locked the
`.chikn` format as genre-agnostic (`fields` map per document). **Engine +
Tauri only** — five-frontend sync is deprecated per
[ADR-004](../adr/ADR-004-deprecated-native-engines.md).

**v1.2 — Novelist feature plans** are UI-layer work: the format stays
genre-agnostic and the Tauri novelist UI writes domain-specific data into the
generic `fields` extensibility that format finalization introduced. The
Tauri scope of Tier 1 and Tier 2 shipped before the Engine hardening phase;
unfinished Tier 3 work is a later priority.

- **[TIER1_novel_structure.md](TIER1_novel_structure.md)** — Tauri scope
  shipped:
  scene-level metadata, characters/locations as entities, plot threads.
- **[TIER2_writer_workflow.md](TIER2_writer_workflow.md)** — Tauri scope
  shipped: flow mode, session targets, per-document snapshots, timeline view.
- **[TIER3_polish.md](TIER3_polish.md)** — later priority: collections, rich
  research (inline PDF/image preview), split editor.

Deliberately **not** on the roadmap: name generators, deep character-psychology forms, snowflake-method spreadsheets. These bloat the UI with features that look good in a feature list and then nobody uses.

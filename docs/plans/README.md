# Plans

Design documents for sizeable future work. Each plan covers motivation, format-spec impact, UX, backend+frontend touchpoints, explicit scope limits, and open questions — enough to start implementing from.

These sit alongside [ROADMAP.md](../ROADMAP.md) (forward-looking feature catalogue) and [DEVLOG.md](../../DEVLOG.md) (history of what shipped). When a plan ships, move its summary into ROADMAP "What's Built" and fold the plan doc into DEVLOG.

## Current phase

**Authoritative:** [`CURRENT_PHASE.md`](../CURRENT_PHASE.md) · [`INVARIANTS.md`](../INVARIANTS.md)

**[PHASE_FORMAT_FINALIZATION.md](PHASE_FORMAT_FINALIZATION.md)** — lock the `.chikn` format as genre-agnostic (`fields` map per document). **Engine + Tauri only** — five-frontend sync is deprecated per [ADR-004](../adr/ADR-004-deprecated-native-engines.md).

**v1.2 — Novelist features** *(UI-layer, paused until format finalization ships).* Bring ChickenScratch up to parity with (and past) the best open-source Scrivener alternatives on the features that actually matter for long-form fiction. Sourced from a comparative survey of Scrivener, Manuskript, bibisco, oStorybook, and yWriter. These are **novelist-UI** plans — the format stays genre-agnostic; novelist UIs write their domain-specific data into the generic `fields` extensibility that format finalization introduces.

- **[TIER1_novel_structure.md](TIER1_novel_structure.md)** — scene-level metadata, characters/locations as entities, plot threads.
- **[TIER2_writer_workflow.md](TIER2_writer_workflow.md)** — scrivenings mode, session targets with deadlines, per-document snapshots, timeline view.
- **[TIER3_polish.md](TIER3_polish.md)** — collections, rich research (inline PDF/image preview), split editor.

Deliberately **not** on the roadmap: name generators, deep character-psychology forms, snowflake-method spreadsheets. These bloat the UI with features that look good in a feature list and then nobody uses.

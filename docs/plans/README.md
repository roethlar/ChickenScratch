# Plans

Design documents for sizeable future work. Each plan covers motivation, format-spec impact, UX, backend+frontend touchpoints, explicit scope limits, and open questions — enough to start implementing from.

These sit alongside [ROADMAP.md](../ROADMAP.md) (forward-looking feature catalogue) and [DEVLOG.md](../../DEVLOG.md) (history of what shipped). When a plan ships, move its summary into ROADMAP "What's Built" and fold the plan doc into DEVLOG.

## Active plans

**v1.2 — Novelist features.** Bring ChickenScratch up to parity with (and past) the best open-source Scrivener alternatives on the features that actually matter for long-form fiction. Sourced from a comparative survey of Scrivener, Manuskript, bibisco, oStorybook, and yWriter.

- **[TIER1_novel_structure.md](TIER1_novel_structure.md)** — scene-level structured metadata, characters/locations as first-class entities, plot threads. Highest leverage; unlocks the other two tiers.
- **[TIER2_writer_workflow.md](TIER2_writer_workflow.md)** — scrivenings mode, session targets with deadlines, per-document snapshots, timeline view.
- **[TIER3_polish.md](TIER3_polish.md)** — collections (saved structured queries), rich research (inline PDF/image preview), split editor.

Deliberately **not** on the roadmap: name generators, deep character-psychology forms, snowflake-method spreadsheets. These bloat the UI with features that look good in a feature list and then nobody uses.

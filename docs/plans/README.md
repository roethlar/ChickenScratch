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

## Queued side quests (owner-queued, outside the active phase)

- **[PLAN_WEB_SHELL.md](PLAN_WEB_SHELL.md)** — `chikn-web`: serve the
  existing React UI over HTTP from a shared `chickenscratch-services`
  command layer, for self-hosted browser access over a private VPN.
  Queued 2026-10-23; every slice shipped 2026-10-24.
- **[PLAN_PROJECT_PROFILES.md](PLAN_PROJECT_PROFILES.md)** — project types
  (Fiction / Notes / Document) that show, hide or rename the novelist UI
  parts; display-only, format untouched. Queued 2026-10-24; all four slices
  shipped the same day.
- **[PLAN_CI_RUST_199.md](PLAN_CI_RUST_199.md)** — CI green again under
  rustc 1.99: Windows `advapi32` link for `libgit2-sys`, macOS `killpg`
  EPERM race in the process helper. Owner go "fix ci" 2026-10-24; pushed
  the same day, Validation green on all runners.
- **[PLAN_WEB_INSTALLER.md](PLAN_WEB_INSTALLER.md)** — one-command Linux
  installer for `chikn-web`: `~/.config/chickenscratch/web.toml` holds the
  projects folder (the owner's NAS) and listen address, a user systemd
  service that waits for the mount, and reruns that upgrade without asking
  again. Queued 2026-10-24; no slice approved.

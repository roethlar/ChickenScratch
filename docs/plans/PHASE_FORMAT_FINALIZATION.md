# Phase — Format Finalization (completed; UI sync superseded)

> **Governance update (2026-06-07):** Step 2 (“Sync the five UIs”) is **superseded** by [ADR-004](../adr/ADR-004-deprecated-native-engines.md). Format work applies to the **Rust engine** and **Tauri reference UI** only. Swift/C#/Qt parity is no longer a goal. See [CURRENT_PHASE.md](../CURRENT_PHASE.md).

**Status:** Completed. Engine format lock shipped 2026-07-09; the owner later
confirmed the enclosing Coherence phase complete, durably recorded
2026-07-12. The five-UI sync scope remains superseded by ADR-004.

**Scope:** Historical phase record; it no longer replaces current work.

The `.chikn` format is one concept shared through the canonical Rust engine.
This phase locked the format as genre-agnostic, then aligned the Tauri
reference UI. The original five-independent-frontend sync scope was
superseded before completion by ADR-004; it did not ship and is not an active
instruction.

Commit `10ec683` had put novelist vocabulary (`pov_character`, `location`,
`story_time`, `duration_minutes`, `threads`, `characters_in_scene`) directly
into the `.chikn` schema. The phase removed that coupling: the format has no
domain, and novelist concepts belong in UIs that interpret generic fields.

---

## Format principles (non-negotiable)

1. **Genre-agnostic.** No POV, no scene, no plot thread at the format level. No room-of-requirement, no lab experiment, no session date either — if it's a domain concept, it's a UI concept.
2. **One generic extensibility point.** UIs write their domain-specific data into a single typed map. The format stores and preserves the map; it does not interpret.
3. **Tolerant readers, preserving writers.** Same rule as the [Folder-First Documents pattern](../FOLDER_FIRST_DOCUMENTS.md). Unknown keys round-trip untouched. Schema additions are non-breaking by construction.
4. **One engine, many interfaces.** Supported applications call
   `chickenscratch-core`; an interface that does not understand a field must
   still preserve it through the engine.

---

## What's in the format (and stays in the format)

- **Identity** — id, name, created, modified, hierarchy.
- **Generic metadata that applies across any domain** — synopsis, label, status, keywords, links, compile flags, word-count target, compile order.
- **Content** — Markdown bodies, sidecar `.meta` files, comments and footnotes (both apply to any prose).
- **Embedded git** — the `.git/` directory as required history.
- **One `fields` map per document** — arbitrary `String -> YamlValue` entries for anything the format itself doesn't know about.

## What's **not** in the format (moves out to UI conventions)

- POV character, location, story time, scene duration, plot threads, "characters in scene" — all novelist-UI vocabulary.
- Anything that starts with "in our domain, a document usually has…" — that's a convention, not the format.
- The list of valid `label` / `status` strings — any UI may present its own presets; the format stores strings.

---

## Steps

### Step 1 — Lock the schema

- Remove the six novelist-typed fields from `Document` and `DocumentMetadata` in `crates/core`.
- Add a single ordered `fields` map to both structs, omitted from serialized
  metadata when empty so documents not using it retain their prior shape.
- Update the reader to parse tolerantly (already does), and the writer to preserve the map.
- Round-trip test: set arbitrary keys → write → read → keys preserved. No-keys test: scene with empty map writes no `fields:` line.
- Publish the updated `CHIKN_FORMAT_SPEC.md` — v1.2 section reframed as "Generic UI extensibility via the `fields` map," not a list of domain-specific field names.

### Step 2 — Align supported applications

Supported applications read and write through the same engine contract:
**read `.meta` tolerantly and preserve `fields` on every write.** Display and
editing of `fields` contents remains optional per interface.

- **Tauri (`src-tauri/` + `ui/`)** — the Inspector's Scene section writes
  known novelist keys into `doc.fields`, not typed format fields.
- **TUI (`crates/tui/`)** and **converter (`crates/cli/`)** — use
  `chickenscratch-core` directly and preserve the engine contract.
- **Qt6, SwiftUI, and WinUI experiments** — removed under ADR-004. Their
  original sync tasks were superseded and were not part of phase completion.

Dropping unknown keys is a data-loss bug. Supported applications ship only
through the preserving engine contract.

### Step 3 — Publish a novelist-UI convention doc

Separate from the format spec. `docs/UI_CONVENTIONS_NOVELIST.md` lists the agreed key names the novelist UIs share in their `fields` maps:

- `pov_character` — slug/id of a character the UI treats as POV
- `location` — slug/id of a location the UI tracks
- `story_time` — free-form string ("Day 3, 22:30") or ISO
- `duration_minutes` — integer
- `threads` — list of thread ids the UI tracks
- `characters_in_scene` — list of character slugs/ids

Any novelist UI that chooses to implement scene metadata uses these exact
keys, so the data interoperates. Other domains (D&D, lab notebook, case file)
publish their own convention docs when they arrive.

### Step 4 — Verify and lock

- Engine round-trip tests prove the `fields` map and unknown keys survive
  read/write cycles.
- Tauri, TUI, and converter validation use the canonical engine contract.
- The declared suite passes for the supported applications.

---

## What followed this phase

The three [tier plans](README.md) for novelist features were reframed as
*novelist-UI* work, not format work. The Tauri scope of Tier 1 and Tier 2
later shipped; the unfinished Tier 3 items remain a later priority:

- **Scene-level metadata** (Tier 1) — Tauri writes/reads the convention keys in `fields`. Format has nothing to add.
- **Characters and locations as entities** (Tier 1) — shipped as novelist
  conventions in `characters/` and `locations/` folders.
- **Plot threads** (Tier 1) — shipped through the novelist-UI
  `threads.yaml` convention, not as typed format schema.
- **Flow mode, timeline, session targets, and snapshots** (Tier 2) — shipped
  in Tauri; **collections, rich research, and split editor** (Tier 3) remain
  UI-layer work for a later priority.

Format finalization let those features use generic conventions without
reopening the core schema.

---

## Out of scope this phase

- Visual novelist features. Not shipping new UX; shipping correctness.
- Removed native experiments gaining new capabilities; ADR-004 superseded
  that scope.
- AI streaming and remote-sync v1.1 items, which proceeded separately and
  are now shipped.

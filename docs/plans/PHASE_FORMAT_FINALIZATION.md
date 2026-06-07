# Phase — Format Finalization, Then UI Sync

> **Governance update (2026-06-07):** Step 2 (“Sync the five UIs”) is **superseded** by [ADR-004](../adr/ADR-004-deprecated-native-engines.md). Format work applies to the **Rust engine** and **Tauri reference UI** only. Swift/C#/Qt parity is no longer a goal. See [CURRENT_PHASE.md](../CURRENT_PHASE.md).

**Status:** Active (engine + Tauri scope only)
**Scope:** Replace all current feature work until complete.

The `.chikn` format is one concept. The five frontends are separate things that agree on it. This phase solidifies that split: lock the format as genre-agnostic, then bring every UI into lockstep with the locked schema.

A recent commit (`10ec683`) put novelist vocabulary (`pov_character`, `location`, `story_time`, `duration_minutes`, `threads`, `characters_in_scene`) directly into the `.chikn` schema. That's wrong. The format has no domain. Novelist concepts belong in the UIs that interpret the format, not in the format itself. This phase fixes that and sets the rule going forward.

---

## Format principles (non-negotiable)

1. **Genre-agnostic.** No POV, no scene, no plot thread at the format level. No room-of-requirement, no lab experiment, no session date either — if it's a domain concept, it's a UI concept.
2. **One generic extensibility point.** UIs write their domain-specific data into a single typed map. The format stores and preserves the map; it does not interpret.
3. **Tolerant readers, preserving writers.** Same rule as the [Folder-First Documents pattern](../FOLDER_FIRST_DOCUMENTS.md). Unknown keys round-trip untouched. Schema additions are non-breaking by construction.
4. **One format, five UIs.** Every frontend reads and writes through the same contract. An entry a UI doesn't understand survives a round-trip through that UI without loss.

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
- Add a single `fields: HashMap<String, serde_yaml::Value>` to both structs, `#[serde(default, skip_serializing_if = "HashMap::is_empty")]` so documents not using it write an identical `.meta` to today's.
- Update the reader to parse tolerantly (already does), and the writer to preserve the map.
- Round-trip test: set arbitrary keys → write → read → keys preserved. No-keys test: scene with empty map writes no `fields:` line.
- Publish the updated `CHIKN_FORMAT_SPEC.md` — v1.2 section reframed as "Generic UI extensibility via the `fields` map," not a list of domain-specific field names.

### Step 2 — Sync the five UIs

For each UI, the required behavior is the same: **read `.meta` tolerantly, preserve `fields` on every write.** Display and editing of `fields` contents is optional per-UI.

- **Tauri (`src-tauri/` + `ui/`)** — already the fullest frontend. Rewire the Inspector's Scene section to write known novelist keys into `doc.fields`, not into typed struct fields. Same UX.
- **TUI (`crates/tui/`)** — uses `chickenscratch-core` directly. Preservation is free. Editing `fields` is optional; v1.2 TUI just preserves.
- **Linux Qt6 (`linux/`)** — uses `chickenscratch-core` via the `cxx-qt` bridge. Preservation is free.
- **macOS SwiftUI (`macos/`)** — has its own `ChiknKit` in Swift. The current `touchMeta` path loads YAML into `[String: Any]` and writes it back, which preserves unknown keys for the save-doc path. Verify; ensure `createDocument` writes `fields: {}` (or just omits the key); ensure the Codable-based project.yaml writer also preserves unknown top-level keys when we get there.
- **Windows WinUI 3 (`windows/`)** — has its own `ChickenScratch.Core` in C# using `YamlDotNet`. Verify deserializer preserves unknowns; implement an extra-fields bag on the C# Document model if needed. Patch the reader and writer to round-trip.

Where a UI's reader is strict (drops unknown keys), that's a data-loss bug this phase fixes. No UI ships until it preserves.

### Step 3 — Publish a novelist-UI convention doc

Separate from the format spec. `docs/UI_CONVENTIONS_NOVELIST.md` lists the agreed key names the novelist UIs share in their `fields` maps:

- `pov_character` — slug/id of a character the UI treats as POV
- `location` — slug/id of a location the UI tracks
- `story_time` — free-form string ("Day 3, 22:30") or ISO
- `duration_minutes` — integer
- `threads` — list of thread ids the UI tracks
- `characters_in_scene` — list of character slugs/ids

Any novelist UI (Tauri Inspector, future SwiftUI scene inspector, etc.) that chooses to implement scene metadata uses these exact keys, so the data interoperates. Other domains (D&D, lab notebook, case file) publish their own convention docs when they arrive.

### Step 4 — Verify and lock

- All five UIs build clean.
- Round-trip a project through each UI: open in Tauri, save in TUI, edit in SwiftUI, open in Tauri again — the `fields` map survives every hop unchanged except for fields the writer UI explicitly modified.
- One end-to-end integration test scripted in the repo.

---

## What comes after this phase

The three [tier plans](README.md) for novelist features (`TIER1_novel_structure.md`, `TIER2_writer_workflow.md`, `TIER3_polish.md`) resume — but reframed as *novelist-UI* work, not format work:

- **Scene-level metadata** (Tier 1) — Tauri writes/reads the convention keys in `fields`. Format has nothing to add.
- **Characters and locations as entities** (Tier 1) — format gets *one* generic mechanism for entity-like folders with sidecars (maybe `entities/` is a convention for "folder of cross-referenceable items," maybe not — we decide post-finalization). Novelist UI makes `characters/` and `locations/` folders inside it by convention.
- **Plot threads** (Tier 1) — `threads.yaml` is novelist-UI convention, not format. The format preserves any additional YAML file it doesn't know about as long as git tracks it.
- **Timeline, scrivenings, session targets, snapshots, collections, rich research, split editor** (Tiers 2–3) — all UI-layer.

Doing format finalization first means these features slot in cleanly without the "oops, we need to extend the format schema again" pattern that just cost us a commit.

---

## Out of scope this phase

- Visual novelist features. Not shipping new UX; shipping correctness.
- Other frontends gaining new capabilities. TUI, Qt6, SwiftUI, WinUI preserve `fields` this phase; they don't gain editing UI for it.
- AI streaming and remote-sync v1.1 items. Those resume after.

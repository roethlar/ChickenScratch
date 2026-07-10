# Plan: Format lock — engine round-trip guarantees and spec alignment

**Status:** Proposed — awaiting owner approval (2026-07-09)

**Owner request (quote):**
> focus on 2. [Step 2 of `CURRENT_PHASE.md` — "Format lock (engine)"]

**Phase check:** [x] Allowed by `CURRENT_PHASE.md` (this *is* Step 2)  [x] Not paused

**Invariants touched:** I1, I2, I4, I5, I6, I9. I5 ("tolerant readers,
preserving writers") is currently **violated** for unknown top-level YAML
keys — fixing that is the heart of this plan.

---

## [MODEL] Audit findings this plan is based on

A six-agent audit (2026-07-09) of the spec, engine model, I/O paths, tests,
plans, and downstream consumers found that the schema work
`PHASE_FORMAT_FINALIZATION.md` Step 1 prescribes is **already implemented**:

- The six novelist-typed fields from commit `10ec683` are gone from core;
  `pov_character` etc. exist only as `fields`-map conventions
  (`crates/core/src/models/document.rs:75-76`, writer test at
  `writer.rs:901-957`).
- `fields: HashMap<String, serde_yaml::Value>` exists on `Document` and
  `DocumentMetadata` with skip-if-empty serialization; round-trip and
  no-`fields:`-line tests exist (`writer.rs:900`, `:959`, `:993`).
- The spec's v1.2 section is already reframed around generic extensibility;
  the Tauri Inspector already reads/writes scene keys via `doc.fields`
  (`ui/src/components/inspector/Inspector.tsx:22-29`).

What actually remains to lock the format:

1. **Unknown top-level keys are silently destroyed on save.** No struct in
   core has a serde catch-all, and the writer rebuilds every YAML file from
   typed structs (`writer.rs:550-584`, `:213-231`). Only keys nested under
   `fields:` survive; a top-level key written by any other/newer tool is
   lost on the first save. Violates I5 and the spec's "Lossless" pitch.
2. **Legacy sidecars from the `10ec683` era** may carry the six novelist
   keys at .meta top level; today those are dropped on read-modify-write.
3. **No format version marker** exists anywhere on disk — a finalized
   format has no hook for future migration or version detection.
4. **`fields` serializes in random order** (`HashMap`), so sidecars have no
   canonical byte form and pollute the embedded git history with spurious
   diffs.
5. **No full-fidelity round-trip test.** All existing round-trip tests
   assert hand-picked fields; a writer regression dropping e.g. `links` or
   `compile_order` would pass the suite.
6. **The spec has drifted from the engine**: it defines four fields the
   engine never implemented and drops on round-trip (`custom_styles`,
   `word_count`, `target`, `character_count` — used in the spec's own
   example); duplicates `target` vs `word_count_target`; never defines
   `links`; and still claims five byte-for-byte reference implementations
   (superseded by ADR-004).

## [MODEL] Intent

When this is done, the `.chikn` format keeps its core promise: **nothing a
writer or any tool puts in a project file is ever silently destroyed by a
save.** Unknown keys survive round-trips at every sanctioned surface, old
scene metadata written by a repudiated schema resurfaces where today's app
reads it, sidecars have one canonical byte form, the format carries a
version marker so future changes are detectable, tests prove full-fidelity
round-trips rather than spot checks, and `CHIKN_FORMAT_SPEC.md` describes
exactly what the engine does. That closes goal G2 for the engine.

## [MODEL] Approach

One slice per commit, in order; each slice lands with its tests and a green
suite. All format behavior changes live in `crates/core` (I2).

**A — Preserve unknown top-level keys (I5 compliance).**
Add a `#[serde(flatten)] extra: BTreeMap<String, serde_yaml::Value>`
catch-all to the wire structs `DocumentMetadata` and `ProjectMetadata`, and
to the models `ProjectMeta` (the `metadata:` block) and `Thread`. At write
time, `.meta` and `project.yaml` extras merge from a re-read of the existing
on-disk file — the exact pattern the writer already uses for `section_type`
/ `scrivener_uuid` (`writer.rs:548`). `ProjectMeta` and `Thread` extras ride
the in-memory model, so no merge needed. Closed structures (hierarchy
`TreeNode`, `Comment`) stay closed — the spec will say so explicitly.
Caveat carried over from the existing pattern: if a `.meta` was quarantined
as corrupt, its extras are lost with the rest (original preserved in the
quarantine file).

**B — Lift legacy novelist keys into `fields` (depends on A).**
In the reader's wire→model mapping: when one of the six legacy keys
(`pov_character`, `location`, `story_time`, `duration_minutes`, `threads`,
`characters_in_scene`) appears at `.meta` top level (i.e. in the new extras
map), move it into `Document.fields` unless `fields` already has that key
(fields wins; the stale duplicate is dropped). The next save then writes it
under `fields:`, where the Inspector and Timeline actually read it. This
hard-codes the six legacy names in core as a **migration shim only** —
clearly commented; it is not a typed field and does not violate I4's ban.
If no such files exist in the wild, the shim is a harmless no-op.

**C — Canonical `fields` serialization.**
Change `fields` from `HashMap` to `BTreeMap` on `Document` and
`DocumentMetadata` (extras maps in A are already `BTreeMap`). Mechanical
type adjustments in the few construction sites (core tests,
`src-tauri/src/commands/document.rs` `create_entity`). Sorted keys give
sidecars a deterministic byte form → quiet git history.

**D — Format version marker.**
Add optional `format_version` to `project.yaml`: writer stamps `"1.2"`;
reader tolerates absence (pre-1.2 file) and any value (never a hard gate —
preservation from A means newer files survive an older engine). Spec
documents that future breaking changes bump it.

**E — Full-fidelity round-trip tests.**
New integration test file in `crates/core/tests/`: (1) document-level —
build a `Document` populating *every* field including comments, links, and
`fields`, write → read → assert full equality; (2) project-level — fixture
project with unknown top-level keys, legacy novelist keys, threads with
foreign keys, `metadata:` extras → read → write → re-read → semantic
equality plus unknown-key survival; (3) write-twice byte-stability — second
save produces byte-identical YAML (guards C). Each new guard is proven by
temporarily reverting its slice and watching it fail (per AGENTS.md
Verification).

**F — Spec alignment (docs, last).**
Update `CHIKN_FORMAT_SPEC.md` to match the now-true engine behavior: remove
`custom_styles` / `word_count` / `target` / `character_count` from the
Extended Schema and the example (never implemented; preserved as unknown
keys if present in files); keep `word_count_target` as the one target
field; define `links`; document the preservation contract from A (and which
structures are closed), the legacy-lift from B, canonical key order from C,
`format_version` from D, and the exact `include_in_compile` wire values;
replace stale "five reference implementations / byte-for-byte" claims with
the ADR-004 reality (one engine, reference UIs). DEVLOG entry (format +
spec changed) and `.agents/state.md` update close the phase step.

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `crates/core/src/core/project/reader.rs` | flatten extras on `DocumentMetadata`/`ProjectMetadata`; legacy-key lift in wire→model mapping; read `format_version` |
| `crates/core/src/core/project/writer.rs` | merge extras on `.meta` + `project.yaml` writes; stamp `format_version`; `BTreeMap` adjustments |
| `crates/core/src/models/document.rs` | `fields` → `BTreeMap` |
| `crates/core/src/models/project.rs` | extras on `ProjectMeta` and `Thread` |
| `crates/core/tests/` (new file) | full-fidelity round-trip integration tests |
| `src-tauri/src/commands/document.rs` | mechanical `BTreeMap` type adjustment (`create_entity`) |
| `docs/CHIKN_FORMAT_SPEC.md` | alignment per slice F |
| `DEVLOG.md`, `.agents/state.md` | closing bookkeeping |

No UI (`ui/`) changes: extras are wire-side or ignored by TS; the `Document`
IPC shape is unchanged. Converter (`crates/cli`) and TUI unaffected.

## [MODEL] Tests

- [ ] Slice A: unknown top-level keys in `.meta`, `project.yaml` (top level
      and `metadata:` block), and thread entries survive read→write→read
- [ ] Slice B: legacy top-level novelist keys lift into `fields`; `fields`
      wins on conflict; keys land under `fields:` on next write
- [ ] Slice C: two consecutive saves of a fields-bearing sidecar are
      byte-identical
- [ ] Slice D: new projects stamp `format_version: "1.2"`; version-less
      legacy projects load and gain the marker on save
- [ ] Slice E: full-equality document round-trip; fixture-project semantic
      round-trip; guard-proof (revert → red, restore → green) for each slice
- [ ] Full declared suite green after every slice (fmt, clippy ×2, core lib
      tests, tauri bin tests, ui lint + build)

## [MODEL] Owner verification (plain English)

Open one of your real projects in the app, edit a scene's text and its
scene details (POV, location), save, close, reopen — everything is exactly
as you left it, and Revision History shows clean, small changes instead of
noisy ones. Most of this work is invisible insurance: it guarantees that
saving a project can never quietly delete information some other tool (or a
future version of this app) put there.

## [YOU] Decisions needed

1. **Old scene details:** projects saved during a short window last spring
   may have scene details (POV, location, story time…) stored in an
   outdated spot the app no longer reads. I plan to move them to the
   current spot on the next save so they show up in the app again
   (recommended — nothing is deleted either way). Alternative: leave them
   invisible but preserved. Approving this plan approves the move unless
   you say otherwise.

## Out of scope (recorded so they aren't lost)

Real findings from the audit that are *not* format-lock work; future work
requests can pick them up:

- Multi-file saves are not transactional (crash mid-save leaves a
  mixed-generation project; embedded git limits the blast radius).
- `save_revision` stages quarantine `*.corrupt-*` files and crash-orphaned
  `.X.tmp-*` files into permanent history (`.gitignore` pattern `*.tmp`
  doesn't match the atomic writer's temp names).
- Corrupt-sidecar quarantine warns only on stderr — invisible in the GUI.
- `include_in_compile` string matching is exact/case-sensitive (`"no"` or
  `"false"` as strings mean *included*).
- No project-level `fields` map (per-document only) — post-finalization
  decision, as is the entity-folder mechanism.
- Spec-listed but never-implemented `custom_styles` / `word_count` /
  `target` / `character_count` are being **dropped from the spec**, not
  implemented (slice F) — they have no consumers anywhere in the workspace.

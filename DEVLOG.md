# DEVLOG

Running log of architectural decisions and significant changes.

Agents: append after significant work per `AGENTS.md` Rule 3.6 — not every session. **Never** append because the owner asked a question (Invariant I0).

---

## 2026-07-09 — Format lock (engine): preservation contract, version marker, canonical bytes, spec alignment

**Change:** Completed `CURRENT_PHASE.md` Step 2 per the owner-approved
`docs/plans/PLAN_FORMAT_LOCK_ENGINE.md`. A six-agent audit first established
that the schema work in `PHASE_FORMAT_FINALIZATION.md` Step 1 (typed novelist
fields out, generic `fields` map in) had already shipped; the remaining gaps
were round-trip guarantees and spec drift. Five engine slices + one spec pass,
one commit each:

1. **Unknown top-level keys survive saves (I5).** serde flatten catch-alls on
   `DocumentMetadata`, `ProjectMetadata`, `ProjectMeta`, `Thread`; `.meta` and
   `project.yaml` extras merge from a re-read of the existing file (same
   pattern as `section_type`/`scrivener_uuid`). Corrupt existing
   `project.yaml` now aborts the save instead of being clobbered. Previously
   any top-level key the structs didn't declare was silently destroyed on the
   first save — contradicting I5 and the spec's "Lossless" pitch.
2. **Legacy lift.** The six repudiated `10ec683`-era top-level novelist keys
   lift into `fields` on read and relocate on the next save; `fields` wins on
   conflict. Owner-approved data migration; nothing is deleted.
3. **Canonical `fields` order.** `HashMap` → `BTreeMap` on `Document` and
   `DocumentMetadata`: sidecars now have one canonical byte form instead of
   per-process-random key order polluting embedded git history.
4. **`format_version` marker.** Writers stamp `"1.2"` into `project.yaml`;
   absent/unknown versions never gate reads. First on-disk migration hook.
5. **Full-fidelity round-trip tests.** New `crates/core/tests/format_round_trip.rs`:
   whole-document equality (every field at once), whole-project state over two
   save/load cycles, byte-stability across consecutive saves, foreign+legacy
   fixture. Models gained `PartialEq`. Proven capable: a writer dropping
   `links` fails these while the whole pre-existing suite stays green.
6. **Spec alignment.** `CHIKN_FORMAT_SPEC.md` now documents the preservation
   contract (and which structures stay closed), canonical serialization, the
   version marker, and the legacy lift; drops never-implemented
   `custom_styles`/`word_count`/`target`/`character_count` (preserved as
   unknowns if present in files); defines `links`; fixes the `metadata:`
   schema to match `ProjectMeta`; replaces the five-implementations section
   with the ADR-004 reality.

Every slice's guard test was proven red with its mechanism reverted before
committing. Full suite (fmt, clippy ×2, core lib, tauri bins, ui lint+build)
green on every slice. Same-day precursor: removed the deleted `linux/` tree
from the workspace manifest, which had broken every cargo command since
2026-06-07.

**Files:** `crates/core/src/core/project/{reader,writer,format}.rs`,
`crates/core/src/models/{document,project,hierarchy}.rs`,
`crates/core/tests/format_round_trip.rs`, `src-tauri/src/commands/{document,threads}.rs`
(mechanical), `docs/CHIKN_FORMAT_SPEC.md`, `docs/plans/PLAN_FORMAT_LOCK_ENGINE.md`.

---

## 2026-06-07 — Agent protocol v2: Rule 0, multi-CLI, governance

**Change:** Canonical `AGENTS.md` for Grok, Codex, Claude Code, Antigravity. **Rule 0 / Invariant I0:** owner questions → answer only, zero repo edits. Engine-only `.chikn` I/O, Tauri reference GUI, deprecated native trees. Invariants, ADRs, `CURRENT_PHASE`, DEVLOG rules for significant work only. Owner does not load files or use jargon.

**Files:** `AGENTS.md`, `CLAUDE.md`, `docs/INVARIANTS.md` (I0), `docs/AGENT-WORKFLOW.md`, `.grok/rules/`, `.claude/rules/`, reviewer checks I0.

---

## 2026-05-07 — Fifth review pass, batch 1: Windows format parity (F-001 → F-006)

**Change:** External GPT review (`docs/GPT_Code_Review.md`) flagged six Windows-specific format/data-integrity issues plus two adjacent cross-frontend wire drifts. All landed here. Build verification on Windows is still blocked by the local CoreCLR crash on macOS .NET 10.0.7, so changes were verified by Rust integration tests that simulate the Windows writer's wire form, plus careful textual review.

**F-001: Windows writer dropped document identity.** `DocumentMetaYaml` had no `Id`, `Name`, or `ParentId` fields, so a Windows save wrote `.meta` files the Rust reader could not key by id — Rust would then synthesize a fresh UUID in memory while the hierarchy still referenced the stale id, leaving the binder pointing at "missing" documents. Added all three to `ProjectYaml.DocumentMetaYaml` plus `Created`/`Modified`; `ProjectWriter.WriteDocument` now populates them from the in-memory `Document`; `ProjectReader` treats `meta.Id` as authoritative (matches the Rust reader's behavior).

**F-002: include_in_compile bool/string wire drift.** Rust `Option<String>` writes `"Yes"`/`"No"`. Windows wrote a YAML bool. `serde_yaml` cannot deserialize a bare boolean into an option-of-string, so a Windows-written `.meta` blocked Tauri project load entirely. Two-sided fix:
- Windows writer now emits `"Yes"`/`"No"` strings (matches Rust canonical).
- Rust reader gains a `deserialize_with` helper that accepts either form (legacy bool → coerced to `"Yes"`/`"No"`), so older Windows projects still open.
- Windows reader recovers a legacy YAML bool from older `.meta` files via a YamlDotNet `RepresentationModel` second-pass parse — YamlDotNet won't deserialize a bool into a string property and would otherwise silently null it.
- Spec updated to document `"Yes"`/`"No"` as canonical with bool legacy accepted.

**F-003: Windows dropped comments / Scrivener ids / session_target / threads.yaml.** Closed POCOs + `IgnoreUnmatchedProperties` meant a Windows round-trip silently stripped these. Added explicit `Comment`, `SectionType`, `ScrivenerUuid` fields to `Document` + `DocumentMetaYaml` + reader/writer. Added `SessionTarget` to `ProjectMetadata` + `ProjectMetaYaml`. Added a `Thread` model + `ThreadsYamlRoot` and a writer that round-trips `threads.yaml` (or removes the sidecar if the project has no threads, mirroring `Writer.swift` and `writer.rs`).

**F-004: Windows reader missed entity documents under `characters/` / `locations/`.** The format intentionally keeps these out of `project.yaml.hierarchy`; Tauri and macOS walk the disk to find them. Windows only walked the hierarchy, so any character or location authored in another frontend was invisible (and would be orphaned on a Windows save). Replaced the hierarchy-driven document collection with a disk walk over `manuscript/`, `research/`, `templates/`, `characters/`, `locations/` — same root list as the Rust reader.

**F-005: permanent folder delete left children behind.** `DeleteNode` called `DeleteNodeFiles(nodeId)`, which only matched when `nodeId` was itself a document id. Permanently deleting a Trash folder left every child `.md`/`.meta` on disk plus every child entry in `project.Documents`, ready for the next repair pass to resurrect. Now recurses through the removed `TreeNode` subtree, deleting each contained document by node (mirrors the Tauri fix).

**F-006: move-with-null-parent dragged nested docs to root.** `DocumentService.MoveNode` unconditionally called `HierarchyOps.MoveNode`, which interprets `newParentId == null` as "remove and re-append at root". Move-Up/Down on a nested document silently extracted it to the root list. Fixed: only call the parent-changing move when a parent id is supplied; otherwise reorder within current parent.

**Bonus drift caught while writing the F-001 regression test:** the `TreeNode` enum's `type` discriminator. Rust serializes variants as `Folder` / `Document` (PascalCase, the variant name), but the macOS Swift writer (`NodeKind.rawValue`) and the Windows C# writer both emit lowercase `folder` / `document`. Either-way means a Tauri reader could not load any project authored on a non-Rust frontend. Added `#[serde(alias = ...)]` on each variant so the Rust reader accepts either case while continuing to write the canonical PascalCase form. Same shape as the F-002 fix: tolerant reader, stable writer.

**Tests:** 59 Rust core lib tests (added `test_include_in_compile_accepts_bool_or_string`), 2 integration tests in the new `cross_frontend_round_trip` suite (Windows-shaped project + legacy bool both round-trip cleanly), `cargo clippy --all-targets -- -D warnings` clean, macOS `swift run ChiknKitChecks` 65/65, UI typecheck + production build clean, ESLint clean. Windows `dotnet build` not run locally due to known CoreCLR crash; needs verification on a Windows host.

---

## 2026-05-07 — Fifth review pass, batch 4: self-healing + import + compile + docs (F-012, F-014, F-015, F-018)

Closing out the fifth review pass. F-013 (full project rewrites) is intentionally deferred — the review marked it as a known-known and the broader writer split is a multi-day refactor that doesn't belong in a fix batch.

**F-012: self-healing claim was hollow.** `read_project` ran strict folder validation FIRST, so a project missing `templates/` or `research/` failed to load even though the existing repair pass already had logic to recreate them. Split validation into `validate_project_root` (truly fatal: path missing, not a directory, no `project.yaml`) and a new `pre_repair_folders` step that creates missing required subfolders before the main read pipeline. Repair-write failures inside `read_project` no longer get `let _ = ...`'d into oblivion — they log to stderr with a clear message about why the in-memory state may diverge from disk. Added `project_self_heals_when_required_folder_missing` integration test covering the fix.

**F-014: compile settings advertised more than they delivered.** `let _ = line_spacing;` discarded the user's setting entirely, and font / margins / fontsize only landed in PDF output. Wired `linestretch={value}` into the PDF Pandoc args (Pandoc's PDF templates honor it: 1.0 single, 2.0 double). For DOCX/ODT/HTML the settings are still Pandoc defaults — applying them properly needs reference templates we don't ship yet. Added a load-bearing comment block clarifying which settings are PDF-effective and which formats currently use Pandoc defaults, so future readers don't think "but we set fontsize" means it's reflected in DOCX output.

**F-015: markdown folder import silently lost source content.** `unwrap_or_default()` on file reads and Pandoc conversions turned every read failure into an empty `Document.content` — a "successful" import that quietly dropped the source. Replaced with explicit `match` on each conversion, accumulating per-file failures into a `Vec<String>`. If every file fails, the whole import errors with the failure list; if some succeed and some fail, the import returns the project (matching prior partial-success expectations) but logs the skipped files to stderr. The "imported empty doc on every read error" behaviour is gone.

**F-018: docs drift.** Updated `docs/CHIKN_FORMAT_SPEC.md` Last Updated to 2026-05-07. Moved AI streaming and remote-merge UX out of the "Planned" section in `docs/ROADMAP.md` — both shipped in v1.1. Updated `TODO.md` Windows parity status to reflect the fifth review pass batch 1 work (full identity + format-data preservation, wire-type fixes, disk-walking reader) and called out that local `dotnet build` is still blocked by the .NET 10.0.7 CoreCLR crash, so a Windows host smoke test remains outstanding. The `include_in_compile` spec entry was already updated in batch 1 to document `"Yes"`/`"No"` canonical with bool legacy.

**F-013 (deferred):** The review marks "full project writes still rewrite every document" as known. Splitting `write_project` into `write_structure_only` / `write_threads_only` / `write_document_meta` / `write_document_content` / `delete_document` etc. is a multi-day refactor with broad call-site impact. Tracked separately; not blocking and not appropriate to bundle into a fix batch.

**Tests:** 59 core lib, 5 src-tauri, 3 cross-frontend integration (added `project_self_heals_when_required_folder_missing`). `cargo clippy --all-targets -- -D warnings` clean. macOS `swift run ChiknKitChecks` 65/65. UI typecheck + production build clean, ESLint clean. Tauri backend builds clean.

---

## 2026-05-07 — Fifth review pass, batch 3: error propagation cleanup (F-011, F-016, F-017)

Three swallowed-error sites across Rust core, macOS Swift, and TUI. All same shape: a write or delete throws, the surrounding code says "operation succeeded", later state diverges from on-disk reality.

**F-011: `restore_document` swallowed sidecar restore errors.** The function restored the `.md` blob and then tried to restore the matching `.meta` blob via `let _ = std::fs::write(...)`. If the sidecar write failed (permissions, disk full, file locked), the user saw a successful document restore commit while metadata, comments, and `fields` stayed at the post-restore state. Now propagates write errors with a clear message — and treats sidecar absence in the commit as success (older commits predate the convention). Sidecar best-effort means "may not exist", not "may fail to write".

**F-016: macOS Writer used `try?` on destructive deletes.** Permanent-delete and stale `threads.yaml` removal both swallowed errors via `try?`, reintroducing the same class of bug the Rust `commands/document.rs` fix addressed. Replaced with a `removeIfExists(at:)` helper that propagates errors but treats "file already gone" (NSFileNoSuchFileError) as idempotent success. Used at all three sites the review flagged.

**F-017: TUI comment writes silenced project-write failures.** Three comment mutations (add, toggle resolve, edit body) all called `let _ = writer::write_project(&mut self.project)` — a failed disk write left the in-memory comment looking saved while `.meta` was unchanged, so the comment vanished on next reload. Each site now matches on the result and routes failures into the status line.

**Tests:** Same suite as batch 2 — Rust unit tests + clippy clean, macOS `swift run ChiknKitChecks` 65/65 pass with the new error propagation paths. No new tests for these (the change is "stop ignoring errors"; covering each failure mode would mean simulating filesystem failures, which has poor test ROI).

---

## 2026-05-07 — Fifth review pass, batch 2: git workflow + Unicode + draft merge (F-007 → F-010)

**F-007: manual git ops bypassed the editor flush.** Earlier review passes carefully wired `flushPendingEditorSave()` into autosave, app close, and Ctrl+S; the Revisions panel was the missing seam. "Type, immediately click Save Revision" silently committed pre-debounce on-disk content while the typed words sat in the Tiptap buffer. Same exposure for restore, draft create/switch/merge, push, fetch, pull. Added `runWithEditorFlush(opName, fn)` — awaits the flush, surfaces any flush error and aborts before the git op runs. Wrapped every git entry point in `Revisions.tsx` (save, restore, new draft, switch draft, merge draft, push, fetch, pull). Force-pull and abort-pull intentionally don't gate (force-pull explicitly discards local; abort restores pre-merge state — the buffer is about to be discarded either way).

**F-008: pull updated disk without reloading React state.** `handlePull` and `handleForcePull` refreshed only the revisions panel after pulling; the project store kept its pre-pull `documents` map. The next autosave then wrote the stale editor buffer back over freshly pulled remote content. Now calls `useProjectStore.getState().openProject(project.path)` after `fast_forward`, `merged`, abort, and force-pull success — re-reads disk so the React store and the working tree agree before the user types again.

**F-009: draft merge had no conflict awareness.** `core::git::merge_draft` called `repo.merge` and immediately invoked `save_revision`, which would happily stage and commit conflict markers. Reshaped to mirror `sync_pull`: merge-analysis first (handles up-to-date and fast-forward without touching the index), then `repo.merge` only when a real merge is needed, then `index.has_conflicts()` check before committing. New `MergeResult` enum (UpToDate / FastForward / Merged / Conflicts { files }) — same shape as `PullResult` but a separate type so callers don't have to track which operation produced the result. Tauri command updated to return `MergeResult`; the UI types and Revisions panel surface conflicts through the same dialog already used by remote pull. `GitService.MergeDraft` on Windows got the same treatment via LibGit2Sharp's `MergeStatus` enum, returning a `MergeOutcome` record so the WinUI status line stops claiming success after a conflict.

**F-010: AI/search panicked on multi-byte content.** Three byte slices in `commands/ai.rs` (`&plain[..2000]`, `plain[..4000].to_string()`, `&plain[..4000]`) and the snippet logic in `commands/search.rs` panicked when the cut landed mid-codepoint. Curly quotes, em dashes, accented characters, emoji, and CJK content all trigger this; for fiction it's not a corner case. Replaced the AI excerpts with a `truncate_chars(s, max_chars)` helper that truncates at `char_indices().nth(max_chars)` — codepoint count instead of byte count, so the limit reads as "characters" instead of UTF-8 bytes. Search snippet builder rewritten to operate in char indices via a new `snippet_around` helper that converts byte positions to char indices using `text[..pos].chars().count()`, expands ±40 chars, and rounds back to byte boundaries via `char_indices().nth()`. Added unit tests covering 4-byte emoji at boundary 2000, curly quote at byte position 39, and combining marks.

**Tests:** Rust now 5 src-tauri tests (new — truncate_chars + snippet_around coverage), 59 core lib, 2 cross-frontend integration. `cargo clippy --all-targets -- -D warnings` clean. macOS `swift run ChiknKitChecks` 65/65. UI typecheck + production build clean, ESLint clean. Tauri backend builds clean. Windows `dotnet build` still unverified locally (CoreCLR crash) — Windows merge changes will need a Windows host smoke test.

---

## 2026-05-03 — Fourth review pass: Tiptap 3 emitUpdate, idle-flush noise, flow-exit corruption, races

**Change:** Seven more findings. Most were caused by interactions between fixes from earlier passes — in particular the "memory before disk" → "disk before memory" → "memory before disk + tracked failure" oscillation around `flushPendingSave`. Settled the model.

**Tiptap 3's `setContent` emits update by default.** Programmatic document loads (`editor.commands.setContent` on flow load and single-doc switch, `clearContent` on no-doc) routed through Tiptap 3's emit-update pipeline → `onUpdate` → `debouncedSave`. So just opening a doc scheduled an autosave 2s later, and that autosave bumped the doc's `modified` even though the user hadn't typed. Fixed by passing `{ emitUpdate: false }` (setContent — `SetContentOptions` object) and `false` (clearContent — boolean, different shape) on every programmatic load.

**`flushPendingSave` ran on idle.** The function wrote to disk every time it was called, regardless of whether anything had changed. Periodic auto-commit and backup handlers call it before checking `git status`, so an idle 10-min interval re-stamped `.meta` (or before this review pass, every doc's `.meta`) and produced timestamp-only commits. Added a `dirtyRef` mirror of the `dirty` state — read synchronously inside the flush callback, set/cleared at the same points as `setDirty`. Flush now no-ops when clean.

**Exiting flow lost or corrupted edits.** `exitFlow` cleared `flowDocs` synchronously in the store; the editor's load effect ran AFTER the clear and called `flushPendingSave`, which delegated to `saveCurrent`, which read `useProjectStore.getState().flowDocs` — null by then. So the save either dropped silently (no docs to dispatch to) or, when followed by `selectDocument(X)`, fell through to the single-doc branch and wrote the entire flow buffer (with its `<!-- CHIKN_FLOW -->` markers) to a single doc's content. Added a `flowDocsRef` populated when entering flow mode — captured copy of the flow set, not a store read. `flushPendingSave` now uses that ref directly and saves each section to its captured target doc, ignoring the cleared store state. Toolbar's Exit Flow button also `await`s `flushPendingEditorSave` before invoking `exitFlow`, closing the timing gap entirely.

**Doc-switch race revisited.** Last review's "disk before memory" ordering meant a quick A→B→A switch could load `project.documents[A].content` (still the pre-save value, because the disk-then-store update hadn't reached the store yet) into the editor, then save THAT stale content over the user's real edits a moment later. Returned to "memory first, disk second" — `applyContentToStore` runs synchronously, so the store reflects the new content the instant the flush starts. Persistence-failure honesty (the reason for the previous flip) is preserved differently: the disk write still rejects on failure, `flushPendingSave` re-throws, and `flushPendingEditorSave` propagates to `beforeunload` / auto-commit.

**Ctrl+S wrote stale `activeDoc.content`.** The save shortcut routed through `useProjectStore.saveActiveDoc()`, which serialized the value held in the store — a snapshot that lags the live editor buffer by up to one debounce window. Re-routed Ctrl+S through `flushPendingEditorSave()` so it writes the live Tiptap markdown.

**`beforeunload` ran backup after a failed flush.** The catch block toasted the error and let the handler proceed — `backup_on_close` then auto-committed the pre-flush on-disk state into git, freezing the loss. Now sets `flushed = false` on catch and bails before the backup step.

**`stripStructuralPadding` ate markdown line-breaks.** The previous regex `[ \t]*\n[ \t]*\n?` consumed trailing spaces before the structural newlines, but two trailing spaces before `\n` are markdown-significant (force a hard line break). Tightened to `\n\n?` only — match only the literal newlines `buildFlowBoundary` adds, never spaces.

**Tests:** 77 Rust (58 lib + 2 integration + 17 doctest), `cargo clippy --all-targets -- -D warnings` clean, macOS `swift run ChiknKitChecks` 65/65, UI typecheck + production build clean, ESLint 0 errors (3 pre-existing `useCallback` `setProject` deps warnings).

---

## 2026-05-03 — Third review pass: persistence-failure honesty, store helper, lint cleanup

**Change:** Seven more review findings, all real correctness issues. Plus the four ESLint errors that have been hanging on for several review cycles.

**Save flush hid persistence failure.** `flushPendingSave` updated `project.documents[id]` and `activeDoc` *before* awaiting the disk write, then caught the disk error and resolved successfully. So `flushPendingEditorSave()` returned `Promise<void>` to its caller (`beforeunload`, the auto-commit interval, the periodic backup interval) even when nothing actually got persisted — the store and the on-disk state diverged silently. Now the order is disk → store, and the catch re-throws so callers can decide whether to skip backup/commit on a failed flush.

**Flow save reported clean after partial failure.** The `for (const sec of sections)` loop swallowed per-section errors and `setDirty(false)` ran unconditionally afterward. A flow save where two of three sections persisted and one failed showed "Saved" in the status bar. Now tracks `anyFailure` and only clears dirty when the whole batch succeeded.

**Periodic auto-commit and backup didn't drain editor edits first.** The 10-min auto-commit interval and the user-configured periodic backup ran `git status` / `backup_on_close` against on-disk state directly, so the snapshot they captured excluded any edits the user had typed in the last 2s of the debounce window. Both intervals now `await flushPendingEditorSave()` before kicking off their work.

**Flow split's `.trim()` ate intentional whitespace.** A blanket `.trim()` on each section's slice was needed to strip the structural `\n\n` that `buildFlowBoundary` adds around markers, but it also ate any leading/trailing whitespace the writer put there on purpose — a doc that ended with a deliberate blank line silently lost it on every flow-mode save and the file drifted toward no-blank-line over time. Replaced with `stripStructuralPadding`, which only consumes up to two leading and two trailing newlines (matching what the writer adds) and preserves anything beyond.

**Project mutations didn't re-derive `activeDoc`.** Several call sites (`CommentsPanel`, `Inspector`, `Binder`, `Corkboard`, `Preview`, `Toolbar`'s addComment, `Revisions` Threads tab, `App.tsx`'s ⌘N handler) called `useProjectStore.setState({ project: updated })` directly. `selectDocument` re-derives `activeDoc` from the project map, but a plain `setState` doesn't — so `activeDoc` continued to point at the pre-mutation document object. Side effects: comments panel showed pre-add state, inspector showed pre-edit metadata, etc. Added a `setProject(project)` helper to the store that updates `project` and re-derives `activeDoc` from `project.documents[activeDocId]`. Migrated all call sites.

**Empty-threads.yaml deletion swallowed fs errors.** Same class of bug as the .md/.meta deletion in the prior review: `let _ = fs::remove_file(&path)` meant a failed unlink left the file on disk with the pre-deletion threads, and the next reader run resurrected every "deleted" thread. Now propagates the error so the writer reports the failure to the caller.

**`move_node` swallowed reorder failure after a parent move.** When both `new_parent_id` and `new_index` were given, the move ran with `?` but the reorder ran with `let _ = ...`. An invalid index — e.g. UI passing a stale position from before another reorder — used to return `Ok(())` while the actual position was wrong. Now propagates.

**Lint cleanup.** ESLint had been carrying four errors across `Editor.tsx`, `FindReplace.tsx`, `Toolbar.tsx`, and `DocumentHistory.tsx`. The setState-in-effect ones are intentional (syncing local state to external editor / git events) and now have targeted disables with justification comments. `Toolbar.FlowButton` had an unused `_props: { editor }` arg dating from a refactor — removed. Lint baseline: 0 errors (down from 4), 3 pre-existing warnings (`useCallback` missing-deps from the new `setProject` migration, pragmatic to leave).

**Tests:** 77 Rust tests (58 lib + 2 integration + 17 doctest), `cargo clippy --all-targets -- -D warnings` clean, macOS `swift run ChiknKitChecks` 65/65, UI typecheck + production build clean, lint 0 errors.

---

## 2026-05-03 — Second review pass: app-close flush, store coherence, more

**Change:** Six findings from a second review cycle, all real. Patched together.

**App-close pending edits.** `beforeunload` triggered `backup_on_close` (which auto-commits whatever's already on disk) but didn't first ask the editor to write its in-flight debounced text. Editor unmount cleared the timer without flushing it. Two changes: the editor's unmount effect now calls `flushPendingSave` (fire-and-forget, fine for the typical "switch projects" path), and Editor publishes the flush via a new `setPendingFlush` / `flushPendingEditorSave` pair on `editorRef` so `App.tsx` can `await` it inside the close handler before kicking off the backup commit. `beforeunload` is famously not a real "wait until I'm done" hook — the WebView can tear down mid-promise — but the synchronous parts of the call (timer cancel, IPC dispatch) still race to completion and the data-loss window shrinks dramatically.

**Store left stale after `flushPendingSave` / `saveCurrent`.** Both wrote to disk via `update_document_content` and then either updated nothing (`flushPendingSave`) or only `activeDoc` (`saveCurrent`); neither updated `project.documents[id]`. On rapid switch-back, `selectDocument` rehydrated `activeDoc` from the stale map and the editor effect loaded *that* into the buffer, silently reverting recent typing. Added an `applyContentToStore(id, markdown)` helper that updates both `project.documents[id]` and `activeDoc` in one pass, called from each save site.

**`FlowBoundary` widget capture + names with quotes.** The widget callback closed over the loop's `match` variable, which a regex's `exec` mutates between iterations — by the time ProseMirror invoked the renderer, `match` was either `null` or a different match's data. Captured the values up front. Switched the widget from `innerHTML` to DOM-API `textContent` so the doc name can't smuggle markup. `escapeHtml` now also escapes `"` (renamed `escapeMarkerName`) so a doc named `Chapter "1"` doesn't terminate the marker's `name="..."` attribute and confuse the regex; matching `decodeMarkerName` reverses the escapes when the widget renders.

**`threads.yaml` not removed when the last thread is deleted.** `write_threads_if_any` returned early when `project.threads` was empty, leaving a stale file on disk that a reload would resurrect. Now removes the file in the empty case (best-effort — a remove-failure is logged-only since the worst case is a stale sidecar, not data loss). Regression test: `test_emptying_threads_removes_file`.

**Permanent delete swallowed filesystem errors.** `delete_node_files` did `let _ = writer::delete_document(...)` and then unconditionally dropped the doc from `project.documents`. A permission denial or full disk would leave orphan `.md` / `.meta` files that the next reload's repair pass would happily re-import. Now propagates the error and the doc stays in state if the disk delete fails.

**Every `write_project` rewrote every `.meta` with a fresh `modified: now()`.** Renaming a single doc, moving any node, or saving any one document all caused the writer to iterate `project.documents.values()` and stamp every sidecar's `modified` timestamp. That makes git diffs noisy (every commit "touches" every doc) and per-document modified dates inaccurate (they all bump together). Switched `write_document` to use `document.modified` from the in-memory state — callers that genuinely change a doc bump that field themselves (most already did). Audited the one missing case: `link_documents` now bumps `doc.modified` on each linked endpoint. Regression test: `test_write_preserves_document_modified`.

**Tests:** 58 lib + 2 integration + 17 doctest = 77 Rust tests passing. `cargo clippy --all-targets -- -D warnings` clean. macOS `swift run ChiknKitChecks` 65/65 green. UI typecheck + production build clean.

**What's still on the medium-deferred list:** the broader "every project write rewrites every doc's content" inefficiency. After this pass the .meta noise is gone, but the writer still iterates and overwrites every `.md` on every save. Files with unchanged content will show byte-identical writes — git won't pick them up — but the I/O is wasted. A real fix splits `write_project` into structure-only (project.yaml + threads.yaml) and per-doc paths, and updates each command to write only what it changed. Larger refactor; punted.

---

## 2026-05-02 — Review-driven fixes (data-loss + correctness)

**Change:** External review surfaced eight bugs across Tauri commands, the React editor, the core reader/writer, and the markdown preview. Most were data-loss class — silent on success, only visible if a writer noticed missing words. Patched in one batch.

**Data-loss bugs:**
- **Permanent delete recreated files** — `delete_node` (`src-tauri/src/commands/document.rs`) called `delete_node_files` to remove the `.md` / `.meta` from disk but left the entry in `project.documents`. The very next `write_project` iterates `project.documents.values()` and rewrites every doc, so the deleted files were re-emitted with their old content. Fixed by also dropping the entry from the in-memory map (and threading `&mut Project` through the recursive helper).
- **Flow mode lost edits to the first document** — the editor concatenated docs as `[doc1, BOUNDARY, doc2, BOUNDARY, doc3]` with no leading boundary. `splitFlowSections` walks markers and only emits sections delimited by them, so any content before the first marker (i.e. all of doc1) was silently dropped on save. Fixed by emitting a leading boundary for every doc — first one too.
- **Quick document switching dropped 2s of debounced edits** — typing then immediately switching to another doc replaced the editor buffer before the pending save fired. The naive flush approach would save the OLD content under the NEW doc's id (since `saveCurrent` reads `activeDoc` dynamically). Added a `flushPendingSave` that captures `docIdRef.current` (the id the editor was bound to) plus the current markdown and writes them explicitly. Called from every transition: single-doc → other doc, single-doc → flow, flow → single-doc, single-doc → no doc.

**Correctness bugs:**
- **Move Up / Move Down extracted nested items to root** — UI sent `newParentId: undefined` meaning "keep current parent," but `move_node` always called `hierarchy::move_node(None)` which means "move to root." Drag-drop reorder had the same shape. Fixed both sides: backend now treats `None` as "keep current parent" and uses `reorder_node`, while the drag-drop handler computes the *target's* parent and passes it explicitly so cross-folder drops land where the user dropped them. (`findNodeIndex` now returns `parentId` alongside index/siblings.)
- **Reader's repair pass added entities to the main hierarchy** — characters under `characters/` and locations under `locations/` are by design *not* in `project.yaml.hierarchy`. UIs surface them in dedicated sections by walking `project.documents`. The repair pass treated them as orphans and migrated them into the binder tree on every reload, slowly draining the entity sections into the main pane. Fixed by skipping `characters/` and `locations/` paths in the orphan check.
- **New projects shipped without a Trash folder** — `create_project` set up Manuscript and Research only; the Trash folder appeared on next reload via the repair pass. First-session "Move to Trash" deletes therefore fell through to permanent deletion. Added Trash to the initial hierarchy.

**UX / security:**
- **Preview rendered untrusted markdown without sanitization** — `Preview.tsx` ran `marked.parse(doc.content)` and dropped the raw HTML into `dangerouslySetInnerHTML`. With `csp: null` in `tauri.conf.json`, a `<script>` smuggled into a `.md` file would execute. Wrapped the parse in DOMPurify (added as a dep with default html profile) so the active vector is closed regardless of CSP state. CSP tightening deferred — the dev workflow uses Vite HMR which would need `'unsafe-eval'` exemptions and the trade-off needs more thought.
- **Compile ignored the configured Pandoc path** — `core::compile::compile` always ran `Command::new("pandoc")` regardless of `settings.general.pandoc_path`. `CompileOptions` gained `pandoc_path: Option<String>`, and `compile_project` now threads the setting through. Same for `import_scriv` / `export_to_scriv` doctests, which had been stale on the existing `Option<&Path>` arg.
- **Auto-save delay was hardcoded 2000ms** despite Settings ▸ Writing exposing the seconds. `Editor` now reads `appSettings.writing.auto_save_seconds` from the store and uses it (clamped to ≥250ms), falling back to 2s while settings hydrate.

**Tests:** Doctest fix unblocks `cargo test -p chickenscratch-core --doc` (17/17). UI typecheck + production build pass. `cargo check` clean. Lint baseline unchanged (5 pre-existing errors, none new).

---

## 2026-05-02 — macOS SwiftUI brought to format + workflow parity (slices A/B/C)

**Change:** The macOS SwiftUI frontend was a shell — open / read / edit / save revision, nothing else. Brought it to feature parity with Tauri for everything that doesn't require a rich-text editor port.

**Slice A — Foundation + scene metadata (~600 LOC).** `ChiknKit` gains a `YAMLValue` Sendable enum so `DocumentMeta.fields: [String: YAMLValue]` can carry arbitrary user-defined entries while the rest of the model stays Sendable. Reader/writer round-trip the `fields:` block; empty maps drop the key entirely. `Project.threads: [Thread]` reads/writes `threads.yaml`; `ProjectMetadata.sessionTarget` reads/writes the nested `session_target` block via `encodeIfPresent` so projects without one carry no key. Reader walks `manuscript/`, `research/`, `templates/`, `characters/`, `locations/` directly from disk (instead of being driven by hierarchy), so entities under `characters/` and `locations/` actually load. Inspector grows a Scene section (POV / location / story_time / duration / threads / other-characters) with an entity menu that lists existing characters/locations and offers inline create. Binder grows Characters and Locations sections (filtered by path prefix from `project.documents`, since entities aren't in `project.yaml.hierarchy`) plus thread color dots beside docs.

**Slice B — Drafts + per-doc history + dangling refs (~400 LOC).** `Git.swift` gains `createDraft` / `listDrafts` / `switchDraft` / `mergeDraft` (shells out to `/usr/bin/git`), and `documentHistory` / `restoreDocument` for per-file timeline. `References.validate(_:)` ports the Tauri `validate_references` command — walks every doc's fields map and reports references to missing characters / locations / threads. RevisionsView restructured into three tabs (History / Drafts / Threads) with a dangling-refs banner on Threads. New `DocumentHistoryView` sheet is wired into the binder context menu ("File History…") for both regular docs and entities.

**Slice C — Stats + Timeline + binder polish (~500 LOC).** `Stats.swift` ports word counter, project stats, writing-history.json round-trip with `start_words` first-of-day capture, and session-progress computation (today_words, days_remaining, needed_per_day rounded up). New `StatsView` sheet shows manuscript words / pages / read time, 14-day daily-word chart, per-doc bars sorted descending, and a session-target editor. New `TimelineView` parses `story_time` (ISO 8601 → seconds, then leading-integer fallback for "Day 3" style) and renders POV / Thread / Single lanes with click-to-open chips. `Editor` gains an idle-hiding `SessionBadge` showing today/goal + days-left + needed/day. Binder context menu gets `Move Up` / `Move Down` / `Move to Trash` / `Delete Permanently` / `Empty Trash`, gated correctly for special folders. Writer gains `deleteNode` (recursive file removal), `moveNode` (with optional `newIndex`), `reorderNode` (within current parent).

**Why not a single sweep:** Sliced because each layer's invariants compound. Slice A is "the format reads and writes everything"; without that, nothing UI-side works. Slice B is the destructive side (drafts, restore) and needs the format layer settled. Slice C is the polish that depends on both. Sliced delivery also kept each round of round-trip checks scoped to what just changed.

**Bugs caught by the round-trip checks** that would have shipped to users:
- `Yams` (libyaml) eagerly parses ISO-shaped scalars into `Date` per YAML 1.1. A foreign field like `story_time: 2026-04-23` would round-trip to a `Date` and disappear from the typed `String`-shaped path. Fixed in `YAMLValue.init?(any:)` by detecting `Date` and re-serializing it back to a string (date-only when midnight UTC, full ISO 8601 otherwise).
- `URL.temporaryDirectory` returns `/var/folders/...` (a symlink to `/private/var/...`) but `FileManager.enumerator` returns URLs with the resolved path. The Reader's relative-path computation silently failed prefix-stripping, the document carried an absolute path forever, and the writer concatenated it onto the project root producing `chikn-test-XXX.chikn/private/var/folders/.../chikn-test-XXX.chikn/manuscript/file.meta`. Fixed by resolving symlinks on both sides before stripping.

**Auto-commit safety net:** `switchDraft`, `mergeDraft`, and `restoreDocument` all auto-commit dirty working-tree state first as `Auto: pre-{op} snapshot`. The Tauri equivalent uses libgit2 with `force()` checkout, which silently discards uncommitted changes; matching that exactly would have been a sharp edge for writers, and the safety commit costs nothing.

**Tests:** `Tests/ChiknKitChecks/main.swift` runs as `swift run ChiknKitChecks` (no XCTest module, since Command Line Tools toolchain doesn't ship one). 18 cases / 65 checks covering fields round-trip, foreign-key preservation, threads.yaml, session_target, entities, dangling refs, drafts (create/switch/merge with cross-branch document visibility), per-doc history + restore (with the new commit forward-only), word counter, project stats, writing-history start_words capture, sessionProgress, deleteNode (file removal), moveNode-into-Trash (file preservation), and reorderNode.

**Still gap vs. Tauri** (each its own multi-day cut):
- Rich-text editor with markdown round-trip — the macOS `TextEditor` is plain text; Tauri uses Tiptap with tiptap-markdown
- AI streaming, comments anchored to text, footnotes, find/replace, flow mode
- Compile/export UI, settings panel (theme/backup/remote/AI/compile), project search, templates CRUD
- Remote sync UI (push/fetch/pull + conflict dialog)
- Drag-drop reorder in binder (keyboard via context menu Move Up/Down today)

---

## 2026-04-27 — Format finalization: genre-agnostic, generic `fields` extensibility

**Change:** Reverted the design from the 04-23 entry below. The `.chikn` format is genre-agnostic; novelist concepts (POV, location, threads, etc.) are UI conventions, not format schema. The format gains exactly one extension point:

```yaml
fields:
  any_string_key: <any YAML value>
```

A `HashMap<String, serde_yaml::Value>` on `Document` and `DocumentMetadata` in core; serialized into `.meta` files as the top-level `fields:` mapping; skipped when empty so projects that ignore the mechanism produce zero diff. Readers preserve unknown entries; writers round-trip them. This is the format-level "tolerant readers, preserving writers" rule from `FOLDER_FIRST_DOCUMENTS.md` made concrete.

**Why:** The 04-23 commit baked `pov_character`, `location`, `story_time`, etc. directly into the format schema. That's a category error — the format core is one concept, the five UIs are separate things that interpret it. Novelist vocabulary belongs in a UI convention doc (`docs/UI_CONVENTIONS_NOVELIST.md`), not in the spec. A TTRPG UI, a lab notebook, or a case-files UI should be able to use `.chikn` without first understanding "POV character." The generic `fields` map is the format saying "UIs, here's where your data goes — I won't read it, and I won't lose it."

**After:**
- `.chikn` v1.2 schema = v1.1 + one optional `fields:` mapping per document. Period.
- Tauri Inspector's Scene section unchanged in UX — it now writes the same six novelist keys into `doc.fields`. Same panel, different persistence layer.
- New convention doc lists the agreed novelist key names so Tauri / SwiftUI / Qt6 / WinUI / TUI novelist modes interoperate. Other domain UIs publish their own.
- All five frontends round-trip arbitrary `fields` entries:
  - Tauri (Rust core) — full editing support for novelist keys
  - TUI, Linux Qt6 — preserve via `chickenscratch-core` (no UI yet for editing)
  - macOS SwiftUI — preserves via dict round-trip in Writer.swift; no editing UI yet
  - Windows WinUI — patched (was the lone broken frontend: closed POCO dropped unknowns). Added `Fields` to `DocumentMetaYaml` + `Document`; reader/writer round-trip
- Three new round-trip tests in core: arbitrary keys/types preserve; empty maps skip serialization; foreign keys hand-injected into a `.meta` survive a read/write/read cycle.

**Phase plan:** [`docs/plans/PHASE_FORMAT_FINALIZATION.md`](docs/plans/PHASE_FORMAT_FINALIZATION.md) for the rationale and rollout. Tier 1/2/3 novelist plans paused — they resume as UI-layer work once this phase ships.

**Commit:** `<pending>`

---

## 2026-04-23 — v1.2 scene-level metadata (first slice of Tier 1) — superseded

**Change:** `.chikn` format gains six optional scene-level fields: `pov_character`, `location`, `story_time`, `duration_minutes`, `threads`, `characters_in_scene`. Tauri inspector gets a collapsible **Scene** section exposing all six as free-form text inputs (entity dropdowns come with Tier 1.2/1.3 when the `characters/` and `locations/` folders land).

**Superseded** by the 04-27 entry above. The novelist-typed fields were the wrong scope — they put domain vocabulary into the format. Replaced with a generic `fields` map; same six keys land as a UI convention instead.

**Why:** First concrete deliverable against the v1.2 novelist-features plan. Peer tools (bibisco, yWriter, oStorybook) ship POV/location/duration as first-class fields; `.chikn` stored everything as free-form keywords which don't validate. Schema additions land first so later features (timeline view, entity cross-refs, collection queries) have the fields they'll read.

**Design notes:**
- All six fields are optional. v1.1 readers ignore them; v1.1 writers preserve them on round-trip (Rust `DocumentMetadata` already `#[serde(default, skip_serializing_if)]` for unknowns we don't explicitly own). A scene with none of the new fields writes an identical `.meta` to what v1.1 produced — zero diff noise for projects that don't use them.
- Free-form strings for POV/location. Eventually they'll be slug/id refs into `characters/` and `locations/` entities; for now writers type names or slugs and the inspector doesn't validate. When entities ship, the input becomes a dropdown and existing strings upgrade to resolved refs.
- Threads as comma-separated ids. A proper multi-select with live thread picker arrives alongside `threads.yaml`.
- Inspector Section is collapsible with a chevron toggle; auto-expands for documents that already have any of the fields set (so writers returning to a scene notice their work is there).
- Tauri command `update_document_metadata` takes a new optional `scene: SceneMetadata` sub-payload rather than six more positional args. Frontend wraps it in a typed `SceneMetadata` interface in `commands/document.ts`.

**Tested:** Two new round-trip tests in `crates/core/src/core/project/writer.rs`:
- `test_scene_metadata_round_trip` — set all six fields, write → read, assert preservation.
- `test_scene_metadata_absent_is_clean` — write a scene with no v1.2 fields, read back the `.meta` text, assert the new keys don't appear.

All 52 lib tests pass; Tauri typecheck clean; UI tsc/eslint clean.

**Commit:** `<pending>`

---

## 2026-04-22 — macOS SwiftUI app — writing, auto-commit, new doc, rename

**Change:** The macOS SwiftUI scaffold becomes a usable editor. Typing in `TextEditor` now persists to disk (debounced 1.2s); the Binder can create new documents and rename existing ones via context menu; ⌘R opens a Save Revision prompt; auto-commit fires at most every 10 minutes with `Auto: <ts>`.

**Why:** The scaffold was read-only — useful for showing Liquid Glass but not for writing. Writing + save + revisions is the bar for "actually usable alternative frontend."

**Design notes:**
- `ChiknKit.Writer` rewrites `.md` content, touches the `.meta` modified timestamp, and rewrites `project.yaml` through Codable structs with explicit `CodingKeys` so the top-level key order (id, name, created, modified, metadata, hierarchy) stays stable across saves — clean diffs against the Rust/C# writers.
- `ChiknKit.Git` shells out to `/usr/bin/git` because SwiftPM doesn't have a zero-friction libgit2 wrapper; every recent macOS has git through Command Line Tools (xcrun). Author is hard-coded to "ChickenScratch <writer@chickenscratch.local>" so commits attribute to the app, not whatever the user last set globally.
- `ProjectStore.saveDocument` is the one write path. After each save it asks "is it time to auto-commit?" (10-min threshold). Named revisions come through a separate explicit path (`saveRevision(message:)`).
- `TextEditor` drives via `.onChange(draft)`, scheduling a debounced Task and flushing on disappear so doc switches don't drop the last keystroke.

**Scope cut:** delete/move/reorder in the binder, inspector editing, comments, footnotes, compile, AI, drafts. Remote sync from this frontend is also still open — the other frontends push via libgit2's credential callback, and shelling out to `git push` on the Swift side would need a different credential story (no in-process callback).

**Commit:** `6d999a1`

---

## 2026-04-21 — Remote sync (push/fetch + status)

**Change:** New `sync` git remote, push/fetch/status commands in core + Tauri, Remote settings tab, Revisions-panel footer widget that shows "N to push · M to pull" and exposes Push/Fetch buttons. Separate from the existing `backup` remote (directory mirror) — `sync` accepts any git URL (HTTPS, SSH, or `file://` for testing).

**Why:** The biggest v1.1 gap. User writes on macOS + Linux + Windows; backup mirrors the project to a local directory but doesn't help you start a new session on a different machine. Remote sync closes that loop.

**Design notes:**
- Remote named `sync` so it doesn't collide with the user's own `origin` or our `backup`. `ensure_sync_remote` updates the URL in place if the setting changes.
- Credential callback handles HTTPS username/PAT first, then SSH-agent fallback for `git@` URLs. No OS keychain yet — the token lives in plaintext in `settings.json`; scope the PAT to one repo.
- `sync_status` returns `(ahead, behind)` from `graph_ahead_behind` against the last-fetched `refs/remotes/sync/<branch>`. Before the first fetch, ahead = total commit count and behind = 0, so the UI has a sensible "push everything" starting state.
- Auto-push on named revision is opt-in (off by default) and fire-and-forget — a failed push never rolls back the revision.

**Scope limits:** Push and fetch. **Not** included: merge of incoming commits, conflict UX, SSH key passphrases. If a fetch brings down commits that diverge from local, the status shows "N to pull" but there's no in-app merge yet — the user would need to pull/merge via CLI. That's the next pass.

**Tested:** Round-trip integration test in `crates/core/tests/remote_sync.rs` — pushes a fresh project to a `file://` bare repo, fetches back, asserts ahead/behind = 0; then adds a revision, asserts ahead = 1, pushes, asserts ahead = 0 again.

**Scope note:** Tauri-only. TUI, SwiftUI, and Linux Qt6 frontends do not push to the sync remote yet (TUI pushes to the `backup` directory mirror on named revision; SwiftUI shells out to `git`, which would need a separate credential story; Qt6 frontend has no git wiring yet).

**Commit:** `ceb3815`

---

## 2026-04-18 — Side-by-side draft comparison

**Change:** New "Compare Drafts" dialog accessible from the Revisions panel when a project has ≥ 2 draft versions.

**Why:** Writers who experiment on branches ("what if this chapter started differently?") want to see what actually changed without committing to a merge.

**After:**
- Backend: `compare_drafts(project_path, draft_a, draft_b)` returns `Vec<FileDiff>` — files that differ between branch tips, skipping `.meta` / `project.yaml` / `.git`
- Backend: `word_diff_drafts(project_path, draft_a, draft_b, doc_path)` — tracked-changes style word diff for a single doc
- Frontend: `DraftCompare` dialog with two dropdowns (pick left / right draft), swap button, file list on left pane, word-level diff view on right pane
- Uses the same green/red strikethrough visual as the per-revision diff viewer
- Non-destructive (read-only comparison)

**Commit:** `e3d31a8`

---

## 2026-04-18 — TUI anchored inline comments

**Change:** F3 in the TUI wraps the currently-selected text with a comment span and prompts for a body. Adds the comment to the document's `.meta` with the same ID as the `data-comment-id` attribute in the span.

**Why:** The TUI had comments only as document-level orphans. Writers expected to anchor comments to specific text, just like the Tauri app. ratatui-textarea exposes `selection_range()` returning `((row, col), (row, col))`, which we use to wrap the selection in lines with `<span class="comment" data-comment-id="X">…</span>`.

**After:**
- F3 (editor focus + active selection): prompt for body, on confirm wrap selection + add to .meta + save
- F2 (any focus): opens comments overlay (unchanged)
- Normalizes selection direction; handles single-line and multi-line selections; char-boundary-safe string slicing

**Commit:** `7762508`

---

## 2026-04-18 — Edit path no longer touches pandoc

**Change:** Replaced pandoc subprocess for markdown ↔ HTML conversion in the Tauri editor with `tiptap-markdown` (in-process, markdown-it + prosemirror-markdown).

**Why:** The previous design ran pandoc as a subprocess on every document load and save. This coupled core editing to an external binary being present, findable, and non-crashing. ~50ms spawn cost per save, two conversion hops per edit session, editing breaks if pandoc is missing. Fundamentally wrong architecture.

**After:**
- Editor load: `setContent(markdown)` — tiptap-markdown parses natively in-browser
- Editor save: `editor.storage.markdown.getMarkdown()` — in-process serialization
- Pandoc is still a dependency but only for compile/export/import paths (triggered by explicit user actions, not every keystroke)
- Per-save latency: ~1ms
- Custom HTML (comment spans, footnote nodes) round-trips via `html: true` option on tiptap-markdown — markdown-it passes inline HTML through untouched

**Commit:** `5c23763`

---

## 2026-04-18 — Canonical format: HTML → Pandoc Markdown

**Change:** `.chikn` projects now store documents as `.md` files, not `.html`. `DOCUMENT_EXTENSION` flipped. Compile pipeline reads markdown directly via pandoc.

**Why:** Editing markdown over HTML on the TUI side was lossy — any inline HTML that markdown couldn't express (comment spans, footnote nodes, colored text, `<u>`) got silently destroyed on every save. The fix wasn't preservation tokens or sidecar files; it was picking a single canonical format both frontends can natively handle.

**After:**
- Storage: Pandoc Markdown (`[text]{.class #id key="value"}` bracketed spans for attributes, `[^1]` footnotes, GFM extensions)
- Compile: `pandoc -f markdown -t docx|pdf|epub|...` — `strip_comments` and `transform_footnotes` helpers deleted; pandoc handles natively
- Scrivener import: RTF → markdown instead of RTF → HTML
- Import pipeline: converts all external formats to markdown
- Tests updated throughout
- Interop win: `.md` files editable in vim, Obsidian, VS Code, any markdown tool

**Commits:** `2c3b8cf` (migration), `5c23763` (edit path cleanup)

---

## 2026-04-18 — Comments and footnotes

**Change:** First-class Word-style comments (anchored to a text span, resolvable, right-gutter panel) and inline footnotes.

**Why:** Writers need marginal notes and citation-style footnotes. Scrivener's annotation feature was a known user request.

**After (Tauri):**
- `CommentMark` TipTap extension (inline span with `data-comment-id`)
- Toolbar speech-bubble icon: select text → prompt → wrap selection
- `CommentsPanel` right gutter: click to scroll-to-anchor, double-click body to edit, resolve/delete
- Per-comment data (id, body, resolved, created/modified) stored in `.meta`; anchor span in content
- Footnote asterisk icon inserts `<sup class="footnote" data-body="...">●</sup>` inline node
- Compile pipeline gained `transform_footnotes` to convert to pandoc-native footnote HTML pattern (later simplified in the markdown migration)

**After (TUI):**
- F2 opens comments overlay modal
- Navigate with ↑↓, `e`/Enter edit, `r` resolve/unresolve, `d` delete, `n` add (orphan — anchored comments require text selection, Tauri-only for now)

**Commit:** `3b207f8`

---

## 2026-04-18 — Push to backup on named revision

**Change:** Both apps push to the backup remote when a named revision is saved (in addition to existing triggers: project close, periodic timer).

**Why:** Named revisions are deliberate milestones. Writers expect "Save Revision" to sync.

**After:**
- Three backup triggers: named revision, project close, interval
- TUI reads shared settings file at `~/.config/chickenscratch/settings.json`
- Fire-and-forget: backup failure doesn't fail the revision

**Commit:** `b3bb126`

---

## 2026-04-18 — TUI editor evolution: tui-textarea → edtui → ratatui-textarea

**Change:** Cycled through three TUI text widgets to land on `ratatui-textarea` 0.9 (ratatui 0.30).

**Why:** The original `tui-textarea` 0.7 doesn't support soft word-wrap — long prose lines horizontal-scroll. Unusable for writing.

**Path:**
1. **tui-textarea 0.7** — no wrap; horizontal scroll breaks prose editing.
2. **edtui 0.9.9** — has wrap but is vim-mode-only. Writers who don't know vim would be lost. Shipped with forced Insert mode as a hack.
3. **ratatui-textarea 0.9 + ratatui 0.30** — native simple-mode editing (no modes), word-wrap via `WrapMode::WordOrGlyph`, Emacs-style shortcuts.

**After:** Ratatui 0.30 upgrade (from 0.29). crossterm 0.29. Editor feels like nano/notepad — type to insert, arrows move, Ctrl+A/E start/end of line. Ctrl+W toggles wrap.

**Commits:** `4da32e3` (initial TUI), `35695fc`, `fe96b9e`, `0201857`

---

## 2026-04-18 — TUI MVP

**Change:** New `chikn` binary — a terminal UI for ChickenScratch, sharing the Rust core library.

**Why:** Writers who live in terminals, ssh sessions, tiling WMs. ~7MB binary vs ~100MB for the Tauri bundle.

**Features:** Binder tree (navigate, open, expand/collapse folders), markdown editor, save (Ctrl+S), save revision (Ctrl+R), quit with confirmation, view mode cycle.

**Commit:** `4da32e3`

---

## 2026-04-08 — v0.1.0-alpha

**Milestone:** Tagged initial test release for alpha feedback from writers.

**Tag:** `v0.1.0-alpha` (commit `ed1dfd9`)

---

## 2026-04-08 — Writing statistics + revision diff

**Change:** Per-document word counts, page estimate, reading time, daily writing history chart. Word-level revision diff (tracked-changes style: green additions, red strikethrough deletions). Auto-commit every 10 minutes when changes detected.

**Commits:** `fc63de2`, `e2884e6`, `0083030`

---

## 2026-04-08 — Keyboard shortcut customization + AI improvements

**Change:** All shortcuts configurable in Settings. AI kill switch (when disabled, hides sparkle menu + summarize button). AI via reqwest instead of curl subprocess. Word count targets per document. Browser-native spellcheck. Search result highlight in editor.

**Commits:** `52d3f14`, `0614eea`, `8d2a0b4`

---

## 2026-04-06 — Settings panel + compile pipeline

**Change:** Comprehensive Settings (General, Writing, Backup, AI, Compile). Writing settings apply to the editor dynamically. Compile dialog with title page, section separators, Shunn manuscript format preset. Per-document include-in-compile toggle, compile-order override.

**Commits:** `e48e7fa`, `64cd3f2`, `e4fce1e`, `43e6918`, `889642f`

---

## 2026-04-06 — Templates + Trash + drag-and-drop

**Change:** Document templates (Scene, Chapter, Character Sheet, Setting, Outline). Delete moves to Trash instead of permanent delete. Reimplemented drag-and-drop with mouse events instead of HTML5 drag API (more reliable).

**Commits:** `c9c8a16`, `76b09bf`, `2f13544`, `7443789`, `fecce9c`

---

## 2026-04-06 — Scrivener import + Pandoc integration

**Change:** Full Scrivener `.scriv` import with metadata, hierarchy, RTF conversion. Import of all Pandoc-supported formats. Pandoc detection with install helper. Self-healing project structure (Manuscript/Research/Trash).

**Commits:** `d2f63d1`, `10b512f`, `988f9fb`, `a1322d8`

---

## 2026-04-06 — Core app MVP

**Change:** First working end-to-end build. Tauri + React + TipTap. git2-rs for revisions. Beforeunload warning, error boundary, recent projects. Project-wide search with editor highlight.

**Commits:** `4041f9b`, `8e4fe0b`, `f77c570`

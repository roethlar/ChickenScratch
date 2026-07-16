# Plan: Tree-replacement epoch invalidation on partial failure

**Owner request (quote):**
> yes

(2026-07-15, approving the proposal to write up "tree-replacement epoch
invalidation on partial failure" — the next single safety concern ranked in
`.agents/state.md` — as a short plan for yes/no approval.)

**Phase check:** [x] Allowed by `CURRENT_PHASE.md` (Step 2 — approved safety
slices)  [x] Not paused

**Invariants touched:** I2 (fix lives entirely in `chickenscratch-core`),
I6 (no writer data loss — strengthens, never weakens, the protections),
I9 (verify before done).

---

## [MODEL] Intent

Operations that replace the files on disk — restoring a revision or a single
document, switching or merging a draft, syncing — currently invalidate
outstanding write authorizations (the "write epoch") only **after every later
step succeeds**. If such an operation fails partway, *after* the working tree
has already been changed, the epoch is never bumped: the editor's old
on-screen state is still authorized to save, and a single auto-save can
silently overwrite the just-restored files with stale text. When this is
done, any failure after the first byte of tree replacement leaves the
project refusing stale writes until it is re-probed — the writer's new
on-disk state cannot be clobbered by a half-finished operation.

## [MODEL] Approach

1. **Guard test first (prove the hole).** Add a core test that issues a
   write authorization, runs a tree-replacing operation with a failure
   injected *after* the working-tree mutation, and asserts the old
   authorization is refused. Confirm it **fails** against current code
   before any fix lands. Failure injection via a small internal refactor:
   the post-mutation tail of each operation runs through a helper the test
   can drive (or a `#[cfg(test)]` failpoint if the refactor is heavier than
   the fix — decided at implementation, whichever is smaller).
2. **Arm the bump at the point of no return.** Add a drop-scope guard in
   `fidelity.rs` (armed from the `WritePermit`) that bumps the project's
   write epoch when the scope exits — on success *and* on error. The
   in-flight operation itself is unaffected: the bump happens when the
   operation scope ends.
3. **Apply to every tree-replacing operation** in `core/git.rs`
   (`restore_document`, `restore_revision`, draft switch/merge, sync/pull):
   arm the guard immediately before the first working-tree mutation and
   remove the end-of-function `permit.bump_epoch()` calls it replaces.
   Failures *before* any mutation still bump nothing (unchanged behavior).
4. **Surfaces (revised after plan-2 review rounds 1–2):** the original claim
   that no app changes are needed was **false**. `ProjectTokens::checkout`
   (`src-tauri/src/commands/mod.rs:45`) treats a stale token as a cache miss
   and transparently re-issues a fresh one; tree-replacing commands refresh
   only on success (`src-tauri/src/commands/git.rs:57`); and the UI reloads
   only on success (`ui/src/components/revisions/Revisions.tsx:108`). So
   after a guarded partial failure the epoch bump refuses the *old* token,
   but the next auto-save silently acquires a *fresh* token and writes stale
   editor content over the partially-replaced tree. Fix at the app layer —
   revised after rounds 2–3: reload alone is neither a save barrier nor
   a buffer reset, and auto-save is not the only stale-content writer:
   - **Save barrier around every tree-replacing operation.** Reloading
     after the fact does not establish "before any further save can run":
     a debounced save already queued behind `ProjectWriteLocks` re-probes
     via `ProjectTokens::checkout` and acquires a fresh token during or
     after the reload, and `openProject` clearing `activeDoc` can itself
     trigger the editor's dirty-buffer flush. Suspend editor auto-save and
     flushes from before the first tree mutation until project reload
     *and* editor-buffer rebuild complete; only then resume saving. The
     barrier must live where the timers and buffers live: the debounce
     (`saveTimer`), dirty flag (`dirtyRef`), flush logic (`saveCurrent`),
     and flow buffers (`flowDocsRef`) are owned by
     `ui/src/components/editor/Editor.tsx`, so the
     `ui/src/components/editor/editorRef.ts` /
     `ui/src/stores/projectStore.ts` seam must expose an *awaitable*
     suspend/resume + rebuild-complete contract that `Editor.tsx`
     implements and every tree-replacing operation awaits (round 3).
   - **Gate every editor-content-bearing command, not only auto-save.**
     `ui/src/components/editor/Toolbar.tsx:116` (`addComment`) and
     `ui/src/components/comments/CommentsPanel.tsx:66` (`deleteComment`)
     serialize the live buffer (`getEditorMarkdown`) and write it via
     `docCmd.*`; invoked while a tree operation holds
     `ProjectWriteLocks` they queue, re-probe after the epoch bump, and
     land the stale buffer under a fresh token. The barrier blocks or
     defers every command that carries editor-derived content (round 3).
   - **Editing is disabled while the barrier is up.** Suspending saves
     while leaving TipTap editable creates silent data loss: keystrokes
     during an awaited restore/pull live only in the old buffer and are
     discarded by the required rebuild. Set the editor non-editable for
     the barrier window (`editor.setEditable(false)`), restoring it
     after rebuild — no reconciliation is attempted (round 3).
   - **Explicit editor-buffer reset/rebuild after reload.** `openProject`
     (`ui/src/stores/projectStore.ts:71`) clears `activeDoc` but leaves
     `flowDocs` intact, and the editor's buffer-load effect does not
     re-run just because the project reloaded — a flow-mode buffer
     survives reload and its next save writes pre-operation sections
     under a fresh token. After reload, rebuild the visible buffer from
     the reloaded documents in single-doc *and* flow mode.
   - **Every tree-mutating result kind, not only thrown errors.**
     Draft-merge and sync-pull conflicts are *successful* `Ok(Conflicts)`
     results returned after the merge has already rewritten the working
     tree (`Revisions.tsx:144`, `:228`), yet both branches only open the
     conflict dialog — no reload — and "Resolve manually" drops back to
     an editable stale buffer. Conflict branches get the same
     barrier/reload/rebuild treatment as error and success paths.
   Add coverage at this layer (see Tests).
5. Land as **one concern, one branch, one commit**, per
   `.agents/repo-guidance.md` Earned Practices.

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `crates/core/src/core/project/fidelity.rs` | Epoch-bump-on-scope-exit guard, armed via `WritePermit` |
| `crates/core/src/core/git.rs` | Arm guard before first tree mutation in each tree-replacing op; drop the success-only `bump_epoch()` calls |
| `crates/core` tests (fidelity/git) | New guard test proving stale state is refused after a partial failure |
| `ui/src/components/revisions/Revisions.tsx` (draft/sync handlers) | Barrier + reload + buffer rebuild on *every* tree-mutating result — failure, `Ok(Conflicts)` (lines 144, 228), success |
| `ui/src/components/revisions/DocumentHistory.tsx` | Same treatment for `restore_document` (currently reloads only in its success path) |
| `ui/src/components/editor/Editor.tsx` | Owns `saveTimer`/`dirtyRef`/`flowDocsRef`/`saveCurrent`: implements the awaitable suspend/resume + rebuild contract; editor non-editable while the barrier is up |
| `ui/src/stores/projectStore.ts`, `ui/src/components/editor/editorRef.ts` | Awaitable save-barrier seam; reset `flowDocs` and rebuild the editor buffer on reload |
| `ui/src/components/editor/Toolbar.tsx`, `ui/src/components/comments/CommentsPanel.tsx` | Comment commands carry `getEditorMarkdown` content — gated behind the same barrier |
| `ui/package.json` (+ vitest harness, added by this plan) | UI has no test runner today (scripts: dev/build/lint/preview only); add vitest + a `test` script so the regressions below are executable, and fold it into the declared verification suite |
| UI tests (new vitest harness) | Regressions: reload-on-failure, queued-save barrier, flow mode, conflict paths, edit overlap, comment-command gating |

## [MODEL] Tests

- [ ] New guard test: authorization issued pre-operation is refused after an
  injected post-mutation failure — shown to fail without the protection,
  pass with it.
- [ ] App-layer regression (added in round 1 revision): after a
  tree-replacing operation fails post-mutation, the UI reloads project
  state and a subsequent auto-save does **not** write the pre-operation
  editor buffer — shown to fail against current error paths, pass with
  the reload fix.
- [ ] Queued-save barrier regression (round 2): a save already queued when
  the operation fails post-mutation must not land after reload — shown to
  fail without the barrier.
- [ ] Flow-mode regression (round 2): with flow mode active, a failed or
  conflicted tree-replacing op followed by an edit must not save
  pre-operation sections (`flowDocs` survives `openProject` today).
- [ ] Conflict-path regression (round 2): draft-merge and sync-pull
  returning conflicts, then "Resolve manually" and an edit, must not save
  the pre-merge buffer.
- [ ] Edit-overlap regression (round 3): typing during an awaited
  tree-replacing operation cannot land — the editor is non-editable for
  the barrier window, and nothing typed is silently replayed or saved
  against the replaced tree.
- [ ] Comment-command regression (round 3): `addComment`/`deleteComment`
  issued around a guarded failure must not write the pre-operation
  buffer.
- [ ] Existing epoch tests (e.g. `token_stale_after_epoch_bump`) still pass.
- [ ] Full declared suite from `.agents/repo-guidance.md` Verification
  (fmt, clippy, tests, release-metadata check, ui lint/build), plus the
  new UI vitest script (round 3: the repo has no UI test runner today —
  the app-layer regressions run on the harness this plan introduces).

## [MODEL] Owner verification (plain English)

"Restore an old revision (or switch drafts) and imagine it errors halfway:
whatever the app was showing before must NOT quietly save over the files on
disk. After the fix, the app refuses that stale save and reloads the project
instead. Everyday use looks identical — this only changes what happens when
an operation breaks partway."

## [YOU] Decisions needed

- Approval to implement this plan (yes/no).
- Round 3: the app-layer regressions need a UI test harness the repo does
  not have. Approve adding vitest (dev dependency + `test` script) inside
  this plan's single commit, or direct a separate preparatory commit.

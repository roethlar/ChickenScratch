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
   arm the guard immediately before the first mutation of *ref, HEAD, or
   working tree — whichever comes first* — and remove the end-of-function
   `permit.bump_epoch()` calls it replaces. Ref moves count as mutations
   (round 6): in the fast-forward branches of `merge_draft` and
   `sync_pull`, `reference.set_target` (`git.rs:601`, `:935`) advances
   the branch ref — which HEAD already resolves through — before the
   fallible `set_head`/`checkout_head` calls. A failure between them
   would leave HEAD effectively advanced under an un-bumped epoch, and
   the next `save_revision` (stages everything, commits onto HEAD —
   `git.rs:230`–`:251`, no dirty/staleness check) would silently revert
   the pulled/merged content with the stale working tree. Arm before
   `set_target` in those two branches. `switch_draft` needs no special
   arming point: its only fallible step after the HEAD move is the
   checkout itself, which the drop guard armed before `checkout_head`
   already covers (round-6 triage narrowed the finding here). Failures
   before any ref/HEAD/tree mutation still bump nothing.
4. **Surfaces (revised across plan-2 review rounds 1–6):** the original claim
   that no app changes are needed was **false**. `ProjectTokens::checkout`
   (`src-tauri/src/commands/mod.rs:45`) treats a stale token as a cache miss
   and transparently re-issues a fresh one; tree-replacing commands refresh
   only on success (`src-tauri/src/commands/git.rs:57`); and the UI reloads
   only on success (`ui/src/components/revisions/Revisions.tsx:108`). So
   after a guarded partial failure the epoch bump refuses the *old* token,
   but the next auto-save silently acquires a *fresh* token and writes stale
   editor content over the partially-replaced tree. Fix at the app layer —
   revised after rounds 2–6: reload alone is neither a save barrier nor
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
   - **Editing is disabled while the barrier is up — DOM *and*
     programmatic.** Suspending saves while leaving TipTap editable
     creates silent data loss: keystrokes during an awaited restore/pull
     live only in the old buffer and are discarded by the required
     rebuild. Set the editor non-editable for the barrier window
     (`editor.setEditable(false)`), restoring it after rebuild — no
     reconciliation is attempted (round 3). `setEditable(false)` blocks
     only DOM input, not command dispatch: Toolbar formatting/link/
     footnote handlers, `FindReplace.tsx` replace ops (`:104`–`:125`),
     and the streaming AI path (`Toolbar.tsx:409`,
     `editor.commands.insertContentAt` per delta) all mutate the buffer
     programmatically while non-editable. The barrier therefore also
     exposes a barrier-active state that every programmatic dispatch
     site checks (no-op while active), and barrier entry cancels or
     awaits in-flight AI transform streams
     (`cancelAiTransformStream` + abort) before the first tree
     mutation (round 4).
   - **The barrier is a counted lease, not a boolean.** `syncBusy`
     gates only fetch/pull/push (`Revisions.tsx:564`–`:574`) and the
     conflict dialog (`:519`–`:525`); restore (`:353`), draft switch
     (`:415`), and merge (`:421`) have no busy guard, so overlapping
     tree-replacing operations are reachable and queue under
     `ProjectWriteLocks`. A boolean suspend/resume flag would let the
     first completion re-enable editing while the second still runs,
     whose rebuild then discards those edits. The contract is
     acquire/release with a count: editing and command dispatch resume
     only when the *last* lease releases. Belt-and-suspenders, all
     tree-replacing triggers are disabled while any lease is held —
     extend the `syncBusy`-style gating to restore/switch/merge
     (round 4). Counting alone does not order the lifecycles: an
     earlier operation can reload, then a queued later operation
     mutates disk, and the earlier rebuild completes last — releasing
     the final lease with the editor on the earlier snapshot while
     disk reflects the later operation. Serialize the complete
     operation-through-rebuild lifecycle (one tree-replacing
     operation admitted at a time), or have the *final* release
     perform a generation-checked fresh rebuild against current disk
     state (round 5).
   - **Freeze before drain.** Every existing trigger first runs
     `await flushPendingEditorSave()` and only then starts the tree
     operation (`Revisions.tsx:75`, `DocumentHistory.tsx:39`/`:70`,
     `App.tsx:202`/`:267`/`:297`). Typing during that in-flight
     drain schedules a new save, and the completing flush
     unconditionally clears the dirty flag (`Editor.tsx:121`,
     `:195`) — barrier entry then cancels the queued timer and the
     rebuild discards the keystrokes. Barrier entry must freeze
     editing and command dispatch *before* the pre-operation drain,
     with the drain itself running under the lease (round 5).
   - **Gate the mutation dispatch layer, not an allowlist of writers.**
     Round 5 named `Inspector.tsx`'s debounced metadata buffer
     (`setTimeout(save, 1500)`, `:364`) and `Corkboard.tsx`
     `handleSummarizeAll` (`:65`–`:93`). Round 6 showed enumeration
     cannot converge: `StatsPanel.tsx:33` (`recordDailyWords` →
     `settings/writing-history.json`), `Preview.tsx:79` `saveMeta` and
     `session.ts:20` `updateSessionTarget` (each re-submits a whole
     captured metadata snapshot via `update_project_metadata`), comment
     updates (`CommentsPanel.tsx:43`/`:78`), thread ops, Corkboard
     linking (`:97`), every Binder mutation, Inspector's *immediate*
     onChange handlers (`:595`/`:622`/`:665`), and App's Ctrl+N handler
     all dispatch project mutations from captured UI state, queue
     behind `ProjectWriteLocks`, and re-probe under the fresh epoch.
     There is no barrier seam today — all ten `ui/src/commands/*.ts`
     modules call `invoke` directly — so add one shared gate in that
     layer: every project-mutating dispatch awaits (or is refused
     while) the lease, and pre-lease in-flight dispatches drain at
     barrier entry (round 6).
   - **Stale-snapshot forms survive reload — resync them.** The
     `Preview.tsx` metadata form resyncs only when `project.path`
     changes (`:66`–`:77`), and `SessionTargetSection` re-submits the
     entire captured `project.metadata` (`session.ts:26`–`:35`). A
     same-path reload after a tree operation therefore leaves them
     holding pre-operation values that one Save click writes wholesale
     over restored state — no queue race required, untouched by
     reload+rebuild. Resync captured-snapshot forms on project reload,
     or version their snapshots so post-reload stale submissions are
     refused (round 6).
   - **App-level revision writers and unresolved conflicts.** The
     auto-commit interval (`App.tsx:261`–`:276`; flush `:267`,
     `saveRevision` `:273`), the periodic backup timer (`:290`–`:303`,
     reusing `backup_on_close` at `:298`), and the close path
     (`flushAndBackupOnClose` `:196`–`:215`) continue past the editor
     flush into git writes that carry no editor content and so escape
     every buffer-level gate. `backup_on_close`
     (`src-tauri/src/commands/git.rs:329`; dirty check `:344`,
     `save_revision` `:345`) and core `save_revision`
     (`git.rs:217`–`:255`, `add_all(["*"])`) check nothing about merge
     state, so after `Ok(Conflicts)` — which refreshes the token
     because it is `is_ok()` — a timer tick or app close commits
     conflict markers wholesale into history (the F-009 fix rerouted
     only `merge_draft`'s internal path). Reachable *today* with no
     operation overlap: any unresolved-conflict window longer than the
     10-minute timer, or a close during resolution. Fix in two layers:
     (a) core-side, per I2 — `save_revision`, and therefore
     `backup_on_close`, refuses while merge state is unresolved
     (`repo.state()` / `MERGE_HEAD` / `index.has_conflicts()`);
     (b) app-side belt-and-suspenders — timer and close continuations
     are skipped/cancelled while a lease is held (round 6).
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
| `ui/src/components/revisions/Revisions.tsx` (draft/sync handlers) | Barrier + reload + buffer rebuild on *every* tree-mutating result — failure, `Ok(Conflicts)` (lines 144, 228), success; disable restore/draft-switch/merge triggers while any barrier lease is held (round 4) |
| `ui/src/components/revisions/DocumentHistory.tsx` | Same treatment for `restore_document` (currently reloads only in its success path) |
| `ui/src/components/editor/Editor.tsx` | Owns `saveTimer`/`dirtyRef`/`flowDocsRef`/`saveCurrent`: implements the awaitable suspend/resume + rebuild contract; editor non-editable while the barrier is up; freeze-before-drain — barrier entry precedes the pre-operation flush, whose unconditional mark-clean (`:121`, `:195`) otherwise loses mid-drain keystrokes (round 5) |
| `ui/src/stores/projectStore.ts`, `ui/src/components/editor/editorRef.ts` | Awaitable save-barrier seam; reset `flowDocs` and rebuild the editor buffer on reload |
| `ui/src/components/editor/Toolbar.tsx`, `ui/src/components/comments/CommentsPanel.tsx`, `ui/src/components/editor/FindReplace.tsx` | Comment commands carry `getEditorMarkdown` content; Toolbar formatting/AI and FindReplace mutate the buffer via command dispatch — all check the barrier-active state; in-flight AI streams cancelled/awaited at barrier entry (round 4) |
| `ui/src/commands/*.ts` (one shared dispatch gate) | No barrier seam exists — all ten command modules call `invoke` directly. One shared gate: every project-mutating dispatch awaits/refuses the lease; in-flight dispatches drain at barrier entry (round 6 — supersedes round 5's per-file gating of `Inspector.tsx`/`Corkboard.tsx`, which remain covered through the seam) |
| `ui/src/components/preview/Preview.tsx`, `ui/src/commands/session.ts`, `ui/src/components/stats/StatsPanel.tsx` | Stale-snapshot writers: meta form resyncs only on path change (`:66`–`:77`); session target re-submits captured metadata wholesale; stats effect fire-and-forgets `recordDailyWords` (`:33`) — resync on reload or refuse post-reload stale submissions (round 6) |
| `ui/src/App.tsx` (auto-commit `:261`, backup timer `:290`, close path `:196`) | Git-write continuations gated by the lease and skipped/cancelled while a lease is held or conflicts are unresolved (round 6) |
| `crates/core/src/core/git.rs` `save_revision` (+ `src-tauri/src/commands/git.rs` `backup_on_close`) | Refuse to commit while merge state is unresolved (`MERGE_HEAD` / `has_conflicts`) — closes the today-reachable conflict-markers commit; core-side per I2 (round 6) |
| `ui/package.json` (+ vitest harness, added by this plan) | UI has no test runner today (scripts: dev/build/lint/preview only); add vitest + a `test` script so the regressions below are executable, and fold it into the declared verification suite |
| UI tests (new vitest harness) | Regressions: reload-on-failure, queued-save barrier, flow mode, conflict paths, edit overlap, comment-command gating, programmatic-dispatch gating, overlapping operations (assert final buffer contents), preflight typing, dispatch-gate + stale-snapshot-form gating, timer/close overlap |
| `.github/workflows/validation.yml`, `.agents/repo-guidance.md` (Verification) | CI currently runs only UI lint/build; add a UI test step and fold the vitest suite into the declared-suite guidance so the regressions cannot fail unnoticed (round 4) |

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
- [ ] Edit-overlap regression (rounds 3–4): typing during an awaited
  tree-replacing operation cannot land — the editor is non-editable for
  the barrier window, and nothing typed is silently replayed or saved
  against the replaced tree. Extended (round 4): programmatic dispatch
  (Toolbar formatting, `FindReplace` replace) is refused while a lease
  is held, and an AI transform stream started before barrier entry
  cannot insert deltas during or after it — shown to fail with
  `setEditable(false)` alone.
- [ ] Overlapping-operation regression (round 4): two tree-replacing
  operations overlapping under `ProjectWriteLocks` — editing and
  command dispatch stay suspended until the *last* lease releases;
  shown to fail with a boolean suspend/resume flag. Extended
  (round 5): the test asserts the *final editor buffer contents*
  match the last operation's disk state, not merely that suspension
  held — shown to fail when the earlier operation's rebuild
  completes last.
- [ ] Preflight-typing regression (round 5): typing while the
  pre-operation `flushPendingEditorSave()` drain is in flight must
  not be silently lost — shown to fail against the current
  unconditional mark-clean (`Editor.tsx:121`, `:195`) without
  freeze-before-drain ordering.
- [ ] Metadata-writer regression (round 5): a pending `Inspector.tsx`
  debounced metadata save or an in-flight `Corkboard.tsx`
  `aiSummarize` batch spanning a tree-replacing operation must not
  overwrite restored metadata with pre-operation values.
- [ ] Ref-move boundary test (round 6): in the `merge_draft` and
  `sync_pull` fast-forward branches, inject a failure between
  `reference.set_target` and the checkout (simulating `set_head`
  failure) — the epoch must already be bumped and the stale permit
  refused; shown to fail while the guard arms only before the
  working-tree write.
- [ ] Dispatch-gate regression (round 6): a representative
  snapshot-clobber writer (session-target save re-submitting six
  captured metadata fields) queued behind a tree-replacing operation
  must not land under a fresh token; and a post-reload stale
  `Preview.tsx` meta submission is refused or the form has been
  resynced.
- [ ] Unresolved-conflict regression (round 6): with merge state in
  progress (`MERGE_HEAD` present / `index.has_conflicts()`), the
  auto-commit timer, `backup_on_close`, and manual `save_revision`
  all refuse to commit — shown to fail today (conflict markers are
  staged wholesale by `add_all(["*"])` and committed).
- [ ] Timer/close overlap regression (round 6): an auto-commit or
  backup continuation that queued behind `ProjectWriteLocks` during a
  guarded operation must not commit a half-replaced or conflicted
  tree after the lock releases.
- [ ] Comment-command regression (round 3): `addComment`/`deleteComment`
  issued around a guarded failure must not write the pre-operation
  buffer.
- [ ] Existing epoch tests (e.g. `token_stale_after_epoch_bump`) still pass.
- [ ] Full declared suite from `.agents/repo-guidance.md` Verification
  (fmt, clippy, tests, release-metadata check, ui lint/build), plus the
  new UI vitest script (round 3: the repo has no UI test runner today —
  the app-layer regressions run on the harness this plan introduces).
  Round 4: `.github/workflows/validation.yml` runs only UI lint/build
  today — add a UI test step and update the declared-suite guidance so
  the new regressions run in CI, not only locally.

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
- Round 4: durable verification touches CI
  (`.github/workflows/validation.yml` gains a UI test step) and the
  declared-suite guidance in `.agents/repo-guidance.md` — confirm these
  ride in this plan's single commit or direct a split.
- Round 6: the unresolved-conflict commit (auto-commit/backup can bake
  conflict markers permanently into history) is reachable **today**,
  independent of this plan's failure window. Its core-side fix
  (`save_revision` refuses during merge state) rides in this slice
  because the slice's promise is hollow without it — but it is
  separable; direct a split into its own slice if preferred.

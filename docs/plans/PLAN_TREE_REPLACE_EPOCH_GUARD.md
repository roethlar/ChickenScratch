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
     layer, with semantics tightened in round 7:
     - *Refuse, never defer.* A dispatch attempted while a lease is
       held is refused/cancelled (surfaced as an error, retried only
       from fresh post-reload state) or carries a generation stamp
       validated at send time. Plain deferral is never compliant:
       a queued dispatch keeps its captured pre-operation arguments
       and would land them under a fresh token after release —
       exactly the clobber the gate exists to stop (round 7).
       Pre-lease in-flight dispatches still drain at barrier entry.
     - *Owner-scoped admission.* Barrier entry returns a lease
       handle; dispatches made under that handle bypass the gate.
       Without this the contract is self-contradictory: the
       pre-operation drain (`docCmd.updateDocumentContent` via
       `Editor.tsx:189`/`:209`) and the tree-replacing command
       itself (`gitCmd.restoreRevision` et al.) are project-mutating
       dispatches through the same gated layer, so a literal
       await-gate deadlocks its own lifecycle and a literal
       refuse-gate self-aborts every operation with a dirty buffer.
       The exemption covers every trigger site (`Revisions.tsx`,
       `DocumentHistory.tsx`, `App.tsx` flush paths) (round 7) —
       *and the post-operation reload itself* (round 8):
       `load_project` is permit-backed and conditionally
       disk-mutating (`src-tauri/src/commands/project.rs:55`–`:59`
       acquires a `WritePermit` and runs `read_project_with_repair`,
       which self-heals missing standard folders and refreshes the
       token cache at `:60`), so the gate must classify it as
       project-mutating; the mandated reload
       (`openProject`/`loadProject` from `Revisions.tsx`,
       `DocumentHistory.tsx:75`) runs while the lease is held and
       must be lease-handle-admitted, or the barrier refuses its own
       recovery path (the reload error is even swallowed silently by
       `projectStore.ts:81`–`:82`) — while the same dispatch without
       the handle stays refused.
     - *Close the seam.* `Preview.tsx:3` imports `invoke` directly
       and `saveMeta` (`:81`) calls `update_project_metadata`
       outside `ui/src/commands/*` — the only project-mutating
       component-level bypass (App.tsx's `backup_on_close` calls are
       handled by the timer/close bullet). Migrate `saveMeta` into
       the commands layer (`session.ts` already wraps the same
       command) and add an ESLint `no-restricted-imports` rule
       scoped to `ui/src/components` so the seam stays closed
       (round 7).
   - **Stale-snapshot forms survive reload — resync them.** The
     `Preview.tsx` metadata form resyncs only when `project.path`
     changes (`:66`–`:77`), `SessionTargetSection` re-submits the
     entire captured `project.metadata` (`session.ts:26`–`:35`), and
     — round 8 — `Inspector.tsx` resyncs its metadata form only when
     the *document id* changes (`:280`–`:307`): after a same-doc
     restore, re-selecting the doc shows pre-operation fields, and
     one debounce tick (`:364`), immediate handler
     (`:595`/`:622`/`:665`), or stale title blur (`renameNode`,
     `:356`) re-submits the full stale snapshot post-release, under
     no lease at all. Resync must therefore key on reload
     generation/content, never on id or path alone. A same-path
     reload after a tree operation otherwise leaves these forms
     holding pre-operation values that one Save click writes
     wholesale over restored state — no queue race required,
     untouched by reload+rebuild. Treat these forms like the editor
     (round 7):
     disable their inputs and Save while any lease is held; on
     reload, resync only non-dirty forms/fields; a dirty draft that
     cannot be preserved is dropped *loudly* (explicit notice), never
     silently — bare resync or versioned refusal alone would discard
     a draft typed before/during the operation without a trace.
     (Unlike the editor, these forms have no pre-operation flush
     path, so freezing alone does not persist an already-dirty form —
     hence the loud-drop requirement.)
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
     (a) core-side, per I2 — automatic writers (`backup_on_close`, the
     auto-commit path) refuse while merge state is unresolved;
     (b) app-side belt-and-suspenders — timer and close continuations
     are skipped/cancelled while a lease is held (round 6).
     Round 7 correction — a *blanket* `save_revision` refusal would
     strand the writer: "Resolve manually" only closes the dialog
     (`Revisions.tsx:470`), no continue-merge command exists anywhere
     (the only `cleanup_state` calls are the clean-merge paths,
     unreachable after `Ok(Conflicts)`, and the two aborts), the
     index stays conflicted after marker edits until *staged* — and
     the app's only staging call is `save_revision`'s own
     `add_all(["*"])`. Worse, `restore_revision`, `restore_document`,
     and `backup_current_work` all call `save_revision` internally,
     so a blanket refusal bricks restore and manual backup too; the
     writer's only exits (abort / force-overwrite) discard the
     resolution. Round 8 killed the round-7 repair (a completion
     shape *inside* `save_revision`) on three verified grounds:
     `git2::Index::add_all` clears conflict entries *unconditionally*
     (verified in the pinned git2 0.19.0 / libgit2 1.8.1 sources —
     staging marks a path resolved regardless of whether markers were
     edited), so no ordering of check/stage/commit both refuses
     untouched markers and admits resolved-but-unstaged edits; core
     `save_revision` cannot distinguish callers (auto-commit uses the
     *same* Tauri command and args as manual save, and
     `backup_current_work` has no dirty-worktree guard), so any
     in-function completion lets a timer tick or backup click
     silently mint a merge commit mid-resolution — and with a
     lingering `MERGE_HEAD`, restore would record a false merge
     parent; and an epoch bump inside `save_revision` invalidates the
     caller's still-live permit, so `backup_current_work`'s
     `push_backup` (`git.rs:744`) and the Tauri command's
     backup/remote pushes (`src-tauri/src/commands/git.rs:24`/`:32`)
     fail or are silently lost after the merge-completing commit.
     The design that survives (rounds 8–9):
     (a) `save_revision` refuses during *any* merge state
     (`MERGE_HEAD` present or index conflicts) — for every caller,
     no provenance needed. Refusal alone is too low in the call
     graph for composite operations (round 9): `restore_document`
     and `restore_revision` pass the status-only dirty guard under a
     clean tree + lingering `MERGE_HEAD`, replace disk content
     (`git.rs:406`–`:411`, `:446`–`:450`), and only *then* hit the
     internal refusal — a half-completed restore whose only exits
     would falsify history (Complete folds the restored tree into a
     fake merge resolution) or discard it (Abort). Both restore
     helpers therefore run a merge-state preflight next to
     `reject_dirty_worktree`, before any disk write. Backup is
     different (round-9 triage): `push_backup` sends only the branch
     ref — a push during lingering `MERGE_HEAD` is benign and
     refusing it would reduce backup protection for zero integrity
     gain — so manual backup refuses only the *commit* half and
     still pushes; `backup_on_close` must not swallow the refusal
     silently (today it discards errors with `let _`).
     Project creation and import never run under merge state and are
     unaffected. The TUI (round 9): document saves are pure file
     writes and genuinely unaffected, but TUI *revision* saves call
     core `save_revision` directly (`tui/app.rs:974`) with no
     merge-state UI at all — the refusal must carry a
     self-describing message ("a merge is in progress — complete or
     abort it in ChickenScratch"). Round-10 correction: the TUI does
     *not* print operation errors verbatim — the revision-failure
     branch formats with `{:?}` (`app.rs:992`, backup errors `:980`;
     only the token-refusal branch at `:967` uses Display) — so the
     TUI scope includes switching those branches to Display
     formatting, with a test on the rendered message. Full TUI
     Complete/Abort parity is not warranted (the TUI cannot create
     merge state).
     (b) A new core `complete_merge(path, message, permit)` is the
     *only* completion path: requires merge state; the user's
     explicit invocation is the resolution signal (replacing the
     impossible index-state detection); stages everything, commits
     with two parents (`HEAD`, `MERGE_HEAD`), `cleanup_state`; the
     epoch bump rides the step-2 drop-scope guard at operation exit
     (never inline), so no caller continuation loses its permit.
     `complete_merge` is an **epoch-bumping operation** and gets the
     full barrier lifecycle (round 9): the plan's barrier rules key
     on epoch-bumping operations, not only tree-replacing ones —
     lease, freeze-before-drain, editor flush and dispatch under the
     owner handle, reload+rebuild, release. Without the flush, a
     writer who resolves markers in the editor and clicks Complete
     before the debounce fires (up to `auto_save_seconds`) commits
     the still-marker-laden disk state as the permanent two-parent
     merge commit, with the real resolution landing afterwards as an
     ordinary edit. Note for implementers: the pre-Complete flush is
     NOT blocked by (a)'s refusal — the editor drain goes through
     `update_document_content` → the writer, never `save_revision`.
     Abort deliberately skips the flush (the buffer holds edits
     being discarded); do not blanket-apply one rule to both exits.
     (c) UI: "Resolve manually" no longer just closes the dialog —
     the app enters a persistent merge-in-progress state (backend
     merge-state query added; none exists today, and the UI must
     survive restart) offering *Complete merge* (→ the new command,
     with confirmation — which narrows but does not close the
     debounce race; the flush in (b) closes it) and *Abort*.
     Restore refusing during merge state (pre-mutation) is correct
     behavior, and the stranded-writer exit is Complete merge.
     (e) Recovery-scoped authority (round 9): `complete_merge`,
     `sync_abort_pull`, and `sync_pull_force` cannot ride an
     ordinary Full-fidelity `WritePermit` — a merge conflict that
     touches `project.yaml` makes the fidelity probe *error*
     (`fidelity.rs:333`–`:335`, so `load_project` won't even open
     the project after restart), and one in a `.meta` probes
     Degraded; either way `ProjectTokens` cannot reissue a permit
     and the promised recovery is unreachable exactly when the
     conflict hits a format file. (This is a live bug today —
     `sync_abort_pull` is already permit-gated — recorded as its own
     ranked finding.) Issue a narrow recovery capability when the
     merge-state query attests an in-progress merge (keyed on
     `MERGE_HEAD`/index conflicts — `DegradedReason` has no merge
     variant and the `project.yaml` case errors before
     classification, so fidelity reasons cannot carry this),
     authorizing only those three commands. The capability carries
     the same safety contract as a `WritePermit` (round 10): non-
     `Clone`, engine-only construction; bound to the canonical
     project root and validated against the target path at use
     (`ensure_valid_root`-equivalent); merge state re-attested at
     use, not only at issue; and it can arm the step-2 drop-scope
     guard — the guard's arming surface is "permit or recovery
     capability", not `WritePermit` alone, or `complete_merge` under
     recovery could not bump the epoch on exit. Negative tests:
     wrong-root use refused; issue/use outside merge state refused.
     `sync_pull_force` needs more than authorization (round 10): its
     own `reject_dirty_worktree` calls (`git.rs:1042`, `:1059`) fire
     on every conflicted tree — conflicts *are* status-dirty — and
     `revalidate_fidelity` (`:1045`) fails on format-file conflicts,
     so the conflict dialog's "Overwrite local with remote" exit is
     unreachable today for any real conflict (live bug, recorded
     with the abort finding). Under an attested merge state the
     force path replaces those checks (the dirty tree is the
     conflict being discarded — that is the command's purpose);
     ordinary force-pull outside merge state keeps them unchanged.
     The read-only open path tolerates an unparsable `project.yaml`
     while `MERGE_HEAD` is present (fall back to the pre-merge
     `HEAD` version for display). Reachability (round 10): today's
     reader cannot express this — `read_project_readonly` →
     `read_project_impl` unconditionally parses the worktree
     `project.yaml` (`reader.rs:311`) — so `reader.rs` enters scope
     with a read-only entry point that accepts verified HEAD
     metadata while preserving the root/safe-read checks, and the
     recovery test must assert `load_project` itself succeeds after
     restart with a conflicted `project.yaml`, not only that the
     recovery commands run.
     (d) Migration: projects carrying a lingering `MERGE_HEAD` from
     today's code hit the preflight/refusal once and are prompted to
     Complete (two-parent commit heals the history) or Abort — not
     bricked. The migration prompt invokes the same `complete_merge`
     and gets the same (b) lifecycle.
     Ordering constraint, stated so it does not rot: any operation
     that uses its permit *after* an internal `save_revision`/
     `complete_merge` call must sequence those uses before any epoch
     bump or reauthorize; `restore_document`/`restore_revision` are
     safe today only because nothing validates the permit after
     their internal save (rounds 7–8).
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
| `ui/src/commands/*.ts` (one shared dispatch gate) | One shared gate: non-owner project-mutating dispatches during a lease are **refused/cancelled** (never deferred with captured args); barrier entry returns a lease handle whose dispatches (drain, the operation itself, **the post-operation reload** — `load_project` is permit-backed and conditionally disk-mutating, round 8) bypass the gate; pre-lease in-flight dispatches drain at barrier entry (rounds 6–8 — supersedes round 5's per-file gating; `Preview.tsx` `saveMeta` migrates into this layer, and an ESLint `no-restricted-imports` rule keeps components off direct `invoke`) |
| `ui/src/components/preview/Preview.tsx`, `ui/src/commands/session.ts`, `ui/src/components/stats/StatsPanel.tsx`, `ui/src/components/inspector/Inspector.tsx` (round 8) | Stale-snapshot writers: forms frozen (inputs + Save disabled) while a lease is held; on reload resync only non-dirty fields, keyed on reload generation/content — Inspector's id-keyed resync (`:280`–`:307`) misses same-doc restores and its debounce/immediate handlers/title blur then clobber post-release; dirty drafts that cannot be preserved are dropped loudly, never silently (rounds 6–8) |
| `ui/src/App.tsx` (auto-commit `:261`, backup timer `:290`, close path `:196`) | Git-write continuations gated by the lease and skipped/cancelled while a lease is held or conflicts are unresolved (round 6) |
| `crates/core/src/core/git.rs` `save_revision` + new `complete_merge` (+ Tauri commands: merge-state query, `complete_merge`, recovery capability) | Rounds 8–9: `save_revision` refuses during any merge state (self-describing error — the TUI prints it verbatim); merge-state preflight in `restore_document`/`restore_revision` before any disk write; manual backup refuses only the commit half (push is benign) and `backup_on_close` surfaces the refusal instead of `let _`; new explicit `complete_merge` (stage, two-parent commit, `cleanup_state`; epoch via drop guard at scope exit; full barrier lifecycle incl. pre-dispatch editor flush); recovery-scoped capability keyed on merge state authorizes `complete_merge`/`sync_abort_pull`/`sync_pull_force` — with a `WritePermit`-equivalent contract (engine-only, root-bound, merge re-attested at use, drop-guard armable; round 10) — and the merge-attested force path replaces `sync_pull_force`'s dirty/fidelity checks (round 10); read-only open tolerates unparsable `project.yaml` under `MERGE_HEAD`; UI merge-in-progress state (survives restart) offers Complete/Abort; lingering `MERGE_HEAD` migrates via the same prompt |
| `crates/core/src/core/project/reader.rs` | Read-only entry point accepting verified HEAD metadata (root/safe-read checks preserved) so the merge-in-progress open works with a conflicted `project.yaml` (round 10) |
| `crates/tui/src/app.rs` (`:980`, `:992`) | Operation/backup error branches switch from `{:?}` to Display so the self-describing merge-state refusal renders readably (round 10) |
| `ui/package.json` (+ vitest harness, added by this plan) | UI has no test runner today (scripts: dev/build/lint/preview only); add vitest + a `test` script so the regressions below are executable, and fold it into the declared verification suite |
| UI tests (new vitest harness) | Regressions: reload-on-failure, queued-save barrier, flow mode, conflict paths, edit overlap, comment-command gating, programmatic-dispatch gating, overlapping operations (assert final buffer contents), preflight typing, dispatch-gate refuse-not-defer + owner admission, form freeze/loud-drop, timer/close overlap |
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
- [ ] Dispatch-gate regression (rounds 6–7): a representative
  snapshot-clobber writer (session-target save re-submitting six
  captured metadata fields) *attempted while a lease is held* is
  refused — shown to fail against a defer-then-send gate, whose
  queued dispatch lands captured pre-operation args under a fresh
  token after release. A gated Preview `saveMeta` (migrated into the
  commands layer) attempted mid-operation is likewise refused. The
  barrier's own drain, operation, *and post-operation reload*
  dispatches (lease-handle path) succeed — shown to
  deadlock/self-abort without owner admission (round 8: a reload
  dispatched under the handle succeeds; the same `loadProject`
  without the handle is refused).
- [ ] Form-freeze regression (rounds 7–8): Preview meta,
  session-target, and Inspector inputs are disabled while a lease is
  held; a dirty draft held at barrier entry either survives reload or
  its loss is surfaced with an explicit notice — a silent discard
  fails the test ("refused or resynced" alone does not pass).
  Post-release case (round 8): after a same-doc restore and lease
  release, re-selecting the doc shows *restored* metadata (Inspector
  resync keyed on reload generation), and a subsequent debounce tick
  or immediate handler does not re-submit pre-operation fields —
  shown to fail with id-keyed resync.
- [ ] Unresolved-conflict regression (rounds 6–9): during merge
  state, `save_revision` refuses for *every* caller — the
  auto-commit timer, `backup_on_close`, manual save,
  `backup_current_work`, and the restore helpers' internal saves —
  shown to fail today (conflict markers are staged wholesale by
  `add_all(["*"])` and committed; `backup_current_work` has no dirty
  guard; restore under a lingering `MERGE_HEAD` would mint a false
  merge parent). Restore preflight (round 9): restore under merge
  state refuses with **zero worktree mutation** — the tree contents
  are asserted unchanged; shown to fail with the refusal only inside
  the internal `save_revision` (post-mutation failure passes the
  weaker test). Manual backup during lingering `MERGE_HEAD` still
  pushes (commit half refused, push half proceeds);
  `backup_on_close` surfaces the refusal. Completion path
  (round 8): explicit `complete_merge` stages, commits with two
  parents (`HEAD`, `MERGE_HEAD`), clears merge state; the epoch bump
  fires at scope exit so a caller continuation (e.g. `push_backup`)
  still holds a valid permit — shown to fail with an inline bump. A
  lingering pre-existing `MERGE_HEAD` surfaces the
  Complete-or-Abort prompt rather than bricking; `complete_merge`
  outside merge state refuses.
- [ ] Complete-merge lifecycle regression (round 9): resolve markers
  in the editor, click Complete before the debounce fires — the
  merge commit's tree contains the resolved content, not markers,
  and nothing typed is discarded; shown to fail without
  freeze-before-drain + flush-under-owner-handle on the Complete
  action.
- [ ] Format-file-conflict recovery regression (rounds 9–10): a
  `sync_pull`/`merge_draft` conflict touching `project.yaml` (and,
  separately, only a `.meta`) can still be Aborted, Completed, AND
  Force-overwritten through a fresh command boundary (fresh
  `ProjectTokens`, simulating restart) via the recovery capability —
  shown to fail today and under an ordinary-permit-only design
  (probe errors or Degraded → no permit → recovery unreachable;
  force additionally blocked by its own dirty checks on every
  conflicted tree). `load_project` itself succeeds after restart
  with the conflicted `project.yaml` (HEAD-metadata fallback) — the
  recovery-commands assertion alone does not prove the display
  path. Capability negative tests: wrong-root use refused;
  issue/use outside merge state refused. TUI: the merge-state
  refusal renders as Display text in the status line, not a `{:?}`
  wrapper.
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
- Round 6 (amended rounds 7–8): the unresolved-conflict commit
  (auto-commit/backup can bake conflict markers permanently into
  history) is reachable **today**, independent of this plan's failure
  window. Round 7 established a blanket refusal would strand manual
  resolution; round 8 established the round-7 repair (completion
  inside `save_revision`) is unimplementable (`add_all` clears
  conflict entries unconditionally; no caller provenance; inline
  epoch bump breaks caller continuations). The surviving design is
  a blanket `save_revision` refusal plus a new explicit
  `complete_merge` command, a backend merge-state query, and a
  merge-in-progress UI state with Complete/Abort. That is a real
  sub-feature riding in this slice because the slice's promise is
  hollow without it — but it is now clearly the largest separable
  sub-slice; direct a split into its own slice if preferred.

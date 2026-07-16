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
4. **Surfaces:** no Tauri/TUI changes expected — both already re-probe and
   reload when a write is refused as stale. Verify that path still holds;
   touch app code only if a call site demonstrably needs it.
5. Land as **one concern, one branch, one commit**, per
   `.agents/repo-guidance.md` Earned Practices.

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `crates/core/src/core/project/fidelity.rs` | Epoch-bump-on-scope-exit guard, armed via `WritePermit` |
| `crates/core/src/core/git.rs` | Arm guard before first tree mutation in each tree-replacing op; drop the success-only `bump_epoch()` calls |
| `crates/core` tests (fidelity/git) | New guard test proving stale state is refused after a partial failure |

## [MODEL] Tests

- [ ] New guard test: authorization issued pre-operation is refused after an
  injected post-mutation failure — shown to fail without the protection,
  pass with it.
- [ ] Existing epoch tests (e.g. `token_stale_after_epoch_bump`) still pass.
- [ ] Full declared suite from `.agents/repo-guidance.md` Verification
  (fmt, clippy, tests, release-metadata check, ui lint/build).

## [MODEL] Owner verification (plain English)

"Restore an old revision (or switch drafts) and imagine it errors halfway:
whatever the app was showing before must NOT quietly save over the files on
disk. After the fix, the app refuses that stale save and reloads the project
instead. Everyday use looks identical — this only changes what happens when
an operation breaks partway."

## [YOU] Decisions needed

- Approval to implement this plan (yes/no). No other open questions.

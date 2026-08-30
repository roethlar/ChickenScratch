# dp-2: Restore confirm promises a History save that never happens

**Severity**: MEDIUM — with unsaved-to-History edits, the promised save
doesn't occur and the restore errors out after the writer already confirmed;
confusing refusal, no data loss (the engine refuses rather than discards).
**Status**: Verified
**Branch**: — (default-branch mode)
**Commit**: `3b47d6f`

## Reviewer comments
Reviewer: codex / gpt-5.6-sol / xhigh / standard
Harness: codex-cli (MCP transport), thread 01a0545b-6929-7ca3-8ac4-796a70a7b1da
Reviewed: 3b47d6f358b34069090ac15cd62ea30755f35091
Base: 31c55c29c79cd36541e3e12f6cc256fb513fd4ed
guard_confirmed: true (sequencing reverted in reviewer's worktree →
restoreFlow guard FAIL; head restored → PASS)
Verdict: accepted — 2026-08-29 (UTC). Comments:
- ui/src/components/revisions/restoreFlow.ts:40 — dirty worktrees saved
  under the active lease before restore; revert failed the guard, head
  passed.
- ui/src/components/revisions/Revisions.tsx:137 — handleRestore routes
  through restoreWithHistorySave; targeted lint and production build pass.
Record committed as part of verification history.

## Evidence
`ui/src/components/revisions/Revisions.tsx:126` (as of `8d69a08`) — confirm
text: "Bring back this version? Your current version is saved to History
first — nothing is lost." (The pre-range text made the same claim: "Your
current work will be preserved as a new revision.")
`crates/core/src/core/git.rs` `restore_revision` —
`reject_dirty_worktree(&repo, "restore this revision")` runs before any
restore; nothing in the UI flow (`handleRestore` → `runEpochOperation` →
`restoreRevision`) saves a revision first.

## Predicted observable failure
Writer edits, autosave dirties the worktree, writer clicks "Bring back this
version" and confirms. The barrier drains the buffer to disk, then
`restore_revision` fails on the dirty tree: error toast, no restore, and no
History entry — the opposite of what the dialog just promised.

## What
The confirmation documents a save-then-restore flow the code never had; the
in-range rewording strengthened the false promise.

## Approach
Make the promise true: inside the restore epoch operation (buffer already
drained), if the worktree has changes, save a revision ("Saved automatically
before bringing back an older version") under the same lease, then restore.
Sequencing lives in a small injectable helper (`restoreFlow.ts`) so the
order — save strictly before restore, abort on save failure — is unit-tested
without mounting the panel. `saveRevision` gains an optional lease so it can
run under the active barrier. No engine change (I2 intact).

## Files changed
- `ui/src/components/revisions/restoreFlow.ts` — new sequencing helper
- `ui/src/components/revisions/Revisions.tsx` — handleRestore uses it
- `ui/src/commands/git.ts` — saveRevision optional lease passthrough
- `ui/src/test/restoreFlow.test.ts` — order/abort guard (new)

## Guard proof
- `ui/src/test/restoreFlow.test.ts` — dirty tree ⇒ save called strictly
  before restore; clean tree ⇒ no save; save failure ⇒ restore never runs.
  Reverting the sequencing fails it.

## Coder dispute (if any)
None.

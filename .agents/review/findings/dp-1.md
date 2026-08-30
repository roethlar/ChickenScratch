# dp-1: "Everything backed up" claims whole-project protection for a one-draft push

**Severity**: HIGH — a protection overclaim on the app's core trust surface:
a writer who loses local data and restores from the remote loses every
alternate draft the UI told them was backed up.
**Status**: Open
**Branch**: — (default-branch mode)
**Commit**: (filled in after commit)

## Evidence
`ui/src/components/revisions/Revisions.tsx:647` — sync status renders
"Everything backed up" when `ahead === 0 && behind === 0`; `:675` — the push
button is titled "Back up online". Both strings landed in `0bcda75` (the
plain-language sweep, in the reviewed range).
`crates/core/src/core/git.rs` `push_remote` — refspec is
`refs/heads/{branch}:refs/heads/{branch}`, the current branch only; the
ahead/behind status is likewise branch-scoped.

## Predicted observable failure
Writer saves unique work on draft `alt`, switches to the main draft, presses
"Back up", sees "Everything backed up". After local loss, recovery from the
remote omits `alt` entirely.

## What
The plain-language sweep translated branch-scoped push jargon into
project-scoped reassurance. The engine behavior (push current branch) is
pre-existing and out of range; the defect in range is the wording overclaim.

## Approach
Scope the words to what the code does: status becomes "This draft is backed
up", the push control becomes "Back up this draft" (title "Back up this
draft online"). Making backup actually cover all drafts is a product/engine
decision recorded for the owner (see `.agents/state.md`), not taken
unilaterally here.

## Files changed
- `ui/src/components/revisions/Revisions.tsx` — the two strings
- `ui/src/test/backupWording.test.ts` — source pin (new)

## Guard proof
- `ui/src/test/backupWording.test.ts` — asserts the draft-scoped strings and
  the absence of "Everything backed up". Reverting the fix fails it.

## Coder dispute (if any)
None on the wording defect. The reviewer's alternative (push every draft
ref) is deferred to the owner as a behavior decision, not adopted silently.

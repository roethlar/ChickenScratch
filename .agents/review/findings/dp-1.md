# dp-1: "Everything backed up" claims whole-project protection for a one-draft push

**Severity**: HIGH — a protection overclaim on the app's core trust surface:
a writer who loses local data and restores from the remote loses every
alternate draft the UI told them was backed up.
**Status**: Verified
**Branch**: — (default-branch mode)
**Commit**: `31c55c2`

## Reviewer comments
Reviewer: codex / gpt-5.6-sol / xhigh / frontier
  escalated: T2 (no recorded frontier pair; owner-named dispatch pair used —
  owner's dispatch word final, recorded not cached)
Harness: codex-cli (MCP transport), thread 01a05457-8e20-7820-a287-9769c46475cb
Reviewed: 31c55c29c79cd36541e3e12f6cc256fb513fd4ed
Base: 981ce784327b461173d62574d2da487857954bea
guard_confirmed: true (revert → backupWording FAIL, restore → PASS, in
reviewer's own worktree)
Verdict: accepted — 2026-08-29 (UTC), comments: none.
Record committed as part of verification history.

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

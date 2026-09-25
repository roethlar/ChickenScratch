# ar-3: Already-staged atomic-write leftovers can still be committed

**Severity**: LOW — needs a leftover already in the index (prior partial save or external tool); then junk enters history
**Status**: Open
**Branch**: — (default-branch mode)
**Commit**: (pending)

Source: Reviewer (generation): codex / gpt-5.6-sol / xhigh / frontier (inline, session-only; same pair the owner named for dp-1..dp-3; no T1 match)
Range: a4d39f1d849e5d92efa8b7aceb2a83b7093d62d3..e2b3d1bd11fd673bde4933bd63b5487fe4c3927b, codex-cli 0.156.0, 2026-09-25 UTC, capability_ok true.
Note: the reviewer's environment routes shell commands through a PreToolUse hook (ptk); recorded as an environment note, not an invalidation.

## Evidence
`crates/core/src/core/git.rs` excludes leftovers only via the `add_all` callback and only for `WT_NEW` status; an existing `INDEX_NEW` entry is kept by `add_all` and written by `write_tree`.

## Predicted observable failure
`manuscript/.one.md.tmp-X` already staged: Save Revision commits it.

## What

## Approach

## Files changed

## Guard proof

## Coder dispute (if any)
None.

## Known gaps

## Reviewer comments

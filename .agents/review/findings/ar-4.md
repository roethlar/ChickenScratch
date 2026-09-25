# ar-4: Relative '..' output paths falsely refused as nested

**Severity**: LOW — a valid CLI invocation (`../Sibling.chikn` from inside a project) is refused; workaround: absolute path
**Status**: Open
**Branch**: — (default-branch mode)
**Commit**: (pending)

Source: Reviewer (generation): codex / gpt-5.6-sol / xhigh / frontier (inline, session-only; same pair the owner named for dp-1..dp-3; no T1 match)
Range: a4d39f1d849e5d92efa8b7aceb2a83b7093d62d3..e2b3d1bd11fd673bde4933bd63b5487fe4c3927b, codex-cli 0.156.0, 2026-09-25 UTC, capability_ok true.
Note: the reviewer's environment routes shell commands through a PreToolUse hook (ptk); recorded as an environment note, not an invalidation.

## Evidence
`crates/core/src/core/project/writer.rs` `enclosing_project` joins relative paths to `current_dir` and walks lexical ancestors without resolving `..`.

## Predicted observable failure
cwd `/work/Outer.chikn`, output `../Sibling.chikn`: `create_project` refuses with 'inside the project Outer'.

## What

## Approach

## Files changed

## Guard proof

## Coder dispute (if any)
None.

## Known gaps

## Reviewer comments

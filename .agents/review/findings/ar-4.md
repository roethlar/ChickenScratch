# ar-4: Relative '..' output paths falsely refused as nested

**Severity**: LOW — a valid CLI invocation (`../Sibling.chikn` from inside a project) is refused; workaround: absolute path
**Status**: In progress
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
`enclosing_project` walked lexical ancestors, so a path like
`Outer.chikn/../Sibling.chikn` listed `Outer.chikn` as an ancestor and was
refused although it lands beside the project.

## Approach
Resolve `.`/`..` lexically, then canonicalize the nearest existing parent
(follows symlinks) before walking ancestors, so the check uses where the
project will actually be created.

## Files changed
- `crates/core/src/core/project/writer.rs` — path resolution in
  `enclosing_project`; guard test extended

## Guard proof
- `core::project::writer::tests::test_create_project_refuses_inside_existing_project`
  — now also creates `Outer.chikn/../Dotted.chikn` and expects success.
  Failed before the fix ("a '..' path that resolves outside the project
  must be allowed"); passes with it. The nested refusals still pass.

## Coder dispute (if any)
None.

## Known gaps
The test uses an absolute path containing `..` rather than changing the
process working directory (global state, racy under parallel tests); the
relative-path case goes through the same resolution after `current_dir`
is joined.

## Reviewer comments
Reviewer: codex / gpt-5.6-sol / high / standard (inline pair, session-only)
Harness: codex-cli 0.156.0 (CLI, --json)
Reviewed: 10b8d684f0187cc23e804b1ca4fa50a3dc976952
Base: ed968955813ac2714149c92d9d03c1831d500658
guard_confirmed: true
Verdict: reopened — 2026-09-25 (UTC). Comments: writer.rs:248 removes
`..` lexically before resolving symlinks; `jump/../Nested.chikn` with
`jump` -> `Outer.chikn/manuscript` lands inside Outer but passes the guard.
Repair (follow-up commit, no amend): canonicalize the longest existing
prefix first (OS order: symlinks, then its `..`), then apply the
non-existent tail. Guard extended: `jump/../Sneaky.chikn` through a
symlink into the project must be refused — FAILED on the reviewed commit,
PASSES with the repair. Redispatch: repair-delta, T5 (standard -> frontier).
Reviewer: codex / gpt-5.6-sol / xhigh / frontier
  escalated: T5 (repair-delta, fresh session)
Harness: codex-cli 0.156.0 (CLI, --json)
Reviewed: 99846bb84980bec56f838e5d84fb80766bd20275
Base: 10b8d684f0187cc23e804b1ca4fa50a3dc976952
guard_confirmed: true
Verdict: reopened — 2026-09-25 (UTC). Comments: the symlink case is
closed, but a missing prefix ending in `..` stops resolution:
`Outer/MissingHop/../../EscapedSibling.chikn` falsely refused;
`jump/MissingHop/../SneakyAfterMissing.chikn` allowed inside Outer.
Repair 2 (follow-up commit): resolve component by component as the OS /
create_dir_all would — canonicalize each existing prefix, enter and leave
missing folders. Guard extended with both reopen cases; the missing-hop
symlink case FAILS against the previous repair's `enclosing_project` and
PASSES now. Redispatch: repair-delta, T5 (ceiling).
Reviewer: codex / gpt-5.6-sol / xhigh / frontier
  escalated: T5 (ceiling) — fresh session
Harness: codex-cli 0.156.0 (CLI, --json)
Reviewed: b0ee3b5eea8e49cde095a13ba59b159c889cb770
Base: 99846bb84980bec56f838e5d84fb80766bd20275
guard_confirmed: true
Verdict: reopened — 2026-09-25 (UTC). Comments: both missing-folder
cases closed; regression: `Outer.chikn/MissingHop/..` does not exist
lexically but resolves to Outer itself; `ancestors().skip(1)` skips it,
so create_project succeeds and overwrites Outer's metadata.
Repair 3 (follow-up commit): `create_project` resolves the target once
(`resolve_creation_path`) and refuses when the path exists as written OR
as resolved, before the nested check (`enclosing_project` now takes the
resolved path). Guard extended: `Outer/NeverMadeHop/..` must be refused
with `project.yaml` byte-identical — FAILED at b0ee3b5, PASSES now.
Redispatch: repair-delta, T5 (ceiling).

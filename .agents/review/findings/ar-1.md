# ar-1: Failed attachment copy leaves a Degraded project presented as writable

**Severity**: HIGH — the writer is shown an editable project whose every save is refused, and reopening shows it read-only
**Status**: In progress
**Branch**: — (default-branch mode)
**Commit**: (pending)

Source: Reviewer (generation): codex / gpt-5.6-sol / xhigh / frontier (inline, session-only; same pair the owner named for dp-1..dp-3; no T1 match)
Range: a4d39f1d849e5d92efa8b7aceb2a83b7093d62d3..e2b3d1bd11fd673bde4933bd63b5487fe4c3927b, codex-cli 0.156.0, 2026-09-25 UTC, capability_ok true.
Note: the reviewer's environment routes shell commands through a PreToolUse hook (ptk); recorded as an environment note, not an invalidation.

## Evidence
`crates/cli/src/scrivener/converter/mod.rs` writes `project.yaml` with every discovered attachment before copying them; a failed copy leaves the hierarchy entry pointing at a missing file, which `fidelity.rs` classifies Degraded. `src-tauri/src/commands/project.rs` ignores the failed `acquire_write_token`; `ui/src/stores/projectStore.ts` sets `readOnly: false`.

## Predicted observable failure
An attachment that exists at discovery but cannot be read or written: import returns with a warning, the UI shows an editable project, no token is cached, subsequent saves are rejected; reopening shows it read-only.

## What
The import report (PLAN_IMPORT_MEDIA_SAFETY step 2) named a failed
attachment but left its hierarchy entry, so the imported project probed
Degraded while the app treated it as writable.

## Approach
Copy attachments before `write_project` (the writer never writes non-.md
content, so the order is safe) and drop every failed attachment from
`documents` and the hierarchy (`remove_hierarchy_paths`, any depth) before
the project is written. The project that is written and committed only
lists files that exist, so it probes Full.

## Files changed
- `crates/cli/src/scrivener/converter/mod.rs` — copy-then-write order,
  `remove_hierarchy_paths`, extended guard test

## Guard proof
- `chikn-converter` `test_import_reports_attachment_that_could_not_be_copied`
  — asserts the failed attachment is absent from the project and the
  output probes Full. Before the fix it failed ("a failed attachment must
  be dropped from the project"); with the fix it passes.

## Coder dispute (if any)
None.

## Known gaps
`import_scrivener` (Tauri) still ignores a failed post-import token
acquisition and the UI assumes writable; with this fix a fresh import
probes Full, so that path is no longer reached by this failure. Not changed
here.

## Reviewer comments

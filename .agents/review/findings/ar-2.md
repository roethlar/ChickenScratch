# ar-2: Attachments missing at discovery are still silently dropped

**Severity**: MEDIUM — the writer believes the import is complete while an attachment was left behind, contradicting PLAN_IMPORT_MEDIA_SAFETY
**Status**: Open
**Branch**: — (default-branch mode)
**Commit**: (pending)

Source: Reviewer (generation): codex / gpt-5.6-sol / xhigh / frontier (inline, session-only; same pair the owner named for dp-1..dp-3; no T1 match)
Range: a4d39f1d849e5d92efa8b7aceb2a83b7093d62d3..e2b3d1bd11fd673bde4933bd63b5487fe4c3927b, codex-cli 0.156.0, 2026-09-25 UTC, capability_ok true.
Note: the reviewer's environment routes shell commands through a PreToolUse hook (ptk); recorded as an environment note, not an invalidation.

## Evidence
`crates/cli/src/scrivener/converter/mod.rs` records an attachment only when `media_src.exists()`; the else branch only `eprintln!`s and never reaches `ImportReport.warnings`.

## Predicted observable failure
A binder item references an attachment absent from `Files/Data`: import succeeds, the attachment is missing, warnings are empty, the GUI shows nothing.

## What

## Approach

## Files changed

## Guard proof

## Coder dispute (if any)
None.

## Known gaps

## Reviewer comments

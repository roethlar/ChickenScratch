# ar-2: Attachments missing at discovery are still silently dropped

**Severity**: MEDIUM — the writer believes the import is complete while an attachment was left behind, contradicting PLAN_IMPORT_MEDIA_SAFETY
**Status**: Verified
**Branch**: — (default-branch mode)
**Commit**: `9f3b594`

Source: Reviewer (generation): codex / gpt-5.6-sol / xhigh / frontier (inline, session-only; same pair the owner named for dp-1..dp-3; no T1 match)
Range: a4d39f1d849e5d92efa8b7aceb2a83b7093d62d3..e2b3d1bd11fd673bde4933bd63b5487fe4c3927b, codex-cli 0.156.0, 2026-09-25 UTC, capability_ok true.
Note: the reviewer's environment routes shell commands through a PreToolUse hook (ptk); recorded as an environment note, not an invalidation.

## Evidence
`crates/cli/src/scrivener/converter/mod.rs` records an attachment only when `media_src.exists()`; the else branch only `eprintln!`s and never reaches `ImportReport.warnings`.

## Predicted observable failure
A binder item references an attachment absent from `Files/Data`: import succeeds, the attachment is missing, warnings are empty, the GUI shows nothing.

## What
Binder items skipped at discovery (attachment missing from the package,
or an item type the importer cannot convert) were only `eprintln!`ed, so
the writer was never told something was left out.

## Approach
Record each skipped item as a `__skipped__<uuid>` entry in the existing
`uuid_to_path` side map at discovery, and fold those messages into
`ImportReport.warnings`, which the app already shows in a persistent
toast. Covers the unknown-type skip too (same silent path).

## Files changed
- `crates/cli/src/scrivener/converter/mod.rs` — record skips, report them,
  new guard test

## Guard proof
- `chikn-converter` `test_import_reports_attachment_missing_from_scrivener_package`
  — a binder Image whose file is absent must be named in warnings.
  Removing the reporting loop makes it FAIL; restoring makes it PASS.

## Coder dispute (if any)
None.

## Known gaps

## Reviewer comments
Reviewer: codex / gpt-5.6-sol / high / standard (inline pair, session-only)
Harness: codex-cli 0.156.0 (CLI, --json)
Reviewed: 9f3b5948f500836d48c24d1e490a29afe45a0ba1
Base: 3fafd3349226a715e37582da210c1cdea02d9f13
guard_confirmed: true
Verdict: accepted — 2026-09-25 (UTC). Comments: none.
Record committed as part of verification history.

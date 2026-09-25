# Plan: Open a project whose notes file is not valid text

**Owner request (quote):**
> "assume I actually want a working app. find issues, plan fixes, fix,
> review with codex." (2026-09-25)

**Status:** Implemented 2026-09-25; guard test bit before the fix (the
exact step-7a I/O error). GUI check pending with the next build.

**Phase check:** [x] Allowed by `CURRENT_PHASE.md` (Engine hardening)
[x] Not paused

**Invariants touched:** I2 (fix lives in core), I6 (no data loss — the
damaged file stays byte-identical), I9 (guard test must bite).

---

## [MODEL] Intent

A writer whose `.meta` notes file got binary garbage in it (bad sync,
disk error) can still open the project read-only, see which file is
damaged, and keep reading their work. Today the project refuses to open
with "IO error: stream did not contain valid UTF-8", which names no file
(break-it step 7a, 2026-08-29 and again 2026-09-25).

## [MODEL] Cause

The fidelity probe already classifies an unreadable sidecar as
`CorruptSidecar` (Degraded), so the app takes the read-only path. But
`reader::read_document_metadata_or_default` reads the file with
`fs::read_to_string(meta_path)?`; invalid UTF-8 returns an I/O error that
aborts the whole read. Bad-YAML sidecars already fall back to defaults.

## [MODEL] Approach

1. Read the sidecar as bytes; if they are not valid UTF-8, treat the file
   exactly like unparseable YAML: log, keep it untouched, fall back to the
   hierarchy identity defaults.
2. Guard test in `reader.rs`: invalid-UTF-8 sidecar → `read_project`
   succeeds, document keeps its hierarchy identity, file byte-identical,
   no quarantine. Revert step 1 to watch it fail.

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `crates/core/src/core/project/reader.rs` | tolerant sidecar read + guard test |

## [MODEL] Tests

- [ ] New reader guard test bites
- [ ] Full declared suite
- [ ] GUI: garbage bytes in a `.meta` → opens read-only, banner names the
      file

## [MODEL] Owner verification (plain English)

"A damaged notes file no longer locks you out: the project opens
read-only and tells you which file is damaged."

## [YOU] Decisions needed

- None.

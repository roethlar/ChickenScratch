# Plan: Scrivener import copies attachments through the engine and says what it skipped

**Owner request (quote):**
> "assume I actually want a working app. find issues, plan fixes, fix,
> review with codex." (2026-09-25)

**Phase check:** [x] Allowed by `CURRENT_PHASE.md` (Engine hardening)
[x] Not paused

**Invariants touched:** I2 (format I/O only through `chickenscratch-core`),
I6 (no silent loss), I9.

Issue: `.agents/app-issues.md` #10 (parked since 2026-07; source
rechecked 2026-09-25).

---

## [MODEL] Intent

Importing a Scrivener project brings its images and PDFs along through
the same safe, atomic write path as everything else, and if any
attachment or the first revision could not be saved, the writer is told
which — instead of the app printing it to a hidden console.

## [MODEL] Current behavior

`crates/cli/src/scrivener/converter/mod.rs` `import_scriv`, inside the
write permit:

- attachments: `fs::create_dir_all` (result ignored) + `fs::copy`;
  failures go to `eprintln!` only. This is format I/O outside core (I2)
  and bypasses the symlink-refusing safe-path checks.
- initial revision: `let _ = save_revision(…)` — a failure is dropped,
  leaving an imported project with no history and no message.

## [MODEL] Approach

1. **Write attachments through core.** Read the source bytes and write
   them with `writer::write_project_app_file(permit, relative, bytes)`
   (safe-path parents, atomic write). Collect every failure (read or
   write) with the attachment's name and reason.
2. **Report problems.** New `import_scriv_with_report` returns an
   `ImportReport { project, warnings }`; warnings list skipped
   attachments and a failed first revision. `import_scriv` keeps its
   signature for the CLI and prints the warnings to stderr as before.
   The Tauri `import_scrivener` command returns the report; the UI shows
   the warnings in a persistent toast after the project opens.
3. Guard tests: an attachment whose source is missing is reported and the
   rest of the import succeeds; a normal attachment lands at its path.

One commit per step.

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `crates/cli/src/scrivener/converter/mod.rs` | core write; report; tests |
| `crates/cli/src/lib.rs` | export the report API |
| `src-tauri/src/commands/project.rs` | return the report |
| `ui/src/commands/project.ts`, `ui/src/stores/projectStore.ts` | show warnings |

## [MODEL] Owner verification (plain English)

"Import a Scrivener project with pictures — the pictures come along; if
any can't be copied, the app tells you which."

## [YOU] Decisions needed

- None.

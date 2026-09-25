# App issues found by hands-on testing (2026-09-25)

Owner goal 2026-09-25: "assume I actually want a working app. find issues,
plan fixes, fix, review with codex." Found by driving the built app with
computer use on disposable copies in `~/chikn-validation/`. One commit per
fix; each engine fix has a plan under `docs/plans/`.

| # | Issue | Severity | Status |
|---|-------|----------|--------|
| 1 | Restore / draft switch silently undone on disk (stale editor buffer) | LOST DATA | Fixed `8679de4` |
| 2 | Read-only banner always blames "an older version"; real reason hidden in tooltip | Misleading | Fixed `15cd5b0` |
| 3 | Non-UTF-8 `.meta` makes project refuse to open with a nameless I/O error | Locked out | Fixed `7c3ebe0` |
| 4 | A project folder (or any git repo) inside a project makes every Save Revision fail: `Failed to stage files: invalid path 'Nested.chikn/'` | History broken | Fixed (staging safety step 1) |
| 5 | New Project dialog defaults to the last folder, which can be inside an existing project; creating a project inside another is allowed | Causes #4 | Fixed (staging safety step 3: engine refuses) |
| 6 | Newly created (and imported) projects are not added to Recent Projects | Usability | Fixed |
| 7 | Error toasts vanish after 3.5 s and use developer wording (`Unknown error: …class=Index (10)`) — easy to miss a failed save | Silent-ish failure | Fixed (errors persist; codes stripped) |
| 8 | Save Revision after `.git` is deleted mid-session fails until relaunch (jargon error) | Recoverable | Fixed (save rebuilds history) |
| 9 | Revision staging uses `add_all(["*"])`; atomic-write temp files `.X.tmp-*` are not ignored (parked since 2026-07) | Junk in history | Fixed (staging safety step 2) |
| 10 | Scrivener asset import copies files with `fs::copy` outside core and reports failures only to stderr (parked since 2026-07) | I2 violation / silent loss | Open |

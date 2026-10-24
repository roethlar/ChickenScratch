# Agent State

First read for current repo state at session start. Session-level layer only —
`docs/CURRENT_PHASE.md` is the authoritative phase plan; `docs/adr/` holds
decisions; `DEVLOG.md` holds history. Completed state entries live in
`docs/history/state-archive.md`.

## Now

- Phase **Engine hardening — protect writers' work** remains active;
  `docs/CURRENT_PHASE.md` owns the phase and exit criteria.
- **Owner 2026-10-23: web shell queued as a side quest, no phase change**
  ("no direction change, just the next side quest. plan it, add it to the
  queue"). Plan: `docs/plans/PLAN_WEB_SHELL.md` — a `chikn-web` server
  that serves the existing React UI over HTTP from a shared
  `chickenscratch-services` command layer. **Owner 2026-10-24: "follow the
  queue and get as much as you can done"** — the go for the plan's slices in
  order, one commit each. Slices 1 (services extraction, ADR-007), 2
  (frontend transport + platform seams), 3a (`chikn-web` server) and 3b (web
  Welcome) and 4a–4d (export download, upload import, server folder picker,
  Scrivener import) and 6 (embed-ui single binary, user guide, release
  runbook, records) landed 2026-10-24 — the browser now covers everything the
  desktop does, including live AI streaming (slice 5, SSE) and a single-
  writer lease for two devices on one project (slice 7). Every slice of the
  plan has shipped; DEVLOG has the entry.
- **Owner goal 2026-09-25: working app — find, plan, fix, Codex review.**
  Hands-on GUI testing found 13 issues; all fixed one commit each
  (`.agents/app-issues.md` is the tracker). Headline: Restore and draft
  switching were silently undone on disk (fixed, GUI re-verified); a
  project inside another broke all revision saves (fixed + now refused);
  the red close button never closed the window (fixed). Codex
  defect review over `a4d39f1..e2b3d1b` raised ar-1..ar-4; all fixed and
  verified accepted (`.agents/review/index.md`).
- **Break-it steps 7–10 rechecked on the current build (2026-09-25)**:
  deleted history rebuilds on reopen and on save; damaged sidecars open
  read-only with the real reason shown. `PLAN_HISTORY_SELF_HEAL.md` is
  superseded.
- **Phase close-out question remains unanswered** (posed 2026-07-17):
  close hardening and move to the Tauri reference-app phase? An agent
  (Claude Code, 2026-08-29) ran the break-it test; the owner has not used
  the app. Resolve that test's evidence first.
- **Parked hardening follow-ups**: the Scrivener asset-import and
  temp-file staging items are fixed (app issues #9, #10). The rest of the
  format-lock audit lives under Out of scope in
  `docs/plans/PLAN_FORMAT_LOCK_ENGINE.md`; reader repair notices remain
  stderr-only and `Project` still has no project-level `fields` map.
- **Draft-peek follow-ups remain unapproved**, rechecked as of `a891ed5`:
  `DocumentHistory.tsx` still uses the old Restore wording. The owner
  decision on backing up all drafts remains open; `push_remote` still
  pushes the current draft only. The completed review record is
  `.agents/review/index.md` (dp-1 through dp-3).
- Vault/remote design remains deliberately paused under
  `docs/CURRENT_PHASE.md` and Slice 2 of
  `docs/plans/PLAN_TRUST_FOUNDATIONS.md`, including the guided-token
  question. No new approval was found in the current records.

## Blockers

- None. Owner go 2026-09-25: find issues, plan, fix, review with Codex.

## Known drift (recorded, not yet fixed)

- **Tauri crate not compiled on magneto (Linux dev box, 2026-10-24).**
  `webkit2gtk-4.1` / `javascriptcoregtk-4.1` are not installed, so
  `cargo check -p chickenscratch` fails in the `javascriptcore-rs-sys` build
  script before reaching our code. The slice-1 Tauri wrappers were
  type-checked against a scratch stub of the `tauri` API (`State`,
  `AppHandle`, `Emitter`, no-op `#[command]`) instead; CI's macOS runner is
  the real check. Installing the WebKitGTK dev package is an owner call
  (system change).
- **Flaky core test (observed once, 2026-10-24):**
  `core::git::tests::failed_restore_after_tree_mutation_invalidates_outstanding_tokens`
  failed at `crates/core/src/core/git.rs:2069` in one full-suite run and
  passed on an immediate rerun and in four other runs the same day. The
  engine was not touched by the web-shell work. Assumption (unverified): the
  test flips the process-wide `failpoints::AFTER_TREE_MUTATION` flag, which
  another restore test running in parallel can observe. Hardening-phase item;
  needs its own go.

## Next

1. **Owner: push** the 12 web-shell commits (`bb6fa82..c3e171d`) when ready
   (`.agents/push-policy.md`; CI has not seen them and is the first real
   compile of the Tauri wrappers). Remaining known gaps, unapproved: Tauri
   `import_scrivener` still ignores a failed post-import token acquisition
   (not reachable after ar-1); welcome-screen errors keep a
   developer-ish "Invalid project:" prefix.
2. Then re-ask whether to close hardening and move to the Tauri
   reference-app phase. The phase declaration remains the owner's call.
   Vault work remains paused.
3. **Web shell side quest**: every slice shipped and the owner has run it
   (2026-10-24: works over the VPN after the plain-HTTP random-id fix
   `a054e9d`). CI on a push is the first check of the Tauri wrappers on a
   machine with WebKitGTK.
4. **Queued side quest: project profiles** (`docs/plans/PLAN_PROJECT_PROFILES.md`,
   owner 2026-10-24 "plan it"): Fiction / Notes / Document types that show,
   hide or rename the novelist UI parts, display-only. No slice approved;
   propose slice 1 (profile table, no visible change) first.

## Verification

- Declared suite: `.agents/repo-guidance.md` Verification section.
- **2026-10-24 as of `c3e171d`** (web shell, 12 unpushed commits): fmt, clippy
  `-D warnings` and tests green for core, services, web, tui, converter;
  release-metadata script; cross-frontend harness; UI lint/build/test (87
  tests). The Tauri crate did not compile on this box (Known drift); its
  wrappers were type-checked against the `tauri` stub at `c3e171d`. Live
  smoke of `chikn-web` (page, auth, create/list project, embedded assets)
  passed. CI has not seen these commits.
- Full declared suite plus the CI cross-frontend harness ran green
  locally 2026-09-25 **as of `cd48252`**; final GUI smoke (restore holds
  on disk, red close button closes) on that build. Desktop GUI checks of each fix
  were run by computer use on disposable copies in `~/chikn-validation/`.
  Nothing from this session has been pushed; CI has not seen it.
- Live CI checked 2026-09-17 **as of `a2d8a39`**: Validation run
  `33335376706` and Tauri Bundles run `33335376726` both succeeded.
- **Tag release path has been exercised**, correcting the superseded
  claim in the archive: `v1.0.0` identifies `4dc0b22`; macOS Signed Release
  run `31232776419` and Windows Signed Release run `31232776468` both
  succeeded on that tag. The GitHub release was published 2026-08-08 and
  has its DMG, MSI, and NSIS installer attached, verified live 2026-09-17.
  Runbook: `RELEASE.md`; release:
  <https://github.com/roethlar/ChickenScratch/releases/tag/v1.0.0>.

## Active Sources

- `AGENTS.md` · `.agents/repo-guidance.md` · `docs/INVARIANTS.md` · `docs/CURRENT_PHASE.md`
- `.agents/decisions.md` (pointer to `docs/adr/`)

## Unrecorded Repo Memory

- None known. The completed legacy review cycle is archived in `REVIEW.md`
  and `.review/`; the format-lock audit owns its out-of-scope findings in
  `docs/plans/PLAN_FORMAT_LOCK_ENGINE.md`.

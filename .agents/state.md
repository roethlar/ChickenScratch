# Agent State

First read for current repo state at session start. Session-level layer only —
`docs/CURRENT_PHASE.md` is the authoritative phase plan; `docs/adr/` holds
decisions; `DEVLOG.md` holds history. Completed state entries live in
`docs/history/state-archive.md`.

## Now

- Phase **Engine hardening — protect writers' work** remains active;
  `docs/CURRENT_PHASE.md` owns the phase and exit criteria.
- **Owner 2026-10-25, awaiting "plan it" (no plan written yet):** two
  directions given in words after the second-machine install.
  (a) "reloading the page should not abandon the open project" … "more than
  that. a reload should not abandon any unsaved changes in the currently open
  doc." Proposed slice (owner has not yet said plan it): a per-note local
  safety copy in browser storage, cleared when the server confirms the save
  and restored on reload/crash with a one-line notice; remember the last open
  project + note and reopen it with the same session seat; a **Projects**
  button that saves, releases the project and shows the list (today
  `closeProject` exists in `projectStore.ts` but nothing in the UI calls it;
  the only way back is a reload). Today's gap: autosave is debounced
  (`Editor.tsx`, min 250 ms / default 2 s) and the web `beforeunload` path
  cannot await the final flush, so a reload inside that window loses the
  last words — an I6 matter.
  (b) "this needs a mobile-friendly UI for taking quick notes or edits on a
  mobile device": one-column layout under a width threshold, binder as a
  drawer, inspector hidden, quick-note button on the project list, touch-size
  controls, home-screen install. Proposed as the slices after (a), in one
  plan. Owner also asked "how do I start a new project once a project has
  been opened? there's no close" → the Projects button above.
- **Owner 2026-10-25: "the install did not ask any new questions"** → fixed
  in `e56483d` (unpushed): an upgrade asks once for a setting the settings
  file has never seen, flags change one setting, `--yes` writes the
  placeholder. Dry-run verified; shellcheck clean.
- **Owner 2026-10-25: "plan" after the first install on a second machine**
  (projects on NFS at `/mnt/notes`): settings save fails there with a keyring
  D-Bus error; the backup folder can only be picked inside the root; git's
  ownership check fires because the share maps uids. Plan:
  `docs/plans/PLAN_WEB_SERVER_FIXES.md` — owner "finish all slices"; all
  three landed 2026-10-25 (`9449c4d` secrets fallback, `0f096eb` backup
  root, `a8db23d` ownership check off on the server by default, following
  the plan's recommendation since the go named all slices). Unpushed.
- **Owner 2026-10-24: "get rid of the access token"** — done the same day
  (`docs/plans/PLAN_WEB_NO_LOGIN.md`, `.agents/decisions.md`): `chikn-web`
  has no login; the private network is the boundary. Token file, `?token=`
  link, UI token gate and `--token` flag are gone. Pushed as `aec8f18` on the
  owner's `/git push all`; Validation run `40157115663` failed only in the
  new Shell scripts step (macOS runner has no shellcheck) — fixed by
  installing it in the step (`8d587a2`, pushed on the owner's "push";
  Validation run `40157357274` green).
- **Owner 2026-10-24: web installer queued** ("this needs an installer for
  the web version. the default data dir should be set at install time, and
  here it will be on my NAS" … "a .config/ entry or similar. and a way to
  upgrade without re-specifying everything"). Plan:
  `docs/plans/PLAN_WEB_INSTALLER.md` — `web.toml` config read by
  `chikn-web`, `scripts/install-web.sh` with a user systemd service that
  waits for the NFS mount, rerun = upgrade. Owner "go" then "do all the
  slices": all three slices landed 2026-10-24 (`ff8ad9b` settings file,
  `1c214ec` installer + shellcheck gate, docs commit). Next evidence is the
  owner running `scripts/install-web.sh` on the server with the NAS path.
- **Owner 2026-10-24: project profiles shipped** (`docs/plans/PLAN_PROJECT_PROFILES.md`,
  go: "complete all slices and push when done"): Fiction / Notes / Document
  types show, hide or rename the novelist UI parts, display-only; New Project
  asks for the type. UI-only plus four backend templates. Pushed with the
  web-shell commits on the owner's instruction.
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

- **CI red under rustc 1.99.1 — fixed and proven green** (Validation run
  `40105127486` at `150bfcf` succeeded on all three runners, 2026-10-24;
  the red run was `40071873531` at `3a8db4e`). Owner go "fix ci", then
  "push"; plan `docs/plans/PLAN_CI_RUST_199.md`. Kept here one cycle as the
  record of cause. (1) Windows: `libgit2-sys 0.17` relied
  on std linking `advapi32.lib`; std 1.99 binds that DLL via raw-dylib, so
  the core test runner failed to link. Fix `3f9024b`: `crates/core/build.rs`
  emits `rustc-link-lib=advapi32` on Windows. Alternative left open:
  upgrade `git2` 0.19 → 0.21. (2) macOS: XNU `killpg` returns `EPERM` for a
  group with no live member and marks the leader exited before it is
  reapable; `kill_tree` accepted `EPERM` only when `try_wait` already saw the
  exit, losing the race in `process_helper_caps_combined_stdout_and_stderr`.
  Fix `818eb2a`: on `EPERM`, poll `try_wait` up to 500 ms before believing
  the error. Neither fix is exercisable on the Linux box (both branches are
  inert here); the GitHub runners were the proof.
- **Desktop wrappers verified by CI**: Tauri Bundles run `40071873562`
  built the Tauri app (macOS) at `3a8db4e`, so the slice-1 wrappers compile
  with the real `tauri` crate; the local WebKitGTK gap below no longer
  blocks anything.
- **Tauri crate not compiled on magneto (Linux dev box, 2026-10-24).**
  `webkit2gtk-4.1` / `javascriptcoregtk-4.1` are not installed, so
  `cargo check -p chickenscratch` fails in the `javascriptcore-rs-sys` build
  script before reaching our code. The slice-1 Tauri wrappers were
  type-checked against a scratch stub of the `tauri` API (`State`,
  `AppHandle`, `Emitter`, no-op `#[command]`) instead; CI's macOS runner is
  the real check. Installing the WebKitGTK dev package is an owner call
  (system change).
- **Flaky core tests — fixed 2026-10-25 (`52ad932`, test-only).**
  `failed_restore_after_tree_mutation…` (seen 2026-10-24) and
  `failed_complete_merge_after_staging…` (2 of 3 full runs on 2026-10-25)
  armed process-wide `AtomicBool` failpoints that any parallel test could
  consume; the flags are thread-local now. 5 of 5 full core runs green. Done
  without a separate go because it blocked verifying the web server fixes;
  no production code changed.

## Next

1. **Push `e56483d`** (installer asks for new settings; needs the owner's
   push go), then the owner reruns `git pull && scripts/install-web.sh` on
   the second machine: it asks the backup question once; Settings → Backup
   then shows a Backups tab. Owner checks still open there: Settings save,
   Save Revision on the NAS lists an entry.
2. **Write the plan the owner asked about in words** when they say "plan it":
   slice 1 = local safety copy + reopen last project/note + Projects button;
   slices 2+ = mobile layout, quick note, touch targets, home-screen install.
   No code before the go.
3. **Web installer shipped and pushed** (`docs/plans/PLAN_WEB_INSTALLER.md`):
   ask for the push go, then the owner's first real install on the server
   (`scripts/install-web.sh`, NAS folder when asked) is the verification.
   Later, needs its own plan: a CI-built Linux tarball with the installer
   inside. CI is green at `150bfcf`. Candidate
   hardening slices, each needing its own go: the flaky restore-token test
   (Known drift), the `git2` 0.19 → 0.21 upgrade that would retire
   `crates/core/build.rs`. Remaining known gaps, unapproved: Tauri
   `import_scrivener` still ignores a failed post-import token acquisition
   (not reachable after ar-1); welcome-screen errors keep a
   developer-ish "Invalid project:" prefix.
4. Then re-ask whether to close hardening and move to the Tauri
   reference-app phase. The phase declaration remains the owner's call.
   Vault work remains paused.
5. **Web shell side quest**: every slice shipped and the owner has run it
   (2026-10-24: works over the VPN after the plain-HTTP random-id fix
   `a054e9d`). CI on a push is the first check of the Tauri wrappers on a
   machine with WebKitGTK.
6. **Project profiles**: all four slices shipped 2026-10-24 (owner: "complete
   all slices and push when done"); next evidence is the owner creating a
   Notes project in the browser and seeing no fiction tools.

## Verification

- Declared suite: `.agents/repo-guidance.md` Verification section.
- **2026-10-25 as of `ad44c22`, pushed by the owner to `origin` and
  `github`**: GitHub Validation run `40162108795` succeeded (all jobs,
  shellcheck step included). `e56483d` (installer) is local only.
- **2026-10-25 as of `52ad932`** (web server fixes): fmt, clippy
  `-D warnings` and tests green for core (138, five full runs), services,
  web, tui, converter; release-metadata script; `bash -n` + shellcheck 0.11
  on the scripts; UI lint/build/test (97). Tauri crate not compiled on this
  box (Known drift). Installer dry runs with `--backup`; live `chikn-web`
  start shows the ownership line and answers `/api/info`.
- **2026-10-24 as of `150bfcf`, pushed to `origin` and `github`**: GitHub
  Validation run `40105127486` succeeded (Linux harness, Windows process
  helper, macOS); Tauri Bundles run `40105127518` was still in progress
  when recorded (it passed at `3a8db4e` and only core changed since).
- **2026-10-24 as of `818eb2a`** (two CI fixes): fmt, clippy
  `-D warnings` and tests green for core, services, web, tui, converter
  (137 core tests, the once-flaky restore test included this run);
  release-metadata script; UI lint/build/test (98 tests). Tauri crate not
  compiled on this box (Known drift). The Windows link line and the Darwin
  `EPERM` branch cannot fire on Linux, so CI is the only proof for both.
- **2026-10-24 as of `c3e171d`** (web shell, 12 unpushed commits): fmt, clippy
  `-D warnings` and tests green for core, services, web, tui, converter;
  release-metadata script; cross-frontend harness; UI lint/build/test (87
  tests). The Tauri crate did not compile on this box (Known drift); its
  wrappers were type-checked against the `tauri` stub at `c3e171d`. Live
  smoke of `chikn-web` (page, auth, create/list project, embedded assets)
  passed. Pushed 2026-10-24 to `origin` (Gitea) and, after the owner added
  the `github` remote, to GitHub: Tauri Bundles `40071873562` succeeded;
  Validation `40071873531` failed in two pre-existing core process-helper
  tests under rustc 1.99.1 (Known drift).
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

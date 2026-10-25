# Agent State

First read for current repo state at session start. Session-level layer only —
`docs/CURRENT_PHASE.md` is the authoritative phase plan; `docs/adr/` holds
decisions; `DEVLOG.md` holds history. Completed state entries live in
`docs/history/state-archive.md`.

## Now

- Phase **Engine hardening — protect writers' work** remains active;
  `docs/CURRENT_PHASE.md` owns the phase and exit criteria.
- **Owner 2026-10-25: reload safety and phone layout shipped.** Directions
  in words after the second-machine install ("reloading the page should not
  abandon the open project" … "any unsaved changes in the currently open
  doc"; "there's no close"; "a mobile-friendly UI for taking quick notes"),
  then the go "proceed with planning as directed in state docs, then
  implement all slices". Plan `docs/plans/PLAN_RELOAD_SAFETY_MOBILE.md`;
  slices landed 2026-10-25 one commit each: `26af1d5` safety copy +
  remembered place + Projects button + `POST /api/lease/release`;
  `e6fe793` one-column phone layout; `bd019a7` Quick note; `e6b88b6` touch
  targets + home-screen manifest; records `ab29eb7`. The owner pushed
  `fe0ffbf` to both remotes 2026-10-25. Next evidence is the owner
  reloading mid-typing in the browser and opening the server address on a
  phone.
- **Owner 2026-10-25: "import scrivener needs to support uploads of
  scrivener files."** Today the web Import Scrivener only accepts a
  `.scriv` folder already copied under the projects root (server folder
  picker). Plan drafted, not yet approved:
  `docs/plans/PLAN_SCRIVENER_UPLOAD.md` (slice 1 server endpoint taking a
  folder pick or a `.zip`, streamed to a temp folder, path-checked, then the
  existing import service; slice 2 the three-way chooser in Welcome, user
  guide). No code changed. Known limits, by design: the safety copy is per browser profile
  (a copy made on the phone restores on the phone); a brand-new tab gets a
  new session id, so the "Take over" offer still applies there; no service
  worker, so the installed icon needs the server reachable.
- **Owner 2026-10-25: governance remediation** (`487e775`): three stale
  path references in `.agents/break-it-validation.md` resolved on owner
  rulings; hygiene lint clean. The pending
  governance refresh left its files staged but uncommitted; its staged
  edit to this file had cut three lines mid-sentence (every line that said
  "unpushed") — restored here with the push claims corrected.
- **Phase close-out question remains unanswered** (posed 2026-07-17):
  close hardening and move to the Tauri reference-app phase? An agent
  (Claude Code, 2026-08-29) ran the break-it test; the owner has not used
  the app. Resolve that test's evidence first.
- **Parked hardening follow-ups**: the Scrivener asset-import and
  temp-file staging items are fixed (app issues #9, #10). The rest of the
  format-lock audit lives under Out of scope in
  `docs/plans/PLAN_FORMAT_LOCK_ENGINE.md`; reader repair notices remain
  stderr-only and `Project` still has no project-level `fields` map
  (`crates/core/src/models/project.rs`, rechecked as of `82ba252`).
- **Draft-peek follow-ups remain unapproved**, rechecked as of `82ba252`:
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

- **Desktop wrappers verified by CI**: Tauri Bundles run `40071873562`
  built the Tauri app (macOS) at `3a8db4e`, so the slice-1 wrappers compile
  with the real `tauri` crate; the local WebKitGTK gap below no longer
  blocks anything.
- **Tauri crate not compiled on the Linux dev box** (machine detail:
  `.agents/machines.md`, magneto). `cargo check -p chickenscratch` fails in
  the `javascriptcore-rs-sys` build script before reaching our code. The
  slice-1 Tauri wrappers were type-checked against a scratch stub of the
  `tauri` API (`State`, `AppHandle`, `Emitter`, no-op `#[command]`)
  instead; CI's macOS runner is the real check. Installing the WebKitGTK
  dev package is an owner call (system change).

## Next

1. **Installer asks for new settings (`e56483d`)**: the owner reruns
   `git pull && scripts/install-web.sh` on the second machine: it asks the
   backup question once; Settings → Backup then shows a Backups tab. Owner
   checks still open there: Settings save, Save Revision on the NAS lists an
   entry.
2. **Scrivener upload in the browser** (`PLAN_SCRIVENER_UPLOAD.md`): drafted
   2026-10-25, awaiting the owner's go; then slice 1 (server endpoint), slice
   2 (chooser + guide), one commit each.
3. **Reload safety + phone layout** (`PLAN_RELOAD_SAFETY_MOBILE.md`): every
   slice shipped and pushed 2026-10-25 (`fe0ffbf`). The owner tries it
   (reload mid-typing; phone; Quick note; Add to Home Screen) after
   `git pull && scripts/install-web.sh` on the server.
4. **Web installer** (`docs/plans/PLAN_WEB_INSTALLER.md`) shipped; the
   owner's first install on a second machine happened 2026-10-25 and raised
   the web server fixes (archived). Later, needs its own plan: a CI-built
   Linux tarball with the installer inside. Candidate hardening slice,
   needing its own go: the `git2` 0.19 → 0.21 upgrade that would retire
   `crates/core/build.rs`. Remaining known gaps, unapproved: Tauri
   `import_scrivener` still ignores a failed post-import token acquisition
   (not reachable after ar-1); welcome-screen errors keep a
   developer-ish "Invalid project:" prefix.
5. Then re-ask whether to close hardening and move to the Tauri
   reference-app phase. The phase declaration remains the owner's call.
   Vault work remains paused.
6. **Project profiles**: all four slices shipped 2026-10-24 (owner: "complete
   all slices and push when done"); next evidence is the owner creating a
   Notes project in the browser and seeing no fiction tools.

## Verification

- Declared suite: `.agents/repo-guidance.md` Verification section.
- **2026-10-25 reload safety + phone layout (slices 1–4, unpushed)**: fmt,
  clippy `-D warnings` and tests green for core, services, web, tui,
  converter; release-metadata script; `bash -n` on the scripts (shellcheck
  is not installed on this box; no script changed); UI lint/build/test
  (125); `cargo check -p chickenscratch-web --features embed-ui` with the
  built `ui/dist`. Tauri crate not compiled on this box (Known drift). Bite
  proof: the editor safety-copy tests fail with the restore and the
  pagehide write removed. CI runs on the next push.
- **2026-10-25 as of `82ba252`**: GitHub Validation runs `40162108795`
  (`ad44c22`), `40164051106` (`487e775`) and `40164229608` (`82ba252`) all
  succeeded (all jobs, shellcheck step included).
- **2026-10-25 as of `52ad932`** (web server fixes): fmt, clippy
  `-D warnings` and tests green for core (138, five full runs), services,
  web, tui, converter; release-metadata script; `bash -n` + shellcheck 0.11
  on the scripts; UI lint/build/test (97). Tauri crate not compiled on this
  box (Known drift). Installer dry runs with `--backup`; live `chikn-web`
  start shows the ownership line and answers `/api/info`.
- **2026-10-24 as of `150bfcf`**: GitHub
  Validation run `40105127486` succeeded (Linux harness, Windows process
  helper, macOS); Tauri Bundles run `40105127518` was still in progress
  when recorded (it passed at `3a8db4e` and only core changed since).
- **2026-10-24 as of `818eb2a`** (two CI fixes): fmt, clippy
  `-D warnings` and tests green for core, services, web, tui, converter
  (137 core tests, the once-flaky restore test included this run);
  release-metadata script; UI lint/build/test (98 tests). Tauri crate not
  compiled on this box (Known drift). The Windows link line and the Darwin
  `EPERM` branch cannot fire on Linux, so CI is the only proof for both.
- **2026-10-24 as of `c3e171d`** (web shell): fmt, clippy
  `-D warnings` and tests green for core, services, web, tui, converter;
  release-metadata script; cross-frontend harness; UI lint/build/test (87
  tests). The Tauri crate did not compile on this box (Known drift); its
  wrappers were type-checked against the `tauri` stub at `c3e171d`. Live
  smoke of `chikn-web` (page, auth, create/list project, embedded assets)
  passed. On GitHub: Tauri Bundles `40071873562` succeeded;
  Validation `40071873531` failed in two pre-existing core process-helper
  tests under rustc 1.99.1 (fixed; archived from Known drift).
- Full declared suite plus the CI cross-frontend harness ran green
  locally 2026-09-25 **as of `cd48252`**; final GUI smoke (restore holds
  on disk, red close button closes) on that build. Desktop GUI checks of each fix
  were run by computer use on disposable copies in `~/chikn-validation/`.
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

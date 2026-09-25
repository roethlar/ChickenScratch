# Agent State

First read for current repo state at session start. Session-level layer only —
`docs/CURRENT_PHASE.md` is the authoritative phase plan; `docs/adr/` holds
decisions; `DEVLOG.md` holds history. Completed state entries live in
`docs/history/state-archive.md`.

## Now

- Phase **Engine hardening — protect writers' work** remains active;
  `docs/CURRENT_PHASE.md` owns the phase and exit criteria.
- **Break-it step 9 ("delete history") passes at the engine layer; a GUI
  rerun is the remaining check.** Engine repro 2026-09-25 as of `53ed7e0`
  (`.agents/break-it-validation.md`): a fresh open rebuilds a deleted
  `.git` and Save Revision commits. `docs/plans/PLAN_HISTORY_SELF_HEAL.md`
  is marked superseded/on hold; implementation approval was never given.
- **Phase close-out question remains unanswered** (posed 2026-07-17):
  close hardening and move to the Tauri reference-app phase? The owner
  detoured to hands-on validation; resolve the validation evidence first.
- **Parked hardening follow-ups**, source rechecked 2026-09-25 as of
  `a891ed5`: Scrivener asset import still calls `fs::copy` outside core and
  reports failures to stderr (`crates/cli/src/scrivener/converter/mod.rs`).
  Revision staging still uses `add_all(["*"])` while generated ignores do
  not cover atomic `.X.tmp-*` names (`crates/core/src/core/git.rs`). Neither
  concern has implementation approval. The rest of the format-lock audit
  lives under Out of scope in `docs/plans/PLAN_FORMAT_LOCK_ENGINE.md`;
  current reader repair notices remain stderr-only and `Project` still has
  no project-level `fields` map.
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

- **Break-it steps 9–10 need a desktop-app rerun** that an agent without
  GUI control cannot do: build the current app, recreate the sandbox, delete
  `.git`, fully quit and relaunch, save a revision, then restore. Open
  questions for that run: did the 2026-08-29 run truly reopen, and why was
  no error shown (the code does toast on save failure).
- **Validation evidence is not ready to resume in place.** The recorded
  test sandbox is absent on the current machine (see `.agents/machines.md`).
  The same run's corrupt-sidecar quarantine also differs from current
  source: the fidelity probe classifies corruption as Degraded and reads
  do not quarantine. Preserve the historical observations, recreate a copy
  of the test project, and identify the app build for the next run.

## Known drift (recorded, not yet fixed)

- None known.

## Next

1. **GUI rerun of break-it steps 9–10** on a freshly built current app
   (owner, or a GUI-capable agent), per `.agents/break-it-validation.md`
   "Engine repro 2026-09-25". Rewrite or retire the history plan from what
   it shows.
2. Then re-ask whether to close hardening and move to the Tauri
   reference-app phase. The phase declaration remains the owner's call.
3. Scrivener asset-import safety remains the next parked concern, one
   approval and one concern at a time. Vault work remains paused.

## Verification

- Declared suite: `.agents/repo-guidance.md` Verification section.
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

# Agent State

First read for current repo state at session start. Session-level layer only —
`docs/CURRENT_PHASE.md` is the authoritative phase plan; `docs/adr/` holds
decisions; `DEVLOG.md` holds history. Completed state entries live in
`docs/history/state-archive.md`.

## Now

- Phase **Engine hardening — protect writers' work** remains active;
  `docs/CURRENT_PHASE.md` owns the phase and exit criteria.
- **Owner goal 2026-09-25: working app — find, plan, fix, Codex review.**
  Hands-on GUI testing found 13 issues; all fixed one commit each
  (`.agents/app-issues.md` is the tracker). Headline: Restore and draft
  switching were silently undone on disk (fixed, GUI re-verified); a
  project inside another broke all revision saves (fixed + now refused);
  the red close button never closed the window (fixed). Codex
  defect review over `a4d39f1..e2b3d1b` dispatched; results go to
  `.agents/review/`.
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

- None known.

## Next

1. **Finish the Codex review** of `a4d39f1..e2b3d1b`: triage each finding
   per `.agents/playbooks/codereview.md`, fix admitted ones one commit
   each, record verdicts in `.agents/review/`.
2. Then re-ask whether to close hardening and move to the Tauri
   reference-app phase. The phase declaration remains the owner's call.
   Vault work remains paused.

## Verification

- Declared suite: `.agents/repo-guidance.md` Verification section.
- Full declared suite plus the CI cross-frontend harness ran green
  locally 2026-09-25 **as of `e2b3d1b`**. Desktop GUI checks of each fix
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

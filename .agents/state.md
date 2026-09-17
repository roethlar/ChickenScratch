# Agent State

First read for current repo state at session start. Session-level layer only —
`docs/CURRENT_PHASE.md` is the authoritative phase plan; `docs/adr/` holds
decisions; `DEVLOG.md` holds history. Completed state entries live in
`docs/history/state-archive.md`.

## Now

- Phase **Engine hardening — protect writers' work** remains active;
  `docs/CURRENT_PHASE.md` owns the phase and exit criteria.
- **Break-it validation needs a current-build recheck before a fix is chosen.**
  `.agents/break-it-validation.md` owns the 2026-08-29 observations and
  unfinished steps. `docs/plans/PLAN_HISTORY_SELF_HEAL.md` was drafted on
  the owner's go to draft a plan; implementation approval is not recorded.
  Its claim that nothing rebuilds missing history contradicts current code
  (see Blockers). Keep the observed release-app failure as evidence without
  treating its proposed cause as verified at the current source revision.
- **Phase close-out question remains unanswered** (posed 2026-07-17):
  close hardening and move to the Tauri reference-app phase? The owner
  detoured to hands-on validation; resolve the validation evidence first.
- **Parked hardening follow-ups**, source rechecked 2026-09-17 as of
  `a2d8a39`: Scrivener asset import still calls `fs::copy` outside core and
  reports failures to stderr (`crates/cli/src/scrivener/converter/mod.rs`).
  Revision staging still uses `add_all(["*"])` while generated ignores do
  not cover atomic `.X.tmp-*` names (`crates/core/src/core/git.rs`). Neither
  concern has implementation approval. The rest of the format-lock audit
  lives under Out of scope in `docs/plans/PLAN_FORMAT_LOCK_ENGINE.md`;
  current reader repair notices remain stderr-only and `Project` still has
  no project-level `fields` map.
- **Draft-peek follow-ups remain unapproved**, rechecked as of `a2d8a39`:
  `DocumentHistory.tsx` still uses the old Restore wording. The owner
  decision on backing up all drafts remains open; `push_remote` still
  pushes the current draft only. The completed review record is
  `.agents/review/index.md` (dp-1 through dp-3).
- Vault/remote design remains deliberately paused under
  `docs/CURRENT_PHASE.md` and Slice 2 of
  `docs/plans/PLAN_TRUST_FOUNDATIONS.md`, including the guided-token
  question. No new approval was found in the current records.

## Blockers

- **The history-fix plan's diagnosis is contradicted by source**, verified
  2026-09-17 as of `a2d8a39`: `load_project_inner` calls
  `reader::read_project_with_repair`; its SelfHeal path calls
  `pre_repair_git`, which recreates an absent repository through
  `git::init_repo`. Existing reader tests cover missing and unopenable
  repositories. The validation record does not identify the signed
  bundle's source revision, so its observed no-op cannot establish that
  this path is absent from current code. Reproduce with an identified
  current build before approving implementation; the plan itself was not
  edited during this records-only sweep.
- **Validation evidence is not ready to resume in place.** The recorded
  test sandbox is absent on the current machine (see `.agents/machines.md`).
  The same run's corrupt-sidecar quarantine also differs from current
  source: the fidelity probe classifies corruption as Degraded and reads
  do not quarantine. Preserve the historical observations, recreate a copy
  of the test project, and identify the app build for the next run.

## Known drift (recorded, not yet fixed)

- `docs/AGENT-WORKFLOW.md` section 5 still lists the earlier narrow command
  set while `.agents/repo-guidance.md` records the current CI-matching
  Rust/UI suite. Rechecked as of `a2d8a39`; the workflow also runs its
  cross-frontend harness and Windows process tests. The conflicting
  workflow guide is outside the records-only cleanup scope.

## Next

1. **Recheck the history-loss validation against an identified current
   build**, using a fresh copy of the test project. Trace the existing
   self-heal path, then revise `docs/plans/PLAN_HISTORY_SELF_HEAL.md` only
   from confirmed behavior before seeking implementation approval.
   Finish the blocked restore validation after history works.
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
- This catchup changed records only. `git diff --check` is the applicable
  local verification; Rust/UI tests and desktop validation were not run.
  Source inspection establishes the plan contradiction, not the runtime
  cause of the historical release-app failure.

## Active Sources

- `AGENTS.md` · `.agents/repo-guidance.md` · `docs/INVARIANTS.md` · `docs/CURRENT_PHASE.md`
- `.agents/decisions.md` (pointer to `docs/adr/`)

## Unrecorded Repo Memory

- None known. The completed legacy review cycle is archived in `REVIEW.md`
  and `.review/`; the format-lock audit owns its out-of-scope findings in
  `docs/plans/PLAN_FORMAT_LOCK_ENGINE.md`.

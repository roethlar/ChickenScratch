# Agent State

First read for current repo state at session start. Session-level layer only —
`docs/CURRENT_PHASE.md` is the authoritative phase plan; `docs/adr/` holds
decisions; `DEVLOG.md` holds history.

## Now

- Phase **Engine hardening — protect writers' work** is active as of
  2026-07-12 (`docs/CURRENT_PHASE.md` is authoritative). Its read-only audit
  and first approved safety slice are complete; the phase remains active.
- **Fresh-fidelity operation boundary shipped** 2026-07-12 in `a0e7621`
  (`docs/plans/PLAN_FRESH_FIDELITY_BOUNDARY.md`; guard proofs and full
  verification in the DEVLOG). Cached `WriteToken`s can no longer authorize a
  later operation by themselves: each logical mutation receives one freshly
  probed, short-lived `WritePermit`. Public/default reads and CLI export are
  pure; benign folder repair is explicit and permit-backed; corrupt sidecars
  stay in place. Tauri invalidates cached authority on refusal and opens each
  permit inside its project lock; TUI and converter boundaries match.
- **Ranked unapproved hardening follow-ups** (current evidence as of
  `a0e7621`): first, tree-replacing Git operations bump the write epoch only
  after every later step succeeds, so a partial post-mutation error can leave
  stale UI state authorized; second, `write_project` can return success for an
  in-memory document omitted from the hierarchy and leave the project
  immediately Degraded; third, Scrivener asset import still copies directly
  into `.chikn` outside core and reports failures only to stderr. Also parked:
  public `init_repo`, non-transactional multi-file saves, revision staging of
  recovery artifacts, and case-sensitive `include_in_compile`. None is
  approved for implementation yet.
- **New ranked finding (2026-07-16, plan-2 review rounds 9–10, verified
  against the working tree at `d851a8f`/`1d34cfe`): the conflict dialog's
  recovery exits are unreachable.** (1) Abort after a format-file
  conflict: markers in `project.yaml` make the fidelity probe error
  (`fidelity.rs:333–:335`; `load_project` then fails outright after
  restart), and markers in a `.meta` probe Degraded — either way
  `ProjectTokens` cannot reissue a permit, and `sync_abort_pull` is
  permit-gated (`src-tauri/src/commands/git.rs:238–:253`). (2) Force
  ("Overwrite local with remote") after *any* conflict: the conflicted
  tree is necessarily status-dirty, and `sync_pull_force` runs
  `reject_dirty_worktree` (`git.rs:1042`, `:1059`) plus
  `revalidate_fidelity` (`:1045`), while `handleForcePull`
  (`Revisions.tsx:257–:277`) never aborts first — so the dialog's third
  exit has never worked from a real conflict. In the worst case the
  user's only exit is external git surgery. Live today, independent of
  the epoch-guard plan; `docs/plans/PLAN_TREE_REPLACE_EPOCH_GUARD.md`
  (design point (e)) carries the fix as a recovery-scoped capability +
  merge-attested force path. Not separately approved for implementation.
- **Coherence is complete.** The owner confirmed on 2026-07-12 that completion
  had already been declared but not saved. Format lock, Tauri alignment,
  deprecation cleanup, and goals G1–G6 are recorded as completed in
  `docs/CURRENT_PHASE.md` and the shipped plans/DEVLOG entries it points to.
- **Write-guard shipped** (2026-07-11, `docs/plans/PLAN_TRUST_FOUNDATIONS.md`
  Slice 1; DEVLOG top entry has the slice-by-slice detail and guard
  proofs): side-effect-free fidelity probe + non-forgeable root-bound,
  epoch-stamped `WriteToken` established the original gate (now extended by
  the operation-scoped `WritePermit` above); Degraded
  projects open read-only in Tauri (banner + disabled affordances +
  skipped auto-saves), TUI (status + refused keys), and never receive a
  byte of writes — including the Statistics writing-history path.
  Same-day follow-up on owner ruling (no orphaned-file support pre-ship):
  the id-inheritance reader shim was reverted; `samples/Corn.chikn`
  regenerated with the current converter; binder-referenced binary assets
  (research PDFs etc.) are now fidelity-neutral while present, and the
  writer structurally refuses text writes into non-.md paths (DEVLOG top
  entry, follow-up paragraph).
- **Push status:** Gitea `origin` and `github` were both verified at `d99bf79`
  on 2026-07-12. After this close-out, local `master` is six commits ahead,
  including code tip `a0e7621`; nothing has been pushed because push policy
  requires the owner's explicit go. Remote CI has therefore not run on these
  local commits.

## Blockers

- None. Vault remote design is deliberately paused work, not a blocker to
  the active engine-hardening phase.

## Known drift (recorded, not yet fixed)

- `.agents/repo-map.json` still carries a 2026-07-09 verification snapshot
  that says CI/release metadata are red and lists the earlier narrow command
  set. Current CI and `.agents/repo-guidance.md` show those claims are stale.
- `docs/AGENT-WORKFLOW.md` section 5 still lists the earlier narrow command
  set while `.agents/repo-guidance.md` records the current CI-matching suite.
  Reconcile both in a deliberate drift pass; neither blocks this phase
  transition.

## Next

1. **Tree-replacement epoch invalidation on partial failure — APPROVED;
   slices 1–3 of 4 landed 2026-07-16, slice 4 mid-flight
   (UNCOMMITTED working-tree changes).** Owner chose "4 pieces"
   (recorded on the plan's status line). Landed: slice 1 vitest
   harness + CI (`cd6afdd`), slice 2 core epoch guard with
   red/green-proven error-path tests (`db8095a`), slice 3 UI operation
   barrier — counted lease, refuse-never-defer dispatch gate, owner
   admission, generation-keyed rebuilds, form freeze/loud-drop, timer
   gating, 26 vitest regressions (`977095b`). Plan:
   `docs/plans/PLAN_TREE_REPLACE_EPOCH_GUARD.md` (review accepted
   round 14; trail in `.agents/review/findings/plan-2.md`).
   **Slice 4 state (merge completion/recovery; fixes the two live
   defects in the ranked entry above): backend written and
   `cargo check`-clean, uncommitted** —
   `fidelity.rs` (attest_merge_in_progress, merge_fingerprint,
   `RecoveryPermit` bound to MERGE_HEAD OID + status fingerprint,
   fails closed on drift), `git.rs` (save_revision blanket
   merge-state refusal with self-describing message;
   reject_merge_in_progress preflights in both restores BEFORE any
   disk write; `MergeState` + `merge_state()`; `complete_merge()`
   two-parent + cleanup_state, epoch via drop guard;
   `force_resolve_merge()` resets to MERGE_HEAD = "theirs" for BOTH
   conflict origins; `sync_abort_pull` now takes `RecoveryPermit`;
   `backup_current_work` skips the commit half mid-merge, still
   pushes), `reader.rs` (`read_project_recovery` + HEAD
   project.yaml fallback, strict hierarchy matching relaxed),
   Tauri (`sync_abort_pull`/new `merge_state`/`complete_merge`/
   `force_resolve_merge` commands via recovery authority, registered
   in main.rs; `load_project` recovery fallback opens mid-merge
   projects read-only after restart; `backup_on_close` no longer
   swallows non-merge save errors), TUI (`app.rs` `{:?}`→Display at
   the revision-failure and backup-failure status lines).
   **Remaining for slice 4:** (a) UI — gated wrappers for
   mergeState/completeMerge/forceResolveMerge in
   `ui/src/commands/git.ts`; persistent merge-in-progress banner
   keyed on the merge_state query (survives restart) with
   Complete/Abort; Complete runs under runEpochOperation WITH drain,
   Abort/Force with skipDrain; ConflictDialog's "Overwrite local
   with remote" re-wired to force_resolve_merge (old syncPullForce
   remains only as the outside-merge Settings escape hatch);
   (b) core tests + red/green drills — save_revision refusal
   (conflicts AND lingering MERGE_HEAD; drill = revert refusal,
   markers get committed), restore-preflight asserting ZERO worktree
   mutation, complete_merge (two parents, cleanup, epoch bumped at
   exit), abort/complete/force through a fresh command boundary with
   a conflicted project.yaml (live-bug regression), force fail-closed
   on fingerprint drift, read_project_recovery (conflicted yaml +
   HEAD/worktree skew loads as unlinked); (c) full declared suite,
   one commit, then: update this entry, DEVLOG entry for the shipped
   plan (AGENT-WORKFLOW §6), re-verify/annotate the ranked live-bug
   entry as fixed, short plain-English owner handoff.
2. Keep writer end-state coherence and the Scrivener asset-import boundary
   parked behind that decision, one concern per later approval.
3. Slice 2 (vault) remains NOT approved. No vault work until a fresh owner
   decision on remote design and the plan's open v1 guided-token question.

## Verification

- Declared suite: `.agents/repo-guidance.md` Verification section (canonical
  command set; do not copy a second enumeration here).
- Last run green locally 2026-07-12 at code tip `a0e7621`: the exact declared
  suite, all targeted fresh-fidelity tests, and the three temporary red/green
  guard drills. Remote CI has not run because the commits remain unpushed.

## Active Sources

- `AGENTS.md` · `.agents/repo-guidance.md` · `docs/INVARIANTS.md` · `docs/CURRENT_PHASE.md`
- `.agents/decisions.md` (pointer to `docs/adr/`)
- `.agents/repo-map.json`

## Unrecorded Repo Memory

- None known. The completed 63-finding review cycle is archived in
  `REVIEW.md` + `.review/`; its hardening (safe paths, keyring secrets,
  dirty-worktree guards, process limits) is covered by `docs/INVARIANTS.md`
  I5–I6 and the engine tests. The format-lock audit's out-of-scope findings
  (non-transactional multi-file save, `save_revision` staging quarantine and
  orphaned temp files, stderr-only quarantine notice, `include_in_compile`
  case sensitivity, no project-level `fields` map) are recorded in
  `docs/plans/PLAN_FORMAT_LOCK_ENGINE.md` under "Out of scope".

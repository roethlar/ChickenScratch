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
- **Tree-replacement epoch guard plan is COMPLETE** — all four slices
  landed 2026-07-16 (`cd6afdd`, `db8095a`, `977095b`, `354fbc0`); DEVLOG
  top entry has the slice-by-slice detail, the codex review trail
  (`.agents/review/findings/s4-1..4.md`, all accepted), and the guard
  proofs. The two live bugs previously ranked here — the conflict
  dialog's unreachable Abort after a format-file conflict and its
  never-working Force exit — are **fixed** by slice 4's recovery
  authority (`RecoveryPermit`), merge-state query, persistent
  merge-in-progress UI, and attestation-bound `force_resolve_merge`.
- **Three owner-directed hardening fixes landed 2026-07-16** as
  per-concern commits on top of the epoch-guard work: `ae6bc01`
  canonicalizes `include_in_compile` YAML-1.1 boolean spellings at parse
  time (tolerant reader; unrecognized strings pass through verbatim);
  `0d3c027` verifies and self-heals the git repo on project open
  (`pre_repair_git` in RepairMode — missing repo recreated, unopenable
  repo left untouched and reported); `d6fa9b5` orders multi-file saves
  crash-safe by writing `project.yaml` last, so a torn save degrades to
  orphan content files (reconciled by self-heal) instead of a manifest
  referencing never-written documents. The first and third close the
  case-sensitive `include_in_compile` and non-transactional multi-file
  save findings parked in `PLAN_FORMAT_LOCK_ENGINE.md` — DEVLOG top
  entry has the guard-test detail.
- **Ranked unapproved hardening follow-ups** (re-based on `d6fa9b5`):
  first, `write_project` can return success for an in-memory document
  omitted from the hierarchy and leave the project immediately Degraded;
  second, Scrivener asset import still copies directly into `.chikn`
  outside core and reports failures only to stderr. Also parked: public
  `init_repo` (still `pub` — the open-time verify above did not change
  API visibility) and revision staging of recovery artifacts. None is
  approved for implementation yet.
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

1. **Await the owner's direction on the next hardening concern.** The
   epoch-guard plan and the three owner-directed fixes are complete
   (see ## Now); the next ranked follow-ups
   (writer end-state coherence, Scrivener asset-import boundary) remain
   parked, one concern per later approval. Per `CURRENT_PHASE.md` Step 3,
   a close-out re-audit of engine mutation entry points is the natural
   next proposal once the owner weighs in.
2. Slice 2 (vault) remains NOT approved. No vault work until a fresh owner
   decision on remote design and the plan's open v1 guided-token question.

## Verification

- Declared suite: `.agents/repo-guidance.md` Verification section (canonical
  command set; do not copy a second enumeration here).
- Last run green locally 2026-07-16 at code tip `d6fa9b5` (rustc 1.97.0,
  current stable — no CI-version gap): the exact declared suite. Each of
  the three new commits was additionally verified in isolation via
  `git rebase --exec` (core lib check + lib tests per commit), so every
  intermediate commit builds and passes.
- Prior green at `354fbc0` included the slice-4 red/green guard drills
  (per finding and per protection; DEVLOG epoch-guard entry).
- Remote CI green as of `c6993a9` (2026-07-16): GitHub Validation (6m07s)
  and Tauri Bundles (13m08s) both passed on the pushed epoch-guard work.

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

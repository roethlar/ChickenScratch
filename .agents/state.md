# Agent State

First read for current repo state at session start. Session-level layer only —
`docs/CURRENT_PHASE.md` is the authoritative phase plan; `docs/adr/` holds
decisions; `DEVLOG.md` holds history.

## Now

- Phase **Engine hardening — protect writers' work** is active as of
  2026-07-12 (`docs/CURRENT_PHASE.md` is authoritative). Its read-only first
  step is complete; advancing the phase did not approve a code slice.
- **Engine-hardening audit completed** 2026-07-12 (read-only; verified
  against `e4acb69`, whose code is unchanged from `e17ceeb`). Highest risk:
  cached `WriteToken`s detect only in-process epoch changes, not external disk
  changes, while public `read_project` still performs repair/quarantine writes
  without a probe or token; CLI export calls that reader directly. A project
  can therefore open Full, become Degraded outside the app, then have newer
  format data downgraded or corrupt metadata replaced on a later read/write.
  Ranked follow-ups: Scrivener asset import writes directly into `.chikn`
  outside core and reports copy failures only to stderr; public `init_repo`
  mutates without a token and creation suppresses its errors; multi-file saves
  remain non-transactional; revision saves stage quarantine/crash-temp
  artifacts; `include_in_compile` strings are case-sensitive. The old
  stderr-only quarantine finding is resolved for reference-app opens, and a
  project-level `fields` map is an extensibility choice, not an integrity bug.
- **Coherence is complete.** The owner confirmed on 2026-07-12 that completion
  had already been declared but not saved. Format lock, Tauri alignment,
  deprecation cleanup, and goals G1–G6 are recorded as completed in
  `docs/CURRENT_PHASE.md` and the shipped plans/DEVLOG entries it points to.
- **Write-guard shipped** (2026-07-11, `docs/plans/PLAN_TRUST_FOUNDATIONS.md`
  Slice 1; DEVLOG top entry has the slice-by-slice detail and guard
  proofs): side-effect-free fidelity probe + non-forgeable root-bound,
  epoch-stamped `WriteToken` gate every mutating engine API; Degraded
  projects open read-only in Tauri (banner + disabled affordances +
  skipped auto-saves), TUI (status + refused keys), and never receive a
  byte of writes — including the Statistics writing-history path.
  Same-day follow-up on owner ruling (no orphaned-file support pre-ship):
  the id-inheritance reader shim was reverted; `samples/Corn.chikn`
  regenerated with the current converter; binder-referenced binary assets
  (research PDFs etc.) are now fidelity-neutral while present, and the
  writer structurally refuses text writes into non-.md paths (DEVLOG top
  entry, follow-up paragraph).
- **Push status** (verified 2026-07-12 before this phase-transition commit):
  local `master`, Gitea `origin`, and `github` were all at `d99bf79`;
  Validation at that bookkeeping tip is green. The latest code tip
  (`e17ceeb`) also has green Validation and Tauri Bundles runs. This
  phase-transition commit remains local until the owner gives a push go.

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

1. **Slice 1 approved 2026-07-12; implementation pending:** seal the
   fresh-fidelity operation boundary per
   `docs/plans/PLAN_FRESH_FIDELITY_BOUNDARY.md`. Public/default reads become
   side-effect-free; every logical mutation receives one freshly probed,
   operation-scoped permit reused across its internal steps. Guard proof:
   acquire a session token, externally degrade the fixture, then permit
   issuance and every public read leave the tree byte-identical.
2. After Slice 1, propose the Scrivener asset-import boundary as the next
   single concern; keep the remaining ranked findings parked above.
3. Slice 2 (vault) remains NOT approved. No vault work until a fresh owner
   decision on remote design and the plan's open v1 guided-token question.

## Verification

- Declared suite: `.agents/repo-guidance.md` Verification section (fmt check,
  clippy, core lib tests, Tauri bin tests, UI lint + build).
- Last run green locally 2026-07-11 before every Slice 1 commit (owner's
  Mac), plus `cargo test -p chickenscratch-core --tests` (write_guard,
  remote_sync, round-trip suites) and clippy on the TUI and converter
  crates. Guard proofs for the new write-guard tests are recorded in the
  DEVLOG top entry.

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

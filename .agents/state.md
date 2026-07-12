# Agent State

First read for current repo state at session start. Session-level layer only —
`docs/CURRENT_PHASE.md` is the authoritative phase plan; `docs/adr/` holds
decisions; `DEVLOG.md` holds history.

## Now

- Phase **Engine hardening — protect writers' work** is active as of
  2026-07-12 (`docs/CURRENT_PHASE.md` is authoritative). The first step is a
  read-only re-verification and prioritization of the existing integrity-risk
  record; advancing the phase does not approve a code slice.
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

1. Run the read-only hardening audit defined in `docs/CURRENT_PHASE.md`:
   re-verify existing integrity findings against current code, then present
   one highest-risk proposed slice in plain English for approval.
2. Slice 2 (vault) remains NOT approved. No vault work until a fresh owner
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

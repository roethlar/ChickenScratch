# Agent State

First read for current repo state at session start. Session-level layer only —
`docs/CURRENT_PHASE.md` is the authoritative phase plan; `docs/adr/` holds
decisions; `DEVLOG.md` holds history.

## Now

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
- Phase **Coherence** (`docs/CURRENT_PHASE.md` is authoritative): Steps 1–3
  effectively done; deprecation cleanup (G4–G6) and ADR-005 binary-only
  distribution executed 2026-07-10 (DEVLOG entries; archived detail in
  `docs/history/state-archive.md`).
- **Push status** (verified 2026-07-11 via git + `gh`): the accumulated
  commits are pushed; local `master`, Gitea `origin` (http://q:3000), and
  `github` (https://github.com/roethlar/ChickenScratch) are all at
  `e17ceeb`. GitHub CI at the tip is green — Validation (run 29147248690)
  and Tauri Bundles (run 29147248729) both succeeded, all four check-runs
  passing. Note: three intermediate Slice 1 commits have red Validation
  runs (Darwin EPERM killing an exited-leader process group, fixed by
  `e17ceeb` itself), so mid-stack commits on master are not individually
  green; the branch tip is.

## Blockers

- None. (The release-metadata blocker recorded here earlier on 2026-07-10
  was resolved the same day by ADR-005: the Arch packaging and its pinned
  checksum were removed; `check-release-metadata.sh` passes.)

## Known drift (recorded, not yet fixed)

The 2026-07-10 cleanup (`docs/plans/PLAN_DEPRECATION_CLEANUP.md`) resolved
the CI/release/README/ARCHITECTURE/ROADMAP entries formerly listed here
(archived detail: DEVLOG top entry). Remaining:

- ~~Doc stragglers~~ swept 2026-07-10 on owner yes (USER_GUIDE,
  EDITOR_DESIGN scope line, TODO parity section, I3/glossary wording —
  one commit each). Remaining mentions are intentional: historical docs
  (`docs/GPT_Code_Review.md`, EDITOR_DESIGN's date-stamped April tree),
  and harmless `.gitignore` / `.gitattributes` patterns.
- `docs/CURRENT_PHASE.md` Step 4 lists "Add `DEPRECATED.md` stubs in
  `macos/`, `windows/`, `linux/`" — moot now the trees are deleted (G6 is
  satisfied by deletion + ADR-004). Owner-controlled file; report-only.

## Next

1. Slice 2 (vault) is pending and NOT approved: the owner said "we're
   nowhere near deciding how remotes will work". No vault work until a
   fresh owner decision on remote design (and the plan's open v1
   guided-token question).
2. Owner decision: G4/G5 work is done and G2/G3/G6 look met — declare
   goals met / advance the phase via `SET PHASE`, or name the next work.
   Checkbox edits in `docs/CURRENT_PHASE.md` are the owner's call.

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

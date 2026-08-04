# Agent State

First read for current repo state at session start. Session-level layer only —
`docs/CURRENT_PHASE.md` is the authoritative phase plan; `docs/adr/` holds
decisions; `DEVLOG.md` holds history.

## Now

- Phase **Engine hardening — protect writers' work** is active as of
  2026-07-12 (`docs/CURRENT_PHASE.md` is authoritative). Its read-only audit
  and first approved safety slice are complete; the phase remains active.
- **Owner-level break-it validation is IN FLIGHT, not yet run**
  (2026-07-17): script at `.agents/break-it-validation.md` — import a
  copy of the owner's "Corn 2.scriv", then crash/corrupt/strand/delete
  attacks against the imported project. First driver attempt (Claude
  Cowork) failed on this machine (`.agents/machines.md`); needs a
  desktop-automation-capable session or owner-driven run. Any LOST DATA
  verdict is a hardening bug and jumps the queue.
- **Phase close-out question posed to the owner, unanswered**
  (2026-07-17): "close the hardening phase and move to the Tauri
  reference-app phase?" The owner detoured to hands-on validation
  first (above) — re-ask after the validation runs.
- **Ranked unapproved hardening follow-ups** (re-based on `7f85510`):
  first, Scrivener asset import still copies directly into `.chikn`
  outside core and reports failures only to stderr. Also parked:
  revision staging of recovery artifacts. Neither is approved for
  implementation yet.
## Blockers

- None. Vault remote design is deliberately paused work, not a blocker to
  the active engine-hardening phase.

## Known drift (recorded, not yet fixed)

- `docs/AGENT-WORKFLOW.md` section 5 still lists the earlier narrow command
  set while `.agents/repo-guidance.md` records the current CI-matching suite.
  Reconcile in a deliberate pass; it does not block the phase.

## Next

1. **Run the break-it validation** (`.agents/break-it-validation.md`)
   with a driver that can operate the desktop app — Cowork failed; use a
   desktop-automation-capable session or walk the owner through it.
   Report the verdict table; any LOST DATA finding becomes the next fix.
2. **Then re-ask the phase close-out yes/no**: hardening phase done,
   move to the Tauri reference-app phase? (Posed 2026-07-17, unanswered.)
   The engine-side close-out evidence is in: mutation entry points
   re-audited 2026-07-17 (all permit-gated; `init_repo` gated behind
   `test-support`), parked findings re-verified against tip.
3. The next ranked follow-up (Scrivener asset-import boundary) remains
   parked, one concern per later approval.
4. Slice 2 (vault) remains NOT approved. No vault work until a fresh owner
   decision on remote design and the plan's open v1 guided-token question.

## Verification

- Declared suite: `.agents/repo-guidance.md` Verification section (canonical
  command set; do not copy a second enumeration here).
- 2026-07-27 at merge `260c744` (worktree branch `init_repo` gating merged
  into master; worktree and branch removed after content-landed proof):
  the full declared suite green locally — fmt, clippy, all Rust tests,
  release metadata, and the ui lint/build/test portion.
- Last full-suite green before that: 2026-07-16 at code tip `d6fa9b5` (rustc 1.97.0,
  current stable — no CI-version gap): the exact declared suite. Each of
  the three new commits was additionally verified in isolation via
  `git rebase --exec` (core lib check + lib tests per commit), so every
  intermediate commit builds and passes.
- 2026-07-16 at `6b4b4ac` (write_project hierarchy guard): `cargo fmt
  --check`, `cargo clippy --all-targets -- -D warnings`, and full test
  suites green across all four crates. Rust portion of the declared
  suite only — the ui/release-metadata portions were untouched by this
  work and last ran green at `d6fa9b5`.
- 2026-07-16 at `7f85510` (`init_repo` test-support gating, on top of
  the master merge `f49d57d`): `cargo fmt --check`, `cargo clippy
  --all-targets -- -D warnings`, and `cargo test --workspace
  --all-targets` green. Rust portion of the declared suite only, as
  above.
- Prior green at `354fbc0` included the slice-4 red/green guard drills
  (per finding and per protection; DEVLOG epoch-guard entry).
- Remote CI as of `552706d` (checked live 2026-07-28): Tauri Bundles green;
  GitHub Validation red on `utils::process::tests::process_helper_caps_
  combined_stdout_and_stderr` with Darwin `EPERM` on the macOS runner — the
  same flake class tolerated by the 2026-07-11 process-helper fix, and the
  identical code passed Validation on `262d00b` and locally, so this reads
  as a runner flake, not a regression. A re-run should confirm.

## Active Sources

- `AGENTS.md` · `.agents/repo-guidance.md` · `docs/INVARIANTS.md` · `docs/CURRENT_PHASE.md`
- `.agents/decisions.md` (pointer to `docs/adr/`)

## Unrecorded Repo Memory

- None known. The completed 63-finding review cycle is archived in
  `REVIEW.md` + `.review/`; its hardening (safe paths, keyring secrets,
  dirty-worktree guards, process limits) is covered by `docs/INVARIANTS.md`
  I5–I6 and the engine tests. The format-lock audit's out-of-scope findings
  are recorded in `docs/plans/PLAN_FORMAT_LOCK_ENGINE.md` under "Out of
  scope" — re-verified against tip 2026-07-16: still open are
  `save_revision` staging quarantine/orphaned temp files, stderr-only
  SelfHeal repair notices, and no project-level `fields` map; the
  non-transactional-save and `include_in_compile` case-sensitivity
  findings are closed there (`d6fa9b5`, `ae6bc01`).

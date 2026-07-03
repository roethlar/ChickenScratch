# Agent State

First read for current repo state at session start. Session-level layer only —
`docs/CURRENT_PHASE.md` is the authoritative phase plan; `docs/adr/` holds
decisions; `DEVLOG.md` holds history.

## Now

- Phase **Coherence** (`docs/CURRENT_PHASE.md`, started 2026-06-07): Step 1
  (governance docs) landed; native frontends (`macos/`, `windows/`, `linux/`)
  were **deleted from the tree** in the same change set; Scrivener
  import/export moved from the engine into `crates/cli`.
- Governance refresh in progress (2026-07-03): `AGENTS.md` reconciled to the
  current AgentGovernanceBootstrap template (portable, generic); repo-specific
  rules carved into `.agents/repo-guidance.md`; harness command wrappers,
  hooks, and shims brought up to date. See `.agents/decisions.md` for why.
- No other feature work in flight.

## Blockers

- **The build is broken on `master`.** Root `Cargo.toml` still lists the
  deleted `linux/` directory as a workspace member, so every `cargo` command
  fails (`cargo metadata` errors on the missing manifest). The verification
  suite cannot run until this one-line removal is made and committed.
- There is an **uncommitted, unverified** local edit to `Cargo.toml` (present
  in the working tree as of 2026-07-03, confirmed via `git diff Cargo.toml`)
  that removes `linux` from the members list — the plausible fix for the
  blocker above. It has not been committed and the suite has not been run
  against it yet. Fixing and verifying this is a code change that needs an
  owner work request.

## Known drift (recorded, not yet fixed)

Left over from the native-tree deletion; most are anticipated by goals G4–G6
in `docs/CURRENT_PHASE.md`:

- `Cargo.toml` workspace members include deleted `linux/` (the build-breaker above).
- `.github/workflows/validation.yml` sets up Swift/.NET and resolves the
  deleted `macos/` and `windows/` trees; the cross-frontend step calls
  `crates/core/tests/cross_frontend/run.sh`, whose Swift/C# harnesses no
  longer exist. CI is red on `master`: `gh run list --branch master --limit 5`
  (2026-07-03) shows the most recent `Validation`, `Windows (WinUI 3)`, and
  `Tauri Bundles` runs on `master` all `completed failure`.
- `.github/workflows/windows.yml` and `macos-release.yml` target deleted trees.
- `scripts/check-release-metadata.sh` (line ~87) checks the deleted
  `linux/Cargo.toml`; `RELEASE.md` lists deleted files among release-version
  updates and runs Swift/.NET validation steps.
- `README.md` still shows the five-platform table (goal G4).
- `docs/ARCHITECTURE.md` describes `macos/`, `windows/`, `linux/` as
  present-but-deprecated; they are deleted.
  `scripts/check-nuget-package-versions.ps1` is orphaned.

## Next

1. Owner decision: fix the broken workspace (remove `linux` from
   `Cargo.toml` members — an uncommitted edit already does this, see
   Blockers) and trim CI/release scripts to engine + Tauri + converter + TUI
   (goals G5/G6). Small, well-defined work request.
2. Then Step 2 of the phase: format lock (genre-agnostic `fields` map, spec
   alignment, round-trip tests) per
   `docs/plans/PHASE_FORMAT_FINALIZATION.md`, engine scope only.

## Verification

- Declared suite: `.agents/repo-guidance.md` Verification section (fmt check,
  clippy, core lib tests, Tauri bin tests, UI lint + build).
- **Currently not runnable** — blocked by the broken workspace manifest above.
  Status recorded in `.agents/repo-map.json`.

## Active Sources

- `AGENTS.md` · `.agents/repo-guidance.md` · `docs/INVARIANTS.md` · `docs/CURRENT_PHASE.md`
- `.agents/decisions.md` (pointer to `docs/adr/`)
- `.agents/repo-map.json`

## Unrecorded Repo Memory

- None known. The completed 63-finding review cycle is archived in
  `REVIEW.md` + `.review/`; its hardening (safe paths, keyring secrets,
  dirty-worktree guards, process limits) is covered by `docs/INVARIANTS.md`
  I5–I6 and the engine tests.

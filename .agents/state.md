# Agent State

First read for current repo state at session start. Session-level layer only —
`docs/CURRENT_PHASE.md` is the authoritative phase plan; `docs/adr/` holds
decisions; `DEVLOG.md` holds history.

## Now

- Phase **Coherence** (`docs/CURRENT_PHASE.md`, started 2026-06-07): Step 1
  (governance docs) landed; native frontends (`macos/`, `windows/`, `linux/`)
  were **deleted from the tree** in the same change set; Scrivener
  import/export moved from the engine into `crates/cli`.
- No feature work in flight.

## Blockers

- **The build is broken on `master`.** Root `Cargo.toml` still lists the
  deleted `linux/` directory as a workspace member, so every `cargo` command
  fails (`cargo metadata` errors on the missing manifest). The verification
  suite cannot run until this one-line removal is made. Fixing it is a code
  change that needs an owner work request.

## Known drift (recorded, not yet fixed)

Left over from the native-tree deletion; most are anticipated by goals G4–G6
in `docs/CURRENT_PHASE.md`:

- `Cargo.toml` workspace members include deleted `linux/` (the build-breaker above).
- `.github/workflows/validation.yml` sets up Swift/.NET and resolves the
  deleted `macos/` and `windows/` trees; the cross-frontend step calls
  `crates/core/tests/cross_frontend/run.sh`, whose Swift/C# harnesses no
  longer exist. CI is red on `master`.
- `.github/workflows/windows.yml` and `macos-release.yml` target deleted trees.
- `scripts/check-release-metadata.sh` (line ~87) checks the deleted
  `linux/Cargo.toml`; `RELEASE.md` lists deleted files among release-version
  updates and runs Swift/.NET validation steps.
- `README.md` still shows the five-platform table (goal G4).
- `docs/ARCHITECTURE.md` and `AGENTS.md` Rule 2/4 describe `macos/`,
  `windows/`, `linux/` as present-but-deprecated; they are deleted.
  `scripts/check-nuget-package-versions.ps1` is orphaned.

## Next

1. Owner decision: fix the broken workspace (remove `linux` from
   `Cargo.toml` members) and trim CI/release scripts to engine + Tauri +
   converter + TUI (goals G5/G6). Small, well-defined work request.
2. Then Step 2 of the phase: format lock (genre-agnostic `fields` map, spec
   alignment, round-trip tests) per
   `docs/plans/PHASE_FORMAT_FINALIZATION.md`, engine scope only.

## Verification

- Declared suite: `AGENTS.md` Verify block and `docs/AGENT-WORKFLOW.md` §5
  (fmt check, clippy, core lib tests, Tauri bin tests, UI lint + build).
- **Currently not runnable** — blocked by the broken workspace manifest above.
  Status recorded in `.agents/repo-map.json`.

## Active Sources

- `AGENTS.md` · `docs/INVARIANTS.md` · `docs/CURRENT_PHASE.md`
- `.agents/decisions.md` (pointer to `docs/adr/`)
- `.agents/repo-map.json`

## Unrecorded Repo Memory

- None known. The completed 63-finding review cycle is archived in
  `REVIEW.md` + `.review/`; its hardening (safe paths, keyring secrets,
  dirty-worktree guards, process limits) is covered by `docs/INVARIANTS.md`
  I5–I6 and the engine tests.

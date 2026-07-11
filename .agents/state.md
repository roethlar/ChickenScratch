# Agent State

First read for current repo state at session start. Session-level layer only —
`docs/CURRENT_PHASE.md` is the authoritative phase plan; `docs/adr/` holds
decisions; `DEVLOG.md` holds history.

## Now

- Phase **Coherence** (`docs/CURRENT_PHASE.md` is authoritative): Steps 1–3
  are effectively done — governance landed 2026-07-03, the engine format
  lock shipped 2026-07-09 (`DEVLOG.md` top entry; plan:
  `docs/plans/PLAN_FORMAT_LOCK_ENGINE.md`, status Shipped), and Tauri
  alignment (Step 3) was found already in place by the format-lock audit.
  What remains is deprecation cleanup (goals G4–G6). Landed-entry detail
  rotated to `docs/history/state-archive.md`.
- **Push status**: in sync as of `a41bdab` (2026-07-10) — the owner approved
  the push and local `master` now matches both remotes. Since the last
  handoff a second remote `github` (https://github.com/roethlar/ChickenScratch)
  exists alongside the Gitea `origin` (http://q:3000); pushes go to both.
  Push policy remains ask-first (`.agents/push-policy.md`).
- No feature work in flight.

## Blockers

- None.

## Known drift (recorded, not yet fixed)

Left over from the native-tree deletion; most are anticipated by goals G4–G6
in `docs/CURRENT_PHASE.md`:

- `.github/workflows/validation.yml` sets up Swift/.NET and resolves the
  deleted `macos/` and `windows/` trees; the cross-frontend step calls
  `crates/core/tests/cross_frontend/run.sh`, whose Swift/C# harnesses no
  longer exist — so CI runs are expected to fail wherever these workflows
  execute. Re-verified red 2026-07-10 via `gh run list` against the
  `github` remote: Validation failed on `aabfd05` (and every earlier
  recorded run); the Tauri Bundles workflow is green.
- `.github/workflows/windows.yml` and `macos-release.yml` target deleted trees.
- `scripts/check-release-metadata.sh` (line ~87) checks the deleted
  `linux/Cargo.toml`; `RELEASE.md` lists deleted files among release-version
  updates and runs Swift/.NET validation steps.
- `README.md` still shows the five-platform table (goal G4).
- `docs/ARCHITECTURE.md` describes `macos/`, `windows/`, `linux/` as
  present-but-deprecated; they are deleted. Its "Cargo workspace" line also
  predates the 2026-07-09 manifest fix (`linux` is no longer a member).
  `scripts/check-nuget-package-versions.ps1` is orphaned.
- `docs/ROADMAP.md` was partially refreshed 2026-07-09 (What's Built +
  current-phase section), but its header still says "Five frontends" and
  the v1.1 "Frontend parity (SwiftUI + Qt6 + WinUI)" section survives —
  G4 cleanup should sweep it too.

## Next

1. Owner decision: trim CI/release scripts to engine + Tauri + converter +
   TUI (goals G5/G6). CI on `master` stays red until this lands — the
   workflows still reference the deleted native trees (see Known drift).
   Small, well-defined work request. README/ARCHITECTURE cleanup (G4) rides
   along; ROADMAP's What's Built and phase sections were already refreshed
   with the format lock.
2. Owner decision: phase Step 2 (format lock) is done and Step 3 was found
   already in place — declare G2 met / advance the phase via `SET PHASE`,
   or name the next work.

## Verification

- Declared suite: `.agents/repo-guidance.md` Verification section (fmt check,
  clippy, core lib tests, Tauri bin tests, UI lint + build).
- Last run green locally 2026-07-09 as of `35f721a` (format-lock slice E;
  every slice A–E ran the full suite green before its commit; commits after
  `35f721a` are docs-only), machine-local (owner's Mac). Remote CI is
  expected red until the workflow drift above is fixed. Status recorded in
  `.agents/repo-map.json`.

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

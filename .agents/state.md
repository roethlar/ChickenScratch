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
- **Deprecation cleanup executed** (2026-07-10, owner-approved
  `docs/plans/PLAN_DEPRECATION_CLEANUP.md`): CI, release gate, and
  README/ARCHITECTURE/ROADMAP no longer reference the deleted native trees
  (G4/G5 work done; DEVLOG top entry). Validation CI will stay red at one
  step for a pre-existing reason — see Blockers.
- **Push status**: local `master` is ahead of both remotes by the cleanup
  commits (plan + slices + close-out); ask-first policy, owner not yet
  asked post-cleanup. Remotes: Gitea `origin` (http://q:3000) and `github`
  (https://github.com/roethlar/ChickenScratch); pushes go to both.
- No feature work in flight.

## Blockers

- **Release-metadata gate red for a pre-existing reason** (found 2026-07-10
  while executing the cleanup plan): `pkg/arch/PKGBUILD` sha256 was pinned
  at `faa9d54` (2026-05-18) for a v1.0.0 release that was never tagged (the
  repo has no tags). With version `1.0.0` (no prerelease suffix) the script
  runs in release mode and compares a HEAD source archive against the pin —
  only matchable at the frozen release commit. Validation CI stays red at
  "Release metadata" until the owner decides: switch to a prerelease
  version until release, restrict the archive comparison to `--release`
  runs, or finish/cut the v1.0.0 release. Decision pending.

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

1. Owner decision: how to resolve the pre-existing release-metadata
   blocker above — it is the only thing keeping Validation CI red.
2. Owner go for pushing the cleanup commits (ask-first policy).
3. Owner decision: G4/G5 work is done and G2/G3/G6 look met — declare
   goals met / advance the phase via `SET PHASE`, or name the next work.
   Checkbox edits in `docs/CURRENT_PHASE.md` are the owner's call.

## Verification

- Declared suite: `.agents/repo-guidance.md` Verification section (fmt check,
  clippy, core lib tests, Tauri bin tests, UI lint + build).
- Last run green locally 2026-07-10 during the deprecation cleanup (on the
  CI-trim slice tree and re-run at close-out; intervening commits touched
  only workflows, scripts, and docs), machine-local (owner's Mac). The
  rust-only format harness and `check-release-metadata.sh` (single
  remaining pre-existing error, see Blockers) were exercised directly.
  Remote Validation CI stays red only at the "Release metadata" step until
  the blocker decision lands.

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

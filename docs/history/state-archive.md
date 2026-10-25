# State Archive

Landed or superseded entries rotated verbatim out of `.agents/state.md` by
the `handoff` operator, newest batch first. History only — never load-bearing.

## Rotated 2026-10-25 (drift sweep as of `82ba252`)

Entries preserved verbatim, including their push claims; current verified
evidence is in `.agents/state.md`.

- **Owner 2026-10-25: "the install did not ask any new questions"** → fixed
  in `e56483d` (pushed 2026-10-25): an upgrade asks once for a setting the
  settings file has never seen, flags change one setting, `--yes` writes the
  placeholder. Dry-run verified; shellcheck clean.
- **Owner 2026-10-25: "plan" after the first install on a second machine**
  (projects on NFS at `/mnt/notes`): settings save fails there with a keyring
  D-Bus error; the backup folder can only be picked inside the root; git's
  ownership check fires because the share maps uids. Plan:
  `docs/plans/PLAN_WEB_SERVER_FIXES.md` — owner "finish all slices"; all
  three landed 2026-10-25 (`9449c4d` secrets fallback, `0f096eb` backup
  root, `a8db23d` ownership check off on the server by default, following
  the plan's recommendation since the go named all slices). Pushed
  2026-10-25.
- **Owner 2026-10-24: "get rid of the access token"** — done the same day
  (`docs/plans/PLAN_WEB_NO_LOGIN.md`, `.agents/decisions.md`): `chikn-web`
  has no login; the private network is the boundary. Token file, `?token=`
  link, UI token gate and `--token` flag are gone. Pushed as `aec8f18` on the
  owner's `/git push all`; Validation run `40157115663` failed only in the
  new Shell scripts step (macOS runner has no shellcheck) — fixed by
  installing it in the step (`8d587a2`, pushed on the owner's "push";
  Validation run `40157357274` green).
- **Owner 2026-10-24: web installer queued** ("this needs an installer for
  the web version. the default data dir should be set at install time, and
  here it will be on my NAS" … "a .config/ entry or similar. and a way to
  upgrade without re-specifying everything"). Plan:
  `docs/plans/PLAN_WEB_INSTALLER.md` — `web.toml` config read by
  `chikn-web`, `scripts/install-web.sh` with a user systemd service that
  waits for the NFS mount, rerun = upgrade. Owner "go" then "do all the
  slices": all three slices landed 2026-10-24 (`ff8ad9b` settings file,
  `1c214ec` installer + shellcheck gate, docs commit). Next evidence is the
  owner running `scripts/install-web.sh` on the server with the NAS path.
- **Owner 2026-10-24: project profiles shipped** (`docs/plans/PLAN_PROJECT_PROFILES.md`,
  go: "complete all slices and push when done"): Fiction / Notes / Document
  types show, hide or rename the novelist UI parts, display-only; New Project
  asks for the type. UI-only plus four backend templates. Pushed with the
  web-shell commits on the owner's instruction.
- **Owner 2026-10-23: web shell queued as a side quest, no phase change**
  ("no direction change, just the next side quest. plan it, add it to the
  queue"). Plan: `docs/plans/PLAN_WEB_SHELL.md` — a `chikn-web` server
  that serves the existing React UI over HTTP from a shared
  `chickenscratch-services` command layer. **Owner 2026-10-24: "follow the
  queue and get as much as you can done"** — the go for the plan's slices in
  order, one commit each. Slices 1 (services extraction, ADR-007), 2
  (frontend transport + platform seams), 3a (`chikn-web` server) and 3b (web
  Welcome) and 4a–4d (export download, upload import, server folder picker,
  Scrivener import) and 6 (embed-ui single binary, user guide, release
  runbook, records) landed 2026-10-24 — the browser now covers everything the
  desktop does, including live AI streaming (slice 5, SSE) and a single-
  writer lease for two devices on one project (slice 7). Every slice of the
  plan has shipped; DEVLOG has the entry.
- **Owner goal 2026-09-25: working app — find, plan, fix, Codex review.**
  Hands-on GUI testing found 13 issues; all fixed one commit each
  (`.agents/app-issues.md` is the tracker). Headline: Restore and draft
  switching were silently undone on disk (fixed, GUI re-verified); a
  project inside another broke all revision saves (fixed + now refused);
  the red close button never closed the window (fixed). Codex
  defect review over `a4d39f1..e2b3d1b` raised ar-1..ar-4; all fixed and
  verified accepted (`.agents/review/index.md`).
- **Break-it steps 7–10 rechecked on the current build (2026-09-25)**:
  deleted history rebuilds on reopen and on save; damaged sidecars open
  read-only with the real reason shown. `PLAN_HISTORY_SELF_HEAL.md` is
  superseded.

Rotated from "Known drift" in the same sweep (both fixed, kept there one
cycle as the record of cause):

- **CI red under rustc 1.99.1 — fixed and proven green** (Validation run
  `40105127486` at `150bfcf` succeeded on all three runners, 2026-10-24;
  the red run was `40071873531` at `3a8db4e`). Owner go "fix ci", then
  "push"; plan `docs/plans/PLAN_CI_RUST_199.md`. Kept here one cycle as the
  record of cause. (1) Windows: `libgit2-sys 0.17` relied
  on std linking `advapi32.lib`; std 1.99 binds that DLL via raw-dylib, so
  the core test runner failed to link. Fix `3f9024b`: `crates/core/build.rs`
  emits `rustc-link-lib=advapi32` on Windows. Alternative left open:
  upgrade `git2` 0.19 → 0.21. (2) macOS: XNU `killpg` returns `EPERM` for a
  group with no live member and marks the leader exited before it is
  reapable; `kill_tree` accepted `EPERM` only when `try_wait` already saw the
  exit, losing the race in `process_helper_caps_combined_stdout_and_stderr`.
  Fix `818eb2a`: on `EPERM`, poll `try_wait` up to 500 ms before believing
  the error. Neither fix is exercisable on the Linux box (both branches are
  inert here); the GitHub runners were the proof.
- **Flaky core tests — fixed 2026-10-25 (`52ad932`, test-only).**
  `failed_restore_after_tree_mutation…` (seen 2026-10-24) and
  `failed_complete_merge_after_staging…` (2 of 3 full runs on 2026-10-25)
  armed process-wide `AtomicBool` failpoints that any parallel test could
  consume; the flags are thread-local now. 5 of 5 full core runs green. Done
  without a separate go because it blocked verifying the web server fixes;
  no production code changed.

## Rotated 2026-09-17 (catchup hygiene sweep)

These entries are preserved verbatim, including superseded release claims;
current verified evidence is in `.agents/state.md`.

- **Draft peek + Restore rewording SHIPPED** (owner-approved
  `docs/plans/PLAN_DRAFT_PEEK.md`, 2026-08-29, commits
  `1ac3ddc`/`846aa10`/`8d69a08`): read-only look inside any other draft
  from the Drafts tab (core reads are pure git-object reads, guard test
  pins zero worktree/HEAD mutation), and History's restore is now "Bring
  back this version" with nothing-is-lost confirm wording. Codereview
  2026-08-29 (codex gpt-5.6-sol @ xhigh) over `c99acdd..0a891b2`:
  3 findings, all fixed and verified accepted (`.agents/review/index.md`
  dp-1..dp-3, commits `31c55c2`/`3b47d6f`/`4b70015`+`bfe52d1`) — backup
  wording now draft-scoped, restore now genuinely saves to History first,
  peek sidecar names YAML-parsed. Candidate follow-ups, not approved:
  `DocumentHistory.tsx` per-document restore still uses the old "Restore"
  wording; **owner decision open** — should "Back up" push *all* drafts
  instead of only the current one? (dp-1 fixed the words, not the scope.)
- **Windows signed release CI landed and PROVEN GREEN** (`229341f`
  2026-08-07; validated by run 31228420981 the same day):
  `.github/workflows/windows-release.yml` builds the first-ever Windows
  Tauri bundles in this repo and signs the MSI and NSIS installers via
  Azure Trusted Signing, with `Get-AuthenticodeSignature ... Status -eq
  'Valid'` hard-asserted and passing. The first attempt 403'd on a wrong
  repo secret (`AZURE_SIGNING_ACCOUNT` must be `roethlar-app-signing`, the
  Artifact Signing account — not the app registration's name); fixed at the
  secret level, no workflow change. The pre-existing macOS signed workflow
  also had its first live-secrets run the same day: run 31229465450, green
  end to end (signed, notarized, stapled, verified). This was
  release-infrastructure work outside the active engine-hardening phase,
  done on an explicit owner instruction spanning all their repos.
- **Tag-triggered GitHub Release publishing landed, NEVER EXERCISED**
  (`7505439` 2026-08-07): both signed-release workflows now also trigger on
  a `v*` tag push and attach their verified artifacts to that tag's DRAFT
  release via `softprops/action-gh-release@v2` — macOS the notarized DMG
  (the `.app` is a directory, so it stays a CI artifact only), Windows the
  signed MSI and NSIS installer. Draft is deliberate: this repo has never
  published a GitHub release and has no release-notes convention, so the
  owner reviews and publishes by hand. The two runs share one concurrency
  group so they converge on one release instead of racing to create it.
  None of the tag path is proven: no `v*` tag has ever existed in this repo,
  so the trigger, release creation, two-workflows-one-release convergence,
  and protected-environment approval on a tag run are verifiable only by
  pushing a real tag. `RELEASE.md` §3-§4 is the runbook.

## Rotated 2026-07-28 (catchup hygiene sweep)

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
  (`.agents/review/findings/s4-1..4.md`, all accepted <!-- lint: allow (range notation for s4-1.md through s4-4.md) -->), and the guard
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
- **`write_project` hierarchy guard shipped** 2026-07-16 (`6b4b4ac`):
  `write_project` now validates every `.md` document target against the
  hierarchy before writing anything and fails fast with `InvalidFormat`
  on an omitted document, instead of silently writing an orphan the
  reader would degrade. Entities under `characters/` and `locations/` <!-- lint: allow (user project folder structure paths) -->
  remain hierarchy-exempt by design; the guard matches on normalized
  spellings so variants cannot slip past. This closes the first ranked
  follow-up from the `d6fa9b5` list.
- **Per-commit owner approval removed** by owner directive 2026-07-16
  (`b043585`; decision in `.agents/decisions.md`, operative rule in
  `.agents/repo-guidance.md` Earned Practices): agents commit verified
  single-concern work without announcing and waiting. Push gating
  (`.agents/push-policy.md`), Git Safety append-only history, and scope
  approval are all unchanged.
- **`init_repo` gated behind `test-support`** 2026-07-16 (`7f85510`):
  the one unguarded write path in `core::git` is now `pub(crate)`,
  re-exported only through a `test_support` module behind a new
  `test-support` cargo feature. Dev-dependency re-declarations turn it
  on for test targets only (dev-deps are ignored by release builds, so
  feature unification never reaches them); all 10 out-of-crate call
  sites rerouted; a self-check test asserts `init_repo` stays
  `pub(crate)`. DEVLOG top entry has the detail.
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

## Rotated 2026-07-09 (handoff after format lock)

- Governance refresh landed as of `2ab2579` (2026-07-03): `AGENTS.md`
  reconciled to the current AgentGovernanceBootstrap template (portable,
  generic); repo-specific rules carved into `.agents/repo-guidance.md`;
  harness command wrappers, hooks, and shims brought up to date. See
  `.agents/decisions.md` for why.
- **Build fixed on `master`** (2026-07-09, owner-approved work request):
  `f049198` removed the deleted `linux/` directory from the workspace
  members in root `Cargo.toml` (Cargo.lock regenerated); `d0f9cad` applied
  mechanical `cargo fmt` to three files whose drift the broken workspace had
  hidden. Full verification suite run green locally as of `d0f9cad`. Note:
  the "uncommitted local edit to Cargo.toml" recorded here on 2026-07-03 no
  longer existed by 2026-07-09 (clean worktree, empty stash); the fix was
  re-made from scratch.
- **Format lock (phase Step 2) shipped 2026-07-09** per owner-approved
  `docs/plans/PLAN_FORMAT_LOCK_ENGINE.md` (status: Shipped): unknown
  top-level key preservation (I5), legacy novelist-key lift, canonical
  `fields` order, `format_version` marker, full-fidelity round-trip tests,
  spec aligned. Summary in `DEVLOG.md` (2026-07-09 entry). The audit also
  found phase Step 3 (Tauri Inspector → `doc.fields`) was already in place
  before this work. G2's engine+spec+tests criteria look satisfied —
  closing/advancing the phase is the owner's call (`SET PHASE`).

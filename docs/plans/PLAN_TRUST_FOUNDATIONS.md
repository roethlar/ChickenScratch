# Plan: Trust foundations — write-guard and vault onboarding

**Status:** Draft — pending owner approval. Drafted at owner instruction
2026-07-11 ("plan what we have so far"); reviewed via
`.agents/playbooks/reviewloop.md` (record: `.agents/review/`).

**Owner request (quote):**
> plan what we have so far

Scope = the two items with settled owner intent from the 2026-07-11
discussions: (1) the app must never save over a project it cannot fully
read (data-destruction bug observed on legacy projects); (2) one-button
private-GitHub backup ("vault") plus any-git-remote pairing. Writing modes,
compare views, Draft-mode redesign, and the ChickenCoop server are OUT of
scope (see ADR-006, status Proposed — authorizes no work; coop is direction
only).

**Phase check:** [x] Not paused. Current phase goals G1–G6 are met or
pending owner `SET PHASE`; this plan is the first post-cleanup work request.

**Invariants touched:** I5/I6 (this plan *strengthens* them), I2 (all format
I/O stays in `crates/core`), I7 (git invisible: vault UX uses no git
vocabulary).

---

## Slice 1 — Write-guard: never save over what was not fully read

### [MODEL] Problem (evidence)

Opening an April-era `.chikn` project (documents stored as
`manuscript/*.html`) with the current app loads **zero documents**
(hierarchy names resolve; content does not). Any save path then writes the
empty in-memory state over real files:

- Observed 2026-07-10 on `Crit2.chikn`: working tree showed
  `manuscript/untitled.html` reduced 118 → 1 lines (uncommitted).
- Observed on `Corn 2.chikn`: the close-time auto-save **committed**
  ("Auto-save on close") a gutted `short-story-format.html` (52 → 1 lines)
  and lossy reflows.

Reader entry: `crates/core/src/core/project/reader.rs::read_project` (it
already "repairs in memory" without rewriting on load). Writers:
`crates/core/src/core/project/writer.rs` (`write_project`,
`write_document`, deletion module) and git mutation paths in
`crates/core/src/core/git.rs` (save revision, auto-commit, backup).
Tauri command surface: `src-tauri/src/commands/` (document/io/git).

### [MODEL] Approach

1. **Fidelity classification in the engine.** `read_project` (or a wrapper
   probe) classifies a load as `Full` or `Degraded { reasons }`. Degraded
   triggers, at minimum:
   - any hierarchy document whose `path` does not end in `.md`;
   - any document file that exists with nonzero bytes but loads as empty
     content;
   - any quarantine/repair event during load.
   Expose the classification on the loaded `Project` (e.g.
   `read_fidelity` field) without changing on-disk format (I4/I5 safe:
   in-memory only).
2. **Hard refusal in every mutating path.** `write_project`,
   `write_document`, document deletion, and all git-mutating operations
   (save revision, auto-commit, auto-save-on-close, backup push of new
   commits) return a typed error (`ProjectReadOnly { reasons }`) when the
   project's fidelity is not `Full`. The guard lives in `crates/core`, not
   in UI, so every frontend inherits it (I2).
3. **Tauri surfacing, plain English.** On Degraded open: banner "This
   project was made by an older version and opens read-only — nothing will
   be changed. [Learn what to do]" — editor read-only, save/revision UI
   disabled, close-time auto-save skipped. No dialog storm: one banner.
4. **Docs:** USER_GUIDE short section "Projects from older versions open
   read-only"; RELEASE.md unaffected.

Out of scope for this slice: automatic migration of HTML-era projects
(rebuild-from-`.scriv` remains the workaround; migration is a separate
future plan if the owner wants it).

### [MODEL] Files

| File / area | Change |
|---|---|
| `crates/core/src/core/project/reader.rs` | fidelity classification |
| `crates/core/src/models.rs` (or project struct home) | `read_fidelity` |
| `crates/core/src/core/project/writer.rs` | refuse on Degraded |
| `crates/core/src/core/project/deletion.rs` | refuse on Degraded |
| `crates/core/src/core/git.rs` | refuse mutations on Degraded |
| `src-tauri/src/commands/*` | map error → read-only state; skip auto-saves |
| `ui/` (banner + disabled states) | read-only presentation |
| `docs/USER_GUIDE.md` | read-only explanation |

### [MODEL] Tests (guard proofs)

- New fixture: minimal legacy project (one `.html` document + hierarchy
  path referencing it). Engine tests assert: loads Degraded; every mutating
  API returns `ProjectReadOnly`; on-disk bytes identical before/after the
  attempts (hash the tree).
- Guard-proof discipline: temporarily disable the guard → the
  bytes-unchanged assertion FAILS (writer gutted the file) → restore →
  PASS. This mirrors the real incident.
- Existing suites stay green: modern-format fixtures still load `Full` and
  write normally (no false positives — assert `Full` on the standard
  fixtures, including `samples/Corn.chikn`).

## Slice 2 — Vault: one-button private GitHub backup + any-remote pairing

### [MODEL] Problem

Backups today require the writer to understand git remotes. Owner-approved
direction (2026-07-11): "we help users set up a private github repo…
one button, after an account setup, calls the gh api, makes the repo,
syncs it. then every save or app cycle, data is in your private repo."
Cloud-sync folders (OneDrive/Dropbox) can corrupt a repo's internals and
are NOT a supported vault location.

### [MODEL] Approach

1. **Engine/service layer** (`src-tauri` service, HTTP via existing
   `reqwest`; no format I/O — engine git push machinery already exists):
   - `vault_create_github { token }` → validate token (GET /user; check
     repo-creation scope), POST `/user/repos { name, private: true }`,
     set project remote, initial push, store token in the OS keyring
     (existing keyring machinery from settings).
   - `vault_pair_remote { url, token? }` → same wiring for any git URL
     (self-hosted Gitea/coop, NAS bare repo, `file://` path).
2. **Auto-push policy (passive):** push after each saved revision and on
   app close, only when a vault is configured. Failures are silent-queued
   and retried next cycle; a small status line ("Backed up 4 min ago" /
   "Backup pending — will retry") is the only surface. Never a modal, never
   git vocabulary (I7).
3. **UI:** Settings → Backup gains "Create your private vault on GitHub"
   (guided: link to account creation, then paste a fine-grained token with
   the two required boxes ticked — v1 is guided-PAT; OAuth device flow
   needs a registered app and is deferred) and "Connect your own vault"
   (URL + optional token).
4. **Docs:** USER_GUIDE "Your vault" section, including the explicit
   "don't put projects inside OneDrive/Dropbox folders" warning and why.

### [MODEL] Files

| File / area | Change |
|---|---|
| `src-tauri/src/commands/git.rs` (or new `vault.rs`) | create/pair/status |
| `src-tauri` settings + keyring | vault config + token storage |
| `crates/core/src/core/git.rs` | reuse push; add retry-queue hook if needed |
| `ui/` Settings Backup pane + status line | vault UX |
| `docs/USER_GUIDE.md` | vault + cloud-folder warning |

### [MODEL] Tests

- Integration: `file://` bare-repo vault — configure, save revision,
  assert commit arrives in vault; kill vault path, save, assert silent
  retry state, restore path, assert catch-up push.
- GitHub API calls: unit-test request construction; live path is a manual
  check (owner or agent with a test account) — recorded, not automated.
- Guard proof: disable auto-push hook → vault-behind assertion FAILS →
  restore → PASS.

## Sequencing, verification, close-out

1. Slice 1 lands first (data safety precedes convenience), one concern per
   commit, declared suite (`.agents/repo-guidance.md` Verification) green
   before each commit claim.
2. Slice 2 follows, same discipline.
3. DEVLOG entry per slice (integrity fix + shipped plan). `.agents/state.md`
   updated at close-out. Push only on owner go.

## [MODEL] Owner verification (plain English)

1. Open your old `Corn 2.chikn`: it opens read-only with a plain banner,
   and nothing in the folder changes, ever. Your rebuilt project still
   opens and saves normally.
2. In Settings → Backup, create your private vault (or point at your Gitea
   box), then just write. The status line says when the book last reached
   the vault; turning off Wi-Fi delays it and it catches up by itself.

## [YOU] Decisions needed

- Approval of this plan's scope and order (guard, then vault; modes/UI
  redesign excluded pending their own decision).
- Slice 2 v1 uses guided token setup (paste one token, guided screenshots)
  rather than full OAuth — acceptable for v1? (OAuth needs a registered
  GitHub app; can follow later without rework.)

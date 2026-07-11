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

(Revised after codex review `plan-1`; the five findings are folded in.)

1. **Side-effect-free preflight, before any load.** `read_project` itself
   mutates disk during load — it creates missing standard folders
   (`reader.rs:250-268,339-379`) and renames corrupt sidecars
   (`reader.rs:936-960,991-1009`) — so classification cannot run after it.
   Add `probe_project_fidelity(root) -> Fidelity` in `crates/core`: a pure
   read-only scan (no folder creation, no renames, no writes of any kind)
   that classifies **before** anything else touches the project.
   `Fidelity::Degraded { reasons }` when any of:
   - any hierarchy document `path` not ending in `.md`;
   - any hierarchy document that does not resolve to a readable, non-empty
     parse (a file with nonzero bytes that yields empty content, an
     unparsable file, or a missing file — a hierarchy entry that "never
     loads" is Degraded, not a warning; cf. `reader.rs:824-846,394-406`);
   - any condition that would trigger a load-time repair or quarantine
     (missing standard folder, corrupt sidecar, orphan adoption);
   - `project.yaml` `format_version` absent-and-legacy-shaped, or **newer
     than this engine writes** (reader accepts anything at
     `reader.rs:41-47` while the writer stamps the current version at
     `writer.rs:244-259` — an unguarded silent-downgrade path; newer or
     unsupported versions are Degraded).
   `Fidelity::Full` requires: every hierarchy document resolves to loaded
   content, no repair conditions, and a supported `format_version`.
2. **Non-forgeable write capability.** Fidelity carried as a field on
   `Project` cannot guard path-only mutators (`writer.rs::delete_document`
   at `writer.rs:739-784`, folder deletion in `deletion.rs`, and the git
   restore / draft / backup / sync mutators in `core/git.rs`). Introduce a
   `WriteToken` (non-`Clone`, non-constructible outside the engine):
   issued only by (a) a `Full` preflight or (b) the project-creation path
   (a project the engine itself just initialized is `Full` by
   construction). Every mutating engine API — `write_project`,
   `write_document`, both deletion paths, and every git-mutating function —
   takes `&WriteToken`; without one the call cannot be expressed. A typed
   `ProjectReadOnly { reasons }` error covers the runtime refusal where a
   token is requested for a Degraded project.
3. **Degraded open path.** For Degraded projects the app loads via a
   repairs-disabled read (pure read; load-time self-heal is suppressed so
   an open leaves the folder byte-identical). Repairs remain allowed on
   `Full` projects (normal self-heal is unchanged).
4. **Tauri surfacing, plain English.** On Degraded open: one banner —
   "This project was made by an older version and opens read-only —
   nothing will be changed. [Learn what to do]" — editor read-only,
   save/revision UI disabled, close-time auto-save skipped. No dialog
   storm.
5. **Docs:** USER_GUIDE short section "Projects from older versions open
   read-only"; RELEASE.md unaffected.

Out of scope for this slice: automatic migration of HTML-era projects
(rebuild-from-`.scriv` remains the workaround; migration is a separate
future plan if the owner wants it).

### [MODEL] Files

| File / area | Change |
|---|---|
| `crates/core/src/core/project/` (new `fidelity.rs` or in `reader.rs`) | side-effect-free `probe_project_fidelity`; `WriteToken` |
| `crates/core/src/core/project/reader.rs` | repairs-disabled read path for Degraded opens |
| `crates/core/src/core/project/writer.rs` | `write_project`, `write_document`, `delete_document` (`:739-784`) take `&WriteToken` |
| `crates/core/src/core/project/deletion.rs` | folder deletion takes `&WriteToken` |
| `crates/core/src/core/git.rs` | every mutating fn (save revision, auto-commit, restore, drafts, backup, sync) takes `&WriteToken` |
| `src-tauri/src/commands/*` | token plumbed via project state; Degraded → read-only state; auto-saves skipped |
| `ui/` (banner + disabled states) | read-only presentation |
| `docs/USER_GUIDE.md` | read-only explanation |

### [MODEL] Tests (guard proofs)

Fixtures (each a minimal on-disk project):
(a) legacy `.html` documents; (b) hierarchy entry referencing a missing
file; (c) corrupt document sidecar; (d) missing standard folder;
(e) `format_version` newer than the engine's.

- Each fixture: `probe_project_fidelity` returns Degraded with the right
  reason; **tree hash identical before vs after the probe AND before vs
  after a Degraded open** (this is what catches load-time folder creation
  and sidecar renames — the probe and the Degraded read must both be
  side-effect-free); every mutating API is uncallable/refused
  (`ProjectReadOnly`), tree hash still identical after the attempts.
- Guard-proof discipline: disable the guard (issue a token uncondition-
  ally) → the bytes-identical assertions FAIL (writer guts fixture (a),
  repairs dirty (b)–(d), version downgrade dirties (e)) → restore → PASS.
  Mirrors the real incident.
- No false positives: modern fixtures, including `samples/Corn.chikn`,
  probe `Full`, open normally, write normally, and load-time self-heal
  still works for `Full` projects (existing repair tests stay green).

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

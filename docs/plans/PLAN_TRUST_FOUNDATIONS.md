# Plan: Trust foundations — write-guard and vault onboarding

**Status:** Slice 1 (write-guard) approved 2026-07-11 and in progress.
Slice 2 (vault) is NOT approved — owner: "we're nowhere near deciding how
remotes will work"; it remains a draft direction only, and no vault work
may start without a fresh owner decision on remote design. Reviewed by
codex, accepted round 3 (`.agents/review/findings/plan-1.md`).

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
   - any hierarchy document that is **unresolved**: file missing, file
     unparsable, or file with nonzero bytes whose parse yields empty
     content. **Zero-byte document files are valid** — the app creates
     them deliberately and `samples/Corn.chikn` contains two (a hierarchy
     entry that "never loads" is Degraded, not a warning; cf.
     `reader.rs:824-846,394-406`);
   - any **content-threatening** repair/quarantine condition: corrupt
     sidecar (load renames it), orphan adoption, duplicate identity.
     **Missing standard folders are NOT Degraded** — recreating an empty
     `research/` or `Trash/` touches no content and stays normal
     self-heal (`samples/Corn.chikn` itself lacks research/templates/
     settings and must probe Full);
   - `project.yaml` `format_version` absent-and-legacy-shaped, or **newer
     than this engine writes** (reader accepts anything at
     `reader.rs:41-47` while the writer stamps the current version at
     `writer.rs:244-259` — an unguarded silent-downgrade path; newer or
     unsupported versions are Degraded).
   `Fidelity::Full` requires: every hierarchy document resolves (zero-byte
   counts as resolved), no content-threatening repair condition, and a
   supported `format_version`.
2. **Non-forgeable, root-bound, epoch-stamped write capability.**
   Fidelity carried as a field on `Project` cannot guard path-only
   mutators (`writer.rs::delete_document` at `writer.rs:739-784`, folder
   deletion in `deletion.rs`, and the git restore / draft / backup / sync
   mutators in `core/git.rs`). Introduce a `WriteToken` (non-`Clone`,
   non-constructible outside the engine) with two bindings:
   - **Root binding:** the token stores the canonical (symlink-resolved)
     project root it was issued for; every mutating API validates its
     target path lies under the token's root. A token for project A can
     never authorize a write into project B.
   - **Epoch binding:** the engine keeps a per-project write epoch; any
     operation that replaces working-tree content (revision restore,
     draft switch, sync pull/merge) bumps the epoch, re-probes fidelity,
     and reissues the token only if still `Full`. A token from before the
     bump is stale and refused — so a pull that drops legacy or
     newer-format content into the tree cannot be followed by writes on a
     borrowed pre-pull token.
   Issued only by (a) a `Full` preflight or (b) the project-creation path
   (a project the engine itself just initialized is `Full` by
   construction). Every mutating engine API — `write_project`,
   `write_document`, both deletion paths, every git-mutating function,
   and **every project-directory-creating helper** (the public
   `safe_path` creators move behind the token or become engine-private) —
   takes `&WriteToken`; without one the call cannot be expressed. A typed
   `ProjectReadOnly { reasons }` error covers the runtime refusal.
   **App-side writes into the project obey the same gate:** the
   Statistics panel's `settings/writing-history.json` write
   (`src-tauri/src/commands/io.rs:436-470`) routes through a token-gated
   engine API — opening Statistics on a Degraded project writes nothing.
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

Degraded fixtures (each a minimal on-disk project):
(a) legacy `.html` documents; (b) hierarchy entry referencing a missing
file; (c) corrupt document sidecar; (e) `format_version` newer than the
engine's.

- Each Degraded fixture: `probe_project_fidelity` returns Degraded with
  the right reason; **tree hash identical before vs after the probe AND
  before vs after a Degraded open** (catches load-time sidecar renames —
  probe and Degraded read must both be side-effect-free); every mutating
  API is uncallable/refused (`ProjectReadOnly`); tree hash still identical
  after the attempts **including opening the Statistics panel** (the
  writing-history path).
- Token-binding tests: a token issued for Full project A is refused
  against project B's paths (cross-project rejection); after a simulated
  tree-replacing operation (restore/pull) the pre-bump token is refused
  (stale-token test).
- Valid-emptiness tests: a zero-byte document file probes `Full` and
  round-trips untouched; `samples/Corn.chikn` (which lacks
  research/templates/settings folders and contains two zero-byte
  documents) probes `Full`, opens normally, self-heals its missing
  folders, and writes normally.
- Guard-proof discipline: disable the guard (issue a token
  unconditionally) → the bytes-identical assertions FAIL (writer guts
  fixture (a), quarantine dirties (c), version downgrade dirties (e)) →
  restore → PASS. Mirrors the real incident.
- Existing repair tests for `Full` projects stay green (self-heal
  unchanged where fidelity is Full).

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

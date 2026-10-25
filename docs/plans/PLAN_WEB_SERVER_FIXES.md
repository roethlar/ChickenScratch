# Plan: Web server on a headless box with a NAS — three fixes

**Owner request (quote):**
> Failed to save settings: Keyring error: Platform secure storage failure:
> DBus error: The name is not activatable

> the Backup Directory for the web version cannot be configured unless it's
> pre-existing within the root folder. the root folder isn't the right place
> for backups.

> what's up with the git backend? (after `git status` in a new project on
> the NAS said "detected dubious ownership" and "No commits yet")

(2026-10-24/25, first real install on a second machine, projects on an NFS
share; then "plan".)

**Phase check:** [x] Allowed — bugs and gaps surfaced by the web-shell side
quest; hardening-phase in spirit  [x] Not paused

**Invariants touched:** I2 (slice 3 touches the engine's git layer, inside
`crates/core`); I6 (secrets must not be lost or leaked — the fallback store
is owner-only and documented). No format change.

---

## [MODEL] Intent

The web version works on a plain Linux server with no desktop session and
with projects on a NAS: settings save, backups go to a folder the owner
chooses anywhere, and revision history works even when the NAS mount maps
file ownership to another user.

## [MODEL] Approach

### Slice 1 — settings save without an OS password store (blocker)

`chickenscratch-services::settings` talks to the OS keyring (`keyring` crate,
Secret Service over D-Bus on Linux) on every load *and* save, even when no
secret is involved; loading already swallows the error, saving propagates it,
so nothing in Settings can be saved on a headless box.

Fix inside `settings.rs`, behind the existing `SecretStore` trait:

- A `FileSecretStore`: `~/.config/chickenscratch/secrets.json`, created
  `0600`, one JSON object keyed by service name. Plain file, owner-only, same
  posture as the token the web shell used to keep.
- A `FallbackSecretStore { os: OsKeyring, file: FileSecretStore }`: every
  call tries the OS keyring first; a *platform* failure (anything other than
  `NoEntry`/`Ambiguous`) falls through to the file store. `delete` clears
  both. The result is remembered for the process so the D-Bus probe happens
  once per run, not on every save.
- `get_app_settings` / `save_app_settings` use the fallback store. A one-line
  `tracing::warn!` the first time the fallback is used, naming the file.
- Tests: a fake store that fails like the headless box proves a save with no
  secrets succeeds, a save with an API key lands in the file store with
  `0600`, and a later load reads it back.

### Slice 2 — backup folder outside the projects root

Today the browser's folder picker lists only sub-folders of the projects
root (`fs_browse.rs`), so the backup folder can only be chosen inside it.
(Typing a path works but the save failed for the slice-1 reason.)

- `web.toml` gains `backup = "/path"` (optional). `--backup` flag and
  `CHIKN_WEB_BACKUP` follow the same precedence as `root`/`bind`.
- `GET /api/info` reports `roots: [{ "label": "Projects", "path": … },
  { "label": "Backups", "path": … }]`; `GET /api/fs/dirs` accepts any path
  inside any listed root (same canonicalize-through-symlinks rule, same
  refusal otherwise). The picker (`ServerFolderPicker`) shows a root switch
  when there is more than one and opens the Backups root when invoked for
  the backup setting.
- Installer: one more question, "Folder for backups (optional, Enter for
  none)", written to `web.toml`; `--backup` flag; `--reconfigure` pre-fills
  it; the systemd mount check covers this folder too if it is on a share.
- Backup commands (`push_backup`, `manual_backup`, `backup_on_close`) keep
  taking the folder from settings exactly as the desktop does; `gate.rs`'s
  "settings paths are deliberately not gated" note stays true.
- User guide: backups section for the web version.

### Slice 3 — history on a NAS that maps ownership

"No commits yet" is by design: `create_project` runs `git init` only
(`writer.rs:234`); the first entry lands on Save Revision / autosave.

"Dubious ownership" is the real finding: libgit2 1.8 (via `git2 0.19`)
performs the same owner check as the git CLI, so on a mount whose files
belong to another uid every `Repository::open` in the engine fails and the
web version cannot save a revision there.

- First, a verification step the owner does, no code: on the server,
  `ls -ln /mnt/notes` and `id -u` should agree. If the NFS export squashes
  uids (`all_squash`, `anonuid`) or the mount maps them, fixing the export
  so the files are the owner's is the clean answer and makes the CLI happy
  too.
- Code, if wanted: `chickenscratch-core` exposes
  `git::trust_repository_ownership()` (wraps
  `git2::opts::set_verify_owner_validation(false)`, process-wide, called
  once). `chikn-web` calls it at startup unless `web.toml` says
  `verify_repo_owner = true`, and logs one line saying so. The desktop app
  keeps libgit2's default. Rationale: the server's repositories all live
  inside a root the gate confines and the owner chose; the check guards
  against hostile repos in shared folders, which is not this deployment.
- A core test proves the wrapper flips the option and that a repo in a
  temp dir still opens (the positive path; the negative path needs a
  second uid and is documented as manual).

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `crates/services/src/settings.rs` | file secret store + fallback; tests |
| `crates/web/src/{config.rs,main.rs,state.rs,projects.rs,fs_browse.rs}`, `crates/web/tests/http.rs` | `backup` setting, `roots` in `/api/info`, multi-root browsing |
| `ui/src/components/shared/ServerFolderPicker.tsx`, `ui/src/commands/web.ts`, `Settings.tsx`, tests | root switch; backup picker opens Backups |
| `scripts/install-web.sh` | backup question / `--backup`; mount check |
| `crates/core/src/core/git.rs` (+ `lib.rs` re-export) | `trust_repository_ownership()` |
| `crates/web/src/{config.rs,main.rs}` | `verify_repo_owner` key, startup call |
| `docs/USER_GUIDE.md`, `docs/ARCHITECTURE.md`, `.agents/state.md`, plan status | records |

## [MODEL] Tests

- [x] `cargo test -p chickenscratch-services` — fallback store cases (5 new)
- [ ] `cargo test -p chickenscratch-web` — `backup` precedence, `/api/info`
      roots, `/api/fs/dirs` inside the backup root and refused outside both
- [ ] UI tests — picker switches roots; Settings uses the Backups root
- [ ] Installer dry run with `--backup` into a throwaway `$HOME`
- [ ] `cargo test -p chickenscratch-core` — ownership wrapper
- [ ] Declared suite; CI on push

## [MODEL] Owner verification (plain English)

1. On the server, open Settings, change anything, Save: no keyring error.
2. Settings → Backup folder → the picker offers "Backups"; pick a folder
   there; Manual Backup writes into it.
3. In a project on the NAS, Save Revision: History lists it.

## [YOU] Decisions needed

- Slice 3 code: should `chikn-web` skip libgit2's ownership check by default
  (one log line, opt back in via `verify_repo_owner = true`), or only when
  the setting is turned on? Recommendation: skip by default for the server;
  the desktop app is unchanged either way.

## Status

- 2026-10-25: drafted on the owner's "plan". No slice approved; slice 1
  (settings save) proposed first.
- 2026-10-25: owner "finish all slices" → slice 1 landed: `FallbackSecretStore`
  (OS keyring, else owner-only `secrets.json`; one probe per process, one
  warning line). Slice 3 default follows the recommendation (skip the check
  in `chikn-web`, `verify_repo_owner = true` opts back in) since the go
  covered all slices without a different answer.

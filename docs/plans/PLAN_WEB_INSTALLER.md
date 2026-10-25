# Plan: Web installer — config file, service, repeatable upgrades

**Owner request (quote):**
> okay this needs an installer for the web version. the default data dir
> should be set at install time, and here it will be on my NAS

> yeah, a .config/ entry or similar. and a way to upgrade without
> re-specifying everything

(2026-10-24, after the web shell and project profiles shipped and CI went
green at `150bfcf`.)

**Phase check:** [x] Allowed by `CURRENT_PHASE.md` — queued side quest like
`PLAN_WEB_SHELL.md`, no phase change  [x] Not paused

**Invariants touched:** none of the format or engine rules. I2 untouched (no
`.chikn` I/O outside core). I3 untouched (Tauri stays the reference GUI). The
web crate gains a config loader; everything else is a shell script and docs.

---

## [MODEL] Intent

The owner installs the web version on the Linux server once with one
command, answers where projects live (a folder on the NAS), and from then on
it runs in the background, survives reboots, and waits for the NAS mount. A
later upgrade is one command again and asks nothing: the answers live in a
config file under `~/.config/chickenscratch/` and the installer reuses them.

## [MODEL] Approach

### Config file (slice 1 — `chikn-web` reads `web.toml`)

`chikn-web` learns one optional file, `~/.config/chickenscratch/web.toml`
(`$XDG_CONFIG_HOME` honoured, next to the existing `web-token`):

```toml
root = "/mnt/stuff/Notes"   # folder that holds the .chikn projects
bind = "0.0.0.0:7391"       # address:port to listen on
# ui_dir = "/path/to/ui/dist"   # only for development builds
```

Precedence, highest first: command-line flag → environment
(`CHIKN_WEB_ROOT`, `CHIKN_WEB_BIND`) → config file → built-in default
(`127.0.0.1:7391`; `root` has no default and stays required). `--root`
becomes optional on the command line; starting with no root anywhere fails
with one plain sentence that names the config file. `chikn-web --config
<path>` points at another file; `chikn-web --show-config` prints what would
be used and where each value came from, for troubleshooting. Loader in a new
`crates/web/src/config.rs`, unit-tested (merge order, missing file is fine,
malformed file is an error that names the line).

### Installer (slice 2 — `scripts/install-web.sh`)

One Bash script, run as the owner's own user (not root) on the server, so
files written to the NAS carry the owner's uid and the config lands in the
owner's `~/.config`. Behaviour:

1. **Questions, with defaults, only when no config exists**: projects
   folder (default `~/Notes`), listen address (default `0.0.0.0:7391` —
   the installer is for a server others reach), start at boot (default yes).
   Every answer can also be given as a flag (`--root`, `--bind`,
   `--no-boot`), and `--yes` accepts defaults for scripted runs.
2. **Rerun = upgrade.** When `web.toml` already exists the script asks
   nothing, keeps the file untouched, replaces the binary and the service
   unit, restarts the service, and prints old → new version. `--reconfigure`
   asks the questions again, pre-filled with current values.
3. **Where the binary comes from**: inside a checkout it builds
   `chikn-web` with `--features embed-ui` (running `npm ci && npm run build`
   first); `--binary <file>` installs a prebuilt one instead (for a future
   release tarball — see "Later"). Installed to `~/.local/bin/chikn-web`;
   the previous binary is kept as `chikn-web.previous` so a bad upgrade can
   be rolled back with `--rollback`.
4. **Service**: a user-level systemd unit
   `~/.config/systemd/user/chikn-web.service` — `ExecStart=chikn-web` (all
   settings from `web.toml`), `Restart=on-failure`, `RestartSec=15`, no
   start-rate limit. If the projects folder sits on a network mount
   (detected with `findmnt --target`, type nfs/nfs4/cifs), the unit gets
   `ExecStartPre=mountpoint -q <mount>` so it simply retries until the NAS
   is there instead of starting against an empty folder. "Start at boot"
   runs `loginctl enable-linger $USER`, which needs sudo once; the script
   says so and prints the exact command if it cannot.
5. **Token**: the server already creates `~/.config/chickenscratch/web-token`
   on first run; the installer waits for the service to come up, reads the
   file, and prints the one link to open (address with `?token=`), plus the
   three commands that matter: status, logs, stop.
6. `scripts/uninstall-web.sh`: stops and removes the service and binary;
   keeps `web.toml`, the token and every project unless `--purge-config`
   is given. Never touches the projects folder.
7. Both scripts pass `bash -n` and `shellcheck` (added to the Validation
   workflow's Linux job when `shellcheck` is available there; it is on
   `ubuntu-latest`).

### Docs (slice 3)

`docs/USER_GUIDE.md` "Web version" rewritten around the installer
(install, upgrade, where the settings live, NAS note, uninstall);
`README.md` one line; `RELEASE.md` adds the installer check to the runbook;
`TODO.md`, `ROADMAP.md`, `DEVLOG.md`, `.agents/state.md`.

### Later, not in this plan

- A CI job that builds `chikn-web-<version>-linux-x86_64.tar.gz` (binary +
  both scripts) and attaches it to GitHub releases, so an upgrade is
  "download, run `install-web.sh`" with no Rust or Node on the server.
- Installers for macOS/Windows servers; a container image.

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `crates/web/src/config.rs` (new), `main.rs` | `web.toml` loader, precedence, `--config`, `--show-config`; `--root` optional |
| `crates/web/Cargo.toml` | `toml` dependency |
| `crates/web/tests/http.rs` or `config` unit tests | merge order, missing/malformed file |
| `scripts/install-web.sh`, `scripts/uninstall-web.sh` (new) | installer / uninstaller |
| `.github/workflows/validation.yml` | `shellcheck scripts/*.sh` on Linux |
| `docs/USER_GUIDE.md`, `README.md`, `RELEASE.md`, `TODO.md`, `docs/ROADMAP.md`, `DEVLOG.md`, `.agents/state.md`, `docs/plans/README.md` | docs and records |

## [MODEL] Tests

- [ ] `cargo test --locked -p chickenscratch-web` — config precedence, no
      file, malformed file, `--show-config` output
- [ ] `bash -n` and `shellcheck` on both scripts
- [ ] Installer dry run on the Linux box into a throwaway `$HOME`
      (`HOME=/tmp/… ./scripts/install-web.sh --root … --yes`), then rerun to
      prove the upgrade path asks nothing; `--rollback`; uninstall
- [ ] Declared suite (`.agents/repo-guidance.md` Verification)

## [MODEL] Owner verification (plain English)

"On the server, run the installer once; type the NAS folder when asked.
Open the printed link from your laptop over the VPN and create a project.
Reboot the server: the site comes back by itself. Later, pull the new code
and run the installer again: it asks nothing and the site is on the new
version with the same projects."

## [YOU] Decisions needed

- None to start slice 1. One assumption to confirm when slice 2 lands: the
  service runs under your own account (so NAS files are yours), which means
  one `sudo loginctl enable-linger` for start-at-boot.

## Status

- 2026-10-24: planned and queued on the owner's "yeah" to the words-first
  proposal. No slice approved yet; propose slice 1 (config file) first.

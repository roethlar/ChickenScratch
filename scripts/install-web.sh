#!/usr/bin/env bash
# Install or upgrade the ChickenScratch web version (chikn-web) on a Linux
# server, for one user, as a background service.
#
#   scripts/install-web.sh                 # first time: asks three questions
#   scripts/install-web.sh                 # again later: upgrade, asks nothing
#   scripts/install-web.sh --reconfigure   # ask the questions again
#   scripts/install-web.sh --rollback      # put the previous version back
#
# Settings live in ~/.config/chickenscratch/web.toml and are never rewritten
# by an upgrade. The service runs under your own account, so project files on
# a NAS are owned by you. Plan: docs/plans/PLAN_WEB_INSTALLER.md.
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/install-web.sh [options]

  --root <folder>     Folder that will hold your projects (asked if missing)
  --bind <addr:port>  Address to listen on (default 0.0.0.0:7391)
  --boot yes|no       Start at boot (default yes; needs one sudo command)
  --yes               Accept defaults for anything not given; never ask
  --reconfigure       Ask the questions again, pre-filled with current values
  --binary <file>     Install this prebuilt chikn-web instead of building
  --no-systemd        Install binary and settings only; no background service
  --print-unit        Show the service file that would be written, then exit
  --rollback          Reinstall the previous binary and restart the service
  --help              This text

Reinstalling with settings already present upgrades the program and keeps
every setting. Environment: CHIKN_BIN_DIR (default ~/.local/bin).
EOF
}

say()  { printf '%s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
die()  { printf 'error: %s\n' "$*" >&2; exit 1; }

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd "$script_dir/.." && pwd)

config_home=${XDG_CONFIG_HOME:-$HOME/.config}
config_dir=$config_home/chickenscratch
config_file=$config_dir/web.toml
token_file=$config_dir/web-token
bin_dir=${CHIKN_BIN_DIR:-$HOME/.local/bin}
bin_path=$bin_dir/chikn-web
unit_dir=$config_home/systemd/user
unit_name=chikn-web.service
unit_path=$unit_dir/$unit_name

opt_root=""
opt_bind=""
opt_boot=""
opt_yes=0
opt_reconfigure=0
opt_binary=""
opt_systemd=1
opt_print_unit=0
opt_rollback=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --root) [[ $# -ge 2 ]] || die "--root needs a folder"; opt_root=$2; shift 2 ;;
    --bind) [[ $# -ge 2 ]] || die "--bind needs addr:port"; opt_bind=$2; shift 2 ;;
    --boot) [[ $# -ge 2 ]] || die "--boot needs yes or no"; opt_boot=$2; shift 2 ;;
    --no-boot) opt_boot=no; shift ;;
    --yes|-y) opt_yes=1; shift ;;
    --reconfigure) opt_reconfigure=1; shift ;;
    --binary) [[ $# -ge 2 ]] || die "--binary needs a file"; opt_binary=$2; shift 2 ;;
    --no-systemd) opt_systemd=0; shift ;;
    --print-unit) opt_print_unit=1; shift ;;
    --rollback) opt_rollback=1; shift ;;
    --help|-h) usage; exit 0 ;;
    *) usage >&2; die "unknown option: $1" ;;
  esac
done

if [[ ${EUID:-$(id -u)} -eq 0 && -z ${CHIKN_ALLOW_ROOT:-} ]]; then
  die "run this as the user who will own the projects, not as root (set CHIKN_ALLOW_ROOT=1 to override)"
fi
case "$opt_boot" in ""|yes|no) ;; *) die "--boot takes yes or no" ;; esac

# ── helpers ────────────────────────────────────────────────────────────────

# Read a key from web.toml: the first `key = "value"` line, unquoted.
toml_get() {
  local key=$1
  [[ -f $config_file ]] || return 0
  sed -n -E "s/^[[:space:]]*${key}[[:space:]]*=[[:space:]]*\"([^\"]*)\".*/\1/p" "$config_file" | head -n 1
}

expand_home() {
  # The patterns match a literal tilde typed by the user; no expansion wanted.
  # shellcheck disable=SC2088
  case "$1" in
    "~") printf '%s\n' "$HOME" ;;
    "~/"*) printf '%s/%s\n' "$HOME" "${1#\~/}" ;;
    *) printf '%s\n' "$1" ;;
  esac
}

ask() {
  # ask <prompt> <default> -> prints the answer
  local prompt=$1 default=$2 answer
  if [[ $opt_yes -eq 1 ]]; then
    printf '%s\n' "$default"
    return
  fi
  [[ -t 0 ]] || die "not running in a terminal; pass --yes or give --root/--bind/--boot"
  read -r -p "$prompt [$default]: " answer </dev/tty
  printf '%s\n' "${answer:-$default}"
}

have_systemd_user() {
  [[ $opt_systemd -eq 1 ]] || return 1
  command -v systemctl >/dev/null 2>&1 || return 1
  systemctl --user show-environment >/dev/null 2>&1
}

service_active() {
  have_systemd_user && systemctl --user is-active --quiet "$unit_name"
}

installed_version() {
  [[ -x $bin_path ]] && "$bin_path" --version 2>/dev/null | awk '{print $NF}' || true
}

# The mount that holds the projects folder, if it is a network share.
# findmnt may list several filesystems on one target (an autofs automount
# plus the NFS share it brought in); any network type among them counts.
network_mount_for() {
  local probe=$1 line fstype mountpoint_
  while [[ ! -e $probe && $probe != / ]]; do probe=$(dirname "$probe"); done
  command -v findmnt >/dev/null 2>&1 || return 0
  while IFS= read -r line; do
    mountpoint_=${line%% *}
    fstype=${line##* }
    case "$fstype" in
      nfs|nfs4|cifs|smb3|smbfs|fuse.sshfs)
        printf '%s\n' "$mountpoint_"
        return 0
        ;;
    esac
  done < <(findmnt -T "$probe" -n -o TARGET,FSTYPE 2>/dev/null || true)
}

render_unit() {
  local root=$1 mount pre="" mp_bin
  mount=$(network_mount_for "$root")
  if [[ -n $mount ]]; then
    mp_bin=$(command -v mountpoint || true)
    [[ -n $mp_bin ]] && pre="ExecStartPre=$mp_bin -q $mount"$'\n'
  fi
  cat <<EOF
# Written by scripts/install-web.sh — rerun the installer rather than editing.
[Unit]
Description=ChickenScratch web (chikn-web)
Documentation=https://github.com/roethlar/ChickenScratch
StartLimitIntervalSec=0

[Service]
Type=simple
${pre}ExecStart=$bin_path
Restart=on-failure
RestartSec=15
Environment=RUST_LOG=info

[Install]
WantedBy=default.target
EOF
}

write_config() {
  local root=$1 bind=$2
  mkdir -p "$config_dir"
  cat >"$config_file" <<EOF
# ChickenScratch web settings — read by chikn-web at start.
# Rerun scripts/install-web.sh --reconfigure to change these with prompts,
# or edit and then: systemctl --user restart chikn-web
root = "$root"   # folder that holds your .chikn projects
bind = "$bind"   # address:port to listen on; 127.0.0.1:7391 = this machine only
EOF
  chmod 600 "$config_file"
}

print_how_to_reach() {
  local bind=$1 host token url
  host=${bind%:*}
  if [[ $host == 0.0.0.0 || $host == "[::]" || $host == "::" ]]; then
    host=$(hostname -I 2>/dev/null | awk '{print $1}')
    [[ -n $host ]] || host=$(hostname)
  fi
  url="http://$host:${bind##*:}/"
  if [[ -r $token_file ]]; then
    token=$(tr -d '[:space:]' <"$token_file")
    say "Open this link once; the browser remembers the token afterwards:"
    say "  ${url}?token=$token"
  else
    say "Open: $url  (the access token is printed in the service log on first start)"
  fi
}

# ── rollback ───────────────────────────────────────────────────────────────

if [[ $opt_rollback -eq 1 ]]; then
  [[ -x $bin_path.previous ]] || die "no previous version kept at $bin_path.previous"
  service_active && systemctl --user stop "$unit_name"
  mv -f "$bin_path" "$bin_path.rolled-back"
  mv -f "$bin_path.previous" "$bin_path"
  mv -f "$bin_path.rolled-back" "$bin_path.previous"
  say "Rolled back to chikn-web $(installed_version)."
  if have_systemd_user && [[ -f $unit_path ]]; then
    systemctl --user start "$unit_name"
    say "Service restarted."
  fi
  exit 0
fi

# ── settings ───────────────────────────────────────────────────────────────

existing_root=$(toml_get root)
existing_bind=$(toml_get bind)
configure=1
if [[ -n $existing_root && $opt_reconfigure -eq 0 && -z $opt_root && -z $opt_bind ]]; then
  configure=0
fi

if [[ $configure -eq 1 ]]; then
  default_root=${existing_root:-$HOME/Notes}
  default_bind=${existing_bind:-0.0.0.0:7391}
  root=${opt_root:-$(ask "Folder that will hold your projects (a NAS path is fine)" "$default_root")}
  bind=${opt_bind:-$(ask "Address to listen on (0.0.0.0 = whole network/VPN)" "$default_bind")}
  root=$(expand_home "$root")
  [[ $root == /* ]] || root=$PWD/$root
  [[ $bind == *:* ]] || die "listen address must look like 0.0.0.0:7391 (got '$bind')"
else
  root=$existing_root
  bind=$existing_bind
  say "Settings found in $config_file — upgrading, keeping them."
fi

boot=$opt_boot
if [[ -z $boot ]]; then
  if [[ $configure -eq 1 ]]; then
    boot=$(ask "Start at boot? (yes/no)" "yes")
  elif have_systemd_user && systemctl --user is-enabled --quiet "$unit_name" 2>/dev/null; then
    boot=yes
  else
    boot=no
  fi
fi
case "$boot" in y|Y|yes|YES|Yes) boot=yes ;; *) boot=no ;; esac

if [[ $opt_print_unit -eq 1 ]]; then
  render_unit "$root"
  exit 0
fi

# ── the program ────────────────────────────────────────────────────────────

if [[ -n $opt_binary ]]; then
  [[ -x $opt_binary ]] || die "$opt_binary is not an executable file"
  new_binary=$opt_binary
elif [[ -f $repo_root/crates/web/Cargo.toml ]]; then
  command -v cargo >/dev/null 2>&1 || die "cargo not found; install Rust or pass --binary"
  command -v npm >/dev/null 2>&1 || die "npm not found; install Node or pass --binary"
  say "Building the web UI…"
  (cd "$repo_root/ui" && npm ci --no-audit --no-fund --silent && npm run build --silent)
  say "Building chikn-web (release, UI embedded)…"
  (cd "$repo_root" && cargo build --release -p chickenscratch-web --features embed-ui --quiet)
  new_binary=$repo_root/target/release/chikn-web
else
  die "not inside the ChickenScratch repository and no --binary given"
fi

old_version=$(installed_version)
new_version=$("$new_binary" --version 2>/dev/null | awk '{print $NF}')

mkdir -p "$bin_dir"
if [[ -d $root ]]; then
  [[ -w $root ]] || die "cannot write to $root"
else
  mkdir -p "$root" || die "cannot create $root"
fi

was_active=0
if service_active; then
  was_active=1
  systemctl --user stop "$unit_name"
fi
if [[ -x $bin_path ]]; then
  cp -f "$bin_path" "$bin_path.previous"
fi
install -m 755 "$new_binary" "$bin_path"

if [[ $configure -eq 1 ]]; then
  write_config "$root" "$bind"
  say "Settings written to $config_file"
fi

# ── the service ────────────────────────────────────────────────────────────

if have_systemd_user; then
  mkdir -p "$unit_dir"
  render_unit "$root" >"$unit_path"
  systemctl --user daemon-reload
  if [[ $boot == yes ]]; then
    systemctl --user enable --quiet "$unit_name"
    linger=$(loginctl show-user "$USER" -p Linger --value 2>/dev/null || echo no)
    if [[ $linger != yes ]]; then
      if [[ -t 0 && $opt_yes -eq 0 ]] && command -v sudo >/dev/null 2>&1; then
        say "Starting at boot needs one admin command (asks for your password):"
        sudo loginctl enable-linger "$USER" || warn "could not enable; run later: sudo loginctl enable-linger $USER"
      else
        warn "to start at boot, run once: sudo loginctl enable-linger $USER"
      fi
    fi
  else
    systemctl --user disable --quiet "$unit_name" 2>/dev/null || true
  fi
  systemctl --user start "$unit_name"
  for _ in $(seq 1 30); do
    systemctl --user is-active --quiet "$unit_name" && [[ -r $token_file ]] && break
    sleep 0.5
  done
  if systemctl --user is-active --quiet "$unit_name"; then
    say ""
    if [[ -n $old_version && $old_version != "$new_version" ]]; then
      say "chikn-web upgraded: $old_version → $new_version, running."
    else
      say "chikn-web $new_version is running."
    fi
    say "Projects folder: $root"
    print_how_to_reach "$bind"
    say ""
    say "Useful commands:"
    say "  systemctl --user status chikn-web      # is it running?"
    say "  journalctl --user -u chikn-web -f      # live log"
    say "  systemctl --user restart chikn-web     # after editing $config_file"
    say "  scripts/install-web.sh --rollback      # back to $old_version"
  else
    warn "the service did not stay up. Last log lines:"
    journalctl --user -u "$unit_name" -n 20 --no-pager >&2 || true
    if [[ $was_active -eq 1 && -x $bin_path.previous ]]; then
      warn "previous version is kept; restore it with: scripts/install-web.sh --rollback"
    fi
    exit 1
  fi
else
  if [[ $opt_systemd -eq 1 ]]; then
    warn "no user systemd available; installed without a background service."
  fi
  say ""
  say "chikn-web $new_version installed at $bin_path (settings: $config_file)."
  say "Start it with:  $bin_path"
  say "Projects folder: $root"
fi

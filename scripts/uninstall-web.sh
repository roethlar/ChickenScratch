#!/usr/bin/env bash
# Remove the ChickenScratch web version installed by scripts/install-web.sh.
#
#   scripts/uninstall-web.sh                 # stop service, remove program
#   scripts/uninstall-web.sh --purge-config  # also remove the settings file
#
# Your projects folder is never touched. Plan: docs/plans/PLAN_WEB_INSTALLER.md.
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/uninstall-web.sh [--purge-config] [--yes]

  --purge-config  Also delete ~/.config/chickenscratch/web.toml
  --yes           Do not ask for confirmation
EOF
}

say()  { printf '%s\n' "$*"; }
die()  { printf 'error: %s\n' "$*" >&2; exit 1; }

config_home=${XDG_CONFIG_HOME:-$HOME/.config}
config_dir=$config_home/chickenscratch
config_file=$config_dir/web.toml
bin_dir=${CHIKN_BIN_DIR:-$HOME/.local/bin}
bin_path=$bin_dir/chikn-web
unit_name=chikn-web.service
unit_path=$config_home/systemd/user/$unit_name

purge=0
yes=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --purge-config) purge=1; shift ;;
    --yes|-y) yes=1; shift ;;
    --help|-h) usage; exit 0 ;;
    *) usage >&2; die "unknown option: $1" ;;
  esac
done

root=""
if [[ -f $config_file ]]; then
  root=$(sed -n -E 's/^[[:space:]]*root[[:space:]]*=[[:space:]]*"([^"]*)".*/\1/p' "$config_file" | head -n 1)
fi

if [[ $yes -eq 0 ]]; then
  [[ -t 0 ]] || die "not running in a terminal; pass --yes"
  say "This removes the chikn-web service and program."
  [[ $purge -eq 1 ]] && say "It also deletes the settings file."
  [[ -n $root ]] && say "Your projects in $root are kept."
  read -r -p "Continue? [y/N]: " answer </dev/tty
  case "$answer" in y|Y|yes|YES|Yes) ;; *) say "Nothing changed."; exit 0 ;; esac
fi

if command -v systemctl >/dev/null 2>&1 && systemctl --user show-environment >/dev/null 2>&1; then
  systemctl --user stop "$unit_name" 2>/dev/null || true
  systemctl --user disable --quiet "$unit_name" 2>/dev/null || true
  if [[ -f $unit_path ]]; then
    rm -f "$unit_path"
    systemctl --user daemon-reload
    say "Service removed."
  fi
fi

removed=0
for f in "$bin_path" "$bin_path.previous"; do
  if [[ -e $f ]]; then rm -f "$f"; removed=1; fi
done
[[ $removed -eq 1 ]] && say "Program removed from $bin_dir."

if [[ $purge -eq 1 ]]; then
  rm -f "$config_file"
  rmdir "$config_dir" 2>/dev/null || true
  say "Settings removed."
else
  [[ -f $config_file ]] && say "Settings kept at $config_file (use --purge-config to remove)."
fi
[[ -n $root ]] && say "Projects left untouched in $root."
exit 0

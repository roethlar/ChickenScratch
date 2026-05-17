#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/create-release-source.sh <version> [git-ref] [output-dir]

Creates the source archive that pkg/arch/PKGBUILD downloads from the GitHub
release assets. The archive uses the Arch-safe package version in its directory
prefix, and git archive honors .gitattributes export-ignore rules so the
PKGBUILD is not part of the tarball it checksums.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -lt 1 || $# -gt 3 ]]; then
  usage >&2
  exit 2
fi

version="$1"
ref="${2:-HEAD}"
output_dir="${3:-dist}"
pkgname="chickenscratch"
pkgver="${version//-/_}"
archive_name="$pkgname-$pkgver.tar.gz"

root=$(git rev-parse --show-toplevel)
cd "$root"

mkdir -p "$output_dir"
archive_path="$output_dir/$archive_name"

git archive --worktree-attributes --format=tar --prefix="$pkgname-$pkgver/" "$ref" | gzip -n > "$archive_path"

if command -v sha256sum >/dev/null 2>&1; then
  sha256sum "$archive_path"
else
  shasum -a 256 "$archive_path"
fi

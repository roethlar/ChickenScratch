#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/check-release-metadata.sh
  scripts/check-release-metadata.sh --release <version> [--require-tag]

Default mode validates the current metadata and infers prerelease versus release
rules from the version string. Prerelease versions allow a placeholder Arch
checksum; release versions require a pinned checksum. Explicit release mode also
validates that all version metadata matches <version> and can optionally require
the local git tag.
EOF
}

release_version=""
require_tag=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --release)
      [[ $# -ge 2 ]] || {
        echo "ERROR: --release requires a version" >&2
        exit 2
      }
      release_version="$2"
      shift 2
      ;;
    --require-tag)
      require_tag=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "ERROR: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

root=$(git rev-parse --show-toplevel)
cd "$root"

errors=0
fail() {
  echo "ERROR: $*" >&2
  errors=$((errors + 1))
}

toml_version() {
  sed -nE 's/^version[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' "$1" | head -n 1
}

json_version() {
  sed -nE 's/^[[:space:]]*"version"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' "$1" | head -n 1
}

pkgbuild_value() {
  local key="$1"
  sed -nE "s/^${key}=['\"]?([^'\"]+)['\"]?$/\1/p" pkg/arch/PKGBUILD | head -n 1
}

expected="${release_version:-$(toml_version src-tauri/Cargo.toml)}"
if [[ -z "$expected" ]]; then
  fail "could not read src-tauri/Cargo.toml version"
fi

arch_expected="${expected//-/_}"
release_mode=0
if [[ -n "$release_version" || "$expected" != *-* ]]; then
  release_mode=1
fi

version_files=(
  src-tauri/Cargo.toml
  crates/core/Cargo.toml
  crates/cli/Cargo.toml
  crates/tui/Cargo.toml
  linux/Cargo.toml
)

for file in "${version_files[@]}"; do
  actual=$(toml_version "$file")
  [[ "$actual" == "$expected" ]] || fail "$file version is '$actual', expected '$expected'"
done

tauri_version=$(json_version src-tauri/tauri.conf.json)
[[ "$tauri_version" == "$expected" ]] || fail "src-tauri/tauri.conf.json version is '$tauri_version', expected '$expected'"

if ! grep -Fq "v$expected" README.md; then
  fail "README.md does not mention v$expected in the public status line"
fi

status_line=$(grep -m 1 '^\*\*Status:\*\*' README.md || true)
if [[ $release_mode -eq 1 ]]; then
  [[ "$expected" != *-* ]] || fail "release version '$expected' must not be a prerelease"
  [[ "$status_line" != *Alpha* && "$status_line" != *alpha* ]] || fail "README.md status still marks the app as alpha"
else
  [[ "$status_line" == *Alpha* || "$status_line" == *alpha* ]] || fail "README.md status no longer marks this prerelease as alpha"
fi

pkgver=$(pkgbuild_value pkgver)
[[ "$pkgver" == "$arch_expected" ]] || fail "pkg/arch/PKGBUILD pkgver is '$pkgver', expected '$arch_expected'"

expected_url='url="https://github.com/roethlar/ChickenScratch"'
grep -Fxq "$expected_url" pkg/arch/PKGBUILD || fail "pkg/arch/PKGBUILD url must be https://github.com/roethlar/ChickenScratch"

expected_upstream='_upstream_version="${pkgver//_/-}"'
grep -Fxq "$expected_upstream" pkg/arch/PKGBUILD || fail "pkg/arch/PKGBUILD must derive _upstream_version from pkgver"

expected_source='source=("$pkgname-$pkgver.tar.gz::$url/releases/download/v$_upstream_version/$pkgname-$pkgver.tar.gz")'
grep -Fxq "$expected_source" pkg/arch/PKGBUILD || fail "pkg/arch/PKGBUILD source must use the release source archive URL"

grep -Fxq "pkg/arch/PKGBUILD export-ignore" .gitattributes || fail ".gitattributes must export-ignore pkg/arch/PKGBUILD"
grep -Fxq "REVIEW.md export-ignore" .gitattributes || fail ".gitattributes must export-ignore REVIEW.md"

sha_line=$(grep -E "^sha256sums=\(" pkg/arch/PKGBUILD || true)
if [[ $release_mode -eq 1 ]]; then
  if [[ "$sha_line" == *SKIP* ]]; then
    fail "pkg/arch/PKGBUILD sha256sums still uses SKIP"
  fi
  sha_value=$(sed -nE "s/^sha256sums=\('([0-9a-fA-F]{64})'\)$/\1/p" pkg/arch/PKGBUILD | head -n 1)
  [[ -n "$sha_value" ]] || fail "pkg/arch/PKGBUILD sha256sums must contain one pinned 64-character SHA-256"
else
  [[ "$sha_line" == "sha256sums=('SKIP')" ]] || fail "prerelease pkg/arch/PKGBUILD should keep sha256sums=('SKIP') until a release source archive exists"
fi

archive_ref="HEAD"
can_compare_archive=1
if [[ $require_tag -eq 1 ]]; then
  if git rev-parse -q --verify "refs/tags/v$expected" >/dev/null; then
    archive_ref="v$expected"
  else
    can_compare_archive=0
    fail "local tag v$expected does not exist"
  fi
fi

if [[ $release_mode -eq 1 && -n "${sha_value:-}" && $can_compare_archive -eq 1 ]]; then
  archive_tmp=$(mktemp -d)
  if archive_output=$(scripts/create-release-source.sh "$expected" "$archive_ref" "$archive_tmp" 2>&1); then
    archive_sha=$(printf '%s\n' "$archive_output" | awk 'NR == 1 { print $1 }')
    [[ "$archive_sha" == "$sha_value" ]] || fail "pkg/arch/PKGBUILD sha256sums is '$sha_value', but $archive_ref source archive is '$archive_sha'"
  else
    fail "could not create release source archive from $archive_ref: $archive_output"
  fi
  rm -rf "$archive_tmp"
fi

if [[ $errors -gt 0 ]]; then
  exit 1
fi

echo "release metadata ok for $expected"

#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/check-release-metadata.sh
  scripts/check-release-metadata.sh --release <version> [--require-tag]

Default mode validates the current metadata and infers prerelease versus release
rules from the version string. Explicit release mode also validates that all
version metadata matches <version> and can optionally require the local git tag.
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

cargo metadata --locked --format-version 1 >/dev/null || fail "Cargo.lock is stale; run cargo metadata and commit Cargo.lock"

toml_version() {
  sed -nE 's/^version[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' "$1" | head -n 1
}

json_version() {
  sed -nE 's/^[[:space:]]*"version"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' "$1" | head -n 1
}

expected="${release_version:-$(toml_version src-tauri/Cargo.toml)}"
if [[ -z "$expected" ]]; then
  fail "could not read src-tauri/Cargo.toml version"
fi

release_mode=0
if [[ -n "$release_version" || "$expected" != *-* ]]; then
  release_mode=1
fi

version_files=(
  src-tauri/Cargo.toml
  crates/core/Cargo.toml
  crates/cli/Cargo.toml
  crates/tui/Cargo.toml
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

if [[ $require_tag -eq 1 ]]; then
  git rev-parse -q --verify "refs/tags/v$expected" >/dev/null || fail "local tag v$expected does not exist"
fi

if [[ $errors -gt 0 ]]; then
  exit 1
fi

echo "release metadata ok for $expected"

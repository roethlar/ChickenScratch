#!/usr/bin/env bash
# Format harness: converts samples/Corn.scriv with the Rust converter, then
# verifies the resulting project with the Rust reader (env-gated test in
# cross_frontend_round_trip.rs), optionally failing on repair markers.
# The Swift/C# writer legs were removed with the deprecated native trees
# (ADR-004); see git history for the multi-toolchain version.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
if [[ -n "${CHIKN_CROSS_FRONTEND_WORKDIR:-}" ]]; then
  WORKDIR="$CHIKN_CROSS_FRONTEND_WORKDIR"
  CLEANUP_WORKDIR=0
else
  WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/chikn-cross-frontend.XXXXXX")"
  CLEANUP_WORKDIR=1
fi
PROJECT="$WORKDIR/Corn.chikn"
MANIFEST="$WORKDIR/manifest.txt"
HIERARCHY_DOCS="$WORKDIR/hierarchy-docs.txt"
PANDOC_SHIM="$WORKDIR/pandoc-shim"
FAIL_ON_REPAIR="${CHIKN_CROSS_FRONTEND_FAIL_ON_REPAIR:-0}"

cleanup() {
  if [[ "$CLEANUP_WORKDIR" == "1" ]]; then
    rm -rf "$WORKDIR"
  fi
}
trap cleanup EXIT

mkdir -p "$WORKDIR"
: > "$MANIFEST"

log() {
  printf '%s\n' "$*"
  printf '%s\n' "$*" >> "$MANIFEST"
}

run_and_capture() {
  local output_file="$1"
  shift

  set +e
  "$@" >"$output_file" 2>&1
  local status=$?
  set -e

  cat "$output_file"
  cat "$output_file" >> "$MANIFEST"
  return "$status"
}

fail_if_repair_markers_present() {
  local output_file="$1"
  local stage="$2"

  if [[ "$FAIL_ON_REPAIR" != "1" ]]; then
    return 0
  fi

  if grep -E '(^|[[:space:]])(pre-repair:|Repair warning:|Repair skipped|Repaired:|Repaired .+ in memory;)' "$output_file" >/dev/null; then
    log "rust-reader:$stage: repair markers found"
    printf 'FAILED: rust-reader:%s emitted repair markers with CHIKN_CROSS_FRONTEND_FAIL_ON_REPAIR=1\n' "$stage" >&2
    return 1
  fi
}

verify_rust_reader() {
  local stage="$1"
  local marker="${2:-}"
  local output_file="$WORKDIR/rust-reader-$stage.log"
  local env_args=(
    "CHIKN_CROSS_FRONTEND_VERIFY=$PROJECT"
  )

  if [[ -n "$marker" ]]; then
    env_args+=("CHIKN_CROSS_FRONTEND_EXPECT_FIELD=$marker")
  fi

  if [[ -n "${CHIKN_CROSS_FRONTEND_DUMP_HIERARCHY_DOCS:-}" ]]; then
    env_args+=("CHIKN_CROSS_FRONTEND_DUMP_HIERARCHY_DOCS=$CHIKN_CROSS_FRONTEND_DUMP_HIERARCHY_DOCS")
  fi

  if [[ -n "${CHIKN_CROSS_FRONTEND_EXPECT_HIERARCHY_DOCS:-}" ]]; then
    env_args+=("CHIKN_CROSS_FRONTEND_EXPECT_HIERARCHY_DOCS=$CHIKN_CROSS_FRONTEND_EXPECT_HIERARCHY_DOCS")
  fi

  log "rust-reader:$stage: start"
  run_and_capture "$output_file" \
    env \
      "${env_args[@]}" \
      cargo test -p chickenscratch-core --test cross_frontend_round_trip \
        verify_cross_frontend_harness_project_from_env -- --exact --nocapture
  fail_if_repair_markers_present "$output_file" "$stage"
  log "rust-reader:$stage: ok"
}

cd "$ROOT"

log "workdir:$WORKDIR"
log "fixture:samples/Corn.scriv"

cat > "$PANDOC_SHIM" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--version" ]]; then
  echo "pandoc 0.0-cross-frontend-shim"
  exit 0
fi

out=""
args=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    -o)
      out="$2"
      shift 2
      ;;
    *)
      args+=("$1")
      shift
      ;;
  esac
done

input="${args[$((${#args[@]} - 1))]}"

if [[ -n "$out" ]]; then
  {
    printf '{\\rtf1\\ansi\n'
    sed 's/[{}\\]/ /g' "$input"
    printf '\n}\n'
  } > "$out"
else
  sed -E 's/\\[a-zA-Z]+-?[0-9]* ?//g; s/[{}\\]/ /g' "$input"
fi
SH
chmod +x "$PANDOC_SHIM"
log "pandoc:$PANDOC_SHIM"

rm -rf "$PROJECT"
cargo build -p chikn-converter
target/debug/chikn-converter --pandoc "$PANDOC_SHIM" samples/Corn.scriv "$PROJECT"
log "rust-converter: ok"
CHIKN_CROSS_FRONTEND_DUMP_HIERARCHY_DOCS="$HIERARCHY_DOCS" verify_rust_reader "after-rust-converter"

log "manifest:$MANIFEST"
log "harness-result: ok"

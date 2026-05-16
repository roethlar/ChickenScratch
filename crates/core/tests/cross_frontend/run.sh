#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
WORKDIR="${CHIKN_CROSS_FRONTEND_WORKDIR:-$(mktemp -d "${TMPDIR:-/tmp}/chikn-cross-frontend.XXXXXX")}"
PROJECT="$WORKDIR/Corn.chikn"
MANIFEST="$WORKDIR/manifest.txt"
PANDOC_SHIM="$WORKDIR/pandoc-shim"

mkdir -p "$WORKDIR"
: > "$MANIFEST"

log() {
  printf '%s\n' "$*"
  printf '%s\n' "$*" >> "$MANIFEST"
}

verify_rust_reader() {
  local stage="$1"
  local marker="${2:-}"

  log "rust-reader:$stage: start"
  if [[ -n "$marker" ]]; then
    CHIKN_CROSS_FRONTEND_VERIFY="$PROJECT" \
    CHIKN_CROSS_FRONTEND_EXPECT_FIELD="$marker" \
      cargo test -p chickenscratch-core --test cross_frontend_round_trip \
        verify_cross_frontend_harness_project_from_env -- --exact --nocapture
  else
    CHIKN_CROSS_FRONTEND_VERIFY="$PROJECT" \
      cargo test -p chickenscratch-core --test cross_frontend_round_trip \
        verify_cross_frontend_harness_project_from_env -- --exact --nocapture
  fi
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
verify_rust_reader "after-rust-converter"

if command -v swift >/dev/null 2>&1; then
  swift run --package-path macos ChiknKitCrossFrontendHarness "$PROJECT"
  log "swift-chiknkit-writer: ok"
  verify_rust_reader "after-swift-writer" "cross_frontend_swift"
else
  log "swift-chiknkit-writer: skipped (swift not found)"
fi

if command -v dotnet >/dev/null 2>&1; then
  dotnet run --project windows/ChickenScratch.Core.Tests/CrossFrontendHarness/ChickenScratch.Core.CrossFrontendHarness.csproj -- "$PROJECT"
  log "csharp-core-writer: ok"
  verify_rust_reader "after-csharp-writer" "cross_frontend_csharp"
else
  log "csharp-core-writer: skipped (dotnet not found)"
fi

log "manifest:$MANIFEST"
log "result: ok"

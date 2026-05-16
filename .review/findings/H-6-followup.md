# H-6-followup: Cross-Frontend Harness Hardening

## Approach

- Added `trap ... EXIT` cleanup for temp workdirs created by `crates/core/tests/cross_frontend/run.sh`.
- Preserved user-provided `CHIKN_CROSS_FRONTEND_WORKDIR` directories and their manifests for inspection.
- Changed missing Swift/dotnet handling to emit explicit `SKIPPED:` lines to stderr and the manifest.
- Added final writer coverage lines so a run with neither optional writer reports `result: ok-with-skipped-toolchains` instead of plain `result: ok`.
- Added `CHIKN_CROSS_FRONTEND_FAIL_ON_REPAIR=1` handling around each Rust verifier invocation. The harness captures verifier output into the manifest and fails when reader repair markers are present.

## Tests

- `crates/core/tests/cross_frontend/run.sh`
- `CHIKN_CROSS_FRONTEND_FAIL_ON_REPAIR=1 crates/core/tests/cross_frontend/run.sh`
- `cargo test -p chickenscratch-core --test cross_frontend_round_trip`

## Files Changed

- `crates/core/tests/cross_frontend/run.sh`
- `crates/core/tests/cross_frontend_round_trip.rs`
- `.review/findings/H-6-followup.md`
- `REVIEW.md`

## Finding

- Current default behavior remains compatible with the existing converted Scrivener fixture.
- If the current Swift/C# writer drift still causes Rust reader repair output, `CHIKN_CROSS_FRONTEND_FAIL_ON_REPAIR=1` is expected to fail. That failure is intentional: it exposes remaining cross-frontend normalization drift without breaking the default harness path.

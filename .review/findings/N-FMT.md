# N-FMT: rustfmt drift

## Approach

Ran `cargo fmt --all` as a mechanical formatting-only branch. The originally listed `src-tauri/src/commands/document.rs` drift had already been resolved by the N-1 branch, so this branch formats the remaining six Rust files.

## Tests

- `cargo fmt --all -- --check`
- `git diff --check`

## Files changed

- `crates/core/src/core/git.rs`
- `crates/core/src/models/project.rs`
- `crates/tui/src/app.rs`
- `crates/tui/src/ui.rs`
- `linux/src/bridge.rs`
- `src-tauri/src/commands/git.rs`
- `.review/findings/N-FMT.md`
- `REVIEW.md`

## Known gaps

None.

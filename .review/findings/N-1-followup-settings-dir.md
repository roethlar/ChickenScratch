# N-1-followup-settings-dir: `record_daily_words` settings folder unguarded

## Approach

Replaced the raw `fs::create_dir_all(project_path/settings)` in `record_daily_words_impl` with `safe_path::ensure_project_subdir_safe(project_path, "settings")`. This applies the same component validation, symlink rejection, one-component-at-a-time creation, and canonical root containment checks used for N-1 entity folders.

Added tests for the normal writing-history path and the hostile-project case where `settings` is a symlink to a directory outside the project. The hostile test asserts the command returns `ChiknError::InvalidFormat` and leaves the symlink target untouched.

## Tests

- `cargo test -p chickenscratch commands::io::tests --bins`
- `cargo clippy -p chickenscratch --all-targets -- -D warnings`
- `git diff --check`

## Files changed

- `src-tauri/src/commands/io.rs`
- `.review/findings/N-1-followup-settings-dir.md`
- `REVIEW.md`

## Known gaps

None.

# Security/Core Review

## Scope

Reviewed current `master` in `/Users/michael/Dev/ChickenScratch` for Rust core security and data-integrity blockers: `crates/core` project reader/writer, `safe_path`, Scrivener import/export, git destructive operations, settings/secrets, and subprocess/resource bounds.

Treated `R-14`, `R-15`, and `R-16` as already-open findings. I did not refile the Rust reader symlink/read-validation gap covered by `R-14`, the non-atomic document/meta writer gap covered by `R-15`, or compile/export test coverage covered by `R-16`.

No source fixes made. `REVIEW.md` was read but not modified.

## Commands Run

- `cat .review/README.md`
- `cat REVIEW.md`
- `git status --short --branch`
- `rg --files`
- `nl -ba crates/core/src/core/project/safe_path.rs`
- `nl -ba crates/core/src/core/project/writer.rs`
- `nl -ba crates/core/src/core/project/reader.rs`
- `nl -ba crates/core/src/core/git.rs`
- `rg -n "restore_document|create_draft|switch_draft|checkout_head|reset\\(|force\\(|std::fs::write|fs::write|remove_file|remove_dir|Command::|process::Command|timeout|kill|api_key|secret|token|keyring|settings" crates/core/src src-tauri/src/commands`
- `nl -ba src-tauri/src/commands/git.rs`
- `nl -ba crates/core/src/core/compile.rs`
- `nl -ba crates/core/src/utils/process.rs`
- `nl -ba src-tauri/src/commands/settings.rs`
- `nl -ba src-tauri/src/commands/io.rs`
- `nl -ba crates/core/src/scrivener/parser/rtf.rs`
- `nl -ba crates/core/src/scrivener/parser/scrivx.rs`
- `nl -ba crates/core/src/scrivener/converter/mod.rs`
- `nl -ba crates/core/src/scrivener/exporter/mod.rs`
- `rg -n "reject_dirty_worktree|checkout_head\\(|checkout_tree\\(|ResetType::Hard|std::fs::write\\(&abs_target|std::fs::write\\(&abs_meta" crates/core/src/core/git.rs`
- `rg -n "export_to_scriv|write_scrivx|project_name|scrivx_path|escape_xml|created|modified" crates/core/src/scrivener/exporter/mod.rs crates/core/src/core/project/reader.rs`
- `rg -n "read_documents_from_folder|is_dir\\(|is_file\\(|symlink|strip_prefix\\(project_path\\)" crates/core/src/core/project/reader.rs crates/core/src/core/project/writer.rs crates/core/src/core/project/safe_path.rs`
- `rg -n "output_bounded|join_reader|TimedOut|kill\\(|recv_timeout|try_wait" crates/core/src/utils/process.rs`
- `rg -n "settings|secret|keyring|token|api key|api_key" REVIEW.md`
- `sed -n '160,205p' REVIEW.md`
- `rg -n "M-2-followup|app-settings|silent-default" REVIEW.md`
- `sed -n '510,535p' REVIEW.md`
- `nl -ba crates/core/src/models/document.rs`
- `nl -ba crates/core/src/models/project.rs | sed -n '70,105p'`
- `nl -ba crates/core/src/core/project/hierarchy.rs | sed -n '1,260p'`
- `rg -n "documents\\.insert|HashMap<String, Document>|Document \\{|path: .*\\.md|unique_slug|duplicate|collision" crates/core/src src-tauri/src/commands`
- `nl -ba src-tauri/src/commands/document.rs | sed -n '1,260p'`
- `nl -ba src-tauri/src/commands/project.rs | sed -n '1,130p'`
- `ls -la .review .review/agent-reviews`
- `sed -n '1,260p' .review/agent-reviews/security-core.md`
- `git status --short --branch`

One attempted `rg` against `REVIEW.md` had unescaped backticks in the shell pattern and partially failed with `zsh: command not found: .md`; it was rerun with `sed`.

No validation tests were run; this was a static review pass with source reads and search commands.

## Findings

### HIGH NEW - Suggested `R-17`: remaining git write paths bypass dirty-worktree and project path safety

Status: New. This extends the verified `H-3` scope, which guarded `restore_revision` and `sync_pull_force` only. It is not a duplicate of `R-14`, `R-15`, or `R-16`.

Evidence:

- `reject_dirty_worktree` exists at `crates/core/src/core/git.rs:148` but is only called by `restore_revision` at `crates/core/src/core/git.rs:409` and `sync_pull_force` at `crates/core/src/core/git.rs:942`.
- `restore_document` writes directly to `project_path.join(doc_path)` at `crates/core/src/core/git.rs:371` and `std::fs::write`s content at `crates/core/src/core/git.rs:375`; it similarly writes the sidecar at `crates/core/src/core/git.rs:389-390`. It does not validate the relative path, reject symlink file targets, use the project writer safety checks, or guard a dirty worktree before overwriting.
- The Tauri command exposes that path directly under only the process-local write lock at `src-tauri/src/commands/git.rs:141-150`; the lock serializes app commands but does not protect against a hostile symlink already on disk or uncommitted worktree state.
- Draft/sync operations still use forced checkout paths without the dirty guard: `create_draft` at `crates/core/src/core/git.rs:451-455`, `switch_draft` at `crates/core/src/core/git.rs:495-498`, `merge_draft` fast-forward at `crates/core/src/core/git.rs:555-568`, and `sync_pull` fast-forward at `crates/core/src/core/git.rs:843-856`.

Impact:

A hostile or locally corrupted worktree can put `manuscript/chapter.md` as a symlink to an outside file and then use document restore to overwrite the outside target, bypassing the Rust writer protections that closed `C-3`. Separately, draft switching and normal fast-forward pull can discard uncommitted autosaved prose because they still use forced checkout without the `H-3` dirty-worktree guard.

Suggested fix:

Route single-document restore through the same relative-path, canonical-root, symlink-rejection, and atomic-write path used by the project writer, or extract those checks into a shared helper. Add `reject_dirty_worktree` or an explicit pre-restore auto-revision policy before every forced checkout/restore path, not just full revision restore and force pull.

Tests/repro idea:

1. Create and commit a project containing `manuscript/a.md`.
2. Replace `manuscript/a.md` in the worktree with a symlink to a temp file outside the project.
3. Call `git::restore_document(project, "manuscript/a.md", old_commit)`.
4. Assert the outside temp file was not changed and the command returns `InvalidFormat`.
5. Add dirty-worktree tests for `switch_draft`, `create_draft`, `merge_draft` fast-forward, and `sync_pull` fast-forward that prove uncommitted document edits remain unchanged and the command fails cleanly.

### HIGH NEW - Suggested `R-18`: Scrivener exporter uses project name as an unchecked output path component

Status: New. This is distinct from `R-14` path validation drift and from `R-16` compile/export test coverage.

Evidence:

- `export_to_scriv` passes `project.name` into `write_scrivx` at `crates/core/src/scrivener/exporter/mod.rs:64`.
- `write_scrivx` constructs the `.scrivx` destination as `scriv_path.join(format!("{}.scrivx", project_name))` at `crates/core/src/scrivener/exporter/mod.rs:206`, then writes it at `crates/core/src/scrivener/exporter/mod.rs:207`.
- `project.name` is read from `project.yaml` without path-component validation and assigned into the loaded project at `crates/core/src/core/project/reader.rs:214-222`.

Impact:

A shared `.chikn` project can set `name` to a traversal or absolute-looking value. When the user exports to Scrivener, the exporter can write the `.scrivx` XML outside the selected `.scriv` directory. Example shape: exporting to `/tmp/Export.scriv` with `name: "../OtherProject/OtherProject"` writes `/tmp/OtherProject/OtherProject.scrivx` if that parent exists. This can corrupt a sibling Scrivener project or create files outside the export root.

Suggested fix:

Do not derive the output filename from untrusted `project.name`. Use the selected `output_path.file_stem()` or a strict single-component sanitized filename. Reject separators, `.`/`..`, root/prefix components, empty names, and control characters before any `Path::join`.

Tests/repro idea:

Create a `Project` with `name = "../victim/victim"` and call `export_to_scriv(&project, temp/Export.scriv, None)` with `temp/victim` pre-created. Assert no file appears at `temp/victim/victim.scrivx` and the exporter returns `InvalidFormat`.

### HIGH NEW - Suggested `R-19`: duplicate document IDs or paths silently alias content

Status: New. This is not covered by `R-14` path traversal, `R-15` atomicity, or `R-16` compile coverage.

Evidence:

- The core model stores documents in `HashMap<String, Document>` keyed by ID at `crates/core/src/models/project.rs:90-91`.
- The reader inserts every discovered document by `.meta` ID with `documents.insert(doc.id.clone(), doc)` at `crates/core/src/core/project/reader.rs:573-574`; duplicate IDs silently overwrite the earlier document with no warning or error.
- The writer validates individual paths but not uniqueness across the document map in `validate_all_document_targets` at `crates/core/src/core/project/writer.rs:247-289`.
- `write_all_documents` iterates all map values at `crates/core/src/core/project/writer.rs:237-242`, and `write_document` writes `document.content` to `project_path.join(&document.path)` at `crates/core/src/core/project/writer.rs:513-580`. If two in-memory documents have the same `path`, the last `HashMap` iteration wins nondeterministically.

Impact:

A sync conflict, hand edit, cross-frontend bug, or malicious project with duplicate `.meta` IDs can make one document disappear from the loaded map or make two binder nodes resolve to the same content. Duplicate paths can cause one document's content and metadata to overwrite another on save. Because the overwrite order follows `HashMap` iteration, the resulting data loss is hard to predict and hard to recover from in the UI.

Suggested fix:

Validate project invariants on load and before write: unique document IDs, unique document paths, hierarchy document IDs that resolve to exactly one loaded document, and hierarchy paths that match the loaded document for that ID. Fail closed with `ChiknError::InvalidFormat` rather than repairing or overwriting.

Tests/repro idea:

Add reader fixtures with two `.md` files whose `.meta` files share the same `id`; assert `read_project` fails. Add writer tests with two `Document`s with distinct IDs but the same `path`; assert `write_project` fails before any document content is written.

### MEDIUM NEW-EVIDENCE - Suggested `R-20`: subprocess timeout can hang on process trees that keep stdout/stderr open

Status: New concrete failure mode in the verified `M-3` implementation. Not a duplicate of `R-14`, `R-15`, or `R-16`.

Evidence:

- `output_bounded` breaks out as soon as the direct child exits at `crates/core/src/utils/process.rs:112-114`, then immediately joins the stdout/stderr reader threads at `crates/core/src/utils/process.rs:160-161`.
- `join_reader` at `crates/core/src/utils/process.rs:209-213` has no timeout.
- On timeout or output cap, the helper kills only the direct child at `crates/core/src/utils/process.rs:117`, `crates/core/src/utils/process.rs:130`, and `crates/core/src/utils/process.rs:140`, then also joins the reader threads without a deadline.
- Compile, Scrivener RTF conversion, import conversion, Pandoc discovery, and settings Pandoc checks all rely on this helper via `crates/core/src/core/compile.rs:173`, `crates/core/src/scrivener/parser/rtf.rs:33`, `crates/core/src/scrivener/parser/rtf.rs:81`, `crates/core/src/scrivener/parser/rtf.rs:123`, `src-tauri/src/commands/io.rs:183`, and `src-tauri/src/commands/settings.rs:588`.

Impact:

If Pandoc or a configured wrapper spawns a subprocess that inherits stdout/stderr and then the parent exits or is killed, the pipe readers may never see EOF. The nominal 60-second timeout no longer bounds the operation, leaving compile/import/Pandoc checks able to hang indefinitely. This matters most for PDF generation, where Pandoc commonly launches TeX subprocesses.

Suggested fix:

Run subprocesses in a killable process group/job object and kill the whole group on timeout/cap. Keep the deadline active while joining output readers, or switch to a nonblocking/evented read loop that can abandon readers after the process tree is killed.

Tests/repro idea:

On Unix, add a process helper test with `sh -c '(sleep 3600) &'` and a short timeout. The direct shell exits promptly while the background process keeps stdout/stderr open. `output_bounded` should return within the timeout instead of waiting for the background process.

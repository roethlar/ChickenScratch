# Chicken Scratch Code Review

## Scope
Repository walkthrough focused on Rust/Tauri backend project + document persistence and API surface. No automated tests were executed.

## Findings

### 1. Document persistence ignores `Document.path`
- **What happens**: `write_document` always uses `get_manuscript_path(project_path)` and writes files as `folder.join(format!("{}.md", document.name))`, and `delete_document` removes from the same folder (`src-tauri/src/core/project/writer.rs:166`, `src-tauri/src/core/project/writer.rs:203`).
- **Impact**: Any document that should live outside the manuscript root (e.g., `research/notes.md`) or inside subfolders is flattened into the manuscript directory on the next save and will be deleted from there only, leaving stale files elsewhere. Project hierarchy quickly diverges from the filesystem.
- **Suggested fix**:
  - Respect the stored `Document.path` when persisting. Parse it relative to the project root, create parent directories as needed, and write/delete exactly at that location.
  - Validate that the resolved path stays inside the project directory to avoid directory traversal.
  - Extend tests in `writer.rs` to cover nested folders and non-manuscript destinations.

### 2. `create_document` mutates the display name instead of the filename
- **What happens**: `create_document` slugifies the user-provided name and stores that slug in `Document.name`, then reuses it for the filename (`src-tauri/src/api/document_commands.rs:40`, `src-tauri/src/api/document_commands.rs:45`). Design docs expect `Document.name` to stay human-readable (`models/document.rs:13`, `docs/design/PHASE_1_DESIGN.md:248`).
- **Impact**: Displayed names lose original formatting (“Chapter 1” becomes “chapter-1”), and two different titles that collapse to the same slug overwrite each other on disk (`writer.rs:172`, `writer.rs:207`). Deleting one also deletes the other’s content.
- **Suggested fix**:
  - Preserve the original display string in `Document.name`.
  - Compute a slug only for the filesystem path (e.g., `let slug = slugify(&name)`), set `Document.path` using the slug, and keep `Document.name = name`.
  - Consider storing the slug separately or recalculating from the path when needed.
  - Add tests covering duplicate-friendly names so collisions are caught.

### 3. Reader misses nested documents and records absolute paths
- **What happens**: `read_all_documents` enumerates only the top level of each folder (`src-tauri/src/core/project/reader.rs:174`). It also stores each `Document.path` using `content_path.to_string_lossy()`, yielding absolute filesystem paths (`reader.rs:237`).
- **Impact**: Documents placed in manuscript subfolders never load into the project model, breaking navigation and hierarchy integrity. Absolute paths violate the contract that `Document.path` is relative to the `.chikn` root, causing cross-machine portability issues and mismatches with hierarchy entries.
- **Suggested fix**:
  - Convert `read_documents_from_folder` into a recursive walker (or call a helper that recurses) so nested directories are processed.
  - Pass the project root into the helper and store `Document.path` as a relative path via `strip_prefix(project_path)`.
  - Extend reader tests to include nested folders and verify relative paths.

### 4. Returned projects report stale `modified` timestamps
- **What happens**: `write_project_metadata` writes a fresh `modified` timestamp to disk but never mutates the in-memory `Project` struct passed through each command (`src-tauri/src/core/project/writer.rs:136`). API handlers immediately return that struct (`src-tauri/src/api/project_commands.rs:112`, `src-tauri/src/api/document_commands.rs:82`).
- **Impact**: Frontends receive `project.modified` values that lag behind the filesystem, so UI indicators and sync logic use incorrect timestamps.
- **Suggested fix**:
  - Update `Project.modified` before returning, either inside `write_project` (mutate the passed struct) or immediately after calling it in each command.
  - Add coverage to verify that the struct and serialized YAML stay in sync.

## Recommended Next Steps
1. Patch persistence to respect document paths and names, then expand unit tests to cover nested structures and duplicate-friendly titles.
2. Update the reader and writer together to ensure round-trip fidelity for hierarchical projects before enabling richer UI features.
3. Once fixes land, run `cargo test --manifest-path src-tauri/Cargo.toml` to confirm the updated suite passes.

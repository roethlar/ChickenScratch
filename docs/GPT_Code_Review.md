# GPT Code Review

Generated: 2026-05-05
Scope: Rust core/Tauri backend, React/Tiptap UI, macOS Swift package, Windows C# core/app surface, TUI, docs/devlog/TODO.
Format: Markdown with stable finding IDs for LLM-agent follow-up.

## Executive Summary

The Rust core, Tauri backend, React UI, and macOS format checks are in good shape at the build/test level. The highest-risk remaining issues are not compiler failures; they are cross-frontend format drift and workflow races around git operations.

The largest correctness gap is the Windows writer. It still cannot safely round-trip canonical `.meta` files for the shared `.chikn` format: it omits required document identity fields, writes `include_in_compile` in a type shape Rust does not expect, and drops several metadata classes. A Windows save can make the same project unreadable or unusable in the Rust/Tauri reader.

The second major risk is that several user-triggered git operations bypass the pending editor flush path that previous review passes carefully added for autosave, app close, and Ctrl+S. Manual revision save, draft switch/merge, restore, push, and pull can run against stale on-disk state while the live Tiptap buffer still has debounced edits in memory. Pull/force-pull also update disk without reloading the React project store.

## Verification Performed

Passed:

- `cargo test --all-targets`
  - Passed Rust unit/integration tests: 58 core tests + 2 remote sync integration tests; binary crates had 0 tests.
  - This command was run twice because the sandboxed run continued after the escalated run was approved; both completed successfully.
- `cargo clippy --all-targets -- -D warnings`
  - Passed cleanly.
- `npm run lint` in `ui/`
  - Passed with no reported errors or warnings.
- `npm run build` in `ui/`
  - Passed.
  - Warnings: large `index` chunk and ineffective dynamic imports for modules also statically imported.
- `swift run ChiknKitChecks` in `macos/`
  - Passed all format/workflow checks.
  - Warnings:
    - `macos/Sources/ChiknKit/Writer.swift:34`: unused `try?` result for initial commit.
    - `macos/Sources/ChiknKit/Writer.swift:58`: redundant `??` after non-optional dictionary.

Blocked:

- `dotnet build ChickenScratch.slnx` in `windows/`
  - Blocked by local `dotnet`/CoreCLR crash on macOS 26.4.1 with .NET 10.0.7.
  - Crash was in native CoreCLR (`libcoreclr.dylib`, `abort()`), not in a ChickenScratch managed stack.
  - The hung build process was stopped.
- Linux Qt frontend was not built locally because it requires Qt/cxx-qt host dependencies and is excluded from root default workspace builds.

## Findings

### F-001: Windows Writer Corrupts Cross-Frontend Document Identity

- Severity: Critical
- Confidence: High
- Area: Windows, `.chikn` format round-trip
- Status: Open
- Affected files:
  - `windows/ChickenScratch.Core/IO/ProjectYaml.cs:33`
  - `windows/ChickenScratch.Core/IO/ProjectWriter.cs:50`
  - `windows/ChickenScratch.Core/IO/ProjectReader.cs:84`
  - `crates/core/src/core/project/reader.rs:482`
  - `docs/CHIKN_FORMAT_SPEC.md:221`

Issue:

The `.meta` minimal schema requires `id`, `name`, `created`, `modified`, and `parent_id`. The Windows `DocumentMetaYaml` shape does not contain `Id`, `Name`, or `ParentId`, and `ProjectWriter.WriteDocument` does not serialize them. Windows uses the hierarchy ID/name while it is running, so this can look fine inside the Windows app, but the Rust reader keys `project.documents` from the `.meta` id. When `.meta` has no id, Rust generates a new one. The hierarchy still points to the old id from `project.yaml`, so Tauri/Rust can end up with binder nodes whose ids do not exist in `project.documents`.

Impact:

A Windows save can make documents fail to open in Tauri/Rust/Swift readers, or cause repair logic to treat documents inconsistently. This is a cross-frontend data integrity bug, not just a parity gap.

Recommendation:

Add `Id`, `Name`, and `ParentId` to Windows `DocumentMetaYaml`; populate them from `Document` in `ProjectWriter.WriteDocument`; read them in `ProjectReader`; and add a Rust-facing regression test that writes a project through the Windows model/writer shape and reopens it with the Rust reader. At minimum, create an automated fixture that asserts every hierarchy document id has a matching `project.documents` entry after a Windows round-trip.

### F-002: Windows Writes `include_in_compile` as Bool While Rust Expects String

- Severity: Critical
- Confidence: High
- Area: Windows, Rust reader interop
- Status: Open
- Affected files:
  - `windows/ChickenScratch.Core/IO/ProjectYaml.cs:40`
  - `windows/ChickenScratch.Core/IO/ProjectWriter.cs:57`
  - `crates/core/src/core/project/reader.rs:103`
  - `crates/core/src/core/project/reader.rs:537`
  - `docs/CHIKN_FORMAT_SPEC.md:241`

Issue:

Rust `DocumentMetadata.include_in_compile` is `Option<String>` and the Rust writer emits `"Yes"` / `"No"`. Windows models the same field as `bool` and writes YAML booleans. `serde_yaml` will not safely deserialize an arbitrary YAML boolean into `Option<String>`.

Impact:

A Windows-written `.meta` can fail to deserialize in Rust, blocking project load. The spec also currently says `include_in_compile: boolean`, while the Rust canonical writer emits strings, so the documented contract and dominant implementation disagree.

Recommendation:

Pick one wire type and support legacy forms. The safest near-term fix is to make the Rust reader accept both bool and string via an untagged helper, while keeping the writer stable. Then update the spec and Windows writer to match the chosen canonical form.

### F-003: Windows Drops Comments, Scrivener Metadata, Session Target, and Other Format Data

- Severity: High
- Confidence: High
- Area: Windows, format parity
- Status: Open
- Affected files:
  - `windows/ChickenScratch.Core/Models/Models.cs:39`
  - `windows/ChickenScratch.Core/IO/ProjectYaml.cs:14`
  - `windows/ChickenScratch.Core/IO/ProjectYaml.cs:33`
  - `windows/ChickenScratch.Core/IO/ProjectWriter.cs:18`
  - `windows/ChickenScratch.Core/IO/YamlHelper.cs:12`

Issue:

Windows now has a `Fields` dictionary, but it still lacks many current shared-format fields:

- document comments
- `section_type`
- `scrivener_uuid`
- `parent_id`
- `session_target`
- `threads.yaml` model/read/write support

`YamlHelper` uses `.IgnoreUnmatchedProperties()`, and the writer serializes closed POCOs, so unsupported fields are dropped from rewritten YAML.

Impact:

Opening and saving in Windows can strip comments, Scrivener round-trip metadata, and session target settings created by Tauri or SwiftUI. This contradicts the devlog/TODO claim that the five frontends preserve the current format.

Recommendation:

Treat Windows as a preserving reader/writer before adding more UI. Add explicit properties for every format-owned field, and add a generic unknown-preservation strategy if closed POCOs remain. Add an end-to-end fixture with comments, Scrivener ids, fields, session target, and threads.

### F-004: Windows Reader Does Not Load Entity Documents Outside Hierarchy

- Severity: High
- Confidence: High
- Area: Windows, v1.2 novelist convention parity
- Status: Open
- Affected files:
  - `windows/ChickenScratch.Core/IO/ProjectReader.cs:36`
  - `windows/ChickenScratch.Core/IO/ProjectReader.cs:70`
  - `crates/core/src/core/project/reader.rs:397`
  - `macos/Sources/ChiknKit/Reader.swift:132`

Issue:

The Rust and Swift readers walk `manuscript/`, `research/`, `templates/`, `characters/`, and `locations/` directly from disk. Windows only collects documents by walking `project.yaml.hierarchy`. Tauri intentionally keeps `characters/` and `locations/` entities out of hierarchy.

Impact:

Windows cannot see character/location entities created by Tauri or SwiftUI. Depending on future save paths, it may also fail to preserve them as first-class project documents.

Recommendation:

Mirror the Rust/Swift disk-walking reader behavior. Entity docs should be loaded into `Project.Documents` without being inserted into hierarchy.

### F-005: Windows Permanent Folder Delete Leaves Child Files and Documents Behind

- Severity: High
- Confidence: High
- Area: Windows document operations
- Status: Open
- Affected files:
  - `windows/ChickenScratch.Core/IO/DocumentService.cs:44`
  - `windows/ChickenScratch.Core/IO/DocumentService.cs:163`

Issue:

`DeleteNode` permanently deletes an item if it is already in Trash. It calls `DeleteNodeFiles(nodeId, project, projectPath)`, but that helper only deletes when `nodeId` is itself a document id. If the deleted Trash item is a folder, child document files and `project.Documents` entries remain.

Impact:

Permanent deletion of a folder can leave orphaned files and document map entries. A later write or repair pass can resurrect the deleted content.

Recommendation:

Delete by removed `TreeNode`, not by id, and recurse through folder children. This should mirror the fixed Tauri implementation in `src-tauri/src/commands/document.rs`.

### F-006: Windows Move/Reorder Has the Old “Null Parent Means Move to Root” Bug

- Severity: High
- Confidence: High
- Area: Windows document operations
- Status: Open
- Affected files:
  - `windows/ChickenScratch.Core/IO/DocumentService.cs:66`
  - `windows/ChickenScratch.Core/IO/HierarchyOps.cs:75`
  - `src-tauri/src/commands/document.rs:426`

Issue:

The Tauri backend was fixed so `new_parent_id = None` means “keep current parent and only reorder.” Windows still calls `HierarchyOps.MoveNode(project.Hierarchy, nodeId, newParentId)` unconditionally. In `HierarchyOps.MoveNode`, `newParentId == null` removes the node and appends it at root.

Impact:

Move Up/Down or same-parent reorder can extract nested documents/folders to root in the Windows app.

Recommendation:

Port the Tauri semantics: only call parent-changing move when a parent id is supplied; otherwise call reorder within current parent.

### F-007: Manual Git Operations Do Not Flush Pending Tiptap Edits

- Severity: High
- Confidence: High
- Area: Tauri UI, git workflow, data loss
- Status: Open
- Affected files:
  - `ui/src/components/revisions/Revisions.tsx:56`
  - `ui/src/components/revisions/Revisions.tsx:70`
  - `ui/src/components/revisions/Revisions.tsx:80`
  - `ui/src/components/revisions/Revisions.tsx:89`
  - `ui/src/components/revisions/Revisions.tsx:96`
  - `ui/src/components/revisions/Revisions.tsx:120`
  - `ui/src/components/revisions/Revisions.tsx:147`
  - `ui/src/components/revisions/Revisions.tsx:187`
  - `ui/src/components/editor/Editor.tsx:152`

Issue:

Recent devlog fixes correctly flush pending editor saves before app close, auto-commit, periodic backup, and Ctrl+S. The Revisions panel still calls git operations directly without first awaiting `flushPendingEditorSave()`.

Affected operations include:

- named Save Revision
- restore revision
- create/switch/merge draft
- push/fetch/pull
- force pull

Impact:

If a writer types and immediately saves a revision, switches draft, restores, or syncs before the debounce fires, git sees stale on-disk content. The named revision may exclude the last edits. Destructive operations can also overwrite on-disk files while the live editor still holds newer memory-only text.

Recommendation:

Create one UI helper, for example `runGitOperationWithEditorFlush`, that awaits `flushPendingEditorSave()` and aborts on failure before every git operation that reads or mutates project files. Add tests/manual regression cases for “type, immediately click Save Revision” and “type, immediately switch draft.”

### F-008: Pull/Fast-Forward/Force Pull Updates Disk Without Reloading React Project State

- Severity: High
- Confidence: High
- Area: Tauri remote sync
- Status: Open
- Affected files:
  - `ui/src/components/revisions/Revisions.tsx:147`
  - `ui/src/components/revisions/Revisions.tsx:187`
  - `crates/core/src/core/git.rs:577`
  - `crates/core/src/core/git.rs:669`

Issue:

`handlePull` and `handleForcePull` refresh only the revisions panel state after a successful pull. They do not reload the project store or editor buffer. The core pull implementation can fast-forward or reset the working tree, so disk changes underneath React.

Impact:

After pulling remote changes, the UI can continue showing stale documents. The next autosave can write stale editor content back over freshly pulled remote content.

Recommendation:

After `fast_forward`, `merged`, and force-pull success, call `openProject(project.path)` or a narrower reload that updates `project.documents`, `activeDoc`, and the editor buffer. Preserve active selection only after confirming the document still exists.

### F-009: Draft Merge Has No Conflict Result and May Commit Conflict Markers

- Severity: High
- Confidence: Medium
- Area: Core git, Tauri revisions
- Status: Open
- Affected files:
  - `crates/core/src/core/git.rs:345`
  - `crates/core/src/core/git.rs:362`
  - `crates/core/src/core/git.rs:366`
  - `ui/src/components/revisions/Revisions.tsx:96`

Issue:

`merge_draft` calls `repo.merge(...)` and then immediately calls `save_revision(...)`. Unlike `sync_pull`, it does not inspect the index for conflicts or return a conflict result to the UI.

Impact:

A conflicting draft merge can leave conflict markers in working files and may stage/commit them through `save_revision`, or at minimum leave the repository in a confusing merge state without UI conflict handling.

Recommendation:

Give draft merge the same shape as remote pull: merge analysis, conflict detection, conflict file list, abort/resolve paths, and no commit while `index.has_conflicts()`.

### F-010: Unicode Content Can Panic AI and Search Commands

- Severity: High
- Confidence: High
- Area: Tauri backend, Unicode correctness
- Status: Open
- Affected files:
  - `src-tauri/src/commands/ai.rs:33`
  - `src-tauri/src/commands/ai.rs:70`
  - `src-tauri/src/commands/ai.rs:335`
  - `src-tauri/src/commands/search.rs:29`

Issue:

Several Rust commands slice strings by byte offsets:

- `&plain[..2000]`
- `plain[..4000].to_string()`
- `&plain[..4000]`
- search snippets using `&plain[start..end]`

Rust string slicing panics if the boundary lands inside a multi-byte UTF-8 codepoint. Fiction projects commonly contain curly quotes, em dashes, accents, emoji, and non-Latin scripts.

Impact:

AI summarize/transform or project search can panic on valid user text.

Recommendation:

Replace byte slicing with character-safe truncation, for example `plain.chars().take(4000).collect::<String>()`. For snippets, use `char_indices` or a helper that expands to valid boundaries. Also build snippets from original-case text rather than lowercased search text.

### F-011: `restore_document` Swallows Sidecar Restore Errors

- Severity: Medium
- Confidence: High
- Area: Core git restore
- Status: Open
- Affected files:
  - `crates/core/src/core/git.rs:224`
  - `crates/core/src/core/git.rs:237`

Issue:

`restore_document` restores the `.md` blob and then tries to restore the matching `.meta` blob, but the sidecar write uses `let _ = std::fs::write(...)`.

Impact:

The app can report a successful document restore and commit it even if content was restored but metadata/comments/fields stayed stale.

Recommendation:

Propagate sidecar write errors. If sidecar restore is best-effort by design, return a structured warning and avoid presenting the operation as fully successful.

### F-012: Reader Self-Healing Is Partly Blocked by Pre-Validation and Silent Repair Writes

- Severity: Medium
- Confidence: High
- Area: Rust project reader, docs consistency
- Status: Open
- Affected files:
  - `crates/core/src/core/project/reader.rs:164`
  - `crates/core/src/core/project/reader.rs:187`
  - `crates/core/src/core/project/reader.rs:190`
  - `crates/core/src/core/project/format.rs:115`
  - `TODO.md:69`

Issue:

The TODO/roadmap describe the project structure as self-healing, but `read_project` calls `validate_project_structure` before repair. Missing required folders fail validation before the repair code can create them. When repair does happen, the write-back error is swallowed.

Impact:

Users can see load failures for states the docs imply should be repaired, and failed repairs can remain invisible.

Recommendation:

Either narrow the self-healing claim or move repair before strict validation. Propagate repair write failures or return a warning object.

### F-013: Full Project Writes Still Rewrite Every Document

- Severity: Medium
- Confidence: High
- Area: Rust writer, performance/race surface
- Status: Known, still open
- Affected files:
  - `crates/core/src/core/project/writer.rs:70`
  - `crates/core/src/core/project/writer.rs:233`
  - `DEVLOG.md:71`

Issue:

`write_project` rewrites every `.md` and `.meta` file for most operations. The devlog already calls this out as deferred. Recent timestamp fixes reduced git noise, but the I/O pattern remains broad.

Impact:

Large projects pay unnecessary write cost. More importantly, broad rewrites increase the blast radius of partial failures and race windows.

Recommendation:

Split writes into explicit APIs: structure-only, threads-only, project-metadata-only, document-content, document-meta, and delete. Update commands to call the narrowest writer.

### F-014: Compile Settings Are Advertised but Mostly Ignored

- Severity: Medium
- Confidence: High
- Area: Compile/export
- Status: Open
- Affected files:
  - `crates/core/src/core/compile.rs:54`
  - `crates/core/src/core/compile.rs:142`
  - `crates/core/src/core/compile.rs:153`
  - `TODO.md:96`

Issue:

The settings/TODO claim settings-driven formatting for font, size, spacing, and margins. In the compile code:

- `line_spacing` is assigned to `_` and never applied.
- font/font size/margins are only applied to PDF variables.
- DOCX/ODT/HTML output mostly ignores the configured typography.

Impact:

The compile UI can give writers false confidence that manuscript settings are reflected in generated DOCX/ODT/HTML files.

Recommendation:

Implement format-specific Pandoc options, reference DOCX/ODT templates, or clearly scope settings by output type. Add tests around generated Pandoc args for each supported format.

### F-015: Markdown Folder Import Silently Converts Failed Imports to Empty Documents

- Severity: Medium
- Confidence: High
- Area: Import
- Status: Open
- Affected files:
  - `src-tauri/src/commands/io.rs:266`
  - `src-tauri/src/commands/io.rs:268`
  - `src-tauri/src/commands/io.rs:299`

Issue:

`import_markdown_folder` uses `unwrap_or_default()` for file reads and Pandoc conversions. Failed reads/conversions become empty document content, and the project is still written and committed.

Impact:

Import can silently lose source content while reporting success.

Recommendation:

Collect per-file import errors and return a partial-import result, or fail the import transaction before writing/committing.

### F-016: macOS Writer Still Swallows Destructive Filesystem Errors

- Severity: Medium
- Confidence: High
- Area: macOS Swift writer
- Status: Open
- Affected files:
  - `macos/Sources/ChiknKit/Writer.swift:325`
  - `macos/Sources/ChiknKit/Writer.swift:331`
  - `macos/Sources/ChiknKit/Writer.swift:440`

Issue:

The Swift writer uses `try?` when deleting document files, metadata files, and stale `threads.yaml`.

Impact:

This reintroduces the same class of issue fixed in Rust: UI state can claim deletion succeeded while files remain on disk and can be resurrected by a later read.

Recommendation:

Propagate delete failures for document and metadata deletes. For stale `threads.yaml`, either propagate or surface a warning because stale threads can reappear on reload.

### F-017: TUI Comment Metadata Writes Swallow Errors

- Severity: Low
- Confidence: High
- Area: TUI
- Status: Open
- Affected files:
  - `crates/tui/src/app.rs:371`
  - `crates/tui/src/app.rs:430`
  - `crates/tui/src/app.rs:455`

Issue:

Several TUI comment mutations call `writer::write_project` with `let _ = ...`, while the main save path correctly propagates errors.

Impact:

Comment add/resolve/edit can appear successful even if the project write failed.

Recommendation:

Return `Result<()>` from these helpers and set status to the concrete write error on failure.

### F-018: Spec and Planning Docs Have Drift Around Current Format and Status

- Severity: Low
- Confidence: High
- Area: Documentation
- Status: Open
- Affected files:
  - `docs/CHIKN_FORMAT_SPEC.md:2`
  - `docs/CHIKN_FORMAT_SPEC.md:241`
  - `docs/CHIKN_FORMAT_SPEC.md:428`
  - `docs/ROADMAP.md:96`
  - `TODO.md:138`

Issue:

The docs are useful, but several parts are stale or internally inconsistent:

- Format spec `Last Updated` is `2026-04-23`, while the devlog says the key v1.2 reversal happened on `2026-04-27`.
- The spec says `include_in_compile: boolean`, while Rust writes `"Yes"` / `"No"`.
- ROADMAP still labels AI streaming and remote merge UX as “Planned” under v1.1, while TODO marks them done.
- TODO says all five UIs were audited for round-trip preservation, but Windows still has serious format-preservation gaps.

Impact:

Future agents may implement against the wrong contract.

Recommendation:

Update the spec after deciding the canonical `include_in_compile` wire type. Move completed v1.1 items out of “planned,” and make Windows parity status explicit.

## Suggested Priority Order

1. Fix Windows `.meta` identity and wire-type compatibility (`F-001`, `F-002`).
2. Add cross-frontend round-trip tests that include Rust/Tauri, Swift, and Windows model/writer shapes (`F-001` through `F-004`).
3. Centralize Tauri git operations behind a pending-editor-flush and project-reload helper (`F-007`, `F-008`).
4. Add conflict-aware draft merge (`F-009`).
5. Patch Unicode slicing panics in AI/search (`F-010`).
6. Replace remaining swallowed filesystem writes/deletes with propagated errors (`F-011`, `F-016`, `F-017`).
7. Decide whether compile settings are real output controls or UI-only hints, then update implementation/docs (`F-014`, `F-018`).

## Regression Test Ideas

- Rust fixture: create project with two docs, comments, fields, links, Scrivener ids, `include_in_compile`, `session_target`, and `threads.yaml`; run through Windows writer shape; reopen with Rust; assert hierarchy ids match document map ids.
- UI manual/Playwright-style test: type into editor and immediately click Save Revision; assert commit contains the typed text.
- UI sync test: type into editor and immediately switch draft/pull; assert operation is blocked until flush succeeds.
- Remote sync test: fast-forward pull changes active document; assert React project store and editor buffer update.
- Rust unit test: AI/search truncation with multibyte characters at positions 1999/2000/3999/4000.
- Import test: folder import with one unreadable/unsupported file returns an error instead of creating an empty document.

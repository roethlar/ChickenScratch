# Cross-Frontend Review

## Scope

Reviewed current `master` for cross-frontend `.chikn` drift and native implementation risks, with emphasis on:

- `macos/Sources/ChiknKit`
- `windows/ChickenScratch.Core`
- `linux/src/bridge.rs` and QML entry points
- Rust core project model serialization, reader/writer behavior, and cross-frontend round-trip tests
- Existing open blockers R-14/R-15/R-16, without restating them unless there was new evidence

I did not edit code, create branches, or touch `REVIEW.md`.

## Commands Run

```bash
git status --short --branch
sed -n '1,240p' .review/README.md
sed -n '1,260p' REVIEW.md
sed -n '508,620p' REVIEW.md
rg -n "R-1[4-6]|R-1[0-9]|\\.chikn|chikn" REVIEW.md
rg --files macos/Sources/ChiknKit windows/ChickenScratch.Core linux crates sample tests .review/findings
find .review -maxdepth 3 -type f | sort
nl -ba crates/core/src/models/{project.rs,document.rs,hierarchy.rs}
nl -ba crates/core/src/core/project/{reader.rs,writer.rs,safe_path.rs,format.rs}
nl -ba crates/core/tests/cross_frontend_round_trip.rs
nl -ba crates/core/tests/cross_frontend/run.sh
nl -ba macos/Sources/ChiknKit/{Models.swift,Reader.swift,Writer.swift}
nl -ba macos/Tests/ChiknKitChecks/main.swift
nl -ba windows/ChickenScratch.Core/Models/Models.cs
nl -ba windows/ChickenScratch.Core/IO/{ProjectReader.cs,ProjectWriter.cs,ProjectYaml.cs,YamlHelper.cs,DocumentService.cs}
nl -ba linux/src/bridge.rs
nl -ba linux/qml/{Main.qml,Binder.qml,Editor.qml}
nl -ba .github/workflows/validation.yml
tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/chikn-review-cross.XXXXXX")"
CHIKN_CROSS_FRONTEND_WORKDIR="$tmpdir" crates/core/tests/cross_frontend/run.sh >/tmp/chikn-review-cross.log 2>&1
rg -n "Repaired:|Repair warning:|pre-repair:|Repair skipped" /tmp/chikn-review-cross.log "$tmpdir/manifest.txt"
rg -n "^\\s*type:" "$tmpdir/Corn.chikn/project.yaml"
git status --short
```

Note: the cross-frontend harness itself reached `result: ok` with both Swift and C# writers run. I preserved and inspected the temp workdir plus `/tmp/chikn-review-cross.log`.

## Findings

### R-17 suggested: macOS drops Rust-written hierarchy and the default CI harness still passes

- **Severity**: HIGH beta data integrity / cross-frontend blocker.
- **Status**: New REVIEW finding. Related to the H-6-followup "repair marker" drift, but not a duplicate of R-14/R-15/R-16. This is the concrete cause and user-facing impact.
- **Evidence**:
  - Rust `TreeNode` is serialized with `#[serde(tag = "type")]`; lowercase is only a read alias for Rust, while the canonical variants remain `Folder` / `Document`: `crates/core/src/models/hierarchy.rs:16`, `crates/core/src/models/hierarchy.rs:18`, `crates/core/src/models/hierarchy.rs:24`.
  - Swift only accepts exact lowercase enum raw values: `macos/Sources/ChiknKit/Models.swift:102` and `macos/Sources/ChiknKit/Reader.swift:64-70`.
  - Swift then rewrites `project.yaml` from the decoded hierarchy: `macos/Sources/ChiknKit/Writer.swift:643-648` and `macos/Sources/ChiknKit/Writer.swift:740-749`.
  - The env verifier only asserts `project.documents` is non-empty, not that hierarchy still references those docs: `crates/core/tests/cross_frontend_round_trip.rs:27-31`.
  - CI runs the harness without `CHIKN_CROSS_FRONTEND_FAIL_ON_REPAIR=1`: `.github/workflows/validation.yml:86-92`.
- **Repro/evidence from run**:
  - After `chikn-converter`: Rust reader loaded `"Corn"` with `16 docs, 3 top-level nodes`.
  - After the Swift writer: Rust reader logged `Repaired: adding 16 orphaned documents to hierarchy` plus added `Manuscript`, `Research`, and `Trash`.
  - After the C# writer, the preserved `project.yaml` had only three empty folders under `hierarchy`, so the manuscript binder structure was gone while the harness still reported `result: ok`.
- **Suggested fix direction**: Make Swift decode hierarchy tags case-insensitively or accept both Rust canonical and lowercase wire forms. Then make the default CI harness fail on repair markers or add a structural assertion that every non-entity hierarchy document id/path survives after each writer pass.
- **Test idea**: Add a Swift check that opens a Rust-written fixture with `type: Document` / `type: Folder`, calls `Writer.saveDocumentMeta`, and asserts `project.yaml.hierarchy` is byte/structure equivalent except for the intended metadata timestamp/field change. Also make `cross_frontend_round_trip.rs` assert hierarchy doc count and ids after each harness stage.

### R-18 suggested: Native readers still persist destructive repair/malformed hierarchy after missing or corrupt document files

- **Severity**: HIGH beta data integrity.
- **Status**: New native parity gap. Related to the H-1/R-15 class, but those current open items do not cover Swift/Windows native readers.
- **Evidence**:
  - Rust was explicitly changed to keep missing hierarchy/document references in memory and only warn: `crates/core/src/core/project/reader.rs:319-327` and `crates/core/src/core/project/reader.rs:371-389`.
  - Windows still removes hierarchy nodes when a referenced file is missing: `windows/ChickenScratch.Core/IO/ProjectReader.cs:68-75` and `windows/ChickenScratch.Core/IO/ProjectReader.cs:286-296`.
  - Native Windows operations commonly do `ReadProject(...)` followed by `WriteProject(...)`, which persists that destructive repair on the next action: `windows/ChickenScratch.Core/IO/DocumentService.cs:8-25`, `windows/ChickenScratch.Core/IO/ProjectWriter.cs:8-43`.
  - Swift silently drops any `.md` whose `.meta` is missing, corrupt, or lacks `id`: `macos/Sources/ChiknKit/Reader.swift:155-159`; it also converts content read errors into empty content: `macos/Sources/ChiknKit/Reader.swift:161`.
  - If the hierarchy still contains a document node but `project.documents` lacks that id, the Swift writer serializes `path` from a missing lookup: `macos/Sources/ChiknKit/Writer.swift:740-746`. That can rewrite document nodes with a nil path on the next project metadata write.
- **Impact**: A transient sync miss, corrupt `.meta`, or temporarily unavailable file can become permanent manifest damage after a harmless native action such as create/rename/save metadata. This reopens the data-loss behavior H-1 closed for the Rust/Tauri reader.
- **Suggested fix direction**: Port the Rust reader's non-destructive missing-file semantics to Windows and Swift. For corrupt/missing meta, either fail load with a clear error or quarantine/recover in the same policy R-15 chooses, but do not drop nodes or rewrite them with empty/null paths.
- **Test idea**: Fixture with a valid hierarchy entry whose `.md` is temporarily renamed away. Open in Windows, perform `DocumentService.CreateDocument`, then assert the old hierarchy entry is still present in `project.yaml`. Swift equivalent: remove/corrupt a `.meta`, call `Reader.readProject` then `Writer.touchProject`, and assert the document node path is preserved or the operation fails before rewriting.

### R-19 suggested: Linux delete removes only the manifest entry; deleted documents resurrect on reload

- **Severity**: HIGH for Linux native data integrity.
- **Status**: New finding. Same failure class as older native delete bugs, but no current open R item covers Linux.
- **Evidence**:
  - Linux `delete_node` removes the node from the in-memory hierarchy and removes the id from `project.documents`, then calls `writer::write_project`: `linux/src/bridge.rs:670-687`.
  - The helper only mutates the hierarchy vector; it never calls `writer::delete_document`: `linux/src/bridge.rs:1391-1399`.
  - Rust reader repair later treats the still-on-disk `.md` as an orphan and re-adds it to hierarchy: `crates/core/src/core/project/reader.rs:342-368`.
  - The Tauri command shows the intended pattern: remove hierarchy, delete `.md`/`.meta`, remove document map entries, then write: `src-tauri/src/commands/document.rs:428-436` and `src-tauri/src/commands/document.rs:441-457`.
- **Impact**: In the Linux UI, deleting a document or folder appears successful, but the content remains on disk. On reopen, repair imports the orphan again, so deleted prose can reappear and Git history records misleading manifest-only deletes.
- **Suggested fix direction**: Reuse the Tauri recursive delete pattern in `linux/src/bridge.rs`: capture the removed subtree, call `writer::delete_document` for each document before dropping it from `project.documents`, and abort the UI operation if any filesystem deletion fails.
- **Test idea**: Linux bridge/core-level unit test that creates a project, adds a doc, calls `delete_node`, then asserts both `.md` and `.meta` are gone and a fresh `reader::read_project` does not contain the deleted id/path.

### R-20 suggested: Linux new-document path generation can create duplicate document paths; core writer does not reject duplicates

- **Severity**: HIGH for Linux native data integrity; MEDIUM as a core validation gap.
- **Status**: New finding.
- **Evidence**:
  - Linux generates a plain slug and path without checking existing document paths: `linux/src/bridge.rs:568-580`, then inserts the new document and writes the whole project: `linux/src/bridge.rs:602-615`.
  - The shared Rust helper already exists and checks existing `Document.path` values: `crates/core/src/utils/slug.rs:41-55`.
  - Tauri uses the shared unique helper: `src-tauri/src/commands/document.rs:294-296`. Windows and macOS also use unique slug helpers: `windows/ChickenScratch.Core/IO/DocumentService.cs:11` and `macos/Sources/ChiknKit/Writer.swift:594-603`.
  - Core writer validates path safety, symlinks, and `.meta` parseability, but it does not reject two documents with the same `document.path`: `crates/core/src/core/project/writer.rs:247-289`.
  - `write_all_documents` then writes every document value to disk; duplicate paths make later writes overwrite earlier content/meta for the same file: `crates/core/src/core/project/writer.rs:237-242` and `crates/core/src/core/project/writer.rs:508-583`.
- **Impact**: Creating two Linux documents with the same title (a common workflow) produces two document ids pointing at the same `manuscript/<slug>.md`. Saves can overwrite content/meta nondeterministically because the backing map iteration order is not a user-facing ordering guarantee, and a reload can only recover one physical file.
- **Suggested fix direction**: Use `chickenscratch_core::utils::slug::unique_slug` in the Linux bridge and add a core writer preflight that rejects duplicate `Document.path` values before writing anything.
- **Test idea**: Linux bridge test or lower-level Rust test that creates two docs named `Chapter One` and asserts paths become `manuscript/chapter-one.md` and `manuscript/chapter-one-1.md`. Core writer regression: build a `Project` with two documents sharing one path and assert `write_project` returns `InvalidFormat` before touching disk.

## Duplicate Notes

- R-14 already covers macOS/Windows unsafe document path joins and symlink bypasses. I did not restate those as a separate finding.
- R-15 already covers Rust `.md`/`.meta` atomicity. The native missing/corrupt-file repair finding above is separate because it affects Swift/Windows read/write behavior and can persist manifest damage even after Rust atomic writes are fixed.
- R-16 is compile/export coverage and was not duplicated.

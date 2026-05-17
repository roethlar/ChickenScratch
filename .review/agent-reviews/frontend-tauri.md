# Frontend/Tauri Review Pass

## Scope

- Review-only pass on current `master` at `abd2fa5`.
- Focused on `ui/src`, `src-tauri/src/commands`, save/compile/restore flows, AI streaming, close handling, dirty-editor state, and release-blocking dialog/accessibility paths.
- R-1..R-13 treated as verified. R-14/R-15/R-16 treated as already open; duplicate notes below only add concrete evidence for those fixes.
- No source fixes attempted. `REVIEW.md` was not edited.

## Commands Run

- `sed -n '1,240p' .review/README.md`
- `sed -n '1,760p' REVIEW.md`
- `git status --short --branch`
- `git rev-parse --short HEAD`
- `rg --files ui/src src-tauri/src`
- `rg -n "dirty|flushPendingEditorSave|save|autosave|onCloseRequested|close|beforeunload|restore|compile|stream|cancel|dialog|confirm|alert|invoke\\(" ui/src src-tauri/src`
- `rg -n "checkout_head\\(|checkout_tree\\(|reset\\(|push_backup\\(|sync_pull\\(|create_draft\\(|switch_draft\\(|restore_document\\(" crates/core/src/core/git.rs src-tauri/src/commands/git.rs ui/src/components/revisions/Revisions.tsx ui/src/components/revisions/DocumentHistory.tsx`
- Focused `nl -ba ... | sed -n ...` reads of `ui/src/App.tsx`, `ui/src/components/editor/Editor.tsx`, `ui/src/components/editor/Toolbar.tsx`, `ui/src/components/editor/editorRef.ts`, `ui/src/stores/projectStore.ts`, `ui/src/components/revisions/Revisions.tsx`, `ui/src/components/revisions/DocumentHistory.tsx`, `ui/src/components/compile/CompileDialog.tsx`, `ui/src/commands/ai.ts`, `src-tauri/src/commands/{ai,document,git,io,project,mod}.rs`, `src-tauri/src/main.rs`, `crates/core/src/core/git.rs`, `crates/core/src/core/project/{reader,writer,safe_path}.rs`, `src-tauri/tauri.conf.json`, and `src-tauri/capabilities/default.json`.

## Findings

### 1. Dirty draft/pull operations still discard uncommitted editor saves

- Severity: HIGH
- Duplicate/new status: New release/beta blocker; related to H-3's earlier dirty-worktree theme, but not R-14/R-15/R-16.
- Suggested REVIEW id: R-17
- File refs: `ui/src/components/revisions/Revisions.tsx:71`, `ui/src/components/revisions/Revisions.tsx:119`, `ui/src/components/revisions/Revisions.tsx:128`, `ui/src/components/revisions/Revisions.tsx:138`, `ui/src/components/revisions/Revisions.tsx:206`; `crates/core/src/core/git.rs:148`, `crates/core/src/core/git.rs:437`, `crates/core/src/core/git.rs:455`, `crates/core/src/core/git.rs:491`, `crates/core/src/core/git.rs:498`, `crates/core/src/core/git.rs:555`, `crates/core/src/core/git.rs:567`, `crates/core/src/core/git.rs:843`, `crates/core/src/core/git.rs:855`.
- Evidence: `runWithEditorFlush` drains the Tiptap debounce to disk, but it does not create a revision or reject a dirty worktree. `create_draft` and `switch_draft` then call `checkout_head(...force())`; `merge_draft` and `sync_pull` do the same on fast-forward paths. The existing `reject_dirty_worktree` helper is wired into `restore_revision` and `sync_pull_force`, but not these operations.
- Impact: A writer can type after the last saved revision, click New Draft Version / Switch Draft / Pull, and have the just-flushed-but-uncommitted document overwritten by a forced checkout. This is a direct beta data-loss path.
- Test/repro idea: Create a project, commit baseline, edit a document without `save_revision`, then call `create_draft`, `switch_draft`, and a fast-forward `sync_pull`; assert each rejects with a dirty-worktree error and leaves the file unchanged. Add a UI harness for "type -> New Draft Version" preserving the typed text.

### 2. Manual Backup reports success while omitting current work

- Severity: HIGH
- Duplicate/new status: New release/beta blocker.
- Suggested REVIEW id: R-18
- File refs: `ui/src/components/revisions/Revisions.tsx:152`, `ui/src/components/revisions/Revisions.tsx:160`, `src-tauri/src/commands/git.rs:93`, `src-tauri/src/commands/git.rs:99`, `src-tauri/src/commands/git.rs:253`, `src-tauri/src/commands/git.rs:261`, `crates/core/src/core/git.rs:633`, `crates/core/src/core/git.rs:673`.
- Evidence: The Revisions footer Backup button calls `gitCmd.pushBackup` directly. It does not use `runWithEditorFlush`, does not call `save_revision`, and the backend `push_backup` only pushes the current branch refspec. By contrast, `backup_on_close` first commits dirty changes with "Auto-save on close" before pushing.
- Impact: After typing, clicking Backup can toast "Backup complete" while the backup repo lacks both pending debounce edits and already-flushed but uncommitted edits. If the local project is lost, the advertised backup is stale.
- Test/repro idea: Commit baseline, edit a document, click/call manual backup, clone the bare backup and assert the edited text is missing. Expected fix should route manual backup through the same flush + auto-revision path as close/periodic backup, or clearly block until the user saves a revision.

### 3. Document/flow switches fire-and-forget failed flushes, then clear dirty state

- Severity: HIGH
- Duplicate/new status: New dirty-editor data-loss path.
- Suggested REVIEW id: R-19
- File refs: `ui/src/stores/projectStore.ts:91`, `ui/src/stores/projectStore.ts:95`, `ui/src/components/editor/Editor.tsx:152`, `ui/src/components/editor/Editor.tsx:208`, `ui/src/components/editor/Editor.tsx:210`, `ui/src/components/editor/Editor.tsx:291`, `ui/src/components/editor/Editor.tsx:327`, `ui/src/components/editor/Editor.tsx:334`, `ui/src/components/editor/Editor.tsx:343`, `ui/src/components/editor/Editor.tsx:318`, `ui/src/components/editor/Editor.tsx:349`, `ui/src/components/editor/Toolbar.tsx:304`, `ui/src/components/editor/Toolbar.tsx:310`.
- Evidence: `flushPendingSave` correctly throws when `update_document_content` fails. The editor load effect calls it without `await` before replacing the buffer for flow entry/exit and document switches, then clears dirty state after loading the new buffer. `selectDocument` itself is synchronous, and the Flow exit button catches a flush failure but exits flow anyway.
- Impact: On disk-full, permission, path-validation, or partial-write failure, the outgoing document can fail to save, the UI still navigates away, and the "Modified" state can be cleared. Close handling later sees no dirty editor and will not block shutdown, so the user can lose the outgoing buffer despite a transient toast.
- Test/repro idea: Mock `update_document_content` to reject, edit doc A, select doc B, and assert navigation is blocked or doc A remains visibly dirty with close prevented. Repeat for flow exit.

### 4. AI replacement deletes the source selection before the stream has succeeded

- Severity: HIGH
- Duplicate/new status: New AI streaming data-loss path.
- Suggested REVIEW id: R-20
- File refs: `ui/src/components/editor/Toolbar.tsx:350`, `ui/src/components/editor/Toolbar.tsx:356`, `ui/src/components/editor/Toolbar.tsx:417`, `ui/src/components/editor/Toolbar.tsx:422`, `ui/src/components/editor/Toolbar.tsx:439`, `ui/src/components/editor/Toolbar.tsx:442`, `ui/src/components/editor/Editor.tsx:254`, `ui/src/components/editor/Editor.tsx:256`.
- Evidence: For polish/expand/simplify, the toolbar captures `selectedText`, immediately deletes the selection, then streams replacement chunks. On `ai:error`, network failure, or context cancellation, the catch path only shows a toast; it does not restore `selectedText`. The deletion is a normal editor update, so autosave can persist it.
- Impact: A common beta path such as invalid API key, dropped network, provider error, or navigating away during a replacement can erase selected prose. Undo may help during the same live session, but the app should not autosave an AI failure as a successful deletion.
- Test/repro idea: Configure a failing AI endpoint, select text, run Polish, and assert the selected text remains after the stream rejects. Also test navigation abort during an in-flight replacement.

### 5. Addenda for R-14/R-15: read/restore paths bypass safe path and atomic-write hardening

- Severity: HIGH
- Duplicate/new status: Duplicate/additional concrete evidence for already-open R-14 and R-15; no new REVIEW id suggested.
- File refs: `crates/core/src/core/project/reader.rs:569`, `crates/core/src/core/project/reader.rs:577`, `crates/core/src/core/project/reader.rs:587`, `crates/core/src/core/project/reader.rs:589`, `crates/core/src/core/project/reader.rs:637`, `crates/core/src/core/project/reader.rs:647`, `ui/src/components/revisions/DocumentHistory.tsx:70`, `ui/src/components/revisions/DocumentHistory.tsx:71`, `src-tauri/src/commands/git.rs:142`, `src-tauri/src/commands/git.rs:149`, `crates/core/src/core/git.rs:371`, `crates/core/src/core/git.rs:373`, `crates/core/src/core/git.rs:375`, `crates/core/src/core/git.rs:389`, `crates/core/src/core/git.rs:390`.
- Evidence: `read_documents_from_folder` uses `path.is_file()` / `path.is_dir()`, which follows symlinks, then `read_document` reads the target and computes a lexical `strip_prefix` path. A hostile project can place `manuscript/leak.md -> /outside/secret` and have Tauri load/display outside content. Separately, Document History restore forwards `doc.path` to `restore_document`, which uses raw `project_path.join(doc_path)`, `create_dir_all`, and bare `fs::write` for both `.md` and `.meta`; it bypasses the writer's safe-path checks and R-15's anticipated atomic document write fix.
- Impact: R-14 should cover reader-side symlink rejection and the git document-restore write path, not only normal writer saves. R-15 should cover `restore_document`'s content/meta writes as well as `writer.rs`.
- Test/repro idea: Add a malicious fixture with a symlinked `manuscript/*.md` and assert `read_project` rejects without reading the target. Add a restore-document test where the target document or `.meta` is a symlink and assert the outside target is untouched. Add a crash/corrupt-write test for `restore_document` alongside the normal writer atomicity tests.

## No Additional Findings In These Areas

- I did not find a new compile/export blocker beyond R-16's already-open zero-coverage issue and the R-15 atomicity concern. The Tauri compile dialog does await `flushPendingEditorSave()` before calling `compile_project`.
- The Tauri close handler uses `onCloseRequested`; the installed `@tauri-apps/api` implementation awaits the async handler before destroying the window, so calling `preventDefault()` after the awaited flush is still effective in this version.

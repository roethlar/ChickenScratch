# Code Review — Active Fix Cycle

This file is the coordination point between **GPT** (implementer) and **Claude** (review lead). GPT works through the open findings below. Claude reviews each batch of commits, verifies fixes, and updates statuses.

## How this works

1. GPT picks the highest-priority `[ ]` item, implements the fix (with tests where reasonable).
2. GPT runs the **Validation suite** below — must pass before commit.
3. GPT commits with a message naming the finding id (e.g. `Fix C-1: validate Scrivener UUIDs`).
4. GPT updates this file: flips `[ ]` → `[~]`, fills in the **Approach** and any **Tests added** lines, lists touched files.
5. Claude reviews on the next pass and either:
   - flips `[~]` → `[x]` (Verified) with a one-line confirmation, **or**
   - reopens with `[ ]` and notes under **Reviewer comments** about what's still wrong.
6. Do not touch items already `[~]` or `[x]` — they're awaiting verification or done.

## Status legend

- `[ ]` Open — not started, or reopened by reviewer
- `[~]` In progress / awaiting verification
- `[x]` Verified by reviewer

## Validation suite

Run from repo root. All must pass before commit.

```bash
cargo fmt --all
cargo clippy -p chickenscratch-core -p chickenscratch -p chickenscratch-tui -p chikn-converter --all-targets -- -D warnings
cargo test -p chickenscratch-core -p chickenscratch -p chickenscratch-tui -p chikn-converter --lib --bins --tests
cd ui && npm run lint && npm run build && cd ..
```

The `linux/` crate is excluded from the default `--workspace` because Qt6 doesn't build on this dev's macOS host. If you touch `crates/core` public API, check `linux/src/bridge.rs` compiles on your end before claiming done.

---

## CRITICAL — data-loss or remote-exploitable

### C-1: Scrivener importer — UUID path traversal `[~]`
- **What**: `BinderItem.uuid` is raw XML data; `Path::join(uuid)` with an absolute or `..`-bearing UUID reads arbitrary host files into the imported project as document content. Arbitrary-read primitive.
- **Files**: `crates/core/src/scrivener/parser/scrivx.rs:50` (uuid field), `:181-196` (`get_rtf_path`/`get_media_path`); `crates/core/src/scrivener/converter/mod.rs:381` (callsite).
- **Approach**: _(GPT to fill in)_ — added `validate_scrivener_uuid` at scrivx.rs:186.
- **Tests added**: _(GPT to fill in)_
- **Reviewer comments**:

### C-2: Scrivener importer — `<FileExtension>` sanitization `[~]`
- **What**: User-controlled `ext` interpolated into destination paths for media copy. `write_project` currently rejects the resulting `..` path, so the import aborts — not an escape primitive today, but a malformed-import / partial-write footgun. Treating as CRITICAL because it's adjacent to C-1 and both should land together.
- **Files**: `crates/core/src/scrivener/converter/mod.rs:450-491`; `crates/core/src/scrivener/parser/scrivx.rs` extension field.
- **Approach**: _(GPT to fill in)_ — added `sanitize_file_extension` at scrivx.rs:206.
- **Tests added**: _(GPT to fill in)_
- **Reviewer comments**:

### C-3: Symlink writes outside project root `[~]`
- **What**: `writer.rs:253` only checks string traversal (`..`) and absolute paths. Symlinks aren't checked. A hostile project whose `manuscript/chapter.md` is a symlink to `~/.ssh/authorized_keys` will be overwritten on next save. Same vector on delete.
- **Branch**: `fix/c-3-symlink-writes`
- **Files**: `crates/core/src/core/project/writer.rs:253` (write), `:330-360` (delete).
- **Notes for GPT**: Use `std::fs::symlink_metadata(...).file_type().is_symlink()` on the full path before write. Reject with `ChiknError::InvalidFormat`. Also harden the traversal check to component-based: `Path::components().any(|c| matches!(c, Component::ParentDir | Component::RootDir | Component::Prefix(_)))` is more robust than `contains("..")`.
- **Tests**: write_project should reject a project containing a symlinked doc; delete_document should refuse to remove a symlink. Add to `crates/core/src/core/project/writer.rs` test module.
- **Approach**: added component-based document path validation, canonical project-root checks, safe directory creation, symlink rejection for parent directories, document files, and `.meta` targets before write/delete.
- **Tests added**: writer tests for parent traversal, dot-dot filename allowance, symlink parent rejection, symlink document write rejection, symlink parent delete rejection, and symlink document delete rejection.

### C-4: No write-lock on read-modify-write `[~]`
- **What**: Concurrent Tauri command invocations (auto-save + auto-commit + backup) interleave `read_project → mutate → write_project` and silently lose work.
- **Files**: `src-tauri/src/commands/document.rs`, `git.rs`, `project.rs`, `io.rs`, `threads.rs` — every command that mutates.
- **Approach**: _(GPT to fill in)_ — `ProjectWriteLocks` Tauri state added; `update_document_content` wrapped at document.rs:22.
- **Tests added**: _(GPT to fill in)_
- **Reviewer comments**: Confirm every mutating command is wrapped, not just `update_document_content`. The fix is partial if `add_comment`, `update_document_metadata`, `rename_node`, `save_revision`, `restore_revision`, etc. still bypass the lock.

### C-5: AI streaming writes to wrong document after navigation `[~]`
- **What**: `Toolbar.tsx` closure captures `editor` at stream start; Tiptap reuses the same instance across docs, so chunks for doc A land in doc B's buffer if the user navigates mid-stream. No cancellation.
- **Files**: `ui/src/components/editor/Toolbar.tsx:327-385` (stream handlers); `src-tauri/src/commands/ai.rs:62-109` (thread spawn, no registry).
- **Notes for GPT**:
  1. Stamp each stream with the originating `docId` at start.
  2. On each chunk, compare to `useProjectStore.getState().activeDocId` — if it has changed, drop the chunk (and emit an abort signal to backend if possible).
  3. Add a backend cancellation channel: thread checks an `Arc<AtomicBool>` between chunks; frontend can signal cancel.
  4. Bonus: `editor.commands.insertContentAt(currentEnd, delta)` (append-only) instead of the current O(n²) re-select/re-insert pattern.
- **Approach**: frontend stamps stream requests with the active project/doc/flow context and refuses to apply chunks after navigation; backend now tracks request ids in a Tauri-managed cancellation registry and streaming loops check an `Arc<AtomicBool>` between chunks. The UI sends `cancel_ai_transform_stream` as soon as context changes, and pre-registration cancellation is remembered to avoid a lost-cancel race.
- **Tests added**: Tauri unit tests for registered cancellation and pre-registration cancellation. UI verified by lint/build.

---

## HIGH

### H-1: Reader "repair" persists data loss on transient missing files `[~]`
- **What**: `read_project` calls `repair_project`; if files were missing (e.g. transient network share, antivirus quarantine, sync conflict), missing docs are removed from `project.documents` AND the repaired project is **written back to disk**. The user's docs are lost from `project.yaml` even if the files come back online.
- **Branch**: `fix/h-1-reader-repair`
- **Files**: `crates/core/src/core/project/reader.rs:228, 232-244, 353-373, 419-428`.
- **Notes for GPT**: Two acceptable shapes:
  - (a) Repair in-memory only; never write back. Surface a structured warning (`ChiknError::ProjectRepaired { missing: Vec<String> }`?) so the UI shows a banner.
  - (b) Write only the additive repairs (created folders, orphan adoption), never the destructive ones (missing-file pruning).
  - Whichever, also fix `read_threads(...).unwrap_or_default()` at reader.rs:228 — parse failure should surface, not silently default. Same for `writing_history.json` at `src-tauri/src/commands/io.rs:423`.
- **Approach**: read repair no longer writes `project.yaml` during load and no longer prunes missing hierarchy/document references; missing files are logged and kept in memory. Corrupt `threads.yaml` and `writing-history.json` now return parse errors instead of defaulting to empty before the next save/write.
- **Tests added**: reader tests for preserving missing-file hierarchy references and project.yaml contents; corrupt `threads.yaml` load failure; Rust command tests cover writing-history parse rejection through the Tauri package.

### H-2: Windows `RestoreRevision` hard-resets history `[~]`
- **What**: `repo.Reset(ResetMode.Hard, commit)` destroys uncommitted work AND moves HEAD destructively. Rust uses `checkout_tree` + forward `save_revision` (preserves history). Cross-frontend divergence in the highest-stakes operation.
- **Files**: `windows/ChickenScratch.Core/Git/GitService.cs:43`.
- **Approach**: _(GPT to fill in)_
- **Tests added**: _(GPT to fill in)_
- **Reviewer comments**:

### H-3: Destructive git ops lack dirty-worktree guards `[~]`
- **What**: `restore_revision` (`git.rs:276-278`) uses `CheckoutBuilder::new().force()` with no dirty check. `sync_pull_force` (`git.rs:797`) does `reset HARD`. Auto-save model means there's always uncommitted state — these ops silently nuke 0–2 seconds of typing.
- **Files**: `crates/core/src/core/git.rs:276-280, 797`.
- **Approach**: _(GPT to fill in)_
- **Tests added**: _(GPT to fill in)_
- **Reviewer comments**: After fix, confirm UI surfaces the "dirty worktree" rejection clearly so the user knows to save first.

### H-4: AI bearer token to unvalidated/HTTP endpoints `[~]`
- **What**: `ai.rs:275-295` accepts `http://` for OpenAI endpoint. No scheme check, no host normalization. `https://api.openai.com.attacker.tld` accepted. Bearer token (line 292) goes wherever.
- **Files**: `src-tauri/src/commands/ai.rs:275-295, 461-477`.
- **Notes for GPT**: At settings save-time, parse the URL with `url::Url`; require `scheme() == "https"`; reject embedded userinfo (`url.username()` non-empty / `url.password().is_some()`). For Ollama (which is local-only), allow `http://localhost` and `http://127.0.0.1` explicitly.
- **Approach**: OpenAI request construction now parses and normalizes the endpoint before attaching Authorization, rejects HTTP including loopback, embedded credentials, query strings, fragments, missing hosts, and malformed URLs. App settings save validates OpenAI endpoints with the same HTTPS-only policy while leaving Ollama local HTTP allowed.
- **Tests added**: Tauri command tests for OpenAI HTTPS default, malformed URL rejection, HTTP cloud rejection, HTTP loopback rejection, settings-save OpenAI HTTP rejection, settings-save HTTPS acceptance, and Ollama local HTTP acceptance.

### H-5: Plaintext API keys + git tokens in settings.json `[ ]`
- **What**: `RemoteSettings.token`, `AiSettings.api_key` written to `dirs::config_dir()/chickenscratch/settings.json` at default permissions.
- **Files**: `src-tauri/src/commands/settings.rs:121-149, 266-270`.
- **Notes for GPT**: Use the `keyring` crate. Store under service `chickenscratch.ai.api_key.{provider}` and `chickenscratch.remote.token.{remote_name}`. Leave a reference (e.g. `{"api_key_in_keyring": true, "provider": "anthropic"}`) in settings.json so the UI can still show "configured / not configured" without round-tripping the secret on every read.

### H-6: Cross-frontend test is a misnomer `[ ]`
- **What**: `crates/core/tests/cross_frontend_round_trip.rs` only exercises the Rust reader against hand-crafted YAML. Doesn't invoke Swift writer or C# writer. The whole F-001..F-018 class of bugs slipped through because of this.
- **Notes for GPT**: Build a shell/Python harness in `crates/core/tests/cross_frontend/` (or as a separate `xtask`) that:
  1. Runs `chikn-converter` to produce a fixture .chikn.
  2. Runs `swift run` against a small Swift harness in `macos/Tests/` that opens, mutates, and re-saves it.
  3. Runs `dotnet run` against a small C# harness in `windows/ChickenScratch.Core.Tests/` that does the same.
  4. After each frontend round-trip, diff `project.yaml` and `.meta` files byte-for-byte against a frozen golden fixture.
  This is significant work — discuss with reviewer first if the scope feels too large for one PR.

### H-7: Stale-disk-state on restore/compile/file-history `[~]`
- **What**: `DocumentHistory.tsx:46` restores active doc, but the editor keeps its dirty buffer and the next debounced save silently undoes the restore. `CompileDialog.tsx:49` reads disk directly with unsaved edits not persisted.
- **Files**: `ui/src/components/revisions/DocumentHistory.tsx:46`, `ui/src/components/compile/CompileDialog.tsx:49`.
- **Approach**: _(GPT to fill in)_
- **Tests added**: _(GPT to fill in)_
- **Reviewer comments**: After flush-before-restore, also force the editor to `setContent` with the restored content (don't rely on `selectDocument(sameId)` to re-trigger the load effect — it won't, because `docIdRef.current === activeDoc.id`).

---

## MEDIUM

### M-1: `ChiknError::Unknown` swallows all git errors `[ ]`
- **What**: 128/128 git error sites collapse into stringly-typed `Unknown(format!(...))`. UI can't branch on auth-vs-conflict-vs-no-remote-vs-not-fast-forward.
- **Files**: `crates/core/src/utils/error.rs:10`; all of `crates/core/src/core/git.rs`.
- **Notes for GPT**: Add `ChiknError::Git(GitError)` with a sub-enum: `Auth, Conflict, NotFastForward, NoUpstream, NoCommits, NotARepo, Other(String)`. Map `git2::ErrorCode` and `git2::ErrorClass` into these. Frontend can then branch on `result.code === "Git.Auth"` and show the right toast.

### M-2: Corrupt sidecars silently overwritten `[~]`
- **What**: `reader.rs:228` `read_threads(...).unwrap_or_default()` — one corrupt `threads.yaml` and the next save erases every thread. Same shape at `writer.rs:285` (swallowed `.meta` parse), `src-tauri/src/commands/io.rs:423` (writing_history wipe).
- **Notes for GPT**: Quarantine the corrupt file (rename to `.corrupt-<timestamp>`) before defaulting, and emit a warning so the user sees a banner. Pair with H-1.
- **Approach**: Corrupt `threads.yaml`, document `.meta`, and `writing_history.json` now fail loudly instead of defaulting and overwriting sidecar state. This does not yet add quarantine/warning UI.
- **Tests**: `cargo test -p chickenscratch-core corrupt --lib`; `cargo test -p chickenscratch writing_history --bins`

### M-3: Pandoc subprocesses have no timeout `[~]`
- **What**: `Command::new("pandoc").output()` blocks forever if pandoc hangs.
- **Files**: `crates/core/src/core/compile.rs:172`, `crates/core/src/scrivener/parser/rtf.rs:23`, `src-tauri/src/commands/io.rs:166`.
- **Notes for GPT**: Use `wait_timeout` crate, or spawn + `child.wait_timeout(Duration::from_secs(60))` + kill on expiry. Add a max-bytes guard on stdout (~50 MB) too.
- **Approach**: Added a shared process runner with a 60-second timeout, kill-on-expiry, and a 50 MB stdout/stderr cap. Routed compile, Scrivener RTF conversion, import conversion, pandoc discovery, and the settings pandoc check through it.
- **Tests**: `cargo test -p chickenscratch-core process --lib`; `cargo check -p chickenscratch`

### M-4: Tauri CSP disabled + `shell:open` unconstrained `[~]`
- **What**: `tauri.conf.json:22` `csp: null`. `tauri.conf.json:36-38` `shell.open: true` with no validator regex. Any renderer injection chains to OS-handler code-exec.
- **Files**: `src-tauri/tauri.conf.json:21-22, 36-38`; `src-tauri/capabilities/default.json:11`.
- **Notes for GPT**: CSP: start with `"csp": "default-src 'self'; img-src 'self' data: asset: https://asset.localhost; style-src 'self' 'unsafe-inline'; script-src 'self'"` and tighten. `shell.open`: change to a URL-scheme regex like `"open": "^https?://"`.
- **Approach**: Added a production CSP plus a dev CSP that keeps Tauri IPC and the Vite dev server reachable. Switched the shell plugin to an HTTPS-only validator and replaced the unscoped capability command with `shell:default`.
- **Tests**: `cargo check -p chickenscratch`

### M-5: `simple_word_diff` O(m·n) without sane bail-out `[~]`
- **What**: `git.rs:973-1033` builds `vec![vec![0u32; n+1]; m+1]` LCS table. Cap is 5000 words → up to 100 MB allocation per call from the revisions UI.
- **Notes for GPT**: Either drop the cap to ~1500 words, or replace with a streaming diff (e.g. `similar` crate). Render a "diff too large" placeholder above the cap.
- **Approach**: Added an LCS matrix cell cap of `1_500 * 1_500`; larger requests now return the existing coarse deleted/added diff without building the table.
- **Tests**: `cargo test -p chickenscratch-core simple_word_diff --lib`

### M-6: `beforeunload` flush is best-effort `[~]`
- **What**: `App.tsx:148` awaits `flushPendingEditorSave()` in `beforeunload`, but `beforeunload` cannot block on real promises across the webview boundary.
- **Notes for GPT**: Switch to Tauri v2's `onCloseRequested` (`@tauri-apps/api/window`) to actually defer close until the save resolves.
- **Approach**: Replaced the Tauri close path with `getCurrentWindow().onCloseRequested`, awaiting the pending editor flush and canceling close if that flush fails; browser `beforeunload` remains only as a fallback outside Tauri.
- **Tests**: `npm run lint`; `npm run build`

---

## LOW

### L-1: Binder re-renders on every editor save `[ ]`
- **What**: Binder subscribes to whole `project`. Every 2s autosave rebuilds `project.documents` → full Binder tree + EntitySections + CommandPalette `flattenDocNames` re-runs.
- **Notes for GPT**: Switch to per-slice selectors with `useShallow`; specifically `(s) => s.project?.hierarchy` and `(s) => s.project?.documents` separately.

### L-2: Bundle is ~890 KB, no code splitting `[ ]`
- **Notes for GPT**: `vite.config.ts` add `build.rollupOptions.output.manualChunks: { tiptap: [...], prosemirror: [...] }`; `React.lazy` the rarely-mounted modals (Timeline, Preview, Compile, Stats, Comments).

### L-3: Modals not real modals — no `role="dialog"`, no focus trap `[ ]`
- **Files**: `ui/src/components/shared/Dialog.tsx`, `compile/CompileDialog.tsx`, `command-palette/CommandPalette.tsx`, `settings/Settings.tsx`, `comments/CommentsPanel.tsx`.

### L-4: Binder is not a keyboard tree `[ ]`
- **Files**: `ui/src/components/binder/Binder.tsx`. No `role="tree"`/`role="treeitem"`, no arrow-key nav, no `aria-expanded`/`aria-selected`.

### L-5: Swift uses literal folder IDs vs Rust UUIDs `[ ]`
- **What**: `macos/Sources/ChiknKit/Writer.swift:24-27` writes `TreeNode(id: "manuscript", ...)`; Rust writer.rs:147-160 writes `uuid::Uuid::new_v4()`. No exploit today but a footgun if any code hardcodes the id shape.
- **Notes for GPT**: Align Swift to UUIDs.

### L-6: Pandoc resolved via $PATH `[ ]`
- **Files**: `src-tauri/src/commands/io.rs:188-222`. Hijackable if a writable dir is ahead of `/usr/local/bin`.
- **Notes for GPT**: Document the requirement in `docs/USER_GUIDE.md` and prefer absolute paths in the candidates list.

### L-7: AI SSE streams no max-bytes guard `[ ]`
- **Files**: `src-tauri/src/commands/ai.rs:170-188, 228-260, 302-323`. Malicious endpoint = unbounded memory.

### L-8: `linux/qml/SettingsDialog.qml` exposes AI tab without backing invokable `[ ]`
- **Files**: `linux/qml/SettingsDialog.qml:78, 233`. Fake-tab footgun.

### L-9: Pandoc stdout unbounded buffer on import `[ ]`
- **Files**: `src-tauri/src/commands/io.rs:166-185, 254-323`. Cap at 50 MB.

---

## Recently landed (awaiting reviewer verification)

_GPT: add commit SHA + short summary here when you commit. Reviewer will scan and update statuses above._

### Review pass 1 (initial cycle, no commits yet)

**WIP detected, uncommitted** — 26 modified files spanning C-1, C-2, C-4, H-2, H-3, H-7 and likely more. The work has not been committed, so it cannot be graded yet. Please commit in coherent slices (one finding id per commit) so I can verify them individually on the next cycle.

**Baseline checks on the WIP tree** (clean ≠ verified — just confirms it isn't broken):
- `cargo build --workspace --exclude chickenscratch-linux`: ✅
- `cargo clippy --all-targets`: ⚠️ 1 new warning — `count_hierarchy_docs` is unused at `crates/core/src/core/project/reader.rs:518`. Either wire it in or delete it before commit.
- `cargo test`: ✅ 70 passed (up from 50 — thank you for the new tests).
- `npm run lint`: ✅
- `npm run build`: not run this cycle.

**Quick scan of WIP file→finding mapping** (for your tracking):
- C-1/C-2 → `crates/core/src/scrivener/parser/{scrivx.rs, mod.rs}`, `converter/mod.rs`, `exporter/mod.rs`
- C-4 → `src-tauri/src/commands/{mod.rs, main.rs, document.rs, project.rs, threads.rs, git.rs, io.rs}`
- H-2 → `windows/ChickenScratch.Core/Git/GitService.cs`
- H-3 → `crates/core/src/core/git.rs`
- H-7 → `ui/src/components/compile/CompileDialog.tsx`, `revisions/DocumentHistory.tsx`
- C-5 → `ui/src/commands/ai.ts`, `components/editor/Toolbar.tsx`
- Other (reader/writer/models, tui/app.rs, tui/ui.rs, linux/bridge.rs, cross_frontend_round_trip.rs) — please clarify in commit messages which finding(s) these address, or whether they're follow-on cleanup.

Once you commit, each commit will get a dedicated subagent review on the next cycle.

### Review pass 2 (still no commits)

**WIP expanded to 30 files** (cycle 1 was 26). REVIEW.md self-updated by GPT with approach + tests sections filled in for:
- **C-3 (symlink)** — looks comprehensive on paper: component-based path validation, canonical root check, symlink rejection for parent / doc / `.meta` on both write and delete, with 6 new writer tests.
- **H-1 (reader repair)** — read repair now in-memory only; missing files logged not pruned; corrupt `threads.yaml` and `writing-history.json` now surface errors instead of defaulting.
- **H-4 (AI endpoint)** — endpoint normalized + HTTPS-required for OpenAI cloud; HTTP allowed only for localhost/127.0.0.1 (Ollama); 7 new tests.

**New WIP files since cycle 1**: `src-tauri/src/commands/settings.rs` (H-4), `src-tauri/src/commands/templates.rs` (likely C-4 lock spread), `ui/src/components/editor/editorRef.ts` (likely C-5).

**Validation**: caught one transient `unexpected closing delimiter` in `chickenscratch` test bin mid-cycle — cleared on retry; signals GPT was actively editing, no real defect. End-state: clippy 0 warnings, eslint clean, tests **70 (core) + 16 (cli) + 3 + 2 = 91 passing**. No regressions.

**Reviewer ask**: please commit **C-3, H-1, H-4** now — they look complete and I want to verify them in isolation before the next layer lands on top. The longer WIP accumulates, the harder it gets to localize any regression I find.

### Review pass 3 (still no commits)

**WIP at 37 files** (+7 since cycle 2). Validation clean: clippy 0 warnings, eslint clean, tests **96 passing** (75 core, 16 cli, +5 from other crates — five new tests since cycle 2).

New surface area noted (still WIP, not graded):
- **M-3 (pandoc timeout)**: new module `crates/core/src/utils/process.rs` plus changes to `compile.rs`, `scrivener/parser/rtf.rs`, `utils/mod.rs`. Looks like the bounded-subprocess wrapper landed.
- **M-4 (CSP + shell.open)**: `src-tauri/tauri.conf.json` and `src-tauri/capabilities/default.json` modified.
- **M-6 (`onCloseRequested`)**: `ui/src/App.tsx` switched to `getCurrentWindow().onCloseRequested(...)` per the prior note.
- **Extra**: `ui/src/components/preview/Preview.tsx` now uses `DOMPurify.sanitize` on rendered markdown. Good defensive hardening — surface it under a new finding (Preview XSS) when you commit so I can grade it.

**Reviewer ask** (repeated and elevated): the WIP is now 37 files spanning 10+ findings. **Please commit before doing more work** — even as separate sequential commits (one per finding) it would dramatically improve verifiability. Right now if a regression shows up, I can't tell which fix caused it.

**Workflow change proposed**: I'm recommending switching to event-driven wakeups (Monitor on git HEAD) plus branch-per-finding so commits become atomic verification units. Awaiting reviewer-side go-ahead from the human before implementing.

---

## Open questions for the reviewer

_GPT: anything you're uncertain about, drop here instead of guessing._

- _(none yet)_

---

## Out-of-scope for this cycle

The architecture finding from the prior synthesis — **triple-implementation of the .chikn format (Rust + Swift + C#)** — is the dominant long-term liability but is not in this fix cycle. H-6 (real cross-frontend test) buys time; collapsing onto a single core via UniFFI/cbindgen is the longer arc. Track separately.

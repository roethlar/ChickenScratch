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

### C-1: Scrivener importer — UUID path traversal `[x]`
- **What**: `BinderItem.uuid` is raw XML data; `Path::join(uuid)` with an absolute or `..`-bearing UUID reads arbitrary host files into the imported project as document content. Arbitrary-read primitive.
- **Branch**: `fix/c-1-scrivener-uuid-validation`
- **Files**: `crates/core/src/scrivener/parser/scrivx.rs:50` (uuid field), `:181-196` (`get_rtf_path`/`get_media_path`); `crates/core/src/scrivener/converter/mod.rs:381` (callsite).
- **Approach**: Added strict hyphenated UUID validation before `get_rtf_path`/`get_media_path` join `Files/Data/<uuid>/...`; absolute paths, parent dirs, prefixes, separators, empty values, and non-UUID-like values now return `ChiknError::InvalidFormat`. `converter/mod.rs` was touched only to propagate those fallible path helper errors and to exercise rejection through the real importer.
- **Tests added**: Parser helper tests for valid sample UUIDs plus absolute/parent-dir/media UUID rejection; importer tests proving absolute and parent-dir UUIDs fail before resolving/writing escaped content.
- **Reviewer verdict**: VERIFIED (commit `be9a297`). Validator at `scrivx.rs:181-208` length-checks `== 36` + per-position hex/hyphen (8-4-4-4-12) + second-pass `Path::components()` rejection of `Prefix`/`RootDir`/`ParentDir`. Four `get_rtf_path`/`get_media_path` callsites in `converter/mod.rs:181, 321, 383, 443` all use `?`; no raw `item.uuid` reaches `Path::join`. Importer-level hostile-fixture tests at `converter/mod.rs:681, 713` call full `import_scriv` and assert `Err(ChiknError::InvalidFormat(_))` via `matches!` (not `is_err`); absolute-path test seeds a real file at `host_path` to prove no read occurred. Sample-fixture regression `test_import_corn_scriv_sample` passes. Error message echoes the UUID via `{:?}` but the validator runs before any filesystem access, so no "file exists" oracle. C-2 scope clean — `file_extension` remains raw at converter changes are pure error propagation. **Minor nit**: no dedicated test for null-byte/non-hex/35-or-37-char inputs, though the validator's `len == 36` + `is_ascii_hexdigit` chains provably reject them via the same code path tested with the absolute-path/parent-dir cases. Add explicit tests on the next pass if you want spec lock.

### C-2: Scrivener importer — `<FileExtension>` sanitization `[x]`
- **What**: User-controlled `ext` interpolated into destination paths for media copy. `write_project` currently rejects the resulting `..` path, so the import aborts — not an escape primitive today, but a malformed-import / partial-write footgun. Treating as CRITICAL because it's adjacent to C-1 and both should land together.
- **Files**: `crates/core/src/scrivener/converter/mod.rs:450-491`; `crates/core/src/scrivener/parser/scrivx.rs` extension field.
- **Branch**: `fix/c-2-scrivener-extension-sanitization`
- **Approach**: Added `sanitize_file_extension` for Scrivener media extensions. It trims whitespace, normalizes a single leading dot, rejects empty values, traversal/separators/absolute-or-prefix-ish forms, shell/path punctuation, multi-component extensions, and overlong extensions. Converter validates media extensions before creating the output project and uses the sanitized extension for both `content.<ext>` source paths and imported media destination paths.
- **Tests added**: Parser/helper tests for common extensions, whitespace/leading-dot normalization, and malicious values; importer tests for copying a normal PDF media item and rejecting `../md` before output project creation; sample Scrivener import regression remains passing.
- **Reviewer verdict**: VERIFIED (commit `cf15459`). Sanitizer at `scrivx.rs:238-261`: trim → leading-dot normalize → 32-char max → `Path::components()` rejection of separators/`..`/absolute/UNC → ASCII alphanumeric (closes shell metachars, spaces, quotes, null bytes, non-ASCII, multi-dot). `validate_media_file_extensions` runs at `converter/mod.rs:78` BEFORE `writer::create_project` at `:81` (recursive walker covers nested folders); pre-write rejection test at `:802` asserts `!output_path.exists()` so no partial state. All 3 callsites use sanitized value (UUID map at `:220`, media source at `:445`, dest filename at `:456`); defense-in-depth re-validation inside `parser/scrivx.rs:228` `get_media_path`. C-1 surface preserved — `get_media_path` validates UUID first then sanitizes ext, no new path-build site bypasses either. C-3 component check still in path. Sample-fixture regression `test_import_corn_scriv_sample` passes. **Minor**: `tar.gz` rejection is the documented Known Gap and acceptable for Scrivener's single-component `content.<ext>` layout.

### C-3: Symlink writes outside project root `[x]`
- **What**: `writer.rs:253` only checks string traversal (`..`) and absolute paths. Symlinks aren't checked. A hostile project whose `manuscript/chapter.md` is a symlink to `~/.ssh/authorized_keys` will be overwritten on next save. Same vector on delete.
- **Branch**: `fix/c-3-symlink-writes`
- **Files**: `crates/core/src/core/project/writer.rs:253` (write), `:330-360` (delete).
- **Notes for GPT**: Use `std::fs::symlink_metadata(...).file_type().is_symlink()` on the full path before write. Reject with `ChiknError::InvalidFormat`. Also harden the traversal check to component-based: `Path::components().any(|c| matches!(c, Component::ParentDir | Component::RootDir | Component::Prefix(_)))` is more robust than `contains("..")`.
- **Tests**: write_project should reject a project containing a symlinked doc; delete_document should refuse to remove a symlink. Add to `crates/core/src/core/project/writer.rs` test module.
- **Approach**: added component-based document path validation, canonical project-root checks, safe directory creation, symlink rejection for parent directories, document files, and `.meta` targets before write/delete.
- **Tests added**: writer tests for parent traversal, dot-dot filename allowance, symlink parent rejection, symlink document write rejection, symlink parent delete rejection, and symlink document delete rejection.
- **Reviewer verdict**: VERIFIED (commit `1333a93`). Component-based check at `writer.rs:293-331`, symlink rejection covers leaf + parent chain + `.meta` for both write and delete (`writer.rs:268-287, 506-519, 593-624`). Fail-closed before `project.yaml` rewrite. 6 new tests assert on `Err` with side-effect checks. **Minor nit (non-blocking)**: `canonical_project_root` is recomputed multiple times per save (`writer.rs:204, 249, 361, 505, 589`) — thread it through once if you touch this code again. Unavoidable TOCTOU residual between symlink check and `fs::write` documented; out of scope for desktop threat model.

### C-4: No write-lock on read-modify-write `[x]`
- **What**: Concurrent Tauri command invocations (auto-save + auto-commit + backup) interleave `read_project → mutate → write_project` and silently lose work.
- **Files**: `src-tauri/src/commands/document.rs`, `git.rs`, `project.rs`, `io.rs`, `threads.rs` — every command that mutates.
- **Approach**: Added process-local `ProjectWriteLocks` Tauri state keyed by normalized project path and registered it in `main.rs`. Wrapped project-disk mutating document, project, thread, template, import/writing-history, and mutating git/backup/sync/restore commands. Pure reads, app settings/keyring writes, AI text generation, and compile external-output generation remain unlocked.
- **Tests added**: Unit tests for same-project serialization and different-project independence in `src-tauri/src/commands/mod.rs`.
- **Touched files**: `src-tauri/src/commands/{mod.rs,document.rs,project.rs,threads.rs,templates.rs,io.rs,git.rs}`, `src-tauri/src/main.rs`, `.review/findings/C-4.md`, `REVIEW.md`.
- **Known gaps**: Command-level concurrent integration tests would require a full Tauri harness; helper tests cover lock behavior directly. Locks are process-local and do not coordinate separate app instances or external tools.
- **Reviewer verdict**: VERIFIED (commit `6151be8`). Two-tier `Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>` with the outer map's lock dropped before the inner per-project lock is acquired (`mod.rs:32-36`) — different projects don't block each other. Every `writer::write_project` callsite (21 sites across document/project/threads/templates/io) is inside `with_project_lock`. The single `fs::write` against a project path (`io.rs:488`) is locked at `:451`. All git mutators wrapped — including `backup_on_close` (`git.rs:251`) which closes the M-6 close-flush race. Reads (`reader::read_project`) are inside the lock closure, not before — read-mutate-write is fully serialized. No re-entrancy: command bodies call only core lib functions, never other Tauri commands. Out-of-scope list verified accurate (none of the unlocked commands touch project disk). Two new tests in `mod.rs:59,101` use real threads + timing assertions (100ms / 1s) to prove serialization on same project and independence across projects. **Minor non-blocking**: `project_lock_key` returns a non-canonical PathBuf for projects that don't exist yet (e.g. mid-`create_project`), so concurrent `create_project` calls with `~/foo` vs `/Users/foo` paths could race. Single-shot project creation; pathological.

### C-5: AI streaming writes to wrong document after navigation `[x]`
- **What**: `Toolbar.tsx` closure captures `editor` at stream start; Tiptap reuses the same instance across docs, so chunks for doc A land in doc B's buffer if the user navigates mid-stream. No cancellation.
- **Branch**: `fix/c-5-ai-stream-doc-routing`
- **Files**: `ui/src/components/editor/Toolbar.tsx:327-385` (stream handlers); `src-tauri/src/commands/ai.rs:62-109` (thread spawn, no registry).
- **Notes for GPT**:
  1. Stamp each stream with the originating `docId` at start.
  2. On each chunk, compare to `useProjectStore.getState().activeDocId` — if it has changed, drop the chunk (and emit an abort signal to backend if possible).
  3. Add a backend cancellation channel: thread checks an `Arc<AtomicBool>` between chunks; frontend can signal cancel.
  4. Bonus: `editor.commands.insertContentAt(currentEnd, delta)` (append-only) instead of the current O(n²) re-select/re-insert pattern.
- **Approach**: frontend stamps each stream with the active project path plus a context key (`doc:<activeDocId>` or `flow:<ordered doc ids>`), verifies the live store context before every chunk/done/error handler, and cancels/aborts the stream when navigation changes that context. Backend tracks request ids in a Tauri-managed cancellation registry and provider streaming loops check an `Arc<AtomicBool>` before work, between reads, and before chunk emission.
- **Tests added**: Tauri unit tests for registered cancellation and pre-registration cancellation. UI verified by lint/build; no UI stream navigation harness exists.
- **Touched files**: `src-tauri/src/commands/ai.rs`, `src-tauri/src/main.rs`, `ui/src/commands/ai.ts`, `ui/src/components/editor/Toolbar.tsx`, `.review/findings/C-5.md`, `REVIEW.md`.
- **Reviewer verdict**: VERIFIED (commit `831593d`). Registry is `Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>` (`ai.rs:41-82`). Pre-registration race handled: `register()` preserves a flag set earlier by `cancel` (`:47-57`); test `ai_stream_registry_remembers_pre_registration_cancellation` (`:682`) asserts this. Cooperative checks at three points in each provider (Ollama `:213-272`, Anthropic `:274-351`, OpenAI `:353-422`): pre-work, between SSE lines, before emit; terminal done/error also gated (`:161`). Frontend uses UUID per stream (`Toolbar.tsx:360`) + `doc:<id>`/`flow:<ids>` context key (`:333-339`); `shouldHandleEvent` (`ai.ts:86-89`) checks live store + calls backend `cancel_ai_transform_stream` so the thread stops (not just local drop). H-4 bearer gate preserved — both attach points (`ai.rs:383, 570`) downstream of `openai_chat_completions_url` validator. Bonus: append-only `insertContentAt` (`Toolbar.tsx:405, 429`). Cleanup via `unregister` on every exit path with `Arc::ptr_eq` so re-registered id isn't accidentally removed. **Minor non-blocking**: `flowDocs` join order at `Toolbar.tsx:336` is input order not sorted; two flow sessions with identical docs in different order would compare unequal keys, but Flow ordering is stable within a session and navigation OUT of flow flips the key anyway. Cooperative cancellation can leave a blocked HTTP read alive until next line / timeout — acceptable per Known Gaps; no events emit after the flag flips.

---

## HIGH

### H-1: Reader "repair" persists data loss on transient missing files `[x]`
- **What**: `read_project` calls `repair_project`; if files were missing (e.g. transient network share, antivirus quarantine, sync conflict), missing docs are removed from `project.documents` AND the repaired project is **written back to disk**. The user's docs are lost from `project.yaml` even if the files come back online.
- **Branch**: `fix/h-1-reader-repair`
- **Files**: `crates/core/src/core/project/reader.rs:228, 232-244, 353-373, 419-428`.
- **Notes for GPT**: Two acceptable shapes:
  - (a) Repair in-memory only; never write back. Surface a structured warning (`ChiknError::ProjectRepaired { missing: Vec<String> }`?) so the UI shows a banner.
  - (b) Write only the additive repairs (created folders, orphan adoption), never the destructive ones (missing-file pruning).
  - Whichever, also fix `read_threads(...).unwrap_or_default()` at reader.rs:228 — parse failure should surface, not silently default. Same for `writing_history.json` at `src-tauri/src/commands/io.rs:423`.
- **Approach**: read repair no longer writes `project.yaml` during load and no longer prunes missing hierarchy/document references; missing files are logged and kept in memory. Corrupt `threads.yaml` and `writing-history.json` now return parse errors instead of defaulting to empty before the next save/write.
- **Tests added**: reader tests for preserving missing-file hierarchy references and project.yaml contents; corrupt `threads.yaml` load failure; Rust command tests cover writing-history parse rejection through the Tauri package.
- **Reviewer verdict**: VERIFIED (commit `f2d8897`). Write-back removed (`reader.rs:194-241`); missing refs preserved in memory (`reader.rs:381-497` only logs); additive folder repair retained through symlink-safe helper. `read_threads(path)?` replaces the old `unwrap_or_default` (`reader.rs:228`). Tests assert hierarchy retention AND `project.yaml` byte-equality before/after read. **Non-blocking nits**: (a) `create_required_folder_if_safe`/`ensure_required_folder_safe` (`reader.rs:306-367`) duplicate C-3's helper in `writer.rs:365+` — extract to a shared module on the next pass. (b) Missing-file warnings only reach stderr (`eprintln!`); no structured signal for the UI to surface a banner — open a separate finding if you want that. (c) `test_threads_corrupt_file_fails_load` only asserts `is_err()`, not the variant.

### H-2: Windows `RestoreRevision` hard-resets history `[x]`
- **What**: `repo.Reset(ResetMode.Hard, commit)` destroys uncommitted work AND moves HEAD destructively. Rust uses `checkout_tree` + forward `save_revision` (preserves history). Cross-frontend divergence in the highest-stakes operation.
- **Files**: `windows/ChickenScratch.Core/Git/GitService.cs:43`.
- **Approach**: replaced hard reset with target-tree checkout, stage-all, and a new restore commit from the current HEAD so history moves forward and the restored commit's parent is the pre-restore HEAD.
- **Tests added**: `windows/ChickenScratch.Core.Tests/GitServiceRestoreHarness` creates two revisions, restores the first, and asserts HEAD is a new commit whose parent is the previous HEAD and whose tree/worktree match the target revision, including deletion of files added later.
- **Reviewer verdict**: VERIFIED (commit `ae7f6b6`). `GitService.cs:37-53` now uses `repo.Checkout(commit.Tree, ...)` + `Commands.Stage` + `repo.Commit(...)`; no `Reset` anywhere in file (grep confirms). Parent of restore commit is the **previous HEAD**, not the target — confirmed by harness assertion at `Program.cs:33`. Cross-frontend semantics match Rust's `restore_revision`. **Minor drift (non-blocking)**: Rust uses `commit.message()` (full body) while Windows uses `commit.MessageShort` (subject only) at `GitService.cs:50` — identical for single-line restore messages but would diverge for multi-line; worth a follow-up parity finding if you care. Windows side lacks an xUnit/NUnit framework; the harness is the test. `dotnet build` passed on this macOS host; harness `GitServiceRestoreHarness: passed`.

### H-3: Destructive git ops lack dirty-worktree guards `[x]`
- **What**: `restore_revision` (`git.rs:276-278`) uses `CheckoutBuilder::new().force()` with no dirty check. `sync_pull_force` (`git.rs:797`) does `reset HARD`. Auto-save model means there's always uncommitted state — these ops silently nuke 0–2 seconds of typing.
- **Files**: `crates/core/src/core/git.rs:276-280, 797`.
- **Approach**: added a shared git status helper and reject dirty worktrees before `restore_revision` force-checkout and `sync_pull_force` hard reset; the current taxonomy has no dirty-specific kind, so rejection uses `GitErrorKind::Conflict` with a save/discard action message.
- **Tests added**: restore dirty rejection without clobbering, clean restore still creates a forward commit, force-pull dirty rejection without clobbering, and clean force-pull overwrite behavior in `crates/core/tests/remote_sync.rs`.
- **Reviewer verdict**: VERIFIED (commit `edf0f82`). `repo_has_changes` + `reject_dirty_worktree` are wired into `restore_revision` and `sync_pull_force` before mutation. `include_untracked` and `recurse_untracked_dirs` are both set, and tests assert dirty restore/force-pull reject without clobbering. Follow-ups noted separately for Windows restore and draft-switch dirty guards.

### H-4: AI bearer token to unvalidated/HTTP endpoints `[x]`
- **What**: `ai.rs:275-295` accepts `http://` for OpenAI endpoint. No scheme check, no host normalization. `https://api.openai.com.attacker.tld` accepted. Bearer token (line 292) goes wherever.
- **Files**: `src-tauri/src/commands/ai.rs:275-295, 461-477`.
- **Branch**: `fix/h-4-openai-endpoint-validation`
- **Notes for GPT**: At settings save-time, parse the URL with `url::Url`; require `scheme() == "https"`; reject embedded userinfo (`url.username()` non-empty / `url.password().is_some()`). For Ollama (which is local-only), allow `http://localhost` and `http://127.0.0.1` explicitly.
- **Approach**: OpenAI request construction now parses and normalizes the endpoint before attaching Authorization, rejects HTTP including loopback, embedded credentials, query strings, fragments, missing hosts, and malformed URLs. App settings save validates OpenAI endpoints with the same HTTPS-only policy while leaving Ollama local HTTP allowed.
- **Tests added**: Tauri command tests for OpenAI HTTPS default, malformed URL rejection, HTTP cloud rejection, HTTP loopback rejection, settings-save OpenAI HTTP rejection, settings-save HTTPS acceptance, and Ollama local HTTP acceptance.
- **Reviewer verdict**: VERIFIED (commit `c52ea4a`). Single helper `openai_chat_completions_url` (`ai.rs:492`) is the only entry; called by `call_openai` (`ai.rs:460`), `stream_openai` (`ai.rs:277`), and `validate_app_settings` (`settings.rs:279`). Positive `scheme() == "https"` check + userinfo/query/fragment/hostless rejection. Bearer-attach paths grep to exactly two sites — both downstream of the validator, no bypass. Ollama exemption is intentional and credential-less so the residual settings-poisoning concern is closed under H-5. **Non-blocking nits**: (a) helper doc-comment should explain the path-append semantics for OpenAI-compat gateways; (b) add a test for the already-complete-path short-circuit branch (`ai.rs:503`); (c) consider an explicit reject for IP-literal hosts shaped like `https://203.0.113.1` if you ever want to enforce DNS hostnames only — currently allowed.

### H-5: Plaintext API keys + git tokens in settings.json `[x]`
- **What**: `RemoteSettings.token`, `AiSettings.api_key` written to `dirs::config_dir()/chickenscratch/settings.json` at default permissions.
- **Files**: `src-tauri/src/commands/settings.rs:121-149, 266-270`.
- **Branch**: `fix/h-5-keyring-settings-secrets`
- **Notes for GPT**: Use the `keyring` crate. Store under service `chickenscratch.ai.api_key.{provider}` and `chickenscratch.remote.token.{remote_name}`. Leave a reference (e.g. `{"api_key_in_keyring": true, "provider": "anthropic"}`) in settings.json so the UI can still show "configured / not configured" without round-tripping the secret on every read.
- **Approach**: Added provider-specific keyring storage for AI API keys (`chickenscratch.ai.api_key.{provider}`) and a `sync` keyring service for the remote token, redacted public settings reads, hydrated internal settings reads for AI/git use, migration of existing plaintext secrets after successful keyring writes, and preserve/replace/delete save semantics for redacted UI payloads.
- **Tests added**: Fake-keyring unit tests for plaintext migration and redaction, failed-migration preservation, hydrated internal reads, preserving existing secrets on unrelated saves, replacing non-empty incoming secrets, provider-specific AI key isolation, and deleting when configured flags are cleared. Validation: `cargo test -p chickenscratch commands::settings::tests -- --nocapture`; `cargo clippy -p chickenscratch --all-targets -- -D warnings`; `cargo test -p chickenscratch --bins`; `cd ui && npm run lint`; `cd ui && npm run build`.
- **Reviewer verdict**: VERIFIED (commit `a570f79`). On-disk `Settings` keeps `Option<String>` for legacy-deserialize, but every public exit (`get_app_settings:481-485`, `save_app_settings_to_path:463,475`) nulls plaintext via `redact_secrets` before `serde_json::to_string_pretty`. No `Serialize` path holds plaintext. Migration writes keyring first then nulls plaintext only on success — failure-mode tested at `:979-1011`. Provider namespacing alnum-normalized via `keyring_component`. Hydrated callsites: `ai.rs:38,70,329`, `git.rs:12,126`. UI placeholder + clear-X buttons drive the delete branch correctly. `keyring 3.6.3` with `apple-native`/`windows-native`/`linux-native-sync-persistent` enabled; no mock backend in production. 13 new tests via `FakeSecretStore` (no real keychain touched). **Non-blocking nits**: (a) single `chickenscratch.remote.token.sync` namespace covers today's single-remote model; namespace-by-URL-hash if multi-remote ships. (b) Migration errors at `load_app_settings_from_path:435-438` are swallowed via `let _ =` — if keyring is locked at app start, plaintext stays in settings.json until next save; consider logging a warning.

### H-6: Cross-frontend test is a misnomer `[x]`
- **What**: `crates/core/tests/cross_frontend_round_trip.rs` only exercises the Rust reader against hand-crafted YAML. Doesn't invoke Swift writer or C# writer. The whole F-001..F-018 class of bugs slipped through because of this.
- **Branch**: `fix/h-6-real-cross-frontend-harness`
- **Notes for GPT**: Build a shell/Python harness in `crates/core/tests/cross_frontend/` (or as a separate `xtask`) that:
  1. Runs `chikn-converter` to produce a fixture .chikn.
  2. Runs `swift run` against a small Swift harness in `macos/Tests/` that opens, mutates, and re-saves it.
  3. Runs `dotnet run` against a small C# harness in `windows/ChickenScratch.Core.Tests/` that does the same.
  4. After each frontend round-trip, diff `project.yaml` and `.meta` files byte-for-byte against a frozen golden fixture.
  This is significant work — discuss with reviewer first if the scope feels too large for one PR.
- **Approach**: added `crates/core/tests/cross_frontend/run.sh`, which converts `samples/Corn.scriv` through `chikn-converter`, runs Swift and C# writer harnesses when toolchains exist, and invokes a Rust-reader verifier after each pass. Added Swift `ChiknKitCrossFrontendHarness` and C# `ChickenScratch.Core.CrossFrontendHarness` entry points; retargeted the C# core library to `net10.0` so it builds on macOS without WinUI.
- **Tests added**: env-driven Rust verifier in `cross_frontend_round_trip.rs`; `cargo test -p chickenscratch-core --test cross_frontend_round_trip`; `cargo build -p chikn-converter`; `swift run --package-path macos ChiknKitChecks`; `dotnet build windows/ChickenScratch.Core/ChickenScratch.Core.csproj`; `crates/core/tests/cross_frontend/run.sh`.
- **Known gap**: harness emits a manifest and verifies Rust load/marker fields instead of byte-for-byte goldens because current writer passes touch volatile timestamps and can rewrite/repair metadata ordering.
- **Reviewer verdict**: VERIFIED (commit `8bd8919`). Windows `Core.csproj` retarget to `net10.0` is clean — scanned all Core/*.cs for Windows-specific imports, found none; the WinUI App still pins `net10.0-windows10.0.19041.0` and a `net10.0-windows` consumer of `net10.0` is supported. `run.sh` is bash-3.2 compatible with proper `set -euo pipefail` and consistent path quoting. Swift + C# harnesses both use public APIs, deterministic mutations, argv-driven, exit nonzero on failure. Rust verify test asserts marker field is present in at least one doc's fields (not tautological). End-to-end runnable on this host (dotnet + swift both available). Spun out `H-6-followup` below.

### H-6-followup: Cross-frontend harness — tighten skips, cleanup, drift gate `[x]`
- **What**: H-6 landed but with explicit known gaps the reviewer surfaced. Three follow-ups to close them:
  1. No `trap … EXIT` cleanup in `run.sh` — `/tmp/chikn-cross-frontend.XXXXXX` workdirs leak on every CI run.
  2. Skip messages on missing Swift/dotnet toolchains go through `log()` to stdout+manifest without an explicit `SKIPPED:` prefix or stderr emission — a CI run with neither toolchain reports `result: ok` with only the converter exercised, which is the silent-pass failure mode the original concern called out.
  3. Verify test asserts marker presence but not absence of repair logs; GPT's own H-6.md acknowledges this. Tighten with a `CHIKN_CROSS_FRONTEND_FAIL_ON_REPAIR=1` mode that greps the output for the reader's repair markers and fails if seen.
- **Files**: `crates/core/tests/cross_frontend/run.sh`, `crates/core/tests/cross_frontend_round_trip.rs`.
- **Notes for GPT**: All three are small and orthogonal. Could be one branch (`fix/h-6-followup-harness-hardening`). If 1 and 2 are trivial but 3 needs design discussion, split.
- **Approach**: added EXIT cleanup for script-created temp workdirs while preserving `CHIKN_CROSS_FRONTEND_WORKDIR`; made missing Swift/dotnet skips emit `SKIPPED:` to stderr and the manifest; added writer coverage/final-result lines so all-skipped optional writers are explicit; added `CHIKN_CROSS_FRONTEND_FAIL_ON_REPAIR=1` output capture and repair-marker failure around Rust verifier runs.
- **Tests added**: `crates/core/tests/cross_frontend/run.sh`; `CHIKN_CROSS_FRONTEND_FAIL_ON_REPAIR=1 crates/core/tests/cross_frontend/run.sh` (expected to fail while current fixture/frontend drift emits repair markers); `cargo test -p chickenscratch-core --test cross_frontend_round_trip`.
- **Touched files**: `crates/core/tests/cross_frontend/run.sh`, `crates/core/tests/cross_frontend_round_trip.rs`, `.review/findings/H-6-followup.md`, `REVIEW.md`.
- **Reviewer verdict**: VERIFIED (commit `ff894a4`). `trap EXIT` at `run.sh:25` is gated by `CLEANUP_WORKDIR` (1 for `mktemp` workdirs, 0 for user-provided `CHIKN_CROSS_FRONTEND_WORKDIR`). `SKIPPED:` emission at `:35-43` writes to both stderr and manifest with the toolchain name; final result line at `:179` is `result: ok-with-skipped-toolchains` only when at least one writer is absent. `CHIKN_CROSS_FRONTEND_FAIL_ON_REPAIR` regex at `:64-77` matches every reader.rs marker (`Repaired:`, `Repair warning:`, `pre-repair:`, etc.) — `run_and_capture` correctly preserves exit status via `set +e`/`set -e`. Bash 3.2 compatible. End-to-end run on this host: `writer-toolchains-ran:2/2`, `result: ok`. Expected-failure semantics on `FAIL_ON_REPAIR=1` with current Corn fixture is documented intent — the gate exposes existing Swift/C# writer drift without breaking the default path.

### H-7: Stale-disk-state on restore/compile/file-history `[x]`
- **What**: `DocumentHistory.tsx:46` restores active doc, but the editor keeps its dirty buffer and the next debounced save silently undoes the restore. `CompileDialog.tsx:49` reads disk directly with unsaved edits not persisted.
- **Files**: `ui/src/components/revisions/DocumentHistory.tsx:46`, `ui/src/components/compile/CompileDialog.tsx:49`.
- **Branch**: `fix/h-7-flush-before-disk-actions`.
- **Approach**: flush pending editor saves before document-history reads, document restore, and compile/export; after active-document restore reload the project through the store and force the mounted editor content to the restored markdown via a new editor-ref helper using `setContent(..., { emitUpdate: false })`.
- **Tests added**: no UI test runner exists; validated with `cd ui && npm run lint && npm run build`. Manual behavior documented in `.review/findings/H-7.md`.
- **Reviewer verdict**: VERIFIED (commit `f85e9b5`). `DocumentHistory.tsx:51-62` flushes before `restoreDocument`, then `loadProject`, then `setProject`, then (on active-doc restore only) `setCurrentEditorMarkdown(restoredDoc.content)` — exactly the reviewer's explicit ask. New `setCurrentEditorMarkdown` helper at `editorRef.ts:16-20` calls `setContent(..., { emitUpdate: false })`, null-guards `currentEditor`. `CompileDialog.tsx:50` awaits `flushPendingEditorSave()` inside the `try` block; flush failure jumps to `catch` and aborts compile with toast. **Non-blocking nits**: (a) other disk-readers (project search, document export, AI summarize/transform commands that read from disk) may need the same flush — audit on next pass. (b) `CompileDialog.tsx:24` uses `useState(() => {…})` as an effect — pre-existing misuse, unrelated to H-7. (c) No automated test coverage (no Vitest setup) — recommended follow-up.

---

## MEDIUM

### M-1: `ChiknError::Unknown` swallows all git errors `[x]`
- **What**: 128/128 git error sites collapse into stringly-typed `Unknown(format!(...))`. UI can't branch on auth-vs-conflict-vs-no-remote-vs-not-fast-forward.
- **Branch**: `fix/m-1-git-error-taxonomy`
- **Files**: `crates/core/src/utils/error.rs:10`; all of `crates/core/src/core/git.rs`.
- **Notes for GPT**: Add `ChiknError::Git(GitError)` with a sub-enum: `Auth, Conflict, NotFastForward, NoUpstream, NoCommits, NotARepo, Other(String)`. Map `git2::ErrorCode` and `git2::ErrorClass` into these. Frontend can then branch on `result.code === "Git.Auth"` and show the right toast.
- **Approach**: reused the git-specific `ChiknError::Git(GitError)` taxonomy present on current `master`, added a conservative git2 classifier in `core/git.rs`, and routed only high-value restore/current-branch/remote sync/pull/push/force-pull/merge-draft paths through it while keeping `ChiknError` string serialization unchanged.
- **Tests added**: `remote_sync` coverage for no commits, missing remote tracking ref, missing remote/repo, and not-fast-forward push.
- **Touched files**: `crates/core/src/core/git.rs`, `crates/core/src/lib.rs`, `crates/core/tests/remote_sync.rs`, `.review/findings/M-1.md`, `REVIEW.md`.
- **Reviewer verdict**: VERIFIED (commit `6a8c6bb`). Classifier at `git.rs:18-106` maps `UnbornBranch → NoCommits`, `NotFound + Reference → NoUpstream`, `Net|Http → RemoteUnavailable`, `NotFastForward`, and `Merge|Checkout → Conflict`; auth path present (Code + Class + message fallback) but untested per acknowledged gap. 4 new tests pattern-match on `GitErrorKind` variants (not just `is_err`). 84 `Unknown` sites remain in `git.rs` (intentional per Known Gaps — index/commit plumbing not user-actionable). Tauri serialization unchanged (string passthrough via `serialize_str`), so frontend wire compat preserved.

### M-2: Corrupt sidecars silently overwritten `[x]`
- **What**: `reader.rs:228` `read_threads(...).unwrap_or_default()` — one corrupt `threads.yaml` and the next save erases every thread. Same shape at `writer.rs:285` (swallowed `.meta` parse), `src-tauri/src/commands/io.rs:423` (writing_history wipe).
- **Branch**: `fix/m-2-corrupt-sidecars`
- **Notes for GPT**: Quarantine the corrupt file (rename to `.corrupt-<timestamp>`) before defaulting, and emit a warning so the user sees a banner. Pair with H-1.
- **Approach**: Corrupt `threads.yaml` already fails load on current master. This branch makes existing document `.meta` parsing fail before `project.yaml` or the sidecar can be rewritten, and makes all project writing-history reads/writes, including session progress, use the same strict parser instead of defaulting corrupt JSON to empty history.
- **Tests**: `cargo test -p chickenscratch-core corrupt --lib` (passed); `cargo test -p chickenscratch writing_history --bins` (passed); `cargo test -p chickenscratch session_progress_rejects_corrupt_writing_history --bins` (passed); `git diff --check` (passed)
- **Reviewer verdict**: VERIFIED after reopen (commit `d89b3a2` on top of `a41a672`). First pass was reopened because `get_session_progress` at `io.rs:533-536` still used the silent `.ok().and_then(...).ok().unwrap_or_default()` pattern. Fix-up extracts `parse_writing_history` as a shared helper used by all three sites (`get_writing_history:440`, `record_daily_words_impl:470`, `get_session_progress:533`); `unwrap_or_default` is gone. New test `writing_history_parser_rejects_corrupt_json` covers the corruption path. `.meta` pre-flight validation (`writer.rs:288, 490-506, 541`) and H-1's `read_threads?` (`reader.rs:228`) retained. M-2-followup-app-settings flagged separately for `settings.rs:354-357, 517-520` silent-default parses.

### M-3: Pandoc subprocesses have no timeout `[x]`
- **What**: `Command::new("pandoc").output()` blocks forever if pandoc hangs.
- **Branch**: `fix/m-3-pandoc-process-limits`
- **Files**: `crates/core/src/core/compile.rs:172`, `crates/core/src/scrivener/parser/rtf.rs:23`, `src-tauri/src/commands/io.rs:166`.
- **Notes for GPT**: Use `wait_timeout` crate, or spawn + `child.wait_timeout(Duration::from_secs(60))` + kill on expiry. Add a max-bytes guard on stdout (~50 MB) too.
- **Approach**: Added a std-only shared process runner with a 60-second timeout, kill-on-expiry, and a 50 MiB combined stdout/stderr cap. Routed compile, Scrivener RTF conversion, import conversion, Pandoc discovery, and the settings Pandoc check through it.
- **Tests**: `cargo fmt --all` (ran; unrelated existing rustfmt churn not committed); `cargo clippy -p chickenscratch-core -p chickenscratch -p chickenscratch-tui -p chikn-converter --all-targets -- -D warnings` (passed); `cargo test -p chickenscratch-core -p chickenscratch -p chickenscratch-tui -p chikn-converter --lib --bins --tests` (passed); `cd ui && npm run lint && npm run build` (passed); `git diff --check` (passed)
- **Reviewer verdict**: VERIFIED after reopen (commit `53995c5`, force-pushed onto `bce74b5`-style single-commit shape). All 4 `clippy::io_other_error` sites at `process.rs:85, 89, 191, 212` now use `io::Error::other(msg)`. Re-ran `cargo clippy ... --all-targets -- -D warnings` clean on the branch. Helper functionality unchanged from the first pass: std-only, both pipes drained concurrently, atomic counter, kill+reap on timeout/cap, 5 tests, no zombies, all 6 pandoc callsites routed.

### M-4: Tauri CSP disabled + `shell:open` unconstrained `[x]`
- **What**: `tauri.conf.json:22` `csp: null`. `tauri.conf.json:36-38` `shell.open: true` with no validator regex. Any renderer injection chains to OS-handler code-exec.
- **Branch**: `fix/m-4-tauri-csp-shell-scope`
- **Files**: `src-tauri/tauri.conf.json:21-22, 36-38`; `src-tauri/capabilities/default.json:11`.
- **Notes for GPT**: CSP: start with `"csp": "default-src 'self'; img-src 'self' data: asset: https://asset.localhost; style-src 'self' 'unsafe-inline'; script-src 'self'"` and tighten. `shell.open`: change to a URL-scheme regex like `"open": "^https?://"`.
- **Approach**: Added a production CSP that restores same-origin defaults while allowing Tauri IPC and local app assets, plus a dev CSP for `http://localhost:1420` and Vite HMR. Switched the shell plugin from `open: true` to an anchored HTTPS-only host-shaped validator, added a config-regression test for prefix-bypass URLs, and replaced the unscoped capability command with `shell:default`.
- **Tests**: `cargo check --manifest-path src-tauri/Cargo.toml` (passed); `cargo test --manifest-path src-tauri/Cargo.toml shell_open_validator --bin chickenscratch` (passed)
- **Reviewer verdict**: VERIFIED after reopen (commit `6f6dd40` on top of `bce74b5`). First pass was reopened for missing `^` anchor (`tauri-plugin-shell` v2.3.5 uses `regex::is_match` substring mode then opens the original unmodified path — bypass via `file:///etc/passwd#https://x`). Fix-up adds full anchors: `^https://[A-Za-z0-9][A-Za-z0-9.-]*(?::[0-9]{1,5})?(?:[/?#][^\s]*)?$`. Inline regex check confirms `file:///etc/passwd#https://x`, `javascript:x//https://y` → rejected; `https://github.com/...`, `https://pandoc.org/...` → accepted. CSP unchanged from first pass (prod-strict, devCsp isolated to Vite). GPT also added a `shell_open_validator` config-regression test for prefix-bypass shapes.

### M-5: `simple_word_diff` O(m·n) without sane bail-out `[x]`
- **What**: `git.rs:973-1033` builds `vec![vec![0u32; n+1]; m+1]` LCS table. Cap is 5000 words → up to 100 MB allocation per call from the revisions UI.
- **Branch**: `fix/m-5-word-diff-bounds`
- **Files**: `crates/core/src/core/git.rs`.
- **Notes for GPT**: Either drop the cap to ~1500 words, or replace with a streaming diff (e.g. `similar` crate). Render a "diff too large" placeholder above the cap.
- **Approach**: Added an LCS matrix cell cap of `1_500 * 1_500`; larger requests now return the existing coarse deleted/added diff without building the table.
- **Tests added**: Small-diff regression preserving LCS output and large-diff regression proving the coarse deleted/added fallback is used above the cap. Validation: `cargo test -p chickenscratch-core simple_word_diff --lib`.
- **Reviewer verdict**: VERIFIED (commit `1cf2252`). Constant `SIMPLE_WORD_DIFF_LCS_CELL_CAP = 1_500 * 1_500` at `git.rs:17`. Pre-allocation cell-count check via `checked_mul((m+1), (n+1))` at `:1128-1134` — overflow-safe and **product**-based (the old per-side guard would let 100 × 100000 pass while blocking the cheap 100k × 50). Fallback `coarse_word_diff` (`:1185-1190`) returns exactly two `(String, String)` entries (`("deleted", old.join())`, `("added", new.join())`) — same shape as the normal return so the UI renders identically; allocation O(input bytes) not O(m·n). 9 MB max allocation vs the previous 100 MB hazard. **Non-blocking nit**: tests don't cover empty inputs, asymmetric inputs (1 × 100000), or the exact-boundary case — all correct by inspection but a spec lock would be tidier.

### M-6: `beforeunload` flush is best-effort `[x]`
- **What**: `App.tsx:148` awaits `flushPendingEditorSave()` in `beforeunload`, but `beforeunload` cannot block on real promises across the webview boundary.
- **Notes for GPT**: Switch to Tauri v2's `onCloseRequested` (`@tauri-apps/api/window`) to actually defer close until the save resolves.
- **Branch**: `fix/m-6-tauri-close-flush`
- **Approach**: Replaced the Tauri close path with `getCurrentWindow().onCloseRequested`, awaiting the pending editor flush and canceling close with a user-visible toast if that flush fails; browser `beforeunload` remains only as a fallback outside Tauri.
- **Tests**: `cd ui && npm run lint`; `cd ui && npm run build`
- **Reviewer verdict**: VERIFIED after reopen (commit `a8723a8` on top of `5c0ff04`). First pass was reopened for silent close-abort. Fix-up adds the exact toast at `App.tsx:177-180`: "Close canceled because the latest editor changes could not be saved. Please retry, or check the editor for errors." Other invariants preserved from the original review: handler is async, awaits flush before backup, `event.preventDefault()` on failure, browser fallback gated correctly via `isTauri()`, subscription cleanup uses `disposed` flag. UI lint clean.

---

## LOW

### L-1: Binder re-renders on every editor save `[x]`
- **What**: Binder subscribes to whole `project`. Every 2s autosave rebuilds `project.documents` → full Binder tree + EntitySections + CommandPalette `flattenDocNames` re-runs.
- **Notes for GPT**: Switch to per-slice selectors with `useShallow`; specifically `(s) => s.project?.hierarchy` and `(s) => s.project?.documents` separately.
- **Branch**: `fix/l-1-binder-selectors`
- **Approach**: Split Binder and CommandPalette away from the full-project subscription. Binder now selects stable project identity fields plus hierarchy with `useShallow`, EntitySection uses a stable visible-entity signature, and ThreadDots subscribes only to the visible thread ids plus thread metadata references. Flow mode still checks `project.documents` at click time so stale hierarchy entries are filtered as before.
- **Tests**: `cd ui && npm run lint && npm run build` (passed; pre-existing Vite chunk/dynamic-import warnings); `git diff --check` (passed)
- **Known gaps**: EntitySection still scans the document map inside its selector to produce the stable signature; eliminating that selector-time scan would need a normalized store/entity index.
- **Reviewer verdict**: VERIFIED (commit `61a8f6e`). `applyContentToStore` (`Editor.tsx:30-49`) spreads only `project` and `project.documents`, leaving `project.hierarchy` reference intact across autosaves — that's the load-bearing invariant L-1 depends on. Subscriptions audited clean: `projectInfo` slice (`Binder.tsx:94-100`) reads only id/name/path with `useShallow`; `hierarchy` slice (`Binder.tsx:101`, `CommandPalette.tsx:30`) returns the same reference across content edits; `entitySignature` (`Binder.tsx:685-693`) returns a primitive string stable across non-entity edits (default `Object.is` works); `ThreadDots` (`:638-645`) reads only the active doc's `fields.threads` ref under `useShallow`; flow mode reads `project.documents` inside callbacks via `getState()` (no subscription). No remaining `(s) => s.project` selectors in Binder or CommandPalette. **Informational**: invariant rests on `applyContentToStore` keeping hierarchy untouched — if a future change introduces a full-project replace in the autosave path, L-1 silently regresses. Worth a code comment at the spread site.

### L-2: Bundle is ~890 KB, no code splitting `[x]`
- **Branch**: `fix/l-2-bundle-code-splitting`
- **Notes for GPT**: `vite.config.ts` add `build.rollupOptions.output.manualChunks: { tiptap: [...], prosemirror: [...] }`; `React.lazy` the rarely-mounted modals (Timeline, Preview, Compile, Stats, Comments).
- **Approach**: Lazy-loaded non-default views and panels, conditionally rendered modal-style components so lazy imports are not fetched at startup, lazy-loaded Binder's per-file history modal, and added Vite manual chunks for major vendor families.
- **Tests**: `cd ui && npm run lint && npm run build` (passed); `git diff --check` (passed).
- **Build output**: startup `index` JS chunk reduced to 49.74 kB; largest JS chunk is `editor-prosemirror` at 330.04 kB; Vite large chunk warning no longer appears.
- **Known gaps**: default editor dependencies still load on startup because Editor remains the first screen for open projects.
- **Reviewer verdict**: VERIFIED (commit `ebd9526`). `vite.config.ts:13-39` `manualChunks` routes React/Tiptap/ProseMirror/Tauri/icons/Zustand/marked+DOMPurify to dedicated chunks. `App.tsx:32-86` lazy-loads 11 panels via `React.lazy` with default-export adapters; `App.tsx:455-469` gates each via `{showFoo && <Foo .../>}` so chunks fetch on open, not at startup. Suspense boundaries at `App.tsx:451, 455` and `Binder.tsx:612`. Editor stays eager (the startup view). Binder DocumentHistory lazy + gated at `Binder.tsx:30-33, 612-620`. Build artifact confirms: startup `index` 50.03 kB (gzip 14.94 kB), largest chunk `editor-prosemirror` 330 kB (gzip 115 kB), no `(!) Some chunks are larger than 500 kB` warning. 94% startup-chunk reduction (890 KB → 50 KB). Per-panel lazy chunks all 4–16 kB pre-gzip. **Minor observations (non-blocking)**: `Suspense fallback={null}` is fine because the surrounding toolbar/binder layout stays mounted; `marked`+`dompurify` are bundled into `preview-markdown` (64 kB), loaded only with Preview.

### L-3: Modals not real modals — no `role="dialog"`, no focus trap `[x]`
- **Branch**: `fix/l-3-modal-a11y`
- **Files**: `ui/src/components/shared/Dialog.tsx`, `ui/src/components/compile/CompileDialog.tsx`, `ui/src/components/command-palette/CommandPalette.tsx`, `ui/src/components/search/ProjectSearch.tsx`, `ui/src/components/settings/Settings.tsx`, `ui/src/components/comments/CommentsPanel.tsx`, `ui/src/components/binder/Binder.tsx`, `ui/src/components/revisions/DocumentHistory.tsx`, `ui/src/components/revisions/DraftCompare.tsx`, `ui/src/components/revisions/Revisions.tsx`.
- **Approach**: Added a shared modal focus trap and applied dialog role, `aria-modal`, labels, Escape handling, focus restoration, and Tab containment to the blocking overlay dialogs. Kept `CommentsPanel` non-modal and labelled it as a complementary side panel.
- **Tests**: `cd ui && npm run lint && npm run build` (passed; pre-existing Vite bundle/dynamic-import warnings); `git diff --check` (passed); two subagent review passes completed and their findings were fixed.
- **Known gaps**: No automated keyboard-navigation harness exists for these interactions; validation is lint/build plus focused code review.
- **Reviewer verdict**: VERIFIED (commit `7e00c59`). `useModalFocusTrap` in `Dialog.tsx:57-105` auto-focuses initialFocusRef → first focusable → container, restores `previousFocus` on cleanup with `document.contains` guard, intercepts Tab/Shift+Tab to wrap inside the modal, handles the escaped-focus edge case (`!dialog.contains(active)` re-routes back), Escape calls `onClose`, `stopPropagation` on every key prevents leak to `App.tsx:143` window-level shortcuts. All 9 modal overlays have `role="dialog"` + `aria-modal="true"` + accessible name + `tabIndex={-1}` + wired focus trap. Icon-only close buttons carry `aria-label`. CompileDialog inputs have `<label htmlFor>` associations. CommandPalette Escape explicitly `stopPropagation`s. CommentsPanel correctly stays non-modal with `role="complementary"` + `aria-labelledby`. **Non-blocking nit**: Settings focuses the X button on open — WCAG-compliant but some UX guidance prefers focusing a primary action.

### L-4: Binder is not a keyboard tree `[x]`
- **Branch**: `fix/l-4-binder-keyboard-tree`
- **Files**: `ui/src/components/binder/Binder.tsx`, `ui/src/editor.css`.
- **Approach**: Scoped `role="tree"` to the hierarchy, added treeitem roles and ARIA state, lifted folder open state for a visible-node model, and added keyboard navigation for arrow keys, Home/End, Enter/Space, Escape, and keyboard context menu access.
- **Tests**: `cd ui && npm run lint && npm run build` (passed); `git diff --check` (passed); two subagent review passes completed and their findings were fixed.
- **Known gaps**: The row `...` affordance remains mouse-only; keyboard users can open the node context menu with ContextMenu or Shift+F10.
- **Reviewer verdict**: VERIFIED (commit `e151aea`). Container at `Binder.tsx:738-754` has `role="tree"` + `aria-label` + `aria-activedescendant` + `tabIndex={0}`. Documents (`:1129-1166`) have `role="treeitem"` + `aria-level` + `aria-selected` (no `aria-expanded` per WAI-ARIA for leaves). Folders (`:1171-1221`) have all four ARIA attrs with `aria-expanded={hasChildren ? open : undefined}` so empty folders correctly omit it. Child groups have `role="group"` (`:1223`). Keyboard handler (`:555-664`) covers Up/Down (clamp + flat visible walk), Home/End, Right (expand → first child / leaf no-op), Left (collapse / move to parent), Enter/Space (open doc / toggle folder), ContextMenu/Shift+F10 (positions at active item's bounding rect, not mouse), Escape (clears). `flattenVisibleTreeNodes` (`:112-133, 531-538`) respects lifted `openFolders` state so the visible list matches the rendered DOM; `focusedNodeId` (`:539-545`) auto-falls-back when selection becomes hidden. `...` affordance is `<span aria-hidden="true">` so it stays out of the tab order. Click/drag/right-click/flow-mode (cmd-click) all preserved. CSS focus rule scoped to `.binder-outline:focus-visible`, not global.

### L-5: Swift uses literal folder IDs vs Rust UUIDs `[x]`
- **What**: `macos/Sources/ChiknKit/Writer.swift:24-27` writes `TreeNode(id: "manuscript", ...)`; Rust writer.rs:147-160 writes `uuid::Uuid::new_v4()`. No exploit today but a footgun if any code hardcodes the id shape.
- **Notes for GPT**: Align Swift to UUIDs.
- **Branch**: `fix/l-5-swift-folder-ids`
- **Approach**: Swift project creation now assigns UUIDs to the required top-level folders, while `createDocument` resolves legacy root aliases such as `manuscript` and `research` to the current UUID root before inserting hierarchy nodes or writing `.meta parent_id`. Disk paths still use the canonical folder names.
- **Tests**: `cd macos && swift run ChiknKitChecks` (passed); `git diff --check` (passed)
- **Known gaps**: SwiftUI still has some display-name/path-prefix heuristics for folder icons and default destinations; this branch removes literal ids from newly written `project.yaml` and keeps legacy alias inputs working.
- **Reviewer verdict**: VERIFIED (commit `8d280a1`). `Writer.swift:24-27` now assigns `UUID().uuidString.lowercased()` to all three roots; display names unchanged; disk dirs created via literal `["manuscript","research","trash"]` loop so on-disk layout is preserved. `resolvedParentID` (`:560-565, 578-590`) resolves legacy aliases via `rootDirectoryByLegacyID` strict-match (lowercase literals only) and falls through unresolved input rather than guessing. Rust reader treats `hierarchy[].id` as opaque — zero `id == "manuscript"` literal comparisons in `crates/core`; matches in `converter/mod.rs:557,565,571` compare `name.to_lowercase()` (Scrivener importer, intentional). All 60+ Swift checks pass on this macOS host including the new UUID-id and alias-routing cases. **Non-blocking cosmetic gap (per Known Gaps)**: `BinderView.swift:240-244` icon switch on `node.id` literals will silently fall through to default folder icon for new Swift-created projects — needs a follow-up to switch to name-based or root-detection heuristic.

### L-6: Pandoc resolved via $PATH `[x]`
- **Files**: `src-tauri/src/commands/io.rs:188-222`. Hijackable if a writable dir is ahead of `/usr/local/bin`.
- **Notes for GPT**: Document the requirement in `docs/USER_GUIDE.md` and prefer absolute paths in the candidates list.
- **Branch**: `fix/l-6-pandoc-path-hardening`
- **Approach**: Added a shared Tauri `resolve_pandoc` helper that rejects non-empty relative custom paths, checks absolute standard install locations, and returns an absolute executable path. Tauri compile, file import, Scrivener import, and `check_pandoc` now use that resolver. Documented the absolute-path requirement in `docs/USER_GUIDE.md`.
- **Tests**: `cargo test --manifest-path src-tauri/Cargo.toml settings::tests --bin chickenscratch` (passed); `cargo check --manifest-path src-tauri/Cargo.toml` (passed); `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` (passed); `git diff --check` (passed)
- **Known gaps**: Core/CLI Scrivener helpers still use PATH when non-Tauri callers pass no Pandoc path.
- **Reviewer verdict**: VERIFIED (commit `6f02674`). `resolve_pandoc` (`settings.rs:603-622`) checks user path first via `normalize_pandoc_path` (absolute-only) then iterates absolute standard candidates. Standard candidates list (`settings.rs:567-583`) covers Windows (`C:\Program Files\Pandoc\pandoc.exe`, `C:\Program Files (x86)\Pandoc\pandoc.exe`) and Unix (`/opt/homebrew/bin`, `/usr/local/bin`, `/usr/bin`, `/opt/local/bin`, `/snap/bin`). `Command::new(absolute_path)` in `pandoc_version` (`:585-601`) doesn't trigger PATH lookup (Rust stdlib semantics). Settings save-time validation (`:550-564`) rejects relative paths with actionable message. All 3 Tauri callsites (`io.rs:30 compile`, `io.rs:198-201 find_pandoc`, `project.rs:51 import_scrivener`) route through the resolver — no legacy fallback list anywhere. `docs/USER_GUIDE.md:198, 334` documents absolute-path requirement with platform examples. Core/CLI Scrivener helpers still accept `None` → use PATH; out of scope per L-6's stated Known Gap (CLI dev path, not Tauri user surface). **Minor non-blocking UX**: a typo absolute path (e.g. `/usr/local/bin/pamdoc`) is neither rejected at save nor surfaced at use — it silently falls through to standard candidates; user wouldn't know their custom path is being ignored.

### L-7: AI SSE streams no max-bytes guard `[x]`
- **Files**: `src-tauri/src/commands/ai.rs:170-188, 228-260, 302-323`. Malicious endpoint = unbounded memory.
- **Approach**: Added an 8 MiB raw response-body byte budget for AI streaming readers before provider-specific SSE/JSON parsing. The shared reader wrapper allows exactly-at-limit EOF, rejects any additional byte, and is used by Ollama, Anthropic, and OpenAI streaming loops without changing cancellation or document-routing event behavior.
- **Tests added**: Focused unit tests for exact-limit EOF, over-limit rejection, and buffered line accounting in `src-tauri/src/commands/ai.rs`.
- **Touched files**: `src-tauri/src/commands/ai.rs`, `.review/findings/L-7.md`, `REVIEW.md`.
- **Reviewer verdict**: VERIFIED (commit `2d2407b`). `ByteBudgetRead` at `ai.rs:192-232` holds inner reader + consumed/limit counters. EOF-at-exact-limit handled correctly via a 1-byte probe (`:215-225`) — naive `>= limit { error }` would have rejected the legitimate exact case. `max_read = min(remaining, buf.len())` ensures one over-budget byte triggers error. All three streams (Ollama `:294`, Anthropic `:362`, OpenAI `:440`) wrap the raw `reqwest::blocking::Response` before `BufReader` — total bytes bounded, not per-chunk. 8 MiB cap is 3 orders of magnitude above typical streaming response size (max_tokens 2000 → ~80 KB with SSE framing). `InvalidData` errors propagate via `line.map_err(...)` to the worker's `emit_error(...)` so the user gets a toast. Cancel/cap race is benign in both orders; registry cleanup runs regardless. 3 new tests assert specific outcomes (not just `is_err`), including the exact-at-EOF case. No double-wrap; non-streaming paths intentionally untouched.

### L-8: `linux/qml/SettingsDialog.qml` exposes AI tab without backing invokable `[x]`
- **Files**: `linux/qml/SettingsDialog.qml:78, 233`. Fake-tab footgun.
- **Approach**: Removed the Linux settings AI tab and its QML-only default seeding because the Linux bridge only round-trips the settings blob and exposes no AI invokables that consume those fields.
- **Tests added**: Focused static validation that `SettingsDialog.qml` no longer contains an AI tab, `settings.ai` bindings, or provider/API key controls. Attempted `cargo test -p chickenscratch-linux settings_default_round_trip`, but this macOS host fails compiling `cxx-qt` against Homebrew Qt before the test binary is built (`qyieldcpu.h` implicit `__yield` under `-Werror`).
- **Touched files**: `linux/qml/SettingsDialog.qml`, `.review/findings/L-8.md`, `REVIEW.md`.
- **Reviewer verdict**: VERIFIED (commit `6cb77f3`). Five tabs only in `SettingsDialog.qml` (General/Writing/Backup/Compile/Remote at `:74-78`), five matching stack pages. Zero `settings.ai` references and zero OpenAI/Anthropic/Ollama/api_key strings across all `linux/qml/*.qml`. Rust bridge symmetry preserved: `linux/src/bridge.rs:1527, 1611-1618` still serializes `AiSettings` for cross-frontend wire compat (a Linux user opening a Tauri/Windows-touched project still round-trips the `ai` JSON cleanly). No other Linux QML dialogs expose AI features. `cargo test -p chickenscratch-linux` not runnable on this macOS host due to the documented Qt6/cxx-qt issue — out of scope for L-8.

### L-9: Pandoc stdout unbounded buffer on import `[x]`
- **Files**: `src-tauri/src/commands/io.rs:166-185, 254-323`. Cap at 50 MB.
- **Branch**: `fix/l-9-pandoc-import-output-cap`
- **Approach**: Closed as covered by the M-3 bounded-process implementation. Tauri file import now runs `convert_to_markdown_via_pandoc` through `output_bounded(..., PANDOC_OUTPUT_LIMIT_BYTES)`, where the shared helper enforces a 50 MiB combined stdout/stderr cap and kills the child on overflow.
- **Tests**: `cargo test -p chickenscratch-core process --lib` (passed); `git diff --check` (passed)
- **Known gaps**: No additional code change in this branch; the import callsite named in L-9 was already fixed by M-3.
- **Reviewer verdict**: VERIFIED as duplicate of M-3. Confirmed at `src-tauri/src/commands/io.rs:6` (imports `output_bounded, PANDOC_OUTPUT_LIMIT_BYTES, PANDOC_TIMEOUT`), `:113` and `:277` (import callsites), `:155` (`convert_to_markdown_via_pandoc` definition), `:183` (actual `output_bounded(..., PANDOC_OUTPUT_LIMIT_BYTES)` call). 50 MiB combined stdout/stderr cap from the M-3 helper kills the child on overflow. No new code needed; finding closed as already-fixed.

---

## Second-pass findings (post-cycle rescan, 2026-05-16)

After the first cycle closed all originally-listed findings, a rescan of the v1.2 feature code that landed between the original audit and now surfaced these. None CRITICAL; the security-tagged ones are pre-auth (require attacker control of a project path or pre-set symlink).

### N-1: `create_entity` bypasses C-3 symlink validation `[x]`
- **What**: `create_entity` in `src-tauri/src/commands/document.rs:356-359` (and the parallel folder-creation in `src-tauri/src/commands/io.rs:358`) calls `std::fs::create_dir_all(&folder_path)` where `folder_path = project_path.join("characters")` or `"locations"`. Unlike the reader's `ensure_required_folder_safe` (`reader.rs:340-367`) and the writer path that C-3 hardened (`writer.rs:293-331`), entity creation does **not** check whether the path is a symlink or escapes the project root before writing. A hostile `.chikn` containing a pre-existing `characters` symlink to `~/.ssh/` would write entity files to the symlink target on first entity creation.
- **Severity**: MEDIUM. Pre-auth requires attacker to plant the symlink (hostile-project model), same threat surface as C-3. Strictly weaker than C-1/C-3 (which had the same root cause and shipped fixes).
- **Files**: `src-tauri/src/commands/document.rs:356-359` (`create_entity`); `src-tauri/src/commands/io.rs:358` (entity-folder creation in import flow if applicable).
- **Notes for GPT**: Reuse the C-3 helper. The cleanest path: extract `ensure_required_folder_safe` (or its writer-side twin) into a public function in `crates/core/src/core/project/writer.rs` (or a new `crates/core/src/core/project/safe_path.rs`) and call it from any code in `src-tauri` that creates a directory inside a project. Add a test: hostile project with `characters` → symlink to `/tmp/escape`; `create_entity` returns `Err(ChiknError::InvalidFormat)` without touching the symlink target.
- **Branch**: `fix/n-1-entity-folder-symlink-validation`
- **Approach**: added shared `safe_path::ensure_project_subdir_safe`, routed `create_entity` through it before project read/write, extracted `create_entity_impl` for tests, and reused the helper from reader required-folder repair.
- **Tests**: `cargo test -p chickenscratch-core safe_path --lib`; `cargo test -p chickenscratch document::tests --bins`; `cargo clippy -p chickenscratch-core -p chickenscratch -p chickenscratch-tui -p chikn-converter --all-targets -- -D warnings`; `cargo test -p chickenscratch-core -p chickenscratch -p chickenscratch-tui -p chikn-converter --lib --bins --tests`; `cd ui && npm run lint && npm run build`; `git diff --check`.
- **Reviewer verdict**: VERIFIED (commit `a754127`). New `safe_path` module at `crates/core/src/core/project/safe_path.rs`: component-by-component validation (`:24-30` matching C-3's writer.rs:374-380 rules), `symlink_metadata` rejection on existing components AND on each just-created component, **one-component-at-a-time** `fs::create_dir` (not `create_dir_all`) catches symlinks materialized between check and create, canonicalize-under-root with project-root canonicalized once up front (catches macOS `/var → /private/var` escapes). `create_entity` (`document.rs:360-362`) runs the helper BEFORE `read_project` so hostile symlink is rejected before any traversal. Reader's old `ensure_required_folder_safe` now delegates to the shared helper (closes the H-1 DRY nit). C-3's writer-side `ensure_existing_path_safe` is intentionally separate per Known Gap (different surface: doc/`.meta` file targets vs directory creation; rules are compatible). 5 new safe_path tests + entity rejection test asserts symlink target unchanged. **Follow-up surfaced (non-blocking)**: `src-tauri/src/commands/io.rs:438` `record_daily_words_impl` uses `fs::create_dir_all` on `project_path/settings` without the helper — same pattern, different surface; opening as N-1-followup-settings-dir.

### N-1-followup-settings-dir: `record_daily_words` settings folder unguarded `[x]`
- **What**: Surfaced during N-1 verification. `src-tauri/src/commands/io.rs:438` `record_daily_words_impl` calls `fs::create_dir_all(project_path/"settings")` without going through `safe_path::ensure_project_subdir_safe`. Same symlink-bypass shape as the original N-1; lower exposure because `settings/` is internal to the app and rarely user-named, but the consistency win is worth taking.
- **Files**: `src-tauri/src/commands/io.rs:438`.
- **Notes for GPT**: One-liner — replace the `create_dir_all` call with `safe_path::ensure_project_subdir_safe(project_path, "settings")?`. Validate `cargo clippy ... -- -D warnings` clean before sentinelling.
- **Branch**: `fix/n-1-followup-settings-dir`
- **Approach**: routed `record_daily_words_impl` settings directory creation through `safe_path::ensure_project_subdir_safe` and added normal + symlink-hostile writing-history tests.
- **Tests**: `cargo test -p chickenscratch commands::io::tests --bins`; `cargo clippy -p chickenscratch --all-targets -- -D warnings`; `git diff --check`.
- **Reviewer verdict**: VERIFIED (commit `1d00394`). `record_daily_words_impl` (`io.rs:437`) now calls `safe_path::ensure_project_subdir_safe(project_path, Path::new("settings"))?` in place of the raw `create_dir_all`. New tests added (symlink-hostile + normal path) — 39 chickenscratch bin tests pass. Clippy clean under `-D warnings`.

### N-2: `DocumentHistory` swallows fetch errors silently `[x]`
- **What**: `ui/src/components/revisions/DocumentHistory.tsx:42-44` — the effect that loads git history catches all errors and sets `revisions = []`. On a corrupt repo, permission-denied, or other failure, the user sees an empty history with no signal that anything failed.
- **Severity**: LOW (UX). Pairs with the M-2 / M-3 pattern of preferring loud failure over silent empty.
- **Files**: `ui/src/components/revisions/DocumentHistory.tsx:42-44`.
- **Notes for GPT**: Replace `.catch(() => { if (!cancelled) setRevisions([]); })` with `.catch((e) => { if (!cancelled) { setRevisions([]); toastError("Failed to load document history: " + e); } })`. Also flush-before-fetch ordering: line 39's `flushPendingEditorSave()` is fired but never awaited inside the effect — a quick race where stale buffer flushes after the doc switch.
- **Branch**: `fix/n-2-document-history-errors`
- **Approach**: split flush and git-history catches; file-history flush failure now toasts and aborts the fetch, while git-history load failure clears revisions and shows a failure toast.
- **Tests**: `cd ui && npm ci && npm run lint && npm run build && git diff --check`.
- **Reviewer verdict**: VERIFIED (commit `5f85bf9`). Effect at `DocumentHistory.tsx:32-58` now wraps the work in an async IIFE so `flushPendingEditorSave()` is actually awaited (closes the race I called out alongside the toast issue). Two separate `try/catch` blocks: flush failure → toast "File history aborted — editor save failed: …" and returns; gitCmd failure → toast "Failed to load document history: …". Cancellation guards (`if (!cancelled)`) preserved on every state mutation. Lint clean.

### N-3: Tightening pass — silent error swallows + perf nits `[x]`
- **What**: A bundle of small low-severity items surfaced in the rescan. Group as one branch since they're all "make this loud" / "guard this edge."
  1. `crates/core/src/core/git.rs:196-200` — `.gitignore` write uses `.ok()` swallow. Init succeeds with a missing gitignore; user has no signal. Convert to `?` or log.
  2. `crates/core/src/core/git.rs:643` — backup `create_dir_all` uses `.ok()` swallow before `Repository::init_bare()`; the subsequent init failure surfaces a less informative message. Either propagate the create_dir error or document the swallow.
  3. `src-tauri/src/commands/threads.rs:30-51` — entity-folder walk has no depth/size cap. Theoretical OOM on pathological projects. Cap at e.g. 1024 entities per type and 8 levels.
  4. `src-tauri/src/commands/threads.rs:56-84` — dangling-ref `check` hard-codes the four convention keys (`pov_character`, `characters_in_scene`, `location`, `threads`). Any custom convention-key referencing entities is silently un-validated. Document this constraint or make the key set configurable.
  5. `ui/src/components/timeline/TimelineView.tsx:17-37` — `parseStoryTime` returns `{ time: NaN, display: "" }` on parse failure; downstream filters work but the all-equal-times case at `:177` makes duration scaling uniformly 50%. Cosmetic; add a "no timeline data" empty state if all parses fail.
  6. `ui/src/components/editor/Editor.tsx:461-464` — `SessionBadge` swallows `get_session_progress` errors silently. Convert to a one-time toast or a small "session tracking unavailable" indicator.
- **Severity**: LOW each, but they cluster as the same "make failures observable" pattern we hardened in H-1 / M-2.
- **Branch**: `fix/n-3-tightening-pass`
- **Approach**: propagated `.gitignore` and backup-dir filesystem errors; capped entity reference scans; documented known validated novelist keys; added invalid-story-time timeline empty state and same-time duration guard; made session-progress failures visible with a one-time toast.
- **Tests**: `cargo test -p chickenscratch-core push_backup_reports_backup_directory_create_failure --lib`; `cargo test -p chickenscratch commands::threads::tests --bins`; `cargo clippy -p chickenscratch-core -p chickenscratch --all-targets -- -D warnings`; `cd ui && npm ci && npm run lint && npm run build`; `cargo fmt --all -- --check`; `git diff --check`.
- **Reviewer verdict**: VERIFIED (commit `b494f9e`). (1) `.gitignore` write uses `?` propagation with path in message at `git.rs:198-207`; (2) `push_backup` reports `create_dir_all` failure before `init_bare` at `:643-655`, new `push_backup_reports_backup_directory_create_failure` test; (3) `threads.rs:24-25, 90-120` enforces `MAX_ENTITIES_PER_TYPE=1024` and `MAX_ENTITY_PATH_DEPTH=8`, both return `InvalidFormat` on overflow (errors out, not silent), two new unit tests; (4) doc-only resolution for the four convention keys in `threads.rs:28-32` and `docs/UI_CONVENTIONS_NOVELIST.md:105` (rule #5) — acceptable per Known Gap; (5) `TimelineView.tsx:17-77, 129-160` distinguishes "no Story Time" vs "all NaN" empty states with distinct messages; `durationWidth` returns `undefined` when all valid times are equal, so no scaling artifact; (6) `Editor.tsx:456, 461, 466-473` uses a `sessionErrorShown` ref guard reset on `[project]` dep change — one toast on first failure per project, suppressed on 30-second refresh until project changes. Validation clean (clippy `-D warnings`, 43+90+4+10 tests pass, eslint clean, no scope creep).

### N-FMT: rustfmt drift in 7 files `[x]`
- **What**: `cargo fmt --all -- --check` shows 32 diff locations across:
  - `crates/core/src/core/git.rs` (10 hunks)
  - `crates/core/src/models/project.rs`
  - `crates/tui/src/app.rs` (3 hunks)
  - `crates/tui/src/ui.rs`
  - `linux/src/bridge.rs` (5+ hunks)
  - `src-tauri/src/commands/document.rs`
  - `src-tauri/src/commands/git.rs`
- **Severity**: LOW. Validation suite in `REVIEW.md` lists `cargo fmt --all` but didn't run it before this commit. No correctness impact.
- **Notes for GPT**: `cargo fmt --all` — single command, atomic commit, no scope creep into unrelated files. Add a pre-commit hook or CI gate so this doesn't drift again.
- **Branch**: `fix/n-fmt-rustfmt-drift`
- **Approach**: ran `cargo fmt --all`; `src-tauri/src/commands/document.rs` was already formatted by N-1, leaving six files in this branch.
- **Tests**: `cargo fmt --all -- --check`; `git diff --check`.
- **Reviewer verdict**: VERIFIED (commit `ae06b9b`). Post-merge `cargo fmt --all -- --check` returns 0 diffs. Diff is format-only across the expected six files (`crates/core/src/core/git.rs`, `crates/core/src/models/project.rs`, `crates/tui/src/app.rs`, `crates/tui/src/ui.rs`, `linux/src/bridge.rs`, `src-tauri/src/commands/git.rs`). Pre-commit hook / CI gate to prevent future drift remains a separate ops item, not blocking.

---

## RELEASE READINESS

### R-1: Tauri release build fails from a clean checkout `[x]`
- **What**: `cargo tauri build` can fail before packaging because Rust `tauri-plugin-dialog` was locked to `2.6.0` while the npm dialog plugin was locked to `2.7.0`, and `ui/dist` was expected to exist before the Tauri build started.
- **Severity**: HIGH for release readiness. The primary desktop release artifact must build from a clean checkout.
- **Branch**: `fix/r-1-tauri-plugin-version-alignment`
- **Approach**: updated `Cargo.lock` to resolve `tauri-plugin-dialog` to `2.7.0` and added a Tauri `beforeBuildCommand` that runs `cd ../ui && npm run build` from the `src-tauri` working directory.
- **Tests**: `cd ui && npm ci`; `rm -rf ui/dist && cargo tauri build`.
- **Files changed**: `Cargo.lock`, `src-tauri/tauri.conf.json`, `.review/findings/R-1.md`, `REVIEW.md`.
- **Known gaps**: bundle targets remain app-only; DMG/AppImage/MSI/installer decisions stay separate release-packaging work.
- **Reviewer verdict**: VERIFIED (commit `46faa17`). `tauri.conf.json:9` now sets `beforeBuildCommand` to `cd ../ui && npm run build`, so `cargo tauri build` from a clean checkout produces `ui/dist` itself. `Cargo.lock` aligns Rust `tauri-plugin-dialog` to `2.7.0` matching the npm side (cascading windows-sys version shuffles in the lock are resolver-normal). Clippy `-D warnings` clean, 147 workspace tests pass. **Self-surfaced finding** — GPT identified this from outside the existing REVIEW.md list, which is the right reviewer behaviour and worth calling out.

### R-2: Windows CI workflow points at stale solution and SDK `[x]`
- **What**: `.github/workflows/windows.yml` installs .NET `8.0.x` while the Windows projects target .NET 10, restores/builds missing `ChickenScratch.sln`, and passes a solution-level `/p:Platform=x64` that is invalid for the checked-in `.slnx`.
- **Severity**: HIGH for release readiness. The only Windows CI workflow is failing by construction.
- **Branch**: `fix/r-2-windows-ci-sdk-slnx`
- **Approach**: install .NET `10.0.x`, restore/build `ChickenScratch.slnx`, remove the invalid solution-level `Platform=x64`, and build both core test harness projects in the final CI step.
- **Tests**: old-path and old-platform restore commands fail as expected; `dotnet restore ChickenScratch.slnx /p:EnableWindowsTargeting=true`; core and both harness Release builds; `git diff --check`.
- **Files changed**: `.github/workflows/windows.yml`, `.review/findings/R-2.md`, `REVIEW.md`.
- **Known gaps**: full WinUI solution build must be validated on Windows; macOS cannot execute the Windows App SDK XAML compiler.
- **Reviewer verdict**: VERIFIED (commit `30e0c79`). `.github/workflows/windows.yml` now installs `10.0.x` SDK (matches the `net10.0` / `net10.0-windows10.0.19041.0` targets), restores against `ChickenScratch.slnx` (the file that actually exists in the repo), drops the invalid `/p:Platform=x64` solution-level override, and the final build step now exercises both harnesses (GitServiceRestoreHarness from H-2, CrossFrontendHarness from H-6) instead of only the Core library. All three referenced files exist in the working tree. The Known Gap about XAML compilation only on Windows is correct — CI runs on `windows-latest` per the workflow's `runs-on`, so that's a non-issue at CI time; macOS-host validation of the WinUI half remains impossible without a Windows machine. Second self-surfaced finding in a row — release readiness was actively broken on multiple fronts.

### R-3: Root MIT license file missing `[~]`
- **What**: README and Cargo metadata declare MIT licensing, and `pkg/arch/PKGBUILD` installs `LICENSE`, but the repository has no root `LICENSE` file.
- **Severity**: MEDIUM for release readiness. Package generation will fail or ship without the declared license file.
- **Branch**: `fix/r-3-root-license`
- **Approach**: added the standard MIT license text at the repository root.
- **Tests**: `test -f LICENSE`; `grep -q "MIT License" LICENSE`; `bash -n pkg/arch/PKGBUILD`; `git diff --check`.
- **Files changed**: `LICENSE`, `.review/findings/R-3.md`, `REVIEW.md`.
- **Known gaps**: Arch source URL/checksum and Tauri bundle license metadata remain separate packaging findings.

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

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

### R-3: Root MIT license file missing `[x]`
- **What**: README and Cargo metadata declare MIT licensing, and `pkg/arch/PKGBUILD` installs `LICENSE`, but the repository has no root `LICENSE` file.
- **Severity**: MEDIUM for release readiness. Package generation will fail or ship without the declared license file.
- **Branch**: `fix/r-3-root-license`
- **Approach**: added the standard MIT license text at the repository root.
- **Tests**: `test -f LICENSE`; `grep -q "MIT License" LICENSE`; `bash -n pkg/arch/PKGBUILD`; `git diff --check`.
- **Files changed**: `LICENSE`, `.review/findings/R-3.md`, `REVIEW.md`.
- **Known gaps**: Arch source URL/checksum and Tauri bundle license metadata remain separate packaging findings.
- **Reviewer verdict**: VERIFIED (commit `c25e0ef`). Standard MIT text added at repo root, matches `pkg/arch/PKGBUILD`'s `install -Dm644 LICENSE` expectation and the `license=('MIT')` declaration. README's "License" section is now backed by an actual file. Aligns with Rust crate metadata (declares MIT). Third self-surfaced release-readiness finding in a row — the project's licensing posture was incomplete relative to its declared metadata.

### R-5: macOS DMG output is documented but not configured `[x]`
- **What**: README promises both `.app` and `.dmg` macOS artifacts, but the shared Tauri config only requested `["app"]`; README also put the `.dmg` in the wrong output directory.
- **Severity**: MEDIUM for release readiness. The default macOS release command should produce the documented artifacts.
- **Branch**: `fix/r-5-macos-dmg-bundle`
- **Approach**: changed Tauri bundle targets to `["app", "dmg"]` and corrected the README macOS bundle paths.
- **Tests**: `cd ui && npm ci`; `cargo tauri build --bundles app,dmg`; `.app` directory check; `.dmg` file check; `git diff --check`.
- **Files changed**: `src-tauri/tauri.conf.json`, `README.md`, `.review/findings/R-5.md`, `REVIEW.md`.
- **Known gaps**: Linux AppImage configuration remains a separate release packaging finding.
- **Reviewer verdict**: VERIFIED (commit `369ca38`). `src-tauri/tauri.conf.json` bundle targets now `["app", "dmg"]` — `cargo tauri build` actually produces the `.dmg` README promises. README path correction is correct: Tauri's bundle layout puts `.app` under `target/release/bundle/macos/` and `.dmg` under `target/release/bundle/dmg/` (separate directories). GPT validated by running `cargo tauri build --bundles app,dmg` and asserting both artifacts exist.

### R-4: Windows README build instructions target stale SDK and invalid solution platform `[x]`
- **What**: README says Windows requires .NET 8 even though the Windows projects target .NET 10, and documents `dotnet build ChickenScratch.slnx /p:Platform=x64`, which is invalid for the current `.slnx`.
- **Severity**: MEDIUM for release readiness. The documented Windows release path should match the project files and CI.
- **Branch**: `fix/r-4-windows-readme-dotnet10`
- **Approach**: updated the prerequisite to .NET 10 and changed the x64 release command to build `ChickenScratch.App/ChickenScratch.App.csproj` directly.
- **Tests**: README grep checks; `dotnet restore ChickenScratch.App/ChickenScratch.App.csproj /p:Platform=x64 /p:EnableWindowsTargeting=true`; `git diff --check`.
- **Files changed**: `README.md`, `.review/findings/R-4.md`, `REVIEW.md`.
- **Known gaps**: full WinUI app build still requires a Windows host.
- **Reviewer verdict**: VERIFIED (commit `7000435`). README's .NET prereq corrected to 10 (matches R-2 CI fix). Build command now targets the App csproj directly with `/p:Platform=x64`, which works because the App project explicitly declares x64 — same root cause as R-2's solution-level `Platform=x64` rejection (MSB4126). Documentation now matches CI behavior.

### R-6: Linux AppImage target is documented but not configured `[x]`
- **What**: README promises Linux AppImage output, but the Tauri bundle config now requests macOS artifacts by default and has no Linux-specific AppImage override.
- **Severity**: MEDIUM for release readiness. Linux release builds should request the documented artifact.
- **Branch**: `fix/r-6-linux-appimage-config`
- **Approach**: added `src-tauri/tauri.linux.conf.json` to override bundle targets to `["appimage"]` on Linux.
- **Tests**: JSON parse check; `grep -q '"appimage"' src-tauri/tauri.linux.conf.json`; `git diff --check`.
- **Files changed**: `src-tauri/tauri.linux.conf.json`, `.review/findings/R-6.md`, `REVIEW.md`.
- **Known gaps**: AppImage packaging still needs validation on a Linux host.
- **Reviewer verdict**: VERIFIED via merge commit `d136898`; stale sentinel/bookkeeping completed after the merge left `.review/ready/R-6.json` in place. Linux-specific Tauri config now overrides bundle targets to `["appimage"]`, leaving the macOS app/dmg targets scoped to the default macOS path.

### R-7: Tauri release artifacts lack CI coverage `[x]`
- **What**: macOS `.app`/`.dmg` and Linux AppImage packaging are now configured, but no CI job exercises those release artifact paths.
- **Severity**: HIGH for release readiness. Release artifacts need native-runner build verification before 1.0.
- **Branch**: `fix/r-7-tauri-bundle-ci`
- **Approach**: added a GitHub Actions workflow with native macOS and Ubuntu jobs that install frontend/Rust/Tauri prerequisites, build the documented Tauri artifacts, verify the outputs exist, and upload them.
- **Tests**: YAML parse check; greps for macOS/Linux build commands and Linux WebKit dependency; `git diff --check`.
- **Files changed**: `.github/workflows/tauri-bundles.yml`, `.review/findings/R-7.md`, `REVIEW.md`.
- **Known gaps**: full validation requires GitHub-hosted macOS and Linux runners.
- **Reviewer verdict**: VERIFIED via merge commit `f806555`. YAML parses; `--bundles app,dmg` / `--bundles appimage` syntax matches Tauri v2 CLI 2.11.1; `productName=ChickenScratch` (no space) produces exactly the `.app` path and `ChickenScratch_*.dmg` glob the verification step asserts; `libwebkit2gtk-4.1-dev` is the correct Tauri v2 Linux WebKit pin and is available on ubuntu-22.04; `beforeBuildCommand` handles the frontend build so the workflow only needs `npm ci`. macOS DMG will be unsigned (CI verification only, not distributable as-is).

### R-8: Release process lacks a canonical runbook `[x]`
- **What**: Release validation, version alignment, tag creation, artifact builds, and Arch checksum updates are spread across memory and review notes instead of one release gate.
- **Severity**: MEDIUM for release readiness. The final 1.0 cut needs a repeatable checklist.
- **Branch**: `fix/r-8-release-runbook`
- **Approach**: added `RELEASE.md` with version metadata paths, validation commands, platform artifact builds, tag commands, Arch checksum procedure, and post-build smoke checks.
- **Tests**: grep checks for Tauri, Arch checksum, and Windows build commands; `git diff --check`.
- **Files changed**: `RELEASE.md`, `.review/findings/R-8.md`, `REVIEW.md`.
- **Known gaps**: this does not bump versions or compute Arch checksums before a release tag exists. Runbook validation step omits `chickenscratch-linux` (cxx-qt Qt6 frontend) — Linux frontend is built via PKGBUILD/native host.
- **Reviewer verdict**: VERIFIED via merge commit `b89ce21`. All version files listed contain `version = "0.1.0-alpha"`; cargo `-p` flags resolve to real workspace packages; `crates/core/tests/cross_frontend/run.sh` exists; PKGBUILD currently has placeholder `sha256sums=('SKIP')` and placeholder url (runbook correctly mandates fixing both before tag); Windows commands match R-2/R-4 csproj-scope `/p:Platform=x64` + `.slnx` solution; Tauri commands match R-5/R-6/R-7 bundle config; smoke checks cover M-1/M-3/C-3 high-risk paths.

### R-9: Core release validation lacks CI coverage `[x]`
- **What**: `RELEASE.md` defines local Rust/UI/cross-frontend validation gates, but no CI workflow runs them on source changes.
- **Severity**: HIGH for release readiness. 1.0 needs continuous validation separate from packaging artifact builds.
- **Branch**: `fix/r-9-core-validation-ci`
- **Approach**: added a macOS GitHub Actions validation workflow for Rust fmt/clippy/tests, UI lint/build, and the cross-frontend harness with Swift and .NET 10 present.
- **Tests**: YAML parse check; greps for fmt/clippy and cross-frontend coverage assertion; `git diff --check`.
- **Files changed**: `.github/workflows/validation.yml`, `.review/findings/R-9.md`, `REVIEW.md`.
- **Known gaps**: Linux Qt frontend remains outside the root validation suite because it requires Qt/cxx-qt native dependencies.
- **Reviewer verdict**: VERIFIED via merge commit `283ef00`. YAML parses; cargo `-p` flags resolve to real workspace packages; `crates/core/tests/cross_frontend/run.sh` honors `CHIKN_CROSS_FRONTEND_WORKDIR` (line 5-6) and emits `writer-toolchains-ran:2/2` (line 176); the harness ran locally end-to-end with the workflow's grep assertion passing; macOS runner is correct (Swift writer requires it; .NET 10 installs cleanly via setup-dotnet@v4); Node 24 and .NET 10 align with R-2/R-7 fixes; trigger paths cover all surfaces named in `RELEASE.md`. Covers the non-packaging RELEASE.md gates exactly.

### R-10: Release metadata lacks a deterministic tag and checksum preflight `[x]`
- **What**: The Arch package still used placeholder upstream/source metadata and the release process had no executable guard for final version/tag/checksum drift.
- **Severity**: HIGH for release readiness. A 1.0 tag should not be cut while package source URLs or checksums are still placeholders.
- **Branch**: `fix/r-10-release-metadata-preflight`
- **Approach**: added release metadata and source archive scripts, fixed the PKGBUILD source shape to use GitHub release assets, excluded the PKGBUILD from release source archives, wired the metadata check into CI, and documented the checksum-safe release sequence.
- **Tests**: prerelease metadata check; expected-failure release-mode check for current 1.0 blockers; script syntax checks; workflow YAML parse; source archive export-ignore check; `git diff --check`.
- **Files changed**: `.gitattributes`, `.github/workflows/validation.yml`, `RELEASE.md`, `pkg/arch/PKGBUILD`, `scripts/check-release-metadata.sh`, `scripts/create-release-source.sh`, `.review/findings/R-10.md`, `REVIEW.md`.
- **Known gaps**: does not bump to `1.0.0`, create a tag, or pin the final Arch checksum.
- **Reviewer verdict**: VERIFIED via merge commit `afafa7e`. Prerelease check passes for current `0.1.0-alpha` state; `--release 1.0.0` produces 11 actionable errors covering every Cargo.toml version, tauri.conf.json, README status, PKGBUILD pkgver/SKIP/64-char-sha check; `--require-tag` adds a 12th error for missing local tag. Source archive correctly includes `pkg/arch/chickenscratch.desktop` and excludes `pkg/arch/PKGBUILD` (chicken-and-egg) and `.review/` via `.gitattributes export-ignore`. PKGBUILD now uses Arch-safe `pkgver=0.1.0_alpha` with `_upstream_version="${pkgver//_/-}"` to drive the GitHub release-asset URL. `validation.yml` step `Release metadata` runs the prerelease check on every push. `RELEASE.md` restructured into Stage 4 (prepare archive + pin checksum before tag) → Stage 5 (cut tag → regenerate archive from tag → `--require-tag` confirmation), which works because `git archive` is deterministic for the same tree.

### R-11: Final 1.0 release metadata is still alpha `[x]`
- **What**: All package/app versions and public status text still advertise `0.1.0-alpha`, and the Arch package checksum is still `SKIP`.
- **Severity**: HIGH for release readiness. The repository cannot be the base of a `v1.0.0` tag until version metadata and package checksum are final.
- **Branch**: `fix/r-11-version-1-0-release-metadata`
- **Approach**: bumped release metadata to `1.0.0`, updated README/RELEASE wording, extended the metadata checker so default mode enforces release rules for non-prerelease versions, excluded `REVIEW.md` from source archives, and pinned the deterministic source archive SHA-256 in `pkg/arch/PKGBUILD`.
- **Tests**: locked cargo metadata; release metadata checks; expected `--require-tag` failure for missing local tag; source archive checksum/export-ignore check; stale alpha version grep; `git diff --check`.
- **Files changed**: `.gitattributes`, `Cargo.lock`, `README.md`, `RELEASE.md`, `crates/core/Cargo.toml`, `crates/cli/Cargo.toml`, `crates/tui/Cargo.toml`, `linux/Cargo.toml`, `pkg/arch/PKGBUILD`, `scripts/check-release-metadata.sh`, `scripts/create-release-source.sh`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `.review/findings/R-11.md`, `REVIEW.md`.
- **Known gaps**: does not create or push `v1.0.0`.
- **Reviewer verdict**: VERIFIED via merge commit `dd5511c`. `cargo metadata --locked` confirms all 5 workspace packages at `1.0.0`; `scripts/check-release-metadata.sh` default mode now infers release rules from non-prerelease string and passes; `--require-tag` produces exactly ONE error (missing local `v1.0.0`), proving every other gate is satisfied. PKGBUILD pinned SHA `a3f63fd…3334` matches `scripts/create-release-source.sh 1.0.0 HEAD` exactly — and the tarball-from-merge-commit produces the same SHA, so reviewer verdicts can't poison the pin (REVIEW.md is now `export-ignore`). `create-release-source.sh` resolves ref→tree first and pins `--mtime=epoch`, so a tarball from `HEAD` and from the eventual `v1.0.0` tag will be byte-identical given the same tree. No `0.1.0-alpha` strings remain in release-tracked files.

### R-12: macOS DMG release command is not CI-safe `[x]`
- **What**: `cargo tauri build --bundles app,dmg` can fail in headless release automation because Tauri's DMG helper runs Finder AppleScript unless `CI=true`.
- **Severity**: MEDIUM for release readiness. The documented macOS artifact command should match the CI-safe path that actually produced the DMG.
- **Branch**: `fix/r-12-ci-safe-macos-dmg`
- **Approach**: set `CI=true` in the macOS bundle workflow build step, changed the release runbook to use `CI=true cargo tauri build --bundles app,dmg`, and updated the Arch source checksum after the release-source-included docs/workflow change.
- **Tests**: workflow YAML parse; greps for `CI=true` runbook/workflow wiring; frontend install; `CI=true cargo tauri build --bundles app,dmg`; app and DMG artifact checks; source archive checksum matches PKGBUILD pin; release metadata check; `git diff --check`.
- **Files changed**: `.github/workflows/tauri-bundles.yml`, `RELEASE.md`, `pkg/arch/PKGBUILD`, `.review/findings/R-12.md`, `REVIEW.md`.
- **Known gaps**: Finder AppleScript DMG layout still requires an interactive GUI session; release automation now avoids that path.
- **Reviewer verdict**: VERIFIED via merge commit `a4a022b` after fix-up `d2aada7` re-pinned the Arch checksum. First pass reopened: original branch shipped the `CI=true` wiring but kept R-11's pre-R-12 PKGBUILD pin `a3f63fd…3334`, while the changes to `RELEASE.md` and the workflow YAML (both inside the release tarball) had bumped the actual archive SHA to `f9e06fe…b47125` — would have failed `makepkg --verifysource` and Stage 5 `--require-tag`. After the re-pin: `create-release-source.sh 1.0.0 HEAD` output matches PKGBUILD exactly; all metadata gates pass; `--require-tag` produces exactly one error (missing local tag). The reopen also exposed a latent gap in `check-release-metadata.sh` (verifies pin shape, not pin-vs-archive equality) — worth a follow-up finding now that R-12 is in.

### R-13: Release metadata check does not compare pinned checksum to archive bytes `[x]`
- **What**: `scripts/check-release-metadata.sh` verifies that `sha256sums` is a pinned 64-character SHA, but did not compare it with the archive generated by `scripts/create-release-source.sh`.
- **Severity**: HIGH for release readiness. A tarball-included file can change after pinning and silently leave `PKGBUILD` with a stale checksum.
- **Branch**: `fix/r-13-verify-source-checksum`
- **Approach**: made release-mode metadata checks generate the source archive and compare its SHA to the PKGBUILD pin; `--require-tag` compares against `v<version>` after confirming the tag exists; re-pinned the checksum after changing the checker script.
- **Tests**: shell syntax; default and explicit release metadata checks; expected missing-tag-only `--require-tag` failure; source archive checksum matches PKGBUILD pin; `git diff --check`.
- **Files changed**: `scripts/check-release-metadata.sh`, `pkg/arch/PKGBUILD`, `.review/findings/R-13.md`, `REVIEW.md`.
- **Known gaps**: does not create or push `v1.0.0`.
- **Reviewer verdict**: VERIFIED via merge commit `0639da6`. Closes the gap flagged in R-12's verdict. Release-mode check now runs `create-release-source.sh "$expected" "$archive_ref" ...` and asserts the first-line SHA equals the PKGBUILD `sha256sums` value; `archive_ref` is `HEAD` by default and `v$expected` when `--require-tag` is used (only after confirming the tag exists, otherwise the archive comparison is skipped to keep the missing-tag error clean). Drift simulation: corrupted PKGBUILD pin produces a precise error naming both the pin and the actual `HEAD` archive SHA. New pin `4af64eb…ddeb47` matches the post-R-13 tree exactly. Going forward, any change to a tarball-included file will fail this gate at PR time, surfacing the re-pin requirement before release.

---

## Beta-readiness audit (2026-05-17)

After R-1..R-13 closed the release-tooling gaps, a fresh four-domain review (data integrity / security / quality / UX) surfaced three stop-ship items for a 0.1.0 beta with real writers. A follow-up four-agent pass added the remaining open items below. These are *app-level* and release-readiness gaps, distinct from the already-verified remediation work above.

### R-14: Cross-frontend path validation drift — macOS Swift and Windows C# writers bypass Rust safe_path `[x]`
- **What**: The Rust core's `crates/core/src/core/project/safe_path.rs` hardening (component-by-component validation, symlink rejection, canonicalize-under-root) is reached only by Tauri commands. `macos/Sources/ChiknKit/Writer.swift:88-89` does `project.path.appendingPathComponent(document.relativePath)` + `content.write(to:)` with no `..`/symlink check. `windows/ChickenScratch.Core/IO/ProjectWriter.cs:54-58` does `Path.Combine(projectPath, doc.Path)` + `File.WriteAllText` — `Path.Combine` with a rooted second segment discards the project root, and `..` is not stripped.
- **Severity**: HIGH for beta security. A shared/cloud-sync'd `.chikn` whose `project.yaml` has `path: "../../etc/launchd.conf"` (mac) or `path: "..\\..\\Users\\victim\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\Startup\\go.bat"` (Windows) overwrites the target on first edit-save in the native UI. Symlinks already on disk in `manuscript/` would similarly redirect saves. Beta testers swapping projects with each other is the threat model.
- **Branch**: `fix/r-14-native-path-validation`
- **Approach**: added shared safe-path helpers for Swift and C# native frontends and hardened the Rust reader side. All three now reject absolute document paths, parent/current directory components, symlink/reparse ancestors, symlinked `.md`/`.meta` targets, and canonical paths outside the project root before native read/write/delete operations.
- **Tests**: Rust reader path/symlink unit tests; Swift `ChiknKitChecks`; Windows safe-path harness; full Rust suite; clippy; UI lint/build; cross-frontend harness; release metadata checksum gate after re-pin.
- **Files changed**: `crates/core/src/core/project/reader.rs`, `macos/Sources/ChiknKit/SafeProjectPath.swift`, `macos/Sources/ChiknKit/Reader.swift`, `macos/Sources/ChiknKit/Writer.swift`, `macos/Sources/ChiknKit/Models.swift`, `macos/Tests/ChiknKitChecks/main.swift`, `windows/ChickenScratch.Core/IO/SafeProjectPath.cs`, `windows/ChickenScratch.Core/IO/ProjectReader.cs`, `windows/ChickenScratch.Core/IO/ProjectWriter.cs`, `windows/ChickenScratch.Core/IO/DocumentService.cs`, `windows/ChickenScratch.Core.Tests/CrossFrontendHarness/Program.cs`, `.review/findings/R-14.md`, `REVIEW.md`, `pkg/arch/PKGBUILD`.
- **Known gaps**: Linux Qt frontend remains out of scope for this branch. The cross-frontend harness still logs existing hierarchy repair after Swift/C# writer passes; that is tracked separately as R-18, not introduced here. Windows symlink-rejection tests skip silently on hosts without symlink privilege (CI must run as admin/Developer Mode) — flag this as a CI config note.
- **Reviewer verdict**: VERIFIED via merge commit `e77051c`. Deep security review confirms a four-layer defense at each trust boundary: (1) component-split rejection of empty/`.`/`..`/rooted segments (Swift `SafeProjectPath.swift:60-81`, C# `SafeProjectPath.cs:103-123`, Rust `reader.rs:737-773`); (2) `lstat`/`FileAttributes.ReparsePoint` checks at every parent component as walked, not only the leaf; (3) symlink/reparse checks on both `.md` and `.meta` leaves; (4) canonical within-project check via relative-path arithmetic (C# rejects `..`-prefixed results from `GetRelativePath`). C# helper rejects rooted second segments via `IsRooted` *before* `Path.Combine`, defeating the Windows `Path.Combine(root, "C:\\foo")` swallow. Rust reader's `validate_hierarchy_document_paths` runs **before** `read_all_documents` — a hostile `project.yaml` is rejected at load, closing the fingerprint-by-display path. Swift `documentURLs` reuses the same helper from `Reader.readDocument`, so enumeration can't smuggle a symlinked file past the read path (harness tests `linkdir` explicitly). PKGBUILD re-pinned proactively to `5c860ab…49c8a`. Native validations all pass: Swift `ChiknKitChecks` (23 sub-checks), Windows safe-paths harness, cross-frontend harness end-to-end with `writer-toolchains-ran:2/2`.

### R-15: Document `.md` and `.meta` writes are not atomic `[x]`
- **What**: `crates/core/src/core/project/writer.rs:580` (content) and `:583` (meta) use bare `fs::write` (truncate+write). Only `project.yaml` (`:230-231`) and `threads.yaml` (`:108-110`) use temp+rename. A power cut or OS kill during save leaves a truncated `.md` (silent prose loss) or partial `.meta` — and a corrupt `.meta` fails the entire project load (`reader.rs:613` propagates the YAML parse error). Combined with `writer.rs:237-245` rewriting every document on every save (`write_project` walks the full map), the crash window covers every document, not just the active one.
- **Severity**: HIGH for beta data integrity. Single-fault path can render a project unloadable, with content recoverable only via terminal `git log`.
- **Branch**: `fix/r-15-atomic-document-writes`
- **Approach**: routed Rust `project.yaml`, non-empty `threads.yaml`, `.md`, and `.meta` writes through a same-directory atomic helper using `tempfile` persist semantics, with fsync and temp cleanup. Rust reader quarantines corrupt document `.meta` files and falls back to hierarchy id/name so the project remains loadable without rewriting `project.yaml`. Windows native `WriteDocument` now stages `.md` and `.meta` sidecars before replace/move; Swift already used atomic writes and was left unchanged.
- **Tests**: `cargo test -p chickenscratch-core core::project::reader --lib`; `cargo test -p chickenscratch-core core::project::writer --lib`; Windows cross-frontend harness `--atomic-writes` and `--safe-paths`; full Rust/UI validation suite; `scripts/check-release-metadata.sh --release 1.0.0`.
- **Files changed**: `crates/core/Cargo.toml`, `crates/core/src/core/project/reader.rs`, `crates/core/src/core/project/writer.rs`, `windows/ChickenScratch.Core/IO/ProjectWriter.cs`, `windows/ChickenScratch.Core.Tests/CrossFrontendHarness/Program.cs`, `.review/findings/R-15.md`, `REVIEW.md`, `pkg/arch/PKGBUILD`.
- **Known gaps**: no portable two-file transaction across `.md` and `.meta`; this fixes truncate-in-place and adds recoverable corrupt sidecars. Native malformed `.meta` policy convergence remains R-19.
- **Reviewer verdict**: VERIFIED via merge commit `50b86e8`. `atomic_write_file` (writer.rs:584) is textbook correct: same-directory tempfile via `tempfile::Builder::tempfile_in(parent)`, `write_all`, permission preservation, `sync_all()` on temp, cross-platform `persist()` (atomic rename on Unix, `MoveFileExW`+`REPLACE_EXISTING` on Windows via `tempfile` crate), Unix-only parent-dir fsync to commit rename metadata. Auto-cleanup via `tempfile` drop semantics on early return (`test_atomic_write_file_removes_temp_after_replace_failure` proves it). `write_document` serializes the meta YAML **before** either write (line 575: `serde_yaml::to_string(&metadata)?`), so a serialization failure leaves both files untouched. Corrupt-meta quarantine renames to `.meta.corrupt-<uuid>` (reader.rs:851) and continues with default metadata using hierarchy id/name — closes D-8 from data integrity audit. Tests: reader 21/21 (+2 new), writer 26/26 (+6 new incl. atomicity, no-temp-leak, replace-failure, corrupt-meta-rejection), cross-frontend harness 2/2 toolchains, Windows `--atomic-writes` harness pass. `tempfile` moved from `[dev-dependencies]` to a runtime dep — necessary and small. PKGBUILD pre-pinned `a06ebdc…0c741`.

### R-16: Compile/export pipeline has zero test coverage `[x]`
- **What**: `crates/core/src/core/compile.rs` (224 lines, `pub fn compile` at `:43`) has no inline tests and no integration tests in `crates/core/tests/`. For a writing app, "compile manuscript to .docx/.md/.pdf" is the day-one validation a writer runs against an imported project. A silent regression here is the most likely cause of a "ChickenScratch corrupted my book" support ticket.
- **Severity**: HIGH for beta confidence. Riskiest untested surface in the codebase; closes the scariest unknown for beta release.
- **Branch**: `fix/r-16-compile-coverage`
- **Approach**: added `crates/core/tests/compile_roundtrip.rs`, which drives the public `compile` entry point against a synthetic on-disk `.chikn` project and a local pandoc shim. It validates title-page assembly, metadata args, manuscript ordering, section separators, include/exclude filtering, advertised format target mapping, and graceful empty-manuscript failure.
- **Tests**: `cargo test -p chickenscratch-core --test compile_roundtrip`; `cargo fmt --all -- --check`; `cargo test -p chickenscratch-core -p chickenscratch -p chickenscratch-tui -p chikn-converter --lib --bins --tests`; `cargo clippy -p chickenscratch-core -p chickenscratch -p chickenscratch-tui -p chikn-converter --all-targets -- -D warnings`; `cd ui && npm run lint`; `cd ui && npm run build`; `scripts/check-release-metadata.sh --release 1.0.0`.
- **Files changed**: `crates/core/tests/compile_roundtrip.rs`, `.review/findings/R-16.md`, `REVIEW.md`, `pkg/arch/PKGBUILD`.
- **Known gaps**: does not validate PDF rendering fidelity or real pandoc output bytes; that needs a pandoc/LaTeX runtime in CI and belongs in a separate packaging smoke test.
- **Reviewer verdict**: VERIFIED via merge commit `f4bdec2`. The pandoc-shim approach (fake binary that logs args + copies input to output) lets the test assert ChickenScratch's compile *contract* without needing pandoc in CI — `--standalone`, `-f markdown`, `-t <fmt>`, `-o <path>`, and metadata args (`title=...`, `author=...`) are all verified. Happy-path test exercises ordering (compile_order=1 sorts before compile_order=2), `***` separator insertion, `include_in_compile=false` exclusion, non-manuscript-folder exclusion, title page + author + word count + `\newpage`. Format-mapping test loops every advertised format and asserts pandoc target (incl. `epub` → `epub3`). Empty-content test asserts `ChiknError::InvalidFormat` AND that no partial output file is left behind — the "compile failed silently" failure mode is closed. GPT did more than the spec required: contract-level pandoc invocation assertions, not just structural invariants. 3/3 tests pass. PKGBUILD pre-pinned (`0dc0360b…d1d8d`).

### R-17: Remaining git mutation paths bypass dirty-worktree, safe-path, and atomic-write protections `[x]`
- **What**: H-3 guarded `restore_revision` and `sync_pull_force`, but other git paths still mutate the worktree without the same protections. `crates/core/src/core/git.rs:341-390` restores one document via raw `project_path.join(doc_path)` + `std::fs::write` for `.md` and `.meta`, bypassing writer symlink/path validation and R-15's planned atomic writes. `create_draft` / `switch_draft` / `merge_draft` fast-forward / `sync_pull` fast-forward still force-checkout at `git.rs:455`, `:498`, `:565-568`, and `:853-856` without `reject_dirty_worktree`. The UI flushes pending editor edits before these operations, but a flush creates dirty files, not a revision.
- **Severity**: HIGH for beta data integrity and targeted overwrite risk. A symlinked restored document can write outside the project, and draft/pull operations can discard uncommitted but freshly flushed prose.
- **Branch**: `fix/r-17-git-mutation-guards`
- **Approach**: routed `restore_document` through a scoped writer helper that applies safe document path validation, symlink rejection, and atomic writes for historical `.md` / `.meta` blobs. Added dirty-worktree guards before branch creation, draft switching, draft merge mutation, and pull mutation; no-op/up-to-date merge/pull paths still return without requiring a clean worktree.
- **Tests**: added dirty-worktree rejection tests for `restore_document`, `create_draft`, `switch_draft`, `merge_draft` fast-forward, and `sync_pull` fast-forward; added Unix symlink restore tests for document and `.meta` targets proving outside files are untouched. Ran focused `remote_sync`, full Rust validation, UI lint/build, and release metadata check.
- **Files changed**: `crates/core/src/core/git.rs`, `crates/core/src/core/project/writer.rs`, `crates/core/tests/remote_sync.rs`, `.review/findings/R-17.md`, `REVIEW.md`, `pkg/arch/PKGBUILD`.
- **Known gaps**: `sync_pull` may fetch remote-tracking refs before rejecting a dirty worktree; this does not overwrite local files or advance the local branch. Force pull remains intentionally destructive after confirmation, guarded only against pre-existing dirty worktree state.
- **Reviewer verdict**: VERIFIED via merge commit `a23c2d0`. All six git mutation paths now guarded — concrete tests with explicit no-side-effect assertions: `restore_document_rejects_dirty_worktree_without_clobbering_file`, `restore_document_rejects_symlink_document_without_touching_outside_file`, `restore_document_rejects_symlink_meta_without_touching_outside_file`, `create_draft_rejects_dirty_worktree_and_does_not_create_branch`, `switch_draft_rejects_dirty_worktree_without_switching_or_clobbering`, `merge_draft_fast_forward_rejects_dirty_worktree_without_advancing_head`, `sync_pull_fast_forward_rejects_dirty_worktree_without_advancing_head`. Each rejection assertion is paired with the absence-of-side-effect assertion, so the test proves the guard isn't just early-returning after the damaging action. `restore_document` now goes through a `pub(crate)` writer helper for raw blob writes, so historical `.md`/`.meta` writes inherit R-14 path validation + R-15 atomic semantics. 17/17 `remote_sync` integration tests pass. PKGBUILD pre-pinned `b5262f7…b57c`.

### R-18: Cross-frontend hierarchy tag drift is still hidden by a permissive harness `[x]`
- **What**: Rust serializes `TreeNode` with canonical `type: Folder` / `type: Document` (`crates/core/src/models/hierarchy.rs:16-25`) and only aliases lowercase on read. Swift accepts only lowercase raw values in `macos/Sources/ChiknKit/Reader.swift:64-70`, then rewrites `project.yaml` from whatever hierarchy it decoded (`Writer.swift:643-648`, `:740-749`). In a live harness run, Rust loaded Corn with 3 top-level nodes; after Swift wrote metadata, Rust had to repair 16 orphaned docs and add standard folders. The default CI harness still passes because `crates/core/tests/cross_frontend_round_trip.rs:27-31` only asserts that documents exist, and `.github/workflows/validation.yml:86-92` does not set `CHIKN_CROSS_FRONTEND_FAIL_ON_REPAIR=1`.
- **Severity**: HIGH for cross-frontend data integrity. A Swift metadata save can erase the Rust-authored binder structure from `project.yaml` while the harness reports success.
- **Branch**: `fix/r-18-hierarchy-tags`
- **Approach**: Swift now decodes hierarchy `type` tags case-insensitively, accepting Rust canonical `Folder` / `Document` and native lowercase forms. The cross-frontend harness records a Rust-converter baseline of hierarchy document ancestry/id/path and requires Swift and C# writer passes to preserve it exactly. CI now runs the harness with `CHIKN_CROSS_FRONTEND_FAIL_ON_REPAIR=1`.
- **Tests**: added Swift canonical-tag preservation fixture and Rust harness ancestry/id/path verifier. Ran Swift `ChiknKitChecks`, strict cross-frontend harness with both Swift and C# writers, focused Rust verifier test, full Rust tests, clippy, UI lint/build, and release metadata check.
- **Files changed**: `macos/Sources/ChiknKit/Reader.swift`, `macos/Tests/ChiknKitChecks/main.swift`, `crates/core/tests/cross_frontend/run.sh`, `crates/core/tests/cross_frontend_round_trip.rs`, `.github/workflows/validation.yml`, `.review/findings/R-18.md`, `REVIEW.md`, `pkg/arch/PKGBUILD`.
- **Known gaps**: the harness now preserves hierarchy document ancestry/id/path, but it does not compare every non-document folder attribute beyond the ancestry path needed to locate each document.
- **Reviewer verdict**: VERIFIED via merge commit `d866896`. The same `CHIKN_CROSS_FRONTEND_FAIL_ON_REPAIR=1` env that originally proved the bug (Swift writer pass triggered orphan repair of 16 docs) now passes on this branch: 16 docs, 3 top-level nodes, no repair markers. Swift `ChiknKitChecks` fixture `"rust canonical hierarchy tags survive Swift metadata write"` asserts three sub-properties: canonical Folder tag loads, canonical Document tag stays under folder, canonical Document path loads. Validation workflow now exports `CHIKN_CROSS_FRONTEND_FAIL_ON_REPAIR=1` so future regressions fail CI rather than emitting silent repair log lines. Rust harness now fingerprints document ancestry+id+path and requires both writer passes to preserve the Rust-converter baseline exactly. PKGBUILD pre-pinned `4a29269…65f0`.

### R-19: Native readers still persist destructive repair/malformed document loss `[x]`
- **What**: Rust reader repair no longer persists missing-file pruning, but native readers still do. Windows removes missing hierarchy nodes in `windows/ChickenScratch.Core/IO/ProjectReader.cs:68-75` and `:286-296`, and native operations commonly `ReadProject` then `WriteProject`, which persists the smaller manifest. Swift silently drops `.md` files whose `.meta` is missing/corrupt/lacks `id` (`macos/Sources/ChiknKit/Reader.swift:155-159`) and turns content read errors into empty content (`:161`); the writer can then serialize document nodes with missing paths from `Writer.swift:740-746`.
- **Severity**: HIGH for beta data integrity. A sync blip, corrupt sidecar, or transient missing file can become permanent project.yaml damage after an unrelated native save.
- **Branch**: `fix/r-19-native-reader-repair`
- **Approach**: port Rust's non-destructive missing-file semantics to Swift and Windows. For corrupt/missing meta, either fail load cleanly or quarantine according to R-15; do not silently drop docs or rewrite document nodes with null/empty paths.
- **Tests**: Swift `ChiknKitChecks` now covers missing body preservation, missing/corrupt/no-id `.meta`, unreadable `.md`, and writer rejection of pathless hierarchy document nodes. Windows cross-frontend harness adds `--native-repair` for missing body preservation, missing/no-id metadata recovery, corrupt metadata rejection, and pathless hierarchy writer rejection. Also reran Windows `--safe-paths`, `--atomic-writes`, Swift build, Windows harness build, the full Rust/UI validation suite, and release metadata check.
- **Files changed**: `macos/Sources/ChiknKit/Models.swift`, `macos/Sources/ChiknKit/Reader.swift`, `macos/Sources/ChiknKit/Writer.swift`, `macos/Tests/ChiknKitChecks/main.swift`, `windows/ChickenScratch.Core/IO/ProjectReader.cs`, `windows/ChickenScratch.Core/IO/ProjectWriter.cs`, `windows/ChickenScratch.Core.Tests/CrossFrontendHarness/Program.cs`, `.review/findings/R-19.md`, `REVIEW.md`, `pkg/arch/PKGBUILD`.
- **Known gaps**: Swift fails closed instead of quarantining corrupt `.meta` files; this prevents destructive native rewrites, but exact quarantine behavior is not yet shared across languages. Linux native delete/repair remains R-20.
- **Reviewer verdict**: VERIFIED via merge commit `f745ef7`. Swift `TreeNode` retains hierarchy `path`, so a missing `.md` body is preserved through a round-trip — concrete test `"missing hierarchy document body is not pruned on native write"` proves the project.yaml `id` and `path` survive a re-write after the body is deleted. Three fail-closed cases verified: missing `.meta` rejects load, corrupt YAML rejects load, missing `id` rejects load. Unreadable `.md` (directory at path) rejects load. Writer rejects hierarchy nodes lacking resolvable paths. Windows cross-frontend `--native-repair` harness passes covering the same matrix (missing body preserved, missing/no-id meta recovery, corrupt meta rejection, pathless writer rejection). Cross-frontend round-trip remains clean with `writer-toolchains-ran:2/2`. PKGBUILD pre-pinned `585365a…edbd`. Swift uses fail-closed instead of R-15's quarantine — different policy, same outcome for the data-integrity-impacting paths.

### R-20: Linux native delete removes manifest entries only; deleted files resurrect on reload `[x]`
- **What**: `linux/src/bridge.rs:670-687` removes a node from hierarchy, removes its id from `project.documents`, then calls `writer::write_project`. It never calls `writer::delete_document`; the helper at `bridge.rs:1391-1399` only mutates the hierarchy vector. On next load, Rust reader sees the still-on-disk `.md` as an orphan and repairs it back into the hierarchy.
- **Severity**: HIGH for Linux native data integrity. Delete appears to work, but prose remains on disk and can reappear, producing misleading git history.
- **Branch**: `fix/r-20-linux-delete-disk-files`
- **Approach**: added shared core `project::deletion::delete_node` that finds the subtree, deletes every descendant `.md` and `.meta` through `writer::delete_document`, and only then prunes hierarchy/document-map entries. Linux `delete_node` now calls that helper, writes the project after successful file deletion, and clears the active editor when any deleted descendant was active.
- **Tests**: added core deletion test for a folder with a nested document: asserts `.md` and `.meta` are removed, `project.documents` is pruned, `project.yaml` is rewritten, and `reader::read_project` does not repair the deleted doc back in. Ran focused core test, full Rust validation, UI lint/build, and release metadata check. Attempted `cargo check -p chickenscratch-linux`; it fails in `cxx-qt`/QtCore on this macOS host before project code (`qyieldcpu.h` implicit `__yield` under `-Werror`).
- **Files changed**: `crates/core/src/core/project/deletion.rs`, `crates/core/src/core/project/mod.rs`, `linux/src/bridge.rs`, `.review/findings/R-20.md`, `REVIEW.md`, `pkg/arch/PKGBUILD`.
- **Known gaps**: direct Linux Qt validation still needs a Linux/Qt host or fixed macOS Qt/cxx-qt toolchain. Tauri still has a private duplicate recursive delete helper; this branch keeps scope to Linux plus testable shared core behavior.
- **Reviewer verdict**: VERIFIED via merge commit `96821dd`. New `core::project::deletion::delete_node` (`crates/core/src/core/project/deletion.rs`) has the correct ordering: walk subtree → delete files via `writer::delete_document` (inheriting R-14 safe-path validation) → only THEN prune hierarchy + document map. On filesystem error mid-way, the loaded `Project` remains structurally intact so the caller doesn't write a smaller manifest. Test `delete_folder_removes_descendant_files_and_prevents_repair_resurrection` covers the exact threat model: delete a folder with a nested document, assert `.md`+`.meta` removed AND `project.documents` pruned AND R-19's reader-repair does not bring it back. Linux bridge now calls the shared helper and clears active editor for any deleted descendant. Local `remove_from_hierarchy` helper deleted so behavior can't drift back to manifest-only. Linux Qt host validation remains acceptable gap (macOS cxx-qt toolchain failure is environmental, not project code). Tauri duplicate helper unification is follow-up work, not a regression. PKGBUILD pre-pinned `b298bd0…587e`.

### R-21: Duplicate document IDs/paths silently alias or overwrite content `[~]`
- **What**: `Project.documents` is a `HashMap<String, Document>` (`crates/core/src/models/project.rs:90-91`), and reader inserts by `.meta` id at `reader.rs:573-574`, so duplicate ids silently overwrite earlier docs. Writer path validation in `writer.rs:247-289` checks each target independently but not uniqueness; `write_all_documents` writes all map values (`writer.rs:237-242`) and duplicate `Document.path` entries race by map iteration order. Linux also generates new paths with plain `make_slug` and no collision check (`linux/src/bridge.rs:568-615`), unlike Tauri's `unique_slug`.
- **Severity**: HIGH for data integrity. Duplicate ids or paths from sync conflicts, hand edits, malicious projects, or Linux duplicate-title creation can hide documents or overwrite content unpredictably.
- **Branch**: `fix/r-21-duplicate-document-identity`
- **Approach**: reader hierarchy collection now rejects duplicate hierarchy document ids/paths, disk traversal rejects duplicate loaded ids/paths before `Project.documents` insertion, and hierarchy nodes are checked against loaded document id/path pairs without breaking R-19 missing-body preservation. Writer validation rejects document map key/id mismatches and duplicate normalized document paths before rewriting project files. Linux native document creation now uses the shared `unique_slug` helper.
- **Tests**: added reader duplicate-id, duplicate-hierarchy-path, and hierarchy id/path mismatch tests; added writer duplicate-path-before-write test; reused `utils::slug::unique_slug` coverage for Linux duplicate-title path generation. Ran focused core tests, full Rust tests, clippy, UI lint/build, and attempted Linux Qt check.
- **Files changed**: `crates/core/src/core/project/reader.rs`, `crates/core/src/core/project/writer.rs`, `linux/src/bridge.rs`, `.review/findings/R-21.md`, `REVIEW.md`, `pkg/arch/PKGBUILD`.
- **Known gaps**: direct Linux Qt validation still needs a Linux/Qt host or fixed macOS Qt/cxx-qt toolchain; this macOS host fails in `cxx-qt`/QtCore before project code.

### R-22: Scrivener exporter uses project name as an unchecked output path component `[x]`
- **What**: `export_to_scriv` passes `project.name` to `write_scrivx` (`crates/core/src/scrivener/exporter/mod.rs:64`), and `write_scrivx` writes `scriv_path.join(format!("{}.scrivx", project_name))` (`:206-207`). `project.name` is loaded from `project.yaml` without filename-component validation. A name such as `../OtherProject/OtherProject` can write the `.scrivx` outside the selected `.scriv` directory if the parent exists.
- **Severity**: HIGH for export-time arbitrary file placement near the chosen output directory.
- **Branch**: `fix/r-22-scrivener-export-path`
- **Approach**: derive the `.scrivx` filename from `output_path.file_stem()` and reject invalid output stems, instead of using untrusted `project.name` as a filesystem component. The project title can remain display metadata; it no longer controls the output path.
- **Tests**: `cargo test -p chickenscratch-core scrivener::exporter --lib`; full Rust test suite; clippy; UI lint/build; release metadata gate before review handoff.
- **Files changed**: `crates/core/src/scrivener/exporter/mod.rs`, `.review/findings/R-22.md`, `REVIEW.md`, `pkg/arch/PKGBUILD`.
- **Known gaps**: none for filesystem placement. The generated XML currently does not emit the project display title; R-22 only fixes the path primitive.
- **Reviewer verdict**: VERIFIED via merge commit `a4fc49f`. New `scrivx_filename_from_output_path` derives the filename from `scriv_path.file_stem()` and rejects empty / `.` / `..` / `/` / `\` / control-char / non-UTF-8 stems before constructing the join. Concrete attack test in `crates/core/src/scrivener/exporter/mod.rs::test_write_scrivx_uses_output_folder_name_not_project_name` passes hostile `project_name = "../victim/victim"` and asserts the `.scrivx` lands in `Export.scriv/Export.scrivx` with no `victim/victim.scrivx` outside the export root. Control-char rejection test covers newline. **PKGBUILD pin updated proactively this time** (`ff4eea0…ca33b8`) — GPT broke the R-12/R-26 recurrence pattern. R-13 gate passes; `--require-tag` produces exactly one error (missing tag). 4/4 exporter tests pass.

### R-23: Manual Backup reports success while omitting current work `[ ]`
- **What**: the Revisions Backup button calls `gitCmd.pushBackup` directly (`ui/src/components/revisions/Revisions.tsx:152-162`). It does not use `runWithEditorFlush`, does not save a revision, and backend `push_backup` only pushes the current branch (`src-tauri/src/commands/git.rs:93-100`). `backup_on_close` is safer because it commits dirty changes before pushing (`src-tauri/src/commands/git.rs:245-261`).
- **Severity**: HIGH for user trust and data recovery. A writer can click Backup, see "Backup complete", and still have neither pending debounce edits nor already-flushed uncommitted edits in the backup repo.
- **Approach**: route manual backup through the same flush + auto-revision + push flow as close/periodic backup, or block with a clear prompt until the user saves a revision.
- **Tests**: commit baseline, edit a doc, trigger manual backup, clone the backup repo, and assert the edited text is present after the fix.
- **Files changed (anticipated)**: `ui/src/components/revisions/Revisions.tsx`, `src-tauri/src/commands/git.rs`, tests or harness coverage.
- **Known gaps**: decide whether manual backup should create an auto revision message or require the user to name it.

### R-24: Document/flow switches do not await failed flushes before replacing buffers and clearing dirty state `[ ]`
- **What**: `flushPendingSave` throws on failed disk writes (`ui/src/components/editor/Editor.tsx:152-214`), but the document-load effect calls it without `await` before replacing editor content for flow entry/exit and single-doc switches (`Editor.tsx:280-349`). It then calls `setDirtyTracked(false)` after loading the new buffer. The Flow exit toolbar catches flush failure but exits flow anyway (`ui/src/components/editor/Toolbar.tsx:304-310`).
- **Severity**: HIGH for dirty-editor data loss under disk-full, permission, path-validation, or write failure conditions.
- **Approach**: make navigation/flow transitions await the flush and block buffer replacement/dirty clear on failure. Centralize document selection/flow exit through async guarded actions rather than synchronous `selectDocument`.
- **Tests**: mock `update_document_content` to reject, edit doc A, select doc B, and assert doc A remains visible/dirty or navigation is blocked. Repeat for flow exit.
- **Files changed (anticipated)**: `ui/src/components/editor/Editor.tsx`, `ui/src/components/editor/Toolbar.tsx`, `ui/src/stores/projectStore.ts`, callers that invoke `selectDocument`.
- **Known gaps**: this may require a small UI state for "navigation blocked by save failure" rather than only a toast.

### R-25: AI replacement deletes selected prose before the stream succeeds `[ ]`
- **What**: for polish/expand/simplify, `Toolbar.tsx:417-430` deletes the selected range first and then inserts streaming chunks. On `ai:error`, network failure, invalid key, or context cancellation, the catch at `Toolbar.tsx:439-442` only toasts; it does not restore the original `selectedText`. Autosave can persist the deletion as ordinary editor content.
- **Severity**: HIGH for AI-assisted editing data loss.
- **Approach**: keep the original selection until the first successful replacement is ready, or insert into a temporary transaction that can roll back. On stream failure/cancellation, restore `selectedText` and selection, and avoid autosaving a failed transform as a deletion.
- **Tests**: configure a failing endpoint, select text, run Polish, assert the original selected text remains. Repeat for navigation abort during replacement.
- **Files changed (anticipated)**: `ui/src/components/editor/Toolbar.tsx`, maybe `ui/src/commands/ai.ts`.
- **Known gaps**: brainstorm mode appends after selection and is lower risk; focus replacement ops first.

### R-26: Release checksum gate can be bypassed by path-filtered CI `[x]`
- **What**: R-13 added source archive SHA comparison in `scripts/check-release-metadata.sh`, but `.github/workflows/validation.yml:5-35` only runs on selected paths. Tarball-included files such as `README.md`, `LICENSE`, docs, and other workflow files can change the release archive SHA without triggering the validation job that checks `pkg/arch/PKGBUILD`.
- **Severity**: HIGH for release readiness. A docs/license/status-only PR can merge with a stale source checksum until an unrelated change happens to run validation.
- **Branch**: `fix/r-26-validation-path-filter`
- **Approach**: removed the `paths:` filters from `.github/workflows/validation.yml` for both `push` and `pull_request`, so the release metadata/checksum job runs for docs, license, workflow, and any other source-archive-affecting change. Left the Tauri bundle workflow scoped to bundle inputs; this finding is about the validation/checksum gate.
- **Tests**: `scripts/check-release-metadata.sh`; `cargo fmt --all -- --check`; `cargo clippy -p chickenscratch-core -p chickenscratch -p chickenscratch-tui -p chikn-converter --all-targets -- -D warnings`; `cargo test -p chickenscratch-core -p chickenscratch -p chickenscratch-tui -p chikn-converter --lib --bins --tests`; `cd ui && npm run lint && npm run build`; `git diff --check`.
- **Files changed**: `.github/workflows/validation.yml`, `.review/findings/R-26.md`, `pkg/arch/PKGBUILD`, `REVIEW.md`.
- **Known gaps**: release archive excludes `.review`, `REVIEW.md`, and `pkg/arch/PKGBUILD`; those can stay outside the trigger if desired.
- **Reviewer verdict**: VERIFIED via merge commit `0a8f581` after fix-up `112031f` re-pinned the PKGBUILD checksum. First pass reopened: the path-filter removal was correct but the workflow file itself is inside the release tarball, so removing the filter without re-pinning made the gate fail at merge time — exact same pattern as R-12. Fix-up replaces pin with `2f3da59…51c6fa` matching the post-R-26 tree. Same-pattern recurrence is a process signal: GPT pre-commit checklist should include `scripts/check-release-metadata.sh --release 1.0.0` whenever any non-`export-ignore` file changes. CI now runs on every push/PR, so the gate will catch this drift automatically at PR time going forward.

### R-27: Rust release/package builds do not enforce `Cargo.lock` `[ ]`
- **What**: the lockfile is valid today, but CI and packaging do not enforce it. Validation uses `cargo clippy` / `cargo test` without a locked preflight (`.github/workflows/validation.yml:72-76`), Tauri bundle jobs run `cargo tauri build` without a lockfile gate (`.github/workflows/tauri-bundles.yml:46-49`, `:103-104`), RELEASE.md documents release commands without `--locked`, and `pkg/arch/PKGBUILD:25` uses `cargo build --release -p chickenscratch` without `--locked`.
- **Severity**: HIGH for reproducible releases and clean-checkout confidence.
- **Approach**: add `cargo metadata --locked` to release metadata validation and run package/release Cargo builds in locked mode where supported. Keep `npm ci` as-is for the UI.
- **Tests**: stale `Cargo.lock` after a manifest change must fail validation, Tauri bundle CI, and Arch package build before any release artifact is produced.
- **Files changed (anticipated)**: `scripts/check-release-metadata.sh`, `.github/workflows/validation.yml`, `.github/workflows/tauri-bundles.yml`, `pkg/arch/PKGBUILD`, `RELEASE.md`.
- **Known gaps**: use a preflight if a wrapper command cannot forward `--locked` directly.

### R-28: Native Swift and Windows dependency resolution is not locked `[ ]`
- **What**: SwiftPM dependency `Yams` is declared as `from: "5.1.3"` and `macos/Package.resolved` is ignored by `.gitignore:25`, so clean CI can resolve a different package than local development. Windows uses floating package versions such as `Microsoft.WindowsAppSDK` `1.8.*` in `windows/ChickenScratch.App/ChickenScratch.App.csproj`, with no tracked `packages.lock.json` and no NuGet locked-mode restore in `.github/workflows/windows.yml:25-31`.
- **Severity**: HIGH for native build reproducibility and cross-frontend harness confidence.
- **Approach**: track `macos/Package.resolved`, stop ignoring it, pin Windows package versions exactly, enable NuGet lock files, and restore with locked mode in CI. Add checks that no package reference uses wildcard versions.
- **Tests**: clean checkout CI should fail if SwiftPM/NuGet lock files are missing or stale.
- **Files changed (anticipated)**: `.gitignore`, `macos/Package.resolved`, `windows/**/*.csproj`, `windows/**/packages.lock.json`, `.github/workflows/validation.yml`, `.github/workflows/windows.yml`.
- **Known gaps**: decide whether app-only Windows packages need separate lock treatment from core harness packages.

### R-29: macOS DMG release path has no signing/notarization gate `[ ]`
- **What**: Tauri config builds app/dmg artifacts (`src-tauri/tauri.conf.json:26-36`), and `.github/workflows/tauri-bundles.yml:46-62` uploads them, but there is no signing identity, hardened runtime, entitlements, notarytool credentials, notarization step, or verification. `RELEASE.md` documents artifact existence checks only, and `docs/ROADMAP.md` still lists macOS code signing as future work.
- **Severity**: HIGH for public macOS distribution; MEDIUM if CI artifacts are explicitly internal-only.
- **Approach**: add a release-only signing/notarization path with GitHub Actions secrets and verify with `codesign --verify --deep --strict`, `spctl --assess`, and `xcrun stapler validate`. If unsigned artifacts remain for CI smoke, separate smoke builds from distributable release builds in docs/workflows.
- **Tests**: release workflow should fail if the `.app` or `.dmg` is unsigned, unnotarized, or unstapled.
- **Files changed (anticipated)**: `.github/workflows/tauri-bundles.yml` or a release workflow, `src-tauri/tauri.conf.json`, signing assets/entitlements if needed, `RELEASE.md`.
- **Known gaps**: requires Apple Developer credentials; cannot be fully validated on local non-release runs.

### R-30: Subprocess timeout can still hang on child process trees that keep pipes open `[ ]`
- **What**: `output_bounded` returns from its polling loop when the direct child exits (`crates/core/src/utils/process.rs:112-114`) and then joins stdout/stderr reader threads without a join deadline (`:160-161`, `:209-213`). On timeout/output cap it kills only the direct child (`:117`, `:130`, `:140`). A wrapper/Pandoc process can spawn a child that inherits stdout/stderr, exits or is killed, and leaves the reader threads blocked forever.
- **Severity**: MEDIUM. M-3 bounded the common case, but process trees can still hang compile/import/Pandoc checks indefinitely.
- **Approach**: run subprocesses in a killable process group/job object and kill the whole group on timeout/cap. Keep a deadline while joining readers or use nonblocking reads that can be abandoned after process-tree kill.
- **Tests**: Unix helper test with `sh -c '(sleep 3600) &'` and a short timeout should return promptly. Add Windows job-object equivalent if possible.
- **Files changed (anticipated)**: `crates/core/src/utils/process.rs`, platform-specific tests.
- **Known gaps**: Windows process-tree handling needs Job Objects or an equivalent crate.

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

# Plan: Web shell — serve the existing app over HTTP (`chikn-web`)

**Status:** Queued 2026-10-23 as an owner side quest. Owner wording: "no
direction change, just the next side quest. plan it, add it to the queue".
The active phase (`docs/CURRENT_PHASE.md`, Engine hardening) is unchanged.
Owner 2026-10-24: "follow the queue and get as much as you can done" —
taken as the go for the queued slices, in order, one commit each.
Progress: slices 1 (services extraction), 2 (frontend transport and platform
seams), 3a (`chikn-web` server: dispatch, auth, gate, static UI) and 3b (web
Welcome: server project list, create by name, token gate) landed 2026-10-24.
The owner-usable milestone is reached: edit, save, revisions from a browser.
Slices 4a (export as download), 4b (import by upload), 4c (server folder
picker), 4d (Scrivener import in the browser) and 6 (embed-ui single binary,
user guide, release runbook, TODO/ROADMAP/DEVLOG records) landed 2026-10-24.
Slice 6 was taken before the optional slice 5 (AI streaming), which with
slice 7 (single-writer lease) remains open.

**Owner request (quote):**
> hi! I want to make a web app version of chickenscratch that I can access
> from anywhere (vpn, connection out of scope for this) to store notes, write
> markdown docs, etc. I think we're most of the way there. can you help? I
> need to control my note taking and text editing without relying on cloud
> services.

> no direction change, just the next side quest. plan it, add it to the queue

**Phase check:** [ ] Not part of the active phase's work order — queued
behind it by the owner as a side quest, phase unchanged. [x] Not paused:
nothing under "Explicitly paused" is touched (no vault/remote-sync design, no
guided-token question; the existing Settings > Remote feature is carried
through the shared layer unchanged, not redesigned).

**Invariants touched:** I2 (all format I/O stays in `chickenscratch-core`;
the server calls it in-process through a *moved*, not duplicated, command
layer), I3 (the React UI in `ui/` stays the one GUI — the server is a second
*shell* for the same page, as the TUI is a second UI; new features still land
once in `ui/`), I6 (no new mutation paths — every write still passes
`ProjectWriteLocks` → `ProjectTokens` → `WritePermit`), I7 (no git jargon
added), I9 (declared suite extended to the new crates). Not touched: I1, I4,
I5 (no on-disk format change). I8: governance edits are limited to recording
this owner-requested addition (ADR-007, crate map, one I2 table row) in the
slice that introduces it.

---

## [MODEL] Intent

`chikn-web` is one Rust binary that runs the engine in-process, serves the
already-built React UI (`ui/dist`) over HTTP, and answers the UI's command
calls as same-origin JSON requests instead of Tauri IPC. The owner runs it on
a machine they control, pointed at one "projects root" folder, reaches it
over their own VPN from any browser, and gets the same editor, binder,
search, revisions, drafts, stats and settings as the desktop app. Projects
stay plain `.chikn` folders (Markdown + YAML + embedded git) on that machine;
nothing is hosted elsewhere (`docs/PROJECT.md`: cloud/SaaS hosting is a
non-goal). The desktop app keeps working unchanged; both shells share one
command layer so neither can drift from the other.

## [MODEL] Evidence and constraints

Verified against `4404fea` (2026-10-23).

- **The UI is already a web page.** `ui/` is Vite + React + TipTap. Tauri
  coupling is confined to: `invoke` from `@tauri-apps/api/core` in the 11
  files under `ui/src/commands/` plus two direct calls in `ui/src/App.tsx`
  (`backup_on_close`, lines ~222 and ~319); `listen` from
  `@tauri-apps/api/event` in `ui/src/commands/ai.ts` only (events
  `ai:chunk`, `ai:done`, `ai:error`); `getCurrentWindow().onCloseRequested`
  in `App.tsx`, already gated by `isTauri()` with a `beforeunload` fallback;
  `@tauri-apps/plugin-dialog` `open`/`save` in `Welcome.tsx` (open project,
  new project, Scrivener import target), `Binder.tsx` (`handleImportFile`),
  `CompileDialog.tsx` (`handleExport`), `Settings.tsx` (backup folder);
  `@tauri-apps/plugin-shell` `open` in `Welcome.tsx` (Pandoc link) and
  `App.tsx` (Help button, dynamic import). `ui/eslint.config.js` already
  forbids `invoke` outside `src/commands/*` for components and stores.
- **The backend command layer is thin and uniform.**
  `src-tauri/src/commands/*.rs` holds 75 `#[tauri::command]` fns (~5.1k
  lines including tests). Each is a synchronous fn taking serde args plus
  `State<'_, ProjectWriteLocks>` / `State<'_, ProjectTokens>` (40 each) and,
  for AI streaming only, `tauri::AppHandle` + `State<'_, AiStreamRegistry>`.
  Bodies call `chickenscratch_core` (`reader`, `writer`, `git`, `hierarchy`,
  `fidelity`) and `chikn_converter`. The only Tauri API uses are `ai.rs`
  (`AppHandle::emit` in `emit_chunk`/`emit_done`/`emit_error`) and
  `project::pick_scriv_folder` (native dialog via `rfd` / `osascript`).
  Several bodies already exist as `*_inner(&ProjectWriteLocks,
  &ProjectTokens)` for the command-boundary tests (`load_project_inner`,
  `sync_abort_pull_inner`, `complete_merge_inner`,
  `force_resolve_merge_inner`) — exactly the shape a shared crate needs.
- **Arg naming.** Tauri renames snake_case Rust params to camelCase on the
  JS side; the UI sends `{ projectPath, docId, … }`. Nested struct fields
  (`Project`, `AppSettings`, …) keep serde's snake_case. The HTTP layer must
  reproduce exactly this: camelCase top-level keys, struct bodies untouched.
  Tauri ignores unknown top-level keys; so must the HTTP layer.
- **Error contract.** `ChiknError` serializes as its `Display` string
  (`crates/core/src/utils/error.rs`); Tauri rejects the `invoke` promise with
  that raw string and the UI renders it via `String(e)` or template literals.
  The web transport must reject with the same raw string (not an `Error`
  wrapper) or every message gains an "Error: " prefix.
- **`src-tauri` is bin-only** (no `lib.rs`); a server cannot reuse its
  bodies without linking Tauri (and WebKitGTK on Linux). The bodies must move
  to a Tauri-free library crate.
- **Settings and secrets are process-local.** `settings.rs` reads and writes
  `~/.config/chickenscratch/settings.json` and `recent.json` for the process
  user; secrets (AI key, remote token) live in the OS keyring (`keyring` 3.x,
  feature `linux-native-sync-persistent`). `load_app_settings_from_path`
  ignores keyring failures (`let _ =`); `save_app_settings_to_path`
  propagates them. A headless host may have no usable keyring; the features
  that need secrets (AI, remote sync) are not required for the notes use
  case.
- **Blocking work.** Bodies block (git2; Pandoc up to `PANDOC_TIMEOUT` =
  60 s; network for sync and AI). Under an async HTTP server they must run on
  blocking threads.
- **Single-instance assumption.** The UI operation barrier
  (`ui/src/commands/barrier.ts`) and whole-document saves
  (`update_document_content`) assume one UI instance per project. Backend
  `ProjectWriteLocks` serializes writes per project path but does not
  arbitrate between two browser tabs: two tabs on one document are
  last-writer-wins, the same as two desktop windows today.
- **Release metadata and the declared suite.**
  `scripts/check-release-metadata.sh` lists version files explicitly
  (`src-tauri`, `crates/core`, `crates/cli`, `crates/tui`); new crates must
  be added so versions stay aligned at `1.0.0`. The suite
  (`docs/AGENT-WORKFLOW.md` §5, `.agents/repo-guidance.md` Verification,
  `.github/workflows/validation.yml`) names crates by `-p`; new crates go in
  all three.
- **Dev box toolchain (2026-10-23):** cargo 1.99.1, node 26, no Pandoc
  installed — export/import tests must not require Pandoc.

## [MODEL] Approach

One concern per commit. Every slice leaves the declared suite green and the
desktop app behaviorally unchanged. Order: 1 → 2 → 3a → 3b → 4 → 5 → 6; 7 is
optional. Slice 1 is a broad mechanical refactor of the desktop backend; ask
for its go separately from the server.

### Slice 1 — Extract `chickenscratch-services` (pure refactor, no behavior change)

New library crate `crates/services/` (package `chickenscratch-services`,
version `1.0.0`). Move from `src-tauri/src/commands/`:

- `mod.rs`: `ProjectWriteLocks`, `ProjectTokens`, `project_lock_key`, and
  their tests — unchanged.
- Every command body as a plain `pub fn` with the same name and args, where
  `State<'_, T>` becomes `&T`. Return types unchanged. Keep
  `#[allow(clippy::too_many_arguments)]` where present.
- `settings.rs`, `templates.rs`, `threads.rs`, `search.rs`, `io.rs`,
  `git.rs`, `document.rs`, `project.rs` (minus `pick_scriv_folder`), and
  `ai.rs` with `AppHandle` replaced by a trait:

  ```rust
  pub trait AiStreamSink: Send + Sync + 'static {
      fn chunk(&self, request_id: &str, delta: &str);
      fn done(&self, request_id: &str);
      fn error(&self, request_id: &str, message: &str);
  }
  pub fn ai_transform_stream(
      sink: Arc<dyn AiStreamSink>, registry: &AiStreamRegistry,
      content: String, operation: String, request_id: String,
  ) -> Result<(), ChiknError>
  ```

  `stream_ollama` / `stream_anthropic` / `stream_openai` take
  `&dyn AiStreamSink` instead of `&AppHandle`. `AiStreamRegistry` stays
  `Clone + Default`.
- Command-boundary tests inside those files (`git.rs` `fresh_boundary()`
  tests, `mod.rs` lock tests, etc.) move with the bodies. Tests that assert
  Tauri config (`main.rs`: shell-open validator, `core:window:allow-destroy`
  capability) stay in `src-tauri`.
- `lib.rs` re-exports what the shells need so they do not depend on core
  directly: `pub use chickenscratch_core::{ChiknError, Project, Document,
  TreeNode, Thread, SessionTarget};` plus `ProjectWriteLocks`,
  `ProjectTokens`, `AiStreamRegistry`, `AiStreamSink`, `LoadedProject`,
  `ImportedProject`, `AppSettings`, `RecentProject`, and the stats/progress
  structs.
- Dependencies move: `chickenscratch-core`, `chikn-converter`, `serde`,
  `serde_json`, `serde_yaml`, `chrono`, `uuid`, `dirs`, `regex`, `reqwest`,
  `keyring`; dev-deps `tempfile`, `git2`, core `test-support`. `rfd` and
  `tauri*` stay in `src-tauri`.

`src-tauri` becomes genuinely thin: each `#[tauri::command]` wrapper derefs
`State` and calls `chickenscratch_services::<module>::<fn>`;
`ai_transform_stream` wraps `AppHandle` in a `TauriSink: AiStreamSink` using
the existing `emit` calls; `pick_scriv_folder` stays as-is. A small
`macro_rules!` for the 40 lock+token wrappers is acceptable only if
`cargo clippy -D warnings` stays clean and `generate_handler!` stays
readable; plain wrappers are fine.

Acceptance (besides the suite): `cargo tree -p chickenscratch-services -e
normal` contains no `tauri`; `src-tauri/Cargo.toml` no longer depends on
`chickenscratch-core` or `chikn-converter`; `grep -rn 'chickenscratch_core\|
chikn_converter' src-tauri/src` is empty; `grep -c 'tauri::command'
src-tauri/src/commands/*.rs` still totals 75; `ui/` untouched.

Paperwork in the same commit: workspace member in `Cargo.toml`;
`-p chickenscratch-services` on the clippy and test lines in
`docs/AGENT-WORKFLOW.md` §5, `.agents/repo-guidance.md` Verification, and
`.github/workflows/validation.yml`; `crates/services/Cargo.toml` in
`version_files` of `scripts/check-release-metadata.sh`;
`docs/ARCHITECTURE.md` crate map and layer diagram (a services box between
the UIs and the engine); `README.md` architecture tree.
`docs/adr/ADR-007-web-shell-shared-services.md` (from `docs/adr/template.md`):
decision = the command layer lives in `chickenscratch-services`; Tauri and
`chikn-web` are thin shells over it; `ui/` remains the single GUI; the web
shell is self-hosted and single-user. Status Accepted, dated the day slice 1
lands, quoting the owner request above. `docs/INVARIANTS.md` I2 table: Tauri
row reads "In-process Rust calls via `chickenscratch-services`" and a new row
`| Web shell (crates/web + ui/) | In-process Rust calls via
chickenscratch-services |` — same rule, one more customer.

### Slice 2 — Frontend transport and platform seams (no behavior change in Tauri)

- `ui/src/commands/transport.ts` exports `invoke<T>(cmd, args?, opts?)`,
  `listen<T>(event, handler): Promise<UnlistenFn>`, `isWeb(): boolean`,
  `supportsAiStreaming(): boolean`. When `isTauri()` (from
  `@tauri-apps/api/core`): delegate to Tauri `invoke` / `listen`. Otherwise:
  - `invoke` → `fetch('/api/cmd/' + cmd, { method: 'POST', headers:
    { 'Content-Type': 'application/json', Authorization: 'Bearer ' + token },
    body: JSON.stringify(args ?? {}), keepalive: opts?.keepalive })`.
    `2xx` → `res.json()`. Non-2xx → parse body as JSON; if a string,
    `throw body` (raw string, matching Tauri); else `throw res.statusText`.
    On `401`: clear the stored token, dispatch
    `window.dispatchEvent(new CustomEvent('cs-web-auth-required'))`, then
    throw. `App.tsx` passes `{ keepalive: true }` for `backup_on_close` so
    the request survives `beforeunload`.
  - Token source, in order: `?token=` on first load (store under
    localStorage key `cs-web-token`, then `history.replaceState` to strip
    it), else localStorage.
  - `listen` → one shared stream per page: `fetch('/api/events', { headers:
    { Authorization } })` parsing `text/event-stream` from `res.body`
    (do not use `EventSource`; it cannot send headers, which would force the
    token into the URL). Dispatch by `event:` name; return an unlisten fn.
    Until slice 5 the server answers `404`; `listen` then resolves to a
    no-op unlisten and `supportsAiStreaming()` is `false`.
- Replace `import { invoke } from "@tauri-apps/api/core"` in all 11
  `ui/src/commands/*.ts` and both `App.tsx` sites with imports from
  `./commands/transport`; `ai.ts` takes `listen` from transport. In
  `aiTransformStream`, when `!supportsAiStreaming()`: call
  `aiTransform(content, operation)`, deliver the whole result via one
  `onChunk`, honor `shouldContinue` / `abortSignal`. The Toolbar call sites
  (`ui/src/components/editor/Toolbar.tsx` ~419, ~439) need no change.
- `ui/src/platform.ts`: `pickFolder(opts)`, `pickSaveFile(opts)`,
  `pickOpenFile(opts)`, `openExternal(url)`, `registerCloseHandler(fn)`.
  Tauri branch = the current plugin-dialog / plugin-shell /
  `onCloseRequested` code moved verbatim. Web branch in this slice:
  `openExternal` → `window.open(url, '_blank', 'noopener,noreferrer')`;
  `registerCloseHandler` → the existing `beforeunload` fallback; the three
  pickers return `null` (callers already treat `null` as cancelled). The
  buttons that would silently do nothing in web mode (Import Scrivener,
  Import File, Export, Choose Backup Folder) show the toast "Not available in
  the web version yet" until slice 4. `Welcome.tsx`, `Binder.tsx`,
  `CompileDialog.tsx`, `Settings.tsx`, `App.tsx` switch to `platform.ts`.
- ESLint: a block over `src/**/*.{ts,tsx}` with `ignores:
  ['src/commands/transport.ts', 'src/platform.ts']` and
  `no-restricted-imports` `patterns: [{ group: ['@tauri-apps/*'] }]` with a
  message naming the two allowed files. This is the guard that keeps the
  doorway single; keep the existing component-level `invoke` rule too.
- Tests (vitest, jsdom): web `invoke` posts to `/api/cmd/<name>` with the
  bearer header and the exact args object; rejects with the raw string body
  on 4xx; fires `cs-web-auth-required` and clears the token on 401;
  `?token=` is captured and stripped; `aiTransformStream` falls back to one
  chunk when streaming is unsupported. Prove the ESLint guard bites with a
  deliberate violating import, then remove it.

### Slice 3a — `chikn-web`: dispatch, auth, path gate, static UI

New binary crate `crates/web/` (package `chickenscratch-web`,
`[[bin]] name = "chikn-web"`, version `1.0.0`). Dependencies:
`chickenscratch-services`, `axum` 0.8, `tokio` (rt-multi-thread, macros,
signal), `tower-http` (fs, set-header, trace), `clap` 4 (derive), `serde`,
`serde_json`, `tracing`, `tracing-subscriber`, `rand`, `subtle`, `dirs`.

CLI:
- `--root <DIR>` (required): projects root; canonicalized at startup; must be
  an existing directory.
- `--bind <ADDR>` default `127.0.0.1:7391`.
- `--token <STRING>` or env `CHIKN_WEB_TOKEN`; else read
  `~/.config/chickenscratch/web-token`; else generate 32 random bytes as
  base64url, write that file with mode 0600, and print once:
  `Access token: …` and `Open: http://<bind>/?token=…`.
- `--ui-dir <DIR>` default `ui/dist` relative to the current directory; if
  it has no `index.html`, exit 2 with "UI build not found — run `cd ui &&
  npm run build` or pass --ui-dir".

Routes:
- `GET /` and `/assets/*` → `ServeDir(ui_dir)` with `index.html` as the
  not-found fallback. Response headers: `Content-Security-Policy:
  default-src 'self'; connect-src 'self'; img-src 'self' data:; font-src
  'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self';
  object-src 'none'; base-uri 'self'; frame-ancestors 'none'`,
  `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`. No CORS
  headers (same-origin only; bearer-in-header means cross-site pages cannot
  call the API).
- `POST /api/cmd/{name}`: auth middleware requires `Authorization: Bearer
  <token>`, compared with `subtle::ConstantTimeEq` → `401` JSON
  `"Unauthorized"`. Body = JSON object of camelCase args. Unknown `{name}` →
  `404` JSON `"Unknown command: <name>"`. Dispatch runs the services fn
  inside `tokio::task::spawn_blocking`. `Ok(v)` → `200` JSON `v`.
  `Err(ChiknError)` → `422` with the error serialized through its existing
  `Serialize` impl (a JSON string). Panic / join error → `500` JSON
  `"Internal error"`. Request logging records method, path without query
  string, status, duration — never bodies, never the query string.
- `GET /api/info` (auth) → `{ root, version, events: false }`.
- `GET /api/projects` (auth) → walk `root` to depth 3; skip hidden entries
  and `.git`; do not descend into a `.chikn` directory; return
  `[{ name, path }]` for directories named `*.chikn` that contain
  `project.yaml`. `name` is the folder stem (`Notes.chikn` → `Notes`);
  the manifest is not parsed here — reading `project.yaml` outside core
  would be format I/O outside the engine (I2).
- `GET /api/events` → `404` until slice 5.

Shared state: `Arc<AppState { root: PathBuf, token: String, locks:
ProjectWriteLocks, tokens: ProjectTokens, ai_registry: AiStreamRegistry }>`.

Command table (`crates/web/src/commands.rs`): one `match name { "create_project"
=> run::<CreateProjectArgs>(…, |a, s| services::project::create_project(a.name,
a.path.resolve(&s.gate)?, &s.locks, &s.tokens)), … }` covering all 75
names, with one `#[derive(Deserialize)] #[serde(rename_all = "camelCase")]`
arg struct per command (no `deny_unknown_fields`, matching Tauri). A
`macro_rules!` that expands name + fields + call is acceptable; the table
stays in this one file. Two names are web-specific until later slices:
`pick_scriv_folder` → `422` `"Not available in the web version"` (slice 4);
`ai_transform_stream` → `422` `"Live AI streaming is not available in the
web version yet"` (slice 5). `cancel_ai_transform_stream` is wired.

Path gate (`crates/web/src/gate.rs`), applied before dispatch to every
path-bearing argument — `path`, `projectPath`, `outputPath`, `filePath`,
`folderPath`, `scrivPath`:
- Arg structs declare these as `PathArg` (newtype over `String` with a
  private field; its only accessor is `resolve(&self, &Gate) ->
  Result<String, GateError>`), so a handler cannot forward a path it has not
  gated.
- Relative paths → `400`. Resolve step by step from the filesystem root,
  following symlinks for components that exist (the approach of core's
  nested-project check from `453c78e` / `b0ee3b5`; call a public core helper
  if one exists by then), then require the result to start with the
  canonical root. For create-style args (`create_project` `path`,
  `import_markdown_folder` / `import_scrivener` `outputPath`,
  `compile_project` `outputPath`) the last component may not exist; its
  parent must resolve under root.
- Violation → `403` JSON `"Path is outside the projects folder"`; the
  offending path is logged at debug level only.
- Settings values that are paths (`backup.backup_directory`,
  `general.pandoc_path`) are **not** gated: they are server-local preferences
  set by the single authenticated user and consumed by the services layer
  exactly as on the desktop. State this in the module header.

Dev loop: `ui/vite.config.ts` adds `server.proxy = { '/api':
'http://127.0.0.1:7391' }` only when env `CHIKN_WEB_PROXY` is set, so the
Tauri dev flow is unaffected and agents can iterate on web mode with HMR.

Startup log: bind address, root, UI dir, token source (flag / env / file /
generated). Graceful shutdown on SIGINT / SIGTERM via
`axum::serve(..).with_graceful_shutdown`.

Tests (`crates/web/tests/http.rs`, `tower::ServiceExt::oneshot` against the
router with a `TempDir` root and a stub `ui_dir` holding a one-line
`index.html`; no Pandoc): missing or wrong token → 401; `create_project`
under root → 200 and the folder exists; `create_project` with `path`
outside root — absolute elsewhere, `..`-laden, and a symlink inside root
pointing outside — → 403 and nothing created; `load_project` → 200 with
`read_only: false`; `update_document_content` then `get_document`
round-trips; `get_document` unknown id → 200 `null`;
`update_document_content` unknown doc → 422 and body equals
`ChiknError::NotFound(..).to_string()`; unknown command → 404; `GET /` → 200
HTML carrying the CSP header; `GET /api/projects` lists the created project
and ignores a sibling folder without `.chikn`. Guard proof: drop the gate
call from `create_project` → the symlink test fails; restore.

Paperwork: workspace member; `-p chickenscratch-web` in the three suite
locations; `crates/web/Cargo.toml` in `check-release-metadata.sh`;
`docs/ARCHITECTURE.md` crate map; `README.md` Platforms row "Web
(self-hosted) | `chikn-web` serving the same React UI | Preview" and a Build
subsection (`cd ui && npm run build`, `cargo build --release -p
chickenscratch-web`, `./target/release/chikn-web --root ~/Notes`).

### Slice 3b — Web Welcome: list, open and create projects without native dialogs

- `Welcome.tsx` web mode: "Open Project" becomes a list from
  `GET /api/projects` (through `ui/src/commands/web.ts`, the only module for
  web-only endpoints, using the same token header) merged with the existing
  recent list (dedupe by path); clicking calls the existing
  `openProject(path)`. "New Project" uses the existing `dialogPrompt`
  (`ui/src/components/shared/Dialog.tsx`) for a name, then
  `createProject(name, root)` where `root` comes from `GET /api/info`. Name
  rule: non-empty, no `/` or `\`, no leading `.`; the engine already refuses
  an existing folder. Hide "Import Scrivener" in web mode until slice 4.
  Recent-project paths display relative to `root` (the existing
  `/Users/<name>/` → `~/` regex stays for Tauri).
- Token prompt `ui/src/components/welcome/WebTokenGate.tsx`: shown when
  `isWeb()` and no token is stored, or on `cs-web-auth-required`. One field
  "Access token" and Continue; stores to localStorage and re-runs
  `loadSettings()` and the project list. Plain wording; no "bearer".
- Tests: vitest for web Welcome (lists, opens on click, creates by name,
  hides Scrivener import) and the token gate flow.

Owner-usable milestone: after 3b the owner can create, open, write, save,
save revisions, browse history, use drafts, search, stats, settings and
themes from a browser.

### Slice 4 — Replace the remaining desktop-only actions in web mode

One commit each:

- **4a Export as download.** Server: `POST /api/export` (auth; gate on
  `projectPath`) with the `compile_project` body minus `outputPath`; the
  handler compiles into a `tempfile::NamedTempFile` with the requested
  extension, streams it back with `Content-Disposition: attachment;
  filename="<title>.<ext>"`, then deletes it. UI: `CompileDialog` web branch
  skips `pickSaveFile`, adds a format `<select>` fed by
  `getCompileFormats()`, and downloads via `fetch` → `Blob` → anchor click.
  Error contract as `/api/cmd`.
- **4b Import by upload.** Server: `POST /api/upload` (auth; multipart,
  one file, 50 MiB cap, filename reduced to a sanitized basename) stores to
  `<config-dir>/chickenscratch/uploads/<uuid>/<basename>` and returns the
  path; the gate's allowed roots become `[root, uploads_dir]`. UI: `Binder`
  import web branch uses `<input type="file">` with the same extension
  filter, uploads, then calls the existing `importFile(projectPath,
  uploadedPath, parentId)`; the server removes the upload directory after
  the command finishes, success or failure.
- **4c Server-side folder picker** for "Choose Backup Folder" and the
  Scrivener source folder (the owner copies the `.scriv` under root first):
  `GET /api/fs/dirs?path=<under root>` returns child directory names (hidden
  skipped; gated); `ui/src/components/shared/ServerFolderPicker.tsx`;
  `platform.pickFolder` web branch uses it. The backup folder stays an
  un-gated settings value — the picker only helps choose it.
- **4d** `pick_scriv_folder` web branch → the folder picker; "Import
  Scrivener" reappears in web mode.

### Slice 5 — AI streaming over SSE (optional; only if AI is wanted in the browser)

`GET /api/events` → `text/event-stream`; a `WebSink: AiStreamSink` pushes
`event: ai:chunk` / `ai:done` / `ai:error` with the same JSON payloads into a
`tokio::sync::broadcast` channel per connected client. `transport.listen`
consumes it; `GET /api/info` reports `events: true` and
`supportsAiStreaming()` follows it. Keyring availability is handled here: if
`keyring::Entry::get_password` fails with a platform error,
`save_app_settings` returns "This computer has no secure keyring; AI keys and
remote tokens cannot be stored" instead of an opaque error (services-level
message; also helps the desktop app on unusual Linux setups).

### Slice 6 — Single binary and run notes

- Cargo feature `embed-ui` on `chickenscratch-web` (`rust-embed` or
  `include_dir` over `../../ui/dist`), **off by default** so `cargo test
  --locked` passes without a UI build; `--ui-dir` still overrides.
  `RELEASE.md` adds `cargo build --release -p chickenscratch-web --features
  embed-ui` after `npm run build`.
- `docs/USER_GUIDE.md` section "Web version (self-hosted)": requirements,
  first run, the access token, reaching it over a VPN hostname, a sample
  `systemd --user` unit, "no TLS here — keep it on your VPN or behind a
  reverse proxy", and browser-reserved shortcuts (Ctrl+N, Ctrl+Shift+P,
  Ctrl+P may be taken by the browser; remap in Settings → Shortcuts).
- `TODO.md` Remaining gains a "Web shell" block mirroring shipped slices;
  `DEVLOG.md` entry when the plan ships (architecture changed).

### Slice 7 — Single-writer session lease (optional hardening)

For two devices open on one project: the transport sends a random per-tab
`X-CS-Session` header; the first mutating call on a project takes a lease;
mutating calls from another session get `409` `"This project is open in
another browser tab or device. Take over?"` and the UI offers Take over
(`POST /api/lease/take`). Leases expire two minutes after the last call. Not
needed for one person on one device at a time.

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `crates/services/` (new, slice 1) | `Cargo.toml`; `src/lib.rs` re-exports; `src/{project,document,git,io,search,settings,templates,threads,ai}.rs` moved bodies and tests; `AiStreamSink` |
| `src-tauri/src/commands/*.rs`, `src-tauri/src/main.rs`, `src-tauri/Cargo.toml` (slice 1) | thin wrappers; `TauriSink`; `pick_scriv_folder` stays; drop moved deps |
| `Cargo.toml` (workspace) | add `crates/services`, `crates/web` |
| `scripts/check-release-metadata.sh` | both new `Cargo.toml`s in `version_files` |
| `.github/workflows/validation.yml`, `docs/AGENT-WORKFLOW.md` §5, `.agents/repo-guidance.md` Verification | `-p chickenscratch-services -p chickenscratch-web` on clippy and test lines |
| `docs/ARCHITECTURE.md`, `README.md`, `docs/INVARIANTS.md` (I2 table), `docs/adr/ADR-007-web-shell-shared-services.md` (new) | record the shared services layer and the web shell |
| `ui/src/commands/transport.ts`, `ui/src/platform.ts` (new, slice 2) | the single IPC/HTTP doorway; desktop/web seams |
| `ui/src/commands/*.ts`, `ui/src/App.tsx`, `ui/src/components/{welcome/Welcome,binder/Binder,compile/CompileDialog,settings/Settings}.tsx` | import from transport/platform; web-mode branches |
| `ui/eslint.config.js` | restrict every `@tauri-apps/*` import to `transport.ts` / `platform.ts` |
| `ui/src/test/transport.test.ts`, `ui/src/test/welcomeWeb.test.tsx`, `ui/src/test/webTokenGate.test.tsx` (new) | web-mode coverage |
| `ui/vite.config.ts` | opt-in `/api` proxy under `CHIKN_WEB_PROXY` |
| `crates/web/` (new, slice 3a) | `Cargo.toml` (`chikn-web` bin); `src/main.rs` (CLI, startup, token file); `src/state.rs`; `src/auth.rs`; `src/gate.rs`; `src/commands.rs` (the table); `src/static_ui.rs`; `src/projects.rs` (`/api/projects`, `/api/info`); `tests/http.rs` |
| `crates/web/src/{export,upload,fs_browse}.rs` (slice 4) | download, upload, folder browser |
| `ui/src/commands/web.ts`, `ui/src/components/welcome/WebTokenGate.tsx`, `ui/src/components/shared/ServerFolderPicker.tsx` (new) | web-only endpoints and dialogs |
| `crates/web/src/events.rs` (slice 5) | SSE and `WebSink` |
| `docs/USER_GUIDE.md`, `RELEASE.md`, `TODO.md`, `DEVLOG.md` (slice 6 / ship) | run notes, release step, record |
| `docs/plans/PLAN_WEB_SHELL.md`, `.agents/state.md` | status per slice |

## [MODEL] Tests

- [ ] Slice 1: declared suite green with the new `-p` entries; `cargo tree`
  shows no `tauri` under `chickenscratch-services`; moved command-boundary
  tests pass unchanged; `src-tauri` has no `chickenscratch_core` /
  `chikn_converter` references.
- [ ] Slice 2: vitest — web `invoke` URL, headers, body; raw-string
  rejection; 401 → auth event and token cleared; `?token=` capture and
  strip; `aiTransformStream` single-chunk fallback; ESLint guard proven to
  bite.
- [ ] Slice 3a: `crates/web/tests/http.rs` as listed, including the symlink
  escape with nothing created; guard proof by removing the gate call.
- [ ] Slice 3b: vitest for web Welcome and token gate.
- [ ] Slice 4: http tests — export returns an attachment for a format that
  does not need Pandoc if the engine offers one, otherwise the test is
  `#[ignore]`d without Pandoc and says so; upload lands under the uploads
  dir and is removed after import; `fs/dirs` refuses paths outside root.
- [ ] Slice 5: SSE delivers `ai:chunk` / `ai:done` from a fake sink;
  disconnect unregisters the client.
- [ ] Every slice: `cargo fmt --all -- --check`; clippy `-D warnings` on all
  crates; `cargo test --locked` on all crates; `scripts/check-release-metadata.sh`;
  `cd ui && npm run lint && npm run build && npm run test`.

## [MODEL] Owner verification (plain English)

After slice 3b, on the machine that will host it:

1. Build once: `cd ui && npm run build && cd .. && cargo build --release -p chickenscratch-web`.
2. Run `./target/release/chikn-web --root ~/Notes --bind 0.0.0.0:7391`. It
   prints a link that includes an access token.
3. From another device on the VPN, open that link. The ChickenScratch
   welcome screen appears with "New Project".
4. Create "Scratch", type a few lines, wait two seconds, reload the page,
   reopen Scratch → the text is there.
5. Save Revision, type more, open History → both states are listed; Restore
   brings the earlier one back and the later one stays in History.
6. Open the same link in a second browser tab → Scratch is listed; no token
   prompt.

After slice 4: Export produces a download; Import File accepts a `.md` or
`.docx` chosen on the device you are browsing from.

## [YOU] Decisions needed

None block the plan. Decisions this plan takes, with reasons:

- **Shared services crate, not a copied command layer.** One canonical
  command layer (`AGENTS.md` one-canonical-location; `docs/ARCHITECTURE.md`
  already names "app services").
- **Token required, no unauthenticated mode, localhost by default.** A
  server that creates and deletes files must not accept writes from anything
  that can reach the port, VPN or not. Cost: paste a token once per browser.
- **Projects-root gate.** The desktop app may touch any path the user picks
  in a native dialog; the browser has no such dialog, so the server confines
  project paths to one folder. Settings paths stay server-local preferences.
- **No TLS, no multi-user, no cloud.** Transport security is the VPN per the
  owner; multi-user and hosting are explicit non-goals (`docs/PROJECT.md`).

At go-time, request one slice at a time. Slice 1 (services extraction) is a
broad mechanical refactor of the desktop backend and gets its own go before
anything server-side.

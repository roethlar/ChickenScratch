# ADR-007: Shells share one command layer; a self-hosted web shell serves the same UI

**Status:** Accepted — on the owner's plain-English direction quoted below
(`docs/HUMAN-GATE.md`: no keyword is required of the owner)\
**Date:** 2026-10-24

## Context

The owner asked (2026-10-23) for "a web app version of chickenscratch that I
can access from anywhere (vpn, connection out of scope for this) to store
notes, write markdown docs, etc." and to "control my note taking and text
editing without relying on cloud services", queued as a side quest with no
change of phase or direction.

Until now the only shell was the Tauri desktop app. Its command layer —
75 synchronous functions that call `chickenscratch-core` for every `.chikn`
read, write and git operation — lived inside the bin-only `src-tauri` crate,
so no other process could reuse it without linking Tauri (and WebKitGTK on
Linux). Any second shell would have had to copy that layer, creating a
second place where app behavior could drift ([ADR-001](ADR-001-chikn-engine-single-source-of-truth.md)
forbids a second engine; a second command layer is the same mistake one
level up).

The React UI in `ui/` is already an ordinary web page: all backend calls go
through one `invoke` doorway, and only a handful of desktop-only affordances
(native file dialogs, open-link, close-window hook, Tauri events) touch the
Tauri APIs.

## Decision

1. **One command layer, `chickenscratch-services` (`crates/services/`).** The
   former `src-tauri/src/commands/*` bodies move there unchanged as plain
   functions; `ProjectWriteLocks`, `ProjectTokens`, settings/keyring, AI
   client and the per-project write-permit discipline move with them. The
   crate depends on no UI framework. Streamed AI output goes through an
   `AiStreamSink` trait that each shell implements.
2. **Shells are thin.** `src-tauri` wraps each service function one-to-one,
   adapting only Tauri `State` extractors and the event emitter; the native
   Scrivener folder picker stays there because it is desktop-only. The
   `chikn-web` server (`crates/web/`, planned in
   `docs/plans/PLAN_WEB_SHELL.md`) wraps the same functions as same-origin
   JSON endpoints and serves the built `ui/dist`.
3. **`ui/` remains the single GUI.** The web server is a second *shell* for
   the same page, as the TUI is a second UI for the same engine. New
   user-facing features still land once, in `ui/` + `chickenscratch-services`.
   [ADR-003](ADR-003-tauri-reference-ui.md) stands: Tauri is the reference
   desktop GUI.
4. **Self-hosted, single-user, no cloud.** `chikn-web` runs on a machine the
   owner controls, confined to one projects folder, behind an access token,
   reached over the owner's own network. `docs/PROJECT.md`'s "not doing
   cloud/SaaS hosting" is unchanged.

## Consequences

### Positive

- Desktop and web cannot drift: there is one body per command.
- `src-tauri` is finally the "thin command layer" `ADR-003` §4 describes.
- The services crate is testable without a webview; the command-boundary
  tests moved with it.

### Negative

- One more crate in the workspace, the declared suite, CI and the release
  metadata check.
- A shell author must remember that anything added to `src-tauri` directly
  (rather than to services) is invisible to the web shell. The acceptance
  rule: `src-tauri/src` references no `chickenscratch_core` reader, writer,
  hierarchy or git function.

## Compliance

- Command bodies → `crates/services/src/`. Tauri wrappers →
  `src-tauri/src/commands/`. Web handlers → `crates/web/src/`.
- `docs/INVARIANTS.md` I2 table lists both shells as in-process callers via
  `chickenscratch-services`.
- `docs/ARCHITECTURE.md` crate map and layer diagram show the services layer
  between the shells and the engine.

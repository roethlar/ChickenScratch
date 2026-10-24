# Architecture

**Last verified:** 2026-10-24 (services extraction, [ADR-007](adr/ADR-007-web-shell-shared-services.md))\
**Governed by:** [`INVARIANTS.md`](INVARIANTS.md)

---

## Layer diagram

```
┌─────────────────────────────────────────────────────────────┐
│  UIs (customers — presentation & commands only)             │
│  ┌──────────────┐  ┌─────────┐  ┌──────────────────────┐  │
│  │ Tauri + React│  │   TUI   │  │ chikn-converter (CLI) │  │
│  │ src-tauri/   │  │crates/  │  │ crates/cli           │  │
│  │ ui/          │  │  tui/   │  │                      │  │
│  └──────┬───────┘  └────┬────┘  └──────────┬───────────┘  │
│         │               │                    │              │
│         └───────────────┴────────────────────┘              │
│                         │ in-process Rust API               │
├─────────────────────────┼───────────────────────────────────┤
│  ENGINE (canonical)     ▼                                   │
│  crates/core — chickenscratch-core                          │
│    • project reader / writer                                │
│    • safe_path, atomic writes                               │
│    • git (revisions, drafts, backup, sync)                  │
│    • compile (Pandoc)                                       │
│    • (Scrivener and other converters live outside the engine and call into it) │
│    • models: Project, Document, TreeNode, Thread, …         │
├─────────────────────────────────────────────────────────────┤
│  ON DISK — .chikn project folder                            │
│    project.yaml, manuscript/*.md + *.meta, .git/, …         │
└─────────────────────────────────────────────────────────────┘

REMOVED (ADR-004; history in git):
  macos/ChiknKit (Swift reimplementation)
  windows/ChickenScratch.Core (C# reimplementation)
  linux/ Qt frontend
```

The Tauri shell and the `chikn-web` shell (`docs/plans/PLAN_WEB_SHELL.md`)
reach the engine through `crates/services` (`chickenscratch-services`), the
shared command layer; the TUI and the converter call the engine directly.

## Crate map

| Path | Crate / binary | Role |
|------|----------------|------|
| `crates/core/` | `chickenscratch-core` (ChickenEngine) | **Engine** — pure .chikn format, git on .chikn projects, compile from .chikn content. Converters (Scrivener etc.) live outside and use the engine. |
| `crates/cli/` | `chikn-converter` | CLI: Scrivener → `.chikn` |
| `crates/tui/` | `chikn` | Terminal UI over engine |
| `crates/services/` | `chickenscratch-services` | **App services** — the command layer every shell wraps: project, document, git, compile, import and search operations; per-project write locks and fresh permits; app settings; keyring; AI client. No UI framework dependency. |
| `src-tauri/` | `chickenscratch` | Tauri desktop shell: one thin wrapper per service function (`State` extractors in, events out) plus the native Scrivener picker |
| `crates/web/` | `chikn-web` (package `chickenscratch-web`) | Self-hosted web shell: serves `ui/dist` and exposes every service function as `POST /api/cmd/<name>` behind a bearer token and a projects-root path gate |
| `ui/` | (npm package) | React frontend: TipTap editor, panels, dialogs |

Cargo workspace: root `Cargo.toml` — members are `crates/core`, `crates/cli`, `crates/tui`, `crates/services`, `crates/web`, `src-tauri`.

## What belongs in the engine vs the app

### Engine (`crates/core`)

- Parsing / writing `project.yaml`, `.meta`, `.md`
- Hierarchy operations, slug uniqueness, validation
- All `git2` operations on embedded project repos
- Pandoc compile orchestration
- Scrivener `.scriv` conversion
- Word diff for revision viewer (data layer)

### App services (`crates/services`)

- One plain function per user-facing operation (the former Tauri command bodies)
- App-global settings (`~/.config/chickenscratch/settings.json`)
- Keyring for secrets (AI keys, remote tokens)
- AI HTTP calls (not format); streamed output goes through the `AiStreamSink` trait
- Per-project write locks and the fresh-permit boundary for every mutation
- Does **not** duplicate reader/writer logic — imports `chickenscratch_core::...`
- Depends on no UI framework ([ADR-007](adr/ADR-007-web-shell-shared-services.md))

### Tauri backend (`src-tauri`)

- Wraps each service function as a Tauri command: `State` extractors in, event emitter out
- Native Scrivener folder picker (desktop-only)
- Nothing else — no format logic, no settings, no git. Acceptance: `src-tauri/src` references no `chickenscratch_core` reader, writer, hierarchy or git *function*

### Web shell (`crates/web`, binary `chikn-web`)

- One Rust process: engine in-process via `chickenscratch-services`, static `ui/dist`, JSON API
- `POST /api/cmd/<name>` mirrors every Tauri command: same camelCase args, same result, errors as the plain `ChiknError` string (`422`)
- Bearer token on every `/api/*` request (`auth.rs`); no cookies, no CORS
- Projects-root gate (`gate.rs`): every path argument must resolve, through symlinks, inside `--root`; `403` otherwise
- Web-only helpers the browser needs because it has no native dialogs: `GET /api/projects`, `GET /api/info`
- Does **not** touch `.chikn` files itself — not even to read a project name (I2)

### React UI (`ui/`)

- Layout, themes, binder, corkboard, inspector UX
- TipTap editor (Markdown ↔ HTML in the webview)
- Calls Tauri commands only — **never** touches `.chikn` files directly

## Removed native experiments

See [ADR-004](adr/ADR-004-deprecated-native-engines.md). The deprecated
`macos/` (SwiftUI + ChiknKit), `windows/` (WinUI + C# core), and `linux/`
(Qt6 + cxx-qt) trees have been deleted from the working tree; their history
remains in git. Format I/O lives only in `crates/core` (I2); any future
native shell calls the Rust engine rather than reimplementing it.

The format harness (`crates/core/tests/cross_frontend/run.sh`) now drives
the Rust converter → Rust reader leg only; the fixture tests in
`crates/core/tests/cross_frontend_round_trip.rs` still guard reader
tolerance for `.meta` shapes the removed writers produced.

## External dependencies

| Dependency | Used for |
|------------|----------|
| Pandoc | Scrivener import, compile/export only — not live editing |
| git2 (bundled libgit2) | Embedded project history — writers do not need system git |
| TipTap | WYSIWYG editing in Tauri UI only |

## Validation entry points

See [`AGENT-WORKFLOW.md`](AGENT-WORKFLOW.md) for full commands. Minimal smoke:

```bash
cargo test -p chickenscratch-core --lib
cargo test -p chickenscratch --bins
cd ui && npm run lint && npm run build
```
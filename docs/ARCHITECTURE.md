# Architecture

**Last verified:** commit `b1ee466` (2026-07-10)  
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

## Crate map

| Path | Crate / binary | Role |
|------|----------------|------|
| `crates/core/` | `chickenscratch-core` (ChickenEngine) | **Engine** — pure .chikn format, git on .chikn projects, compile from .chikn content. Converters (Scrivener etc.) live outside and use the engine. |
| `crates/cli/` | `chikn-converter` | CLI: Scrivener → `.chikn` |
| `crates/tui/` | `chikn` | Terminal UI over engine |
| `src-tauri/` | `chickenscratch` | Tauri backend: thin commands → core + app services (settings, AI, keyring) |
| `ui/` | (npm package) | React frontend: TipTap editor, panels, dialogs |

Cargo workspace: root `Cargo.toml` — members are `crates/core`, `crates/cli`, `crates/tui`, `src-tauri`.

## What belongs in the engine vs the app

### Engine (`crates/core`)

- Parsing / writing `project.yaml`, `.meta`, `.md`
- Hierarchy operations, slug uniqueness, validation
- All `git2` operations on embedded project repos
- Pandoc compile orchestration
- Scrivener `.scriv` conversion
- Word diff for revision viewer (data layer)

### Tauri backend (`src-tauri`)

- Expose engine operations as Tauri commands
- App-global settings (`~/.config/chickenscratch/settings.json`)
- Keyring for secrets (AI keys, remote tokens)
- AI HTTP calls (not format)
- Process locks for concurrent project writes
- Does **not** duplicate reader/writer logic — imports `chickenscratch_core::...`

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
# Project — Plain English

**Audience:** Repository owner and agents.  
**Last aligned with code:** commit `b27f315` (2026-05-24)

---

## What this is

**ChickenScratch** is a writing application built around the **`.chikn` format** — your work stored as a folder of plain Markdown files with a built-in history, not locked inside a proprietary blob.

Think: *Scrivener’s project model, but your novel is just files you can copy, read in any text editor, and keep for decades.*

## Why it exists

- **Portability** — projects outlive any one app
- **Safety** — auto-save, checkpoints, recoverable history
- **Escape from vendor lock-in** — especially for writers on **Linux** (no native Scrivener) and anyone leaving Scrivener
- **Open source** (MIT)

## What is “done” for v1

1. **Format** stable and documented (`.chikn` v1.x)
2. **Engine** (`chickenscratch-core`) is the only code that mutates `.chikn` on disk
3. **One GUI** — Tauri desktop on macOS and Linux (Windows Tauri when added)
4. **Converter** — import Scrivener `.scriv` → `.chikn`
5. **TUI** — prove the format works without a GUI
6. Owner can describe a feature in plain English; agents ship it without owner reading code

## What we are NOT doing (until owner changes priorities)

- Maintaining five separate UIs at feature parity
- Rewriting the engine in Swift, C#, or Qt
- Marketing or website work before the above is coherent
- Cloud/SaaS hosting of user projects

## Priority order (do not reorder without `SET PRIORITY`)

1. **Governance & coherence** — invariants, single engine, deprecate drift sources
2. **Format finalization** — genre-agnostic schema, `fields` map, spec locked
3. **Engine hardening** — tests, no duplicate writers
4. **Tauri reference app** — only GUI that gets features
5. **Windows Tauri build** — same app, not WinUI parity chase
6. **Novelist UI features** — inspector, corkboard, etc. (UI layer only)
7. **Marketing** — after 1–4 are true

## Glossary (owner ↔ agent)

| Owner says | Agent understands |
|------------|-------------------|
| “the format” | `.chikn` on-disk layout + `CHIKN_FORMAT_SPEC.md` |
| “the engine” | `crates/core` (`chickenscratch-core`) |
| “the app” | Tauri + React (`src-tauri/` + `ui/`) |
| “converter” | `chikn-converter` binary (`crates/cli/`) |
| “terminal app” | `chikn` TUI (`crates/tui/`) |
| “save a revision” | git commit via engine, writer-friendly label |
| “deprecated UIs” | the removed `macos/`, `windows/`, `linux/` native experiments (ADR-004; history in git) |

## Where to read next

| Question | Document |
|----------|----------|
| What must never break? | [`INVARIANTS.md`](INVARIANTS.md) |
| How is code organized? | [`ARCHITECTURE.md`](ARCHITECTURE.md) |
| What are we working on now? | [`CURRENT_PHASE.md`](CURRENT_PHASE.md) |
| How should agents work? | [`AGENT-WORKFLOW.md`](AGENT-WORKFLOW.md) |
| How does the owner direct work? | [`HUMAN-GATE.md`](HUMAN-GATE.md) |
| Why was X decided? | [`adr/`](adr/) |
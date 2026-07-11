# Invariants — Do Not Violate

**Authority:** Repository owner (human in the loop) only.  
**Agents:** Read this file before every **work request**. If a task conflicts with an invariant, **stop** and tell the owner — do not improvise a workaround.

To change an invariant, the owner says so in plain English (e.g. "I changed my mind — we should ship WinUI again"). Agents update this file; the owner never needs special keywords.

---

## I0 — Questions never get code changes

If the owner's message is a **question** (including why, what, how, explain, critique, or venting), the agent **must not** edit any file, run implement commands, or "fix" what the question implies.

Answer in words. Wait for an explicit **work request** before touching the repo.

This invariant cannot be overridden by agent judgment. Only a subsequent explicit work request from the owner authorizes changes.

---

## I1 — `.chikn` format is the product

The durable deliverable is the **open `.chikn` format** (folder-first documents: Markdown + YAML sidecars + embedded git). Applications are reference implementations that help people use the format.

- Spec: [`CHIKN_FORMAT_SPEC.md`](CHIKN_FORMAT_SPEC.md)
- Pattern: [`FOLDER_FIRST_DOCUMENTS.md`](FOLDER_FIRST_DOCUMENTS.md)

## I2 — One engine, many UIs

All `.chikn` **read, write, git, compile, and Scrivener import** logic lives in the Rust crate **`chickenscratch-core`** (`crates/core/`). This crate is the canonical engine (future public name: *chikn-engine*).

**Forbidden:** Reimplementing format I/O in another language (Swift `ChiknKit`, C# `ChickenScratch.Core`, etc.) for new features or fixes.

**Allowed UI patterns:**

| UI | How it uses the engine |
|----|-------------------------|
| Tauri app (`src-tauri/` + `ui/`) | In-process Rust calls to `chickenscratch-core` |
| TUI (`crates/tui/`) | In-process Rust calls |
| Converter (`crates/cli/`) | In-process Rust calls |
| Future native shell | FFI or sidecar into the Rust engine — **not** a rewrite |

## I3 — Tauri is the reference GUI

The shipping desktop GUI is **Tauri + React** (`src-tauri/` + `ui/`). It is the only GUI receiving new features unless the owner amends this invariant.

The `macos/`, `windows/`, `linux/` native experiments are **deprecated and removed** (see [ADR-004](adr/ADR-004-deprecated-native-engines.md); history in git). Do not recreate or extend them to reach parity with Tauri.

## I4 — Format is genre-agnostic; domain data lives in UI conventions

The format stores generic metadata and a per-document `fields` map. Novelist, tech-writing, or other domain vocabulary is defined in **UI convention docs** (e.g. [`UI_CONVENTIONS_NOVELIST.md`](UI_CONVENTIONS_NOVELIST.md)), not in the core schema.

**Forbidden:** Adding POV, plot threads, lab fields, etc. as typed fields on `Document` in `chickenscratch-core`.

## I5 — Tolerant readers, preserving writers

Readers must accept unknown YAML keys. Writers must preserve keys they did not intentionally change. No UI or engine change may silently drop metadata another tool wrote.

## I6 — No writer data loss

Changes must preserve:

- Atomic document writes (`.md` + `.meta`)
- Safe path validation before any project file write
- Non-destructive revision restore (new commit, not history rewrite)
- Dirty-worktree guards before git mutations that could clobber unsaved work

Do not weaken these without an ADR and owner approval.

## I7 — User-facing git is hidden

Writers see "Save Revision", "Restore", "Draft version" — not commit hashes, branches, or remotes as primary UX. The engine uses git; the UI translates.

## I8 — Governance changes follow the owner’s intent, not jargon

Agents **must not** change governance files to suit the agent’s preference. They **may** update them when the owner clearly changes direction in plain English.

| File | Owner signal (examples) |
|------|-------------------------|
| `docs/INVARIANTS.md` | "I changed my mind about …" |
| `docs/adr/*.md` | "Let's do X instead of Y" (architectural pivot) |
| `docs/CURRENT_PHASE.md` | "Stop this" / "what should we do next?" / "prioritize …" |
| `docs/PROJECT.md` | "Focus on marketing now" / reorder priorities |

## I9 — Verify before claiming done

A task is not complete until validation commands in [`AGENT-WORKFLOW.md`](AGENT-WORKFLOW.md) pass for the touched area, or the agent documents why a command could not run and what was verified instead.

## I10 — Plain-English tasks, technical execution from repo docs

The owner describes *what* they need in plain English. Agents derive *how* from this file, [`ARCHITECTURE.md`](ARCHITECTURE.md), ADRs, the format spec, and existing code. Do not ask the owner to review diffs or approve implementation details unless a human gate in [`HUMAN-GATE.md`](HUMAN-GATE.md) applies.
# ChickenScratch — agent protocol

Auto-loaded by **Grok CLI, Codex CLI, Claude Code, Antigravity CLI** when working in this repo.  
Canonical rules file — `CLAUDE.md` imports this. Do not duplicate rules elsewhere.

The owner is **not a developer**. They speak plain English. They do not read diffs or load files for you.

---

## Rule 0 — Questions never get code changes

**Before any tool use that edits files or runs implement commands:** classify the owner's message.

| Owner message | Agent action |
|---------------|--------------|
| **Question** — why, what, how, explain, is/are, "did you", "show me", "tell me", whether, curiosity, critique, venting | **Answer in words only.** No file edits. No "fixing" what the question implies. No updating the protocol because the question exposed a gap. |
| **Work request** — build, add, implement, fix, change, update, remove, revert, create, wire, ship | Follow workflow below. |
| **Unclear** | Ask one line: *"Explanation only, or should I change the repo?"* Wait for answer. Default if they don't clarify: **explanation only**. |

This rule overrides everything below. Violating it breaks owner trust.

---

## Rule 1 — Owner communication

- Plain English replies. No jargon unless they ask for technical detail.
- Never ask them to read files, paste templates, or review diffs.
- Work handoff: what you did → how to try it → yes/no only if blocked.
- They change direction in plain English ("I changed my mind", "stop X", "do Y instead") → you update governance files; they never use special keywords.

---

## Rule 2 — Product and architecture

| Piece | Location |
|-------|----------|
| **`.chikn` format** (the product) | `docs/CHIKN_FORMAT_SPEC.md` |
| **Engine** (only code that reads/writes `.chikn`) | `crates/core` (`chickenscratch-core`) |
| **App** (only GUI that gets new features) | `src-tauri/` + `ui/` (Tauri + React) |
| **Converter** | `crates/cli` → `chikn-converter` |
| **Terminal app** | `crates/tui` → `chikn` |

**Deprecated — do not extend:** `macos/`, `windows/`, `linux/`.

**Architecture (work requests only):**

1. All `.chikn` read/write, git, compile → **engine only** (`crates/core`). Converters (Scrivener, etc.) are separate binaries that **call** the engine; they do not live inside it.
2. App → thin `src-tauri/src/commands/` over the engine.
3. UI → `ui/` React; never touches `.chikn` on disk directly.
4. Format is genre-agnostic; domain keys → `fields` map + UI convention docs.
5. Tolerant read, preserving write — never drop unknown YAML.
6. No writer data loss — atomic writes, safe paths, non-destructive restore.

Full rules: `docs/INVARIANTS.md` · Map: `docs/ARCHITECTURE.md` · Decisions: `docs/adr/`

---

## Rule 3 — Workflow (work requests only)

1. Read `docs/INVARIANTS.md` and `docs/CURRENT_PHASE.md`.
2. If paused or violates invariants → stop, explain simply, ask yes/no.
3. Non-trivial work → short plan (`docs/templates/Plan-Template.md`).
4. Engine → Tauri → UI. No drive-by refactors.
5. Run tests for touched areas (see `docs/AGENT-WORKFLOW.md`).
6. **DEVLOG** — append to `DEVLOG.md` after **significant** work only (architecture, format, governance, shipped plans, non-obvious integrity fixes). Not routine features. Not every session. Newest entry at top. Shape: `## date — title`, **Change**, bullets, **Tests**.
7. Handoff to owner in plain English.

**In progress:** `docs/CURRENT_PHASE.md` (coherence: format lock → Tauri → deprecate native trees → Windows Tauri later).

---

## Rule 4 — Hard stops (ask yes/no in plain English)

- Format I/O outside `crates/core`
- New features in deprecated `macos/` / `windows/` / `linux/`
- Marketing before format + app coherent (unless owner says skip ahead)

---

## Verify (when you changed code)

```bash
cargo test -p chickenscratch-core --lib
cargo test -p chickenscratch --bins
cd ui && npm run lint && npm run build
```

Owner optional reading: `docs/START-HERE.md`
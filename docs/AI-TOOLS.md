# AI coding tools — how this repo briefs them

**For the owner:** you don't configure any of this. Work in the repo; say what you want.

| Tool | What it auto-loads from this repo |
|------|----------------------------------|
| **Grok CLI** | `AGENTS.md`, `CLAUDE.md`, `.grok/rules/*.md` |
| **Codex CLI** | `AGENTS.md` (walks from repo root to cwd) |
| **Claude Code** | `CLAUDE.md` → imports `AGENTS.md` via `@AGENTS.md` |
| **Antigravity** | `AGENTS.md` + optional `GEMINI.md` (Antigravity-only tweaks) |

**Canonical file:** [`AGENTS.md`](../AGENTS.md) — edit this if project rules change. Other files should point here, not duplicate.

**Owner guide:** [`START-HERE.md`](START-HERE.md)
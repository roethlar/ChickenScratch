# DEVLOG

Running log of architectural decisions and significant changes.

---

## 2026-04-21 — Remote sync (push/fetch + status)

**Change:** New `sync` git remote, push/fetch/status commands in core + Tauri, Remote settings tab, Revisions-panel footer widget that shows "N to push · M to pull" and exposes Push/Fetch buttons. Separate from the existing `backup` remote (directory mirror) — `sync` accepts any git URL (HTTPS, SSH, or `file://` for testing).

**Why:** The biggest v1.1 gap. User writes on macOS + Linux + Windows; backup mirrors the project to a local directory but doesn't help you start a new session on a different machine. Remote sync closes that loop.

**Design notes:**
- Remote named `sync` so it doesn't collide with the user's own `origin` or our `backup`. `ensure_sync_remote` updates the URL in place if the setting changes.
- Credential callback handles HTTPS username/PAT first, then SSH-agent fallback for `git@` URLs. No OS keychain yet — the token lives in plaintext in `settings.json`; scope the PAT to one repo.
- `sync_status` returns `(ahead, behind)` from `graph_ahead_behind` against the last-fetched `refs/remotes/sync/<branch>`. Before the first fetch, ahead = total commit count and behind = 0, so the UI has a sensible "push everything" starting state.
- Auto-push on named revision is opt-in (off by default) and fire-and-forget — a failed push never rolls back the revision.

**Scope limits:** Push and fetch. **Not** included: merge of incoming commits, conflict UX, SSH key passphrases. If a fetch brings down commits that diverge from local, the status shows "N to pull" but there's no in-app merge yet — the user would need to pull/merge via CLI. That's the next pass.

**Tested:** Round-trip integration test in `crates/core/tests/remote_sync.rs` — pushes a fresh project to a `file://` bare repo, fetches back, asserts ahead/behind = 0; then adds a revision, asserts ahead = 1, pushes, asserts ahead = 0 again.

**Commit:** `<pending>`

---

## 2026-04-18 — TUI anchored inline comments

**Change:** F3 in the TUI wraps the currently-selected text with a comment span and prompts for a body. Adds the comment to the document's `.meta` with the same ID as the `data-comment-id` attribute in the span.

**Why:** The TUI had comments only as document-level orphans. Writers expected to anchor comments to specific text, just like the Tauri app. ratatui-textarea exposes `selection_range()` returning `((row, col), (row, col))`, which we use to wrap the selection in lines with `<span class="comment" data-comment-id="X">…</span>`.

**After:**
- F3 (editor focus + active selection): prompt for body, on confirm wrap selection + add to .meta + save
- F2 (any focus): opens comments overlay (unchanged)
- Normalizes selection direction; handles single-line and multi-line selections; char-boundary-safe string slicing

**Commit:** `<pending>`

---

## 2026-04-18 — Edit path no longer touches pandoc

**Change:** Replaced pandoc subprocess for markdown ↔ HTML conversion in the Tauri editor with `tiptap-markdown` (in-process, markdown-it + prosemirror-markdown).

**Why:** The previous design ran pandoc as a subprocess on every document load and save. This coupled core editing to an external binary being present, findable, and non-crashing. ~50ms spawn cost per save, two conversion hops per edit session, editing breaks if pandoc is missing. Fundamentally wrong architecture.

**After:**
- Editor load: `setContent(markdown)` — tiptap-markdown parses natively in-browser
- Editor save: `editor.storage.markdown.getMarkdown()` — in-process serialization
- Pandoc is still a dependency but only for compile/export/import paths (triggered by explicit user actions, not every keystroke)
- Per-save latency: ~1ms
- Custom HTML (comment spans, footnote nodes) round-trips via `html: true` option on tiptap-markdown — markdown-it passes inline HTML through untouched

**Commit:** `5c23763`

---

## 2026-04-18 — Canonical format: HTML → Pandoc Markdown

**Change:** `.chikn` projects now store documents as `.md` files, not `.html`. `DOCUMENT_EXTENSION` flipped. Compile pipeline reads markdown directly via pandoc.

**Why:** Editing markdown over HTML on the TUI side was lossy — any inline HTML that markdown couldn't express (comment spans, footnote nodes, colored text, `<u>`) got silently destroyed on every save. The fix wasn't preservation tokens or sidecar files; it was picking a single canonical format both frontends can natively handle.

**After:**
- Storage: Pandoc Markdown (`[text]{.class #id key="value"}` bracketed spans for attributes, `[^1]` footnotes, GFM extensions)
- Compile: `pandoc -f markdown -t docx|pdf|epub|...` — `strip_comments` and `transform_footnotes` helpers deleted; pandoc handles natively
- Scrivener import: RTF → markdown instead of RTF → HTML
- Import pipeline: converts all external formats to markdown
- Tests updated throughout
- Interop win: `.md` files editable in vim, Obsidian, VS Code, any markdown tool

**Commits:** `2c3b8cf` (migration), `5c23763` (edit path cleanup)

---

## 2026-04-18 — Comments and footnotes

**Change:** First-class Word-style comments (anchored to a text span, resolvable, right-gutter panel) and inline footnotes.

**Why:** Writers need marginal notes and citation-style footnotes. Scrivener's annotation feature was a known user request.

**After (Tauri):**
- `CommentMark` TipTap extension (inline span with `data-comment-id`)
- Toolbar speech-bubble icon: select text → prompt → wrap selection
- `CommentsPanel` right gutter: click to scroll-to-anchor, double-click body to edit, resolve/delete
- Per-comment data (id, body, resolved, created/modified) stored in `.meta`; anchor span in content
- Footnote asterisk icon inserts `<sup class="footnote" data-body="...">●</sup>` inline node
- Compile pipeline gained `transform_footnotes` to convert to pandoc-native footnote HTML pattern (later simplified in the markdown migration)

**After (TUI):**
- F2 opens comments overlay modal
- Navigate with ↑↓, `e`/Enter edit, `r` resolve/unresolve, `d` delete, `n` add (orphan — anchored comments require text selection, Tauri-only for now)

**Commit:** `3b207f8`

---

## 2026-04-18 — Push to backup on named revision

**Change:** Both apps push to the backup remote when a named revision is saved (in addition to existing triggers: project close, periodic timer).

**Why:** Named revisions are deliberate milestones. Writers expect "Save Revision" to sync.

**After:**
- Three backup triggers: named revision, project close, interval
- TUI reads shared settings file at `~/.config/chickenscratch/settings.json`
- Fire-and-forget: backup failure doesn't fail the revision

**Commit:** `b3bb126`

---

## 2026-04-18 — TUI editor evolution: tui-textarea → edtui → ratatui-textarea

**Change:** Cycled through three TUI text widgets to land on `ratatui-textarea` 0.9 (ratatui 0.30).

**Why:** The original `tui-textarea` 0.7 doesn't support soft word-wrap — long prose lines horizontal-scroll. Unusable for writing.

**Path:**
1. **tui-textarea 0.7** — no wrap; horizontal scroll breaks prose editing.
2. **edtui 0.9.9** — has wrap but is vim-mode-only. Writers who don't know vim would be lost. Shipped with forced Insert mode as a hack.
3. **ratatui-textarea 0.9 + ratatui 0.30** — native simple-mode editing (no modes), word-wrap via `WrapMode::WordOrGlyph`, Emacs-style shortcuts.

**After:** Ratatui 0.30 upgrade (from 0.29). crossterm 0.29. Editor feels like nano/notepad — type to insert, arrows move, Ctrl+A/E start/end of line. Ctrl+W toggles wrap.

**Commits:** `4da32e3` (initial TUI), `35695fc`, `fe96b9e`, `0201857`

---

## 2026-04-18 — TUI MVP

**Change:** New `chikn` binary — a terminal UI for ChickenScratch, sharing the Rust core library.

**Why:** Writers who live in terminals, ssh sessions, tiling WMs. ~7MB binary vs ~100MB for the Tauri bundle.

**Features:** Binder tree (navigate, open, expand/collapse folders), markdown editor, save (Ctrl+S), save revision (Ctrl+R), quit with confirmation, view mode cycle.

**Commit:** `4da32e3`

---

## 2026-04-08 — v0.1.0-alpha

**Milestone:** Tagged initial test release for alpha feedback from writers.

**Tag:** `v0.1.0-alpha` (commit `ed1dfd9`)

---

## 2026-04-08 — Writing statistics + revision diff

**Change:** Per-document word counts, page estimate, reading time, daily writing history chart. Word-level revision diff (tracked-changes style: green additions, red strikethrough deletions). Auto-commit every 10 minutes when changes detected.

**Commits:** `fc63de2`, `e2884e6`, `0083030`

---

## 2026-04-08 — Keyboard shortcut customization + AI improvements

**Change:** All shortcuts configurable in Settings. AI kill switch (when disabled, hides sparkle menu + summarize button). AI via reqwest instead of curl subprocess. Word count targets per document. Browser-native spellcheck. Search result highlight in editor.

**Commits:** `52d3f14`, `0614eea`, `8d2a0b4`

---

## 2026-04-06 — Settings panel + compile pipeline

**Change:** Comprehensive Settings (General, Writing, Backup, AI, Compile). Writing settings apply to the editor dynamically. Compile dialog with title page, section separators, Shunn manuscript format preset. Per-document include-in-compile toggle, compile-order override.

**Commits:** `e48e7fa`, `64cd3f2`, `e4fce1e`, `43e6918`, `889642f`

---

## 2026-04-06 — Templates + Trash + drag-and-drop

**Change:** Document templates (Scene, Chapter, Character Sheet, Setting, Outline). Delete moves to Trash instead of permanent delete. Reimplemented drag-and-drop with mouse events instead of HTML5 drag API (more reliable).

**Commits:** `c9c8a16`, `76b09bf`, `2f13544`, `7443789`, `fecce9c`

---

## 2026-04-06 — Scrivener import + Pandoc integration

**Change:** Full Scrivener `.scriv` import with metadata, hierarchy, RTF conversion. Import of all Pandoc-supported formats. Pandoc detection with install helper. Self-healing project structure (Manuscript/Research/Trash).

**Commits:** `d2f63d1`, `10b512f`, `988f9fb`, `a1322d8`

---

## 2026-04-06 — Core app MVP

**Change:** First working end-to-end build. Tauri + React + TipTap. git2-rs for revisions. Beforeunload warning, error boundary, recent projects. Project-wide search with editor highlight.

**Commits:** `4041f9b`, `8e4fe0b`, `f77c570`

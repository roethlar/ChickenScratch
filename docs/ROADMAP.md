# ChickenScratch — Roadmap

## Current State (v0.1.0-alpha)

ChickenScratch is a functional cross-platform writing app in alpha testing. Core features implemented. Seeking feedback from writers to identify issues, missing functionality, and UX problems before stable release.

**Two frontends, one canonical storage format:**
- **Tauri + React + TipTap** — desktop GUI with WYSIWYG rich-text editing
- **Rust + ratatui + ratatui-textarea** — terminal UI (`chikn` binary)
- **Canonical storage** — Pandoc Markdown (`.md` files on disk)

Pandoc is a runtime dependency but only for compile/export and import — not for core editing. The Tauri editor uses `tiptap-markdown` for in-process markdown ↔ HTML. The TUI edits markdown directly, no conversion at all.

### What's Built

**Editor (Tauri)**
- TipTap WYSIWYG with formatting toolbar (bold, italic, underline, strike, headings, lists, blockquote, code, links)
- Inline comments (right-gutter panel) and footnotes
- Find & Replace (Ctrl+F / Ctrl+H)
- AI text operations (polish, expand, simplify, brainstorm)
- Browser-native spell check
- Auto-save, session word count, focus mode with typewriter scrolling
- Light/dark/sepia themes, print support (Ctrl+P)

**Editor (TUI)**
- Simple-mode editing (type to insert, Emacs-style shortcuts, no vim modes)
- Native soft word-wrap (toggle via Ctrl+W)
- Edit / Preview view modes (Ctrl+T)
- Comments overlay (F2) with add/edit/resolve/delete
- Save (Ctrl+S), Save Revision (Ctrl+R)

**Organization**
- Binder: drag-and-drop, context menus, folder management, width-resizable
- Manuscript/Research/Trash structure (auto-created, self-healing)
- Move to... folder picker, Empty Trash
- Templates: Scene, Chapter, Character Sheet, Setting, Outline
- Inspector: synopsis, label, status, keywords, include-in-compile, word count target, compile order
- Corkboard: card grid with grouping, AI summaries, document linking
- Manuscript preview: continuous prose with type-aware section headers
- Command palette (Ctrl+K)
- Project-wide search (Ctrl+Shift+P) with editor highlight

**Compile / Export**
- DOCX, PDF, EPUB, HTML, ODT via Pandoc
- Compile dialog with title page, section separators, Shunn manuscript format
- Per-document include/exclude and compile-order override
- Settings-driven formatting

**Import**
- Scrivener (.scriv) with metadata, hierarchy, RTF → markdown
- All Pandoc-supported formats → markdown
- Markdown folder import

**Revisions**
- Embedded git2-rs (no system git required)
- Save revision, history, restore
- Word-level diff viewer (tracked-changes style)
- Draft versions (branches): create, switch, merge
- Auto-commit every 10 minutes
- Auto-backup on close + periodic + on-named-revision

**Statistics**
- Per-document word counts with bar chart
- Word count targets with progress bar
- Page estimate, reading time
- Daily writing history chart (14-day view)

**Settings**
- General, Writing, Backup, AI (with kill switch), Compile, Shortcuts
- All keyboard shortcuts customizable

**Infrastructure**
- Error boundary, toast notifications, custom dialogs
- Window/panel state persistence
- Pandoc detection with install helper
- Recent projects, Wayland compatibility
- AI via reqwest (no curl dependency)

---

## v1.1 — Planned

### AI Streaming Responses
Stream AI output word-by-word instead of waiting for full response.
- **Requires:** Tauri async events, reqwest streaming
- **Providers:** Ollama (newline-delimited JSON), Anthropic/OpenAI (SSE)
- **UX:** Text appears progressively in editor/corkboard

### Side-by-Side Draft Comparison
Compare two draft versions (branches) visually.
- Two-pane view: old draft left, current right
- Synced scrolling
- Word-level highlighting of differences

### Remote Sync
Push/pull to GitHub, Gitea, or any git remote.
- **Settings:** Remote URL, auth (SSH key or token)
- **git2-rs:** push/pull with credential handling
- **Conflict resolution:** Show conflicts, let writer choose
- **UI:** Sync button in Revisions panel

### TUI inline-anchored comments
Currently the TUI can only add document-level (orphan) comments. Adding anchored comments requires text selection in ratatui-textarea plus a way to wrap the selected range with a comment span in the stored markdown. Doable; not yet built.

---

## Platform Packaging

### Windows (.msi)
- CI/CD via GitHub Actions
- Tauri MSI/NSIS targets
- Pandoc path detection for Windows
- File dialog testing

### Linux (Flatpak)
- Flatpak manifest with org.freedesktop.Platform
- WebKitGTK runtime
- Filesystem permissions for project dirs
- Flathub submission

### macOS Code Signing
- Apple Developer account
- Signing identity in tauri.conf.json
- Notarization via xcrun notarytool
- CI/CD automation

### Auto-Update
- tauri-plugin-updater
- GitHub Releases as update source
- Update notification toast
- Signed updates (public/private key pair)

---

## Format evolution (future)

### djot migration
[djot](https://djot.net) is the successor format to CommonMark, designed by pandoc's author to fix round-trip and attribute-handling issues. The Rust parser `jotdown` is fast and the syntax for attributes (`[text]{.class #id key=val}`) is nearly identical to what we already use with pandoc. When djot reaches 1.0 and TipTap gains a djot serializer, a format version bump to `.chikn` v2.0 using djot is worth considering — same writer-visible syntax, cleaner semantics, faster parsing.

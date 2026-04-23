# ChickenScratch — Roadmap

## Current State (v0.1.0-alpha)

ChickenScratch is a functional cross-platform writing app in alpha testing. Core features implemented. Seeking feedback from writers to identify issues, missing functionality, and UX problems before stable release.

**Five frontends, one canonical storage format:**
- **Tauri + React + TipTap** — fullest feature set; daily-driver on macOS + Linux
- **WinUI 3 + C#** (`windows/`) — native Windows app, alpha
- **SwiftUI + Liquid Glass** (`macos/`, macOS 26+) — early scaffold with writing + revisions
- **Qt6 + cxx-qt** (`linux/`, Wayland-native) — early scaffold with binder/editor/inspector/find-replace
- **Ratatui TUI** (`chikn` binary, any OS) — keyboard-first terminal editor
- **Canonical storage** — Pandoc Markdown (`.md` files on disk)

Pandoc is a runtime dependency but only for compile/export and import — not for core editing. The Tauri editor uses `tiptap-markdown` for in-process markdown ↔ HTML. The TUI and SwiftUI edit markdown directly, no conversion at all.

Not every frontend is at feature parity with Tauri — the SwiftUI, Qt6, and WinUI apps are catching up. Where a feature below says just "Editor" it's shipped on all editor frontends; feature sections named "(Tauri)" are Tauri-only until ported.

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
- Anchored inline comments (F3 wraps the current selection with a comment span)
- Save (Ctrl+S), Save Revision (Ctrl+R) with push-to-backup

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

**Revisions (Tauri)**
- Embedded git2-rs (no system git required)
- Save revision, history, restore
- Word-level diff viewer (tracked-changes style)
- Draft versions (branches): create, switch, merge
- Side-by-side draft comparison (Compare Drafts dialog)
- Auto-commit every 10 minutes
- Auto-backup on close + periodic + on-named-revision
- Remote sync: push/fetch to any git URL with HTTPS-token auth, ahead/behind status

**Statistics**
- Per-document word counts with bar chart
- Word count targets with progress bar
- Page estimate, reading time
- Daily writing history chart (14-day view)

**Settings**
- General, Writing, Backup, Remote, AI (with kill switch), Compile, Shortcuts
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

### Remote Sync — merge UX for conflicting pulls
Push, fetch, and ahead/behind status shipped in v0.1.0-alpha. Missing: when a fetch brings down commits that conflict with local work, there's no in-app resolution — writer falls back to CLI `git merge`. Plan:
- Detect divergent state in `sync_status` (ahead > 0 && behind > 0)
- Offer "Pull & merge" with a conflict summary
- For conflicts in markdown files, surface a three-way view in the revision diff UI

### Frontend parity (SwiftUI + Qt6 + WinUI)
Bring the newer native frontends up to Tauri's feature set. Highest-leverage gaps:
- SwiftUI: delete/move/reorder binder, inspector editing, comments, footnotes, compile, AI, drafts, remote sync
- Qt6/Linux: comments, footnotes, revisions UI, compile, AI, settings, templates
- WinUI: ongoing — tracked in `windows/` commit history

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

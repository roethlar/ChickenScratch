# ChickenScratch — Roadmap

## Current State (v0.1.0-alpha)

ChickenScratch is a functional cross-platform writing app in initial alpha testing. Core features are implemented. Seeking feedback from writers to identify issues, missing functionality, and UX problems before a stable release.

### What's Built

**Editor**
- TipTap WYSIWYG with formatting toolbar (bold, italic, underline, strike, headings, lists, blockquote, code, links)
- Find & Replace (Ctrl+F / Ctrl+H)
- AI text operations (select text → polish, expand, simplify, brainstorm)
- Browser-native spell check
- Auto-save with debounce (configurable interval)
- Session word count in status bar
- Focus mode with typewriter scrolling
- Light/dark/sepia themes
- Print support (Ctrl+P)

**Organization**
- Binder with drag-and-drop, context menus, folder management
- Manuscript/Research/Trash structure (auto-created, self-healing)
- Delete moves to Trash, Empty Trash permanently deletes
- Move to... folder picker
- Binder width resizable (drag edge)
- Templates: Scene, Chapter, Character Sheet, Setting, Outline
- Inspector: synopsis, label, status, keywords, include in compile, word count target, compile order
- Corkboard: card grid with grouping, AI summaries, document linking
- Manuscript preview: continuous prose with type-aware section headers
- Command palette (Ctrl+K)
- Project-wide search (Ctrl+Shift+P) with editor highlight

**Compile/Export**
- Export to DOCX, PDF, EPUB, HTML, ODT via Pandoc
- Compile dialog: title page, section separators, manuscript format preset (Shunn)
- Per-document include/exclude toggle
- Per-document compile order override
- Settings-driven formatting (font, size, spacing, margins)

**Import**
- Scrivener (.scriv) with full metadata, hierarchy, RTF conversion
- All Pandoc-supported formats (DOCX, ODT, RTF, EPUB, MD, LaTeX, etc.)
- Markdown folder import

**Revisions**
- Embedded git (git2-rs, no system git required)
- Save revision, view history, restore
- Word-level diff viewer (tracked-changes style)
- Draft versions (branches): create, switch, merge
- Auto-commit every 10 minutes
- Auto-backup on close + periodic backup
- Filesystem backup push

**Statistics**
- Per-document word counts with bar chart
- Word count targets with progress bar
- Page estimate and reading time
- Daily writing history chart (14 days)

**Settings**
- General: theme, Pandoc path, recent projects limit
- Writing: font, size, paragraph style, auto-save interval
- Backup: directory, auto-backup on close, interval
- AI: enable/disable, provider (Ollama/Anthropic/OpenAI), model, API key
- Compile: format, font, size, spacing, margins
- Shortcuts: all keyboard shortcuts customizable

**Infrastructure**
- Error boundary (graceful crash recovery)
- Toast notifications
- Custom dialog system (works in Tauri webview)
- Window/panel state persistence (localStorage)
- Pandoc detection with install helper
- Wayland compatibility

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
- Apple Developer account ($99/year)
- Signing identity in tauri.conf.json
- Notarization via xcrun notarytool
- CI/CD automation

### Auto-Update
- tauri-plugin-updater
- GitHub Releases as update source
- Update notification toast
- Signed updates (public/private key pair)

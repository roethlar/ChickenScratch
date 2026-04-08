# ChickenScratch — TODO

See [ROADMAP.md](docs/ROADMAP.md) for detailed implementation plans.

## Done

- [x] Core .chikn format (read/write/validate/self-heal)
- [x] Scrivener import (RTF->HTML, metadata, hierarchy, media, links)
- [x] Scrivener export (HTML->RTF)
- [x] Standalone converter CLI (chikn-converter)
- [x] git2-rs integration (save revision, history, restore, branches, backup)
- [x] Tauri app with React frontend
- [x] WYSIWYG HTML editor (TipTap with formatting toolbar)
- [x] Find & Replace (Ctrl+F / Ctrl+H)
- [x] Binder with mouse-based drag-and-drop, context menus, ... menu button
- [x] Rename, Move Up/Down, Move to... (folder picker), Delete (to Trash)
- [x] Empty Trash
- [x] Manuscript/Research/Trash project structure (auto-created, self-healing)
- [x] Corkboard card view with AI summaries and document linking
- [x] Inspector panel (synopsis, label, status, keywords, include in compile)
- [x] Manuscript preview (continuous prose, type-aware section headers)
- [x] Revisions UI (save revision, history, restore, draft versions)
- [x] Word-level revision diff (tracked-changes style: green additions, red strikethrough deletions)
- [x] Filesystem backup (git remote push to configurable directory)
- [x] Auto-backup on close + periodic auto-backup timer
- [x] Auto-commit every 10 minutes when changes detected
- [x] Compile/export to DOCX, PDF, EPUB, HTML, ODT
- [x] Compile dialog with title page, section separators, manuscript format preset
- [x] Per-document compile order override
- [x] Import all Pandoc-supported formats (DOCX, ODT, RTF, EPUB, MD, LaTeX, etc.)
- [x] Light/dark/sepia themes
- [x] Focus mode with typewriter scrolling
- [x] Command palette (Ctrl+K)
- [x] Project-wide search (Ctrl+Shift+P) with editor highlight on navigate
- [x] Comprehensive settings panel (General, Writing, Backup, AI, Compile, Shortcuts)
- [x] Customizable keyboard shortcuts
- [x] Writing settings apply to editor dynamically (font, size, paragraph style)
- [x] Compile settings apply to export (font, spacing, margins)
- [x] Recent projects list on welcome screen
- [x] Error boundary
- [x] Custom dialog system (works in Tauri webview)
- [x] Toast notifications
- [x] Pandoc detection with path fallbacks + install helper
- [x] Session word count in status bar
- [x] Writing statistics panel (per-doc word counts, page estimate, reading time)
- [x] Per-document word count targets with progress bar
- [x] Daily writing history chart (14-day bar chart)
- [x] Window/panel state persistence
- [x] Binder width resizing (drag edge, persisted)
- [x] include_in_compile toggle wired end-to-end
- [x] Document links on corkboard (bidirectional, persisted)
- [x] Templates (Scene, Chapter, Character Sheet, Setting, Outline)
- [x] New from Template in binder menu
- [x] AI text operations on selection (polish, expand, simplify, brainstorm)
- [x] AI kill switch (settings toggle hides all AI UI)
- [x] AI uses reqwest HTTP client (no curl dependency)
- [x] Browser-native spell check
- [x] Print support (Ctrl+P, hides chrome)
- [x] App icon
- [x] PKGBUILD for Arch Linux AUR
- [x] User guide (docs/USER_GUIDE.md)
- [x] Developer README
- [x] Wayland compatibility (WebKitGTK DMA-BUF workaround)

## Remaining

### v1.1 — Nice to Have
- [ ] AI streaming responses (word-by-word via Tauri events)
- [ ] Side-by-side draft comparison mode
- [ ] Remote sync (push/pull to GitHub/Gitea)

### Platform Packaging
- [ ] Windows testing and packaging (.msi)
- [ ] Flatpak for Linux
- [ ] Auto-update mechanism (tauri-plugin-updater)
- [ ] macOS code signing + notarization

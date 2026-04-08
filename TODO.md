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
- [x] Find & Replace
- [x] Binder with mouse-based drag-and-drop, context menus, ... menu button
- [x] Rename, Move Up/Down, Move to... (folder picker), Delete (to Trash)
- [x] Empty Trash
- [x] Manuscript/Research/Trash project structure (auto-created, self-healing)
- [x] Corkboard card view with AI summaries and document linking
- [x] Inspector panel (synopsis, label, status, keywords, include in compile)
- [x] Manuscript preview (continuous prose, type-aware section headers)
- [x] Revisions UI (save revision, history, restore, draft versions)
- [x] Filesystem backup (git remote push to configurable directory)
- [x] Auto-backup on close + periodic auto-backup timer
- [x] Compile/export to DOCX, PDF, EPUB, HTML, ODT with settings
- [x] Import all Pandoc-supported formats (DOCX, ODT, RTF, EPUB, MD, LaTeX, etc.)
- [x] Light/dark/sepia themes
- [x] Focus mode
- [x] Command palette
- [x] Project-wide search
- [x] Comprehensive settings panel (General, Writing, Backup, AI, Compile)
- [x] Writing settings apply to editor dynamically (font, size, paragraph style)
- [x] Compile settings apply to export (font, spacing, margins)
- [x] Recent projects list
- [x] Error boundary
- [x] Custom dialog system (works in Tauri webview)
- [x] Toast notifications
- [x] Pandoc detection with path fallbacks + install helper + version in Settings
- [x] Session word count in status bar
- [x] Window/panel state persistence
- [x] include_in_compile toggle wired end-to-end
- [x] Document links on corkboard (bidirectional, persisted)
- [x] Empty cards visually muted
- [x] Tree indent guide lines
- [x] Templates (Scene, Chapter, Character Sheet, Setting, Outline)
- [x] New from Template in binder menu
- [x] New documents default to Manuscript folder
- [x] App icon
- [x] PKGBUILD for Arch Linux AUR
- [x] User guide (docs/USER_GUIDE.md)
- [x] Developer README (build/architecture only)
- [x] Roadmap with implementation plans (docs/ROADMAP.md)
- [x] All tests passing, zero compiler warnings

## Remaining

### Revision Diff Viewer
- [ ] Word-level diff between revisions (tracked-changes style, not git diff)
- [ ] Additions highlighted green, deletions red/strikethrough
- [ ] Side-by-side draft comparison mode

### Search
- [x] Highlight matches in editor when navigating project search results

### Statistics
- [x] Writing statistics view (per-doc word counts, page estimate, read time)
- [x] Word count targets per document (in inspector with progress bar)
- [ ] Daily/weekly writing history chart

### Compile
- [x] Front matter (title page with title, author, word count)
- [x] Section separator customization (# # #, * * *, etc.)
- [x] Manuscript format preset (Courier, double-spaced, 1" margins)
- [x] Per-document compile order override (in Inspector, sorted at compile time)

### AI
- [ ] Replace curl shell-out with reqwest HTTP client
- [ ] Streaming responses
- [x] Text operations on selection (polish, expand, brainstorm, simplify)

### Git
- [x] Auto-commit every 10 minutes when changes detected
- [ ] Remote sync (push/pull to GitHub/Gitea)

### UI/UX
- [x] Binder width resizing (drag edge, persisted)
- [x] Spell check integration (browser-native spellcheck enabled)
- [x] Print support (Ctrl+P, hides chrome)
- [x] Keyboard shortcut customization (Settings > Shortcuts panel)

### Platform
- [ ] Windows testing and packaging (.msi)
- [ ] Flatpak for Linux
- [ ] Auto-update mechanism
- [ ] macOS code signing

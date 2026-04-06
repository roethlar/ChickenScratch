# ChickenScratch — TODO

## Done

- [x] Core .chikn format (read/write/validate/self-heal)
- [x] Scrivener import (RTF->HTML, metadata, hierarchy, media, links)
- [x] Scrivener export (HTML->RTF)
- [x] Standalone converter CLI (chikn-converter)
- [x] git2-rs integration (save revision, history, restore, branches, backup)
- [x] Tauri app with React frontend
- [x] WYSIWYG HTML editor (TipTap with formatting toolbar)
- [x] Find & Replace
- [x] Binder with drag-and-drop, context menus, rename, move, delete
- [x] Binder ... menu button (no right-click needed)
- [x] Manuscript/Research/Trash project structure
- [x] Corkboard card view with AI summaries and document linking
- [x] Inspector panel (synopsis, label, status, keywords, include in compile)
- [x] Manuscript preview (continuous prose, type-aware section headers)
- [x] Revisions UI (save revision, history, restore, draft versions)
- [x] Filesystem backup (git remote push to configurable directory)
- [x] Auto-backup on close + periodic auto-backup timer
- [x] Compile/export to DOCX, PDF, EPUB, HTML, ODT
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
- [x] Pandoc detection with path fallbacks + install helper
- [x] Pandoc version shown in Settings
- [x] Session word count in status bar
- [x] Window/panel state persistence
- [x] include_in_compile toggle wired end-to-end
- [x] Document links on corkboard (bidirectional, persisted)
- [x] Empty cards visually muted
- [x] App icon
- [x] PKGBUILD for Arch Linux AUR
- [x] User guide (docs/USER_GUIDE.md)
- [x] Developer README (build/architecture only)
- [x] All tests passing, zero compiler warnings

## Remaining

### Templates
- [ ] Default templates (Scene, Chapter, Character Sheet, Setting)
- [ ] "New from Template" in binder context menu
- [ ] "Save as Template" for documents

### Search
- [ ] Highlight matches in editor when navigating project search results

### Statistics
- [ ] Writing statistics panel (per-doc and project word counts)
- [ ] Word count targets per document
- [ ] Daily/weekly writing history

### Git
- [ ] Diff viewer (show what changed in a revision)
- [ ] Side-by-side draft comparison
- [ ] Auto-commit on configurable interval
- [ ] Remote sync (push/pull to GitHub/Gitea)

### AI
- [ ] Replace curl shell-out with reqwest HTTP client
- [ ] Streaming responses
- [ ] More AI actions (polish, expand, brainstorm)
- [ ] AI settings UI accessible from corkboard (not just Settings panel)

### Compile
- [ ] Per-document compile order override
- [ ] Section separator customization
- [ ] Front matter (title page, copyright, dedication)
- [ ] Manuscript format preset (Shunn standard submission format)

### Platform
- [ ] Windows testing and packaging (.msi)
- [ ] Flatpak for Linux
- [ ] Auto-update mechanism
- [ ] Code signing for macOS

### Polish
- [ ] Spell check integration
- [ ] Keyboard shortcut customization
- [ ] Binder width resizing
- [ ] Print support

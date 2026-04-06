# ChickenScratch — TODO

## Done

- [x] Core .chikn format (read/write/validate/self-heal)
- [x] Scrivener import (RTF->HTML, metadata, hierarchy, media, links)
- [x] Scrivener export (HTML->RTF)
- [x] Standalone converter CLI (chikn-converter)
- [x] git2-rs integration (save revision, history, restore, branches, backup)
- [x] Tauri app with React frontend
- [x] WYSIWYG HTML editor (TipTap with formatting toolbar)
- [x] Find & Replace (Cmd+F / Cmd+H)
- [x] Binder with drag-and-drop, context menus, rename, move, delete
- [x] Manuscript/Research/Trash project structure
- [x] Corkboard card view with AI summaries
- [x] Inspector panel (synopsis, label, status, keywords, include in compile)
- [x] Manuscript preview (continuous prose, type-aware section headers)
- [x] Revisions UI (save revision, history, restore, draft versions)
- [x] Filesystem backup (git remote push to configurable directory)
- [x] Auto-backup on close
- [x] Compile/export to DOCX, PDF, EPUB, HTML, ODT
- [x] Import .md/.txt files
- [x] Light/dark/sepia themes
- [x] Focus mode (Cmd+Shift+F)
- [x] Command palette (Cmd+K)
- [x] Project-wide search (Cmd+Shift+P)
- [x] Comprehensive settings panel (General, Writing, Backup, AI, Compile)
- [x] Recent projects list
- [x] Error boundary
- [x] Custom dialog system (prompt/confirm that works in Tauri)
- [x] Toast notifications
- [x] Pandoc detection with path fallbacks
- [x] App icon
- [x] PKGBUILD for Arch Linux AUR

## MVP Remaining

### Pandoc
- [ ] "Install Pandoc" helper — platform-specific install command or link
- [ ] Show detected Pandoc version in Settings

### Documentation
- [ ] User guide (docs/USER_GUIDE.md) — getting started, features, keyboard shortcuts
- [ ] In-app Help menu linking to user guide
- [ ] Update README with accurate feature state and screenshots

### Polish
- [ ] Apply writing settings (font, size, paragraph style) to editor dynamically
- [ ] Apply compile settings to export output
- [ ] Session word count tracking in status bar
- [ ] Window state persistence (size, position, panel widths)

### Known Issues
- [ ] Scrivener template docs (Short Story Format, etc.) import into Manuscript — should skip or go to Templates
- [ ] Corkboard "Summarize" button doesn't show error if Ollama isn't running
- [ ] Some imported Scrivener projects have empty-content documents that show blank cards
- [ ] One unit test failing (test_read_project_success) due to metadata field addition

## Post-MVP

### Templates
- [ ] Default templates (Scene, Chapter, Character Sheet, Setting)
- [ ] "New from Template" in binder
- [ ] "Save as Template" for documents

### Search
- [ ] Highlight matches in editor when navigating search results
- [ ] Search within current document (separate from project search)

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

### Compile
- [ ] Per-document compile order override
- [ ] Section separator customization
- [ ] Front matter (title page, copyright, dedication)
- [ ] Manuscript format preset (Shunn standard)

### Platform
- [ ] Windows testing and packaging (.msi)
- [ ] Flatpak for Linux
- [ ] Auto-update mechanism

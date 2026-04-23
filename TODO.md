# ChickenScratch — TODO

See [ROADMAP.md](docs/ROADMAP.md) for feature plans and [DEVLOG.md](DEVLOG.md) for change history.

## Done

### Architecture
- [x] Core .chikn format (read/write/validate/self-heal)
- [x] Canonical storage: Pandoc Markdown (.md files), no HTML on disk
- [x] Scrivener import (RTF → markdown via pandoc, metadata, hierarchy, media, links)
- [x] Scrivener export (HTML → RTF)
- [x] Standalone converter CLI (chikn-converter)
- [x] git2-rs integration (save revision, history, restore, branches, backup)
- [x] TUI binary (`chikn`) sharing the core library

### Editor (Tauri)
- [x] TipTap WYSIWYG with formatting toolbar
- [x] tiptap-markdown for in-process markdown ↔ HTML (no pandoc subprocess on edit)
- [x] Find & Replace (Ctrl+F / Ctrl+H)
- [x] AI text operations (select text → polish, expand, simplify, brainstorm)
- [x] Browser-native spell check
- [x] Auto-save with debounce (configurable interval)
- [x] Session word count in status bar
- [x] Focus mode with typewriter scrolling
- [x] Light/dark/sepia themes
- [x] Print support (Ctrl+P)
- [x] Inline comments (select text → speech-bubble icon), right gutter panel
- [x] Footnotes (asterisk icon → insert inline footnote)

### Editor (TUI)
- [x] ratatui 0.30 + ratatui-textarea with native soft word-wrap
- [x] Simple-mode editing (no vim modes, type-to-insert)
- [x] Edits markdown directly — zero conversion on load/save
- [x] Edit / Preview view modes (Ctrl+T to toggle)
- [x] Wrap toggle (Ctrl+W)
- [x] Comments overlay (F2) with navigate/edit/resolve/delete + new orphan comment
- [x] Anchored inline comments (F3 on a text selection wraps with comment span)

### Editor (macOS SwiftUI, Liquid Glass)
- [x] Three-pane Liquid Glass shell (NavigationSplitView, inspector, toolbar)
- [x] Read project.yaml + .meta + .md via `ChiknKit`
- [x] Writing saves (debounced 1.2s) + auto-commit every 10 min
- [x] New document (⌘N + binder toolbar) and rename (context menu)
- [x] Save Revision (⌘R) via `/usr/bin/git`

### Editor (Linux Qt6, cxx-qt)
- [x] QML three-pane shell (binder, editor, inspector) with Material Dark
- [x] Open project via native folder dialog, load documents on click
- [x] Ctrl+S save, live word count
- [x] Collapsible binder tree with chevrons
- [x] Inspector editing: title, synopsis, label, status, keywords, compile, word target
- [x] Find/replace overlay (Ctrl+F / Ctrl+H) with match count + nav + replace all

### Organization
- [x] Binder with mouse-based drag-and-drop, context menus, ... menu button
- [x] Rename, Move Up/Down, Move to... (folder picker), Delete (to Trash)
- [x] Empty Trash
- [x] Manuscript/Research/Trash structure (auto-created, self-healing)
- [x] Binder width resizable (drag edge, persisted)
- [x] Templates (Scene, Chapter, Character Sheet, Setting, Outline)
- [x] "New from Template" in binder context menu
- [x] Inspector: synopsis, label, status, keywords, include in compile, word count target, compile order
- [x] Corkboard: card grid with grouping, AI summaries, document linking
- [x] Manuscript preview: continuous prose (markdown rendered via marked)
- [x] Command palette (Ctrl+K)
- [x] Project-wide search (Ctrl+Shift+P) with editor highlight on navigate

### Compile / Export
- [x] Export to DOCX, PDF, EPUB, HTML, ODT via Pandoc (reads markdown directly)
- [x] Compile dialog: title page, section separators, Shunn manuscript format
- [x] Per-document "include in compile" toggle
- [x] Per-document compile order override
- [x] Settings-driven formatting (font, size, spacing, margins)

### Import
- [x] Scrivener (.scriv) with metadata, hierarchy, RTF conversion
- [x] All Pandoc-supported formats → markdown (DOCX, ODT, RTF, EPUB, MD, LaTeX, etc.)
- [x] Markdown folder import

### Revisions (Tauri)
- [x] Embedded git (git2-rs, no system git required)
- [x] Save revision (Ctrl+R), view history, restore
- [x] Word-level diff viewer (tracked-changes style)
- [x] Draft versions (branches): create, switch, merge
- [x] Side-by-side draft comparison (Compare Drafts dialog)
- [x] Auto-commit every 10 minutes if changes detected
- [x] Auto-backup on close + periodic backup
- [x] Push to backup on named revision (Tauri + TUI)
- [x] Remote sync: push/fetch to any git URL with HTTPS-token auth, ahead/behind status

### Statistics
- [x] Per-document word counts with bar chart
- [x] Word count targets with progress bar
- [x] Page estimate and reading time
- [x] Daily writing history chart (14-day bar chart)

### Settings
- [x] General: theme, Pandoc path, recent projects limit
- [x] Writing: font, size, paragraph style, auto-save interval
- [x] Backup: directory, auto-backup on close, interval
- [x] Remote: URL, HTTPS username/token, auto-push on named revision
- [x] AI: enable/disable (kill switch), provider, model, API key
- [x] Compile: default format, font, size, spacing, margins
- [x] Customizable keyboard shortcuts

### Infrastructure
- [x] Error boundary (graceful crash recovery)
- [x] Toast notifications
- [x] Custom dialog system (works in Tauri webview)
- [x] Window/panel state persistence (localStorage)
- [x] Pandoc detection with path fallbacks + install helper
- [x] Recent projects list on welcome screen
- [x] Wayland compatibility (WebKitGTK DMA-BUF workaround)
- [x] AI HTTP via reqwest (no curl dependency)
- [x] App icon (chicken + quill)
- [x] PKGBUILD for Arch Linux AUR
- [x] User guide (docs/USER_GUIDE.md)

## Remaining

### v1.1 — Feature work (Tauri)
- [ ] AI streaming responses (word-by-word via Tauri events)
- [ ] Remote sync — merge UX on conflicting pull (push/fetch/status shipped)

### v1.2 — Novelist features
See [docs/plans/](docs/plans/) for design specs.

**Tier 1 — Novel structure** ([plan](docs/plans/TIER1_novel_structure.md)):
- [x] Scene-level metadata in `.meta` (pov_character, location, story_time, duration_minutes, threads, characters_in_scene) — core + Tauri inspector (free-form inputs)
- [x] Format spec v1.2 section documenting the new schema
- [ ] Characters + locations as first-class entities (new top-level folders, `ChiknKit` + Rust readers, binder sections, entity editor, self-healing)
- [ ] Upgrade scene inspector inputs to entity dropdowns once entities ship
- [ ] Plot threads (`threads.yaml`, scene tagging, Threads tab in Revisions panel, binder color-dots)
- [ ] Cross-reference validation (dangling-ref warnings, non-fatal)

**Tier 2 — Writer workflow** ([plan](docs/plans/TIER2_writer_workflow.md)):
- [ ] Scrivenings mode (multi-doc continuous-edit with boundary markers)
- [ ] Session targets: words/session, deadline, total — in `project.yaml`; badge + welcome-card + stats section
- [ ] Per-document snapshots (`document_history`, `restore_document`, DocumentHistory modal)
- [ ] Timeline view (story-time ordered, POV/thread lanes)

**Tier 3 — Polish** ([plan](docs/plans/TIER3_polish.md)):
- [ ] Collections (saved structured queries with a menu-driven builder, binder section)
- [ ] Rich research (inline PDF / image / audio preview; external-open fallback; drag-drop import)
- [ ] Split editor (two independent TipTap panes with orientation toggle)

### Native-frontend parity
- [ ] SwiftUI: delete/move/reorder in binder, inspector editing, comments, footnotes, drafts, compile, AI, remote sync
- [ ] SwiftUI v1.2 read-only: display new scene metadata, characters, locations, threads
- [ ] Linux (Qt6): revisions UI, comments, footnotes, compile, AI, settings, templates, drafts, remote sync
- [ ] Linux v1.2 read-only: display new scene metadata, characters, locations, threads
- [ ] Windows (WinUI): tracked separately in `windows/` — bring to full parity

### Platform packaging
- [ ] Windows testing and packaging (.msi)
- [ ] Flatpak for Linux
- [ ] Auto-update mechanism (tauri-plugin-updater)
- [ ] macOS code signing + notarization

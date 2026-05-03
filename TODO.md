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
- [x] **Format parity (Slice A):** `fields` map round-trip, threads.yaml, session_target, characters/locations folders, foreign-key preservation through writer
- [x] **Inspector Scene section:** POV / location / story_time / duration / threads / other-characters with novelist convention keys; entity menu + create-on-the-fly; thread chips
- [x] **Binder entity sections:** Characters and Locations rendered from disk (not hierarchy); thread color dots beside docs
- [x] **Revisions parity (Slice B):** Drafts tab (create/switch/merge), per-doc history modal accessible from binder context menu (`Git.documentHistory` + `Git.restoreDocument`), Threads tab with dangling refs banner
- [x] **Stats + Timeline + binder polish (Slice C):** Stats panel (manuscript words, pages, read time, 14-day daily chart, per-doc bars), Timeline view (POV/Thread/Single lanes, story_time parsing), session badge in editor (auto-hides), binder Move to Trash / Delete Permanently / Empty Trash / Move Up/Down, auto-commit before destructive git ops (switch/merge/restore)
- [x] Round-trip checks (`swift run ChiknKitChecks`) covering all of the above — 18 cases / 65 checks

### Editor (Linux Qt6, cxx-qt)
- [x] QML three-pane shell (binder, editor, inspector) with Material Dark
- [x] Open project via native folder dialog, load documents on click
- [x] Ctrl+S save, live word count
- [x] Collapsible binder tree with chevrons
- [x] Inspector editing: title, synopsis, label, status, keywords, compile, word target
- [x] Find/replace overlay (Ctrl+F / Ctrl+H) with match count + nav + replace all
- [x] Comments (Ctrl+; on selection) + Comments section in Inspector with Go-to / Resolve / Delete
- [x] Footnotes (Ctrl+Shift+F) inserting `<sup class="footnote">` markers
- [x] Compile dialog (Ctrl+Shift+E): docx/pdf/epub/html/odt, manuscript-format preset, custom typography
- [x] Settings dialog (Ctrl+,) — General/Writing/Backup/AI/Compile/Remote tabs, round-trips ~/.config/chickenscratch/settings.json with Tauri
- [x] Live theme switch (dark/light/sepia), live editor font, auto-save timer driven by Writing.auto_save_seconds

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
- [x] AI streaming responses — `ai_transform_stream` emits `ai:chunk`/`ai:done`/`ai:error` events; AiMenu replaces selection incrementally as tokens arrive (Ollama, Anthropic, OpenAI)
- [x] Remote sync pull + merge UX — `sync_pull` returns up_to_date / fast_forward / merged / conflicts; conflict dialog offers Resolve manually / Abort / Overwrite local with remote

### Current phase — Format finalization
See [docs/plans/PHASE_FORMAT_FINALIZATION.md](docs/plans/PHASE_FORMAT_FINALIZATION.md).
- [x] Replace novelist-typed fields with generic `fields` map in `crates/core` (Document + DocumentMetadata)
- [x] Round-trip + foreign-key-preservation tests in core (53/53 lib tests pass)
- [x] Format spec v1.2 reframed as "generic UI extensibility via `fields`"
- [x] Novelist-UI convention doc (`docs/UI_CONVENTIONS_NOVELIST.md`)
- [x] Tauri: Inspector Scene section writes through generic `fields` payload
- [x] Audit all 5 UIs for round-trip preservation
- [x] Patch Windows WinUI (only frontend that dropped unknowns: closed POCO had no `Fields`); add to `DocumentMetaYaml` + `Document` + reader + writer
- [ ] End-to-end round-trip test scripted across all 5 UIs (manual procedure documented; automated test deferred)

### v1.2 — Novelist UI features (paused, resumes after format finalization)
Reframed: these are UI-layer plans now. The format ships generic extensibility; UIs implement novelist conventions on top.

**Tier 1 — Novel structure** ([plan](docs/plans/TIER1_novel_structure.md)):
- [x] Scene-level metadata via convention keys in `Document.fields` (Tauri Inspector ships free-form inputs today)
- [x] Characters + locations as first-class entities — `characters/` + `locations/` folders read by core, Tauri Binder shows entity sections, `create_entity` command
- [x] Upgrade scene inspector inputs to entity datalists with inline `+ Create character` / `+ Create location` shortcuts
- [x] Plot threads — `threads.yaml` at project root, Thread struct in core, Tauri commands (list/create/update/delete), Threads tab in Revisions, multi-select chip widget in Inspector, binder colour dots
- [x] Cross-reference validation (`validate_references` command, soft warning banner in Threads tab listing dangling pov/location/threads/characters_in_scene refs)

**Tier 2 — Writer workflow** ([plan](docs/plans/TIER2_writer_workflow.md)):
- [x] Flow mode (renamed from Scrivenings) — multi-doc continuous editing with boundary markers, Ctrl+click in binder to multi-select or open folder in flow
- [x] Session targets — `SessionTarget` on `ProjectMeta`; `get_session_progress` (today_words / days_remaining / needed_per_day); editable in Stats panel; floating SessionBadge auto-hides while writing
- [x] Per-document snapshots — `document_history`, `restore_document`, DocumentHistory modal accessible from binder context menu
- [x] Timeline view — story-time ordered scenes, POV/thread lanes, click chip to open scene

**Tier 3 — Polish** ([plan](docs/plans/TIER3_polish.md)):
- [ ] Collections (saved structured queries with a menu-driven builder, binder section)
- [ ] Rich research (inline PDF / image / audio preview; external-open fallback; drag-drop import)
- [ ] Split editor (two independent TipTap panes with orientation toggle)

### Native-frontend parity
- [x] SwiftUI v1.2: scene metadata, characters, locations, threads (full editing, not read-only)
- [x] SwiftUI: delete (Trash + permanent), move up/down, drafts, per-doc history
- [ ] SwiftUI: drag-drop reorder in binder (keyboard-only via Move Up/Down today)
- [ ] SwiftUI: rich-text editor with markdown round-trip (currently plain `TextEditor`)
- [ ] SwiftUI: comments, footnotes, find/replace, flow mode
- [ ] SwiftUI: compile/export UI, settings panel, project search, templates CRUD
- [ ] SwiftUI: AI streaming, remote sync UI (push/fetch/pull + conflict dialog)
- [ ] Linux (Qt6): AI, templates, drafts, remote sync (revisions, comments, footnotes, compile, settings shipped)
- [ ] Linux v1.2 read-only: display new scene metadata, characters, locations, threads
- [ ] Windows (WinUI): tracked separately in `windows/` — bring to full parity

### Platform packaging
- [ ] Windows testing and packaging (.msi)
- [ ] Flatpak for Linux
- [ ] Auto-update mechanism (tauri-plugin-updater)
- [ ] macOS code signing + notarization

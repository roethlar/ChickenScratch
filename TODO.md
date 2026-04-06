# ChickenScratch — TODO

## Backend: Features That Need Building

### Templates System
- [ ] Define template format (what goes in a template — name, content, metadata presets?)
- [ ] Tauri command: `list_templates(project_path)` — scan templates/ folder
- [ ] Tauri command: `create_from_template(project_path, template_id, parent_id?)` — create new doc from template
- [ ] Tauri command: `save_as_template(project_path, doc_id)` — save existing doc as reusable template
- [ ] Ship default templates: Scene, Chapter, Character Sheet, Setting, Outline

### Compile Settings
- [ ] Define compile settings schema (stored in settings/compile.yaml)
  - Include/exclude documents (per-doc toggle)
  - Document ordering overrides
  - Output formatting: font, spacing, margins, page size
  - Front matter: title page, copyright, dedication
  - Section separators (scene breaks)
  - Manuscript format preset (Courier 12pt, double-spaced, 1" margins)
- [ ] Tauri command: `get_compile_settings(project_path)` — read settings/compile.yaml
- [ ] Tauri command: `save_compile_settings(project_path, settings)` — write settings
- [ ] Update `compile_project()` to respect compile settings instead of just dumping all HTML
- [ ] Per-document "include in compile" flag (already in spec, not wired up)

### Full-Text Search
- [ ] Tauri command: `search_project(project_path, query)` — search content across all documents
- [ ] Return results with doc ID, name, matching line, context snippet
- [ ] Consider tantivy or simple grep-style search for v1

### Writing Statistics
- [ ] Tauri command: `get_project_stats(project_path)` — per-document and total word counts
- [ ] Per-document word count targets (store in .meta)
- [ ] Project-level word count goal
- [ ] Session word count tracking (words written since app opened)
- [ ] Daily/weekly writing history (store in settings/)

## Backend: Polish

### Git Integration
- [x] git2-rs integration (already working)
- [ ] Auto-commit on configurable interval (e.g. every 10 minutes of active writing)
- [ ] Diff viewer data: `get_revision_diff(project_path, commit_id)` — return per-file diffs
- [ ] Compare two drafts: `diff_drafts(project_path, draft_a, draft_b)`
- [ ] Remote sync: `push_remote(project_path, remote_url)` / `pull_remote()`

### Compile/Export
- [ ] Respect per-document "include in compile" toggle
- [ ] Section separator customization
- [ ] Manuscript format preset (standard submission format)
- [ ] Progress callback for long compilations

### AI Features
- [ ] Replace curl shell-out with reqwest HTTP client
- [ ] Streaming responses for long summaries
- [ ] AI settings UI (provider selection, API key input, model selection)

## Frontend: Wire Up Existing Backend

### Templates UI
- [ ] "New from Template" option in binder context menu / new doc dialog
- [ ] "Save as Template" in document context menu
- [ ] Templates panel or section in binder showing available templates

### Compile Settings UI
- [ ] Compile settings dialog (accessible from export button)
- [ ] Per-document "include in compile" checkbox in inspector
- [ ] Document ordering preview before compile
- [ ] Format-specific options (page size for PDF, etc.)

### Search UI
- [ ] Search panel (Ctrl+Shift+F) — search across all documents
- [ ] Results list with click-to-navigate
- [ ] Highlight matches in editor when navigating results

### Statistics UI
- [ ] Statistics panel showing project/chapter word counts
- [ ] Word count target progress bars
- [ ] Session word count in status bar
- [ ] Writing streak / history chart

### Revisions UI
- [ ] Diff viewer — show what changed in a revision
- [ ] Side-by-side draft comparison
- [ ] Auto-save revision indicator

### Inspector Enhancements
- [ ] "Include in compile" toggle
- [ ] Word count target per document
- [ ] Custom metadata fields

## Infrastructure

- [ ] Error boundary (React) — catch component crashes gracefully
- [ ] Beforeunload check — warn if unsaved changes on close
- [ ] Recent projects list (persist in app settings, show on welcome screen)
- [ ] Window state persistence (size, position, panel widths)
- [ ] Pandoc availability check on startup with helpful error

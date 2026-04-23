# Tier 3 — Polish

**Priority:** v1.2, lower leverage
**Status:** Planned
**Format impact:** Collections add to `project.yaml`; the other two are pure UX
**Depends on:** Collections are more useful once Tier 1 structured fields exist

Three nice-to-have features that close obvious gaps against Scrivener. Each is small, independent, and ships whenever it gets done — don't let them block Tier 1 / Tier 2.

---

## 1. Collections (saved structured queries)

### Why

Writers want named filters: "Sarah's scenes," "still in draft," "climax + resolution." Today you'd retype a search every time. Scrivener calls this "Collections"; yWriter calls it "Views." They're sticky saved queries over the binder.

Tier 1 makes this way more powerful: once scenes have `pov_character`, `threads`, `location`, structured queries stop being vaguely-match-text and become precise.

### Format changes

Add to `project.yaml`:

```yaml
collections:
  - id: 01234abcd
    name: Sarah's scenes
    icon: person                  # optional Lucide icon name
    query:
      all:
        - field: pov_character
          equals: sarah-bennett
        - field: status
          equals: Draft
  - id: 56789efgh
    name: Climax + Resolution
    query:
      any:
        - field: label
          equals: Climax
        - field: label
          equals: Resolution
```

Query schema — minimal JSON-Logic-esque, Rust-parseable:
- `all: [clauses]` — AND
- `any: [clauses]` — OR
- `not: clause` — NOT
- Leaf clause: `{field, equals|contains|in|exists, value}`

Fields supported initially: `pov_character`, `location`, `status`, `label`, `threads` (list membership), `keywords` (list membership), `include_in_compile`, `word_count` (numeric comparison).

### UX

- **Binder gains a "Collections" section** below Trash.
- **"+ New Collection"** opens a query builder: add rows of `field / operator / value`, group with AND/OR. Save, name, pick icon.
- **Click a collection** → binder's right pane shows just the matching scenes, in manuscript order. Clicking a scene opens it normally.
- **Edit / delete** via context menu on the collection.

### Implementation

- **Rust core:** new `Collection` struct in `project::types`, query evaluation in `project::query`. Reader/writer preserves unknown operators for forward-compat.
- **Tauri commands:** `list_collections`, `create_collection`, `update_collection`, `delete_collection`, `run_collection(id) -> Vec<DocumentID>`. Reuses existing project data — no new indexing.
- **UI:** `components/collections/CollectionsSection.tsx` in the binder; `CollectionEditor.tsx` modal.

### Scope limits

- **Read-only collections.** They filter; they don't bulk-edit. A v1.3 "change status on all matching" is conceivable but dangerous.
- **Query builder, not a DSL text box.** No free-form query parsing in v1.2 — menu-driven only. Writers aren't typing `pov_character=sarah AND status=Draft` by hand.

### Open questions

- Should collections be order-preserving (manuscript order) or custom? Proposal: manuscript order by default, with a future option for manual drag-to-reorder within a collection.
- Should deleting a character update collections that reference them? Proposal: yes — remove the clause referencing the deleted id and warn.

---

## 2. Rich research (inline PDF/image preview)

### Why

Writers drop reference images, PDFs, and the occasional audio file into `research/`. Right now the binder treats them like any other file — clicking does nothing useful. Scrivener lets you view PDFs, images, and web pages in-app as part of the research material. This is the smallest gap with Scrivener that writers notice immediately.

### Format changes

None. Research folder already accepts arbitrary file types — the format spec is silent on what lives there (correctly). We just need to *render* more of them.

### UX

Detect file extension on click in the binder:

- **Images** (`.png`, `.jpg`, `.jpeg`, `.webp`, `.gif`, `.heic`) → replace the editor pane with an image viewer (zoom/fit-window).
- **PDFs** (`.pdf`) → embedded PDF viewer. Tauri: use a `<iframe src="...">` with Tauri's asset protocol; on the WinUI frontend use WebView2's native PDF viewer; on SwiftUI use `QuickLookThumbnailing` / `PDFKit`.
- **Plain text / logs** (`.txt`, `.log`) → text view, read-only.
- **Markdown research notes** (`.md`) — already work; no change.
- **Audio** (`.mp3`, `.wav`, `.m4a`) → HTML5 `<audio controls>` in the editor pane.
- **Other / unknown** → show a placeholder with size, mtime, and "Open externally" button (routes through `open` / `xdg-open` / `Start-Process`).

Drag-and-drop: dragging a file from Finder/Explorer onto the binder copies it into `research/` at the drop location and opens it.

### Implementation

- **Tauri backend:** no new commands for viewing (the webview renders via asset protocol). New `import_file_to_research(project_path, source_path, parent_id)` that copies the file in, adds a hierarchy entry, commits.
- **UI:** new `components/editor/MediaViewer.tsx` that dispatches by file extension. PDF via `<iframe>` or `react-pdf` (iframe is simpler and already works with Tauri's asset protocol).
- **Binder drag-drop:** HTML5 drag-and-drop of external files — Tauri 2 supports this natively via `window.__TAURI__` file-drop events.

### Scope limits

- **No editing of non-markdown files** — just viewing. Writers don't edit PDFs in a writing app.
- **No preview for .docx / .odt.** Writers wanting Word preview can open externally.
- **Size cap:** files over 100 MB warn before copying. Git will hate large binaries regardless.

### Open questions

- Git LFS for large media? Proposal: out of scope for v1.2. Mention in docs as "big PDFs bloat your repo; keep them elsewhere and link."
- Should images get a caption field? Proposal: no — keep them as plain files. Captions live in the markdown that references the image.

---

## 3. Split editor

### Why

Consistency passes need two docs open. "Does Sarah's dialogue in Chapter 8 match her voice in Chapter 2?" With one editor pane you're flipping back and forth. Scrivener's split editor (horizontal or vertical) is the fix. Every writer who uses it loves it.

Scrivenings mode (Tier 2) covers "edit continuous prose"; split editor covers "edit two unrelated documents simultaneously."

### UX

- **Binder context menu gains "Open in split"** — opens the document in a second pane below the current editor (horizontal split by default).
- **Keyboard:** `Cmd/Ctrl+\` (currently Toggle Binder — reassign to `Cmd/Ctrl+Shift+\`) opens the current selection in a split, or toggles split off if already split.
- **Pane controls**: each pane has its own title header with a close button. Clicking a pane focuses it; Cmd/Ctrl+1/2 jumps between panes.
- **Both panes are full editors:** toolbar, formatting, auto-save, comments all work independently.
- **Orientation toggle**: small button in the split divider rotates horizontal ↔ vertical. Persisted per project.

### Implementation

- **Frontend-only.** Zustand gains `editor.split: { topDocId, bottomDocId, orientation } | null`.
- **Rendering:** `components/editor/EditorArea.tsx` branches on split state — renders either one `<Editor>` or two inside a flex container with a draggable divider.
- **Each pane owns a TipTap instance** — no shared state, no weird cross-pane cursor issues. Independent auto-save.
- **Active doc** for Inspector purposes: whichever pane has focus. Store tracks `activePaneId`.

### Scope limits

- **Two panes max.** Three+ editor panes in a writing app is a bad idea — use your second monitor.
- **No cross-pane drag-and-drop** of text fragments between docs in v1.2. (TipTap can do this; scope cut for launch.)
- **Split doesn't nest with scrivenings.** Starting scrivenings collapses split; starting split collapses scrivenings. Either mode, not both.

### Open questions

- Remember split state across sessions? Proposal: yes — persist in localStorage so restoring a project restores the split.
- Can the two panes show different views of the same doc (e.g., one renderered, one source)? Proposal: no — two different docs or nothing. "Different views of the same doc" is a separate, narrower feature (v1.3 if someone asks).

---

## Cross-frontend rollout

All three are Tauri-first. The others skip them for v1.2:
- **TUI** — split editor makes no sense; rich research is limited to text; collections could ship as a F-key menu. Defer.
- **SwiftUI / Qt6** — collections useful enough to port (schema is in the project.yaml); rich research and split editor skip for v1.2.
- **WinUI** — follows its own roadmap.

# ChickenScratch Editor — Design Document

**Status:** Shipped (v0.1.0-alpha). Historical record of the design the editor was built to. Phases 1–6 are all delivered; this document is preserved to explain the *why* behind current structure rather than to plan future work.
**Date:** 2026-04-02 (original); last major update 2026-04-21
**Scope:** Cross-platform Tauri editor for .chikn writing projects

---

## Goal

Build a single cross-platform editor (Windows, macOS, Linux) that writers would choose over Scrivener. It must feel like a focused writing tool, not a developer IDE. The editor understands .chikn natively, imports Scrivener and Markdown, uses embedded git for revisions, and gets out of the writer's way.

---

## Tech Stack

| Layer | Technology | Rationale |
|-------|-----------|-----------|
| App shell | Tauri 2.0 | One codebase, native webview, Rust backend already exists |
| Backend | Rust (chickenscratch-core) | Proven .chikn I/O, Scrivener conversion, snapshots — 60 tests passing |
| Frontend | React 18 + TypeScript | Ecosystem, TipTap compatibility, Tauri integration |
| Editor | TipTap v2 (ProseMirror) | Best-in-class rich text for web; Markdown serialization, extensible |
| Styling | Tailwind CSS 4 | Rapid UI development, easy theming |
| State | Zustand | Minimal, proven with React |
| Git | git2-rs | Embedded git — no system git dependency for users |
| Icons | Lucide React | Clean, consistent icon set |

**External dependency:** Pandoc is required only for import (Scrivener RTF, DOCX, ODT, etc.) and compile/export. The edit path uses `tiptap-markdown` in-process for markdown ↔ HTML — no pandoc subprocess fires on load or save, so routine writing works with pandoc missing. See DEVLOG 2026-04-18 for the migration rationale.

---

## Architecture

```
┌─────────────────────────────────────────────┐
│                  Tauri Window                │
│  ┌───────────────────────────────────────┐  │
│  │           React Frontend              │  │
│  │  ┌─────────┬──────────┬───────────┐   │  │
│  │  │ Binder  │  Editor  │ Inspector │   │  │
│  │  │ (tree)  │ (TipTap) │ (metadata)│   │  │
│  │  └─────────┴──────────┴───────────┘   │  │
│  │          Zustand Store                │  │
│  └──────────────┬────────────────────────┘  │
│                 │ invoke()                   │
│  ┌──────────────┴────────────────────────┐  │
│  │        Tauri Command API (Rust)       │  │
│  │  ┌────────────────────────────────┐   │  │
│  │  │     chickenscratch-core        │   │  │
│  │  │  (format, scrivener, git, etc) │   │  │
│  │  └────────────────────────────────┘   │  │
│  └───────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

The frontend never touches the filesystem directly. All I/O goes through Tauri `invoke()` calls to Rust commands that use `chickenscratch-core`.

---

## UI Layout

The editor has three modes that shift the interface:

### Writing Mode (default)

```
┌──────────────────────────────────────────────────┐
│  [=] Project Name              [word count] [⚙]  │  ← Header bar (minimal)
├────────┬─────────────────────────────────────────┤
│        │                                         │
│ Binder │              Editor                     │
│  (tree │         (TipTap WYSIWYG)                │
│   nav) │                                         │
│        │     Comfortable column width,           │
│  240px │     centered, great typography           │
│        │                                         │
│        │                                         │
├────────┴─────────────────────────────────────────┤
│  Status: Draft · 2,547 words · Auto-saved        │  ← Status bar
└──────────────────────────────────────────────────┘
```

### Focus Mode (distraction-free)

Everything hides except the text. The editor content centers on screen with generous margins. A faint word count floats at the bottom. Moving the mouse to the left edge reveals the binder; moving to the top reveals the toolbar. `Esc` exits focus mode.

```
┌──────────────────────────────────────────────────┐
│                                                  │
│                                                  │
│                                                  │
│           The cursor blinks in the               │
│           center of the page. Nothing            │
│           else exists. Just the words.           │
│                                                  │
│                                                  │
│                                                  │
│                                    2,547 words   │
└──────────────────────────────────────────────────┘
```

### Organize Mode

Binder expands, inspector panel appears on right with metadata. Optional corkboard replaces editor area.

```
┌──────────────────────────────────────────────────┐
│  [=] Project Name         [Corkboard] [Editor]   │
├────────┬──────────────────────────┬──────────────┤
│        │                          │  Inspector   │
│ Binder │     Editor / Corkboard   │              │
│        │                          │  Synopsis:   │
│  tree  │     [index] [cards]      │  Label:      │
│  nav   │     [laid ] [out  ]      │  Status:     │
│        │     [here ] [     ]      │  Keywords:   │
│  280px │                          │  Target:     │
│        │                          │   240px      │
├────────┴──────────────────────────┴──────────────┤
│  Status bar                                      │
└──────────────────────────────────────────────────┘
```

---

## Features by Phase

### Phase 1: App Shell & File I/O

Get Tauri running, wire up the core library, open and save projects.

- Tauri 2.0 project scaffold (React + TypeScript + Tailwind)
- Tauri command API wrapping chickenscratch-core:
  - `create_project`, `load_project`, `save_project`
  - `create_document`, `update_document`, `delete_document`, `get_document`
  - `add_to_hierarchy`, `remove_from_hierarchy`, `move_node`, `reorder_node`
  - `import_scrivener_project`, `export_to_scrivener`
- Welcome screen: Create Project, Open Project, Recent Projects
- File dialogs (native via Tauri dialog plugin)
- Import .scriv files (calls existing converter)
- Import .md files (new: create .chikn from a folder of .md files)
- Basic error display (toast/notification)

### Phase 2: Editor Core

Make typing feel great.

- TipTap editor with proper Markdown round-trip:
  - Serialize TipTap JSON -> Markdown on save
  - Parse Markdown -> TipTap JSON on load
  - Uses `tiptap-markdown` (in-process, `html: true` so custom inline HTML — comment spans, footnotes — round-trips untouched)
- Supported formatting:
  - Headings (H1-H4)
  - Bold, italic, strikethrough
  - Block quotes
  - Ordered and unordered lists
  - Code blocks
  - Horizontal rules
  - Links
  - Footnotes (extended)
- Typography:
  - Default serif font for prose (Georgia, Literata, or similar)
  - Comfortable line height (1.6-1.8)
  - Max content width (~680px, centered)
  - Generous paragraph spacing
- Auto-save:
  - Debounced (2 second delay after last keystroke)
  - Writes .md content + updates .meta modified timestamp
  - Visual indicator in status bar ("Saving..." / "Saved")
- Live word count in status bar (per-document and session)
- Formatting toolbar:
  - Minimal — icon buttons, no text labels
  - Appears at top of editor or as floating bubble on selection
  - Can be hidden entirely

### Phase 3: Binder & Organization

Scrivener's killer feature: the document tree.

- Binder sidebar:
  - Tree view of project hierarchy
  - Documents (click to open in editor)
  - Folders (expand/collapse, click to see contents)
  - Top-level sections: Manuscript, Research, Templates (from .chikn spec)
  - Visual indicators: modified (dot), word count, compile status
- Document operations:
  - Create document (in folder or root)
  - Create folder
  - Rename (inline edit)
  - Delete (with confirmation)
  - Drag-and-drop reordering and reparenting
- Inspector panel (right sidebar, toggle):
  - Document title
  - Synopsis (editable text area)
  - Label (dropdown/tag)
  - Status (dropdown: Draft, Revised, Final, etc.)
  - Keywords (tag input)
  - Word count / target
  - Created / modified timestamps
  - Include in compile (toggle)
- Corkboard view (optional, replaces editor area):
  - Index cards laid out in grid
  - Each card shows: title, synopsis snippet, label color, status
  - Click card to open in editor
  - Drag cards to reorder

### Phase 4: Focus & Polish

Make it feel like a real product.

- Theme system:
  - Light theme (warm, paper-like)
  - Dark theme (true dark, easy on eyes at night)
  - Sepia theme (classic writing app feel)
  - Theme affects editor, binder, inspector, and all chrome
  - CSS custom properties for easy theming
- Focus mode:
  - `Cmd/Ctrl+Shift+F` to enter
  - Hides binder, inspector, toolbar, status bar
  - Editor content centered with large margins
  - Optional typewriter scrolling (active line stays at vertical center)
  - Subtle fade on non-active paragraphs (optional)
  - Mouse-edge hover to reveal binder/toolbar
  - `Esc` to exit
- Command palette:
  - `Cmd/Ctrl+K` to open
  - Fuzzy search across: documents, commands, settings
  - Quick document switching without touching the binder
  - Actions: "New Document", "Save Revision", "Toggle Focus Mode", etc.
- Keyboard shortcuts:
  - `Cmd/Ctrl+S` — Save (force, even with auto-save)
  - `Cmd/Ctrl+N` — New document
  - `Cmd/Ctrl+O` — Open project
  - `Cmd/Ctrl+P` — Command palette (alias)
  - `Cmd/Ctrl+B` — Toggle binder
  - `Cmd/Ctrl+I` — Toggle inspector (when visible)
  - `Cmd/Ctrl+\` — Toggle focus mode
  - `Cmd/Ctrl+Shift+F` — Focus mode
  - Standard formatting shortcuts (Cmd+B bold, etc.)
- Recent projects list (persisted in app settings)
- Window state persistence (size, position, panel widths)

### Phase 5: Git Integration (Revisions)

Writers see "Revisions" — never "git".

The spec requires every .chikn to be a git repo. We use `git2-rs` in the Rust backend so users never need git installed.

- Auto-initialize:
  - `git init` on project creation
  - Write .gitignore (revs/, .DS_Store, etc.)
  - Initial commit
- Save Revision (`Cmd/Ctrl+Shift+S`):
  - Prompt for short description (pre-filled: "Update: {current doc name}")
  - `git add . && git commit -m "{description}"`
  - Writer-friendly commit message format
- Revision History panel:
  - Timeline of commits with descriptions and timestamps
  - Click to view state at that point
  - Restore to a previous revision (creates new commit, never destructive)
- Draft Versions (branches):
  - "New Draft Version" creates a git branch
  - Switch between drafts via UI
  - "Merge Draft" merges branch back
  - Writer-friendly names, no "branch" terminology shown
- Compare Drafts:
  - Side-by-side diff of current vs. previous revision
  - Highlight additions (green) and deletions (red)
  - Per-document comparison
- Automatic snapshots:
  - Periodic auto-commit (configurable interval, default: every 10 minutes of active writing)
  - Message: "Auto: {timestamp}"
  - Distinct from manual "Save Revision"

### Phase 6: Import & Export

- Import Scrivener (.scriv):
  - File dialog to select .scriv folder
  - Progress indicator (RTF conversion can be slow)
  - Preview imported structure before confirming
  - Creates new .chikn project
- Import Markdown:
  - Select folder of .md files
  - Auto-detect hierarchy from folder structure
  - Or import single .md as new document into existing project
- Import plain text (.txt):
  - Single file import as new document
- Export / Compile:
  - Merge selected documents into single output
  - Output formats (via Pandoc): DOCX, PDF, EPUB, HTML, RTF
  - Basic compile settings: include/exclude documents, ordering
  - Standard manuscript format option (Courier, double-spaced, etc.)
- Export to Scrivener (.scriv):
  - Calls existing export_to_scriv
  - Full metadata round-trip

---

## Tauri Command API

New `crates/tauri/` crate wrapping `chickenscratch-core`. Commands organized by domain:

### Project Commands
```
create_project(name, path) -> Project
load_project(path) -> Project
save_project(project) -> Project
close_project()
get_recent_projects() -> Vec<RecentProject>
```

### Document Commands
```
create_document(project_path, name, parent_id?) -> (Project, Document)
get_document(project_path, doc_id) -> Document
update_document(project_path, doc_id, content) -> ()
delete_document(project_path, doc_id) -> Project
rename_document(project_path, doc_id, new_name) -> Project
```

### Hierarchy Commands
```
move_node(project_path, node_id, new_parent_id?, new_index?) -> Project
```

### Import/Export Commands
```
import_scrivener(scriv_path, output_path) -> Project
import_markdown(md_path, project_path?) -> Project
export_to_scrivener(project_path, output_path) -> ()
compile(project_path, format, options) -> PathBuf
```

### Git Commands (new)
```
git_init(project_path) -> ()
git_save_revision(project_path, message) -> ()
git_list_revisions(project_path) -> Vec<Revision>
git_restore_revision(project_path, commit_id) -> ()
git_create_draft(project_path, name) -> ()
git_list_drafts(project_path) -> Vec<Draft>
git_switch_draft(project_path, name) -> ()
git_merge_draft(project_path, name) -> ()
git_diff(project_path, commit_a, commit_b) -> Vec<FileDiff>
```

### Settings Commands
```
get_settings() -> AppSettings
update_settings(settings) -> ()
```

---

## Data Flow

### Opening a project
1. User clicks "Open" or selects from recent list
2. Frontend calls `load_project(path)` via Tauri invoke
3. Rust reads project.yaml, all .md and .meta files
4. Returns `Project` struct (hierarchy + documents map)
5. Frontend stores in Zustand, renders binder tree
6. First document in manuscript auto-selected, content loaded into TipTap

### Editing a document
1. User types in TipTap editor
2. On each change, TipTap fires `onUpdate`
3. Debouncer waits 2s of inactivity
4. Frontend serializes TipTap content to Markdown
5. Calls `update_document(project_path, doc_id, markdown_content)`
6. Rust writes .md file, updates .meta modified timestamp
7. Status bar shows "Saved"

### Saving a revision
1. User presses Cmd+Shift+S or clicks "Save Revision"
2. Modal prompts for description
3. Frontend calls `git_save_revision(project_path, message)`
4. Rust: `git add . && git commit -m "{message}"`
5. Revision appears in history panel

### Importing Scrivener
1. User selects File > Import > Scrivener Project
2. Native file dialog for .scriv folder
3. Second dialog for output location
4. Frontend calls `import_scrivener(scriv_path, output_path)`
5. Rust converts (RTF->MD via Pandoc, builds hierarchy)
6. Returns new Project, frontend opens it

---

## Typography Spec

The editor's text rendering is the product's most important visual element.

```css
/* Prose editing area */
.editor-content {
  font-family: 'Literata', 'Georgia', 'Times New Roman', serif;
  font-size: 18px;
  line-height: 1.75;
  max-width: 680px;
  margin: 0 auto;
  padding: 3rem 1.5rem;
  color: #1a1a1a;           /* Light theme */
}

/* Headings */
h1 { font-size: 1.8em; margin-top: 2em; }
h2 { font-size: 1.4em; margin-top: 1.6em; }
h3 { font-size: 1.15em; margin-top: 1.3em; }

/* Blockquotes */
blockquote {
  border-left: 3px solid #d0d0d0;
  padding-left: 1.2em;
  color: #555;
  font-style: italic;
}

/* Dark theme overrides */
.theme-dark .editor-content {
  color: #d4d4d4;
  background: #1e1e1e;
}

/* Focus mode */
.focus-mode .editor-content {
  max-width: 600px;
  font-size: 20px;
  padding: 20vh 2rem;
}
```

Font loading: Bundle Literata (Google Fonts, OFL) with the app so it works offline.

---

## File Structure (as shipped)

```
ChickenScratch/
├── Cargo.toml                   # Workspace
├── crates/
│   ├── core/                    # chickenscratch-core library
│   ├── cli/                     # chikn-converter CLI
│   └── tui/                     # chikn terminal UI (ratatui)
├── src-tauri/                   # Tauri app backend
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── icons/
│   ├── src/
│   │   ├── main.rs              # Tauri entry point
│   │   └── commands/            # Tauri command handlers
│   │       ├── mod.rs
│   │       ├── project.rs
│   │       ├── document.rs
│   │       ├── io.rs
│   │       ├── git.rs
│   │       ├── ai.rs
│   │       ├── search.rs
│   │       ├── templates.rs
│   │       └── settings.rs
│   └── capabilities/
├── windows/                     # WinUI 3 (Windows App SDK, C#) frontend
│   ├── ChickenScratch.Core/     # C# library: .chikn I/O, git, compile
│   └── ChickenScratch.App/      # App shell
├── ui/                          # React frontend (Tauri webview)
│   ├── package.json
│   ├── index.html
│   ├── vite.config.ts
│   ├── tailwind.config.ts
│   ├── tsconfig.json
│   ├── src/
│   │   ├── main.tsx
│   │   ├── App.tsx
│   │   ├── commands/            # Typed Tauri invoke wrappers
│   │   │   ├── project.ts
│   │   │   ├── document.ts
│   │   │   ├── import.ts
│   │   │   └── git.ts
│   │   ├── stores/              # Zustand stores
│   │   │   ├── projectStore.ts
│   │   │   ├── editorStore.ts
│   │   │   └── settingsStore.ts
│   │   ├── components/
│   │   │   ├── welcome/         # Welcome screen
│   │   │   ├── binder/          # Document tree sidebar
│   │   │   ├── editor/          # TipTap editor + toolbar
│   │   │   ├── inspector/       # Metadata panel
│   │   │   ├── corkboard/       # Index card view
│   │   │   ├── revisions/       # Git history UI
│   │   │   ├── command-palette/ # Cmd+K palette
│   │   │   └── shared/          # Buttons, modals, toast, etc.
│   │   ├── hooks/               # React hooks
│   │   ├── lib/                 # Markdown serialization, utils
│   │   └── styles/
│   │       ├── editor.css       # TipTap/ProseMirror overrides
│   │       └── themes/          # Light, dark, sepia CSS vars
│   └── public/
│       └── fonts/               # Bundled Literata font
├── samples/
├── docs/
└── README.md
```

---

## What "Distraction-Free" Means Concretely

1. **Default state is calm.** The binder is narrow. The toolbar is minimal or hidden. No ribbon, no tabs, no sidebar ads. The editor area dominates.

2. **Focus mode is one keypress away.** Everything disappears except the text. No chrome, no cursor blinking in a toolbar, no notification badges.

3. **Typewriter scrolling.** The active line stays at the vertical center of the screen. The page scrolls around the cursor, not the other way around. This keeps the writer's eyes in one place.

4. **No formatting anxiety.** The WYSIWYG editor shows formatting inline (bold text looks bold) but there are no format painters, style panes, or "Normal" dropdowns. Markdown syntax is invisible but present.

5. **Sounds and motion are absent.** No animations that delay input. No bouncy transitions. No sounds. Instant response to every keystroke.

6. **Session awareness.** A subtle word count shows progress. An optional session target ("500 words today") provides gentle motivation without pressure.

---

## What Scrivener Organization Means Concretely

1. **The Binder is a tree, not a file list.** Writers structure novels as folders (Parts, Chapters) containing documents (Scenes). Drag-and-drop rearrangement is essential — restructuring a novel means moving scenes between chapters.

2. **Every document has metadata.** Synopsis (what happens in this scene), label (POV character, timeline), status (Draft, Revised, Done), keywords. This lives in the inspector, not the document body.

3. **The Corkboard is a spatial overview.** Each document becomes an index card showing its title and synopsis. Writers use this to see narrative flow, spot pacing issues, rearrange structure visually.

4. **Compile = controlled output.** Select which documents to include, in what order, with what formatting. The manuscript in the editor is a collection of fragments; compile produces a unified document.

5. **Research lives alongside writing.** Character sheets, world-building notes, reference images — all in the same project, accessible from the binder, but never included in compile output.

---

## Implementation Phases & Milestones

### Phase 1: App Shell & File I/O
**Goal:** Open a .chikn project and see its contents.
**Milestone:** Can create, open, and save a project. Documents appear in a tree. Clicking a document shows its content.

### Phase 2: Editor Core
**Goal:** Writing feels great.
**Milestone:** Can write prose with formatting. Auto-save works. Typography is polished. Word count is live.

### Phase 3: Binder & Organization
**Goal:** Organize like Scrivener.
**Milestone:** Full drag-and-drop binder. Create/delete/rename. Inspector shows and edits metadata. Corkboard view works.

### Phase 4: Focus & Polish
**Goal:** Ship-quality UX.
**Milestone:** Dark/light/sepia themes. Focus mode with typewriter scrolling. Command palette. Keyboard shortcuts. Feels like a real product.

### Phase 5: Git Integration
**Goal:** Revisions without git knowledge.
**Milestone:** Save Revision, view history, restore, draft versions, compare.

### Phase 6: Import & Export
**Goal:** Get work in and out.
**Milestone:** Import .scriv, .md. Compile to DOCX/PDF/EPUB. Export to .scriv.

---

## Non-Goals (explicitly out of scope for v1)

- Mobile/tablet support
- Cloud sync as a hosted service (git remote push-to-backup is the mechanism)
- Collaboration/multi-author (future)
- Plugin/extension system
- Screenplay/script formatting

Originally listed here, now shipped: AI writing assistance (text operations and summaries) and spell check (browser-native in the Tauri webview).

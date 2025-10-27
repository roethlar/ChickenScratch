# GTK4 WYSIWYG Editor Design

## Goals
- Provide a native GTK4 desktop editor for `.chikn` projects.
- Offer WYSIWYG editing on top of the Markdown-based manuscript files.
- Reuse the existing Chicken Scratch Rust core (project/document reader & writer).
- Maintain lossless round-trip with the `.chikn` specification (project.yaml + `.md` + `.meta`).

## High-Level Architecture
- **Crate**: `gtk-editor` (binary) alongside the existing Tauri crate.
- **Dependency**: Depends on `chicken-scratch` (`src-tauri`) as a library for `.chikn` IO.
- **Subsystems**:
  - `app`: GTK application bootstrap and signal wiring.
  - `state`: Holds current project/document state, dirty flags, and shared references.
  - `ui`: Builds widgets (window, tree view, formatting toolbar, editor pane).
  - `markdown`: Handles Markdown ⇄ rich text conversion for the GTK `TextBuffer`.
  - `project`: Lightweight façade around core read/write operations.

## UI Overview
- **Header Bar**: App title, open/save actions, file information.
- **Project Tree** (`TreeView` + `TreeStore`):
  - Mirrors `Project.hierarchy` (folders/documents).
  - Selecting a document loads it into the editor.
- **Formatting Toolbar**:
  - Toggle buttons for **Bold**, *Italic*, ~~Strikethrough~~, code, headings (H1/H2).
  - Buttons manipulate `TextTag`s on the editor buffer.
- **Editor Pane** (`TextView`):
  - Displays formatted text.
  - Listens for changes to mark document as dirty.
  - Applies text tags supplied by the Markdown parser.
- **Status Bar**:
  - Shows document name, word count, unsaved state.

## Data Flow
1. **Open Project**:
   - File chooser selects a `.chikn` directory.
   - `project::open` calls `core::project::reader::read_project`.
   - Hierarchy populates the tree view; first document auto-loads.
2. **Load Document**:
   - Markdown + metadata fetched from `Project.documents`.
   - `markdown::apply_to_buffer` converts to plain text + formatting tags inside the `TextBuffer`.
3. **Edit Document**:
   - Text/tag changes set `state.dirty = true`.
   - Toolbar toggles manipulate tags via reusable helpers in `ui`.
4. **Save Document**:
   - `markdown::buffer_to_markdown` reconstructs Markdown for the active document.
   - Document entry updated in-memory; metadata timestamps refreshed.
   - `project::save` delegates to `core::project::writer::write_project`.
   - Save success clears dirty flag and status indicator.

## Markdown Handling
- Parsing uses `pulldown-cmark`:
  - Produces plain text string while recording style spans.
- Supports headings, bold, italic, strikethrough, code spans, block quotes, bullet lists.
  - Applies GTK `TextTag`s (`bold`, `italic`, `strikethrough`, `code`, `heading1`, `heading2`, `list-item`).
- Serialization inspects the buffer:
  - Splits text into runs per tag toggle.
- Reconstructs Markdown markers (`**`, `*`, `~~`, `` ` ``) and heading prefixes.
  - For list items (`list-item` tag) prepends `- `.

## Error Handling
- User-facing dialogs for:
  - Invalid project structure (`ChiknError`).
  - Failed save operations.
  - Unsaved changes when switching documents or closing.
- Logs errors to stderr for debugging.

## Future Enhancements
- Metadata side panel for `.meta` editing (labels, status, keywords).
- Git integration (commit history, revision badges).
- Advanced formatting (tables, comments).
- Synchronised preview pane (Pandoc render).

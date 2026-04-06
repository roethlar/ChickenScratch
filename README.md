# ChickenScratch

A cross-platform writing app for fiction writers. Open-source Scrivener alternative with git-native revision control.

**Status:** Pre-alpha (v0.1.0)

## Features

### Editor
- WYSIWYG rich text editor with formatting toolbar (bold, italic, underline, strikethrough, headings, lists, blockquotes, links)
- Find & Replace (Cmd+F / Cmd+H)
- HTML content format — preserves all formatting from Scrivener (underline, superscript, text styles)
- Auto-save with configurable delay
- Live word count
- Focus mode — hide all UI, just write (Cmd+Shift+F)

### Organization
- **Binder** — hierarchical document tree with drag-and-drop reordering
- **Manuscript / Research / Trash** — structured project layout
- Create, rename, delete, move documents and folders
- Context menus and keyboard shortcuts

### Corkboard
- Card view of manuscript documents
- AI-generated scene summaries (Ollama, Anthropic, or OpenAI)
- Group by label, status, or keyword
- Content preview when no synopsis is set

### Manuscript Preview
- Continuous prose view of the entire manuscript
- Project metadata (title, author, type, genre)
- Type-aware formatting (novels show chapter headings, short stories show scene breaks)

### Revisions
- Save Revision — named checkpoints of your work
- Revision History — timeline of all revisions
- Restore — go back to any previous state (non-destructive)
- Draft Versions — try alternate approaches without losing work
- Filesystem backup — automatic push to a backup directory on close

### Import / Export
- **Import Scrivener** (.scriv) — full conversion with formatting, metadata, hierarchy, media files, internal links
- **Import Markdown / Text** — single files or folders
- **Export** to DOCX, PDF, EPUB, HTML, ODT via Pandoc
- Per-document "Include in Compile" toggle

### Settings
- General: theme, Pandoc path
- Writing: font, size, paragraph style, auto-save
- Backup: directory, auto-backup on close, interval
- AI: enable/disable, provider, model, API key
- Compile: format, font, spacing, margins

### Other
- Light, dark, and sepia themes
- Command palette (Cmd+K)
- Project-wide search (Cmd+Shift+P)
- Recent projects list
- Cross-platform: macOS, Linux, Windows

## Install

### macOS
Download `ChickenScratch_x.x.x_aarch64.dmg` from [Releases](../../releases).

### Arch Linux (AUR)
```
yay -S chickenscratch
```

### Build from source
Requires: Rust, Node.js, Pandoc (for import/export)

```bash
git clone https://github.com/yourusername/ChickenScratch.git
cd ChickenScratch
cd ui && npm install && cd ..
cargo tauri build
```

The app bundle will be in `target/release/bundle/`.

### Dependencies
- **Pandoc** — required for Scrivener import and manuscript export
  - macOS: `brew install pandoc`
  - Arch: `pacman -S pandoc`
  - Others: [pandoc.org/installing](https://pandoc.org/installing.html)
- **Ollama** (optional) — for local AI summaries: [ollama.com](https://ollama.com)

## Converter CLI

Standalone command-line converter:

```bash
cargo build --release -p chikn-converter

# Direction auto-detected from extension
./target/release/chikn-converter MyNovel.scriv          # -> MyNovel.chikn
./target/release/chikn-converter MyNovel.chikn          # -> MyNovel.scriv
./target/release/chikn-converter MyNovel.scriv out.chikn # explicit output
```

## .chikn Format

Projects are folders containing:
- `project.yaml` — hierarchy and project metadata
- `manuscript/*.html` — document content
- `manuscript/*.meta` — document metadata (synopsis, labels, status)
- `research/` — reference material
- `.git/` — revision history

Plain text, git-friendly, no vendor lock-in. Edit in any text editor if needed.

## Development

```bash
# Terminal 1: frontend dev server
cd ui && npx vite --port 1420

# Terminal 2: app with hot reload
cargo tauri dev
```

## Keyboard Shortcuts

| Action | Shortcut |
|--------|----------|
| Save | Cmd+S |
| New Document | Cmd+N |
| Find | Cmd+F |
| Find & Replace | Cmd+H |
| Command Palette | Cmd+K |
| Project Search | Cmd+Shift+P |
| Focus Mode | Cmd+Shift+F |
| Toggle Binder | Cmd+\\ |
| Toggle Inspector | Cmd+Shift+I |

## License

MIT

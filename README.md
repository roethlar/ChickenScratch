# ChickenScratch

A cross-platform writing app for fiction writers. Open-source Scrivener alternative with git-native revision control.

**Status:** Pre-alpha (v0.1.0) — functional but rough edges remain.

## What It Does

- **WYSIWYG editor** with formatting toolbar (bold, italic, underline, headings, lists)
- **Binder** — organize your manuscript into documents and folders, drag to reorder
- **Corkboard** — card view of your scenes with AI-generated summaries
- **Manuscript preview** — read your entire manuscript as continuous prose
- **Revisions** — save checkpoints, create draft versions, restore any previous state (powered by embedded git)
- **Scrivener import** — convert .scriv projects with formatting, metadata, and structure preserved
- **Export** — compile to DOCX, PDF, EPUB, HTML, ODT via Pandoc
- **Focus mode** — distraction-free writing, everything hides except your text
- **Themes** — light, dark, sepia
- **Cross-platform** — macOS, Linux, Windows

## Format

Projects use the `.chikn` format — a folder containing HTML documents, YAML metadata, and a git repository. Human-readable, git-friendly, no vendor lock-in.

## Install

### macOS
Download the `.dmg` from [Releases](../../releases), or build from source.

### Arch Linux (AUR)
```
yay -S chickenscratch
```

### Build from source
Requires: Rust, Node.js, Pandoc

```bash
git clone https://github.com/yourusername/ChickenScratch.git
cd ChickenScratch
cd ui && npm install && cd ..
cargo tauri build
```

The app bundle will be in `target/release/bundle/`.

### Dependencies
- **Pandoc** — required for Scrivener import and manuscript export. Install from [pandoc.org](https://pandoc.org/installing.html).
- **Ollama** (optional) — for local AI summaries. Install from [ollama.com](https://ollama.com).

## Converter CLI

Standalone converter for batch operations:

```bash
cargo build --release -p chikn-converter

# Scrivener to ChickenScratch
./target/release/chikn-converter MyNovel.scriv

# ChickenScratch to Scrivener  
./target/release/chikn-converter MyNovel.chikn
```

## Development

```bash
# Terminal 1: frontend dev server
cd ui && npx vite --port 1420

# Terminal 2: app with hot reload
cargo tauri dev
```

## License

MIT

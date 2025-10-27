# 🐔 Chicken Scratch

**Where messy drafts become masterpieces**

A cross-platform, distraction-free word processor for writers with full Scrivener compatibility, git-native workflows, and AI writing assistance.

---

## Overview

**Chicken Scratch** is a feature-complete Scrivener alternative designed to enable macOS writers to seamlessly migrate to Linux (and Windows) without sacrificing functionality. Built with Tauri 2.0, Rust, and React, it combines professional writing tools with modern workflows.

### Key Features

- ✅ **Full Scrivener Compatibility** - Bidirectional import/export with zero data loss
- ✅ **Git-Native Workflows** - Version control with writer-friendly terminology ("Revisions" not "branches")
- ✅ **AI Writing Assistant** - Parallel writing mode with OpenAI, Anthropic, or Ollama
- ✅ **Distraction-Free Modes** - Fullscreen, typewriter scrolling, focus mode, zen mode
- ✅ **Multi-Format Export** - DOCX, PDF, EPUB, Markdown via Pandoc
- ✅ **Cross-Platform** - Windows, macOS, Linux with native performance

### File Format

- **Native Format:** `.chikn` (Pandoc Markdown + YAML metadata)
- **Import:** `.scriv` (Scrivener), `.md` (Markdown), `.rtf` (Rich Text)
- **Export:** `.scriv`, `.docx`, `.pdf`, `.epub`, `.html`, `.md`

---

## Project Status

**Current Phase:** Foundation Setup Complete
**Timeline:** 12-13 months to v1.0 (estimated)
**Progress:** Scaffold complete, ready for Phase 1 implementation

### Completed
- [x] Complete project specification
- [x] Technical architecture design
- [x] Tauri 2.0 + Rust + React scaffold
- [x] Build system configuration
- [x] Development documentation

### Next Steps
- [ ] Implement `.chikn` format reader/writer (Phase 1, Weeks 1-2)
- [ ] Build TipTap editor with Markdown support (Phase 1, Weeks 3-4)
- [ ] Create document navigator with tree view (Phase 1, Weeks 3-4)
- [ ] Add auto-save and basic workflows (Phase 1, Weeks 5-6)

---

## Documentation

- **[Project Specification](docs/PROJECT_SPECIFICATION.md)** - Complete feature overview
- **[Architecture](docs/ARCHITECTURE.md)** - System design and technical details
- **[AI Development Guide](docs/AI_DEVELOPMENT_GUIDE.md)** - Coding patterns for AI
- **[Project Estimates](docs/PROJECT_ESTIMATES.md)** - Timeline and effort breakdown
- **[Phase 1 Design](docs/design/PHASE_1_DESIGN.md)** - Current phase implementation plan
- **[GTK4 Editor Design](docs/GTK4_EDITOR_DESIGN.md)** - Native GTK WYSIWYG architecture
- **[Development Setup](docs/DEVELOPMENT_SETUP.md)** - Environment setup guide
- **[Session Notes](docs/SESSION_NOTES.md)** - Development progress tracking

---

## Quick Start (Development)

### Prerequisites
- Rust (stable channel)
- Node.js (v20+)
- System dependencies (see [DEVELOPMENT_SETUP.md](docs/DEVELOPMENT_SETUP.md))

### Build & Run

```bash
# Install dependencies
npm install

# Run development build
npm run tauri:dev

# Run tests
cargo test --manifest-path=src-tauri/Cargo.toml
npm test
```

### GTK4 WYSIWYG Editor (Rust)

```bash
cargo run --manifest-path gtk-editor/Cargo.toml
```

> **Note:** Building the GTK editor requires system packages that provide GTK 4 development headers (e.g. `libgtk-4-dev`, `gtk4-devel`, or `gtk4` depending on your distro). See the `gtk4` crate documentation for the exact package names per platform.

---

## Technology Stack

**Backend (Rust):**
- Tauri 2.0 framework
- serde (serialization)
- git2-rs (git operations - Phase 4)
- Custom Scrivener parser (Phase 2)

**Frontend (React + TypeScript):**
- React 18
- TipTap (rich text editor)
- Zustand (state management)
- Tailwind CSS (styling)
- Radix UI (accessible components)

**External Tools:**
- Pandoc (document conversion)
- Git (version control)

---

## License

MIT (to be confirmed)

---

## Contributing

This project is 100% AI-developed. See [AI_DEVELOPMENT_GUIDE.md](docs/AI_DEVELOPMENT_GUIDE.md) for development patterns and conventions.

---

## Roadmap

### Phase 1: Foundation (Months 1-2) - **IN PROGRESS**
- Basic editor with Markdown support
- `.chikn` format implementation
- Document navigator
- Project create/open/save

### Phase 2: Scrivener Compatibility (Months 3-4)
- .scriv import/export
- RTF conversion
- Metadata preservation

### Phase 3: Rich Features (Months 5-6)
- Full formatting support
- Custom styles
- Compile/export (DOCX, PDF)

### Phase 4: Git Integration (Months 7-8)
- Version control workflows
- Branch management ("Revisions")
- Remote sync (GitHub, Gitea)

### Phase 5: AI Assistant (Months 9-10)
- Multi-provider LLM integration
- Parallel writing mode
- AI operations (polish, expand, brainstorm)

### Phase 6: Polish & Launch (Months 11-12)
- Distraction-free modes
- Theme system
- Accessibility (WCAG AA)
- Cross-platform testing
- v1.0 release

---

**Built with ❤️ (and AI) for writers everywhere**

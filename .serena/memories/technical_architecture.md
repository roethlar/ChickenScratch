# Chicken Scratch - Technical Architecture

## Project Overview
- **Name**: Chicken Scratch
- **File Extension**: .chikn
- **Tagline**: "Where messy drafts become masterpieces"
- **Purpose**: Feature-complete cross-platform Scrivener alternative for writer migration from macOS to Linux/Windows

## Technology Stack

### Core Technologies
- **Framework**: Tauri 2.0 (Rust backend + React/TypeScript frontend)
- **Backend**: Rust 2021 edition
- **Frontend**: React 18+ with TypeScript, Vite bundler
- **Styling**: Tailwind CSS + CSS variables for theming
- **State Management**: Zustand
- **Rich Text Editor**: TipTap (ProseMirror-based) for Markdown + formatting
- **Format Converter**: Pandoc (external dependency)
- **Git Operations**: git2-rs library

### File Format: .chikn

#### Structure
```
MyNovel.chikn/
├── project.yaml              # Project metadata, settings, hierarchy
├── manuscript/
│   ├── ch01-opening.md      # Pandoc Markdown content
│   ├── ch01-opening.meta    # YAML: formatting, Scrivener metadata
│   ├── ch02-conflict.md
│   └── ch02-conflict.meta
├── research/                 # Research folder
├── templates/                # Character/setting templates
├── settings/                 # Compile settings, themes
└── .git/                     # Optional git integration
```

#### Format Philosophy
- **Git-friendly**: Plain text Markdown for content, YAML for metadata
- **Lossless round-trip**: Full Scrivener compatibility via metadata preservation
- **Human-readable**: Writers can edit .md files directly if needed
- **Pandoc Markdown**: Extended syntax for rich formatting via custom styles

#### Metadata Structure (.meta files)
```yaml
custom_styles:
  - name: "Emphasis Red"
    color: "#ff0000"
    font: "Georgia"
    size: 14
    bold: true

scrivener_metadata:
  label: "Scene"
  status: "First Draft"
  keywords: ["opening", "protagonist"]
  synopsis: "Character discovers..."
  
document_metadata:
  word_count: 1247
  target: 2000
  created: "2025-01-15"
  modified: "2025-01-20"
```

## Architecture Components

### Rust Backend Modules

#### 1. Core Module (`src-tauri/src/core/`)
- **project.rs**: Project management (create, load, save)
- **document.rs**: Document operations (read, write, tree manipulation)

#### 2. Models Module (`src-tauri/src/models/`)
- **Project**: Root project structure
- **Document**: Individual document with content + metadata
- **TreeNode**: Hierarchical document organization

#### 3. API Module (`src-tauri/src/api/`)
- **project_commands.rs**: Tauri commands for project operations
- **document_commands.rs**: Tauri commands for document operations

#### 4. Utils Module (`src-tauri/src/utils/`)
- **error.rs**: Custom error types and handling
- **file.rs**: File system utilities
- **yaml_parser.rs**: YAML parsing/serialization

### React Frontend Components

#### 1. Layout Components (`src/components/layout/`)
- **MainLayout**: App shell with navigation
- **Titlebar**: Custom window controls (optional)
- **Statusbar**: Word count, stats display

#### 2. Editor Components (`src/components/editor/`)
- **Editor**: TipTap editor wrapper with formatting toolbar
- **FormattingToolbar**: Rich text controls
- **FocusMode**: Distraction-free writing overlay

#### 3. Navigator Components (`src/components/navigator/`)
- **DocumentTree**: Hierarchical document browser
- **TreeNode**: Individual tree item (drag-drop support)

#### 4. State Management (`src/store/`)
- **useProjectStore**: Zustand store for project state
- **useEditorStore**: Editor-specific state
- **useSettingsStore**: User preferences

#### 5. Hooks (`src/hooks/`)
- **useProject**: Project lifecycle management
- **useDocument**: Document operations
- **useAutoSave**: Auto-save with debouncing

## Key Features

### Phase 1: Foundation (Months 1-2)
- Create/load/save .chikn projects
- Document tree navigation
- Basic Markdown editing with TipTap
- Auto-save functionality

### Phase 2: Scrivener Compatibility (Months 3-4)
- Import .scriv → .chikn converter
- Export .chikn → .scriv with metadata preservation
- RTF parsing and conversion
- Metadata handling (labels, status, keywords, synopsis)

### Phase 3: Rich Features (Months 5-6)
- Full RTF formatting in editor
- Compile/export to multiple formats (DOCX, PDF via Pandoc)
- Character/setting templates
- Research folder management

### Phase 4: Git Integration (Months 7-8)
- Local git repository initialization
- Auto-commit with configurable frequency
- Branch management ("Revisions" UI for writers)
- Remote sync (GitHub/Gitea)

### Phase 5: AI Assistant (Months 9-10)
- **Parallel Writing Mode**: Side-by-side (author draft | AI suggestions)
- Multi-provider support:
  - OpenAI (GPT-4 Turbo)
  - Anthropic (Claude 3.5 Sonnet)
  - Ollama (local models: Llama 3.1, Mistral)
  - Custom endpoints
- AI features:
  - Polish & refine
  - Continue writing
  - Brainstorming chat
  - Character voice consistency check
  - Pacing analysis
- Slide-out AI panel (configurable side: left/right/top/bottom)

### Phase 6: Polish & UX (Months 11-12)
- Distraction-free modes:
  - Fullscreen fade
  - Typewriter scrolling
  - Focus mode (dim surrounding text)
  - Zen mode (centered column)
  - Customizable width/background
- Theme system with live preview
- Word count goals & statistics
- Pomodoro timer integration
- Accessibility (keyboard shortcuts, screen readers)
- Cross-platform testing & bug fixes

## Development Philosophy

### AI-Friendly Design
- **Modular architecture**: Small, focused files (<500 lines)
- **Clear separation of concerns**: Rust backend, React frontend
- **Comprehensive documentation**: Inline comments, design docs
- **Consistent patterns**: Established conventions for commands, hooks, components
- **Type safety**: Rust + TypeScript for compile-time error detection

### Testing Strategy
- **Unit tests**: Rust (cargo test), TypeScript (Vitest)
- **Integration tests**: Tauri command testing
- **E2E tests**: Playwright for full user workflows
- **Coverage target**: 70%+ critical paths

### Performance Targets
- **Cold start**: <3 seconds on modern hardware
- **Document switching**: <200ms
- **Auto-save debounce**: 2 seconds after typing stops
- **Memory usage**: <200MB for typical project (100 documents)

## Timeline Estimate
- **Total Duration**: 12-15 months to v1.0
- **Confidence**: 77% for 13-month realistic scenario
- **Complexity Score**: 7.1/10 (High due to Scrivener compatibility)

# Chicken Scratch - Architecture Document

**Version:** 1.0
**Date:** 2025-10-01

---

## 1. Architecture Overview

### 1.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   TAURI APPLICATION                      │
│                                                          │
│  ┌────────────────────────────────────────────────┐    │
│  │          FRONTEND (React + TypeScript)         │    │
│  │                                                 │    │
│  │  ┌──────────┐  ┌──────────┐  ┌─────────────┐  │    │
│  │  │          │  │          │  │             │  │    │
│  │  │ Editor   │  │ Navigator│  │ AI Assistant│  │    │
│  │  │ (TipTap) │  │          │  │             │  │    │
│  │  │          │  │          │  │             │  │    │
│  │  └──────────┘  └──────────┘  └─────────────┘  │    │
│  │                                                 │    │
│  │  ┌──────────────────────────────────────────┐  │    │
│  │  │        State Management (Zustand)        │  │    │
│  │  └──────────────────────────────────────────┘  │    │
│  │                                                 │    │
│  │  ┌──────────────────────────────────────────┐  │    │
│  │  │         Tauri IPC Commands               │  │    │
│  │  └──────────────────────────────────────────┘  │    │
│  └─────────────────────────────────────────────────┘    │
│                          │                              │
│                          ▼                              │
│  ┌─────────────────────────────────────────────────┐    │
│  │           BACKEND (Rust)                        │    │
│  │                                                  │    │
│  │  ┌─────────┐  ┌──────────┐  ┌──────────────┐   │    │
│  │  │ Project │  │ Scrivener│  │ Git          │   │    │
│  │  │ Manager │  │ Compat   │  │ Integration  │   │    │
│  │  │(.chikn) │  │ (.scriv) │  │              │   │    │
│  │  └─────────┘  └──────────┘  └──────────────┘   │    │
│  │                                                  │    │
│  │  ┌─────────┐  ┌──────────┐  ┌──────────────┐   │    │
│  │  │ Format  │  │ AI       │  │ File         │   │    │
│  │  │ Convert │  │ Provider │  │ Watcher      │   │    │
│  │  │(Pandoc) │  │ Client   │  │              │   │    │
│  │  └─────────┘  └──────────┘  └──────────────┘   │    │
│  └──────────────────────────────────────────────────┘    │
│                                                          │
└──────────────────────────────────────────────────────────┘
                          │
                          ▼
          ┌───────────────────────────────┐
          │   EXTERNAL SERVICES           │
          │                               │
          │  - Pandoc (format conversion) │
          │  - Git (version control)      │
          │  - OpenAI API                 │
          │  - Anthropic API              │
          │  - Ollama (local LLMs)        │
          └───────────────────────────────┘
```

### 1.2 Technology Stack

**Frontend:**
- **Framework:** React 18+ with TypeScript
- **State Management:** Zustand (lightweight, AI-friendly)
- **Text Editor:** TipTap (ProseMirror-based)
- **UI Components:** Radix UI (accessible, unstyled primitives)
- **Styling:** Tailwind CSS (utility-first, customizable themes)
- **Icons:** Lucide React (consistent, minimal)

**Backend:**
- **Framework:** Tauri 2.0 (Rust-based)
- **Core Language:** Rust (stable channel)
- **Git Library:** git2-rs (libgit2 bindings)
- **RTF Parser:** Custom implementation with xml-rs for .scrivx
- **Async Runtime:** Tokio (async file operations, network requests)

**External Tools:**
- **Pandoc:** Universal document converter (shell invocation)
- **Git:** Native git binary for advanced operations

---

## 2. Rust Backend Architecture

### 2.1 Module Structure

```
src-tauri/
├── main.rs                 # Tauri app entry point
├── lib.rs                  # Library exports
│
├── core/                   # Core business logic
│   ├── mod.rs
│   ├── project/            # .chikn project management
│   │   ├── mod.rs
│   │   ├── format.rs       # .chikn format spec
│   │   ├── reader.rs       # Read .chikn projects
│   │   ├── writer.rs       # Write .chikn projects
│   │   └── validator.rs    # Validate project integrity
│   │
│   ├── scrivener/          # .scriv compatibility
│   │   ├── mod.rs
│   │   ├── importer.rs     # .scriv → .chikn
│   │   ├── exporter.rs     # .chikn → .scriv
│   │   ├── scrivx_parser.rs # Parse .scrivx XML
│   │   └── rtf_handler.rs  # RTF read/write
│   │
│   ├── git/                # Git operations
│   │   ├── mod.rs
│   │   ├── repository.rs   # Init, commit, branch
│   │   ├── remote.rs       # Push, pull, fetch
│   │   ├── diff.rs         # Diff visualization
│   │   └── merge.rs        # Conflict resolution
│   │
│   └── formats/            # Format conversions
│       ├── mod.rs
│       ├── markdown.rs     # Pandoc Markdown handling
│       ├── pandoc.rs       # Pandoc CLI wrapper
│       └── rtf.rs          # RTF utilities
│
├── models/                 # Data models
│   ├── mod.rs
│   ├── project.rs          # Project structure
│   ├── document.rs         # Document model
│   ├── metadata.rs         # Metadata structures
│   └── settings.rs         # User settings
│
├── api/                    # Tauri commands (frontend ↔ backend)
│   ├── mod.rs
│   ├── project_commands.rs # Project CRUD operations
│   ├── document_commands.rs# Document operations
│   ├── git_commands.rs     # Git commands
│   ├── ai_commands.rs      # AI provider integration
│   └── settings_commands.rs# Settings management
│
├── services/               # External integrations
│   ├── mod.rs
│   ├── ai/                 # AI provider clients
│   │   ├── mod.rs
│   │   ├── openai.rs       # OpenAI API client
│   │   ├── anthropic.rs    # Anthropic API client
│   │   ├── ollama.rs       # Ollama client
│   │   └── provider.rs     # Trait for AI providers
│   │
│   └── file_watcher.rs     # File system monitoring
│
└── utils/                  # Utilities
    ├── mod.rs
    ├── error.rs            # Error types
    ├── config.rs           # App configuration
    └── logger.rs           # Logging setup
```

### 2.2 Key Rust Traits & Abstractions

**Project Trait:**
```rust
pub trait ProjectFormat {
    fn read(path: &Path) -> Result<Project, ProjectError>;
    fn write(project: &Project, path: &Path) -> Result<(), ProjectError>;
    fn validate(path: &Path) -> Result<bool, ProjectError>;
}

impl ProjectFormat for ChiknFormat { /* .chikn implementation */ }
impl ProjectFormat for ScrivenerFormat { /* .scriv implementation */ }
```

**AI Provider Trait:**
```rust
#[async_trait]
pub trait AIProvider {
    async fn complete(&self, prompt: &str, context: &str) -> Result<String, AIError>;
    async fn stream_complete(&self, prompt: &str, context: &str) -> Result<Stream<String>, AIError>;
    fn provider_name(&self) -> &str;
    fn estimate_cost(&self, tokens: usize) -> f64;
}

impl AIProvider for OpenAIClient { /* ... */ }
impl AIProvider for AnthropicClient { /* ... */ }
impl AIProvider for OllamaClient { /* ... */ }
```

### 2.3 Error Handling Strategy

**Centralized Error Types:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum ChickenScratchError {
    #[error("Project error: {0}")]
    Project(#[from] ProjectError),

    #[error("Scrivener import error: {0}")]
    ScrivenerImport(String),

    #[error("Git operation failed: {0}")]
    Git(#[from] git2::Error),

    #[error("AI provider error: {0}")]
    AI(#[from] AIError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// Convert to frontend-friendly JSON errors
impl From<ChickenScratchError> for tauri::Error {
    fn from(err: ChickenScratchError) -> Self {
        tauri::Error::Anyhow(anyhow::Error::from(err))
    }
}
```

---

## 3. React Frontend Architecture

### 3.1 Component Structure

```
src/
├── App.tsx                 # Root component
├── main.tsx                # Entry point
│
├── components/             # UI components
│   ├── layout/
│   │   ├── AppLayout.tsx   # Main app layout
│   │   ├── Sidebar.tsx     # Navigator sidebar
│   │   ├── Inspector.tsx   # Metadata inspector
│   │   └── StatusBar.tsx   # Bottom status bar
│   │
│   ├── editor/
│   │   ├── Editor.tsx      # Main editor component
│   │   ├── EditorToolbar.tsx # Formatting toolbar
│   │   ├── EditorStyles.tsx  # Style management
│   │   └── FocusModes.tsx    # Distraction-free overlays
│   │
│   ├── navigator/
│   │   ├── Navigator.tsx     # Document tree
│   │   ├── DocumentTree.tsx  # Tree component
│   │   ├── TreeItem.tsx      # Individual tree node
│   │   └── SearchBar.tsx     # Filter/search documents
│   │
│   ├── ai/
│   │   ├── AIPanel.tsx       # AI assistant panel
│   │   ├── ParallelView.tsx  # Side-by-side editor
│   │   ├── SuggestionCard.tsx# AI suggestion display
│   │   └── PromptLibrary.tsx # Prompt templates
│   │
│   ├── git/
│   │   ├── GitPanel.tsx      # Git UI
│   │   ├── CommitDialog.tsx  # Commit message UI
│   │   ├── BranchSelector.tsx# Branch/revision picker
│   │   └── ConflictResolver.tsx # Merge conflict UI
│   │
│   └── dialogs/
│       ├── SettingsDialog.tsx # App settings
│       ├── ImportDialog.tsx   # Import wizard
│       ├── ExportDialog.tsx   # Export options
│       └── ThemeEditor.tsx    # Theme customization
│
├── hooks/                  # Custom React hooks
│   ├── useProject.ts       # Project state
│   ├── useDocument.ts      # Current document
│   ├── useGit.ts           # Git operations
│   ├── useAI.ts            # AI provider integration
│   ├── useFocusMode.ts     # Focus mode state
│   └── useKeyboardShortcuts.ts # Keyboard bindings
│
├── state/                  # Zustand stores
│   ├── projectStore.ts     # Project & document state
│   ├── editorStore.ts      # Editor state (cursor, selection)
│   ├── uiStore.ts          # UI state (panels, modals)
│   ├── gitStore.ts         # Git state
│   └── settingsStore.ts    # User preferences
│
├── services/               # Frontend services
│   ├── tauri.ts            # Tauri command wrappers
│   ├── storage.ts          # Local storage helpers
│   └── shortcuts.ts        # Keyboard shortcut manager
│
├── types/                  # TypeScript types
│   ├── project.ts          # Project types
│   ├── document.ts         # Document types
│   ├── git.ts              # Git types
│   └── ai.ts               # AI types
│
└── utils/                  # Frontend utilities
    ├── markdown.ts         # Markdown helpers
    ├── formatting.ts       # Text formatting utils
    └── validation.ts       # Input validation
```

### 3.2 State Management (Zustand)

**Project Store:**
```typescript
interface ProjectState {
  currentProject: Project | null;
  documents: Map<string, Document>;
  activeDocumentId: string | null;

  // Actions
  loadProject: (path: string) => Promise<void>;
  saveProject: () => Promise<void>;
  createDocument: (parentId: string, name: string) => Promise<void>;
  updateDocument: (id: string, content: string) => Promise<void>;
  deleteDocument: (id: string) => Promise<void>;
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  currentProject: null,
  documents: new Map(),
  activeDocumentId: null,

  loadProject: async (path: string) => {
    const project = await invoke<Project>('load_project', { path });
    set({ currentProject: project });
  },

  // ... other actions
}));
```

**Editor Store:**
```typescript
interface EditorState {
  cursorPosition: number;
  selection: { from: number; to: number } | null;
  focusMode: FocusMode | null;
  wordCount: number;

  // Actions
  setCursorPosition: (pos: number) => void;
  setSelection: (from: number, to: number) => void;
  toggleFocusMode: (mode: FocusMode) => void;
  updateWordCount: (count: number) => void;
}
```

### 3.3 Tauri IPC Communication

**Frontend → Backend Commands:**
```typescript
// Project operations
await invoke('load_project', { path: '/path/to/project.chikn' });
await invoke('save_project', { project });
await invoke('import_scrivener', { scrivPath: '/path/to/project.scriv' });
await invoke('export_scrivener', { outputPath: '/path/to/output.scriv' });

// Document operations
await invoke('create_document', { parentId: 'uuid', name: 'Chapter 1' });
await invoke('update_document', { id: 'uuid', content: '...' });
await invoke('delete_document', { id: 'uuid' });

// Git operations
await invoke('git_init', { projectPath: '...' });
await invoke('git_commit', { message: 'Auto-save', files: ['...'] });
await invoke('git_push', { remote: 'origin', branch: 'main' });

// AI operations
const result = await invoke('ai_complete', {
  provider: 'openai',
  prompt: 'Improve this text: ...',
  context: '...'
});
```

**Backend → Frontend Events:**
```typescript
// Listen for file changes
listen('file-changed', (event) => {
  const { documentId, content } = event.payload;
  updateDocument(documentId, content);
});

// Listen for git status changes
listen('git-status-changed', (event) => {
  const { status } = event.payload;
  updateGitStatus(status);
});
```

---

## 4. Data Flow Architecture

### 4.1 Document Editing Flow

```
User types in Editor
        │
        ▼
TipTap onChange event
        │
        ▼
Editor Store (local state)
        │
        ▼
Debounced save (500ms)
        │
        ▼
Tauri IPC: update_document
        │
        ▼
Rust Backend: write to .chikn
        │
        ▼
File Watcher detects change
        │
        ▼
Emit file-changed event
        │
        ▼
Frontend updates UI (if needed)
```

### 4.2 Scrivener Import Flow

```
User selects .scriv folder
        │
        ▼
Frontend: invoke('import_scrivener')
        │
        ▼
Backend: Parse .scrivx XML
        │
        ▼
Extract document hierarchy
        │
        ▼
For each RTF file:
  - Read RTF content
  - Convert to Pandoc Markdown
  - Extract custom styles
  - Save to .meta YAML
        │
        ▼
Create .chikn project structure
        │
        ▼
Return project path to frontend
        │
        ▼
Frontend: load_project
```

### 4.3 AI Assistance Flow

```
User highlights text
        │
        ▼
User clicks "Polish" button
        │
        ▼
Frontend: gather context
  - Selected text
  - Document context (optional)
  - Character sheets (optional)
        │
        ▼
Tauri IPC: ai_complete
        │
        ▼
Backend: select AI provider
  - Check settings
  - Get API key or Ollama URL
        │
        ▼
Call AI provider API
        │
        ▼
Stream response back to frontend
        │
        ▼
Display in Parallel View
  - Left: original text
  - Right: AI suggestion
        │
        ▼
User accepts/rejects/merges
```

---

## 5. File Format Specifications

### 5.1 .chikn Project Structure

**Directory Layout:**
```
MyNovel.chikn/
├── project.yaml              # Root project metadata
├── manuscript/               # Main content
│   ├── chapter-01.md
│   ├── chapter-01.meta       # Metadata & styles for chapter-01
│   ├── chapter-02.md
│   └── chapter-02.meta
├── research/                 # Reference materials
│   ├── character-alice.md
│   └── setting-dystopia.md
├── templates/                # Document templates
│   ├── character.template
│   └── scene.template
├── settings/                 # Project settings
│   ├── compile.yaml          # Compile/export settings
│   └── theme.yaml            # Custom theme
└── .git/                     # Git repository (optional)
```

**project.yaml Schema:**
```yaml
version: "1.0"
name: "My Novel"
author: "Jane Doe"
created: "2025-01-15T10:30:00Z"
modified: "2025-01-20T14:22:00Z"

hierarchy:
  - id: "uuid-1"
    type: "folder"
    name: "Manuscript"
    children:
      - id: "uuid-2"
        type: "document"
        name: "Chapter 01 - Opening"
        path: "manuscript/chapter-01.md"
      - id: "uuid-3"
        type: "document"
        name: "Chapter 02 - Conflict"
        path: "manuscript/chapter-02.md"

  - id: "uuid-4"
    type: "folder"
    name: "Research"
    children:
      - id: "uuid-5"
        type: "document"
        name: "Alice (Protagonist)"
        path: "research/character-alice.md"

labels:
  - id: 1
    name: "Scene"
    color: "#ff6b6b"
  - id: 2
    name: "Chapter"
    color: "#4ecdc4"

status_items:
  - id: 1
    name: "To Do"
  - id: 2
    name: "First Draft"
  - id: 3
    name: "Revised"
  - id: 4
    name: "Final"

settings:
  default_label: 1
  default_status: 1
  word_count_target: 80000
```

**Document Metadata (.meta) Schema:**
```yaml
# chapter-01.meta
document_id: "uuid-2"

scrivener_metadata:
  label_id: 1              # "Scene"
  status_id: 2             # "First Draft"
  keywords: ["opening", "protagonist", "inciting incident"]
  synopsis: "Alice wakes up in a dystopian world..."

custom_styles:
  - name: "Emphasis"
    color: "#ff0000"
    font: "Georgia"
    size: 14
    bold: true
  - name: "Internal Thought"
    color: "#666666"
    italic: true

statistics:
  word_count: 1247
  character_count: 7830
  target: 2000

timestamps:
  created: "2025-01-15T11:00:00Z"
  modified: "2025-01-20T14:22:00Z"
```

### 5.2 Pandoc Markdown Extensions

**Custom Style Syntax:**
```markdown
Alice felt [a deep unease]{custom-style="Internal Thought"}
as she walked through [the gray streets]{custom-style="Emphasis"}.
```

**Metadata in Markdown (Optional YAML Frontmatter):**
```markdown
---
title: "Chapter 01 - Opening"
author: "Jane Doe"
date: 2025-01-15
---

# Chapter 01

Alice woke up to the sound of sirens...
```

---

## 6. Security & Privacy

### 6.1 Data Protection

**Local-First Philosophy:**
- All projects stored locally by default
- No telemetry or data collection without explicit opt-in
- Git sync is optional, user-controlled

**AI Privacy:**
- Clear warnings when cloud AI providers are used
- Option to use Ollama (local LLMs) for complete privacy
- Per-project AI opt-in (prevent accidental data leakage)
- Never send project data without user action

### 6.2 API Key Management

**Secure Storage:**
- API keys stored in system keychain/credential manager
- Tauri's `tauri-plugin-secure-storage` for encrypted storage
- Never log or expose API keys in error messages

**Access Control:**
- Backend handles all API calls (keys never exposed to frontend JS)
- Rate limiting on AI calls to prevent accidental token burn
- Optional budget limits per month

---

## 7. Performance Optimization

### 7.1 Large Project Handling

**Virtual Scrolling:**
- Document navigator uses virtual scrolling (react-window) for 500+ docs
- Only render visible documents in tree view

**Lazy Loading:**
- Load document content on-demand (not all at once)
- Cache recently accessed documents in memory (LRU cache)

**Background Processing:**
- Format conversions (RTF ↔ Markdown) run in Rust async tasks
- File watching uses efficient debouncing (avoid excessive saves)

### 7.2 Editor Performance

**TipTap Optimizations:**
- Debounced state updates (500ms) to reduce re-renders
- Memoized document content to prevent unnecessary parsing
- Syntax highlighting limited to viewport (don't highlight 10k lines)

**Rendering:**
- Use `React.memo` for expensive components
- Virtualize long documents (lazy-render paragraphs out of viewport)

---

## 8. Testing Strategy

### 8.1 Backend Tests (Rust)

**Unit Tests:**
- Core logic (project read/write, format conversions)
- Scrivener import/export with sample projects
- Git operations (mock git2::Repository)

**Integration Tests:**
- Full .chikn ↔ .scriv round-trip validation
- Pandoc integration tests (verify output formats)

**Test Coverage:**
```bash
cargo tarpaulin --out Html --output-dir coverage/
# Target: 80%+ coverage for core modules
```

### 8.2 Frontend Tests (TypeScript)

**Component Tests (Vitest + Testing Library):**
- Editor component (text input, formatting, shortcuts)
- Navigator (tree operations, drag-drop)
- AI panel (suggestion display, accept/reject)

**E2E Tests (Playwright):**
- Project creation and loading
- Scrivener import flow
- Git commit workflow
- AI assistance workflow

### 8.3 Compatibility Tests

**Scrivener Round-Trip:**
- Automated test suite with diverse .scriv projects
- Validate: import → export → import (content identical)
- Test edge cases: custom metadata, complex RTF formatting

---

## 9. Deployment & Distribution

### 9.1 Build Process

**Tauri Build:**
```bash
# Development build
npm run tauri dev

# Production build (all platforms)
npm run tauri build
```

**Platform-Specific Outputs:**
- **Linux:** AppImage, .deb, .rpm
- **macOS:** .dmg, .app bundle (signed & notarized)
- **Windows:** .exe installer (NSIS), .msi

### 9.2 CI/CD Pipeline

**GitHub Actions Workflow:**
1. Run tests (Rust + TypeScript)
2. Build for all platforms (matrix build)
3. Code signing (macOS, Windows)
4. Upload artifacts to GitHub Releases
5. Publish to distribution channels (Flathub, Homebrew, Winget)

---

## 10. Future Extensibility

### 10.1 Plugin System (Post-1.0)

**Architecture:**
- WebAssembly plugins for custom format converters
- JavaScript/TypeScript plugins for editor extensions
- Lua scripting for user automation

**Plugin API:**
- Editor API (insert text, apply styles)
- Project API (access documents, metadata)
- UI API (add menu items, panels)

### 10.2 Cloud Sync (Post-1.0)

**Architecture:**
- Backend sync service (optional hosted version)
- Conflict-free replicated data type (CRDT) for real-time collaboration
- End-to-end encryption for cloud-stored projects

---

## Conclusion

This architecture is designed for:
1. **AI Development Efficiency:** Modular, well-documented, type-safe codebase
2. **Data Integrity:** Lossless format conversions, comprehensive testing
3. **User Experience:** Performant, accessible, distraction-free interface
4. **Extensibility:** Plugin support, cloud sync, future feature additions

The separation between Rust backend (data reliability) and React frontend (UX polish) allows AI to focus on discrete, well-scoped modules while maintaining system coherence.

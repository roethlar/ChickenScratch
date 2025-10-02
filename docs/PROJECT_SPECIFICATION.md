# Chicken Scratch - Project Specification

**Version:** 1.0
**Date:** 2025-10-01
**Status:** Specification Phase

---

## Executive Summary

**Chicken Scratch** is a cross-platform, distraction-free word processor designed to enable writers to transition from macOS to Linux while maintaining full compatibility with Scrivener projects. The application prioritizes feature completeness over minimalism, offering professional writing tools with modern workflows including git integration and AI-powered writing assistance.

**Core Mission:** Enable macOS writers to seamlessly migrate to Linux without sacrificing Scrivener's powerful project management and organizational capabilities.

---

## 1. Product Overview

### 1.1 Name & Branding

- **Application Name:** Chicken Scratch
- **File Extension:** `.chikn`
- **Tagline:** "Where messy drafts become masterpieces"
- **Brand Personality:** Unpretentious, writer-focused, self-aware humor, professional quality

### 1.2 Target Audience

- Writers using Scrivener on macOS who need/want Linux alternatives
- Cross-platform writers needing consistent tools across operating systems
- Writers seeking git-native workflows for version control and collaboration
- Authors, novelists, screenwriters, academics, journalists, poets, non-fiction writers
- Technical proficiency: Beginner to advanced (UI must accommodate tech-phobic users)

### 1.3 Key Differentiators

1. **Full Scrivener Compatibility:** Bidirectional import/export with zero data loss
2. **Git-Native Workflows:** First-class version control with writer-friendly terminology
3. **AI Writing Assistant:** Parallel writing mode with multiple LLM providers (OpenAI, Anthropic, Ollama)
4. **Format Flexibility:** Native git-friendly format with universal import/export (Scrivener, Markdown, RTF, DOCX, PDF, EPUB)
5. **Cross-Platform Excellence:** True native experience on Windows, macOS, Linux
6. **Distraction-Free Mastery:** Multiple focus modes with full customization

---

## 2. Technical Architecture

### 2.1 Technology Stack

**Framework:** Tauri 2.0
- **Backend:** Rust (core logic, file operations, git integration)
- **Frontend:** React + TypeScript (UI, editor, visual components)
- **Rationale:**
  - AI-friendly development (massive React/TypeScript training data)
  - Native performance (Rust backend)
  - Cross-platform with single codebase
  - Modern, polished UI capabilities
  - Rust backend provides "cool factor" while React ensures velocity

**Key Dependencies:**
- **Text Editor:** TipTap (ProseMirror-based, extensible, Markdown + RTF support)
- **Format Conversion:** Pandoc (universal document conversion)
- **Git Operations:** git2-rs (Rust git library)
- **RTF Handling:** Custom RTF parser/writer with Pandoc integration
- **LLM Integration:** OpenAI SDK, Anthropic SDK, Ollama API client

### 2.2 Native File Format: `.chikn`

**Structure:**
```
MyNovel.chikn/
├── project.yaml              # Project metadata, settings, hierarchy
├── manuscript/
│   ├── ch01-opening.md      # Pandoc Markdown content
│   ├── ch01-opening.meta    # YAML: formatting, Scrivener metadata
│   ├── ch02-conflict.md
│   └── ch02-conflict.meta
├── research/                # Research folder
│   └── notes.md
├── templates/               # Character/setting templates
│   ├── character.template
│   └── setting.template
├── settings/                # Compile settings, themes, preferences
│   └── compile.yaml
└── .git/                    # Git integration (optional)
```

**Format Design Principles:**
- **Git-Friendly:** Plain text (Markdown + YAML) for perfect diff/merge
- **Lossless Scrivener Round-Trip:** Preserve all metadata for perfect .scriv export
- **Human-Readable:** Writers can edit .md files in any text editor
- **AI-Friendly:** Simple, predictable structure for LLM editing
- **Extensible:** Add features without breaking old projects

**Content Storage:**
- **Text:** Pandoc extended Markdown (GFM + custom extensions)
- **Formatting:** Custom styles defined in `.meta` YAML files
- **Rich Formatting:** Pandoc attributes: `[text]{custom-style="Emphasis Red"}`

**Metadata Schema (per document):**
```yaml
# ch01-opening.meta
custom_styles:
  - name: "Emphasis Red"
    color: "#ff0000"
    font: "Georgia"
    size: 14
    bold: true
  - name: "Thought Italic"
    color: "#666666"
    italic: true

scrivener_metadata:
  label: "Scene"
  status: "First Draft"
  keywords: ["opening", "protagonist", "inciting incident"]
  synopsis: "Character wakes up, discovers the truth..."

document_metadata:
  word_count: 1247
  target: 2000
  created: "2025-01-15T10:30:00Z"
  modified: "2025-01-20T14:22:00Z"
```

### 2.3 Scrivener Compatibility

**Import Strategy:**
1. Parse `.scrivx` XML for project structure and metadata
2. Read RTF files from `Files/Data/{UUID}/content.rtf`
3. Convert RTF → Pandoc Markdown with style preservation
4. Extract all Scrivener metadata (labels, status, keywords, synopsis, custom fields)
5. Create `.chikn` project with full fidelity

**Export Strategy:**
1. Convert Pandoc Markdown → RTF with custom styles
2. Generate `.scrivx` XML with proper hierarchy and UUIDs
3. Create `Files/Data/{UUID}/content.rtf` structure
4. Preserve all Scrivener metadata exactly
5. Result: Scrivener can open exported `.scriv` with zero data loss

**Scrivener Version Support:**
- Primary: Scrivener 3.x (macOS, Windows)
- Secondary: Auto-upgrade Scrivener 1.x projects to 3.x format

### 2.4 Multi-Format Import/Export

**Import Formats:**
- `.scriv` (Scrivener projects) → Native `.chikn`
- `.md` (Markdown files) → Hierarchy via H1=Chapter, H2=Scene or YAML frontmatter
- `.rtf` (Rich Text) → Single document import or project creation
- `.docx` (Word) via Pandoc → Document import
- `.txt` (Plain text) → Basic import

**Export Formats:**
- `.scriv` (Scrivener) - Full bidirectional compatibility
- `.md` (Markdown) - Plain, GFM, or Pandoc flavors
- `.rtf` (Rich Text) - Preserve formatting
- `.docx` (Word) via Pandoc - Standard manuscript format
- `.pdf` via Pandoc - Final output
- `.html` - Web publishing
- `.epub` - eBook format for novels
- `.fountain` - Screenplay format (future consideration)

---

## 3. Core Features

### 3.1 Essential Features (Version 1.0)

#### Document Management
- **Hierarchical Organization:** Nested folders and documents (Manuscript, Research, Characters, Places, Notes)
- **Drag-Drop Reordering:** Intuitive scene/chapter reorganization
- **Document Navigator:** Tree view with expand/collapse, search/filter
- **Multiple Documents:** Switch between open documents with tabs or quick switcher
- **Document Templates:** Character sketches, setting descriptions, chapter templates

#### Text Editing
- **Rich Text Formatting:** Bold, italic, underline, strikethrough, fonts, colors, sizes
- **Custom Styles:** User-defined named styles (saved to `.meta` files)
- **Markdown Support:** Write in Markdown, toggle live preview
- **Word Count:** Real-time count, targets per document and project-wide
- **Find & Replace:** Project-wide or current document, regex support

#### Scrivener Feature Parity
- **Labels:** Color-coded document labels (Scene, Chapter, etc.)
- **Status:** Document status tracking (To Do, First Draft, Revised, Final, Done)
- **Keywords:** Taggable keywords for filtering and organization
- **Synopsis:** Per-document synopsis/summary field
- **Research Folder:** Import PDFs, images, web pages, reference documents
- **Compile/Export:** Export full manuscript with formatting to .scriv, .docx, .pdf

#### Metadata Management
- **Custom Metadata Fields:** User-defined fields (e.g., POV, Setting, Timeline)
- **Metadata Display:** Sidebar or inspector view for quick editing
- **Bulk Operations:** Apply labels/status/keywords to multiple documents

#### Version Control (Git Integration)
- **Local-First:** All work saves locally by default
- **Git Repository:** Optional git initialization for version control
- **Simple UI:** Writer-friendly terminology ("Revisions" instead of "branches")
- **Auto-Commit:** Configurable auto-save commits
- **Manual Commits:** Git commit UI with message input
- **Remote Sync:** Push/pull to GitHub, Gitea, or generic git remotes
- **Branch Management:** Create/switch "Revisions" (branches) for alternate drafts
- **Conflict Resolution:** Basic merge conflict UI for RTF/Markdown files

### 3.2 AI Writing Assistant

**Core Capabilities:**
- **Parallel Writing Mode:** Side-by-side panes (user draft left, AI suggestions right)
- **AI Panel Position:** Slide out from any edge (top, right, bottom, left - user preference)
- **Accept/Reject/Merge:** Clear controls for AI suggestions
- **Context Awareness:** AI can access character sheets, world notes, previous chapters (user-controlled)

**AI Operations:**
- **Polish & Refine:** Improve prose quality, flow, grammar
- **Expand:** Add sensory detail, description, dialogue
- **Summarize:** Condense chapters or scenes
- **Rewrite Tone:** Adjust dialogue/narrative tone (e.g., more confrontational, formal, casual)
- **Continue Writing:** AI suggests next paragraph/scene based on context
- **Character Voice Check:** Verify dialogue consistency with character profiles
- **Consistency Analysis:** Flag timeline issues, plot holes, contradictions
- **Brainstorming:** Scene suggestions, plot development, character interviews
- **Cliché Detection:** Highlight overused phrases

**LLM Provider Support:**
- **OpenAI:** GPT-4, GPT-4 Turbo (no 3.5 models)
- **Anthropic:** Claude 3.5 Sonnet, Claude 3 Opus
- **Ollama:** Local models (Llama 3.1, Mistral, others)
- **Custom Endpoints:** OpenRouter, LocalAI, custom API-compatible servers

**Settings & Preferences:**
- **Provider Configuration:** API keys for cloud providers, Ollama server URL
- **Model Selection:** Per-provider default models
- **Per-Feature Overrides:** Use Claude for polish, GPT-4 for brainstorming, Ollama for quick edits
- **Privacy Controls:** Warnings when cloud AI is used, opt-in per project
- **Context Settings:** Choose what AI can see (current doc only vs full project)
- **Cost Awareness:** Token usage estimates, optional monthly budget limits
- **Prompt Library:** Pre-built prompts for common tasks, user customization

### 3.3 Distraction-Free Writing Modes

**All modes user-configurable and toggleable:**

1. **Fullscreen Fade Mode**
   - Entire screen except editor fades to background
   - UI elements appear on mouse movement to screen edges (FocusWriter style)
   - Customizable fade speed and trigger zones

2. **Typewriter Scrolling**
   - Current line stays vertically centered
   - Text scrolls up as you type
   - Configurable center position (30%, 50%, 70% from top)

3. **Focus Mode**
   - Dim all text except current paragraph or sentence
   - Adjustable dim level and focus scope
   - Smooth transitions as cursor moves

4. **Zen Mode**
   - Centered column with customizable width (50-90% of screen)
   - Background dimming or custom image/color
   - Minimal chrome, hidden UI

5. **Custom Editor Appearance**
   - Editor width (fixed pixels or percentage)
   - Background (solid color, gradient, image, animated)
   - Font selection (system fonts + Google Fonts integration)
   - Line spacing, paragraph spacing
   - Cursor style and blink rate

**Focus Mode Preferences:**
- Save per-project or global settings
- Quick toggle via keyboard shortcuts (F11, Cmd/Ctrl+Shift+F, etc.)
- Multiple saved "Focus Profiles" for different writing contexts

### 3.4 Nice-to-Have Features (Post-1.0)

- **Snapshots:** Per-document version snapshots with restore capability
- **Comments & Annotations:** Inline comments, highlights, notes
- **Scrivenings Mode:** View/edit multiple documents as continuous text
- **Collections:** Custom document groupings beyond folder hierarchy
- **Cork Board View:** Visual card-based outlining (low priority)
- **Outliner View:** Spreadsheet-style document overview with metadata columns
- **Writing History:** Track daily word counts, writing streaks, progress graphs
- **Project Statistics:** Detailed analytics, readability scores
- **Dictation Integration:** Voice-to-text for drafting
- **Collaboration:** Real-time co-editing, comments, suggestions (post-2.0)

---

## 4. User Interface Design

### 4.1 Layout Philosophy

**Default View on Launch:**
- **User Preference:** Remember last layout state
- **Options:**
  - Two-pane: Navigator (left) + Editor (right)
  - Editor-only: Navigator hidden until toggled
  - Custom layouts with saved workspaces

**Layout Components:**
- **Navigator Panel:** Document tree, search, filter, drag-drop
- **Editor Panel:** Rich text editor, formatting toolbar (collapsible)
- **Inspector Panel:** Metadata, synopsis, notes (optional, right sidebar)
- **AI Assistant Panel:** Slide out from user-selected edge (top/right/bottom/left)
- **Status Bar:** Word count, targets, git status, focus mode toggle

### 4.2 Design Principles

**Visual Wow Factor:**
- **Modern Aesthetics:** Clean, polished, professional design
- **Smooth Animations:** Fade transitions, smooth scrolling, fluid panel movements
- **Responsive:** Adapt gracefully to window resizing, multi-monitor setups
- **Theming:** Light/dark modes, custom themes with color palette editor
- **Iconography:** Clear, intuitive icons with tooltips
- **Typography:** Beautiful default fonts, extensive customization

**Accessibility:**
- **Keyboard Navigation:** Full app usable without mouse
- **Screen Reader Support:** Proper ARIA labels, semantic HTML
- **Contrast Compliance:** WCAG AA compliant color schemes
- **Font Scaling:** Respect system font size preferences
- **Reduced Motion:** Option to disable animations

**Writer-Friendly UX:**
- **Simple Language:** Avoid technical jargon (e.g., "Revisions" not "Branches")
- **Progressive Disclosure:** Hide complexity until needed
- **Undo/Redo:** Comprehensive history with visual timeline
- **Auto-Save:** Aggressive background saving, never lose work
- **Crash Recovery:** Auto-recover unsaved changes on restart

### 4.3 Keyboard Shortcuts

**Essential Shortcuts:**
- `Ctrl/Cmd + N` - New document
- `Ctrl/Cmd + Shift + N` - New folder
- `Ctrl/Cmd + S` - Manual save (always safe)
- `Ctrl/Cmd + F` - Find
- `Ctrl/Cmd + Shift + F` - Find in project
- `F11` - Toggle fullscreen
- `Ctrl/Cmd + Shift + D` - Toggle distraction-free mode
- `Ctrl/Cmd + \` - Toggle navigator panel
- `Ctrl/Cmd + Shift + \` - Toggle inspector panel
- `Ctrl/Cmd + Shift + A` - Toggle AI assistant panel
- `Ctrl/Cmd + G` - Git commit
- `Ctrl/Cmd + Shift + G` - Git sync (push/pull)

**User Customization:**
- Rebindable shortcuts via settings
- Import/export keybinding profiles

---

## 5. AI-Friendly Development Architecture

### 5.1 Modular Design Principles

**Goal:** Codebase optimized for AI development with Claude/GPT-4

**Module Size Constraints:**
- **Max lines per file:** 500 lines (strict)
- **Max function length:** 50 lines (prefer 20-30)
- **Max file complexity:** McCabe complexity < 10 per function

**File Organization:**
```
chicken-scratch/
├── src-tauri/              # Rust backend
│   ├── core/               # Core logic (file ops, git)
│   │   ├── project/        # .chikn format handling
│   │   ├── scrivener/      # .scriv import/export
│   │   ├── git/            # Git operations
│   │   └── formats/        # Format conversions
│   ├── models/             # Data models
│   └── api/                # Tauri commands (frontend↔backend)
├── src/                    # React frontend
│   ├── components/         # UI components
│   │   ├── editor/         # Text editor
│   │   ├── navigator/      # Document tree
│   │   ├── ai/             # AI assistant
│   │   └── dialogs/        # Modals, settings
│   ├── hooks/              # React hooks
│   ├── state/              # State management (Zustand/Jotai)
│   └── utils/              # Helper functions
├── docs/                   # Documentation
│   ├── architecture/       # Architecture decision records
│   ├── api/                # API documentation
│   └── guides/             # Development guides
└── tests/                  # Automated tests
    ├── unit/
    ├── integration/
    └── e2e/
```

### 5.2 Documentation Strategy

**For AI Context:**
- **README per module:** Purpose, dependencies, API surface
- **Inline comments:** Explain WHY, not WHAT
- **Type annotations:** Comprehensive TypeScript types, Rust type signatures
- **API contracts:** Clear input/output specs for all functions
- **Examples:** Code examples in docstrings for complex functions

**Architecture Decision Records (ADRs):**
- Document key decisions: Why Tauri? Why Pandoc? Format design rationale
- Store in `docs/architecture/` for AI context retrieval

### 5.3 Testing Strategy

**Automated Testing (AI can write and maintain):**
- **Unit Tests:** Rust (cargo test), TypeScript (Vitest)
- **Integration Tests:** Backend↔Frontend API contracts
- **E2E Tests:** Playwright for critical user workflows
- **Format Tests:** .chikn ↔ .scriv round-trip validation

**Manual Testing:**
- User acceptance testing for UX polish
- Cross-platform validation (Windows, macOS, Linux)
- Scrivener compatibility verification

**Test Coverage Goals:**
- Core logic (Rust backend): 80%+
- Format conversion: 90%+ (critical for data integrity)
- UI components: 60%+ (focus on user workflows)

### 5.4 Dependency Management

**Rust (Cargo):**
- Workspace structure for modular crates
- Minimal dependencies, prefer std library when possible
- Document all dependency choices in ADRs

**TypeScript (npm/pnpm):**
- Monorepo with workspaces if needed
- Lock versions strictly (pnpm preferred for determinism)
- Regular dependency audits for security

---

## 6. Project Plan & Milestones

### Phase 1: Foundation (Months 1-2)
**Goal:** Core infrastructure and basic editing

**Deliverables:**
- [ ] Tauri 2.0 app scaffold (Rust + React)
- [ ] Basic TipTap editor integration
- [ ] `.chikn` format spec implementation (read/write)
- [ ] Simple document navigator (tree view, create/delete docs)
- [ ] Project creation/open/save workflows
- [ ] Basic Markdown editing with live preview toggle

**Milestone:** Open `.chikn` project, create/edit documents, save changes

### Phase 2: Scrivener Compatibility (Months 3-4)
**Goal:** Full Scrivener import/export

**Deliverables:**
- [ ] .scriv XML parser (read project structure)
- [ ] RTF import (RTF → Pandoc Markdown conversion)
- [ ] Metadata extraction (labels, status, keywords, synopsis)
- [ ] .scriv export (Markdown → RTF, generate .scrivx)
- [ ] Round-trip validation tests (import → export → import = no data loss)
- [ ] Research folder support (import PDFs, images, docs)

**Milestone:** Import existing Scrivener project, edit, export back to .scriv with zero data loss

### Phase 3: Rich Features (Months 5-6)
**Goal:** Professional writing tools

**Deliverables:**
- [ ] Full RTF formatting (fonts, colors, styles)
- [ ] Custom style system (create/apply/manage styles)
- [ ] Metadata management UI (labels, status, keywords, synopsis)
- [ ] Word count targets (per-doc and project-wide)
- [ ] Find & replace (project-wide)
- [ ] Document templates (character, setting, chapter)
- [ ] Compile/export (DOCX, PDF via Pandoc)

**Milestone:** Feature parity with Scrivener for core writing workflows

### Phase 4: Git Integration (Months 7-8)
**Goal:** Version control workflows

**Deliverables:**
- [ ] Git initialization for projects
- [ ] Auto-commit system (configurable intervals)
- [ ] Manual commit UI (message, author)
- [ ] Branch management ("Revisions" UI)
- [ ] Remote sync (GitHub, Gitea push/pull)
- [ ] Conflict resolution UI (basic merge tools)
- [ ] Git status visualization (changed files, commit history)

**Milestone:** Full git workflows with writer-friendly UI

### Phase 5: AI Assistant (Months 9-10)
**Goal:** LLM-powered writing tools

**Deliverables:**
- [ ] AI provider integration (OpenAI, Anthropic, Ollama APIs)
- [ ] Parallel writing mode UI (side-by-side panes)
- [ ] AI panel (slide from any edge)
- [ ] Core AI operations (polish, expand, summarize, rewrite tone, continue)
- [ ] Context management (select what AI sees)
- [ ] Settings UI (API keys, model selection, per-feature overrides)
- [ ] Privacy controls (opt-in, warnings, local-only option)

**Milestone:** AI writing assistant with multi-provider support

### Phase 6: Distraction-Free & Polish (Months 11-12)
**Goal:** UX excellence and final polish

**Deliverables:**
- [ ] All distraction-free modes (fullscreen, typewriter, focus, zen)
- [ ] Theme system (light/dark, custom themes)
- [ ] Focus profiles (save/load preferred settings)
- [ ] Animations and transitions polish
- [ ] Accessibility compliance (WCAG AA)
- [ ] Keyboard navigation completeness
- [ ] Performance optimization (large projects, smooth scrolling)
- [ ] Cross-platform testing and bug fixes

**Milestone:** Production-ready 1.0 release

### Phase 7: Launch & Feedback (Month 13+)
**Goal:** Release and iterate

**Deliverables:**
- [ ] Beta testing with writer community
- [ ] Documentation (user manual, video tutorials)
- [ ] Website and marketing materials
- [ ] Package distributions (AppImage, .deb, .dmg, .exe)
- [ ] Bug fixes from beta feedback
- [ ] v1.0 public release
- [ ] Post-launch: nice-to-have features (snapshots, collections, etc.)

---

## 7. Success Criteria

### 7.1 Feature Completeness
- [ ] Full Scrivener import with 100% data preservation
- [ ] Export to .scriv that Scrivener opens without errors
- [ ] All essential features functional (see 3.1)
- [ ] Git workflows intuitive for non-technical writers
- [ ] AI assistant provides measurable writing value

### 7.2 Quality Metrics
- [ ] Zero data loss in .chikn ↔ .scriv conversions (validated by automated tests)
- [ ] 80%+ core backend test coverage
- [ ] Smooth performance with projects up to 500 documents
- [ ] Cross-platform parity (Windows, macOS, Linux identical UX)
- [ ] Crash-free for 99% of user sessions (telemetry data)

### 7.3 User Satisfaction
- [ ] Writers successfully migrate from Scrivener without workflow disruption
- [ ] Positive feedback on distraction-free writing experience
- [ ] AI features used regularly (>30% of active users enable AI)
- [ ] Community adoption (forums, subreddit, Discord)

---

## 8. Risks & Mitigations

### 8.1 Technical Risks

**Risk:** Scrivener format changes break compatibility
- **Mitigation:** Monitor Scrivener updates, maintain compatibility layer, version detection

**Risk:** Pandoc conversion loses formatting edge cases
- **Mitigation:** Extensive test suite with diverse RTF samples, fallback to RTF passthrough when needed

**Risk:** Git workflows confuse non-technical users
- **Mitigation:** Progressive disclosure (git is optional), clear onboarding, writer-friendly terminology

**Risk:** AI API costs surprise users
- **Mitigation:** Cost warnings, budget limits, prominent local-only option (Ollama)

### 8.2 Legal Risks

**Risk:** Literature & Latte (Scrivener) legal action over format compatibility
- **Mitigation:** Clean-room implementation, no Scrivener code copied, focus on interoperability (fair use)

**Risk:** Google Bard trademark (if we had chosen "Bard")
- **Mitigation:** N/A - chose "Chicken Scratch" / `.chikn` (clear of conflicts)

### 8.3 Project Risks

**Risk:** Scope creep (too many features delay launch)
- **Mitigation:** Strict MVP definition, nice-to-have features post-1.0, ruthless prioritization

**Risk:** AI development velocity slower than expected
- **Mitigation:** Modular architecture, comprehensive docs, incremental delivery, human oversight for complex features

---

## 9. Open Questions & Future Considerations

### 9.1 Open Questions
- [ ] Should we support Scrivener 1.x projects natively, or require upgrade to 3.x?
- [ ] Telemetry/analytics for improving UX (opt-in only)?
- [ ] Plugin system for community extensions?
- [ ] Mobile companion app (iOS/Android) for on-the-go editing?

### 9.2 Future Features (Post-1.0)
- Collaboration (multi-user editing, shared projects, comments)
- Cloud sync without git (Dropbox-style, simpler than git for some users)
- Publishing integrations (Wattpad, Medium, WordPress direct publishing)
- Advanced outlining (mind maps, story structure templates)
- Screenplay-specific features (Fountain format, industry-standard formatting)
- Academic writing (citations, bibliographies, LaTeX export)

---

## 10. Conclusion

**Chicken Scratch** aims to be the definitive Scrivener alternative for cross-platform writers, with particular focus on enabling macOS→Linux migration. By combining Scrivener's organizational power, git-native workflows, AI writing assistance, and distraction-free UX, we create a tool that respects writers' existing workflows while enabling modern, efficient writing practices.

**Success means:** Writers can seamlessly transition between platforms, maintain perfect Scrivener compatibility, leverage AI without losing authorship, and enjoy a beautiful, distraction-free writing experience—all with zero data loss and maximum creative control.

**Development Philosophy:** Build iteratively, prioritize data integrity, optimize for AI development, and always put the writer's needs first.

---

**Next Steps:**
1. Initialize project repository structure
2. Set up Tauri 2.0 development environment
3. Implement Phase 1 foundation (basic editor + .chikn format)
4. Begin Scrivener compatibility research and implementation

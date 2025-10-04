# ChickenScratch: Feature-Complete Cross-Platform Scrivener Editor

## Vision
Feature-complete, cross-platform Scrivener editor to enable macOS-to-Linux writer migration. Not lightweight - prioritize feature parity and quality over minimal footprint.

## Core Requirements (from conversation)

### 1. Target Users
- Writers working with Scrivener projects
- Users transitioning from macOS to Linux/Windows
- Feature-complete solution (not lightweight alternative)

### 2. Scrivener Compatibility
- **Full bidirectional compatibility**: Zero data loss when editing .scriv projects
- Must support all Scrivener features EXCEPT cork board (low priority):
  - Document hierarchy editing
  - Metadata (labels, status, keywords, synopsis)
  - Research folder
  - Character/setting templates
  - Compile/export functionality
  - Outliner views

### 3. File Format Support
- **Primary format**: .scriv (native Scrivener format)
- **Import/Export targets**: .md (Markdown), .rtf (RTF)
- Formats don't need to coexist - they are conversion targets
- Default should be whatever preserves the most data (.scriv)

### 4. Git Integration
- **Local save is default**
- Optional git-based workflows configurable in preferences:
  - Auto-commit (configurable frequency)
  - Manual commit with UI
  - Background sync to GitHub/Gitea
  - Branch-per-draft workflows
- All git options should be configurable in preferences GUI

### 5. Markdown Features
- Extended markdown (GFM, tables, footnotes)
- Live preview toggle option
- Pandoc as dependency is acceptable

### 6. Distraction-Free Writing
- **Toggleable** focus mode (not always-on)
- Customizable:
  - Editor width
  - Background
  - Themes
- Full UI available when needed
- Features:
  - Word count goals and statistics
  - Pomodoro timers / session tracking
  - Auto-save frequency preferences
  - Dark mode / theme customization

## Technical Constraints

### AI-Development Optimized
- 100% AI-developed codebase
- Source files must be manageable without overwhelming limited contexts
- Modular architecture with clear boundaries
- Well-documented for LLM understanding
- Design stored in Serena MCP for session persistence

### Technology Preferences
- **Rust preferred** ("coolness factor")
- But pragmatic choice prioritized over ideology
- Pandoc dependency acceptable
- Cross-platform: Windows, macOS, Linux

## Design Notes from Discussion

### Git + Scrivener Format
- No tension between .scriv and git
- .scriv folders work fine in git repos (proven by user's existing workflow)
- RTF files are text-based and trackable
- XML metadata files diff cleanly

### UI Philosophy
- Distraction-free ≠ limited UI
- Button for focus writing mode when desired
- Full features available when needed

# Final Architecture Decisions - October 4, 2025

## Version Control: Hybrid Strategy (APPROVED)

### Implementation
```
.chikn structure:
MyProject.chikn/
├── project.yaml
├── manuscript/
├── research/
├── templates/
├── settings/
├── revs/                    # Automatic snapshots
│   ├── snapshot-*.tar.gz   # Compressed working tree
│   └── manifest.json       # Snapshot metadata
└── .git/                    # Optional git repo
```

### Components

**1. Snapshots (revs/) - Always Enabled**
- Automatic tarball snapshots
- Before: compile, major operations, on interval
- Keep last N (default: 10, configurable)
- Compress entire working tree (NOT .git)
- Quick restore for "oops" moments
- No git knowledge required
- Self-contained safety net

**2. Git Integration - Optional/Automatic**
- Use `git2` Rust crate (libgit2)
- Auto-detect: internal .git OR parent .git OR create new
- Invisible to writer (no git jargon in UI)
- Powers: Milestones, Sandboxes, History, Diffs
- Auto-commit on meaningful events
- Writer-friendly terms only

**3. Detection Logic**
```rust
fn detect_git_mode(chikn_path: &Path) -> GitMode {
    if chikn_path.join(".git").exists() {
        GitMode::Internal      // .git inside .chikn
    } else if find_parent_git(chikn_path).is_some() {
        GitMode::External      // .git in parent folder
    } else {
        GitMode::CreateInternal  // Init new .git
    }
}
```

### UI Terminology (Phase 4)
- ❌ Never show: commit, branch, merge, push, pull
- ✅ Show instead: Milestone, Sandbox, History, Sync
- ✅ Actions: "Mark Milestone", "Create Sandbox Draft", "Merge Sandbox"
- ✅ Status: "Last saved", "3 milestones", "2 sandboxes active"

### .gitignore Pattern
```gitignore
# Inside .chikn (if using internal git)
revs/              # Don't version the snapshots
.DS_Store
*.tmp

# In parent repo (if using external git)
*.chikn/.git/      # Ignore nested repos
*.chikn/revs/      # Ignore snapshots
```

## Dependencies: Final Decisions

### Pandoc - KEEP AND BUNDLE

**Decision:** Bundle Pandoc binary with application

**Rationale:**
- Multi-format export essential (DOCX, PDF, ODT, RTF)
- Publishing requirements non-negotiable
- Rust replacement = months of work
- No viable alternative for v1.0

**Implementation:**
- Include Pandoc binary in Tauri bundle
- GPL compliance: Publish build scripts, add attribution
- Platform-specific binaries (Windows, macOS, Linux)
- ~50MB installer increase (acceptable)
- Document in About dialog and LICENSE

**Location:**
```
dist/
└── resources/
    └── pandoc/
        ├── pandoc.exe         # Windows
        ├── pandoc             # Linux
        └── pandoc             # macOS
```

### Git - USE git2 CRATE

**Decision:** Use git2 Rust crate (libgit2 bindings)

**Rationale:**
- No system git installation needed
- Statically linked with app
- GPLv2 with linking exception (compatible)
- Full git functionality
- Cross-platform

**Features Available:**
- Init, commit, branch, merge
- Push, pull, fetch (remote sync)
- Diff, log, status
- Worktrees (advanced)

## Scope: Full Feature Set (CONFIRMED)

### v1.0 Includes ALL Phases

**NO cuts - everything stays:**

**Phase 3: Rich Features**
- ✅ TipTap editor with full formatting
- ✅ Complete theme system (not just dark/light)
- ✅ All metadata fields
- ✅ Compile/export to all formats

**Phase 4: Git Integration**
- ✅ Hybrid snapshot + git strategy
- ✅ Full git workflows
- ✅ Remote sync (GitHub/Gitea)
- ✅ Conflict resolution UI

**Phase 5: AI Assistant**
- ✅ Multi-provider (OpenAI, Anthropic, Ollama)
- ✅ Parallel writing mode
- ✅ All AI features as specified
- ✅ Context management

**Phase 6: Polish**
- ✅ All distraction-free modes
- ✅ Full theme customization
- ✅ Accessibility compliance
- ✅ Cross-platform testing

### Rationale for Full Scope

User requirement: "I wouldn't ship an alpha/beta that isn't feature-complete"
External reviews attempted cuts - REJECTED by user
All features essential for Scrivener parity
AI assistant is differentiator, not optional

## Implementation Order (FINAL)

**Confirmed sequence:**
1. Phase 3: Frontend (React + TipTap)
2. Phase 4: Git integration (hybrid strategy)
3. Phase 5: AI assistant (multi-provider)
4. Phase 6: Polish and testing

**Current status:** Phase 1-2 complete, ready for Phase 3

## Technical Constraints (ACCEPTED)

### External Dependencies (Both Required)

**Pandoc:**
- Bundle with installer
- GPL compliance mandatory
- ~50MB installer increase
- Platform-specific binaries

**Git (via git2 crate):**
- Statically linked
- No system installation
- Invisible to users
- Powers version features

### No Alternatives Considered
- Rust RTF/DOCX/PDF reimplementation = months
- Not viable for timeline
- Pandoc is industry standard
- Accept the dependency

## Architecture Patterns (FINAL)

### File-First, Not Database

**Structure:**
- .chikn folders are canonical source
- No project database
- Lightweight registry for recent files only
- Portable (copy folder = copy project)
- Git and snapshots are layers, not locks

### Git Patterns Supported

**Pattern A: Internal git (self-contained)**
```
Novel.chikn/
└── .git/              # Project-specific repo
```

**Pattern B: External git (workspace)**
```
Fiction/
├── .git/              # Workspace repo
├── Novel1.chikn/
└── Novel2.chikn/
```

**Pattern C: No git (snapshots only)**
```
Novel.chikn/
└── revs/              # Just snapshots
```

**All three patterns supported automatically**

## Decisions Rejected

### Cuts Proposed by External Reviews
❌ Delay AI to v2.0
❌ Simplify theme system
❌ Limit metadata to compile-critical
❌ Reduce scope for faster release

**User decision:** Full feature set or don't ship

### Pandoc Replacement
❌ Rust-based RTF converter
❌ Custom DOCX generator
❌ Typst for PDF
❌ Any reimplementation

**User decision:** Not reinventing wheels, keep Pandoc

## Success Criteria (UNCHANGED)

### v1.0 Requirements
- Full Scrivener bidirectional compatibility
- Zero data loss in conversions
- All planned features functional
- Git workflows invisible but powerful
- AI assistant working
- Cross-platform parity
- Professional polish

**No compromise on quality or completeness**

## Next Session Actions

**Ready to proceed:**
1. Update .chikn spec to include revs/ folder
2. Begin Phase 3: Frontend development
3. Implement with full feature scope
4. No further strategic decisions needed

**Strategic clarity achieved - move to implementation**

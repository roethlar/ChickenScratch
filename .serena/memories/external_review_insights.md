# External Review Insights - October 4, 2025

## Reviews Conducted

**Sources:**
1. Grok (X/Twitter AI)
2. Gemini (Google)
3. GPT-4 (OpenAI)

**Documents Reviewed:**
- PROJECT_STATUS.md
- DEVLOG.md
- TECHNICAL_SUMMARY.md
- GIT_BACKUP_STRATEGY.md
- All specification documents

## Universal Consensus

### Market Viability: ✅ YES

**All reviewers agreed:**
- Clear niche market (Scrivener Linux migration)
- Underserved audience (10k-50k potential users initially)
- Real pain point (Linux writers using Wine/hacks)
- Lossless round-trip is killer feature
- No direct competitor with this focus

**Target Audience:**
- Linux writers migrating from macOS Scrivener
- Tech-savvy authors wanting git workflows
- Writers seeking non-proprietary formats
- Open-source advocates

### Version Control Strategy: HYBRID APPROACH

**Unanimous recommendation:**

1. **Built-in snapshots (revs/)**
   - Always enabled
   - No git knowledge required
   - Quick "oops" recovery
   - Scrivener parity (safety net)
   - Tarball compression
   - Keep last N (configurable)

2. **Invisible git plumbing**
   - Use git2 crate (libgit2)
   - Auto-commit on milestones
   - Powers advanced features
   - Never expose git jargon
   - Writer-friendly UI terms

3. **External repo detection**
   - Walk up directory tree
   - Support "one repo, many projects" workflow
   - Avoid nested repo warnings
   - Flexible for all use cases

**Implementation Details:**
- Use `git2` Rust crate (no system git needed)
- Snapshots = tarball of working tree (NOT .git)
- .gitignore excludes revs/ folder
- Auto-detect internal vs external .git
- Worktrees for per-project branches (advanced)

### Pandoc Dependency: KEEP IT

**Critical Decision:**

**All reviewers confirmed:**
- Cannot replace Pandoc in under a day
- Multi-format export essential (DOCX, PDF, ODT, RTF)
- Publishing requirements non-negotiable
- Rust reimplementation = months of work

**Solution:**
- Bundle Pandoc binary with installer
- Accept GPL licensing obligations
- ~50MB installer increase (acceptable)
- Clear attribution in About dialog
- Document source/build scripts

**Alternative Rejected:**
- Rust RTF/DOCX/PDF converters insufficient
- Would need custom typesetting engine
- Not feasible for v1.0 timeline

## Priority Recommendations

### Immediate Next Steps

**All reviews said: START FRONTEND (Phase 3)**

**Rationale:**
- Backend is production-ready (51 tests passing)
- 14 Tauri commands ready for testing
- Cannot get user feedback without UI
- Writers judge by interface
- Need MVP for validation

**No cuts to v1.0 scope - ALL features stay:**
- ✅ AI Assistant (Phase 5)
- ✅ Full theme system
- ✅ Complete metadata
- ✅ All distraction-free modes

### Before v1.0 Beta

1. **Round-trip validation**
   - Export Corn.chikn → .scriv
   - Open in actual Scrivener app
   - Verify complete data preservation

2. **Cross-platform testing**
   - Windows build and test
   - Linux build and test
   - CI/CD matrix setup

3. **User documentation**
   - Installation guide
   - User manual
   - Tutorial videos

## Market Positioning

### Unique Selling Points (from reviews)

1. **"Scrivener for Linux, Evolved"**
   - Only tool with lossless bidirectional conversion
   - Git-native workflows
   - No vendor lock-in

2. **"No Subscription, No Lock-in"**
   - One-time purchase model
   - Open format (.chikn = Markdown + YAML)
   - Future-proof

3. **"Blazing Fast, Native Performance"**
   - Tauri + Rust advantage
   - Not Java/Electron bloat
   - Cross-platform native

### Competitive Landscape

**Main Competitors:**
- Manuskript (free, Linux, lacks import/export polish)
- NovelWriter (Markdown-focused, no Scrivener compat)
- Atticus (paid, good formatting, no Linux)
- Bibisco (free, basic features)
- Scrivener (macOS/Windows only, Wine on Linux)

**Differentiation:**
- Only tool focused on migration
- Only lossless round-trip guarantee
- Only git-native workflow
- Only cross-platform with Scrivener parity

## Technical Decisions Validated

### Keep as Planned
1. ✅ Tauri 2.0 + Rust + React
2. ✅ .chikn format (Markdown + YAML)
3. ✅ Pandoc for multi-format export
4. ✅ git2 crate for version control
5. ✅ Full feature set for v1.0

### Implementation Order
1. **Now:** Fix remaining issues, commit state
2. **Next:** Frontend (Phase 3) - React + TipTap
3. **Then:** Git integration (Phase 4) with hybrid strategy
4. **Then:** AI Assistant (Phase 5)
5. **Finally:** Polish (Phase 6)

## User Feedback Strategy

**Recommendations from reviews:**

1. **Beta Testing**
   - Target: r/scrivener, r/writers, NaNoWriMo
   - Goal: 50-100 testers
   - When: After basic UI working

2. **Positioning**
   - "Scrivener for Linux" angle
   - Emphasize lossless migration
   - Highlight git-friendly format

3. **Community**
   - Open source on GitHub
   - Clear contribution guidelines
   - Writer-friendly documentation

## Risk Assessment Updates

**Validated by Reviews:**

1. **Pandoc dependency** - Acceptable with bundling
2. **Git complexity** - Mitigated by invisible plumbing + snapshots
3. **Market size** - Sufficient for sustainable project
4. **Competition** - Differentiated enough to succeed

**New Risks Identified:**

1. **Learning curve** - Must be easier than Scrivener
2. **Setup friction** - Bundled Pandoc critical
3. **Cross-platform bugs** - Test before beta
4. **User expectations** - Scrivener parity is high bar

## Strategic Decisions Made

### Version Control Architecture (FINAL)
```
MyNovel.chikn/
├── project.yaml
├── manuscript/
├── research/
├── revs/                    # Snapshots (always enabled)
│   ├── snap-001.tar.gz
│   ├── snap-002.tar.gz
│   └── manifest.json
└── .git/                    # Optional (auto-created or detected)
```

**Features:**
- Snapshots: Simple, fast, writer-friendly
- Git: Invisible, powers Milestones/Sandboxes
- Detection: Internal .git OR external .git OR neither
- UI: Writer terminology only ("Milestone" not "commit")

### Scope Decision (FINAL)
- ✅ NO cuts to v1.0
- ✅ ALL features as specified
- ✅ Full Scrivener parity
- ✅ Complete AI assistant
- ✅ Full theme system

### Dependency Strategy (FINAL)
- ✅ Bundle Pandoc binary
- ✅ Use git2 crate (libgit2)
- ✅ Accept GPL obligations
- ✅ ~50MB installer acceptable

## Next Session Priorities

**Confirmed Order:**
1. Frontend (Phase 3) - React + TipTap editor
2. Git integration (Phase 4) - Hybrid strategy
3. AI Assistant (Phase 5) - Multi-provider
4. Polish (Phase 6) - Final testing

**No further strategic decisions needed - proceed with implementation**

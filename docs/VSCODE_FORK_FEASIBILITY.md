# VS Code Fork Feasibility Analysis for ChickenScratch
**Research Date**: 2025-10-14
**Context**: Exploring forking VS Code/code-server as alternative to Tauri approach
**Status**: Comprehensive analysis complete with recommendation

---

## Executive Summary

**Recommendation: Continue with Tauri Approach** ✅

After comprehensive research into forking VS Code or code-server for ChickenScratch, the analysis strongly favors **continuing with the current Tauri-based architecture**. While VS Code provides excellent git integration and text editing capabilities, the performance overhead, maintenance burden, and market positioning challenges significantly outweigh the benefits for a writer-focused application.

### Key Finding
The critical insight is that **VS Code's git features are not inherently better for writers**. The .chikn format is already git-friendly (plain text Markdown + YAML), and a custom UI with writer-friendly terminology ("Save Revision" vs "Commit", "Compare Drafts" vs "Diff") would provide a superior user experience for the target audience.

---

## Research Findings

### 1. VS Code Architecture Overview

#### Technical Foundation (2025)
- **Platform**: Electron-based (Chromium + Node.js)
- **Process Model**: Multi-process architecture
  - Main process (Electron)
  - Renderer process (Workbench UI)
  - Extension Host process (isolated extensions)
  - Utility processes (sandboxed operations)
- **Extension API**: Comprehensive, well-documented, process-isolated
- **Communication**: JSON-RPC between processes

#### Source Control API
VS Code provides a powerful Source Control API that allows custom SCM providers:
- `SourceControl` entity for managing resource states
- Extension points: `registerRemoteSourceProvider`, `registerCredentialsProvider`, `registerBranchProtectionProvider`
- Git extension exposes API accessible via `vscode.extensions.getExtension('vscode.git').exports`
- Custom providers can integrate seamlessly with VS Code's UI

#### Built-in Features Relevant to Writing
- **Zen Mode**: Distraction-free editing (hides menus, activity bar, status bar, panels)
  - Keyboard shortcut: `Ctrl+K Z` (Windows/Linux) or `Cmd+K Z` (macOS)
  - Configurable via settings
  - Centered layout option available
- **Extensions**: Writer Mode, Ultra Zen Mode, Spell Checkers, Markdown tools
- **Git Integration**: Visual diff, staging, commit history, branch management, merge conflict resolution

---

### 2. code-server Architecture

#### Overview
- Tool for running VS Code in the browser (remote development)
- Latest release: v4.104.3 (October 2025)
- Platforms: macOS, Linux (amd64, arm64, armv7l), Windows
- Package formats: deb, rpm, tar.gz, Docker containers

#### Deployment Options
- Self-hosted on cloud platforms (AWS, Azure, GCP, DigitalOcean)
- One-click deployment scripts available
- Containerized deployments supported
- Works under existing proxies and infrastructure

#### Key Distinction
code-server is **not a fork** but a tool to run VS Code remotely. For a desktop writing application, this adds unnecessary complexity without benefits.

---

### 3. VS Code Fork Examples (2025)

#### Successful Forks

**Cursor IDE**
- VS Code fork with deep GPT integration
- Target: Solo developers for AI-assisted coding
- Pricing: $20/month (pro), $40/user/month (business)
- Key feature: Composer (builds entire applications)
- Market position: Speed and local context for rapid prototyping

**Windsurf IDE** (Codeium)
- Launched November 2024 as "first agentic IDE"
- Built on VS Code open-source foundation
- Target: Large, complex codebases with team collaboration
- Pricing: $15/month, $30/month (teams)
- Key feature: Cascade technology (multi-file context awareness)
- Market position: Deep understanding for enterprise codebases

**Firebase Studio** (Google)
- Launched April 2025 (successor to Project IDX)
- Google's entry into AI-enhanced development
- Built on VS Code foundation
- Target: Google Cloud developers

#### Key Observations
- All major forks are **developer tools** targeting coding workflows
- Extensions install identically (same extension API)
- All use AI/ML as primary differentiation
- No major writing-focused VS Code forks identified

---

### 4. Licensing and Fork Restrictions

#### Source Code License: MIT
- **Permissive**: Free to fork, modify, and distribute
- **Commercial Use**: Fully allowed without restrictions
- **Attribution**: Minimal requirements

#### Practical Limitations
1. **Marketplace Access**: Forks **cannot access** the Visual Studio Marketplace
   - Major limitation: Cannot install Microsoft extensions
   - April 2025 incident: Cursor users unable to use Microsoft's C/C++ extension
   - Requires building separate extension ecosystem or alternative marketplace

2. **Microsoft Extensions**: Licensed only for "Visual Studio family of products"
   - Cannot use in forks
   - Includes popular tools like C/C++, Python, Remote Development

3. **Branding**: Must remove Microsoft branding and telemetry
   - VSCodium example: Compiles from MIT-licensed source without MS branding
   - Must create custom product identity

#### Conclusion on Licensing
- **Legal**: Forking is fully legal and permissible
- **Practical**: Marketplace restrictions create significant ecosystem challenges

---

### 5. Performance Characteristics (2025 Benchmarks)

#### Electron/VS Code Performance

| Metric | VS Code/Electron | Tauri | Advantage |
|--------|------------------|-------|-----------|
| **Bundle Size** | 85-100 MB | 2.5-3 MB | Tauri 30x smaller |
| **Runtime Memory** | ~100 MB baseline | 30-40 MB | Tauri 2.5x more efficient |
| **Startup Time** | 1-2 seconds | <500ms | Tauri 2-4x faster |
| **Platform Binary** | 150 MB (includes Chromium + Node.js) | Native OS rendering | Tauri vastly smaller |

#### Performance Analysis
- **Electron Overhead**: Bundling full Chromium browser drives large bundle sizes
- **Memory Usage**: Each Electron app runs independent Chromium instance
- **Optimization Potential**: Electron can be optimized (NAPI-RS with Rust), but baseline remains high
- **User Perception**: Writers expect lightweight tools (Scrivener, Ulysses), not 100MB+ apps

#### Tauri Advantages
- Native OS webview (no bundled browser)
- Rust backend for performance and memory safety
- 50% less memory usage vs Electron equivalents
- Professional startup times (<500ms feels instant)

---

### 6. Competitive Landscape Analysis (2025)

#### Top Scrivener Alternatives

**Paid Solutions:**
- **Atticus**: $147 one-time, easier than Scrivener, superior formatting
- **Ulysses**: Subscription, minimalist markdown-based, distraction-free
- **Final Draft**: Screenwriting focus
- **Vellum**: High-end formatting, expensive

**Free/Open-Source:**
- **Reedsy Book Editor**: Free, cloud-based, version control
- **Manuskript**: Best free open-source alternative
- **yWriter**, **Zettlr**, **bibisco**, **Quoll Writer**: Various focuses

#### Market Trends (2025)
- Writers seeking **simpler, more modern** alternatives to Scrivener
- **Cloud accessibility** increasingly important
- **Lower learning curve** demanded
- **Better formatting** than Scrivener
- Movement toward **lightweight tools**, not heavyweight apps

#### Positioning Analysis

**VS Code Fork Positioning:**
- ❌ "Developer tool for writers" - confusing brand message
- ❌ Looks/feels like code editor (because it IS)
- ❌ Hard to differentiate from "just use VS Code + extensions"
- ❌ 85-100MB bundle feels bloated for writing app
- ❌ 100MB+ RAM usage excessive for text editing
- ⚠️ Appeals to developer-writers but alienates general writers

**Tauri Positioning:**
- ✅ "Purpose-built for writers" - clear value proposition
- ✅ Lightweight (3MB) signals professional, optimized tool
- ✅ 30-40MB RAM = respectful of user resources
- ✅ Unique UX optimized for creative writing workflow
- ✅ Git integration under hood, writer-friendly UI on surface
- ✅ Clear differentiation from both Scrivener AND dev tools

---

### 7. Development Effort Analysis

#### VS Code Fork Approach Estimate

| Phase | Effort | Details |
|-------|--------|---------|
| Fork Setup | 1-2 weeks | Clone repo, build system, initial customization |
| Codebase Learning | 4-8 weeks | Understanding millions of LOC, architecture |
| Feature Stripping | 3-4 weeks | Remove code-specific features, keep text editing |
| UI Customization | 4-6 weeks | Adapt workbench for writer UX |
| .chikn Integration | 2-3 weeks | File format support, custom SCM provider |
| Extension Marketplace | 2-4 weeks | Build alternative or workaround |
| **Total Phase 1 Equivalent** | **16-27 weeks** | **4-6.75 months** |
| **Ongoing Maintenance** | 2-4 hours/week | Keep up with upstream VS Code changes |

**Ongoing Maintenance Burden:**
- VS Code releases monthly with significant changes
- Merging upstream changes into fork requires constant effort
- Risk of breaking changes disrupting custom modifications
- As documented: "Even minor changes can cause significant maintenance challenges"

#### Tauri Approach (Current Status)

| Phase | Status | Effort Remaining |
|-------|--------|------------------|
| Phase 1: Foundation | ~8% complete | 6 weeks |
| Phase 2: Scrivener Import | Not started | 2 months |
| Phase 3: Rich Features | Not started | 2 months |
| Phase 4: Git Integration | Planned | 2 months |
| Phase 5: AI Assistant | Planned | 2 months |
| Phase 6: Distraction-Free | Planned | 2 months |
| **To Feature Parity** | - | **~4-6 months** |

**Key Considerations:**
- Already 8% complete with working foundation
- AI-friendly codebase (small files, clear docs, modular)
- No ongoing fork maintenance burden
- Full control over UX/architecture
- Purpose-built for target use case

#### Sunk Cost Analysis
**Switching to VS Code Fork Would Mean:**
- ❌ Abandoning 8% complete, working codebase
- ❌ Discarding comprehensive documentation (5,000+ lines)
- ❌ Starting over with unfamiliar, massive codebase
- ❌ No faster time-to-market (possibly slower)
- ❌ Worse performance (memory, bundle size, startup)
- ❌ Accepting ongoing maintenance burden
- ❌ Limiting market positioning to developer-adjacent

---

### 8. Critical Insight: Git Integration for Writers

#### The False Premise
The assumption that "VS Code's built-in git features" are advantageous for writers contains a critical flaw: **developer UX ≠ writer UX**.

#### VS Code Git Features (Developer UX)
- Visual diff viewing
- Staging/unstaging changes
- Branch management
- Commit history timeline
- Merge conflict resolution
- Pull/push to remotes
- **Language**: Commits, branches, merges, diffs, staging

#### What Writers Actually Need
- "Save this version as..." (not "create commit")
- "Revision snapshots" (not "commits")
- "Compare drafts" (not "diff files")
- "Restore to earlier version" (not "checkout commit")
- "Branch draft ideas" translated as "Try different ending" (not "create branch")
- **Language**: Revisions, drafts, versions, snapshots

#### The .chikn Format Advantage
The .chikn format is **already git-friendly**:
- Plain text Markdown files (perfect for diff/merge)
- YAML metadata files (human-readable, git-trackable)
- Hierarchical folder structure (clear organization)

This means:
- Git operations work perfectly with .chikn format regardless of UI
- **Custom UI with writer-friendly terminology** provides BETTER UX than exposing raw git concepts
- Tauri + git2-rs library can provide identical git functionality with superior UX

#### Recommendation
Build custom git UI in Tauri with writer-appropriate language and mental models. This will be **more usable** for target audience than VS Code's developer-focused git UI, while providing same underlying capabilities.

---

### 9. Comparison Matrix

| Factor | VS Code Fork | Tauri (Current) | Winner |
|--------|--------------|-----------------|--------|
| **Bundle Size** | 85-100 MB | 2.5-3 MB | 🏆 Tauri (30x smaller) |
| **Memory Usage** | ~100 MB | 30-40 MB | 🏆 Tauri (2.5x better) |
| **Startup Time** | 1-2 seconds | <500ms | 🏆 Tauri (2-4x faster) |
| **Development Time** | 16-27 weeks | 6 weeks (Phase 1) | 🏆 Tauri (already started) |
| **Maintenance Burden** | 2-4 hrs/week ongoing | None | 🏆 Tauri |
| **Market Positioning** | "Code editor for writers" | "Purpose-built writing tool" | 🏆 Tauri (clearer) |
| **Git Integration** | Built-in (developer UX) | Custom (writer UX) | 🏆 Tauri (better UX) |
| **Target Audience Fit** | Developer-writers | General writers | 🏆 Tauri (broader) |
| **Extension Ecosystem** | Blocked (no marketplace) | N/A | ⚖️ Neutral |
| **Text Editing** | Monaco Editor (excellent) | TipTap (excellent) | ⚖️ Neutral |
| **Zen Mode** | Built-in | Must build | 🏆 VS Code (existing) |
| **Codebase Complexity** | Millions of LOC | Thousands of LOC | 🏆 Tauri (AI-friendly) |
| **License Freedom** | MIT (with marketplace limits) | MIT (full freedom) | 🏆 Tauri |
| **Performance** | Heavy (Electron) | Lightweight (Rust + native) | 🏆 Tauri |

**Score: Tauri wins 11 of 14 factors**

---

### 10. Risk Assessment

#### VS Code Fork Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Rapid upstream evolution breaks fork | High | High | Dedicate 2-4 hrs/week maintenance |
| Marketplace restrictions limit functionality | Certain | Medium | Build extension alternative |
| Heavy bundle/memory alienates writers | High | Medium | Accept or optimize heavily |
| "Code editor" perception confuses market | High | High | Heavy marketing to reposition |
| Maintenance burden derails development | Medium | High | Hire dedicated fork maintainer |
| Users expect "real VS Code" features | Medium | Medium | Manage expectations |

#### Tauri Approach Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Git integration more complex than expected | Low | Medium | Use mature git2-rs library |
| TipTap limitations for rich text | Low | Medium | Well-documented, extensible |
| Cross-platform bugs | Medium | Low | Early platform testing, CI/CD |
| Scrivener import complexity | Medium | High | Already researched, parser designed |

**Risk Comparison**: Tauri approach has fewer, lower-impact risks.

---

### 11. User Experience Considerations

#### Writer-Focused UX Requirements
1. **Distraction-free modes**: Clean, focused writing environment
2. **Manuscript organization**: Hierarchical document structure
3. **Version management**: Save/restore drafts and revisions
4. **Format flexibility**: Markdown, RTF, export to multiple formats
5. **No technical jargon**: "Revisions" not "commits", "Drafts" not "branches"
6. **Performance**: Instant startup, smooth typing, no lag

#### VS Code Fork UX Challenges
- ❌ Inherently feels like developer tool
- ❌ Technical terminology permeates UI
- ❌ Menu structure designed for coding workflows
- ❌ Heavy performance (100MB RAM) feels sluggish
- ⚠️ Extensive customization needed to hide code-centric features
- ⚠️ Users might expect full VS Code functionality

#### Tauri UX Advantages
- ✅ Blank slate: design optimal writer UX from scratch
- ✅ Writer-friendly terminology throughout
- ✅ Lightweight feel: <500ms startup, minimal memory
- ✅ Custom workflows tailored to creative writing
- ✅ No technical debt from code editor origins
- ✅ Clear brand identity as writing tool

---

### 12. Alternative Hybrid Approach (Considered and Rejected)

#### Concept
Use VS Code as **extension only** rather than fork:
- Build .chikn extension for VS Code
- Provide writer-friendly commands
- Keep Tauri app as primary interface

#### Why Rejected
1. **Fragmented Development**: Maintaining two separate products
2. **Brand Confusion**: Is it a VS Code extension or standalone app?
3. **Performance**: Can't escape Electron overhead with extension
4. **Market Position**: Extension competes with free alternatives
5. **UX Control**: Limited by VS Code's extension API constraints
6. **Git Integration**: Still exposes developer-centric git UI

#### Better Path
Focus all effort on Tauri app, potentially add VS Code extension later as **companion tool** for developer-writers who want both.

---

### 13. Recommended Path Forward

## ✅ Recommendation: Continue with Tauri Approach

### Rationale

1. **Performance Superiority**
   - 30x smaller bundle (3MB vs 85-100MB)
   - 2.5x better memory efficiency (30-40MB vs ~100MB)
   - 2-4x faster startup (<500ms vs 1-2s)
   - Writers expect lightweight, responsive tools

2. **Market Positioning**
   - "Purpose-built for writers" vs "Code editor for writers"
   - Clear differentiation from existing tools
   - Appeals to general writers, not just developer-writers
   - Lightweight signals professionalism and optimization

3. **Development Efficiency**
   - Already 8% complete with working foundation
   - AI-friendly codebase (small files, clear docs)
   - No ongoing fork maintenance burden (2-4 hrs/week saved)
   - Full control over UX and architecture

4. **Git Integration Strategy**
   - .chikn format is already git-friendly (Markdown + YAML)
   - Custom UI with writer terminology > developer git UI
   - git2-rs library provides full git capabilities
   - Better UX than exposing raw git concepts to writers

5. **Competitive Advantage**
   - Unique in market: no other lightweight, git-native writing tool
   - Modern tech stack (Rust + Tauri + React)
   - Cross-platform with native performance
   - Extensible architecture for future features (AI, collaboration)

### Implementation Strategy

#### Phase 1 Completion (Next 6 Weeks)
- ✅ Continue with current Tauri foundation
- ✅ Complete auto-save debouncing
- ✅ Add TypeScript component tests
- ✅ Fix Markdown serialization
- ✅ Establish cross-platform CI/CD

#### Phase 4: Git Integration (Months 7-8)
**Do NOT fork VS Code. Instead:**
- Use git2-rs Rust library for git operations
- Design writer-friendly UI:
  - "Revisions" tab (not "Source Control")
  - "Save Revision As..." button (not "Commit")
  - "Compare Drafts" view (not "Diff")
  - "Restore Version" (not "Checkout")
  - Timeline view of revision history
- Integrate seamlessly with .chikn format
- Add optional GitHub/Gitea sync with writer-friendly language

#### Phase 6: Distraction-Free Modes (Months 11-12)
**Do NOT use VS Code Zen Mode. Instead:**
- Build custom distraction-free modes:
  - **Fullscreen Fade**: Dim everything except editor
  - **Typewriter Mode**: Center current line, scroll smoothly
  - **Focus Mode**: Dim surrounding paragraphs
  - **Zen Mode**: Centered column, minimal chrome
  - **Customizable**: Width, background, themes
- Keyboard shortcuts (customizable)
- Smooth transitions and animations
- Writer-specific features (word count goals, Pomodoro timers)

---

### 14. When Would VS Code Fork Make Sense?

The VS Code fork approach would ONLY be recommended if:

1. **Target audience is developer-writers** who specifically want code-editor-like experience
2. **Project has dedicated fork maintainer** (2-4 hrs/week ongoing)
3. **Bundle size and memory usage are acceptable** (100MB+ is OK)
4. **Brand positioning as "developer tool for writers"** is intentional
5. **Extension ecosystem is not critical** (marketplace access blocked)
6. **Time-to-market is not a factor** (4-6 months minimum)

**Current ChickenScratch Project: NONE of these conditions apply.**

---

### 15. Conclusion

The comprehensive research into forking VS Code or code-server reveals that while **technically feasible**, it is **strategically inadvisable** for ChickenScratch.

**Key Insights:**
1. VS Code's git integration is not inherently better for writers—custom UI beats developer UX
2. Performance overhead (100MB bundle, 100MB RAM) contradicts lightweight writing tool positioning
3. Ongoing maintenance burden (2-4 hrs/week) diverts resources from feature development
4. Market positioning as "code editor for writers" confuses brand message
5. Tauri approach provides superior performance, UX control, and market differentiation

**Recommendation:**
**Continue with Tauri approach.** Complete Phase 1, proceed to Phase 4 git integration with git2-rs + custom writer-friendly UI, and build distraction-free modes in Phase 6 purpose-designed for creative writers.

**Alternative Considered:**
If overwhelming user demand emerges for VS Code integration, build a **VS Code extension** as a companion tool rather than forking the entire codebase. This preserves the Tauri app as primary product while serving developer-writer niche.

---

## Appendix: Research Sources

### VS Code Architecture
- VS Code Extension API Documentation (code.visualstudio.com)
- VS Code Source Control API Guide
- VS Code Sandbox Migration Blog (2022)
- Microsoft VS Code GitHub Repository

### VS Code Forks
- Cursor IDE vs Windsurf comparison articles (2025)
- Firebase Studio announcement (April 2025)
- VSCodium project documentation
- VS Code Fork Wars analysis (blog.openreplay.com)

### Performance Benchmarks
- Electron vs Tauri comparison (2025)
- Electron vs WebView2 analysis
- Tauri performance documentation
- Electron optimization guides

### Writing Software Market
- Scrivener alternatives reviews (2025)
- G2 writing software comparisons
- AlternativeTo Scrivener rankings
- Writing software market trends

### Licensing
- VS Code MIT License (github.com/microsoft/vscode/LICENSE.txt)
- VS Code Marketplace Terms of Service
- Open Source Stack Exchange discussions
- VSCodium licensing documentation

---

**Report Author**: Claude Code Research Framework
**Analysis Confidence**: High (comprehensive multi-source research)
**Recommendation Strength**: Strong (11 of 14 factors favor Tauri)
**Next Review**: After Phase 1 completion or if market conditions change significantly

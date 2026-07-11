# ChickenScratch — Roadmap

## Current State (v1.0.0 release target)

ChickenScratch is a functional cross-platform writing app. Core features implemented. Seeking feedback from writers to identify issues, missing functionality, and UX problems before stable release.

**One engine, one GUI, one canonical storage format** ([ADR-004](adr/ADR-004-deprecated-native-engines.md)):
- **Rust engine** (`crates/core`) — the only `.chikn` reader/writer
- **Tauri + React + TipTap** — the desktop app; daily-driver on macOS + Linux, Windows bundle planned ([CURRENT_PHASE.md](CURRENT_PHASE.md) Step 5)
- **Ratatui TUI** (`chikn` binary, any OS) — keyboard-first terminal editor
- **Converter CLI** (`chikn-converter`) — Scrivener → `.chikn`
- **Canonical storage** — Pandoc Markdown (`.md` files on disk)

The earlier SwiftUI, WinUI, and Qt6 native experiments were removed per ADR-004; their history stays in git.

Pandoc is a runtime dependency but only for compile/export and import — not for core editing. The Tauri editor uses `tiptap-markdown` for in-process markdown ↔ HTML. The TUI edits markdown directly, no conversion at all.

Feature sections named "(Tauri)" are desktop-app features; "(TUI)" sections describe the terminal editor.

### What's Built

**Editor (Tauri)**
- TipTap WYSIWYG with formatting toolbar (bold, italic, underline, strike, headings, lists, blockquote, code, links)
- Inline comments (right-gutter panel) and footnotes
- Find & Replace (Ctrl+F / Ctrl+H)
- AI text operations (polish, expand, simplify, brainstorm)
- Browser-native spell check
- Auto-save, session word count, focus mode with typewriter scrolling
- Light/dark/sepia themes, print support (Ctrl+P)

**Editor (TUI)**
- Simple-mode editing (type to insert, Emacs-style shortcuts, no vim modes)
- Native soft word-wrap (toggle via Ctrl+W)
- Edit / Preview view modes (Ctrl+T)
- Comments overlay (F2) with add/edit/resolve/delete
- Anchored inline comments (F3 wraps the current selection with a comment span)
- Save (Ctrl+S), Save Revision (Ctrl+R) with push-to-backup

**Organization**
- Binder: drag-and-drop, context menus, folder management, width-resizable
- Manuscript/Research/Trash structure (auto-created, self-healing)
- Move to... folder picker, Empty Trash
- Templates: Scene, Chapter, Character Sheet, Setting, Outline
- Inspector: synopsis, label, status, keywords, include-in-compile, word count target, compile order
- Corkboard: card grid with grouping, AI summaries, document linking
- Manuscript preview: continuous prose with type-aware section headers
- Command palette (Ctrl+K)
- Project-wide search (Ctrl+Shift+P) with editor highlight

**Compile / Export**
- DOCX, PDF, EPUB, HTML, ODT via Pandoc
- Compile dialog with title page, section separators, Shunn manuscript format
- Per-document include/exclude and compile-order override
- Settings-driven formatting

**Import**
- Scrivener (.scriv) with metadata, hierarchy, RTF → markdown
- All Pandoc-supported formats → markdown
- Markdown folder import

**Revisions (Tauri)**
- Embedded git2-rs (no system git required)
- Save revision, history, restore
- Word-level diff viewer (tracked-changes style)
- Draft versions (branches): create, switch, merge
- Side-by-side draft comparison (Compare Drafts dialog)
- Auto-commit every 10 minutes
- Auto-backup on close + periodic + on-named-revision
- Remote sync: push/fetch to any git URL with HTTPS-token auth, ahead/behind status

**Statistics**
- Per-document word counts with bar chart
- Word count targets with progress bar
- Page estimate, reading time
- Daily writing history chart (14-day view)

**Settings**
- General, Writing, Backup, Remote, AI (with kill switch), Compile, Shortcuts
- All keyboard shortcuts customizable

**Infrastructure**
- Error boundary, toast notifications, custom dialogs
- Window/panel state persistence
- Pandoc detection with install helper
- Recent projects, Wayland compatibility
- AI via reqwest (no curl dependency)

**Format (engine) — v1.2 lock, shipped 2026-07-09**
- Genre-agnostic schema: one generic `fields` map per document; novelist keys are UI conventions ([UI_CONVENTIONS_NOVELIST.md](UI_CONVENTIONS_NOVELIST.md))
- Unknown-key preservation at every open surface (`.meta`, `project.yaml`, thread entries) — saves never silently destroy other tools' data
- Legacy top-level novelist keys auto-lift into `fields`
- `format_version` marker in `project.yaml`; canonical (sorted, byte-stable) serialization
- Full-fidelity round-trip test suite (`crates/core/tests/format_round_trip.rs`); spec matches engine behavior ([CHIKN_FORMAT_SPEC.md](CHIKN_FORMAT_SPEC.md))

---

## v1.1 — Shipped

### AI Streaming Responses ✓
`ai_transform_stream` emits `ai:chunk` / `ai:done` / `ai:error` events on a per-`request_id` channel; the AiMenu replaces selection incrementally as tokens arrive. Providers: Ollama (newline-delimited JSON), Anthropic and OpenAI (SSE).

### Remote Sync — merge UX for conflicting pulls ✓
`sync_pull` returns a tagged `PullResult` (up_to_date / fast_forward / merged / conflicts{files}); the UI surfaces a real merge-conflict dialog with abort / force-pull / resolve-manually paths. Draft merge gained the same treatment in fifth review batch — `merge_draft` returns a sibling `MergeResult` enum.

### Frontend parity (SwiftUI + Qt6 + WinUI) — superseded, will not ship
Superseded by [ADR-004](adr/ADR-004-deprecated-native-engines.md) before completion: the native frontends were deprecated and later removed. Kept here as history; the partial-parity details are in git history of this file.

---

## Current phase — Format finalization, then UI sync

The `.chikn` format is one concept, genre-agnostic. This phase solidifies that split: lock the format schema and give UIs a single generic extensibility point (`fields` map per document). See [plans/PHASE_FORMAT_FINALIZATION.md](plans/PHASE_FORMAT_FINALIZATION.md) for the original plan and [plans/PLAN_FORMAT_LOCK_ENGINE.md](plans/PLAN_FORMAT_LOCK_ENGINE.md) for the engine lock that shipped 2026-07-09 (see "Format (engine)" under What's Built). Multi-frontend sync was superseded by [ADR-004](adr/ADR-004-deprecated-native-engines.md) — engine + Tauri only.

Remaining phase work is cleanup, not format: CI/release scripts and docs still referencing the deleted native trees (goals G4–G6 in [CURRENT_PHASE.md](CURRENT_PHASE.md)). Feature work below (including the Tier 1/2/3 novelist plans) resumes once the owner closes the phase.

---

## v1.2 — Novelist features (UI-layer, resumes after format finalization)

Comparative survey across Scrivener, Manuskript, bibisco, oStorybook, and yWriter identified features that genuinely change novelist workflows. These are **UI-layer** plans now — the format stays genre-agnostic; the novelist UIs interpret generic `fields` entries per the convention in `docs/UI_CONVENTIONS_NOVELIST.md`.

**Tier 1 — [Novel Structure](plans/TIER1_novel_structure.md)** (highest leverage):
- Scene-level metadata (POV, location, story time, duration, threads) — novelist UI reads/writes known keys in `doc.fields`
- Characters and locations as first-class entities — convention: `characters/` and `locations/` folders, handled by novelist UIs that opt in
- Plot threads — `threads.yaml` as a novelist-UI convention file, format preserves it like any other tracked file

**Tier 2 — [Writer Workflow](plans/TIER2_writer_workflow.md)**:
- Scrivenings mode (edit multiple documents as continuous prose)
- Session targets with deadlines (floating progress badge + welcome-card)
- Per-document snapshots (git log scoped to one file + one-doc restore)
- Timeline view (chronological scene view using the novelist `story_time` convention)

**Tier 3 — [Polish](plans/TIER3_polish.md)**:
- Collections (saved structured queries — operate over `fields` keys too)
- Rich research (inline PDF/image/audio preview)
- Split editor (two panes, independent editors)

Deliberately **not** planned: name generators, deep character-psychology forms, Snowflake-method spreadsheets. See [plans/README.md](plans/README.md).

---

## Platform Packaging

### Windows (.msi)
- CI/CD via GitHub Actions
- Tauri MSI/NSIS targets
- Pandoc path detection for Windows
- File dialog testing

### Linux (Flatpak)
- Flatpak manifest with org.freedesktop.Platform
- WebKitGTK runtime
- Filesystem permissions for project dirs
- Flathub submission

### macOS Code Signing
- Protected release workflow for Developer ID signing and notarization
- App Store Connect API credentials in the `release-macos` GitHub environment
- Release artifact verification with `codesign`, `spctl`, and `stapler`
- Unsigned macOS CI artifacts are smoke-only and not public downloads

### Auto-Update
- tauri-plugin-updater
- GitHub Releases as update source
- Update notification toast
- Signed updates (public/private key pair)

---

## Format evolution (future)

### djot migration
[djot](https://djot.net) is the successor format to CommonMark, designed by pandoc's author to fix round-trip and attribute-handling issues. The Rust parser `jotdown` is fast and the syntax for attributes (`[text]{.class #id key=val}`) is nearly identical to what we already use with pandoc. When djot reaches 1.0 and TipTap gains a djot serializer, a format version bump to `.chikn` v2.0 using djot is worth considering — same writer-visible syntax, cleaner semantics, faster parsing.

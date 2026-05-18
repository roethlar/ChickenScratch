# ChickenScratch — Roadmap

## Current State (v0.1.0-alpha)

ChickenScratch is a functional cross-platform writing app in alpha testing. Core features implemented. Seeking feedback from writers to identify issues, missing functionality, and UX problems before stable release.

**Five frontends, one canonical storage format:**
- **Tauri + React + TipTap** — fullest feature set; daily-driver on macOS + Linux
- **WinUI 3 + C#** (`windows/`) — native Windows app, alpha
- **SwiftUI + Liquid Glass** (`macos/`, macOS 26+) — three-pane shell with format parity (fields/threads/session_target round-trip), Scene-section inspector, characters/locations binder sections, drafts & per-doc history, stats panel, timeline view; rich-text editor, AI, comments, compile, and remote-sync UIs not yet ported
- **Qt6 + cxx-qt** (`linux/`, Wayland-native) — early scaffold with binder/editor/inspector/find-replace
- **Ratatui TUI** (`chikn` binary, any OS) — keyboard-first terminal editor
- **Canonical storage** — Pandoc Markdown (`.md` files on disk)

Pandoc is a runtime dependency but only for compile/export and import — not for core editing. The Tauri editor uses `tiptap-markdown` for in-process markdown ↔ HTML. The TUI and SwiftUI edit markdown directly, no conversion at all.

Not every frontend is at feature parity with Tauri — the SwiftUI, Qt6, and WinUI apps are catching up. Where a feature below says just "Editor" it's shipped on all editor frontends; feature sections named "(Tauri)" are Tauri-only until ported.

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

---

## v1.1 — Shipped

### AI Streaming Responses ✓
`ai_transform_stream` emits `ai:chunk` / `ai:done` / `ai:error` events on a per-`request_id` channel; the AiMenu replaces selection incrementally as tokens arrive. Providers: Ollama (newline-delimited JSON), Anthropic and OpenAI (SSE).

### Remote Sync — merge UX for conflicting pulls ✓
`sync_pull` returns a tagged `PullResult` (up_to_date / fast_forward / merged / conflicts{files}); the UI surfaces a real merge-conflict dialog with abort / force-pull / resolve-manually paths. Draft merge gained the same treatment in fifth review batch — `merge_draft` returns a sibling `MergeResult` enum.

### Frontend parity (SwiftUI + Qt6 + WinUI)
Bring the newer native frontends up to Tauri's feature set. Highest-leverage gaps:
- SwiftUI: rich-text editor (currently plain TextEditor — no markdown rendering or formatting toolbar), drag-drop reorder, comments, footnotes, find/replace, flow mode, compile/export UI, settings panel, AI streaming, remote sync UI. Foundation parity (fields/threads/session_target), inspector scene metadata, binder entities + thread dots, drafts, per-doc history, stats, and timeline are shipped.
- Qt6/Linux: comments, footnotes, revisions UI, compile, AI, settings, templates
- WinUI: ongoing — tracked in `windows/` commit history

---

## Current phase — Format finalization, then UI sync

The `.chikn` format is one concept, genre-agnostic. The five frontends agree on it. This phase solidifies that split: lock the format schema, give UIs a single generic extensibility point (`fields` map per document), and bring every frontend into lockstep. See [plans/PHASE_FORMAT_FINALIZATION.md](plans/PHASE_FORMAT_FINALIZATION.md) for the full plan.

Until this phase is done, feature work below (including the Tier 1/2/3 novelist plans and the v1.1 AI/remote-sync items) is paused. It resumes cleanly once the format is locked.

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

# ChickenScratch — Roadmap & Implementation Plans

## Revision Diff Viewer (Track Changes)

**Goal:** Show what changed between revisions as Word-style tracked changes — additions highlighted green, deletions red with strikethrough. Not a git diff.

**Implementation:**

1. **Backend: `get_revision_content`** — new Tauri command that takes a project path and commit ID, returns the content of a specific document at that revision. Uses `git2` to checkout a file from a specific commit without modifying the working tree.

2. **Word-level diff** — use the `diff` npm package (or `jsdiff`) in the frontend. Compare the old and current document content at the word level, not line level. Output is an array of `{added, removed, value}` chunks.

3. **Render as styled HTML** — merge the diff chunks into a single HTML string:
   - Unchanged text: normal
   - Added text: `<ins style="background:#d4edda">added words</ins>`
   - Deleted text: `<del style="background:#f8d7da;text-decoration:line-through">removed words</del>`

4. **UI** — new "Changes" tab in the Revisions panel, or a modal that opens when clicking a revision. Shows the document with tracked changes rendered. Dropdown to select which document to compare.

5. **Side-by-side comparison** — optional second mode: old version on left, new version on right, both scrolling in sync.

**Dependencies:** `diff` npm package (~8KB). No Rust changes beyond the `get_revision_content` command.

**Effort:** Medium. The diff rendering is the bulk of the work.

---

## Search: Highlight Matches in Editor

**Goal:** When navigating to a document from project search results, highlight the matching text in the editor.

**Implementation:**

1. **Pass search query to editor** — when a search result is clicked, store the query string in the project store alongside the selected document.

2. **TipTap search decoration** — use TipTap's `SearchAndReplace` extension or a custom `Decoration` plugin to highlight all occurrences of the query in the document. Mark them with a CSS class (`.search-highlight { background: #fef08a; }`).

3. **Scroll to first match** — after decorations are applied, scroll the editor to the first match position.

4. **Clear on interaction** — remove highlights when the user starts editing or clears the search.

**Dependencies:** None. TipTap's ProseMirror decorations API handles this.

**Effort:** Small.

---

## Writing Statistics Panel

**Goal:** Show per-document and project-level word counts, progress toward targets, and writing history.

**Implementation:**

1. **Statistics view** — new view accessible from the toolbar (bar chart icon). Three sections:
   - **Project overview:** total word count, document count, average words per document
   - **Per-document table:** name, word count, target, progress bar, last modified. Sortable by column.
   - **Session stats:** words written this session, time elapsed, words per hour

2. **Word count targets** — per-document target stored in `.meta` (already in the spec as `target` field). Editable in the inspector. Progress bar shows percentage.

3. **Project-level target** — stored in `project.yaml` metadata. "50,000 words for NaNoWriMo" type goal.

4. **Writing history** — store daily word counts in `settings/history.json` inside the project. Each entry: `{ date, wordCount, delta }`. Updated on save. Display as a simple bar chart (last 30 days) or streak indicator.

**Backend:**
- `get_project_stats` command — calculates all word counts from loaded project
- Writing history persisted in project's settings/ folder

**Effort:** Medium. The stats calculation is trivial; the UI for the chart and table is the work.

---

## Auto-Commit on Configurable Interval

**Goal:** Periodically save a revision automatically so the writer never loses more than N minutes of work, even if they forget to Save Revision.

**Implementation:**

1. **Setting:** `auto_commit_minutes` in Settings > Backup (default: 0 = disabled). When set, a timer runs in the frontend.

2. **Timer in App.tsx** — alongside the existing auto-backup timer, add an auto-commit timer that calls `save_revision` with message "Auto-save: {timestamp}" if there are uncommitted changes.

3. **Backend:** `has_changes` already exists. The frontend checks it before committing to avoid empty commits.

4. **UI indicator:** When auto-commit fires, show a subtle toast or status bar flash "Auto-saved revision".

5. **History distinction:** Auto-commits should be visually distinguishable from manual revisions in the history (lighter color, or a small "auto" badge).

**Effort:** Small. Timer + existing commands.

---

## Compile: Front Matter

**Goal:** Add title page, copyright page, and dedication to compiled output.

**Implementation:**

1. **Project metadata** — `title`, `author`, `copyright_notice`, `dedication` fields in `project.yaml` metadata. Already partially there (title, author exist). Add `copyright_notice` and `dedication`.

2. **Compile prepends front matter** — before the manuscript content, inject:
   - Title page: centered title, author name, optional subtitle
   - Copyright: `© {year} {author}. {copyright_notice}`
   - Dedication: italic, centered

3. **Settings toggle** — in Settings > Compile: "Include title page", "Include copyright", "Include dedication" checkboxes.

4. **HTML template** — front matter rendered as styled HTML sections with page-break-after CSS for PDF/DOCX.

**Effort:** Small. HTML generation + a few settings fields.

---

## Compile: Section Separators

**Goal:** Customizable scene break markers between sections in compiled output.

**Implementation:**

1. **Setting:** `section_separator` in Settings > Compile. Options: "# # #", "* * *", "—", blank line, page break, custom text.

2. **Compile inserts separator** — between each document's content in the compiled HTML, insert the separator as a styled `<div class="scene-break">`.

3. **CSS in compiled output** — center the separator, add vertical spacing.

**Effort:** Small.

---

## Compile: Manuscript Format Preset (Shunn Standard)

**Goal:** One-click "Standard Manuscript Format" that sets all compile options to industry standard.

**Implementation:**

1. **Preset definition:** Courier/Courier New 12pt, double-spaced, 1" margins, header with "Author / Title / Page", 0.5" paragraph indent, "# # #" scene breaks, title page with word count.

2. **Button in Settings > Compile:** "Apply Manuscript Format" button that sets all fields to the preset values.

3. **Also accessible from Export dialog** — dropdown to select format preset before exporting.

**Effort:** Small. Just preset values for existing settings.

---

## Binder Width Resizing

**Goal:** Drag the binder edge to resize its width.

**Implementation:**

1. **Resize handle** — a 4px-wide div on the right edge of the binder. Cursor changes to `col-resize` on hover.

2. **Mouse drag** — on mousedown, track mousemove to update binder width. Store in localStorage for persistence.

3. **Min/max constraints** — minimum 160px, maximum 400px.

4. **CSS** — binder width set via CSS custom property `--binder-width` instead of fixed `240px`.

**Effort:** Small.

---

## Spell Check Integration

**Goal:** Underline misspelled words in the editor.

**Implementation:**

1. **Browser spell check** — the simplest approach: add `spellcheck="true"` to the TipTap editor element. WebKit/Tauri's webview has built-in spell checking on macOS and Linux (using the system dictionaries).

2. **Toggle in Settings > Writing** — `spell_check` boolean (already exists in settings model).

3. **Apply to editor** — set the `spellcheck` attribute on the editor content div based on the setting.

**Effort:** Tiny. The browser does all the work.

---

## Print Support

**Goal:** Print the current document or the full manuscript.

**Implementation:**

1. **Print current document** — `window.print()` with a print stylesheet that hides the binder, toolbar, and other chrome. The editor content fills the page.

2. **Print manuscript** — switch to Preview view, then `window.print()`. The preview already renders the full manuscript.

3. **Print stylesheet** — `@media print` CSS: hide nav, set margins, use serif font, proper page breaks.

4. **Keyboard shortcut** — Ctrl/Cmd+P triggers print.

**Effort:** Small.

---

## AI: Replace curl with reqwest

**Goal:** Use a proper HTTP client instead of shelling out to curl.

**Implementation:**

1. Add `reqwest` to `src-tauri/Cargo.toml` with `json` feature.
2. Replace `Command::new("curl")` calls in `ai.rs` with `reqwest::Client::post()`.
3. Async — Tauri commands are already async-capable. Use `#[tauri::command(async)]`.
4. Error handling improves — reqwest gives typed errors instead of parsing curl output.

**Effort:** Small-medium. Straightforward replacement.

---

## AI: Streaming Responses

**Goal:** Show AI summaries appearing word-by-word instead of waiting for the full response.

**Implementation:**

1. **Ollama streaming** — Ollama's `/api/generate` supports `"stream": true` which returns newline-delimited JSON chunks.
2. **Tauri event channel** — use Tauri's event system to emit chunks from Rust to the frontend as they arrive.
3. **Frontend** — update the card synopsis progressively as chunks arrive.
4. **Anthropic/OpenAI** — both support streaming via SSE. Same event channel approach.

**Effort:** Medium. Streaming plumbing is the work.

---

## AI: More Actions (Polish, Expand, Brainstorm)

**Goal:** Context menu on selected text offering AI operations.

**Implementation:**

1. **Selection menu** — TipTap bubble menu that appears when text is selected, offering:
   - "Polish" — rewrite for clarity/flow
   - "Expand" — add detail to the selected passage
   - "Brainstorm" — suggest alternatives
   - "Simplify" — reduce complexity

2. **Backend** — generic `ai_transform(content, action)` command that sends appropriate prompts.

3. **UI** — show original and AI suggestion side by side, with Accept/Reject buttons. Never auto-replace the writer's text.

**Effort:** Medium. The prompt engineering and UI for accept/reject is the work.

---

## Remote Sync (GitHub/Gitea)

**Goal:** Push/pull to a remote git repository for backup and collaboration.

**Implementation:**

1. **Settings** — remote URL, authentication (SSH key path or token).
2. **git2-rs push/pull** — `push_remote` and `pull_remote` functions using git2's remote operations.
3. **Authentication** — SSH agent or key file for SSH URLs, credential helper for HTTPS.
4. **Conflict handling** — if pull finds conflicts, show them to the writer and let them choose which version to keep. Use the self-healing repair to fix any inconsistencies.
5. **UI** — "Sync" button in Revisions panel. Status indicator showing sync state.

**Effort:** Large. Authentication and conflict resolution are complex.

---

## Platform: Windows Testing & Packaging

**Goal:** ChickenScratch works on Windows with proper installer.

**Implementation:**

1. **CI/CD** — GitHub Actions workflow that builds on Windows.
2. **Tauri targets** — `msi` and `nsis` installers already supported by Tauri's `"all"` target.
3. **Testing** — Pandoc path detection needs `C:\Program Files\Pandoc\pandoc.exe` added to candidates.
4. **File dialogs** — verify Open/Save dialogs work on Windows (Scrivener UTI picker is macOS-only, already falls back to directory picker).

**Effort:** Medium. Mostly testing and fixing platform-specific issues.

---

## Platform: Flatpak for Linux

**Goal:** Sandboxed Linux package for non-Arch distributions.

**Implementation:**

1. **Flatpak manifest** — `com.chickenscratch.editor.yml` defining the build.
2. **Runtime** — `org.freedesktop.Platform` with WebKitGTK.
3. **Permissions** — filesystem access for project directories, network for AI features.
4. **Flathub submission** — follow Flathub guidelines for review.

**Effort:** Medium. Flatpak manifests require iteration to get right.

---

## Platform: Auto-Update

**Goal:** App checks for updates and can update itself.

**Implementation:**

1. **Tauri updater plugin** — `tauri-plugin-updater` provides built-in update checking.
2. **Update server** — GitHub Releases as the update source. Tauri updater supports this natively.
3. **UI** — notification toast when update is available, "Update Now" button.
4. **Signing** — updates must be signed. Tauri handles this with a public/private key pair.

**Effort:** Medium. Plugin setup + signing infrastructure.

---

## Platform: macOS Code Signing

**Goal:** App doesn't show "unidentified developer" warning.

**Implementation:**

1. **Apple Developer account** — $99/year.
2. **Tauri signing** — configure `tauri.conf.json` with signing identity.
3. **Notarization** — submit to Apple for notarization via `xcrun notarytool`.
4. **CI/CD** — automate signing in GitHub Actions.

**Effort:** Small (technical), but requires Apple Developer enrollment.

---

## Keyboard Shortcut Customization

**Goal:** Let users rebind keyboard shortcuts.

**Implementation:**

1. **Settings section** — "Keyboard Shortcuts" tab showing all actions and their current bindings.
2. **Rebind UI** — click a shortcut, press new key combination, save.
3. **Storage** — custom bindings in `settings.json`.
4. **App.tsx** — read bindings from settings instead of hardcoded key checks.

**Effort:** Medium. The rebinding UI and dynamic dispatch are the work.

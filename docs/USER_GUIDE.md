# ChickenScratch User Guide

## Getting Started

### Creating a New Project

1. Open ChickenScratch
2. Click **New Project**
3. Choose a name and location
4. Your project opens with three folders: **Manuscript**, **Research**, and **Trash**

### Opening an Existing Project

Click **Open Project** and select a `.chikn` folder. Recent projects appear on the welcome screen — click one to open it directly.

### Importing from Scrivener

1. Click **Import Scrivener**
2. Select your `.scriv` project (on macOS, click the .scriv file; on Linux, select the folder)
3. Choose where to save the converted `.chikn` project
4. Your project opens with all documents, formatting, and metadata preserved

**Note:** Pandoc must be installed for Scrivener import to work. ChickenScratch will show a warning with install instructions if it's missing.

---

## The Interface

### Binder (Left Sidebar)

The binder shows your project structure as a tree. It has three permanent sections:

- **Manuscript** — Your writing. Documents here appear in the preview and get included when you export.
- **Research** — Reference material. Character notes, world-building, research. Never included in exports.
- **Trash** — Deleted items go here. You can recover them by dragging back to Manuscript or Research.

**Creating documents:** Click the **+** button in the binder header, or right-click and choose **New Document**. If a folder is selected, the new document goes inside it. Click empty space in the binder to deselect, then + creates at the root level.

**Creating folders:** Click the **folder+** button, or right-click and choose **New Folder**.

**Organizing:** Drag documents to reorder them or move them between folders. Right-click for Move Up/Move Down. Right-click to Rename or Delete.

### Editor (Center)

Click a document in the binder to open it in the editor. The toolbar above the editor shows:

- **Document name** on the left
- **Formatting buttons:** Bold, Italic, Underline, Strikethrough, Headings (H1-H3), Lists, Blockquote, Horizontal Rule, Undo, Redo

**Auto-save:** Your work saves automatically after you stop typing (default: 2 seconds). The status bar at the bottom shows "Saved" or "Modified".

**Find & Replace:** Press **Ctrl/Cmd+F** to find within the current document. Press **Ctrl/Cmd+H** to find and replace.

**Comments:** Select text and click the **speech-bubble** icon in the toolbar (or use the toolbar) to attach a comment to that span. Comments appear in a right-gutter panel — click one to scroll to its anchor, double-click the body to edit, or mark it resolved. Comments are stored in each document's `.meta` sidecar and round-trip through the markdown file via inline span attributes.

**Footnotes:** Click the **asterisk** icon in the toolbar to insert a footnote at the cursor. Footnotes render inline during editing and export as proper footnotes in DOCX/PDF/EPUB.

**AI Text Operations:** Select text and right-click (or use the AI menu) to run **Polish**, **Expand**, **Simplify**, or **Brainstorm** on the selection. Requires AI enabled in Settings.

### Inspector (Right Sidebar)

Click the **panel icon** in the toolbar to open the inspector. For the selected document, you can edit:

- **Title** — Click to rename
- **Synopsis** — A short description of what happens in this scene
- **Label** — Categorize by type (Scene, Chapter, Notes, etc.)
- **Status** — Track progress (Draft, Revised, Final, etc.)
- **Keywords** — Comma-separated tags for searching and grouping
- **Include in Compile** — Toggle whether this document appears in exports

### Views

Three view buttons in the toolbar switch between:

1. **Editor** (pen icon) — Write and edit individual documents
2. **Corkboard** (grid icon) — See all manuscript documents as cards with synopses
3. **Preview** (book icon) — Read your entire manuscript as continuous prose

---

## Corkboard

The corkboard shows each manuscript document as a card displaying its title and synopsis (or a content preview if no synopsis is set).

**Group by:** Use the dropdown to organize cards by Label, Status, or Keyword.

**AI Summaries:** Click the **Summarize** button to automatically generate one-sentence synopses for all cards. Requires Ollama running locally (default) or an AI provider configured in Settings.

---

## Manuscript Preview

The preview stitches all Manuscript documents together as continuous prose. At the top:

- **Project type** (Novel, Short Story, etc.)
- **Title and author**
- **Section count and total word count**

Click **Edit Details** to set the title, author, type, genre, theme, and summary.

For **Novels**, section titles appear as chapter headings. For **Short Stories**, sections flow continuously with scene break markers.

---

## Revisions

Click the **clock icon** in the toolbar to open the Revisions panel.

### Save Revision

Type a description (e.g., "Finished Chapter 3 rewrite") and press Enter. This creates a named checkpoint you can return to at any time.

### Revision History

The History tab shows all your saved revisions with timestamps. Click the restore icon on any revision to go back to that state. **Restoring is non-destructive** — it creates a new revision with the old content, so you can always undo a restore.

**Auto-commit:** Every 10 minutes of work, if anything changed, ChickenScratch saves an automatic revision so you never lose more than a few minutes of work.

### Revision Diff

Click the **diff** icon next to any revision to see what changed since the previous one. The viewer shows word-level tracked changes (insertions in green, deletions in red) per document — designed to read like a Word track-changes view rather than a code diff.

### Draft Versions

The Drafts tab lets you create alternate versions of your manuscript. Click **New Draft Version**, give it a name (e.g., "alternate ending"), and work on it separately. Switch between drafts to compare approaches. **Merge Draft** combines a draft back into your main version.

### Backup

Click the **Backup** button at the bottom of the Revisions panel to push your project to a backup directory. Configure the backup directory in **Settings > Backup** for automatic backup every time you close the app.

**Tip:** Set the backup directory to a cloud-synced folder (Dropbox, iCloud Drive, Google Drive) for automatic offsite backup with full version history.

---

## Writing Statistics

Click the **chart icon** in the toolbar to open the statistics panel.

- **Per-document word counts** — Bar chart of every manuscript document with a progress bar against its target (set the target in Inspector).
- **Project totals** — Running word count, page estimate (at 250 words/page), and reading time estimate.
- **Daily writing history** — 14-day bar chart showing how many words you wrote each day. A day's total is recorded automatically when you save.

---

## Command Palette

Press **Ctrl/Cmd+K** to open the command palette. Type to filter actions (new document, toggle focus, compile, open settings, etc.) and press Enter to run. Every menu item is reachable from here without leaving the keyboard.

---

## Project Search

Press **Ctrl/Cmd+Shift+P** to search across every document in the project. Results are grouped by document. Click a result to jump to the editor with that match highlighted.

---

## Focus Mode

Press **Ctrl/Cmd+Shift+F** or click the **maximize icon** in the toolbar. Everything disappears except your text — no binder, no toolbar, no distractions.

- Hover the left edge of the screen to reveal the binder
- Press **Escape** to exit focus mode

---

## Exporting Your Manuscript

Click the **export icon** (file with arrow) in the toolbar to open the Compile dialog.

**Fields:**
- **Title** and **Author** — Prefilled from project metadata; edit for this export only.
- **Section Separator** — The string placed between documents in the compiled output. Default `# # #`; leave blank for no separator.
- **Include title page** — Adds a first page with title and author, centered.
- **Standard manuscript format (Shunn)** — Courier 12pt, double-spaced, 1" margins — the format most fiction markets accept for submissions.

Click **Export** and choose a filename/location. Formats: Word (.docx), PDF, EPUB, HTML, OpenDocument (.odt). Documents with "Include in Compile" unchecked in the Inspector are skipped. Per-document compile order (set in Inspector) overrides binder order.

Default font/spacing/margins (when Shunn format is off) come from **Settings > Compile**.

---

## Settings

Click the **gear icon** in the toolbar to open Settings.

### General
- **Theme:** Light, Dark, or Sepia
- **Pandoc Path:** Override auto-detection if Pandoc is installed in a non-standard location

### Writing
- **Font:** Choose your editing font (Literata, Georgia, Times, Palatino, System)
- **Font Size:** Editor text size
- **Paragraph Style:** Block (spacing between paragraphs) or Indent (first-line indent)
- **Auto-Save Delay:** How long after you stop typing before saving (seconds)

### Backup
- **Backup Directory:** Where to store backup copies of your projects
- **Auto-Backup on Close:** Automatically back up when closing the app
- **Auto-Backup Interval:** How often to back up while working (minutes)

### AI
- **Enable AI Features:** Master toggle for all AI functionality
- **Provider:** Ollama (local, free), Anthropic (Claude), or OpenAI (ChatGPT)
- **Model:** Which model to use for summaries
- **API Key:** Required for Anthropic and OpenAI (stored locally, never shared)

### Compile
- **Default Format:** Your preferred export format
- **Manuscript Font:** Font used in exported documents
- **Font Size, Spacing, Margins:** Export formatting

---

## Keyboard Shortcuts

| Action | macOS | Linux/Windows |
|--------|-------|---------------|
| Save | Cmd+S | Ctrl+S |
| New Document | Cmd+N | Ctrl+N |
| Find in Document | Cmd+F | Ctrl+F |
| Find & Replace | Cmd+H | Ctrl+H |
| Command Palette | Cmd+K | Ctrl+K |
| Project Search | Cmd+Shift+P | Ctrl+Shift+P |
| Focus Mode | Cmd+Shift+F | Ctrl+Shift+F |
| Toggle Binder | Cmd+\\ | Ctrl+\\ |
| Toggle Inspector | Cmd+Shift+I | Ctrl+Shift+I |
| Bold | Cmd+B | Ctrl+B |
| Italic | Cmd+I | Ctrl+I |
| Underline | Cmd+U | Ctrl+U |
| Undo | Cmd+Z | Ctrl+Z |
| Redo | Cmd+Shift+Z | Ctrl+Shift+Z |

---

## The .chikn Format

Your project is a folder containing:

```
MyNovel.chikn/
├── .git/              ← Revision history (automatic)
├── .gitignore
├── project.yaml       ← Project structure and metadata
├── manuscript/        ← Your writing
│   ├── chapter-1.md
│   ├── chapter-1.meta
│   ├── chapter-2.md
│   └── chapter-2.meta
├── research/          ← Reference material
├── templates/
└── settings/
```

- **project.yaml** — Your document hierarchy, project name, and metadata
- **.md files** — Document content (Pandoc Markdown, editable in any text editor)
- **.meta files** — Document metadata (synopsis, label, status, keywords)
- **.git/** — Full version history of every revision you've saved

You can edit these files in any text editor if needed. The format is designed to be human-readable and git-friendly.

---

## Terminal UI (`chikn`)

ChickenScratch ships with a terminal frontend for writing in an SSH session, a tmux pane, or any environment where a full GUI isn't practical. It reads and writes the same `.chikn` projects as the desktop app.

```bash
chikn ~/Writing/MyNovel.chikn
```

**Layout:** binder on the left, editor on the right, status bar at the bottom.

**Keys:**

| Action | Keys |
|--------|------|
| Quit | `q` / `Esc` (from binder) |
| Navigate binder | `↑`/`↓` or `j`/`k` |
| Open document | `Enter` on a binder item |
| Focus editor / binder | `Tab` |
| Save | `Ctrl+S` |
| Save named revision | `Ctrl+R` |
| Cycle view (edit/preview) | `Ctrl+T` |
| Toggle soft word-wrap | `Ctrl+W` |
| Comments overlay | `F2` |
| Anchor comment to selection | `F3` (with text selected via `Shift+arrows`) |
| Command prompt | `;` |
| Show key help | `?` |

The TUI edits markdown directly (no HTML conversion), so files written here are identical to files written by the desktop app. It shares the same settings file (`~/.config/chickenscratch/settings.json`) and will push to backup on named revision when a backup directory is configured.

---

## Troubleshooting

### "Pandoc is not installed"
Install Pandoc for Scrivener import and export:
- **macOS:** `brew install pandoc`
- **Arch Linux:** `pacman -S pandoc`
- **Others:** Download from [pandoc.org](https://pandoc.org/installing.html)

### AI summaries aren't working
- Check that AI is enabled in **Settings > AI**
- For Ollama: make sure it's running (`ollama serve`) and the model is downloaded (`ollama pull llama3.2`)
- For Anthropic/OpenAI: check your API key in Settings

### Imported Scrivener project looks wrong
- Some Scrivener compile template documents may appear in the Manuscript folder. You can move or delete them.
- Formatting is preserved as accurately as possible, but some Scrivener-specific features (compile placeholders, custom styles) don't have equivalents.

### App crashes or shows blank screen
The error recovery screen should appear — click "Try Again" or "Reload App". If the problem persists, your project data is safe (it's on disk, not in the app).

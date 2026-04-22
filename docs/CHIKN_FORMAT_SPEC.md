# .chikn Format Specification v1.1
**Status**: Living Specification
**Last Updated**: 2026-04-18
**Purpose**: Define the .chikn project format for creative writing with git integration

---

## Design Philosophy

The .chikn format is designed to be:
- **Git-friendly**: Plain text files that diff/merge cleanly
- **Human-readable**: Writers can edit files directly if needed
- **Lossless**: Full Scrivener compatibility via metadata preservation
- **Simple**: YAML for structure, Markdown for content
- **Extensible**: Room for future features without breaking changes

---

## Project Structure

```
MyNovel.chikn/
├── .git/                     # Git repository (REQUIRED)
├── .gitignore               # Git ignore rules
├── project.yaml             # Project metadata and hierarchy (REQUIRED)
├── manuscript/              # Main writing folder
│   ├── chapter-01.md        # Document content (Markdown)
│   ├── chapter-01.meta      # Document metadata (YAML)
│   ├── chapter-02.md
│   ├── chapter-02.meta
│   └── subfolder/           # Nested organization allowed
│       ├── scene-01.md
│       └── scene-01.meta
├── research/                # Research materials
│   ├── character-notes.md
│   ├── character-notes.meta
│   └── references/
├── templates/               # Character/setting templates
│   ├── character-template.md
│   └── setting-template.md
└── settings/                # Compile settings, preferences
    └── compile-formats.yaml
```

Revision history lives entirely in `.git/`. The `revs/` tarball scheme in older drafts is not implemented and not required by this spec.

---

## File Format Details

### 1. project.yaml (REQUIRED)

**Location**: `{ProjectName}.chikn/project.yaml`
**Purpose**: Project metadata and document hierarchy
**Format**: YAML

#### Schema

```yaml
# Required fields
id: string                    # UUID for project (generated on creation)
name: string                  # Project display name
created: string               # ISO 8601 timestamp
modified: string              # ISO 8601 timestamp (updated on save)

# Document hierarchy
hierarchy: TreeNode[]         # Top-level hierarchy (see TreeNode schema)

# Optional fields (for future use)
settings:
  auto_save_interval: number  # Seconds between auto-saves (default: 2)
  word_count_goal: number     # Daily/project word count target
  compile_format: string      # Default compile format

metadata:
  author: string              # Project author
  genre: string               # Fiction genre
  target_audience: string     # Target reader demographic
  tags: string[]              # Project tags/categories
```

#### TreeNode Schema

```yaml
TreeNode:
  type: "Document" | "Folder"
  id: string                  # UUID for this node
  name: string                # Display name

  # For Document type:
  path: string               # Relative path to .md file (e.g., "manuscript/ch01.md")

  # For Folder type:
  children: TreeNode[]        # Nested documents/folders (optional, can be empty)
```

#### Example project.yaml

```yaml
id: 8d9f6c30-317e-4aea-9c34-1dbc5c8d6b44
name: My Novel
created: 2025-10-14T10:30:00Z
modified: 2025-10-14T15:45:00Z

hierarchy:
  - type: Folder
    id: folder-001
    name: Manuscript
    children:
      - type: Document
        id: doc-001
        name: Chapter 1
        path: manuscript/chapter-01.md
      - type: Document
        id: doc-002
        name: Chapter 2
        path: manuscript/chapter-02.md

  - type: Folder
    id: folder-002
    name: Characters
    children:
      - type: Document
        id: doc-003
        name: Protagonist Notes
        path: research/protagonist.md

  - type: Folder
    id: folder-003
    name: Template Sheets
    children: []

settings:
  auto_save_interval: 2
  word_count_goal: 2000
```

---

### 2. Document Files (.md)

**Location**: Paths specified in hierarchy (typically `manuscript/`, `research/`)
**Purpose**: Document content
**Format**: Pandoc Markdown (extended)

#### Supported Markdown Features

**Basic Formatting**:
- Headings: `# H1` through `###### H6`
- Bold: `**bold**` or `__bold__`
- Italic: `*italic*` or `_italic_`
- Strikethrough: `~~strikethrough~~`
- Code: `` `inline` `` and ` ```language ``` ` blocks

**Extended Features** (Pandoc):
- Footnotes: `^[inline footnote]` or `[^ref]`
- Tables (GFM syntax)
- Definition lists
- Subscript/Superscript: `H~2~O`, `E=mc^2^`
- Smart quotes: `"curly"` and `'quotes'`
- Em-dashes: `---`
- Ellipses: `...`

**Custom Styles** via Pandoc bracketed spans (`[text]{attrs}`):
```markdown
This is [emphasized text]{.emphasis-red}

Colored text: [red words]{style="color:#ff0000"}
Underlined: [text]{.underline}
```

**Inline Comments** (anchored to a span of text):
```markdown
The protagonist <span class="comment" data-comment-id="c1">hesitates</span>.
```
The comment body, resolved state, and timestamps live in the sidecar `.meta` file under the `comments` field, keyed by `id`. The anchor span in the content uses raw HTML (valid pandoc markdown); markdown tools that don't understand it preserve it as-is.

**Footnotes**:
```markdown
^[Inline footnote body] — for short notes
[^ref] ... [^ref]: Body text — for longer notes
```
Both forms compile to proper footnotes in DOCX/PDF/EPUB via pandoc.

#### File Naming Convention

- **Slugified from display name**: "Chapter 1" → `chapter-01.md`
- **Lowercase with hyphens**: Consistent, URL-safe
- **No special characters**: `a-z`, `0-9`, `-` only
- **Unique within folder**: Avoid name collisions

#### Content Format

```markdown
# Chapter Title

Document content goes here.

Can include **rich** _formatting_.

> Block quotes for emphasis

- Lists
- Are
- Supported

And paragraphs with proper spacing.
```

**Important**: Content files should NOT include YAML frontmatter. All metadata goes in separate .meta files.

---

### 3. Metadata Files (.meta)

**Location**: Same directory as corresponding .md file
**Naming**: `{document-slug}.meta` (matches .md filename)
**Purpose**: Document metadata, formatting, Scrivener compatibility
**Format**: YAML

#### Minimal Schema

```yaml
# Required fields
id: string                    # UUID (matches hierarchy entry)
name: string                  # Display name (can differ from filename)
created: string               # ISO 8601 or legacy format
modified: string              # ISO 8601 timestamp
parent_id: string | null      # Parent folder ID (null for root)
```

#### Extended Schema (Scrivener Compatibility)

```yaml
# Scrivener metadata fields
label: string                 # Label/tag (e.g., "Scene", "Chapter")
status: string                # Status (e.g., "First Draft", "Revised")
keywords: string[]            # Tags/keywords for searching
synopsis: string              # Short summary/synopsis
section_type: string          # Scrivener section type UUID
include_in_compile: boolean   # Include when compiling/exporting

# Custom formatting (for rich text preservation)
custom_styles:
  - name: string              # Style name (e.g., "Emphasis Red")
    color: string             # Hex color (#ff0000)
    font: string              # Font family
    size: number              # Font size in points
    bold: boolean
    italic: boolean
    underline: boolean

# Document statistics
word_count: number            # Current word count
target: number                # Word count target
character_count: number       # Character count (with spaces)

# Scrivener compatibility
scrivener_uuid: string        # Original Scrivener UUID (for import/export)

# Compile ordering & targeting
word_count_target: integer    # Target for this document (0 = no target)
compile_order: integer        # Override order at compile time (0 = use hierarchy order)

# Inline comments anchored to spans in the content
comments:
  - id: string                # Matches data-comment-id in the content span
    body: string              # Comment text
    resolved: boolean         # Whether reviewer has resolved it
    created: string           # ISO 8601
    modified: string          # ISO 8601
```

#### Example .meta file

```yaml
id: doc-001
name: Chapter 1: The Beginning
created: 2025-10-14T10:00:00Z
modified: 2025-10-14T15:30:00Z
parent_id: folder-manuscript

# Scrivener metadata
label: Chapter
status: First Draft
keywords:
  - opening
  - protagonist
  - setup
synopsis: "Hero discovers the call to adventure"

# Statistics
word_count: 2547
target: 3000

# Custom formatting
custom_styles:
  - name: Dream Sequence
    color: "#666666"
    font: Georgia
    size: 12
    italic: true
```

---

### 4. Git Integration (REQUIRED)

Every .chikn project **must** be a git repository. Git provides:
- Version control (document history)
- Branching (alternate drafts, revisions)
- Backup (remote sync to GitHub/Gitea)
- Collaboration (multi-author projects)

#### Git Initialization

When creating a new .chikn project:

```bash
cd MyNovel.chikn
git init
git add .
git commit -m "Initial commit: Project created"
```

#### .gitignore

```gitignore
# OS files
.DS_Store
Thumbs.db

# Automatic snapshots (use git history instead)
revs/

# Editor temp files
*.tmp
*.swp
*~

# Optional: Large media files (use git-lfs if needed)
# research/**/*.mp4
# research/**/*.mov
```

#### Git Workflow for Writers

**Commits = Revisions**:
- Save work-in-progress: `git add . && git commit -m "Draft: Chapter 3 complete"`
- Major milestones: `git tag v1.0-first-draft`

**Branches = Alternate Versions**:
- Try different ending: `git checkout -b alternate-ending`
- Experimental scenes: `git checkout -b experiment-flashback`
- Merge back: `git checkout main && git merge alternate-ending`

**History = Time Machine**:
- View revisions: `git log --oneline`
- Compare versions: `git diff HEAD~5 chapter-01.md`
- Restore old version: `git checkout <commit> -- chapter-01.md`

#### Writer-Friendly Git Commands (for UI)

| Git Command | Writer-Friendly Label | Description |
|-------------|----------------------|-------------|
| `git commit` | "Save Revision" | Save current state with description |
| `git log` | "Revision History" | View timeline of changes |
| `git diff` | "Compare Drafts" | See what changed |
| `git checkout <commit>` | "Restore Version" | Go back to earlier revision |
| `git branch` | "Create Draft Version" | Try alternate approach |
| `git merge` | "Merge Drafts" | Combine alternate versions |
| `git tag` | "Milestone" | Mark important versions |
| `git push` | "Backup to Cloud" | Sync to GitHub/Gitea |

---

### 5. Folder Conventions

#### manuscript/
**Purpose**: Primary writing content (chapters, scenes, sections)
**Organization**: Free-form, hierarchical nesting allowed
**Examples**:
- `manuscript/chapter-01.md`
- `manuscript/part-one/chapter-01.md`
- `manuscript/scenes/opening.md`

#### research/
**Purpose**: Background research, notes, references
**Organization**: Free-form
**Examples**:
- `research/character-notes.md`
- `research/historical-facts.md`
- `research/references/article.pdf`

#### templates/
**Purpose**: Reusable templates (character sheets, setting descriptions)
**Organization**: Flat or categorized
**Examples**:
- `templates/character-template.md`
- `templates/setting-template.md`
- `templates/scene-template.md`

#### settings/
**Purpose**: Project preferences, compile configurations
**Organization**: YAML configuration files
**Examples**:
- `settings/compile-formats.yaml`
- `settings/editor-preferences.yaml`
- `settings/themes.yaml`

---

## Format Versioning

### Version 1.0
- project.yaml with hierarchy
- Markdown documents with .meta sidecar files
- Git integration required
- Simple metadata schema

### Version 1.1 (Current)
- Comments anchored to spans via inline HTML + `.meta` sidecar
- Inline and reference footnotes (pandoc-native)
- `word_count_target`, `compile_order`, `include_in_compile` fields
- Bracketed-span syntax documented for custom styles (`[text]{.class}`)
- Scrivener import/export with full metadata round-trip

### Future Versions (Planned)

**Version 1.2** (Collaboration):
- Multi-author metadata on comments
- Threaded comment replies
- Review/approval workflows

**Version 2.0** (Format evolution):
- Potential migration to [djot](https://djot.net) once it reaches 1.0
  — cleaner attribute semantics, faster parsing via `jotdown` (Rust),
  same writer-visible `[text]{.class}` syntax

---

## File Operations

### Creating Documents

1. Generate unique ID (UUID recommended)
2. Create slug from name: `slugify("Chapter 1")` → `chapter-01`
3. Create files:
   - `manuscript/chapter-01.md` (content)
   - `manuscript/chapter-01.meta` (metadata)
4. Add to `project.yaml` hierarchy
5. Update `project.modified` timestamp
6. Git commit (optional): `git add . && git commit -m "Add: Chapter 1"`

### Updating Documents

1. Modify .md file content
2. Update .meta file `modified` timestamp
3. Update `project.yaml` modified timestamp
4. Git commit (optional): `git commit -am "Update: Chapter 1 draft"`

### Deleting Documents

1. Remove entry from `project.yaml` hierarchy
2. Delete .md file
3. Delete .meta file
4. Update `project.modified` timestamp
5. Git commit: `git commit -am "Delete: Chapter 1"`

### Moving Documents

1. Update `parent_id` in .meta file
2. Update hierarchy in `project.yaml`
3. Optionally move physical files (or leave in place)
4. Update `project.modified` timestamp
5. Git commit: `git commit -am "Reorganize: Move Chapter 1"`

---

## Validation Rules

### project.yaml
- ✅ Must exist at project root
- ✅ Must be valid YAML
- ✅ Must have: `id`, `name`, `hierarchy`, `created`, `modified`
- ✅ All hierarchy IDs must be unique
- ✅ Document paths must exist as .md files
- ✅ Folder IDs must match parent_id references

### Document Files (.md)
- ✅ Must be UTF-8 encoded
- ✅ Must be valid Markdown (Pandoc flavor)
- ✅ Should have corresponding .meta file
- ✅ Path must be relative (no `..` traversal, no absolute paths)

### Metadata Files (.meta)
- ✅ Must be valid YAML
- ✅ Must have: `id`, `name`, `created`, `modified`, `parent_id`
- ✅ ID must match hierarchy entry
- ✅ parent_id must reference valid folder or be null

### Git Repository
- ✅ `.git/` directory must exist at project root
- ✅ Project should have at least one commit
- ✅ `.gitignore` should exist (recommended)

---

## Git Conventions

### Commit Messages

**Format**: `{Action}: {Subject}`

**Actions**:
- `Add`: New document created
- `Update`: Content or metadata changed
- `Delete`: Document removed
- `Reorganize`: Hierarchy changed
- `Milestone`: Major progress marker
- `Revision`: Significant revision/draft completed

**Examples**:
```
Add: Chapter 1 - The Beginning
Update: Chapter 1 draft complete
Delete: Removed old opening scene
Reorganize: Moved flashback to Part Two
Milestone: First draft complete
Revision: Second draft - Chapter 1-5
```

### Branching Strategy

**main** (or **master**): Primary working draft
**draft-v2**: Major revision attempts
**alternate-ending**: Experimental variations
**collab-{author}**: Collaboration branches

### Tags (Milestones)

```
v0.1-outline          # Outline complete
v1.0-first-draft      # First draft complete
v2.0-second-draft     # Revision complete
v3.0-final            # Ready for submission
```

### Remote Sync

**Recommended**:
- GitHub (private repo for privacy)
- Gitea (self-hosted)
- GitLab (private repo)

**Automatic sync**:
- On save: Auto-commit to local git
- Every N commits: Auto-push to remote (configurable)
- Manual: User-triggered backup

---

## Scrivener Compatibility

### Import from .scriv

When importing Scrivener projects:

1. **Parse .scrivx XML** → extract hierarchy and metadata
2. **Convert RTF to Markdown** → use Pandoc
3. **Map UUIDs** → preserve Scrivener document IDs in `scrivener_uuid` field
4. **Preserve metadata** → labels, status, keywords, synopsis
5. **Create .chikn structure** → project.yaml + .md + .meta files

### Export to .scriv

When exporting to Scrivener:

1. **Generate .scrivx XML** → from project.yaml hierarchy
2. **Convert Markdown to RTF** → use Pandoc
3. **Restore UUIDs** → use `scrivener_uuid` if present, generate if missing
4. **Apply metadata** → labels, status, keywords to XML
5. **Create .scriv structure** → Files/Data/{UUID}/content.rtf

### Metadata Mapping

| .chikn Field | Scrivener Field | Notes |
|--------------|-----------------|-------|
| `id` | Document ID | Internal .chikn ID |
| `scrivener_uuid` | BinderItem UUID | For round-trip compatibility |
| `name` | Title | Display name |
| `label` | Label | Scene/Chapter/etc |
| `status` | Status | First Draft/Revised/etc |
| `keywords` | Keywords | Tags for searching |
| `synopsis` | Synopsis | Short summary |
| `section_type` | SectionType | Document type UUID |
| `include_in_compile` | IncludeInCompile | Compile flag |

---

## Research Folder Conventions

### Purpose
- Character development notes
- Setting descriptions
- Historical research
- Reference images/PDFs
- World-building materials

### Organization

**Flat Structure** (simple):
```
research/
├── character-protagonist.md
├── character-antagonist.md
├── setting-hometown.md
└── historical-notes.md
```

**Categorized** (complex):
```
research/
├── characters/
│   ├── protagonist.md
│   └── antagonist.md
├── settings/
│   ├── hometown.md
│   └── forest.md
├── history/
│   └── timeline.md
└── references/
    ├── article.pdf
    └── interview.mp3
```

### Research Documents in Hierarchy

Research documents appear in `project.yaml` hierarchy (optionally):
- Can be top-level folders
- Can be nested under "Research" folder
- Not required to be in hierarchy (can be standalone)

---

## Templates

### Character Template Example

**File**: `templates/character-template.md`

```markdown
# Character Name

## Basic Info
- **Age**:
- **Occupation**:
- **Location**:

## Physical Description
- **Appearance**:
- **Distinguishing Features**:

## Personality
- **Traits**:
- **Quirks**:
- **Motivations**:

## Background
- **History**:
- **Relationships**:
- **Secrets**:

## Story Role
- **Arc**:
- **Key Scenes**:
- **Conflicts**:
```

### Using Templates

1. Copy template to manuscript/research folder
2. Rename: `character-template.md` → `protagonist.md`
3. Fill in details
4. Add to hierarchy if desired

---

## Compile/Export Settings

### settings/compile-formats.yaml

```yaml
formats:
  - id: manuscript-standard
    name: Standard Manuscript
    description: Industry standard manuscript format
    output: docx
    options:
      font: Courier New
      size: 12pt
      line_spacing: double
      margins: 1in
      page_numbers: true
      header: "{author-lastname} / {title} / {page}"

  - id: ebook-kindle
    name: Kindle eBook
    description: Amazon Kindle format
    output: epub
    options:
      include_toc: true
      chapter_breaks: h1
      cover_image: settings/cover.jpg

  - id: pdf-print
    name: Print PDF
    description: PDF for print-on-demand
    output: pdf
    options:
      trim_size: 6x9
      margins: 0.75in
      gutter: 0.25in
      headers_footers: true
```

---

## Migration Path

### From Scrivener (.scriv)
1. Run Scrivener import command
2. Specify .scriv path
3. Choose conversion options (RTF → Markdown)
4. Creates .chikn project with full metadata

### To Scrivener (.scriv)
1. Run Scrivener export command
2. Specify output path
3. Converts Markdown → RTF
4. Creates .scriv with all metadata preserved

### From Other Formats
- **Word .docx**: Import as single document or split by headings
- **Markdown files**: Import with auto-generated hierarchy
- **Plain text**: Import with basic metadata

---

## Best Practices

### File Organization
- Keep manuscripts focused: Use folders for parts/acts
- Use descriptive names: "chapter-01-the-beginning" not "ch1"
- Avoid deep nesting: Max 3-4 levels deep
- Use research folder liberally: Better organized than cluttered manuscript

### Git Workflow
- **Commit frequently**: After each writing session
- **Use branches**: For major revisions or experiments
- **Tag milestones**: v1.0-first-draft, v2.0-revision, etc.
- **Push regularly**: Backup to remote at least daily
- **Write good messages**: "Update: Chapter 3 - Added conflict scene"

### Metadata Management
- Keep synopsis updated: Helps with navigation
- Use labels consistently: Define label set early
- Tag generously: Keywords make searching easy
- Set word count targets: Track progress

### Performance
- Keep documents focused: Break long chapters into scenes
- Avoid huge single files: <10,000 words per document recommended
- Use folders: Better than flat structure with 100+ files

---

## Editor Implementation Requirements

Any editor supporting .chikn format must:

1. **Parse project.yaml**: Load hierarchy and display document tree
2. **Load documents**: Read .md content and .meta metadata
3. **Save atomically**: Write temp files, then rename
4. **Update timestamps**: Modify `modified` field on save
5. **Maintain hierarchy**: Keep project.yaml in sync with file operations
6. **Git integration**: Support commits, branches, history viewing
7. **Validate paths**: Prevent directory traversal, absolute paths
8. **Handle missing files**: Gracefully handle deleted/moved files

### Optional Features
- WYSIWYG markdown editing
- Word count tracking
- Spell checking
- Auto-save with debouncing
- Git auto-commit
- Distraction-free modes
- AI writing assistance
- Compile/export to other formats

---

## Reference Implementations

Two in-tree implementations of this spec, both backed by the same Rust library (`chickenscratch-core`):

- **`src-tauri/`** — Tauri desktop app (Rust backend, React/TipTap frontend). Canonical reference for read/write, hierarchy, comments, footnotes, compile.
- **`crates/tui/`** — `chikn` terminal UI (Rust, ratatui). Exercises the same core library against a markdown-native editor.

A third C# implementation at `windows/` (WinUI 3) is in active development and targets byte-for-byte compatibility with the Rust implementations.

See also `crates/cli/` (`chikn-converter`) for the standalone Scrivener ↔ .chikn converter.

---

## Appendix: Sample Project

See `samples/Corn.chikn/` for a complete working example with:
- Complex hierarchy (folders, nested documents)
- Multiple document types (scenes, notes, templates)
- Proper metadata files
- Realistic content

---

**End of Specification**

**Questions/Clarifications**: Create GitHub issue or update this document
**Reference**: samples/Corn.chikn/ for working example

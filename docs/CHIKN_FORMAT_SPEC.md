# .chikn Format Specification v1.2
**Status**: Living Specification
**Last Updated**: 2026-07-09
**Purpose**: Define the .chikn project format for creative writing with git integration

---

## Design Philosophy

The .chikn format is designed to be:
- **Git-friendly**: Plain text files that diff/merge cleanly, re-emitted in
  one canonical form so history records real edits only
- **Human-readable**: Writers can edit files directly if needed
- **Lossless**: Saving never silently destroys data — unknown YAML keys
  written by other or newer tools survive round-trips (see *Unknown-key
  preservation*), and Scrivener metadata is preserved for round-trip
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
# Version marker (stamped by writers; absent in pre-v1.2 projects)
format_version: string        # e.g. "1.2" — see Format Versioning

# Required fields
id: string                    # UUID for project (generated on creation)
name: string                  # Project display name
created: string               # ISO 8601 timestamp
modified: string              # ISO 8601 timestamp (updated on save)

# Document hierarchy
hierarchy: TreeNode[]         # Top-level hierarchy (see TreeNode schema)

# Optional project-level metadata (all fields optional)
metadata:
  title: string               # Work title (can differ from project name)
  author: string              # Project author
  project_type: string        # e.g. "Novel", "Short Story", "Screenplay"
  genre: string               # Fiction genre
  theme: string               # Central theme
  summary: string             # Short project summary
  session_target:             # Writer session goals (novelist convention)
    words_per_session: number
    deadline: string          # ISO date (YYYY-MM-DD)
    total_target: number
```

Unknown keys — at the top level of `project.yaml` or inside the
`metadata:` block — are tolerated on read and preserved verbatim on write
(see *Unknown-key preservation* under Version 1.2).

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
format_version: '1.2'
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
include_in_compile: "Yes"|"No"  # Include when compiling/exporting.
                              # Canonical wire form is the string "Yes"/"No"
                              # (Scrivener legacy). Readers MUST also accept
                              # a YAML boolean for round-trip with frontends
                              # that historically wrote `true`/`false` here.
                              # Writers emit exactly "Yes" or "No"; readers
                              # treat any value other than the exact string
                              # "No" (or boolean false) as included.

# Scrivener compatibility
scrivener_uuid: string        # Original Scrivener UUID (for import/export)

# Document connections
links: string[]               # IDs of related documents (bidirectional
                              # linking is a UI behavior; the format stores
                              # the ID list as written)

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

Earlier drafts of this section also listed `custom_styles`, `word_count`,
`target`, and `character_count`. No implementation ever wrote or read them
— word/character counts are derived from content, `target` duplicated
`word_count_target`, and rich-text styles are carried in the Markdown
itself as `[text]{.class}` spans — so v1.2 removes them from the schema.
Files that contain them anyway keep them: they round-trip as preserved
unknown keys.

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
word_count_target: 3000

# Generic UI extensibility (see Version 1.2)
fields:
  pov_character: sarah-bennett
  story_time: Day 1, 08:15
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

**On-disk marker.** Writers stamp `format_version` (e.g. `"1.2"`) at the
top of `project.yaml` on every save. The marker is a detection hook for
future migrations, never a read gate: readers accept any value, and a
missing marker simply means the project was last written before v1.2
locked the format (it gains the marker on its next save). A file claiming
a newer version still loads — unknown-key preservation means an older
engine passes through what it doesn't understand instead of destroying
it. Breaking schema changes bump this version together with this spec.

### Version 1.0
- project.yaml with hierarchy
- Markdown documents with .meta sidecar files
- Git integration required
- Simple metadata schema

### Version 1.1
- Comments anchored to spans via inline HTML + `.meta` sidecar
- Inline and reference footnotes (pandoc-native)
- `word_count_target`, `compile_order`, `include_in_compile` fields
- Bracketed-span syntax documented for custom styles (`[text]{.class}`)
- Scrivener import/export with full metadata round-trip

### Version 1.2 (Current) — Generic UI extensibility

The v1.2 schema change is a single addition: **one generic `fields` map per document**, in which UIs store domain-specific data the format itself does not interpret. The format is genre-agnostic; UI conventions are layered on top.

**Schema addition** — in each `.meta` file, optional:

```yaml
# Existing format-level fields (id, name, created, modified, synopsis,
# label, status, keywords, include_in_compile, word_count_target,
# compile_order, comments, links) continue to work unchanged.
fields:
  # Arbitrary string -> YAML-value entries. The format preserves them
  # on round-trip without interpretation. UIs agree on their own key
  # names in separate convention documents.
  pov_character: sarah-bennett
  duration_minutes: 45
  threads:
    - main-plot
    - romance
```

**Contract:**

- Entirely optional; documents without it write no `fields:` key at all (clean diff for projects that ignore the mechanism).
- **Reader tolerance:** any `.meta` file that contains a `fields:` mapping is valid regardless of the keys inside. Readers that don't understand a specific key load it as an opaque `serde_yaml::Value` (or equivalent in other languages) and preserve it.
- **Writer preservation:** when a UI reads a document, modifies part of it, and writes it back, every entry in `fields` survives — including entries that UI does not understand. This is the "tolerant readers, preserving writers" rule from [`FOLDER_FIRST_DOCUMENTS.md`](FOLDER_FIRST_DOCUMENTS.md).
- **No format-level vocabulary.** The format does not define `pov_character` or any other key name. Domain-specific key lists live in separate UI convention documents (for example, `docs/UI_CONVENTIONS_NOVELIST.md`).

**What this replaces.** An earlier draft of v1.2 added typed domain fields (`pov_character`, `location`, `story_time`, `duration_minutes`, `threads`, `characters_in_scene`) directly to the schema. That was a design error — those are novelist-UI concepts, not format concepts. v1.2 ships the generic mechanism instead; novelist UIs write the same six keys into `fields` per their convention doc.

**Legacy migration.** Sidecars written during that draft's window may carry the six keys at the `.meta` top level. Readers lift them into `fields` on load (an existing `fields` entry wins over the stale top-level duplicate), and the next save relocates them under `fields:` on disk. Nothing is deleted; the keys move to where current UIs read them.

**Unknown-key preservation (I5).** Beyond the `fields` map, the format guarantees that unknown YAML keys written by other or newer tools survive read→write cycles at these surfaces:

- the top level of a document's `.meta` sidecar,
- the top level of `project.yaml` and the keys inside its `metadata:` block,
- individual thread entries in `threads.yaml`.

Readers tolerate the unknown keys; writers re-emit them verbatim. `fields` remains the *sanctioned* extensibility surface — UIs should never invent top-level keys — but a top-level key that exists anyway is never silently destroyed. Two structures are **closed**: hierarchy nodes in `project.yaml` (`type`/`id`/`name`/`path`/`children` only) and `comments` entries; adding keys to those requires a format version bump.

**Canonical serialization.** Writers emit each YAML file in one canonical form: known keys in schema order, `fields` entries and preserved unknown keys in sorted (lexicographic) order. Saving the same state twice produces byte-identical `.meta` and `threads.yaml` files (`project.yaml` differs only in its top-level `modified:` timestamp), so the embedded git history records real edits only.

**Out of the format, in the UIs:**

- Scene-level metadata (POV, location, story time, thread membership) — novelist-UI convention, stored in `fields`.
- Characters / locations as entities — novelist-UI convention: `characters/` and `locations/` folders are ordinary sub-folders the format treats like any other, with sidecar `.meta` files that carry id/name/aliases in the format core plus type-specific data in `fields`.
- Plot threads, collections, session targets — novelist-UI conventions that live either in `fields` or in separate novelist-UI YAML files the format tracks but doesn't interpret.

See [`plans/PHASE_FORMAT_FINALIZATION.md`](plans/PHASE_FORMAT_FINALIZATION.md) for the phased rollout, and the Tier 1/2/3 plans under `plans/` for the novelist-UI feature designs that build on this mechanism.

### Future Versions (Planned)

**Version 1.3** (Collaboration):
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
- ✅ May have `format_version` (recommended; stamped by writers)
- ✅ All hierarchy IDs must be unique
- ✅ Document paths must exist as .md files
- ✅ Folder IDs must match parent_id references
- ✅ Unknown top-level and `metadata:`-block keys are valid and must be preserved

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
- ✅ Unknown top-level keys are valid and must be preserved

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

One engine implements this spec: **`crates/core/` (`chickenscratch-core`)** — the only code that reads or writes the format ([ADR-001](adr/ADR-001-single-engine.md), invariant I2). Everything else is a frontend over it:

- **`src-tauri/`** + **`ui/`** — Tauri desktop app (Rust backend, React/TipTap frontend). The reference GUI ([ADR-003](adr/ADR-003-tauri-reference-ui.md)).
- **`crates/tui/`** — `chikn` terminal UI (Rust, ratatui).
- **`crates/cli/`** — `chikn-converter`, the standalone Scrivener ↔ .chikn converter.

Earlier native reimplementations (Swift `ChiknKit`, C# `ChickenScratch.Core`, Qt6) are deprecated and removed from the tree ([ADR-004](adr/ADR-004-deprecated-native-engines.md)); references to "five frontends" or byte-for-byte ports in older documents are historical.

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

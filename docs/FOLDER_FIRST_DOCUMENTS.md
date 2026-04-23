# Folder-First Documents

A design pattern for structured document projects that need history, tooling interoperability, and long-term durability. Extracted from the [`.chikn`](CHIKN_FORMAT_SPEC.md) format underneath ChickenScratch; applicable well beyond prose fiction.

---

## The gap this fills

Most document formats today are one of three shapes:

- **Single binary file, proprietary.** Word docs, Scrivener's inner RTF, Sketch files, OneNote. Opaque to every tool that isn't the one that made them; brittle across decades; one vendor decides what "open" means.
- **Single structured file, open.** Markdown, JSON, Jupyter's `.ipynb`. Diffable and portable, but can't represent a whole *project* — no hierarchy, no per-item metadata, no revision history baked in. Jupyter paid the merge-conflict tax for trying to cram structure into one JSON blob.
- **Raw git repo with loose text files.** Perfectly durable, but with no contract on layout, no per-item metadata, no way to say "this whole directory is one coherent thing."

Between "document file" and "bare repository" sits a real format niche. Scrivener's `.scriv` tried to occupy it with a proprietary folder. Jupyter tried by pretending a notebook is a single file. Obsidian punted and left structure to user convention. A **folder-first document** is the clean version of what each of those almost-was.

---

## The pattern, in six decisions

A folder-first document is a directory on disk with these six properties:

1. **The unit is a folder, not a file.** The project is the containing directory. What you email, zip, commit, or back up is the folder.
2. **A manifest declares structure.** A single YAML file at the root declares hierarchy, identity, and project-wide metadata. The filesystem layout is storage; the manifest is structure.
3. **Content is plain-text with a widely-readable syntax.** Usually Markdown. Whatever enables `grep`, `git diff`, and opening in any text editor.
4. **Per-item metadata lives in sidecars, not frontmatter.** Each content file has a paired `.meta` file (YAML) for typed metadata. Editing metadata never touches content; editing content never touches metadata.
5. **History is embedded git.** A `.git/` inside the project root. Revisions are real commits; any git tool works. Readers assume this exists (and initialize it if missing).
6. **Readers tolerate unknown fields; writers preserve them.** Schema extensions are non-breaking by default. Old tools keep working; new tools add capability.

Together these six give you: atomic backup (copy the folder), clean git diffs (Markdown + YAML, no binary churn), durability (you can read the content in a hex editor if nothing else survives), metadata granularity (per-item undo without content churn), tool interop (any text editor opens any file), and evolvability (new fields don't break old readers).

---

## The decisions, in detail

### 1. The unit is a folder

Every OS and every git host already understands folders. No magic bundling, no custom filesystem support, no registry. Scrivener's `.scriv` extension fools Finder into showing a single icon, but the underlying folder works the same without it. Extension registration is icing, never load-bearing.

This rules out: zipped-bundle formats (opaque to git), nested archives, dependencies on a hidden global registry ("this project doesn't work unless row 542 exists in our SaaS").

### 2. Manifest declares structure

A YAML file at the root (`project.yaml`, `book.yaml`, whatever fits) holds:

- **Identity** — UUID, name, created/modified timestamps.
- **Hierarchy** — a tree of entries referencing files or subfolders, in the order the project wants them seen. The filesystem holds bytes; the manifest says what they mean and how they're ordered.
- **Project-wide metadata** — title, author, tags, or whatever is global to the project.
- **Project-scoped settings** — preferences that travel with the project, not with the machine.

Why YAML over JSON: humans write and read YAML; JSON is for wire formats. TOML is fine too. The choice is less important than the principle: plaintext, human-legible, deterministic.

Why a manifest at all: renaming files shouldn't require renaming every link to them; reordering shouldn't require shell shuffles; seeing the whole structure at a glance shouldn't require a file-tree tool. The manifest is the single artifact that encodes the project's shape.

### 3. Plain-text content

Default to Markdown. It's the lingua franca — GitHub renders it, VS Code renders it, every LLM reads it natively, `vim` handles it, `grep` handles it. Exceptions (images, PDFs, audio, attachments) live in the same folder and are referenced from the manifest or from links in the Markdown. They don't need their own metadata (nothing to say about a `.jpg` beyond its filename).

Anti-pattern: "Let's allow rich text in the content files — HTML, inline SVG, custom XML." You just made the content unreadable by anything that doesn't speak your custom dialect. Keep content Markdown; extend via sidecars, not syntax.

### 4. Sidecar metadata

Each content item that needs typed metadata gets a `.meta` file next to it:

```
manuscript/
├── chapter-01.md
├── chapter-01.meta
├── chapter-02.md
└── chapter-02.meta
```

Tiny, YAML, typed. Fields like:

```yaml
id: 550e8400-e29b-41d4-a716-446655440000
name: Chapter One
created: 2026-01-01T12:00:00Z
modified: 2026-04-23T14:30:00Z
# plus whatever's domain-specific
```

Sidecars cost twice as many files. In exchange:

- **Edits to metadata don't churn the content file.** Bump a status field; Markdown diff is empty. Change a paragraph; sidecar diff is empty. Clean git history on both axes.
- **Tools that don't understand your metadata still read the content.** A Markdown renderer doesn't need to strip a custom YAML frontmatter block it doesn't understand.
- **Partial merges work.** Two writers editing two different fields in the same item produce two separate diffs on two separate files, not a three-way conflict on a frontmatter block.

### 5. Embedded git history

A `.git/` inside the project root. Not optional. Readers check for it on open; if missing, initialize one and commit the current state. Revisions are real commits with real messages.

Why git specifically: it's free, battle-tested, handles binary and text, handles merges with a decent conflict story, and every reader can use it via every git library in every language. The pattern explicitly opts into "git is a dependency" so you don't reinvent version control.

Consequence: opening a project folder in a parent git repo triggers the "nested repo" warning. Handle it: either document the pattern (each folder-first document is its own repo; parent repos that want to track several use submodules or worktrees), or — if the domain allows — put `.git/` in a sibling directory via `GIT_DIR`. Default to "`.git/` inside" for simplicity; the sibling-dir variant exists for fleet management.

### 6. Tolerant readers, preserving writers

Two rules that keep the format evolvable:

- **Readers tolerate unknown fields.** If the manifest has a new top-level key your reader doesn't know, skip it — don't error, don't warn.
- **Writers preserve what they didn't touch.** Read manifest → modify one field → write back, and the unknown fields ride along unchanged.

With these rules, schema additions are non-breaking by default. A document written in 2030 opens in a 2026 reader. Schema *removals* and renames are breaking; guard them behind a major version bump. In practice you rarely need to remove.

---

## When to use the pattern

**Good fits:**

- **Human-authored prose projects** — novels, screenplays, dissertations, research reports, TTRPG campaigns.
- **Knowledge bases** — team wikis, personal notebooks, documentation that outlives tools.
- **Structured issue / task data** — RFCs, design docs, tickets, where "hierarchy + typed metadata + history" matters.
- **Course authoring** — hierarchical units with typed metadata (duration, difficulty, prereqs), students fork via git.
- **Case files** — legal, medical, journalism: documents with amendments, strict audit trail, exhibits.
- **Dataset curation for ML / linguistics** — corpora with per-example annotations and revision history across passes.
- **Any domain where "the project is many items, structure matters, and you want humans to read the storage."**

**Bad fits:**

- **Single standalone files.** A résumé doesn't need a folder. Plain Markdown is fine.
- **Collaborative real-time editing.** Git's per-commit model doesn't handle live cursors; use Yjs, Automerge, or similar CRDTs.
- **High-frequency writes.** Anything committing per keystroke abuses git. Batch-save.
- **Binary-heavy content.** Git-LFS helps but doesn't love you. A photo-heavy project with minimal text belongs elsewhere.
- **Data best served by a database.** If you'd reach for SQLite, use SQLite. Don't abuse YAML manifests for relational queries.

---

## Related work and prior art

- **Scrivener `.scriv`** — folder-first, but closed RTF inside and no git contract. Metadata in XML, not sidecars. Vendor-owned.
- **Jupyter `.ipynb`** — single-file JSON, merge conflicts constant. Not a folder-first format, but shares the "document format readable by many tools" ethos.
- **Obsidian vaults** — folder-first, plain Markdown, but no manifest and no sidecar convention. Structure lives in filenames and wikilinks, not a schema. Excellent for free-form; underpowered for structured projects.
- **Static site generator projects (Hugo, Jekyll, mdBook)** — close to the pattern, but the manifest declares *site* config, not content hierarchy; hierarchy lives in folder structure plus frontmatter. Sidecar metadata is absent.
- **Git-annex / Git-LFS** — solve the binary-content problem within a git repo. Compatible with the pattern; pick them up when media files matter.
- **Fountain** (screenplay format) — an open plaintext format for a narrow domain. Single-file scope; complementary to folder-first, not competing.
- **`.chikn`** — the reference implementation this pattern was extracted from. Adds a fiction-novel schema on top of the six decisions. See [`CHIKN_FORMAT_SPEC.md`](CHIKN_FORMAT_SPEC.md).

---

## Anti-patterns

- **Frontmatter creep.** "Let's just put metadata at the top of the Markdown." Five fields become twenty, become tool-specific, become unparseable. Sidecars prevent this by making adding metadata cost more up-front — friction in the right place.
- **Binary manifests.** SQLite / protobuf / `.plist` for the manifest kills diffability, the format's main strength. YAML or TOML. Always.
- **Hidden state.** A cache directory inside the project that's not in the manifest and not in `.gitignore`. Either commit it (declare it) or ignore it (add it to `.gitignore`). Middle ground breeds surprise.
- **Schema rigidity.** Erroring on an unknown field. Don't. Tolerant readers are the whole point of rule #6.
- **Extension as load-bearing.** Treating the `.chikn` / `.foo` / `.bar` extension as required for correctness. Extensions help OS icons; the manifest is what matters. A folder-first document works whether the folder is named `MyProject` or `MyProject.chikn`.
- **Double manifests.** Shipping both a `manifest.yaml` and a `project.json` for "compatibility." Pick one. Tolerant readers make compatibility the future manifest's problem, not today's.

---

## How to start a new format with this pattern

1. **Decide the manifest schema first.** What's at the root? What's hierarchical? What's typed vs. free-form? Write the schema before writing code.
2. **Decide one content syntax.** Probably Markdown. Don't let a second in unless you genuinely need to.
3. **Write the reader before the writer.** Readers expose assumptions writers paper over.
4. **Ship the spec as Markdown in the repo.** Version it alongside your reference implementation. `FORMAT_SPEC.md`.
5. **Keep the reference implementation small.** A few hundred lines reading and writing the format. Applications consume it.
6. **Commit real sample projects.** Implementers and future-you need them.
7. **Say "tolerant readers, preserving writers" three times.** Then mean it.

---

## License and provenance

This pattern was extracted from the [ChickenScratch](https://github.com/roethlar/ChickenScratch) project's `.chikn` format. It isn't trademarked; use it, steal it, rename it, extend it. The value is in the six decisions, not the label.

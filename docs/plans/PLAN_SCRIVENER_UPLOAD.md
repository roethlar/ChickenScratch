# Plan: Import anything — a new project from any document, folder or writing-app project

(File kept under its first name, `PLAN_SCRIVENER_UPLOAD.md`; the scope grew
past Scrivener on the owner's direction the same day.)

**Owner request (quote):**
> import scrivener needs to support uploads of scrivener files

> import restricted to scrivener pigeonholes this app. a long time ago I
> planned to support importing any kind of doc that pandoc can import.
> scrivener as a project is obviously different from a word doc or markdown
> file, but both need to work. project style imports from the new project
> screen, individual docs from within a project. so a new project should
> import a .odt, .doc, .docx, .md, .markdown, .txt, anything pandoc
> supports. what other apps save projects like scrivener? ulysses? those
> need support.

(2026-10-25.)

**Phase check:** [x] Allowed — reference-app feature over the existing
converter and import services; no format or engine change  [x] Not paused

**Invariants touched:** I2 (uploads are staged in a temporary folder and
handed to the existing converter and services, which write through core;
new project-format readers live in the converter crate, never in the UI);
I6 (a refused or failed import leaves no half-made project and nothing on
disk); I7 untouched.

---

## [MODEL] Intent

One **Import** button on the project screen, in the browser and on the
desktop, takes whatever the writer has and makes a project of it: a single
document in any format Pandoc reads (a Word manuscript becomes one chapter
per top-level heading), a folder or zip of Markdown or text files (Obsidian,
iA Writer, Ulysses external folders, a Notion or Bear export), a Scrivener
project, and, in later slices, the open project formats of other writing
apps. In the browser the file or folder comes from the device you are
browsing on. Inside a project, **Import File** keeps adding a single document
to the binder as it does today.

## [MODEL] Approach

### Slice 1 — one upload door and one detector

`POST /api/import-project-upload` (`crates/web/src/project_upload.rs`),
replacing nothing: `/api/import-upload` (single document into an open
project) stays.

- Multipart: `outputPath` (gate `creatable`, as `import_scrivener` checks
  today), then one or more `file` fields. A single file is taken as is; a
  single `.zip` is extracted; several files are a folder pick, each
  filename carrying its path inside the folder (how a browser folder pick
  sends them).
- Each file is streamed to a temporary folder outside the projects root.
  Paths are checked piece by piece (no `..`, no absolute or empty part, no
  control characters); zip entries go through the same check (zip-slip).
  Any bad path fails the request with 400 and the folder is removed. The
  `zip` crate (deflate only) becomes a dependency of `crates/web`.
- **Detection** (`chikn_converter::detect_import_kind(staged_root)`, shared
  with the desktop): a `.scrivx` at the top or one folder down → Scrivener;
  otherwise a tree holding `.md`/`.markdown`/`.txt` files → Markdown folder
  (the existing `import_markdown_folder`); otherwise exactly one file with
  a Pandoc-readable extension → single document; anything else → 400 "This
  is not a document or project ChickenScratch can import" before Pandoc or
  the projects folder is touched. Later slices add kinds here only.
- Route body limit 512 MiB (research folders hold PDFs); over it, 413 and
  the UI points at the copy-to-server path, which stays.
- Answers `{ project, warnings }` like `import_scrivener`.

### Slice 2 — a project from one document

`chickenscratch_services::io::import_document_as_project(file, output)`:
create the project (profile Notes unless the document looks like a
manuscript, see below), run the existing single-file import, then **split
on top-level headings** when the document has two or more of them: one
binder document per heading, named by the heading, in order, under
Manuscript; text before the first heading becomes a "Front matter" document.
One or no top-level heading → one document named from the file. EPUB
chapters arrive as top-level headings from Pandoc and so split the same way.
The desktop `import_markdown_folder`/`import_scrivener` commands gain a
sibling `import_document_as_project`; the web route calls it on the detected
single document.

Formats: everything the current Import File list accepts (`docx`, `odt`,
`rtf`, `epub`, `md`, `markdown`, `txt`, `html`, `latex`, `rst`, `org`,
`textile`, `fb2`). **Legacy `.doc` is not readable by Pandoc** (today
`io.rs` maps `doc` to the `docx` reader, which fails on a real `.doc`); the
import answers "Save it as .docx first" unless LibreOffice is installed on
the machine, in which case `soffice --headless --convert-to docx` runs first.
Apple Pages: same message (export to Word).

### Slice 3 — the Import button, both shells

- Project screen (`Welcome.tsx`): **Import Scrivener** becomes **Import**.
  In the browser it opens a chooser: **Choose a file** (document or zip),
  **Choose a folder** (hidden where the browser cannot pick folders, which
  includes iPhone Safari; a one-line hint says how to zip a folder on Mac,
  Windows and the iPhone Files app), **It is already on the server** (the
  existing server folder picker, now for any folder). On the desktop the
  native open dialog accepts a file or a folder and the same detector runs
  on the chosen path. Then the name prompt as today, then the import; the
  chooser shows "Uploading <size>…" while a browser upload runs.
- Store: `importProject(kind-agnostic)` sharing the settle code with
  `importScrivener` (open the new project, surface warnings).
- User guide: "Importing" section rewritten around the one button; the
  browser section's "Import uploads" bullet and the phone subsection say
  which way to use; troubleshooting covers "too large", "not a project" and
  "`.doc` needs saving as `.docx`".

### Slice 4 — other writing apps' project formats (one slice each, open formats only)

Each is a reader in `crates/cli/src/<format>/` producing the same
`ImportReport` the Scrivener reader produces, plus a detector entry:

- **TextBundle / TextPack** (`.textbundle` folder, `.textpack` zip:
  `info.json`, `text.md`, `assets/`): the interchange format Ulysses, Bear
  and iA Writer export; a folder of them is a project.
- **yWriter** (`.yw7`, one XML file: chapters, scenes, characters, locations,
  items): scenes become documents under chapter folders; characters and
  places become Research notes.
- **novelWriter** (folder: `nwProject.nwx` XML tree + `content/*.nwd`
  Markdown-like files).
- **Manuskript** (`.msk` zip or folder with a `MANUSKRIPT` marker: outline
  as text files, characters and world as YAML-ish text).

Each slice needs a small sample project for a test; exact file layouts are
confirmed against the apps' documentation or source before coding (all four
are documented or open source). Ulysses' own library files (`.ulyz`) are
undocumented and change between versions: not supported; writers export
TextBundle or Markdown instead. Storyist, Papyrus Author, Vellum, Dabble,
Novlr, Atticus and Campfire are closed or cloud-only: their Word or Markdown
exports go through slices 1–3.

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `crates/web/src/project_upload.rs` (new), `app.rs`, `Cargo.toml` (+ `zip`), `Cargo.lock`, `tests/http.rs` | staging, path checks, zip, detector call, handler, own body limit |
| `crates/cli/src/detect.rs` (new), `lib.rs` | `detect_import_kind` |
| `crates/services/src/io.rs`, `project.rs`; `crates/web/src/commands.rs`; `src-tauri/src/commands/*.rs` | `import_document_as_project`, heading split, `.doc` message / LibreOffice step; command wrappers |
| `ui/src/commands/web.ts`, `io.ts`, `stores/projectStore.ts` | `importProjectUpload`, `importProject` |
| `ui/src/components/welcome/ImportDialog.tsx` (new), `Welcome.tsx`, `editor.css`, `platform.ts` | the Import button and chooser; desktop file-or-folder pick |
| `crates/cli/src/{textbundle,ywriter,novelwriter,manuskript}/` (slice 4, one per slice) | readers + fixtures |
| `docs/USER_GUIDE.md`, `docs/plans/README.md`, `.agents/state.md`, `DEVLOG.md` | records |

## [MODEL] Tests

- [ ] `crates/web` unit: path pieces with `..`, absolute or control
      characters refused; a folder-pick filename lands under the staging
      folder; zip-slip entry refused.
- [ ] `crates/cli` detector: Scrivener at top and one down; Markdown folder;
      single `docx`; a folder of PDFs → not importable.
- [ ] `crates/services`: a Markdown document with three top-level headings
      becomes three documents in order with front matter; one heading → one
      document; `.doc` without LibreOffice → the save-as message.
- [ ] `crates/web/tests/http.rs`: zip-slip → 400, nothing written; no
      recognisable content → 400, no `.chikn`; `outputPath` outside the
      root → 403; a Markdown zip → 200 with the documents; a two-file
      `.scriv` folder pick → 200 (Pandoc cases skip with a printed line
      where Pandoc is absent, which is CI).
- [ ] UI: the chooser renders its choices and hides the folder choice when
      unsupported; a file pick asks for a name and posts with
      `<root>/<name>.chikn`; the server-path choice still works; cancel
      sends nothing.
- [ ] Slice 4, each: a fixture project converts to the expected tree.
- [ ] Declared suite (`.agents/repo-guidance.md`), Tauri crate excepted on
      this box (Known drift); CI on push.

## [MODEL] Owner verification (plain English)

1. Browser, computer: **Import → Choose a file**, pick a Word manuscript with
   chapter headings. A project opens with one document per chapter.
2. Browser, phone: zip a Scrivener project in Files (long-press → Compress),
   **Import → Choose a file**, pick the zip. The converted project opens.
3. **Import → Choose a folder**, pick an Obsidian vault or iA Writer folder.
   The notes appear in the binder.
4. Pick a `.doc` from an old Word: a plain message says to save it as `.docx`.
5. Desktop: the same **Import** button accepts a file or a folder.

## [YOU] Decisions needed

- **Which app formats in slice 4.** Recommended: TextBundle/TextPack (covers
  Ulysses, Bear, iA Writer exports), yWriter, novelWriter, Manuskript, in
  that order; drop or add names.
- Taken unless told otherwise: upload limit 512 MiB with the copy-to-server
  path kept; a Word manuscript splits into chapters on top-level headings;
  Ulysses' internal library files are not read (export instead).

## Status

- 2026-10-25: first draft (Scrivener upload only) superseded the same day by
  the owner: "import restricted to scrivener pigeonholes this app". Redrafted
  as above. Awaiting the owner's go and the slice-4 format list.

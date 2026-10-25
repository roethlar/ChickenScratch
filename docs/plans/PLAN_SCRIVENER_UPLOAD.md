# Plan: Import projects on the project screen, documents inside a project

(File kept under its first name, `PLAN_SCRIVENER_UPLOAD.md`; the scope grew
past Scrivener on the owner's direction the same day.)

**Owner request (quote):**
> import scrivener needs to support uploads of scrivener files

> import restricted to scrivener pigeonholes this app. a long time ago I
> planned to support importing any kind of doc that pandoc can import.
> scrivener as a project is obviously different from a word doc or markdown
> file, but both need to work. project style imports from the new project
> screen, individual docs from within a project. […] what other apps save
> projects like scrivener? ulysses? those need support.

> project-style documents become projects. individual documents are imported
> INTO existing projects. automatically splitting things by chapter headings
> can be an option but not automatic.

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

Two doors, each doing one thing. On the **project screen**, **Import**
takes a *project* saved by another writing app (Scrivener today; the open
formats of other apps as later slices; a folder of Markdown counts) and
makes a `.chikn` project of it. **Inside a project**, **Import File** adds a
*document* in any format Pandoc reads, with an optional, off-by-default
split into one document per top-level heading. Both work the same in the
browser as on the desktop: in the browser, the folder or file comes from the
device you are browsing on.

**How a browser uploads a whole folder.** Every desktop browser (Chrome,
Edge, Firefox, Safari) lets a page ask for a *folder* instead of a file (the
`webkitdirectory` file input, also reached by dragging a folder onto the
page). The browser then sends every file inside it with its path within the
folder, so a `.scriv` folder uploads as is, nothing to zip. Phones are the
exception: iPhone Safari cannot pick a folder, so there the writer zips it
first (long-press → Compress in Files) and uploads the zip. The server
accepts both shapes and treats them alike.

## [MODEL] Approach

### Slice 1 — the project door on the server

`POST /api/import-project-upload` (`crates/web/src/project_upload.rs`); the
existing `/api/import-upload` (one document into an open project) stays.

- Multipart: `outputPath` (gate `creatable`, as `import_scrivener` checks
  today), then one or more `file` fields. Several files are a folder pick,
  each filename carrying its path inside the folder; a single `.zip` is
  extracted; a single file of any other kind is refused with 400
  "Documents go into a project: open or create one, then use Import File".
- Each file is streamed to a temporary folder outside the projects root.
  Paths are checked piece by piece (no `..`, no absolute or empty part, no
  control characters); zip entries go through the same check (zip-slip).
  Any bad path fails the request with 400 and the folder is removed. The
  `zip` crate (deflate only) becomes a dependency of `crates/web`.
- **Detection** (`chikn_converter::detect_project_kind(staged_root)`, shared
  with the desktop): a `.scrivx` at the top or one folder down → Scrivener;
  otherwise a tree of `.md`/`.markdown`/`.txt` files → Markdown folder (the
  existing `import_markdown_folder`); anything else → 400 "This is not a
  project ChickenScratch can import" before Pandoc or the projects folder is
  touched. Later slices add kinds here only.
- Route body limit 512 MiB (research folders hold PDFs); over it, 413 and
  the UI points at the copy-to-server path, which stays.
- Answers `{ project, warnings }` like `import_scrivener`.

### Slice 2 — the Import button, both shells

- Project screen (`Welcome.tsx`): **Import Scrivener** becomes **Import
  Project**. In the browser it opens a chooser: **Choose the project
  folder** (folder pick; hidden where the browser cannot pick folders,
  which includes iPhone Safari, with a one-line hint on zipping), **Upload
  a zip of it**, **It is already on the server** (the existing server
  folder picker, now for any project folder). On the desktop the native
  dialog picks a folder (or a `.zip`) and the same detector runs on it. Then
  the name prompt as today, then the import; the chooser shows
  "Uploading <size>…" while a browser upload runs.
- Store: `importProject` replaces `importScrivener`, same settle code (open
  the new project, surface warnings).
- User guide: the Scrivener section becomes "Importing a project"; the
  browser section's "Import uploads" bullet and the phone subsection say
  folder on a computer, zip on a phone; troubleshooting covers "too large"
  and "not a project".

### Slice 3 — Import File reads everything Pandoc reads, split optional

Inside a project, both shells:

- The accepted list grows from today's 14 extensions to every Pandoc
  reader that makes sense for prose: adds `typst`, `djot`, `mediawiki`,
  `dokuwiki`, `creole`, `t2t`, `opml`, `ipynb`, `man`, `csv`/`tsv` (as
  tables). `crates/services/src/io.rs` maps each to its reader; the UI list
  and the web upload accept the same set.
- **Legacy `.doc` is not readable by Pandoc** (today `io.rs` maps `doc` to
  the `docx` reader, which fails on a real `.doc` with a confusing error).
  With LibreOffice on the machine, `soffice --headless --convert-to docx`
  runs first; without it, the message is "Save it as .docx first". Apple
  Pages gets the same message.
- **Split, as an option.** After the file is picked, a small dialog offers
  "Split into one document per top-level heading" (checkbox, off by
  default, remembered in settings under Writing). On: `import_file` gains a
  `split_headings: bool`; the converted Markdown is cut at each level-one
  heading, one binder document per heading named by it, in order, text
  before the first heading as "Front matter"; a file with fewer than two
  such headings imports whole with a one-line notice. Off: today's single
  document.

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
exports go in through Import File.

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `crates/web/src/project_upload.rs` (new), `app.rs`, `Cargo.toml` (+ `zip`), `Cargo.lock`, `tests/http.rs` | staging, path checks, zip, detector call, handler, own body limit |
| `crates/cli/src/detect.rs` (new), `lib.rs` | `detect_project_kind` |
| `crates/services/src/project.rs`, `io.rs`; `crates/web/src/commands.rs`; `src-tauri/src/commands/*.rs` | `import_project` (kind-dispatching), wider reader map, `.doc` step, `split_headings` |
| `ui/src/commands/web.ts`, `io.ts`, `project.ts`, `stores/projectStore.ts` | `importProjectUpload`, `importProject`, `split` flag |
| `ui/src/components/welcome/ImportProjectDialog.tsx` (new), `Welcome.tsx`, `editor.css`, `platform.ts` | the chooser; desktop folder-or-zip pick |
| `ui/src/components/binder/Binder.tsx`, a small `ImportFileOptions` dialog | the split checkbox, wider accept list |
| `crates/cli/src/{textbundle,ywriter,novelwriter,manuskript}/` (slice 4, one per slice) | readers + fixtures |
| `docs/USER_GUIDE.md`, `docs/plans/README.md`, `.agents/state.md`, `DEVLOG.md` | records |

## [MODEL] Tests

- [ ] `crates/web` unit: path pieces with `..`, absolute or control
      characters refused; a folder-pick filename lands under the staging
      folder; zip-slip entry refused.
- [ ] `crates/cli` detector: Scrivener at top and one down; Markdown folder;
      a lone `docx` or a folder of PDFs → not a project.
- [ ] `crates/services`: split on: three top-level headings → three
      documents in order plus front matter; one heading → whole file and
      the notice; split off → one document; `.doc` without LibreOffice →
      the save-as message; every new extension maps to a Pandoc reader.
- [ ] `crates/web/tests/http.rs`: zip-slip → 400, nothing written; a lone
      document → 400 with the "use Import File" message; `outputPath`
      outside the root → 403; a Markdown zip → 200 with the documents; a
      two-file `.scriv` folder pick → 200 (Pandoc cases skip with a printed
      line where Pandoc is absent, which is CI).
- [ ] UI: the chooser renders its choices and hides the folder choice when
      unsupported; a folder pick asks for a name and posts every file with
      its relative path to `<root>/<name>.chikn`; the server-path choice
      still works; cancel sends nothing; Import File shows the split
      checkbox off by default and passes it through.
- [ ] Slice 4, each: a fixture project converts to the expected tree.
- [ ] Declared suite (`.agents/repo-guidance.md`), Tauri crate excepted on
      this box (Known drift); CI on push.

## [MODEL] Owner verification (plain English)

1. Browser, computer: **Import Project → Choose the project folder**, pick a
   `.scriv` folder. It uploads without zipping and the converted project
   opens.
2. Browser, phone: zip the Scrivener project in Files (long-press →
   Compress), **Import Project → Upload a zip of it**. Same result.
3. Inside a project, **Import File** with a Word manuscript: it comes in as
   one document. Again with the split box ticked: one document per chapter.
4. Pick a `.doc` from an old Word: a plain message says to save it as `.docx`.
5. **Import Project** with a lone Word file: a plain message says documents
   go in through Import File.

## [YOU] Decisions needed

- **Which app formats in slice 4.** Recommended: TextBundle/TextPack (covers
  Ulysses, Bear, iA Writer exports), yWriter, novelWriter, Manuskript, in
  that order; drop or add names.
- Taken unless told otherwise: upload limit 512 MiB with the copy-to-server
  path kept; the split option is off by default and remembered; Ulysses'
  internal library files are not read (export instead).

## Status

- 2026-10-25: first draft (Scrivener upload only) superseded the same day by
  the owner: "import restricted to scrivener pigeonholes this app". Second
  draft corrected the same day: projects on the project screen, documents
  inside a project, splitting optional. Awaiting the owner's go and the
  slice-4 format list.

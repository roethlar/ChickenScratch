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

> let's keep it .zip, .7z, .tar.gz or some supported archive in both for
> consistency

> for a first pass, let's stick with formats we can easily validate for free

(2026-10-27, answering the decisions below.)

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
browser as on the desktop: the file comes from the device you are using.

**A project uploads as one archive, everywhere.** A project is a folder, and
the writer uploads it as a single archive: `.zip`, `.7z`, `.tar.gz`/`.tgz`
or plain `.tar`. The same rule holds on a computer and on a phone, in the
browser and on the desktop (owner, 2026-10-27: one shape "in both for
consistency"), so there is one chooser, one hint ("compress the project
folder first": right-click → Compress on a Mac, Send to → Compressed folder
on Windows, long-press → Compress in Files on an iPhone) and one server path
to test. The browser's folder picker (`webkitdirectory`) is not used: it is
missing on iPhone Safari and would give the two shells different behaviour.
A project already on the machine the app runs on still goes in through the
existing folder picker, untouched.

## [MODEL] Approach

### Slice 1 — the project door on the server

`POST /api/import-project-upload` (`crates/web/src/project_upload.rs`); the
existing `/api/import-upload` (one document into an open project) stays.

- Multipart: `outputPath` (gate `creatable`, as `import_scrivener` checks
  today), then exactly one `file` field holding an archive: `.zip`, `.7z`,
  `.tar.gz`/`.tgz` or `.tar`, told apart by their magic bytes, not the
  name. Anything else is refused with 400: a document ("Documents go into a
  project: open or create one, then use Import File") or an unknown file
  ("Upload the project folder as a .zip, .7z or .tar.gz archive").
- The archive is streamed to a temporary file, then unpacked into a
  temporary folder outside the projects root by one `unpack_archive`
  function in the converter crate (`crates/cli/src/archive.rs`, shared with
  the desktop so both shells accept the same formats): `zip` (deflate),
  `sevenz-rust` (7z, LZMA/LZMA2, unencrypted), `tar` + `flate2` (tar and
  tar.gz). Every entry path is checked piece by piece (no `..`, no absolute
  or empty part, no control characters, no symlinks or hard links,
  no device nodes) and an unpacked-size ceiling (4 GiB) and entry count
  ceiling (50 000) stop decompression bombs. Any failure removes the
  folder and answers 400 with nothing written. A single top-level folder
  inside the archive (the usual `Corn.scriv/`) is entered automatically.
- **Detection** (`chikn_converter::detect_project_kind(staged_root)`, shared
  with the desktop): a `.scrivx` at the top or one folder down → Scrivener;
  otherwise a tree of `.md`/`.markdown`/`.txt` files → Markdown folder (the
  existing `import_markdown_folder`); anything else → 400 "This is not a
  project ChickenScratch can import" before Pandoc or the projects folder is
  touched. Later slices add kinds here only.
- Route body limit 512 MiB (research folders hold PDFs); over it, 413 and
  the UI points at the on-this-machine folder path, which stays.
- Answers `{ project, warnings }` like `import_scrivener`.

### Slice 2 — the Import button, both shells

- Project screen (`Welcome.tsx`): **Import Scrivener** becomes **Import
  Project**. Both shells open the same chooser with two choices: **Upload
  an archive of the project** (`.zip`, `.7z`, `.tar.gz`; one line under it
  on how to compress a folder on Mac, Windows and iPhone) and **It is
  already on this machine** (the existing folder picker, now for any
  project folder). On the desktop the archive choice uses the native file
  dialog filtered to those extensions and unpacks with the same
  `unpack_archive`; the detector then runs on the unpacked folder exactly as
  on the server. Then the name prompt as today, then the import; the
  chooser shows "Uploading <size>…" while a browser upload runs.
- Store: `importProject` replaces `importScrivener`, same settle code (open
  the new project, surface warnings).
- User guide: the Scrivener section becomes "Importing a project"; the
  browser section's "Import uploads" bullet and the phone subsection say
  "compress the project folder, then upload the archive" with the three
  how-to lines; troubleshooting covers "too large", "not a project" and
  "not an archive we read" (encrypted or RAR archives: re-save as zip).

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

### Slice 4 — other writing apps' project formats (one slice each; first pass: free apps only)

Each is a reader in `crates/cli/src/<format>/` producing the same
`ImportReport` the Scrivener reader produces, plus a detector entry. The
first pass (owner, 2026-10-27) takes only formats whose producing app is
free, so a sample project can be made in the real app on this Linux box and
the import checked against what the app shows:

- **novelWriter** (free, open source, runs on Linux; folder:
  `nwProject.nwx` XML tree + `content/*.nwd` Markdown-like files).
- **Manuskript** (free, open source, runs on Linux; `.msk` zip or folder
  with a `MANUSKRIPT` marker: outline as text files, characters and world as
  YAML-ish text).
- **TextBundle / TextPack** (`.textbundle` folder, `.textpack` zip:
  `info.json`, `text.md`, `assets/`): the interchange format Ulysses, Bear
  and iA Writer export. Validated for free with Zettlr (free, open source),
  which exports TextBundle/TextPack; a folder of bundles is a project.

Deferred past the first pass: **yWriter** (`.yw7`; the app is free but
Windows-only, so checking needs Mono or Wine here). Ulysses' own library
files (`.ulyz`) are undocumented and change between versions: not
supported; writers export TextBundle or Markdown instead. Storyist, Papyrus
Author, Vellum, Dabble, Novlr, Atticus and Campfire are closed or
cloud-only: their Word or Markdown exports go in through Import File.

Each slice needs a small sample project made in the app for its test; exact
file layouts are confirmed against the app's documentation or source before
coding.

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `crates/web/src/project_upload.rs` (new), `app.rs`, `tests/http.rs` | streaming to a temp file, unpack, detector call, handler, own body limit |
| `crates/cli/src/archive.rs` (new), `detect.rs` (new), `lib.rs`, `Cargo.toml` (+ `zip`, `sevenz-rust`, `tar`, `flate2`), `Cargo.lock` | `unpack_archive` (format sniffing, path and size checks), `detect_project_kind` |
| `crates/services/src/project.rs`, `io.rs`; `crates/web/src/commands.rs`; `src-tauri/src/commands/*.rs` | `import_project` (kind-dispatching), wider reader map, `.doc` step, `split_headings` |
| `ui/src/commands/web.ts`, `io.ts`, `project.ts`, `stores/projectStore.ts` | `importProjectUpload`, `importProject`, `split` flag |
| `ui/src/components/welcome/ImportProjectDialog.tsx` (new), `Welcome.tsx`, `editor.css`, `platform.ts` | the chooser; desktop archive pick |
| `ui/src/components/binder/Binder.tsx`, a small `ImportFileOptions` dialog | the split checkbox, wider accept list |
| `crates/cli/src/{novelwriter,manuskript,textbundle}/` (slice 4, one per slice) | readers + fixtures made in the apps |
| `docs/USER_GUIDE.md`, `docs/plans/README.md`, `.agents/state.md`, `DEVLOG.md` | records |

## [MODEL] Tests

- [ ] `crates/cli` archive: each of zip, 7z, tar.gz and tar unpacks a
      two-file fixture to the same tree; sniffing ignores a wrong
      extension; entries with `..`, absolute paths, control characters,
      symlinks or hard links are refused and nothing is left on disk; the
      size and entry-count ceilings stop a small bomb; an encrypted 7z and a
      RAR are refused with the "re-save as zip" message; a single top-level
      folder is entered.
- [ ] `crates/cli` detector: Scrivener at top and one down; Markdown folder;
      a lone `docx` or a folder of PDFs → not a project.
- [ ] `crates/services`: split on: three top-level headings → three
      documents in order plus front matter; one heading → whole file and
      the notice; split off → one document; `.doc` without LibreOffice →
      the save-as message; every new extension maps to a Pandoc reader.
- [ ] `crates/web/tests/http.rs`: zip-slip → 400, nothing written; a lone
      document → 400 with the "use Import File" message; an unknown file →
      400 with the "upload an archive" message; `outputPath` outside the
      root → 403; a Markdown tar.gz → 200 with the documents; a `.scriv`
      zip → 200 (Pandoc cases skip with a printed line where Pandoc is
      absent, which is CI).
- [ ] UI: the chooser renders its two choices; an archive pick asks for a
      name and posts the one file to `<root>/<name>.chikn`; the
      on-this-machine choice still works; cancel sends nothing; Import File
      shows the split checkbox off by default and passes it through.
- [ ] Slice 4, each: a fixture project converts to the expected tree.
- [ ] Declared suite (`.agents/repo-guidance.md`), Tauri crate excepted on
      this box (Known drift); CI on push.

## [MODEL] Owner verification (plain English)

1. Browser, computer: compress a `.scriv` folder (right-click → Compress),
   **Import Project → Upload an archive of the project**, pick the zip. The
   converted project opens.
2. Browser, phone: compress the same project in Files (long-press →
   Compress) and upload it the same way. Same result. A `.7z` or `.tar.gz`
   of the folder works too.
3. Inside a project, **Import File** with a Word manuscript: it comes in as
   one document. Again with the split box ticked: one document per chapter.
4. Pick a `.doc` from an old Word: a plain message says to save it as `.docx`.
5. **Import Project** with a lone Word file: a plain message says documents
   go in through Import File.

## [YOU] Decisions

- **Settled 2026-10-27 — upload shape:** a project uploads as one archive
  (`.zip`, `.7z`, `.tar.gz`/`.tgz`, `.tar`) on every device and in both
  shells; no browser folder picker. The on-this-machine folder path stays.
- **Settled 2026-10-27 — slice 4 first pass:** only formats a free app can
  produce for checking: novelWriter, Manuskript, TextBundle (via Zettlr).
  yWriter deferred; Ulysses library files not read (export instead).
- Taken unless told otherwise: upload limit 512 MiB; unpacked ceiling 4 GiB
  and 50 000 entries; encrypted archives refused rather than prompting for
  a password; the split option is off by default and remembered.
- **Open:** the go to begin, and whether the plan gets a Codex review
  (gpt-6-astra, xhigh) before slice 1 as the fidelity plan did
  (recommended: yes, same reviewer for plan and slices).

## Status

- 2026-10-25: first draft (Scrivener upload only) superseded the same day by
  the owner: "import restricted to scrivener pigeonholes this app". Second
  draft corrected the same day: projects on the project screen, documents
  inside a project, splitting optional. Awaiting the owner's go and the
  slice-4 format list.
- 2026-10-27: owner answered both decisions: one archive shape everywhere
  (zip, 7z, tar.gz) instead of a browser folder pick; slice 4 limited to
  formats a free app can produce (novelWriter, Manuskript, TextBundle via
  Zettlr; yWriter deferred). Plan revised accordingly. Awaiting the go.

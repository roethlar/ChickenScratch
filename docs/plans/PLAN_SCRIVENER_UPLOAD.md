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
disk: every import builds the project under a temporary name and renames it
into place only on success, R1-1); I7 untouched.

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
to test. The browser's folder picker (`webkitdirectory`) is not used, by
the owner's consistency ruling, not for want of support (iPhone Safari has
had it since 18.4, March 2025; a folder pick would still give the browser
and the desktop two different shapes). A project already on the machine the
app runs on still goes in through the existing folder picker, untouched.

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
  `sevenz-rust2` (the maintained fork of `sevenz-rust`; 7z, LZMA/LZMA2,
  unencrypted), `tar` + `flate2` (tar and tar.gz). Everything the archive
  says about itself is checked before it is believed:
  - **Entry paths** (R1-4): the name is split on both `/` and `\`; every
    piece must be non-empty, not `.` or `..`, free of control characters
    and of `:`, `*`, `?`, `"`, `<`, `>`, `|`, not ending in a dot or
    space, and not a Windows device name (`CON`, `PRN`, `AUX`, `NUL`,
    `COM1`–`COM9`, `LPT1`–`LPT9`, with or without an extension). The
    pieces are joined under the staging folder ourselves, never by pushing
    the raw name (a `C:` prefix or a leading `\` would replace the base).
    Two entries whose normalised paths collide case-insensitively are
    refused, so the result is the same on Linux and Windows and nothing is
    overwritten.
  - **Entry kinds**: only files and directories. Tar entry types for
    symbolic links, hard links, devices and FIFOs are refused; zip entries
    whose Unix mode (external attributes) says symlink are refused; 7z
    entries with the Unix-extension attribute and a symlink mode, or the
    Windows reparse-point attribute, are refused (7z has no hard links).
  - **Decoder allocations** (R1-3): the crates allocate from header fields
    before any entry is yielded, so counting extracted bytes is not enough.
    7z: the 32-byte signature header is read first; `NextHeaderSize` above
    16 MiB, or an offset outside the file, is refused before the crate
    parses anything; after the parse, each coder's dictionary size (LZMA
    5-byte properties, LZMA2 1-byte code) is read from the archive's
    folder/coder records and refused above 256 MiB (7-Zip's "ultra" preset
    uses 64 MiB). Tar: a header-only walk of the stream (512-byte headers,
    sizes from the size field) runs first and refuses any long-name, PAX or
    global-header entry (`L`, `K`, `x`, `g`) above 1 MiB and any declared
    total above the ceiling; only then does the `tar` crate read it (gzip is
    streamed twice, which is cheap). Zip: the central directory is read by
    the crate, but the entry count is capped first from the end-of-central-
    directory record. Unpacked bytes including metadata are capped at 4 GiB
    and entries at 50 000; one unpack runs at a time per process; a 120 s
    wall-clock bound aborts a slow decoder. The 512 MiB body limit bounds
    the compressed input. If `sevenz-rust2` turns out not to expose the
    coder records, the dictionary check reads them from the header bytes
    ourselves; it is not dropped.
  - Any failure removes the staging folder and answers 400 with nothing
    written.
- **Detection** (R1-6, R1-12; `chikn_converter::detect_project(staged_root)
  -> Detected { kind, source_root }`, shared with the desktop). Before
  anything else, a zip that is really a document container is recognised
  (`[Content_Types].xml` → Word/OOXML; a `mimetype` entry naming
  OpenDocument or EPUB) and answered 400 "Documents go into a project: open
  or create one, then use Import File". Then operating-system droppings
  (`__MACOSX/`, `.DS_Store`, `Thumbs.db`, `desktop.ini`) are ignored and a
  single remaining top-level folder is entered, repeatedly, so
  `Corn.scriv/` beside `__MACOSX/` still resolves to `Corn.scriv`. A
  `.scrivx` in the resolved root → Scrivener with that root; otherwise a
  tree holding `.md`/`.markdown`/`.txt` files → Markdown folder; two or
  more candidate projects → 400 "This archive holds more than one
  project"; anything else → 400 "This is not a project ChickenScratch can
  import". The converter receives `source_root`, never the staging root.
  Later slices add kinds here only.
- **One transaction for every project import** (R1-1):
  `services::import_project` builds the project at
  `<outputPath>.importing-<random>` beside the final path, and renames it
  to `outputPath` only when the converter returned; any error removes the
  temporary project and the staging folder, so a failed or refused import
  leaves nothing on disk and the same destination can be tried again. A
  destination that already exists is refused up front (`creatable`) and is
  never touched or removed. Both existing importers (`import_scrivener`,
  `import_markdown_folder`) move under this wrapper.
- **The Markdown-folder importer learns the whole tree** (R1-2): today it
  reads only the top-level files and prints skipped files to stderr. It
  becomes recursive and deterministic (folders → binder folders, entries
  sorted by name, duplicate document names disambiguated by suffix) and
  returns an `ImportReport` whose warnings name every skipped file, so the
  browser shows them; a tree with no readable document fails the import.
- Route body limit 512 MiB (research folders hold PDFs); over it, 413 and
  the UI points at the on-this-machine folder path, which stays.
- Answers `{ project, warnings }` like `import_scrivener`.

### Slice 2 — the Import button, both shells

- Project screen (`Welcome.tsx`): **Import Scrivener** becomes **Import
  Project**. Both shells open the same chooser with two choices: **Upload
  an archive of the project** (`.zip`, `.7z`, `.tar.gz`, `.tgz`, `.tar`,
  one suffix list shared by both shells and their hints, R1-8; one line
  under it on how to compress a folder on Mac, Windows and iPhone) and **It
  is already on this machine** (the existing folder picker, now for any
  project folder). On the desktop the archive choice uses the native file
  dialog filtered to that same list and unpacks with the same
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

- **One registry of import formats** (R1-7), `crates/services/src/
  formats.rs`: filename extension → Pandoc reader, used by the services'
  validation, the web upload route and both choosers (the UI reads a
  generated `ui/src/importFormats.json`; a services test asserts the file
  equals the registry, so the three lists cannot drift). Today's 15 UI
  extensions are kept and corrected: `.html`/`.htm` go to the `html` reader
  (today they fall through to Markdown and keep their tags). Added, with
  real extensions rather than reader names: `.typ` (typst), `.dj` (djot),
  `.wiki`/`.mediawiki` (mediawiki), `.adoc`/`.asciidoc` (asciidoc; the
  installed Pandoc 3.11 lists the reader), `.dbk` (docbook), `.creole`,
  `.t2t`, `.opml`,
  `.ipynb`, `.man`, `.muse`, `.pod`, `.csv`/`.tsv` (as tables). Ambiguous
  `.xml` and extension-less DokuWiki pages stay out. `.doc` leaves the
  accepted list and takes the LibreOffice step below.
- **Legacy `.doc` is not readable by Pandoc** (today `io.rs` maps `doc` to
  the `docx` reader, which fails on a real `.doc` with a confusing error).
  With LibreOffice on the machine, `soffice --headless --convert-to docx`
  runs first; without it, the message is "Save it as .docx first". Apple
  Pages gets the same message.
- **Split, as an option.** After the file is picked, a small dialog offers
  "Split into one document per top-level heading" (checkbox, off by
  default, remembered in settings under Writing). On: `import_file` gains a
  `split_headings: bool`, and the split happens in Pandoc's own document
  model, not on the Markdown text (R1-5): the file is read to Pandoc JSON,
  the block list is cut at each level-one `Header`, and each part is
  written back with `pandoc -f json -t markdown`. Footnotes are inline
  `Note` elements in that model and reference links are already resolved,
  so each chapter carries its own notes and link targets; a `#` line inside
  a code fence or a Setext heading is whatever the parser says it is. One
  binder document per heading named by it, in order, text before the first
  heading as "Front matter"; a file with fewer than two such headings
  imports whole with a one-line notice. Off: today's single document.

### Slice 4 — other writing apps' project formats (one slice each; first pass: free apps only)

Each is a reader in `crates/cli/src/<format>/` producing the same
`ImportReport` the Scrivener reader produces, plus a detector entry. The
first pass (owner, 2026-10-27) takes only formats whose producing app is
free, so a sample project can be made in the real app on this Linux box and
the import checked against what the app shows:

- **novelWriter** (free, open source, runs on Linux). Current releases
  (26.x) keep the tree in `nwProject.nwx` (XML) and each document in
  `content/<handle>.md` with a TOML header between `+++` lines (name,
  parent, handle, class, layout, hashes, dates) followed by the text
  (R1-9); the 2.x layout used `content/<handle>.nwd` with `%%~name:`
  header lines. The first pass reads the current layout from a fixture
  made in the installed app; a `.nwd` project is read too when the header
  grammar is confirmed from the project's open source, otherwise refused
  with "open and save this project in a current novelWriter first".
- **Manuskript** (free, open source, runs on Linux). A `.msk` file holding
  the format number beside a folder of the same name, or one zip of that
  folder, with a `MANUSKRIPT` marker (R1-10): `infos.txt` and
  `summary.txt` (key/value), `outline/` (folders with `folder.txt`, texts
  as `<id>-<slug>.md` with key/value lines, two blank lines, then
  Markdown), `characters/<id>-<slug>.txt` (key/value), `world.opml` (OPML
  XML, nested `outline` elements) and `plots.xml`. The outline becomes the
  manuscript; characters, world entries and plots become Research notes.
- **TextBundle / TextPack** (`.textbundle` folder, `.textpack` zip:
  `info.json`, one `text.*` file whose type `info.json` declares, and
  `assets/`): the interchange format Ulysses, Bear and iA Writer export.
  The text file is found by name stem, not a fixed `text.md` (R1-11):
  Markdown (`text.md`, `text.markdown`, `net.daringfireball.markdown`) and
  plain text are read; another declared type is skipped with a warning.
  Assets are copied into the project as attachments through core and their
  `assets/…` references rewritten. Validated for free with Zettlr (free,
  open source), which exports TextBundle/TextPack; a folder of bundles is a
  project.

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
| `crates/cli/src/archive.rs` (new), `detect.rs` (new), `lib.rs`, `Cargo.toml` (+ `zip`, `sevenz-rust2`, `tar`, `flate2`), `Cargo.lock` | `unpack_archive` (format sniffing, header pre-checks, path/kind/size checks), `detect_project` |
| `crates/services/src/project.rs`, `io.rs`, `formats.rs` (new); `crates/web/src/commands.rs`; `src-tauri/src/commands/*.rs` | `import_project` (temporary destination + rename, kind dispatch), recursive Markdown-folder import with warnings, the format registry, `.doc` step, AST `split_headings` |
| `ui/src/commands/web.ts`, `io.ts`, `project.ts`, `stores/projectStore.ts`, `ui/src/importFormats.json` (generated) | `importProjectUpload`, `importProject`, `split` flag, shared format and archive suffix lists |
| `ui/src/components/welcome/ImportProjectDialog.tsx` (new), `Welcome.tsx`, `editor.css`, `platform.ts` | the chooser; desktop archive pick |
| `ui/src/components/binder/Binder.tsx`, a small `ImportFileOptions` dialog | the split checkbox, wider accept list |
| `crates/cli/src/{novelwriter,manuskript,textbundle}/` (slice 4, one per slice) | readers + fixtures made in the apps |
| `docs/USER_GUIDE.md`, `docs/plans/README.md`, `.agents/state.md`, `DEVLOG.md` | records |

## [MODEL] Tests

- [ ] `crates/cli` archive: each of zip, 7z, tar.gz, tgz and tar unpacks a
      two-file fixture to the same tree; sniffing ignores a wrong
      extension; entries named with `..`, a leading `/` or `\`, a `C:`
      prefix, control characters, a trailing dot, a device name, or two
      names differing only in case are refused and nothing is left on disk;
      tar symlink, hard-link and device entries, a zip symlink and a 7z
      symlink are refused; the size and entry-count ceilings stop a small
      expansion bomb; a 7z whose signature header declares a 100 MiB
      header, a 7z with a 1 GiB dictionary, and a tar with a 16 MiB PAX
      header are refused before any large allocation (asserted with a
      memory ceiling on the test process); an encrypted 7z and a RAR are
      refused with the "re-save as zip" message; a single top-level folder
      is entered.
- [ ] `crates/cli` detector, run through conversion: Scrivener at top, one
      down, and beside `__MACOSX/`; a Markdown tree whose documents are all
      below the root; two `.scriv` folders → "more than one project"; a real
      `.docx`, an `.odt` and an `.epub` (zip containers) → the "use Import
      File" message; a folder of PDFs → not a project.
- [ ] `crates/services` import transaction: a converter error after the
      project was created leaves nothing at `outputPath` and no staging
      folder, and a second attempt at the same path succeeds; an existing
      destination is refused and left byte-for-byte untouched. Markdown
      folder: nested folders become binder folders in sorted order;
      duplicate names are disambiguated; one unreadable file among three is
      reported in the warnings and the other two import; a tree with no
      readable file fails.
- [ ] `crates/services` split: three top-level headings → three documents
      in order plus front matter; a footnote referenced in chapter one and
      defined after chapter two stays in chapter one; a reference-style
      link shared by two chapters resolves in both; a `# ` line inside a
      fenced code block and a Setext heading are treated as the parser
      says; one heading → whole file and the notice; split off → one
      document; `.doc` without LibreOffice → the save-as message. Format
      registry: every extension maps to a reader Pandoc lists; `.html`
      converts a heading and emphasis to Markdown; `ui/src/importFormats.
      json` equals the registry.
- [ ] `crates/web/tests/http.rs`: zip-slip → 400, nothing written; a lone
      Markdown document → 400 with the "use Import File" message; a real
      `.docx` → the same message; an unknown file → 400 with the "upload an
      archive" message; `outputPath` outside the root → 403; a Markdown
      tar.gz → 200 with the documents and warnings; a `.scriv` zip → 200
      (Pandoc cases skip with a printed line where Pandoc is absent, which
      is CI).
- [ ] UI: the chooser renders its two choices and accepts every suffix in
      the shared list, `.tgz` and `.tar` included; an archive pick asks for
      a name and posts the one file to `<root>/<name>.chikn`; the
      on-this-machine choice still works; cancel sends nothing; Import File
      offers every extension in `importFormats.json` and shows the split
      checkbox off by default and passes it through.
- [ ] Slice 4, each: a fixture project converts to the expected tree.
- [ ] Declared suite (`.agents/repo-guidance.md`), Tauri crate excepted on
      this box (Known drift); CI on push.

## [MODEL] Owner verification (plain English)

1. Browser, computer: compress a `.scriv` folder (right-click → Compress),
   **Import Project → Upload an archive of the project**, pick the zip. The
   converted project opens.
2. Browser, phone: compress the same project in Files (long-press →
   Compress) and upload it the same way. Same result. A `.7z`, `.tar.gz`
   or `.tar` of the folder works too, and a zip that also holds the Mac's
   `__MACOSX` folder still imports.
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
  and 50 000 entries; 7z header above 16 MiB or dictionary above 256 MiB
  refused; encrypted archives refused rather than prompting for a
  password; the split option is off by default and remembered; the
  dependencies (`zip` MIT, `sevenz-rust2` Apache-2.0, `tar` and `flate2`
  MIT/Apache-2.0) are licence-compatible with this MIT repository.
- Settled 2026-10-27: the go ("set the push policy to always, push, then
  go"); plan reviewed by Codex gpt-6-astra xhigh before slice 1, slices
  reviewed by the same reviewer (`.agents/review/findings/plan-4.md`).

## Status

- 2026-10-25: first draft (Scrivener upload only) superseded the same day by
  the owner: "import restricted to scrivener pigeonholes this app". Second
  draft corrected the same day: projects on the project screen, documents
  inside a project, splitting optional. Awaiting the owner's go and the
  slice-4 format list.
- 2026-10-27: owner answered both decisions: one archive shape everywhere
  (zip, 7z, tar.gz) instead of a browser folder pick; slice 4 limited to
  formats a free app can produce (novelWriter, Manuskript, TextBundle via
  Zettlr; yWriter deferred). Plan revised accordingly. Owner go the same
  day.
- 2026-10-27 review round 1 (`plan-4`, codex gpt-6-astra xhigh,
  `4b30022`): reopened, thirteen findings, all admitted after checking
  against the code and the cited sources: imports left a half-made project
  on failure (R1-1: temporary destination + rename); the Markdown-folder
  importer read only top-level files and hid skips in stderr (R1-2);
  archive header fields drive allocations before extraction (R1-3: header
  pre-checks, dictionary cap, tar header walk, `sevenz-rust2`); Windows
  path forms and case collisions (R1-4); heading splits lost footnotes
  (R1-5: split in Pandoc's model); detection must return the project root
  and handle `__MACOSX` and multiple candidates (R1-6); reader names were
  used as extensions, `.html` fell through to Markdown, the list has 15
  entries (R1-7: one registry); chooser filters missed `.tgz`/`.tar`
  (R1-8); novelWriter now writes `content/<handle>.md` with TOML headers
  (R1-9); Manuskript's world is OPML (R1-10); TextBundle's text file is
  `text.*` (R1-11); a `.docx` is a zip and needs the document message
  (R1-12); the iPhone Safari claim was out of date (R1-13).

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
converter and import services, plus three small engine additions it needs
(a project-publication transaction in core, compile finding project images
and resolving links between documents, a Markdown-bodied footnote element
and an anchor element understood by compile); no format change
[x] Not paused

**Invariants touched:** I2 (uploads are staged in a temporary folder and
handed to the existing converter and services, which write through core;
new project-format readers live in the converter crate, never in the UI;
the `.chikn` publication step itself is a core API, R3-3); I6 (a refused or
failed import leaves no half-made project and nothing on disk: every import
builds the project under a temporary name and core renames it into place
only on success, R1-1); I7 untouched.

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
  unencrypted), `tar` + `flate2` (tar and tar.gz).
- **Unpacking runs in a worker process the caller can kill** (R2-3, R2-5).
  The decoders allocate from header fields before any entry is yielded
  (7z decodes an *encoded header* inside `Archive::read`; tar reads PAX
  bodies), so no check made from inside the process can run before those
  allocations, and a timed-out `spawn_blocking` task cannot be stopped.
  `chikn_converter::unpack::run(archive, dest, limits)` therefore
  re-invokes the current executable (`std::env::current_exe()`) with a
  hidden `--unpack-worker` argument that every binary honours first thing
  in `main` (web server, Tauri shell, converter CLI). The worker lives in
  the converter crate, below `services` (R3-1: `services` already depends
  on the converter, so a worker in `services` called from the converter CLI
  would be a dependency cycle); `services` supervises it. The parent owns
  the admission slot (one unpack at a time per process), waits with a 120 s
  wall clock, kills the child on timeout or when the HTTP request is
  cancelled (the request future's drop guard kills and waits), removes the
  staging folder, and releases the slot only after the child has exited.
  **The worker's memory is bounded by its own allocator, on every
  platform** (R3-2, R4-1, R5-1). The decoders are pure Rust (`zip`,
  `sevenz-rust2`, `tar`, `flate2` with its default `miniz_oxide` backend),
  so every byte they take comes through Rust's global allocator. In worker
  mode the binary installs a counting `#[global_allocator]` wrapper around
  the system allocator (`chikn_converter::unpack::BudgetAlloc`): a budget
  of 1 GiB is armed as the worker's first act, every allocation adds its
  size to an atomic counter, every free subtracts it, and an allocation
  that would cross the budget returns null: an infallible allocation then
  aborts through `handle_alloc_error`, a fallible one (`try_reserve`)
  returns an error the decoder surfaces; the parent treats every
  unsuccessful child outcome alike and removes the staging folder. The
  bound is claimed only once the archive crates are locked and a test shows
  the counter moving while each decoder runs, with the resolved dependency
  graph checked for any allocation path that bypasses Rust's global
  allocator (C code, `mmap`); a decoder that cannot be shown to allocate
  through the wrapper is dropped from the accepted formats rather than
  shipped with an unproven bound. Outside worker mode the budget is disarmed and the wrapper is a
  pass-through, so the application's own memory use is unaffected. The
  budget is enforced *before* the allocation happens, so there is no
  overshoot window and no sampling gap. Supplementary, per platform: Linux
  additionally sets `setrlimit(RLIMIT_AS, 1 GiB)` after `exec` (its own
  baseline is a few tens of MiB; on macOS that call cannot be used because
  every process maps the 6 GiB shared region), and Windows runs the child
  inside a Job Object with a 1 GiB process-memory limit, created suspended
  through `CreateProcessW` (the `windows` crate, already a Tauri dependency;
  stable `std::process` cannot hand back the primary thread) and resumed
  after assignment. Every binary performs the worker dispatch in a plain
  synchronous `main` before any runtime starts (the web server's
  `#[tokio::main]` becomes an explicit runtime built after the dispatch).
  The child reports the outcome as one JSON line on stdout, including its
  peak counted allocation; a killed or crashed child is a refusal with
  "This archive could not be unpacked safely". The in-process checks below
  are defence in depth, not the only line. macOS and Windows behaviour cannot be run on
  the development machine (Linux); the slice is not called done until the
  worker has unpacked a real archive and refused a bomb on each, in CI's
  Tauri Bundles jobs or by the owner, and the record says which.
  - **Entry paths** (R1-4, R2-6, R2-7): the name is split on both `/` and
    `\`. A leading `./` is dropped and a trailing separator marks a
    directory entry (so `Book/`, `./` and `./chapter.md` from ordinary
    archivers are accepted). After that, every piece must be non-empty, not
    `.` or `..`, free of control characters and of `:`, `*`, `?`, `"`, `<`,
    `>`, `|`, not ending in a dot or space, and not a Windows device name
    (`CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`, `LPT1`–`LPT9`, with or
    without an extension). The pieces are joined under the staging folder
    ourselves, never by pushing the raw name (a `C:` prefix or a leading
    `\` would replace the base). A case-folded registry of every path
    *and every ancestor directory*, explicit or implied, refuses two
    spellings of the same name (`A/one.md` beside `a/two.md`) and a file
    and a directory of the same name, so the result is the same on Linux
    and Windows and nothing is overwritten.
  - **Entry kinds**: only files and directories. Tar entry types for
    symbolic links, hard links, devices and FIFOs are refused; zip entries
    whose Unix mode (external attributes) says symlink are refused; 7z
    entries with the Unix-extension attribute and a symlink mode, or the
    Windows reparse-point attribute, are refused (7z has no hard links);
    7z anti-items are refused.
  - **Header sanity** (R1-3, R2-4): 7z: the 32-byte signature header is
    read before the crate parses anything; `NextHeaderSize` above 16 MiB,
    or an offset outside the file, is refused; after the parse each
    content coder's dictionary size (LZMA 5-byte properties, LZMA2 1-byte
    code, from the block's coder records) is refused above 256 MiB. Tar: a
    header-only walk of the stream runs first; it parses PAX and global
    extension headers (`x`, `g`) itself and accepts a `size` record only
    when it equals the following member's raw size field (ordinary
    archivers write such records for small files too, R3-7), refusing a
    divergent one so the walk and the `tar` crate cannot disagree about
    where an entry ends; it refuses any `L`/`K`/`x`/`g` body above 1 MiB
    and any declared total above the ceiling; only then does the `tar`
    crate read it (gzip is streamed twice, which is cheap). Zip: the entry count is read from
    the end-of-central-directory record actually selected by the crate
    (ZIP64 included) and refused above the ceiling before the directory is
    read. Unpacked bytes including metadata are capped at 4 GiB and entries
    at 50 000. The 512 MiB body limit bounds the compressed input.
  - Any failure removes the staging folder and answers 400 with nothing
    written.
- **Detection** (R1-6, R1-12, R2-8; `chikn_converter::detect_project(
  staged_root) -> Detected { kind, source_root }`, shared with the
  desktop). In order:
  1. A zip that is really a document container (`[Content_Types].xml` →
     Word/OOXML; a `mimetype` entry naming OpenDocument or EPUB) → 400
     "Documents go into a project: open or create one, then use Import
     File".
  2. Operating-system droppings (`__MACOSX/`, `.DS_Store`, `Thumbs.db`,
     `desktop.ini`) are ignored everywhere below.
  3. **Strong markers are discovered first, anywhere in the tree**, by
     walking the complete entry list the unpacker already holds (at most
     50 000 entries, so no depth limit is needed and "no marker anywhere"
     is a real statement, R3-6): a `.scrivx` file (Scrivener, root = its
     folder); later
     slices add `nwProject.nwx` (novelWriter), a `MANUSKRIPT` file
     (Manuskript) and `info.json` beside a `text.*` file (TextBundle).
     Exactly one marker → that project with that root, whatever else sits
     beside it (`Corn.scriv/` next to a `README.md` still imports the
     Scrivener project). Two or more markers, nested or siblings → 400
     "This archive holds more than one project" (slice 4 defines the one
     exception: a folder holding only TextBundles is one project).
  4. Only when no marker exists anywhere: a tree holding `.md`/
     `.markdown`/`.txt` files → Markdown folder, rooted at the single
     meaningful top-level folder if there is one, else at the staging
     root.
  5. Anything else → 400 "This is not a project ChickenScratch can
     import".
  The converter receives `source_root`, never the staging root. Later
  slices add kinds in step 3 only.
- **One transaction for every project import** (R1-1, R2-1, R2-2, R3-3,
  R3-4, R3-5). `services::import_project` owns admission and
  orchestration: it takes a process-wide lock on the final path, checks
  that nothing exists there (the web gate's `creatable` only authorises the
  root: it returns an existing path unchanged, so the service makes the
  existence check itself; an empty folder, a file, a dangling symlink and
  an existing project are all refused and never touched), and has the
  converter build the project at `<outputPath>.importing-<random>` beside
  the final path. Publication is a core API,
  `core::project::publish_imported(temp, final) -> Published`, because
  moving and removing a `.chikn` folder is `.chikn` I/O (I2): it reads the
  finished project at the temporary path first (every fallible read
  happens before the commit point), then renames with a call that cannot
  replace an existing entry (`renameat2(RENAME_NOREPLACE)` on Linux,
  `MoveFileExW` without `REPLACE_EXISTING` on Windows, `renamex_np(
  RENAME_EXCL)` on macOS). Where the storage refuses the exclusive rename
  (`EINVAL`/`ENOTSUP`, some network filesystems), the import fails safely
  with "This projects folder's storage cannot publish a project safely;
  choose another folder" and the temporary project is removed; there is no
  plain-rename fallback (R3-4). The rename is the commit point: after it,
  `Published` carries the already-read project with its `path` rewritten
  to the final folder and the write token is re-acquired there; if that
  re-acquisition fails, the result is a truthful
  `PublishedNotOpened { path, reason }` rather than a failure that would
  leave a published project behind a "failed" message (R3-5). That outcome
  is a **tagged result all the way to the screen** (R4-2):
  `services::ImportOutcome = Opened { project, warnings } |
  PublishedNotOpened { path, reason, warnings }`, carried as a tagged JSON
  object by the web route and the Tauri command, and handled by one store
  function in the UI: `Opened` runs today's settle code; `PublishedNotOpened`
  opens no editor, adds the path to recent projects so the project is
  reachable from the project screen, and shows "Imported to <path>, but it
  could not be opened: <reason>. Open it from the project list." No layer
  reaches into `project.name` on that branch. Any error *before* the commit
  point removes the temporary project and the staging folder, so a failed
  or refused import leaves nothing on disk and the same destination can be
  tried again. The embedded `.git` moves with the folder; nothing in it
  records the absolute path. Both existing importers (`import_scrivener`,
  `import_markdown_folder`) move under this wrapper.
- **The Markdown-folder importer learns the whole tree** (R1-2): today it
  reads only the top-level files and prints skipped files to stderr. It
  becomes recursive and deterministic (folders → binder folders, entries
  sorted by name, duplicate document names disambiguated by suffix) and
  returns an `ImportReport` whose warnings name every skipped file, so the
  browser shows them; a tree with no readable document fails the import.
- Route body limit 512 MiB (research folders hold PDFs); over it, 413 and
  the UI points at the on-this-machine folder path, which stays.
- Answers the tagged `ImportOutcome` above (`Opened` carries `{ project,
  warnings }` like `import_scrivener` today).

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
- Store: `importProject` replaces `importScrivener`; it switches on the
  tagged outcome (`Opened` → today's settle code: open the new project,
  surface warnings; `PublishedNotOpened` → recents entry and the message
  above, nothing opened).
- User guide: the Scrivener section becomes "Importing a project"; the
  browser section's "Import uploads" bullet and the phone subsection say
  "compress the project folder, then upload the archive" with the three
  how-to lines; troubleshooting covers "too large", "not a project" and
  "not an archive we read" (encrypted or RAR archives: re-save as zip).

### Slice 3 — what imports create survives the editor and compile (lands before slice 4)

Four gaps the reviews found are not in the importers but in what receives
their output (R3-8, R3-10, R3-11, R3-12). Without this slice the promises
of slice 4 would be broken the first time a writer saved or compiled, so it
lands first. The owner ruled (2026-10-27) that links between documents must
work in compiled output for the import to ship as a feature, so the fourth
gap is in scope here, with the design below.

- **Images in the editor** (R3-11, R4-6). The editor registers no image
  node, so a Markdown image is dropped the moment a document is saved. Both
  shells gain an image node (`@tiptap/extension-image`, serialised back to
  `![alt](path)` by `tiptap-markdown`), and a resolver that turns a
  project-relative target (`research/figure.png`) into something the shell
  can display. Desktop: Tauri's built-in asset protocol cannot do this
  safely, because its scope is additive (an allow entry cannot be removed,
  and a `forbid` outlives a later `allow`, so a project closed and reopened
  in one session would lose its images, R5-6). The shell instead registers
  its own URI scheme handler (`register_uri_scheme_protocol("chikn-asset",
  …)`) that answers each request by checking, at request time, the
  currently open project held in shared state: the path must resolve inside
  that project's folder, else 403; closing the project makes every URL
  refuse at once, reopening makes it work again, with no scope mutation.
  The CSP `img-src` lists `chikn-asset:` and both `http://chikn-asset.
  localhost` and `https://chikn-asset.localhost` (the origins the two
  platforms use), and the resolver builds those URLs. Web:
  `GET /api/asset?project=…&path=…`, gated to files inside the open project
  (same gate as the other project routes).
  Imported images therefore display, survive an edit and a save, and
  reopen. This also fixes the same loss for Markdown a writer brings in
  today. The desktop part is verified in the built shell (CI's Tauri
  Bundles or the owner), since this machine does not build the Tauri crate.
- **Anchors and document links the editor keeps** (R4-4). A new
  `AnchorNode` (inline atom, `<span class="anchor" id="…"></span>`, parsed
  and serialised as that HTML exactly as `FootnoteNode` does) preserves an
  identifier through open, edit and save; compile's Lua turns it into a
  Pandoc `Span` with that identifier so DOCX gets a bookmark. The `Link`
  extension is configured (`isAllowedUri`) to accept project-relative
  targets (no scheme, no `..`, a `.md` path optionally followed by
  `#fragment`) instead of discarding them, and clicking such a link opens
  that document in the binder.
- **Compile finds project images** (R3-12). Core writes the combined
  manuscript to the system temporary folder and runs Pandoc there, so a
  project-relative image is "not fetched" and replaced by its description.
  `compile` passes `--resource-path=<project root>` and the stored
  project-relative convention is the one it resolves; DOCX/ODT output
  embeds the image.
- **Compile resolves links between documents, from the parse it already
  does** (R3-10, R4-5, R5-4, R5-5, R6-1 to R6-4, R7-1 to R7-7; owner
  ruling 2026-10-27). The converter writes links between documents as
  project-relative paths (`manuscript/two.md#two`) and the split in slice 4
  writes the same form; today compile passes them through as hyperlinks to
  a relative path, which no DOCX or PDF reader can follow. The earlier
  designs failed because each one changed *what* Pandoc parsed (markers,
  per-file scope, sentinels); this one changes nothing about the parse and
  works on its result.
  1. **Gate.** Compile runs today's combined Markdown through Pandoc to
     JSON (filters attached, as now). If no `Link` target in that JSON,
     after stripping a leading `./` and percent-decoding, is exactly the
     project-relative path of a document in the project (with or without a
     `#fragment`; exact membership in the document list, excluded documents
     included, so an ordinary relative link to `research/notes.pdf` is not
     one), compile continues from the *same Markdown* with today's
     invocation: no second parse, no renaming, output as today. Native
     projects without such links never enter the steps below.
  2. **Provenance by document markers in the combined text.** Block
     counting cannot tell documents apart when an empty separator lets two
     lists or two indented code blocks merge (R8-1), so compile marks the
     text itself: before and after each document's Markdown it writes an
     HTML comment on its own line between blank lines,
     `<!-- chikn:begin <index> <nonce> -->` and `<!-- chikn:end <index>
     <nonce> -->` (one random nonce per compile run, so no literal comment
     in a document can impersonate a marker). Pandoc reads each as a
     top-level `RawBlock html`, which the filter removes and which every
     non-HTML writer would drop anyway; title page and separators lie
     outside begin/end pairs and are generated blocks. Two exceptions keep
     today's parse intact: the first document gets no begin marker when no
     title page is generated (its `%` title block must stay at the very
     start, R6-4), and the markers are only written when the gate of step 1
     fired, so projects without links between documents compile from
     unmarked text as today. A marker swallowed by an unclosed fence or Div
     (which already garbles today's output by swallowing the next document)
     is detected as a missing or out-of-order marker; compile then emits a
     named warning ("document X runs into the next document; links between
     documents were not resolved"), leaves links as they are and keeps the
     content, the stray comment line inside that code block included. Two
     lists or code blocks that today merge across an empty separator stay
     separate when markers are present, which the record notes as the one
     content difference, confined to projects with links between
     documents.
  3. **Knowing which identifiers are explicit.** Pandoc's JSON does not say
     whether an identifier was written by the author or generated from the
     heading text, and the two behave differently (explicit ones are kept
     verbatim, generated ones get `-1`, `-2` when an earlier document used
     them). Compile recovers the distinction with one more parse of the
     same marked text, `-auto_identifiers` added: it has the same block and
     inline structure, and an element whose identifier is non-empty there
     is explicit. The same must hold inside the editor's Markdown-bodied
     footnotes, which the Lua filter decodes with `pandoc.read(body,
     "markdown")`: in the diagnostic run the filter reads them with the
     outer run's reader extensions (`PANDOC_READER_OPTIONS.extensions`,
     which carries the `-auto_identifiers` setting), so a generated heading
     identifier inside a note is empty there too and cannot pose as an
     explicit one (R10-2). Likewise each document's solo parse, when needed,
     runs with the same reader options as its position in the combined text
     (title-block rule above), so its block count and order equal its
     section's; a block's kind may differ where a reference resolves only
     in the combined text (a lone reference image is a `Para` alone and a
     `Figure` in company), so a block-path match also checks the matched
     element's kind, and the blocks inside `Note`s are not part of any
     block path (R10-1).
  4. **Finding a target, exactly or not at all.** For a link `doc.md#id`:
     (a) if the section of `doc` holds an element whose *explicit*
     identifier is `id`, that element is the target (two such elements is a
     duplicate explicit identifier in that document and gets a named
     warning); a generated identifier equal to `id` on another element in
     the section cannot mislead this rule because it is not explicit
     (R9-1, R9-3); (b) otherwise `doc` is parsed on its own (small
     independent runs, at most four at a time, only for documents a link
     needs): if a *block-level* element there (header, Div, code block,
     table, figure) *outside any footnote* carries the generated identifier
     `id`, its block path (the sequence of block indices from the document's
     top level down to it, never descending into a `Note`) names the same
     element in the section, because the two parses of the same text have
     the same block count and order; cross-document reference and footnote
     resolution changes inlines and note contents, never top-level block
     positions, so this match is exact where text or rank matching was not
     (R8-2, R9-2); note contents are excluded because the combined text may
     use a later document's definition of the same note label, so the
     note's blocks can differ from the solo parse (R10-1); (c)
     otherwise, for an identifier inside a footnote (a note is rendered
     where it is referenced, not where it is defined), an element with that
     *explicit* identifier inside `Note` contents anywhere in the combined
     document, unique, is the target (R8-5); (d) otherwise the link becomes
     its text and compile emits a named warning naming the link and the
     document, so an unresolvable link is reported, never silently kept as
     a dead hyperlink. Inline-level targets (spans, images) are reached only
     by explicit identifiers, through (a) or (c).
  5. **Final identifiers are allocated per element, not per name.** Every
     element that carries an identifier in the combined parse, or that a
     link resolved to, receives a fresh final identifier: `d<index>_<n>`
     for the n-th such element of document `index`, `g<n>_<m>` on generated
     blocks (letters, digits and underscores only, the form Word documents
     accept for bookmark names; Pandoc writes identifiers into bookmarks
     unchanged); so two elements that Pandoc left with the same string (an
     explicit `{#introduction-1}` beside a generated `introduction-1`,
     R9-3) end with distinct bookmarks, and nothing can collide with a
     document anchor. Every link whose target was a bare `#id` is resolved
     the way the output would resolve it today, to the first element in
     document order whose combined identifier is `id`, and rewritten to
     that element's final identifier, so native navigation keeps exactly
     the target it has today.
  6. **Destinations every writer emits, placed where the target is.** The
     writer table from round 7, re-run and recorded at the start of the
     slice, decides how a resolved target is materialised: a `Header`
     target uses its own final identifier (bookmark in DOCX and ODT, `id`
     in HTML and EPUB, `\label` in LaTeX); a `Span` elsewhere than a
     heading is used where it stands; any other inline target (image, link,
     inline code) gets an empty identified `Span` immediately before it,
     since the writers emit no bookmark for those elements (R10-4); a `Div`
     target gets its destination at its real beginning: a `Span` as the
     first inline of its first block when that block has inlines, otherwise
     an empty `Plain` holding the `Span` before that block (R9-6); a code
     block, table, figure, raw block or rule target gets the same
     `Plain`+`Span` immediately before it (the one case that adds an empty
     paragraph in DOCX and ODT, recorded). Then one rule applies last, to
     every destination however it was chosen: **a destination that would
     sit inside a heading becomes that heading's own identifier**, because
     LaTeX gives a span inside a heading a `\hypertarget` but no `\label`
     (R8-3, R9-5, and a heading-first Div, R10-3). Nothing is ever moved to
     the start of the document (R8-4). A whole-document link (`doc.md`)
     targets the section's first block by the same table.
  7. **Rewrite.** `doc.md#id` → `#<final identifier of the resolved
     destination>`; `doc.md` → the section's destination; a link to a
     document outside the compile → its text alone; everything else
     untouched. Notes with `data-format="markdown"` are decoded before the
     walk so links inside them are `Link` nodes resolved like any other.
     The result goes to the output format with `-f json` and the same
     options as today.
  For projects without links between documents nothing changes. For
  projects with them, the Markdown Pandoc parses is today's text plus the
  marker comments, so title blocks, duplicate definitions and separators
  behave as they do now; identifiers and link targets change, and the one
  merge-across-empty-separator difference above is recorded.
- **Markdown-bodied footnotes** (R3-8): the `data-format="markdown"`
  reading in `cs-footnotes.lua` and the attribute round trip in
  `FootnoteNode`, as described in slice 4.

Core changes stay inside `crates/core/src/core/compile.rs`,
`cs-footnotes.lua` and the new `publish_imported`; no format change. This
slice lands **before** Import File (R4-8): slice 4's acceptance tests run
imported content through this editor and this compile.

### Slice 4 — Import File reads everything Pandoc reads, split optional

Inside a project, both shells:

- **One registry of import formats** (R1-7, R2-14), `crates/services/src/
  formats.rs`: filename extension → handler, where a handler is either a
  Pandoc reader name or a preprocessing step. It drives the services'
  validation, the web upload route and both choosers (the UI reads a
  generated `ui/src/importFormats.json`; a services test asserts the file
  equals the registry, so the three lists cannot drift). Today's 15 UI
  extensions are kept and corrected: `.html`/`.htm` go to the `html` reader
  (today they fall through to Markdown and keep their tags). Added, with
  real extensions rather than reader names: `.typ` (typst), `.dj` (djot),
  `.wiki`/`.mediawiki` (mediawiki), `.adoc`/`.asciidoc` (asciidoc; the
  installed Pandoc 3.11 lists the reader), `.dbk` (docbook), `.creole`,
  `.t2t`, `.opml`, `.ipynb`, `.man`, `.muse`, `.pod`, `.csv`/`.tsv` (as
  tables). Ambiguous `.xml` and extension-less DokuWiki pages stay out.
  `.doc` stays selectable in both shells with the preprocessing handler
  below, so the LibreOffice step and the save-as message are reachable.
- **Legacy `.doc` is not readable by Pandoc** (today `io.rs` maps `doc` to
  the `docx` reader, which fails on a real `.doc` with a confusing error).
  With LibreOffice on the machine, `soffice --headless --convert-to docx`
  runs first; without it, the message is "Save it as .docx first". Apple
  Pages gets the same message.
- **Every Pandoc import keeps its pictures and its notes** (R2-9, R2-13;
  split or not). The Pandoc read runs with `--extract-media=<staging>/
  media`, so images embedded in a DOCX, ODT or EPUB land as files; they are
  added to the project as attachments the way the Scrivener converter adds
  research files (file beside its `.meta` sidecar, written through core)
  before the staging folder is removed, and the image targets in the text
  are rewritten to them (stored project-relative, `research/<file>`, the
  converter's convention). **Order of work inside Pandoc's model** (R4-7):
  media targets, identifiers and links are mapped and rewritten first,
  while notes are still `Note` nodes whose links the walk can see; only
  then does the note filter run. That filter
  (`crates/services/src/cs-import-notes.lua`, the inverse of core's
  `cs-footnotes.lua`) turns every Pandoc `Note` into the editor's footnote
  element, emitted exactly as Pandoc's Markdown reader would produce it
  from the editor's own file: three inlines, `RawInline("<sup class=
  \"footnote\" data-body=\"…\">")`, `Str("●")`, `RawInline("</sup>")`,
  so compile's filter finds the closing tag it expects. **The body
  attribute is entity-encoded, newlines included** (R5-2): `"`, `&`, `<`,
  `>` as their named entities and every newline as `&#10;`, because a
  literal blank line inside an inline HTML tag ends the paragraph for the
  editor's Markdown parser, which then loses the footnote and shows its
  body as text on the next open. The same rule applies to every writer of
  the element: the import filter, the editor's `FootnoteNode` serialiser
  (today it writes the attribute raw, so a multi-paragraph note from the
  Scrivener converter already suffers this on its second open) and the
  converter's `footnote_element`; every reader already decodes entities
  (the browser's DOM parser for the editor, Pandoc's HTML reader) except
  compile's `cs-footnotes.lua`, which reads the attribute text itself and
  gains the decode. The Markdown writer runs as the one shared
  configuration `formats::MARKDOWN_WRITER` (`markdown-raw_attribute`, used
  by the unsplit path, the split path and the media rewrite alike, R5-3) so
  raw HTML is written literally rather than as `` `…`{=html} ``, which the
  editor would show as code (R4-3). Today that element's `data-body` is plain text, so a note with
  emphasis, a link or a list would be flattened (R3-8). The element gains
  an optional `data-format="markdown"` attribute: when present, the body is
  Markdown (the note's blocks written with `pandoc.write(…, "markdown")`)
  and compile's `cs-footnotes.lua` reads it with `pandoc.read` instead of
  treating it as text; when absent, today's plain-text meaning is unchanged,
  so every existing note keeps its behaviour. The editor's `FootnoteNode`
  carries the attribute through its parse/render round trip and shows such
  a body as Markdown source in its editing field. No `[^n]` labels are
  written, so chapters cannot collide when compiled together (today's
  concatenating compile would merge two imported files' `[^1]` notes).
- **Split, as an option** (R1-5, R2-10, R2-11, R2-12). After the file is
  picked, a small dialog offers "Split into one document per top-level
  heading" (checkbox, off by default, remembered in settings under
  Writing). On: `import_file` gains a `split_headings: bool`, and the split
  happens in Pandoc's document model: the file is read to Pandoc JSON
  (with the media and note filters above), top-level `Div` wrappers that
  hold a level-one `Header` are hoisted so `<section><h1>` HTML and fenced
  Divs split like plain headings; a hoisted wrapper's identifier is not
  lost: it becomes an anchor element at the start of the heading it
  wrapped, in the form the editor keeps (`<span class="anchor"
  id="part-two"></span>`, the `AnchorNode` of slice 3; a bracketed
  `[]{#id}` span would be flattened to visible text on the editor's next
  save, R3-9, R4-4). The block list is cut at each level-one
  `Header` at that level only (headings inside block quotes, lists or notes
  are not boundaries). Each part is written back with `pandoc -f json -t
  <formats::MARKDOWN_WRITER>` (the shared `markdown-raw_attribute`
  configuration, R5-3). Before writing, a map of *every* identifier in each
  part (headers, Divs, Spans, code blocks, images, tables) → destination
  document rewrites every `#id` link to the document-qualified form
  `<document>.md#id`, whether the target ends up in another part or in the
  same one, so the editor can open the document it names and compile can
  resolve it to that document's element (R5-4). Every heading such a link
  points at gets an explicit identifier that a generated one can never
  equal, `{#S<n>}` numbered within the document (generated identifiers from
  ASCII text are lowercase, so an uppercase ASCII letter settles it), with
  `n` chosen so that the identifier is not already present anywhere in the
  part, footnote bodies included (every identifier in the part's model is
  reserved first, R10-5), and the link is written to that identifier; because it differs from the generated one, the Markdown
  writer emits the attribute (R9-4: an attribute equal to the generated
  identifier is omitted), and the slice's tests read the written Markdown to
  prove it is there. A hoisted wrapper's identifier (`<section
  id="part-two">`) is folded into the same scheme: links to the wrapper and
  to its heading both go to the heading's `{#S<n>}`, so no anchor span is
  placed inside a heading, where LaTeX would give it no label (R9-5); the
  `AnchorNode` stays for anchors that stand on their own. A link whose
  target identifier exists nowhere is left as it was. Document
  metadata (`meta`: title, author,
  date, anything else) is not dropped: title, author and date open the
  "Front matter" document as a heading and lines, and the full metadata is
  kept as custom fields on that document's `.meta` sidecar (when there is
  no text before the first heading, Front matter holds the metadata
  alone). One binder document per heading named by it, in order; a file
  with fewer than two such headings imports whole with a one-line notice.
  Off: today's single document, unchanged bytes for Markdown and plain
  text.

### Slice 5 — other writing apps' project formats (one slice each; first pass: free apps only)

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
| `crates/cli/src/archive.rs` (new), `unpack.rs` (new), `detect.rs` (new), `lib.rs`, `main.rs`, `Cargo.toml` (+ `zip`, `sevenz-rust2`, `tar`, `flate2`), `Cargo.lock` | `unpack_archive` (format sniffing, header pre-checks, path/kind/size checks), the unpack worker (`--unpack-worker`, `BudgetAlloc` counting allocator, per-platform supplementary limits, JSON outcome with peak), `detect_project` |
| `crates/core/src/core/project/publish.rs` (new), `compile.rs`, `cs-footnotes.lua` | `publish_imported` (read, exclusive rename, no fallback), `--resource-path`, document-link resolution (gate, block-count alignment with per-document parses, identifier renaming, target materialisation, rewrite), anchor elements as `Span`s, `data-format="markdown"` note bodies, entity decoding of the body attribute |
| `crates/services/src/project.rs`, `io.rs`, `formats.rs` (new), `cs-import-notes.lua` (new); `crates/web/src/commands.rs`, `main.rs`, `assets.rs` (new); `src-tauri/src/commands/*.rs`, `main.rs` | `import_project` (lock, existence check, temporary destination, core publication), recursive Markdown-folder import with warnings, the format registry with handlers, worker dispatch honoured first in every `main`, media extraction and attachment, notes to the editor's form with Markdown bodies, `.doc` step, model-level `split_headings`, the gated asset route |
| `ui/src/components/editor/Editor.tsx`, `ImageNode.ts` (new), `AnchorNode.ts` (new), `FootnoteNode.ts`, `ui/src/platform.ts`; `src-tauri/src/assets.rs` (new), `tauri.conf.json`, `main.rs` | image node and serialisation, anchor node, project-relative links accepted and opened, project-asset URL resolver per shell, `data-format` round trip and entity-encoded body attribute, the `chikn-asset` scheme handler and CSP |
| `ui/src/commands/web.ts`, `io.ts`, `project.ts`, `stores/projectStore.ts`, `ui/src/importFormats.json` (generated) | `importProjectUpload`, `importProject`, `split` flag, shared format and archive suffix lists |
| `ui/src/components/welcome/ImportProjectDialog.tsx` (new), `Welcome.tsx`, `editor.css`, `platform.ts` | the chooser; desktop archive pick |
| `ui/src/components/binder/Binder.tsx`, a small `ImportFileOptions` dialog | the split checkbox, wider accept list |
| `crates/cli/src/{novelwriter,manuskript,textbundle}/` (slice 5, one per slice) | readers + fixtures made in the apps |
| `docs/USER_GUIDE.md`, `docs/plans/README.md`, `.agents/state.md`, `DEVLOG.md` | records |

## [MODEL] Tests

- [ ] `crates/cli` archive: each of zip, 7z, tar.gz, tgz and tar unpacks a
      two-file fixture to the same tree; fixtures made by ordinary
      archivers (`zip -r`, `tar -C Book -czf … .`, `7z a`) with `Book/`,
      `./` and `./chapter.md` entries unpack; sniffing ignores a wrong
      extension; entries named with `..`, a leading `/` or `\`, a `C:`
      prefix, an interior `.`, control characters, a trailing dot, a device
      name, two names differing only in case, `A/one.md` beside `a/two.md`,
      or a file and a folder of one name are refused and nothing is left on
      disk; tar symlink, hard-link and device entries, a zip symlink, a 7z
      symlink and a 7z anti-item are refused; the size and entry-count
      ceilings stop a small expansion bomb; a 7z whose signature header
      declares a 100 MiB header, a 7z with a 1 GiB dictionary, a tar with a
      16 MiB PAX header, and a tar whose PAX `size` record disagrees with
      the raw size are refused; an encrypted 7z and a RAR are refused with
      the "re-save as zip" message.
- [ ] `crates/cli` unpack worker: a tiny valid archive unpacks through
      the worker launched from each production executable (converter CLI
      directly; web server and Tauri binaries through their `main`
      dispatch, the Tauri one as a plain `cargo test` of the dispatch
      function where the crate builds); a 7z with an encoded header that
      expands to 2 GiB and a zip declaring ten million entries are refused
      under the worker's memory limit without the parent's memory growing
      (asserted through the child's exit status, not by surviving the
      allocation in-process, and with the worker's reported peak counted
      allocation recorded in the test log); a direct test of `BudgetAlloc`
      shows an allocation that would cross the budget refused and the
      counter returning to zero after frees; the same two archives
      pass/refuse correctly when the parent has first mapped 2 GiB of
      address space (R3-2); a worker made
      to sleep past the wall clock is killed, the staging folder is gone
      and the admission slot is free afterwards; dropping the request
      future mid-unpack kills the worker; a crashed worker is a refusal,
      not a hang. Tar: a small file with a matching PAX `size` record
      unpacks; a divergent record is refused.
- [ ] `crates/cli` detector, run through conversion: Scrivener at top, one
      down, beside `__MACOSX/`, beside a `README.md`, and alone eight
      folders deep under a README; a Markdown tree whose documents are all
      below the root; two `.scriv` folders, a `.scriv` nested inside
      another project's tree, and a shallow marker with a second one eight
      folders deep → "more than one project"; a real `.docx`, an `.odt` and
      an `.epub` (zip containers) → the "use Import File" message; a folder
      of PDFs → not a project.
- [ ] `crates/core` publication: `publish_imported` refuses an existing
      empty folder created *between* the caller's check and the rename
      (the temporary project stays, nothing is replaced; Linux CI proves
      `RENAME_NOREPLACE`); with the exclusive rename forced unsupported it
      fails safely with the storage message and removes the temporary
      project; a read error injected before the commit removes the
      temporary project; a token failure injected after the commit yields
      `ImportedButNotOpened` with the final path and the project intact.
- [ ] `crates/services` import transaction: a converter error after the
      project was created leaves nothing at `outputPath` and no staging
      folder, and a second attempt at the same path succeeds; an existing
      empty folder, an existing file, a dangling symlink and an existing
      project at `outputPath` are each refused and left byte-for-byte
      untouched; after a successful import the returned `Project.path`
      names the final folder, no `.importing-*` entry remains, and the
      project can be opened, edited and a revision saved; with the
      post-commit token failure injected, the web route answers the tagged
      `PublishedNotOpened`, and (with slice 2) the UI store (mocked
      command) adds the path to recents, opens nothing and shows the
      message. Markdown folder:
      nested folders become binder folders in sorted order; duplicate names
      are disambiguated; one unreadable file among three is reported in the
      warnings and the other two import; a tree with no readable file
      fails.
- [ ] `crates/services` import content: a DOCX with an embedded image
      imports with the image as an attachment and a project-relative
      target after the upload is deleted; a DOCX with two footnotes imports
      as two editor footnote elements and no `[^n]` label, and the
      importer's actual Markdown output, saved through the editor, still
      compiles to two notes (no `` `…`{=html} `` code spans), and the same
      holds for the split path's output; a footnote with emphasis, a link
      and a list imports with `data-format="markdown"` and compiles (core)
      to a note with that emphasis, link destination and list; a note with
      two paragraphs and a list survives import → editor save → fresh
      editor open → second save → compile with its note count, body
      structure and surrounding text intact (the attribute newline
      encoding); a footnote in chapter one whose link targets chapter two
      is rewritten to the document-qualified form like any other link,
      compiles to a working link to chapter two and to its text when
      chapter two is excluded; a plain-text note without the attribute
      compiles exactly as today; two imported chapters each holding a footnote compile to
      two distinct notes.
- [ ] `crates/core` compile (slice 3): a project-relative image compiles
      into DOCX and ODT output as an embedded picture while the test runs
      from another working directory. Links between documents, each
      asserted on the destination's text in DOCX and HTML and on a matching
      `\label` in the LaTeX source and a bookmark in ODT: a link to an
      included document's heading lands on that heading; a link to the
      document itself lands at its start for a document opening with a
      heading (the heading's own label), with a paragraph, and with a code
      block (the recorded empty paragraph); a link to an excluded document
      becomes its text; two documents each headed `# Introduction` and a
      link to the second lands on the second, while a native bare
      `#introduction-1` link still lands where it does today; document B
      with two `# Introduction` headings after A has one resolves
      `b.md#introduction` and `b.md#introduction-1` to B's first and second
      by block path (R8-2, R9-1); B with `# Intro[^n]` (note defined in A)
      followed by `# Intro` resolves `b.md#intro` to the second heading
      (R9-2); B with a generated `# Introduction` followed by
      `# Introduction {#introduction-1}` yields two distinct bookmarks and a
      link to the explicit one lands on the second (R9-3); a reference
      definition carrying `{#linkid}` in B used from A does not disturb A's
      targets; two documents with explicit `{#target}` resolve separately;
      the split's `{#S1}` heading is found by explicit identifier and the
      written Markdown of the split contains the attribute (R9-4); a part
      that already contains `[anchor]{#S1}` gets its linked heading a
      different `S<n>` and both resolve after an editor save and compile
      (R10-5); a link to
      a hoisted `<section id="part-two">` wrapper lands on its heading with
      a LaTeX `\label` (R9-5); a link to an identified table in the middle
      of a document lands there (never at the document's start, R8-4), as
      does a link into a document with no headings; a link to an identified
      code block resolves in ODT through its materialised `Span`; a Div
      opening with a code block then a paragraph, and a Div holding only
      code, each get their destination before the code (R9-6); a link into
      a heading with an explicit identifier inside B's footnote referenced
      from A resolves (R8-5); A's `[^n]` note holding `# Wanted` while B
      redefines `[^n]` with `# Wrong` does not let `a.md#wanted` reach the
      wrong heading (it resolves only through an explicit identifier or
      becomes text with the warning, R10-1); a Markdown-bodied note holding
      `# Generated` and `# Real {#generated}` resolves a link to the
      explicit one without a duplicate warning (R10-2); a link to a
      heading-first Div lands on the heading with a LaTeX label (R10-3); a
      link to an identified link and to identified inline code each get a
      destination that exists in DOCX, ODT and LaTeX (R10-4); a native footnote defined in document two and
      referenced from document one, with a heading in its body, leaves
      document one's later heading correctly targeted; a heading literally
      named `d0-x` does not collide with the first document's anchor and a
      heading named `section` does not collide with a separator; duplicate
      reference definitions and duplicate footnote definitions across
      documents resolve as today; an unresolvable `b.md#nowhere` becomes its
      text with the named warning; a document with a fence closed only in
      the next document produces the named warning, unchanged content and
      unresolved links, never moved or truncated text; with an empty
      separator, two adjacent lists stay two lists and the record's note
      applies; a later document beginning with `% Title` lines, and a
      first document beginning with them under both title-page settings,
      resolve and keep today's text; a 100-document project with one link
      between documents compiles within four concurrent solo parses and a
      recorded time bound. Unchanged path: an existing sample project, a
      project with a leading `%` title block and a project with a relative
      link to `research/notes.pdf` compile through today's Markdown
      invocation with no markers (asserted on the Pandoc arguments and on
      normalised DOCX/HTML structure, not raw bytes: EPUB output embeds a
      fresh UUID on every run).
- [ ] UI editor (slice 3): a document with `![Figure](research/figure.png)`
      opens, the image node is present, an edit and a save keep the image
      line byte-for-byte, and the resolver produces the asset URL for each
      shell; the web asset route refuses a path outside the project; a
      footnote with `data-format="markdown"` and a two-paragraph body
      round-trips its attribute and body through parse, serialise, parse
      again and serialise again (newlines encoded as `&#10;`, decoded on
      read); an anchor element and a link to `manuscript/two.md#two`
      survive open, edit and save byte-for-byte, and clicking the link
      opens that document; the desktop asset handler (unit test on the
      request function with a fake state): serves a file inside the open
      project, refuses a path outside it, refuses everything once the
      project is closed, serves again after reopening, and serves project
      B's files after switching from A (display verified in the built
      shell).
- [ ] `crates/services` split: three top-level headings → three documents
      in order plus front matter; a footnote referenced in chapter one and
      defined after chapter two stays in chapter one, and the two chapters
      compile to two distinct notes; a reference-style link shared by two
      chapters resolves in both; a link from chapter one to a heading in
      chapter two becomes a document link to chapter two, and a link to a
      `<section id="part-two">` wrapper's identifier lands on an anchor
      that exists in the second document; HTML with two `<section><h1>`
      blocks and Markdown with headings inside fenced Divs split into two
      documents; a heading inside a block quote is not a
      boundary; a `# ` line inside a fenced code block and a Setext heading
      are treated as the parser says; YAML title, author and date survive
      into Front matter and its sidecar; one heading → whole file and the
      notice; split off → one document with unchanged bytes for Markdown;
      `.doc` without LibreOffice → the save-as message, through both
      shells' accepted lists. Format registry: every Pandoc-handled
      extension maps to a reader Pandoc lists; `.html` converts a heading
      and emphasis to Markdown; `ui/src/importFormats.json` equals the
      registry.
- [ ] `crates/web/tests/http.rs`: zip-slip → 400, nothing written; a lone
      Markdown document → 400 with the "use Import File" message; a real
      `.docx` → the same message; an unknown file → 400 with the "upload an
      archive" message; `outputPath` outside the root → 403; `outputPath`
      naming an existing empty folder → 409 and the folder untouched; a
      Markdown tar.gz → 200 with the documents and warnings; a `.scriv`
      zip → 200 (Pandoc cases skip with a printed line where Pandoc is
      absent, which is CI).
- [ ] UI: the chooser renders its two choices and accepts every suffix in
      the shared list, `.tgz` and `.tar` included; an archive pick asks for
      a name and posts the one file to `<root>/<name>.chikn`; the
      on-this-machine choice still works; cancel sends nothing; Import File
      offers every extension in `importFormats.json`, `.doc` included, and
      shows the split checkbox off by default and passes it through.
- [ ] Slice 5, each: a fixture project made in the app converts to the
      expected tree; the Manuskript fixture holds nested world entries,
      characters and outline documents; the TextBundle fixture uses
      `text.markdown`, not `text.md`.
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
- **Settled 2026-10-27 — app-format first pass (slice 5):** only formats a
  free app can produce for checking: novelWriter, Manuskript, TextBundle
  (via Zettlr). yWriter deferred; Ulysses library files not read (export
  instead).
- Taken unless told otherwise: upload limit 512 MiB; unpacked ceiling 4 GiB
  and 50 000 entries; the unpack worker gets a 1 GiB allocation budget and
  120 s; compile keeps its single combined parse and resolves document
  links by aligning it with per-document parses; 7z
  header above 16 MiB or dictionary above 256 MiB refused; encrypted
  archives refused rather than prompting for a password; the split option
  is off by default and remembered; imported footnotes take the editor's
  footnote form rather than `[^n]` labels, with `data-format="markdown"`
  for formatted bodies; storage that cannot rename exclusively refuses
  the import rather than risking a replacement; the engine additions
  (publication API, compile link and image resolution) are part of this
  plan; the
  dependencies (`zip` MIT, `sevenz-rust2` Apache-2.0, `tar` and `flate2`
  MIT/Apache-2.0) are licence-compatible with this MIT repository.
- Settled 2026-10-27: the go ("set the push policy to always, push, then
  go"); plan reviewed by Codex gpt-6-astra xhigh before slice 1, slices
  reviewed by the same reviewer (`.agents/review/findings/plan-4.md`).
- **Owner ruling 2026-10-27 — links between documents must work in
  compiled output.** After round 7 the orchestrator proposed deferring
  compile link resolution to a separate plan and rendering such links as
  text. The owner refused: "a link resolving to plain text means this
  cannot ship with honest import functionality as a feature since it's
  broken. no." The feature stays in slice 3 with the design above.

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
- 2026-10-27 review round 2 (`plan-4`, `da9d73a`): reopened; R1-2, R1-7
  to R1-13 closed, R1-1, R1-3 to R1-6 not yet, fourteen findings, all
  admitted after checking: the web gate's `creatable` returns an existing
  path and a plain rename replaces an empty folder (R2-1); the returned
  project kept the temporary path (R2-2); 7z decodes encoded headers and
  tar reads PAX bodies before any in-process check, and a timed-out
  blocking task cannot be stopped (R2-3, R2-5: unpacking moves into a
  killable worker process with an OS memory limit); a PAX `size` record
  lets the pre-walk and the crate disagree (R2-4); `Book/` and `./`
  entries were refused (R2-6); implied ancestor directories escaped the
  collision check (R2-7); a nested project beside a README fell to the
  Markdown fallback (R2-8); per-chapter `[^1]` labels collide at compile
  (R2-9: notes become the editor's footnote elements on import);
  section-wrapped headings, cross-chapter fragment links and document
  metadata were lost by the split (R2-10 to R2-12); embedded images were
  never extracted (R2-13); `.doc` had been removed from the only list the
  choosers read (R2-14).
- 2026-10-27 review round 3 (`plan-4`, `fa9a445`): reopened; R1-4, R1-5,
  R1-6, R2-2, R2-4 to R2-10, R2-12, R2-14 closed; twelve findings, all
  admitted after checking: a worker in `services` called from the
  converter CLI is a Cargo cycle (R3-1: worker moves to the converter
  crate); a `pre_exec` address-space limit is applied to the parent's
  image and macOS refuses it (R3-2: the worker limits itself after exec);
  publication is `.chikn` I/O and belongs in core (R3-3); the plain-rename
  fallback restored the replacement race (R3-4: fail safely instead); a
  failure after the rename left a published project behind a "failed"
  result (R3-5: rename is the commit point, truthful
  `ImportedButNotOpened`); a depth limit cannot establish "no marker
  anywhere" (R3-6: walk the whole entry list); small files carry PAX
  `size` records too (R3-7: accept when equal to the raw size); the
  footnote element's body is plain text (R3-8: `data-format="markdown"`);
  hoisting dropped wrapper identifiers (R3-9); compile leaves document
  links literal (R3-10), the editor drops images on save (R3-11) and
  compile cannot fetch project images (R3-12): new slice 4 "what imports
  create survives the editor and compile"; app formats become slice 5.
- 2026-10-27 review round 4 (`plan-4`, `20ffe17`): reopened; R2-1, R3-1,
  R3-3, R3-4, R3-6, R3-7, R3-8, R3-12 closed; eight findings, all
  admitted after checking: `RLIMIT_AS` is unusable on macOS because of the
  6 GiB shared region (R4-1: parent-side watchdog there, Job Object on
  Windows via `CreateProcessW`, sync `main` dispatch); the
  not-opened outcome had no shell contract (R4-2: tagged `ImportOutcome`
  to the store); Pandoc's Markdown writer emits raw HTML as
  `` `…`{=html} `` (R4-3: `markdown-raw_attribute`, three-inline element);
  the editor flattens `[]{#id}` and discards relative links (R4-4:
  `AnchorNode`, `Link` accepts project-relative targets); a global
  fragment lookup lands on the wrong document (R4-5: per-file compile with
  `--file-scope`); the desktop asset protocol is not enabled (R4-6);
  encoding notes before the link walk hides their links (R4-7: notes
  encoded last); slice 3's tests needed slice 4 (R4-8: the consumer slice
  is now slice 3 and lands first).
- 2026-10-27 review round 5 (`plan-4`, `5ca2e2f`): reopened; R1-1, R3-2,
  R3-5, R3-9, R4-2, R4-4, R4-7, R4-8 closed; six findings, all admitted
  after checking: a sampled watchdog is not a memory bound (R5-1: a
  counting allocator inside the worker, budget enforced before each
  allocation, on every platform); the footnote element's attribute carried
  literal newlines, so a multi-paragraph note breaks on the editor's second
  open (R5-2: entity-encoded attribute, `&#10;`, in every writer and the
  compile reader; a pre-existing defect for Scrivener imports); the split
  command still named the plain `markdown` writer (R5-3: one shared writer
  constant); links inside notes were decoded after the file-scope rewrite
  (R5-4); `--file-scope` breaks native projects' cross-document reference
  definitions and bare fragment links (R5-5: compile keeps the single
  combined parse and maps document-qualified links through per-document
  solo parses matched by position); Tauri's asset scope cannot be revoked
  and re-allowed (R5-6: an app-owned `chikn-asset` scheme handler that
  checks the open project per request).
- 2026-10-27 review round 6 (`plan-4`, `c042cc5`): reopened; R1-3, R2-3,
  R2-13, R3-11, R4-1, R4-3, R4-6, R5-1, R5-2, R5-3, R5-4, R5-6 closed; four
  findings, all in the compile link design and all admitted: header
  positions are not stable provenance when a note defined elsewhere brings
  a heading into a document (R6-1); the map covered headers only, not the
  anchors the split creates (R6-2); explicit duplicate identifiers and a
  native `doc-0` collide (R6-3); a marker before the first document breaks
  a `%` title block (R6-4). Redesign: compile is unchanged unless the
  combined parse contains document-qualified links; then each document is
  parsed in its own run with the others appended behind a sentinel (exact
  provenance, references still resolve), every identifier is renamed with
  a document prefix, documents are wrapped in identified Divs, and links
  are resolved by lookup on any element kind.
- 2026-10-27 review round 7 (`plan-4`, `e857033`): reopened; R4-5 and
  R6-1 closed; eight findings, seven of them in the compile link design
  (the sentinel can land inside an unclosed Div or code fence, R7-1;
  moving a document first changes which duplicate definition wins, R7-2;
  native `#introduction-1` links lose their target, R7-3; ODT drops a code
  block's identifier and renders an identified Div as a section, not a
  bookmark, R7-4; the first document's `%` lines change meaning with the
  generated title page, R7-5; generated title and separator blocks escape
  the identifier namespace, R7-6; a hundred whole-project parses have no
  resource bound, R7-7) and one in the acceptance oracle (EPUB output is
  never byte-identical, R7-8). All admitted. Decision: compile link
  resolution leaves this plan and is deferred to its own plan; compile
  renders document links as their text, nothing more. The allocator bound
  is stated as conditional on coverage evidence once the crates are
  locked, as the reviewer asked.
- 2026-10-27 owner ruling: the deferral is refused ("a link resolving to
  plain text means this cannot ship with honest import functionality as a
  feature since it's broken. no"). Round 8, dispatched against the deferral
  revision, was stopped before a verdict. Compile link resolution is back
  in slice 3 with a design that changes nothing about what Pandoc parses:
  gate on document links in the combined JSON; provenance by solo block
  counts (no markers, no sentinel, no whole-project re-parse; a count
  mismatch means an unclosed fence, which gets a named warning and
  unresolved links, never moved content); per-section identifier map by
  order with notes skipped and suffix-checked; renaming of every
  identifier with a section or generated prefix; bare links rewritten
  through the combined identifier they already target; targets
  materialised as identified `Span`s where a writer would not emit one;
  rewrite; same options to the output format.
- 2026-10-27 review round 8 (`plan-4`, `81f17bb`): reopened; R7-1 to R7-8,
  R5-5, R6-3, R6-4 closed; five findings, all admitted after checking:
  an empty separator lets adjacent lists merge, breaking block counts
  (R8-1); solo and combined identifier sequences are neither one-to-one
  nor suffix-related when a document has two same-named headings or a
  cross-document reference adds an identified `Link` (R8-2); a `Span`
  inside a heading gives LaTeX no `\label` (R8-3); materialised
  destinations were placed at the document's start (R8-4); identifiers
  inside notes had no mapping (R8-5). Redesign: provenance by per-run
  nonce'd begin/end comment markers written only when the gate fires (no
  marker before an unprefixed first document); targets found by identifier
  in the section, then by kind, text and rank from a solo parse, then by
  explicit identifier inside notes, else text with a named warning;
  destinations materialised at the target by a per-writer table, headings
  by their own identifier; the split writes explicit `{#id}` on linked
  headings. The reviewer's 138 boundary probes, writer table, gate check
  against core's document paths and option-preservation check are recorded
  in `plan-4`.
- 2026-10-27 review round 9 (`plan-4`, `1b9fdcd`): reopened; R8-1, R8-3,
  R8-4, R8-5 closed; six findings, all in identifier lookup and
  destinations, all admitted after checking: identifier equality picks the
  wrong repeated heading once an earlier document shifted the suffixes
  (R9-1); text-and-rank matching breaks when a note reference inside a
  heading resolves (R9-2); an explicit `{#introduction-1}` beside a
  generated one yields duplicate bookmarks (R9-3); the Markdown writer
  omits an explicit identifier equal to the generated one (R9-4); a span
  inside a heading has no LaTeX label (R9-5); a Div's "first inline-bearing
  block" skips opening code (R9-6). Redesign: a second parse of the same
  marked text with `-auto_identifiers` tells explicit identifiers from
  generated ones; explicit targets resolve exactly, generated block-level
  targets resolve by block path from a solo parse of identical structure,
  explicit targets inside notes by unique search, else text with a named
  warning; final identifiers are allocated per element; spans inside
  headings redirect to the heading; Div destinations go at the Div's real
  beginning; the split writes `{#S<n>}` identifiers (never equal to a
  generated one, so always emitted) and folds wrapper identifiers into the
  heading.
- 2026-10-27 review round 10 (`plan-4`, `ae6e568`): reopened; R8-2 and
  R9-1 to R9-6 closed; five findings, all admitted after checking: block
  paths into footnotes can reach a later document's redefinition of the
  note (R10-1: notes excluded from block paths, kind checked); the
  diagnostic parse did not reach Markdown-bodied notes (R10-2: the filter
  mirrors the outer reader extensions); a heading-first Div put its span
  inside the heading (R10-3: one last rule redirects any in-heading
  destination to the heading); identified links and inline code had no
  destination rule (R10-4: span before them); `S<n>` could collide with an
  existing explicit identifier (R10-5: reserve every identifier in the
  part first). Final identifiers use underscores for Word's bookmark
  rules. The reviewer confirmed the diagnostic parse keeps structure under
  all 32 extension combinations, `{#S1}` survives reader, writer, editor
  and all five writers, and generated identifiers from ASCII text are
  lowercase (non-ASCII capitals can appear, which the `S<n>` form does not
  rely on).

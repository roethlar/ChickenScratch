# Plan: Import Scrivener in the browser takes an upload

**Owner request (quote):**
> import scrivener needs to support uploads of scrivener files

(2026-10-25, after the reload-safety and phone work shipped.)

**Phase check:** [x] Allowed — a gap in the web shell's reference
implementation of an existing feature, no engine or format change  [x] Not
paused

**Invariants touched:** I2 (no format I/O outside `crates/core`: the upload
is staged in a temporary folder and handed to the existing converter and
service, which already write through core); I6 (a failed or refused upload
leaves nothing on disk and no half-made project); I7 untouched.

---

## [MODEL] Intent

Today the web version's **Import Scrivener** button can only point at a
`.scriv` folder that someone has already copied under the projects folder on
the server. After this plan, the button takes the Scrivener project from the
device you are browsing on, like Import File already does: pick the `.scriv`
folder itself (desktop browsers) or a `.zip` of it (phones, or any browser),
give the new project a name, and the server converts it. The copy-it-to-the-
server-first path stays as a third choice for projects too big to upload.

## [MODEL] Approach

### Slice 1 — the server takes a `.scriv` upload

`POST /api/import-scrivener-upload` (`crates/web/src/scriv_upload.rs`), next
to the existing `/api/import-upload`:

- Multipart fields: `outputPath` (text, checked with the gate's `creatable`
  rule exactly as the `import_scrivener` command does), then one or more
  `file` fields. One field whose name ends in `.zip` is an archive of the
  `.scriv` folder; otherwise each field is one file of the folder and its
  filename is the path inside it (`Draft.scriv/Files/Data/<id>/content.rtf`),
  which is how a browser folder pick sends files.
- Every file is streamed to a temporary folder outside the projects root as
  it arrives, never held whole in memory. Relative paths are checked piece
  by piece: no `..`, no empty or absolute parts, no control characters; a
  bad one fails the whole request with 400 and the folder is removed. Zip
  entries go through the same check (zip-slip), using the archive's
  enclosed-name rule; the archive itself is read with the `zip` crate
  (deflate only), a new dependency of `crates/web` only.
- The `.scriv` root is the parent of the `.scrivx` file, looked for at the
  top of the staged tree and one folder down (a zip made from inside the
  folder has it at the top; one made of the folder has it one down). No
  `.scrivx` → 400 "This upload is not a Scrivener project", before anything
  touches the projects folder or Pandoc.
- The request then runs the existing `project::import_scrivener` service on
  the staged root (same write lock, same token warm-up, same Pandoc
  resolution) and answers with the same `{ project, warnings }` the command
  answers. The temporary folder is deleted when the request ends, success or
  not.
- The route gets its own body limit, 512 MiB, above the 50 MiB default used
  elsewhere, because research folders hold PDFs and images. Over the limit,
  axum answers 413 and the UI says the project is too large to upload and
  points at the copy-to-server path.
- Not under the lease guard: no project is open while importing, and the
  new project's lease is claimed when the UI opens it, as today.

### Slice 2 — the browser offers the three ways in

`Welcome.tsx` (web branch only; desktop untouched):

- **Import Scrivener** opens a small chooser dialog (`ImportScrivenerDialog`,
  under `components/welcome/`): **Choose the .scriv folder** (a hidden
  `<input type="file" webkitdirectory>`; hidden on browsers that lack
  folder picking, which includes iPhone Safari), **Upload a .zip of it**
  (`<input accept=".zip">`), and **It is already on the server** (the
  existing server folder picker). A one-line hint says how to make the zip
  (right-click → Compress on a Mac, Send to → Compressed folder on Windows,
  long-press → Compress in the iPhone Files app).
- After the pick, the name prompt runs as today (default from the folder or
  archive name without `.scriv`/`.zip`); then `importScrivenerUpload(files,
  outputPath)` in `commands/web.ts` posts the multipart body. A folder pick
  skips files that are not part of the project data (`.DS_Store`,
  Scrivener `*.scriv/Settings/ui*.plist` are harmless and kept; only
  `.git` folders and empty files are dropped, so a zipped copy and a folder
  pick send the same tree).
- The store gains `importScrivenerUpload`, sharing the settle code with
  `importScrivener` (open the new project, surface warnings, forget nothing
  on failure). While uploading, the Welcome buttons are disabled and the
  chooser shows "Uploading <size>… this can take a minute on a phone".
- User guide: the "Import uploads" bullet of **What is different in a
  browser** and the "On a phone" subsection say which way to use; the
  troubleshooting entry covers the too-large case.

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `crates/web/src/scriv_upload.rs` (new), `app.rs`, `Cargo.toml` (+ `zip`), `Cargo.lock` | staging, path checks, zip extraction, `.scrivx` lookup, the handler, its own body limit |
| `crates/web/tests/http.rs` | the HTTP tests below |
| `ui/src/commands/web.ts`, `ui/src/stores/projectStore.ts` | `importScrivenerUpload` |
| `ui/src/components/welcome/ImportScrivenerDialog.tsx` (new), `Welcome.tsx`, `editor.css` | the three-way chooser |
| `ui/src/test/welcomeWeb.test.tsx`, `ui/src/test/importScrivenerDialog.test.tsx` (new) | the UI tests below |
| `docs/USER_GUIDE.md`, `docs/plans/README.md`, `.agents/state.md` | records |

## [MODEL] Tests

- [ ] `crates/web` unit: a relative path with `..`, an absolute part, or a
      control character is refused; `Draft.scriv/Files/Data/x/content.rtf`
      lands under the staging folder; the `.scrivx` is found at the top and
      one folder down, and absent → the "not a Scrivener project" error.
- [ ] `crates/web/tests/http.rs`: a zip whose entry climbs out → 400 and
      nothing written; a folder upload without a `.scrivx` → 400 and no
      `.chikn` created; `outputPath` outside the root → 403; a two-file
      folder upload (minimal `.scrivx` + one `content.rtf`) → 200 with the
      document in the answer and the `.chikn` under the root, and the same
      project sent as a zip gives the same documents. The two 200 cases run
      only where Pandoc is installed (this box has it; CI has not) and print
      a skip line otherwise; the refusals run everywhere.
- [ ] UI: the chooser renders the three choices and hides the folder choice
      when `webkitdirectory` is unsupported; picking a zip asks for a name
      and calls `importScrivenerUpload` with the file and
      `<root>/<name>.chikn`; the server-path choice still calls the old
      `importScrivener` with the picked folder; cancelling the name prompt
      sends nothing.
- [ ] Declared suite (`.agents/repo-guidance.md`), Tauri crate excepted on
      this box (Known drift); CI on push.

## [MODEL] Owner verification (plain English)

1. On the computer, in the browser: **Import Scrivener → Choose the .scriv
   folder**, pick a Scrivener project, accept the name. The converted
   project opens, and it is listed under Projects on this server.
2. On the phone: in Files, long-press the `.scriv` folder → Compress; then in
   the app, **Import Scrivener → Upload a .zip of it**, pick the zip. Same
   result.
3. A zip of something that is not a Scrivener project is refused with a plain
   message and nothing appears in the project list.

## [YOU] Decisions needed

- None blocking. Taken: the upload limit is 512 MiB (bigger projects use the
  existing copy-to-server path, which stays); zip and folder pick are both
  offered because phones cannot pick folders and desktop browsers can.

## Status

- 2026-10-25: drafted. Awaiting the owner's go.

# plan-4: Review of PLAN_SCRIVENER_UPLOAD.md

**Type**: plan review (the doc-review adaptation used for plan-1..plan-3:
no branch, no guard proof; the reviewer verifies the plan's factual claims
against the code, the installed tools and the third-party formats' sources)
**Status**: In progress — rounds 1 to 5 reopened (2026-10-27); fifth
revision committed, round 6 pending
**Subject**: `docs/plans/PLAN_SCRIVENER_UPLOAD.md`
**Owner go**: "set the push policy to always, push, then go" (2026-10-27),
after the two plan decisions were answered the same day ("let's keep it
.zip, .7z, .tar.gz or some supported archive in both for consistency";
"for a first pass, let's stick with formats we can easily validate for
free").

## Round 1 dispatch

- **Reviewer**: codex-cli 0.164.0 (the MCP codex server failed to connect
  this session), `codex exec --ephemeral -s workspace-write --add-dir /tmp
  -C <disposable clone> -m gpt-6-astra -c model_reasoning_effort=xhigh -o
  <file> -` with the prompt on stdin; the clone was a fresh `git clone` of
  the repository at the pinned head under the session scratchpad.
  Transcript header: `model: gpt-6-astra`, `provider: openai`, `reasoning
  effort: xhigh`, `sandbox: workspace-write [workdir, /tmp]`, session
  `01a17dcf-fb73-7ca1-a5fe-10743d8712f2`.
- **Tier**: frontier (owner-named pair gpt-6-astra @ xhigh, carried over
  from plan-3; grade word not stated by the owner)
- **Reviewed SHA**: `4b30022787067e5c8652d323e8ca0e8874baf69c` (master; the
  plan's last revision is `b985135`)
- **Base SHA**: `fe0ffbf8254d833f0b94a54e97632449c2473c85` (plan absent)
- **Dispatched**: 2026-10-27 14:42 UTC; finished 14:55 UTC; exit 0; 203,259 tokens.
- **Capability proof**: read the plan file and ran `git rev-parse HEAD` →
  `4b30022…`; both succeeded. The reviewer also ran the converter and web
  crate tests (all passing), a scratch probe against the services crate
  that reproduced findings 1 and 2, Pandoc 3.11 probes for findings 5 and
  7, and read the published sources of `sevenz-rust`, `tar`, novelWriter
  v26.2, Manuskript and the TextBundle specification. No repository file
  changed; no remote contacted.
- **Verdict**: `reopened`, thirteen findings. The reviewer stated the two
  owner decisions remain accepted.

## Round 1 findings and dispositions

Each was checked against the working tree or the cited source before
acceptance (orchestrator, 2026-10-27).

| # | Finding | Severity | Checked | Disposition |
|---|---------|----------|---------|-------------|
| R1-1 | Both importers create the `.chikn` project before converting and leave it on failure | HIGH | `io.rs:231`, `converter/mod.rs:110` create first; no cleanup path | Admitted: `import_project` builds at a temporary sibling path and renames on success; existing destinations never touched |
| R1-2 | Markdown-folder import reads only top-level files; skips go to stderr | HIGH | `io.rs:233–236` filters `is_file`; `io.rs:338` `eprintln!` | Admitted: recursive, deterministic, warnings returned |
| R1-3 | Byte/entry ceilings do not bound header-driven allocations in `sevenz-rust` and `tar` | HIGH | crate sources as cited; `sevenz-rust` 0.6.1 is unmaintained, `sevenz-rust2` 0.24.1 is its fork | Admitted: 7z signature-header and dictionary pre-checks, tar header walk, one unpack at a time, wall-clock bound; `sevenz-rust2` |
| R1-4 | `C:name` and `\name` escape a naive join; case collisions | HIGH | Rust `PathBuf::push` semantics as cited | Admitted: own path grammar, both separators, device names, case-insensitive collision refusal |
| R1-5 | Cutting Markdown text at headings separates footnote references from definitions | HIGH | Pandoc emits note definitions at the end | Admitted: split in Pandoc's document model (JSON), write each part back |
| R1-6 | Detector returns no project root; `__MACOSX` sibling defeats single-folder unwrap; `find_scrivx_file` searches one directory | MEDIUM | `converter/mod.rs:213–225` | Admitted: `detect_project -> { kind, source_root }`, droppings ignored, multiple candidates refused |
| R1-7 | Reader names used as extensions; `.html` falls through to Markdown; list has 15 entries; AsciiDoc/DocBook missing | MEDIUM | `io.rs:158–171` has no `html` arm; `Binder.tsx:414–416` lists 15; `pandoc --list-input-formats` includes `asciidoc`, `docbook` | Admitted: one registry shared by validation and both choosers, generated UI list asserted equal |
| R1-8 | Chooser filters omit `.tgz` and `.tar` | MEDIUM | plan text | Admitted: one shared suffix list |
| R1-9 | novelWriter 26.x writes `content/<handle>.md` with a TOML `+++` header, not `.nwd` | MEDIUM | `novelwriter/core/document.py` at v26.2 | Admitted: current layout first; legacy `.nwd` read when its grammar is confirmed, else refused with guidance |
| R1-10 | Manuskript world data is `world.opml` (XML), not YAML-like | MEDIUM | `manuskript/load_save/version_1.py` | Admitted: layout restated from the source |
| R1-11 | TextBundle's text file is `text.*` typed by `info.json`, not a fixed `text.md` | MEDIUM | textbundle.org spec | Admitted: find by stem, read Markdown and plain text, warn on others, assets through core |
| R1-12 | A `.docx` (a zip) passes archive sniffing and gets "not a project" instead of the document message | LOW | zip container formats | Admitted: document-container detection before "not a project" |
| R1-13 | "iPhone Safari cannot pick a folder" is outdated (18.4) | LOW | WebKit 18.4 release notes as cited | Admitted: rationale now rests on the owner's consistency ruling |

Declined: none. Reviewer notes carried into the plan: licence compatibility
(all MIT or Apache-2.0); 7z symlink detection must come from the entry
attributes, hard links do not exist in 7z.

## Round 2 dispatch

- **Reviewer**: same harness and incantation, fresh session
  `01a17de2-8741-7d11-90eb-122406480b55`; transcript header `model:
  gpt-6-astra`, `reasoning effort: xhigh`, `sandbox: workspace-write`.
- **Tier**: frontier (T5 at the ceiling: fresh frontier session)
- **Reviewed SHA**: `da9d73ad1c48714e339a8404399f9d0438c93fd0` (first
  revision)
- **Base SHA**: `4b30022787067e5c8652d323e8ca0e8874baf69c` (round-1 head)
- **Dispatched**: 2026-10-27 15:02 UTC; finished 15:17 UTC; exit 0;
  196,601 tokens.
- **Capability proof**: read the plan and ran `git rev-parse HEAD` →
  `da9d73a…`; both succeeded. The reviewer reproduced a plain rename
  replacing an empty destination folder, built a tar whose PAX `size`
  record hid 2 MiB from a raw header walk, and ran Pandoc 3.11 probes for
  the footnote-label collision, section-wrapped headings, dropped metadata
  and missing DOCX media. It could not build a zip runtime probe offline
  (`typed-path` missing) and did not run the producing apps or the full
  suite. No repository file changed; no remote contacted.
- **Verdict**: `reopened`. Closed: R1-2, R1-7, R1-8, R1-9, R1-10, R1-11,
  R1-12, R1-13. Not closed: R1-1, R1-3, R1-4, R1-5, R1-6 (reasons below).

## Round 2 findings and dispositions

Each checked against the working tree or the cited source before
acceptance (orchestrator, 2026-10-27).

| # | Finding | Severity | Checked | Disposition |
|---|---------|----------|---------|-------------|
| R2-1 | `creatable` returns an existing path; a plain rename replaces an empty folder | MEDIUM | `gate.rs:106` falls through to `existing` | Admitted: service-level lock + existence check, no-replace rename (`RENAME_NOREPLACE` / `MoveFileExW` / `RENAME_EXCL`) |
| R2-2 | Returned `Project.path` names the temporary folder | HIGH | `writer.rs:195` sets the path at creation; `projectStore.ts:105` stores it | Admitted: re-read the project from the final path after the rename |
| R2-3 | 7z decodes encoded headers inside `Archive::read` before any check | HIGH | `sevenz-rust2` reader source as cited | Admitted: unpacking moves into a killable worker process with an OS memory limit; header checks stay as defence in depth |
| R2-4 | A PAX `size` record makes the pre-walk and `tar` disagree | HIGH | `tar` 0.4.46 `archive.rs:335`; reviewer's differential fixture | Admitted: PAX/global headers carrying `size` are refused |
| R2-5 | A timed-out `spawn_blocking` task cannot be stopped | HIGH | `upload.rs:83` pattern; Tokio docs | Admitted: the worker process is killed by the parent, which keeps the admission slot until exit |
| R2-6 | The grammar refused `Book/`, `./` and `./chapter.md` | HIGH | reviewer-generated archives | Admitted: trailing separator = directory entry, leading `./` dropped |
| R2-7 | Implied ancestor directories escape the collision check | MEDIUM | reasoning on case-folding | Admitted: registry of every ancestor, file/dir conflicts refused |
| R2-8 | A nested project beside a README falls to the Markdown fallback | HIGH | plan text | Admitted: markers discovered first anywhere to depth 6, ambiguity refused, fallback only when no marker exists |
| R2-9 | Per-chapter `[^1]` labels collide when compile concatenates | HIGH | `compile.rs:124` joins sections as text | Admitted: imported notes become the editor's footnote elements (Lua filter), no labels written |
| R2-10 | Section-wrapped headings are not top-level `Header`s | MEDIUM | Pandoc 3.11 HTML reader | Admitted: hoist top-level Divs holding a level-one header |
| R2-11 | Cross-chapter `#id` links keep pointing into the wrong document | MEDIUM | reasoning | Admitted: identifier map, rewrite to the converter's document-link form |
| R2-12 | The non-standalone Markdown writer drops `meta` | MEDIUM | Pandoc 3.11 | Admitted: title/author/date into Front matter, full metadata into its sidecar |
| R2-13 | Embedded DOCX images are never extracted | HIGH | `io.rs:177` runs Pandoc without `--extract-media` | Admitted: `--extract-media`, attachments through core, targets rewritten (split or not) |
| R2-14 | `.doc` was removed from the one list both choosers read | MEDIUM | plan text | Admitted: registry entries are handlers; `.doc` keeps a preprocessing handler |

Declined: none. Reviewer note carried into the plan: the zip entry-count
check must read the EOCD/ZIP64 record the crate actually selects.

## Round 3 dispatch

- **Reviewer**: same harness and incantation, fresh session
  `01a17df5-d49f-7283-9904-c991c6d92c5a`; transcript header `model:
  gpt-6-astra`, `reasoning effort: xhigh`, `sandbox: workspace-write`.
- **Tier**: frontier (T5 at the ceiling: fresh frontier session)
- **Reviewed SHA**: `fa9a445a9bc8df71b855a093033563c8c9f90aac` (second
  revision)
- **Base SHA**: `da9d73ad1c48714e339a8404399f9d0438c93fd0` (round-2 head)
- **Dispatched**: 2026-10-27 15:23 UTC; finished 15:45 UTC; exit 0;
  141,420 tokens.
- **Capability proof**: read the plan and ran `git rev-parse HEAD` →
  `fa9a445…`; both succeeded. The reviewer ran the converter, services
  and web tests (171 passing), reproduced the Cargo dependency cycle in a
  scratch workspace, `RENAME_NOREPLACE` versus plain rename on Linux, a
  valid small-file PAX `size` record, the editor's image loss on a
  TipTap/Markdown round trip with the locked dependency versions, and
  Pandoc 3.11 probes for plain-text note bodies, section identifiers,
  unresolved document links and unfetched project images. macOS and
  Windows behaviour was read from source, not run. No repository file
  changed; no remote contacted.
- **Verdict**: `reopened`. Closed: R1-4, R1-5, R1-6, R2-2, R2-4, R2-5,
  R2-6, R2-7, R2-8, R2-9, R2-10, R2-12, R2-14. Not closed: R1-1, R1-3,
  R2-1, R2-3, R2-11, R2-13 (each mapped to a round-3 finding below).

## Round 3 findings and dispositions

Each checked against the working tree or the cited source before
acceptance (orchestrator, 2026-10-27).

| # | Finding | Severity | Checked | Disposition |
|---|---------|----------|---------|-------------|
| R3-1 | A worker in `services` called from the converter CLI is a Cargo cycle | HIGH | `crates/services/Cargo.toml` depends on `chikn-converter` | Admitted: worker lives in the converter crate; services supervises |
| R3-2 | `pre_exec` `RLIMIT_AS` applies to the parent's image; XNU refuses lowering it below current mappings | HIGH | XNU `kern_resource.c` as cited (source reading) | Admitted: the worker sets its own limit after exec; Windows Job Object unchanged |
| R3-3 | Publication (rename and cleanup of a `.chikn`) outside core breaks I2 | MEDIUM | `docs/INVARIANTS.md` I2; core has no publication API | Admitted: `core::project::publish_imported`; phase check and invariants updated |
| R3-4 | The plain-rename fallback restores the replacement race | MEDIUM | Linux probe: plain rename replaced an empty folder | Admitted: fail safely, no fallback |
| R3-5 | A failure after the rename leaves a published project behind a "failed" result | MEDIUM | `reader.rs:426–444` fallible reads | Admitted: read before the commit, rename is the commit, `ImportedButNotOpened` after it |
| R3-6 | A depth-6 search cannot establish "no marker anywhere" | HIGH | plan text | Admitted: walk the whole entry list |
| R3-7 | Small files carry PAX `size` records too | MEDIUM | reviewer's five-byte fixture | Admitted: accept when equal to the raw size |
| R3-8 | `data-body` is plain text; formatted notes flatten | HIGH | `cs-footnotes.lua:51–80`, `FootnoteNode.ts:17`; the Scrivener converter already writes plain-text bodies | Admitted: optional `data-format="markdown"`, backward compatible |
| R3-9 | Hoisting a `<section id>` wrapper loses its identifier | MEDIUM | Pandoc 3.11 HTML reader | Admitted: anchor span at the heading; map every identifier |
| R3-10 | Compile leaves document links as literal paths | MEDIUM | `compile.rs:124–153` | Admitted: slice 4, document anchors and link resolution in core compile |
| R3-11 | The editor has no image node and drops images on save | HIGH | `Editor.tsx:355–384`; reviewer's round trip | Admitted: slice 4, image node and per-shell asset resolver (pre-existing loss, fixed here) |
| R3-12 | Compile runs Pandoc in the temp folder without a resource path | HIGH | `compile.rs:136–153,190` | Admitted: slice 4, `--resource-path=<project root>` |

Declined: none. Scope note: round 3 moved three engine additions into the
plan (publication API, compile link and image resolution, Markdown-bodied
notes); the phase check now says so instead of "no engine change".

## Round 4 dispatch

- **Reviewer**: same harness and incantation, fresh session
  `01a17e0e-5484-7f30-bd0c-efc176e0805b`; transcript header `model:
  gpt-6-astra`, `reasoning effort: xhigh`, `sandbox: workspace-write`.
- **Tier**: frontier (T5 at the ceiling: fresh frontier session)
- **Reviewed SHA**: `20ffe17496b60e8f8fa9756d2ca0d8a582c09cd4` (third
  revision)
- **Base SHA**: `fa9a445a9bc8df71b855a093033563c8c9f90aac` (round-3 head)
- **Dispatched**: 2026-10-27 15:50 UTC; finished 16:07 UTC; exit 0;
  181,564 tokens.
- **Capability proof**: read the plan and ran `git rev-parse HEAD` →
  `20ffe17…`; both succeeded. The reviewer ran Pandoc 3.11 probes
  (`--resource-path` embedding the image in DOCX and ODT from another
  directory; the Markdown writer's `` `…`{=html} `` output for a raw
  footnote element; duplicate heading identifiers across concatenated
  documents), locked-editor round trips (`[]{#id}` flattened, a relative
  `.md#` link dropped, an empty identified span discarded), the PAX
  fixtures and a Linux `RENAME_NOREPLACE` probe. macOS limits were read
  from XNU source; the Tauri shell and the proposed Image dependency were
  not built. No repository file changed; no remote contacted.
- **Verdict**: `reopened`. Closed: R2-1, R3-1, R3-3, R3-4, R3-6, R3-7,
  R3-8, R3-12. Not closed: R1-1, R1-3, R2-3, R2-11, R2-13, R3-2, R3-5,
  R3-9, R3-10, R3-11 (each mapped to a round-4 finding below).

## Round 4 findings and dispositions

Each checked against the working tree or the cited source before
acceptance (orchestrator, 2026-10-27).

| # | Finding | Severity | Checked | Disposition |
|---|---------|----------|---------|-------------|
| R4-1 | A fresh macOS worker maps the 6 GiB shared region, so `RLIMIT_AS` 1 GiB is refused | HIGH | XNU `shared_region.h`, `vm_map.c` as cited (source reading) | Admitted: Linux `RLIMIT_AS` + watchdog, macOS parent-side RSS watchdog, Windows Job Object via `CreateProcessW`; dispatch in a sync `main`; macOS/Windows runs required before done |
| R4-2 | `ImportedButNotOpened` contradicted the `{ project, warnings }` contract and the store's settle code | MEDIUM | `projectStore.ts:105` reads `project.name` | Admitted: tagged `ImportOutcome` through services, both transports and the store |
| R4-3 | The Markdown writer emits a raw-HTML inline as `` `…`{=html} `` | HIGH | Pandoc 3.11 probe | Admitted: `markdown-raw_attribute`; element emitted as three inlines as the reader would produce |
| R4-4 | The editor flattens `[]{#id}`, drops relative `.md#` links and empty identified spans | MEDIUM | reviewer's locked-dependency round trips; `Editor.tsx:356` Link config | Admitted: `AnchorNode`; `Link.isAllowedUri` accepts project-relative targets; click opens the document |
| R4-5 | A global fragment lookup lands on the first of two `#introduction`s; `doc-<stem>` collides across folders | MEDIUM | Pandoc identifier disambiguation `introduction-1` | Admitted: per-file compile with `--file-scope` (verified first thing in the slice; Lua fallback) |
| R4-6 | The desktop asset protocol is not enabled and the CSP lists only one origin | MEDIUM | `tauri.conf.json:21`, `Cargo.toml:17` | Admitted: enable, runtime scope per open project, both origins in CSP; verified in the built shell |
| R4-7 | Encoding notes before the link walk hides their links from the rewrite | MEDIUM | filter order as written | Admitted: media/identifier/link rewriting first, note encoding last |
| R4-8 | Slice 3's acceptance criteria needed slice 4 | MEDIUM | plan text | Admitted: the consumer slice becomes slice 3 and lands first; Import File is slice 4 |

Declined: none. Reviewer notes carried into the plan: stable
`std::process` cannot provide the primary-thread handle needed to assign a
Job Object before resume, so the Windows launch uses `CreateProcessW`
directly; final-path token re-acquisition is necessary and recents use the
returned path; no other persisted creation-path dependency was found.

## Round 5 dispatch

- **Reviewer**: same harness and incantation, fresh session
  `01a17e21-d1c5-7430-aaa3-9a9c632e0298`; transcript header `model:
  gpt-6-astra`, `reasoning effort: xhigh`, `sandbox: workspace-write`.
- **Tier**: frontier (T5 at the ceiling: fresh frontier session)
- **Reviewed SHA**: `5ca2e2f54aa8d56243fb0b0efab1f71a53dace41` (fourth
  revision)
- **Base SHA**: `20ffe17496b60e8f8fa9756d2ca0d8a582c09cd4` (round-4 head)
- **Dispatched**: 2026-10-27 16:11 UTC; finished 16:23 UTC; exit 0;
  118,797 tokens.
- **Capability proof**: read the plan and ran `git rev-parse HEAD` →
  `5ca2e2f…`; both succeeded. The reviewer ran Pandoc 3.11 probes (the
  three-inline element under `markdown-raw_attribute` read back by Pandoc
  and the editor; the split writer's `` `…`{=html} `` output; `--file-scope`
  targets for `two.md`, `two.md#id`, `#id` and an excluded file; a native
  reference definition and a bare fragment link breaking under
  `--file-scope`), locked-editor probes (anchor node and `isAllowedUri`
  relative links preserved; a multi-paragraph `data-body` with the new
  attribute lost on the second open), a Linux post-exec `RLIMIT_AS` probe
  with a 2 GiB parent, and read the locked Tauri 2.10.3 scope source. No
  macOS/Windows runs; Tauri not built. No repository file changed; no
  remote contacted.
- **Verdict**: `reopened`. Closed: R1-1, R3-2, R3-5, R3-9, R4-2, R4-4,
  R4-7, R4-8. Not closed: R1-3, R2-3, R2-11, R2-13, R3-10, R3-11, R4-1,
  R4-3, R4-5, R4-6 (each mapped to a round-5 finding below).

## Round 5 findings and dispositions

Each checked against the working tree or the cited source before
acceptance (orchestrator, 2026-10-27).

| # | Finding | Severity | Checked | Disposition |
|---|---------|----------|---------|-------------|
| R5-1 | A 250 ms RSS watchdog bounds neither overshoot nor allocations freed between samples | HIGH | reasoning on the algorithm | Admitted: counting `#[global_allocator]` in worker mode, budget enforced before each allocation; OS limits supplementary |
| R5-2 | `FootnoteNode` writes literal newlines into the attribute; a blank line ends the inline HTML for the editor's parser on the next open | HIGH | `FootnoteNode.ts:21–37`; reviewer's reopen probe | Admitted: entity-encode the body (`&#10;`) in every writer, decode in `cs-footnotes.lua`; noted as pre-existing for Scrivener imports |
| R5-3 | The split command named plain `markdown`, restoring `` `…`{=html} `` | HIGH | plan text; Pandoc probe | Admitted: one shared `formats::MARKDOWN_WRITER` |
| R5-4 | Notes decoded after the file-scope rewrite lose their source document | MEDIUM | reasoning on filter order | Admitted (superseded with R5-5): notes decoded in the combined filter before the link walk; imported links are document-qualified |
| R5-5 | `--file-scope` breaks native cross-document reference definitions and bare fragment links | MEDIUM | reviewer's Pandoc 3.11 probe | Admitted: compile keeps the single combined parse; per-document solo parses matched by header position give the identifier map |
| R5-6 | Tauri's fs scope is additive; revoke-then-reallow is impossible | MEDIUM | Tauri 2.10.3 `scope/fs.rs` as cited | Admitted: app-owned `chikn-asset` scheme handler checking the open project per request |

Declined: none. Reviewer note carried into the plan: the store assertion
for `PublishedNotOpened` belongs with slice 2.

## Closeout

Pending: round 6 review of the fifth revision (same reviewer, fresh
session, pins base = round-5 head `5ca2e2f`, head = the revision commit).

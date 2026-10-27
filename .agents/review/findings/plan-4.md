# plan-4: Review of PLAN_SCRIVENER_UPLOAD.md

**Type**: plan review (the doc-review adaptation used for plan-1..plan-3:
no branch, no guard proof; the reviewer verifies the plan's factual claims
against the code, the installed tools and the third-party formats' sources)
**Status**: In progress — rounds 1 to 14 reopened (2026-10-27); the
deferral in the seventh revision was refused by the owner; fifteenth
revision committed, round 15 pending
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

## Round 6 dispatch

- **Reviewer**: same harness and incantation, fresh session
  `01a17e30-183e-7523-a4ee-309510cbf117`; transcript header `model:
  gpt-6-astra`, `reasoning effort: xhigh`, `sandbox: workspace-write`.
- **Tier**: frontier (T5 at the ceiling: fresh frontier session)
- **Reviewed SHA**: `c042cc56bf41b1479e0e8233f378d9c456be4290` (fifth
  revision)
- **Base SHA**: `5ca2e2f54aa8d56243fb0b0efab1f71a53dace41` (round-5 head)
- **Dispatched**: 2026-10-27 16:27 UTC; finished 16:42 UTC; exit 0;
  154,541 tokens.
- **Capability proof**: read the plan and ran `git rev-parse HEAD` →
  `c042cc5…`; both succeeded. The reviewer compiled a Rust probe of the
  counting allocator (infallible allocation aborts, `try_reserve` errors,
  disarmed mode passes), ran three editor parse/save cycles on an
  entity-encoded two-paragraph footnote with the locked dependencies, and
  ran Pandoc 3.11 probes: a cross-document native footnote shifting header
  positions, a `Span` anchor absent from header lists, duplicate explicit
  `{#target}` identifiers and a `doc-0` collision in DOCX bookmarks, a
  marker before a `%` title block, and Div/code-block/heading-span
  bookmark behaviour in DOCX. It noted that `Cargo.lock` does not yet hold
  the archive crates, so allocator coverage of the eventual decoder graph
  is not certifiable before they are added. No repository file changed; no
  remote contacted.
- **Verdict**: `reopened`. Closed: R1-3, R2-3, R2-13, R3-11, R4-1, R4-3,
  R4-6, R5-1, R5-2, R5-3, R5-4, R5-6. Not closed: R2-11, R3-10, R4-5, R5-5
  (each mapped to a round-6 finding below).

## Round 6 findings and dispositions

Each checked against the working tree or the cited source before
acceptance (orchestrator, 2026-10-27).

| # | Finding | Severity | Checked | Disposition |
|---|---------|----------|---------|-------------|
| R6-1 | A note defined in another document brings a heading into this one; header positions diverge between solo and combined parses | MEDIUM | reviewer's Pandoc 3.11 probe | Admitted: positional matching abandoned; each document parsed in its own run with the others appended behind a sentinel, giving exact provenance |
| R6-2 | The map covered headers only; the split's `Span` anchors and heading-less documents have no entry | MEDIUM | plan text | Admitted: every identified element is renamed and resolvable |
| R6-3 | Explicit duplicate identifiers survive the combined parse; `doc-0` collides with a native heading | MEDIUM | reviewer's DOCX bookmark probe | Admitted: all identifiers renamed `d<index>-<id>`, documents wrapped in `d<index>` Divs |
| R6-4 | A marker before the first document turns a `%` title block into text | MEDIUM | reviewer's Pandoc 3.11 probe | Admitted: no markers in the source; compile is unchanged unless document-qualified links exist; later documents parsed with the title block disabled |

Declined: none. Reviewer notes carried into the plan: a rejected fallible
allocation returns an error rather than aborting (the parent treats every
unsuccessful outcome alike); allocator coverage is evidenced by a counter
test once the crates are locked.

## Round 7 dispatch

- **Reviewer**: same harness and incantation, fresh session
  `01a17e43-024d-7a22-929d-f6e8ac3620f4`; transcript header `model:
  gpt-6-astra`, `reasoning effort: xhigh`, `sandbox: workspace-write`.
- **Tier**: frontier (T5 at the ceiling: fresh frontier session)
- **Reviewed SHA**: `e8570337752bead322ed622c8bd1f412bb513846` (sixth
  revision)
- **Base SHA**: `c042cc56bf41b1479e0e8233f378d9c456be4290` (round-6 head)
- **Dispatched**: 2026-10-27 16:48 UTC; finished 16:59 UTC; exit 0;
  88,671 tokens.
- **Capability proof**: read the plan and ran `git rev-parse HEAD` →
  `e857033…`; both succeeded. The reviewer ran Pandoc 3.11 probes of the
  sentinel inside unclosed Divs and code fences, duplicate reference and
  footnote definitions under reordering, auto-deduplicated fragments, each
  writer's treatment of identified Divs and code blocks (DOCX, HTML, EPUB,
  LaTeX, ODT), the `%` title block with a generated title page, generated
  separator identifiers, a 100-document parse (0.73 s, 277 MiB) and EPUB
  byte-instability. No repository file changed; no remote contacted.
- **Verdict**: `reopened`. Closed: R4-5, R6-1. Not closed: R2-11, R3-10,
  R5-5, R6-2, R6-3, R6-4 (all in the compile link design).

## Round 7 findings and dispositions

Each checked against the cited probe before disposition (orchestrator,
2026-10-27). All eight are admitted; seven are closed by removing the
compile link-resolution feature from this plan rather than by a further
redesign, a scope decision recorded in the plan's Decisions section and
flagged to the owner.

| # | Finding | Severity | Checked | Disposition |
|---|---------|----------|---------|-------------|
| R7-1 | The sentinel lands inside an unclosed Div or a code fence closed by appended text | HIGH | reviewer's Pandoc probe | Admitted; feature deferred |
| R7-2 | Moving a document first changes which duplicate definition wins | MEDIUM | reviewer's Pandoc probe | Admitted; feature deferred |
| R7-3 | Native `#introduction-1` links lose their target under renaming | MEDIUM | reasoning on the design | Admitted; feature deferred |
| R7-4 | ODT drops a code block's identifier and renders an identified Div as a section | MEDIUM | reviewer's writer table | Admitted; feature deferred (anchor `Span`s, which ODT does bookmark, stay for the editor's anchors) |
| R7-5 | The first document's `%` lines change meaning when the generated title page is on | MEDIUM | `compile.rs:108–121` | Admitted; feature deferred |
| R7-6 | Generated title and separator blocks escape the identifier namespace | MEDIUM | reviewer's DOCX bookmarks | Admitted; feature deferred |
| R7-7 | A hundred whole-project parses have no resource bound | HIGH | reviewer's measurement | Admitted; feature deferred |
| R7-8 | Byte-identical output is not a valid oracle (EPUB UUIDs) | LOW | reviewer's probe | Admitted: unchanged path asserted on Pandoc arguments and normalised structure |

**Owner ruling 2026-10-27, superseding the dispositions above.** The
owner refused the deferral: "a link resolving to plain text means this
cannot ship with honest import functionality as a feature since it's
broken. no." Compile link resolution returns to slice 3 in the eighth
revision with a design that leaves the Markdown Pandoc parses unchanged:
R7-1 (no sentinel; provenance by solo block counts, mismatch → named
warning, content never moved), R7-2 and R7-5 (the combined parse is
today's text, so definition precedence and title blocks are unchanged),
R7-3 (bare links are rewritten through the combined identifier they
already target), R7-4 (targets materialised as identified `Span`s, which
every writer emits), R7-6 (generated blocks renamed with a `g<n>-` prefix),
R7-7 (per-document solo parses only, four at a time, no whole-project
re-parse). A round 8 dispatched against the deferral revision (session
`01a17e51-354b-7171-a7d3-8a94dc4e0340`, 17:05 UTC) was stopped before a
verdict when the ruling arrived; it is not counted.

What this plan keeps from the earlier compile work unchanged:
`--resource-path` for project images (verified by the reviewer in round
4), anchor `Span`s and `data-format="markdown"` notes in the Lua filter. The reviewer's
observation that R1-3/R2-3/R4-1/R5-1 are closed only conditionally on
allocator coverage is now stated in the plan: the bound is claimed only
with coverage evidence, and a decoder without it is dropped.

Declined: none.

## Round 8 dispatch

- **Reviewer**: same harness and incantation, fresh session
  `01a17e58-2e49-70d1-be96-909d6847f9c7`; transcript header `model:
  gpt-6-astra`, `reasoning effort: xhigh`, `sandbox: workspace-write`.
- **Tier**: frontier (T5 at the ceiling: fresh frontier session)
- **Reviewed SHA**: `81f17bbb57e3695c945a5f5a6e29704927f8292c` (eighth
  revision; `0ff3ad6` reinstated the feature on the owner's ruling,
  `81f17bb` added the title-block mirror)
- **Base SHA**: `e8570337752bead322ed622c8bd1f412bb513846` (round-7 head;
  the refused `0e7ac5f` was reviewed by a stopped dispatch and is not a
  round)
- **Dispatched**: 2026-10-27 17:11 UTC; finished 17:26 UTC; exit 0;
  136,927 tokens.
- **Capability proof**: read the plan and ran `git rev-parse HEAD` →
  `81f17bb…`; both succeeded. The reviewer ran 138 block-boundary probes
  on Pandoc 3.11 (every requested ending and start, default/blank/rule
  separators, title page on and off), a writer-destination table (DOCX,
  ODT, HTML, EPUB, LaTeX for `Span`s in headings, paragraphs and standalone
  `Plain`s before tables, code, raw blocks, rules and Divs), the gate
  against core's loaded `Document.path` values and the converter's link
  form, option preservation through a JSON-mediated second run (metadata,
  `--standalone`, LaTeX variables, footnotes, reference docs), and a
  100-document timing (0.95 s combined, 22 s of solo parses at four
  concurrent). No TeX engine for PDF rendering. No repository file
  changed; no remote contacted.
- **Verdict**: `reopened`. Closed: R7-1 to R7-8, R5-5, R6-3, R6-4. Not
  closed: R2-11, R3-10, R6-2 (mapped to the findings below).

## Round 8 findings and dispositions

Each checked against the cited probe before acceptance (orchestrator,
2026-10-27).

| # | Finding | Severity | Checked | Disposition |
|---|---------|----------|---------|-------------|
| R8-1 | With an empty separator adjacent lists or code blocks merge, so block counts do not add up for valid documents | MEDIUM | reviewer's probe on compile's exact concatenation | Admitted: provenance by nonce'd begin/end comment markers written only when the gate fires; the merge difference is recorded |
| R8-2 | Solo and combined identifier sequences are not one-to-one (two same-named headings; a `{#linkid}` reference definition adds a `Link`) | MEDIUM | reviewer's Pandoc probes | Admitted: lookup by identifier in the section, then by kind, text and rank from the solo parse; the split writes explicit `{#id}` on linked headings |
| R8-3 | A `Span` inside a heading gives LaTeX `\hypertarget` but no `\label` | MEDIUM | reviewer's LaTeX output | Admitted: heading targets use the heading's own identifier |
| R8-4 | Materialised destinations were placed at the document's start | MEDIUM | plan text | Admitted: destinations placed at the target by a per-writer table; section start only for whole-document links |
| R8-5 | Identifiers inside notes were excluded from the map but renamed | MEDIUM | reviewer's fixture | Admitted: explicit identifiers inside notes searched across the document as the third rule |

Declined: none. Reviewer notes carried into the plan: the fence fixture
needs a closing fence in a later document (an unmatched opener alone is
text); duplicate footnote definitions and a `section`-named heading join
the fixtures; the gate uses loaded document paths, not identifiers.

## Round 9 dispatch

- **Reviewer**: same harness and incantation, fresh session
  `01a17e69-f87e-7061-8e54-5870e511ad95`; transcript header `model:
  gpt-6-astra`, `reasoning effort: xhigh`, `sandbox: workspace-write`.
- **Tier**: frontier (T5 at the ceiling: fresh frontier session)
- **Reviewed SHA**: `1b9fdcd2020b668a975c14a567a9a96f553c422f` (ninth
  revision)
- **Base SHA**: `81f17bbb57e3695c945a5f5a6e29704927f8292c` (round-8 head)
- **Dispatched**: 2026-10-27 17:30 UTC; finished 17:41 UTC; exit 0;
  107,422 tokens.
- **Capability proof**: read the plan and ran `git rev-parse HEAD` →
  `1b9fdcd…`; both succeeded. The reviewer re-ran the 138 boundary cases
  plus 30 new ones with the comment markers (markers arrive as top-level
  `RawBlock html` on every valid boundary; ordinary comments neither
  swallow nor impersonate them; adjacent lists and indented code blocks
  separate as the plan says; crossing fences and Divs are detected), ran
  the editor round trip on literal `{#id}` attributes (preserved), and
  reproduced the six findings below on Pandoc 3.11 and in DOCX, ODT, HTML
  and LaTeX output. No TeX engine. No repository file changed; no remote
  contacted.
- **Verdict**: `reopened`. Closed: R8-1, R8-3, R8-4, R8-5. Not closed:
  R8-2, R2-11, R3-10, R6-2 (mapped to the findings below).

## Round 9 findings and dispositions

Each checked against the cited probe before acceptance (orchestrator,
2026-10-27).

| # | Finding | Severity | Checked | Disposition |
|---|---------|----------|---------|-------------|
| R9-1 | Identifier equality picks B's first heading for `b.md#introduction-1` once A shifted the suffixes | MEDIUM | reviewer's Pandoc probe | Admitted: explicit identifiers (known from a `-auto_identifiers` parse) resolve exactly; generated ones by block path from a solo parse |
| R9-2 | Text-and-rank matching breaks when `# Intro[^n]` resolves in the combined parse | MEDIUM | reviewer's Pandoc probe | Admitted: block path, which reference resolution cannot shift |
| R9-3 | Explicit `{#introduction-1}` beside a generated `introduction-1` yields duplicate bookmarks after prefixing | MEDIUM | reviewer's DOCX/ODT/HTML/LaTeX output | Admitted: final identifiers allocated per element, not per name |
| R9-4 | The Markdown writer omits an explicit identifier equal to the generated one | MEDIUM | reviewer's Pandoc probe | Admitted: the split writes `{#S<n>}` (uppercase, never a generated form), so the attribute is always emitted; tests read the written Markdown |
| R9-5 | A hoisted anchor span inside a heading has no LaTeX label | MEDIUM | reviewer's LaTeX output | Admitted: wrapper identifiers folded into the heading's `{#S<n>}`; spans inside headings redirect to the heading |
| R9-6 | A Div's "first inline-bearing block" skips opening code; a code-only Div has none | MEDIUM | reviewer's ODT output | Admitted: destination at the Div's real beginning, `Plain`+`Span` before a block without inlines |

Declined: none.

## Round 10 dispatch

- **Reviewer**: same harness and incantation, fresh session
  `01a17e78-1252-7e63-8e4a-9d9a42369098`; transcript header `model:
  gpt-6-astra`, `reasoning effort: xhigh`, `sandbox: workspace-write`.
- **Tier**: frontier (T5 at the ceiling: fresh frontier session)
- **Reviewed SHA**: `ae6e5689666e4f8be129387934554296e7fb96a4` (tenth
  revision)
- **Base SHA**: `1b9fdcd2020b668a975c14a567a9a96f553c422f` (round-9 head)
- **Dispatched**: 2026-10-27 17:46 UTC; finished 17:56 UTC; exit 0;
  90,180 tokens.
- **Capability proof**: read the plan and ran `git rev-parse HEAD` →
  `ae6e568…`; both succeeded. The reviewer probed on Pandoc 3.11: the
  `-auto_identifiers` diagnostic parse keeps node structure under all 32
  combinations of the attribute-related extensions and leaves explicit
  identifiers on every element kind; solo and combined block positions for
  headings survive reference links, reference images and note references
  (a lone reference image changes `Para` to `Figure`); `{#S1}` survives the
  Markdown reader, the `markdown-raw_attribute` writer, two editor
  edit/save cycles and gives destinations in DOCX, ODT, HTML, EPUB and
  LaTeX; generated identifiers from non-ASCII text can carry uppercase
  letters (mathematical capitals), ASCII ones cannot; Pandoc writes `d0-0`
  into DOCX bookmark names unchanged (Word navigation not verified). No
  repository file changed; no remote contacted.
- **Verdict**: `reopened`. Closed: R8-2, R9-1 to R9-6. Not closed: R2-11,
  R3-10, R6-2 (mapped to the findings below).

## Round 10 findings and dispositions

Each checked against the cited probe before acceptance (orchestrator,
2026-10-27).

| # | Finding | Severity | Checked | Disposition |
|---|---------|----------|---------|-------------|
| R10-1 | A block path into a footnote reaches a later document's redefinition of that note | MEDIUM | reviewer's Pandoc probe | Admitted: notes excluded from block paths; kind of the matched element checked |
| R10-2 | The diagnostic reader options do not reach `pandoc.read` of Markdown-bodied notes | MEDIUM | reviewer's filter adaptation | Admitted: the filter mirrors `PANDOC_READER_OPTIONS.extensions` in nested reads |
| R10-3 | A heading-first Div places its span inside the heading: LaTeX `\hypertarget` without `\label` | MEDIUM | reviewer's LaTeX output | Admitted: a final rule redirects any in-heading destination to the heading |
| R10-4 | Identified links and inline code have no destination rule; hyperlinks without bookmarks | MEDIUM | reviewer's DOCX/ODT/LaTeX output | Admitted: an identified `Span` immediately before any such inline target |
| R10-5 | `S<n>` can collide with an existing explicit `{#S1}` in the part | MEDIUM | reasoning on the writer's output | Admitted: every identifier in the part, notes included, is reserved before allocation |

Declined: none. Carried into the plan: final identifiers use underscores
(`d0_0`) to match Word's bookmark naming rules; the uppercase argument is
stated for ASCII only.

## Round 11 dispatch

- **Reviewer**: same harness and incantation, fresh session
  `01a17e84-4c7d-7902-9a3b-d8ed4266e3a9`; transcript header `model:
  gpt-6-astra`, `reasoning effort: xhigh`, `sandbox: workspace-write`.
- **Tier**: frontier (T5 at the ceiling: fresh frontier session)
- **Reviewed SHA**: `0a329baf908d09af1e5d309b4e195ed132038386` (eleventh
  revision)
- **Base SHA**: `ae6e5689666e4f8be129387934554296e7fb96a4` (round-10 head)
- **Dispatched**: 2026-10-27 17:59 UTC; finished 18:06 UTC; exit 0;
  73,024 tokens.
- **Capability proof**: read the plan and ran `git rev-parse HEAD` →
  `0a329ba…`; both succeeded. The reviewer probed on Pandoc 3.11: the Lua
  `pandoc.read` form that honours `-auto_identifiers` (format table with
  `extensions`; the reader-options argument does not); block paths outside
  notes identical across 13 fixtures (reference-image `Figure`, definition
  lists, tables, lists, quotes, three nested Divs); heading redirects for
  heading-first Divs, spans, links and code in LaTeX, DOCX and ODT;
  adjacent-span destinations and `d0_0` identifiers in all five writers;
  `S<n>` reservation against identifiers inside notes. It read slice 3 end
  to end for contradictions. No repository file changed; no remote
  contacted.
- **Verdict**: `reopened` on one finding. Closed: R10-1 to R10-5, R2-11.
  Not closed: R3-10, R6-2 (both the finding below).

## Round 11 findings and dispositions

| # | Finding | Severity | Checked | Disposition |
|---|---------|----------|---------|-------------|
| R11-1 | A whole-document link to a section opening with a list, quote, definition list or line block, or to an empty section, has no destination rule | MEDIUM | reviewer's Pandoc probe; core includes definitions-only documents | Admitted: the section-start rule is total (`Plain`+`Span` before any first block without direct inlines; as the only block of an empty section) |

Declined: none. Carried into the plan: the working `pandoc.read` call
form.

## Round 12 dispatch

- **Reviewer**: same harness and incantation, fresh session
  `01a17e8c-f1ad-78e2-b213-a307b3072862`; transcript header `model:
  gpt-6-astra`, `reasoning effort: xhigh`, `sandbox: workspace-write`.
- **Tier**: frontier (T5 at the ceiling: fresh frontier session)
- **Reviewed SHA**: `92eca42d8ff785357cb0f9259ead2eb7760bd4ef` (twelfth
  revision)
- **Base SHA**: `0a329baf908d09af1e5d309b4e195ed132038386` (round-11 head)
- **Dispatched**: 2026-10-27 18:08 UTC; finished 18:17 UTC; exit 0;
  142,864 tokens.
- **Capability proof**: read the plan and ran `git rev-parse HEAD` →
  `92eca42…`; both succeeded. The reviewer ran 25 destination probes on
  Pandoc 3.11 (`Plain`+`Span` before a list, block quote, definition list,
  line block and as an empty section's only block, in DOCX, ODT, HTML,
  EPUB and LaTeX; all passed, all failed with the destination removed),
  confirmed the revised `pandoc.read` form, ran the locked editor on a
  bracketed span (flattened to `\[…\]{#spot}`), and read the whole plan
  end to end, all five slices. No repository file changed; no remote
  contacted.
- **Verdict**: `reopened` on three findings. Closed: R11-1, R3-10, R6-2
  (the compile design has no open finding).

## Round 12 findings and dispositions

| # | Finding | Severity | Checked | Disposition |
|---|---------|----------|---------|-------------|
| R12-1 | The split writes `[text]{#spot}` for identified spans; the editor flattens it to visible brackets on save | MEDIUM | reviewer's editor probe with the locked dependencies | Admitted: identified inlines a link points at are written as `AnchorNode` elements before the element |
| R12-2 | The Markdown-folder import copies links between files unchanged; `two.md#second` fails compile's exact-path gate | MEDIUM | `io.rs:271`; reviewer's DOCX relationship probe | Admitted: destination paths allocated first, links resolved relative to each source file and rewritten in Pandoc's model |
| R12-3 | Slice 3's tests named the splitter's writer and allocation, which slice 4 implements | MEDIUM | plan text | Admitted: handwritten fixtures stay in slice 3; splitter assertions move to slice 4 |

Declined: none. Carried into the plan: definition lists and line blocks
join the section-start fixtures.

## Round 13 dispatch

- **Reviewer**: same harness and incantation, fresh session
  `01a17e96-ca0c-78d2-b777-60c08c27b4e8`; transcript header `model:
  gpt-6-astra`, `reasoning effort: xhigh`, `sandbox: workspace-write`.
- **Tier**: frontier (T5 at the ceiling: fresh frontier session)
- **Reviewed SHA**: `5c8770391ed12f6cf8b2510c5b763195e6c29dcc` (thirteenth
  revision)
- **Base SHA**: `92eca42d8ff785357cb0f9259ead2eb7760bd4ef` (round-12 head)
- **Dispatched**: 2026-10-27 18:19 UTC; finished 18:26 UTC; exit 0;
  100,181 tokens.
- **Capability proof**: read the plan and ran `git rev-parse HEAD` →
  `5c87703…`; both succeeded. The reviewer ran 12 editor cases (anchor
  elements through two save/reopen cycles, read back as explicit `Span`s)
  and 60 writer-destination checks, the folder link rewrite (code literals
  and fragments kept, `../` and disambiguated names handled, gate passed
  against core-loaded paths), and found the footnote renumbering below with
  a Pandoc 3.11 compile probe. No repository file changed; no remote
  contacted.
- **Verdict**: `reopened` on one finding. Closed: R12-1, R12-2 (original
  failure), R12-3.

## Round 13 findings and dispositions

| # | Finding | Severity | Checked | Disposition |
|---|---------|----------|---------|-------------|
| R13-1 | The folder import's JSON → Markdown round trip renumbers footnotes to `[^1]`, so two files' notes collide when compile concatenates them | HIGH | reviewer's compile probe | Admitted: the shared note filter runs after the link rewrite; landing order 3, 1, 2, 4, 5 so the consumer slice is in place first |

Declined: none.

## Round 14 dispatch

- **Reviewer**: same harness and incantation, fresh session
  `01a17e9f-2ff8-7e52-a539-1504e38f38c7`; transcript header `model:
  gpt-6-astra`, `reasoning effort: xhigh`, `sandbox: workspace-write`.
- **Tier**: frontier (T5 at the ceiling: fresh frontier session)
- **Reviewed SHA**: `1a51d3aefb40158a952dc386f919179a151c9bf2` (fourteenth
  revision)
- **Base SHA**: `5c8770391ed12f6cf8b2510c5b763195e6c29dcc` (round-13 head)
- **Dispatched**: 2026-10-27 18:28 UTC; finished 18:34 UTC; exit 0;
  87,299 tokens.
- **Capability proof**: read the plan and ran `git rev-parse HEAD` →
  `1a51d3a…`; both succeeded. The reviewer reproduced the footnote
  collision without conversion and its absence with it, through two editor
  cycles and all five writers; confirmed the no-rewrite branch byte-exact;
  checked every listed test against the landing order 3, 1, 2, 4, 5 (no
  later-slice dependency); and found the anchor round-trip failure below
  with a scratch prototype on the locked editor. No repository file
  changed; no remote contacted.
- **Verdict**: `reopened` on one finding. Closed: R13-1.

## Round 14 findings and dispositions

| # | Finding | Severity | Checked | Disposition |
|---|---------|----------|---------|-------------|
| R14-1 | The folder import's rewrite writes an anchor element back as `[]{#spot .anchor}`, which the editor flattens on save | MEDIUM | reviewer's editor probe | Admitted: the shared inline-target normalisation runs in the folder path before note encoding, landed with slice 1 |

Declined: none.

## Closeout

Pending: round 15 review of the fifteenth revision (same reviewer, fresh
session, pins base = round-14 head `1a51d3a`, head = the revision commit).

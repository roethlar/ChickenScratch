# plan-4: Review of PLAN_SCRIVENER_UPLOAD.md

**Type**: plan review (the doc-review adaptation used for plan-1..plan-3:
no branch, no guard proof; the reviewer verifies the plan's factual claims
against the code, the installed tools and the third-party formats' sources)
**Status**: In progress — rounds 1 and 2 reopened (2026-10-27); second
revision committed, round 3 pending
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

## Closeout

Pending: round 3 review of the second revision (same reviewer, fresh
session, pins base = round-2 head `da9d73a`, head = the revision commit).

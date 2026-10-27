# plan-4: Review of PLAN_SCRIVENER_UPLOAD.md

**Type**: plan review (the doc-review adaptation used for plan-1..plan-3:
no branch, no guard proof; the reviewer verifies the plan's factual claims
against the code, the installed tools and the third-party formats' sources)
**Status**: In progress — round 1 reopened (2026-10-27); revision committed,
round 2 pending
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

## Closeout

Pending: round 2 review of the revised plan (same reviewer, fresh session,
pins base = round-1 head `4b30022`, head = the revision commit).

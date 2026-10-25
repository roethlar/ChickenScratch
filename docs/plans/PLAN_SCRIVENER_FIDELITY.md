# Plan: Scrivener import keeps the whole project

**Owner request (quote):**
> have we even perfected scrivener imports?

(2026-10-25, after the import-anything plan was drafted. The answer was no,
with evidence; the owner's go: "plan, review the plan with gpt-6-astra xhigh,
then begin once you have consensus. review code slices with the same codex
model".)

**Phase check:** [x] Allowed — `docs/CURRENT_PHASE.md` is "Engine hardening —
protect writers' work"; an import that loses paragraphs, footnotes, notes and
metadata loses writers' work  [x] Not paused

**Invariants touched:** I2 (every change lives in the converter crate and
the shells' command wrappers; the converter writes through core as today);
I6 (import never silently drops a document, a note, a comment or a footnote;
whatever cannot be converted is named in the warnings); I7 untouched.

---

## What the check found (2026-10-25, `chikn-converter` on `samples/*.scriv`)

All three samples are Scrivener 3 layout (`Files/Data/<UUID>/content.rtf`,
`.scrivx` `Version="2.0"`). Pandoc 3.11 on this box.

1. **One sample fails outright.** `Zoe 2.scriv`: Pandoc's RTF reader rejects
   one document at `Zo\'eb's` (a hex-escaped `ë` followed by an apostrophe)
   and the whole import aborts with the raw Pandoc error. No per-document
   fallback, no partial result.
2. **Paragraphs merge.** Scrivener (Cocoa RTF) ends each paragraph with a
   backslash and a newline, which the RTF spec defines as `\par`. Pandoc's
   reader treats it as a line break, so every paragraph of a document lands
   in one Markdown paragraph joined by hard breaks (`\` at line ends; `<br>`
   in HTML). Compile to Word or EPUB produces one paragraph per document.
3. **Apostrophes are escaped.** Pandoc's `markdown` writer has smart quotes
   on, so every `'` is written `\'` (`samples/Corn.chikn` shows it). The
   editor renders it, but the `.md` files, which are the product, carry it;
   `markdown-smart` writes plain apostrophes.
4. **Binder shape.** Everything outside the Draft folder that is not
   Research or Trash (Characters, Places, Notes, Front Matter, Template
   Sheets) is filed under Manuscript. Scrivener's Trash is dropped entirely
   (Corn: 4 of 20 text items were in the Trash; nothing live was lost, the
   earlier "empty documents vanish" reading was wrong). Two more defects
   the Codex review found in the code (no sample exercises them): a `Text`
   item's children are never converted, and `IncludeInCompile` is parsed
   but never applied.
5. **Not read at all** (by code; no sample exercises them): document notes
   (`notes.rtf`), inline comments and footnotes (`content.comments` plus
   `scrivcmt://` anchors in the RTF; inline `{\Scrv_annot}` / `{\Scrv_fn}`
   groups), synopses (`synopsis.txt`; the parser reads a `<Synopsis>`
   element Scrivener 3 does not write), labels, status and keywords (the
   parser reads `<Label>`/`<Status>`/`<Keywords>` names inside `MetaData`;
   Scrivener 3 writes `<LabelID>`, `<StatusID>` and `<KeywordID>` references
   into tables at the top of the `.scrivx`), custom metadata, snapshots,
   collections. Scrivener 2 layout (`Files/Docs/<N>.rtf`) is unsupported.

Facts about the Scrivener 3 side files above come from the format as it is
documented in the apps' own file layout and from the sample `.scrivx`
tables; the samples assign no labels and have no side files, so each slice
builds a hand-written fixture and the owner's own project is the final check.

## [MODEL] Intent

A writer who imports a Scrivener 3 project gets all of it: every document in
the right folder with its paragraphs intact, plain text that reads like the
original, the synopsis, label, status and keywords on each document, the
document notes, every inline comment and footnote as the editor's own
comments and footnotes, and the Trash kept as Trash. A document Pandoc cannot
read becomes plain text with a named warning instead of killing the import.

## [MODEL] Approach

### Slice 1 — the import survives bad RTF and keeps paragraphs and apostrophes

`crates/cli/src/scrivener/parser/rtf.rs` (+ new `rtf_prepare.rs`):

- **Prepare before Pandoc** (`rtf_prepare.rs`, a small RTF tokenizer that
  walks groups and control words, not a regex): rewrite `\` + newline to
  `\par `; rewrite each stand-alone `\'xx` hex escape to `\uN ?` using the
  file's `\ansicpg` (default 1252); for every `\uN` keep the code point and
  replace its fallback sequence (the next `\ucN` items of the current
  group, default 1, each a plain character, a `\'xx` escape or a `\\`, `\{`,
  `\}` escape) with one `?`, and emit every `\ucN` as `\uc1` so the reader
  and this pass agree on the count; `\ucN` is tracked per group like the
  spec says. **The emitted form is `\uN ?`, space then question mark**:
  measured on Pandoc 3.11, its reader takes whatever character follows the
  number as the delimiter, so `\u233?` eats the real space after the `?`
  (`caféfin`, review finding plan-3 R1-1 round 2) while `\u233 ?` gives
  `café fin` in Pandoc and in a spec-following reader alike; Pandoc does
  honour `\ucN` and its group scope (`\uc2` skips two). Hyperlink fields and
  every other group pass through untouched. Pure function, unit-tested on
  the prepared RTF **and**, where Pandoc is installed, on Pandoc's final
  text for the same inputs: the `\'eb's` shape, `\uc1` plus hex fallback
  (`café fin`, with its space), `\uc2` plus two fallbacks, `\uc0` with the
  delimiter space, a fallback that is itself an escaped brace, nested
  groups with different `\ucN`, and a hyperlink field.
- **Writer flags**: `-t markdown-smart --wrap=none` (plain apostrophes and
  quotes; curly quotes in the RTF stay curly as Unicode).
- **Per-document fallback** (`rtf_text.rs`): if Pandoc still fails, a
  built-in RTF text extractor yields plain paragraphs. It tracks
  destinations, not just starred ones: `{\*…}` groups are skipped except
  `\fldinst`, and the unstarred non-body destinations (`fonttbl`,
  `colortbl`, `stylesheet`, `info`, `pict`, `header*`, `footer*`,
  `listtable`, `listoverridetable`, `themedata` and the other tables) are
  skipped whole, so font names, colour tables and picture hex never reach
  the prose; `\binN` payloads are skipped by length. It honours `\par`,
  `\line`, `\tab`, `\'xx`, `\uN` with its fallback skip per `\ucN`, the
  named characters (`\emdash`, `\rquote`, …) and `\{ \} \\`. A `\field`
  group becomes the Markdown link `[result text](target)` so the
  `scrivcmt://` anchors slice 4 needs survive this path too. **The
  extracted text is serialised as literal text for Pandoc Markdown**, the
  dialect compile uses (`-f markdown`, all extensions on): **every ASCII
  punctuation character is backslash-escaped**, the same rule slice 4 uses
  for footnote bodies. No allow-list: a list of "dangerous" characters was
  wrong three rounds running (`&` entities, `$` math, `%` title blocks, `:`
  definition lists, `--`/`...`/quotes under smart punctuation). Pandoc
  accepts a backslash before any ASCII punctuation and renders the
  character itself, so `\.`, `\-`, `\"` and `\%` are all literal; the
  editor's Markdown parser follows CommonMark, which reads them the same
  way. The escaping happens before the generated links are inserted (link
  text escaped, targets not). The `.md` on disk is uglier for these
  fallback documents; the editor re-serialises cleanly on the next save,
  and the warning already says the document came in plain.
  The document is imported as that text and a warning names it: "<title>:
  formatting could not be read; text imported plain". The import as a whole
  succeeds. Pandoc missing entirely still fails up front as today.
- `samples/Zoe 2.scriv` imports (test, skipped when Pandoc is absent, which
  is CI); the Corn opening document has four Markdown paragraphs and no `\'`;
  a fixture whose RTF forces the fallback and carries a hyperlink field
  keeps the link.

### Slice 2 — the binder lands where Scrivener had it

`converter/mod.rs`:

- Only the Draft folder's contents become Manuscript. Every other top-level
  item outside Research and Trash goes under Research with its folder name
  kept (Characters, Places, Notes, Front Matter, Template Sheets).
- **`IncludeInCompile` is parsed today but never applied**: every imported
  document inherits the model default `true`
  (`converter/mod.rs` builds each `Document` with `..Document::default()`).
  Map `<IncludeInCompile>No</IncludeInCompile>` to `include_in_compile:
  false` on text documents and on folder-owned text; test that the flag is
  persisted and that compile output leaves the excluded document out.
- **Text items with children lose their children today**: the UUID-map
  pre-pass recurses into them but the conversion branch for `Text` does
  not, so a Scrivener text container's whole subtree is silently absent.
  Convert a text item with children the way a folder with text is already
  converted: a folder named by the item, the item's own text as that
  folder's first document, then the children in order. Test a text parent
  with both a text child and a folder descendant: content, order and
  presence after reopening.
- Trash items are converted like any other and placed in the project's
  Trash folder (nothing a writer parked is dropped). Media in Trash is
  copied the same way as research media.

### Slice 3 — synopsis, label, status, keywords, custom metadata, notes

`parser/scrivx.rs`, `converter/mod.rs`:

- Parse the `.scrivx` tables: `LabelSettings/Labels/Label[@ID]`,
  `StatusSettings/StatusItems/Status[@ID]`, `Keywords/Keyword[@ID]/Title`.
  Per item, resolve `MetaData/LabelID`, `StatusID` and
  `Keywords/KeywordID` to names; keep accepting the old name-form elements.
  Only Scrivener's empty sentinel (label or status ID `-1`, titled "No
  Label" / "No Status") imports as none. `DefaultLabelID` /
  `DefaultStatusID` is the writer's choice for new documents, often a real
  label; an item carrying that ID keeps it. Test both: the sentinel → none,
  a non-empty default → its name.
- Synopsis: `Files/Data/<UUID>/synopsis.txt` (plain text), falling back to
  the `<Synopsis>` element if present.
- Custom metadata: `MetaData/CustomMetaData/MetaDataItem` (`FieldID`,
  `Value`) with the field titles from the project's `CustomMetaDataSettings`
  table → `Document.fields["custom.<title>"]`. Two fields may share a title
  with different IDs; when titles collide, every colliding field is keyed
  by ID instead: the whole project switches to `custom.<id>` for every
  custom value, with the titles kept once as the map
  `fields["custom_field_titles"] = { <id>: <title> }`. IDs are unique by
  construction and no title key exists in that mode, so no generated key
  can collide with another. Test two same-titled fields with different
  values (both kept, titles recoverable) and the unique-title case.
- Document notes: `Files/Data/<UUID>/notes.rtf` through the same RTF path →
  Markdown in `Document.fields["notes"]`. The Inspector gains a **Notes**
  box bound to `fields.notes` (plain multi-line text, saved like the
  synopsis), so the imported notes are visible and editable, not buried.
- **Folders carry metadata too.** A `TreeNode::Folder` has only an id, a
  name and children (`crates/core/src/models/hierarchy.rs`), so a folder's
  synopsis, label, status, keywords, custom metadata or notes have nowhere
  to live unless the folder has a companion document. Today that document
  is created only when the folder's own text is non-empty. Change: create
  the folder's companion document (same name, first in the folder) whenever
  the folder has text **or** any of those side values, with empty content
  if there is no text. Fixture: a notes-only folder; after reopening, its
  notes, synopsis and label are on that document.
- Section type stays as it is; include-in-compile is mapped in slice 2.

### Slice 4 — inline comments and footnotes

- `Files/Data/<UUID>/content.comments` (`<Comments><Comment ID="…"
  Footnote="Yes|No" …>` with an RTF body) is parsed; bodies go through the
  RTF path to plain text.
- Anchors: Scrivener 3 marks the commented text as a hyperlink
  `scrivcmt://<ID>` in the RTF; Pandoc emits it as a Markdown link. After
  conversion, each `[text](scrivcmt://ID)` becomes
  `<span class="comment" data-comment-id="ID">text</span>` plus a
  `Comment { id, body, resolved: false }` for comments, or the editor's
  footnote element `<sup class="footnote" data-body="…">●</sup>` placed after
  the text for footnotes. An anchor with no matching comment is unwrapped
  and named in the warnings.
- **Footnotes must survive compile.** `FootnoteNode.ts` says the Rust side
  turns that element into a Pandoc footnote on compile, but
  `crates/core/src/core/compile.rs` has no such step: today every footnote,
  imported or typed, is lost from compiled output (its body is an attribute
  Pandoc never reads). This slice adds the step in core compile: before
  Pandoc runs, each `<sup class="footnote" data-body="…">…</sup>` becomes a
  **reference footnote** `[^cs-fnN]` with N unique across the whole compiled
  document, and its definition `[^cs-fnN]: …` is appended after the last
  section. The body (plain text, attribute-unescaped) is serialised as
  literal text: every ASCII punctuation character is backslash-escaped, and
  a body with blank lines becomes several paragraphs in the definition
  (continuation paragraphs indented four spaces, as Pandoc requires), so a
  `]`, a `<redacted>` or a two-paragraph note survive intact. Pandoc's
  inline form `^[…]` is not used: it is single-paragraph only and a `]` in
  the body ends it early. Tests: footnote bodies `A] B`, `<redacted> *x*`
  and a two-paragraph body each compile to Markdown with one reference note
  whose rendered text equals the body, and, where Pandoc is present, to
  DOCX whose `word/footnotes.xml` holds the body text and whose
  `word/document.xml` holds none of it; two footnotes get distinct ids; a
  document without footnotes compiles byte-identical to before.
- Anchors are gathered before any lossy step: the prepare step records each
  `scrivcmt://<ID>` field it passes, and after conversion (Pandoc or the
  fallback) every sidecar body is reconciled against the anchors found in
  the text. A body with no anchor is appended to the document's comments
  unanchored (text "Comment with no place in the text: …") and named in the
  warnings; an anchor with no body is unwrapped and named. Nothing in
  `content.comments` is dropped.
- Inline annotations and footnotes exist in two encodings and both are
  handled. Scrivener writes them as *literal text* inside the RTF (escaped
  braces, so other RTF apps show them): `{\Scrv_annot \color={\R=…\G=…\B=…}
  \text=…\end_Scrv_annot}` and `{\Scrv_fn=…\end_Scrv_fn}`. After
  conversion they are therefore plain text in the Markdown, where a pass
  turns each into a comment span or footnote element at that position.
  The raw-group form (unescaped braces, produced by other tools) is
  rewritten by the prepare step into the hyperlink form plus a synthesised
  body. Fixtures: a hand-written RTF for each encoding, with exact expected
  Markdown; a Scrivener-generated document carrying one of each is the
  owner's verification item, and if one is provided before the slice
  lands it joins the fixtures.

### Slice 5 — Scrivener 2 layout (optional, last)

`Files/Docs/<N>.rtf`, `<N>_notes.rtf`, `<N>_synopsis.txt`, `ID=` attributes,
name-form labels. One path-resolution function keyed by the `.scrivx`
version. Only if the owner has or expects such projects; otherwise the
import says plainly "This project was saved by Scrivener 2; open and save it
in Scrivener 3 first".

### Records

`samples/Corn.chikn` regenerated with the finished converter (the write-guard
test requires it to probe Full; the cross-frontend harness converts
`Corn.scriv` with a Pandoc shim, so the shim learns the new writer flag);
user guide "Imported Scrivener project looks wrong" rewritten; DEVLOG entry;
`.agents/state.md`.

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `crates/cli/src/scrivener/parser/rtf.rs`, `rtf_prepare.rs` (new), `rtf_text.rs` (new) | prepare step, `markdown-smart`, fallback extractor, per-document warning |
| `crates/cli/src/scrivener/parser/scrivx.rs` | label/status/keyword/custom-metadata tables and ID resolution; comments file parser |
| `crates/cli/src/scrivener/converter/mod.rs` | binder placement, Trash, synopsis/notes side files, comment and footnote anchors, warnings |
| `crates/core/tests/cross_frontend/run.sh` (shim) | accept the new Pandoc flag |
| `ui/src/components/inspector/*` | Notes box bound to `fields.notes` |
| `samples/Corn.chikn` | regenerated |
| `docs/USER_GUIDE.md`, `DEVLOG.md`, `docs/plans/README.md`, `.agents/state.md` | records |

## [MODEL] Tests

- [ ] prepare (exact prepared RTF, and exact Pandoc plain text when Pandoc
      is present): `\`+newline → `\par`; `\'eb's` → `\u235 ?'s` and reads
      `Zoë's`; `\'xx` under another `\ansicpg` is left alone;
      `\uc1 caf\u233\'e9 fin` → `\uc1 caf\u233 ? fin` and reads `café fin`;
      `\uc2` with two fallbacks → `\uc1 … \uN ?`, reads one character;
      `\uc0 caf\u233  fin` reads `café fin`; a fallback that is `\{`; nested
      groups with different `\ucN`; a hyperlink field passes through
      byte-for-byte.
- [ ] fallback extractor: an actual sample header (`fonttbl` with
      Palatino, `colortbl`, `{\*\expandedcolortbl}`) plus a `\pict`
      destination, two paragraphs, a tab, a hex escape and a `\uN` with
      fallback → two plain paragraphs with the right characters and no font
      names, colour tables or hex; a `\field` hyperlink → `[text](url)`;
      text containing `*these stars*`, `<redacted>`, `[brackets]`,
      `&copy;`, `$x$`, `^up^`, `@key`, a backslash, a first line
      `% 80% of the village had gone.`, a paragraph `Term` followed by
      `: definition`, and `"hello" -- paused...` each read back exactly as
      written after a round trip through Pandoc Markdown as compile uses it
      (reader, metadata and standalone options of `compile.rs`; exact
      rendered text; Pandoc present), and the editor's Markdown parser
      renders the same text.
- [ ] import of a fixture whose one document is unreadable RTF → project
      with the plain text, its hyperlink kept, and one warning naming the
      document; `Zoe 2` imports (Pandoc present).
- [ ] Corn opening document: four paragraphs, no `\'`, no trailing `\`.
- [ ] binder: Characters/Places under Research; Draft under Manuscript;
      Trash items in Trash with their text; a text item with a text child
      and a folder descendant keeps all three in order after reopening;
      `IncludeInCompile` No → `include_in_compile: false` persisted and the
      document absent from compiled output.
- [ ] metadata fixture: label, status, two keywords, synopsis.txt, a custom
      field and notes.rtf arrive on the document; sentinel label `-1` →
      none; a non-empty `DefaultLabelID` → its name; two same-titled custom
      fields keep both values.
- [ ] comments fixture: one comment and one footnote anchored by hyperlink
      → span + `Comment` and the footnote element; the same document through
      the forced fallback path gives the same anchors; a body with no anchor
      is kept unanchored and warned; an anchor with no body is unwrapped and
      warned; literal-text `Scrv_annot`/`Scrv_fn` → comment span and
      footnote element; raw-group form → the same.
- [ ] Inspector shows and saves the Notes box (UI test).
- [ ] Declared suite (`.agents/repo-guidance.md`), Tauri crate excepted on
      this box (Known drift); CI on push.

## [MODEL] Owner verification (plain English)

1. Import the Zoe 2 sample (or any project that used to fail): it opens, and
   any document that needed the plain-text fallback is named in the notice.
2. Open an imported chapter: paragraphs are separate paragraphs, apostrophes
   are plain apostrophes.
3. Characters and Places are in Research; the Trash folder holds what
   Scrivener's Trash held.
4. Import one of your own Scrivener 3 projects: labels, status, keywords,
   synopsis and notes show in the Inspector; comments and footnotes appear
   in the text as the editor's own.

## [YOU] Decisions needed

- **Where the Scrivener importer lives, in writing.** `docs/INVARIANTS.md`
  I2 and `docs/adr/ADR-001` say Scrivener import logic lives in
  `chickenscratch-core`; I2's own table, `docs/ARCHITECTURE.md` and the code
  put the converter in `crates/cli` (`chikn-converter`) calling the engine,
  and every accepted Scrivener fix so far (PLAN_IMPORT_MEDIA_SAFETY, review
  findings ar-1..ar-4) landed there. This plan follows the code and
  ARCHITECTURE: the converter crate, writing through core. The wording in
  INVARIANTS.md and ADR-001 needs the owner's ruling and a one-line fix;
  nothing in this plan moves code between crates.
- Taken: non-Draft folders go under Research (not Manuscript); Trash is
  kept; document notes are stored as a generic `notes` field and shown in
  the Inspector; Scrivener 2 support is the optional last slice.

## Status

- 2026-10-25: drafted. Owner go covers implementation once the Codex review
  (gpt-6-astra, xhigh) reaches consensus.
- 2026-10-25 round 1 (`604791d`): reopened, eight findings, all admitted
  after checking the code (`.agents/review/findings/plan-3.md`): Unicode
  fallback corruption in the hex rewrite; text-item children not converted;
  `IncludeInCompile` never applied; inline annotations are literal text;
  the fallback dropped hyperlink anchors; the default label is not the empty
  label; same-titled custom fields collide; I2 wording contradiction. Plan
  revised as above.
- 2026-10-25 round 2 (`9a6b105`): reopened. Five round-1 findings resolved;
  R1-1 (the `\uN?` form still broke under Pandoc, measured and fixed to
  `\uN ?`), R1-7 (collision-free custom keys) revised again; R1-8 is an
  owner ruling, not a reviewer call, and is recorded under Decisions. Four
  new findings, all admitted: footnotes never survive compile (core compile
  step added to slice 4); notes-only folders had no persistence target
  (companion document rule in slice 3); fallback text must be escaped for
  Markdown; the fallback must skip unstarred non-body destinations.
- 2026-10-25 round 3 (`027aa63`): reopened on two items, both admitted: the
  escape list lacked `&` and `$` (Pandoc renders entities and TeX math);
  the inline footnote form `^[…]` breaks on a `]` or a second paragraph, so
  compile now emits reference footnotes with escaped, multi-paragraph
  bodies. Everything else resolved; R1-8 awaits the owner.
- 2026-10-25 round 4 (`5639711`): reopened on R2-3 alone (the allow-list
  still missed `%`, `:` and smart punctuation); the fallback now escapes
  every ASCII punctuation character, no allow-list. The reference-footnote
  step was checked against real DOCX output and accepted.
- 2026-10-25 round 5 (`c3e67b0`): **accepted**, no findings; the reviewer
  probed the escape rule through compile's Pandoc options and the editor's
  parser. Only R1-8 (owner ruling on the I2 wording) stays open; it moves no
  code either way, so implementation begins in the converter crate under
  the recommended reading (option A) and the owner can still rule B.
- 2026-10-25: slice 1 landed (`caf8a5e`). Its Codex review returned six
  findings (`.agents/review/index.md` s1-1..s1-6), each fixed in its own
  commit and re-reviewed; three repairs followed re-reviews. The I2 wording
  ruling (R1-8) is still with the owner.
- 2026-10-25: slice 2 landed (`0d813f9`, binder placement, Trash kept, text
  containers, IncludeInCompile); slice 3 landed (synopsis, label, status,
  keywords, custom metadata, notes, companion documents, Inspector Notes
  box). Slice 4 landed (comments and footnotes from `content.comments`
  and the inline forms; core compile emits reference footnotes). Codex
  reviews of slices 2–4 and of the s1-3/s1-4/s1-6 second repairs pending.
  Remaining: records (user guide, DEVLOG, `samples/Corn.chikn`
  regeneration); slice 5 (Scrivener 2) optional.

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
   earlier "empty documents vanish" reading was wrong).
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

- **Prepare before Pandoc**: rewrite `\` + newline to `\par `; rewrite each
  `\'xx` hex escape to `\uN?` using the file's `\ansicpg` (default 1252), so
  the sequence Pandoc trips on never reaches it; leave everything else.
  Pure function, unit-tested on the three failure shapes.
- **Writer flags**: `-t markdown-smart --wrap=none` (plain apostrophes and
  quotes; curly quotes in the RTF stay curly as Unicode).
- **Per-document fallback**: if Pandoc still fails, a small built-in RTF text
  extractor (skip `{\*…}` destinations, drop control words, honour `\par`,
  `\line`, `\tab`, `\'xx`, `\uN`, `\{ \} \\`) yields plain paragraphs, the
  document is imported as that text, and a warning names the document:
  "<title>: formatting could not be read; text imported plain". The import
  as a whole succeeds. Pandoc missing entirely still fails up front as today.
- `samples/Zoe 2.scriv` imports (test, skipped when Pandoc is absent, which
  is CI); the Corn opening document has four Markdown paragraphs and no `\'`.

### Slice 2 — the binder lands where Scrivener had it

`converter/mod.rs`:

- Only the Draft folder's contents become Manuscript. Every other top-level
  item outside Research and Trash goes under Research with its folder name
  kept (Characters, Places, Notes, Front Matter, Template Sheets). The
  Scrivener `IncludeInCompile` flag is already honoured, so compile output
  does not change for the Draft.
- Trash items are converted like any other and placed in the project's
  Trash folder (nothing a writer parked is dropped). Media in Trash is
  copied the same way as research media.
- Text items with children keep their children (already handled) and the
  parent text becomes the folder's first document, as today.

### Slice 3 — synopsis, label, status, keywords, custom metadata, notes

`parser/scrivx.rs`, `converter/mod.rs`:

- Parse the `.scrivx` tables: `LabelSettings/Labels/Label[@ID]`,
  `StatusSettings/StatusItems/Status[@ID]`, `Keywords/Keyword[@ID]/Title`.
  Per item, resolve `MetaData/LabelID`, `StatusID` and
  `Keywords/KeywordID` to names; keep accepting the old name-form elements.
  Scrivener's default label/status ("No Label", "No Status", or
  `DefaultLabelID`) imports as none.
- Synopsis: `Files/Data/<UUID>/synopsis.txt` (plain text), falling back to
  the `<Synopsis>` element if present.
- Custom metadata: `MetaData/CustomMetaData/MetaDataItem` (`FieldID`,
  `Value`) with the field titles from the project's `CustomMetaDataSettings`
  table → `Document.fields["custom.<title>"]`.
- Document notes: `Files/Data/<UUID>/notes.rtf` through the same RTF path →
  Markdown in `Document.fields["notes"]`. The Inspector gains a **Notes**
  box bound to `fields.notes` (plain multi-line text, saved like the
  synopsis), so the imported notes are visible and editable, not buried.
- Section type and include-in-compile stay as they are.

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
- Inline groups from older documents, `{\Scrv_annot …}` and `{\Scrv_fn …}`,
  are rewritten by the prepare step into the same hyperlink form with a
  synthesised comment, so both styles land identically.

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

- [ ] prepare: `\`+newline → `\par`; `\'eb` → `\u235?` under cpg 1252;
      `\'xx` under another `\ansicpg` maps by that page; braces and `\\`
      untouched.
- [ ] fallback extractor: a Cocoa RTF with two paragraphs, a tab, a hex
      escape and a `{\*\expandedcolortbl}` group → two plain paragraphs.
- [ ] import of a fixture whose one document is unreadable RTF → project
      with the plain text and one warning naming the document; `Zoe 2`
      imports (Pandoc present).
- [ ] Corn opening document: four paragraphs, no `\'`, no trailing `\`.
- [ ] binder: Characters/Places under Research; Draft under Manuscript;
      Trash items in Trash with their text.
- [ ] metadata fixture: label, status, two keywords, synopsis.txt, a custom
      field and notes.rtf arrive on the document; default label → none.
- [ ] comments fixture: one comment and one footnote anchored in the RTF →
      span + `Comment` and the footnote element; an orphan anchor is
      unwrapped and warned.
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

- None blocking. Taken: non-Draft folders go under Research (not Manuscript);
  Trash is kept; document notes are stored as a generic `notes` field and
  shown in the Inspector; Scrivener 2 support is the optional last slice.

## Status

- 2026-10-25: drafted. Review with Codex (gpt-6-astra, xhigh) pending; the
  owner's go covers implementation once that review reaches consensus.

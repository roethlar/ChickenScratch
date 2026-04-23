# Tier 2 — Writer Workflow

**Priority:** v1.2
**Status:** Planned
**Format impact:** Minimal — session targets go in `project.yaml`; everything else is UX
**Depends on:** Timeline view depends on Tier 1 scene `story_time`; others are independent

Four quality-of-life features that peer tools have and writers ask about. None change how the format works at a deep level; they're frontend-heavy additions that reuse the data model you already have (plus Tier 1 for timeline).

---

## 1. Scrivenings mode

### Why

Preview mode is read-only: the writer sees manuscript flow but can't fix anything from there. Scrivener's "Scrivenings" mode is the inverse — all selected documents concatenated in the editor with dividers, fully editable, saves split back to per-file. This is the workflow for revision passes: "I want to smooth the transitions across chapters 7–9 without switching docs eight times."

### UX

- Multi-select documents in the binder (Cmd/Ctrl+click, Shift+click).
- Toolbar gains a **Scrivenings** button (stacked-pages icon) and a menu item under View.
- Single-selection on a folder enters Scrivenings over all its scenes in order.
- Editor shows each document as a section with a visible divider (title + subtle horizontal rule). Inline editing works normally; saving writes each section back to its original file.
- Status bar: "Editing 4 documents · 8,214 words combined."
- Exit: click any single doc in the binder, or the X button in the header.

### Implementation

- **Frontend-only** for the initial pass — no core/Tauri changes needed.
- **Data flow:** Zustand store gains `scrivenings: Document[] | null`. When set, the editor renders a single TipTap instance whose doc is the concatenation of markdown bodies with separator markers (a custom node or an HTML comment `<!-- CHIKN_DOC_BOUNDARY id="..." -->`).
- **Save path:** split the TipTap markdown output at boundary markers, write each section back via `update_document_content`. Debounced like normal save.
- **Boundary preservation:** use invisible marker nodes in TipTap rather than visible HRs — that way writers can delete between boundaries without corrupting the split logic. Visual dividers are pure CSS on the marker nodes.

### Scope limits

- **Manuscript-only** first pass. Mixing research and manuscript in one scrivenings view gets confusing and has no clear use case.
- **No per-section metadata editing** — that still happens in the Inspector (it tracks the caret position and shows the containing document's meta).
- **Boundaries are immutable in-session:** writers can't drag a section up or reorder from inside scrivenings. Use the binder for that.

### Open questions

- When the writer deletes all content in one section, does the file stay (empty) or auto-delete on exit? Proposal: stays. Writer intent around empty files is ambiguous; keep it and let them delete explicitly.
- What about comments/footnotes that span a section? Proposal: shouldn't be possible — comments anchor to a text range, boundary nodes are opaque terminators.

---

## 2. Session targets with deadlines

### Why

The writing history chart shows yesterday's count but says nothing about tomorrow's goal. Scrivener's Project Targets panel is a beloved motivator — "write 1,500 today, 87 days to deadline, 72,000 words left." Same data model as our existing per-doc targets; new surface.

### Format changes

Small additions to `project.yaml`:

```yaml
session_target:
  words_per_session: 1000      # optional daily goal
  deadline: 2025-12-31         # optional; ISO date
  total_target: 90000          # optional; full manuscript target
```

All optional; absent = feature disabled for that project.

### UX

- **Settings gains a "Targets" sub-section** under Writing (or at the project level — see open questions): fields for words/session, deadline, total target.
- **Floating progress badge** in the editor corner while writing (bottom-right, auto-hides after 3s of inactivity): `"Today: 428 / 1000 · 54 days left · 1,205/day needed"`. Clicking it opens the full stats panel.
- **Welcome screen card** per recent project: shows "Today: 428/1000" as a progress ring.
- **Stats panel** gains a "Session target" section above the daily history chart: today's progress bar, deadline countdown, required daily average.

### Implementation

- **Rust core:** add `SessionTarget` to `Project` struct; serialize/deserialize in project.yaml.
- **Tauri command:** `get_session_progress(project_path) -> SessionProgress { today_words, target, days_remaining, needed_per_day }`. Reuses existing daily-history data plus live word counting.
- **UI:** new `components/stats/SessionBadge.tsx` (editor corner) and `components/stats/SessionSection.tsx` (stats panel).

### Scope limits

- **Target is words, not pages or chapters.** Pages vary too much by font; chapters by arbitrary length.
- **Single deadline per project.** No milestones, no phase targets. v1.3 if writers ask.
- **No motivational nagging.** No "You missed yesterday!" toasts. The badge is ambient; the writer reads it or doesn't.

### Open questions

- Project-level vs per-user targets? Proposal: project-level. A short story's target is different from a novel's; storing on the project makes it right for that project regardless of which machine you open it on.
- Counting rule: cumulative new words vs net (minus deletions)? Proposal: net. Writers often re-write; cumulative count rewards churn.

---

## 3. Per-document snapshots

### Why

Revisions are project-wide: a Save Revision is a commit of everything. Writers also want the narrower operation — "what did just *this scene* look like yesterday?" Scrivener has per-doc snapshots as a separate mechanism (they pre-date git). We already have the data (every scene's history is in git); we just haven't surfaced it per-file.

### UX

- **Binder context menu gains "File History…"** on any document. Opens a panel showing revisions that touched this file (newest first): message, date, word-diff preview on hover.
- **Restore this document** button per revision: writes the file content from that commit, stages, commits with message "Restored 'Chapter 3' to <short-id>". Non-destructive (standard restore pattern).
- **Inline mini-history** in the Inspector: last three revisions that touched the active document, clickable to expand the full panel.

### Implementation

- **Rust core:** new `document_history(project_path, doc_path) -> Vec<Revision>` — `git log -- <path>`, map each commit to a Revision. Already-close helper: extend `list_revisions` with an optional `path` filter.
- **Rust core:** new `restore_document(project_path, doc_path, commit_id)` — read the file from that tree via `repo.find_commit(oid).tree().get_path().peel_to_blob()`, write to disk, call `save_revision`.
- **Tauri:** `document_history`, `restore_document` wrappers.
- **UI:** new `components/revisions/DocumentHistory.tsx` modal.

### Scope limits

- **Markdown documents only.** Binary files (images, PDFs) in research are out of scope — git handles them but diffing them is meaningless.
- **No cherry-pick across drafts** — restoring always operates on the current branch's history.

### Open questions

- Should the word-diff preview render on hover or only on click? Proposal: click — word-diffing every commit on hover is expensive for long scenes.

---

## 4. Timeline view

### Why

Manuscript order is rarely story order. Flashbacks, parallel storylines, non-linear structures — all need to be seen on a timeline distinct from the binder sequence. Manuskript, oStorybook, and bibisco all ship this; it's the most-requested view in novelist circles that Scrivener lacks.

### Depends on

Tier 1 scene metadata: `story_time` and `threads` are what the timeline visualizes. A project with no `story_time` fields shows a banner: "Add a Story Time to scenes in the Inspector to see them here."

### UX

- **Fourth view button** in the editor toolbar (after Binder/Corkboard/Preview): **Timeline**.
- **Horizontal timeline** of all scenes with `story_time` set, positioned by parsed time.
- **Lanes**:
  - Default: one lane per POV character (with Tier 1's `pov_character` field)
  - Toggle: one lane per plot thread
  - Toggle: single lane, ordered chronologically
- **Scene chip**: synopsis truncated, thread color-dots. Click to open in editor.
- **Unplaced scenes** (no story_time) appear in a "Unplaced" row at the bottom.
- **Read-only.** Dragging doesn't reorder the manuscript — the timeline is a *view* of story-time, not authoritative.

### Implementation

- **Parsing `story_time`:** accept ISO 8601 (`2024-03-15T22:30`), plain ISO date, or a free-form string (bucket those in a fuzzy "unplaced but labeled" lane). Parser tries structured formats first; falls back to "ordered by first appearance."
- **Frontend-heavy:** new `components/timeline/TimelineView.tsx`. Uses canvas or SVG for layout. `react-flow` is overkill; a custom grid with CSS is simpler.
- **No backend changes** — just a new view over existing data.

### Scope limits

- **No editing from the timeline.** To change a scene's story_time, open it and use the Inspector.
- **No parallel-timeline overlays** (two timelines side by side). Use lanes.
- **No zoom/pan past reasonable UI scales.** A 3-year story in 30-minute increments is ridiculous; we won't handle pathological ranges.

### Open questions

- What does "Day 3, 22:30" (free-form string) place on the timeline? Proposal: parse the numeric component, order by that; if none parseable, alphabetical.
- Should unplaced scenes be draggable into the timeline to assign story_time? Proposal: yes — drag drops open a "Set story time" prompt. Minor UX polish, not blocking.

---

## Cross-frontend rollout

All four features are Tauri-first. TUI gets:
- **Session target** — a one-line status bar ("Today 428/1000") when configured. Trivial.
- **Per-doc history** — `git log -- <path>` exposed as a new overlay (F6?).
- **Scrivenings + Timeline** — skip in v1.2; these are visual-heavy and the TUI audience doesn't need them.

SwiftUI / Qt6 / WinUI get **session target display** (read-only) and defer the rest.

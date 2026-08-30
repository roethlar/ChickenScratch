# Plan: Draft peek — read another draft without switching

**Owner request (quote):**
> I don't see how to preview what's in another draft. restore seems to be
> the only option, and it sounds destructive

**Phase check:** [x] Allowed by `CURRENT_PHASE.md` — owner-directed
(2026-08-29). "Novelist feature expansion" is listed as a later priority,
but this is a trust fix, not expansion: the owner cannot see what a draft
holds without a workspace-changing action, and the safest control
("Restore") reads as destructive. Nothing under "Explicitly paused" is
touched (no vault/remote work). [x] Not paused.

**Invariants touched:** I2 (all git reads stay in `chickenscratch-core`),
I6 (peek must be strictly read-only — no checkout, no worktree mutation),
I7 (wording translates git, no jargon).

---

## [MODEL] Intent

The owner can click any draft in the Drafts tab and *read* it — documents
listed, contents shown — without leaving the draft they're working in,
under a banner that says nothing is being changed. Separately, "Restore"
is reworded so it says what it actually does (brings a version back;
current work is saved to History first — nothing is deleted).

## [MODEL] Approach

Three slices, one concern per commit, engine first:

1. **Core: read-only draft reading.** Two functions in
   `crates/core/src/core/git.rs`, modeled on `compare_drafts`'s
   branch→tip→tree resolution (no checkout, no `WritePermit` — pure
   reads): `list_draft_documents(path, draft)` walks the branch tip tree
   and returns document entries (skipping `project.yaml`, `.meta`,
   `.git*`), and `read_draft_document(path, draft, doc_path)` returns one
   document's text from that tree. Guard test proves the worktree and
   HEAD are byte-identical before/after both calls.
2. **App: peek view.** Tauri commands wrapping the two functions
   (`src-tauri/src/commands/git.rs`, registered in `main.rs`); new
   `DraftPeek` dialog (same pattern as `DraftCompare`): document list on
   the left, read-only text on the right, banner "Just looking — nothing
   in your current draft changes", actions Close and "Switch to this
   draft". Entry point: clicking a draft row (or an eye icon) in the
   Drafts tab.
3. **Wording: Restore reassurance.** History tab restore control retitled
   "Bring back this version"; confirm text becomes "Bring back this
   version? Your current version is saved to History first — nothing is
   lost." UI strings only.

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `crates/core/src/core/git.rs` | `list_draft_documents`, `read_draft_document` (read-only) + unit/guard tests |
| `src-tauri/src/commands/git.rs`, `src-tauri/src/main.rs` | command plumbing for the two reads |
| `ui/src/components/revisions/DraftPeek.tsx` (new) | read-only peek dialog with banner |
| `ui/src/components/revisions/Revisions.tsx` | Drafts-tab entry point; Restore wording |
| `ui/src/test/` | vitest coverage for peek + wording |

## [MODEL] Tests

- [ ] `cargo test -p chickenscratch-core` — new tests: list/read return the
      *other* draft's content; worktree + HEAD unchanged after reads
      (guard: fails if an implementation ever checks out).
- [ ] `ui: npm run test` — DraftPeek renders banner + read-only content;
      no write command invoked from peek; Restore strings asserted.
- [ ] Full declared suite (`.agents/repo-guidance.md` Verification) before
      each commit.

## [MODEL] Owner verification (plain English)

Open the app → Revisions panel → Drafts tab → click a draft. A window
opens saying "Just looking — nothing in your current draft changes"; you
can read its documents, then close it and you're exactly where you were.
In History, the circular-arrow button now says "Bring back this version"
and asks with the reassurance wording before doing anything.

## [YOU] Decisions needed

- None. (Peek opens as a dialog like Compare Drafts; a side-by-side "peek
  while writing" layout is out of scope for this slice.)

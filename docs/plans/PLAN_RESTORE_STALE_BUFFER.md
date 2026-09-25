# Plan: Stop the editor overwriting a restored version

**Owner request (quote):**
> "continue with as much as possible while not requiring me" (2026-09-25).
> Drafting only; implementation approval is not given.

**Phase check:** [x] Allowed by `CURRENT_PHASE.md` (Engine hardening —
"close each approved high-risk item one concern at a time, with a guard
test") [x] Not paused

**Invariants touched:** I6 (no writer data loss), I9 (guard test must bite).

---

## [MODEL] Intent

When a writer clicks "Bring back this version", the older text must be
what is on disk afterward — not just what is on screen. Today the app
restores the older version and then immediately writes the pre-restore
text back over it. The screen shows the restored text, so the writer
believes it worked; after closing and reopening, the newer text is back
and the restore is silently undone (GUI rerun 2026-09-25,
`.agents/break-it-validation.md`).

## [MODEL] Cause (confirmed 2026-09-25)

- The engine is correct: the "Restored to: …" commit holds the older
  text.
- `ui/src/components/editor/Editor.tsx` calls
  `editor.setEditable(!readOnly && !barrierActive)`. In Tiptap 3.22,
  `setEditable(editable, emitUpdate = true)` emits an `update` event by
  default. When the restore's barrier releases, that event reaches
  `onUpdate` → `debouncedSave`, which marks the buffer dirty. The
  buffer-reload effect then runs `flushPendingSave()` before replacing the
  buffer, and writes the stale pre-restore text over the restored file
  (same second as the restore commit; `.meta` and `project.yaml` get a
  fresh `modified`).
- Proof: a throwaway build with `setEditable(..., false)` (not committed)
  left the disk matching the restored revision.
- Assumption, unverified: every barrier operation (switch draft, merge,
  pull, abort) shares this path and can write stale text into the new
  tree the same way.

## [MODEL] Approach

1. Pass `emitUpdate = false` to both `setEditable` calls in the editor
   (read-only and barrier toggles). Editability changes are never edits.
2. Check the side effect seen in the throwaway build: the status bar
   read "0 words" after opening a document until the first keystroke.
   If the word count depended on that spurious update, compute it on
   buffer load instead.
3. Guard test (UI, vitest): toggling the barrier on and off must not
   mark the buffer dirty or call `updateDocumentContent`. Prove it
   bites by reverting step 1.

One commit per step, per `.agents/repo-guidance.md`.

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `ui/src/components/editor/Editor.tsx` | `setEditable(…, false)`; word-count fix if needed |
| `ui/src/**/*.test.ts(x)` | barrier-toggle guard test |

## [MODEL] Tests

- [ ] New UI guard test fails without the fix, passes with it
- [ ] Full declared suite (`.agents/repo-guidance.md` Verification)
- [ ] Manual (agent via computer use): edit, save, restore an older
      version, quit, reopen — the older text is still there; repeat for
      switching drafts

## [MODEL] Owner verification (plain English)

"Make a change, save a version, bring back the earlier version, quit and
reopen — the earlier text is still there."

## [YOU] Decisions needed

- Go / no-go to implement.

# Plan: Plain-language UI — remove git jargon

**Owner request (quote):**
> too much git technical stuff leaking into the UI for humanities majors.

**Phase check:** [x] Allowed by `CURRENT_PHASE.md` — owner-directed polish;
touches only UI wording, not the engine, and nothing under "Explicitly
paused". [x] Not paused.

**Invariants touched:** none (no format I/O, no engine change; strings only).

---

## [MODEL] Intent

A writer who has never used git can read every button, banner, and message
in the app and understand it. Git words (push, pull, fetch, merge, remote,
conflict markers, repository, fast-forward) disappear from the visible UI —
replaced with plain English about backing up, getting changes, and combining
versions. Behavior does not change; only words do.

## [MODEL] Approach

Replace visible strings in place (no i18n framework — the app has none and
this stays one concern). Proposed vocabulary:

| Today (git-speak) | Proposed (writer-speak) |
|---|---|
| Settings tab **Remote** | **Sync & Backup** |
| "Remote URL" + hint about bare repos | "Backup address" + hint: "Where your project is backed up online — e.g. a free GitHub account." (drop `file:///…/bare-repo.git` and ssh-agent jargon into a collapsed "Advanced" hint) |
| "Auto-Push on Save Revision" | "Back up automatically when you save a revision" |
| **Push** button / "Pushed to remote." | **Back up** / "Backed up online." |
| **Pull** button / "Pulled (fast-forward)." / "Pulled and merged." | **Get changes** / "Latest changes downloaded." / "Changes downloaded and combined with yours." |
| **Fetch** button / "Fetched from remote." | **Check** / "Checked — see summary above." |
| "Up to date with remote" | "Everything backed up" |
| "N to push · N to pull" | "N to back up · N to download" |
| "Configure a remote in Settings › Remote" | "Set up online backup in Settings › Sync & Backup" |
| "Merge conflicts" dialog title | "Both versions changed the same text" |
| "…show both versions with conflict markers" | "…now show both versions, marked so you can choose" |
| "Abort merge (keep your version)" | "Keep my version" |
| "Take the incoming version" | "Use the other version" |
| "Resolve manually" | "Let me pick line by line" |
| Merge banner "A merge is in progress… complete the merge — or abort" | "Two versions are being combined… finish combining — or go back to your version" |
| "Complete merge" / "Abort merge" buttons | "Finish combining" / "Go back to my version" |
| "Merge completed." / "Merge aborted; …" toasts | "Versions combined." / "Your version is back." |
| "Merge into current draft" (revision action) | "Combine into current draft" |
| Raw error toasts (`Merge failed: ${e}` etc.) | Friendly first line ("Couldn't combine versions.") with the technical detail kept after it for support |

Icons (owner 2026-08-29: "the git svg icons also make no sense to an
author"). Same slice — swap git-shaped glyphs for plain ones, all from the
already-bundled lucide set:

| Today (git icon) | Where | Proposed |
|---|---|---|
| `GitBranch` | Drafts tab, "New Draft Version" | `FileStack` (pile of drafts) |
| `GitBranch` | switch-to-draft button | `ArrowRightLeft` (swap) |
| `GitBranch` | Threads tab | `Route` (a path through the story) |
| `GitMerge` | merge banner + combine action | `Combine` |
| `GitCompare` | Compare Drafts | `Columns2` (side-by-side) |
| `Cloud` / `Upload` / `Download` | sync toolbar | keep — already plain |

Slice is UI-only: `ui/src/components/revisions/Revisions.tsx`,
`MergeBanner.tsx`, `DraftCompare.tsx`,
`ui/src/components/settings/Settings.tsx`, plus any test
expectations on these strings. One commit.

Out of scope (possible follow-ups, not in this slice): removing the Fetch
button entirely, renaming "revision" concepts, TUI wording.

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `ui/src/components/revisions/Revisions.tsx` | sync toolbar, toasts, conflict dialog wording |
| `ui/src/components/revisions/MergeBanner.tsx` | banner + prompt wording, icon |
| `ui/src/components/revisions/DraftCompare.tsx` | compare icon |
| `ui/src/components/settings/Settings.tsx` | Remote tab label, field labels, hints |
| `ui/src/test/*.test.*` | update string expectations |

## [MODEL] Tests

- [ ] `cd ui && npm run lint && npm run build && npm run test`
- [ ] Grep check: no visible string contains push/pull/fetch/merge/remote/repository (code identifiers exempt)

## [MODEL] Owner verification (plain English)

Open the app → Revisions panel and Settings. Every button and message reads
like English, not git. Sync still works the same.

## [YOU] Decisions needed

- Approve the vocabulary above (or name any word you'd rather use — e.g.
  "Back up" vs "Send").

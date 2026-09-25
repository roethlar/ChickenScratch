# Owner-level "break-it" validation (in flight)

Manual/driven acceptance test for the engine-hardening phase: import a copy
of a real Scrivener project, then attack it and confirm the protections hold.
Written 2026-07-17; the first driven run is recorded below. The signed
bundle source revision was not recorded, so observations must be rechecked
against an identified current build before selecting a code fix. Any
confirmed LOST DATA verdict becomes the next hardening fix.

Ground rules for whoever drives it (agent or owner):

- Work only on COPIES. Never open or modify `~/Dev/scrivner` originals.
- No commits, pushes, or code edits — tester role only.
- If the app won't build or a step fails, stop and report; don't improvise.
- Record what actually happens even when it contradicts the expectation.

## Script

1. `mkdir -p ~/chikn-validation && cp -R ~/Dev/scrivner/"Corn 2.scriv" ~/chikn-validation/`
   ("Corn 2.scriv" chosen 2026-07-17: largest real project, ~18 documents;
   same story as the repo's `samples/Corn.chikn`.)
2. Find the app launch command (README/docs; Tauri app: `src-tauri/` + `ui/`),
   install deps, launch in dev mode.
3. Import the copy as a new .chikn project in `~/chikn-validation/`;
   binder should show ~18 documents.
4. Edit a chapter; Save Revision "validation edit 1"; edit; save
   "validation edit 2".
5. Revision history shows both.
6. CRASH MID-SAVE: force-kill during a save; relaunch. Expect: clean open,
   nothing before the last save missing; inspect folder for leftover junk.
7. CORRUPT A FILE: overwrite one `.meta` with garbage; reopen. Expect:
   READ-ONLY open with plain-English warning naming the file; garbage file
   byte-identical afterward. Restore contents; project writable again.
8. STRANDED FILE: drop `manuscript/stray.md`; reopen. Expect: read-only warning or safe adoption — record which; nothing else changes. Remove it. <!-- lint: allow (hypothetical path in manual validation script) -->
9. DELETE HISTORY: delete the project's `.git`; reopen. Expect: repo quietly
   rebuilt (open-time self-heal), project writable, new Save Revision works.
10. RESTORE: one more edit + revision, then restore the previous revision.
    Expect: text reverts, restore appears as a NEW history entry.
11. Wrap-up table: attack | expected | actual | verdict
    (SURVIVED / DEGRADED SAFELY / LOST DATA) + screenshots.

## Status

- 2026-07-17: script prepared; Claude Cowork could not drive it; owner went
  to bed before an alternative driver ran. Not yet executed.
- 2026-08-29: executed steps 1–9 (Claude Code driving the **signed release
  bundle** `target/release/bundle/macos/ChickenScratch.app`, not dev mode —
  both were running; the release app took focus and all interaction hit it).
  Stopped at step 9 per ground rules (step failed; step 10 needs history).
  Results below. The test project was left in its post-step-9 state
  (no `.git`, one uncommitted edit); current machine availability is
  recorded in `.agents/machines.md`.

## Source check 2026-09-17

As of `a2d8a39`, the Tauri open path already calls permit-backed reader
self-heal, including `pre_repair_git` for an absent repository. Current
reads also do not quarantine corrupt sidecars. These contradict the plan
diagnosis and parts of the observed release behavior; the runtime cause
remains unverified. Preserve the results below as historical observations,
not a verified description of this source revision. See `.agents/state.md`
Blockers for the next action. No desktop test was rerun during this check.

## Engine repro 2026-09-25

As of `53ed7e0`, a throwaway command-boundary test (discarded worktree,
nothing committed) drove the real `load_project_inner` path:

- **Delete `.git`, then a fresh open (new token registry, as after an app
  relaunch):** the project opens writable, `.git` is recreated, and
  `save_revision` commits; `list_revisions` shows the new revision. The
  step-9 expectation holds at the engine/command layer.
- **Delete `.git` while the project stays open (no reopen):**
  `save_revision` returns `Err(Unknown("Not a git repo: …"))` — an error,
  not a silent no-op. Nothing rebuilds history until the next open.
- The open path (`reader.rs`, `fidelity.rs`, `commands/project.rs`) is
  unchanged since before the 2026-08-29 run; self-heal landed 2026-07-16
  (`0d3c027`). The release bundle used on 2026-08-29 no longer exists, so
  its build date cannot be checked.
- UI: with no drafts listed, `Revisions.tsx` falls back to the badge
  "main", so the observed "master→main" flip and "No revisions yet"
  match an empty rebuilt repo as well as an absent one.
- `handleSave` catches a rejected `save_revision` and calls `toastError`;
  `ToastProvider` is mounted in `main.tsx`. The code offers no explanation
  for the missing toast. Assumption, unverified: the 2026-08-29 run either
  did not truly reopen (same-session save) or used a stale bundle, and the
  missing toast has a separate cause (e.g. `flushPendingEditorSave` never
  settling).

Remaining check needs the desktop app (owner or a GUI-capable driver):
build the current app, recreate the sandbox (script step 1), and re-run
steps 9–10 with a full quit and relaunch between deleting `.git` and
saving. Also try a save after deleting `.git` *without* a relaunch and
note whether an error appears.

## Results 2026-08-29

| # | Attack | Expected | Actual | Verdict |
|---|--------|----------|--------|---------|
| 3 | Import Corn 2.scriv | binder ~18 docs | All docs imported, corkboard + editor fine. Bug: corkboard card previews show raw escapes (`Henks\'`, `\"…\"`); editor text is clean | SURVIVED (cosmetic import/preview bug) |
| 4–5 | Two edits + Save Revision ×2 | both in history | Both appear ("validation edit 1"/"2") | SURVIVED |
| 6 | kill -9 ×3 (after save; between autosave and commit) | clean reopen, nothing before last save missing, no junk | Clean reopen every time; no lock/tmp junk; autosaved-but-uncommitted text survived and was committable after relaunch. Minor: session killed right after import lost its Recent Projects entry | SURVIVED |
| 7a | `opening.meta` → invalid-UTF-8 garbage | read-only open, plain-English warning naming file | Project **refuses to open**: banner "IO error: stream did not contain valid UTF-8" — dev-speak, no filename, back to welcome screen. Garbage byte-identical after | DEGRADED SAFELY (message fails I-level "plain English, name the file") |
| 7b | `opening.meta` → valid-UTF-8 garbage (bad YAML) | same | Opens **normally, zero warning**; corrupt file silently renamed to `opening.meta.corrupt-<uuid>` (byte-identical), metadata reset to defaults. Writer never told | DEGRADED SAFELY but **silent** — warning missing |
| 8 | Stranded `manuscript/stray.md` | read-only warning or safe adoption | Neither: file silently ignored — not in binder, no warning; file itself untouched | DEGRADED SAFELY but **silent** |
| 9 | Delete `.git`; reopen | repo quietly rebuilt, Save Revision works | Opens fine, panel says "No revisions yet" (branch badge flips master→main), but **no repo is created — Save Revision silently no-ops**: no commit, no draft, no error; describe text just sits there. Text autosave to disk still works | **LOST DATA** (requested revision silently discarded) — hardening bug, becomes next fix |
| 10 | Restore previous revision | revert + new history entry | Not runnable — no history exists after step 9 | BLOCKED by 9 |

Screenshots live in the driving session, not on disk.

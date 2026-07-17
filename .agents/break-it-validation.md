# Owner-level "break-it" validation (in flight)

Manual/driven acceptance test for the engine-hardening phase: import a copy
of a real Scrivener project, then attack it and confirm the protections hold.
Written 2026-07-17; not yet run (first driver attempt failed — see
`.agents/machines.md`). Any LOST DATA verdict is a hardening bug and becomes
the next fix.

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
8. STRANDED FILE: drop `manuscript/stray.md`; reopen. Expect: read-only
   warning or safe adoption — record which; nothing else changes. Remove it.
9. DELETE HISTORY: delete the project's `.git`; reopen. Expect: repo quietly
   rebuilt (open-time self-heal), project writable, new Save Revision works.
10. RESTORE: one more edit + revision, then restore the previous revision.
    Expect: text reverts, restore appears as a NEW history entry.
11. Wrap-up table: attack | expected | actual | verdict
    (SURVIVED / DEGRADED SAFELY / LOST DATA) + screenshots.

## Status

- 2026-07-17: script prepared; Claude Cowork could not drive it; owner went
  to bed before an alternative driver ran. Not yet executed.

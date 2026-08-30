# Plan: History self-heal — rebuild a missing revision repo at open

**Owner request (quote):**
> "go" (2026-08-29, approving: draft the fix plan for the break-it step-9
> LOST DATA finding). The validation script's owner-set expectation for
> step 9: "repo quietly rebuilt (open-time self-heal), project writable,
> new Save Revision works."

**Phase check:** [x] Allowed by `CURRENT_PHASE.md` (Engine hardening —
"close each approved high-risk item one concern at a time, with a guard
test") [x] Not paused

**Invariants touched:** I2 (fix lives in core, not the app), I6 (no writer
data loss), I7 (user-facing git is hidden — the writer sees "history
rebuilt", not git jargon), I9 (guard test must bite).

---

## [MODEL] Intent

A writer whose project lost its revision history (deleted `.git`, cloud-sync
mangling, disk repair) opens the project and keeps working: the app quietly
rebuilds history at open, tells them in plain English that earlier revisions
could not be recovered but their words are intact, and Save Revision works
again immediately. Today the same writer sees a healthy-looking project
whose Save Revision does nothing, with no error — every revision they think
they saved is silently discarded (break-it validation 2026-08-29, step 9,
verdict LOST DATA).

## [MODEL] Current behavior (traced 2026-08-29)

- `fidelity::probe_project_fidelity` classifies a repo-less project as
  `Full`, so `load_project_inner` (`src-tauri/src/commands/project.rs`)
  opens it writable. Nothing anywhere rebuilds the repo; the Revisions
  panel shows "No revisions yet" with a default "main" badge.
- Core is correct: `git::save_revision` returns
  `Err(ChiknError::Unknown("Not a git repo: …"))`
  (`crates/core/src/core/git.rs` ~252). The Tauri command propagates it.
- The UI `handleSave` (`ui/src/components/revisions/Revisions.tsx`) has a
  catch → `toastError("Save failed: …")`, yet no toast appears within the
  3.5 s toast window (verified with immediate screenshots on two app
  launches). Where the rejection is swallowed is unresolved — see Open
  questions. Text autosave to disk keeps working, so the on-disk words are
  safe; what's lost is every requested revision, silently.

## [MODEL] Approach

1. **Core self-heal at open (the fix).** New core function
   `git::ensure_history(path, …)`: if the project directory is a valid
   `.chikn` tree but has no repository, initialize one and create a baseline
   commit of the current tree ("History rebuilt — earlier revisions were
   lost outside the app"). Called from the open path before write-token
   acquisition, only when the fidelity probe says the tree itself is
   healthy. Never runs when a repo exists (a corrupt-but-present repo is a
   different concern and must not be re-initialized over — data loss risk).
2. **Tell the writer.** `LoadedProject` grows a plain-English `notices`
   list (mirroring `read_only_reasons`); the UI shows it once as a banner or
   toast: "This project's revision history was missing and has been rebuilt.
   Your text is intact; earlier revisions could not be recovered." No
   acknowledgment gate — the project stays usable.
3. **Loud failures (defense in depth).** Find and fix why the Save Revision
   rejection never surfaced: reproduce with dev tools open, fix the swallow
   point, and ensure `saving` state always resets so the button cannot wedge
   shut after one failure. Any future save failure must toast.
4. **Guard tests that bite.** Core: delete `.git` in a fixture, open,
   assert repo exists + baseline commit + `save_revision` succeeds; assert a
   corrupt-but-present repo is NOT re-initialized. Command boundary
   (`load_project_inner` is already test-shared): same fixture, assert
   `notices` carries the rebuild message. Prove each bites by reverting the
   fix and watching it fail.

Slices land in that order, one commit per concern, per
`.agents/repo-guidance.md`.

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `crates/core/src/core/git.rs` | `ensure_history` + baseline-commit helper |
| `crates/core/src/core/project/fidelity.rs` | classify missing-repo distinctly so open can trigger the rebuild (probe stays side-effect-free) |
| `src-tauri/src/commands/project.rs` | call self-heal in `load_project_inner`; `notices` on `LoadedProject` |
| `ui/src/components/revisions/Revisions.tsx`, `ui/src/components/shared/Toast.tsx` | show notices; fix swallowed save error / stuck `saving` |
| core + command-boundary tests | guard tests above |

## [MODEL] Tests

- [ ] `cargo test -p chickenscratch-core` — new `ensure_history` guards
- [ ] command-boundary test: open repo-less fixture → writable + notice
- [ ] full declared suite (`.agents/repo-guidance.md` Verification) before
      claiming done
- [ ] manual: re-run break-it steps 9–10 against the built app

## [MODEL] Owner verification (plain English)

"Open the corn test project (the one whose history we deleted) → you should
see a note that history was rebuilt → type a description and Save Revision →
it appears under History. Restore then works too."

## [YOU] Decisions needed

- None blocking. Proposed default: rebuild silently rescues going forward
  but cannot resurrect pre-deletion revisions; the notice says so plainly.

## Open questions

- Where exactly the save-error rejection is swallowed in the release app
  (no toast despite a catch path; possibly a never-settling IPC promise
  wedging `saving=true`). Resolve during slice 3 — assumption, not yet
  verified.
- Steps 7a/7b/8 findings (dev-speak UTF-8 error, silent meta quarantine,
  silent stray-file ignore) are separate concerns, deliberately out of
  scope here.

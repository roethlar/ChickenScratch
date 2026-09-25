# Plan: Keep Save Revision working when the folder holds extra things

**Owner request (quote):**
> "assume I actually want a working app. find issues, plan fixes, fix,
> review with codex." (2026-09-25)

**Phase check:** [x] Allowed by `CURRENT_PHASE.md` (Engine hardening)
[x] Not paused

**Invariants touched:** I2 (fixes live in core), I6 (no writer data
loss — a project whose history can never be saved again loses every later
revision), I9 (guard tests must bite).

Issues: `.agents/app-issues.md` #4, #5, #9.

---

## [MODEL] Intent

Save Revision, Restore, and draft switching keep working even if the
project folder contains another project, some other git folder, or a
leftover temporary file from an interrupted save. And the app stops the
writer from creating or importing a project inside an existing one in the
first place.

## [MODEL] Current behavior (observed in the app 2026-09-25)

- New Project's save dialog opens in the last-used folder. Saving there
  while it is inside `Corn.chikn` creates `Corn.chikn/Nested.chikn`, with
  its own `.git`.
- In `Corn`, every Save Revision then fails: `Failed to stage files:
  invalid path 'Nested.chikn/'` — `git::save_revision` stages with
  `index.add_all(["*"])`, which refuses an embedded repository. The same
  staging runs in `complete_merge`.
- `repo_has_changes` counts the untracked nested repo, so the tree is
  permanently "dirty": restore's save-first step and draft switching hit
  the same staging failure or refuse outright.
- `writer`'s atomic write leaves `.<name>.tmp-*` files if interrupted;
  they are neither ignored nor filtered, so the next revision commits them.

## [MODEL] Approach

1. **Skip nested repositories when staging and when checking for changes
   (core).** One shared predicate in `git.rs`: an untracked directory
   entry that contains `.git` is never staged (the `add_all` callback
   returns "skip") and is not counted by `repo_has_changes`. Used by
   `save_revision` and `complete_merge`. Guard test: a project with a
   nested repo saves a revision, the nested folder is absent from the
   commit tree, and `has_changes` is false afterward.
2. **Skip atomic-write leftovers the same way.** File names matching
   `.<name>.tmp-<suffix>` are never staged or counted. Guard test: a
   leftover temp file is not in the commit and does not make the tree
   dirty.
3. **Refuse to create a project inside an existing project (core).**
   `writer::create_project` walks the target's ancestors; if one holds a
   `project.yaml` and looks like a `.chikn` project, return a plain-English
   error naming it ("… is inside the project 'Corn'. Choose a folder
   outside it."). Covers New Project and Scrivener import (both call
   `create_project`). Guard test for both nested and sibling paths.

One commit per step.

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `crates/core/src/core/git.rs` | shared skip predicate; staging callback; status filter; tests |
| `crates/core/src/core/project/writer.rs` | ancestor check in `create_project`; tests |

## [MODEL] Tests

- [ ] Each guard test fails with its fix reverted
- [ ] Full declared suite
- [ ] GUI: outer project with a nested one saves revisions; New Project
      inside a project shows the plain-English refusal

## [MODEL] Owner verification (plain English)

"Saving a version keeps working even if another project ended up inside
this one's folder — and the app won't let you put one there."

## [YOU] Decisions needed

- None. Existing nested projects are left in place and simply ignored by
  the outer project's history (moving the writer's files would be riskier
  than leaving them).

# Plan: Save Revision rebuilds a history folder that vanished mid-session

**Owner request (quote):**
> "assume I actually want a working app. find issues, plan fixes, fix,
> review with codex." (2026-09-25)

**Status:** Implemented 2026-09-25; guard test failed before the fix.

**Phase check:** [x] Allowed by `CURRENT_PHASE.md` (Engine hardening)
[x] Not paused

**Invariants touched:** I2 (core), I6 (no writer data loss), I9.

Issue: `.agents/app-issues.md` #8. Supersedes the remaining intent of
`docs/plans/PLAN_HISTORY_SELF_HEAL.md`.

---

## [MODEL] Intent

If a project's `.git` disappears while the project is open (sync tool,
disk cleanup), the writer's next Save Revision just works — the same
quiet rebuild the app already does when a project is opened. Today it
fails with "Not a git repo … class=Repository (6)" until the app is
relaunched.

## [MODEL] Approach

1. In `git::save_revision`, after the permit check: if `.git` is absent,
   recreate it with `init_repo` (the same verify-or-create used by the
   open-time self-heal; never touches an existing repo). Then stage and
   commit as usual.
2. Guard test: open token, delete `.git`, `save_revision` succeeds and the
   new commit holds the current text. Revert to watch it fail.

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `crates/core/src/core/git.rs` | rebuild before staging; guard test |

## [MODEL] Owner verification (plain English)

"Even if the hidden history folder goes missing while you work, Save
Revision keeps working."

## [YOU] Decisions needed

- None.

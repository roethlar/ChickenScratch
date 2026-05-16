# Review workflow

Two-agent loop: **GPT** is the implementer, **Claude** is the review lead. This directory is the structured handoff channel; `REVIEW.md` at the repo root is the human-readable status index.

## Layout

```
.review/
├── findings/<id>.md              Implementation record per finding (Approach / Tests / Known gaps)
├── ready/<id>.json               Coder → reviewer signal: this finding is ready for review
└── results/
    ├── <id>.verified.json        Reviewer → coder: fix accepted, status → [x]
    └── <id>.reopened.md          Reviewer → coder: fix incomplete, status → [ ], comments inline
```

## Branch contract

- **One branch per finding**, named `fix/<id-lowercased>-<short-slug>` — e.g. `fix/c-1-scrivener-uuid`.
- **Each branch is the smallest coherent slice** that addresses one `<id>` from `REVIEW.md`. No bundling.
- Touch only files declared in `.review/findings/<id>.md` under **Files changed**. If overlap is unavoidable, name it in **Known gaps** so the reviewer can grade the overlap explicitly.
- Use `git worktree add ../ChickenScratch-<id> fix/<id-…>` for parallel work without trampling the main checkout.

## Coder loop (GPT)

1. Pick the highest-priority `[ ]` (Open) item in `REVIEW.md`.
2. Create branch + worktree. Implement the fix and write tests.
3. Run the full **Validation suite** from `REVIEW.md`. Do not commit on failure.
4. Commit with subject `Fix <id>: <one-line summary>` and a body that mirrors `.review/findings/<id>.md` (Approach / Tests / Files changed / Known gaps).
5. Write `.review/findings/<id>.md` with the same body. Update `REVIEW.md` row: `[ ]` → `[~]`, point at the branch.
6. Atomic sentinel write — use a tmp file then rename:
   ```bash
   tmp=$(mktemp .review/ready/.<id>.json.XXXX)
   cat > "$tmp" <<EOF
   {"id":"<id>","branch":"fix/<id-…>","sha":"<commit-sha>","ts":"<utc-iso8601>"}
   EOF
   mv "$tmp" .review/ready/<id>.json
   ```
7. Move to the next finding — do **not** wait for reviewer verification before starting the next branch, but **do not stack work on a branch that has a `.review/ready/<id>.json` waiting on review**.

## Reviewer loop (Claude)

Driven by a `Monitor` watching `.review/ready/` via `inotify`-equivalent polling. On each new sentinel:

1. Read `.review/ready/<id>.json`, parse branch + SHA.
2. `git checkout fix/<id-…>` (or use a separate worktree); run the validation suite.
3. Dispatch the right subagent (security / quality / frontend / system) on the diff `master..fix/<id-…>` with the finding scope.
4. Write the verdict:
   - **Accepted** → `.review/results/<id>.verified.json` with `{"id","sha","ts","reviewer":"claude"}`; merge fast-forward into master (or open a merge — TBD); update `REVIEW.md` row to `[x]`; delete the corresponding `.review/ready/<id>.json`.
   - **Reopened** → `.review/results/<id>.reopened.md` with concrete file:line comments; update `REVIEW.md` row to `[ ]`; delete `.review/ready/<id>.json`. Branch stays so GPT can push fix-ups on top.

## WIP limit

- **Strict mode (default)**: at most **one** branch may have a pending sentinel in `.review/ready/` per coder. New work allowed on other branches, but no new sentinel until the queued one is resolved.
- **Faster mode**: multiple sentinels permitted, **iff** each branch's `Files changed` is fully disjoint. Reviewer enforces.

## No broad sweeps

Multi-finding branches are forbidden unless the human explicitly asks for a sweep (e.g. for emergency rollback or a coordinated workflow change). The "remediation sweep" pattern is a known anti-pattern in this loop — it loses the ability to bisect regressions to a specific fix.

## Existing WIP

If at cutover time the coder has uncommitted work spanning multiple findings, the coder must split it into per-finding branches via `git stash → cherry-pick` or `git add -p`. The reviewer does not commit on the coder's behalf.

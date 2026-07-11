# Repo-Specific Guidance
<!-- Extends AGENTS.md; never overrides it. Rules and pointers only — state
     lives in .agents/state.md. -->

## Mission Detail

ChickenScratch is a writing app built around the open **`.chikn` format** —
plain Markdown files with YAML sidecars and embedded git history, so a
writer's project outlives any one app. The format is the durable product;
applications (Tauri desktop, TUI, converter) are reference implementations.
See `docs/PROJECT.md` for the full plain-English mission, priority order, and
glossary.

The owner is not a developer. Give plain-English handoffs: what changed, how
to try it, and a yes/no question only if something is genuinely blocked.
Never ask the owner to read diffs, code, or governance files. See
`docs/HUMAN-GATE.md` for how the owner directs work and how agents should
interpret plain-English direction changes (e.g. "I changed my mind about …"
updates `docs/INVARIANTS.md`; "let's prioritize Y instead" reorders
`docs/CURRENT_PHASE.md`).

## Reading Order

For a work request, read in this order before making changes:

1. `docs/INVARIANTS.md` — hard rules I0–I10 (format ownership, engine
   boundary, no data loss, verify-before-done). Authoritative; if a task
   conflicts with an invariant, stop and ask the owner rather than
   improvising a workaround.
2. `docs/CURRENT_PHASE.md` — the current active phase and whether new work is
   paused.
3. `docs/ARCHITECTURE.md` — how the codebase maps to the invariants.
4. `docs/CHIKN_FORMAT_SPEC.md` — when touching the on-disk format.
5. The relevant file under `docs/adr/` — when touching an architectural
   decision.
6. `docs/AGENT-WORKFLOW.md` — the full step-by-step workflow (accept/reject,
   planning threshold, implementation order, DEVLOG discipline, handoff
   format) that this file only summarizes.

Non-trivial work (touches the engine, the format spec, git writes, or more
than two files) needs a short plan first: `docs/templates/Plan-Template.md`.

## Verification

Declared suite (`docs/AGENT-WORKFLOW.md` §5, confirmed against
`.github/workflows/validation.yml`, which triggers on `push`/`pull_request`
with no branch filter and so runs on `master`):

```bash
cargo fmt --all -- --check
cargo clippy -p chickenscratch-core --all-targets -- -D warnings
cargo test -p chickenscratch-core --lib
cargo clippy -p chickenscratch --all-targets -- -D warnings
cargo test -p chickenscratch --bins
cd ui && npm run lint && npm run build
```

Full release checklist: `RELEASE.md`.

Current runnability status lives in `.agents/state.md` (Verification), not
here.

`docs/AGENT-WORKFLOW.md` §6 governs `DEVLOG.md`: append only after
significant work (architecture, format, governance, a shipped plan, or a
non-obvious integrity fix) — not every session, not routine features. Newest
entry at top.

## Remotes & Sync

No repo-specific remote or sync process beyond standard git. Push policy
lives in `.agents/push-policy.md`, not here.

## Earned Practices

- **One concern per branch/commit on multi-fix work.** When working through a
  list of findings or fixes, each branch and commit addresses exactly one
  item; commit each before starting the next. Multi-fix sweep branches
  require an explicit owner request. Source: `.review/README.md` and the
  incident recorded in `REVIEW.md` review passes 1–3, where a coder agent
  accumulated uncommitted fixes across 26, then 30, then 37 files before its
  first commit, leaving the reviewer unable to verify any fix in isolation or
  bisect regressions. (This is also recorded as a candidate cross-repo
  harvest idea in `.agents/harvest.md`.)
- **Deprecated trees get no new features.** `macos/`, `windows/`, `linux/`
  were deprecated native experiments
  (`docs/adr/ADR-004-deprecated-native-engines.md`), deleted from the
  working tree (history stays in git). Do not recreate or extend them;
  format I/O outside `crates/core` is a hard stop requiring an explicit
  owner go.
- **Marketing waits.** No marketing or website work before the format and
  the Tauri app are coherent, unless the owner explicitly says to skip ahead
  (`docs/PROJECT.md` priority order).

# plan-2: Review of PLAN_TREE_REPLACE_EPOCH_GUARD.md

**Type**: plan review (adapted from code-finding flow; no branch, no guard
proof — the artifact is a plan document on `master`. The doc-review analog of
the guard proof: the reviewer verifies the plan's factual claims against the
actual code, and `guard_confirmed` attests that verification.)
**Status**: In progress — round 1 reopened, plan revised, round 2 pending
**Subject**: `docs/plans/PLAN_TREE_REPLACE_EPOCH_GUARD.md`

## Round 1 dispatch

- **Reviewer**: codex-cli 0.144.4, `codex exec --ephemeral -s read-only
  --json --output-schema … -o …` (probed + smoke-tested this session; see
  `.agents/review/harnesses.local.json`)
- **Reviewed SHA**: `2a063c87b8cfa66c30daeec5642b889e5098d7a5` (plan commit)
- **Base SHA**: `066a2a81d796b92dd68721cfb05bf8356b66c492` (parent; plan
  absent there)
- **Bound**: 1800 s (plan-1 lesson: 900 s killed a wider-scope dispatch)
- **Dispatched**: 2026-07-15
- **Verdict**: `reopened` (envelope valid: verdict in enum,
  `guard_confirmed: true`, reviewed/base SHAs match dispatch, exit 0)

## Round 1 finding

> `src-tauri/src/commands/mod.rs:42` — The surface claim is false:
> `checkout` transparently replaces a stale token after re-probing (lines
> 45–60), while tree-replacing commands refresh only on success
> (`src-tauri/src/commands/git.rs:57`) and the UI reloads only after the
> command succeeds (`ui/src/components/revisions/Revisions.tsx:108`). After
> a guarded partial failure, a still-Full partial tree therefore lets the
> next auto-save acquire a fresh token and write stale editor state. The
> core-only guard test can pass while the clobber remains; the plan must
> add app-layer refusal/reload handling and coverage.

**Disposition: ADMITTED** — verified independently before accepting (the
anti-capitulation gate), all three citations hold:

1. `mod.rs:45–60`: `checkout` treats staleness as a cache miss — silently
   re-probes and returns a fresh token; staleness never surfaces as a
   refusal to callers.
2. `git.rs:57–60`: `tokens.refresh()` only under `result.is_ok()`;
   `with_write_permit` evicts the token on fidelity refusal, priming the
   transparent re-acquire.
3. `Revisions.tsx`: `handleSave` refreshes only `if (ok)`; `handleRestore`
   reloads only after `restoreRevision` resolves; error paths only toast.

Predicted observable failure confirmed: guarded partial failure → epoch
bump refuses old token, UI keeps stale buffer, no reload → next auto-save
re-probes, gets a fresh token against the mutated tree → stale content
clobbers the partially-replaced tree with valid authorization. The epoch
guard protects the *token*, not the *content*.

**Plan revision**: step 4 rewritten (app-layer reload on tree-replacing
command failure, before any further save), Files table + Tests checklist
extended with the app-layer regression. Committed for round 2.

## Round 2 dispatch

- **Reviewed SHA**: (revision commit — recorded at dispatch)
- **Base SHA**: `066a2a81d796b92dd68721cfb05bf8356b66c492` (unchanged —
  the plan-less parent, so the full plan stays in scope each round)
- **Verdict**: pending

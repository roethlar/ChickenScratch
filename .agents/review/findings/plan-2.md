# plan-2: Review of PLAN_TREE_REPLACE_EPOCH_GUARD.md

**Type**: plan review (adapted from code-finding flow; no branch, no guard
proof — the artifact is a plan document on `master`. The doc-review analog of
the guard proof: the reviewer verifies the plan's factual claims against the
actual code, and `guard_confirmed` attests that verification.)
**Status**: In progress — rounds 1–2 reopened, plan revised twice, round 3 pending
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

- **Reviewer**: codex-cli 0.144.4, same invocation as round 1
  (`codex exec --ephemeral -s read-only --json --output-schema … -o …`)
- **Reviewed SHA**: `1f9387054eacd66261373c21a8905a7f7ecefc44` (plan
  revision commit)
- **Base SHA**: `066a2a81d796b92dd68721cfb05bf8356b66c492` (unchanged —
  the plan-less parent, so the full plan stays in scope each round)
- **Bound**: 1800 s
- **Dispatched**: 2026-07-15 (prompt: `/tmp/plan2-r2-prompt.md`; verdict →
  `/tmp/plan2-r2-review-last.json`; round-1 finding quoted so the reviewer
  verifies the revision resolves it without re-litigating)
- **Verdict**: `reopened` (envelope valid: verdict in enum,
  `guard_confirmed: true`, reviewed/base SHAs match dispatch, exit 0)

## Round 2 findings

Four comments, each verified independently against the working tree before
accepting (anti-capitulation gate). All four **ADMITTED**:

1. **`ui/src/stores/projectStore.ts:71` — flow-mode buffer survives
   reload.** Confirmed: `openProject`'s `set` resets `activeDocId`/
   `activeDoc` but not `flowDocs` (line 71–79; `enterFlow` at 142 is the
   only setter, `exitFlow`/`selectDocument` the only clearers). A
   flow-mode buffer stays live across restore/switch/pull; its next edit
   auto-saves pre-operation sections under a refreshed token.
2. **Plan step 4 (round-1 revision) asserted an ordering it didn't
   enforce.** "Reload before any further save can run" is not established
   by calling `openProject` + `refresh`: a debounced save already queued
   behind `ProjectWriteLocks` re-probes via `ProjectTokens::checkout` and
   gets a fresh token during/after reload; and `openProject` clearing
   `activeDoc` can itself trigger the editor's dirty-buffer flush.
   Requires an explicit save barrier (suspend auto-save/flush from before
   first mutation until reload + buffer rebuild complete).
3. **`ui/src/components/revisions/Revisions.tsx:144`/`:228` —
   `Ok(Conflicts)` mutates the tree without reload.** Confirmed:
   `handleMergeDraft` (144) and `handlePull`'s `case "conflicts"` (228)
   only call `setConflictFiles(...)`; `onResolveManually` merely clears
   the dialog (`:471`). The merge has already rewritten the tree, so
   "Resolve manually" + edit saves a pre-merge buffer under a fresh
   token. Coverage must span every tree-mutating result kind.
4. **File map omitted `DocumentHistory.tsx`.** Confirmed: it owns the
   `restore_document` handler (`handleRestore`, line 61) and reloads/
   rebuilds only in its success path — the `catch` (line 84) only toasts.

**Plan revision (round 2)**: step 4 rewritten as three sub-requirements
(save barrier; explicit buffer reset/rebuild incl. flow mode; every
tree-mutating result kind incl. `Ok(Conflicts)`); Files table gains
`DocumentHistory.tsx` and the `projectStore.ts`/`editorRef.ts` barrier
seam; Tests checklist gains queued-save, flow-mode, and conflict-path
regressions. Committed for round 3.

## Round 3 dispatch

- **Verdict**: pending

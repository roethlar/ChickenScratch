# plan-2: Review of PLAN_TREE_REPLACE_EPOCH_GUARD.md

**Type**: plan review (adapted from code-finding flow; no branch, no guard
proof — the artifact is a plan document on `master`. The doc-review analog of
the guard proof: the reviewer verifies the plan's factual claims against the
actual code, and `guard_confirmed` attests that verification.)
**Status**: In progress — rounds 1–6 all reopened; every finding verified and folded in; round 7 pending
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

- **Reviewer**: codex-cli 0.144.4, same invocation as rounds 1–2
  (`codex exec --ephemeral -s read-only --json --output-schema … -o …`)
- **Reviewed SHA**: `108fb2a3c7de6cab8e301e4f2e4c955dac5209b9` (round-2
  plan revision commit)
- **Base SHA**: `066a2a81d796b92dd68721cfb05bf8356b66c492` (unchanged —
  the plan-less parent, so the full plan stays in scope each round)
- **Bound**: 1800 s
- **Dispatched**: 2026-07-15 (prompt: `/tmp/plan2-r3-prompt.md`; verdict →
  `/tmp/plan2-r3-review-last.json`; all four round-2 findings quoted
  verbatim so the reviewer verifies the revision resolves them without
  re-litigating; explicitly asked to probe for REMAINING stale-buffer
  paths the barrier-as-specified does not close)
- **Verdict**: `reopened` (envelope valid: verdict in enum,
  `guard_confirmed: true`, reviewed/base SHAs match dispatch, exit 0;
  job 3, ~10 min, usage 2.59M in / 27.0k out)

## Round 3 findings

Four comments, each verified independently against the working tree before
accepting (anti-capitulation gate). All four **ADMITTED**:

1. **`Toolbar.tsx:116` / `CommentsPanel.tsx:66` — comment commands carry
   editor content around the barrier.** Confirmed: `addComment` serializes
   the live buffer (`getEditorMarkdown(editor)`, Toolbar.tsx:119) and sends
   it as `newContent` via `docCmd.addComment`; `handleDelete` does the same
   (CommentsPanel.tsx:68) via `docCmd.deleteComment`. Neither goes through
   the auto-save/flush path the barrier gates; queued behind
   `ProjectWriteLocks` during a tree op they re-probe after the epoch bump
   and write the stale buffer. Step 4 must gate every editor-content-bearing
   command, with a regression.
2. **Plan leaves TipTap editable during the barrier — silent data loss.**
   Confirmed as a plan gap: nothing in step 4 disables or reconciles live
   edits during an awaited tree op, so the required buffer rebuild discards
   anything typed in the window. Plan must disable editing (or specify
   reconciliation) during the barrier, plus an edit-overlap regression.
3. **File map omits `ui/src/components/editor/Editor.tsx`.** Confirmed: it
   owns `saveTimer` (:56), `flowDocsRef` (:63), `dirtyRef` (:70),
   `saveCurrent` (:79), and the debounce/flush logic; `projectStore.ts` /
   `editorRef.ts` alone cannot suppress those paths or signal rebuild
   completion. Include Editor.tsx and an awaitable barrier/rebuild contract.
4. **No app-layer test harness exists.** Confirmed: `ui/package.json`
   scripts are only dev/build/lint/preview; no vitest/jest/playwright in
   devDependencies. The plan's queued-save, flow-mode, and conflict
   regressions are not executable as declared. Plan must select and scope
   the harness and its verification/CI command.

**Plan revision (round 3)**: step 4 gains three requirements — the save
barrier is an *awaitable* suspend/resume + rebuild-complete contract
implemented by `Editor.tsx` (owner of `saveTimer`/`dirtyRef`/
`flowDocsRef`/`saveCurrent`); every editor-content-bearing command
(`addComment`/`deleteComment`) is gated behind the same barrier; the
editor is non-editable while the barrier is up (no reconciliation).
Files table gains `Editor.tsx`, `Toolbar.tsx`/`CommentsPanel.tsx`, and
`ui/package.json` + vitest harness (UI has no test runner today). Tests
checklist gains edit-overlap and comment-command regressions; declared
suite extended with the new UI test script. Decisions section asks the
owner whether vitest lands in the same commit or a preparatory one.
Committed for round 4.

## Round 4 — reopened (all findings admitted)

- Dispatched: codex 0.144.4 (gpt-5.6), read-only, --ephemeral, --output-schema, job 4, prompt `/tmp/plan2-r4-prompt.md`, last message `/tmp/plan2-r4-review-last.json`.
- Reviewed SHA: `dc8e295db14f3409e38e3484190869a498311bb4`; base `066a2a81d796b92dd68721cfb05bf8356b66c492`. Verdict: `reopened`, `guard_confirmed: true`.

Findings and triage:

1. `PLAN_TREE_REPLACE_EPOCH_GUARD.md:87` — `setEditable(false)` blocks DOM editing, not command dispatch; Toolbar formatting/footnote/streaming-AI and `FindReplace.tsx` still mutate the stale buffer; in-flight AI can land changes the rebuild discards. **ADMITTED**: verified `FindReplace.tsx:84–125`, ~20 Toolbar `chain()` sites, and `Toolbar.tsx:409` `editor.commands.insertContentAt` per stream delta.
2. `PLAN_TREE_REPLACE_EPOCH_GUARD.md:76` — no concurrency semantics; overlapping ops queue under `ProjectWriteLocks`; a boolean suspend/resume flag re-enables editing after the first completes. **ADMITTED**: verified `syncBusy` gates only fetch/pull/push (`Revisions.tsx:564–574`) and conflict dialog (`:519–525`); restore (`:353`), draft switch (`:415`), merge (`:421`) have no busy guard.
3. `PLAN_TREE_REPLACE_EPOCH_GUARD.md:123` — vitest script not folded into durable verification; CI runs only UI lint/build. **ADMITTED**: verified `.github/workflows/validation.yml` has `npm run lint` + `npm run build` for UI, no test step.

Revision folded in: barrier-active state checked by every programmatic dispatch site + cancel/await in-flight AI streams at barrier entry; barrier is a counted lease with UI-side trigger gating (extend `syncBusy` to restore/switch/merge); CI gains a UI test step and declared-suite guidance updated. Files table adds `FindReplace.tsx` and `validation.yml`/repo-guidance rows; Tests add programmatic-dispatch + AI-stream extension of edit-overlap and an overlapping-operation regression; Decisions adds the round-4 CI-scope question. Round 5 to verify.

## Round 5 — reopened (all findings admitted)

- Dispatched: codex 0.144.4 (gpt-5.6), read-only, --ephemeral, --output-schema, job 5, prompt `/tmp/plan2-r5-prompt.md`, last message `/tmp/plan2-r5-review-last.json`.
- Reviewed SHA: `e6f7d2cf94b9663bea68d8edd99c484f18f1c882`; base `066a2a81d796b92dd68721cfb05bf8356b66c492`. Verdict: `reopened`, `guard_confirmed: true`.

Findings and triage:

1. `PLAN_TREE_REPLACE_EPOCH_GUARD.md:63` — no freeze-before-drain ordering: handlers `await flushPendingEditorSave()` *before* the tree op/barrier; typing during the in-flight drain schedules a new save that the completing flush marks clean, and barrier entry cancels it. **ADMITTED**: verified `Editor.tsx` flush captures markdown at `:168` then unconditionally `setDirtyTracked(false)` at `:195` (same pattern `:88`/`:121` in `saveCurrent`); drain call sites `Revisions.tsx:75`, `DocumentHistory.tsx:39`/`:70`, `App.tsx:202`/`:267`/`:297` all precede barrier entry.
2. `PLAN_TREE_REPLACE_EPOCH_GUARD.md:79` — barrier limited to TipTap-derived writers; `Inspector.tsx` and `Corkboard.tsx` can overwrite restored metadata. **ADMITTED**: verified Inspector's debounced metadata save (`setTimeout(save, 1500)`, ~`:361`) and Corkboard `handleSummarizeAll` (`:65`–`:93`) writing captured pre-op `label`/`status`/`keywords` via `updateDocumentMetadata` in an async loop holding a stale `latest` project reference.
3. `PLAN_TREE_REPLACE_EPOCH_GUARD.md:104` — counted lease prevents premature resume but does not order reload/rebuild lifecycles; an earlier op's rebuild can complete last, leaving the buffer on an earlier snapshot than disk. **ADMITTED**: the plan itself (:104–:115) treats queued overlap as reachable; nothing in the lease contract sequences rebuild completion against a later op's disk mutation; trigger disabling is React-state based (async) and explicitly belt-and-suspenders.

Revision folded in: step 4 gains freeze-before-drain (barrier entry precedes the pre-operation drain; drain runs under the lease) and non-TipTap writer gating (Inspector debounced metadata, Corkboard `aiSummarize` batch); the counted-lease bullet requires serializing the operation-through-rebuild lifecycle or a generation-checked fresh rebuild at final release. Files table: `Editor.tsx` row extended (mark-clean race `:121`/`:195`); new `Inspector.tsx`/`Corkboard.tsx` row; UI-tests row updated. Tests: overlapping-operation regression now asserts final buffer contents; new preflight-typing and metadata-writer regressions. Intro updated to "rounds 2–5". No new owner decision required. Round 6 to verify.

## Round 6 — reopened (all findings admitted; one narrowed)

- Dispatched: codex 0.144.4 (gpt-5.6), job 6, prompt `/tmp/plan2-r6-prompt.md`, last message `/tmp/plan2-r6-review-last.json`. **Process deviation:** this dispatch ran `--dangerously-bypass-approvals-and-sandbox` (danger-full-access) instead of rounds 1–5's `-s read-only --ephemeral`; working tree verified clean at `b4686cb` afterwards (`git status` empty, HEAD unchanged). Round 7 returns to read-only.
- Reviewed SHA: `b4686cb7738ce9e7b6e124928e58b3c1d295fe86`; base `066a2a81d796b92dd68721cfb05bf8356b66c492`. Verdict: `reopened`, `guard_confirmed: true` (envelope valid, SHAs echo dispatch, exit 0).
- Triage method: three parallel adversarial verifier agents (workflow `wf_0cb9dc91-d53`), one per finding, each instructed to refute; every citation checked against the working tree.

Findings and triage:

1. `PLAN:49` — arming point too late in fast-forward paths: `merge_draft`/`sync_pull` move the branch ref (`set_target`, `git.rs:601`/`:935`) before fallible `set_head`/checkout; plan arms only before the first working-tree write. **ADMITTED, NARROWED**: FF chain confirmed end-to-end incl. `save_revision` staging everything and committing onto the advanced HEAD with the stale tree (`git.rs:230–:251`) — a silent revert of pulled content. The `switch_draft` grouping was REFUTED: its only fallible post-HEAD-move step is `checkout_head` itself, which the drop guard (armed before checkout, bumps on error) already covers. Fix: arm before `set_target` in the two FF branches; count ref/HEAD moves as mutations; injection point between `set_target` and checkout.
2. `PLAN:135` — non-TipTap protection is an allowlist. **ADMITTED**: all cited writers verified (`StatsPanel.tsx:33` `recordDailyWords` → `settings/writing-history.json`; `Preview.tsx:79` `saveMeta`; `session.ts:20` `updateSessionTarget`; `CommentsPanel.tsx:43`/`:78` comment updates; threads; Corkboard linking `:97`; Binder create/rename/delete/move; Inspector immediate onChange `:595`/`:622`/`:665`; App Ctrl+N). Verifier found two aggravations: no dispatch seam exists (all ten `ui/src/commands/*.ts` call `invoke` directly → one shared gate is the cheap fix), and `Preview.tsx`/`session.ts` snapshot forms survive a same-path reload (resync only on `project.path` change, `:66–:77`) so reload+rebuild alone never fixes them. Minor cite corrections: StatsPanel write at `:33`; Inspector debounce at `:364`; CommentsPanel deleteComment at `:69`.
3. `PLAN:63` — app-level revision writers outside the barrier. **ADMITTED, WIDENED**: auto-commit (`App.tsx:261–:276`), backup timer (`:290–:303` reusing `backup_on_close`), close path (`:196–:215`) verified; `backup_on_close` (`commands/git.rs:329`, dirty check `:344`, `save_revision` `:345`) and core `save_revision` (`add_all(["*"])`, single-parent commit) have no merge-state check. The conflict-markers commit is reachable **today** with no overlap (conflict window > 10-min timer, or close during resolution); F-009 fixed only `merge_draft`'s internal path. Fix is two-layer: core-side merge-state refusal in `save_revision`/`backup_on_close` (I2), app-side lease-scoped skip/cancel as belt-and-suspenders.

Revision folded in: approach step 3 re-anchors the guard to the first ref/HEAD/tree mutation (arm before `set_target` in the two FF branches; `switch_draft` explicitly needs no special point); step 4 replaces the writer allowlist with one shared dispatch-layer gate in `ui/src/commands/*`, adds stale-snapshot form resync-on-reload, and adds the app-revision-writers/unresolved-conflicts bullet. Files table: dispatch-gate row supersedes the Inspector/Corkboard row; new Preview/session/StatsPanel, App.tsx, and core `save_revision` merge-state rows. Tests: ref-move boundary, dispatch-gate, unresolved-conflict (fails today), and timer/close overlap regressions. Decisions: round-6 entry asks the owner whether the today-reachable conflict-commit fix rides in this slice or splits. Round 7 to verify.

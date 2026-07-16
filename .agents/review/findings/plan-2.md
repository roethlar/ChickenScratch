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

## Round 7 dispatch

- **Reviewer**: codex-cli 0.144.4, back on rounds 1-5 invocation
  (`codex exec --ephemeral -s read-only --json --output-schema ... -o ...`)
- **Reviewed SHA**: `d7394821db0f1b445a8d9707f5ee887bc3295334` (round-6
  revision commit)
- **Base SHA**: `066a2a81d796b92dd68721cfb05bf8356b66c492` (unchanged)
- **Bound**: 1800 s
- **Dispatched**: 2026-07-16 (prompt `/tmp/plan2-r7-prompt.md`; verdict ->
  `/tmp/plan2-r7-review-last.json`; round-6 findings quoted verbatim with
  the recorded switch_draft narrowing flagged as settled triage; reviewer
  asked to probe remaining bypasses of the dispatch gate, other
  ref-move-first orderings, and false-positive risk of the
  unresolved-conflict refusal)
- **Verdict**: `reopened` (envelope valid; see Round 7 record below)

## Round 7 — reopened (all five findings admitted)

- Verdict received 2026-07-16: `reopened`, `guard_confirmed: true`, envelope valid (verdict in enum, reviewed/base SHAs echo dispatch `d739482`/`066a2a8`, exit 0). Verdict file `/tmp/plan2-r7-review-last.json`.
- Triage method: five parallel adversarial verifier agents (workflow `wf_dcbf226c-7bc`), one per finding, each instructed to refute against plan text and code. All five **CONFIRMED**, several with widened or corrected scope.

Findings and triage:

1. `PLAN:163` — the dispatch-gate wording ("awaits (or is refused while) the lease") permits defer-then-send: a queued dispatch keeps captured pre-operation args and lands them under a fresh token after release. **ADMITTED**: verifier confirmed the Approach text offers "awaits" as first-class while the plan's own Tests section (dispatch-gate regression) forbids the outcome — an internal contradiction a cold implementer resolves the wrong way. Fix: refuse/cancel or generation-validate; deferral never compliant.
2. `PLAN:163` — the gate lacks an owner-scoped admission path and is self-contradictory with freeze-before-drain: the pre-operation drain (`docCmd.updateDocumentContent` via `Editor.tsx:189`/`:209`) and the tree-replacing command itself (`gitCmd.restoreRevision`) dispatch through the gated layer, so await-gate deadlocks its own lifecycle and refuse-gate self-aborts every dirty-buffer operation. **ADMITTED**: verified end-to-end (flush routing editorRef→Editor.tsx→document.ts; ops through git.ts). Fix: lease handle; owner dispatches bypass; exemption covers Revisions/DocumentHistory/App.tsx trigger sites.
3. `PLAN:167` — `Preview.tsx:3` imports `invoke` directly; `saveMeta` (`:81`) calls `update_project_metadata` outside the seam. **ADMITTED**: verifier swept ui/src — Preview is the only project-mutating component-level bypass (App.tsx `backup_on_close` :209/:298 is covered by the timer/close bullet; all other component invokes are non-mutating plugins). Fix: migrate saveMeta into the commands layer + ESLint `no-restricted-imports` to keep the seam closed.
4. `PLAN:173` — resync/versioned-refusal alone silently discards form drafts typed before/during an operation; forms are not disabled today and the plan freezes only the editor and the tree-op triggers. **ADMITTED**: verified no disabled props on Preview meta or session-target inputs; noted forms have no flush analog, so freezing alone cannot persist an already-dirty form → loud-drop requirement. SessionTargetSection already discards drafts on project-ref change today (plan systematizes an existing hazard there; Preview is where it would be newly introduced).
5. `PLAN:192` — blanket `save_revision` merge-state refusal strands manual resolution. **ADMITTED, WIDENED**: verified "Resolve manually" only closes the dialog (`Revisions.tsx:470`); no continue-merge command exists (only cleanup_state calls are unreachable clean-merge paths and the two aborts); index stays conflicted until staged and the app's only staging call is `save_revision`'s own `add_all`; `restore_revision`/`restore_document`/`backup_current_work` call `save_revision` internally so a blanket refusal bricks restore and manual backup too; today's save_revision never calls `cleanup_state`, so pre-existing projects can carry lingering `MERGE_HEAD` — a MERGE_HEAD-keyed refusal would brick them permanently. Fix: merge-aware completion shape in `save_revision` (refuse only on index conflicts; on clean staging commit with two parents [HEAD, MERGE_HEAD] + `cleanup_state` + epoch bump; automatic writers refuse during any merge state; the completion shape self-heals lingering MERGE_HEAD).

Revision folded in: dispatch-gate bullet rewritten with three round-7 sub-clauses (refuse-never-defer; owner-scoped lease-handle admission; seam closure via Preview migration + ESLint rule); stale-snapshot bullet now freezes forms during lease, resyncs only non-dirty fields, drops undroppable drafts loudly; conflict bullet replaced the blanket refusal with the merge-aware completion shape + migration note. Files table rows updated (gate, forms, save_revision); Tests: dispatch-gate regression now asserts refuse-not-defer + owner-admission (deadlock/self-abort shown), new form-freeze/loud-drop regression, unresolved-conflict regression extended with the completion path and MERGE_HEAD self-heal. Decisions: round-6 conflict-split entry amended — the fix grew into the completion shape and is the largest separable sub-slice. Round 8 to verify.

## Round 8 dispatch

- **Reviewer**: codex-cli 0.144.4, rounds 1-5/7 invocation
  (`codex exec --ephemeral -s read-only --json --output-schema ... -o ...`)
- **Reviewed SHA**: `e7576f99ff57cf2ea0a2421c38320cbafdae16ea` (round-7
  revision commit)
- **Base SHA**: `066a2a81d796b92dd68721cfb05bf8356b66c492` (unchanged)
- **Bound**: 1800 s
- **Dispatched**: 2026-07-16 (prompt `/tmp/plan2-r8-prompt.md`; verdict ->
  `/tmp/plan2-r8-review-last.json`; round-7 findings quoted verbatim;
  reviewer asked specifically about lease-handle coverage of the reload
  dispatches, completion-shape interaction with save_revision's internal
  callers (could restore silently complete a half-resolved merge?), and
  internal consistency after seven rounds)
- **Verdict**: `reopened` (envelope valid; see Round 8 record below)

## Round 8 — reopened (all five findings admitted; round-7 completion shape replaced)

- Verdict received 2026-07-16: `reopened`, `guard_confirmed: true`, envelope valid (verdict in enum, reviewed/base SHAs echo dispatch `e7576f9`/`066a2a8`, exit 0). Verdict file `/tmp/plan2-r8-review-last.json`.
- Triage method: five parallel adversarial verifier agents (workflow `wf_507803db-40d`), one per finding. All five **CONFIRMED**; r8-3/r8-4/r8-5 together invalidated the round-7 merge-completion design, which is replaced this round.

Findings and triage:

1. `PLAN:172` — owner admission omits the post-operation reload. **ADMITTED**: `load_project` is permit-backed and conditionally disk-mutating (`project.rs:55–:59` acquires a WritePermit and runs `read_project_with_repair`, which self-heals missing folders — `reader.rs:306–:308`, `:380–:411`; token cache refreshed at `:60`); the mandated reload runs while the lease is held, so a gate built to the plan's enumeration refuses its own recovery (reload error swallowed by `projectStore.ts:81–:82`) or, mis-classified read-only, bypasses the gate. Omission was in three plan locations (admission clause, Files gate row, dispatch-gate test). Nuances recorded: mutation is conditional; the Degraded path is permit-free; classification must be per-command since the UI can't see fidelity.
2. `PLAN:192` — Inspector is a residual stale-snapshot writer. **ADMITTED**: resync keys on document id only (`Inspector.tsx:280–:307`), so a same-doc restore leaves stale fields that the debounce (`:364`), immediate handlers (`:595`/`:622`/`:665`), or a stale title blur (`renameNode` `:356`) re-submit post-release, under no lease. Fix: Inspector joins the forms bullet; resync keys on reload generation/content, never id/path alone; regressions extended to the post-release path (existing tests only covered saves spanning the operation).
3. `PLAN:239` — completion algorithm incompatible with git2 semantics. **ADMITTED, decisive**: verifier confirmed from the pinned crate sources (git2 0.19.0 `index.rs:292–:294`; vendored libgit2 1.8.1 `index.c:1550`) that `add_all` clears conflict entries unconditionally — check-before-staging strands resolved-but-unstaged writers (staging is unreachable), check-after-staging blesses untouched markers as a two-parent merge commit and `cleanup_state` then disarms the automatic-writer refusal. No ordering satisfies both test bullets the plan carried.
4. `PLAN:244` — no caller provenance. **ADMITTED, WIDENED**: auto-commit uses the same Tauri `save_revision` command and args as manual save (only the message differs), so manual-vs-automatic is not implementable at core or command layer as written; `backup_current_work` has no dirty guard (`git.rs:738–:739`); restore under lingering `MERGE_HEAD` would mint a false merge parent, corrupting history-diff logic. Wider caller surface recorded: project creation, import, TUI save also reach `save_revision`. UI timer gating needs a backend merge-state query (none exists) to survive restart.
5. `PLAN:242` — inline epoch bump invalidates the caller's live permit. **ADMITTED**: `backup_current_work` calls `push_backup` with the same permit (`git.rs:744` → `ensure_valid_for` `:682` → stale → ReadOnly error after the commit landed); the Tauri `save_revision` command's backup/remote pushes (`src-tauri git.rs:24`/`:32`) are silently lost. Ordering constraint recorded: restore helpers are safe only because nothing validates the permit after their internal save.

Round-8 redesign folded in (replaces the round-7 completion shape): `save_revision` refuses during any merge state, all callers, no provenance; new explicit core `complete_merge` (stage, two-parent commit, `cleanup_state`; epoch bump via the step-2 drop guard at scope exit, never inline); backend merge-state query + persistent merge-in-progress UI with Complete/Abort (the user's explicit act replaces the impossible index-state detection); lingering `MERGE_HEAD` migrates via the same prompt. Owner-admission clause/Files/Tests now cover the post-op reload; Inspector joins the forms bullet with generation-keyed resync; regressions updated (reload-under-handle, post-release Inspector clobber, complete_merge continuation with live permit). Decisions: conflict sub-slice re-flagged as the largest separable piece (now a real sub-feature: command + query + UI state). Round 9 to verify.

## Round 9 dispatch

- **Reviewer**: codex-cli 0.144.4, standard invocation
  (`codex exec --ephemeral -s read-only --json --output-schema ... -o ...`)
- **Reviewed SHA**: `d851a8f5f75274d8f3378c312298d436ad53f302` (round-8
  redesign commit)
- **Base SHA**: `066a2a81d796b92dd68721cfb05bf8356b66c492` (unchanged)
- **Bound**: 1800 s
- **Dispatched**: 2026-07-16 (prompt `/tmp/plan2-r9-prompt.md`; verdict ->
  `/tmp/plan2-r9-review-last.json`; round-8 findings quoted verbatim;
  reviewer asked to probe the redesign's own seams: does blanket
  save_revision refusal break creation/import/TUI flows, how does
  complete_merge interact with the dirty guards and the lease, does the
  merge-state query survive restarts; explicit note that after eight
  rounds a clean pass is the honest signal if the plan now holds)
- **Verdict**: `reopened` (envelope valid; see Round 9 record below)

## Round 9 — reopened (three admitted, one admitted-in-part)

- Verdict received 2026-07-16: `reopened`, `guard_confirmed: true`, envelope valid (SHAs echo dispatch `d851a8f`/`066a2a8`, exit 0). Verdict file `/tmp/plan2-r9-review-last.json`.
- Triage method: four parallel adversarial verifier agents (workflow `wf_3311533f-244`), one per finding. First partial refutation of the loop: r9-1's backup half was declined on evidence.

Findings and triage:

1. `PLAN:280` — save_revision-only refusal too low in the call graph. **PARTIAL → restore half ADMITTED, backup half DECLINED.** Restore: both helpers pass the status-only dirty guard under clean tree + lingering `MERGE_HEAD`, mutate disk (`git.rs:406–:411`, `:446–:450`), then hit the internal refusal — half-completed restore whose exits falsify history (Complete) or discard it (Abort); trigger state reachable via today's auto-commit-during-conflict bug (the plan's own migration cohort). Fix: merge-state preflight next to `reject_dirty_worktree`, before any write; test asserts zero worktree mutation. Backup: DECLINED — `push_backup` pushes only the branch ref (`git.rs:716–:724`), `MERGE_HEAD` is local-only, a clean tree commits nothing; refusing the push would reduce backup protection for zero integrity gain. Manual backup refuses only the commit half; `backup_on_close` must surface (not `let _`) the refusal.
2. `PLAN:282` — TUI claim false. **ADMITTED, NARROWED**: TUI *revision* saves call core `save_revision` directly (`tui/app.rs:974`) with zero merge/conflict UI; TUI *document* saves are pure file writes and genuinely unaffected; creation/import unaffected. Proportionate fix: self-describing refusal message (TUI status line prints core errors verbatim) — full TUI Complete/Abort parity NOT warranted (TUI cannot create merge state). Known limitation noted: markers in `project.yaml` make the project unopenable in both UIs (pre-existing).
3. `PLAN:284` — ordinary permits cannot authorize recovery. **ADMITTED, WIDENED**: a `project.yaml` conflict makes the probe *error* (`fidelity.rs:333–:335`; `load_project` fails after restart — worse than Degraded), a `.meta` conflict probes Degraded; `ProjectTokens` cannot reissue; `sync_abort_pull` is permit-gated → recovery unreachable exactly when conflicts touch format files. Verifier refuted the finding's own mechanism (fidelity reasons can't carry this — `DegradedReason` has no merge variant and the yaml case errors before classification): the capability must key on repo merge state via the new query. **Also a live pre-existing bug today** — recorded as a new ranked finding in `.agents/state.md`. Fix: recovery-scoped capability authorizing `complete_merge`/`sync_abort_pull`/`sync_pull_force`; read-only open tolerates unparsable `project.yaml` under `MERGE_HEAD` (fall back to pre-merge HEAD copy for display).
4. `PLAN:291` — Complete merge lacked a frontend lifecycle. **ADMITTED, with taxonomic root cause**: the plan's barrier rules keyed on "tree-replacing" and `complete_merge` replaces no files — but it is epoch-bumping and commit-minting. The debounce race (resolve markers in editor, click Complete before `auto_save_seconds`) commits marker-laden disk as the permanent two-parent merge commit, with the real resolution landing as a later ordinary edit (queued save re-tokens via checkout; no lease held → gate inactive). Fix: barrier lifecycle keys on *epoch-bumping operations*; `complete_merge` (and the migration prompt path) enrolled — lease, freeze-before-drain, flush + dispatch under owner handle, reload+rebuild; explicit note that the flush is not blocked by the merge-state refusal (drain goes through `update_document_content`, never `save_revision`); Abort deliberately skips the flush; the confirmation dialog narrows but does not close the race.

Revision folded in: design points (a)–(e) rewritten (restore preflight; backup commit-half-only refusal; TUI self-describing error; complete_merge full lifecycle; recovery-scoped capability; migration prompt lifecycle). Files row updated. Tests: zero-mutation restore preflight, complete-merge lifecycle regression, format-file-conflict recovery regression (fresh command boundary, Abort AND Complete). `.agents/state.md` gains the pre-existing abort-unreachable finding. Round 10 to verify.

## Round 10 dispatch

- **Reviewer**: codex-cli 0.144.4, standard invocation
  (`codex exec --ephemeral -s read-only --json --output-schema ... -o ...`)
- **Reviewed SHA**: `1d34cfe7c389d2105a5e6d7ada876b523c508884` (round-9
  revision commit)
- **Base SHA**: `066a2a81d796b92dd68721cfb05bf8356b66c492` (unchanged)
- **Bound**: 1800 s
- **Dispatched**: 2026-07-16 (prompt `/tmp/plan2-r10-prompt.md`; verdict ->
  `/tmp/plan2-r10-review-last.json`; round-9 findings + dispositions quoted
  incl. the backup-push refutation; reviewer asked to probe the recovery
  capability (forgeable? preserves safe-path checks?), the epoch-bumping
  re-keying (consistency sweep for leftover "tree-replacing" wording), and
  the read-only-open fallback implementability)
- **Verdict**: `reopened` (envelope valid; see Round 10 record below)

## Round 10 — reopened (all four findings admitted)

- Verdict received 2026-07-16: `reopened`, `guard_confirmed: true`, envelope valid (SHAs echo dispatch `1d34cfe`/`066a2a8`, exit 0). Verdict file `/tmp/plan2-r10-review-last.json`.
- Triage method: the 4-agent verification workflow failed on a subagent session limit, so triage fell back to the playbook's single-agent mode — the coder verified every citation inline against the working tree (same discipline: refute-first, exact lines). ptk was disconnected mid-round; built-in tools used (PTK_DIRECT).

Findings and triage:

1. `PLAN:304` — "TUI prints core errors verbatim" is false. **ADMITTED, nuanced**: the revision-failure branch formats with `{:?}` (`tui/app.rs:992`; backup errors `:980`); only the token-refusal branch (`:967`) uses Display. The merge-state refusal would flow through `:992`. Fix: switch those branches to Display + rendered-message test.
2. `PLAN:346` — recovery capability lacked `WritePermit`'s safety contract. **ADMITTED**: plan step 2 arms the drop guard "from the `WritePermit`" (`:42–:43`, Files `:388`) and point (e) specified no construction/validation contract. Fix: engine-only non-Clone construction, canonical-root binding validated at use, merge state re-attested at use, guard arming surface widened to "permit or recovery capability"; wrong-root and outside-merge negative tests.
3. `PLAN:336` — capability alone doesn't unblock `sync_pull_force`. **ADMITTED, WIDENED**: `reject_dirty_worktree` (`git.rs:1042`, `:1059`) fires on every conflicted tree and `revalidate_fidelity` (`:1045`) fails on format-file conflicts — so the conflict dialog's "Overwrite local with remote" exit is broken **today for any real conflict** (`handleForcePull` never aborts first). Second live pre-existing bug; state.md ranked finding extended to cover both exits. Fix: merge-attested force path replaces those checks under attested merge state only; conflict-to-Force regression added.
4. `PLAN:351` — HEAD-copy fallback unreachable through the reader API. **ADMITTED**: `read_project_readonly` → `read_project_impl` unconditionally parses the worktree `project.yaml` (`reader.rs:311`); `reader.rs` was absent from Files. Fix: reader.rs in scope with a read-only entry accepting verified HEAD metadata (root/safe-read checks preserved); recovery test asserts `load_project` succeeds after restart.

Revision folded in: design point (a) TUI sentence corrected (+ Display fix in scope); point (e) gains the capability contract, the merge-attested force path, and the reader-fallback reachability requirement; Files gains `reader.rs` and `tui/app.rs` rows and the capability-contract wording; the recovery regression now covers Abort AND Complete AND Force, `load_project`-after-restart, capability negative tests, and TUI rendering. `.agents/state.md` ranked finding extended (force exit broken today for any conflict). Round 11 to verify.

## Round 11 dispatch

- **Reviewer**: codex-cli 0.144.4, standard invocation
  (`codex exec --ephemeral -s read-only --json --output-schema ... -o ...`)
- **Reviewed SHA**: `72326e499738136b533a10f1404e0836f486c06e` (round-10
  revision commit)
- **Base SHA**: `066a2a81d796b92dd68721cfb05bf8356b66c492` (unchanged)
- **Bound**: 1800 s
- **Dispatched**: 2026-07-16 (prompt `/tmp/plan2-r11-prompt.md`; verdict ->
  `/tmp/plan2-r11-review-last.json`; round-10 findings + dispositions
  quoted; reviewer asked to probe the merge-attested force path (stale
  attestation; pre-merge uncommitted edits), the widened guard-arming
  surface, the reader HEAD-metadata entry's interaction with hierarchy/
  safe-path validation, and overall internal consistency after ten rounds)
- **Verdict**: `reopened` (envelope valid; see Round 11 record below)

## Round 11 — reopened (all four findings admitted)

- Verdict received 2026-07-16: `reopened`, `guard_confirmed: true`, envelope valid (SHAs echo dispatch `72326e4`/`066a2a8`). Verdict file `/tmp/plan2-r11-review-last.json`.
- Triage method: inline single-agent verification (subagent pool still capped); r11-1/r11-2/r11-4 confirmed from evidence already gathered this session, r11-3 confirmed by fresh reads of `reader.rs:310–:324` and `:855–:882`.

Findings and triage:

1. `PLAN:43` — step 2 and the Files `fidelity.rs` row still restricted guard arming to `WritePermit`, contradicting round-10's permit-or-recovery requirement; no test asserted epoch invalidation after recovery-authority operations. **ADMITTED** (internal-consistency defect — round 10 widened only point (e)). Fixed in step 2, the Files row, and a new epoch-invalidation-after-recovery test (incl. injected partial failure).
2. `PLAN:372` — merge-attested force path lacked last-safe-point re-attestation. **ADMITTED**: today's code deliberately re-checks after the blocking fetch (`git.rs:1057–:1059`); the attested bypass without re-attestation could hard-reset unrelated new work if the merge completed/aborted between validation and reset. Fixed: re-attest before the reset, fall back to ordinary checks on failure; race regression.
3. `PLAN:382` — HEAD-metadata fallback still fails on HEAD/worktree hierarchy skew. **ADMITTED**: identities derive from the hierarchy (`reader.rs:315`) but sidecars load from the worktree (`:316`), and `:317`'s strict matching (`:861–:875`) hard-errors on ID/path mismatch (e.g. remote delete/recreate at the same path). Fixed: recovery-mode (display-only) load relaxes strict matching — skewed entries load as unlinked/placeholder, never an open failure; ordinary load unchanged; skew test.
4. `PLAN:413` — "one concern, one branch, one commit" no longer honest with three splits open in Decisions. **ADMITTED** (governance): step 5 now enumerates four commit boundaries (core guard; merge/recovery; UI barrier; vitest+CI) and requires the owner's sweep-or-split choice recorded on the plan status line before implementation; the round-3/4/6 Decisions questions collapse into that single choice.

Round 12 to verify.

## Round 12 dispatch

- **Reviewer**: codex-cli 0.144.4, standard invocation
  (`codex exec --ephemeral -s read-only --json --output-schema ... -o ...`)
- **Reviewed SHA**: `0a830e93d9efb29ad8a44d31875b86867c2654c5` (round-11
  revision commit)
- **Base SHA**: `066a2a81d796b92dd68721cfb05bf8356b66c492` (unchanged)
- **Bound**: 1800 s
- **Dispatched**: 2026-07-16 (prompt `/tmp/plan2-r12-prompt.md`; verdict ->
  `/tmp/plan2-r12-review-last.json`; round-11 findings + dispositions
  quoted; reviewer asked to probe the four commit boundaries (independent
  greenness and honest temporary exposures), placeholder-leak risk in the
  recovery-mode load, and re-attestation fallback semantics)
- **Verdict**: `reopened` (envelope valid; see Round 12 record below)

## Round 12 — reopened (all three findings admitted)

- Verdict received 2026-07-16: `reopened`, `guard_confirmed: true`, envelope valid (SHAs echo dispatch `0a830e9`/`066a2a8`). Verdict file `/tmp/plan2-r12-review-last.json`.
- Triage method: inline single-agent verification; all three findings checked against plan text and evidence already gathered this session (step-5 dependency DAG; `sync_pull_force`'s remote-only target `git.rs:1044–:1051`; the conflict dialog serving both pull and draft origins, `Revisions.tsx:38–:40`; clean-tree-passes-ordinary-checks logic).

Findings and triage:

1. `PLAN:434` — the "independently green" boundary order was impossible: the merge/recovery UI needs the barrier (deferred to iii) and its regressions need vitest (deferred to iv); the core-guard boundary silently left the app-layer clobber live. **ADMITTED**: step 5 reordered into dependency order — (1) vitest+CI, (2) core guard (with the intermediate exposures stated: UI clobber until (3), conflict commit until (4)), (3) UI barrier, (4) merge/recovery. Decisions entry renumbered to match.
2. `PLAN:572` — the recovery test promised Force for both conflict origins, but `sync_pull_force` fetches/resets to `refs/remotes/sync/<branch>`; after a draft conflict there may be no sync remote, or an unrelated one. **ADMITTED**: attested force is now source-aware (remote tracking ref for pull; `MERGE_HEAD` for draft merges), with per-origin resulting-tree assertions. (Latent-wrong-source is masked today by live bug #2 — the dirty check fires first — so no new state.md entry.)
3. `PLAN:379` — failed re-attestation must fail closed: another process completing/aborting the merge leaves a clean Full tree, so the round-11 "fall back to ordinary checks" would pass and the reset would discard the new state (and contradict outside-merge-refused). **ADMITTED**: fail closed with fresh authority/confirmation; race regression covers clean-completion and abort states.

Round 13 to verify.

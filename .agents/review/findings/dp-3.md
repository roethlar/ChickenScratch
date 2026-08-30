# dp-3: Draft peek parses sidecar names as text lines, not YAML

**Severity**: LOW — display-name cosmetics in the peek list only; falls back
to the file stem, never blocks reading.
**Status**: Verified
**Branch**: — (default-branch mode)
**Commit**: `4b70015` + repair `bfe52d1`

## Reviewer comments
Round 1 — Reviewer: codex / gpt-5.6-sol / xhigh / standard
Harness: codex-cli (MCP transport), thread 01a05461-2ec3-7b62-a7f2-d8ae0f22cffa
Reviewed: 4b70015475c69df651b6ebeb8b10b4ab3ffc2ec8
Base: 3b47d6f358b34069090ac15cd62ea30755f35091
guard_confirmed: true. Verdict: reopened — 2026-08-29 (UTC):
- crates/core/src/core/git.rs:1770 — post-decode trim() strips semantic
  whitespace from quoted names (`'  Padded  '` → `Padded`), diverging from
  the project reader. Remove the trim.

Round 2 (repair-delta) — Reviewer: codex / gpt-5.6-sol / xhigh / frontier
  escalated: T5 (no recorded frontier pair; owner-named dispatch pair used)
Harness: codex-cli (MCP transport), thread 01a05466-edc3-7941-98a1-8edd9ce66f80
Reviewed: bfe52d1e110a9abc2b8b3b40e1f340283a73b1c3
Base: 4b70015475c69df651b6ebeb8b10b4ab3ffc2ec8 (pre-repair head)
guard_confirmed: true (trim reinstated in reviewer worktree → padded
assertion FAIL; head restored → PASS)
Verdict: accepted — 2026-08-29 (UTC), comments: none.
Record committed as part of verification history.

## Evidence
`crates/core/src/core/git.rs` `sidecar_name` closure in
`list_draft_documents` (landed `1ac3ddc`) — matches a literal `name:` line
prefix and hand-strips one pair of surrounding quotes.

## Predicted observable failure
A sidecar the real project reader accepts — `name : X` (space before colon),
single-quoted names with doubled internal quotes (`'It''s Corn'`, which
serde_yaml itself emits), block/folded scalars — shows the wrong name in
Draft Peek: doubled quotes, `>-`, or the bare file stem.

## What
The peek re-implemented a YAML subset instead of using the YAML parser the
crate already depends on; tolerant fallback (I5) masked the mismatch.

## Approach
Parse the sidecar blob with `serde_yaml::Value` and read `name` as a string;
keep the stem fallback for missing/invalid sidecars. Guard test uses a
doubled-single-quote name the line parser mangles.

## Files changed
- `crates/core/src/core/git.rs` — sidecar_name via serde_yaml
- `crates/core/tests/draft_peek.rs` — YAML-quoting test case

## Guard proof
- `crates/core/tests/draft_peek.rs::peek_parses_yaml_sidecar_names` —
  `name: 'It''s Corn'` must display as `It's Corn`. Reverting to the line
  parser fails it.

## Coder dispute (if any)
None.

# dp-3: Draft peek parses sidecar names as text lines, not YAML

**Severity**: LOW — display-name cosmetics in the peek list only; falls back
to the file stem, never blocks reading.
**Status**: Open
**Branch**: — (default-branch mode)
**Commit**: (filled in after commit)

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

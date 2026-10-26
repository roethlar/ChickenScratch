# Review status

Workflow: see `.agents/playbooks/codereview.md`.
Per-item detail: see `.agents/review/findings/<id>.md`.

## Legend
- `[ ]` Admitted, open
- `[~]` In progress / pending review
- `[x]` Verified / accepted
- `[!]` Contested — awaiting owner adjudication
- `[-]` Declined at intake

## Items

| ID     | Type | Subject                                   | Status | Branch |
|--------|------|-------------------------------------------|--------|--------|
| plan-1 | plan | `docs/plans/PLAN_TRUST_FOUNDATIONS.md`    | `[x]`  | n/a (doc review on master) |
| plan-2 | plan | `docs/plans/PLAN_TREE_REPLACE_EPOCH_GUARD.md` | `[x]`  | n/a (doc review on master) |
| plan-3 | plan | `docs/plans/PLAN_SCRIVENER_FIDELITY.md` (codex gpt-6-astra xhigh, 5 rounds; I2 wording ruling with owner) | `[x]`  | n/a (doc review on master) |
| s1-1 | code | Fallback skipping deletes the letter after a character control word (range b5f958b..caf8a5e, codex; fix + 2 repairs) | `[x]`  | — (default-branch mode) |
| s1-2 | code | Truncated hyperlink field loses its buffered text (range b5f958b..caf8a5e, codex) | `[x]`  | — (default-branch mode) |
| s1-3 | code | Fallback decodes every ANSI byte as Windows-1252 (range b5f958b..caf8a5e, codex) | `[~]`  | — (default-branch mode) |
| s1-4 | code | Surrogate pairs disappear in the fallback (range b5f958b..caf8a5e, codex) | `[x]`  | — (default-branch mode) |
| s1-5 | code | Unescaped `)` in a hyperlink target truncates the link (range b5f958b..caf8a5e, codex) | `[x]`  | — (default-branch mode) |
| s1-6 | code | `\line` breaks render as spaces (range b5f958b..caf8a5e, codex) | `[x]`  | — (default-branch mode) |
| s2-1 | code | Internal links resolve against predicted paths; duplicate titles redirect them (slice 2 review, codex) | `[x]`  | — (default-branch mode) |
| s3-1 | code | Opening the Inspector on a selected document can wipe its metadata (slice 3 review, codex) | `[~]`  | — (default-branch mode) |
| s3-2 | code | Research roots and attachments bypass the metadata import (slice 3 review, codex) | `[~]`  | — (default-branch mode) |
| s3-3 | code | An unreadable synopsis file is treated as no synopsis (slice 3 review, codex) | `[x]`  | — (default-branch mode) |
| s4-1 | code | Non-breaking space before an inline annotation panics the import (slice 4 review, codex) | `[~]`  | — (default-branch mode) |
| s4-2 | code | Two inline annotations at a paragraph start panic the import (slice 4 review, codex) | `[x]`  | — (default-branch mode) |
| s4-3 | code | Literal inline-note text decoded twice, braces and backslashes lost (slice 4 review, codex) | `[~]`  | — (default-branch mode) |
| s4-4 | code | Inline notes lose the document's encoding (slice 4 review, codex) | `[~]`  | — (default-branch mode) |
| s4-5 | code | A bracket in anchor text keeps a footnote from attaching (slice 4 review, codex) | `[~]`  | — (default-branch mode) |
| s4-6 | code | A folder with empty text drops its comments (slice 4 review, codex) | `[~]`  | — (default-branch mode) |
| s4-7 | code | A quoted `>` in a footnote body drops it from export (slice 4 review, codex) | `[~]`  | — (default-branch mode) |
| s4-8 | code | Compile rewrites footnote-shaped markup inside code examples (slice 4 review, codex) | `[~]`  | — (default-branch mode) |
| s4-1 | code | Force not bound to the confirmed merge (slice 4, codex) | `[x]`  | n/a (pre-commit fix in slice-4 working tree) |
| s4-2 | code | Recovery fingerprint omits index contents (slice 4, codex) | `[x]`  | n/a (pre-commit fix in slice-4 working tree) |
| s4-3 | code | No re-attestation at the last safe point (slice 4, codex) | `[x]`  | n/a (pre-commit fix in slice-4 working tree) |
| s4-4 | code | Stale merge-state responses cross projects (slice 4, codex) | `[x]`  | n/a (pre-commit fix in slice-4 working tree) |
| dp-1 | code | "Everything backed up" overclaims a one-draft push (range c99acdd..0a891b2, codex) | `[x]`  | — (default-branch mode) |
| dp-2 | code | Restore confirm promises a History save that never happens (range c99acdd..0a891b2, codex) | `[x]`  | — (default-branch mode) |
| dp-3 | code | Peek sidecar names parsed as text lines, not YAML (range c99acdd..0a891b2, codex) | `[x]`  | — (default-branch mode) |
| ar-1 | code | Failed attachment copy leaves a Degraded project shown writable (range a4d39f1..e2b3d1b, codex) | `[x]`  | — (default-branch mode) |
| ar-2 | code | Attachments missing at discovery still silently dropped (range a4d39f1..e2b3d1b, codex) | `[x]`  | — (default-branch mode) |
| ar-3 | code | Already-staged temp leftovers can still be committed (range a4d39f1..e2b3d1b, codex) | `[x]`  | — (default-branch mode) |
| ar-4 | code | Relative '..' output paths falsely refused as nested (range a4d39f1..e2b3d1b, codex) | `[x]`  | — (default-branch mode) |

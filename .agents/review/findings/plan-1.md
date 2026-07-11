# plan-1: Review of PLAN_TRUST_FOUNDATIONS.md

**Type**: plan review (adapted from code-finding flow; no branch, no guard
proof — the artifact is a plan document on `master`)
**Status**: In progress (reopened → revising)
**Subject**: `docs/plans/PLAN_TRUST_FOUNDATIONS.md`

## Reviewer comments

- **Reviewer**: codex-cli 0.144.1, `codex exec -s read-only
  --output-schema … -o …`
- **Reviewed SHA**: `e612bc2fcb0bea8e6c1b751cf4225936b846c6f0`
- **Base SHA**: `bf39a24e12031cc4316ded2b98f8517b07f70592`
- **Verdict**: `reopened` — 2026-07-11T05:3xZ (UTC)
- **Note**: first dispatch (same prompt, wider scope) was killed by the
  orchestrator's 900 s timeout before any verdict; re-dispatched at 1800 s.

Findings (all admitted by the coder; none disputed):

1. Classification-after-read is too late: `read_project` already mutates
   disk during load (creates missing folders `reader.rs:250-268,339-379`;
   renames corrupt sidecars `reader.rs:936-960,991-1009`). Need a
   side-effect-free preflight or deferred repairs, and before/after-open
   tree-hash tests for those fixtures.
2. `Full` under-specified: a hierarchy document that never loaded is
   accepted (`reader.rs:824-846`); a missing path only warns
   (`reader.rs:394-406`). Full must require every hierarchy document to
   resolve to loaded content; add a missing-document fixture.
3. `format_version` is an unguarded downgrade path: reader accepts any
   version (`reader.rs:41-47`); writer stamps current (`writer.rs:244-259`).
   Unsupported/newer versions must classify Degraded before any write.
4. Fidelity carried only on `Project` cannot guard path-only mutators
   (`writer.rs::delete_document`, git restore/draft/backup/sync) or the
   new-project init case. Specify one authoritative side-effect-free path
   guard or a non-forgeable write capability through these APIs.
5. File map error: deletion integration point is
   `writer.rs::delete_document` (`writer.rs:739-784`), not (only)
   `deletion.rs`.

## Coder disposition (round 1)

All five admitted. #5 amended rather than replaced: both `deletion.rs`
(folder deletion) and `writer.rs::delete_document` are guarded. Plan
revised (`3284953`); re-dispatched.

## Reviewer comments — round 2

- **Reviewer**: codex-cli 0.144.1 (same incantation)
- **Reviewed SHA**: `3284953764af0b657d61df41299ad59847131dc9`
- **Base SHA**: `e612bc2fcb0bea8e6c1b751cf4225936b846c6f0`
- **Verdict**: `reopened` — 2026-07-11T06:0xZ (UTC)

Findings (all admitted; resolutions in parentheses):

1. WriteToken not bound to a project root — a token from Full project A
   could mutate Degraded project B (bind token to canonical root; mutators
   validate target under root; cross-project rejection test).
2. No token freshness rule — restore/draft-switch/merge/pull can replace
   the tree with legacy/future content while an old token persists (epoch
   on the token; tree-replacing ops bump epoch, re-probe, reissue; stale
   token refused; test).
3. Repair policy contradiction — missing standard folders marked Degraded
   while Full self-heal is preserved, and `samples/Corn.chikn` itself
   lacks research/templates/settings (resolution: missing standard folders
   are benign self-heal, NOT Degraded; Degraded is reserved for
   content-threatening conditions; fixture (d) becomes a Full-with-
   self-heal case).
4. "Non-empty parse" wording conflicts with valid zero-byte documents —
   Corn contains two (resolution: zero-byte documents are valid; only
   nonzero-bytes-yielding-empty, unparsable, or missing files are
   unresolved; explicit test).
5. Mutator coverage incomplete — public `safe_path` helpers create
   directories tokenlessly and `src-tauri/src/commands/io.rs:436-470`
   writes `settings/writing-history.json` on opening Statistics
   (resolution: project-mutating helpers gated behind the token; the
   writing-history write routed through a token-gated engine API; no-write
   test on Degraded projects covering the stats path).

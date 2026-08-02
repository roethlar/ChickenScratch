# Review status

Workflow: see `.agents/playbooks/reviewloop.md`.
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
| s4-1 | code | Force not bound to the confirmed merge (slice 4, codex) | `[x]`  | n/a (pre-commit fix in slice-4 working tree) |
| s4-2 | code | Recovery fingerprint omits index contents (slice 4, codex) | `[x]`  | n/a (pre-commit fix in slice-4 working tree) |
| s4-3 | code | No re-attestation at the last safe point (slice 4, codex) | `[x]`  | n/a (pre-commit fix in slice-4 working tree) |
| s4-4 | code | Stale merge-state responses cross projects (slice 4, codex) | `[x]`  | n/a (pre-commit fix in slice-4 working tree) |

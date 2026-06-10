# Harvest Report: ChickenScratch, 2026-06-10

Governance rules from this repo that other repos would benefit from.

## Ideas

### One concern per branch and commit during multi-fix agent work

- **Source:** `.review/README.md` ("No broad sweeps") and the incident recorded
  in `REVIEW.md` review passes 1–3: a coder agent accumulated uncommitted fixes
  across 26, then 30, then 37 files spanning 10+ findings before its first
  commit, leaving the reviewer unable to verify any fix in isolation or bisect
  regressions.
- **The rule:** When an agent works through a list of findings or fixes, each
  branch and commit must address exactly one item, and the agent must commit
  each item before starting the next; multi-fix sweep branches require an
  explicit owner request.
- **Why it generalizes:** Agents on long fix lists default to batching, and
  every repo loses bisectability and per-fix review the same way.
- **Proposed home:** `AGENTS.template.md` (a working-rule invariant for
  multi-item fix work).

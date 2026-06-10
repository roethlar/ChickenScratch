# Agent Decisions

**Durable decisions for this repo live in `docs/adr/` (ADR-001..ADR-004 and
onward).** That tree is the canonical decision log — referenced throughout the
docs and by `AGENTS.md` Rule 2. Do not record competing decisions here; add a
new ADR via `docs/adr/template.md` when the owner settles something
architectural, per Invariant I8.

This file exists to satisfy the standard `.agents/` layout and records only
layout-level decisions.

## Decisions

### 2026-06-10 - Standard `.agents/` layer added on top of the June 7 protocol

Status: Active

Decision:
`AGENTS.md` (June 7 protocol) remains the canonical agent guidance, unchanged
except for an appended Bootstrap Handoff section and a session-state pointer.
`.agents/state.md` is the session-level current-state file; `docs/adr/`
remains the decision log; `.agents/repo-map.json` records the verification
commands and their current status.

Reason:
The June 7 governance system is current, deliberate, and fitted to the owner.
The bootstrap migration therefore added the smallest standard layer that makes
state durable across sessions, instead of replacing working guidance.

Supersedes:
`REVIEW.md` as a current-state entry point (banner applied; file retained as
the audit record of the completed 63-finding review cycle).

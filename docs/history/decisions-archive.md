# Decisions Archive

Closed decisions from `.agents/decisions.md`, moved here verbatim when
superseded or adopted, per that file's lifecycle discipline. This is the
provenance log; `.agents/decisions.md` holds what is currently in force or
still open.

## Decisions

### 2026-06-10 - Standard `.agents/` layer added on top of the June 7 protocol

Status: Superseded 2026-07-03

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

Superseded by:
2026-07-03 - AGENTS.md reconciled to the portable AgentGovernanceBootstrap
template (see `.agents/decisions.md`). The toolkit template has since added a
hard requirement — `AGENTS.md` is portable and replaced whole on refresh,
never hand-composed — that this repo's bespoke `AGENTS.md` did not meet; this
decision's rationale (keep working guidance rather than replace it) no longer
matches the toolkit's current design, so it was revisited rather than
silently overridden.

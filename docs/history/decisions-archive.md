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

### 2026-10-24 - Web shell has no login

Status: Adopted 2026-10-24; archived 2026-10-25 (drift sweep). The operative
rule lives in `docs/adr/ADR-007-*.md` §4, `docs/ARCHITECTURE.md` and
`docs/USER_GUIDE.md`. Original record below, verbatim (it had been filed
under Open Decisions in `.agents/decisions.md`).

## 2026-10-24 — Web shell has no login

Owner: "get rid of the access token. I didn't ask for that." The bearer
token the web shell carried since `PLAN_WEB_SHELL.md` slice 3a is removed:
no `auth.rs`, no token file, no `?token=` link, no token gate in the UI.
The server is reachable by anyone who can reach its address; the owner's
private network / VPN is the boundary (as stated in the original request:
"vpn, connection out of scope"). Guidance updated: ADR-007 §4,
`docs/ARCHITECTURE.md`, `docs/USER_GUIDE.md` "Keeping it safe". A future
login, if ever wanted, is a new owner request.

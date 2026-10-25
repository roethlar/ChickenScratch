# Agent Decisions

Closed decisions are archived verbatim at `docs/history/decisions-archive.md`
per the lifecycle rule below; this file holds what is currently in force or
still open.

**Architectural decisions for this repo live in `docs/adr/` (ADR-001..ADR-004
and onward).** That tree is the canonical decision log for product and
architecture. Do not record competing product/architecture decisions here —
add a new ADR via `docs/adr/template.md` when the owner settles something
architectural, per `docs/INVARIANTS.md` I8. This file records governance
and layout-level decisions only (how agents work, not what the product does).

## Decision lifecycle

- **Open** — assessed but not yet acted on; lives in the Open Decisions queue
  with evidence, options, and a standing recommendation.
- **Active** — in force now.
- **Adopted YYYY-MM-DD** — an Open finding acted on; note where the rule
  landed, keep the finding as rationale until archived.
- **Superseded** — replaced by a later decision; name the replacement.

When an entry becomes purely historical (Adopted or Superseded, with the live
rule owned elsewhere), archive it verbatim to `docs/history/decisions-archive.md`
in the same change; never leave a stub, never summarize.

## Decisions

### 2026-07-16 - Per-commit owner approval removed

Status: Active

Decision:
Agents commit completed work without asking first. When a single concern is
implemented and the declared verification for the touched area passes, make
the commit — do not announce it and wait for a yes. The operative rule lives
in `.agents/repo-guidance.md` (Earned Practices, "Commit without asking").

Not changed by this decision:

- Pushing still follows `.agents/push-policy.md` (always ask).
- History rewrites (amend, rebase, squash, force-push, reordering or
  collapsing existing commits) remain forbidden without explicit owner
  approval per `AGENTS.md` Git Safety.
- One concern per branch/commit still applies (`.agents/repo-guidance.md`,
  Earned Practices).
- Scope approval is untouched: no code change without an approved plan
  (`AGENTS.md`), and which hardening concern to work on next is still the
  owner's call (`.agents/state.md`). This decision governs only the act of
  committing work that is already in scope and verified.

Reason:
Explicit owner directive, 2026-07-16. The prior working convention —
announce each commit and wait for an explicit yes — added a mechanical
round-trip per commit without adding safety: commits are local, history is
append-only under Git Safety, and pushing is separately gated. It also cut
against `docs/HUMAN-GATE.md`, which expects the owner to answer real
decision questions, not procedural ones.

### 2026-07-03 - AGENTS.md reconciled to the portable AgentGovernanceBootstrap template

Status: Active

Decision:
`AGENTS.md` is replaced whole with the current AgentGovernanceBootstrap
template (portable, generic — Prime Invariants, Universal Invariants,
Operator Requests, Verification, Git Safety). All ChickenScratch-specific
rules that used to live directly in `AGENTS.md` (owner communication style,
the product/architecture map, the work-request workflow, hard stops, the
DEVLOG rule) are carved into `.agents/repo-guidance.md`, which extends
`AGENTS.md` and points at `docs/PROJECT.md`, `docs/INVARIANTS.md`,
`docs/CURRENT_PHASE.md`, `docs/ARCHITECTURE.md`, `docs/AGENT-WORKFLOW.md`,
and `docs/HUMAN-GATE.md` rather than duplicating them. Harness command
wrappers (`catchup`, `handoff`, `drift`, `decision`, `plan`, `playbook`),
re-ground hooks, the `AGENTS.md` pre-edit tripwire, and the `CLAUDE.md` /
`GEMINI.md` shims were brought in line with the current toolkit templates at
the same time. `.gitignore`'s blanket `.claude/` rule was narrowed to
`.claude/settings.local.json` so the command wrappers and hooks are
committable.

Reason:
The 2026-06-10 decision (archived in `docs/history/decisions-archive.md`)
deliberately kept the bespoke June 7 protocol as `AGENTS.md` verbatim,
reasoning that it was "current, deliberate, and fitted to the owner." The
toolkit's template has since (2026-07-02) added a hard, explicit invariant
that did not exist as firmly on 2026-06-10: `AGENTS.md` must be portable
(true of any repo, unchanged) and is written only by a gated bootstrap/update
run as the template verbatim — never hand-composed or partially edited
outside that run. The repo's June 7 protocol duplicated content that already
had a canonical home (`docs/INVARIANTS.md`, `docs/PROJECT.md`,
`docs/AGENT-WORKFLOW.md`, `docs/HUMAN-GATE.md`), which the toolkit's
one-canonical-location and no-duplication invariants both flag directly.
Reconciling now, rather than leaving the June 10 rationale in place
unrevisited, keeps the repo aligned with the toolkit it opted into.

Supersedes:
2026-06-10 - Standard `.agents/` layer added on top of the June 7 protocol
(archived in `docs/history/decisions-archive.md`).

## Open Decisions (deferred - not yet adopted)

None recorded.

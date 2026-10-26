# Harvest Report: ChickenScratch, 2026-06-10

Governance rules from this repo that other repos would benefit from.

## Ideas

### Reinterpret a format inside the tool that parses it

- **Source:** the Scrivener-fidelity reviews of 2026-10-25/26
  (`.agents/review/findings/s4-8.md`, `s4-5.md`): a fix that rewrote
  footnote markup in Markdown text was reopened five times, and one that
  found link anchors in Markdown six times, each round on another construct
  the real parser (Pandoc) reads differently — fences in quotes, code spans
  across lines, wrappers in lists, destinations with spaces or unbalanced
  parentheses. Both closed the day they moved into Pandoc as Lua filters
  working on the parsed document.
- **The rule:** When a change must tell structure from content in a format
  that another tool already parses (Markdown via Pandoc, HTML via a DOM,
  RTF via its reader), do the work inside that tool's parse — a filter, an
  AST walk, a plugin — rather than re-deriving its rules on the text. A
  text-level approximation is acceptable only with a bounded, named set of
  inputs.
- **Why it generalizes:** every text-level reimplementation of another
  parser's grammar is a second parser that must agree with the first on
  every input; reviewers find the disagreements one at a time, forever.
- **Proposed home:** `AGENTS.template.md` (a design rule under Verification
  or Earned Practices).

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

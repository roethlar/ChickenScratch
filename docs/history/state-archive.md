# State Archive

Landed or superseded entries rotated verbatim out of `.agents/state.md` by
the `handoff` operator, newest batch first. History only — never load-bearing.

## Rotated 2026-07-09 (handoff after format lock)

- Governance refresh landed as of `2ab2579` (2026-07-03): `AGENTS.md`
  reconciled to the current AgentGovernanceBootstrap template (portable,
  generic); repo-specific rules carved into `.agents/repo-guidance.md`;
  harness command wrappers, hooks, and shims brought up to date. See
  `.agents/decisions.md` for why.
- **Build fixed on `master`** (2026-07-09, owner-approved work request):
  `f049198` removed the deleted `linux/` directory from the workspace
  members in root `Cargo.toml` (Cargo.lock regenerated); `d0f9cad` applied
  mechanical `cargo fmt` to three files whose drift the broken workspace had
  hidden. Full verification suite run green locally as of `d0f9cad`. Note:
  the "uncommitted local edit to Cargo.toml" recorded here on 2026-07-03 no
  longer existed by 2026-07-09 (clean worktree, empty stash); the fix was
  re-made from scratch.
- **Format lock (phase Step 2) shipped 2026-07-09** per owner-approved
  `docs/plans/PLAN_FORMAT_LOCK_ENGINE.md` (status: Shipped): unknown
  top-level key preservation (I5), legacy novelist-key lift, canonical
  `fields` order, `format_version` marker, full-fidelity round-trip tests,
  spec aligned. Summary in `DEVLOG.md` (2026-07-09 entry). The audit also
  found phase Step 3 (Tauri Inspector → `doc.fields`) was already in place
  before this work. G2's engine+spec+tests criteria look satisfied —
  closing/advancing the phase is the owner's call (`SET PHASE`).

# State Archive

Landed or superseded entries rotated verbatim out of `.agents/state.md` by
the `handoff` operator, newest batch first. History only — never load-bearing.

## Rotated 2026-07-28 (catchup hygiene sweep)

- **Fresh-fidelity operation boundary shipped** 2026-07-12 in `a0e7621`
  (`docs/plans/PLAN_FRESH_FIDELITY_BOUNDARY.md`; guard proofs and full
  verification in the DEVLOG). Cached `WriteToken`s can no longer authorize a
  later operation by themselves: each logical mutation receives one freshly
  probed, short-lived `WritePermit`. Public/default reads and CLI export are
  pure; benign folder repair is explicit and permit-backed; corrupt sidecars
  stay in place. Tauri invalidates cached authority on refusal and opens each
  permit inside its project lock; TUI and converter boundaries match.
- **Tree-replacement epoch guard plan is COMPLETE** — all four slices
  landed 2026-07-16 (`cd6afdd`, `db8095a`, `977095b`, `354fbc0`); DEVLOG
  top entry has the slice-by-slice detail, the codex review trail
  (`.agents/review/findings/s4-1..4.md`, all accepted <!-- lint: allow (range notation for s4-1.md through s4-4.md) -->), and the guard
  proofs. The two live bugs previously ranked here — the conflict
  dialog's unreachable Abort after a format-file conflict and its
  never-working Force exit — are **fixed** by slice 4's recovery
  authority (`RecoveryPermit`), merge-state query, persistent
  merge-in-progress UI, and attestation-bound `force_resolve_merge`.
- **Three owner-directed hardening fixes landed 2026-07-16** as
  per-concern commits on top of the epoch-guard work: `ae6bc01`
  canonicalizes `include_in_compile` YAML-1.1 boolean spellings at parse
  time (tolerant reader; unrecognized strings pass through verbatim);
  `0d3c027` verifies and self-heals the git repo on project open
  (`pre_repair_git` in RepairMode — missing repo recreated, unopenable
  repo left untouched and reported); `d6fa9b5` orders multi-file saves
  crash-safe by writing `project.yaml` last, so a torn save degrades to
  orphan content files (reconciled by self-heal) instead of a manifest
  referencing never-written documents. The first and third close the
  case-sensitive `include_in_compile` and non-transactional multi-file
  save findings parked in `PLAN_FORMAT_LOCK_ENGINE.md` — DEVLOG top
  entry has the guard-test detail.
- **`write_project` hierarchy guard shipped** 2026-07-16 (`6b4b4ac`):
  `write_project` now validates every `.md` document target against the
  hierarchy before writing anything and fails fast with `InvalidFormat`
  on an omitted document, instead of silently writing an orphan the
  reader would degrade. Entities under `characters/` and `locations/` <!-- lint: allow (user project folder structure paths) -->
  remain hierarchy-exempt by design; the guard matches on normalized
  spellings so variants cannot slip past. This closes the first ranked
  follow-up from the `d6fa9b5` list.
- **Per-commit owner approval removed** by owner directive 2026-07-16
  (`b043585`; decision in `.agents/decisions.md`, operative rule in
  `.agents/repo-guidance.md` Earned Practices): agents commit verified
  single-concern work without announcing and waiting. Push gating
  (`.agents/push-policy.md`), Git Safety append-only history, and scope
  approval are all unchanged.
- **`init_repo` gated behind `test-support`** 2026-07-16 (`7f85510`):
  the one unguarded write path in `core::git` is now `pub(crate)`,
  re-exported only through a `test_support` module behind a new
  `test-support` cargo feature. Dev-dependency re-declarations turn it
  on for test targets only (dev-deps are ignored by release builds, so
  feature unification never reaches them); all 10 out-of-crate call
  sites rerouted; a self-check test asserts `init_repo` stays
  `pub(crate)`. DEVLOG top entry has the detail.
- **Coherence is complete.** The owner confirmed on 2026-07-12 that completion
  had already been declared but not saved. Format lock, Tauri alignment,
  deprecation cleanup, and goals G1–G6 are recorded as completed in
  `docs/CURRENT_PHASE.md` and the shipped plans/DEVLOG entries it points to.
- **Write-guard shipped** (2026-07-11, `docs/plans/PLAN_TRUST_FOUNDATIONS.md`
  Slice 1; DEVLOG top entry has the slice-by-slice detail and guard
  proofs): side-effect-free fidelity probe + non-forgeable root-bound,
  epoch-stamped `WriteToken` established the original gate (now extended by
  the operation-scoped `WritePermit` above); Degraded
  projects open read-only in Tauri (banner + disabled affordances +
  skipped auto-saves), TUI (status + refused keys), and never receive a
  byte of writes — including the Statistics writing-history path.
  Same-day follow-up on owner ruling (no orphaned-file support pre-ship):
  the id-inheritance reader shim was reverted; `samples/Corn.chikn`
  regenerated with the current converter; binder-referenced binary assets
  (research PDFs etc.) are now fidelity-neutral while present, and the
  writer structurally refuses text writes into non-.md paths (DEVLOG top
  entry, follow-up paragraph).

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

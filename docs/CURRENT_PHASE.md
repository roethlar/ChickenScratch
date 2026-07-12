# Current Phase

**Status:** Active — advanced by the owner on 2026-07-12
**Started:** 2026-07-12

---

## Name: Engine hardening — protect writers' work

Strengthen the canonical Rust engine's safety and recovery guarantees before
expanding the reference app. This is the next unfinished priority in
[PROJECT.md](PROJECT.md): Governance/Coherence and format finalization were
completed in the preceding phase.

## Previous phase

**Coherence — single engine, one GUI** is complete. The owner confirmed that
completion had already been declared and this durable record was missing.
Its exit criteria landed through the format-lock and deprecation-cleanup
plans: agent guidance is established, the format is locked and tested, all
format I/O is centralized in `chickenscratch-core`, Tauri is the reference
GUI, deprecated native trees are removed, and CI/release guidance follows the
supported Rust applications.

## Goals (exit criteria)

- [ ] Re-verify and prioritize the remaining recorded engine-integrity risks.
- [ ] Close each approved high-risk item one concern at a time, with a guard
  test that fails when the protection is removed.
- [ ] Re-verify that all `.chikn` writes remain centralized in
  `chickenscratch-core`; no duplicate writer or app-level format I/O exists.
- [ ] Confirm project mutation paths preserve user data and cannot bypass the
  write guard, safe paths, the I6 atomic document-write guarantee, or
  dirty-worktree protections.
- [ ] Keep the declared validation suite and release-metadata check green for
  every landed slice.
- [ ] Record the owner's declaration when hardening is complete.

## Active work order

### Step 1 — Hardening audit

Re-verify the integrity findings already recorded under "Out of scope" in
[PLAN_FORMAT_LOCK_ENGINE.md](plans/PLAN_FORMAT_LOCK_ENGINE.md) against the
current engine. Inspect current save, revision, restore, and recovery paths
for equivalent risks. Rank findings by plausible data loss, not convenience.

This audit is read-only. Present one proposed safety slice at a time in plain
English; advancing the phase does not itself approve a code change.

### Step 2 — Approved safety slices

For each approved slice: write or identify the guarding test, prove it fails
without the protection, implement inside `chickenscratch-core` first, update
Tauri/TUI surfaces only where required, run the declared suite, and commit the
single concern.

### Step 3 — Close-out

Re-audit engine mutation entry points, resolve or explicitly park the
remaining findings with current evidence, then ask the owner whether to move
to the Tauri reference-app phase.

## Explicitly paused

- Vault/remote work: remote design is not settled and Slice 2 of
  [PLAN_TRUST_FOUNDATIONS.md](plans/PLAN_TRUST_FOUNDATIONS.md) is not approved.
- Windows Tauri, novelist feature expansion, and marketing remain later
  priorities unless the owner changes direction.
- WinUI, SwiftUI, and Qt feature parity remain superseded by ADR-004.

## How agents pick up a task

1. Read [`INVARIANTS.md`](INVARIANTS.md).
2. Read this file and do not start anything listed under **Explicitly paused**.
3. Read [`ARCHITECTURE.md`](ARCHITECTURE.md) for the engine boundary.
4. Follow [`AGENT-WORKFLOW.md`](AGENT-WORKFLOW.md).
5. Use [`templates/Plan-Template.md`](templates/Plan-Template.md) before code
   work that touches the engine, git writes, the format spec, or more than two
   files.

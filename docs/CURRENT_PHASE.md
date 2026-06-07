# Current Phase

**Status:** Active — change only when owner says `SET PHASE`  
**Started:** 2026-06-07

---

## Name: Coherence — single engine, one GUI

Stop drift. Align the repository with [INVARIANTS.md](INVARIANTS.md) and ADRs 001–004 before new features or marketing.

## Goals (exit criteria)

- [ ] **G1** — Agent docs complete; owner can assign plain-English tasks (`AGENTS.md`, this file, `HUMAN-GATE.md`)
- [ ] **G2** — Format finalization complete per schema rules below (engine + spec + tests)
- [ ] **G3** — No new code in deprecated `ChiknKit` / `ChickenScratch.Core` format paths
- [ ] **G4** — README and `docs/ROADMAP.md` reflect Tauri + engine (not “five frontends at parity”)
- [ ] **G5** — CI validation focuses on engine + Tauri + converter + TUI (deprecate WinUI workflow as release gate — optional sub-task)
- [ ] **G6** — Deprecated directories archived or clearly marked in-tree

## Active work order

### Step 1 — Governance (this commit)

Agent files, invariants, ADRs, architecture map.

### Step 2 — Format lock (engine)

From [PHASE_FORMAT_FINALIZATION.md](plans/PHASE_FORMAT_FINALIZATION.md), **engine scope only**:

- Genre-agnostic `fields` map on documents (no typed novelist fields in core)
- `CHIKN_FORMAT_SPEC.md` matches engine behavior
- Round-trip tests in `chickenscratch-core`

**Out of scope for Step 2:** Syncing Swift/C#/Qt UIs (superseded by ADR-004).

### Step 3 — Tauri alignment

- Inspector scene metadata → `doc.fields` per [UI_CONVENTIONS_NOVELIST.md](UI_CONVENTIONS_NOVELIST.md)
- Any engine feature exposed only through Tauri commands

### Step 4 — Deprecation cleanup

- README platform table
- Add `DEPRECATED.md` stubs in `macos/`, `windows/`, `linux/`
- Trim `cross_frontend` CI requirements (owner approval for deletion)

### Step 5 — Windows Tauri (after 1–4)

Add Windows bundle to CI — same app as macOS/Linux.

## Explicitly paused

- Novelist tier features ([plans/TIER1](plans/TIER1_novel_structure.md), etc.) until G2 complete
- Marketing materials
- WinUI / SwiftUI / Qt feature parity
- Multi-repo split (revisit after G2 + G3)

## How agents pick up a task

1. Read [`INVARIANTS.md`](INVARIANTS.md)
2. Read this file — if the task is in **Paused**, stop and tell the owner
3. Read [`ARCHITECTURE.md`](ARCHITECTURE.md) for where to edit
4. Follow [`AGENT-WORKFLOW.md`](AGENT-WORKFLOW.md)
5. Use [`templates/Plan-Template.md`](templates/Plan-Template.md) for non-trivial work
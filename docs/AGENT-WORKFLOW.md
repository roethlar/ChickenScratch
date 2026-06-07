# Agent workflow

For **work requests only**. If the owner asked a **question**, Rule 0 in `AGENTS.md` / Invariant I0 applies — answer in words, stop here.

---

## 0. Classify the message (mandatory first step)

See `AGENTS.md` Rule 0. Question → no file changes. Unclear → ask *"Explanation only, or change the repo?"* Default: explanation only.

---

## 1. Startup (work requests)

Read:

1. [`INVARIANTS.md`](INVARIANTS.md) — especially I0
2. [`CURRENT_PHASE.md`](CURRENT_PHASE.md)
3. [`ARCHITECTURE.md`](ARCHITECTURE.md)
4. [`CHIKN_FORMAT_SPEC.md`](CHIKN_FORMAT_SPEC.md) — if touching on-disk format
5. Relevant file in [`adr/`](adr/) — if touching architecture decisions

---

## 2. Accept or reject

| Situation | Action |
|-----------|--------|
| Violates an invariant | Stop. Explain simply. Ask yes/no. |
| Paused in `CURRENT_PHASE.md` | Stop. Explain what's in progress. Ask skip or wait. |
| WinUI/Swift/Qt parity | Stop. Offer Tauri path. |
| Format I/O outside `crates/core` | Stop. |
| Valid work request | Proceed. Plan if non-trivial. |

---

## 3. Plan (non-trivial)

Touches engine, format spec, git writes, or >2 files → use [`templates/Plan-Template.md`](templates/Plan-Template.md).

---

## 4. Implement

Engine (`crates/core`) → Tauri commands → UI (`ui/`). Match existing style. No drive-by refactors. Governance files only when owner changes direction in plain English.

---

## 5. Validate

```bash
cargo fmt --all -- --check
cargo clippy -p chickenscratch-core --all-targets -- -D warnings
cargo test -p chickenscratch-core --lib
cargo clippy -p chickenscratch --all-targets -- -D warnings
cargo test -p chickenscratch --bins
cd ui && npm run lint && npm run build
```

Full release: [`RELEASE.md`](../RELEASE.md).

---

## 6. DEVLOG (significant work only)

Append to [`DEVLOG.md`](../DEVLOG.md) when:

- Architecture, format on-disk, or governance changed
- A plan from `docs/plans/` shipped (per `docs/plans/README.md`)
- Non-obvious integrity/security fix future agents must know

Skip for routine features and small fixes. Newest entry at top.

---

## 7. Handoff (plain English)

1. What you did  
2. How to try it  
3. What was verified  
4. Open risks, if any  
5. Whether DEVLOG was updated (owner does not need to read it)
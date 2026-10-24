# Architecture Decision Records

Binding decisions for agents and contributors. **Status: Accepted** records are inviolable unless the owner says `AMEND ADR`.

| ADR | Title |
|-----|-------|
| [001](ADR-001-chikn-engine-single-source-of-truth.md) | Rust engine is single source of truth for `.chikn` I/O |
| [002](ADR-002-format-is-the-product.md) | `.chikn` format is the product; apps are reference implementations |
| [003](ADR-003-tauri-reference-ui.md) | Tauri + React is the reference GUI |
| [004](ADR-004-deprecated-native-engines.md) | Deprecated native frontends (macOS/Windows/Linux trees) |
| [005](ADR-005-binary-only-distribution.md) | Binary-only distribution; no in-repo source-package channels |
| [006](ADR-006-writing-modes.md) | The UI is organized around switchable writing modes (Proposed — draft notes, authorizes no work) |
| [007](ADR-007-web-shell-shared-services.md) | Shells share one command layer (`chickenscratch-services`); self-hosted web shell serves the same UI |

New ADRs: copy [`template.md`](template.md), set Status to **Proposed**, owner accepts with `AMEND ADR`.
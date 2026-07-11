# Plan: Deprecation cleanup — CI, release gate, and docs (G4–G6)

**Status:** Draft — awaiting owner approval (2026-07-10). Execution may not
begin until the status line records approval.

**Owner request (quote):**
> Owner approved on 2026-07-10: "yes" to drafting this plan for the G4–G6
> cleanup — trim CI and release scripts to engine + Tauri + converter + TUI,
> and sweep README, ARCHITECTURE, and ROADMAP. Execution requires approval of
> this plan (which also satisfies `CURRENT_PHASE.md` Step 4's "owner approval
> for deletion" of cross-frontend CI requirements).

**Phase check:** [x] Allowed by `CURRENT_PHASE.md` (Step 4 — Deprecation
cleanup; goals G4, G5, G6)  [x] Not paused

**Invariants touched:** none directly (no engine, format, or git-write
changes). Supports I3 and ADR-004. I9 governs verification below.

---

## [MODEL] Intent

CI on `master` goes green and every workflow, release script, and top-level
doc references only the supported pieces: engine (`crates/core`), Tauri app
(`src-tauri` + `ui`), converter (`crates/cli`), TUI (`crates/tui`). No
references to the deleted `macos/`, `windows/`, `linux/` trees remain outside
historical records (`DEVLOG.md`, `REVIEW.md`, `docs/adr/`, `docs/history/`,
`docs/plans/`, `.review/`).

## [MODEL] Context (verified 2026-07-10 at `3a486e3`)

- `macos/`, `windows/`, `linux/` are already deleted from the tree. G6 is
  therefore satisfied by deletion + ADR-004; no in-tree stubs are needed.
- GitHub **Validation** workflow is red on every recent `master` push (Swift
  package lock step resolves the deleted `macos/`; NuGet step needs deleted
  `windows/`; release-metadata step checks deleted `linux/Cargo.toml`;
  cross-frontend step greps `writer-toolchains-ran:2/2`, impossible with the
  Swift/C# harnesses gone). **Tauri Bundles** is green — leave it alone.
- `.github/workflows/macos-release.yml` builds the **Tauri** app (signed
  macOS release). It does not touch deleted trees. Keep unchanged.
- `crates/core/tests/cross_frontend_round_trip.rs` has fixture-based reader
  tests plus one env-gated verifier that self-skips without
  `CHIKN_CROSS_FRONTEND_VERIFY`. Keep the file unchanged.
- Root `Cargo.toml` workspace members are already correct
  (`core`, `cli`, `tui`, `src-tauri`).

## [MODEL] Approach

One slice per commit, committed before starting the next, full verification
before each commit claim. Order chosen so CI-facing fixes land first.

### Slice A — `validation.yml` + rust-only harness (G5)

1. `.github/workflows/validation.yml`: delete the `Setup .NET`,
   `Swift package lock`, and `Windows NuGet lock` steps. Keep everything
   else, including the `rust-process-windows` job (engine code, still
   supported).
2. `crates/core/tests/cross_frontend/run.sh`: remove the Swift and dotnet
   writer stanzas, `SWIFT_WRITER_RAN` / `CSHARP_WRITER_RAN`,
   `skip_toolchain`, `join_by_comma`, and the `writer-toolchains-ran` /
   `skipped-toolchains` manifest lines. Keep: pandoc shim, converter build +
   run against `samples/Corn.scriv`, `verify_rust_reader` with
   `FAIL_ON_REPAIR` and hierarchy-docs dump. End with `log "result: ok"`
   unconditionally (the script is `set -euo pipefail`; reaching the end means
   the Rust path passed).
3. In `validation.yml`'s cross-frontend step, replace the
   `grep -q 'writer-toolchains-ran:2/2' …` line with
   `grep -q 'result: ok' "$CHIKN_CROSS_FRONTEND_WORKDIR/manifest.txt"`.
4. Update the header comment of `run.sh` (and the doc-comment of
   `cross_frontend_round_trip.rs` only if it becomes inaccurate) to say the
   harness drives the Rust converter → Rust reader path.

### Slice B — delete dead Windows CI (G5)

1. Confirm with a repo-wide grep that `windows.yml` and
   `scripts/check-nuget-package-versions.ps1` are referenced only by each
   other, `validation.yml` (step already removed in Slice A), and historical
   docs.
2. Delete `.github/workflows/windows.yml` and
   `scripts/check-nuget-package-versions.ps1`.

### Slice C — release gate (G5)

1. `scripts/check-release-metadata.sh`: remove `linux/Cargo.toml` from
   `version_files`.
2. `RELEASE.md`: remove `linux/Cargo.toml` from the files-to-update list;
   remove the `swift package resolve` / `git diff macos/Package.resolved` and
   `windows … dotnet restore` lines from §2; replace the §3 Windows (WinUI)
   build block with a one-line note that Windows ships later as a Tauri
   bundle (`CURRENT_PHASE.md` Step 5). Keep the cross-frontend `run.sh` line
   (now rust-only).

### Slice D — `README.md` (G4)

1. Replace the five-row Platforms table with the supported set: Tauri
   (macOS/Linux, 1.0 target), TUI, converter CLI; note Windows arrives as a
   Tauri bundle (phase Step 5).
2. Delete the `Windows (WinUI 3)`, `macOS (SwiftUI, Liquid Glass)`, and
   `Linux (Qt6, cxx-qt)` build sections.
3. Architecture tree: drop `windows/`, `macos/`, `linux/` entries.
4. Dependencies: drop Windows App SDK, cxx-qt, LibGit2Sharp.
5. Do **not** touch the `**Status:**` line beyond removing the deprecated-
   trees sentence if desired — `scripts/check-release-metadata.sh` greps
   README for `v<version>` and alpha/status wording; keep `v1.0.0` present
   and run the script before committing.

### Slice E — `docs/ARCHITECTURE.md` (G4)

1. Deprecated-paths section: the three trees are deleted, not
   present-but-deprecated — say so, point at ADR-004 and git history.
2. Fix the "Cargo workspace" line: members are `core`, `cli`, `tui`,
   `src-tauri` (no `linux`).
3. Update the layer-diagram `DEPRECATED` block and the cross-frontend tests
   note (rust-only regression harness).
4. Bump the `Last verified` line to the current commit + date.

### Slice F — `docs/ROADMAP.md` (G4)

1. Replace the "Five frontends, one canonical storage format" block with the
   supported set (engine + Tauri + TUI + converter + canonical Markdown
   storage); mark the SwiftUI/Qt6/WinUI bullets historical per ADR-004 or
   delete them.
2. Delete the "Not every frontend is at feature parity…" paragraph.
3. Mark the v1.1 "Frontend parity (SwiftUI + Qt6 + WinUI)" subsection as
   superseded by ADR-004 (it is unshipped work that will not happen).
4. Fix the intro line under Current State if it still implies five apps.

### Slice G — close-out

1. `DEVLOG.md`: one entry, newest-at-top (CI/release/docs now match ADR-004
   reality; Validation red → green).
2. `.agents/state.md`: remove the resolved Known-drift entries; update Now /
   Next. G4/G5/G6 checkboxes in `docs/CURRENT_PHASE.md` and phase advance
   remain the owner's call (`SET PHASE`) — report, don't edit.
3. Ask the owner before pushing (`.agents/push-policy.md`); after push,
   confirm the GitHub Validation run on the new head is green and record it.

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `.github/workflows/validation.yml` | Remove .NET/Swift/NuGet steps; new harness grep |
| `crates/core/tests/cross_frontend/run.sh` | Rust-only harness; `result: ok` marker |
| `.github/workflows/windows.yml` | Delete |
| `scripts/check-nuget-package-versions.ps1` | Delete |
| `scripts/check-release-metadata.sh` | Drop `linux/Cargo.toml` |
| `RELEASE.md` | Drop deleted-tree steps; Windows-via-Tauri note |
| `README.md` | Supported-platforms table, build sections, tree, deps |
| `docs/ARCHITECTURE.md` | Deleted-trees wording, workspace line, last-verified |
| `docs/ROADMAP.md` | Header block, parity paragraph/section |
| `DEVLOG.md`, `.agents/state.md` | Close-out (Slice G) |

Not touched: `.github/workflows/macos-release.yml`, `tauri-bundles.yml`,
`docs/CURRENT_PHASE.md`, `samples/Corn.scriv`, engine/app source, anything
under `docs/adr/`, `docs/history/`, `.review/`.

## [MODEL] Tests

Before each commit (declared suite, `.agents/repo-guidance.md` Verification):

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy -p chickenscratch-core --all-targets -- -D warnings`
- [ ] `cargo test -p chickenscratch-core --lib`
- [ ] `cargo clippy -p chickenscratch --all-targets -- -D warnings`
- [ ] `cargo test -p chickenscratch --bins`
- [ ] `cd ui && npm run lint && npm run build`

Slice-specific:

- [ ] Slices A/C/D: `scripts/check-release-metadata.sh` passes locally
- [ ] Slice A: `bash crates/core/tests/cross_frontend/run.sh` passes locally
      and its manifest contains `result: ok`
- [ ] After Slice F: repo-wide grep for `macos/`, `windows/`, `linux/`
      shows hits only in historical files (list in the handoff)
- [ ] After push (owner-approved): GitHub **Validation** run on the new head
      is green — this is the G5 exit proof

## [MODEL] Owner verification (plain English)

Open the project's GitHub page → Actions → the "Validation" check on the
newest master commit is green (it has been red since June). The README front
page now lists only the apps that actually exist.

## [YOU] Decisions needed

- None beyond approval of this plan. Narrowing `run.sh` to rust-only (rather
  than deleting it) and deleting `windows.yml` +
  `check-nuget-package-versions.ps1` are within ADR-004's allowed
  maintenance; plan approval covers Step 4's deletion gate.

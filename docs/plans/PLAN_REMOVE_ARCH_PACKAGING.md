# Plan: Remove Arch source-package channel (ADR-005)

**Status:** Executed 2026-07-10 (approved same day — owner's distribution
statement quoted in `docs/adr/ADR-005-binary-only-distribution.md` selected
removal of the Arch packaging, "option 1" of the choices presented in chat).
All tests in the plan passed; awaiting push + GitHub confirmation of a fully
green Validation run.

**Owner request (quote):** see ADR-005 Context.

**Phase check:** [x] Allowed by `CURRENT_PHASE.md` (coherence/cleanup; not a
paused item)  [x] Not paused

**Invariants touched:** none (no engine/format/git-write changes). ADR-005
records the decision; the R-13 review gate is retired with its subject.

---

## [MODEL] Intent

No trace of the Arch source-package channel remains outside history:
`pkg/` deleted, release gate validates version metadata only, runbook and
README describe binary distribution. CI's "Release metadata" step passes on
a moving master, making the Validation workflow fully green.

## [MODEL] Approach (one commit per slice)

1. **Record** — ADR-005 + this plan (docs commit).
2. **Remove** — delete `pkg/` (PKGBUILD + desktop file, used only by the
   recipe) and `scripts/create-release-source.sh` (referenced only by the
   release gate and runbook). In `scripts/check-release-metadata.sh` remove:
   `pkgbuild_value()`, `arch_expected`, all PKGBUILD field checks, both
   `.gitattributes` export-ignore checks, the `sha256sums` checks, and the
   archive-creation/comparison block; keep version-file checks, tauri.conf
   check, README checks, `Cargo.lock` freshness, and the `--require-tag`
   tag-existence check; update the usage text. Remove the
   `pkg/arch/PKGBUILD export-ignore` line from `.gitattributes` (keep the
   `REVIEW.md` line — harmless). Remove the Arch bullets/tree entry from
   `README.md`; remove RELEASE.md §4 (Arch source) and the archive/makepkg
   steps from §5 (renumber); annotate the checked PKGBUILD item in
   `TODO.md` with "(removed, ADR-005)".
3. **Close out** — DEVLOG entry; `.agents/state.md` Blockers entry resolved;
   `.agents/repo-map.json` REVIEW.md note updated; plan status → Executed.

## [MODEL] Tests

- [ ] `bash -n scripts/check-release-metadata.sh`
- [ ] `scripts/check-release-metadata.sh` (default mode) exits 0
- [ ] `scripts/check-release-metadata.sh --release 1.0.0` exits 0;
      `--release 1.0.0 --require-tag` fails only on the missing tag
- [ ] repo grep for `pkg/arch|PKGBUILD|makepkg|create-release-source` hits
      only history (DEVLOG, REVIEW, .review, docs/history, docs/plans, ADR)
- [ ] Declared suite green at close-out (no Rust/UI files touched)
- [ ] After owner-approved push: Validation workflow fully green on GitHub

## [MODEL] Owner verification (plain English)

GitHub → Actions: the "Validation" check on the newest change is green,
every step, for the first time since May.

## [YOU] Decisions needed

- None; ADR-005 records the owner's decision.

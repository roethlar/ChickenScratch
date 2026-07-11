# ADR-005: Binary-only distribution; no in-repo source-package channels

**Status:** Accepted  
**Date:** 2026-07-10

## Context

`pkg/arch/PKGBUILD` (an Arch Linux source-package recipe) was added as
release preparation for a v1.0.0 that was never tagged or published. Its
pinned source-archive checksum, deliberately compared against a HEAD archive
on every CI run (review finding R-13, `REVIEW.md`), assumed master sat frozen
at a release point; on a moving branch the comparison can never pass, keeping
the Validation workflow's "Release metadata" step permanently red (DEVLOG
2026-07-10).

Owner distribution intent, stated 2026-07-10:

> this will ship, if it ever ships, as a binary. authors aren't going to
> know from makepkg or yay. they're going to install the app from flathub
> or app store or a future website as an installer.

## Decision

ChickenScratch ships to writers as **built binaries** — Flathub, app stores,
website installers (see `docs/ROADMAP.md` § Platform Packaging). Those
channels perform their own download-integrity verification.

No source-package channel (Arch PKGBUILD / AUR or similar) is maintained in
this repository. `pkg/` and its supporting machinery are removed:
`scripts/create-release-source.sh`, the PKGBUILD/source-archive checks in
`scripts/check-release-metadata.sh`, and the related `.gitattributes`
export-ignore requirement.

## Consequences

- `scripts/check-release-metadata.sh` validates version metadata and README
  status only; the R-13 archive-vs-pin gate is retired **with its subject**,
  not bypassed — there is no pin left to protect.
- CI's "Release metadata" step can pass on a moving master.
- If a distro source package is ever wanted post-release, it can be rebuilt
  then (afternoon-scale; see git history of `pkg/arch/`), or left to distro
  maintainers downstream, which is the usual arrangement.
- `REVIEW.md` no longer needs to stay export-ignored for pin stability
  (`.agents/repo-map.json` note updated).

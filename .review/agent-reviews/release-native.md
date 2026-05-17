# Release / Native Build Review

## Scope

Reviewed current `master` for release, CI, packaging, and native build readiness only:

- `.github/workflows/*`
- `RELEASE.md`, `README.md`, `docs/ROADMAP.md`
- `scripts/check-release-metadata.sh`, `scripts/create-release-source.sh`
- `pkg/arch/PKGBUILD`
- `src-tauri/tauri*.conf.json`, `src-tauri/Cargo.toml`, Tauri bundle/install paths
- Cargo/npm/Swift/NuGet lock and restore behavior
- Native build scaffolding for macOS Swift and Windows WinUI

R-1..R-13 were treated as verified. R-14/R-15/R-16 were treated as already-open and are not repeated here.

## Commands run

- `sed -n '1,220p' .review/README.md`
- `sed -n '1,260p' REVIEW.md`
- `rg -n "R-1|R-14|R-15|R-16|release|native|packag|Tauri|tag|checksum|CI|GitHub|Cargo.lock|package-lock" REVIEW.md`
- `sed -n '508,560p' REVIEW.md`
- `find . -maxdepth 3 -type f \( -path './.github/*' -o -path './scripts/*' -o -name '*RELEASE*' -o -name 'Cargo.toml' -o -name 'Cargo.lock' -o -name 'package.json' -o -name 'package-lock.json' -o -path './pkg/*' -o -path './src-tauri/*' \) | sort`
- `nl -ba .github/workflows/validation.yml`
- `nl -ba .github/workflows/tauri-bundles.yml`
- `nl -ba .github/workflows/windows.yml`
- `nl -ba RELEASE.md`
- `nl -ba src-tauri/tauri.conf.json`
- `nl -ba src-tauri/tauri.linux.conf.json`
- `nl -ba scripts/check-release-metadata.sh`
- `nl -ba scripts/create-release-source.sh`
- `nl -ba pkg/arch/PKGBUILD`
- `nl -ba .gitattributes`
- `scripts/check-release-metadata.sh`
- `scripts/check-release-metadata.sh --release 1.0.0 --require-tag`
- `cargo metadata --locked --no-deps --format-version 1`
- `rg -n "cargo (clippy|test|build|tauri|metadata)|--locked|npm ci|npm install|swift run|Package.resolved|macos/Package.resolved" .github RELEASE.md README.md pkg/arch/PKGBUILD .gitignore macos/Package.swift crates/core/tests/cross_frontend/run.sh scripts -S`
- `nl -ba macos/Package.swift`
- `find macos -maxdepth 2 -name 'Package.resolved' -print`
- `git check-ignore -v macos/Package.resolved`
- `nl -ba windows/ChickenScratch.App/ChickenScratch.App.csproj`
- `find windows -name 'packages.lock.json' -print`
- `tmpdir=$(mktemp -d); scripts/create-release-source.sh 1.0.0 HEAD "$tmpdir" >/tmp/chickenscratch-release-source.out; tar -tzf "$tmpdir/chickenscratch-1.0.0.tar.gz" | rg '^(chickenscratch-1\.0\.0/(README\.md|docs/ROADMAP\.md|LICENSE))$'; rm -rf "$tmpdir"`

## Findings

### R-17 candidate: Release checksum gate is path-filtered away for tarball-included files

- **Severity**: HIGH for release readiness.
- **Duplicate/new status**: New. Related to R-13's checksum gate, but not the same bug: R-13 added the comparison, this gap lets changes bypass the workflow that runs it.
- **Evidence**:
  - `scripts/check-release-metadata.sh:145-149` compares the generated source archive SHA to `pkg/arch/PKGBUILD`.
  - `scripts/create-release-source.sh:38-40` archives the repository tree, with export-ignore rules only excluding `.review`, `REVIEW.md`, and `pkg/arch/PKGBUILD` via `.gitattributes:13-18`.
  - `README.md`, `LICENSE`, and `docs/ROADMAP.md` are included in the release tarball; verified with `scripts/create-release-source.sh 1.0.0 HEAD "$tmpdir"` and `tar -tzf`.
  - `.github/workflows/validation.yml:5-19` and `:21-35` do not trigger on `README.md`, `LICENSE`, `docs/**`, `.github/workflows/tauri-bundles.yml`, or `.github/workflows/windows.yml`, even though those files are tarball-included and can change the checksum. `README.md` is also directly checked by `scripts/check-release-metadata.sh:96-103`.
- **Impact**: A docs/workflow/license/status change can merge without running the release metadata check, leaving the pinned Arch checksum stale until a later unrelated change happens to run validation. This weakens the R-13 gate precisely for "release-source included but not code" changes.
- **Suggested REVIEW id if new**: R-17.
- **Tests/repro idea**: In a throwaway branch, edit `docs/ROADMAP.md` or `README.md` only. GitHub's current path filter will not schedule `validation.yml`, but `scripts/check-release-metadata.sh` would fail locally because the generated archive SHA no longer matches `pkg/arch/PKGBUILD`. Fix by removing the path filters for validation, or by making the release-metadata job run for every PR/push that can affect the source archive.

### R-18 candidate: Rust release and package builds do not enforce `Cargo.lock`

- **Severity**: HIGH for clean checkout reproducibility.
- **Duplicate/new status**: New. R-11 used `cargo metadata --locked` as a one-off verification, but no current CI/runbook/package gate enforces it.
- **Evidence**:
  - `.github/workflows/validation.yml:72-76` runs `cargo clippy` and `cargo test` without `--locked`.
  - `.github/workflows/tauri-bundles.yml:46-49` and `:103-104` run `cargo tauri build` without any locked Cargo preflight or locked Cargo build mode.
  - `RELEASE.md:33-35`, `:49`, and `:59` document release validation and Tauri bundle commands without `--locked`.
  - `pkg/arch/PKGBUILD:20` correctly uses `npm ci`, but `pkg/arch/PKGBUILD:25` uses `cargo build --release -p chickenscratch` without `--locked`.
  - `cargo metadata --locked --no-deps --format-version 1` currently passes, so the lockfile is good today; the issue is that the gate is not wired.
- **Impact**: A future manifest change can be validated and packaged with a locally rewritten Cargo lockfile that is not actually committed to the release source. That makes clean-checkout and Arch builds depend on live registry resolution rather than the checked-in `Cargo.lock`.
- **Suggested REVIEW id if new**: R-18.
- **Tests/repro idea**: In a temp clone, change a Cargo dependency requirement and update the Arch checksum while leaving `Cargo.lock` stale. The current metadata script can be made to pass after the checksum update, but `cargo metadata --locked` will fail. Add `cargo metadata --locked` to `scripts/check-release-metadata.sh`, run Cargo CI/package commands in locked mode where supported, and add the same locked preflight before Tauri bundle builds.

### R-19 candidate: Native Swift and Windows dependency resolution is not locked

- **Severity**: HIGH for native build readiness, especially because validation uses both native writer toolchains as release confidence signals.
- **Duplicate/new status**: New. Not R-14/R-15/R-16; this is dependency reproducibility, not app-level writer safety.
- **Evidence**:
  - `macos/Package.swift:13-15` declares Yams as `.package(url: "https://github.com/jpsim/Yams", from: "5.1.3")`.
  - `.gitignore:25` ignores `macos/Package.resolved`, and `git ls-files` shows only `Cargo.lock` and `ui/package-lock.json` are tracked lockfiles. A local ignored `macos/Package.resolved` currently pins Yams `5.4.0`, but a clean checkout will not have it.
  - `crates/core/tests/cross_frontend/run.sh:153-157` runs `swift run --package-path macos ChiknKitCrossFrontendHarness`, and `.github/workflows/validation.yml:86-92` makes that harness part of CI.
  - `windows/ChickenScratch.App/ChickenScratch.App.csproj:19` uses floating `Microsoft.WindowsAppSDK` version `1.8.*`.
  - No `windows/**/packages.lock.json` exists, and `.github/workflows/windows.yml:25-31` restores/builds without NuGet locked mode.
- **Impact**: The native harnesses and Windows app can change behavior or fail based on newly published SwiftPM/NuGet packages, without a repository change. That makes CI failures and native release builds harder to reproduce, and it undercuts the cross-frontend validation signal.
- **Suggested REVIEW id if new**: R-19.
- **Tests/repro idea**: Commit `macos/Package.resolved`, stop ignoring it, pin Windows package versions exactly, enable NuGet lock files, and run CI restores with locked mode. Add a check that `git ls-files` contains `macos/Package.resolved` and `windows/**/packages.lock.json`, and that no `PackageReference` uses a wildcard version.

### R-20 candidate: macOS DMG path still has no signing/notarization release gate

- **Severity**: HIGH for public macOS distribution; MEDIUM if the CI artifact is explicitly internal-only.
- **Duplicate/new status**: New as an open REVIEW item. The gap is mentioned in `docs/ROADMAP.md`, and R-7's verdict called CI DMGs unsigned, but no open release blocker tracks it.
- **Evidence**:
  - `src-tauri/tauri.conf.json:26-36` enables app/dmg bundling and icons only; there is no macOS signing/notarization configuration in the release Tauri config.
  - `.github/workflows/tauri-bundles.yml:46-62` builds and uploads the `.app`/`.dmg` without Apple certificate, signing identity, hardened runtime, entitlements, or notarytool credentials/steps.
  - `RELEASE.md:46-55` documents `CI=true cargo tauri build --bundles app,dmg` and artifact existence checks, but no signing or notarization verification.
  - `docs/ROADMAP.md:151-155` still lists "macOS Code Signing" as future work.
- **Impact**: The macOS release job verifies that a DMG can be produced, but not that it is distributable to normal macOS users. A public beta/release cut from this path will produce unsigned/unnotarized artifacts unless signing is handled manually outside the documented gate.
- **Suggested REVIEW id if new**: R-20.
- **Tests/repro idea**: Add a release-only signing/notarization path using GitHub Actions secrets and verify the result with `codesign --verify --deep --strict`, `spctl --assess`, and `xcrun stapler validate` on the DMG/app. If unsigned CI artifacts are intentionally kept, split the workflow into "packaging smoke" and "distributable release" so the release runbook cannot confuse the two.

# ChickenScratch Release Runbook

This runbook is the release gate for a public desktop build. It records the commands that must pass and the metadata that must be updated before cutting a tag.

## 1. Choose The Release Version

Use one canonical version string everywhere.

Current release value:

- `1.0.0`

Files that must be updated for each release:

- `README.md`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`
- `crates/core/Cargo.toml`
- `crates/cli/Cargo.toml`
- `crates/tui/Cargo.toml`

Do not tag until `scripts/check-release-metadata.sh --release <version>` passes and any remaining `rg '0\.1\.0-alpha|Alpha|alpha'` matches have been reviewed as intentional.

## 2. Required Local Validation

Run from the repository root:

```bash
scripts/check-release-metadata.sh
cargo fmt --all -- --check
cargo metadata --locked --format-version 1 >/dev/null
cargo clippy --locked -p chickenscratch-core -p chickenscratch -p chickenscratch-tui -p chikn-converter --all-targets -- -D warnings
cargo test --locked -p chickenscratch-core -p chickenscratch -p chickenscratch-tui -p chikn-converter --lib --bins --tests
cd ui && npm ci && npm run lint && npm run build && cd ..
```

Run format-harness validation (Rust converter → Rust reader):

```bash
crates/core/tests/cross_frontend/run.sh
```

## 3. Desktop Artifact Builds

macOS unsigned smoke build:

```bash
CI=true cargo tauri build --bundles app,dmg -- --locked
test -d target/release/bundle/macos/ChickenScratch.app
test -n "$(find target/release/bundle/dmg -name 'ChickenScratch_*.dmg' -print -quit)"
```

`CI=true` makes Tauri skip Finder AppleScript DMG layout work, matching the GitHub Actions path and headless release automation.

The unsigned smoke artifact uploaded by `Tauri Bundles` is not a public release artifact. For public macOS distribution, the protected `macOS Signed Release` workflow runs automatically on a `v*` tag push; it also stays available as `workflow_dispatch` for smoke-testing the signing path without cutting a tag. The workflow must run in the `release-macos` environment with these secrets:

- `APPLE_CERTIFICATE`: base64-encoded Developer ID Application `.p12`
- `APPLE_CERTIFICATE_PASSWORD`: password for the `.p12`
- `APPLE_SIGNING_IDENTITY`: exact Developer ID Application identity
- `APPLE_API_ISSUER`: App Store Connect API issuer ID
- `APPLE_API_KEY`: App Store Connect API key ID
- `APPLE_API_KEY_P8`: App Store Connect private key contents
- `KEYCHAIN_PASSWORD`: temporary CI keychain password

The release workflow imports the Developer ID certificate, writes the App Store Connect API key, builds with `cargo tauri build --bundles app,dmg -- --locked`, explicitly submits and staples the DMG with `xcrun notarytool`, and fails unless all signing, notarization, and stapling checks pass:

```bash
APP=target/release/bundle/macos/ChickenScratch.app
DMG="$(find target/release/bundle/dmg -name 'ChickenScratch_*.dmg' -print -quit)"

codesign --verify --deep --strict --verbose=2 "$APP"
codesign -dv --verbose=4 "$APP" 2>&1 | grep 'Authority=Developer ID Application'
xcrun notarytool submit "$DMG" --key "$APPLE_API_KEY_PATH" --key-id "$APPLE_API_KEY" --issuer "$APPLE_API_ISSUER" --wait
xcrun stapler staple "$DMG"
spctl --assess --type execute --verbose=4 "$APP"
xcrun stapler validate "$APP"
xcrun stapler validate "$DMG"
spctl --assess --type open --context context:primary-signature --verbose=4 "$DMG"
```

Do not attach the unsigned smoke artifact to a public release. The signed, notarized, and stapled DMG from `macOS Signed Release` is the only public macOS artifact; the `.app` bundle is a directory, so it stays a CI artifact for inspection and is never a release asset.

Linux:

```bash
cargo tauri build --bundles appimage -- --locked
test -n "$(find target/release/bundle/appimage -name '*.AppImage' -print -quit)"
```

Windows: there is no unsigned smoke build in CI. For public Windows distribution, the protected `Windows Signed Release` workflow runs automatically on a `v*` tag push; like the macOS one it also stays available as `workflow_dispatch`. The workflow must run in the `release-windows` environment with these secrets:

- `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`: Azure app-registration credentials, read automatically by the `TrustedSigning` module's `EnvironmentCredential`
- `AZURE_SIGNING_ENDPOINT`: Azure Trusted Signing endpoint URL
- `AZURE_SIGNING_ACCOUNT`: Azure Trusted Signing account name
- `AZURE_CERT_PROFILE`: certificate profile name

The workflow installs the `TrustedSigning` PowerShell module, builds with `cargo tauri build --bundles msi,nsis -- --locked`, signs both installers, and fails unless every artifact reports a valid Authenticode signature:

```powershell
Invoke-TrustedSigning -Endpoint $env:AZURE_SIGNING_ENDPOINT -CodeSigningAccountName $env:AZURE_SIGNING_ACCOUNT -CertificateProfileName $env:AZURE_CERT_PROFILE -TimestampRfc3161 http://timestamp.acs.microsoft.com -TimestampDigest SHA256 -FileDigest SHA256 -Files <installer>
(Get-AuthenticodeSignature -FilePath <installer>).Status -eq 'Valid'
```

`--bundles msi,nsis` overrides `bundle.targets` in `src-tauri/tauri.conf.json`, which stays pinned to the macOS set; the Linux AppImage job overrides it the same way. The deprecated WinUI build was removed with the `windows/` tree (ADR-004).

Linux and Windows artifact builds must be validated on their native hosts or via CI.

## 4. Cut The Tag

After validation passes and final release metadata is committed, create the tag:

```bash
git tag -a v<version> -m "ChickenScratch <version>"
git push origin master v<version>
```

Confirm the metadata and tag agree:

```bash
scripts/check-release-metadata.sh --release "$version" --require-tag
```

Pushing the tag starts `macOS Signed Release` and `Windows Signed Release`. Each builds, signs, and verifies its own artifacts, then attaches them to a draft GitHub release named `ChickenScratch v<version>` through `softprops/action-gh-release@v2`:

- macOS contributes the signed, notarized, stapled `.dmg`.
- Windows contributes the signed `.msi` and `-setup.exe`.

Both runs write to the same tag's release. They share one concurrency group, so the second queues instead of racing the first to create it, and `action-gh-release` upserts per file: it replaces only an asset of the same name and never clears assets it did not upload, so neither platform can drop the other's installers. Both pass `draft: true`, which is what keeps the release unpublished after an upload.

The release is deliberately left as a draft. Review the attached assets, write the release notes by hand, and publish it manually; this repo generates no changelog. Both attach steps set `fail_on_unmatched_files: true` and are guarded on `refs/tags/`, so a run fails instead of publishing a partial release, and a `workflow_dispatch` run from a branch attaches nothing.

Distribution note (ADR-005): writers install built binaries (Flathub, app
stores, website installers); no source-package channel is maintained in-repo.

## 5. Post-build Smoke Checks

Open a sample project in the released app and verify:

- Create, edit, rename, move, and delete a document.
- Git history shows a new save revision.
- Restore a previous document revision.
- Compile/export includes the latest editor contents.
- App close waits for pending saves.
- Settings secrets are not written in plaintext.
- Scrivener import rejects hostile path traversal fixtures.

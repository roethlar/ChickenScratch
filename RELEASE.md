# ChickenScratch Release Runbook

This runbook is the release gate for a public desktop build. It records the commands that must pass and the metadata that must be updated before cutting a tag.

## 1. Choose The Release Version

Use one canonical version string everywhere.

Current pre-release value:

- `0.1.0-alpha`

Files that must be updated for a 1.0 release:

- `README.md`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`
- `crates/core/Cargo.toml`
- `crates/cli/Cargo.toml`
- `crates/tui/Cargo.toml`
- `linux/Cargo.toml`
- `pkg/arch/PKGBUILD`

Do not tag until `rg '0\.1\.0-alpha|Alpha|alpha'` has been reviewed and any remaining alpha text is intentional.

## 2. Required Local Validation

Run from the repository root:

```bash
cargo fmt --all -- --check
cargo clippy -p chickenscratch-core -p chickenscratch -p chickenscratch-tui -p chikn-converter --all-targets -- -D warnings
cargo test -p chickenscratch-core -p chickenscratch -p chickenscratch-tui -p chikn-converter --lib --bins --tests
cd ui && npm ci && npm run lint && npm run build && cd ..
```

Run cross-frontend format validation:

```bash
crates/core/tests/cross_frontend/run.sh
```

## 3. Desktop Artifact Builds

macOS:

```bash
cargo tauri build --bundles app,dmg
test -d target/release/bundle/macos/ChickenScratch.app
test -n "$(find target/release/bundle/dmg -name 'ChickenScratch_*.dmg' -print -quit)"
```

Linux:

```bash
cargo tauri build --bundles appimage
test -n "$(find target/release/bundle/appimage -name '*.AppImage' -print -quit)"
```

Windows:

```powershell
cd windows
dotnet restore ChickenScratch.slnx
dotnet build ChickenScratch.slnx /p:Configuration=Release --no-restore
dotnet build ChickenScratch.App/ChickenScratch.App.csproj /p:Platform=x64 /p:Configuration=Release
```

Linux and Windows artifact builds must be validated on their native hosts or via CI.

## 4. Cut The Tag

After validation passes and version metadata is updated:

```bash
git tag -a v<version> -m "ChickenScratch <version>"
git push origin master v<version>
```

Example for a 1.0 release:

```bash
git tag -a v1.0.0 -m "ChickenScratch 1.0.0"
git push origin master v1.0.0
```

## 5. Update Arch Package Source

The Arch package must point at a real tagged release archive and use a pinned checksum.

After the tag exists:

```bash
version=1.0.0
url="https://github.com/roethlar/ChickenScratch/archive/refs/tags/v${version}.tar.gz"
curl -L "$url" -o "/tmp/chickenscratch-${version}.tar.gz"
sha256sum "/tmp/chickenscratch-${version}.tar.gz"
```

Then update `pkg/arch/PKGBUILD`:

- `pkgver` must match the release version in Arch-safe form.
- `url` must point at the public upstream repository.
- `source` must point at the tagged release archive.
- `sha256sums` must contain the computed checksum, not `SKIP`.

Validate on Arch Linux:

```bash
cd pkg/arch
makepkg --printsrcinfo
makepkg --verifysource
makepkg -f
```

## 6. Post-build Smoke Checks

Open a sample project in the released app and verify:

- Create, edit, rename, move, and delete a document.
- Git history shows a new save revision.
- Restore a previous document revision.
- Compile/export includes the latest editor contents.
- App close waits for pending saves.
- Settings secrets are not written in plaintext.
- Scrivener import rejects hostile path traversal fixtures.

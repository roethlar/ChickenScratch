# ChickenScratch

A cross-platform writing app for fiction writers. Open-source Scrivener alternative with git-native revision control.

**Status:** Alpha (v0.1.0-alpha) — functional, seeking feedback from writers

For usage instructions, see the [User Guide](docs/USER_GUIDE.md).

## Build

Requires: Rust, Node.js, Pandoc

```bash
git clone https://github.com/yourusername/ChickenScratch.git
cd ChickenScratch
cd ui && npm install && cd ..
cargo tauri build
```

Output is in `target/release/bundle/`.

### Platform packages

- **macOS:** `.app` and `.dmg` in `target/release/bundle/macos/`
- **Linux:** AppImage in `target/release/bundle/appimage/`
- **Arch Linux:** PKGBUILD in `pkg/arch/`
- **Windows:** `.msi` in `target/release/bundle/msi/`

### Development

```bash
# Terminal 1
cd ui && npx vite --port 1420

# Terminal 2
cargo tauri dev
```

### Converter CLI

Standalone Scrivener converter, no GUI needed:

```bash
cargo build --release -p chikn-converter
./target/release/chikn-converter MyNovel.scriv
```

## Architecture

```
ChickenScratch/
├── crates/core/     # Rust library: .chikn format, Scrivener conversion, git
├── crates/cli/      # chikn-converter binary
├── src-tauri/       # Tauri app backend (commands, settings, AI)
├── ui/              # React + TypeScript + TipTap frontend
├── pkg/arch/        # Arch Linux PKGBUILD
└── docs/            # Format spec, design docs, user guide
```

## Dependencies

- [Tauri 2](https://tauri.app/) — app framework
- [TipTap](https://tiptap.dev/) — WYSIWYG editor
- [git2-rs](https://github.com/rust-lang/git2-rs) — embedded git (no system git needed)
- [Pandoc](https://pandoc.org/) — document format conversion

## License

MIT

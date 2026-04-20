# ChickenScratch

A cross-platform writing app for fiction writers. Open-source Scrivener alternative with git-native revision control.

**Status:** Alpha (v0.1.0-alpha) — functional, seeking feedback from writers

For usage instructions, see the [User Guide](docs/USER_GUIDE.md).

## Platforms

| Platform | Implementation | Status |
|----------|---------------|--------|
| macOS / Linux | Tauri + Rust + React | Alpha |
| Windows | WinUI 3 (Windows App SDK) + C# | In development |
| TUI | Ratatui + Rust | In development |

## Build

### Tauri (macOS / Linux)

Requires: Rust, Node.js, Pandoc

```bash
git clone https://github.com/roethlar/ChickenScratch.git
cd ChickenScratch
cd ui && npm install && cd ..
cargo tauri build
```

Output is in `target/release/bundle/`.

- **macOS:** `.app` and `.dmg` in `target/release/bundle/macos/`
- **Linux:** AppImage in `target/release/bundle/appimage/`
- **Arch Linux:** PKGBUILD in `pkg/arch/`

#### Development

```bash
# Terminal 1
cd ui && npx vite --port 1420

# Terminal 2
cargo tauri dev
```

### Windows (WinUI 3)

Requires: .NET 8 SDK, Windows App SDK, Pandoc

```bash
cd windows
dotnet build ChickenScratch.sln /p:Platform=x64 /p:Configuration=Release
```

Output is in `windows/ChickenScratch.App/bin/x64/Release/`.

### Converter CLI

Standalone Scrivener converter, no GUI needed:

```bash
cargo build --release -p chikn-converter
./target/release/chikn-converter MyNovel.scriv
```

## Architecture

```
ChickenScratch/
├── crates/core/        # Rust library: .chikn format, Scrivener conversion, git
├── crates/cli/         # chikn-converter binary
├── crates/tui/         # Terminal UI (Ratatui)
├── src-tauri/          # Tauri app backend (commands, settings, AI)
├── ui/                 # React + TypeScript + TipTap frontend
├── windows/            # WinUI 3 app (C# / Windows App SDK)
│   ├── ChickenScratch.Core/   # C# library: .chikn I/O, git, compile
│   └── ChickenScratch.App/    # WinUI 3 app shell
├── pkg/arch/           # Arch Linux PKGBUILD
└── docs/               # Format spec, design docs, user guide
```

## Dependencies

- [Tauri 2](https://tauri.app/) — app framework (macOS/Linux)
- [Windows App SDK](https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/) — app framework (Windows)
- [TipTap](https://tiptap.dev/) — WYSIWYG editor (shared via WebView2 on Windows)
- [git2-rs](https://github.com/rust-lang/git2-rs) / [LibGit2Sharp](https://github.com/libgit2/libgit2sharp) — embedded git
- [Pandoc](https://pandoc.org/) — document format conversion

## License

MIT

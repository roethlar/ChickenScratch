# ChickenScratch

A cross-platform writing app for fiction writers. Open-source Scrivener alternative with git-native revision control.

**The product is the [`.chikn` format](docs/CHIKN_FORMAT_SPEC.md)** — plain Markdown projects with embedded history. This repo’s Rust **engine** (`crates/core`) is the canonical reader/writer; the **Tauri app** is the reference GUI.

**Owner:** open this folder in Grok, Codex, Claude Code, or Antigravity CLI — say what you need ([`docs/START-HERE.md`](docs/START-HERE.md))

**Status:** v1.0.0 release target — Tauri desktop is the primary supported build. `macos/`, `windows/`, `linux/` native trees are deprecated.

For usage instructions, see the [User Guide](docs/USER_GUIDE.md).

## Platforms

| Platform | Implementation | Status |
|----------|---------------|--------|
| macOS / Linux | Tauri + Rust + React | 1.0 release target — fullest feature set |
| macOS (native) | SwiftUI + Liquid Glass (macOS 26+) | Early scaffold — writing + revisions |
| Windows | WinUI 3 (Windows App SDK) + C# | Preview — packaging (.msi) pending |
| Linux (native) | Qt6 Wayland + cxx-qt | Early scaffold — binder + editor + inspector |
| TUI (any OS) | Ratatui + Rust (`chikn` binary) | Preview |

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

- **macOS:** `.app` in `target/release/bundle/macos/`; `.dmg` in `target/release/bundle/dmg/`
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

Requires: .NET 10 SDK, Windows App SDK, Pandoc

```bash
cd windows
dotnet build ChickenScratch.App/ChickenScratch.App.csproj /p:Platform=x64 /p:Configuration=Release
```

Output is in `windows/ChickenScratch.App/bin/x64/Release/`.

### macOS (SwiftUI, Liquid Glass)

Requires: macOS 26 (Tahoe), Swift 6.1+ (Xcode 26 or the matching CLT).

```bash
cd macos
swift build
swift run ChickenScratch
```

Or open `macos/Package.swift` in Xcode 26.

### Linux (Qt6, cxx-qt)

Requires: Rust, Qt 6.x (`qtbase`, `qtdeclarative`), Pandoc.

```bash
cargo build --release -p chickenscratch-linux
./target/release/chickenscratch-linux
```

`linux/` is excluded from the workspace `default-members` so `cargo build` at the root doesn't require Qt. Build it explicitly by package name, or `cd linux && cargo build`.

### TUI (`chikn`)

Terminal UI for SSH / tmux / anywhere without a GUI. No external deps beyond Rust.

```bash
cargo build --release -p chickenscratch-tui
./target/release/chikn ~/Writing/MyNovel.chikn
```

### Converter CLI

Standalone Scrivener converter, no GUI needed:

```bash
cargo build --release -p chikn-converter
./target/release/chikn-converter MyNovel.scriv
```

## Format

Projects are stored in the open [`.chikn`](docs/CHIKN_FORMAT_SPEC.md) format: a folder of Markdown plus YAML manifests plus embedded git history. Fully human-readable, diff-clean, and editable in any text editor.

The underlying design — "a project is a folder, content is Markdown, metadata is in sidecars, history is embedded git, schemas evolve by rule" — is documented as a reusable pattern in [**Folder-First Documents**](docs/FOLDER_FIRST_DOCUMENTS.md). `.chikn` is the reference implementation; the pattern itself applies well beyond prose fiction (lab notebooks, knowledge bases, case files, course material, TTRPG campaigns, anywhere a project is more than one file and history matters).

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
├── macos/              # Native SwiftUI app (Liquid Glass, macOS 26+)
│   ├── Sources/ChiknKit/          # Swift library: .chikn reader/writer
│   └── Sources/ChickenScratchApp/ # SwiftUI app shell
├── linux/              # Qt6 Wayland-native app (cxx-qt)
│   ├── src/                       # Rust bridge + main
│   └── qml/                       # QML UI (binder, editor, inspector, find/replace)
├── pkg/arch/           # Arch Linux PKGBUILD
└── docs/               # Format spec, design docs, user guide
```

## Dependencies

- [Tauri 2](https://tauri.app/) — app framework (Tauri frontend)
- [Windows App SDK](https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/) — app framework (Windows)
- [cxx-qt](https://github.com/KDAB/cxx-qt) — Rust ↔ Qt6 bindings (Linux native)
- [TipTap](https://tiptap.dev/) — WYSIWYG editor (Tauri frontend)
- [git2-rs](https://github.com/rust-lang/git2-rs) / [LibGit2Sharp](https://github.com/libgit2/libgit2sharp) — embedded git
- [Ratatui](https://ratatui.rs/) — TUI framework
- [Pandoc](https://pandoc.org/) — document format conversion

## License

MIT

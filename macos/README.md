# ChickenScratch — macOS (SwiftUI)

Native macOS frontend for ChickenScratch, written in SwiftUI with Apple's Liquid Glass design language. Reads the same `.chikn` projects as the Tauri, WinUI 3, and TUI frontends.

**Status:** Early scaffold. Can open a `.chikn` project and browse its binder in a three-pane window. Editing, revisions, compile, comments — not yet.

## Requirements

- macOS 26 (Tahoe) or later — required for `glassEffect` / `GlassEffectContainer`
- Swift 6.1 toolchain (Xcode 26 or Command Line Tools)

## Build

```bash
cd macos
swift build
swift run ChickenScratch
```

Or open `Package.swift` in Xcode 26 and run.

## Layout

```
macos/
├── Package.swift                       # SwiftPM manifest
├── Sources/
│   ├── ChickenScratchApp/              # Executable app target
│   │   ├── ChickenScratchApp.swift     # @main App + Scene
│   │   ├── Design/
│   │   │   └── Glass.swift             # Shared glass modifiers
│   │   ├── State/
│   │   │   └── ProjectStore.swift      # @Observable store
│   │   └── Views/
│   │       ├── RootView.swift          # Welcome vs project switch
│   │       ├── WelcomeView.swift       # New / Open / Recent
│   │       ├── ProjectWindow.swift     # Three-pane NavigationSplitView
│   │       ├── Binder/BinderView.swift
│   │       ├── Editor/EditorView.swift
│   │       └── Inspector/InspectorView.swift
│   └── ChiknKit/                       # .chikn format library
│       ├── Models.swift                # Project, Document, TreeNode
│       └── Reader.swift                # Load project.yaml + documents
└── Tests/
    └── ChiknKitTests/
        └── ReaderTests.swift
```

## Liquid Glass notes

Liquid Glass is reserved for the "navigation layer that floats above app content":

- **Sidebar (Binder):** `NavigationSplitView` automatically renders the sidebar on floating glass.
- **Toolbar:** system auto-applies glass to `.toolbar` items; buttons use `.buttonStyle(.glass)` / `.glassProminent`.
- **Inspector:** explicit `GlassEffectContainer` with `.glassEffect(.regular, in: .rect(cornerRadius: .containerConcentric))`.
- **Editor:** no glass. Content layer only. Putting glass on the editor harms legibility.

Glass corners use `.containerConcentric` so the inspector/sidebar align with the window chrome regardless of window radius.

# SwiftUI WYSIWYG Editor

The SwiftUI reference editor is a native macOS experience for editing `.chikn` projects with live formatting and git-backed "Revisions" workflows. It lives in `swiftui-editor/` as a Swift Package Manager executable target.

## Features

- **Fast project loading** – Parses `project.yaml` and document `.meta` files via Yams, mapping to shared `ChiknProject` models.
- **Rich text editing** – Wraps `NSTextView` inside SwiftUI for a true WYSIWYG surface with bold/italic/underline, headings, and bullet tools.
- **Markdown round-trip** – Converts between Markdown and attributed text using `AttributedString` APIs, preserving Git-friendly plain text files.
- **Metadata inspector** – Displays and updates document titles, synopsis, keywords, and creation/modification timestamps.
- **Auto-save** – Debounced persistence writes Markdown and YAML as you type.
- **Revisions (git)** – Initializes repositories, shows pending changes, commits with writer-centric labels, pushes/pulls, and manages alternate branches.

## Requirements

- macOS 13 or newer
- Xcode 15+ or Swift 5.9 toolchain (`swift build`)
- System git available on the `PATH`

## Running

```bash
open swiftui-editor/Package.swift   # launch in Xcode
# or build with the Swift CLI
cd swiftui-editor
swift build
```

In Xcode, choose the `ChickenScratchEditor` scheme and run. Use ⌘O or **File → Open** to select a `.chikn` project directory (see `samples/` for fixtures).

## Git Workflows

The git panel surfaces the writer terminology defined in the spec:

- **Enable Revisions** → `git init`
- **Save Revision** → stages all changes and commits with the provided summary
- **Sync** → `git pull` then `git push`
- **Alternate drafts** → branch creation and switching (`git checkout -b`, `git checkout`)

Git status is sourced from `git status --porcelain -b`, mapping entries to friendly labels (Added, Modified, Deleted, New). Branch/ahead/behind counts are surfaced alongside the current upstream.

## Architecture Snapshot

```
swiftui-editor/
├── Package.swift                 # SPM executable target
├── Sources/ChickenScratchEditor/
│   ├── ChickenScratchEditorApp.swift
│   ├── Models/                    # ChiknProject, ChiknDocument, Tree nodes
│   ├── Services/                  # YAML loader/writer, Markdown, Git
│   ├── ViewModels/                # Project + Git state machines
│   ├── Views/                     # Navigator, Editor, Inspector, Git panel
│   └── Utilities/                 # Date parsing, shell wrapper, debouncer
└── Tests/                         # Placeholder SPM test target
```

The editor intentionally reuses the documented `.chikn` schema so saved projects remain compatible with the Rust/Tauri implementation.

## Limitations

- External Swift toolchain is required; the dev container may not ship `swift` by default.
- Currently optimized for Markdown + inline styles; Scrivener metadata beyond label/status/synopsis is read-only.
- Git operations depend on existing remote configuration when syncing.


# Chicken Scratch — Linux (Qt6 / cxx-qt)

Wayland-native Qt Quick frontend that shares `crates/core` with the Tauri, TUI, and WinUI apps.

## Prerequisites

- Rust stable
- Qt 6.5+ (tested against 6.11)
  - `qt6-base`, `qt6-declarative`, `qt6-quickcontrols2` (Arch)
  - `qt6-base-dev`, `qt6-declarative-dev`, `qt6-quickcontrols2-dev` (Debian/Ubuntu)

## Build & run

```
cargo run -p chickenscratch-linux
```

Forces Wayland:

```
QT_QPA_PLATFORM=wayland cargo run -p chickenscratch-linux
```

## Current scope (first scaffold)

- Three-pane layout: binder / editor / inspector stub
- Material Dark theme, accent colour keyed to the app identity
- `Ctrl+O` opens a `.chikn` project via native folder dialog
- Click a document in the binder → loads markdown into the editor
- `Ctrl+S` saves; footer shows Saved / Modified / Saving status
- Word count live in the inspector

## Follow-ups

- Inspector metadata editing (title, synopsis, label, status, keywords)
- Tree view instead of flattened list
- Comments + footnotes gutter
- Revisions panel
- Compile dialog
- Find/Replace
- Project Search palette
- Command Palette
- AI panel
- Settings
- Templates
- Session word-count / writing history

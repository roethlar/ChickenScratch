# ChickenScratch VS Code Extension

Native .chikn project support for VS Code and code-server.

## Features

- Automatic .chikn project detection
- Manuscript tree view in sidebar
- Create/delete documents and folders
- Integrated with .chikn format (project.yaml + .md + .meta files)

## Installation

### For Local Development

```bash
cd vscode-extension
npm install
npm run compile
```

Then press F5 in VS Code to launch Extension Development Host.

### For code-server

```bash
cd vscode-extension
npm install
npm run compile
code-server --install-extension .
```

## Usage

1. Open a folder containing `.chikn` projects
2. View will appear in activity bar with manuscript icon
3. Use commands:
   - "ChickenScratch: New Project" - Create .chikn project
   - Click + icon to create documents
   - Click trash icon to delete

## Development

```bash
npm run watch   # Compile on file changes
npm run lint    # Run linter
```

## .chikn Format Support

This extension understands the .chikn format:
- `project.yaml` - Project metadata and hierarchy
- `manuscript/*.md` - Document content (Markdown)
- `manuscript/*.meta` - Document metadata (YAML)

## TODO

- [ ] Nested folder support
- [ ] Drag-and-drop reordering
- [ ] Document metadata editing
- [ ] Integration with git commands

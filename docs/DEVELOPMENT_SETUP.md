# Chicken Scratch - Development Setup

**Version:** 1.0
**Date:** 2025-10-01

---

## Prerequisites

### Required Software

1. **Rust** (stable channel)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup update stable
   ```

2. **Node.js** (v20+)
   ```bash
   # Via nvm (recommended)
   nvm install 20
   nvm use 20
   ```

3. **pnpm** (package manager)
   ```bash
   npm install -g pnpm
   ```

4. **Tauri CLI**
   ```bash
   cargo install tauri-cli@^2.0
   # Or via npm
   npm install -g @tauri-apps/cli@^2.0
   ```

5. **System Dependencies**

   **Linux:**
   ```bash
   # Debian/Ubuntu
   sudo apt update
   sudo apt install libwebkit2gtk-4.1-dev \
     build-essential \
     curl \
     wget \
     file \
     libssl-dev \
     libgtk-3-dev \
     libayatana-appindicator3-dev \
     librsvg2-dev

   # Arch Linux
   sudo pacman -S webkit2gtk base-devel curl wget file openssl gtk3 librsvg
   ```

   **macOS:**
   ```bash
   xcode-select --install
   ```

   **Windows:**
   - Install Visual Studio 2022 with C++ desktop development
   - Install WebView2 (usually pre-installed on Windows 11)

---

## Project Setup

### 1. Clone Repository

```bash
git clone https://github.com/your-org/chicken-scratch.git
cd chicken-scratch
```

### 2. Install Dependencies

```bash
# Install Node.js dependencies
pnpm install

# Rust dependencies are managed by Cargo (auto-installed on first build)
```

### 3. Development Build

```bash
# Start development server (hot reload)
pnpm tauri dev

# This will:
# 1. Start Vite dev server (React frontend)
# 2. Compile Rust backend
# 3. Launch Tauri application window
```

### 4. Production Build

```bash
# Build for current platform
pnpm tauri build

# Output locations:
# - Linux: src-tauri/target/release/bundle/appimage/
# - macOS: src-tauri/target/release/bundle/dmg/
# - Windows: src-tauri/target/release/bundle/nsis/
```

---

## Development Workflow

### Running Tests

**Rust Backend Tests:**
```bash
cd src-tauri
cargo test

# With coverage
cargo tarpaulin --out Html
```

**Frontend Tests:**
```bash
# Unit/component tests
pnpm test

# Coverage
pnpm test:coverage

# UI mode (interactive)
pnpm test:ui
```

### Linting & Formatting

**Rust:**
```bash
cd src-tauri
cargo fmt          # Format code
cargo clippy      # Lint with suggestions
```

**TypeScript:**
```bash
pnpm lint         # ESLint
pnpm format       # Prettier
```

### Project Structure

```
chicken-scratch/
├── docs/                   # Project documentation
├── src/                    # React frontend
│   ├── components/        # UI components
│   ├── hooks/             # Custom React hooks
│   ├── store/             # Zustand state management
│   ├── types/             # TypeScript types
│   └── utils/             # Frontend utilities
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── api/           # Tauri commands
│   │   ├── core/          # Business logic
│   │   ├── models/        # Data models
│   │   └── utils/         # Utilities
│   └── Cargo.toml
├── package.json
└── README.md
```

---

## Common Tasks

### Creating New Rust Module

1. Create file in appropriate directory (e.g., `src-tauri/src/core/new_module.rs`)
2. Add module declaration in parent `mod.rs`
3. Export public types in `lib.rs` if needed
4. Write tests in the same file under `#[cfg(test)]` section

### Creating New React Component

1. Create file in `src/components/category/ComponentName.tsx`
2. Follow TypeScript component template in `AI_DEVELOPMENT_GUIDE.md`
3. Add tests in `ComponentName.test.tsx` (same directory)
4. Export from component module if needed

### Adding Tauri Command

1. Implement command in `src-tauri/src/api/category_commands.rs`
2. Add to `generate_handler![]` in `main.rs`
3. Create TypeScript wrapper in `src/utils/tauri.ts`
4. Use in React components via `invoke()`

---

## Troubleshooting

### Build Failures

**"Failed to bundle project":**
- Ensure all system dependencies installed
- Check `src-tauri/target/` for detailed error logs
- Verify Rust version: `rustc --version` (should be 1.70+)

**"Port 1420 already in use":**
- Kill existing process: `lsof -ti:1420 | xargs kill -9`
- Or change port in `vite.config.ts`

**"Cannot find module '@tauri-apps/api'":**
- Reinstall dependencies: `pnpm install --force`
- Clear cache: `rm -rf node_modules pnpm-lock.yaml`

### Runtime Issues

**Tauri commands not working:**
- Check command is registered in `main.rs`
- Verify function signature matches Tauri requirements
- Check browser console for frontend errors

**File system permissions:**
- Ensure app has read/write access to project directories
- On macOS, grant Full Disk Access in System Preferences

---

## IDE Setup

### VS Code (Recommended)

**Extensions:**
- rust-analyzer (Rust language server)
- Tauri (Tauri development)
- ESLint (TypeScript linting)
- Prettier (Code formatting)
- Tailwind CSS IntelliSense

**Settings (.vscode/settings.json):**
```json
{
  "rust-analyzer.checkOnSave.command": "clippy",
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  },
  "[typescript]": {
    "editor.defaultFormatter": "esbenp.prettier-vscode"
  },
  "[typescriptreact]": {
    "editor.defaultFormatter": "esbenp.prettier-vscode"
  }
}
```

---

## External Dependencies

### Pandoc (Required for Phase 3+)

**Linux:**
```bash
# Debian/Ubuntu
sudo apt install pandoc

# Arch
sudo pacman -S pandoc
```

**macOS:**
```bash
brew install pandoc
```

**Windows:**
Download installer from https://pandoc.org/installing.html

### Git (Required for Phase 4+)

Usually pre-installed on most systems. Verify:
```bash
git --version
```

---

## Next Steps

After setup:
1. Read `docs/PROJECT_SPECIFICATION.md` for complete feature overview
2. Read `docs/ARCHITECTURE.md` for system architecture
3. Read `docs/AI_DEVELOPMENT_GUIDE.md` for coding conventions
4. Read `docs/design/PHASE_1_DESIGN.md` for current phase details
5. Start with Phase 1 implementation tasks

---

## Support

- **Documentation:** `docs/` directory
- **Issues:** GitHub Issues (when repository is public)
- **Discussions:** GitHub Discussions

**Happy coding! 🐔**

# UI-Agnostic Architecture Design

**Version:** 1.0
**Date:** 2025-10-10
**Status:** Design Phase

---

## Executive Summary

This document proposes refactoring ChickenScratch's Rust backend into a UI-agnostic core library that can be used by multiple frontend implementations:

- **SwiftUI** (macOS native)
- **GTK** (Linux native)
- **WinUI 3** (Windows native)
- **Tauri/React** (web-based, optional)

**Goal:** Write the core `.chikn` format handling, Scrivener conversion, and git integration once in Rust, then provide language-specific bindings for each UI framework.

---

## Current Architecture Issues

### Problem 1: Tauri Lock-In

Current code structure:
```
src-tauri/
├── src/
│   ├── api/              # Tauri commands (Tauri-specific)
│   ├── core/             # Business logic (Pure Rust ✓)
│   ├── models/           # Data structures (Pure Rust ✓)
│   ├── scrivener/        # Scrivener parser (Pure Rust ✓)
│   └── utils/            # Utilities (Pure Rust ✓)
```

**Issue:** `api/` folder contains Tauri commands like:
```rust
#[tauri::command]
pub async fn load_project(path: String) -> Result<Project, ChiknError>
```

SwiftUI, GTK, and WinUI cannot call `#[tauri::command]` functions. They need:
- **Swift:** Functions callable from Swift code
- **GTK (Rust):** Direct Rust function calls
- **WinUI (C#):** C-compatible functions or .NET bindings

### Problem 2: Cannot Verify Backend Works

Current Tauri app won't compile because React frontend (`../dist`) doesn't exist yet. This blocks:
- Running tests
- Validating that `.chikn` format reading/writing works
- Verifying Scrivener import functionality

### Problem 3: Future UI Flexibility

If we continue down the Tauri path:
- Tauri becomes the "official" architecture
- Building native UIs later requires rework
- We lose ability to choose best UI framework per platform

---

## Proposed Architecture

### New Project Structure

```
ChickenScratch/
│
├── core/                          # Pure Rust library (UI-agnostic)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                 # Public API exports
│   │   ├── project/               # .chikn format (read/write)
│   │   ├── scrivener/             # .scriv import/export
│   │   ├── git/                   # Git operations (future)
│   │   ├── snapshots/             # revs/ snapshot system
│   │   ├── models/                # Data structures
│   │   └── error.rs               # Error types
│   └── tests/                     # Integration tests
│
├── bindings/                      # Language bindings (bridges)
│   ├── swift/                     # Swift bindings for macOS
│   │   ├── Cargo.toml
│   │   ├── src/lib.rs             # Rust→Swift bridge
│   │   └── ChickenScratchCore/   # Swift package
│   │       └── Sources/
│   │           └── ChickenScratchCore.swift
│   │
│   ├── dotnet/                    # C# bindings for Windows
│   │   ├── Cargo.toml
│   │   ├── src/lib.rs             # Rust→C# bridge
│   │   └── ChickenScratchCore/   # .NET library
│   │
│   └── tauri/                     # Tauri bindings (optional web UI)
│       ├── Cargo.toml
│       ├── src/
│       │   ├── main.rs
│       │   └── commands.rs        # Tauri command wrappers
│       └── ...
│
├── apps/                          # UI applications
│   ├── macos/                     # SwiftUI app
│   │   └── ChickenScratch.xcodeproj
│   │
│   ├── linux/                     # GTK app (Rust)
│   │   └── Cargo.toml
│   │
│   ├── windows/                   # WinUI 3 app (C#)
│   │   └── ChickenScratch.sln
│   │
│   └── web/                       # Tauri app (optional)
│       └── (React frontend)
│
└── docs/
    └── UI_AGNOSTIC_ARCHITECTURE.md  # This document
```

---

## Core Library Design

### Public API (`core/src/lib.rs`)

The core library exposes a pure Rust API:

```rust
// Public API for all UIs to use
pub mod project;      // Project CRUD operations
pub mod document;     // Document operations
pub mod scrivener;    // Scrivener import/export
pub mod snapshots;    // Snapshot create/restore
pub mod models;       // Data structures (Project, Document, etc.)
pub mod error;        // Error types

// Re-export commonly used types
pub use models::{Project, Document, TreeNode};
pub use error::ChiknError;

// Example public functions
pub use project::{
    create_project,
    load_project,
    save_project,
};
```

**Key Principles:**
1. **No UI dependencies** - No Tauri, no GUI frameworks
2. **Standard Rust types** - Use `String`, `PathBuf`, `Result`, not UI-specific types
3. **Serializable data** - All structs derive `Serialize`/`Deserialize` for easy cross-language transfer
4. **Well-tested** - Unit and integration tests don't require UI
5. **Platform-agnostic** - File paths use `std::path::PathBuf` (works on all OSes)

---

## Language Bindings (Bridges)

Each UI framework needs a "binding" layer that translates between Rust and the target language.

### Option 1: C FFI (Foreign Function Interface)

**What is FFI?**
FFI = "Foreign Function Interface" - a way for one programming language to call functions written in another language. Most languages can call C functions, so we expose Rust functions using C-compatible types.

**How it works:**
1. Rust exposes functions with C-compatible signatures (no Rust-specific types like `String`)
2. Use `extern "C"` to make functions callable from C
3. Swift/C# call these C functions
4. Convert data between Rust ↔ C ↔ Swift/C#

**Example (Swift FFI):**

Rust side (`bindings/swift/src/lib.rs`):
```rust
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use chickenscratch_core::{load_project, Project};

// C-compatible function (no Rust String, uses C char*)
#[no_mangle]
pub extern "C" fn chikn_load_project(path: *const c_char) -> *mut Project {
    // Convert C string → Rust String
    let c_str = unsafe { CStr::from_ptr(path) };
    let path_str = c_str.to_str().unwrap();

    // Call core library
    let project = load_project(path_str).unwrap();

    // Convert Rust Project → C pointer
    Box::into_raw(Box::new(project))
}

#[no_mangle]
pub extern "C" fn chikn_free_project(project: *mut Project) {
    unsafe { Box::from_raw(project) };  // Free memory
}
```

Swift side (`bindings/swift/ChickenScratchCore/Sources/ChickenScratchCore.swift`):
```swift
import Foundation

// Import C functions from Rust library
@_silgen_name("chikn_load_project")
func chikn_load_project(_ path: UnsafePointer<CChar>) -> OpaquePointer

@_silgen_name("chikn_free_project")
func chikn_free_project(_ project: OpaquePointer)

// Swift wrapper
class ChickenScratchCore {
    func loadProject(path: String) -> Project? {
        let cPath = path.withCString { $0 }
        let projectPtr = chikn_load_project(cPath)

        // Convert C data → Swift struct
        let project = Project(from: projectPtr)

        chikn_free_project(projectPtr)
        return project
    }
}
```

**Pros:**
- Universal: Works with Swift, C#, Python, JavaScript, etc.
- Full control over memory and performance

**Cons:**
- Manual memory management (unsafe Rust required)
- Manual type conversion (Rust String ↔ C char* ↔ Swift String)
- Verbose and error-prone

---

### Option 2: UniFFI (Unified FFI)

**What is UniFFI?**
UniFFI is a Mozilla tool that generates FFI bindings automatically. You write an interface definition file (IDL), and it generates:
- C-compatible Rust code
- Swift bindings
- Kotlin bindings (Android)
- Python bindings

**How it works:**
1. Write `.udl` interface file describing your API
2. UniFFI generates all the binding code
3. Call Rust functions from Swift/Kotlin/Python naturally

**Example (UniFFI):**

Interface definition (`core/src/chickenscratch.udl`):
```idl
namespace chickenscratch {
    Project load_project(string path);
    void save_project(Project project);
};

dictionary Project {
    string id;
    string name;
    string path;
    sequence<TreeNode> hierarchy;
};

dictionary TreeNode {
    string id;
    string name;
    NodeType type;
};

enum NodeType {
    "Document",
    "Folder"
};
```

Rust implementation stays the same:
```rust
pub fn load_project(path: String) -> Project {
    // Implementation unchanged
}
```

Swift usage (auto-generated):
```swift
import ChickenScratchCore

let project = loadProject(path: "/path/to/project.chikn")
print(project.name)
```

**Pros:**
- Automatic code generation (less manual work)
- Type-safe (compile errors if Swift/Rust don't match)
- Handles memory management automatically
- Idiomatic code in target language (Swift feels like Swift)

**Cons:**
- Learning curve (new tool to understand)
- Less control over generated code
- Adds build complexity

---

### Option 3: Direct Rust (GTK)

For GTK on Linux, we don't need FFI at all - GTK has Rust bindings (`gtk-rs`). The UI can directly call the core library:

```rust
// apps/linux/src/main.rs
use chickenscratch_core::{load_project, save_project};
use gtk::prelude::*;

fn on_open_project(path: &str) {
    let project = load_project(path).unwrap();
    // Update GTK UI with project data
}
```

**Pros:**
- No FFI needed (pure Rust)
- Best performance
- Type safety at compile time

**Cons:**
- Only works for Rust-based UIs (GTK)

---

## Data Exchange Format

All UIs need to exchange complex data (projects, documents, hierarchies). Options:

### Option A: JSON over FFI

Rust serializes to JSON, passes string to UI, UI deserializes:

Rust:
```rust
#[no_mangle]
pub extern "C" fn chikn_load_project(path: *const c_char) -> *mut c_char {
    let project = load_project(path_str).unwrap();
    let json = serde_json::to_string(&project).unwrap();
    CString::new(json).unwrap().into_raw()
}
```

Swift:
```swift
let jsonString = String(cString: chikn_load_project(cPath))
let project = try JSONDecoder().decode(Project.self, from: jsonString.data)
```

**Pros:**
- Simple (all languages understand JSON)
- Flexible (easy to add fields)

**Cons:**
- Serialization overhead
- Runtime errors if JSON schema mismatches

### Option B: Structured FFI (UniFFI)

UniFFI generates type-safe conversions automatically. No JSON serialization needed.

**Pros:**
- Compile-time type safety
- Better performance (no JSON parsing)

**Cons:**
- Requires UniFFI

---

## Migration Strategy

### Phase 1: Extract Core Library (Week 1)

**Goal:** Separate UI-agnostic code from Tauri-specific code.

**Tasks:**
1. Create `core/` directory as new Cargo workspace member
2. Move existing code:
   - `src-tauri/src/core/` → `core/src/`
   - `src-tauri/src/models/` → `core/src/models/`
   - `src-tauri/src/scrivener/` → `core/src/scrivener/`
   - `src-tauri/src/utils/` → `core/src/utils/`
3. Remove all Tauri dependencies from `core/Cargo.toml`
4. Fix compilation errors
5. Run tests: `cargo test -p chickenscratch-core`

**Success criteria:**
- Core library compiles independently
- All tests pass
- Zero Tauri dependencies in core

---

### Phase 2: Create Swift Bindings (Week 2)

**Goal:** Prove SwiftUI can call Rust core library.

**Decision needed:** FFI vs UniFFI (see OPEN_QUESTIONS.md)

**Tasks (if FFI):**
1. Create `bindings/swift/` crate
2. Write C FFI wrappers for essential functions:
   - `chikn_load_project`
   - `chikn_save_project`
   - `chikn_create_document`
3. Build as dynamic library (`.dylib`)
4. Create Swift package that imports `.dylib`
5. Write Swift wrapper classes

**Tasks (if UniFFI):**
1. Add UniFFI dependency to core
2. Write `chickenscratch.udl` interface definition
3. Add UniFFI build step
4. Generate Swift bindings
5. Create Swift package

**Success criteria:**
- Swift can load a `.chikn` project
- Swift can read project metadata
- Can call from Swift without crashes

---

### Phase 3: Build Minimal SwiftUI App (Week 3)

**Goal:** Working macOS app that opens `.chikn` projects.

**Tasks:**
1. Create Xcode project in `apps/macos/`
2. Link Swift bindings
3. Build UI:
   - File picker to select `.chikn` folder
   - Display project name
   - Show document tree (NavigationView)
   - Display selected document content (TextEditor)
4. Test with sample `.chikn` project

**Success criteria:**
- macOS app runs on your MacBook
- Can open existing `.chikn` project
- Can view document hierarchy
- Can read document content

---

### Phase 4: Parallel - Keep Tauri Option (Week 3)

**Goal:** Maintain Tauri as optional web UI.

**Tasks:**
1. Move Tauri app to `bindings/tauri/`
2. Update Tauri commands to use `chickenscratch_core` crate
3. Verify Tauri app still works (once React frontend exists)

**Success criteria:**
- Tauri app uses same core library as SwiftUI
- Both UIs can open same `.chikn` projects
- No code duplication between UIs

---

### Phase 5: Design Native UI Language (Week 4+)

**Goal:** Use SwiftUI app as reference for GTK and WinUI implementations.

Once SwiftUI is working:
1. Document UI patterns (how navigation works, editor layout, etc.)
2. Design consistent UX across platforms
3. Use SwiftUI as "reference implementation"
4. Build GTK (Rust) and WinUI (C#) apps following same patterns

---

## Testing Strategy

### Core Library Tests

Pure Rust unit tests (no UI needed):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_load_project() {
        let project = create_project("/tmp/test.chikn", "Test").unwrap();
        let loaded = load_project("/tmp/test.chikn").unwrap();
        assert_eq!(project.name, loaded.name);
    }

    #[test]
    fn test_scrivener_import() {
        let project = import_scrivener("/path/to/test.scriv").unwrap();
        assert_eq!(project.documents.len(), 5);
    }
}
```

**Run tests:** `cargo test -p chickenscratch-core`

No UI required to validate backend works!

---

### Binding Tests

Each binding layer needs tests:

**Swift:**
```swift
import XCTest
@testable import ChickenScratchCore

class CoreTests: XCTestCase {
    func testLoadProject() {
        let core = ChickenScratchCore()
        let project = core.loadProject(path: "/tmp/test.chikn")
        XCTAssertNotNil(project)
    }
}
```

---

## Build System

### Cargo Workspace

`Cargo.toml` (root):
```toml
[workspace]
members = [
    "core",
    "bindings/swift",
    "bindings/dotnet",
    "bindings/tauri",
    "apps/linux",
]

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_yaml = "0.9"
```

### Swift Package Manager

`bindings/swift/Package.swift`:
```swift
// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "ChickenScratchCore",
    products: [
        .library(name: "ChickenScratchCore", targets: ["ChickenScratchCore"]),
    ],
    targets: [
        .target(
            name: "ChickenScratchCore",
            dependencies: [],
            resources: [.process("libchickenscratch_swift.dylib")]
        ),
    ]
)
```

### Build Commands

```bash
# Build core library
cargo build -p chickenscratch-core

# Build Swift bindings
cd bindings/swift
cargo build --release
swift build

# Build SwiftUI app
cd apps/macos
xcodebuild
```

---

## Platform-Specific Considerations

### macOS (SwiftUI)

**Pros:**
- SwiftUI is modern, powerful, Apple-supported
- Great for macOS-first development
- Strong integration with macOS features

**Cons:**
- Swift bindings need FFI or UniFFI
- Xcode required for development

**Distribution:**
- `.app` bundle
- Mac App Store (requires signing)
- Direct download (.dmg)

---

### Linux (GTK)

**Pros:**
- GTK has mature Rust bindings (`gtk4-rs`)
- Can directly use core library (no FFI)
- Native Linux look and feel

**Cons:**
- GTK learning curve
- UI might not match SwiftUI exactly

**Distribution:**
- AppImage (portable)
- Flatpak (sandboxed)
- .deb/.rpm (system packages)

---

### Windows (WinUI 3)

**Pros:**
- Modern Windows UI framework
- C# is widely known
- Good Visual Studio support

**Cons:**
- Requires C# ↔ Rust bindings
- .NET runtime dependency

**Distribution:**
- MSIX package (Windows Store)
- .exe installer (NSIS, WiX)

---

### Web (Tauri - Optional)

**Pros:**
- Already started
- Cross-platform (Chromebooks, tablets)
- Web technologies familiar

**Cons:**
- Web-like feel (not native)
- Performance overhead

**Use case:**
- Hosted web app (future cloud product)
- Fallback for platforms without native app

---

## Open Questions

See `OPEN_QUESTIONS.md` for detailed questions that need answers before implementation.

Key decisions needed:
1. FFI vs UniFFI for Swift bindings?
2. JSON serialization vs structured data over FFI?
3. Keep Tauri or remove completely?
4. SwiftUI-first or parallel UI development?

---

## Success Metrics

**Architecture is successful if:**
1. ✅ Core library compiles and tests pass independently (no UI)
2. ✅ SwiftUI app can load `.chikn` projects
3. ✅ GTK app (future) uses same core without code duplication
4. ✅ WinUI app (future) uses same core without code duplication
5. ✅ Adding new features only requires updating core library, not all UIs
6. ✅ Different UIs can interoperate (same `.chikn` format, no corruption)

---

## Timeline Estimate

**Week 1:** Extract core library, run tests
**Week 2:** Create Swift bindings, verify FFI works
**Week 3:** Build minimal SwiftUI app, test on MacBook
**Week 4:** Refine UI, add basic editing
**Week 5+:** Expand to GTK and WinUI

**Total: ~1-2 months for UI-agnostic foundation + SwiftUI MVP**

---

## Conclusion

This architecture enables:
- **Platform-native UIs** that feel right on each OS
- **Code reuse** - write core logic once in Rust
- **Flexibility** - can choose best UI framework per platform
- **Testability** - validate backend without building UIs
- **Future-proof** - easy to add new UIs (iOS, Android) later

Next step: Review this design, answer open questions, then begin Phase 1 implementation.

# Open Questions - UI-Agnostic Architecture

**Date:** 2025-10-10
**Status:** Awaiting Decisions

These questions arose during the architecture design. They need answers before implementation begins.

---

## Critical Decisions (Block Implementation)

### Q1: FFI vs UniFFI for Language Bindings?

**Context:**
To call Rust code from Swift/C#, we need "bindings" that translate between languages.

**Option A: Manual C FFI (Foreign Function Interface)**

**How it works:**
- Rust exposes functions using C-compatible types (`extern "C"`)
- Swift/C# call these C functions
- We manually convert data types (Rust `String` → C `char*` → Swift `String`)

**Example:**
```rust
// Rust
#[no_mangle]
pub extern "C" fn load_project(path: *const c_char) -> *mut Project {
    // Manual conversion, memory management
}
```

```swift
// Swift
let projectPtr = load_project(cPath)  // Call C function
let project = convertToSwift(projectPtr)  // Manual conversion
```

**Pros:**
- Full control
- Works with any language
- No extra dependencies

**Cons:**
- Manual memory management (unsafe Rust)
- Verbose, error-prone
- Need to write conversion code for every function
- Runtime errors (crashes if memory management wrong)

---

**Option B: UniFFI (Unified FFI)**

**How it works:**
- Write `.udl` interface file describing your API
- UniFFI auto-generates all binding code
- Call Rust naturally from Swift/C#

**Example:**
```idl
// chickenscratch.udl
namespace chickenscratch {
    Project load_project(string path);
};

dictionary Project {
    string id;
    string name;
};
```

```swift
// Swift (auto-generated, type-safe)
let project = loadProject(path: "/path/to/project.chikn")
print(project.name)  // Feels like native Swift
```

**Pros:**
- Auto-generated (less code to write)
- Type-safe (compile errors, not crashes)
- Memory management handled automatically
- Idiomatic code in target language
- Mozilla-backed, actively maintained

**Cons:**
- New tool to learn
- Less control over generated code
- Adds build complexity
- Smaller community than raw FFI

---

**Question:** Which binding approach should we use?

**Recommendation:** UniFFI
- We're building multiple language bindings (Swift, C#, maybe more)
- Auto-generation saves significant time
- Type safety prevents whole classes of bugs
- Rust core stays clean (no unsafe FFI code)

**Counterpoint:**
If you prefer maximum control and want to learn FFI deeply, manual FFI is viable but slower.

**What do AI models recommend?**

---

### Q2: Data Serialization Strategy?

**Context:**
Complex data (Projects with nested documents) needs to cross language boundaries.

**Option A: JSON Serialization**

Rust → JSON string → Swift parses JSON:
```rust
pub extern "C" fn load_project(path: *const c_char) -> *mut c_char {
    let project = core::load_project(path)?;
    let json = serde_json::to_string(&project)?;
    CString::new(json).unwrap().into_raw()
}
```

**Pros:**
- Simple
- All languages understand JSON
- Easy debugging (can print JSON)

**Cons:**
- Serialization overhead (slow for large projects)
- Runtime errors if schema mismatches
- More memory allocations

---

**Option B: Structured Data via UniFFI**

UniFFI generates proper type conversions:
```rust
// Rust returns Project struct directly
pub fn load_project(path: String) -> Project {
    // ...
}
```

Swift gets Swift struct, C# gets C# class - no JSON:
```swift
let project: Project = loadProject(path: path)
```

**Pros:**
- Faster (no JSON parsing)
- Compile-time type checking
- Less memory overhead

**Cons:**
- Requires UniFFI
- More complex build process

---

**Question:** How should data cross language boundaries?

**Recommendation:** Structured data via UniFFI
- Better performance
- Type safety prevents bugs
- More maintainable long-term

**Alternative:** Start with JSON for prototyping, migrate to structured later?

**What do AI models recommend?**

---

### Q3: Tauri - Keep or Remove?

**Context:**
Current codebase uses Tauri (Rust + React). Do we keep this path open?

**Option A: Keep Tauri Bindings**
- Move to `bindings/tauri/` as one option among many
- Maintain thin wrapper around core library
- Use for future web UI or hosted app

**Pros:**
- Preserves existing work
- Enables web-based UI (Chromebooks, tablets)
- Could become cloud/hosted product

**Cons:**
- Maintenance burden (another binding to maintain)
- React frontend still needs building

---

**Option B: Remove Tauri Entirely**
- Delete `src-tauri/` after extracting core
- Focus only on native UIs (SwiftUI, GTK, WinUI)
- Build web UI later if needed

**Pros:**
- Cleaner focus
- Less complexity
- Native UIs are the priority

**Cons:**
- Lose option for web-based access
- Might regret later if cloud product needed

---

**Option C: Defer Decision**
- Extract core library now
- Keep Tauri code but don't actively maintain
- Revisit after SwiftUI MVP works

---

**Question:** What should we do with Tauri?

**Your earlier comment:** "the web UI will come later if people want it"

**Recommendation:** Option C (defer)
- Extract core, focus on SwiftUI
- Keep Tauri code in repo but frozen
- Rebuild web UI later if revenue justifies it

**What do AI models recommend?**

---

### Q4: SwiftUI-First or Parallel Development?

**Context:**
Do we build SwiftUI alone first, or start all native UIs simultaneously?

**Option A: SwiftUI-First (Sequential)**

Timeline:
1. Week 1-3: Build SwiftUI MVP
2. Week 4: Document UI patterns from SwiftUI
3. Week 5+: Build GTK using SwiftUI as reference
4. Week 8+: Build WinUI using same patterns

**Pros:**
- Focused development
- SwiftUI becomes reference implementation
- Learn from mistakes before repeating
- Can test on your MacBook immediately

**Cons:**
- Slower to cross-platform
- GTK/WinUI might diverge from SwiftUI patterns
- Risk of SwiftUI-specific assumptions

---

**Option B: Parallel Development**

Timeline:
1. Week 1: Design UI patterns (mockups, wireframes)
2. Week 2-6: Build all three UIs simultaneously
3. Week 7+: Refine and align UIs

**Pros:**
- Faster to cross-platform parity
- Forces platform-agnostic UI design
- No single platform privileged

**Cons:**
- More complex coordination
- Need developers for each platform
- Harder to test without all three OSes

---

**Question:** SwiftUI-first or parallel?

**Your earlier comment:** "once that's perfected, we can use that UI design language to build the other native apps"

**Recommendation:** Option A (SwiftUI-first)
- You have a MacBook (can test immediately)
- 100% AI-coded (easier to focus on one UI at a time)
- Establish patterns before scaling

**What do AI models recommend?**

---

## Technical Questions (Need Research)

### Q5: Swift Package Manager vs Xcode Project?

**Question:** How should Swift bindings be distributed?

**Option A:** Swift Package
- Standalone package that Xcode project imports
- Can be reused in multiple apps
- Standard Swift distribution

**Option B:** Directly in Xcode project
- Rust library compiled into app bundle
- Simpler setup
- Less portable

**What's the standard practice for Rust → Swift bindings?**

---

### Q6: Dynamic vs Static Linking?

**Question:** Should Rust core be compiled as:
- **Dynamic library** (.dylib on macOS, .so on Linux, .dll on Windows)?
- **Static library** (compiled into app binary)?

**Dynamic (.dylib):**
- Smaller app binary
- Core library can be updated independently
- Shared between multiple apps

**Static:**
- Larger app binary
- No runtime dependencies
- Simpler distribution

**Which is better for desktop apps?**

---

### Q7: Memory Management Strategy?

**Context:**
Rust owns memory, Swift/C# have garbage collection. Who owns a `Project` object?

**Option A: Rust owns data**
- Swift gets pointers to Rust-owned data
- Must call "free" functions when done
- Risk of memory leaks if Swift forgets to free

**Option B: Copy data to Swift**
- Rust serializes data
- Swift owns its own copy
- Rust frees immediately
- Safer but uses more memory

**If using UniFFI:** This is handled automatically (UniFFI uses Arc/RefCounted)

**Question:** Manual memory management or rely on UniFFI?

---

### Q8: Error Handling Across Languages?

**Context:**
Rust uses `Result<T, ChiknError>`. How do errors reach Swift/C#?

**Option A: Exceptions**
- Rust catches errors, converts to C error codes
- Swift/C# throw exceptions

**Option B: Result types**
- Swift/C# get `Result<Project, Error>` equivalent
- More Rust-like, explicit error handling

**Option C: UniFFI auto-conversion**
- UniFFI generates error handling automatically
- Rust errors become Swift errors naturally

**Question:** How should errors propagate?

---

### Q9: Async/Await Compatibility?

**Context:**
Current Rust uses `async fn`. Swift and C# also have async/await. Do they interoperate?

**Challenge:**
- Rust async runtime (Tokio) ≠ Swift async runtime
- Can't directly await Rust futures from Swift

**Options:**
1. Block in Rust, return synchronously (simple but blocks UI thread)
2. Use callbacks (works but old-school)
3. Use UniFFI async support (experimental, may have issues)

**Question:** Synchronous or asynchronous API for UIs?

**Recommendation:** Start synchronous, add async if needed
- Most operations are fast (reading .chikn files)
- UI can dispatch to background thread if needed

---

## Architecture Questions (Design Choices)

### Q10: Workspace Layout - Mono-repo or Multi-repo?

**Current proposal:** Mono-repo (everything in ChickenScratch/)

**Alternative:** Separate repos:
- `chickenscratch-core` (Rust library)
- `chickenscratch-macos` (SwiftUI app)
- `chickenscratch-linux` (GTK app)
- `chickenscratch-windows` (WinUI app)

**Question:** Mono-repo or multi-repo?

**Recommendation:** Mono-repo
- Easier coordination
- Shared docs and issues
- Atomic changes across core + UIs

**Downside:** Large repo, but not a problem for a single project

---

### Q11: Versioning Strategy?

**Question:** How do we version core library vs UI apps?

**Option A: Synchronized versioning**
- Core v1.0 → All UIs v1.0
- Breaking change → bump all versions

**Option B: Independent versioning**
- Core v1.2 + SwiftUI v1.5 + GTK v1.3
- UIs evolve independently

**Recommendation:** Synchronized for v1.0, independent later
- Start simple (everything is v1.0)
- Allow divergence once stable

---

### Q12: Testing - Unit Tests vs Integration Tests?

**Question:** How much testing in core vs UI layers?

**Recommendation:**
- **Core (Rust):** Heavy unit testing (80%+ coverage)
  - Format reading/writing
  - Scrivener conversion
  - Snapshot system
- **Bindings:** Light testing (FFI works, data converts)
- **UI Apps:** Manual testing + UI automation (Playwright, XCTest)

**Is this the right balance?**

---

### Q13: CI/CD - Build All Platforms?

**Question:** Should CI build all UIs on every commit?

**Challenge:**
- macOS builds require macOS runners (expensive)
- Windows builds require Windows runners
- Linux is easy (Ubuntu runners)

**Options:**
1. Build all platforms on every commit (expensive, slow)
2. Build only Linux in CI, manual builds for macOS/Windows
3. Build macOS/Windows only on releases

**Recommendation:** Start with option 2, move to option 1 when funded

---

## UX/UI Design Questions

### Q14: UI Consistency vs Platform Conventions?

**Context:**
SwiftUI, GTK, and WinUI have different design languages.

**Question:** Should UIs look identical or follow platform conventions?

**Example:**
- macOS: Top menu bar, aqua buttons
- Windows: Ribbon interface, Metro design
- Linux: Traditional menus, GNOME HIG

**Option A: Identical UI**
- Same layout, colors, spacing
- Users have consistent experience
- Branded "ChickenScratch look"

**Option B: Platform-native**
- macOS looks like macOS
- Windows looks like Windows
- Users feel at home on their OS

**Your earlier comment:** "Platform specifics are great! But they are gravy. The meat has to be the same everywhere."

**Interpretation:** Same features everywhere, but native look-and-feel?

**Question for clarification:** What does "meat" vs "gravy" mean?
- Meat = feature parity?
- Gravy = platform-specific UI polish?

---

### Q15: Distraction-Free Modes - Per-Platform or Unified?

**Context:**
Project spec includes fullscreen, typewriter scrolling, focus mode.

**Question:** Do these work identically on all platforms?

**Challenge:**
- macOS fullscreen (swipe to dedicated space)
- Windows fullscreen (different behavior)
- Linux fullscreen (window manager dependent)

**Should these modes feel identical or adapt to platform?**

---

## Questions About Your Requirements

### Q16: You Have No Rust/Swift/GTK/C# Experience

**Context:** You said "I have no experience with rust, swift, gtk, or dotnet"

**Question:** How hands-on do you want to be?

**Option A: Review-only**
- AI generates all code
- You test on MacBook
- You provide feedback
- AI iterates

**Option B: Learning mode**
- AI explains code in detail
- You modify code yourself
- AI reviews your changes
- Slower but you learn

**Which approach fits your goals?**

---

### Q17: "100% AI-coded"

**Context:** You mentioned "This is 100% AI coded"

**Question:** Does this mean:
- AI generates all code (you never edit)?
- AI writes first draft, you refine?
- Collaborative (AI + you both code)?

**Impacts:**
- How much documentation AI should write
- How detailed explanations need to be
- Testing strategy (manual vs automated)

---

### Q18: Revenue Model Impact on Architecture?

**Context:** You mentioned "until we have actual revenue" regarding cloud features.

**Question:** Should architecture optimize for:
- **Free/open-source** (maximize community adoption)?
- **Paid desktop app** (focus on polish, App Store)?
- **Future SaaS** (enable cloud later)?

**Impacts:**
- Build system (App Store signing, sandboxing)
- Update mechanism (built-in or external?)
- Analytics/telemetry (privacy-first or growth-focused?)

---

## Summary of Decisions Needed

**Critical (block work):**
1. ✅ FFI vs UniFFI? → Recommendation: UniFFI
2. ✅ JSON vs structured data? → Recommendation: Structured
3. ⏸️ Keep Tauri? → Recommendation: Defer decision
4. ✅ SwiftUI-first? → Recommendation: Yes

**Technical (can research):**
5. Swift Package vs Xcode?
6. Dynamic vs static linking?
7. Memory management?
8. Error handling?
9. Async/await?

**Architecture (design choices):**
10. Mono-repo? → Recommendation: Yes
11. Versioning strategy?
12. Testing balance?
13. CI/CD scope?

**UX/Design:**
14. UI consistency vs platform native?
15. Distraction-free mode platform adaptation?

**Clarifications:**
16. Review-only or learning mode?
17. Definition of "100% AI-coded"?
18. Revenue model impact?

---

## Next Steps

1. **Send these docs to AI models** (Claude, GPT-4, Gemini, etc.)
2. **Gather recommendations** on critical decisions
3. **Clarify requirements** (Q14-18)
4. **Make decisions** on critical path items
5. **Begin Phase 1 implementation** (extract core library)

---

**Note for AI model reviewers:**

Please provide:
- Recommendations on technical choices (FFI vs UniFFI, etc.)
- Best practices for Rust ↔ Swift/C# interop
- Potential pitfalls we haven't considered
- Alternative approaches we missed
- Answers to "What's the standard practice?" questions

Thank you!

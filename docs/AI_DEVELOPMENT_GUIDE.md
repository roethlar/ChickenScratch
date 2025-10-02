# Chicken Scratch - AI Development Guide

**Version:** 1.0
**Date:** 2025-10-01

---

## Purpose

This guide optimizes the Chicken Scratch codebase for development by AI assistants (Claude, GPT-4, etc.). It defines patterns, conventions, and strategies to maximize AI development velocity while maintaining code quality.

---

## 1. Core Principles

### 1.1 AI-Friendly Coding Standards

**Modularity:**
- **Max 500 lines per file** (strict limit)
- **Max 50 lines per function** (prefer 20-30)
- **Single Responsibility:** Each module/function does ONE thing

**Explicit Over Implicit:**
- Comprehensive type annotations (Rust types, TypeScript interfaces)
- Explicit error handling (no silent failures)
- Clear function names (verb + noun: `parseScrivenerXML`, not `parse`)

**Documentation-First:**
- Every module has a header comment explaining its purpose
- Complex logic has inline comments explaining WHY (not WHAT)
- Public APIs have docstrings with examples

### 1.2 Code Organization Strategy

**Flat Over Deep:**
```
✅ Good:
src/components/editor/Editor.tsx
src/components/editor/Toolbar.tsx
src/components/editor/Styles.tsx

❌ Avoid:
src/components/editor/ui/main/Editor.tsx
src/components/editor/ui/toolbar/Toolbar.tsx
```

**Co-location:**
- Keep related files together
- Tests next to source files: `editor.rs` + `editor_test.rs`
- Types with implementations, not in separate `types/` folder

---

## 2. Module Template

### 2.1 Rust Module Template

```rust
//! # Module Name
//!
//! Brief description of what this module does.
//!
//! ## Responsibilities
//! - Specific task 1
//! - Specific task 2
//!
//! ## Dependencies
//! - External crate usage
//! - Internal module dependencies
//!
//! ## Example
//! ```
//! use crate::module::function;
//! let result = function(input);
//! ```

use std::path::Path;
use serde::{Deserialize, Serialize};

/// Brief description of the struct.
///
/// Longer explanation if needed, including:
/// - Key fields and their purpose
/// - Important invariants
/// - Example usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyStruct {
    /// Description of field
    pub field1: String,

    /// Description of field
    pub field2: u32,
}

impl MyStruct {
    /// Brief description of what this function does.
    ///
    /// # Arguments
    /// * `param1` - Description of parameter
    /// * `param2` - Description of parameter
    ///
    /// # Returns
    /// Description of return value
    ///
    /// # Errors
    /// When this function fails and why
    ///
    /// # Example
    /// ```
    /// let result = MyStruct::new("value", 42);
    /// ```
    pub fn new(param1: &str, param2: u32) -> Result<Self, MyError> {
        // Implementation
        Ok(Self {
            field1: param1.to_string(),
            field2: param2,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_struct_creation() {
        let result = MyStruct::new("test", 10);
        assert!(result.is_ok());
    }
}
```

### 2.2 TypeScript Module Template

```typescript
/**
 * # Module Name
 *
 * Brief description of what this module does.
 *
 * ## Responsibilities
 * - Specific task 1
 * - Specific task 2
 *
 * @module components/editor
 */

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api';

/**
 * Props for MyComponent
 */
export interface MyComponentProps {
  /** Description of prop */
  prop1: string;

  /** Description of prop */
  prop2?: number;

  /** Callback description */
  onAction: (data: string) => void;
}

/**
 * Brief description of component.
 *
 * Longer explanation including:
 * - Key features
 * - Important behavior
 *
 * @example
 * ```tsx
 * <MyComponent prop1="value" onAction={handleAction} />
 * ```
 */
export const MyComponent: React.FC<MyComponentProps> = ({
  prop1,
  prop2 = 0,
  onAction
}) => {
  // State with clear variable names
  const [data, setData] = useState<string>('');

  // Effects with clear purpose comments
  useEffect(() => {
    // Load initial data from backend
    invoke<string>('get_data', { id: prop1 })
      .then(setData)
      .catch(console.error);
  }, [prop1]);

  return (
    <div>
      {/* Clear JSX structure */}
    </div>
  );
};
```

---

## 3. AI Development Workflow

### 3.1 Task Breakdown Pattern

**When AI receives a task:**

1. **Understand Scope:**
   - Read relevant documentation (this guide, architecture docs)
   - Identify affected modules
   - Estimate complexity (simple/moderate/complex)

2. **Plan Implementation:**
   - List specific files to create/modify
   - Identify dependencies and integration points
   - Determine test coverage needed

3. **Implement Incrementally:**
   - Start with data models (types/structs)
   - Implement core logic
   - Add tests
   - Integrate with existing code
   - Update documentation

4. **Validate:**
   - Run tests (unit, integration)
   - Check linting/formatting
   - Verify documentation completeness

### 3.2 Example: Implementing Scrivener Import

**Task:** "Implement .scriv import functionality"

**AI Breakdown:**
```
1. Understand Scope:
   - Read docs/ARCHITECTURE.md § 2.2 (Scrivener Compatibility)
   - Identify modules: src-tauri/core/scrivener/
   - Complexity: Complex (involves XML parsing, RTF conversion, metadata extraction)

2. Plan Implementation:
   Files to create:
   - src-tauri/core/scrivener/scrivx_parser.rs (parse .scrivx XML)
   - src-tauri/core/scrivener/rtf_handler.rs (read RTF files)
   - src-tauri/core/scrivener/importer.rs (orchestrate import)
   - src-tauri/api/import_commands.rs (Tauri command)

   Dependencies:
   - xml-rs for XML parsing
   - rtf-parser crate or custom implementation
   - serde for serialization

   Integration points:
   - src-tauri/core/project/writer.rs (save as .chikn)
   - Frontend: src/components/dialogs/ImportDialog.tsx

3. Implement:
   Step 1: Create scrivx_parser.rs
   Step 2: Implement rtf_handler.rs
   Step 3: Build importer.rs orchestration
   Step 4: Add Tauri command
   Step 5: Write tests for each module
   Step 6: Integrate with frontend

4. Validate:
   - Run: cargo test scrivener
   - Check: Sample .scriv imports successfully
   - Verify: Round-trip .scriv → .chikn → .scriv preserves data
```

---

## 4. Coding Patterns & Anti-Patterns

### 4.1 Rust Patterns

**✅ Good: Explicit Error Handling**
```rust
pub fn read_project(path: &Path) -> Result<Project, ProjectError> {
    let file = std::fs::read_to_string(path)
        .map_err(|e| ProjectError::FileRead(e.to_string()))?;

    let project: Project = serde_yaml::from_str(&file)
        .map_err(|e| ProjectError::ParseError(e.to_string()))?;

    Ok(project)
}
```

**❌ Bad: Silent Failures**
```rust
pub fn read_project(path: &Path) -> Option<Project> {
    let file = std::fs::read_to_string(path).ok()?;
    serde_yaml::from_str(&file).ok()
}
```

**✅ Good: Type-Safe APIs**
```rust
#[derive(Debug, Clone)]
pub struct DocumentId(String);

impl DocumentId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// Use DocumentId in APIs, not raw String
pub fn get_document(id: DocumentId) -> Result<Document, Error> {
    // ...
}
```

**❌ Bad: Primitive Obsession**
```rust
pub fn get_document(id: String) -> Result<Document, Error> {
    // Easy to pass wrong string type
}
```

### 4.2 TypeScript Patterns

**✅ Good: Strict Types**
```typescript
interface Document {
  id: string;
  name: string;
  content: string;
  metadata: DocumentMetadata;
}

interface DocumentMetadata {
  labelId: number;
  statusId: number;
  keywords: string[];
  synopsis: string;
}

// Use strict types in functions
async function loadDocument(id: string): Promise<Document> {
  return await invoke<Document>('load_document', { id });
}
```

**❌ Bad: Any Types**
```typescript
async function loadDocument(id: any): Promise<any> {
  return await invoke('load_document', { id });
}
```

**✅ Good: Custom Hooks**
```typescript
export function useDocument(id: string) {
  const [document, setDocument] = useState<Document | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    setLoading(true);
    invoke<Document>('load_document', { id })
      .then(setDocument)
      .catch(setError)
      .finally(() => setLoading(false));
  }, [id]);

  return { document, loading, error };
}

// Usage
const { document, loading, error } = useDocument(documentId);
```

**❌ Bad: Inline Logic**
```typescript
// Logic scattered across components
const MyComponent = () => {
  const [doc, setDoc] = useState(null);
  useEffect(() => {
    invoke('load_document').then(setDoc);
  }, []);
  // Duplicated in multiple components
}
```

---

## 5. Testing Guidelines for AI

### 5.1 Test Structure

**Unit Test Template (Rust):**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Test description: what is being tested and expected outcome
    #[test]
    fn test_function_name_happy_path() {
        // Arrange: Set up test data
        let input = create_test_input();

        // Act: Execute function
        let result = function_under_test(input);

        // Assert: Verify outcome
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value.field, expected_value);
    }

    #[test]
    fn test_function_name_error_case() {
        // Arrange: Set up invalid data
        let input = create_invalid_input();

        // Act & Assert: Verify error
        let result = function_under_test(input);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Expected error message"
        );
    }
}
```

**Component Test Template (TypeScript):**
```typescript
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { MyComponent } from './MyComponent';

describe('MyComponent', () => {
  it('renders with initial props', () => {
    // Arrange
    const mockCallback = vi.fn();

    // Act
    render(<MyComponent prop1="test" onAction={mockCallback} />);

    // Assert
    expect(screen.getByText('test')).toBeInTheDocument();
  });

  it('calls callback on user action', () => {
    // Arrange
    const mockCallback = vi.fn();
    render(<MyComponent prop1="test" onAction={mockCallback} />);

    // Act
    fireEvent.click(screen.getByRole('button'));

    // Assert
    expect(mockCallback).toHaveBeenCalledWith('expected-data');
  });
});
```

### 5.2 Test Coverage Targets

**Priority Levels:**
1. **Critical (90%+ coverage):** Data integrity (format conversions, Scrivener import/export)
2. **High (80%+ coverage):** Core logic (project management, git operations)
3. **Medium (60%+ coverage):** UI components (editor, navigator)
4. **Low (30%+ coverage):** UI chrome (dialogs, settings)

**Coverage Commands:**
```bash
# Rust backend
cargo tarpaulin --out Html --output-dir coverage/

# TypeScript frontend
npm run test:coverage
```

---

## 6. Error Handling for AI

### 6.1 Error Type Hierarchy

```rust
// src-tauri/utils/error.rs

#[derive(Debug, thiserror::Error)]
pub enum ChickenScratchError {
    #[error("Project error: {0}")]
    Project(#[from] ProjectError),

    #[error("Scrivener error: {0}")]
    Scrivener(#[from] ScrivenerError),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("AI error: {0}")]
    AI(#[from] AIError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ScrivenerError {
    #[error("Invalid .scrivx XML: {0}")]
    InvalidScrivx(String),

    #[error("RTF conversion failed: {0}")]
    RtfConversion(String),

    #[error("Missing required field: {0}")]
    MissingField(String),
}
```

### 6.2 Error Propagation Pattern

**Consistent Result Types:**
```rust
// All public functions return Result with specific error type
pub fn import_scrivener(path: &Path) -> Result<Project, ScrivenerError> {
    let scrivx = parse_scrivx(path)?; // Propagate ScrivenerError
    let documents = extract_documents(&scrivx)?;
    let project = create_project(documents)?;
    Ok(project)
}

// Convert to top-level error for Tauri commands
#[tauri::command]
pub async fn import_scrivener_command(
    path: String
) -> Result<Project, ChickenScratchError> {
    import_scrivener(Path::new(&path))
        .map_err(ChickenScratchError::Scrivener)
}
```

**Frontend Error Handling:**
```typescript
try {
  const project = await invoke<Project>('import_scrivener_command', { path });
  setProject(project);
} catch (error) {
  // Tauri errors come as strings
  const errorMessage = error as string;

  if (errorMessage.includes('Invalid .scrivx')) {
    showError('This Scrivener project appears corrupted. Please check the .scrivx file.');
  } else if (errorMessage.includes('RTF conversion')) {
    showError('Failed to convert Scrivener RTF files. Some formatting may be lost.');
  } else {
    showError(`Import failed: ${errorMessage}`);
  }
}
```

---

## 7. Git Workflow for AI Development

### 7.1 Commit Message Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Formatting, missing semi-colons, etc.
- `refactor`: Code change that neither fixes a bug nor adds a feature
- `test`: Adding missing tests
- `chore`: Updating build tasks, package manager configs, etc.

**Examples:**
```
feat(scrivener): implement .scriv import

- Parse .scrivx XML for project structure
- Convert RTF files to Pandoc Markdown
- Extract metadata (labels, status, keywords)
- Create .chikn project with full fidelity

Closes #123
```

```
fix(editor): resolve cursor position bug in focus mode

Cursor was jumping to start of line when entering focus mode.
Fixed by preserving cursorPosition state before mode transition.

Fixes #456
```

### 7.2 Branch Naming

**Pattern:** `<type>/<short-description>`

**Examples:**
- `feat/scrivener-import`
- `fix/editor-cursor-bug`
- `docs/api-reference`
- `refactor/project-module`

---

## 8. Documentation Maintenance

### 8.1 When to Update Docs

**Architecture Changes:**
- New module added → Update `docs/ARCHITECTURE.md` with module description
- API changed → Update API documentation
- Tech stack change → Update specification and architecture docs

**Feature Implementation:**
- New feature complete → Add to feature list in `docs/PROJECT_SPECIFICATION.md`
- User-facing change → Update user guide (when created)

**AI Development Notes:**
- Complex decision made → Create Architecture Decision Record (ADR)
- Tricky implementation → Add to this guide as a pattern/anti-pattern

### 8.2 Architecture Decision Records (ADRs)

**Template:** `docs/architecture/NNNN-title.md`

```markdown
# NNNN: Title of Decision

**Status:** Accepted | Rejected | Deprecated | Superseded

**Date:** 2025-01-15

## Context
What is the issue motivating this decision?

## Decision
What is the change we're proposing/doing?

## Consequences
What becomes easier or harder after this decision?

### Positive
- Benefit 1
- Benefit 2

### Negative
- Trade-off 1
- Trade-off 2

## Alternatives Considered
- Alternative 1: Why rejected
- Alternative 2: Why rejected
```

**Example:**
```markdown
# 0001: Use Tauri Instead of Electron

**Status:** Accepted

**Date:** 2025-10-01

## Context
Need a cross-platform framework for Chicken Scratch. Options: Electron, Tauri, Qt.
Priorities: AI development efficiency, native performance, cross-platform parity.

## Decision
Use Tauri 2.0 with Rust backend + React frontend.

## Consequences

### Positive
- Rust backend = type safety, performance, "cool factor"
- React frontend = massive AI training data, rapid UI iteration
- Smaller binary size than Electron
- Native performance for file operations

### Negative
- Tauri less mature than Electron (fewer community resources)
- Rust learning curve (mitigated by AI development)

## Alternatives Considered
- Electron: Larger binaries, more resource usage, but most mature
- Qt: Native performance, but harder for AI to write (less training data)
```

---

## 9. AI Context Management

### 9.1 Context Files for AI

**Essential Context (provide to AI at task start):**
1. `docs/PROJECT_SPECIFICATION.md` - What we're building
2. `docs/ARCHITECTURE.md` - How it's structured
3. `docs/AI_DEVELOPMENT_GUIDE.md` - This file (development patterns)
4. Relevant module README (e.g., `src-tauri/core/scrivener/README.md`)

**Incremental Context (provide as needed):**
- Specific module source code (when implementing/modifying)
- Related test files (when writing tests)
- ADRs for relevant decisions

### 9.2 Context Size Management

**Strategies to Stay Within Token Limits:**

1. **Focused Scope:**
   - Provide only files directly related to current task
   - Don't include entire codebase

2. **Summaries Over Full Content:**
   - For large files, provide summary + link to full content
   - Example: "This module handles X. See full code at src/module.rs"

3. **Incremental Updates:**
   - AI implements feature in steps
   - Validate each step before moving to next
   - Update context with new code as it's written

4. **Module Boundaries:**
   - Well-defined module boundaries = AI works on one module at a time
   - Minimal cross-module knowledge needed

---

## 10. Handoff Checklist

### 10.1 When AI Completes a Task

**Before marking complete, verify:**
- [ ] All planned files created/modified
- [ ] Tests written and passing
- [ ] Documentation updated (inline comments, module docs)
- [ ] Linting/formatting passes (`cargo fmt`, `npm run lint`)
- [ ] No compiler warnings or errors
- [ ] Integration points work (if applicable)
- [ ] Commit message follows format
- [ ] ADR created if architectural decision made

### 10.2 Handoff to Human Developer

**Provide summary including:**
1. What was implemented
2. Files changed (with brief description of changes)
3. Test coverage added
4. Known limitations or TODOs
5. Suggested next steps

**Example Handoff:**
```
Task Complete: Scrivener Import Feature

Files Changed:
- src-tauri/core/scrivener/scrivx_parser.rs (new) - Parse .scrivx XML
- src-tauri/core/scrivener/rtf_handler.rs (new) - RTF file reading
- src-tauri/core/scrivener/importer.rs (new) - Import orchestration
- src-tauri/api/import_commands.rs (new) - Tauri command
- src/components/dialogs/ImportDialog.tsx (new) - Import UI

Tests Added:
- 15 unit tests in scrivener module (87% coverage)
- 3 integration tests for full import flow
- Sample .scriv projects in tests/fixtures/

Known Limitations:
- Custom Scrivener metadata fields not yet supported (TODO)
- RTF images not handled (requires image extraction logic)

Suggested Next Steps:
1. Manual testing with diverse .scriv projects
2. Add support for custom metadata fields
3. Implement RTF image extraction
```

---

## 11. Common AI Development Scenarios

### 11.1 Adding a New Feature

**Steps:**
1. Read specification and architecture docs
2. Identify affected modules
3. Create new modules if needed (follow templates)
4. Implement backend logic (Rust)
5. Add Tauri commands for frontend communication
6. Implement frontend UI (React)
7. Write tests (unit + integration)
8. Update documentation
9. Create ADR if architectural change

### 11.2 Fixing a Bug

**Steps:**
1. Understand the bug (reproduce if possible)
2. Identify root cause (which module/function)
3. Write a failing test that demonstrates the bug
4. Fix the bug
5. Verify test now passes
6. Check for similar bugs elsewhere (pattern)
7. Update documentation if behavior changed

### 11.3 Refactoring Code

**Steps:**
1. Identify code smell or improvement opportunity
2. Write tests for existing behavior (if not already tested)
3. Refactor while ensuring tests pass
4. Verify no regressions (run full test suite)
5. Update documentation if API changed
6. Commit with clear refactor message

---

## 12. Quality Assurance

### 12.1 Pre-Commit Checklist

- [ ] Code compiles without warnings
- [ ] Tests pass (`cargo test`, `npm test`)
- [ ] Linting passes (`cargo clippy`, `npm run lint`)
- [ ] Formatting applied (`cargo fmt`, `npm run format`)
- [ ] Documentation updated (if needed)
- [ ] Commit message follows format

### 12.2 Code Review Criteria (for AI-generated code)

**Functionality:**
- Does it solve the stated problem?
- Are edge cases handled?
- Is error handling comprehensive?

**Quality:**
- Follows coding patterns in this guide?
- Module size within limits (<500 lines)?
- Functions concise (<50 lines)?
- Clear, self-documenting code?

**Testing:**
- Adequate test coverage?
- Tests are meaningful (not just passing)?
- Edge cases tested?

**Documentation:**
- Module purpose documented?
- Complex logic explained?
- Public APIs have docstrings?

---

## Conclusion

This guide equips AI developers with the patterns, templates, and workflows to build Chicken Scratch efficiently. By following these conventions, we ensure:

1. **Consistency:** All code follows the same patterns
2. **Maintainability:** Future AI (or human) developers can understand and modify code easily
3. **Quality:** Comprehensive testing and documentation
4. **Velocity:** AI can work autonomously with clear guidelines

**Remember:** This is a living document. Update it as new patterns emerge or better practices are discovered.

---

**Next Steps for AI:**
1. Read `docs/PROJECT_SPECIFICATION.md` to understand what we're building
2. Read `docs/ARCHITECTURE.md` to understand the system structure
3. Review relevant module documentation before implementing features
4. Follow templates and patterns in this guide
5. Update this guide when discovering new useful patterns

# Phase 1 Design: Foundation & Basic Editor

**Version:** 1.0
**Date:** 2025-10-01
**Status:** Design Phase
**Target:** Months 1-2

---

## 1. Phase 1 Overview

### 1.1 Goal
Build the foundational infrastructure and basic editing capabilities for Chicken Scratch.

### 1.2 Deliverables
- ✅ Tauri 2.0 application scaffold (Rust backend + React frontend)
- ✅ Basic TipTap editor integration with Markdown support
- ✅ `.chikn` format implementation (read/write)
- ✅ Simple document navigator (tree view, CRUD operations)
- ✅ Project creation/open/save workflows
- ✅ Basic Markdown editing with live preview toggle

### 1.3 Success Criteria
**Milestone:** User can:
1. Create a new `.chikn` project
2. Add folders and documents to the navigator
3. Write Markdown content in the editor
4. Toggle between edit and preview modes
5. Save changes to disk
6. Reopen the project with all content preserved

---

## 2. System Architecture (Phase 1)

### 2.1 Component Diagram

```
┌─────────────────────────────────────────────────────┐
│                 TAURI APPLICATION                    │
│                                                      │
│  ┌──────────────────────────────────────────────┐  │
│  │          FRONTEND (React + TypeScript)       │  │
│  │                                               │  │
│  │  ┌────────────┐         ┌─────────────────┐  │  │
│  │  │            │         │                 │  │  │
│  │  │ Navigator  │◄───────►│  Editor (MD)    │  │  │
│  │  │ Component  │         │  (TipTap)       │  │  │
│  │  │            │         │                 │  │  │
│  │  └────────────┘         └─────────────────┘  │  │
│  │       │                          │           │  │
│  │       └────────┬─────────────────┘           │  │
│  │                ▼                             │  │
│  │  ┌──────────────────────────────────────┐   │  │
│  │  │   Project Store (Zustand)            │   │  │
│  │  │   - currentProject                   │   │  │
│  │  │   - documents map                    │   │  │
│  │  │   - activeDocumentId                 │   │  │
│  │  └──────────────────────────────────────┘   │  │
│  │                ▼                             │  │
│  │  ┌──────────────────────────────────────┐   │  │
│  │  │   Tauri IPC Commands                 │   │  │
│  │  └──────────────────────────────────────┘   │  │
│  └───────────────────────┬──────────────────────┘  │
│                          ▼                         │
│  ┌──────────────────────────────────────────────┐  │
│  │         BACKEND (Rust)                       │  │
│  │                                               │  │
│  │  ┌──────────────┐    ┌──────────────────┐   │  │
│  │  │  Project     │    │  Document        │   │  │
│  │  │  Manager     │    │  Manager         │   │  │
│  │  │  (.chikn)    │    │                  │   │  │
│  │  └──────────────┘    └──────────────────┘   │  │
│  │                                               │  │
│  │  ┌──────────────────────────────────────┐   │  │
│  │  │  Data Models                         │   │  │
│  │  │  - Project, Document, Hierarchy      │   │  │
│  │  └──────────────────────────────────────┘   │  │
│  └───────────────────────────────────────────────┘  │
│                          │                         │
│                          ▼                         │
│                  ┌──────────────┐                  │
│                  │  File System │                  │
│                  │  .chikn/     │                  │
│                  └──────────────┘                  │
└──────────────────────────────────────────────────────┘
```

### 2.2 Data Flow (Phase 1)

**Project Creation Flow:**
```
User clicks "New Project"
        │
        ▼
Frontend: Show dialog (name, location)
        │
        ▼
User submits form
        │
        ▼
invoke('create_project', { name, path })
        │
        ▼
Backend: Create .chikn directory structure
        │
        ▼
Backend: Generate project.yaml
        │
        ▼
Return project path
        │
        ▼
Frontend: invoke('load_project', { path })
        │
        ▼
Update Zustand store with project data
        │
        ▼
UI renders Navigator + Editor
```

**Document Editing Flow:**
```
User types in Editor
        │
        ▼
TipTap onChange event
        │
        ▼
Update local Zustand store
        │
        ▼
Debounced save (500ms)
        │
        ▼
invoke('update_document', { id, content })
        │
        ▼
Backend: Write to .chikn/{path}.md
        │
        ▼
Return success
        │
        ▼
Update store timestamp
```

---

## 3. Rust Backend Design (Phase 1)

### 3.1 Module Structure (MVP)

```
src-tauri/
├── main.rs                 # Tauri entry point + builder
├── lib.rs                  # Re-export core types
│
├── core/
│   ├── mod.rs
│   ├── project/
│   │   ├── mod.rs
│   │   ├── format.rs       # .chikn spec constants
│   │   ├── reader.rs       # Read .chikn projects
│   │   ├── writer.rs       # Write .chikn projects
│   │   └── hierarchy.rs    # Document tree operations
│   │
│   └── document/
│       ├── mod.rs
│       └── manager.rs      # CRUD for documents
│
├── models/
│   ├── mod.rs
│   ├── project.rs          # Project struct
│   ├── document.rs         # Document struct
│   └── hierarchy.rs        # TreeNode enum
│
├── api/
│   ├── mod.rs
│   ├── project_commands.rs # create_project, load_project, save_project
│   └── document_commands.rs# create_document, update_document, delete_document
│
└── utils/
    ├── mod.rs
    ├── error.rs            # Error types
    └── fs.rs               # File system helpers
```

### 3.2 Data Models

**Project Model:**
```rust
// models/project.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Project unique ID
    pub id: String,

    /// Project name
    pub name: String,

    /// File system path to .chikn folder
    pub path: String,

    /// Document hierarchy (root nodes)
    pub hierarchy: Vec<TreeNode>,

    /// All documents by ID
    pub documents: HashMap<String, Document>,

    /// Project creation timestamp
    pub created: String,

    /// Last modified timestamp
    pub modified: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TreeNode {
    Folder {
        id: String,
        name: String,
        children: Vec<TreeNode>,
    },
    Document {
        id: String,
        name: String,
        path: String,  // Relative path within .chikn/
    },
}
```

**Document Model:**
```rust
// models/document.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Unique document ID
    pub id: String,

    /// Document name (e.g., "Chapter 01")
    pub name: String,

    /// File path relative to .chikn/ (e.g., "manuscript/ch01.md")
    pub path: String,

    /// Markdown content
    pub content: String,

    /// Parent ID (folder or root)
    pub parent_id: Option<String>,

    /// Creation timestamp
    pub created: String,

    /// Last modified timestamp
    pub modified: String,
}
```

### 3.3 Core API Commands

**Project Commands:**
```rust
// api/project_commands.rs

use crate::core::project::{reader, writer};
use crate::models::Project;
use crate::utils::error::ChiknError;

/// Create a new .chikn project
#[tauri::command]
pub async fn create_project(
    name: String,
    path: String
) -> Result<String, ChiknError> {
    // Generate project ID
    let id = uuid::Uuid::new_v4().to_string();

    // Create .chikn directory structure
    let project_path = format!("{}/{}.chikn", path, name);
    std::fs::create_dir_all(&project_path)?;
    std::fs::create_dir_all(format!("{}/manuscript", project_path))?;
    std::fs::create_dir_all(format!("{}/research", project_path))?;

    // Create initial project.yaml
    let project = Project {
        id: id.clone(),
        name: name.clone(),
        path: project_path.clone(),
        hierarchy: vec![
            TreeNode::Folder {
                id: "manuscript".to_string(),
                name: "Manuscript".to_string(),
                children: vec![],
            },
            TreeNode::Folder {
                id: "research".to_string(),
                name: "Research".to_string(),
                children: vec![],
            },
        ],
        documents: HashMap::new(),
        created: chrono::Utc::now().to_rfc3339(),
        modified: chrono::Utc::now().to_rfc3339(),
    };

    writer::write_project(&project)?;

    Ok(project_path)
}

/// Load an existing .chikn project
#[tauri::command]
pub async fn load_project(path: String) -> Result<Project, ChiknError> {
    reader::read_project(&path)
}

/// Save project metadata (hierarchy, settings)
#[tauri::command]
pub async fn save_project(project: Project) -> Result<(), ChiknError> {
    writer::write_project(&project)
}
```

**Document Commands:**
```rust
// api/document_commands.rs

use crate::models::{Document, TreeNode};
use crate::utils::error::ChiknError;

/// Create a new document
#[tauri::command]
pub async fn create_document(
    project_path: String,
    parent_id: String,
    name: String
) -> Result<Document, ChiknError> {
    // Generate document ID
    let id = uuid::Uuid::new_v4().to_string();

    // Determine file path based on parent
    let relative_path = if parent_id == "manuscript" {
        format!("manuscript/{}.md", slugify(&name))
    } else if parent_id == "research" {
        format!("research/{}.md", slugify(&name))
    } else {
        // For nested folders, build path from hierarchy
        format!("manuscript/{}.md", slugify(&name))
    };

    let document = Document {
        id: id.clone(),
        name: name.clone(),
        path: relative_path.clone(),
        content: String::new(),
        parent_id: Some(parent_id),
        created: chrono::Utc::now().to_rfc3339(),
        modified: chrono::Utc::now().to_rfc3339(),
    };

    // Write empty .md file
    let full_path = format!("{}/{}", project_path, relative_path);
    std::fs::write(full_path, "")?;

    Ok(document)
}

/// Update document content
#[tauri::command]
pub async fn update_document(
    project_path: String,
    id: String,
    content: String
) -> Result<(), ChiknError> {
    // Load project to get document path
    let project = reader::read_project(&project_path)?;

    let document = project.documents.get(&id)
        .ok_or_else(|| ChiknError::NotFound(format!("Document {}", id)))?;

    // Write content to .md file
    let full_path = format!("{}/{}", project_path, document.path);
    std::fs::write(full_path, content)?;

    Ok(())
}

/// Delete a document
#[tauri::command]
pub async fn delete_document(
    project_path: String,
    id: String
) -> Result<(), ChiknError> {
    let project = reader::read_project(&project_path)?;

    let document = project.documents.get(&id)
        .ok_or_else(|| ChiknError::NotFound(format!("Document {}", id)))?;

    // Delete .md file
    let full_path = format!("{}/{}", project_path, document.path);
    std::fs::remove_file(full_path)?;

    Ok(())
}
```

### 3.4 Error Handling (Phase 1)

```rust
// utils/error.rs

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChiknError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_yaml::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid project: {0}")]
    InvalidProject(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

// Convert to Tauri-compatible error
impl From<ChiknError> for tauri::Error {
    fn from(err: ChiknError) -> Self {
        tauri::Error::Anyhow(anyhow::Error::msg(err.to_string()))
    }
}
```

---

## 4. React Frontend Design (Phase 1)

### 4.1 Component Structure (MVP)

```
src/
├── App.tsx                 # Root component
├── main.tsx                # Entry point
│
├── components/
│   ├── layout/
│   │   ├── AppLayout.tsx   # Main layout (Navigator + Editor)
│   │   └── StatusBar.tsx   # Bottom status bar (word count, save status)
│   │
│   ├── navigator/
│   │   ├── Navigator.tsx       # Document tree container
│   │   ├── TreeView.tsx        # Recursive tree component
│   │   ├── TreeItem.tsx        # Individual tree node
│   │   └── TreeActions.tsx     # Context menu (new doc, delete, etc.)
│   │
│   └── editor/
│       ├── Editor.tsx          # TipTap editor wrapper
│       ├── EditorToolbar.tsx   # Formatting toolbar (basic MD)
│       └── PreviewToggle.tsx   # Toggle edit/preview mode
│
├── hooks/
│   ├── useProject.ts       # Project state + operations
│   ├── useDocument.ts      # Document CRUD operations
│   └── useAutoSave.ts      # Debounced save logic
│
├── store/
│   └── projectStore.ts     # Zustand store (project + documents)
│
├── types/
│   ├── project.ts          # TypeScript interfaces
│   └── document.ts
│
└── utils/
    ├── tauri.ts            # Tauri command wrappers
    └── slug.ts             # Slugify helper
```

### 4.2 Zustand Store Design

```typescript
// store/projectStore.ts

import { create } from 'zustand';
import { invoke } from '@tauri-apps/api';

export interface Project {
  id: string;
  name: string;
  path: string;
  hierarchy: TreeNode[];
  documents: Record<string, Document>;
  created: string;
  modified: string;
}

export type TreeNode =
  | { type: 'Folder'; id: string; name: string; children: TreeNode[] }
  | { type: 'Document'; id: string; name: string; path: string };

export interface Document {
  id: string;
  name: string;
  path: string;
  content: string;
  parent_id: string | null;
  created: string;
  modified: string;
}

interface ProjectState {
  // State
  currentProject: Project | null;
  activeDocumentId: string | null;
  isLoading: boolean;
  error: string | null;

  // Actions
  createProject: (name: string, path: string) => Promise<void>;
  loadProject: (path: string) => Promise<void>;
  saveProject: () => Promise<void>;

  createDocument: (parentId: string, name: string) => Promise<void>;
  updateDocument: (id: string, content: string) => Promise<void>;
  deleteDocument: (id: string) => Promise<void>;
  setActiveDocument: (id: string) => void;

  // Helpers
  getActiveDocument: () => Document | null;
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  // Initial state
  currentProject: null,
  activeDocumentId: null,
  isLoading: false,
  error: null,

  // Actions
  createProject: async (name: string, path: string) => {
    set({ isLoading: true, error: null });
    try {
      const projectPath = await invoke<string>('create_project', { name, path });
      const project = await invoke<Project>('load_project', { path: projectPath });
      set({ currentProject: project, isLoading: false });
    } catch (error) {
      set({ error: String(error), isLoading: false });
    }
  },

  loadProject: async (path: string) => {
    set({ isLoading: true, error: null });
    try {
      const project = await invoke<Project>('load_project', { path });
      set({ currentProject: project, isLoading: false });
    } catch (error) {
      set({ error: String(error), isLoading: false });
    }
  },

  saveProject: async () => {
    const { currentProject } = get();
    if (!currentProject) return;

    try {
      await invoke('save_project', { project: currentProject });
    } catch (error) {
      set({ error: String(error) });
    }
  },

  createDocument: async (parentId: string, name: string) => {
    const { currentProject } = get();
    if (!currentProject) return;

    set({ isLoading: true });
    try {
      const document = await invoke<Document>('create_document', {
        projectPath: currentProject.path,
        parentId,
        name,
      });

      // Update store with new document
      set((state) => ({
        currentProject: {
          ...state.currentProject!,
          documents: {
            ...state.currentProject!.documents,
            [document.id]: document,
          },
        },
        activeDocumentId: document.id,
        isLoading: false,
      }));
    } catch (error) {
      set({ error: String(error), isLoading: false });
    }
  },

  updateDocument: async (id: string, content: string) => {
    const { currentProject } = get();
    if (!currentProject) return;

    try {
      await invoke('update_document', {
        projectPath: currentProject.path,
        id,
        content,
      });

      // Update store
      set((state) => ({
        currentProject: {
          ...state.currentProject!,
          documents: {
            ...state.currentProject!.documents,
            [id]: {
              ...state.currentProject!.documents[id],
              content,
              modified: new Date().toISOString(),
            },
          },
        },
      }));
    } catch (error) {
      set({ error: String(error) });
    }
  },

  deleteDocument: async (id: string) => {
    const { currentProject } = get();
    if (!currentProject) return;

    try {
      await invoke('delete_document', {
        projectPath: currentProject.path,
        id,
      });

      // Remove from store
      set((state) => {
        const { [id]: removed, ...remaining } = state.currentProject!.documents;
        return {
          currentProject: {
            ...state.currentProject!,
            documents: remaining,
          },
          activeDocumentId: state.activeDocumentId === id ? null : state.activeDocumentId,
        };
      });
    } catch (error) {
      set({ error: String(error) });
    }
  },

  setActiveDocument: (id: string) => {
    set({ activeDocumentId: id });
  },

  getActiveDocument: () => {
    const { currentProject, activeDocumentId } = get();
    if (!currentProject || !activeDocumentId) return null;
    return currentProject.documents[activeDocumentId] || null;
  },
}));
```

### 4.3 Key React Components

**Editor Component:**
```typescript
// components/editor/Editor.tsx

import { useEditor, EditorContent } from '@tiptap/react';
import StarterKit from '@tiptap/starter-kit';
import Markdown from '@tiptap/extension-markdown';
import { useProjectStore } from '../../store/projectStore';
import { useAutoSave } from '../../hooks/useAutoSave';

export const Editor: React.FC = () => {
  const activeDocument = useProjectStore((state) => state.getActiveDocument());
  const updateDocument = useProjectStore((state) => state.updateDocument);

  const editor = useEditor({
    extensions: [
      StarterKit,
      Markdown,
    ],
    content: activeDocument?.content || '',
    onUpdate: ({ editor }) => {
      if (activeDocument) {
        const markdown = editor.storage.markdown.getMarkdown();
        updateDocument(activeDocument.id, markdown);
      }
    },
  });

  // Auto-save with 500ms debounce
  useAutoSave(activeDocument, editor);

  if (!activeDocument) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        Select a document to start editing
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      <EditorToolbar editor={editor} />
      <div className="flex-1 overflow-auto p-4">
        <EditorContent editor={editor} className="prose max-w-none" />
      </div>
    </div>
  );
};
```

**Navigator Component:**
```typescript
// components/navigator/Navigator.tsx

import { useState } from 'react';
import { useProjectStore } from '../../store/projectStore';
import { TreeView } from './TreeView';
import { TreeActions } from './TreeActions';

export const Navigator: React.FC = () => {
  const currentProject = useProjectStore((state) => state.currentProject);
  const createDocument = useProjectStore((state) => state.createDocument);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);

  if (!currentProject) {
    return (
      <div className="p-4 text-gray-500">
        No project loaded
      </div>
    );
  }

  const handleCreateDocument = async (parentId: string) => {
    const name = prompt('Document name:');
    if (name) {
      await createDocument(parentId, name);
    }
  };

  return (
    <div className="h-full flex flex-col border-r">
      <div className="p-2 border-b font-semibold">
        {currentProject.name}
      </div>

      <div className="flex-1 overflow-auto">
        <TreeView
          nodes={currentProject.hierarchy}
          selectedId={selectedNodeId}
          onSelect={setSelectedNodeId}
        />
      </div>

      {selectedNodeId && (
        <TreeActions
          nodeId={selectedNodeId}
          onCreateDocument={() => handleCreateDocument(selectedNodeId)}
        />
      )}
    </div>
  );
};
```

---

## 5. Testing Strategy (Phase 1)

### 5.1 Backend Tests

**Unit Tests:**
```rust
// core/project/reader_test.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_project_valid() {
        // Create temp .chikn folder
        let temp_dir = create_test_project();

        // Read project
        let result = read_project(temp_dir.path());
        assert!(result.is_ok());

        let project = result.unwrap();
        assert_eq!(project.name, "Test Project");
        assert_eq!(project.hierarchy.len(), 2); // Manuscript + Research
    }

    #[test]
    fn test_read_project_invalid_yaml() {
        let temp_dir = create_invalid_project();
        let result = read_project(temp_dir.path());
        assert!(result.is_err());
    }
}
```

**Integration Tests:**
```rust
// tests/project_workflow.rs

#[test]
fn test_full_project_workflow() {
    // Create project
    let project_path = create_project("My Novel", "/tmp").unwrap();

    // Load project
    let project = load_project(&project_path).unwrap();
    assert_eq!(project.name, "My Novel");

    // Create document
    let doc = create_document(&project_path, "manuscript", "Chapter 01").unwrap();
    assert_eq!(doc.name, "Chapter 01");

    // Update document
    update_document(&project_path, &doc.id, "# Chapter 01\n\nContent...").unwrap();

    // Reload and verify
    let reloaded = load_project(&project_path).unwrap();
    let doc = reloaded.documents.get(&doc.id).unwrap();
    assert!(doc.content.contains("Chapter 01"));
}
```

### 5.2 Frontend Tests

**Component Tests:**
```typescript
// components/editor/Editor.test.tsx

import { render, screen } from '@testing-library/react';
import { describe, it, expect, beforeEach } from 'vitest';
import { Editor } from './Editor';
import { useProjectStore } from '../../store/projectStore';

describe('Editor', () => {
  beforeEach(() => {
    useProjectStore.setState({
      currentProject: mockProject,
      activeDocumentId: 'doc1',
    });
  });

  it('renders editor when document is active', () => {
    render(<Editor />);
    expect(screen.getByRole('textbox')).toBeInTheDocument();
  });

  it('shows placeholder when no document selected', () => {
    useProjectStore.setState({ activeDocumentId: null });
    render(<Editor />);
    expect(screen.getByText(/Select a document/i)).toBeInTheDocument();
  });
});
```

### 5.3 E2E Tests (Playwright)

```typescript
// e2e/basic-workflow.spec.ts

import { test, expect } from '@playwright/test';

test('create project and add document', async ({ page }) => {
  await page.goto('/');

  // Create project
  await page.click('button:has-text("New Project")');
  await page.fill('input[name="projectName"]', 'Test Novel');
  await page.click('button:has-text("Create")');

  // Verify project loaded
  await expect(page.locator('text=Test Novel')).toBeVisible();

  // Create document
  await page.click('text=Manuscript');
  await page.click('button:has-text("New Document")');
  await page.fill('input[name="documentName"]', 'Chapter 01');
  await page.click('button:has-text("OK")');

  // Verify document created
  await expect(page.locator('text=Chapter 01')).toBeVisible();

  // Edit document
  await page.click('text=Chapter 01');
  await page.fill('[contenteditable]', '# Chapter 01\n\nOnce upon a time...');

  // Verify content saved
  await page.waitForTimeout(600); // Wait for debounce
  await page.reload();
  await expect(page.locator('text=Once upon a time')).toBeVisible();
});
```

---

## 6. Implementation Checklist

### 6.1 Week 1-2: Backend Foundation
- [ ] Set up Tauri 2.0 project structure
- [ ] Implement Project data model
- [ ] Implement Document data model
- [ ] Write project reader (load .chikn)
- [ ] Write project writer (save .chikn)
- [ ] Implement create_project command
- [ ] Implement load_project command
- [ ] Write unit tests for project operations

### 6.2 Week 3-4: Frontend Foundation
- [ ] Set up React + TypeScript + Vite
- [ ] Implement Zustand project store
- [ ] Build Navigator component (tree view)
- [ ] Integrate TipTap editor
- [ ] Implement basic Markdown support
- [ ] Add auto-save with debouncing
- [ ] Build StatusBar component
- [ ] Write component tests

### 6.3 Week 5-6: Integration & Polish
- [ ] Implement create_document command
- [ ] Implement update_document command
- [ ] Implement delete_document command
- [ ] Wire up Navigator ↔ Editor communication
- [ ] Add keyboard shortcuts (Ctrl+S, Ctrl+N)
- [ ] Implement live preview toggle
- [ ] Write E2E tests
- [ ] Performance optimization (virtual scrolling if needed)

### 6.4 Week 7-8: Testing & Bug Fixes
- [ ] Run full test suite (unit + integration + E2E)
- [ ] Manual testing on Windows, macOS, Linux
- [ ] Fix critical bugs
- [ ] Refine UX based on testing feedback
- [ ] Documentation updates
- [ ] Prepare demo for milestone validation

---

## 7. Open Questions & Decisions

### 7.1 Technical Decisions

**Q1: Should we use ProseMirror directly or TipTap?**
- **Decision:** TipTap (higher-level API, Markdown support built-in)
- **Rationale:** Faster development, AI-friendly React components

**Q2: How to handle large documents (10k+ lines)?**
- **Decision:** Virtual scrolling for tree view, lazy rendering for editor
- **Rationale:** Performance first, scale to professional manuscript sizes

**Q3: File watching for external edits?**
- **Decision:** Not in Phase 1, add in Phase 3
- **Rationale:** Keep MVP simple, focus on core editing workflow

### 7.2 UX Decisions

**Q1: Default layout: Navigator + Editor or Editor-only?**
- **Decision:** Navigator + Editor (two-pane) by default
- **Rationale:** Writers need to see project structure, but allow hiding later

**Q2: Auto-save interval?**
- **Decision:** 500ms debounce (aggressive auto-save)
- **Rationale:** Never lose work, writers expect this behavior

---

## 8. Success Metrics

### 8.1 Phase 1 Completion Criteria

**Functional:**
- [ ] User can create a .chikn project
- [ ] User can create folders and documents
- [ ] User can edit Markdown content
- [ ] User can toggle preview mode
- [ ] Changes persist across app restarts

**Quality:**
- [ ] 80%+ backend test coverage
- [ ] 60%+ frontend test coverage
- [ ] Zero crashes during basic workflows
- [ ] Sub-100ms UI response time for typing

**Performance:**
- [ ] Handles projects with 50 documents smoothly
- [ ] Editor scrolling at 60fps
- [ ] Auto-save completes in <50ms

---

## Next Steps

After Phase 1 completion:
1. **Demo & Validation:** Show working prototype to stakeholders
2. **Phase 2 Planning:** Begin Scrivener compatibility design
3. **Iterate:** Address feedback, refine UX
4. **Document:** Update architecture docs with learnings

**Phase 1 Target:** Months 1-2
**Phase 2 Start:** Month 3

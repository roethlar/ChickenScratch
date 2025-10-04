# Chicken Scratch - Tauri API Reference

Complete API documentation for frontend integration with Chicken Scratch backend.

## Project Commands

### `create_project(name, path)`

Creates a new .chikn project.

**Parameters:**
- `name: string` - Project name
- `path: string` - Parent directory path

**Returns:** `Promise<string>` - Full path to created project

**Example:**
```javascript
const projectPath = await invoke('create_project', {
  name: 'My Novel',
  path: '/Users/john/Documents'
});
// Returns: '/Users/john/Documents/My Novel.chikn'
```

---

### `load_project(path)`

Loads an existing .chikn project from disk.

**Parameters:**
- `path: string` - Full path to .chikn directory

**Returns:** `Promise<Project>` - Complete project with all documents

**Example:**
```javascript
const project = await invoke('load_project', {
  path: '/Users/john/Documents/My Novel.chikn'
});
```

---

### `save_project(project)`

Saves project metadata and hierarchy. Does NOT save document content - use `update_document()` for that.

**Parameters:**
- `project: Project` - Project with updated metadata

**Returns:** `Promise<void>`

**Example:**
```javascript
await invoke('save_project', { project });
```

---

### `add_to_hierarchy(project, node)`

Adds a document/folder to root level of hierarchy.

**Parameters:**
- `project: Project` - Current project
- `node: TreeNode` - Document or Folder to add

**Returns:** `Promise<Project>` - Updated project

**Example:**
```javascript
const newDoc = {
  type: 'Document',
  id: 'doc123',
  name: 'Chapter 1',
  path: 'manuscript/chapter-01.md'
};
const updatedProject = await invoke('add_to_hierarchy', {
  project,
  node: newDoc
});
```

---

### `add_to_folder(project, parentId, node)`

Adds a document as child of specific folder.

**Parameters:**
- `project: Project` - Current project
- `parentId: string` - ID of parent folder
- `node: TreeNode` - Node to add

**Returns:** `Promise<Project>` - Updated project

**Example:**
```javascript
const updatedProject = await invoke('add_to_folder', {
  project,
  parentId: 'folder123',
  node: newDocument
});
```

---

### `remove_from_hierarchy(project, nodeId)`

Removes a node from hierarchy.

**Parameters:**
- `project: Project` - Current project
- `nodeId: string` - ID of node to remove

**Returns:** `Promise<Project>` - Updated project

**Example:**
```javascript
const updatedProject = await invoke('remove_from_hierarchy', {
  project,
  nodeId: 'doc123'
});
```

---

### `move_node(project, nodeId, newParentId)`

Moves a node to new parent location.

**Parameters:**
- `project: Project` - Current project
- `nodeId: string` - ID of node to move
- `newParentId: string | null` - New parent ID (null for root)

**Returns:** `Promise<Project>` - Updated project

**Example:**
```javascript
// Move to root
await invoke('move_node', { project, nodeId: 'doc123', newParentId: null });

// Move to folder
await invoke('move_node', { project, nodeId: 'doc123', newParentId: 'folder456' });
```

---

### `reorder_node(project, nodeId, newIndex)`

Reorders node within current parent.

**Parameters:**
- `project: Project` - Current project
- `nodeId: string` - ID of node to reorder
- `newIndex: number` - New position (0-based)

**Returns:** `Promise<Project>` - Updated project

**Example:**
```javascript
const updatedProject = await invoke('reorder_node', {
  project,
  nodeId: 'doc123',
  newIndex: 1
});
```

## Document Commands

### `create_document(project, name, parentId)`

Creates a new document.

**Parameters:**
- `project: Project` - Current project
- `name: string` - Document name (will be slugified)
- `parentId: string | null` - Parent folder ID (optional)

**Returns:** `Promise<[Project, Document]>` - Updated project and new document

**Example:**
```javascript
const [updatedProject, newDoc] = await invoke('create_document', {
  project,
  name: 'Chapter 1',
  parentId: null
});
```

---

### `update_document(project, documentId, content)`

Updates document content.

**Parameters:**
- `project: Project` - Current project
- `documentId: string` - ID of document to update
- `content: string` - New Markdown content

**Returns:** `Promise<Project>` - Updated project

**Example:**
```javascript
const updatedProject = await invoke('update_document', {
  project,
  documentId: 'doc123',
  content: '# Chapter 1\n\nOnce upon a time...'
});
```

---

### `delete_document(project, documentId)`

Deletes a document.

**Parameters:**
- `project: Project` - Current project
- `documentId: string` - ID of document to delete

**Returns:** `Promise<Project>` - Updated project

**Example:**
```javascript
const updatedProject = await invoke('delete_document', {
  project,
  documentId: 'doc123'
});
```

---

### `get_document(project, documentId)`

Gets a specific document by ID.

**Parameters:**
- `project: Project` - Current project
- `documentId: string` - ID of document to retrieve

**Returns:** `Promise<Document>` - The requested document

**Example:**
```javascript
const doc = await invoke('get_document', {
  project,
  documentId: 'doc123'
});
console.log(doc.content);
```

## Type Definitions

### Project
```typescript
interface Project {
  id: string;
  name: string;
  path: string;
  hierarchy: TreeNode[];
  documents: Record<string, Document>;
  created: string; // RFC3339 timestamp
  modified: string; // RFC3339 timestamp
}
```

### TreeNode
```typescript
type TreeNode =
  | {
      type: 'Document';
      id: string;
      name: string;
      path: string;
    }
  | {
      type: 'Folder';
      id: string;
      name: string;
      children: TreeNode[];
    };
```

### Document
```typescript
interface Document {
  id: string;
  name: string;
  path: string;
  content: string;
  parentId: string | null;
  created: string; // RFC3339 timestamp
  modified: string; // RFC3339 timestamp
}
```

## Error Handling

All commands return `Result<T, ChiknError>`. Errors are serialized as strings.

**Example:**
```javascript
try {
  const project = await invoke('load_project', { path });
} catch (error) {
  console.error('Failed to load project:', error);
  // error is a string describing what went wrong
}
```

## Common Patterns

### Creating a New Project
```javascript
// 1. Create project
const projectPath = await invoke('create_project', {
  name: 'My Novel',
  path: '/Users/john/Documents'
});

// 2. Load it
const project = await invoke('load_project', { path: projectPath });

// 3. Add first document
const [updatedProject, firstDoc] = await invoke('create_document', {
  project,
  name: 'Chapter 1',
  parentId: null
});
```

### Editing a Document
```javascript
// 1. Get document
const doc = await invoke('get_document', { project, documentId: 'doc123' });

// 2. Modify content
const newContent = doc.content + '\n\nNew paragraph...';

// 3. Save
const updatedProject = await invoke('update_document', {
  project,
  documentId: 'doc123',
  content: newContent
});
```

### Reorganizing Hierarchy
```javascript
// Move document to folder
await invoke('move_node', {
  project,
  nodeId: 'doc123',
  newParentId: 'folder456'
});

// Reorder within folder
await invoke('reorder_node', {
  project,
  nodeId: 'doc123',
  newIndex: 0 // move to first position
});
```

## Notes

- All project operations automatically save to disk
- Document content updates modify the .md file immediately
- Timestamps are in RFC3339 format (e.g., "2025-01-01T12:00:00Z")
- Document names are slugified for filenames (e.g., "Chapter 1" → "chapter-1.md")
- Project hierarchy changes update project.yaml atomically

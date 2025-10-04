/**
 * TypeScript types matching Rust backend structures
 */

export type TreeNode =
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

export interface Document {
  id: string;
  name: string;
  path: string;
  content: string;
  parentId: string | null;
  created: string;
  modified: string;
}

export interface Project {
  id: string;
  name: string;
  path: string;
  hierarchy: TreeNode[];
  documents: Record<string, Document>;
  created: string;
  modified: string;
}

export interface SnapshotEntry {
  filename: string;
  created: string;
  description?: string;
  snapshotType: 'Automatic' | 'Manual' | 'BeforeOperation';
  sizeBytes: number;
}

export interface SnapshotManifest {
  snapshots: SnapshotEntry[];
}

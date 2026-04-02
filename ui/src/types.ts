export type TreeNode =
  | { type: "Document"; id: string; name: string; path: string }
  | { type: "Folder"; id: string; name: string; children: TreeNode[] };

export interface Document {
  id: string;
  name: string;
  path: string;
  content: string;
  parent_id: string | null;
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

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
  synopsis?: string | null;
  label?: string | null;
  status?: string | null;
  keywords?: string[] | null;
  links?: string[] | null;
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

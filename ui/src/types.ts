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
  include_in_compile: boolean;
  word_count_target: number;
  compile_order: number;
  comments?: Comment[];
  /**
   * v1.2 generic UI extensibility. Format-level field that preserves
   * arbitrary UI-layer entries. Key conventions are defined per-domain
   * (see docs/UI_CONVENTIONS_NOVELIST.md for novelist-mode keys).
   */
  fields?: Record<string, unknown>;
}

export interface Comment {
  id: string;
  body: string;
  resolved: boolean;
  created: string;
  modified: string;
}

export interface SessionTarget {
  words_per_session?: number | null;
  deadline?: string | null;
  total_target?: number | null;
}

export interface ProjectMeta {
  title?: string | null;
  author?: string | null;
  project_type?: string | null;
  genre?: string | null;
  theme?: string | null;
  summary?: string | null;
  session_target?: SessionTarget | null;
}

export interface Thread {
  id: string;
  name: string;
  color?: string | null;
  description?: string | null;
}

export interface Project {
  id: string;
  name: string;
  path: string;
  hierarchy: TreeNode[];
  documents: Record<string, Document>;
  created: string;
  modified: string;
  metadata: ProjectMeta;
  threads?: Thread[];
}

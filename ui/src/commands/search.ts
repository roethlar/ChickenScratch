import { invoke } from "@tauri-apps/api/core";

export interface SearchResult {
  doc_id: string;
  doc_name: string;
  snippet: string;
  match_count: number;
}

export async function searchProject(
  projectPath: string,
  query: string
): Promise<SearchResult[]> {
  return invoke("search_project", { projectPath, query });
}

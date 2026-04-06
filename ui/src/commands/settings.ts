import { invoke } from "@tauri-apps/api/core";

export interface RecentProject {
  name: string;
  path: string;
}

export async function getRecentProjects(): Promise<RecentProject[]> {
  return invoke("get_recent_projects");
}

export async function addRecentProject(
  name: string,
  path: string
): Promise<void> {
  return invoke("add_recent_project", { name, path });
}

export async function checkPandoc(): Promise<string> {
  return invoke("check_pandoc");
}

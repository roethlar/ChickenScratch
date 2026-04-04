import { invoke } from "@tauri-apps/api/core";
import type { Project } from "../types";

export async function createProject(
  name: string,
  path: string
): Promise<Project> {
  return invoke("create_project", { name, path });
}

export async function loadProject(path: string): Promise<Project> {
  return invoke("load_project", { path });
}

export async function saveProject(project: Project): Promise<Project> {
  return invoke("save_project", { project });
}

export async function importScrivener(
  scrivPath: string,
  outputPath: string
): Promise<Project> {
  return invoke("import_scrivener", {
    scrivPath,
    outputPath,
  });
}

export async function pickScrivFolder(): Promise<string | null> {
  return invoke("pick_scriv_folder");
}

import { invoke } from "@tauri-apps/api/core";
import { mutatingInvoke } from "./gateway";
import type { LeaseHandle } from "./barrier";
import type { LoadedProject, Project, SessionTarget } from "../types";

export async function createProject(
  name: string,
  path: string
): Promise<Project> {
  return mutatingInvoke("create_project", { name, path });
}

/**
 * Gated as project-mutating even though it reads: the backend `load_project`
 * acquires a WritePermit and self-heals missing standard folders, and it
 * refreshes the token cache. The barrier lifecycle passes its lease so the
 * mandated post-operation reload is owner-admitted (review round 8).
 */
export async function loadProject(
  path: string,
  lease?: LeaseHandle
): Promise<LoadedProject> {
  return mutatingInvoke("load_project", { path }, lease);
}

export async function saveProject(project: Project): Promise<Project> {
  return mutatingInvoke("save_project", { project });
}

export async function importScrivener(
  scrivPath: string,
  outputPath: string
): Promise<Project> {
  return mutatingInvoke("import_scrivener", {
    scrivPath,
    outputPath,
  });
}

export async function pickScrivFolder(): Promise<string | null> {
  return invoke("pick_scriv_folder");
}

/** Whole-project metadata update. Preview's Edit Details form dispatches
 *  through here (migrated off a component-level `invoke`, review round 7)
 *  so the barrier gate sees it. */
export async function updateProjectMetadata(args: {
  projectPath: string;
  title: string | null;
  author: string | null;
  projectType: string | null;
  genre: string | null;
  theme: string | null;
  summary: string | null;
  sessionTarget: SessionTarget | null;
}): Promise<Project> {
  return mutatingInvoke("update_project_metadata", { ...args });
}

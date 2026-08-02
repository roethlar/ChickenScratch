import { invoke } from "@tauri-apps/api/core";
import { mutatingInvoke } from "./gateway";
import type { Project, Thread } from "../types";

export async function listThreads(projectPath: string): Promise<Thread[]> {
  return invoke("list_threads", { projectPath });
}

export async function createThread(
  projectPath: string,
  name: string,
  color?: string,
  description?: string
): Promise<Project> {
  return mutatingInvoke("create_thread", {
    projectPath,
    name,
    color: color ?? null,
    description: description ?? null,
  });
}

export async function updateThread(
  projectPath: string,
  id: string,
  fields: { name?: string; color?: string | null; description?: string | null }
): Promise<Project> {
  return mutatingInvoke("update_thread", {
    projectPath,
    id,
    name: fields.name ?? null,
    color: fields.color === undefined ? null : fields.color,
    description: fields.description === undefined ? null : fields.description,
  });
}

export async function deleteThread(
  projectPath: string,
  id: string
): Promise<Project> {
  return mutatingInvoke("delete_thread", { projectPath, id });
}

export interface DanglingRef {
  doc_id: string;
  doc_name: string;
  field: string;
  missing_id: string;
}

export async function validateReferences(
  projectPath: string
): Promise<DanglingRef[]> {
  return invoke("validate_references", { projectPath });
}

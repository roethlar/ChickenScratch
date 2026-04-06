import { invoke } from "@tauri-apps/api/core";
import type { Project } from "../types";

export interface Template {
  id: string;
  name: string;
  content: string;
}

export async function listTemplates(): Promise<Template[]> {
  return invoke("list_templates");
}

export async function createFromTemplate(
  projectPath: string,
  templateId: string,
  name: string,
  parentId?: string
): Promise<Project> {
  return invoke("create_from_template", {
    projectPath,
    templateId,
    name,
    parentId: parentId ?? null,
  });
}

export async function saveAsTemplate(
  projectPath: string,
  docId: string
): Promise<Template> {
  return invoke("save_as_template", { projectPath, docId });
}

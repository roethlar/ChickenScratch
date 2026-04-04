import { invoke } from "@tauri-apps/api/core";
import type { Document, Project } from "../types";

export async function getDocument(
  projectPath: string,
  docId: string
): Promise<Document | null> {
  return invoke("get_document", { projectPath, docId });
}

export async function updateDocumentContent(
  projectPath: string,
  docId: string,
  content: string
): Promise<void> {
  return invoke("update_document_content", { projectPath, docId, content });
}

export async function createDocument(
  projectPath: string,
  name: string,
  parentId?: string
): Promise<Project> {
  return invoke("create_document", {
    projectPath,
    name,
    parentId: parentId ?? null,
  });
}

export async function createFolder(
  projectPath: string,
  name: string,
  parentId?: string
): Promise<Project> {
  return invoke("create_folder", {
    projectPath,
    name,
    parentId: parentId ?? null,
  });
}

export async function deleteNode(
  projectPath: string,
  nodeId: string
): Promise<Project> {
  return invoke("delete_node", { projectPath, nodeId });
}

export async function moveNode(
  projectPath: string,
  nodeId: string,
  newParentId?: string,
  newIndex?: number
): Promise<Project> {
  return invoke("move_node", {
    projectPath,
    nodeId,
    newParentId: newParentId ?? null,
    newIndex: newIndex ?? null,
  });
}

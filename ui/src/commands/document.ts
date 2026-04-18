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

export async function updateDocumentMetadata(
  projectPath: string,
  docId: string,
  meta: {
    synopsis?: string | null;
    label?: string | null;
    status?: string | null;
    keywords?: string[] | null;
    include_in_compile?: boolean | null;
    word_count_target?: number | null;
    compile_order?: number | null;
  }
): Promise<Project> {
  return invoke("update_document_metadata", {
    projectPath,
    docId,
    ...meta,
  });
}

export async function linkDocuments(
  projectPath: string,
  docIdA: string,
  docIdB: string
): Promise<Project> {
  return invoke("link_documents", { projectPath, docIdA, docIdB });
}

export async function renameNode(
  projectPath: string,
  nodeId: string,
  newName: string
): Promise<Project> {
  return invoke("rename_node", { projectPath, nodeId, newName });
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

export async function addComment(
  projectPath: string,
  docId: string,
  commentId: string,
  body: string,
  newContent: string
): Promise<Project> {
  return invoke("add_comment", { projectPath, docId, commentId, body, newContent });
}

export async function updateComment(
  projectPath: string,
  docId: string,
  commentId: string,
  body?: string,
  resolved?: boolean
): Promise<Project> {
  return invoke("update_comment", {
    projectPath,
    docId,
    commentId,
    body: body ?? null,
    resolved: resolved ?? null,
  });
}

export async function deleteComment(
  projectPath: string,
  docId: string,
  commentId: string,
  newContent: string
): Promise<Project> {
  return invoke("delete_comment", { projectPath, docId, commentId, newContent });
}

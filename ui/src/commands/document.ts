import { invoke } from "@tauri-apps/api/core";
import type { Document } from "../types";

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

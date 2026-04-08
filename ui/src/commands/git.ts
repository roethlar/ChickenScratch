import { invoke } from "@tauri-apps/api/core";

export interface Revision {
  id: string;
  message: string;
  timestamp: string;
  short_id: string;
}

export interface DraftVersion {
  name: string;
  is_active: boolean;
}

export async function saveRevision(
  projectPath: string,
  message: string
): Promise<Revision> {
  return invoke("save_revision", { projectPath, message });
}

export async function listRevisions(
  projectPath: string
): Promise<Revision[]> {
  return invoke("list_revisions", { projectPath });
}

export async function restoreRevision(
  projectPath: string,
  commitId: string
): Promise<Revision> {
  return invoke("restore_revision", { projectPath, commitId });
}

export async function createDraft(
  projectPath: string,
  name: string
): Promise<void> {
  return invoke("create_draft", { projectPath, name });
}

export async function listDrafts(
  projectPath: string
): Promise<DraftVersion[]> {
  return invoke("list_drafts", { projectPath });
}

export async function switchDraft(
  projectPath: string,
  name: string
): Promise<void> {
  return invoke("switch_draft", { projectPath, name });
}

export async function mergeDraft(
  projectPath: string,
  name: string
): Promise<void> {
  return invoke("merge_draft", { projectPath, name });
}

export async function pushBackup(
  projectPath: string,
  backupDir: string
): Promise<void> {
  return invoke("push_backup", { projectPath, backupDir });
}

export interface FileDiff {
  path: string;
  status: string;
}

export async function revisionDiff(
  projectPath: string,
  commitId: string
): Promise<FileDiff[]> {
  return invoke("revision_diff", { projectPath, commitId });
}

export async function wordDiff(
  projectPath: string,
  commitId: string,
  docPath: string
): Promise<[string, string][]> {
  return invoke("word_diff", { projectPath, commitId, docPath });
}

export async function hasChanges(
  projectPath: string
): Promise<boolean> {
  return invoke("has_changes", { projectPath });
}

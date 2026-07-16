import { invoke } from "@tauri-apps/api/core";
import { mutatingInvoke } from "./gateway";
import type { LeaseHandle } from "./barrier";

export interface Revision {
  id: string;
  message: string;
  timestamp: string;
  short_id: string;
  author: string;
}

export interface DraftVersion {
  name: string;
  is_active: boolean;
}

export async function saveRevision(
  projectPath: string,
  message: string
): Promise<Revision> {
  return mutatingInvoke("save_revision", { projectPath, message });
}

export async function listRevisions(
  projectPath: string
): Promise<Revision[]> {
  return invoke("list_revisions", { projectPath });
}

export async function restoreRevision(
  projectPath: string,
  commitId: string,
  lease?: LeaseHandle
): Promise<Revision> {
  return mutatingInvoke("restore_revision", { projectPath, commitId }, lease);
}

export async function createDraft(
  projectPath: string,
  name: string,
  lease?: LeaseHandle
): Promise<void> {
  return mutatingInvoke("create_draft", { projectPath, name }, lease);
}

export async function listDrafts(
  projectPath: string
): Promise<DraftVersion[]> {
  return invoke("list_drafts", { projectPath });
}

export async function switchDraft(
  projectPath: string,
  name: string,
  lease?: LeaseHandle
): Promise<void> {
  return mutatingInvoke("switch_draft", { projectPath, name }, lease);
}

/** Result of a draft merge. Same four cases as `PullResult` so the UI can
 *  reuse the conflict dialog. (F-009) */
export type MergeResult =
  | { kind: "up_to_date" }
  | { kind: "fast_forward" }
  | { kind: "merged" }
  | { kind: "conflicts"; files: string[] };

export async function mergeDraft(
  projectPath: string,
  name: string,
  lease?: LeaseHandle
): Promise<MergeResult> {
  return mutatingInvoke("merge_draft", { projectPath, name }, lease);
}

export async function pushBackup(
  projectPath: string,
  backupDir: string
): Promise<void> {
  return mutatingInvoke("push_backup", { projectPath, backupDir });
}

export async function manualBackup(
  projectPath: string,
  backupDir: string
): Promise<Revision | null> {
  return mutatingInvoke("manual_backup", { projectPath, backupDir });
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

export async function compareDrafts(
  projectPath: string,
  draftA: string,
  draftB: string
): Promise<FileDiff[]> {
  return invoke("compare_drafts", { projectPath, draftA, draftB });
}

export async function wordDiffDrafts(
  projectPath: string,
  draftA: string,
  draftB: string,
  docPath: string
): Promise<[string, string][]> {
  return invoke("word_diff_drafts", { projectPath, draftA, draftB, docPath });
}

export async function hasChanges(
  projectPath: string
): Promise<boolean> {
  return invoke("has_changes", { projectPath });
}

// ── Remote sync ──────────────────────────────────────────

export interface SyncStatus {
  ahead: number;
  behind: number;
  branch: string;
  has_remote: boolean;
}

export async function syncPush(projectPath: string): Promise<void> {
  return mutatingInvoke("sync_push", { projectPath });
}

export async function syncFetch(projectPath: string): Promise<void> {
  return mutatingInvoke("sync_fetch", { projectPath });
}

export async function syncStatus(projectPath: string): Promise<SyncStatus> {
  return invoke("sync_status", { projectPath });
}

export type PullResult =
  | { kind: "up_to_date" }
  | { kind: "fast_forward" }
  | { kind: "merged" }
  | { kind: "conflicts"; files: string[] };

export async function syncPull(
  projectPath: string,
  lease?: LeaseHandle
): Promise<PullResult> {
  return mutatingInvoke("sync_pull", { projectPath }, lease);
}

export async function syncAbortPull(
  projectPath: string,
  lease?: LeaseHandle
): Promise<void> {
  return mutatingInvoke("sync_abort_pull", { projectPath }, lease);
}

export async function syncPullForce(
  projectPath: string,
  lease?: LeaseHandle
): Promise<void> {
  return mutatingInvoke("sync_pull_force", { projectPath }, lease);
}

export async function documentHistory(
  projectPath: string,
  docPath: string
): Promise<Revision[]> {
  return invoke("document_history", { projectPath, docPath });
}

export async function restoreDocument(
  projectPath: string,
  docPath: string,
  commitId: string,
  lease?: LeaseHandle
): Promise<Revision> {
  return mutatingInvoke(
    "restore_document",
    { projectPath, docPath, commitId },
    lease
  );
}

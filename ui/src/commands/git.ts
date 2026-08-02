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

/** Outside-merge escape hatch only: fetches and hard-resets to the sync
 *  remote, and its own dirty/fidelity checks refuse every conflicted tree.
 *  Conflict recovery goes through `forceResolveMerge` instead (plan
 *  slice 4 — routing conflicts here was the unreachable-Force live bug). */
export async function syncPullForce(
  projectPath: string,
  lease?: LeaseHandle
): Promise<void> {
  return mutatingInvoke("sync_pull_force", { projectPath }, lease);
}

// ── Merge recovery (plan slice 4) ────────────────────────

/** Snapshot of the repository's merge state. The persistent
 *  merge-in-progress banner keys on this — it needs no fidelity probe and
 *  so still answers when conflicts touch format files. `attestation`
 *  opaquely names the specific merge AND local state; a force
 *  confirmation binds to it. */
export interface MergeState {
  in_progress: boolean;
  conflicted_files: string[];
  attestation: string | null;
}

/** Read-only query — no permit, no gate, answers even mid-conflict. */
export async function mergeState(projectPath: string): Promise<MergeState> {
  return invoke("merge_state", { projectPath });
}

/** Complete an in-progress merge after manual resolution: stages
 *  everything and commits with two parents. The caller must run this
 *  under the barrier lifecycle WITH the editor drain — the writer's
 *  just-resolved markers may still sit in the debounce window. */
export async function completeMerge(
  projectPath: string,
  message: string,
  lease?: LeaseHandle
): Promise<Revision> {
  return mutatingInvoke("complete_merge", { projectPath, message }, lease);
}

/** Resolve an in-progress merge by taking the incoming version
 *  (`MERGE_HEAD`) wholesale — works for both pull and draft-merge
 *  conflicts. Run with skipDrain: the buffer holds edits being discarded.
 *  `attestation` is the merge state the writer was shown when they
 *  confirmed the discard — the backend refuses if the live merge no
 *  longer matches it (finding s4-1). */
export async function forceResolveMerge(
  projectPath: string,
  attestation: string,
  lease?: LeaseHandle
): Promise<void> {
  return mutatingInvoke(
    "force_resolve_merge",
    { projectPath, attestation },
    lease
  );
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

import type { LeaseHandle } from "../../commands/barrier";
import * as gitCmd from "../../commands/git";
import type { Revision } from "../../commands/git";

/**
 * Deliver exactly what the restore confirmation promises (finding dp-2):
 * "Your current version is saved to History first — nothing is lost."
 *
 * Runs inside an epoch operation (editor already drained to disk): when
 * the worktree holds changes not yet in History, save them as a revision
 * strictly BEFORE restoring — the engine refuses to restore over a dirty
 * tree, so without this step the confirmed restore would just error. If
 * the save fails, the restore is not attempted.
 *
 * Injectable commands keep the ordering unit-testable without the panel.
 */
export const AUTO_SAVE_BEFORE_RESTORE_MESSAGE =
  "Saved automatically before bringing back an older version";

export interface RestoreCommands {
  hasChanges: (projectPath: string) => Promise<boolean>;
  saveRevision: (
    projectPath: string,
    message: string,
    lease?: LeaseHandle
  ) => Promise<Revision>;
  restoreRevision: (
    projectPath: string,
    commitId: string,
    lease?: LeaseHandle
  ) => Promise<Revision>;
}

export async function restoreWithHistorySave(
  projectPath: string,
  commitId: string,
  lease: LeaseHandle,
  cmds: RestoreCommands = gitCmd
): Promise<Revision> {
  if (await cmds.hasChanges(projectPath)) {
    await cmds.saveRevision(projectPath, AUTO_SAVE_BEFORE_RESTORE_MESSAGE, lease);
  }
  return cmds.restoreRevision(projectPath, commitId, lease);
}

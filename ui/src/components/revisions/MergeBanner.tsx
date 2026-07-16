import { GitMerge } from "lucide-react";
import { dialogConfirm, dialogPrompt } from "../shared/Dialog";
import { toastError, toastSuccess } from "../shared/Toast";
import * as gitCmd from "../../commands/git";
import type { MergeState } from "../../commands/git";
import { runEpochOperation } from "../../operations";
import { useBarrierActive } from "../../hooks/useBarrier";

/**
 * Persistent merge-in-progress banner (plan slice 4, review rounds 6–9):
 * "Resolve manually" is no longer a dead end — while merge state exists
 * the writer always has Complete (the ONLY path that can mint the merge
 * commit; every automatic writer refuses mid-merge) and Abort. Complete
 * runs the full barrier lifecycle WITH the editor drain, so markers the
 * writer just resolved cannot be left in the debounce window while the
 * commit snapshots the stale disk state; Abort skips the drain — the
 * buffer holds edits being discarded.
 */
export function MergeBanner({
  projectPath,
  state,
  onChanged,
}: {
  projectPath: string;
  state: MergeState;
  onChanged: () => void;
}) {
  const barrierActive = useBarrierActive();

  const handleComplete = async () => {
    const message = await dialogPrompt(
      "Describe this merge for the revision history:",
      "Merged incoming changes"
    );
    if (!message?.trim()) return;
    try {
      await runEpochOperation((lease) =>
        gitCmd.completeMerge(projectPath, message.trim(), lease)
      );
      toastSuccess("Merge completed.");
    } catch (e) {
      toastError(`Complete failed: ${e}`);
    }
    onChanged();
  };

  const handleAbort = async () => {
    if (
      !(await dialogConfirm(
        "Abort the merge and go back to your version from before it? The incoming changes will be discarded."
      ))
    )
      return;
    try {
      // skipDrain: the buffer may hold conflict markers or partial
      // resolutions the writer chose to throw away.
      await runEpochOperation(
        (lease) => gitCmd.syncAbortPull(projectPath, lease),
        { skipDrain: true }
      );
      toastSuccess("Merge aborted; your version restored.");
    } catch (e) {
      toastError(`Abort failed: ${e}`);
    }
    onChanged();
  };

  const conflictCount = state.conflicted_files.length;
  return (
    <div className="merge-banner">
      <GitMerge size={14} />
      <span className="merge-banner-text">
        A merge is in progress
        {conflictCount > 0 &&
          ` — ${conflictCount} file${conflictCount === 1 ? "" : "s"} still` +
            " showing conflict markers"}
        . Edit the marked files to keep what you want, then complete the
        merge — or abort to go back.
      </span>
      <div className="merge-banner-actions">
        <button onClick={handleComplete} disabled={barrierActive}>
          Complete merge
        </button>
        <button onClick={handleAbort} disabled={barrierActive}>
          Abort merge
        </button>
      </div>
    </div>
  );
}

import { Combine } from "lucide-react";
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
      "Describe this combined version for the revision history:",
      "Combined incoming changes"
    );
    if (!message?.trim()) return;
    try {
      await runEpochOperation((lease) =>
        gitCmd.completeMerge(projectPath, message.trim(), lease)
      );
      toastSuccess("Versions combined.");
    } catch (e) {
      toastError(`Couldn't finish combining. (${e})`);
    }
    onChanged();
  };

  const handleAbort = async () => {
    if (
      !(await dialogConfirm(
        "Go back to your version from before the combine? The incoming changes will be discarded."
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
      toastSuccess("Your version is back.");
    } catch (e) {
      toastError(`Couldn't go back. (${e})`);
    }
    onChanged();
  };

  const conflictCount = state.conflicted_files.length;
  return (
    <div className="merge-banner">
      <Combine size={14} />
      <span className="merge-banner-text">
        Two versions are being combined
        {conflictCount > 0 &&
          ` — ${conflictCount} file${conflictCount === 1 ? "" : "s"} still` +
            " showing both versions"}
        . Edit the marked files to keep what you want, then finish
        combining — or go back to your version.
      </span>
      <div className="merge-banner-actions">
        <button onClick={handleComplete} disabled={barrierActive}>
          Finish combining
        </button>
        <button onClick={handleAbort} disabled={barrierActive}>
          Go back to my version
        </button>
      </div>
    </div>
  );
}

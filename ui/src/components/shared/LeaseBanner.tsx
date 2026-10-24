import { useEffect, useState } from "react";
import { Users } from "lucide-react";
import { LEASE_CONFLICT_EVENT, type LeaseConflictDetail } from "../../commands/transport";
import { takeOverProject } from "../../commands/web";
import { flushPendingEditorSave } from "../editor/editorRef";
import { toastError, toastSuccess } from "./Toast";
import { projectPathOf } from "../../utils/projectPathOf";

/**
 * Browser shell only: shown when the server refused a write because another
 * tab or device holds this project. Taking over makes this tab the writer and
 * re-sends the pending editor save.
 */
export function LeaseBanner() {
  const [projectPath, setProjectPath] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    const onConflict = (event: Event) => {
      const detail = (event as CustomEvent<LeaseConflictDetail>).detail;
      const path = projectPathOf(detail?.args);
      if (path) setProjectPath(path);
    };
    window.addEventListener(LEASE_CONFLICT_EVENT, onConflict);
    return () => window.removeEventListener(LEASE_CONFLICT_EVENT, onConflict);
  }, []);

  if (!projectPath) return null;

  const takeOver = async () => {
    setBusy(true);
    try {
      await takeOverProject(projectPath);
      setProjectPath(null);
      try {
        await flushPendingEditorSave();
      } catch {
        /* the next autosave retries */
      }
      toastSuccess("This tab is now the writer for this project.");
    } catch (e) {
      toastError(`Could not take over: ${e}`);
    }
    setBusy(false);
  };

  return (
    <div className="lease-banner" role="alert">
      <Users size={16} />
      <span className="lease-banner-text">
        This project is open in another browser tab or device. Your changes here are not being
        saved until you take over.
      </span>
      <button className="dialog-btn ok" onClick={takeOver} disabled={busy}>
        Take over
      </button>
      <button className="dialog-btn cancel" onClick={() => setProjectPath(null)} disabled={busy}>
        Dismiss
      </button>
    </div>
  );
}

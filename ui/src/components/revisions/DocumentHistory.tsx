import { useState, useEffect, useCallback, useId, useRef } from "react";
import * as gitCmd from "../../commands/git";
import type { Revision } from "../../commands/git";
import { useProjectStore } from "../../stores/projectStore";
import { dialogConfirm, useModalFocusTrap } from "../shared/Dialog";
import { toastSuccess, toastError } from "../shared/Toast";
import { X, RotateCcw } from "lucide-react";
import { flushPendingEditorSave, setCurrentEditorMarkdown } from "../editor/editorRef";

interface Props {
  open: boolean;
  docId: string | null;
  onClose: () => void;
}

export function DocumentHistory({ open, docId, onClose }: Props) {
  const project = useProjectStore((s) => s.project);
  const doc = docId && project ? project.documents[docId] : null;
  const [revisions, setRevisions] = useState<Revision[]>([]);
  const [busy, setBusy] = useState(false);
  const titleId = useId();
  const closeButtonRef = useRef<HTMLButtonElement>(null);
  const { dialogRef, onDialogKeyDown } = useModalFocusTrap<HTMLDivElement>(
    open && !!doc,
    onClose,
    closeButtonRef
  );

  // Loading revisions from git is an external-system query — an effect is
  // the right boundary, and the synchronous setRevisions([]) clears stale
  // results from the previous doc before the async fetch lands.
  useEffect(() => {
    if (!open || !project || !doc) return;
    let cancelled = false;
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setRevisions([]);
    (async () => {
      try {
        await flushPendingEditorSave();
      } catch (e) {
        if (!cancelled) {
          setRevisions([]);
          toastError(`File history aborted — editor save failed: ${e}`);
        }
        return;
      }

      try {
        const r = await gitCmd.documentHistory(project.path, doc.path);
        if (!cancelled) setRevisions(r);
      } catch (e) {
        if (!cancelled) {
          setRevisions([]);
          toastError(`Failed to load document history: ${e}`);
        }
      }
    })();
    return () => { cancelled = true; };
  }, [open, project, doc]);

  const handleRestore = useCallback(
    async (rev: Revision) => {
      if (!project || !doc) return;
      const short = rev.id.slice(0, 8);
      if (!(await dialogConfirm(
        `Restore "${doc.name}" to its state at ${short}? A new revision will record the restore.`
      ))) return;
      setBusy(true);
      try {
        await flushPendingEditorSave();
        await gitCmd.restoreDocument(project.path, doc.path, rev.id);
        // Reload the project and explicitly replace the editor buffer for
        // same-doc restores; selecting the same id won't re-run Editor's load
        // effect, so relying on selection can leave stale text queued to save.
        const Project = await import("../../commands/project");
        const reloaded = await Project.loadProject(project.path);
        useProjectStore.getState().setProject(reloaded);
        const restoredDoc = reloaded.documents[doc.id];
        if (useProjectStore.getState().activeDocId === doc.id && restoredDoc) {
          setCurrentEditorMarkdown(restoredDoc.content || "");
        }
        toastSuccess("Document restored.");
        onClose();
      } catch (e) {
        toastError(`Restore failed: ${e}`);
      }
      setBusy(false);
    },
    [project, doc, onClose]
  );

  if (!open || !doc) return null;

  return (
    <div className="doc-history-overlay" onClick={onClose}>
      <div
        ref={dialogRef}
        className="doc-history-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={onDialogKeyDown}
      >
        <div className="doc-history-header">
          <span className="doc-history-title" id={titleId}>File History — {doc.name}</span>
          <button
            ref={closeButtonRef}
            className="doc-history-close"
            onClick={onClose}
            aria-label="Close file history dialog"
          >
            <X size={16} />
          </button>
        </div>
        {revisions.length === 0 ? (
          <div className="doc-history-empty">
            No commits touch this file yet. Save a revision after editing to
            create one.
          </div>
        ) : (
          <div className="doc-history-list">
            {revisions.map((rev) => (
              <div key={rev.id} className="doc-history-item">
                <div className="doc-history-info">
                  <div className="doc-history-msg">{rev.message}</div>
                  <div className="doc-history-meta">
                    <code>{rev.id.slice(0, 8)}</code>
                    <span>{new Date(rev.timestamp).toLocaleString()}</span>
                    <span>{rev.author}</span>
                  </div>
                </div>
                <button
                  className="doc-history-restore"
                  disabled={busy}
                  onClick={() => handleRestore(rev)}
                  title="Restore this version"
                >
                  <RotateCcw size={12} /> Restore
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

import { useState, useEffect, useId, useRef } from "react";
import { X, Eye, ArrowRightLeft } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import * as gitCmd from "../../commands/git";
import type { DraftDocument } from "../../commands/git";
import { useModalFocusTrap } from "../shared/Dialog";

interface DraftPeekProps {
  /** Name of the draft being peeked at, or null when closed. */
  draft: string | null;
  /** True when the peeked draft is the one already open in the editor. */
  isActive: boolean;
  onClose: () => void;
  /** "Switch to this draft" — delegates to the existing switch flow. */
  onSwitch: (name: string) => void;
}

/**
 * Read-only look inside another draft (PLAN_DRAFT_PEEK.md, slice 2).
 *
 * Strictly a viewer: the only commands it invokes are the two read-only
 * peek reads. Nothing here can change the current draft — the banner says
 * so, and the vitest coverage pins it.
 */
export function DraftPeek({ draft, isActive, onClose, onSwitch }: DraftPeekProps) {
  const project = useProjectStore((s) => s.project);
  const [docs, setDocs] = useState<DraftDocument[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [text, setText] = useState<string | null>(null);
  const [loadError, setLoadError] = useState(false);
  const titleId = useId();
  const closeButtonRef = useRef<HTMLButtonElement>(null);
  const open = draft !== null;
  const { dialogRef, onDialogKeyDown } = useModalFocusTrap<HTMLDivElement>(
    open,
    onClose,
    closeButtonRef
  );

  // Loading the document list from git when a draft is chosen — an effect is
  // the right boundary for reacting to (draft, project) changes.
  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setDocs([]);
    setSelected(null);
    setText(null);
    setLoadError(false);
    if (!draft || !project) return;
    let cancelled = false;
    gitCmd
      .listDraftDocuments(project.path, draft)
      .then((d) => {
        if (cancelled) return;
        setDocs(d);
        if (d.length > 0) setSelected(d[0].path);
      })
      .catch(() => {
        if (!cancelled) setLoadError(true);
      });
    return () => {
      cancelled = true;
    };
  }, [draft, project]);

  // Fetch the selected document's text (read-only) whenever it changes.
  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setText(null);
    if (!draft || !project || !selected) return;
    let cancelled = false;
    gitCmd
      .readDraftDocument(project.path, draft, selected)
      .then((t) => {
        if (!cancelled) setText(t);
      })
      .catch(() => {
        if (!cancelled) setText("Couldn't read this document.");
      });
    return () => {
      cancelled = true;
    };
  }, [draft, project, selected]);

  if (!open) return null;

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div
        ref={dialogRef}
        className="draft-compare-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={onDialogKeyDown}
      >
        <div className="draft-compare-header">
          <Eye size={14} />
          <span id={titleId}>Looking inside “{draft}”</span>
          <div style={{ flex: 1 }} />
          <button
            ref={closeButtonRef}
            onClick={onClose}
            className="draft-compare-close"
            aria-label="Close draft peek"
          >
            <X size={14} />
          </button>
        </div>

        <div className="draft-peek-banner">
          Just looking — nothing in your current draft changes.
        </div>

        <div className="draft-compare-body">
          <div className="draft-compare-filelist">
            <div className="draft-compare-filelist-title">
              {loadError
                ? "Couldn't open this draft"
                : docs.length === 0
                  ? "No documents"
                  : `${docs.length} document${docs.length === 1 ? "" : "s"}`}
            </div>
            {docs.map((d) => (
              <button
                key={d.path}
                className={`draft-compare-file ${selected === d.path ? "active" : ""}`}
                onClick={() => setSelected(d.path)}
              >
                {d.name}
              </button>
            ))}
          </div>

          <div className="draft-compare-content">
            {selected === null ? (
              <div className="draft-compare-hint">
                Pick a document to read it
              </div>
            ) : text === null ? (
              <div className="draft-compare-hint">Opening…</div>
            ) : (
              <div className="draft-peek-text">{text}</div>
            )}
          </div>
        </div>

        <div className="draft-peek-footer">
          <button className="draft-peek-close-btn" onClick={onClose}>
            Close
          </button>
          {!isActive && (
            <button
              className="draft-peek-switch-btn"
              onClick={() => draft && onSwitch(draft)}
            >
              <ArrowRightLeft size={12} /> Switch to this draft
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

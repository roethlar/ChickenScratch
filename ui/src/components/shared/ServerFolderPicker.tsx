import { useEffect, useId, useRef, useState } from "react";
import { ChevronRight, CornerLeftUp, Folder, X } from "lucide-react";
import { listServerDirs, type DirListing } from "../../commands/web";
import { useModalFocusTrap } from "./Dialog";
import {
  currentFolderPick,
  settleFolderPick,
  subscribeFolderPick,
  type FolderPickRequest,
} from "./folderPickerRequest";

/**
 * The web shell's stand-in for a native folder dialog: browse folders under
 * the server's projects root — or its backup folder, when the server has one
 * — and choose one. Callers `await pickServerFolder()` (see
 * `folderPickerRequest.ts`); this host, mounted once in `main.tsx`, renders
 * the pending request.
 */
export function ServerFolderPickerHost() {
  const [request, setRequest] = useState<FolderPickRequest | null>(currentFolderPick());
  useEffect(() => subscribeFolderPick(() => setRequest(currentFolderPick())), []);
  if (!request) return null;
  return (
    <ServerFolderPicker
      title={request.title}
      suffix={request.suffix}
      startRoot={request.startRoot}
      onPick={(path) => settleFolderPick(path)}
      onCancel={() => settleFolderPick(null)}
    />
  );
}

interface ServerFolderPickerProps {
  title: string;
  suffix?: string;
  /** Open in the server root with this label ("Backups") if it exists. */
  startRoot?: string;
  onPick: (path: string) => void;
  onCancel: () => void;
}

export function ServerFolderPicker({
  title,
  suffix,
  startRoot,
  onPick,
  onCancel,
}: ServerFolderPickerProps) {
  const [listing, setListing] = useState<DirListing | null>(null);
  const [target, setTarget] = useState<string>("");
  const [error, setError] = useState<string | null>(null);
  // The requested start root is applied once, after the first listing tells
  // us which roots exist; browsing afterwards is the user's.
  const [startApplied, setStartApplied] = useState(!startRoot);
  const titleId = useId();
  const firstRef = useRef<HTMLButtonElement>(null);
  const { dialogRef, onDialogKeyDown } = useModalFocusTrap<HTMLDivElement>(true, onCancel, firstRef);

  useEffect(() => {
    let cancelled = false;
    listServerDirs(target)
      .then((result) => {
        if (cancelled) return;
        if (!startApplied) {
          setStartApplied(true);
          const wanted = result.roots.find((r) => r.label === startRoot);
          if (wanted && wanted.path !== result.path) {
            setTarget(wanted.path);
            return;
          }
        }
        setListing(result);
        setError(null);
      })
      .catch((e) => {
        if (!cancelled) setError(String(e));
      });
    return () => {
      cancelled = true;
    };
  }, [target, startApplied, startRoot]);

  const roots = listing?.roots ?? [];
  const currentRoot = roots.find((r) => listing?.path === r.path || listing?.path.startsWith(r.path + "/"));

  const choosable =
    !!listing && (!suffix || listing.path.toLowerCase().endsWith(suffix.toLowerCase()));

  return (
    <div className="dialog-overlay" onClick={onCancel}>
      <div
        ref={dialogRef}
        className="server-folder-picker"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={onDialogKeyDown}
      >
        <div className="server-folder-picker-header">
          <h3 id={titleId}>{title}</h3>
          <button onClick={onCancel} className="compile-close" aria-label="Close folder picker">
            <X size={16} />
          </button>
        </div>
        {roots.length > 1 && (
          <div className="server-folder-picker-roots" role="tablist" aria-label="Browse in">
            {roots.map((root) => (
              <button
                key={root.path}
                role="tab"
                aria-selected={currentRoot?.path === root.path}
                className={
                  "server-folder-picker-root" + (currentRoot?.path === root.path ? " active" : "")
                }
                onClick={() => setTarget(root.path)}
                title={root.path}
              >
                {root.label}
              </button>
            ))}
          </div>
        )}
        <div className="server-folder-picker-path" title={listing?.path ?? ""}>
          {listing ? listing.path : "Loading…"}
        </div>
        {error && <div className="welcome-error">{error}</div>}
        <div className="server-folder-picker-list" role="list">
          {listing?.parent && (
            <button
              ref={firstRef}
              className="server-folder-picker-item"
              onClick={() => setTarget(listing.parent ?? "")}
              role="listitem"
            >
              <CornerLeftUp size={14} /> <span>Up one folder</span>
            </button>
          )}
          {listing && listing.dirs.length === 0 && !listing.parent && (
            <p className="welcome-recent-empty">No folders here yet.</p>
          )}
          {listing?.dirs.map((dir) => (
            <button
              key={dir.path}
              className="server-folder-picker-item"
              onClick={() => setTarget(dir.path)}
              role="listitem"
              title={dir.path}
            >
              <Folder size={14} /> <span>{dir.name}</span> <ChevronRight size={14} />
            </button>
          ))}
        </div>
        <div className="compile-footer">
          <button className="dialog-btn cancel" onClick={onCancel}>Cancel</button>
          <button
            className="dialog-btn ok"
            disabled={!choosable}
            onClick={() => listing && onPick(listing.path)}
            title={suffix && !choosable ? `Choose a folder ending in ${suffix}` : undefined}
          >
            Choose this folder
          </button>
        </div>
      </div>
    </div>
  );
}

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
 * the server's projects root and choose one. Callers `await
 * pickServerFolder()` (see `folderPickerRequest.ts`); this host, mounted once
 * in `main.tsx`, renders the pending request.
 */
export function ServerFolderPickerHost() {
  const [request, setRequest] = useState<FolderPickRequest | null>(currentFolderPick());
  useEffect(() => subscribeFolderPick(() => setRequest(currentFolderPick())), []);
  if (!request) return null;
  return (
    <ServerFolderPicker
      title={request.title}
      suffix={request.suffix}
      onPick={(path) => settleFolderPick(path)}
      onCancel={() => settleFolderPick(null)}
    />
  );
}

interface ServerFolderPickerProps {
  title: string;
  suffix?: string;
  onPick: (path: string) => void;
  onCancel: () => void;
}

export function ServerFolderPicker({ title, suffix, onPick, onCancel }: ServerFolderPickerProps) {
  const [listing, setListing] = useState<DirListing | null>(null);
  const [target, setTarget] = useState<string>("");
  const [error, setError] = useState<string | null>(null);
  const titleId = useId();
  const firstRef = useRef<HTMLButtonElement>(null);
  const { dialogRef, onDialogKeyDown } = useModalFocusTrap<HTMLDivElement>(true, onCancel, firstRef);

  useEffect(() => {
    let cancelled = false;
    listServerDirs(target)
      .then((result) => {
        if (cancelled) return;
        setListing(result);
        setError(null);
      })
      .catch((e) => {
        if (!cancelled) setError(String(e));
      });
    return () => {
      cancelled = true;
    };
  }, [target]);

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

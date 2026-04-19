import { useState, useEffect, useCallback } from "react";
import { X, GitCompare, ArrowRightLeft } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import * as gitCmd from "../../commands/git";
import type { FileDiff, DraftVersion } from "../../commands/git";

interface DraftCompareProps {
  open: boolean;
  onClose: () => void;
}

export function DraftCompare({ open, onClose }: DraftCompareProps) {
  const project = useProjectStore((s) => s.project);
  const [drafts, setDrafts] = useState<DraftVersion[]>([]);
  const [draftA, setDraftA] = useState<string>("");
  const [draftB, setDraftB] = useState<string>("");
  const [files, setFiles] = useState<FileDiff[]>([]);
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [wordDiff, setWordDiff] = useState<[string, string][] | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!open || !project) return;
    gitCmd.listDrafts(project.path).then((d) => {
      setDrafts(d);
      const active = d.find((x) => x.is_active)?.name || d[0]?.name || "main";
      const other = d.find((x) => x.name !== active)?.name || active;
      setDraftA(other);
      setDraftB(active);
    }).catch(() => {});
  }, [open, project]);

  const runCompare = useCallback(async () => {
    if (!project || !draftA || !draftB || draftA === draftB) {
      setFiles([]);
      return;
    }
    setLoading(true);
    try {
      const f = await gitCmd.compareDrafts(project.path, draftA, draftB);
      setFiles(f);
      setSelectedFile(null);
      setWordDiff(null);
    } catch {
      setFiles([]);
    }
    setLoading(false);
  }, [project, draftA, draftB]);

  useEffect(() => {
    if (open) runCompare();
  }, [open, draftA, draftB, runCompare]);

  const selectFile = useCallback(async (path: string) => {
    if (!project) return;
    setSelectedFile(path);
    try {
      const wd = await gitCmd.wordDiffDrafts(project.path, draftA, draftB, path);
      setWordDiff(wd);
    } catch {
      setWordDiff(null);
    }
  }, [project, draftA, draftB]);

  if (!open) return null;

  const swap = () => {
    const tmp = draftA;
    setDraftA(draftB);
    setDraftB(tmp);
  };

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="draft-compare-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="draft-compare-header">
          <GitCompare size={14} />
          <span>Compare Drafts</span>
          <div style={{ flex: 1 }} />
          <button onClick={onClose} className="draft-compare-close">
            <X size={14} />
          </button>
        </div>

        <div className="draft-compare-picker">
          <select value={draftA} onChange={(e) => setDraftA(e.target.value)}>
            {drafts.map((d) => (
              <option key={d.name} value={d.name}>{d.name}{d.is_active ? " (current)" : ""}</option>
            ))}
          </select>
          <button onClick={swap} title="Swap sides" className="draft-compare-swap">
            <ArrowRightLeft size={14} />
          </button>
          <select value={draftB} onChange={(e) => setDraftB(e.target.value)}>
            {drafts.map((d) => (
              <option key={d.name} value={d.name}>{d.name}{d.is_active ? " (current)" : ""}</option>
            ))}
          </select>
        </div>

        <div className="draft-compare-body">
          <div className="draft-compare-filelist">
            <div className="draft-compare-filelist-title">
              {loading ? "Comparing..." : files.length === 0 ? "No differences" : `${files.length} changed`}
            </div>
            {files.map((f) => (
              <button
                key={f.path}
                className={`draft-compare-file diff-${f.status} ${selectedFile === f.path ? "active" : ""}`}
                onClick={() => selectFile(f.path)}
              >
                <span className="diff-badge">{f.status[0].toUpperCase()}</span>
                {f.path}
              </button>
            ))}
          </div>

          <div className="draft-compare-content">
            {!selectedFile && files.length > 0 && (
              <div className="draft-compare-hint">Select a file to see changes</div>
            )}
            {selectedFile && wordDiff && (
              <div className="word-diff-view">
                {wordDiff.map(([kind, text], i) => (
                  <span key={i} className={`word-diff-${kind}`}>{text} </span>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

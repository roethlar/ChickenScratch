import { useState, useEffect, useCallback } from "react";
import { dialogPrompt, dialogConfirm } from "../shared/Dialog";
import {
  Save,
  History,
  RotateCcw,
  GitBranch,
  GitMerge,
  HardDrive,
  Cloud,
  Upload,
  Download,
} from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { toastSuccess, toastError } from "../shared/Toast";
import * as gitCmd from "../../commands/git";
import type { Revision, DraftVersion, FileDiff, SyncStatus } from "../../commands/git";

export function Revisions() {
  const project = useProjectStore((s) => s.project);
  const [revisions, setRevisions] = useState<Revision[]>([]);
  const [drafts, setDrafts] = useState<DraftVersion[]>([]);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState("");
  const [tab, setTab] = useState<"history" | "drafts">("history");
  const [diffId, setDiffId] = useState<string | null>(null);
  const [diffFiles, setDiffFiles] = useState<FileDiff[]>([]);
  const [wordDiffData, setWordDiffData] = useState<[string, string][] | null>(null);
  const [wordDiffFile, setWordDiffFile] = useState<string | null>(null);
  const [syncStatus, setSyncStatus] = useState<SyncStatus | null>(null);
  const [syncBusy, setSyncBusy] = useState(false);

  const refresh = useCallback(async () => {
    if (!project) return;
    const [revs, drs, st] = await Promise.all([
      gitCmd.listRevisions(project.path),
      gitCmd.listDrafts(project.path),
      gitCmd.syncStatus(project.path).catch(() => null),
    ]);
    setRevisions(revs);
    setDrafts(drs);
    setSyncStatus(st);
  }, [project]);

  // Loading revisions and drafts from an external system (git) — an effect is
  // the correct boundary for this async side-effect that sets multiple states.
  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    refresh();
  }, [refresh]);

  const handleSave = async () => {
    if (!project || !message.trim()) return;
    setSaving(true);
    try {
      await gitCmd.saveRevision(project.path, message.trim());
      setMessage("");
      toastSuccess("Revision saved.");
      await refresh();
    } catch (e) {
      toastError(`Save failed: ${e}`);
    }
    setSaving(false);
  };

  const handleRestore = async (commitId: string) => {
    if (!project) return;
    if (!(await dialogConfirm("Restore to this revision? Your current work will be preserved as a new revision.")))
      return;
    await gitCmd.restoreRevision(project.path, commitId);
    // Reload the project to reflect restored state
    await useProjectStore.getState().openProject(project.path);
    await refresh();
  };

  const handleNewDraft = async () => {
    if (!project) return;
    const name = await dialogPrompt("Draft version name:");
    if (!name) return;
    await gitCmd.createDraft(project.path, name);
    await useProjectStore.getState().openProject(project.path);
    await refresh();
  };

  const handleSwitchDraft = async (name: string) => {
    if (!project) return;
    await gitCmd.switchDraft(project.path, name);
    await useProjectStore.getState().openProject(project.path);
    await refresh();
  };

  const handleMergeDraft = async (name: string) => {
    if (!project) return;
    if (!(await dialogConfirm(`Merge "${name}" into the current draft?`))) return;
    await gitCmd.mergeDraft(project.path, name);
    await useProjectStore.getState().openProject(project.path);
    await refresh();
  };

  const handleBackup = async () => {
    if (!project) return;
    const dir = await dialogPrompt(
      "Backup directory:",
      localStorage.getItem("chickenscratch-backup-dir") || ""
    );
    if (!dir) return;
    localStorage.setItem("chickenscratch-backup-dir", dir);
    try {
      await gitCmd.pushBackup(project.path, dir);
      toastSuccess("Backup complete.");
    } catch (e) {
      toastError(`Backup failed: ${e}`);
    }
  };

  const handlePush = async () => {
    if (!project) return;
    setSyncBusy(true);
    try {
      await gitCmd.syncPush(project.path);
      toastSuccess("Pushed to remote.");
      await refresh();
    } catch (e) {
      toastError(`Push failed: ${e}`);
    }
    setSyncBusy(false);
  };

  const handleFetch = async () => {
    if (!project) return;
    setSyncBusy(true);
    try {
      await gitCmd.syncFetch(project.path);
      toastSuccess("Fetched from remote.");
      await refresh();
    } catch (e) {
      toastError(`Fetch failed: ${e}`);
    }
    setSyncBusy(false);
  };

  if (!project) return null;

  const activeDraft = drafts.find((d) => d.is_active)?.name || "main";

  return (
    <div className="revisions">
      <div className="revisions-header">
        <span>Revisions</span>
        <span className="revisions-branch">{activeDraft}</span>
      </div>

      <div className="revisions-save">
        <input
          value={message}
          onChange={(e) => setMessage(e.target.value)}
          placeholder="Describe this revision..."
          onKeyDown={(e) => e.key === "Enter" && handleSave()}
        />
        <button onClick={handleSave} disabled={saving || !message.trim()}>
          <Save size={14} />
        </button>
      </div>

      <div className="revisions-tabs">
        <button
          className={tab === "history" ? "active" : ""}
          onClick={() => setTab("history")}
        >
          <History size={14} /> History
        </button>
        <button
          className={tab === "drafts" ? "active" : ""}
          onClick={() => setTab("drafts")}
        >
          <GitBranch size={14} /> Drafts
        </button>
      </div>

      <div className="revisions-body">
        {tab === "history" && (
          <div className="revisions-list">
            {revisions.map((rev) => (
              <div key={rev.id}>
                <div className="revision-item">
                  <div
                    className="revision-info revision-clickable"
                    onClick={async () => {
                      if (diffId === rev.id) { setDiffId(null); setDiffFiles([]); return; }
                      setDiffId(rev.id);
                      try {
                        const files = await gitCmd.revisionDiff(project!.path, rev.id);
                        setDiffFiles(files);
                      } catch { setDiffFiles([]); }
                    }}
                    title="Click to see changes"
                  >
                    <span className="revision-msg">{rev.message}</span>
                    <span className="revision-time">
                      {new Date(rev.timestamp).toLocaleDateString(undefined, {
                        month: "short",
                        day: "numeric",
                        hour: "numeric",
                        minute: "2-digit",
                      })}
                    </span>
                  </div>
                  <button
                    className="revision-restore"
                    onClick={() => handleRestore(rev.id)}
                    title="Restore to this revision"
                  >
                    <RotateCcw size={12} />
                  </button>
                </div>
                {diffId === rev.id && diffFiles.length > 0 && (
                  <div className="revision-diff">
                    {diffFiles.map((f, i) => (
                      <button
                        key={i}
                        className={`revision-diff-file diff-${f.status} ${wordDiffFile === f.path ? "active" : ""}`}
                        onClick={async () => {
                          if (wordDiffFile === f.path) {
                            setWordDiffFile(null);
                            setWordDiffData(null);
                            return;
                          }
                          setWordDiffFile(f.path);
                          try {
                            const data = await gitCmd.wordDiff(project!.path, rev.id, f.path);
                            setWordDiffData(data);
                          } catch { setWordDiffData(null); }
                        }}
                      >
                        <span className="diff-badge">{f.status[0].toUpperCase()}</span>
                        {f.path}
                      </button>
                    ))}
                    {wordDiffData && wordDiffFile && (
                      <div className="word-diff-view">
                        {wordDiffData.map(([kind, text], i) => (
                          <span key={i} className={`word-diff-${kind}`}>{text} </span>
                        ))}
                      </div>
                    )}
                  </div>
                )}
              </div>
            ))}
            {revisions.length === 0 && (
              <div className="revisions-empty">No revisions yet</div>
            )}
          </div>
        )}

        {tab === "drafts" && (
          <div className="revisions-list">
            {drafts.map((draft) => (
              <div
                key={draft.name}
                className={`revision-item ${draft.is_active ? "active-draft" : ""}`}
              >
                <div className="revision-info">
                  <span className="revision-msg">
                    {draft.name}
                    {draft.is_active && " (active)"}
                  </span>
                </div>
                {!draft.is_active && (
                  <div className="draft-actions">
                    <button
                      onClick={() => handleSwitchDraft(draft.name)}
                      title="Switch to this draft"
                    >
                      <GitBranch size={12} />
                    </button>
                    <button
                      onClick={() => handleMergeDraft(draft.name)}
                      title="Merge into current draft"
                    >
                      <GitMerge size={12} />
                    </button>
                  </div>
                )}
              </div>
            ))}
            <button className="drafts-new-btn" onClick={handleNewDraft}>
              <GitBranch size={14} /> New Draft Version
            </button>
          </div>
        )}
      </div>

      <div className="revisions-footer">
        <button className="revisions-backup-btn" onClick={handleBackup}>
          <HardDrive size={14} /> Backup
        </button>
        <SyncControls
          status={syncStatus}
          busy={syncBusy}
          onPush={handlePush}
          onFetch={handleFetch}
        />
      </div>
    </div>
  );
}

function SyncControls({
  status,
  busy,
  onPush,
  onFetch,
}: {
  status: SyncStatus | null;
  busy: boolean;
  onPush: () => void;
  onFetch: () => void;
}) {
  const enabled = !!status?.has_remote;
  const summary = (() => {
    if (!enabled) return "Configure a remote in Settings › Remote";
    if (status!.ahead === 0 && status!.behind === 0) return "Up to date with remote";
    const parts: string[] = [];
    if (status!.ahead) parts.push(`${status!.ahead} to push`);
    if (status!.behind) parts.push(`${status!.behind} to pull`);
    return parts.join(" · ");
  })();

  return (
    <div className="revisions-sync">
      <div className="revisions-sync-summary">
        <Cloud size={12} />
        <span>{summary}</span>
      </div>
      <div className="revisions-sync-actions">
        <button onClick={onFetch} disabled={!enabled || busy} title="Fetch from remote">
          <Download size={12} /> Fetch
        </button>
        <button onClick={onPush} disabled={!enabled || busy} title="Push to remote">
          <Upload size={12} /> Push
        </button>
      </div>
    </div>
  );
}

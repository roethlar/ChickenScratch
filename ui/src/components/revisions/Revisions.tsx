import { useState, useEffect, useCallback } from "react";
import { dialogPrompt, dialogConfirm } from "../shared/Dialog";
import {
  Save,
  History,
  RotateCcw,
  GitBranch,
  GitMerge,
  HardDrive,
} from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { toastSuccess, toastError } from "../shared/Toast";
import * as gitCmd from "../../commands/git";
import type { Revision, DraftVersion, FileDiff } from "../../commands/git";

export function Revisions() {
  const project = useProjectStore((s) => s.project);
  const [revisions, setRevisions] = useState<Revision[]>([]);
  const [drafts, setDrafts] = useState<DraftVersion[]>([]);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState("");
  const [tab, setTab] = useState<"history" | "drafts">("history");
  const [diffId, setDiffId] = useState<string | null>(null);
  const [diffFiles, setDiffFiles] = useState<FileDiff[]>([]);

  const refresh = useCallback(async () => {
    if (!project) return;
    const [revs, drs] = await Promise.all([
      gitCmd.listRevisions(project.path),
      gitCmd.listDrafts(project.path),
    ]);
    setRevisions(revs);
    setDrafts(drs);
  }, [project]);

  useEffect(() => {
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
                      <div key={i} className={`revision-diff-file diff-${f.status}`}>
                        <span className="diff-badge">{f.status[0].toUpperCase()}</span>
                        {f.path}
                      </div>
                    ))}
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
      </div>
    </div>
  );
}

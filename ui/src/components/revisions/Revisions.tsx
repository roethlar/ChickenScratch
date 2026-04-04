import { useState, useEffect, useCallback } from "react";
import {
  Save,
  History,
  RotateCcw,
  GitBranch,
  GitMerge,
  HardDrive,
} from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import * as gitCmd from "../../commands/git";
import type { Revision, DraftVersion } from "../../commands/git";

export function Revisions() {
  const project = useProjectStore((s) => s.project);
  const [revisions, setRevisions] = useState<Revision[]>([]);
  const [drafts, setDrafts] = useState<DraftVersion[]>([]);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState("");
  const [tab, setTab] = useState<"history" | "drafts">("history");

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
      await refresh();
    } catch (e) {
      alert(`Save failed: ${e}`);
    }
    setSaving(false);
  };

  const handleRestore = async (commitId: string) => {
    if (!project) return;
    if (!confirm("Restore to this revision? Your current work will be preserved as a new revision."))
      return;
    await gitCmd.restoreRevision(project.path, commitId);
    // Reload the project to reflect restored state
    await useProjectStore.getState().openProject(project.path);
    await refresh();
  };

  const handleNewDraft = async () => {
    if (!project) return;
    const name = prompt("Draft version name:");
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
    if (!confirm(`Merge "${name}" into the current draft?`)) return;
    await gitCmd.mergeDraft(project.path, name);
    await useProjectStore.getState().openProject(project.path);
    await refresh();
  };

  const handleBackup = async () => {
    if (!project) return;
    const dir = prompt(
      "Backup directory:",
      localStorage.getItem("chickenscratch-backup-dir") || ""
    );
    if (!dir) return;
    localStorage.setItem("chickenscratch-backup-dir", dir);
    try {
      await gitCmd.pushBackup(project.path, dir);
      alert("Backup complete.");
    } catch (e) {
      alert(`Backup failed: ${e}`);
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
              <div key={rev.id} className="revision-item">
                <div className="revision-info">
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

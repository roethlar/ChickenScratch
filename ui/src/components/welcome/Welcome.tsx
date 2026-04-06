import { useState, useEffect } from "react";
import { FolderOpen, FilePlus, FileDown, Clock, AlertTriangle } from "lucide-react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useProjectStore } from "../../stores/projectStore";
import { pickScrivFolder } from "../../commands/project";
import {
  getRecentProjects,
  checkPandoc,
  type RecentProject,
} from "../../commands/settings";

export function Welcome() {
  const openProject = useProjectStore((s) => s.openProject);
  const createProject = useProjectStore((s) => s.createProject);
  const importScrivener = useProjectStore((s) => s.importScrivener);
  const error = useProjectStore((s) => s.error);
  const clearError = useProjectStore((s) => s.clearError);
  const [busy, setBusy] = useState(false);
  const [recent, setRecent] = useState<RecentProject[]>([]);
  const [pandocWarning, setPandocWarning] = useState<string | null>(null);

  useEffect(() => {
    getRecentProjects().then(setRecent).catch(() => {});
    checkPandoc().catch(() => {
      setPandocWarning(
        "Pandoc is not installed. Scrivener import and manuscript export will not work. Install from pandoc.org."
      );
    });
  }, []);

  const handleOpen = async () => {
    const selected = await open({
      directory: true,
      title: "Open .chikn Project",
    });
    if (selected) {
      setBusy(true);
      await openProject(selected);
      setBusy(false);
    }
  };

  const handleCreate = async () => {
    const dir = await save({
      title: "Create New Project",
      defaultPath: "Untitled.chikn",
    });
    if (dir) {
      setBusy(true);
      const name = dir.split("/").pop()?.replace(".chikn", "") || "Untitled";
      const parent = dir.substring(0, dir.lastIndexOf("/"));
      await createProject(name, parent);
      setBusy(false);
    }
  };

  const handleImport = async () => {
    const scrivPath = await pickScrivFolder();
    if (!scrivPath) return;

    const defaultName =
      scrivPath.split("/").pop()?.replace(".scriv", ".chikn") || "Imported.chikn";
    const outputPath = await save({
      title: "Save Converted Project As",
      defaultPath: defaultName,
    });
    if (!outputPath) return;

    setBusy(true);
    await importScrivener(scrivPath, outputPath);
    setBusy(false);
  };

  const handleOpenRecent = async (path: string) => {
    setBusy(true);
    await openProject(path);
    setBusy(false);
  };

  return (
    <div className="welcome">
      <div className="welcome-inner">
        <h1 className="welcome-title">ChickenScratch</h1>
        <p className="welcome-sub">Where messy drafts become masterpieces</p>

        {pandocWarning && (
          <div className="welcome-warning">
            <AlertTriangle size={16} />
            {pandocWarning}
          </div>
        )}

        {error && (
          <div className="welcome-error" onClick={clearError}>
            {error}
          </div>
        )}

        <div className="welcome-actions">
          <button className="welcome-btn" onClick={handleOpen} disabled={busy}>
            <FolderOpen size={20} />
            <span>Open Project</span>
          </button>
          <button className="welcome-btn" onClick={handleCreate} disabled={busy}>
            <FilePlus size={20} />
            <span>New Project</span>
          </button>
          <button className="welcome-btn" onClick={handleImport} disabled={busy}>
            <FileDown size={20} />
            <span>Import Scrivener</span>
          </button>
        </div>

        {recent.length > 0 && (
          <div className="welcome-recent">
            <h3 className="welcome-recent-title">
              <Clock size={14} /> Recent Projects
            </h3>
            {recent.map((r) => (
              <button
                key={r.path}
                className="welcome-recent-item"
                onClick={() => handleOpenRecent(r.path)}
                disabled={busy}
                title={r.path}
              >
                <span className="welcome-recent-name">{r.name}</span>
                <span className="welcome-recent-path">
                  {r.path.replace(/^\/Users\/\w+\//, "~/")}
                </span>
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

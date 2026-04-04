import { useState } from "react";
import { FolderOpen, FilePlus, FileDown } from "lucide-react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useProjectStore } from "../../stores/projectStore";

export function Welcome() {
  const openProject = useProjectStore((s) => s.openProject);
  const createProject = useProjectStore((s) => s.createProject);
  const importScrivener = useProjectStore((s) => s.importScrivener);
  const error = useProjectStore((s) => s.error);
  const clearError = useProjectStore((s) => s.clearError);
  const [busy, setBusy] = useState(false);

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
      // Extract name from the chosen path
      const name = dir.split("/").pop()?.replace(".chikn", "") || "Untitled";
      // Pass the parent directory — create_project appends {name}.chikn
      const parent = dir.substring(0, dir.lastIndexOf("/"));
      await createProject(name, parent);
      setBusy(false);
    }
  };

  const handleImport = async () => {
    const scrivPath = await open({
      directory: true,
      title: "Select .scriv Project",
    });
    if (!scrivPath) return;

    const outputPath = await save({
      title: "Save Converted Project As",
      defaultPath: scrivPath.split("/").pop()?.replace(".scriv", ".chikn") || "Imported.chikn",
    });
    if (!outputPath) return;

    setBusy(true);
    await importScrivener(scrivPath, outputPath);
    setBusy(false);
  };

  return (
    <div className="welcome">
      <div className="welcome-inner">
        <h1 className="welcome-title">Chicken Scratch</h1>
        <p className="welcome-sub">Where messy drafts become masterpieces</p>

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
      </div>
    </div>
  );
}

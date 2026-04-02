import { FolderOpen, FilePlus, FileDown } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { useProjectStore } from "../../stores/projectStore";

export function Welcome() {
  const openProject = useProjectStore((s) => s.openProject);

  const handleOpen = async () => {
    const selected = await open({
      directory: true,
      title: "Open .chikn Project",
      filters: [],
    });
    if (selected) {
      await openProject(selected);
    }
  };

  return (
    <div className="welcome">
      <div className="welcome-inner">
        <h1 className="welcome-title">Chicken Scratch</h1>
        <p className="welcome-sub">Where messy drafts become masterpieces</p>

        <div className="welcome-actions">
          <button className="welcome-btn" onClick={handleOpen}>
            <FolderOpen size={20} />
            <span>Open Project</span>
          </button>
          <button className="welcome-btn" disabled>
            <FilePlus size={20} />
            <span>New Project</span>
          </button>
          <button className="welcome-btn" disabled>
            <FileDown size={20} />
            <span>Import Scrivener</span>
          </button>
        </div>
      </div>
    </div>
  );
}

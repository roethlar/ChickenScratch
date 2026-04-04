import { useState } from "react";
import { useProjectStore } from "./stores/projectStore";
import { Welcome } from "./components/welcome/Welcome";
import { Binder } from "./components/binder/Binder";
import { Editor } from "./components/editor/Editor";
import { Corkboard } from "./components/corkboard/Corkboard";
import { Inspector } from "./components/inspector/Inspector";
import { PenLine, LayoutGrid, PanelRight } from "lucide-react";

type View = "editor" | "corkboard";

export default function App() {
  const project = useProjectStore((s) => s.project);
  const [view, setView] = useState<View>("editor");
  const [showInspector, setShowInspector] = useState(false);

  if (!project) {
    return <Welcome />;
  }

  return (
    <div className="app">
      <Binder />
      <div className="main-area">
        <div className="view-toolbar">
          <button
            className={`view-btn ${view === "editor" ? "active" : ""}`}
            onClick={() => setView("editor")}
            title="Editor"
          >
            <PenLine size={16} />
          </button>
          <button
            className={`view-btn ${view === "corkboard" ? "active" : ""}`}
            onClick={() => setView("corkboard")}
            title="Corkboard"
          >
            <LayoutGrid size={16} />
          </button>
          <div style={{ flex: 1 }} />
          <button
            className={`view-btn ${showInspector ? "active" : ""}`}
            onClick={() => setShowInspector(!showInspector)}
            title="Inspector"
          >
            <PanelRight size={16} />
          </button>
        </div>
        {view === "editor" ? <Editor /> : <Corkboard />}
      </div>
      {showInspector && <Inspector />}
    </div>
  );
}

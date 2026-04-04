import { useState, useEffect } from "react";
import { useProjectStore } from "./stores/projectStore";
import { useSettingsStore } from "./stores/settingsStore";
import { Welcome } from "./components/welcome/Welcome";
import { Binder } from "./components/binder/Binder";
import { Editor } from "./components/editor/Editor";
import { Corkboard } from "./components/corkboard/Corkboard";
import { Inspector } from "./components/inspector/Inspector";
import {
  PenLine,
  LayoutGrid,
  PanelRight,
  History,
  Maximize,
  FileOutput,
  BookOpen as BookOpenIcon,
  Sun,
  Moon,
  BookOpen,
  Search,
} from "lucide-react";
import { CommandPalette } from "./components/command-palette/CommandPalette";
import { Revisions } from "./components/revisions/Revisions";
import { Preview } from "./components/preview/Preview";
import { save } from "@tauri-apps/plugin-dialog";
import { compileProject } from "./commands/io";

type View = "editor" | "corkboard" | "preview";

export default function App() {
  const project = useProjectStore((s) => s.project);
  const { theme, setTheme, focusMode, toggleFocusMode } = useSettingsStore();
  const [view, setView] = useState<View>("editor");
  const [showInspector, setShowInspector] = useState(false);
  const [showRevisions, setShowRevisions] = useState(false);
  const [showPalette, setShowPalette] = useState(false);

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key === "k") {
        e.preventDefault();
        setShowPalette((s) => !s);
      }
      if (mod && e.shiftKey && e.key === "f") {
        e.preventDefault();
        toggleFocusMode();
      }
      if (e.key === "Escape") {
        if (showPalette) setShowPalette(false);
        else if (focusMode) toggleFocusMode();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [focusMode, showPalette, toggleFocusMode]);

  if (!project) {
    return <Welcome />;
  }

  const handleCompile = async () => {
    if (!project) return;
    const outputPath = await save({
      title: "Export Manuscript",
      defaultPath: `${project.name}.docx`,
      filters: [
        { name: "Word", extensions: ["docx"] },
        { name: "PDF", extensions: ["pdf"] },
        { name: "EPUB", extensions: ["epub"] },
        { name: "HTML", extensions: ["html"] },
        { name: "OpenDocument", extensions: ["odt"] },
      ],
    });
    if (!outputPath) return;
    const ext = outputPath.split(".").pop() || "docx";
    try {
      await compileProject(project.path, outputPath, ext, project.name);
      alert("Export complete: " + outputPath);
    } catch (e) {
      alert("Export failed: " + e);
    }
  };

  const themeIcons: Record<string, typeof Sun> = {
    light: Sun,
    dark: Moon,
    sepia: BookOpen,
  };
  const nextTheme = theme === "light" ? "dark" : theme === "dark" ? "sepia" : "light";
  const ThemeIcon = themeIcons[theme];

  return (
    <div className={`app ${focusMode ? "focus-mode" : ""}`}>
      {focusMode && <div className="binder-reveal" />}
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
          <button
            className={`view-btn ${view === "preview" ? "active" : ""}`}
            onClick={() => setView("preview")}
            title="Manuscript Preview"
          >
            <BookOpenIcon size={16} />
          </button>
          <div style={{ flex: 1 }} />
          <button
            className="view-btn"
            onClick={handleCompile}
            title="Export manuscript"
          >
            <FileOutput size={16} />
          </button>
          <button
            className="view-btn"
            onClick={() => setTheme(nextTheme)}
            title={`Theme: ${theme} (click for ${nextTheme})`}
          >
            <ThemeIcon size={16} />
          </button>
          <button
            className={`view-btn ${focusMode ? "active" : ""}`}
            onClick={toggleFocusMode}
            title="Focus mode (Cmd+Shift+F)"
          >
            <Maximize size={16} />
          </button>
          <button
            className={`view-btn ${showRevisions ? "active" : ""}`}
            onClick={() => setShowRevisions(!showRevisions)}
            title="Revisions"
          >
            <History size={16} />
          </button>
          <button
            className={`view-btn ${showInspector ? "active" : ""}`}
            onClick={() => setShowInspector(!showInspector)}
            title="Inspector"
          >
            <PanelRight size={16} />
          </button>
        </div>
        {view === "editor" ? <Editor /> : view === "corkboard" ? <Corkboard /> : <Preview />}
      </div>
      {showRevisions && <Revisions />}
      {showInspector && <Inspector />}
      <CommandPalette open={showPalette} onClose={() => setShowPalette(false)} />
    </div>
  );
}

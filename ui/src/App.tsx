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
  Maximize,
  Sun,
  Moon,
  BookOpen,
} from "lucide-react";

type View = "editor" | "corkboard";

export default function App() {
  const project = useProjectStore((s) => s.project);
  const { theme, setTheme, focusMode, toggleFocusMode } = useSettingsStore();
  const [view, setView] = useState<View>("editor");
  const [showInspector, setShowInspector] = useState(false);

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.shiftKey && e.key === "f") {
        e.preventDefault();
        toggleFocusMode();
      }
      if (e.key === "Escape" && focusMode) {
        toggleFocusMode();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [focusMode, toggleFocusMode]);

  if (!project) {
    return <Welcome />;
  }

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
          <div style={{ flex: 1 }} />
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
